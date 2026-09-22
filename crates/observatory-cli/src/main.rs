//! `stellar-contract-observatory` command-line interface.
//!
//! Exit codes:
//! * `0` success
//! * `2` usage error
//! * `3` input or filesystem error
//! * `4` parse/decode error
//! * `5` unsupported feature
//! * `6` an interface change was incompatible
//! * `7` verification mismatch
//! * `8` verification was inconclusive

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use observatory_compat::{assess, CompatibilityPolicy, CompatibilityStatus};
use observatory_core::{
    ExitCode as ObservatoryExit, Network, ObservatoryError, OutputFormat, Result,
};
use observatory_diff::{diff, Severity};
use observatory_interface::ContractInterface;
use observatory_output::{report, ErrorReport};
use observatory_rpc::{Endpoint, MockTransport, RpcClient};
use observatory_verify::VerificationStatus;
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(
    name = "stellar-contract-observatory",
    version,
    about = "Contract intelligence, inspection, compatibility, and verification for Stellar/Soroban",
    long_about = None
)]
struct Cli {
    /// Emit machine-readable JSON.
    #[arg(long, global = true)]
    json: bool,

    /// Increase logging verbosity (repeatable).
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Suppress non-essential output.
    #[arg(short, long, global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Inspect a WASM contract artifact.
    Inspect {
        /// Path to a `.wasm` file.
        wasm: PathBuf,
        /// Also run bounded interface/artifact heuristics.
        #[arg(long)]
        security: bool,
    },
    /// Inspect the contract specification embedded in a WASM artifact.
    Spec {
        /// Path to a `.wasm` file.
        wasm: PathBuf,
        /// Show a single function by name.
        #[arg(long)]
        function: Option<String>,
        /// Show a single type by name.
        #[arg(long, value_name = "TYPE")]
        r#type: Option<String>,
    },
    /// Analyze contract events from a JSON fixture.
    Events {
        /// Path to an event fixture JSON file.
        file: PathBuf,
        /// Filter by contract id.
        #[arg(long)]
        contract: Option<String>,
        /// Filter by topic text.
        #[arg(long)]
        topic: Option<String>,
        /// Filter by event type.
        #[arg(long = "type")]
        event_type: Option<String>,
        /// Limit the number of returned events.
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Diff two contract interfaces.
    Diff {
        /// Baseline contract WASM.
        old: PathBuf,
        /// Updated contract WASM.
        new: PathBuf,
    },
    /// Assess compatibility of two contract interfaces.
    Compat {
        /// Baseline contract WASM.
        old: PathBuf,
        /// Updated contract WASM.
        new: PathBuf,
        /// Compatibility policy: `strict` or `lenient`.
        #[arg(long, default_value = "strict")]
        policy: String,
    },
    /// Compute deterministic fingerprints for a contract artifact.
    Fingerprint {
        /// Path to a `.wasm` file.
        wasm: PathBuf,
    },
    /// Inspect a deployed contract using a mocked RPC fixture.
    #[command(subcommand)]
    Deployment(DeploymentCommand),
    /// Verify a local artifact against a deployment, or compare build metadata.
    #[command(subcommand)]
    Verify(VerifyCommand),
    /// Run the REST API server or print its OpenAPI document.
    #[command(subcommand)]
    Api(ApiCommand),
    /// Report environment and capabilities.
    Doctor,
}

#[derive(Debug, Subcommand)]
enum DeploymentCommand {
    /// Inspect a deployed contract.
    Inspect {
        /// Contract id (C... strkey).
        contract: String,
        /// JSON fixture mapping RPC method to result.
        #[arg(long)]
        rpc_fixture: PathBuf,
        /// Network label.
        #[arg(long, default_value = "testnet")]
        network: String,
        /// RPC endpoint URL (recorded only in this build).
        #[arg(long)]
        rpc_url: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum VerifyCommand {
    /// Verify a local artifact against a deployed contract.
    Artifact {
        /// Path to a `.wasm` file.
        wasm: PathBuf,
        /// Contract id (C... strkey).
        #[arg(long)]
        contract: String,
        /// JSON fixture mapping RPC method to result.
        #[arg(long)]
        rpc_fixture: PathBuf,
        /// Network label.
        #[arg(long, default_value = "testnet")]
        network: String,
    },
    /// Parse and print a build metadata JSON document.
    Metadata {
        /// Path to a build metadata JSON file.
        file: PathBuf,
    },
    /// Compare two build metadata JSON documents.
    Compare {
        /// Left metadata JSON file.
        left: PathBuf,
        /// Right metadata JSON file.
        right: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum ApiCommand {
    /// Serve the REST API.
    Serve {
        /// Address to bind, for example 127.0.0.1:8080.
        #[arg(long, default_value = "127.0.0.1:8080")]
        bind: String,
        /// Disable authentication (local development only).
        #[arg(long)]
        no_auth: bool,
        /// Requests allowed per window.
        #[arg(long)]
        rate_limit: Option<u32>,
        /// Rate-limit window length in seconds.
        #[arg(long, default_value_t = 60)]
        window_secs: u64,
        /// Register a key as ROLE:PLAINTEXT (repeatable), for example developer:sco_...
        #[arg(long = "key")]
        keys: Vec<String>,
    },
    /// Print the OpenAPI document.
    Openapi,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let format = if cli.json {
        OutputFormat::Json
    } else {
        OutputFormat::Human
    };
    match run(&cli, format) {
        Ok(code) => ExitCode::from(code.code()),
        Err(error) => {
            emit_error(format, &error);
            ExitCode::from(error.exit_code().code())
        }
    }
}

fn run(cli: &Cli, format: OutputFormat) -> Result<ObservatoryExit> {
    match &cli.command {
        Command::Inspect { wasm, security } => {
            let bytes = observatory_wasm::load_file(wasm)?;
            let report = observatory_wasm::inspect(&bytes)?;
            let interface = ContractInterface::from_wasm(&bytes).ok();
            let mut audit = observatory_audit::audit_artifact(&report);
            if *security {
                if let Some(interface) = &interface {
                    audit = audit.merge(observatory_audit::audit_interface(interface));
                }
            }
            let payload = serde_json::json!({
                "wasm": report,
                "interface": interface,
                "audit": if *security { Some(audit) } else { None },
            });
            emit(format, "inspect", &payload, || print_inspect(&payload))?;
            Ok(ObservatoryExit::Success)
        }
        Command::Spec {
            wasm,
            function,
            r#type,
        } => {
            let bytes = observatory_wasm::load_file(wasm)?;
            let spec = observatory_spec::parse_wasm(&bytes)?;
            let summary = observatory_spec::summarize(&spec);
            if let Some(name) = function {
                let found = spec.function(name).cloned().ok_or_else(|| {
                    ObservatoryError::invalid(format!("no function named `{name}`"))
                })?;
                let payload = serde_json::json!({ "function": &found, "summary": &summary });
                let display = format!("function {}({})", found.name, render_params(&found.inputs));
                emit(format, "spec.function", &payload, || {
                    println!("{display}");
                })?;
                return Ok(ObservatoryExit::Success);
            }
            if let Some(name) = r#type {
                let payload = serde_json::json!({
                    "structs": spec.structs.iter().filter(|item| &item.name == name).collect::<Vec<_>>(),
                    "unions": spec.unions.iter().filter(|item| &item.name == name).collect::<Vec<_>>(),
                    "enums": spec.enums.iter().filter(|item| &item.name == name).collect::<Vec<_>>(),
                    "error_enums": spec.error_enums.iter().filter(|item| &item.name == name).collect::<Vec<_>>(),
                });
                let empty = payload["structs"].as_array().is_none_or(Vec::is_empty)
                    && payload["unions"].as_array().is_none_or(Vec::is_empty)
                    && payload["enums"].as_array().is_none_or(Vec::is_empty)
                    && payload["error_enums"].as_array().is_none_or(Vec::is_empty);
                if empty {
                    return Err(ObservatoryError::invalid(format!("no type named `{name}`")));
                }
                emit(format, "spec.type", &payload, || print_type(&payload))?;
                return Ok(ObservatoryExit::Success);
            }
            let payload = serde_json::json!({ "spec": &spec, "summary": &summary });
            emit(format, "spec", &payload, || print_spec(&spec, &summary))?;
            Ok(ObservatoryExit::Success)
        }
        Command::Events {
            file,
            contract,
            topic,
            event_type,
            limit,
        } => {
            let text = read_text(file)?;
            let events = observatory_events::parse_events_str(&text)?;
            let filtered = observatory_events::filter(
                &events,
                &observatory_events::EventFilter {
                    contract: contract.clone(),
                    topic: topic.clone(),
                    event_type: event_type.clone(),
                    limit: *limit,
                },
            );
            let payload = serde_json::json!({
                "total": events.len(),
                "returned": filtered.len(),
                "events": filtered,
            });
            emit(format, "events", &payload, || print_events(&payload))?;
            Ok(ObservatoryExit::Success)
        }
        Command::Diff { old, new } => {
            let old_interface = load_interface(old)?;
            let new_interface = load_interface(new)?;
            let result = diff(&old_interface, &new_interface);
            emit(format, "diff", &result, || print_diff(&result))?;
            Ok(ObservatoryExit::Success)
        }
        Command::Compat { old, new, policy } => {
            let policy = match policy.as_str() {
                "strict" => CompatibilityPolicy::strict(),
                "lenient" => CompatibilityPolicy::lenient(),
                other => {
                    return Err(ObservatoryError::invalid(format!(
                        "unknown policy `{other}`; expected strict or lenient"
                    )))
                }
            };
            let old_interface = load_interface(old)?;
            let new_interface = load_interface(new)?;
            let assessment = assess(&old_interface, &new_interface, policy);
            emit(format, "compat", &assessment, || print_compat(&assessment))?;
            Ok(match assessment.status {
                CompatibilityStatus::Incompatible => ObservatoryExit::Incompatible,
                _ => ObservatoryExit::Success,
            })
        }
        Command::Fingerprint { wasm } => {
            let bytes = observatory_wasm::load_file(wasm)?;
            let artifact = observatory_wasm::artifact_fingerprint(&bytes);
            let interface = ContractInterface::from_wasm(&bytes)
                .ok()
                .map(|interface| interface.fingerprint())
                .transpose()?;
            let payload = serde_json::json!({
                "artifact_sha256": artifact,
                "interface_sha256": interface,
            });
            emit(format, "fingerprint", &payload, || {
                print_fingerprint(&payload)
            })?;
            Ok(ObservatoryExit::Success)
        }
        Command::Deployment(DeploymentCommand::Inspect {
            contract,
            rpc_fixture,
            network,
            rpc_url,
        }) => {
            let client = fixture_client(rpc_fixture)?;
            let _ = network.parse::<Network>()?;
            if let Some(url) = rpc_url {
                let _ = Endpoint::parse(url)?;
            }
            let info = observatory_deployment::inspect(&client, contract)?;
            emit(format, "deployment", &info, || print_deployment(&info))?;
            Ok(ObservatoryExit::Success)
        }
        Command::Verify(VerifyCommand::Artifact {
            wasm,
            contract,
            rpc_fixture,
            network,
        }) => {
            let bytes = observatory_wasm::load_file(wasm)?;
            let client = fixture_client(rpc_fixture)?;
            let result =
                observatory_verify::verify_local_vs_deployed(&bytes, &client, contract, network)?;
            emit(format, "verify", &result, || print_verification(&result))?;
            Ok(match result.status {
                VerificationStatus::Match => ObservatoryExit::Success,
                VerificationStatus::Mismatch => ObservatoryExit::Mismatch,
                VerificationStatus::InsufficientData | VerificationStatus::Error => {
                    ObservatoryExit::InsufficientData
                }
            })
        }
        Command::Verify(VerifyCommand::Metadata { file }) => {
            let value = read_json(file)?;
            let metadata = observatory_verify::BuildMetadata::from_json(&value)?;
            emit(format, "verify.metadata", &metadata, || {
                for (key, value) in &metadata.entries {
                    println!("{key} = {value}");
                }
            })?;
            Ok(ObservatoryExit::Success)
        }
        Command::Verify(VerifyCommand::Compare { left, right }) => {
            let left = observatory_verify::BuildMetadata::from_json(&read_json(left)?)?;
            let right = observatory_verify::BuildMetadata::from_json(&read_json(right)?)?;
            let comparison = observatory_verify::compare_build_metadata(&left, &right);
            emit(format, "verify.compare", &comparison, || {
                println!("equal: {}", comparison.equal);
                for difference in &comparison.differences {
                    println!(
                        "  {}\n    left:  {}\n    right: {}",
                        difference.key,
                        difference.left.as_deref().unwrap_or("<none>"),
                        difference.right.as_deref().unwrap_or("<none>")
                    );
                }
            })?;
            Ok(ObservatoryExit::Success)
        }
        Command::Api(ApiCommand::Openapi) => {
            let document = observatory_api::openapi::document();
            let json = serde_json::to_string_pretty(&document)
                .map_err(|error| ObservatoryError::internal(error.to_string()))?;
            println!("{json}");
            Ok(ObservatoryExit::Success)
        }
        Command::Api(ApiCommand::Serve {
            bind,
            no_auth,
            rate_limit,
            window_secs,
            keys,
        }) => {
            let config = observatory_platform::PlatformConfig {
                require_auth: !no_auth,
                rate_limit: observatory_platform::RateLimitConfig {
                    limit: rate_limit.unwrap_or(120),
                    window_secs: *window_secs,
                },
            };
            let api = observatory_api::Api::new(config);
            for spec in keys {
                let (role, plaintext) = spec.split_once(':').ok_or_else(|| {
                    ObservatoryError::invalid(format!("--key must be ROLE:KEY, got `{spec}`"))
                })?;
                let role: observatory_platform::Role =
                    role.parse().map_err(ObservatoryError::invalid)?;
                api.insert_plaintext_key(plaintext, role, "cli")?;
            }
            let listener = std::net::TcpListener::bind(bind).map_err(ObservatoryError::Io)?;
            let addr = listener.local_addr().map_err(ObservatoryError::Io)?;
            if !cli.quiet {
                eprintln!("stellar-contract-observatory API listening on http://{addr}");
            }
            observatory_api::http::serve(listener, std::sync::Arc::new(api));
            Ok(ObservatoryExit::Success)
        }
        Command::Doctor => {
            let networks = Network::all()
                .iter()
                .map(|network| {
                    serde_json::json!({
                        "name": network.name(),
                        "passphrase": network.passphrase(),
                        "default_rpc_url": network.default_rpc_url(),
                    })
                })
                .collect::<Vec<_>>();
            let payload = serde_json::json!({
                "tool": observatory_core::TOOL_NAME,
                "version": env!("CARGO_PKG_VERSION"),
                "schema_version": observatory_core::SCHEMA_VERSION,
                "capabilities": [
                    "inspect", "spec", "events", "diff", "compat",
                    "fingerprint", "deployment", "verify", "doctor"
                ],
                "rpc_transport": "fixture/mock only (no live HTTP transport yet)",
                "networks": networks,
            });
            emit(format, "doctor", &payload, || {
                println!("{} {}", payload["tool"], payload["version"]);
                println!("rpc transport: {}", payload["rpc_transport"]);
            })?;
            Ok(ObservatoryExit::Success)
        }
    }
}

fn load_interface(path: &Path) -> Result<ContractInterface> {
    let bytes = observatory_wasm::load_file(path)?;
    ContractInterface::from_wasm(&bytes)
}

fn fixture_client(path: &Path) -> Result<RpcClient<MockTransport>> {
    let value = read_json(path)?;
    let transport = MockTransport::from_json(&value)?;
    let endpoint = Endpoint::parse("https://fixture.invalid")?;
    Ok(RpcClient::new(endpoint, transport))
}

fn read_text(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).map_err(|error| {
        ObservatoryError::invalid(format!("cannot read `{}`: {error}", path.display()))
    })
}

fn read_json(path: &Path) -> Result<serde_json::Value> {
    let text = read_text(path)?;
    serde_json::from_str(&text).map_err(|error| {
        ObservatoryError::decode(format!("invalid JSON in `{}`: {error}", path.display()))
    })
}

fn emit<T: Serialize>(
    format: OutputFormat,
    kind: &str,
    data: &T,
    human: impl FnOnce(),
) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", report(kind, data).to_json_pretty()?);
        }
        OutputFormat::Human => human(),
    }
    Ok(())
}

fn emit_error(format: OutputFormat, error: &ObservatoryError) {
    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                ErrorReport::from_error(error).to_json().unwrap_or_default()
            );
        }
        OutputFormat::Human => eprintln!("error: {error}"),
    }
}

fn render_params(params: &[observatory_spec::SpecParam]) -> String {
    params
        .iter()
        .map(|param| format!("{}: {}", param.name, param.ty))
        .collect::<Vec<_>>()
        .join(", ")
}

fn print_inspect(payload: &serde_json::Value) {
    let wasm = &payload["wasm"];
    println!(
        "artifact: {}",
        wasm["artifact_sha256"].as_str().unwrap_or_default()
    );
    println!(
        "size: {} bytes, version {}",
        wasm["size_bytes"], wasm["version"]
    );
    println!(
        "functions: {}, exports: {}, imports: {}, custom sections: {}",
        wasm["function_count"],
        wasm["exports"].as_array().map_or(0, Vec::len),
        wasm["imports"].as_array().map_or(0, Vec::len),
        wasm["custom_sections"].as_array().map_or(0, Vec::len)
    );
    println!(
        "contract spec: {}",
        if wasm["has_contract_spec"].as_bool().unwrap_or(false) {
            "present"
        } else {
            "absent"
        }
    );
    if let Some(audit) = payload.get("audit").filter(|value| !value.is_null()) {
        println!("audit: {}", audit["status"].as_str().unwrap_or("PASS"));
    }
}

fn print_spec(spec: &observatory_spec::ContractSpec, summary: &observatory_spec::SpecReport) {
    println!(
        "functions: {}, structs: {}, unions: {}, enums: {}, errors: {}, events: {}",
        summary.functions,
        summary.structs,
        summary.unions,
        summary.enums,
        summary.error_enums,
        summary.events
    );
    for function in &spec.functions {
        println!(
            "  fn {}({}) -> {}",
            function.name,
            render_params(&function.inputs),
            function
                .outputs
                .first()
                .map(ToString::to_string)
                .unwrap_or_else(|| "()".to_string())
        );
    }
}

fn print_type(payload: &serde_json::Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(payload).unwrap_or_default()
    );
}

fn print_events(payload: &serde_json::Value) {
    println!("events: {} of {}", payload["returned"], payload["total"]);
    if let Some(events) = payload["events"].as_array() {
        for event in events {
            println!(
                "  [{}] {} topics={}",
                event["ledger"],
                event["event_type"].as_str().unwrap_or(""),
                event["topics"].as_array().map_or(0, Vec::len)
            );
        }
    }
}

fn print_diff(result: &observatory_diff::InterfaceDiff) {
    println!(
        "changes: {} breaking, {} non-breaking, {} informational",
        result.summary.breaking, result.summary.non_breaking, result.summary.informational
    );
    for change in &result.changes {
        let marker = match change.severity {
            Severity::Breaking => "!",
            Severity::NonBreaking => "+",
            Severity::Informational => "~",
        };
        println!("{marker} {}: {}", change.path, change.detail);
    }
}

fn print_compat(assessment: &observatory_compat::CompatibilityAssessment) {
    let status = match assessment.status {
        CompatibilityStatus::Compatible => "compatible",
        CompatibilityStatus::Incompatible => "incompatible",
        CompatibilityStatus::Unknown => "unknown",
    };
    println!("status: {status}");
    for reason in &assessment.reasons {
        println!(
            "  {} {}",
            if reason.breaking { "breaking" } else { "note" },
            reason.message
        );
    }
}

fn print_fingerprint(payload: &serde_json::Value) {
    println!("artifact_sha256: {}", payload["artifact_sha256"]);
    if let Some(interface) = payload["interface_sha256"].as_str() {
        println!("interface_sha256: {interface}");
    }
}

fn print_deployment(info: &observatory_deployment::DeploymentInfo) {
    println!("contract: {}", info.contract_id);
    println!("found: {}", info.found);
    println!(
        "wasm_hash: {}",
        info.wasm_hash.as_deref().unwrap_or("<unknown>")
    );
}

fn print_verification(result: &observatory_verify::VerificationResult) {
    println!("status: {:?}", result.status);
    println!("{detail}", detail = result.detail);
    println!("local_wasm_hash: {}", result.local_wasm_hash);
    if let Some(deployed) = &result.deployed_wasm_hash {
        println!("deployed_wasm_hash: {deployed}");
    }
}
