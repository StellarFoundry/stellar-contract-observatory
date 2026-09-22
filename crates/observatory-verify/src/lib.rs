//! Local-versus-deployed verification and reproducibility metadata analysis.
//!
//! # What verification means here
//!
//! Verification compares the **artifact identity** of a local WASM file against
//! the deployed contract's executable hash. It does **not** reproduce the build
//! from source and therefore does not claim source-level reproducibility. Build
//! metadata parsing is provided to reason about reproducibility separately.
//!
//! Metadata is read from the real Soroban custom sections:
//!
//! * `contractenvmetav0` — `SCEnvMetaEntry` with protocol and pre-release.
//! * `contractmetav0` — `SCMetaEntry` key/value pairs.

use std::collections::BTreeMap;
use std::io::Cursor;

use observatory_core::limits::MAX_SPEC_ENTRIES;
use observatory_core::{ObservatoryError, Result};
use observatory_rpc::{RpcClient, Transport};
use serde::{Deserialize, Serialize};
use stellar_xdr::{Limited, Limits, ReadXdr, ScEnvMetaEntry, ScMetaEntry, ScMetaV0};

/// The outcome of a local-versus-deployed comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationStatus {
    /// The local artifact matches the deployed executable hash.
    Match,
    /// The local artifact does not match the deployed executable hash.
    Mismatch,
    /// There was not enough on-chain data to decide.
    InsufficientData,
    /// The comparison could not be completed.
    Error,
}

/// The result of verifying a local artifact against a deployment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VerificationResult {
    /// The verdict.
    pub status: VerificationStatus,
    /// The contract id that was queried.
    pub contract_id: String,
    /// The network label used.
    pub network: String,
    /// SHA-256 of the local artifact.
    pub local_wasm_hash: String,
    /// The deployed executable hash, if known.
    pub deployed_wasm_hash: Option<String>,
    /// A human-readable explanation.
    pub detail: String,
}

/// Environment metadata embedded in `contractenvmetav0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct EnvMeta {
    /// Protocol version the contract targets.
    pub protocol: u32,
    /// Pre-release version.
    pub pre_release: u32,
}

/// Build metadata embedded in `contractmetav0`.
///
/// Unknown keys are preserved verbatim, so the model remains extensible as the
/// SDK adds metadata.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BuildMetadata {
    /// Metadata key/value pairs.
    pub entries: BTreeMap<String, String>,
}

impl BuildMetadata {
    /// Look up a metadata value.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(String::as_str)
    }

    /// Parse a metadata document from JSON (`{ "key": "value", ... }`).
    pub fn from_json(value: &serde_json::Value) -> Result<Self> {
        let object = value
            .as_object()
            .ok_or_else(|| ObservatoryError::invalid("build metadata must be a JSON object"))?;
        let mut entries = BTreeMap::new();
        for (key, value) in object {
            let value = value.as_str().ok_or_else(|| {
                ObservatoryError::invalid(format!("metadata `{key}` is not a string"))
            })?;
            entries.insert(key.clone(), value.to_string());
        }
        Ok(BuildMetadata { entries })
    }
}

/// A single differing metadata key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MetadataDifference {
    /// The key.
    pub key: String,
    /// Value on the left, if present.
    pub left: Option<String>,
    /// Value on the right, if present.
    pub right: Option<String>,
}

/// The comparison of two build metadata documents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MetadataComparison {
    /// Whether the two documents are identical.
    pub equal: bool,
    /// Number of keys compared (union of both key sets).
    pub compared_keys: usize,
    /// Differences, sorted by key.
    pub differences: Vec<MetadataDifference>,
}

/// Compare two build metadata documents.
#[must_use]
pub fn compare_build_metadata(left: &BuildMetadata, right: &BuildMetadata) -> MetadataComparison {
    let mut keys: Vec<&String> = left.entries.keys().chain(right.entries.keys()).collect();
    keys.sort();
    keys.dedup();
    let mut differences = Vec::new();
    for key in &keys {
        let l = left.entries.get(*key);
        let r = right.entries.get(*key);
        if l != r {
            differences.push(MetadataDifference {
                key: (*key).clone(),
                left: l.cloned(),
                right: r.cloned(),
            });
        }
    }
    MetadataComparison {
        equal: differences.is_empty(),
        compared_keys: keys.len(),
        differences,
    }
}

/// Parse `contractenvmetav0` from a WASM artifact, if present.
pub fn parse_env_meta(wasm: &[u8]) -> Result<Option<EnvMeta>> {
    let Some(section) = observatory_wasm::extract_custom_section(
        wasm,
        observatory_wasm::CONTRACT_ENV_META_SECTION,
    )?
    else {
        return Ok(None);
    };
    let entries: Vec<ScEnvMetaEntry> = parse_stream(&section)?;
    if let Some(ScEnvMetaEntry::ScEnvMetaKindInterfaceVersion(version)) = entries.into_iter().next()
    {
        return Ok(Some(EnvMeta {
            protocol: version.protocol,
            pre_release: version.pre_release,
        }));
    }
    Ok(None)
}

/// Parse `contractmetav0` from a WASM artifact, if present.
pub fn parse_build_metadata(wasm: &[u8]) -> Result<Option<BuildMetadata>> {
    let Some(section) =
        observatory_wasm::extract_custom_section(wasm, observatory_wasm::CONTRACT_META_SECTION)?
    else {
        return Ok(None);
    };
    let entries: Vec<ScMetaEntry> = parse_stream(&section)?;
    let mut metadata = BuildMetadata::default();
    for entry in entries {
        let ScMetaEntry::ScMetaV0(ScMetaV0 { key, val }) = entry;
        metadata.entries.insert(key.to_string(), val.to_string());
    }
    Ok(Some(metadata))
}

/// Verify a local artifact against a deployed contract by executable hash.
pub fn verify_local_vs_deployed<T: Transport>(
    local_wasm: &[u8],
    client: &RpcClient<T>,
    contract_id: &str,
    network: &str,
) -> Result<VerificationResult> {
    let local_hash = observatory_wasm::artifact_fingerprint(local_wasm);
    let deployment = observatory_deployment::inspect(client, contract_id)?;

    let (status, detail) = match (deployment.found, deployment.wasm_hash.clone()) {
        (false, _) => (
            VerificationStatus::InsufficientData,
            "no contract instance entry was returned for the contract id".to_string(),
        ),
        (true, None) => (
            VerificationStatus::InsufficientData,
            "the deployed contract is not WASM-backed, so there is no hash to compare".to_string(),
        ),
        (true, Some(deployed)) if deployed == local_hash => (
            VerificationStatus::Match,
            "local artifact hash matches the deployed executable hash".to_string(),
        ),
        (true, Some(deployed)) => (
            VerificationStatus::Mismatch,
            format!("local artifact hash does not match the deployed hash {deployed}"),
        ),
    };

    Ok(VerificationResult {
        status,
        contract_id: contract_id.to_string(),
        network: network.to_string(),
        local_wasm_hash: local_hash,
        deployed_wasm_hash: deployment.wasm_hash,
        detail,
    })
}

fn parse_stream<T: ReadXdr>(bytes: &[u8]) -> Result<Vec<T>> {
    let mut limited = Limited::new(Cursor::new(bytes), Limits::none());
    let mut entries = Vec::new();
    while (limited.inner.position() as usize) < bytes.len() {
        if entries.len() >= MAX_SPEC_ENTRIES {
            return Err(ObservatoryError::decode(
                "metadata stream exceeds the entry limit",
            ));
        }
        let entry = T::read_xdr(&mut limited).map_err(|error| {
            ObservatoryError::decode(format!("invalid metadata entry: {error}"))
        })?;
        entries.push(entry);
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use observatory_rpc::{Endpoint, MockTransport};
    use observatory_testutil::{custom_section, minimal_module};
    use serde_json::json;
    use stellar_xdr::{
        ContractDataDurability, ContractDataEntry, ContractExecutable, ContractId, ExtensionPoint,
        Hash, LedgerEntryData, ScAddress, ScContractInstance, ScVal, WriteXdr,
    };

    fn module_with_env_meta() -> Vec<u8> {
        let entry = ScEnvMetaEntry::ScEnvMetaKindInterfaceVersion(
            stellar_xdr::ScEnvMetaEntryInterfaceVersion {
                protocol: 22,
                pre_release: 0,
            },
        );
        let payload = entry.to_xdr(Limits::none()).unwrap();
        let mut module = minimal_module();
        module.extend(custom_section(
            observatory_wasm::CONTRACT_ENV_META_SECTION,
            &payload,
        ));
        module
    }

    fn module_with_build_meta() -> Vec<u8> {
        let entries = vec![
            ScMetaEntry::ScMetaV0(ScMetaV0 {
                key: "rsver".try_into().unwrap(),
                val: "1.85.0".try_into().unwrap(),
            }),
            ScMetaEntry::ScMetaV0(ScMetaV0 {
                key: "rssdkver".try_into().unwrap(),
                val: "22.0.0".try_into().unwrap(),
            }),
        ];
        let mut payload = Vec::new();
        for entry in entries {
            payload.extend(entry.to_xdr(Limits::none()).unwrap());
        }
        let mut module = minimal_module();
        module.extend(custom_section(
            observatory_wasm::CONTRACT_META_SECTION,
            &payload,
        ));
        module
    }

    #[test]
    fn parses_env_meta() {
        let meta = parse_env_meta(&module_with_env_meta()).unwrap().unwrap();
        assert_eq!(meta.protocol, 22);
        assert_eq!(parse_env_meta(&minimal_module()).unwrap(), None);
    }

    #[test]
    fn parses_build_metadata() {
        let metadata = parse_build_metadata(&module_with_build_meta())
            .unwrap()
            .unwrap();
        assert_eq!(metadata.get("rsver"), Some("1.85.0"));
        assert_eq!(metadata.get("rssdkver"), Some("22.0.0"));
    }

    #[test]
    fn compares_build_metadata() {
        let a =
            BuildMetadata::from_json(&json!({ "rsver": "1.85.0", "profile": "release" })).unwrap();
        let b = BuildMetadata::from_json(&json!({ "rsver": "1.86.0" })).unwrap();
        let comparison = compare_build_metadata(&a, &b);
        assert!(!comparison.equal);
        assert_eq!(comparison.compared_keys, 2);
        assert_eq!(comparison.differences.len(), 2);

        let same = compare_build_metadata(&a, &a);
        assert!(same.equal);
        assert!(same.differences.is_empty());
    }

    fn deployed_transport(hash_byte: u8) -> MockTransport {
        let data = LedgerEntryData::ContractData(ContractDataEntry {
            ext: ExtensionPoint::V0,
            contract: ScAddress::Contract(ContractId(Hash([9u8; 32]))),
            key: ScVal::LedgerKeyContractInstance,
            durability: ContractDataDurability::Persistent,
            val: ScVal::ContractInstance(ScContractInstance {
                executable: ContractExecutable::Wasm(Hash([hash_byte; 32])),
                storage: None,
            }),
        });
        MockTransport::new().with_result(
            "getLedgerEntries",
            json!({ "entries": [ { "key": "k", "xdr": data.to_xdr_base64(Limits::none()).unwrap() } ] }),
        )
    }

    #[test]
    fn verifies_match_and_mismatch() {
        let contract_id = format!("{}", stellar_strkey::Contract([9u8; 32]));
        let local = minimal_module();
        let local_hash = observatory_wasm::artifact_fingerprint(&local);

        // Deployed hash equal to local: build a transport whose decoded hash we control.
        let matching = MockTransport::new().with_result(
            "getLedgerEntries",
            json!({ "entries": [ { "key": "k", "xdr": wasm_instance_xdr(&local_hash) } ] }),
        );
        let client = RpcClient::new(Endpoint::parse("https://example.org").unwrap(), matching);
        let result = verify_local_vs_deployed(&local, &client, &contract_id, "testnet").unwrap();
        assert_eq!(result.status, VerificationStatus::Match);

        let mismatching = deployed_transport(1);
        let client = RpcClient::new(Endpoint::parse("https://example.org").unwrap(), mismatching);
        let result = verify_local_vs_deployed(&local, &client, &contract_id, "testnet").unwrap();
        assert_eq!(result.status, VerificationStatus::Mismatch);
    }

    /// Build a ledger entry whose executable is the WASM hash derived from
    /// `hex_hash`; used to exercise the match path deterministically.
    fn wasm_instance_xdr(hex_hash: &str) -> String {
        let bytes = hex::decode(hex_hash).unwrap();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&bytes);
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

    #[test]
    fn missing_deployment_is_insufficient_data() {
        let contract_id = format!("{}", stellar_strkey::Contract([9u8; 32]));
        let transport =
            MockTransport::new().with_result("getLedgerEntries", json!({ "entries": [] }));
        let client = RpcClient::new(Endpoint::parse("https://example.org").unwrap(), transport);
        let result =
            verify_local_vs_deployed(&minimal_module(), &client, &contract_id, "testnet").unwrap();
        assert_eq!(result.status, VerificationStatus::InsufficientData);
    }
}
