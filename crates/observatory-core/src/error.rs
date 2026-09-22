//! Error taxonomy and process exit codes.
//!
//! Every fallible operation in the workspace returns [`ObservatoryError`]. Each
//! variant carries a stable machine-readable [`ErrorCode`] and maps to a stable
//! process [`ExitCode`] so scripts and CI can branch on failure kind.

use std::fmt;

/// Stable, machine-readable error codes emitted in JSON error reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// Invalid user input or arguments.
    InvalidInput,
    /// Filesystem or I/O failure.
    Io,
    /// A decode/parse failure on otherwise valid input.
    Decode,
    /// A well-formed input uses a feature the tool does not support.
    Unsupported,
    /// A malformed or invalid WASM module.
    Wasm,
    /// A malformed or invalid contract specification.
    Spec,
    /// An RPC transport or protocol failure.
    Rpc,
    /// Verification failed or was inconclusive.
    Verification,
    /// An internal invariant was violated.
    Internal,
}

impl ErrorCode {
    /// The stable string form used in JSON output.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::InvalidInput => "invalid_input",
            ErrorCode::Io => "io",
            ErrorCode::Decode => "decode",
            ErrorCode::Unsupported => "unsupported",
            ErrorCode::Wasm => "wasm",
            ErrorCode::Spec => "spec",
            ErrorCode::Rpc => "rpc",
            ErrorCode::Verification => "verification",
            ErrorCode::Internal => "internal",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stable process exit codes.
///
/// These are part of the public contract with CI users and must not change
/// without a documented deprecation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExitCode {
    /// Success.
    Success = 0,
    /// An unexpected internal failure.
    Internal = 1,
    /// Usage error (clap also uses this).
    Usage = 2,
    /// Input or filesystem error.
    Input = 3,
    /// A parse/decode error.
    Parse = 4,
    /// A requested feature is unsupported.
    Unsupported = 5,
    /// An interface change was classified as incompatible.
    Incompatible = 6,
    /// Verification found a mismatch.
    Mismatch = 7,
    /// Verification could not be completed with available data.
    InsufficientData = 8,
}

impl ExitCode {
    /// The numeric code.
    #[must_use]
    pub fn code(self) -> u8 {
        self as u8
    }
}

/// The workspace-wide error type.
#[derive(Debug, thiserror::Error)]
pub enum ObservatoryError {
    /// Invalid user input.
    #[error("invalid input: {0}")]
    InvalidInput(String),
    /// Filesystem or I/O failure.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// Decode/parse failure.
    #[error("decode error: {0}")]
    Decode(String),
    /// A well-formed input using an unsupported feature.
    #[error("unsupported: {0}")]
    Unsupported(String),
    /// Malformed or invalid WASM.
    #[error("wasm error: {0}")]
    Wasm(String),
    /// Malformed or invalid contract spec.
    #[error("spec error: {0}")]
    Spec(String),
    /// RPC failure.
    #[error("rpc error: {0}")]
    Rpc(String),
    /// Failed or inconclusive verification.
    #[error("verification error: {0}")]
    Verification(String),
    /// Internal invariant violation.
    #[error("internal error: {0}")]
    Internal(String),
}

impl ObservatoryError {
    /// Construct an invalid-input error.
    pub fn invalid(message: impl Into<String>) -> Self {
        ObservatoryError::InvalidInput(message.into())
    }

    /// Construct a decode error.
    pub fn decode(message: impl Into<String>) -> Self {
        ObservatoryError::Decode(message.into())
    }

    /// Construct an unsupported-feature error.
    pub fn unsupported(message: impl Into<String>) -> Self {
        ObservatoryError::Unsupported(message.into())
    }

    /// Construct a WASM error.
    pub fn wasm(message: impl Into<String>) -> Self {
        ObservatoryError::Wasm(message.into())
    }

    /// Construct a spec error.
    pub fn spec(message: impl Into<String>) -> Self {
        ObservatoryError::Spec(message.into())
    }

    /// Construct an RPC error.
    pub fn rpc(message: impl Into<String>) -> Self {
        ObservatoryError::Rpc(message.into())
    }

    /// Construct a verification error.
    pub fn verification(message: impl Into<String>) -> Self {
        ObservatoryError::Verification(message.into())
    }

    /// Construct an internal error.
    pub fn internal(message: impl Into<String>) -> Self {
        ObservatoryError::Internal(message.into())
    }

    /// The stable machine-readable error code.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            ObservatoryError::InvalidInput(_) => ErrorCode::InvalidInput,
            ObservatoryError::Io(_) => ErrorCode::Io,
            ObservatoryError::Decode(_) => ErrorCode::Decode,
            ObservatoryError::Unsupported(_) => ErrorCode::Unsupported,
            ObservatoryError::Wasm(_) => ErrorCode::Wasm,
            ObservatoryError::Spec(_) => ErrorCode::Spec,
            ObservatoryError::Rpc(_) => ErrorCode::Rpc,
            ObservatoryError::Verification(_) => ErrorCode::Verification,
            ObservatoryError::Internal(_) => ErrorCode::Internal,
        }
    }

    /// The process exit code that best represents this failure.
    #[must_use]
    pub fn exit_code(&self) -> ExitCode {
        match self {
            ObservatoryError::InvalidInput(_) => ExitCode::Usage,
            ObservatoryError::Io(_) => ExitCode::Input,
            ObservatoryError::Decode(_) => ExitCode::Parse,
            ObservatoryError::Unsupported(_) => ExitCode::Unsupported,
            ObservatoryError::Wasm(_) | ObservatoryError::Spec(_) => ExitCode::Parse,
            ObservatoryError::Rpc(_) => ExitCode::Input,
            ObservatoryError::Verification(_) => ExitCode::InsufficientData,
            ObservatoryError::Internal(_) => ExitCode::Internal,
        }
    }
}

/// Convenience alias used throughout the workspace.
pub type Result<T> = std::result::Result<T, ObservatoryError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_are_stable_strings() {
        assert_eq!(ObservatoryError::wasm("x").code().as_str(), "wasm");
        assert_eq!(
            ObservatoryError::invalid("x").code(),
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn exit_codes_map_sensibly() {
        assert_eq!(ObservatoryError::invalid("x").exit_code(), ExitCode::Usage);
        assert_eq!(
            ObservatoryError::verification("x").exit_code(),
            ExitCode::InsufficientData
        );
    }

    #[test]
    fn io_errors_convert() {
        let err: ObservatoryError =
            std::io::Error::new(std::io::ErrorKind::NotFound, "missing").into();
        assert_eq!(err.code(), ErrorCode::Io);
    }
}
