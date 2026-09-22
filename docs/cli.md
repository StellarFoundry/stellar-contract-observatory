# CLI reference

```
stellar-contract-observatory [OPTIONS] <COMMAND>
```

## Global options

| Option | Description |
| ------ | ----------- |
| `--json` | Emit a machine-readable JSON report envelope (alias for `--format json`). |
| `--format <text\|json>` | Output format; defaults to `text`. |
| `-q`, `--quiet` | Suppress non-essential diagnostics (progress/notes). Primary results still print. |
| `-v`, `--verbose` | Emit diagnostics to stderr (repeatable). |
| `-h`, `--help` | Print help. |
| `-V`, `--version` | Print version. |

## Commands

### `inspect <WASM> [--security]`

Inspects a WASM artifact: size, version, artifact SHA-256, type/function/table/
global counts, memories, imports, exports, custom sections, and code size. With
`--security`, runs bounded heuristics.

### `spec <WASM> [--function <NAME>] [--type <TYPE>]`

Summarizes the contract specification. `--function` prints a single function;
`--type` prints a single user-defined type.

### `events <FILE> [--contract <ID>] [--topic <TEXT>] [--type <TYPE>] [--limit <N>]`

Decodes an event fixture and filters it. `--limit` is applied last.

### `diff <OLD> <NEW>`

Diff two contract interfaces. Changes are classified as breaking,
non-breaking, or informational. Always exits `0`.

### `compat <OLD> <NEW> [--policy strict|lenient]`

Assess compatibility under a documented policy. Exits `6` when incompatible.

### `fingerprint <WASM>`

Prints the artifact SHA-256 and, when a specification is present, the canonical
interface fingerprint.

### `deployment inspect <CONTRACT_ID> --rpc-fixture <FILE> [--network <NET>] [--rpc-url <URL>]`

Inspects a deployed contract using a deterministic RPC fixture. The live HTTP
transport is not shipped yet; `--rpc-url` is validated and recorded only.

### `verify artifact <WASM> --contract <ID> --rpc-fixture <FILE> [--network <NET>]`

Compares the local artifact hash against the deployed executable hash.

- `MATCH` exits `0`.
- `MISMATCH` exits `7`.
- `INSUFFICIENT_DATA` / `ERROR` exit `8`.

### `verify metadata <FILE>` and `verify compare <LEFT> <RIGHT>`

Parse and compare build metadata JSON documents.

### `doctor`

Prints capabilities, schema version, networks, and the RPC transport status.

## Exit codes

| Code | Meaning |
| ---- | ------- |
| `0` | success |
| `2` | usage error |
| `3` | input / filesystem error |
| `4` | parse / decode error |
| `5` | unsupported feature |
| `6` | incompatible interface change |
| `7` | verification mismatch |
| `8` | verification inconclusive |

## JSON envelope

Every `--json` report has the shape:

```json
{
  "schema_version": "1.0",
  "kind": "inspect",
  "tool": "stellar-contract-observatory",
  "version": "0.1.0",
  "data": { }
}
```

Errors serialize as `{ "schema_version", "kind": "error", "code", "message" }`.
