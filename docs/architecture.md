# Architecture

## Goals

The Observatory turns contract artifacts and interface metadata into
deterministic, machine-readable reports. It is offline-first, never executes
untrusted code, and separates *description* (what changed) from *judgement*
(whether that is compatible).

## Layering

```
                 observatory-cli
                       |
     +-----------------+------------------+
     |        |        |       |          |
  output   audit   verify   compat      events
     |        |        |       |          |
     |        |        |     diff         |
     |        |        |       |          |
     |        |   deployment interface    |
     |        |        |       |          |
     |        |   rpc       spec         |
     |        |        |       |          |
     +--------+--------+-------+----------+
                       |
                 observatory-wasm
                       |
                 observatory-core
```

Dependencies point downward. Lower-level crates never depend on higher-level
ones. The CLI is the only crate that composes the whole graph.

## Crate responsibilities

| Crate | Responsibility | Depends on |
| ----- | -------------- | ---------- |
| `observatory-core` | errors, exit codes, limits, hashing, networks, output format | — |
| `observatory-wasm` | bounded WASM inspection, custom-section extraction, artifact hash | core |
| `observatory-spec` | `contractspecv0` decoding and normalization | core, wasm |
| `observatory-interface` | canonical interface model, interface fingerprint | core, spec, wasm |
| `observatory-events` | event fixture decoding, normalization, filtering | core |
| `observatory-diff` | interface diffing and change classification | core, interface, spec |
| `observatory-compat` | compatibility policy and verdicts | core, diff, interface, spec |
| `observatory-rpc` | transport abstraction, typed models, mock/fixture transport | core |
| `observatory-deployment` | deployed contract inspection, WASM hash recovery | core, rpc |
| `observatory-verify` | verification verdicts and build metadata | core, deployment, rpc, wasm |
| `observatory-audit` | bounded heuristics | core, interface, spec, wasm |
| `observatory-output` | report envelopes, JSON rendering | core |
| `observatory-cli` | command-line interface | all of the above |
| `observatory-testutil` | deterministic fixture builders | core, stellar-xdr |

## Data flow

```
WASM bytes ──► observatory-wasm ──► WasmReport
     │
     └─ contractspecv0 ──► observatory-spec ──► ContractSpec
                                   │
                                   └─► observatory-interface ──► ContractInterface
                                                                     │
                                        ┌────────────────────────────┤
                                        ▼                            ▼
                                   observatory-diff ──► InterfaceDiff
                                        │
                                        ▼
                                   observatory-compat ──► CompatibilityAssessment

RPC fixture ──► observatory-rpc ──► observatory-deployment ──► DeploymentInfo
                                             │
                                             ▼
                                     observatory-verify ──► VerificationResult
```

## Key decisions

- **Never execute WASM.** Inspection is purely static, using a bounded streaming
  parser (`wasmparser`).
- **Real Stellar types.** Specification, events, and ledger entries use
  `stellar-xdr`; there is no bespoke wire format.
- **Description vs. judgement.** `observatory-diff` enumerates changes;
  `observatory-compat` applies an explicit policy. Compatibility is never a
  bare boolean.
- **Canonical ordering.** Name-keyed collections are sorted; sequence-keyed
  collections (parameters, fields, cases) preserve their declared order because
  that order is semantic.
- **Offline by default.** No unit test touches the network. The RPC layer is
  transport-agnostic and ships a deterministic mock/fixture transport.
- **Machine-readable first.** Every report is wrapped in a versioned envelope.

## Extension points

- Add a `Transport` implementation to enable a live RPC provider.
- Add diff/compat rules by extending `observatory-compat` policy.
- Add audit rules by appending findings in `observatory-audit`.
- Add output formats by extending `observatory-output`.
