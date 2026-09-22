//! Typed request models for the REST API.
//!
//! All artifact input is base64-encoded WASM. Requests are validated before any
//! analysis runs, and the decoded size is bounded.

use observatory_core::limits::MAX_WASM_BYTES;
use observatory_core::{ObservatoryError, Result};
use serde::Deserialize;

use base64::Engine as _;

/// Request body for inspection.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectRequest {
    /// Base64-encoded WASM artifact.
    pub wasm_base64: String,
    /// Whether to include security heuristics.
    #[serde(default)]
    pub security: bool,
}

/// Request body for a single-artifact operation.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WasmRequest {
    /// Base64-encoded WASM artifact.
    pub wasm_base64: String,
}

/// Request body for a two-artifact operation.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiffRequest {
    /// Base64-encoded baseline artifact.
    pub old_wasm_base64: String,
    /// Base64-encoded updated artifact.
    pub new_wasm_base64: String,
}

/// Request body for a compatibility assessment.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompatibilityRequest {
    /// Base64-encoded baseline artifact.
    pub old_wasm_base64: String,
    /// Base64-encoded updated artifact.
    pub new_wasm_base64: String,
    /// Policy name: `strict` (default) or `lenient`.
    #[serde(default = "default_policy")]
    pub policy: String,
}

fn default_policy() -> String {
    "strict".to_string()
}

/// Decode a base64 WASM field with a hard size bound.
pub fn decode_wasm(field: &str) -> Result<Vec<u8>> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(field.trim())
        .map_err(|error| ObservatoryError::invalid(format!("invalid base64 artifact: {error}")))?;
    if bytes.len() > MAX_WASM_BYTES {
        return Err(ObservatoryError::invalid(format!(
            "artifact is {} bytes, exceeding the {} byte limit",
            bytes.len(),
            MAX_WASM_BYTES
        )));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn decodes_valid_base64() {
        let encoded = base64::engine::general_purpose::STANDARD.encode(b"hello");
        assert_eq!(decode_wasm(&encoded).unwrap(), b"hello");
    }

    #[test]
    fn rejects_invalid_base64() {
        assert!(decode_wasm("not base64!!!").is_err());
    }

    #[test]
    fn parses_requests_and_rejects_unknown_fields() {
        let request: InspectRequest = serde_json::from_value(json!({
            "wasm_base64": "AA==", "security": true
        }))
        .unwrap();
        assert!(request.security);
        assert!(
            serde_json::from_value::<InspectRequest>(json!({ "wasm_base64": "AA==", "x": 1 }))
                .is_err()
        );
        let compatibility: CompatibilityRequest = serde_json::from_value(json!({
            "old_wasm_base64": "AA==", "new_wasm_base64": "AA=="
        }))
        .unwrap();
        assert_eq!(compatibility.policy, "strict");
    }
}
