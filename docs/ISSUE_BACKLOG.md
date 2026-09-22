# Issue backlog

This document summarizes the issue backlog by phase. It is a map, not a
contract. Every issue is real engineering work derived from the current
architecture; there are no placeholder or filler issues.

As of the platform/API increment the backlog contains **60 open issues**.

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

## Scope note

This backlog intentionally lists only issues that were individually authored and
reviewed against the architecture. It is smaller than an aspirational maximum on
purpose: filling to a target number would create duplicates and busywork, which
the project's contribution rules forbid. Additional issues should be filed as
the implementation evolves, not pre-generated.

## Creating new issues

Use the feature or bug template. Include a concrete objective, scope, and
testable acceptance criteria. Add `phase:*`, `type:*`, and where useful
`complexity:*` labels, and state dependencies explicitly.
