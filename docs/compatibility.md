# Compatibility

`compat` applies an explicit policy to the interface diff and returns a verdict
with reasons. It is intentionally not a boolean.

## Statuses

- `compatible` — no breaking change under the active policy.
- `incompatible` — at least one breaking change under the active policy.
- `unknown` — there is not enough information to decide. In particular, when the
  baseline interface is empty there is no public surface to compare, so the
  result is `unknown` rather than `compatible`.

## Policy rules

| Rule | Strict (default) | Lenient |
| ---- | ---------------- | ------- |
| Removing a function | incompatible | note |
| Changing a function signature | incompatible | note |
| Removing a type | incompatible | note |
| Changing a type definition | incompatible | note |
| Removing an event | incompatible | note |
| Changing an event | incompatible | note |
| Adding a function, type, or event | compatible | compatible |
| Documentation-only change | compatible | compatible |

Relaxing a rule does **not** hide the change: it is reported as a non-breaking
note with its path and explanation.

## Why a policy

Compatibility depends on consumers. A removed event may break an indexer while
being irrelevant to a caller that only invokes functions. The policy makes the
assumption explicit and reviewable rather than baking it into the tool.

## JSON output

```json
{
  "status": "incompatible",
  "policy": { "signature_change_is_breaking": true, ... },
  "reasons": [
    { "breaking": true, "path": "function::transfer", "message": "..." }
  ],
  "diff": { "changes": [], "summary": { } }
}
```

## Exit codes

`compat` exits `6` on `incompatible` and `0` otherwise, so CI can gate merges.
