//! API-key authentication.
//!
//! Keys are generated with OS randomness and stored only as SHA-256 hashes. The
//! plaintext key is returned exactly once, at creation. The record never
//! serializes the hash, so a leaked record cannot be replayed.

use std::collections::BTreeMap;

use observatory_core::hash::sha256_hex;
use observatory_core::{ObservatoryError, Result};
use serde::Serialize;

use crate::rbac::Role;

/// Prefix identifying an Observatory API key.
pub const KEY_PREFIX: &str = "sco";

/// A stored API-key record. The secret hash is never serialized.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ApiKeyRecord {
    /// Public key identifier (also present in the plaintext key).
    pub id: String,
    /// SHA-256 of the secret. Excluded from serialization.
    #[serde(skip)]
    pub hash: String,
    /// The role granted to the key.
    pub role: Role,
    /// A human label.
    pub label: String,
    /// Whether the key has been revoked.
    pub revoked: bool,
}

/// The authenticated identity behind a request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Principal {
    /// The API-key id.
    pub key_id: String,
    /// The granted role.
    pub role: Role,
}

/// An in-memory API-key store.
#[derive(Debug, Default)]
pub struct ApiKeyStore {
    keys: BTreeMap<String, ApiKeyRecord>,
}

impl ApiKeyStore {
    /// Create an empty store.
    #[must_use]
    pub fn new() -> Self {
        ApiKeyStore::default()
    }

    /// Generate a new key for `role`. Returns `(plaintext, record)`.
    ///
    /// The plaintext is shown once and never stored.
    pub fn generate(&mut self, role: Role, label: &str) -> Result<(String, ApiKeyRecord)> {
        let mut secret = [0u8; 32];
        getrandom::getrandom(&mut secret)
            .map_err(|error| ObservatoryError::internal(format!("randomness failure: {error}")))?;
        let secret_hex = hex::encode(secret);
        let id = sha256_hex(format!("{label}:{secret_hex}").as_bytes())[..16].to_string();
        let plaintext = format!("{KEY_PREFIX}_{id}_{secret_hex}");
        let record = ApiKeyRecord {
            id: id.clone(),
            hash: sha256_hex(secret_hex.as_bytes()),
            role,
            label: label.to_string(),
            revoked: false,
        };
        self.keys.insert(id, record.clone());
        Ok((plaintext, record))
    }

    /// Insert a record directly (for fixtures and imports).
    pub fn insert_hashed(
        &mut self,
        id: impl Into<String>,
        hash: impl Into<String>,
        role: Role,
        label: &str,
    ) -> ApiKeyRecord {
        let record = ApiKeyRecord {
            id: id.into(),
            hash: hash.into(),
            role,
            label: label.to_string(),
            revoked: false,
        };
        self.keys.insert(record.id.clone(), record.clone());
        record
    }

    /// Register a key from its plaintext form (`sco_<id>_<secret>`).
    ///
    /// Only the hash is stored. This is how a server is configured with keys
    /// without a persistence layer.
    pub fn insert_plaintext(
        &mut self,
        plaintext: &str,
        role: Role,
        label: &str,
    ) -> Result<ApiKeyRecord> {
        let mut parts = plaintext.trim().splitn(3, '_');
        if parts.next() != Some(KEY_PREFIX) {
            return Err(ObservatoryError::invalid("API key must start with `sco_`"));
        }
        let id = parts
            .next()
            .ok_or_else(|| ObservatoryError::invalid("API key is missing its id"))?
            .to_string();
        let secret = parts
            .next()
            .ok_or_else(|| ObservatoryError::invalid("API key is missing its secret"))?;
        let record = ApiKeyRecord {
            id: id.clone(),
            hash: sha256_hex(secret.as_bytes()),
            role,
            label: label.to_string(),
            revoked: false,
        };
        self.keys.insert(id, record.clone());
        Ok(record)
    }

    /// Revoke a key by id. Returns whether a key was revoked.
    pub fn revoke(&mut self, id: &str) -> bool {
        match self.keys.get_mut(id) {
            Some(record) => {
                record.revoked = true;
                true
            }
            None => false,
        }
    }

    /// The number of stored keys.
    #[must_use]
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Whether the store is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Authenticate a presented key, returning the principal on success.
    #[must_use]
    pub fn authenticate(&self, provided: &str) -> Option<Principal> {
        let mut parts = provided.trim().splitn(3, '_');
        if parts.next()? != KEY_PREFIX {
            return None;
        }
        let id = parts.next()?;
        let secret = parts.next()?;
        let record = self.keys.get(id)?;
        if record.revoked {
            return None;
        }
        if sha256_hex(secret.as_bytes()) != record.hash {
            return None;
        }
        Some(Principal {
            key_id: record.id.clone(),
            role: record.role,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_keys_authenticate() {
        let mut store = ApiKeyStore::new();
        let (plaintext, record) = store.generate(Role::Developer, "ci").unwrap();
        assert!(plaintext.starts_with("sco_"));
        let principal = store.authenticate(&plaintext).unwrap();
        assert_eq!(principal.key_id, record.id);
        assert_eq!(principal.role, Role::Developer);
    }

    #[test]
    fn wrong_secret_and_prefix_fail() {
        let mut store = ApiKeyStore::new();
        let (_, record) = store.generate(Role::Viewer, "x").unwrap();
        assert!(store.authenticate("sco_wrong_deadbeef").is_none());
        assert!(store.authenticate("nope").is_none());
        let forged = format!("sco_{}_00", record.id);
        assert!(store.authenticate(&forged).is_none());
    }

    #[test]
    fn revoked_keys_fail() {
        let mut store = ApiKeyStore::new();
        let (plaintext, record) = store.generate(Role::Admin, "ops").unwrap();
        assert!(store.revoke(&record.id));
        assert!(store.authenticate(&plaintext).is_none());
        assert!(!store.revoke("missing"));
    }

    #[test]
    fn records_never_serialize_the_hash() {
        let mut store = ApiKeyStore::new();
        let (_, record) = store.generate(Role::Viewer, "x").unwrap();
        let json = serde_json::to_string(&record).unwrap();
        assert!(!json.contains(&record.hash));
        assert!(!json.contains("hash"));
    }
}
