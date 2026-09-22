# Issue backlog

This document summarizes the issue backlog by phase. It is a map, not a
contract. Every issue is real engineering work derived from the current
architecture; there are no placeholder or reward-driven issues.

As of the initial implementation the backlog contains **40 open issues**.

## How to read an issue

Every issue states Context, Problem, Objective, Scope, Acceptance criteria, and
Testing. Dependencies are stated explicitly (`Depends on #X`).

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
        └─────► #36 event-rpc
```

No circular dependencies exist.

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
