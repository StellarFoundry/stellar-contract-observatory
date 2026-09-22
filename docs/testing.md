# Testing

## The local gate

Run the same checks as CI before opening a pull request:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

## Test layout

- Unit tests live next to the code in `#[cfg(test)]` modules.
- CLI integration tests live in `crates/observatory-cli/tests/cli.rs` and spawn
  the built binary against deterministic fixtures.
- `observatory-testutil` builds fixtures from bytes and real `stellar-xdr`
  values; no checked-in binary blobs are required so tests stay reviewable.

## Determinism rules

- Unit tests must not touch the network.
- Fixtures are built deterministically; tests assert exact values.
- RPC behaviour is exercised through `MockTransport` and fixture JSON.

## Fixtures

| Fixture | Built by |
| ------- | -------- |
| Minimal WASM module | `observatory_testutil::minimal_module` |
| Module with a custom section | `observatory_testutil::module_with_custom_section` |
| Module with a contract spec | `observatory_testutil::spec::*` + `module_with_spec` |
| Event fixtures | `observatory-events` test helpers (base64 `ScVal`) |
| Ledger entries | `stellar-xdr` `LedgerEntryData` encoded in tests |

## Adding tests

- Decoding changes must include malformed-input tests.
- New diff/compat rules must include a positive and a negative case.
- New CLI flags must include an integration test asserting exit code and output.
