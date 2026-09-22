# Contributing to Stellar Contract Observatory

Thanks for helping build contract intelligence tooling for Stellar and Soroban.

## Ground rules

- Be respectful; see [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
- Never commit secrets, keys, or credentials.
- Do not claim a feature works unless it is implemented and tested.
- Core inspection must work offline and must never execute untrusted contract code.
- Prefer composing with official crates (`stellar-xdr`, `stellar-strkey`) over
  reimplementing Stellar primitives.

## Setup

```bash
git clone https://github.com/StellarFoundry/stellar-contract-platform
cd stellar-contract-platform
cargo build
cargo test
```

A recent stable Rust toolchain is required. The workspace `rust-version` is the
MSRV.

## Before opening a pull request

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

## Architecture

Read [docs/architecture.md](docs/architecture.md). Crate boundaries are
meaningful: lower-level crates must not depend on higher-level ones. The CLI
composes the libraries; libraries never depend on the CLI.

## Adding functionality

- Every fallible function returns `observatory_core::ObservatoryError`.
- Parsing untrusted input must be bounded and must return structured errors.
- Add deterministic tests; do not require network access in unit tests.
- Decoding changes must include malformed-input tests.
- Update the relevant document under `docs/` and `CHANGELOG.md` for
  user-visible changes.

## Commits

Use [Conventional Commits](https://www.conventionalcommits.org/): `feat`, `fix`,
`refactor`, `test`, `docs`, `ci`, `security`, `perf`, `build`, `chore`.

## Definition of done

- Compiles with clippy `-D warnings`.
- `cargo fmt` is clean.
- Tests added and passing.
- Documentation updated.
- No secrets, placeholders, or fake implementations.

## License

Contributions are licensed under Apache-2.0.
