//! Hashing helpers.
//!
//! Deterministic fingerprints use SHA-256. This is an artifact/interface
//! identity primitive, not a signature scheme.

use sha2::{Digest, Sha256};

/// Return the lowercase hex SHA-256 digest of `bytes`.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vector_empty() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn known_vector_abc() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn different_inputs_differ() {
        assert_ne!(sha256_hex(b"a"), sha256_hex(b"b"));
    }
}
