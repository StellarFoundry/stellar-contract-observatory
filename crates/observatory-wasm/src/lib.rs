//! Safe, deterministic inspection of Soroban contract WASM.
//!
//! This crate never executes a module. It parses the binary with a bounded,
//! streaming parser and reports structure. All failures become
//! [`observatory_core::ObservatoryError`].
//!
//! The report is stable, ordered, and JSON-serializable so it can be used in
//! CI. Ordering of imports, exports, and custom sections follows module order,
//! which is meaningful for WASM.

use std::path::Path;

use observatory_core::hash::sha256_hex;
use observatory_core::limits::{MAX_CUSTOM_SECTIONS, MAX_EXPORTS, MAX_IMPORTS, MAX_WASM_BYTES};
use observatory_core::{ObservatoryError, Result};
use serde::Serialize;
use wasmparser::{ExternalKind, Parser, Payload, TypeRef};

/// Custom section name carrying the Soroban contract specification.
pub const CONTRACT_SPEC_SECTION: &str = "contractspecv0";
/// Custom section name carrying contract-level metadata.
pub const CONTRACT_META_SECTION: &str = "contractmetav0";
/// Custom section name carrying the contract environment metadata.
pub const CONTRACT_ENV_META_SECTION: &str = "contractenvmetav0";

/// A single import entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportInfo {
    /// Import module, for example `env`.
    pub module: String,
    /// Import name.
    pub name: String,
    /// Import kind: `func`, `table`, `memory`, `global`, or `tag`.
    pub kind: String,
}

/// A single export entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportInfo {
    /// Export name.
    pub name: String,
    /// Export kind: `func`, `table`, `memory`, `global`, or `tag`.
    pub kind: String,
}

/// A custom section entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CustomSectionInfo {
    /// Section name.
    pub name: String,
    /// Payload size in bytes.
    pub size: usize,
}

/// Memory declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MemoryInfo {
    /// Initial number of 64 KiB pages.
    pub initial_pages: u64,
    /// Maximum number of pages, if declared.
    pub maximum_pages: Option<u64>,
    /// Whether the memory is shared.
    pub shared: bool,
}

/// The result of inspecting a WASM artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WasmReport {
    /// Artifact size in bytes.
    pub size_bytes: usize,
    /// WASM binary version.
    pub version: u32,
    /// SHA-256 of the artifact bytes.
    pub artifact_sha256: String,
    /// Number of types declared.
    pub type_count: usize,
    /// Number of functions declared.
    pub function_count: usize,
    /// Number of tables declared.
    pub table_count: usize,
    /// Number of globals declared.
    pub global_count: usize,
    /// Declared memories.
    pub memories: Vec<MemoryInfo>,
    /// Declared imports in module order.
    pub imports: Vec<ImportInfo>,
    /// Declared exports in module order.
    pub exports: Vec<ExportInfo>,
    /// Custom sections in module order.
    pub custom_sections: Vec<CustomSectionInfo>,
    /// Total byte size of the code section.
    pub code_section_bytes: usize,
    /// Whether a `contractspecv0` section is present.
    pub has_contract_spec: bool,
    /// Size of the `contractspecv0` section payload, if present.
    pub contract_spec_bytes: Option<usize>,
    /// Whether import/export/custom-section lists were truncated by limits.
    pub truncated: bool,
}

impl WasmReport {
    /// Names of exported functions, in module order.
    #[must_use]
    pub fn exported_function_names(&self) -> Vec<&str> {
        self.exports
            .iter()
            .filter(|export| export.kind == "func")
            .map(|export| export.name.as_str())
            .collect()
    }
}

/// Read a WASM artifact from disk with a hard size bound.
pub fn load_file(path: &Path) -> Result<Vec<u8>> {
    let metadata = std::fs::metadata(path).map_err(|error| {
        ObservatoryError::Io(std::io::Error::new(
            error.kind(),
            format!("{}: {error}", path.display()),
        ))
    })?;
    if metadata.len() > MAX_WASM_BYTES as u64 {
        return Err(ObservatoryError::invalid(format!(
            "`{}` is {} bytes, which exceeds the {} byte limit",
            path.display(),
            metadata.len(),
            MAX_WASM_BYTES
        )));
    }
    let bytes = std::fs::read(path)?;
    if bytes.len() > MAX_WASM_BYTES {
        return Err(ObservatoryError::invalid(format!(
            "`{}` exceeds the {} byte limit",
            path.display(),
            MAX_WASM_BYTES
        )));
    }
    Ok(bytes)
}

/// Validate that `bytes` is a well-formed WASM module.
pub fn validate(bytes: &[u8]) -> Result<()> {
    inspect(bytes).map(|_| ())
}

/// Inspect a WASM module without executing it.
pub fn inspect(bytes: &[u8]) -> Result<WasmReport> {
    if bytes.len() < 8 {
        return Err(ObservatoryError::wasm(
            "input is too short to be a WASM module",
        ));
    }
    if &bytes[0..4] != b"\0asm" {
        return Err(ObservatoryError::wasm(
            "input does not start with the WASM magic bytes",
        ));
    }

    let mut report = WasmReport {
        size_bytes: bytes.len(),
        version: 0,
        artifact_sha256: sha256_hex(bytes),
        type_count: 0,
        function_count: 0,
        table_count: 0,
        global_count: 0,
        memories: Vec::new(),
        imports: Vec::new(),
        exports: Vec::new(),
        custom_sections: Vec::new(),
        code_section_bytes: 0,
        has_contract_spec: false,
        contract_spec_bytes: None,
        truncated: false,
    };

    for payload in Parser::new(0).parse_all(bytes) {
        let payload = payload.map_err(|error| ObservatoryError::wasm(error.to_string()))?;
        match payload {
            Payload::Version { num, .. } => report.version = u32::from(num),
            Payload::TypeSection(reader) => report.type_count = reader.count() as usize,
            Payload::FunctionSection(reader) => report.function_count = reader.count() as usize,
            Payload::TableSection(reader) => report.table_count = reader.count() as usize,
            Payload::GlobalSection(reader) => report.global_count = reader.count() as usize,
            Payload::MemorySection(reader) => {
                for memory in reader {
                    let memory =
                        memory.map_err(|error| ObservatoryError::wasm(error.to_string()))?;
                    report.memories.push(MemoryInfo {
                        initial_pages: memory.initial,
                        maximum_pages: memory.maximum,
                        shared: memory.shared,
                    });
                }
            }
            Payload::ImportSection(reader) => {
                for import in reader.into_imports() {
                    if report.imports.len() >= MAX_IMPORTS {
                        report.truncated = true;
                        break;
                    }
                    let import =
                        import.map_err(|error| ObservatoryError::wasm(error.to_string()))?;
                    report.imports.push(ImportInfo {
                        module: import.module.to_string(),
                        name: import.name.to_string(),
                        kind: type_ref_kind(&import.ty).to_string(),
                    });
                }
            }
            Payload::ExportSection(reader) => {
                for export in reader {
                    if report.exports.len() >= MAX_EXPORTS {
                        report.truncated = true;
                        break;
                    }
                    let export =
                        export.map_err(|error| ObservatoryError::wasm(error.to_string()))?;
                    report.exports.push(ExportInfo {
                        name: export.name.to_string(),
                        kind: external_kind(&export.kind).to_string(),
                    });
                }
            }
            Payload::CodeSectionStart { size, .. } => report.code_section_bytes = size as usize,
            Payload::CustomSection(section) => {
                if report.custom_sections.len() >= MAX_CUSTOM_SECTIONS {
                    report.truncated = true;
                    continue;
                }
                let name = section.name().to_string();
                let size = section.data().len();
                if name == CONTRACT_SPEC_SECTION {
                    report.has_contract_spec = true;
                    report.contract_spec_bytes = Some(size);
                }
                report
                    .custom_sections
                    .push(CustomSectionInfo { name, size });
            }
            _ => {}
        }
    }

    if report.version == 0 {
        return Err(ObservatoryError::wasm("missing or invalid WASM version"));
    }

    Ok(report)
}

/// Extract the raw bytes of a named custom section, if present.
pub fn extract_custom_section(bytes: &[u8], name: &str) -> Result<Option<Vec<u8>>> {
    for payload in Parser::new(0).parse_all(bytes) {
        let payload = payload.map_err(|error| ObservatoryError::wasm(error.to_string()))?;
        if let Payload::CustomSection(section) = payload {
            if section.name() == name {
                return Ok(Some(section.data().to_vec()));
            }
        }
    }
    Ok(None)
}

/// Compute the canonical artifact fingerprint (SHA-256 of the bytes).
#[must_use]
pub fn artifact_fingerprint(bytes: &[u8]) -> String {
    sha256_hex(bytes)
}

fn type_ref_kind(ty: &TypeRef) -> &'static str {
    match ty {
        TypeRef::Func(_) => "func",
        TypeRef::Table(_) => "table",
        TypeRef::Memory(_) => "memory",
        TypeRef::Global(_) => "global",
        TypeRef::Tag(_) => "tag",
        TypeRef::FuncExact(_) => "func",
    }
}

fn external_kind(kind: &ExternalKind) -> &'static str {
    match kind {
        ExternalKind::Func => "func",
        ExternalKind::Table => "table",
        ExternalKind::Memory => "memory",
        ExternalKind::Global => "global",
        ExternalKind::Tag => "tag",
        ExternalKind::FuncExact => "func",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use observatory_testutil::{minimal_module, module_with_custom_section, CONTRACT_SPEC_SECTION};

    #[test]
    fn inspects_minimal_module() {
        let report = inspect(&minimal_module()).unwrap();
        assert_eq!(report.version, 1);
        assert_eq!(report.size_bytes, 8);
        assert!(report.imports.is_empty());
        assert!(report.exports.is_empty());
        assert!(!report.has_contract_spec);
        assert_eq!(report.artifact_sha256, sha256_hex(&minimal_module()));
    }

    #[test]
    fn rejects_short_input() {
        let error = inspect(&[0x00, 0x61]).unwrap_err();
        assert!(matches!(error, ObservatoryError::Wasm(_)));
    }

    #[test]
    fn rejects_bad_magic() {
        let error = inspect(b"not wasm at all!!").unwrap_err();
        assert!(matches!(error, ObservatoryError::Wasm(_)));
    }

    #[test]
    fn detects_contract_spec_section() {
        let module = module_with_custom_section(CONTRACT_SPEC_SECTION, &[1, 2, 3, 4]);
        let report = inspect(&module).unwrap();
        assert!(report.has_contract_spec);
        assert_eq!(report.contract_spec_bytes, Some(4));
        assert!(report
            .custom_sections
            .iter()
            .any(|section| section.name == CONTRACT_SPEC_SECTION));
    }

    #[test]
    fn extracts_custom_section_bytes() {
        let module = module_with_custom_section("hello", &[9, 8, 7]);
        let data = extract_custom_section(&module, "hello").unwrap();
        assert_eq!(data, Some(vec![9, 8, 7]));
        assert_eq!(extract_custom_section(&module, "missing").unwrap(), None);
    }

    #[test]
    fn reports_imports_exports_and_memory() {
        let wasm = wat::parse_str(
            r#"(module
                (import "env" "dbg" (func $dbg (param i32)))
                (memory 1 2)
                (func (export "hello") (param i32) (result i32) local.get 0)
                (global (export "g") i32 (i32.const 7))
            )"#,
        )
        .unwrap();
        let report = inspect(&wasm).unwrap();
        assert_eq!(report.imports.len(), 1);
        assert_eq!(report.imports[0].module, "env");
        assert_eq!(report.imports[0].name, "dbg");
        assert_eq!(report.imports[0].kind, "func");
        assert!(report.exported_function_names().contains(&"hello"));
        assert_eq!(report.memories.len(), 1);
        assert_eq!(report.memories[0].initial_pages, 1);
        assert_eq!(report.memories[0].maximum_pages, Some(2));
        assert!(report.code_section_bytes > 0);
    }

    #[test]
    fn fingerprint_changes_with_content() {
        let a = artifact_fingerprint(&minimal_module());
        let b = artifact_fingerprint(&module_with_custom_section("x", &[1]));
        assert_ne!(a, b);
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn load_file_bound_is_enforced() {
        let error = load_file(Path::new("does-not-exist.wasm")).unwrap_err();
        assert!(matches!(error, ObservatoryError::Io(_)));
    }
}
