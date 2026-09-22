//! Output format selection shared by the CLI and reporters.

use std::fmt;
use std::str::FromStr;

use crate::error::ObservatoryError;

/// How a report should be rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Human-readable text.
    #[default]
    Human,
    /// Stable, machine-readable JSON.
    Json,
}

impl FromStr for OutputFormat {
    type Err = ObservatoryError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "human" | "text" => Ok(OutputFormat::Human),
            "json" => Ok(OutputFormat::Json),
            other => Err(ObservatoryError::invalid(format!(
                "unknown output format `{other}`; expected human or json"
            ))),
        }
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            OutputFormat::Human => "human",
            OutputFormat::Json => "json",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_formats() {
        assert_eq!("json".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert_eq!("TEXT".parse::<OutputFormat>().unwrap(), OutputFormat::Human);
        assert!("xml".parse::<OutputFormat>().is_err());
    }
}
