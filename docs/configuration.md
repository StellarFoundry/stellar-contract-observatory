# Configuration

Projects can declare defaults in a configuration file. Flags always win over the
file, and the file wins over built-in defaults.

## Discovery and selection

The CLI loads a configuration file from the current directory if one exists, in
this priority order:

1. `observatory.yml`
2. `observatory.yaml`
3. `observatory.toml`
4. `observatory.json`

Use `--config <FILE>` to point at a specific file. The format is inferred from
the extension. Parsing is **strict**: unknown fields are rejected so a typo does
not silently disable a setting.

## Schema

```yaml
project:
  name: my-project
  network: testnet        # default network for verify/deployment

analysis:
  compatibility:
    policy: strict        # strict | lenient

rpc:
  endpoint: https://soroban-testnet.stellar.org

api:
  require_auth: true
  rate_limit:
    limit: 120
    window_secs: 60

ci:
  fail_on: MISMATCH       # statuses that should fail a build
```

The same content is valid TOML or JSON. Example TOML:

```toml
[project]
network = "testnet"

[analysis.compatibility]
policy = "lenient"

[api]
require_auth = false

[api.rate_limit]
limit = 60
window_secs = 30
```

## What the file affects

| Field | Effect |
| ----- | ------ |
| `project.network` | Default network for `verify` and `deployment` when `--network` is omitted. |
| `analysis.compatibility.policy` | Default policy for `compat` when `--policy` is omitted. |
| `api.require_auth` | Default authentication requirement for `api serve`. |
| `api.rate_limit` | Default rate limit for `api serve`. |
| `rpc.endpoint` | Documented default endpoint (recorded; live transport is future work). |
| `ci.fail_on` | Documented CI failure policy (advisory in this build). |

## Precedence

1. Explicit CLI flags (highest)
2. `--config` file, or a discovered file
3. Built-in defaults (lowest)

## Validation

- Unknown top-level or nested fields are errors.
- An unknown `analysis.compatibility.policy` is an error.
- An unsupported file extension is an error.

## Not included

Live environment-variable overrides are not implemented yet; the `rpc.endpoint`
and `ci.fail_on` fields are recorded and documented but only `project.network`,
`analysis.compatibility.policy`, and the `api.*` fields change behaviour today.
