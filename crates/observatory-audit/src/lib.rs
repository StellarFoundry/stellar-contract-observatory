//! Bounded, deterministic contract interface and artifact heuristics.
//!
//! These checks are intentionally narrow and fully deterministic. They are
//! **not** a security audit and must not be presented as one. Heuristics marked
//! `experimental` may change or be removed.
//!
//! Each check emits a finding with a stable id so CI can suppress or track
//! individual rules.

use observatory_interface::ContractInterface;
use observatory_spec::{SpecSeverity, SpecType};
use observatory_wasm::WasmReport;
use serde::Serialize;

/// Overall audit status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AuditStatus {
    /// No findings.
    Pass,
    /// Only warnings.
    Warn,
    /// At least one error.
    Error,
}

/// Severity of a single finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FindingSeverity {
    /// A non-fatal issue.
    Warn,
    /// A definite inconsistency.
    Error,
}

/// A single audit finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    /// Stable rule id, for example `interface.empty-name`.
    pub id: String,
    /// Severity.
    pub severity: FindingSeverity,
    /// Human-readable explanation.
    pub message: String,
    /// Optional stable path.
    pub path: Option<String>,
    /// Whether the rule is experimental.
    pub experimental: bool,
}

/// The result of running heuristics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuditReport {
    /// Overall status.
    pub status: AuditStatus,
    /// Findings, deterministically ordered by id then path.
    pub findings: Vec<Finding>,
}

impl AuditReport {
    fn from_findings(mut findings: Vec<Finding>) -> Self {
        findings.sort_by(|a, b| a.id.cmp(&b.id).then_with(|| a.path.cmp(&b.path)));
        let status = if findings
            .iter()
            .any(|finding| finding.severity == FindingSeverity::Error)
        {
            AuditStatus::Error
        } else if findings.is_empty() {
            AuditStatus::Pass
        } else {
            AuditStatus::Warn
        };
        AuditReport { status, findings }
    }

    /// Merge two reports into one.
    #[must_use]
    pub fn merge(self, other: AuditReport) -> AuditReport {
        let mut findings = self.findings;
        findings.extend(other.findings);
        AuditReport::from_findings(findings)
    }

    /// Count findings of a severity.
    #[must_use]
    pub fn count(&self, severity: FindingSeverity) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.severity == severity)
            .count()
    }
}

/// Run interface heuristics.
#[must_use]
pub fn audit_interface(interface: &ContractInterface) -> AuditReport {
    let mut findings = Vec::new();

    for issue in observatory_spec::validate(&interface.to_spec()) {
        findings.push(Finding {
            id: format!(
                "spec.{}",
                match issue.severity {
                    SpecSeverity::Error => "error",
                    SpecSeverity::Warning => "warning",
                    SpecSeverity::Info => "info",
                }
            ),
            severity: match issue.severity {
                SpecSeverity::Error => FindingSeverity::Error,
                SpecSeverity::Warning => FindingSeverity::Warn,
                SpecSeverity::Info => FindingSeverity::Warn,
            },
            message: issue.message,
            path: None,
            experimental: false,
        });
    }

    if interface.functions.is_empty() && !interface.is_empty() {
        findings.push(Finding {
            id: "interface.no-functions".to_string(),
            severity: FindingSeverity::Warn,
            message: "interface declares types or events but no functions".to_string(),
            path: None,
            experimental: true,
        });
    }

    if interface.functions.is_empty() && interface.is_empty() {
        findings.push(Finding {
            id: "interface.empty".to_string(),
            severity: FindingSeverity::Warn,
            message: "interface declares no public entities".to_string(),
            path: None,
            experimental: true,
        });
    }

    for function in &interface.functions {
        if function.name.trim().is_empty() {
            findings.push(Finding {
                id: "function.empty-name".to_string(),
                severity: FindingSeverity::Error,
                message: "a function has an empty name".to_string(),
                path: Some("function::".to_string()),
                experimental: false,
            });
        }
        let mut names: Vec<&str> = function
            .inputs
            .iter()
            .map(|input| input.name.as_str())
            .collect();
        names.sort_unstable();
        for pair in names.windows(2) {
            if !pair[0].is_empty() && pair[0] == pair[1] {
                findings.push(Finding {
                    id: "function.duplicate-param".to_string(),
                    severity: FindingSeverity::Warn,
                    message: format!(
                        "function `{}` has duplicate parameter name `{}`",
                        function.name, pair[0]
                    ),
                    path: Some(format!("function::{}", function.name)),
                    experimental: true,
                });
            }
        }
        for input in &function.inputs {
            if references_unknown(interface, &input.ty) {
                findings.push(Finding {
                    id: "function.unknown-type".to_string(),
                    severity: FindingSeverity::Error,
                    message: format!(
                        "function `{}` parameter `{}` references an unknown type",
                        function.name, input.name
                    ),
                    path: Some(format!("function::{}", function.name)),
                    experimental: false,
                });
            }
        }
    }

    for event in &interface.events {
        if event.prefix_topics.len() > 2 {
            findings.push(Finding {
                id: "event.too-many-prefix-topics".to_string(),
                severity: FindingSeverity::Error,
                message: format!(
                    "event `{}` declares {} prefix topics; the specification allows at most 2",
                    event.name,
                    event.prefix_topics.len()
                ),
                path: Some(format!("event::{}", event.name)),
                experimental: false,
            });
        }
    }

    AuditReport::from_findings(findings)
}

/// Run artifact heuristics.
#[must_use]
pub fn audit_artifact(report: &WasmReport) -> AuditReport {
    let mut findings = Vec::new();

    if report.code_section_bytes == 0 {
        findings.push(Finding {
            id: "artifact.no-code".to_string(),
            severity: FindingSeverity::Warn,
            message: "module contains no code section".to_string(),
            path: None,
            experimental: false,
        });
    }
    if !report.has_contract_spec {
        findings.push(Finding {
            id: "artifact.no-contract-spec".to_string(),
            severity: FindingSeverity::Warn,
            message: "module has no `contractspecv0` section; it may not be a Soroban contract"
                .to_string(),
            path: None,
            experimental: true,
        });
    }
    if report.truncated {
        findings.push(Finding {
            id: "artifact.truncated-report".to_string(),
            severity: FindingSeverity::Warn,
            message: "import, export, or custom-section listing was truncated by limits"
                .to_string(),
            path: None,
            experimental: false,
        });
    }
    if report.memories.is_empty() {
        findings.push(Finding {
            id: "artifact.no-memory".to_string(),
            severity: FindingSeverity::Warn,
            message: "module declares no linear memory".to_string(),
            path: None,
            experimental: true,
        });
    }
    if let Some(spec_bytes) = report.contract_spec_bytes {
        if spec_bytes > 4 * 1024 * 1024 {
            findings.push(Finding {
                id: "artifact.oversized-spec".to_string(),
                severity: FindingSeverity::Warn,
                message: format!(
                    "contract specification section is unusually large ({spec_bytes} bytes)"
                ),
                path: None,
                experimental: true,
            });
        }
    }

    AuditReport::from_findings(findings)
}

fn references_unknown(interface: &ContractInterface, ty: &SpecType) -> bool {
    match ty {
        SpecType::Udt { name } => !interface.has_type(name),
        SpecType::Option(inner) | SpecType::Vec(inner) => references_unknown(interface, inner),
        SpecType::Result { ok, error } => {
            references_unknown(interface, ok) || references_unknown(interface, error)
        }
        SpecType::Map { key, value } => {
            references_unknown(interface, key) || references_unknown(interface, value)
        }
        SpecType::Tuple(types) => types.iter().any(|ty| references_unknown(interface, ty)),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use observatory_spec::normalize;
    use observatory_testutil::spec::{spec_function, spec_struct, ty_udt};
    use observatory_testutil::{minimal_module, module_with_spec};
    use stellar_xdr::{ScSpecEntry, ScSpecTypeDef};

    fn interface(entries: Vec<ScSpecEntry>) -> ContractInterface {
        ContractInterface::from_spec(normalize(entries))
    }

    #[test]
    fn clean_interface_passes() {
        let report = audit_interface(&interface(vec![
            spec_function("hello", &[("to", ty_udt("Greeting"))], None),
            spec_struct("Greeting", &[("name", ScSpecTypeDef::String)]),
        ]));
        assert_eq!(report.status, AuditStatus::Pass);
        assert!(report.findings.is_empty());
    }

    #[test]
    fn duplicate_functions_are_errors() {
        let report = audit_interface(&interface(vec![
            spec_function("a", &[], None),
            spec_function("a", &[], None),
        ]));
        assert_eq!(report.status, AuditStatus::Error);
        assert!(report.count(FindingSeverity::Error) >= 1);
    }

    #[test]
    fn unknown_type_reference_is_error() {
        let report = audit_interface(&interface(vec![spec_function(
            "f",
            &[("x", ty_udt("Missing"))],
            None,
        )]));
        assert_eq!(report.status, AuditStatus::Error);
        assert!(report
            .findings
            .iter()
            .any(|f| f.id == "function.unknown-type"));
    }

    #[test]
    fn artifact_without_spec_warns() {
        let wasm = module_with_spec(&[spec_function("f", &[], None)]);
        let inspected = observatory_wasm::inspect(&wasm).unwrap();
        let report = audit_artifact(&inspected);
        assert_eq!(report.status, AuditStatus::Warn);
        assert!(!report
            .findings
            .iter()
            .any(|f| f.id == "artifact.no-contract-spec"));
        let empty = observatory_wasm::inspect(&minimal_module()).unwrap();
        let report = audit_artifact(&empty);
        assert!(report
            .findings
            .iter()
            .any(|f| f.id == "artifact.no-contract-spec"));
    }

    #[test]
    fn merge_is_deterministic() {
        let a = audit_interface(&interface(vec![spec_function("a", &[], None)]));
        let b = audit_artifact(&observatory_wasm::inspect(&minimal_module()).unwrap());
        let first = a.clone().merge(b.clone());
        let second = a.merge(b);
        assert_eq!(first, second);
    }
}
