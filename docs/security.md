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

## What is out of scope

- Detecting malicious contract logic.
- Validating that a deployed contract is safe.
- Trusting or authenticating RPC endpoints (the live transport is not shipped).

## Reporting

See [SECURITY.md](../SECURITY.md).
