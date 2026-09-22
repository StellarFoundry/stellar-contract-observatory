# Roadmap

This roadmap lists real, unimplemented work derived from the current
architecture. It is not a promise of dates. Concrete tasks are tracked as
GitHub issues.

## Near term

- Live HTTP RPC transport behind the `Transport` trait, with explicit opt-in.
- Deployed WASM retrieval (`LedgerKeyContractCode`) and interface comparison of
  local vs. deployed code.
- Rename detection in the diff engine.
- Interface diff output formats beyond text/JSON.

## Medium term

- GitHub Action and an `observatory.yml` configuration schema.
- Report generation (Markdown) for inspection, diff, and compatibility.
- Additional hash algorithms / selectable fingerprints.
- More audit rules, each with a documented rationale.

## Longer term

- Reproducibility workflow (rebuild and compare), distinct from metadata
  comparison.
- Multi-provider RPC support and caching.
- Fuzzing targets for the WASM, spec, event, and diff parsers.
- crates.io packaging and release automation.

## Non-goals

- Executing contract code.
- Acting as a security auditor.
- Replacing the Stellar CLI, SDKs, or binding generators.
