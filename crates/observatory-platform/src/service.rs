//! Coarse application services composed from the contract-intelligence crates.
//!
//! The service layer performs no analysis itself; it sequences the engine
//! crates and returns typed outcomes. Transports (CLI, HTTP) call these
//! services so analysis logic lives in exactly one place.

use observatory_audit::AuditReport;
use observatory_compat::{assess, CompatibilityAssessment, CompatibilityPolicy};
use observatory_core::Result;
use observatory_diff::{diff, InterfaceDiff};
use observatory_events::{parse_events_str, EventRecord};
use observatory_interface::ContractInterface;
use observatory_rpc::{RpcClient, Transport};
use observatory_spec::{parse_wasm, summarize, ContractSpec, SpecReport};
use observatory_verify::{verify_local_vs_deployed, VerificationResult};
use observatory_wasm::{inspect, WasmReport};
use serde::Serialize;

/// Stateless application service over the contract-intelligence engine.
#[derive(Debug, Clone, Copy, Default)]
pub struct Observatory;

/// Outcome of an inspection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InspectOutcome {
    /// The WASM report.
    pub wasm: WasmReport,
    /// The canonical interface, when a specification is present.
    pub interface: Option<ContractInterface>,
    /// Security heuristics, when requested.
    pub audit: Option<AuditReport>,
}

/// Outcome of a specification read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpecOutcome {
    /// The normalized specification.
    pub spec: ContractSpec,
    /// Counts and validation issues.
    pub summary: SpecReport,
}

/// Outcome of fingerprinting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FingerprintOutcome {
    /// SHA-256 of the artifact bytes.
    pub artifact_sha256: String,
    /// Fingerprint of the canonical interface, when present.
    pub interface_sha256: Option<String>,
}

impl Observatory {
    /// Create the service.
    #[must_use]
    pub fn new() -> Self {
        Observatory
    }

    /// Inspect a WASM artifact, optionally running security heuristics.
    pub fn inspect(&self, wasm: &[u8], security: bool) -> Result<InspectOutcome> {
        let report = inspect(wasm)?;
        let interface = ContractInterface::from_wasm(wasm).ok();
        let audit = if security {
            let mut audit = observatory_audit::audit_artifact(&report);
            if let Some(interface) = &interface {
                audit = audit.merge(observatory_audit::audit_interface(interface));
            }
            Some(audit)
        } else {
            None
        };
        Ok(InspectOutcome {
            wasm: report,
            interface,
            audit,
        })
    }

    /// Read and validate the contract specification.
    pub fn spec(&self, wasm: &[u8]) -> Result<SpecOutcome> {
        let spec = parse_wasm(wasm)?;
        let summary = summarize(&spec);
        Ok(SpecOutcome { spec, summary })
    }

    /// Diff two interfaces.
    pub fn diff(&self, old: &[u8], new: &[u8]) -> Result<InterfaceDiff> {
        let old = ContractInterface::from_wasm(old)?;
        let new = ContractInterface::from_wasm(new)?;
        Ok(diff(&old, &new))
    }

    /// Assess compatibility under a policy.
    pub fn compatibility(
        &self,
        old: &[u8],
        new: &[u8],
        policy: CompatibilityPolicy,
    ) -> Result<CompatibilityAssessment> {
        let old = ContractInterface::from_wasm(old)?;
        let new = ContractInterface::from_wasm(new)?;
        Ok(assess(&old, &new, policy))
    }

    /// Compute artifact and interface fingerprints.
    pub fn fingerprint(&self, wasm: &[u8]) -> Result<FingerprintOutcome> {
        let artifact_sha256 = observatory_wasm::artifact_fingerprint(wasm);
        let interface_sha256 = match ContractInterface::from_wasm(wasm) {
            Ok(interface) => Some(interface.fingerprint()?),
            Err(_) => None,
        };
        Ok(FingerprintOutcome {
            artifact_sha256,
            interface_sha256,
        })
    }

    /// Run security heuristics over an artifact and its interface.
    pub fn security(&self, wasm: &[u8]) -> Result<AuditReport> {
        let report = inspect(wasm)?;
        let mut audit = observatory_audit::audit_artifact(&report);
        if let Ok(interface) = ContractInterface::from_wasm(wasm) {
            audit = audit.merge(observatory_audit::audit_interface(&interface));
        }
        Ok(audit)
    }

    /// Parse an event fixture.
    pub fn events(&self, json: &str) -> Result<Vec<EventRecord>> {
        parse_events_str(json)
    }

    /// Verify a local artifact against a deployment over a transport.
    pub fn verify<T: Transport>(
        &self,
        local_wasm: &[u8],
        client: &RpcClient<T>,
        contract_id: &str,
        network: &str,
    ) -> Result<VerificationResult> {
        verify_local_vs_deployed(local_wasm, client, contract_id, network)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use observatory_testutil::module_with_spec;
    use observatory_testutil::spec::{spec_function, spec_struct, ty_udt};
    use stellar_xdr::ScSpecTypeDef;

    fn contract() -> Vec<u8> {
        module_with_spec(&[
            spec_function(
                "transfer",
                &[("to", ty_udt("Recipient")), ("amount", ScSpecTypeDef::I128)],
                Some(ScSpecTypeDef::Bool),
            ),
            spec_struct("Recipient", &[("address", ScSpecTypeDef::Address)]),
        ])
    }

    #[test]
    fn inspect_reports_interface_and_audit() {
        let outcome = Observatory.inspect(&contract(), true).unwrap();
        assert!(outcome.wasm.has_contract_spec);
        assert!(outcome.interface.is_some());
        assert!(outcome.audit.is_some());
    }

    #[test]
    fn spec_summarizes() {
        let outcome = Observatory.spec(&contract()).unwrap();
        assert_eq!(outcome.summary.functions, 1);
        assert_eq!(outcome.summary.structs, 1);
    }

    #[test]
    fn fingerprint_is_stable_and_meaningful() {
        let a = Observatory.fingerprint(&contract()).unwrap();
        let b = Observatory.fingerprint(&contract()).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.artifact_sha256.len(), 64);
        assert!(a.interface_sha256.is_some());
    }

    #[test]
    fn diff_and_compatibility() {
        let old = module_with_spec(&[spec_function("a", &[], None), spec_function("b", &[], None)]);
        let new = module_with_spec(&[spec_function("a", &[], None)]);
        let diff = Observatory.diff(&old, &new).unwrap();
        assert_eq!(diff.summary.breaking, 1);
        let assessment = Observatory
            .compatibility(&old, &new, CompatibilityPolicy::strict())
            .unwrap();
        assert!(assessment.is_incompatible());
    }

    #[test]
    fn malformed_wasm_is_a_structured_error() {
        let error = Observatory.inspect(b"not wasm", false).unwrap_err();
        assert!(matches!(error, observatory_core::ObservatoryError::Wasm(_)));
    }
}
