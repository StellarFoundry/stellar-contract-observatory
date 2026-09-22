//! Resource limits applied to untrusted input.
//!
//! Contract artifacts and specification sections are untrusted. Every parser in
//! the workspace receives bounded input and returns a structured error instead
//! of allocating without limit.

/// Maximum size of a WASM artifact read from disk, in bytes (64 MiB).
pub const MAX_WASM_BYTES: usize = 64 * 1024 * 1024;

/// Maximum size of a `contractspecv0` custom section, in bytes (16 MiB).
pub const MAX_SPEC_SECTION_BYTES: usize = 16 * 1024 * 1024;

/// Maximum number of specification entries parsed from a single section.
pub const MAX_SPEC_ENTRIES: usize = 100_000;

/// Maximum number of events loaded from a single fixture.
pub const MAX_EVENTS: usize = 100_000;

/// Maximum size of an event fixture file, in bytes (64 MiB).
pub const MAX_EVENT_FILE_BYTES: usize = 64 * 1024 * 1024;

/// Maximum number of custom sections retained in an inspection report.
pub const MAX_CUSTOM_SECTIONS: usize = 4096;

/// Maximum number of imports retained in an inspection report.
pub const MAX_IMPORTS: usize = 100_000;

/// Maximum number of exports retained in an inspection report.
pub const MAX_EXPORTS: usize = 100_000;
