//! End-to-end CLI tests. These spawn the built binary and exercise the full
//! pipeline against deterministic fixtures; no network access is required.

use std::io::Write;

use assert_cmd::Command;
use observatory_testutil::spec::{spec_function, spec_struct, ty_udt};
use observatory_testutil::{minimal_module, module_with_spec};
use predicates::prelude::*;
use stellar_xdr::{ScSpecEntry, ScSpecTypeDef};

fn binary() -> Command {
    Command::cargo_bin("stellar-contract-observatory").expect("binary builds")
}

fn write_temp(bytes: &[u8], suffix: &str) -> tempfile::NamedTempFile {
    let mut file = tempfile::Builder::new()
        .suffix(suffix)
        .tempfile()
        .expect("create temp file");
    file.write_all(bytes).expect("write temp file");
    file.flush().expect("flush temp file");
    file
}

fn contract_entries() -> Vec<ScSpecEntry> {
    vec![
        spec_function(
            "transfer",
            &[("to", ty_udt("Recipient")), ("amount", ScSpecTypeDef::I128)],
            Some(ScSpecTypeDef::Bool),
        ),
        spec_struct("Recipient", &[("address", ScSpecTypeDef::Address)]),
    ]
}

#[test]
fn inspect_reports_contract_spec() {
    let file = write_temp(&module_with_spec(&contract_entries()), ".wasm");
    binary()
        .args(["inspect", file.path().to_str().unwrap(), "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"has_contract_spec\": true"))
        .stdout(predicate::str::contains("\"schema_version\""));
}

#[test]
fn inspect_security_emits_audit() {
    let file = write_temp(&module_with_spec(&contract_entries()), ".wasm");
    binary()
        .args([
            "inspect",
            file.path().to_str().unwrap(),
            "--security",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"audit\""));
}

#[test]
fn spec_lists_functions() {
    let file = write_temp(&module_with_spec(&contract_entries()), ".wasm");
    binary()
        .args(["spec", file.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("transfer"));
}

#[test]
fn fingerprint_is_sha256() {
    let file = write_temp(&minimal_module(), ".wasm");
    binary()
        .args(["fingerprint", file.path().to_str().unwrap(), "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("artifact_sha256"))
        .stdout(predicate::str::contains("interface_sha256"));
}

#[test]
fn diff_and_compat_report_breaking_change() {
    let old = write_temp(
        &module_with_spec(&[spec_function("a", &[], None), spec_function("b", &[], None)]),
        ".wasm",
    );
    let new = write_temp(&module_with_spec(&[spec_function("a", &[], None)]), ".wasm");

    binary()
        .args([
            "diff",
            old.path().to_str().unwrap(),
            new.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("function::b"))
        .stdout(predicate::str::contains("breaking"));

    binary()
        .args([
            "compat",
            old.path().to_str().unwrap(),
            new.path().to_str().unwrap(),
        ])
        .assert()
        .code(6)
        .stdout(predicate::str::contains("incompatible"));
}

#[test]
fn events_filters_fixture() {
    use stellar_xdr::{Limits, ScSymbol, StringM, WriteXdr};
    let topic = stellar_xdr::ScVal::Symbol(ScSymbol(StringM::try_from("transfer").unwrap()))
        .to_xdr_base64(Limits::none())
        .unwrap();
    let fixture = format!(
        r#"{{"events":[{{"contractId":"CAAA","type":"contract","topics":["{topic}"],"data":null}}]}}"#
    );
    let file = write_temp(fixture.as_bytes(), ".json");
    binary()
        .args([
            "events",
            file.path().to_str().unwrap(),
            "--topic",
            "transfer",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"returned\": 1"));
}

#[test]
fn doctor_reports_capabilities() {
    binary()
        .args(["doctor", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("fingerprint"))
        .stdout(predicate::str::contains("fixture/mock only"));
}

#[test]
fn missing_file_is_an_input_error() {
    binary()
        .args(["inspect", "definitely-missing.wasm"])
        .assert()
        .code(3);
}

#[test]
fn invalid_arguments_are_a_usage_error() {
    binary().arg("not-a-command").assert().code(2);
}

#[test]
fn api_openapi_documents_contract_routes() {
    binary()
        .args(["api", "openapi"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"openapi\""))
        .stdout(predicate::str::contains("/api/v1/contracts/inspect"))
        .stdout(predicate::str::contains("securitySchemes"));
}

#[test]
fn format_json_matches_the_json_flag() {
    binary()
        .args(["doctor", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"schema_version\""));
}

#[test]
fn verbose_writes_diagnostics_to_stderr_and_quiet_suppresses_them() {
    binary()
        .args(["doctor", "--verbose"])
        .assert()
        .success()
        .stderr(predicate::str::contains("running `doctor`"));
    binary()
        .args(["doctor"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

fn instance_xdr(hash: [u8; 32]) -> String {
    use stellar_xdr::{
        ContractDataDurability, ContractDataEntry, ContractExecutable, ContractId, ExtensionPoint,
        Hash, LedgerEntryData, Limits, ScAddress, ScContractInstance, ScVal, WriteXdr,
    };
    let data = LedgerEntryData::ContractData(ContractDataEntry {
        ext: ExtensionPoint::V0,
        contract: ScAddress::Contract(ContractId(Hash([9u8; 32]))),
        key: ScVal::LedgerKeyContractInstance,
        durability: ContractDataDurability::Persistent,
        val: ScVal::ContractInstance(ScContractInstance {
            executable: ContractExecutable::Wasm(Hash(hash)),
            storage: None,
        }),
    });
    data.to_xdr_base64(Limits::none()).unwrap()
}

fn code_xdr(wasm: &[u8], hash: [u8; 32]) -> String {
    use stellar_xdr::{
        ContractCodeCostInputs, ContractCodeEntry, ContractCodeEntryExt, ContractCodeEntryV1,
        ExtensionPoint, Hash, LedgerEntryData, Limits, WriteXdr,
    };
    let data = LedgerEntryData::ContractCode(ContractCodeEntry {
        ext: ContractCodeEntryExt::V1(ContractCodeEntryV1 {
            ext: ExtensionPoint::V0,
            cost_inputs: ContractCodeCostInputs {
                ext: ExtensionPoint::V0,
                n_instructions: 1,
                n_functions: 1,
                n_globals: 0,
                n_table_entries: 0,
                n_types: 0,
                n_data_segments: 0,
                n_elem_segments: 0,
                n_imports: 0,
                n_exports: 1,
                n_data_segment_bytes: 0,
            },
        }),
        hash: Hash(hash),
        code: wasm.to_vec().try_into().unwrap(),
    });
    data.to_xdr_base64(Limits::none()).unwrap()
}

#[test]
fn verify_interface_reports_a_match_against_a_fixture() {
    let wasm = module_with_spec(&[spec_function("hello", &[], Some(ScSpecTypeDef::Bool))]);
    let contract_id = format!("{}", stellar_strkey::Contract([9u8; 32]));
    let fixture = format!(
        r#"{{"getLedgerEntries":{{"__sequence":[{{"entries":[{{"key":"","xdr":"{}"}}]}},{{"entries":[{{"key":"","xdr":"{}"}}]}}]}}}}"#,
        instance_xdr([7u8; 32]),
        code_xdr(&wasm, [7u8; 32])
    );
    let wasm_file = write_temp(&wasm, ".wasm");
    let fixture_file = write_temp(fixture.as_bytes(), ".json");
    binary()
        .args([
            "verify",
            "interface",
            wasm_file.path().to_str().unwrap(),
            "--contract",
            &contract_id,
            "--rpc-fixture",
            fixture_file.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"interface_match\": true"));
}
