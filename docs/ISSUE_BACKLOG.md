# Issue backlog

This document summarizes the issue backlog by phase. It is a map, not a
contract. Every issue is real engineering work derived from the current
architecture; there are no placeholder or filler issues.

As of the expanded backlog the repository contains **130 open issues**.

## How to read an issue

Every issue states Context, Problem, Objective, Scope, Acceptance criteria, and
Testing. Dependencies are stated explicitly (`Depends on #X`).

Following the Drips guide *Creating Meaningful Issues*, every issue also includes
concrete **Relevant modules/files**, **Implementation guidance** (edge cases and
constraints without micromanaging), an **Example commit message**, and
**Guidelines** (assignment required, `Closes #`, and the local gate). Complexity
is tagged with `complexity:trivial|medium|high`; individual issues never state a
point or reward total.

## Themes by phase

| Phase | Focus | Example issues |
| ----- | ----- | -------------- |
| foundation | workspace health, MSRV, fixtures | #1 |
| wasm | deeper section coverage, fuzzing | #4, #5 |
| spec | forward compatibility, fuzzing, correctness | #6, #7, #33 |
| interface | canonicalization, schema | #3, #34 |
| events | raw XDR input, diagnostic events, RPC events | #10, #11, #36 |
| diff | rename detection, rendering, fixtures | #8, #9, #32 |
| compatibility | configurable policy | #12 |
| deployment | live transport, deployed code, caching | #13, #14, #16, #37 |
| verification | deployed interface comparison | #15 |
| reproducibility | metadata modelling and reporting | #18, #19, #20 |
| security | more heuristics, false-positive corpus | #21, #22, #23 |
| output | JSON Schemas, golden files | #2, #24 |
| cli | completions, formatting, exit-code coverage | #25, #26, #38 |
| ci | MSRV, Action, config, baseline check | #1, #27, #28, #39 |
| testing | fuzzing, property tests, mock server | #5, #7, #17, #22 |
| performance | criterion benchmarks | #30 |
| ecosystem | real-artifact interoperability | #31 |
| release | packaging and dry-run | #29 |
| documentation | troubleshooting, vocabulary, heuristics | #20, #23, #37, #40 |

## Dependency graph

```
#13 live-rpc ──► #14 deployed-wasm ──► #15 verify-interface
        ├─────► #36 event-rpc
        └─────► #58 ssrf (must land with #13)
#41 api-key-endpoints ──► #54 key-rotation
#47 webhook-registration ──► #57 webhook-delivery
#13 ──► #52 api-verify-endpoint
#36 ──► #53 api-events-endpoint
```

No circular dependencies exist.

## Platform and API themes (issues #41–#60)

After the developer-platform layer landed, the backlog was extended from the
actual new surface:

| Theme | Example issues |
| ----- | -------------- |
| API key management and persistence | #41, #42, #54 |
| Authorization and audit | #43 |
| Rate limits and CORS | #44, #56 |
| OpenAPI and schemas | #45 |
| Observability (metrics, counters) | #46, #55 |
| Webhooks | #47, #57 |
| Background jobs | #48 |
| SDKs (Python, TypeScript, Rust) | #49, #50, #51 |
| API + live RPC integration | #52, #53, #58 |
| HTTP hardening and load | #59, #60 |

## Depth and platform-hardening themes (issues #61–#130)

A second expansion added 70 distinct issues derived from the real surface, with
no duplicates of the first 60:

| Theme | Example issues |
| ----- | -------------- |
| WASM depth (name/producers sections, memory64, streaming hash, component rejection) | #61–#66 |
| Specification depth (docs rendering, length limits, UDT docs) | #67–#69 |
| Interface depth (round-trip, schema policy, subset check) | #70–#72 |
| Diff/compat depth (type/event renames, filters, policies) | #73–#81 |
| Fingerprinting options (algorithm, events-only, doc-insensitive) | #82–#84 |
| Events depth (structured filters, ledger ranges, CSV, raw XDR, schema) | #85–#88, #112 |
| Audit/security model (confidence/evidence, suppressions, SARIF, rule config) | #89, #90, #113–#116 |
| RPC depth (retry, timeouts, limits, error taxonomy, getContractData, health) | #117–#122 |
| Deployment/verify depth (networks, storage, cross-network, batch, exit policy) | #91–#95 |
| Reproducibility (dependency metadata, SBOM) | #96, #97 |
| API hardening (pagination, idempotency, negotiation, error catalog, scopes) | #98, #99, #123–#127 |
| Observability (tracing, redaction, Prometheus) | #100, #101, #128 |
| Configuration (profiles, precedence) | #102, #129 |
| CI/release (OpenAPI validation, dependency audit, fuzz smoke, coverage, binaries, SBOM, container) | #103–#109 |
| Documentation (ADRs, glossary) | #110, #130 |

The count is a consequence of the engineering surface, not a target: each issue
was authored independently and audited for duplicates before creation.

## Scope note

This backlog intentionally lists only issues that were individually authored and
reviewed against the architecture. It is smaller than an aspirational maximum on
purpose: filling to a target number would create duplicates and busywork, which
the project's contribution rules forbid. Additional issues should be filed as
the implementation evolves, not pre-generated.

## Issue-quality audit

The backlog is audited against the Drips *Creating Meaningful Issues* guidance:

- Every issue carries exactly one `complexity:trivial|medium|high` label.
- Every issue contains Context, Problem, Objective, Scope, Acceptance criteria,
  Testing, Relevant modules/files, Implementation guidance, an example commit
  message, and Guidelines.
- No duplicate titles (checked programmatically).
- No reward, point, or budget figures anywhere.
- Dependencies are explicit and non-circular.
- Complexity is assigned from each issue's own scope, never from an aggregate
  target.

## Creating new issues

Use the feature or bug template. Include a concrete objective, scope, and
testable acceptance criteria. Add `phase:*`, `type:*`, and where useful
`complexity:*` labels, and state dependencies explicitly.
