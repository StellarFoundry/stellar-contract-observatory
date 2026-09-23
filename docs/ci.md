# CI integration

The Observatory is designed to run in CI: deterministic output, stable JSON
envelopes, and meaningful exit codes.

## Exit codes for gating

| Code | Use |
| ---- | --- |
| `0` | pass |
| `6` | incompatible interface change (`compat`) |
| `7` | verification mismatch (`verify artifact`) |
| `8` | verification inconclusive |

## Example: detect breaking interface changes

```yaml
name: Interface compatibility
on: pull_request
jobs:
  compat:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
        with:
          fetch-depth: 0
      - uses: dtolnay/rust-toolchain@stable
      - name: Build
        run: cargo build --release
      - name: Build baseline
        run: |
          git worktree add baseline origin/${{ github.base_ref }}
          cargo build --release --manifest-path baseline/Cargo.toml || true
      - name: Compare interfaces
        run: |
          ./target/release/stellar-contract-platform compat \
            baseline/target/release/contract.wasm \
            contract.wasm --json > compat.json
```

## Example: verify a released artifact

```yaml
      - name: Verify artifact
        run: |
          ./target/release/stellar-contract-platform verify artifact \
            contract.wasm --contract C... --rpc-fixture ci/rpc.json --network testnet
```

## JSON reports

Every command supports `--json`. Reports are wrapped in a versioned envelope:

```json
{ "schema_version": "1.0", "kind": "compat", "tool": "...", "version": "...", "data": {} }
```

Automation should pin `schema_version` and treat unknown `kind` values as errors.

## GitHub Action

A reusable Action and an `observatory.yml` configuration schema are **not**
implemented yet; they are tracked as future work. Until then, invoke the binary
directly as shown above.
