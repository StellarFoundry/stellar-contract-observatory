# Installation

## Requirements

- A recent stable Rust toolchain (see the workspace `rust-version`).
- No network access is required for inspection, diffing, or compatibility.

## From source

```bash
git clone https://github.com/StellarFoundry/stellar-contract-observatory
cd stellar-contract-observatory
cargo build --release
```

## Verifying the build

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

## Platform support

The CLI and libraries are pure Rust and build on Linux, macOS, and Windows. CI
runs the test suite on all three.

## Packaging

Crates are not published to crates.io yet; the project is pre-1.0. Distribution
and release packaging are tracked as future work.
