//! Parse and normalize the Soroban contract specification.
//!
//! The specification is embedded in a `contractspecv0` custom section as a
//! stream of XDR-encoded `SCSpecEntry` values. This crate decodes that stream
//! with a bounded reader and converts it into the parser-independent
//! [`model::ContractSpec`].

pub mod model;

use std::io::Cursor;

use observatory_core::limits::{MAX_SPEC_ENTRIES, MAX_SPEC_SECTION_BYTES};
use observatory_core::{ObservatoryError, Result};
use serde::Serialize;
use stellar_xdr::{
    Limited, Limits, ReadXdr, ScSpecEntry, ScSpecEventDataFormat, ScSpecEventParamLocationV0,
    ScSpecTypeDef,
};

pub use model::{
    ContractSpec, SpecEnum, SpecEnumCase, SpecErrorEnum, SpecEvent, SpecEventParam, SpecField,
    SpecFunction, SpecParam, SpecStruct, SpecType, SpecUnion, SpecUnionCase,
};

/// Severity of a specification validation issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SpecSeverity {
    /// Informational.
    Info,
    /// Non-fatal inconsistency.
    Warning,
    /// The specification is internally inconsistent.
    Error,
}

/// A single validation issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpecIssue {
    /// Severity.
    pub severity: SpecSeverity,
    /// Human-readable description.
    pub message: String,
}

/// Aggregate counts and validation issues for a specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpecReport {
    /// Number of functions.
    pub functions: usize,
    /// Number of structs.
    pub structs: usize,
    /// Number of unions.
    pub unions: usize,
    /// Number of enums.
    pub enums: usize,
    /// Number of error enums.
    pub error_enums: usize,
    /// Number of events.
    pub events: usize,
    /// Validation issues.
    pub issues: Vec<SpecIssue>,
}

/// Decode a stream of `SCSpecEntry` values from `bytes`.
pub fn parse_entries(bytes: &[u8]) -> Result<Vec<ScSpecEntry>> {
    if bytes.len() > MAX_SPEC_SECTION_BYTES {
        return Err(ObservatoryError::spec(format!(
            "specification section is {} bytes, exceeding the {} byte limit",
            bytes.len(),
            MAX_SPEC_SECTION_BYTES
        )));
    }
    let mut limited = Limited::new(Cursor::new(bytes), Limits::none());
    let mut entries = Vec::new();
    while (limited.inner.position() as usize) < bytes.len() {
        if entries.len() >= MAX_SPEC_ENTRIES {
            return Err(ObservatoryError::spec(format!(
                "specification has more than {MAX_SPEC_ENTRIES} entries"
            )));
        }
        let entry = ScSpecEntry::read_xdr(&mut limited)
            .map_err(|error| ObservatoryError::spec(format!("invalid spec entry: {error}")))?;
        entries.push(entry);
    }
    Ok(entries)
}

/// Normalize decoded entries into the parser-independent model.
#[must_use]
pub fn normalize(entries: Vec<ScSpecEntry>) -> ContractSpec {
    let mut spec = ContractSpec::default();
    for entry in entries {
        match entry {
            ScSpecEntry::FunctionV0(function) => spec.functions.push(SpecFunction {
                name: function.name.0.to_string(),
                doc: function.doc.to_string(),
                inputs: function
                    .inputs
                    .into_iter()
                    .map(|input| SpecParam {
                        name: input.name.to_string(),
                        doc: input.doc.to_string(),
                        ty: convert_type(input.type_),
                    })
                    .collect(),
                outputs: function.outputs.into_iter().map(convert_type).collect(),
            }),
            ScSpecEntry::UdtStructV0(item) => spec.structs.push(SpecStruct {
                name: item.name.to_string(),
                doc: item.doc.to_string(),
                fields: item
                    .fields
                    .into_iter()
                    .map(|field| SpecField {
                        name: field.name.to_string(),
                        doc: field.doc.to_string(),
                        ty: convert_type(field.type_),
                    })
                    .collect(),
            }),
            ScSpecEntry::UdtUnionV0(item) => spec.unions.push(SpecUnion {
                name: item.name.to_string(),
                doc: item.doc.to_string(),
                cases: item
                    .cases
                    .into_iter()
                    .map(|case| match case {
                        stellar_xdr::ScSpecUdtUnionCaseV0::VoidV0(void) => SpecUnionCase {
                            name: void.name.to_string(),
                            doc: void.doc.to_string(),
                            types: Vec::new(),
                        },
                        stellar_xdr::ScSpecUdtUnionCaseV0::TupleV0(tuple) => SpecUnionCase {
                            name: tuple.name.to_string(),
                            doc: tuple.doc.to_string(),
                            types: tuple.type_.into_iter().map(convert_type).collect(),
                        },
                    })
                    .collect(),
            }),
            ScSpecEntry::UdtEnumV0(item) => spec.enums.push(SpecEnum {
                name: item.name.to_string(),
                doc: item.doc.to_string(),
                cases: item
                    .cases
                    .into_iter()
                    .map(|case| SpecEnumCase {
                        name: case.name.to_string(),
                        value: case.value,
                    })
                    .collect(),
            }),
            ScSpecEntry::UdtErrorEnumV0(item) => spec.error_enums.push(SpecErrorEnum {
                name: item.name.to_string(),
                doc: item.doc.to_string(),
                cases: item
                    .cases
                    .into_iter()
                    .map(|case| SpecEnumCase {
                        name: case.name.to_string(),
                        value: case.value,
                    })
                    .collect(),
            }),
            ScSpecEntry::EventV0(item) => spec.events.push(SpecEvent {
                name: item.name.0.to_string(),
                doc: item.doc.to_string(),
                prefix_topics: item
                    .prefix_topics
                    .into_iter()
                    .map(|topic| topic.0.to_string())
                    .collect(),
                params: item
                    .params
                    .into_iter()
                    .map(|param| SpecEventParam {
                        name: param.name.to_string(),
                        ty: convert_type(param.type_),
                        location: event_location(&param.location).to_string(),
                    })
                    .collect(),
                data_format: event_data_format(&item.data_format).to_string(),
            }),
        }
    }
    spec
}

/// Parse a raw specification section into the normalized model.
pub fn parse(bytes: &[u8]) -> Result<ContractSpec> {
    Ok(normalize(parse_entries(bytes)?))
}

/// Extract and parse the `contractspecv0` section from a WASM module.
pub fn parse_wasm(wasm: &[u8]) -> Result<ContractSpec> {
    let section =
        observatory_wasm::extract_custom_section(wasm, observatory_wasm::CONTRACT_SPEC_SECTION)?
            .ok_or_else(|| {
                ObservatoryError::spec(
                    "WASM module has no `contractspecv0` section; it may not be a Soroban contract",
                )
            })?;
    parse(&section)
}

/// Validate a normalized specification and return any issues.
#[must_use]
pub fn validate(spec: &ContractSpec) -> Vec<SpecIssue> {
    let mut issues = Vec::new();

    let mut function_names: Vec<&str> = spec.functions.iter().map(|f| f.name.as_str()).collect();
    function_names.sort_unstable();
    for pair in function_names.windows(2) {
        if pair[0] == pair[1] {
            issues.push(SpecIssue {
                severity: SpecSeverity::Error,
                message: format!("duplicate function name `{}`", pair[0]),
            });
        }
    }

    let mut udt_names = spec.udt_names();
    let original_len = udt_names.len();
    udt_names.dedup();
    if udt_names.len() != original_len {
        issues.push(SpecIssue {
            severity: SpecSeverity::Error,
            message: "duplicate user-defined type name".to_string(),
        });
    }

    let known: std::collections::BTreeSet<&str> = udt_names.iter().map(String::as_str).collect();
    let mut references = Vec::new();
    for function in &spec.functions {
        for input in &function.inputs {
            collect_references(&input.ty, &mut references);
        }
        for output in &function.outputs {
            collect_references(output, &mut references);
        }
    }
    for item in &spec.structs {
        for field in &item.fields {
            collect_references(&field.ty, &mut references);
        }
    }
    for item in &spec.unions {
        for case in &item.cases {
            for ty in &case.types {
                collect_references(ty, &mut references);
            }
        }
    }
    for event in &spec.events {
        for param in &event.params {
            collect_references(&param.ty, &mut references);
        }
    }
    references.sort_unstable();
    references.dedup();
    for reference in references {
        if !known.contains(reference.as_str()) {
            issues.push(SpecIssue {
                severity: SpecSeverity::Error,
                message: format!("reference to unknown type `{reference}`"),
            });
        }
    }

    let mut event_names: Vec<&str> = spec.events.iter().map(|e| e.name.as_str()).collect();
    event_names.sort_unstable();
    for pair in event_names.windows(2) {
        if pair[0] == pair[1] {
            issues.push(SpecIssue {
                severity: SpecSeverity::Warning,
                message: format!("duplicate event name `{}`", pair[0]),
            });
        }
    }

    for function in &spec.functions {
        if function.outputs.len() > 1 {
            issues.push(SpecIssue {
                severity: SpecSeverity::Error,
                message: format!(
                    "function `{}` declares {} return types; the specification allows at most one",
                    function.name,
                    function.outputs.len()
                ),
            });
        }
    }

    issues
}

/// Summarize a specification into counts plus validation issues.
#[must_use]
pub fn summarize(spec: &ContractSpec) -> SpecReport {
    SpecReport {
        functions: spec.functions.len(),
        structs: spec.structs.len(),
        unions: spec.unions.len(),
        enums: spec.enums.len(),
        error_enums: spec.error_enums.len(),
        events: spec.events.len(),
        issues: validate(spec),
    }
}

fn collect_references(ty: &SpecType, out: &mut Vec<String>) {
    match ty {
        SpecType::Option(inner) | SpecType::Vec(inner) => collect_references(inner, out),
        SpecType::Result { ok, error } => {
            collect_references(ok, out);
            collect_references(error, out);
        }
        SpecType::Map { key, value } => {
            collect_references(key, out);
            collect_references(value, out);
        }
        SpecType::Tuple(types) => {
            for ty in types {
                collect_references(ty, out);
            }
        }
        SpecType::Udt { name } => out.push(name.clone()),
        _ => {}
    }
}

fn convert_type(ty: ScSpecTypeDef) -> SpecType {
    match ty {
        ScSpecTypeDef::Val => SpecType::Val,
        ScSpecTypeDef::Bool => SpecType::Bool,
        ScSpecTypeDef::Void => SpecType::Void,
        ScSpecTypeDef::Error => SpecType::Error,
        ScSpecTypeDef::U32 => SpecType::U32,
        ScSpecTypeDef::I32 => SpecType::I32,
        ScSpecTypeDef::U64 => SpecType::U64,
        ScSpecTypeDef::I64 => SpecType::I64,
        ScSpecTypeDef::Timepoint => SpecType::Timepoint,
        ScSpecTypeDef::Duration => SpecType::Duration,
        ScSpecTypeDef::U128 => SpecType::U128,
        ScSpecTypeDef::I128 => SpecType::I128,
        ScSpecTypeDef::U256 => SpecType::U256,
        ScSpecTypeDef::I256 => SpecType::I256,
        ScSpecTypeDef::Bytes => SpecType::Bytes,
        ScSpecTypeDef::String => SpecType::String,
        ScSpecTypeDef::Symbol => SpecType::Symbol,
        ScSpecTypeDef::Address => SpecType::Address,
        ScSpecTypeDef::MuxedAddress => SpecType::MuxedAddress,
        ScSpecTypeDef::Option(option) => {
            SpecType::Option(Box::new(convert_type(*option.value_type)))
        }
        ScSpecTypeDef::Result(result) => SpecType::Result {
            ok: Box::new(convert_type(*result.ok_type)),
            error: Box::new(convert_type(*result.error_type)),
        },
        ScSpecTypeDef::Vec(vector) => SpecType::Vec(Box::new(convert_type(*vector.element_type))),
        ScSpecTypeDef::Map(map) => SpecType::Map {
            key: Box::new(convert_type(*map.key_type)),
            value: Box::new(convert_type(*map.value_type)),
        },
        ScSpecTypeDef::Tuple(tuple) => {
            SpecType::Tuple(tuple.value_types.into_iter().map(convert_type).collect())
        }
        ScSpecTypeDef::BytesN(bytes) => SpecType::BytesN { bytes: bytes.n },
        ScSpecTypeDef::Udt(udt) => SpecType::Udt {
            name: udt.name.to_string(),
        },
    }
}

fn event_location(location: &ScSpecEventParamLocationV0) -> &'static str {
    match location {
        ScSpecEventParamLocationV0::Data => "data",
        ScSpecEventParamLocationV0::TopicList => "topic_list",
    }
}

fn event_data_format(format: &ScSpecEventDataFormat) -> &'static str {
    match format {
        ScSpecEventDataFormat::SingleValue => "single_value",
        ScSpecEventDataFormat::Vec => "vec",
        ScSpecEventDataFormat::Map => "map",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use observatory_testutil::spec::{
        spec_enum, spec_event, spec_function, spec_struct, spec_union, ty_udt,
    };
    use observatory_testutil::{encode_spec_entries, module_with_spec};
    use stellar_xdr::{ScSpecEventParamLocationV0, ScSpecTypeDef, ScSpecTypeOption};

    fn sample_entries() -> Vec<ScSpecEntry> {
        vec![
            spec_function(
                "transfer",
                &[("to", ty_udt("Recipient")), ("amount", ScSpecTypeDef::I128)],
                Some(ScSpecTypeDef::Bool),
            ),
            spec_struct("Recipient", &[("address", ScSpecTypeDef::Address)]),
            spec_enum("Status", &[("Active", 0), ("Paused", 1)]),
            spec_event(
                "transfer",
                &["transfer"],
                &[(
                    "amount",
                    ScSpecTypeDef::I128,
                    ScSpecEventParamLocationV0::Data,
                )],
            ),
        ]
    }

    #[test]
    fn parses_and_normalizes_entries() {
        let bytes = encode_spec_entries(&sample_entries());
        let spec = parse(&bytes).unwrap();
        assert_eq!(spec.functions.len(), 1);
        assert_eq!(spec.structs.len(), 1);
        assert_eq!(spec.enums.len(), 1);
        assert_eq!(spec.events.len(), 1);
        let function = spec.function("transfer").unwrap();
        assert_eq!(function.inputs.len(), 2);
        assert_eq!(
            function.inputs[0].ty,
            SpecType::Udt {
                name: "Recipient".into()
            }
        );
        assert_eq!(function.outputs, vec![SpecType::Bool]);
        assert_eq!(spec.events[0].prefix_topics, vec!["transfer".to_string()]);
    }

    #[test]
    fn parses_from_wasm() {
        let wasm = module_with_spec(&sample_entries());
        let spec = parse_wasm(&wasm).unwrap();
        assert_eq!(spec.functions.len(), 1);
    }

    #[test]
    fn missing_spec_section_errors() {
        let error = parse_wasm(&observatory_testutil::minimal_module()).unwrap_err();
        assert!(matches!(error, ObservatoryError::Spec(_)));
    }

    #[test]
    fn malformed_spec_errors() {
        let error = parse(&[0xff, 0xff, 0xff, 0xff]).unwrap_err();
        assert!(matches!(error, ObservatoryError::Spec(_)));
    }

    #[test]
    fn validation_flags_duplicates_and_missing_refs() {
        let spec = normalize(vec![
            spec_function("a", &[], None),
            spec_function("a", &[], None),
            spec_function("b", &[("x", ty_udt("Missing"))], None),
        ]);
        let issues = validate(&spec);
        assert!(issues
            .iter()
            .any(|issue| issue.message.contains("duplicate function name `a`")));
        assert!(issues
            .iter()
            .any(|issue| issue.message.contains("unknown type `Missing`")));
    }

    #[test]
    fn valid_spec_has_no_errors() {
        let spec = normalize(sample_entries());
        let issues = validate(&spec);
        assert!(
            issues
                .iter()
                .all(|issue| issue.severity != SpecSeverity::Error),
            "unexpected issues: {issues:?}"
        );
    }

    #[test]
    fn converts_complex_types() {
        let option = ScSpecTypeDef::Option(Box::new(ScSpecTypeOption {
            value_type: Box::new(ScSpecTypeDef::U32),
        }));
        let spec = normalize(vec![spec_function("f", &[("o", option)], None)]);
        assert_eq!(spec.functions[0].inputs[0].ty.to_string(), "option<u32>");
    }

    #[test]
    fn union_void_and_tuple_cases() {
        let spec = normalize(vec![spec_union(
            "Choice",
            &[("None", None), ("Some", Some(vec![ScSpecTypeDef::U32]))],
        )]);
        assert_eq!(spec.unions[0].cases[0].types.len(), 0);
        assert_eq!(spec.unions[0].cases[1].types, vec![SpecType::U32]);
    }

    #[test]
    fn summarize_counts_entities() {
        let report = summarize(&normalize(sample_entries()));
        assert_eq!(report.functions, 1);
        assert_eq!(report.events, 1);
        assert!(report
            .issues
            .iter()
            .all(|i| i.severity != SpecSeverity::Error));
    }
}
