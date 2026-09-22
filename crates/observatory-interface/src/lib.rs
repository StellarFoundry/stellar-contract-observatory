//! Canonical contract interface model and deterministic fingerprinting.
//!
//! The interface is derived from the parsed [`observatory_spec::ContractSpec`]
//! and canonicalized so that two semantically identical interfaces produce the
//! same fingerprint regardless of declaration order.
//!
//! # Ordering semantics
//!
//! * Functions, structs, unions, enums, error enums, and events are **sorted by
//!   name**; their declaration order is not semantic.
//! * Function parameters, struct fields, union cases, and enum cases **retain
//!   their declared order**, which is semantic for XDR encoding and call ABI.
//!
//! A fingerprint covers the *interface only*. Two contracts with the same
//! interface can have different implementations, so interface equality does not
//! imply implementation equality.

use observatory_core::hash::sha256_hex;
use observatory_core::{ObservatoryError, Result};
use observatory_spec::{
    ContractSpec, SpecEnum, SpecErrorEnum, SpecEvent, SpecFunction, SpecStruct, SpecType, SpecUnion,
};
use serde::{Deserialize, Serialize};

/// Version of the canonical interface representation.
pub const INTERFACE_SCHEMA_VERSION: &str = "1.0";

/// A canonical, order-independent view of a contract's public interface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractInterface {
    /// Schema version of this representation.
    pub schema_version: String,
    /// Functions, sorted by name.
    pub functions: Vec<SpecFunction>,
    /// Structs, sorted by name.
    pub structs: Vec<SpecStruct>,
    /// Unions, sorted by name.
    pub unions: Vec<SpecUnion>,
    /// Enums, sorted by name.
    pub enums: Vec<SpecEnum>,
    /// Error enums, sorted by name.
    pub error_enums: Vec<SpecErrorEnum>,
    /// Events, sorted by name.
    pub events: Vec<SpecEvent>,
}

impl ContractInterface {
    /// Build a canonical interface from a normalized specification.
    #[must_use]
    pub fn from_spec(spec: ContractSpec) -> Self {
        let mut interface = ContractInterface {
            schema_version: INTERFACE_SCHEMA_VERSION.to_string(),
            functions: spec.functions,
            structs: spec.structs,
            unions: spec.unions,
            enums: spec.enums,
            error_enums: spec.error_enums,
            events: spec.events,
        };
        interface.canonicalize();
        interface
    }

    /// Parse a contract specification directly from WASM bytes.
    pub fn from_wasm(wasm: &[u8]) -> Result<Self> {
        Ok(ContractInterface::from_spec(observatory_spec::parse_wasm(
            wasm,
        )?))
    }

    /// Parse a contract interface from a raw specification section.
    pub fn from_spec_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(ContractInterface::from_spec(observatory_spec::parse(
            bytes,
        )?))
    }

    /// Sort all name-keyed collections. Field/parameter/case order is preserved.
    pub fn canonicalize(&mut self) {
        self.functions.sort_by(|a, b| a.name.cmp(&b.name));
        self.structs.sort_by(|a, b| a.name.cmp(&b.name));
        self.unions.sort_by(|a, b| a.name.cmp(&b.name));
        self.enums.sort_by(|a, b| a.name.cmp(&b.name));
        self.error_enums.sort_by(|a, b| a.name.cmp(&b.name));
        self.events.sort_by(|a, b| a.name.cmp(&b.name));
    }

    /// Whether the interface declares nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
            && self.structs.is_empty()
            && self.unions.is_empty()
            && self.enums.is_empty()
            && self.error_enums.is_empty()
            && self.events.is_empty()
    }

    /// Find a function by name.
    #[must_use]
    pub fn function(&self, name: &str) -> Option<&SpecFunction> {
        self.functions.iter().find(|function| function.name == name)
    }

    /// Find a user-defined type by name across all type kinds.
    #[must_use]
    pub fn has_type(&self, name: &str) -> bool {
        self.structs.iter().any(|item| item.name == name)
            || self.unions.iter().any(|item| item.name == name)
            || self.enums.iter().any(|item| item.name == name)
            || self.error_enums.iter().any(|item| item.name == name)
    }

    /// All type names, sorted.
    #[must_use]
    pub fn type_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .structs
            .iter()
            .map(|item| item.name.clone())
            .chain(self.unions.iter().map(|item| item.name.clone()))
            .chain(self.enums.iter().map(|item| item.name.clone()))
            .chain(self.error_enums.iter().map(|item| item.name.clone()))
            .collect();
        names.sort();
        names
    }

    /// Total number of interface entities.
    #[must_use]
    pub fn entity_count(&self) -> usize {
        self.functions.len()
            + self.structs.len()
            + self.unions.len()
            + self.enums.len()
            + self.error_enums.len()
            + self.events.len()
    }

    /// The canonical JSON encoding used for fingerprinting.
    ///
    /// `serde_json` orders object keys deterministically, so this encoding is
    /// stable for a given interface.
    pub fn canonical_json(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(|error| {
            ObservatoryError::internal(format!("interface serialization: {error}"))
        })
    }

    /// The deterministic SHA-256 fingerprint of the canonical interface.
    pub fn fingerprint(&self) -> Result<String> {
        Ok(sha256_hex(&self.canonical_json()?))
    }

    /// Reconstruct a normalized specification for validation or summarization.
    #[must_use]
    pub fn to_spec(&self) -> ContractSpec {
        ContractSpec {
            functions: self.functions.clone(),
            structs: self.structs.clone(),
            unions: self.unions.clone(),
            enums: self.enums.clone(),
            error_enums: self.error_enums.clone(),
            events: self.events.clone(),
        }
    }
}

/// Whether `ty` references a named user-defined type.
#[must_use]
pub fn type_references_udt(ty: &SpecType) -> bool {
    match ty {
        SpecType::Udt { .. } => true,
        SpecType::Option(inner) | SpecType::Vec(inner) => type_references_udt(inner),
        SpecType::Result { ok, error } => type_references_udt(ok) || type_references_udt(error),
        SpecType::Map { key, value } => type_references_udt(key) || type_references_udt(value),
        SpecType::Tuple(types) => types.iter().any(type_references_udt),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use observatory_spec::normalize;
    use observatory_testutil::spec::{spec_enum, spec_function, spec_struct, ty_udt};
    use stellar_xdr::{ScSpecEntry, ScSpecTypeDef};

    fn spec(entries: Vec<ScSpecEntry>) -> ContractSpec {
        normalize(entries)
    }

    #[test]
    fn canonicalizes_by_name() {
        let a = ContractInterface::from_spec(spec(vec![
            spec_function("b", &[], None),
            spec_function("a", &[], None),
        ]));
        let b = ContractInterface::from_spec(spec(vec![
            spec_function("a", &[], None),
            spec_function("b", &[], None),
        ]));
        let names: Vec<_> = a.functions.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["a", "b"]);
        assert_eq!(a.fingerprint().unwrap(), b.fingerprint().unwrap());
    }

    #[test]
    fn fingerprint_changes_with_interface() {
        let a = ContractInterface::from_spec(spec(vec![spec_function(
            "f",
            &[("x", ScSpecTypeDef::U32)],
            None,
        )]));
        let b = ContractInterface::from_spec(spec(vec![spec_function(
            "f",
            &[("x", ScSpecTypeDef::I64)],
            None,
        )]));
        assert_ne!(a.fingerprint().unwrap(), b.fingerprint().unwrap());
    }

    #[test]
    fn parameter_order_is_semantic() {
        let a = ContractInterface::from_spec(spec(vec![spec_function(
            "f",
            &[("x", ScSpecTypeDef::U32), ("y", ScSpecTypeDef::U32)],
            None,
        )]));
        let b = ContractInterface::from_spec(spec(vec![spec_function(
            "f",
            &[("y", ScSpecTypeDef::U32), ("x", ScSpecTypeDef::U32)],
            None,
        )]));
        assert_ne!(a.fingerprint().unwrap(), b.fingerprint().unwrap());
    }

    #[test]
    fn type_lookup_and_counts() {
        let interface = ContractInterface::from_spec(spec(vec![
            spec_function("f", &[("s", ty_udt("S"))], None),
            spec_struct("S", &[("a", ScSpecTypeDef::U32)]),
            spec_enum("E", &[("A", 0)]),
        ]));
        assert!(interface.has_type("S"));
        assert!(interface.has_type("E"));
        assert!(!interface.has_type("Missing"));
        assert_eq!(interface.entity_count(), 3);
        assert_eq!(
            interface.type_names(),
            vec!["E".to_string(), "S".to_string()]
        );
    }

    #[test]
    fn empty_interface_is_empty() {
        let interface = ContractInterface::from_spec(ContractSpec::default());
        assert!(interface.is_empty());
        assert_eq!(interface.entity_count(), 0);
    }

    #[test]
    fn udt_reference_detection() {
        assert!(type_references_udt(&SpecType::Udt { name: "S".into() }));
        assert!(type_references_udt(&SpecType::Vec(Box::new(
            SpecType::Udt { name: "S".into() }
        ))));
        assert!(!type_references_udt(&SpecType::U32));
    }

    #[test]
    fn known_fingerprint_vector() {
        let interface = ContractInterface::from_spec(spec(vec![spec_function(
            "ping",
            &[("n", ScSpecTypeDef::U32)],
            Some(ScSpecTypeDef::Bool),
        )]));
        assert_eq!(
            interface.fingerprint().unwrap(),
            "55b4750964574865fa8ecb0a1b6c772fd9b666cf5e6f556832011ad1f3a0f1d4"
        );
    }
}
