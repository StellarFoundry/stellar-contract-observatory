//! Project configuration (`observatory.yml`, `.yaml`, `.toml`, or `.json`).
//!
//! The configuration is validated strictly: unknown fields are rejected so a
//! typo cannot silently disable a setting. The schema is intentionally small and
//! only exposes settings that the implementation actually honours.

use std::path::{Path, PathBuf};

use observatory_core::{ObservatoryError, Result};
use serde::{Deserialize, Serialize};

use crate::ratelimit::RateLimitConfig;
use crate::rbac::Role;

/// The filename stems discovered automatically, in priority order.
pub const DISCOVERY_NAMES: &[&str] = &[
    "observatory.yml",
    "observatory.yaml",
    "observatory.toml",
    "observatory.json",
];

/// A complete project configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    /// Project identity.
    #[serde(default)]
    pub project: ProjectSection,
    /// Analysis defaults.
    #[serde(default)]
    pub analysis: AnalysisSection,
    /// RPC defaults.
    #[serde(default)]
    pub rpc: RpcSection,
    /// API/platform defaults.
    #[serde(default)]
    pub api: ApiSection,
    /// CI defaults.
    #[serde(default)]
    pub ci: CiSection,
}

/// Project identity.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectSection {
    /// Project name.
    #[serde(default)]
    pub name: Option<String>,
    /// Default network label.
    #[serde(default)]
    pub network: Option<String>,
}

/// Analysis defaults.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisSection {
    /// Compatibility analysis defaults.
    #[serde(default)]
    pub compatibility: CompatibilitySection,
}

/// Compatibility defaults.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompatibilitySection {
    /// Policy name: `strict` or `lenient`.
    #[serde(default = "default_policy")]
    pub policy: String,
}

fn default_policy() -> String {
    "strict".to_string()
}

impl Default for CompatibilitySection {
    fn default() -> Self {
        CompatibilitySection {
            policy: default_policy(),
        }
    }
}

/// RPC defaults.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RpcSection {
    /// Default endpoint URL.
    #[serde(default)]
    pub endpoint: Option<String>,
}

/// API/platform defaults.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiSection {
    /// Whether analysis endpoints require authentication.
    #[serde(default = "default_true")]
    pub require_auth: bool,
    /// Optional rate-limit override.
    #[serde(default)]
    pub rate_limit: Option<RateLimitSection>,
}

fn default_true() -> bool {
    true
}

impl Default for ApiSection {
    fn default() -> Self {
        ApiSection {
            require_auth: true,
            rate_limit: None,
        }
    }
}

/// Rate-limit configuration for the project file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RateLimitSection {
    /// Requests per window.
    pub limit: u32,
    /// Window length in seconds.
    pub window_secs: u64,
}

/// CI defaults.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CiSection {
    /// Comma-separated verification statuses that should fail the build.
    #[serde(default)]
    pub fail_on: Option<String>,
}

/// A supported configuration file format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    /// YAML.
    Yaml,
    /// TOML.
    Toml,
    /// JSON.
    Json,
}

impl ConfigFormat {
    /// Infer the format from a file extension.
    pub fn from_path(path: &Path) -> Result<Self> {
        match path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("yml") | Some("yaml") => Ok(ConfigFormat::Yaml),
            Some("toml") => Ok(ConfigFormat::Toml),
            Some("json") => Ok(ConfigFormat::Json),
            other => Err(ObservatoryError::invalid(format!(
                "unsupported configuration extension `{}`; expected yml, yaml, toml, or json",
                other.unwrap_or("<none>")
            ))),
        }
    }
}

impl ProjectConfig {
    /// Parse configuration text in the given format.
    pub fn parse(input: &str, format: ConfigFormat) -> Result<Self> {
        let parsed = match format {
            ConfigFormat::Yaml => serde_yml::from_str(input).map_err(|error| error.to_string()),
            ConfigFormat::Toml => toml::from_str(input).map_err(|error| error.to_string()),
            ConfigFormat::Json => serde_json::from_str(input).map_err(|error| error.to_string()),
        };
        parsed.map_err(|error| ObservatoryError::invalid(format!("invalid configuration: {error}")))
    }

    /// Load configuration from a path, inferring the format.
    pub fn from_path(path: &Path) -> Result<Self> {
        let format = ConfigFormat::from_path(path)?;
        let text = std::fs::read_to_string(path).map_err(|error| {
            ObservatoryError::invalid(format!("cannot read `{}`: {error}", path.display()))
        })?;
        Self::parse(&text, format)
    }

    /// Find and load a configuration file in `directory`, if one exists.
    pub fn discover(directory: &Path) -> Result<Option<(PathBuf, Self)>> {
        for name in DISCOVERY_NAMES {
            let candidate = directory.join(name);
            if candidate.is_file() {
                let config = Self::from_path(&candidate)?;
                return Ok(Some((candidate, config)));
            }
        }
        Ok(None)
    }

    /// Resolve the compatibility policy, validating the name.
    pub fn compatibility_policy(&self) -> Result<crate::CompatibilityPolicy> {
        match self.analysis.compatibility.policy.as_str() {
            "strict" => Ok(crate::CompatibilityPolicy::strict()),
            "lenient" => Ok(crate::CompatibilityPolicy::lenient()),
            other => Err(ObservatoryError::invalid(format!(
                "unknown compatibility policy `{other}`; expected strict or lenient"
            ))),
        }
    }

    /// Resolve the rate-limit configuration.
    #[must_use]
    pub fn rate_limit(&self) -> RateLimitConfig {
        self.api
            .rate_limit
            .map(|section| RateLimitConfig {
                limit: section.limit,
                window_secs: section.window_secs,
            })
            .unwrap_or_default()
    }

    /// Resolve the default network label, if configured.
    pub fn network(&self) -> Option<&str> {
        self.project.network.as_deref()
    }
}

/// Parse a role name (re-exported convenience for configuration-driven setups).
pub fn parse_role(value: &str) -> Result<Role> {
    value
        .parse()
        .map_err(|error: String| ObservatoryError::invalid(error))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_yaml_json_and_toml() {
        let yaml = "project:\n  name: demo\n  network: testnet\nanalysis:\n  compatibility:\n    policy: lenient\n";
        let config = ProjectConfig::parse(yaml, ConfigFormat::Yaml).unwrap();
        assert_eq!(config.project.name.as_deref(), Some("demo"));
        assert_eq!(config.network(), Some("testnet"));
        assert!(
            !config
                .compatibility_policy()
                .unwrap()
                .function_removal_is_breaking
        );

        let json = r#"{ "project": { "network": "public" } }"#;
        let config = ProjectConfig::parse(json, ConfigFormat::Json).unwrap();
        assert_eq!(config.network(), Some("public"));

        let toml = "[api]\nrequire_auth = false\n[api.rate_limit]\nlimit = 5\nwindow_secs = 30\n";
        let config = ProjectConfig::parse(toml, ConfigFormat::Toml).unwrap();
        assert!(!config.api.require_auth);
        assert_eq!(config.rate_limit().limit, 5);
        assert_eq!(config.rate_limit().window_secs, 30);
    }

    #[test]
    fn rejects_unknown_fields() {
        let error = ProjectConfig::parse("project:\n  nope: 1\n", ConfigFormat::Yaml).unwrap_err();
        assert!(matches!(error, ObservatoryError::InvalidInput(_)));
    }

    #[test]
    fn rejects_unknown_policy() {
        let config = ProjectConfig::parse(
            "analysis:\n  compatibility:\n    policy: wild\n",
            ConfigFormat::Yaml,
        )
        .unwrap();
        assert!(config.compatibility_policy().is_err());
    }

    #[test]
    fn infers_format_from_extension() {
        assert_eq!(
            ConfigFormat::from_path(Path::new("observatory.yml")).unwrap(),
            ConfigFormat::Yaml
        );
        assert_eq!(
            ConfigFormat::from_path(Path::new("observatory.toml")).unwrap(),
            ConfigFormat::Toml
        );
        assert!(ConfigFormat::from_path(Path::new("observatory.txt")).is_err());
    }
}
