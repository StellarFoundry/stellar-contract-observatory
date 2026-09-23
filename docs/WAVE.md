# Contributing through a Drips Wave

This project is prepared for participation in an open-source Wave. This page
explains how issues are scoped, how to claim one, and what to expect. It does
**not** state point values, budgets, or reward projections; Drips determines
applicable points and budgets through its own system.

## Complexity

Every issue carries one complexity label:

| Label | Typical work |
| ----- | ------------ |
| `complexity:trivial` | Small, clearly bounded change with obvious acceptance criteria. |
| `complexity:medium` | A standard feature or logic touching several parts of the codebase. |
| `complexity:high` | Complex engineering: integrations or architectural changes. |

Complexity is assigned from the scope of each individual issue, never from an
aggregate target. Issues are not split or inflated to change their size; if a
task is too large it is broken into genuinely independent units, and trivial work
is combined with related work rather than filed alone.

## What makes an issue here legitimate

The backlog is derived from the actual architecture, not generated to a count.
Each issue includes:

- Context, Problem, and Objective
- Scope and (where relevant) out-of-scope
- Acceptance criteria
- Testing expectations
- Relevant modules/files
- Implementation guidance (edge cases and constraints, without prescribing every
  line)
- An example commit message
- Guidelines: assignment required before starting, `Closes #N`, and the local gate

## Claiming an issue

1. Pick an issue. `complexity:trivial` and well-scoped `complexity:medium`
   issues are the easiest entry points.
2. Request assignment by commenting on the issue. Do not open a pull request for
   an issue you have not been assigned.
3. Wait for a maintainer to assign you, then begin.

## Working on an issue

1. Fork the repository and create a branch named for the change, for example
   `feat/rpc-live-transport`.
2. Follow the issue's Scope and Acceptance criteria; those define "done".
3. Write the tests the issue asks for.
4. Run the local gate:

   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets --all-features -- -D warnings
   cargo test --workspace
   ```

5. Open a pull request whose description includes `Closes #<issue-number>` and how
   you verified the change.

## Honesty expectations

- Do not claim a feature works unless it is implemented and tested.
- Do not fabricate benchmark results, CI results, or protocol behaviour.
- Keep parsing of untrusted input bounded; never execute untrusted WASM.
- If something cannot be completed, document it and file a precise follow-up.

## What maintainers provide

- A prompt initial response to new issues and pull requests during an active
  Wave.
- A review decision (approve, request changes, or close with an explanation).
- Clear, actionable feedback and a merge when the acceptance criteria are met.

## Security

Do not discuss a security vulnerability in a public issue or pull request.
Follow [`SECURITY.md`](../SECURITY.md).

## Definition of done

- The change matches the acceptance criteria.
- The requested tests are added and the local gate passes.
- Documentation and `CHANGELOG.md` are updated for user-visible changes.
- The pull request is reviewed and merged.
