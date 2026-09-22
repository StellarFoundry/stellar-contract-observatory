# Security Policy

## Reporting a vulnerability

Do not open a public issue for a security vulnerability. Report it privately to
the maintainers (for example, via the repository's security advisories) with:

- a description of the issue,
- a minimal reproduction,
- the affected version or commit.

## Scope

Stellar Contract Observatory parses untrusted input: WASM artifacts,
`contractspecv0` sections, event fixtures, and RPC fixtures. The main classes of
concern are:

- unbounded parsing or allocation on malicious input,
- panics or aborts on malformed input,
- execution of untrusted contract code (this must never happen),
- path traversal or unsafe file handling,
- secrets or credentials leaking into output.

## Guarantees and non-guarantees

- The toolkit **never executes** WASM. Inspection is static and bounded.
- All untrusted parsers apply documented limits and return structured errors.
- The toolkit is **not** a security auditor. The heuristics in
  `observatory-audit` are bounded and deterministic, and are explicitly not a
  substitute for an audit.
- Deployment and verification currently run against deterministic RPC fixtures.
  A live HTTP transport is not shipped yet; do not point the tool at untrusted
  RPC endpoints in the expectation that it validates them.

## Supported versions

The project is pre-1.0. Security fixes are applied to the default branch.
