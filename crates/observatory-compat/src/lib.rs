//! Documented interface compatibility rules engine.
//!
//! Compatibility is a policy decision layered on top of the raw diff. The
//! engine never returns a bare boolean: it returns a status, the explicit rules
//! that fired, and the underlying changes.
//!
//! # Default (strict) rules
//!
//! A change is **incompatible** when it is a removal or a modification of an
//! existing public contract surface:
//!
//! | Rule | Default |
//! | ---- | ------- |
//! | Removing a function | incompatible |
//! | Changing a function signature (parameters or return type) | incompatible |
//! | Removing a type | incompatible |
//! | Changing a type definition | incompatible |
//! | Removing an event | incompatible |
//! | Changing an event | incompatible |
//! | Adding a function, type, or event | compatible |
//! | Documentation-only changes | compatible (informational) |
//!
//! Any of the removal/modification rules can be relaxed through
//! [`CompatibilityPolicy`]. Relaxing a rule does not make the change disappear;
//! it is reported as a non-breaking note.
//!
//! If the baseline interface is empty there is nothing to compare against, so
//! the status is [`CompatibilityStatus::Unknown`] rather than a false verdict.

use observatory_diff::{diff, ChangeKind, InterfaceDiff, Severity};
use observatory_interface::ContractInterface;
use serde::{Deserialize, Serialize};

/// The overall compatibility verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityStatus {
    /// No breaking change under the active policy.
    Compatible,
    /// At least one breaking change under the active policy.
    Incompatible,
    /// There is insufficient information to decide.
    Unknown,
}

/// Configuration of the compatibility rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityPolicy {
    /// Removing a function is breaking.
    pub function_removal_is_breaking: bool,
    /// Changing a function signature is breaking.
    pub signature_change_is_breaking: bool,
    /// Removing a type is breaking.
    pub type_removal_is_breaking: bool,
    /// Changing a type definition is breaking.
    pub type_change_is_breaking: bool,
    /// Removing an event is breaking.
    pub event_removal_is_breaking: bool,
    /// Changing an event definition is breaking.
    pub event_change_is_breaking: bool,
}

impl Default for CompatibilityPolicy {
    fn default() -> Self {
        CompatibilityPolicy::strict()
    }
}

impl CompatibilityPolicy {
    /// The default strict policy.
    #[must_use]
    pub fn strict() -> Self {
        CompatibilityPolicy {
            function_removal_is_breaking: true,
            signature_change_is_breaking: true,
            type_removal_is_breaking: true,
            type_change_is_breaking: true,
            event_removal_is_breaking: true,
            event_change_is_breaking: true,
        }
    }

    /// A lenient policy that treats all changes as non-breaking notes.
    ///
    /// This is useful only when downstream consumers are known to tolerate any
    /// interface change; use it deliberately.
    #[must_use]
    pub fn lenient() -> Self {
        CompatibilityPolicy {
            function_removal_is_breaking: false,
            signature_change_is_breaking: false,
            type_removal_is_breaking: false,
            type_change_is_breaking: false,
            event_removal_is_breaking: false,
            event_change_is_breaking: false,
        }
    }

    /// Whether a change kind is breaking under this policy.
    #[must_use]
    pub fn is_breaking(&self, kind: ChangeKind) -> bool {
        match kind {
            ChangeKind::FunctionRemoved => self.function_removal_is_breaking,
            ChangeKind::FunctionRenamed | ChangeKind::FunctionChanged => {
                self.signature_change_is_breaking
            }
            ChangeKind::TypeRemoved => self.type_removal_is_breaking,
            ChangeKind::TypeChanged => self.type_change_is_breaking,
            ChangeKind::EventRemoved => self.event_removal_is_breaking,
            ChangeKind::EventChanged => self.event_change_is_breaking,
            ChangeKind::FunctionAdded | ChangeKind::TypeAdded | ChangeKind::EventAdded => false,
        }
    }
}

/// One rule outcome with its explanation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompatibilityReason {
    /// Whether this reason makes the interface incompatible.
    pub breaking: bool,
    /// A stable path, for example `function::transfer`.
    pub path: String,
    /// Human-readable explanation.
    pub message: String,
}

/// The result of assessing compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompatibilityAssessment {
    /// Overall verdict.
    pub status: CompatibilityStatus,
    /// The active policy.
    pub policy: CompatibilityPolicy,
    /// Reasons that contributed to the verdict, deterministically ordered.
    pub reasons: Vec<CompatibilityReason>,
    /// The underlying diff.
    pub diff: InterfaceDiff,
}

impl CompatibilityAssessment {
    /// Whether the status is [`CompatibilityStatus::Incompatible`].
    #[must_use]
    pub fn is_incompatible(&self) -> bool {
        self.status == CompatibilityStatus::Incompatible
    }
}

/// Assess compatibility of `new` relative to the `old` baseline under `policy`.
#[must_use]
pub fn assess(
    old: &ContractInterface,
    new: &ContractInterface,
    policy: CompatibilityPolicy,
) -> CompatibilityAssessment {
    let diff = diff(old, new);

    if old.is_empty() {
        return CompatibilityAssessment {
            status: CompatibilityStatus::Unknown,
            policy,
            reasons: vec![CompatibilityReason {
                breaking: false,
                path: "interface".to_string(),
                message:
                    "baseline interface is empty; there is no public surface to compare against"
                        .to_string(),
            }],
            diff,
        };
    }

    let mut reasons = Vec::new();
    let mut incompatible = false;
    for change in &diff.changes {
        if change.severity == Severity::Informational {
            continue;
        }
        let breaking = policy.is_breaking(change.kind);
        incompatible |= breaking;
        reasons.push(CompatibilityReason {
            breaking,
            path: change.path.clone(),
            message: change.detail.clone(),
        });
    }

    let status = if incompatible {
        CompatibilityStatus::Incompatible
    } else {
        CompatibilityStatus::Compatible
    };

    CompatibilityAssessment {
        status,
        policy,
        reasons,
        diff,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use observatory_spec::normalize;
    use observatory_testutil::spec::{spec_event, spec_function, spec_struct};
    use stellar_xdr::{ScSpecEntry, ScSpecTypeDef};

    fn interface(entries: Vec<ScSpecEntry>) -> ContractInterface {
        ContractInterface::from_spec(normalize(entries))
    }

    #[test]
    fn removed_function_is_incompatible() {
        let old = interface(vec![
            spec_function("a", &[], None),
            spec_function("b", &[], None),
        ]);
        let new = interface(vec![spec_function("a", &[], None)]);
        let assessment = assess(&old, &new, CompatibilityPolicy::default());
        assert_eq!(assessment.status, CompatibilityStatus::Incompatible);
        assert!(assessment.is_incompatible());
        assert!(assessment
            .reasons
            .iter()
            .any(|r| r.breaking && r.path == "function::b"));
    }

    #[test]
    fn added_function_is_compatible() {
        let old = interface(vec![spec_function("a", &[], None)]);
        let new = interface(vec![
            spec_function("a", &[], None),
            spec_function("b", &[], None),
        ]);
        let assessment = assess(&old, &new, CompatibilityPolicy::default());
        assert_eq!(assessment.status, CompatibilityStatus::Compatible);
    }

    #[test]
    fn changed_signature_is_incompatible() {
        let old = interface(vec![spec_function("f", &[("x", ScSpecTypeDef::U32)], None)]);
        let new = interface(vec![spec_function("f", &[("x", ScSpecTypeDef::U64)], None)]);
        let assessment = assess(&old, &new, CompatibilityPolicy::default());
        assert_eq!(assessment.status, CompatibilityStatus::Incompatible);
    }

    #[test]
    fn type_change_is_incompatible() {
        let old = interface(vec![spec_struct("S", &[("a", ScSpecTypeDef::U32)])]);
        let new = interface(vec![spec_struct("S", &[("a", ScSpecTypeDef::U64)])]);
        assert_eq!(
            assess(&old, &new, CompatibilityPolicy::default()).status,
            CompatibilityStatus::Incompatible
        );
    }

    #[test]
    fn lenient_policy_accepts_removals_as_notes() {
        let old = interface(vec![
            spec_function("a", &[], None),
            spec_event("e", &[], &[]),
        ]);
        let new = interface(vec![spec_function("a", &[], None)]);
        let assessment = assess(&old, &new, CompatibilityPolicy::lenient());
        assert_eq!(assessment.status, CompatibilityStatus::Compatible);
        assert!(assessment
            .reasons
            .iter()
            .any(|r| !r.breaking && r.path == "event::e"));
    }

    #[test]
    fn empty_baseline_is_unknown() {
        let new = interface(vec![spec_function("a", &[], None)]);
        let assessment = assess(
            &ContractInterface::from_spec(Default::default()),
            &new,
            CompatibilityPolicy::default(),
        );
        assert_eq!(assessment.status, CompatibilityStatus::Unknown);
    }

    #[test]
    fn assessment_is_deterministic() {
        let old = interface(vec![
            spec_function("b", &[], None),
            spec_function("a", &[], None),
        ]);
        let new = interface(vec![spec_function("c", &[], None)]);
        let first = assess(&old, &new, CompatibilityPolicy::default());
        let second = assess(&old, &new, CompatibilityPolicy::default());
        assert_eq!(first, second);
    }
}
