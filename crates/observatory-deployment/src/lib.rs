//! Deployed contract inspection.
//!
//! Deployment lookup reads the contract instance ledger entry through
//! `getLedgerEntries` and decodes the returned `LedgerEntryData` XDR to recover
//! the deployed WASM hash. The ledger key and entry are real Stellar XDR types;
//! nothing about the RPC shape is invented.

use observatory_core::{ObservatoryError, Result};
use observatory_rpc::{LedgerEntryResult, RpcClient, Transport};
use serde::Serialize;
use stellar_xdr::{
    ContractDataDurability, ContractExecutable, ContractId, Hash, LedgerEntryData, LedgerKey,
    LedgerKeyContractData, Limits, ReadXdr, ScAddress, ScVal, WriteXdr,
};

/// Information recovered about a deployed contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeploymentInfo {
    /// The contract id (C... strkey) that was queried.
    pub contract_id: String,
    /// The deployed executable WASM hash (hex), if the contract is WASM-backed.
    pub wasm_hash: Option<String>,
    /// Last modified ledger sequence of the instance entry.
    pub last_modified_ledger_seq: Option<u32>,
    /// Live-until ledger sequence of the instance entry.
    pub live_until_ledger_seq: Option<u32>,
    /// Whether a contract instance entry was found.
    pub found: bool,
}

/// Decode a contract id strkey into its 32-byte hash.
pub fn contract_id_hash(input: &str) -> Result<[u8; 32]> {
    let parsed = stellar_strkey::Strkey::from_string(input.trim())
        .map_err(|error| ObservatoryError::invalid(format!("invalid contract id: {error}")))?;
    match parsed {
        stellar_strkey::Strkey::Contract(contract) => Ok(contract.0),
        _ => Err(ObservatoryError::invalid(
            "provided strkey is not a contract id (expected a C... strkey)",
        )),
    }
}

/// Build the base64-XDR ledger key for a contract instance entry.
pub fn contract_instance_key(contract_id: &str) -> Result<String> {
    let hash = contract_id_hash(contract_id)?;
    let key = LedgerKey::ContractData(LedgerKeyContractData {
        contract: ScAddress::Contract(ContractId(Hash(hash))),
        key: ScVal::LedgerKeyContractInstance,
        durability: ContractDataDurability::Persistent,
    });
    key.to_xdr_base64(Limits::none()).map_err(|error| {
        ObservatoryError::internal(format!("failed to encode ledger key: {error}"))
    })
}

/// Inspect a deployed contract, returning its deployed WASM hash when known.
pub fn inspect<T: Transport>(client: &RpcClient<T>, contract_id: &str) -> Result<DeploymentInfo> {
    let key = contract_instance_key(contract_id)?;
    let entries = client.get_ledger_entries(&[key])?;
    let Some(entry) = entries.first() else {
        return Ok(DeploymentInfo {
            contract_id: contract_id.to_string(),
            wasm_hash: None,
            last_modified_ledger_seq: None,
            live_until_ledger_seq: None,
            found: false,
        });
    };
    Ok(DeploymentInfo {
        contract_id: contract_id.to_string(),
        wasm_hash: decode_wasm_hash(entry)?,
        last_modified_ledger_seq: entry.last_modified_ledger_seq,
        live_until_ledger_seq: entry.live_until_ledger_seq,
        found: true,
    })
}

/// Decode the WASM hash from a ledger entry result, if it is a WASM-backed instance.
pub fn decode_wasm_hash(entry: &LedgerEntryResult) -> Result<Option<String>> {
    let data = LedgerEntryData::from_xdr_base64(&entry.xdr, Limits::none())
        .map_err(|error| ObservatoryError::decode(format!("invalid ledger entry XDR: {error}")))?;
    let LedgerEntryData::ContractData(contract) = data else {
        return Ok(None);
    };
    let ScVal::ContractInstance(instance) = contract.val else {
        return Ok(None);
    };
    match instance.executable {
        ContractExecutable::Wasm(hash) => Ok(Some(hex::encode(hash.0))),
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use observatory_rpc::{Endpoint, MockTransport};
    use serde_json::json;
    use stellar_xdr::{
        ContractDataEntry, ExtensionPoint, LedgerEntryData, ScContractInstance, WriteXdr,
    };

    fn contract_id() -> String {
        format!("{}", stellar_strkey::Contract([9u8; 32]))
    }

    fn instance_entry(hash_byte: u8) -> LedgerEntryResult {
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
        LedgerEntryResult {
            key: String::new(),
            xdr: data.to_xdr_base64(Limits::none()).unwrap(),
            last_modified_ledger_seq: Some(123),
            live_until_ledger_seq: None,
        }
    }

    #[test]
    fn parses_contract_id_and_builds_key() {
        let id = contract_id();
        assert_eq!(contract_id_hash(&id).unwrap(), [9u8; 32]);
        let key = contract_instance_key(&id).unwrap();
        assert!(!key.is_empty());
    }

    #[test]
    fn rejects_non_contract_strkey() {
        let account = stellar_strkey::ed25519::PublicKey([0u8; 32]).to_string();
        assert!(contract_id_hash(&account).is_err());
        assert!(contract_id_hash("not-a-strkey").is_err());
    }

    #[test]
    fn decodes_wasm_hash() {
        let entry = instance_entry(7);
        assert_eq!(
            decode_wasm_hash(&entry).unwrap(),
            Some(hex::encode([7u8; 32]))
        );
    }

    #[test]
    fn mock_deployment_inspection() {
        let entry = instance_entry(3);
        let transport = MockTransport::new().with_result(
            "getLedgerEntries",
            json!({ "entries": [ {
                "key": entry.key,
                "xdr": entry.xdr,
                "lastModifiedLedgerSeq": 123
            } ] }),
        );
        let client = RpcClient::new(Endpoint::parse("https://example.org").unwrap(), transport);
        let info = inspect(&client, &contract_id()).unwrap();
        assert!(info.found);
        assert_eq!(info.wasm_hash, Some(hex::encode([3u8; 32])));
        assert_eq!(info.last_modified_ledger_seq, Some(123));
    }

    #[test]
    fn missing_deployment_is_reported() {
        let transport =
            MockTransport::new().with_result("getLedgerEntries", json!({ "entries": [] }));
        let client = RpcClient::new(Endpoint::parse("https://example.org").unwrap(), transport);
        let info = inspect(&client, &contract_id()).unwrap();
        assert!(!info.found);
        assert_eq!(info.wasm_hash, None);
    }
}
