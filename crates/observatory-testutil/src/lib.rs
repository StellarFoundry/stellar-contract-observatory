//! Deterministic fixture builders shared by Observatory test suites.
//!
//! Fixtures are constructed from bytes rather than checked-in binaries so tests
//! are reproducible and reviewable. WASM custom sections are assembled by hand
//! because the contract specification lives in a `contractspecv0` custom
//! section carrying XDR-encoded entries.

use stellar_xdr::{Limits, ScSpecEntry, WriteXdr};

pub mod spec;

/// The WASM 1.0 module header.
pub const WASM_HEADER: [u8; 8] = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];

/// The Soroban contract specification custom section name.
pub const CONTRACT_SPEC_SECTION: &str = "contractspecv0";

/// Encode an unsigned integer as LEB128 (the WASM variable-length integer).
#[must_use]
pub fn leb128(mut value: u64) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
    out
}

/// Build a WASM custom section with the given name and payload.
#[must_use]
pub fn custom_section(name: &str, payload: &[u8]) -> Vec<u8> {
    let mut contents = Vec::new();
    contents.extend(leb128(name.len() as u64));
    contents.extend_from_slice(name.as_bytes());
    contents.extend_from_slice(payload);
    let mut out = vec![0x00];
    out.extend(leb128(contents.len() as u64));
    out.extend(contents);
    out
}

/// A minimal, valid, empty WASM module.
#[must_use]
pub fn minimal_module() -> Vec<u8> {
    WASM_HEADER.to_vec()
}

/// A WASM module containing a single custom section.
#[must_use]
pub fn module_with_custom_section(name: &str, payload: &[u8]) -> Vec<u8> {
    let mut out = minimal_module();
    out.extend(custom_section(name, payload));
    out
}

/// Encode a sequence of contract specification entries as a raw XDR stream.
#[must_use]
pub fn encode_spec_entries(entries: &[ScSpecEntry]) -> Vec<u8> {
    let mut out = Vec::new();
    for entry in entries {
        let bytes = entry.to_xdr(Limits::none()).expect("encode spec entry");
        out.extend(bytes);
    }
    out
}

/// A WASM module containing a `contractspecv0` section for `entries`.
#[must_use]
pub fn module_with_spec(entries: &[ScSpecEntry]) -> Vec<u8> {
    module_with_custom_section(CONTRACT_SPEC_SECTION, &encode_spec_entries(entries))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leb128_encodes_known_values() {
        assert_eq!(leb128(0), vec![0x00]);
        assert_eq!(leb128(127), vec![0x7f]);
        assert_eq!(leb128(128), vec![0x80, 0x01]);
        assert_eq!(leb128(624485), vec![0xe5, 0x8e, 0x26]);
    }

    #[test]
    fn custom_section_has_correct_shape() {
        let section = custom_section("hi", &[1, 2, 3]);
        assert_eq!(section[0], 0x00);
        assert_eq!(section[1] as usize, 1 + 2 + 3);
        assert_eq!(section[2], 2);
        assert_eq!(&section[3..5], b"hi");
        assert_eq!(&section[5..], &[1, 2, 3]);
    }

    #[test]
    fn minimal_module_is_eight_bytes() {
        assert_eq!(minimal_module(), WASM_HEADER);
    }
}
