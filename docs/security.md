# Security

## Model

Stellar Contract Observatory treats every artifact, specification, event, and RPC
response as untrusted input. The core guarantees:

- **No execution.** WASM is never run. Inspection is static and streaming.
- **Bounded parsing.** Input sizes and entry counts are limited; failures return
  structured errors, never panics on malformed input.
- **No network by default.** Unit tests and core workflows are offline.
- **Structured errors.** Errors carry stable machine-readable codes.

## Heuristics

`inspect --security` runs bounded, deterministic checks from `observatory-audit`:

- specification validation (duplicates, unknown references, arity),
- empty/duplicate names,
- events with more than two prefix topics,
- artifact checks (no code section, no `contractspecv0`, truncated listings,
  oversized specification sections).

Rules marked `experimental` may change. Heuristic ids are stable for CI.

> These checks are **not** a security audit and provide no guarantees. They catch
> structural inconsistencies, nothing more.

## SSRF policy

The RPC layer ships an address policy (`observatory_rpc::net`) that must be
applied before any live request. It classifies literal addresses and resolves
hostnames, then rejects any non-public result:

- loopback, private (RFC 1918), link-local, and cloud metadata
  (`169.254.169.254`);
- IPv6 unique-local and link-local;
- shared (`100.64.0.0/10`), benchmarking, documentation, unspecified, multicast,
  broadcast, and reserved ranges;
- IPv4-mapped IPv6 addresses are classified as IPv4, so `::ffff:127.0.0.1` is
  rejected.

`SsrfPolicy::strict()` blocks loopback; `SsrfPolicy::local()` permits it for
local development only. Endpoints can be checked with
`Endpoint::validate_ssrf(&policy)`. The live transport (issue #13) must call this
before connecting.

Because the current build performs no server-initiated requests, SSRF is
structurally absent today; this policy exists so the live transport can land
safely.

## What is out of scope

- Detecting malicious contract logic.
- Validating that a deployed contract is safe.
- Trusting or authenticating RPC endpoints (the live transport is not shipped).

## Reporting

See [SECURITY.md](../SECURITY.md).
