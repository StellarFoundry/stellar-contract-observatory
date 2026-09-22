//! Platform configuration with strict validation.

use observatory_core::{ObservatoryError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ratelimit::RateLimitConfig;

/// Configuration for the platform/API layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformConfig {
    /// Whether analysis endpoints require authentication.
    #[serde(default = "default_true")]
    pub require_auth: bool,
    /// Rate limiting configuration.
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
}

fn default_true() -> bool {
    true
}

impl Default for PlatformConfig {
    fn default() -> Self {
        PlatformConfig {
            require_auth: true,
            rate_limit: RateLimitConfig::default(),
        }
    }
}

impl PlatformConfig {
    /// Parse configuration from a JSON value, rejecting unknown fields.
    pub fn from_json(value: &Value) -> Result<Self> {
        serde_json::from_value(value.clone()).map_err(|error| {
            ObservatoryError::invalid(format!("invalid platform configuration: {error}"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_are_sane() {
        let config = PlatformConfig::default();
        assert!(config.require_auth);
        assert_eq!(config.rate_limit.limit, 120);
    }

    #[test]
    fn parses_valid_configuration() {
        let config = PlatformConfig::from_json(&json!({
            "require_auth": false,
            "rate_limit": { "limit": 10, "window_secs": 30 }
        }))
        .unwrap();
        assert!(!config.require_auth);
        assert_eq!(config.rate_limit.limit, 10);
    }

    #[test]
    fn rejects_unknown_fields() {
        let error = PlatformConfig::from_json(&json!({ "unknown": true })).unwrap_err();
        assert!(matches!(error, ObservatoryError::InvalidInput(_)));
    }
}
