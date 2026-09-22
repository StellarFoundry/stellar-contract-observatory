# Verification

Verification compares a local artifact against a deployment. It is explicit about
what is being compared.

## Artifact verification

`verify artifact` compares the SHA-256 of the local artifact against the deployed
executable hash.

```bash
stellar-contract-observatory verify artifact contract.wasm \
  --contract C... --rpc-fixture rpc.json --network testnet
```

| Status | Meaning | Exit code |
| ------ | ------- | --------- |
| `MATCH` | Local hash equals the deployed executable hash. | `0` |
| `MISMATCH` | Hashes differ. | `7` |
| `INSUFFICIENT_DATA` | No instance entry, or the contract is not WASM-backed. | `8` |
| `ERROR` | The comparison could not be completed. | `8` |

### What this proves, and what it does not

Matching hashes prove that the local and deployed **artifacts** are identical.
They do **not** prove that the artifact was built reproducibly from a given
source tree. Source-level reproducibility is a separate problem; see
[reproducibility.md](reproducibility.md). The tool never claims source-level
verification from a hash comparison.

## Build metadata

`verify metadata <file>` parses a build metadata JSON document and prints its
key/value pairs.

`verify compare <left> <right>` compares two documents and reports the union of
differing keys:

```bash
stellar-contract-observatory verify compare build-a.json build-b.json --json
```

The comparison reports `equal`, `compared_keys`, and `differences`.
