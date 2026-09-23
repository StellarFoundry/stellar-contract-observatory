# Stellar Contract Observatory

Contract intelligence, inspection, interface analysis, compatibility, deployment
verification, and reproducibility tooling for **Stellar / Soroban** contracts.

`stellar-contract-platform` is a Rust-first, offline-by-default toolkit that
answers concrete questions about a Soroban contract artifact:

- What is inside this WASM?
- What functions, types, and events does it expose?
- How does this interface differ from the previous version?
- Is the change compatible under explicit, documented rules?
- Does this local artifact match the deployed contract?
- What is the deterministic artifact and interface fingerprint?
- What build metadata is embedded, and does it match another build?

## What it does

- **Inspect** a WASM artifact safely (bounded, no execution): sections, imports,
  exports, memories, custom sections, code size, and a SHA-256 artifact hash.
- **Spec** extraction: decode the `contractspecv0` section into a normalized,
  parser-independent model of functions, structs, unions, enums, error enums,
  and events.
- **Interface model**: canonicalize the interface and compute a deterministic
  interface fingerprint that ignores declaration order but preserves semantic
  order (parameters, fields, cases).
- **Diff**: enumerate interface changes and classify them as breaking,
  non-breaking, or informational.
- **Compat**: apply an explicit, configurable policy to produce a
  `compatible` / `incompatible` / `unknown` verdict with reasons.
- **Events**: decode event fixtures (base64 `ScVal` topics/data) and filter by
  contract, topic, and type.
- **Deployment** (fixture-backed): decode the contract instance ledger entry to
  recover the deployed WASM hash.
- **Verify**: compare a local artifact hash against the deployed executable hash
  and compare build metadata documents.
- **Fingerprint**: SHA-256 artifact and canonical interface fingerprints.
- **Security heuristics**: a bounded set of deterministic interface/artifact
  checks (not an auditor).

Every command supports stable machine-readable JSON via `--json`.

## What it does NOT do

- It does not execute contract code. Inspection never runs untrusted WASM.
- It does not claim to be a security auditor. Heuristics are bounded and
  documented, and are not a substitute for an audit.
- It does not reproduce builds from source. Verification compares artifact
  identity (hashes), not source-level reproducibility.
- It does not ship a live HTTP RPC transport yet. Deployment and verification
  run against deterministic RPC fixtures; the live transport is tracked as work.
- It does not replace the Stellar CLI, an SDK, or a binding generator.

## How it differs

- **vs. `stellar-devkit`**: DevKit is a general Stellar/Soroban developer
  infrastructure toolkit (RPC, XDR/SCVal inspection, general utilities). The
  Observatory is specifically about *contract intelligence*: interface models,
  interface diffing, documented compatibility rules, deployment verification,
  and reproducibility metadata.
- **vs. Stellar CLI**: The CLI builds, signs, and deploys. The Observatory
  analyzes contract artifacts and interfaces and produces automation-friendly
  reports.
- **vs. SDKs / binding generators**: Those generate code to call contracts. The
  Observatory models and compares the interface itself.
- **vs. auditors**: The Observatory provides deterministic, bounded checks and
  interfaces; it makes no security guarantees.

## Install and build

```bash
cargo build --release
./target/release/stellar-contract-platform doctor
```

A recent stable Rust toolchain is required.

## Quickstart

```bash
# Inspect an artifact (JSON)
stellar-contract-platform inspect contract.wasm --json

# Summarize the contract specification
stellar-contract-platform spec contract.wasm

# Diff two interfaces and assess compatibility
stellar-contract-platform diff old.wasm new.wasm
stellar-contract-platform compat old.wasm new.wasm

# Fingerprints
stellar-contract-platform fingerprint contract.wasm --json

# Analyze an event fixture
stellar-contract-platform events events.json --topic transfer

# Verify against a deterministic RPC fixture
stellar-contract-platform verify artifact contract.wasm \
  --contract C... --rpc-fixture rpc.json --network testnet
```

Exit codes: `0` success, `2` usage, `3` input, `4` parse, `5` unsupported,
`6` incompatible, `7` verification mismatch, `8` inconclusive.

## Workspace layout

| Crate | Responsibility |
| ----- | -------------- |
| `observatory-core` | errors, exit codes, limits, hashing, networks, output format |
| `observatory-wasm` | bounded WASM inspection and artifact fingerprinting |
| `observatory-spec` | `contractspecv0` parsing and normalization |
| `observatory-interface` | canonical interface model and interface fingerprinting |
| `observatory-events` | event fixture decoding, normalization, filtering |
| `observatory-diff` | deterministic interface diffing |
| `observatory-compat` | documented compatibility rules engine |
| `observatory-rpc` | transport abstraction, typed models, mock/fixture transport |
| `observatory-deployment` | deployed contract inspection and WASM hash retrieval |
| `observatory-verify` | local-vs-deployed verification and build metadata |
| `observatory-audit` | bounded interface/artifact heuristics |
| `observatory-output` | versioned report envelopes and JSON rendering |
| `observatory-platform` | service layer, API-key auth, RBAC, rate limiting, config |
| `observatory-api` | versioned REST API, routing, HTTP adapter, OpenAPI |
| `observatory-cli` | the `stellar-contract-platform` binary |
| `observatory-testutil` | deterministic fixture builders (test-only) |

## Developer platform

The same contract intelligence is available as a versioned REST API:

```bash
stellar-contract-platform api serve --bind 127.0.0.1:8080 --no-auth
stellar-contract-platform api openapi
```

The API adds API-key authentication, role-based authorization, rate limiting,
request IDs, and a generated OpenAPI document. It is a thin transport over the
same services the CLI uses and contains no analysis logic. See [docs/API.md](docs/API.md).

## Documentation

- [Architecture](docs/architecture.md)
- [Getting started](docs/getting-started.md)
- [CLI reference](docs/cli.md)
- [Configuration](docs/configuration.md)
- [REST API](docs/API.md)
- [Contract inspection](docs/contract-inspection.md)
- [Contract specifications](docs/contract-specifications.md)
- [Interface diff](docs/interface-diff.md)
- [Compatibility](docs/compatibility.md)
- [Events](docs/events.md)
- [Deployment](docs/deployment.md)
- [Verification](docs/verification.md)
- [Reproducibility](docs/reproducibility.md)
- [Fingerprinting](docs/fingerprinting.md)
- [Testing](docs/testing.md)
- [Security](docs/security.md)
- [CI integration](docs/ci.md)
- [Roadmap](docs/roadmap.md)
- [Contributing through a Drips Wave](docs/WAVE.md)
- [Project status](docs/PROJECT_STATUS.md)

## Status

Early but real. See [docs/PROJECT_STATUS.md](docs/PROJECT_STATUS.md) for exactly
what is implemented, known limitations, and open work. Only documented features
exist.

## License

Apache-2.0. See [LICENSE](LICENSE).
