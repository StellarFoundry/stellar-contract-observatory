//! Stellar RPC transport abstraction and typed models.
//!
//! The core never hard-codes a provider. A [`Transport`] moves JSON-RPC
//! envelopes; [`RpcClient`] builds requests, normalizes errors, and returns
//! typed responses. [`MockTransport`] provides deterministic responses so tests
//! never touch the network.
//!
//! Only methods with a stable, documented shape are modelled here. Anything not
//! modelled is intentionally absent rather than guessed.

use std::cell::RefCell;
use std::collections::BTreeMap;

use observatory_core::{ObservatoryError, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub mod net;

pub use net::{AddressClass, SsrfPolicy};

/// A JSON-RPC transport. Implementations move already-built envelopes.
pub trait Transport {
    /// Send a JSON-RPC request envelope and return the response envelope.
    fn send(&self, request: &Value) -> Result<Value>;
}

impl<T: Transport + ?Sized> Transport for &T {
    fn send(&self, request: &Value) -> Result<Value> {
        (**self).send(request)
    }
}

/// A validated RPC endpoint URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    url: String,
    host: String,
    loopback: bool,
}

impl Endpoint {
    /// Parse and validate an endpoint.
    ///
    /// Rules: `https` is required except for loopback hosts, and embedded
    /// credentials are rejected.
    pub fn parse(input: &str) -> Result<Self> {
        let url = input.trim();
        if url.is_empty() {
            return Err(ObservatoryError::invalid("empty RPC endpoint"));
        }
        let (scheme, rest) = url
            .split_once("://")
            .ok_or_else(|| ObservatoryError::invalid("RPC endpoint is missing a scheme"))?;
        let scheme = scheme.to_ascii_lowercase();
        if scheme != "https" && scheme != "http" {
            return Err(ObservatoryError::invalid(format!(
                "unsupported endpoint scheme `{scheme}`; expected https"
            )));
        }
        let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
        if authority.is_empty() {
            return Err(ObservatoryError::invalid("RPC endpoint is missing a host"));
        }
        if authority.contains('@') {
            return Err(ObservatoryError::invalid(
                "RPC endpoint must not embed credentials",
            ));
        }
        let host = if let Some(stripped) = authority.strip_prefix('[') {
            stripped
                .split_once(']')
                .map(|(host, _)| host.to_string())
                .ok_or_else(|| ObservatoryError::invalid("malformed IPv6 host in endpoint"))?
        } else {
            authority.split(':').next().unwrap_or(authority).to_string()
        };
        if host.is_empty() {
            return Err(ObservatoryError::invalid("RPC endpoint is missing a host"));
        }
        let lower = host.to_ascii_lowercase();
        let loopback = matches!(lower.as_str(), "localhost" | "127.0.0.1" | "::1")
            || lower.ends_with(".localhost");
        if scheme == "http" && !loopback {
            return Err(ObservatoryError::invalid(
                "plain http is only allowed for loopback endpoints",
            ));
        }
        Ok(Endpoint {
            url: url.to_string(),
            host,
            loopback,
        })
    }

    /// The endpoint as provided.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.url
    }

    /// The parsed host.
    #[must_use]
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Whether the host is a loopback address.
    #[must_use]
    pub fn is_loopback(&self) -> bool {
        self.loopback
    }

    /// The effective port, using scheme defaults when omitted.
    #[must_use]
    pub fn port(&self) -> u16 {
        let (scheme, rest) = self.url.split_once("://").unwrap_or(("", ""));
        let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
        let default = if scheme.eq_ignore_ascii_case("http") {
            80
        } else {
            443
        };
        if let Some(stripped) = authority.strip_prefix('[') {
            if let Some((_, after)) = stripped.split_once(']') {
                if let Some(port) = after.strip_prefix(':').and_then(|p| p.parse().ok()) {
                    return port;
                }
            }
            return default;
        }
        authority
            .rsplit_once(':')
            .and_then(|(_, port)| port.parse().ok())
            .unwrap_or(default)
    }

    /// Validate the endpoint host against the SSRF policy.
    ///
    /// A live transport must call this before connecting.
    pub fn validate_ssrf(&self, policy: &SsrfPolicy) -> Result<Vec<std::net::IpAddr>> {
        policy.validate(&self.host, self.port())
    }
}

/// Latest ledger information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LatestLedger {
    /// Ledger sequence.
    pub sequence: u32,
    /// Ledger hash/id, when provided.
    pub id: Option<String>,
    /// Protocol version, when provided.
    pub protocol_version: Option<u32>,
}

/// Network information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkInfo {
    /// Network passphrase.
    pub passphrase: String,
    /// Protocol version, when provided.
    pub protocol_version: Option<u32>,
}

/// One ledger entry returned by `getLedgerEntries`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerEntryResult {
    /// Base64 XDR of the ledger key.
    pub key: String,
    /// Base64 XDR of the ledger entry data.
    pub xdr: String,
    /// Last modified ledger sequence.
    pub last_modified_ledger_seq: Option<u32>,
    /// Live-until ledger sequence.
    pub live_until_ledger_seq: Option<u32>,
}

/// A typed RPC client over a [`Transport`].
#[derive(Debug, Clone)]
pub struct RpcClient<T: Transport> {
    endpoint: Endpoint,
    transport: T,
}

impl<T: Transport> RpcClient<T> {
    /// Create a client.
    pub fn new(endpoint: Endpoint, transport: T) -> Self {
        RpcClient {
            endpoint,
            transport,
        }
    }

    /// The endpoint in use.
    #[must_use]
    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    /// Call a JSON-RPC method and return its `result`, normalizing errors.
    pub fn call(&self, method: &str, params: Value) -> Result<Value> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });
        let response = self.transport.send(&request)?;
        if let Some(error) = response.get("error") {
            let code = error.get("code").and_then(Value::as_i64).unwrap_or(0);
            let message = error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("unknown RPC error");
            return Err(ObservatoryError::rpc(format!(
                "{method} failed: {message} (code {code})"
            )));
        }
        response
            .get("result")
            .cloned()
            .ok_or_else(|| ObservatoryError::rpc(format!("{method} response had no result")))
    }

    /// Fetch the latest ledger.
    pub fn get_latest_ledger(&self) -> Result<LatestLedger> {
        let result = self.call("getLatestLedger", json!({}))?;
        Ok(LatestLedger {
            sequence: result.get("sequence").and_then(Value::as_u64).unwrap_or(0) as u32,
            id: result.get("id").and_then(Value::as_str).map(str::to_string),
            protocol_version: result
                .get("protocolVersion")
                .and_then(Value::as_u64)
                .map(|value| value as u32),
        })
    }

    /// Fetch network information.
    pub fn get_network(&self) -> Result<NetworkInfo> {
        let result = self.call("getNetwork", json!({}))?;
        Ok(NetworkInfo {
            passphrase: result
                .get("passphrase")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            protocol_version: result
                .get("protocolVersion")
                .and_then(Value::as_u64)
                .map(|value| value as u32),
        })
    }

    /// Fetch ledger entries for base64-XDR keys.
    pub fn get_ledger_entries(&self, keys: &[String]) -> Result<Vec<LedgerEntryResult>> {
        let result = self.call("getLedgerEntries", json!({ "keys": keys }))?;
        let entries = result
            .get("entries")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                ObservatoryError::rpc("getLedgerEntries response had no entries array")
            })?;
        Ok(entries
            .iter()
            .map(|entry| LedgerEntryResult {
                key: entry
                    .get("key")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                xdr: entry
                    .get("xdr")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                last_modified_ledger_seq: entry
                    .get("lastModifiedLedgerSeq")
                    .and_then(Value::as_u64)
                    .map(|value| value as u32),
                live_until_ledger_seq: entry
                    .get("liveUntilLedgerSeq")
                    .and_then(Value::as_u64)
                    .map(|value| value as u32),
            })
            .collect())
    }
}

/// A deterministic in-memory transport for tests.
#[derive(Debug, Default)]
pub struct MockTransport {
    results: BTreeMap<String, Value>,
    errors: BTreeMap<String, Value>,
    calls: RefCell<Vec<String>>,
}

impl MockTransport {
    /// Create an empty mock transport.
    #[must_use]
    pub fn new() -> Self {
        MockTransport::default()
    }

    /// Register a successful result for a method.
    #[must_use]
    pub fn with_result(mut self, method: &str, result: Value) -> Self {
        self.results.insert(method.to_string(), result);
        self
    }

    /// Register an error for a method.
    #[must_use]
    pub fn with_error(mut self, method: &str, code: i64, message: &str) -> Self {
        self.errors.insert(
            method.to_string(),
            json!({ "code": code, "message": message }),
        );
        self
    }

    /// The methods called, in order.
    #[must_use]
    pub fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }

    /// Build a transport from a JSON object mapping method name to result.
    ///
    /// A value of the form `{ "__error": { "code": -32000, "message": "..." } }`
    /// registers an error response for that method.
    pub fn from_json(value: &Value) -> Result<Self> {
        let object = value
            .as_object()
            .ok_or_else(|| ObservatoryError::rpc("RPC fixture must be a JSON object"))?;
        let mut transport = MockTransport::new();
        for (method, result) in object {
            if let Some(error) = result.get("__error") {
                let code = error.get("code").and_then(Value::as_i64).unwrap_or(-32000);
                let message = error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("error");
                transport = transport.with_error(method, code, message);
            } else {
                transport = transport.with_result(method, result.clone());
            }
        }
        Ok(transport)
    }
}

impl Transport for MockTransport {
    fn send(&self, request: &Value) -> Result<Value> {
        let method = request
            .get("method")
            .and_then(Value::as_str)
            .ok_or_else(|| ObservatoryError::rpc("mock request missing method"))?;
        self.calls.borrow_mut().push(method.to_string());
        if let Some(error) = self.errors.get(method) {
            return Ok(json!({ "jsonrpc": "2.0", "id": 1, "error": error.clone() }));
        }
        if let Some(result) = self.results.get(method) {
            return Ok(json!({ "jsonrpc": "2.0", "id": 1, "result": result.clone() }));
        }
        Ok(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": { "code": -32601, "message": format!("method not mocked: {method}") }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_https() {
        let endpoint = Endpoint::parse("https://soroban-testnet.stellar.org").unwrap();
        assert_eq!(endpoint.host(), "soroban-testnet.stellar.org");
        assert!(!endpoint.is_loopback());
    }

    #[test]
    fn accepts_loopback_http() {
        assert!(Endpoint::parse("http://localhost:8000")
            .unwrap()
            .is_loopback());
        assert!(Endpoint::parse("http://127.0.0.1:8000")
            .unwrap()
            .is_loopback());
    }

    #[test]
    fn rejects_remote_http_and_credentials_and_schemes() {
        assert!(Endpoint::parse("http://example.com").is_err());
        assert!(Endpoint::parse("https://user:pass@example.com").is_err());
        assert!(Endpoint::parse("ftp://example.com").is_err());
        assert!(Endpoint::parse("not a url").is_err());
        assert!(Endpoint::parse("").is_err());
    }

    #[test]
    fn calls_typed_methods() {
        let transport = MockTransport::new()
            .with_result("getLatestLedger", json!({ "sequence": 42, "id": "abc" }))
            .with_result(
                "getNetwork",
                json!({ "passphrase": "Test SDF Network ; September 2015" }),
            )
            .with_result(
                "getLedgerEntries",
                json!({ "entries": [ { "key": "k", "xdr": "x", "lastModifiedLedgerSeq": 7 } ] }),
            );
        let client = RpcClient::new(Endpoint::parse("https://example.org").unwrap(), transport);
        assert_eq!(client.get_latest_ledger().unwrap().sequence, 42);
        assert!(client
            .get_network()
            .unwrap()
            .passphrase
            .contains("Test SDF"));
        let entries = client.get_ledger_entries(&["key".to_string()]).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].last_modified_ledger_seq, Some(7));
        assert_eq!(
            client.endpoint,
            Endpoint::parse("https://example.org").unwrap()
        );
    }

    #[test]
    fn normalizes_rpc_errors() {
        let transport = MockTransport::new().with_error("getNetwork", -32601, "not found");
        let client = RpcClient::new(Endpoint::parse("https://example.org").unwrap(), transport);
        let error = client.get_network().unwrap_err();
        assert!(matches!(error, ObservatoryError::Rpc(_)));
        assert!(error.to_string().contains("not found"));
    }

    #[test]
    fn records_calls() {
        let transport = MockTransport::new().with_result("getNetwork", json!({}));
        let client = RpcClient::new(Endpoint::parse("https://example.org").unwrap(), &transport);
        client.get_network().unwrap();
        assert_eq!(transport.calls(), vec!["getNetwork".to_string()]);
        let _ = &client;
    }

    #[test]
    fn builds_transport_from_json_fixture() {
        let fixture = json!({
            "getLatestLedger": { "sequence": 9 },
            "getNetwork": { "__error": { "code": -32000, "message": "boom" } }
        });
        let transport = MockTransport::from_json(&fixture).unwrap();
        let client = RpcClient::new(Endpoint::parse("https://example.org").unwrap(), transport);
        assert_eq!(client.get_latest_ledger().unwrap().sequence, 9);
        assert!(client.get_network().is_err());
        assert!(MockTransport::from_json(&json!([])).is_err());
    }

    #[test]
    fn endpoint_ports_use_scheme_defaults_and_overrides() {
        assert_eq!(Endpoint::parse("https://example.org").unwrap().port(), 443);
        assert_eq!(
            Endpoint::parse("https://example.org:8443").unwrap().port(),
            8443
        );
        assert_eq!(
            Endpoint::parse("http://localhost:8000").unwrap().port(),
            8000
        );
        assert_eq!(Endpoint::parse("http://localhost").unwrap().port(), 80);
        assert_eq!(Endpoint::parse("https://[::1]:9000").unwrap().port(), 9000);
    }

    #[test]
    fn endpoint_ssrf_policy_is_enforced() {
        let public = Endpoint::parse("https://8.8.8.8").unwrap();
        assert!(public.validate_ssrf(&SsrfPolicy::strict()).is_ok());

        let loopback = Endpoint::parse("http://127.0.0.1:8080").unwrap();
        assert!(loopback.validate_ssrf(&SsrfPolicy::strict()).is_err());
        assert!(loopback.validate_ssrf(&SsrfPolicy::local()).is_ok());

        let metadata = Endpoint::parse("https://169.254.169.254").unwrap();
        assert!(metadata.validate_ssrf(&SsrfPolicy::local()).is_err());
    }
}
