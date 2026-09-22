//! Shared foundation for the Stellar Contract Observatory.
//!
//! This crate deliberately has no Stellar dependency. It defines the error
//! taxonomy, resource limits, hashing helpers, network profiles, and output
//! format used by the rest of the workspace.

pub mod error;
pub mod hash;
pub mod limits;
pub mod network;
pub mod output;

pub use error::{ErrorCode, ExitCode, ObservatoryError, Result};
pub use network::Network;
pub use output::OutputFormat;

/// The binary/tool name used in user-facing output.
pub const TOOL_NAME: &str = "stellar-contract-observatory";

/// The machine-readable output schema version emitted with every JSON report.
pub const SCHEMA_VERSION: &str = "1.0";
