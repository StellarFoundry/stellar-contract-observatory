# Getting started

## Install

Build from source (a recent stable Rust toolchain is required):

```bash
git clone https://github.com/StellarFoundry/stellar-contract-platform
cd stellar-contract-platform
cargo build --release
```

The binary is `target/release/stellar-contract-observatory`.

## First run

```bash
stellar-contract-observatory doctor
```

`doctor` prints capabilities, the active RPC transport (fixture/mock only), and
known networks.

## Inspect an artifact

```bash
stellar-contract-observatory inspect contract.wasm
stellar-contract-observatory inspect contract.wasm --json
```

Inspection is static and bounded; the artifact is never executed.

## Read the interface

```bash
stellar-contract-observatory spec contract.wasm
stellar-contract-observatory spec contract.wasm --function transfer
```

## Compare two versions

```bash
stellar-contract-observatory diff old.wasm new.wasm
stellar-contract-observatory compat old.wasm new.wasm
```

`compat` exits `6` when an interface change is incompatible under the active
policy, which is useful in CI.

## Fingerprints

```bash
stellar-contract-observatory fingerprint contract.wasm --json
```

The artifact fingerprint and the canonical interface fingerprint are different:
equal interfaces do not imply equal implementations.

## Where to go next

- [Contract inspection](contract-inspection.md)
- [Compatibility](compatibility.md)
- [Verification](verification.md)
- [CI integration](ci.md)
