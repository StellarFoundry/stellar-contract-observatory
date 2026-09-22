//! Ergonomic builders for `ScSpecEntry` fixtures.
//!
//! These construct the exact XDR types the Soroban contract specification uses
//! so tests exercise the real parser, not a mock.

use stellar_xdr::{
    ScSpecEntry, ScSpecEventDataFormat, ScSpecEventParamLocationV0, ScSpecEventParamV0,
    ScSpecEventV0, ScSpecFunctionInputV0, ScSpecFunctionV0, ScSpecTypeDef, ScSpecTypeUdt,
    ScSpecUdtEnumCaseV0, ScSpecUdtEnumV0, ScSpecUdtErrorEnumCaseV0, ScSpecUdtErrorEnumV0,
    ScSpecUdtStructFieldV0, ScSpecUdtStructV0, ScSpecUdtUnionCaseTupleV0, ScSpecUdtUnionCaseV0,
    ScSpecUdtUnionCaseVoidV0, ScSpecUdtUnionV0, ScSymbol, StringM, VecM,
};

fn sm<const MAX: u32>(value: &str) -> StringM<MAX> {
    StringM::try_from(value).expect("value within limit")
}

fn sym(value: &str) -> ScSymbol {
    ScSymbol::from(sm::<32>(value))
}

fn vecm<T, const MAX: u32>(items: Vec<T>) -> VecM<T, MAX> {
    VecM::try_from(items).expect("within limit")
}

/// A reference to a user-defined type by name.
#[must_use]
pub fn ty_udt(name: &str) -> ScSpecTypeDef {
    ScSpecTypeDef::Udt(ScSpecTypeUdt { name: sm(name) })
}

/// A `ScSpecEntry::FunctionV0` with the given inputs and optional return type.
#[must_use]
pub fn spec_function(
    name: &str,
    inputs: &[(&str, ScSpecTypeDef)],
    output: Option<ScSpecTypeDef>,
) -> ScSpecEntry {
    let inputs = inputs
        .iter()
        .map(|(name, ty)| ScSpecFunctionInputV0 {
            doc: Default::default(),
            name: sm(name),
            type_: ty.clone(),
        })
        .collect();
    let outputs = output.into_iter().collect();
    ScSpecEntry::FunctionV0(ScSpecFunctionV0 {
        doc: Default::default(),
        name: sym(name),
        inputs: vecm(inputs),
        outputs: vecm(outputs),
    })
}

/// A `ScSpecEntry::UdtStructV0` with the given fields.
#[must_use]
pub fn spec_struct(name: &str, fields: &[(&str, ScSpecTypeDef)]) -> ScSpecEntry {
    let fields = fields
        .iter()
        .map(|(name, ty)| ScSpecUdtStructFieldV0 {
            doc: Default::default(),
            name: sm(name),
            type_: ty.clone(),
        })
        .collect();
    ScSpecEntry::UdtStructV0(ScSpecUdtStructV0 {
        doc: Default::default(),
        lib: Default::default(),
        name: sm(name),
        fields: vecm(fields),
    })
}

/// A `ScSpecEntry::UdtEnumV0` with the given cases.
#[must_use]
pub fn spec_enum(name: &str, cases: &[(&str, u32)]) -> ScSpecEntry {
    let cases = cases
        .iter()
        .map(|(name, value)| ScSpecUdtEnumCaseV0 {
            doc: Default::default(),
            name: sm(name),
            value: *value,
        })
        .collect();
    ScSpecEntry::UdtEnumV0(ScSpecUdtEnumV0 {
        doc: Default::default(),
        lib: Default::default(),
        name: sm(name),
        cases: vecm(cases),
    })
}

/// A `ScSpecEntry::UdtErrorEnumV0` with the given cases.
#[must_use]
pub fn spec_error_enum(name: &str, cases: &[(&str, u32)]) -> ScSpecEntry {
    let cases = cases
        .iter()
        .map(|(name, value)| ScSpecUdtErrorEnumCaseV0 {
            doc: Default::default(),
            name: sm(name),
            value: *value,
        })
        .collect();
    ScSpecEntry::UdtErrorEnumV0(ScSpecUdtErrorEnumV0 {
        doc: Default::default(),
        lib: Default::default(),
        name: sm(name),
        cases: vecm(cases),
    })
}

/// A `ScSpecEntry::UdtUnionV0`. `Some(types)` builds a tuple case; `None` a void case.
#[must_use]
pub fn spec_union(name: &str, cases: &[(&str, Option<Vec<ScSpecTypeDef>>)]) -> ScSpecEntry {
    let cases = cases
        .iter()
        .map(|(name, types)| match types {
            Some(types) => ScSpecUdtUnionCaseV0::TupleV0(ScSpecUdtUnionCaseTupleV0 {
                doc: Default::default(),
                name: sm(name),
                type_: vecm(types.clone()),
            }),
            None => ScSpecUdtUnionCaseV0::VoidV0(ScSpecUdtUnionCaseVoidV0 {
                doc: Default::default(),
                name: sm(name),
            }),
        })
        .collect();
    ScSpecEntry::UdtUnionV0(ScSpecUdtUnionV0 {
        doc: Default::default(),
        lib: Default::default(),
        name: sm(name),
        cases: vecm(cases),
    })
}

/// A `ScSpecEntry::EventV0`.
#[must_use]
pub fn spec_event(
    name: &str,
    prefix_topics: &[&str],
    params: &[(&str, ScSpecTypeDef, ScSpecEventParamLocationV0)],
) -> ScSpecEntry {
    let params = params
        .iter()
        .map(|(name, ty, location)| ScSpecEventParamV0 {
            doc: Default::default(),
            name: sm(name),
            type_: ty.clone(),
            location: *location,
        })
        .collect();
    ScSpecEntry::EventV0(ScSpecEventV0 {
        doc: Default::default(),
        lib: Default::default(),
        name: sym(name),
        prefix_topics: vecm(prefix_topics.iter().map(|t| sym(t)).collect()),
        params: vecm(params),
        data_format: ScSpecEventDataFormat::SingleValue,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use stellar_xdr::{Limits, WriteXdr};

    #[test]
    fn builds_and_encodes_function() {
        let entry = spec_function("hello", &[("to", ty_udt("Greeting"))], None);
        let bytes = entry.to_xdr(Limits::none()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn builds_struct_enum_union_event() {
        for entry in [
            spec_struct("S", &[("a", ScSpecTypeDef::U32)]),
            spec_enum("E", &[("A", 0), ("B", 1)]),
            spec_union(
                "U",
                &[("None", None), ("Some", Some(vec![ScSpecTypeDef::U32]))],
            ),
            spec_event(
                "evt",
                &["transfer"],
                &[("amt", ScSpecTypeDef::I128, ScSpecEventParamLocationV0::Data)],
            ),
        ] {
            assert!(!entry.to_xdr(Limits::none()).unwrap().is_empty());
        }
    }
}
