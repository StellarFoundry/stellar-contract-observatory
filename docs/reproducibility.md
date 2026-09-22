# Reproducibility

Reproducibility is about whether an artifact can be rebuilt from source and
compared. This project does **not** reproduce builds; it reads and compares the
metadata a build embeds, so that a reproducibility decision can be made.

## Build metadata sources

The Soroban toolchain can embed metadata in custom sections:

- `contractenvmetav0` — environment metadata (`SCEnvMetaEntry`), including the
  target protocol and pre-release.
- `contractmetav0` — build metadata (`SCMetaEntry`) as key/value pairs, for
  example compiler or SDK versions.

`observatory-verify` parses both with the real XDR types. Unknown keys are
preserved verbatim, so the model stays extensible.

## Comparing metadata

`verify compare` reports:

- `equal` — whether all keys agree,
- `compared_keys` — the size of the union of keys,
- `differences` — per-key `left`/`right` values (absent keys are `null`).

## Concepts kept separate

| Concept | Covered by |
| ------- | ---------- |
| Artifact identity | SHA-256 of the WASM bytes |
| Interface identity | canonical interface fingerprint |
| Build metadata equality | `verify compare` |
| Source reproducibility | **not implemented** (requires rebuilding) |

Do not collapse these. Equal metadata does not prove reproducibility, and equal
artifacts do not prove equal source.
