# Development guide

See [CONTRIBUTING.md](../CONTRIBUTING.md) for the contribution process. This page
covers working in the codebase.

## Prerequisites

- A recent stable Rust toolchain.
- No network access is required to build or test.

## Common commands

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all
cargo run -p observatory-cli -- doctor
```

## Adding a crate

1. Add the crate under `crates/`.
2. Register it in the workspace `members` and add a workspace dependency entry.
3. Keep the dependency direction: lower layers never depend on higher layers.

## Error handling

Return `observatory_core::ObservatoryError`. Use the constructors
(`invalid`, `decode`, `unsupported`, `wasm`, `spec`, `rpc`, `verification`,
`internal`) so the machine-readable code is meaningful.

## Bounds and fixtures

- Never parse untrusted input without a limit from `observatory_core::limits`.
- Add fixtures through `observatory-testutil` rather than binary blobs.

## Documentation

Update the relevant `docs/` page and `CHANGELOG.md` for user-visible changes.
