# Maintenance

How the project is maintained.

## Issue triage

- Issues must describe a concrete engineering objective with scope and
  acceptance criteria. Issues without them are rewritten or closed.
- Each issue carries a `phase:*` label and a `type:*` label; complexity uses
  `complexity:trivial|medium|high` where useful.
- Duplicates are closed and merged into a canonical issue.
- Dependencies are stated explicitly (`Depends on #X`). Circular dependencies
  are not allowed.

## Pull requests

- One logical change per pull request. Reference the issue with `Closes #N`.
- The PR template requires the local gate to pass and documentation to be
  updated.
- Reviewers check: bounded parsing for new untrusted input, deterministic
  output, tests for malformed input, and no fabricated claims.

## Bug handling

1. Reproduce with a minimal fixture.
2. Add a regression test that fails before the fix.
3. Fix and confirm the test passes.
4. Record user-visible fixes in `CHANGELOG.md`.

## Compatibility changes

Compatibility semantics live in `observatory-compat` and are documented in
[docs/compatibility.md](compatibility.md). Changing a rule is a public behaviour
change: it requires a documentation update, a changelog entry, and tests for the
old and new classification. New rules are added to the policy rather than hidden.

## Releases

- Semantic versioning. Pre-1.0, breaking changes may occur in minor releases.
- `CHANGELOG.md` follows Keep a Changelog.
- Do not claim a package is published unless it is.
- MSRV is the workspace `rust-version`; raising it is a documented change.

## Security reports

Follow [SECURITY.md](../SECURITY.md). Do not discuss vulnerabilities in public
issues. Fixes are applied to the default branch.

## Documentation

Every user-visible capability has a page under `docs/`. Documentation must
describe real behaviour and must state limitations explicitly.
