# REST API

The Observatory exposes its contract-intelligence services over a versioned REST
API. The API is a thin transport over the same services the CLI uses; it performs
no analysis of its own.

- Base path: `/api/v1`
- Content type: `application/json`
- Transport: a minimal bounded HTTP/1.1 adapter (`observatory-api::http`)
- OpenAPI: `stellar-contract-platform api openapi`

## Running locally

```bash
# Read-only dev server with authentication disabled
stellar-contract-platform api serve --bind 127.0.0.1:8080 --no-auth

# With authentication and a pre-registered key
stellar-contract-platform api serve --bind 127.0.0.1:8080 \
  --key developer:sco_<id>_<secret>
```

> The bundled HTTP adapter is intended for local and test use. It is bounded and
> deterministic but is not a hardened edge server.

## Authentication

Analysis endpoints require an API key, supplied either as a bearer token or a
header:

```
Authorization: Bearer sco_<id>_<secret>
X-API-Key: sco_<id>_<secret>
```

Keys are random; only a SHA-256 hash is stored, and the hash is never returned.
The plaintext is shown exactly once at creation.

| Status | Body `code` | Meaning |
| ------ | ----------- | ------- |
| 401 | `unauthorized` | Missing or invalid key |
| 403 | `forbidden` | Key lacks the required permission |

## Authorization

Roles are hierarchical. Each endpoint requires a permission.

| Role | Permissions |
| ---- | ----------- |
| `viewer` | `read_health`, `read_version` |
| `developer` | viewer + `run_analysis` |
| `maintainer` | developer + `manage_webhooks` |
| `admin` | maintainer + `manage_keys` |

Analysis endpoints require `run_analysis`. `GET /health`, `/readiness`, and
`/version` are public.

## Rate limiting

Fixed-window, keyed by API-key id (or client IP when unauthenticated).

```
X-RateLimit-Limit: 120
X-RateLimit-Remaining: 119
X-RateLimit-Reset: 60
```

Exceeding the limit returns `429` with `code: "rate_limited"`. Limits are
configurable via `PlatformConfig` (`rate_limit.limit`, `rate_limit.window_secs`).

## Request IDs

Every response carries `X-Request-Id`, a stable identifier for correlation.

## Endpoints

### `GET /api/v1/health` (public)
`{ "schema_version", "kind": "health", "tool", "version", "data": { "status": "ok" } }`

### `GET /api/v1/readiness` (public)
Reports readiness and whether authentication is required.

### `GET /api/v1/version` (public)
Tool version, schema version, and API version.

### `POST /api/v1/contracts/inspect`
Body: `{ "wasm_base64": "...", "security": false }`.
Returns the WASM report, the canonical interface (when present), and the audit
(when `security` is true).

### `POST /api/v1/contracts/spec`
Body: `{ "wasm_base64": "..." }`. Returns the normalized spec and its summary.

### `POST /api/v1/contracts/diff`
Body: `{ "old_wasm_base64": "...", "new_wasm_base64": "..." }`. Returns the
interface diff.

### `POST /api/v1/contracts/compatibility`
Body: `{ "old_wasm_base64": "...", "new_wasm_base64": "...", "policy": "strict" }`.
Returns the compatibility assessment. `policy` is `strict` (default) or
`lenient`.

### `POST /api/v1/contracts/fingerprint`
Body: `{ "wasm_base64": "..." }`. Returns artifact and interface fingerprints.

### `POST /api/v1/contracts/security`
Body: `{ "wasm_base64": "..." }`. Returns bounded heuristic findings.

## Response envelope

Success:

```json
{ "schema_version": "1.0", "kind": "inspect", "tool": "stellar-contract-platform", "version": "0.1.0", "data": { } }
```

Error:

```json
{ "schema_version": "1.0", "kind": "error", "code": "bad_request", "message": "..." }
```

## Status codes

| Status | Meaning |
| ------ | ------- |
| 200 | success |
| 400 | invalid request or artifact |
| 401 | authentication required or invalid |
| 403 | authenticated but not permitted |
| 404 | unknown route |
| 405 | method not allowed for a known route |
| 413 | request body too large |
| 415 | non-JSON content type |
| 422 | unsupported feature |
| 429 | rate limited |
| 500 | internal error |
| 502 | upstream RPC error |

## OpenAPI synchronization

`observatory-api::openapi::document()` is generated from the single route table
and a test asserts every route is documented and vice versa, so the
specification cannot silently drift.

## Not included

- Webhook management endpoints (planned).
- Deployment/verification endpoints over live RPC (blocked on the live
  transport, issue #13).
- Persistence of keys, users, or audit events (planned).
