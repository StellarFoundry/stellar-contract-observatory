# Project status

_Last updated after the initial implementation. This page is kept honest: it
lists only what exists and tests support._

## Implemented capabilities

| Area | Status | Where |
| ---- | ------ | ----- |
| Bounded WASM inspection | Implemented | `observatory-wasm` |
| Artifact fingerprint (SHA-256) | Implemented | `observatory-wasm` |
| `contractspecv0` parsing + validation | Implemented | `observatory-spec` |
| Canonical interface model | Implemented | `observatory-interface` |
| Interface fingerprint (+ known vector) | Implemented | `observatory-interface` |
| Interface diff (severity classification) | Implemented | `observatory-diff` |
| Compatibility policy engine | Implemented | `observatory-compat` |
| Event fixture decode + filter | Implemented | `observatory-events` |
| RPC abstraction + mock/fixture transport | Implemented | `observatory-rpc` |
| Endpoint validation (https/loopback) | Implemented | `observatory-rpc` |
| Deployed WASM hash recovery | Implemented | `observatory-deployment` |
| Local-vs-deployed verification | Implemented | `observatory-verify` |
| `contractmetav0` / `contractenvmetav0` parsing | Implemented | `observatory-verify` |
| Build metadata comparison | Implemented | `observatory-verify` |
| Bounded security heuristics | Implemented | `observatory-audit` |
| Versioned JSON report envelopes | Implemented | `observatory-output` |
| CLI: inspect/spec/events/diff/compat/fingerprint/deployment/verify/doctor | Implemented | `observatory-cli` |

## Known limitations

- **No live RPC transport.** Deployment and verification use a deterministic
  fixture transport. `--rpc-url` is validated but not dialled.
- **Hash-only artifact verification.** No source-level reproducibility; no
  rebuild.
- **No deployed interface comparison.** The deployed WASM is not fetched and
  compared yet.
- **No rename detection** in the diff engine.
- **No GitHub Action / `observatory.yml`** yet.
- **No Markdown report generation** yet.
- **No fuzzing** yet; parsing is unit-tested against malformed input only.
- **Not published** to crates.io.

## Test status

- `cargo test --workspace --all-features`: **94 tests pass** locally
  (Linux/Windows/macOS CI is configured; results are reported on GitHub).
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean.
- `cargo doc --workspace --no-deps` with `-D warnings`: clean.

Tests include unit tests per crate and 9 end-to-end CLI integration tests.

## CI status

A CI workflow (`.github/workflows/ci.yml`) runs format, clippy, tests on Linux,
Windows, and macOS, and a documentation build. CI results live on the repository's
Actions page; this document does not claim a passing run unless it was observed.

## Architecture

Fourteen crates with a strict downward dependency direction; see
[architecture.md](architecture.md). The CLI is the only composition point.

## Open technical areas

- Live transport and deployed interface comparison.
- Reproducibility (rebuild) workflow.
- Rename detection.
- Fuzzing and property testing.
- Report generation and the CI Action.
- Release packaging.

## Roadmap

See [roadmap.md](roadmap.md). Concrete tasks are tracked as GitHub issues and
summarized in [ISSUE_BACKLOG.md](ISSUE_BACKLOG.md).
