# Project status

_Last updated after the platform/API layer was added. This page lists only what
exists and tests support._

## Current architecture

Sixteen crates with a strict downward dependency direction. The contract
intelligence engine is unchanged; a new platform layer wraps it:

```
HTTP adapter ──► API router (+ auth/RBAC/rate limit) ──► platform services ──► engine
 observatory-api        observatory-api::api            observatory-platform   wasm/spec/diff/...
```

See [architecture.md](architecture.md) for the full graph and the implementation
matrix.

## Implemented capabilities

| Area | Status | Where |
| ---- | ------ | ----- |
| Bounded WASM inspection | Implemented | `observatory-wasm` |
| Artifact fingerprint (SHA-256) | Implemented | `observatory-wasm` |
| `contractspecv0` parsing + validation | Implemented | `observatory-spec` |
| Canonical interface model + fingerprint | Implemented | `observatory-interface` |
| Interface diff (severity classification) | Implemented | `observatory-diff` |
| Compatibility policy engine | Implemented | `observatory-compat` |
| Event fixture decode + filter | Implemented | `observatory-events` |
| RPC abstraction + mock/fixture transport | Implemented | `observatory-rpc` |
| Deployed WASM hash recovery (fixture) | Implemented | `observatory-deployment` |
| Local-vs-deployed verification (hash) | Implemented | `observatory-verify` |
| Build metadata parse + compare | Implemented | `observatory-verify` |
| Bounded security heuristics | Implemented | `observatory-audit` |
| Versioned JSON report envelopes | Implemented | `observatory-output` |
| Application service layer | Implemented | `observatory-platform` |
| API-key authentication | Implemented | `observatory-platform` |
| RBAC (viewer/developer/maintainer/admin) | Implemented | `observatory-platform` |
| Fixed-window rate limiting | Implemented | `observatory-platform` |
| REST API (`/api/v1`) | Implemented | `observatory-api` |
| OpenAPI document (+ sync test) | Implemented | `observatory-api` |
| CLI: inspect/spec/events/diff/compat/fingerprint/deployment/verify/doctor/api | Implemented | `observatory-cli` |

## Known limitations

- **No live RPC transport.** Deployment and verification use a deterministic
  fixture transport. `--rpc-url` is validated but not dialled (issue #13).
- **Hash-only artifact verification.** No source-level reproducibility; no
  rebuild (future).
- **No deployed interface comparison.** The deployed WASM is not fetched
  (issue #14/#15).
- **No persistence.** API keys, users, audit events, and webhooks are in-memory
  only; a restart loses them.
- **No webhooks, SDKs, or GitHub Action** yet.
- **HTTP adapter is local/test grade.** Bounded and deterministic, not a
  hardened edge server.
- **Not published** to crates.io.

## Active development areas

- Live RPC transport and SSRF hardening (must land together; #13).
- Deployed code retrieval and interface comparison (#14, #15).
- Rename detection (#8) and Markdown diff (#9).
- Configurable compatibility policy file (#12).
- Persistence and webhook delivery (planned).

## Contributor opportunities

The open issues are the contributor surface. Each carries Context, Problem,
Objective, Scope, Relevant modules/files, Implementation guidance, Acceptance
criteria, Testing, an example commit message, and guidelines. See
[ISSUE_BACKLOG.md](ISSUE_BACKLOG.md).

## Testing status

- `cargo test --workspace --all-features`: **129 tests pass** locally.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean.
- `cargo doc --workspace --no-deps` with `-D warnings`: clean.
- Tests include crate unit tests, 10 CLI end-to-end tests, and 18 API tests
  (routing, auth, RBAC, rate limiting, OpenAPI synchronization).

## Security status

- Never executes untrusted WASM; all parsing is bounded.
- API keys are random and stored only as SHA-256 hashes (never serialized).
- Authorization is enforced per endpoint.
- HTTP bodies, artifacts, and header counts are bounded.
- No server-initiated HTTP requests exist yet, so SSRF is structurally absent;
  the live transport must add SSRF validation before it lands.

## Deployment and release status

- Not deployed as a hosted service.
- Not published to crates.io.
- CI runs format, clippy, tests (Linux/Windows/macOS), and docs.

## Roadmap

See [roadmap.md](roadmap.md).
