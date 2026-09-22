//! Stable report envelope and JSON rendering.
//!
//! Every JSON report is wrapped in an envelope carrying the tool name, version,
//! report kind, and schema version. This lets automation pin a schema and detect
//! changes. Human mode prints the same data without the envelope.

use observatory_core::{ObservatoryError, Result, SCHEMA_VERSION, TOOL_NAME};
use serde::{Deserialize, Serialize};

/// A machine-readable report envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report<T> {
    /// Schema version of the envelope.
    pub schema_version: String,
    /// The report kind, for example `inspect`.
    pub kind: String,
    /// Producing tool.
    pub tool: String,
    /// Producing tool version.
    pub version: String,
    /// The report payload.
    pub data: T,
}

/// Wrap a payload in a versioned envelope.
pub fn report<T>(kind: &str, data: T) -> Report<T> {
    Report {
        schema_version: SCHEMA_VERSION.to_string(),
        kind: kind.to_string(),
        tool: TOOL_NAME.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        data,
    }
}

impl<T: Serialize> Report<T> {
    /// Serialize to compact JSON.
    pub fn to_json(&self) -> Result<String> {
        to_json(self)
    }

    /// Serialize to pretty JSON.
    pub fn to_json_pretty(&self) -> Result<String> {
        to_json_pretty(self)
    }
}

/// A machine-readable error report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ErrorReport {
    /// Schema version.
    pub schema_version: String,
    /// Always `error`.
    pub kind: String,
    /// Stable error code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

impl ErrorReport {
    /// Build an error report from a typed error.
    #[must_use]
    pub fn from_error(error: &ObservatoryError) -> Self {
        ErrorReport {
            schema_version: SCHEMA_VERSION.to_string(),
            kind: "error".to_string(),
            code: error.code().as_str().to_string(),
            message: error.to_string(),
        }
    }

    /// Serialize to compact JSON.
    pub fn to_json(&self) -> Result<String> {
        to_json(self)
    }
}

/// Serialize any value to compact JSON.
pub fn to_json<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value)
        .map_err(|error| ObservatoryError::internal(format!("json serialization: {error}")))
}

/// Serialize any value to pretty JSON.
pub fn to_json_pretty<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_string_pretty(value)
        .map_err(|error| ObservatoryError::internal(format!("json serialization: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct Payload {
        answer: u32,
    }

    #[test]
    fn envelope_carries_schema_and_kind() {
        let report = report("inspect", Payload { answer: 42 });
        assert_eq!(report.schema_version, SCHEMA_VERSION);
        assert_eq!(report.kind, "inspect");
        let json = report.to_json().unwrap();
        assert!(json.contains("\"schema_version\":\""));
        assert!(json.contains("\"answer\":42"));
    }

    #[test]
    fn error_report_uses_stable_code() {
        let error = ObservatoryError::wasm("bad");
        let report = ErrorReport::from_error(&error);
        assert_eq!(report.kind, "error");
        assert_eq!(report.code, "wasm");
        assert!(report.to_json().unwrap().contains("\"code\":\"wasm\""));
    }

    #[test]
    fn json_output_is_deterministic() {
        let a = report("x", Payload { answer: 1 }).to_json().unwrap();
        let b = report("x", Payload { answer: 1 }).to_json().unwrap();
        assert_eq!(a, b);
    }
}
