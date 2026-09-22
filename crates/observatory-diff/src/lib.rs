//! Deterministic diffing of two contract interfaces.
//!
//! The diff enumerates *what changed* and assigns each change a default
//! severity. It does not claim a compatibility verdict; that is the job of
//! `observatory-compat`, which applies an explicit policy on top of these
//! changes.
//!
//! # Default severity rules
//!
//! | Change | Default severity |
//! | ------ | ---------------- |
//! | Function added | non-breaking |
//! | Function removed | breaking |
//! | Function signature changed | breaking |
//! | Function documentation only | informational |
//! | Type added | non-breaking |
//! | Type removed | breaking |
//! | Type definition changed | breaking |
//! | Type documentation only | informational |
//! | Event added | non-breaking |
//! | Event removed | breaking |
//! | Event definition changed | breaking |

use observatory_interface::ContractInterface;
use observatory_spec::{SpecEvent, SpecFunction, SpecStruct, SpecType, SpecUnion};
use serde::Serialize;

/// Default severity assigned to a change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Purely informational; no interface impact.
    Informational,
    /// Additive or otherwise safe.
    NonBreaking,
    /// Potentially or definitely breaks consumers.
    Breaking,
}

/// The kind of a change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    /// A function was added.
    FunctionAdded,
    /// A function was removed.
    FunctionRemoved,
    /// A function changed.
    FunctionChanged,
    /// A type was added.
    TypeAdded,
    /// A type was removed.
    TypeRemoved,
    /// A type changed.
    TypeChanged,
    /// An event was added.
    EventAdded,
    /// An event was removed.
    EventRemoved,
    /// An event changed.
    EventChanged,
}

impl ChangeKind {
    fn rank(self) -> u8 {
        match self {
            ChangeKind::FunctionRemoved => 0,
            ChangeKind::FunctionChanged => 1,
            ChangeKind::FunctionAdded => 2,
            ChangeKind::TypeRemoved => 3,
            ChangeKind::TypeChanged => 4,
            ChangeKind::TypeAdded => 5,
            ChangeKind::EventRemoved => 6,
            ChangeKind::EventChanged => 7,
            ChangeKind::EventAdded => 8,
        }
    }
}

/// A single interface change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Change {
    /// The kind of change.
    pub kind: ChangeKind,
    /// Default severity.
    pub severity: Severity,
    /// A stable path, for example `function::transfer`.
    pub path: String,
    /// Human-readable detail.
    pub detail: String,
    /// Previous rendering, when applicable.
    pub old: Option<String>,
    /// New rendering, when applicable.
    pub new: Option<String>,
}

/// Aggregate counts of a diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct DiffSummary {
    /// Breaking change count.
    pub breaking: usize,
    /// Non-breaking change count.
    pub non_breaking: usize,
    /// Informational change count.
    pub informational: usize,
}

impl DiffSummary {
    /// Total number of changes.
    #[must_use]
    pub fn total(&self) -> usize {
        self.breaking + self.non_breaking + self.informational
    }
}

/// The result of diffing two interfaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InterfaceDiff {
    /// All changes, deterministically ordered.
    pub changes: Vec<Change>,
    /// Aggregate counts.
    pub summary: DiffSummary,
}

impl InterfaceDiff {
    /// Whether any change is breaking.
    #[must_use]
    pub fn has_breaking_changes(&self) -> bool {
        self.summary.breaking > 0
    }
}

/// Diff two canonical interfaces. The result is deterministic.
#[must_use]
pub fn diff(old: &ContractInterface, new: &ContractInterface) -> InterfaceDiff {
    let mut changes = Vec::new();
    diff_functions(old, new, &mut changes);
    diff_types(old, new, &mut changes);
    diff_events(old, new, &mut changes);

    changes.sort_by(|a, b| {
        a.kind
            .rank()
            .cmp(&b.kind.rank())
            .then_with(|| a.path.cmp(&b.path))
            .then_with(|| a.detail.cmp(&b.detail))
    });

    let mut summary = DiffSummary::default();
    for change in &changes {
        match change.severity {
            Severity::Breaking => summary.breaking += 1,
            Severity::NonBreaking => summary.non_breaking += 1,
            Severity::Informational => summary.informational += 1,
        }
    }
    InterfaceDiff { changes, summary }
}

fn diff_functions(old: &ContractInterface, new: &ContractInterface, out: &mut Vec<Change>) {
    for function in &old.functions {
        match new.function(&function.name) {
            None => out.push(Change {
                kind: ChangeKind::FunctionRemoved,
                severity: Severity::Breaking,
                path: format!("function::{}", function.name),
                detail: format!("function `{}` was removed", function.name),
                old: Some(render_function(function)),
                new: None,
            }),
            Some(replacement) => {
                if function == replacement {
                    continue;
                }
                if same_signature(function, replacement) {
                    out.push(Change {
                        kind: ChangeKind::FunctionChanged,
                        severity: Severity::Informational,
                        path: format!("function::{}", function.name),
                        detail: format!("documentation of `{}` changed", function.name),
                        old: None,
                        new: None,
                    });
                } else {
                    out.push(Change {
                        kind: ChangeKind::FunctionChanged,
                        severity: Severity::Breaking,
                        path: format!("function::{}", function.name),
                        detail: describe_function_change(function, replacement),
                        old: Some(render_function(function)),
                        new: Some(render_function(replacement)),
                    });
                }
            }
        }
    }
    for function in &new.functions {
        if old.function(&function.name).is_none() {
            out.push(Change {
                kind: ChangeKind::FunctionAdded,
                severity: Severity::NonBreaking,
                path: format!("function::{}", function.name),
                detail: format!("function `{}` was added", function.name),
                old: None,
                new: Some(render_function(function)),
            });
        }
    }
}

fn diff_types(old: &ContractInterface, new: &ContractInterface, out: &mut Vec<Change>) {
    let old_types = type_map(old);
    let new_types = type_map(new);
    for (name, old_render) in &old_types {
        match new_types.get(name) {
            None => out.push(Change {
                kind: ChangeKind::TypeRemoved,
                severity: Severity::Breaking,
                path: format!("type::{name}"),
                detail: format!("type `{name}` was removed"),
                old: Some(old_render.clone()),
                new: None,
            }),
            Some(new_render) => {
                if old_render != new_render {
                    out.push(Change {
                        kind: ChangeKind::TypeChanged,
                        severity: Severity::Breaking,
                        path: format!("type::{name}"),
                        detail: format!("type `{name}` changed"),
                        old: Some(old_render.clone()),
                        new: Some(new_render.clone()),
                    });
                }
            }
        }
    }
    for (name, new_render) in &new_types {
        if !old_types.contains_key(name) {
            out.push(Change {
                kind: ChangeKind::TypeAdded,
                severity: Severity::NonBreaking,
                path: format!("type::{name}"),
                detail: format!("type `{name}` was added"),
                old: None,
                new: Some(new_render.clone()),
            });
        }
    }
}

fn diff_events(old: &ContractInterface, new: &ContractInterface, out: &mut Vec<Change>) {
    for event in &old.events {
        match new
            .events
            .iter()
            .find(|candidate| candidate.name == event.name)
        {
            None => out.push(Change {
                kind: ChangeKind::EventRemoved,
                severity: Severity::Breaking,
                path: format!("event::{}", event.name),
                detail: format!("event `{}` was removed", event.name),
                old: Some(render_event(event)),
                new: None,
            }),
            Some(replacement) => {
                if event != replacement {
                    out.push(Change {
                        kind: ChangeKind::EventChanged,
                        severity: Severity::Breaking,
                        path: format!("event::{}", event.name),
                        detail: format!("event `{}` changed", event.name),
                        old: Some(render_event(event)),
                        new: Some(render_event(replacement)),
                    });
                }
            }
        }
    }
    for event in &new.events {
        if !old
            .events
            .iter()
            .any(|candidate| candidate.name == event.name)
        {
            out.push(Change {
                kind: ChangeKind::EventAdded,
                severity: Severity::NonBreaking,
                path: format!("event::{}", event.name),
                detail: format!("event `{}` was added", event.name),
                old: None,
                new: Some(render_event(event)),
            });
        }
    }
}

fn type_map(interface: &ContractInterface) -> std::collections::BTreeMap<String, String> {
    let mut map = std::collections::BTreeMap::new();
    for item in &interface.structs {
        map.insert(item.name.clone(), render_struct(item));
    }
    for item in &interface.unions {
        map.insert(item.name.clone(), render_union(item));
    }
    for item in &interface.enums {
        map.insert(item.name.clone(), render_enum(&item.name, &item.cases));
    }
    for item in &interface.error_enums {
        map.insert(item.name.clone(), render_enum(&item.name, &item.cases));
    }
    map
}

fn same_signature(a: &SpecFunction, b: &SpecFunction) -> bool {
    a.inputs == b.inputs && a.outputs == b.outputs
}

fn describe_function_change(old: &SpecFunction, new: &SpecFunction) -> String {
    let mut parts = Vec::new();
    let max = old.inputs.len().max(new.inputs.len());
    for index in 0..max {
        match (old.inputs.get(index), new.inputs.get(index)) {
            (Some(left), Some(right)) if left.name == right.name && left.ty == right.ty => {}
            (Some(left), Some(right)) => parts.push(format!(
                "parameter {} changed from `{}: {}` to `{}: {}`",
                index + 1,
                left.name,
                left.ty,
                right.name,
                right.ty
            )),
            (Some(left), None) => parts.push(format!(
                "parameter {} `{}: {}` removed",
                index + 1,
                left.name,
                left.ty
            )),
            (None, Some(right)) => parts.push(format!(
                "parameter {} `{}: {}` added",
                index + 1,
                right.name,
                right.ty
            )),
            (None, None) => {}
        }
    }
    if old.outputs != new.outputs {
        parts.push(format!(
            "return type changed from `{}` to `{}`",
            render_outputs(&old.outputs),
            render_outputs(&new.outputs)
        ));
    }
    if parts.is_empty() {
        "documentation changed".to_string()
    } else {
        parts.join("; ")
    }
}

fn render_outputs(outputs: &[SpecType]) -> String {
    match outputs.first() {
        Some(ty) => ty.to_string(),
        None => "()".to_string(),
    }
}

fn render_function(function: &SpecFunction) -> String {
    let params = function
        .inputs
        .iter()
        .map(|input| format!("{}: {}", input.name, input.ty))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{}({}) -> {}",
        function.name,
        params,
        render_outputs(&function.outputs)
    )
}

fn render_struct(item: &SpecStruct) -> String {
    let fields = item
        .fields
        .iter()
        .map(|field| format!("{}: {}", field.name, field.ty))
        .collect::<Vec<_>>()
        .join(", ");
    format!("struct {}({})", item.name, fields)
}

fn render_union(item: &SpecUnion) -> String {
    let cases = item
        .cases
        .iter()
        .map(|case| {
            if case.types.is_empty() {
                case.name.clone()
            } else {
                let types = case
                    .types
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}({types})", case.name)
            }
        })
        .collect::<Vec<_>>()
        .join(" | ");
    format!("union {} {{ {cases} }}", item.name)
}

fn render_enum(name: &str, cases: &[observatory_spec::SpecEnumCase]) -> String {
    let cases = cases
        .iter()
        .map(|case| format!("{} = {}", case.name, case.value))
        .collect::<Vec<_>>()
        .join(", ");
    format!("enum {name} {{ {cases} }}")
}

fn render_event(event: &SpecEvent) -> String {
    let params = event
        .params
        .iter()
        .map(|param| format!("{}: {} ({})", param.name, param.ty, param.location))
        .collect::<Vec<_>>()
        .join(", ");
    format!("event {}({})", event.name, params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use observatory_spec::normalize;
    use observatory_testutil::spec::{spec_enum, spec_event, spec_function, spec_struct};
    use stellar_xdr::{ScSpecEntry, ScSpecTypeDef};

    fn interface(entries: Vec<ScSpecEntry>) -> ContractInterface {
        ContractInterface::from_spec(normalize(entries))
    }

    #[test]
    fn detects_added_and_removed_functions() {
        let old = interface(vec![
            spec_function("keep", &[], None),
            spec_function("gone", &[], None),
        ]);
        let new = interface(vec![
            spec_function("keep", &[], None),
            spec_function("fresh", &[], None),
        ]);
        let result = diff(&old, &new);
        assert!(result
            .changes
            .iter()
            .any(|c| c.kind == ChangeKind::FunctionRemoved
                && c.path == "function::gone"
                && c.severity == Severity::Breaking));
        assert!(result
            .changes
            .iter()
            .any(|c| c.kind == ChangeKind::FunctionAdded
                && c.path == "function::fresh"
                && c.severity == Severity::NonBreaking));
        assert_eq!(result.summary.breaking, 1);
        assert_eq!(result.summary.non_breaking, 1);
        assert!(result.has_breaking_changes());
    }

    #[test]
    fn detects_parameter_and_return_changes() {
        let old = interface(vec![spec_function(
            "f",
            &[("x", ScSpecTypeDef::U32)],
            Some(ScSpecTypeDef::Bool),
        )]);
        let new = interface(vec![spec_function(
            "f",
            &[("x", ScSpecTypeDef::U64)],
            Some(ScSpecTypeDef::Bool),
        )]);
        let result = diff(&old, &new);
        let change = result
            .changes
            .iter()
            .find(|c| c.kind == ChangeKind::FunctionChanged)
            .unwrap();
        assert_eq!(change.severity, Severity::Breaking);
        assert!(change.detail.contains("parameter 1 changed"));
    }

    #[test]
    fn documentation_only_change_is_informational() {
        let function = spec_function("f", &[("x", ScSpecTypeDef::U32)], None);
        let mut with_doc = spec_function("f", &[("x", ScSpecTypeDef::U32)], None);
        if let ScSpecEntry::FunctionV0(function) = &mut with_doc {
            function.doc = stellar_xdr::StringM::try_from("hello").unwrap();
        }
        let result = diff(&interface(vec![function]), &interface(vec![with_doc]));
        assert_eq!(result.changes.len(), 1);
        assert_eq!(result.changes[0].severity, Severity::Informational);
    }

    #[test]
    fn detects_type_and_event_changes() {
        let old = interface(vec![
            spec_struct("S", &[("a", ScSpecTypeDef::U32)]),
            spec_enum("E", &[("A", 0)]),
            spec_event("evt", &[], &[]),
        ]);
        let new = interface(vec![
            spec_struct("S", &[("a", ScSpecTypeDef::U64)]),
            spec_event("evt", &[], &[]),
        ]);
        let result = diff(&old, &new);
        assert!(result
            .changes
            .iter()
            .any(|c| c.kind == ChangeKind::TypeChanged));
        assert!(result
            .changes
            .iter()
            .any(|c| c.kind == ChangeKind::TypeRemoved));
        assert!(!result
            .changes
            .iter()
            .any(|c| c.kind == ChangeKind::EventRemoved || c.kind == ChangeKind::EventChanged));
    }

    #[test]
    fn identical_interfaces_have_no_changes() {
        let entries = vec![spec_function("a", &[], None), spec_struct("S", &[])];
        let result = diff(&interface(entries.clone()), &interface(entries));
        assert!(result.changes.is_empty());
        assert_eq!(result.summary.total(), 0);
    }

    #[test]
    fn diff_is_deterministic() {
        let old = interface(vec![
            spec_function("b", &[], None),
            spec_function("a", &[], None),
        ]);
        let new = interface(vec![spec_function("c", &[], None)]);
        let first = diff(&old, &new);
        let second = diff(&old, &new);
        assert_eq!(first, second);
    }
}
