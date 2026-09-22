//! Parser-independent normalized contract specification model.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A normalized Soroban contract specification type.
///
/// This is intentionally independent of any single XDR parser so downstream
/// analysis does not depend on parser internals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecType {
    /// The untyped `Val`.
    Val,
    /// Boolean.
    Bool,
    /// Unit/void.
    Void,
    /// Error value.
    Error,
    /// 32-bit unsigned integer.
    U32,
    /// 32-bit signed integer.
    I32,
    /// 64-bit unsigned integer.
    U64,
    /// 64-bit signed integer.
    I64,
    /// Timepoint.
    Timepoint,
    /// Duration.
    Duration,
    /// 128-bit unsigned integer.
    U128,
    /// 128-bit signed integer.
    I128,
    /// 256-bit unsigned integer.
    U256,
    /// 256-bit signed integer.
    I256,
    /// Variable-length bytes.
    Bytes,
    /// String.
    String,
    /// Symbol.
    Symbol,
    /// Address.
    Address,
    /// Muxed address.
    MuxedAddress,
    /// Optional value.
    Option(Box<SpecType>),
    /// Result with ok and error types.
    Result {
        /// Ok type.
        ok: Box<SpecType>,
        /// Error type.
        error: Box<SpecType>,
    },
    /// Vector.
    Vec(Box<SpecType>),
    /// Map.
    Map {
        /// Key type.
        key: Box<SpecType>,
        /// Value type.
        value: Box<SpecType>,
    },
    /// Tuple.
    Tuple(Vec<SpecType>),
    /// Fixed-size byte array.
    BytesN {
        /// Length in bytes.
        bytes: u32,
    },
    /// Reference to a named user-defined type.
    Udt {
        /// The referenced type name.
        name: String,
    },
}

impl fmt::Display for SpecType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpecType::Val => f.write_str("val"),
            SpecType::Bool => f.write_str("bool"),
            SpecType::Void => f.write_str("void"),
            SpecType::Error => f.write_str("error"),
            SpecType::U32 => f.write_str("u32"),
            SpecType::I32 => f.write_str("i32"),
            SpecType::U64 => f.write_str("u64"),
            SpecType::I64 => f.write_str("i64"),
            SpecType::Timepoint => f.write_str("timepoint"),
            SpecType::Duration => f.write_str("duration"),
            SpecType::U128 => f.write_str("u128"),
            SpecType::I128 => f.write_str("i128"),
            SpecType::U256 => f.write_str("u256"),
            SpecType::I256 => f.write_str("i256"),
            SpecType::Bytes => f.write_str("bytes"),
            SpecType::String => f.write_str("string"),
            SpecType::Symbol => f.write_str("symbol"),
            SpecType::Address => f.write_str("address"),
            SpecType::MuxedAddress => f.write_str("muxed_address"),
            SpecType::Option(inner) => write!(f, "option<{inner}>"),
            SpecType::Result { ok, error } => write!(f, "result<{ok}, {error}>"),
            SpecType::Vec(inner) => write!(f, "vec<{inner}>"),
            SpecType::Map { key, value } => write!(f, "map<{key}, {value}>"),
            SpecType::Tuple(types) => {
                f.write_str("tuple<")?;
                for (index, ty) in types.iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{ty}")?;
                }
                f.write_str(">")
            }
            SpecType::BytesN { bytes } => write!(f, "bytes[{bytes}]"),
            SpecType::Udt { name } => f.write_str(name),
        }
    }
}

/// A function input parameter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecParam {
    /// Parameter name.
    pub name: String,
    /// Documentation string.
    pub doc: String,
    /// Parameter type.
    pub ty: SpecType,
}

/// A contract function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecFunction {
    /// Function name.
    pub name: String,
    /// Documentation string.
    pub doc: String,
    /// Ordered inputs. Parameter order is semantic.
    pub inputs: Vec<SpecParam>,
    /// Return types. At most one is allowed by the specification.
    pub outputs: Vec<SpecType>,
}

/// A struct field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecField {
    /// Field name.
    pub name: String,
    /// Documentation string.
    pub doc: String,
    /// Field type.
    pub ty: SpecType,
}

/// A user-defined struct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecStruct {
    /// Type name.
    pub name: String,
    /// Documentation string.
    pub doc: String,
    /// Ordered fields. Field order is semantic for encoding.
    pub fields: Vec<SpecField>,
}

/// A union case. An empty `types` list denotes a void case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecUnionCase {
    /// Case name.
    pub name: String,
    /// Documentation string.
    pub doc: String,
    /// Tuple payload types; empty for void cases.
    pub types: Vec<SpecType>,
}

/// A user-defined union.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecUnion {
    /// Type name.
    pub name: String,
    /// Documentation string.
    pub doc: String,
    /// Ordered cases.
    pub cases: Vec<SpecUnionCase>,
}

/// A single enum case with its numeric value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecEnumCase {
    /// Case name.
    pub name: String,
    /// Numeric value.
    pub value: u32,
}

/// A user-defined enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecEnum {
    /// Type name.
    pub name: String,
    /// Documentation string.
    pub doc: String,
    /// Ordered cases.
    pub cases: Vec<SpecEnumCase>,
}

/// A user-defined error enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecErrorEnum {
    /// Type name.
    pub name: String,
    /// Documentation string.
    pub doc: String,
    /// Ordered cases.
    pub cases: Vec<SpecEnumCase>,
}

/// An event parameter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecEventParam {
    /// Parameter name.
    pub name: String,
    /// Parameter type.
    pub ty: SpecType,
    /// Where the parameter appears: `data` or `topic_list`.
    pub location: String,
}

/// A declared contract event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecEvent {
    /// Event name.
    pub name: String,
    /// Documentation string.
    pub doc: String,
    /// Prefix topics (at most two by specification).
    pub prefix_topics: Vec<String>,
    /// Ordered parameters.
    pub params: Vec<SpecEventParam>,
    /// Data format: `single_value`, `vec`, or `map`.
    pub data_format: String,
}

/// The normalized contract specification.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ContractSpec {
    /// Declared functions.
    pub functions: Vec<SpecFunction>,
    /// Declared structs.
    pub structs: Vec<SpecStruct>,
    /// Declared unions.
    pub unions: Vec<SpecUnion>,
    /// Declared enums.
    pub enums: Vec<SpecEnum>,
    /// Declared error enums.
    pub error_enums: Vec<SpecErrorEnum>,
    /// Declared events.
    pub events: Vec<SpecEvent>,
}

impl ContractSpec {
    /// Whether the specification declares nothing.
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

    /// All named user-defined types, sorted by name.
    #[must_use]
    pub fn udt_names(&self) -> Vec<String> {
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

    /// Total number of declared entities.
    #[must_use]
    pub fn entity_count(&self) -> usize {
        self.functions.len()
            + self.structs.len()
            + self.unions.len()
            + self.enums.len()
            + self.error_enums.len()
            + self.events.len()
    }
}
