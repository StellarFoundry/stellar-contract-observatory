# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Workspace with 14 crates covering inspection, specification, interface model,
  events, diff, compatibility, RPC, deployment, verification, audit, output, and
  the CLI.
- `observatory-wasm`: bounded, non-executing WASM inspection (sections, imports,
  exports, memories, custom sections, code size) and SHA-256 artifact hashing.
- `observatory-spec`: bounded `contractspecv0` parsing into a normalized,
  parser-independent model (functions, structs, unions, enums, error enums,
  events) with validation.
- `observatory-interface`: canonical interface model with deterministic
  interface fingerprinting and a known-value test vector.
- `observatory-events`: event fixture decoding, normalization, and filtering.
- `observatory-diff`: deterministic interface diff with breaking, non-breaking,
  and informational classification.
- `observatory-compat`: documented, configurable compatibility policy returning
  compatible / incompatible / unknown with reasons.
- `observatory-rpc`: transport abstraction, endpoint validation, typed
  `getLatestLedger` / `getNetwork` / `getLedgerEntries` models, and a
  deterministic mock/fixture transport.
- `observatory-deployment`: deployed contract inspection and `ContractExecutable`
  WASM hash recovery from real ledger-entry XDR.
- `observatory-verify`: local-vs-deployed verification and `contractmetav0` /
  `contractenvmetav0` build metadata parsing and comparison.
- `observatory-audit`: bounded, deterministic interface/artifact heuristics with
  clearly labelled experimental rules.
- `observatory-output`: versioned JSON report envelopes and stable error reports.
- `observatory-cli`: `inspect`, `spec`, `events`, `diff`, `compat`,
  `fingerprint`, `deployment`, `verify`, and `doctor`, all with `--json`.
- CI: format, clippy, and tests on Linux, Windows, and macOS.

### Notes

- No live HTTP RPC transport is shipped yet. Deployment and verification run
  against deterministic RPC fixtures.
- The audit crate is not a security auditor.
