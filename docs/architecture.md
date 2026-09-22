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

## Implementation matrix

Status is based on source and tests, not on the roadmap. Evidence is the crate
and its test suite.

| Capability | Status | Evidence | Missing work |
| ---------- | ------ | -------- | ------------ |
| WASM inspection | Implemented | `observatory-wasm`, 8 tests | start/data/element/table detail (#4) |
| Artifact fingerprint | Implemented | `observatory-wasm::artifact_fingerprint` | — |
| Contract metadata | Implemented | `observatory-verify` parses `contractmetav0`/`contractenvmetav0` | typed key model (#18) |
| Contract spec parsing | Implemented | `observatory-spec`, 9 tests | forward-compat entry kinds (#6) |
| Interface model | Implemented | `observatory-interface`, 7 tests | JSON schema (#34) |
| Interface diff | Implemented | `observatory-diff`, 6 tests | rename detection (#8), Markdown (#9) |
| Compatibility | Implemented | `observatory-compat`, 7 tests | policy file (#12) |
| Events | Implemented | `observatory-events`, 7 tests | raw XDR (#10), RPC (#36) |
| RPC abstraction | Implemented (mock/fixture) | `observatory-rpc`, 6 tests | live HTTPS transport (#13) |
| Deployment inspection | Implemented (fixture) | `observatory-deployment`, 6 tests | deployed WASM fetch (#14) |
| Verification | Implemented (hash only) | `observatory-verify`, 5 tests | deployed interface comparison (#15) |
| Reproducibility metadata | Implemented (parse/compare) | `observatory-verify` | rebuild workflow (future) |
| Security heuristics | Implemented (bounded) | `observatory-audit`, 5 tests | more rules (#21), FP corpus (#22) |
| JSON reports | Implemented | `observatory-output` + API envelope | JSON schemas (#24) |
| CLI | Implemented | `observatory-cli`, 10 e2e tests | completions (#25), `--format` (#38) |
| REST API | Implemented | `observatory-api`, 18 tests | deployed endpoints, more routes |
| Authentication | Implemented (API keys) | `observatory-platform` auth, 4 tests | persistence/rotation |
| Authorization (RBAC) | Implemented | `observatory-platform` rbac, 3 tests | resource-level roles |
| Rate limiting | Implemented (fixed window) | `observatory-platform` ratelimit, 3 tests | distributed limiting |
| OpenAPI | Implemented + sync test | `observatory-api::openapi` | richer schemas |
| Webhooks | Missing | — | #new (backlog) |
| SDKs | Missing | — | #new (backlog) |
| Persistence | Missing | — | #new (backlog) |
| Live RPC | Missing | — | #13 |

## Platform layer

The developer-platform layer wraps the intelligence engine without duplicating
any analysis:

```
HTTP adapter (observatory-api::http)
        │
Router + middleware (observatory-api::api)  ← auth, RBAC, rate limit
        │
Application services (observatory-platform::service::Observatory)
        │
Contract-intelligence engine (wasm/spec/interface/diff/compat/...)
```

- `observatory-platform` owns authentication, authorization, rate limiting, and
  configuration. It depends on the engine; the engine never depends on it.
- `observatory-api` owns HTTP concerns, request DTOs, routing, and OpenAPI. It
  contains no analysis logic.
- The router is transport-agnostic (`Api::handle` takes a parsed request and
  returns a response), so it is fully testable without sockets.

## Platform security model

- **No execution.** The API only calls the same bounded, non-executing
  inspection path as the CLI.
- **Authentication.** Keys are random, stored only as SHA-256 hashes, and the
  hash is never serialized. The plaintext is returned once at creation.
- **Authorization.** Roles are hierarchical (`viewer ⊂ developer ⊂ maintainer ⊂
  admin`) and checked per endpoint via explicit permissions.
- **Bounds.** HTTP bodies are bounded (`MAX_BODY_BYTES`), artifact payloads are
  bounded (`MAX_WASM_BYTES`), and header counts are bounded.
- **Rate limiting.** Fixed-window, per key or client, with standard headers.
- **Errors.** Stable machine-readable codes; internal messages are mapped to
  statuses without leaking stack details.

SSRF is mitigated structurally: this build performs no server-initiated HTTP
requests, so user input cannot drive one. The live RPC transport (#13) must add
SSRF protections before it lands.
