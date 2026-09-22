//! Role-based access control.
//!
//! Authorization is deliberately separate from authentication: a [`Principal`]
//! proves *who* is calling, and this module answers *whether* they may perform a
//! [`Permission`]. Roles are hierarchical: `admin ⊃ maintainer ⊃ developer ⊃
//! viewer`.

use serde::{Deserialize, Serialize};

/// A platform role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Read-only access to health, readiness, and version.
    Viewer,
    /// Viewer access plus running analyses.
    Developer,
    /// Developer access plus webhook management.
    Maintainer,
    /// Maintainer access plus API-key management.
    Admin,
}

impl Role {
    /// The stable lowercase name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Role::Viewer => "viewer",
            Role::Developer => "developer",
            Role::Maintainer => "maintainer",
            Role::Admin => "admin",
        }
    }

    /// The permissions granted to this role.
    #[must_use]
    pub fn permissions(self) -> &'static [Permission] {
        match self {
            Role::Viewer => &[Permission::ReadHealth, Permission::ReadVersion],
            Role::Developer => &[
                Permission::ReadHealth,
                Permission::ReadVersion,
                Permission::RunAnalysis,
            ],
            Role::Maintainer => &[
                Permission::ReadHealth,
                Permission::ReadVersion,
                Permission::RunAnalysis,
                Permission::ManageWebhooks,
            ],
            Role::Admin => &[
                Permission::ReadHealth,
                Permission::ReadVersion,
                Permission::RunAnalysis,
                Permission::ManageWebhooks,
                Permission::ManageKeys,
            ],
        }
    }
}

impl std::str::FromStr for Role {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "viewer" => Ok(Role::Viewer),
            "developer" => Ok(Role::Developer),
            "maintainer" => Ok(Role::Maintainer),
            "admin" => Ok(Role::Admin),
            other => Err(format!(
                "unknown role `{other}`; expected viewer, developer, maintainer, or admin"
            )),
        }
    }
}

/// A discrete action that can be authorized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Permission {
    /// Read health and readiness.
    ReadHealth,
    /// Read version information.
    ReadVersion,
    /// Run contract analysis.
    RunAnalysis,
    /// Manage webhook registrations.
    ManageWebhooks,
    /// Manage API keys.
    ManageKeys,
}

impl Permission {
    /// The stable name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Permission::ReadHealth => "read_health",
            Permission::ReadVersion => "read_version",
            Permission::RunAnalysis => "run_analysis",
            Permission::ManageWebhooks => "manage_webhooks",
            Permission::ManageKeys => "manage_keys",
        }
    }
}

/// Whether `role` may exercise `permission`.
#[must_use]
pub fn authorize(role: Role, permission: Permission) -> bool {
    role.permissions().contains(&permission)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_are_hierarchical() {
        assert!(authorize(Role::Viewer, Permission::ReadVersion));
        assert!(!authorize(Role::Viewer, Permission::RunAnalysis));
        assert!(authorize(Role::Developer, Permission::RunAnalysis));
        assert!(!authorize(Role::Developer, Permission::ManageKeys));
        assert!(authorize(Role::Maintainer, Permission::ManageWebhooks));
        assert!(authorize(Role::Admin, Permission::ManageKeys));
    }

    #[test]
    fn higher_roles_include_lower_permissions() {
        for permission in [
            Permission::ReadHealth,
            Permission::ReadVersion,
            Permission::RunAnalysis,
        ] {
            assert!(authorize(Role::Admin, permission));
            assert!(authorize(Role::Developer, permission));
        }
    }

    #[test]
    fn names_are_stable() {
        assert_eq!(Role::Developer.name(), "developer");
        assert_eq!(Permission::RunAnalysis.name(), "run_analysis");
    }
}
