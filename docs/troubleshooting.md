# Troubleshooting

## `error: cannot read ...: The system cannot find the file specified`

The path passed to a command does not exist. The CLI exits `3` for input errors.

## `error: wasm error: input does not start with the WASM magic bytes`

The file is not a WASM module. Confirm you are pointing at a `.wasm` artifact.

## `error: spec error: WASM module has no 'contractspecv0' section`

The module is WASM but has no Soroban contract specification. It may be a plain
library, or built without the Soroban contract macro.

## `spec` shows `unknown type` validation warnings

A type referenced by a function or field is not declared in the specification.
This usually indicates a build issue or a hand-crafted fixture.

## `compat` exits `6`

The change is incompatible under the active policy. Inspect the reasons, or use
`--policy lenient` if your consumers tolerate removals (deliberately).

## `compat` returns `unknown`

The baseline interface is empty. There is no public surface to compare, so the
tool refuses to claim compatibility.

## `verify artifact` exits `8`

The deployment data was insufficient: no instance entry, or the contract is not
WASM-backed. Confirm the contract id and the RPC fixture.

## `deployment` says the live transport is unavailable

This build ships only a deterministic fixture transport. Provide
`--rpc-fixture`. A live HTTP transport is tracked as future work.

## JSON mode prints an error object

With `--json`, errors are emitted as `{ "kind": "error", "code": ..., "message":
... }` on stdout, and the process exits with the corresponding code.
