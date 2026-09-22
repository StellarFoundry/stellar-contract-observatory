//! Versioned REST router with authentication, authorization, and rate limiting.
//!
//! The router is transport-agnostic: [`Api::handle`] takes a parsed [`Request`]
//! and returns a [`Response`]. The HTTP adapter in `crate::http` is a thin
//! wrapper, so the routing logic is fully unit-testable without sockets.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

use observatory_core::{ObservatoryError, Result as CoreResult, SCHEMA_VERSION, TOOL_NAME};
use observatory_output::report;
use observatory_platform::{
    authorize, ApiKeyStore, CompatibilityPolicy, Observatory, Permission, PlatformConfig,
    Principal, RateLimiter, Role,
};
use serde::Serialize;
use serde_json::Value;

use crate::dto::{decode_wasm, CompatibilityRequest, DiffRequest, InspectRequest, WasmRequest};
use crate::http::{Request, Response};

/// The API version prefix.
pub const API_PREFIX: &str = "/api/v1";

/// The declared routes: `(method, path)`. Used by the router and by the
/// OpenAPI synchronization test.
pub const ROUTES: &[(&str, &str)] = &[
    ("GET", "/api/v1/health"),
    ("GET", "/api/v1/readiness"),
    ("GET", "/api/v1/version"),
    ("POST", "/api/v1/contracts/inspect"),
    ("POST", "/api/v1/contracts/spec"),
    ("POST", "/api/v1/contracts/diff"),
    ("POST", "/api/v1/contracts/compatibility"),
    ("POST", "/api/v1/contracts/fingerprint"),
    ("POST", "/api/v1/contracts/security"),
];

/// An API error with an HTTP status and a stable code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError {
    /// HTTP status code.
    pub status: u16,
    /// Stable machine-readable code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

impl ApiError {
    fn new(status: u16, code: &str, message: impl Into<String>) -> Self {
        ApiError {
            status,
            code: code.to_string(),
            message: message.into(),
        }
    }

    fn unauthorized() -> Self {
        ApiError::new(401, "unauthorized", "a valid API key is required")
    }

    fn forbidden() -> Self {
        ApiError::new(
            403,
            "forbidden",
            "the API key lacks the required permission",
        )
    }

    fn bad_request(message: impl Into<String>) -> Self {
        ApiError::new(400, "bad_request", message)
    }
}

impl From<ObservatoryError> for ApiError {
    fn from(error: ObservatoryError) -> Self {
        let status = match error {
            ObservatoryError::InvalidInput(_) => 400,
            ObservatoryError::Decode(_) | ObservatoryError::Wasm(_) | ObservatoryError::Spec(_) => {
                400
            }
            ObservatoryError::Unsupported(_) => 422,
            ObservatoryError::Rpc(_) => 502,
            ObservatoryError::Verification(_) => 422,
            ObservatoryError::Io(_) | ObservatoryError::Internal(_) => 500,
        };
        ApiError {
            status,
            code: error.code().as_str().to_string(),
            message: error.to_string(),
        }
    }
}

/// The API application.
pub struct Api {
    observatory: Observatory,
    keys: RwLock<ApiKeyStore>,
    limiter: RateLimiter,
    config: PlatformConfig,
    counter: AtomicU64,
}

impl Api {
    /// Create an API with the given configuration and no keys.
    #[must_use]
    pub fn new(config: PlatformConfig) -> Self {
        let limiter = RateLimiter::new(config.rate_limit);
        Api {
            observatory: Observatory::new(),
            keys: RwLock::new(ApiKeyStore::new()),
            limiter,
            config,
            counter: AtomicU64::new(0),
        }
    }

    /// The active configuration.
    #[must_use]
    pub fn config(&self) -> &PlatformConfig {
        &self.config
    }

    /// Generate a key and return its plaintext exactly once.
    pub fn generate_key(&self, role: Role, label: &str) -> CoreResult<String> {
        let mut keys = self.keys.write().expect("key store poisoned");
        let (plaintext, _record) = keys.generate(role, label)?;
        Ok(plaintext)
    }

    /// Insert a pre-hashed key (fixtures/imports).
    pub fn insert_hashed_key(
        &self,
        id: impl Into<String>,
        hash: impl Into<String>,
        role: Role,
        label: &str,
    ) {
        let mut keys = self.keys.write().expect("key store poisoned");
        keys.insert_hashed(id, hash, role, label);
    }

    /// Register a key from its plaintext form (`sco_<id>_<secret>`).
    pub fn insert_plaintext_key(&self, plaintext: &str, role: Role, label: &str) -> CoreResult<()> {
        let mut keys = self.keys.write().expect("key store poisoned");
        keys.insert_plaintext(plaintext, role, label)?;
        Ok(())
    }

    /// Revoke a key.
    pub fn revoke_key(&self, id: &str) -> bool {
        let mut keys = self.keys.write().expect("key store poisoned");
        keys.revoke(id)
    }

    /// Handle a request.
    #[must_use]
    pub fn handle(&self, request: &Request, client_id: &str, now_secs: u64) -> Response {
        let request_id = format!(
            "req-{:016x}",
            self.counter.fetch_add(1, Ordering::Relaxed) + 1
        );
        let principal = self.principal(request);
        let rate_key = principal
            .as_ref()
            .map(|principal| principal.key_id.clone())
            .unwrap_or_else(|| format!("client:{client_id}"));
        let decision = self.limiter.check(&rate_key, now_secs);

        let mut response = if !decision.allowed {
            error_response(429, "rate_limited", "rate limit exceeded")
        } else {
            match self.route(request, principal.as_ref()) {
                Ok(response) => response,
                Err(error) => error_response(error.status, &error.code, &error.message),
            }
        };
        response
            .headers
            .push(("X-Request-Id".to_string(), request_id));
        response
            .headers
            .push(("X-RateLimit-Limit".to_string(), decision.limit.to_string()));
        response.headers.push((
            "X-RateLimit-Remaining".to_string(),
            decision.remaining.to_string(),
        ));
        response.headers.push((
            "X-RateLimit-Reset".to_string(),
            decision.reset_after_secs.to_string(),
        ));
        response
    }

    fn principal(&self, request: &Request) -> Option<Principal> {
        let provided = request
            .header("x-api-key")
            .map(str::to_string)
            .or_else(|| {
                request
                    .header("authorization")
                    .and_then(|value| value.strip_prefix("Bearer "))
                    .map(str::to_string)
            })?;
        let keys = self.keys.read().expect("key store poisoned");
        keys.authenticate(&provided)
    }

    fn require(
        &self,
        principal: Option<&Principal>,
        permission: Permission,
    ) -> Result<(), ApiError> {
        if !self.config.require_auth {
            return Ok(());
        }
        let principal = principal.ok_or_else(ApiError::unauthorized)?;
        if authorize(principal.role, permission) {
            Ok(())
        } else {
            Err(ApiError::forbidden())
        }
    }

    fn route(
        &self,
        request: &Request,
        principal: Option<&Principal>,
    ) -> Result<Response, ApiError> {
        match (request.method.as_str(), request.path.as_str()) {
            ("GET", "/api/v1/health") => Ok(response(
                200,
                "health",
                &serde_json::json!({ "status": "ok" }),
            )),
            ("GET", "/api/v1/readiness") => Ok(response(
                200,
                "readiness",
                &serde_json::json!({
                    "status": "ready",
                    "auth_required": self.config.require_auth,
                }),
            )),
            ("GET", "/api/v1/version") => Ok(response(
                200,
                "version",
                &serde_json::json!({
                    "tool": TOOL_NAME,
                    "version": env!("CARGO_PKG_VERSION"),
                    "schema_version": SCHEMA_VERSION,
                    "api_version": "v1",
                }),
            )),
            ("POST", "/api/v1/contracts/inspect") => {
                self.require(principal, Permission::RunAnalysis)?;
                let body: InspectRequest = parse_json(request)?;
                let wasm = decode_wasm(&body.wasm_base64).map_err(ApiError::from)?;
                let outcome = self
                    .observatory
                    .inspect(&wasm, body.security)
                    .map_err(ApiError::from)?;
                Ok(response(200, "inspect", &outcome))
            }
            ("POST", "/api/v1/contracts/spec") => {
                self.require(principal, Permission::RunAnalysis)?;
                let body: WasmRequest = parse_json(request)?;
                let wasm = decode_wasm(&body.wasm_base64).map_err(ApiError::from)?;
                let outcome = self.observatory.spec(&wasm).map_err(ApiError::from)?;
                Ok(response(200, "spec", &outcome))
            }
            ("POST", "/api/v1/contracts/diff") => {
                self.require(principal, Permission::RunAnalysis)?;
                let body: DiffRequest = parse_json(request)?;
                let old = decode_wasm(&body.old_wasm_base64).map_err(ApiError::from)?;
                let new = decode_wasm(&body.new_wasm_base64).map_err(ApiError::from)?;
                let outcome = self.observatory.diff(&old, &new).map_err(ApiError::from)?;
                Ok(response(200, "diff", &outcome))
            }
            ("POST", "/api/v1/contracts/compatibility") => {
                self.require(principal, Permission::RunAnalysis)?;
                let body: CompatibilityRequest = parse_json(request)?;
                let policy = match body.policy.as_str() {
                    "strict" => CompatibilityPolicy::strict(),
                    "lenient" => CompatibilityPolicy::lenient(),
                    other => {
                        return Err(ApiError::bad_request(format!(
                            "unknown policy `{other}`; expected strict or lenient"
                        )))
                    }
                };
                let old = decode_wasm(&body.old_wasm_base64).map_err(ApiError::from)?;
                let new = decode_wasm(&body.new_wasm_base64).map_err(ApiError::from)?;
                let outcome = self
                    .observatory
                    .compatibility(&old, &new, policy)
                    .map_err(ApiError::from)?;
                Ok(response(200, "compat", &outcome))
            }
            ("POST", "/api/v1/contracts/fingerprint") => {
                self.require(principal, Permission::RunAnalysis)?;
                let body: WasmRequest = parse_json(request)?;
                let wasm = decode_wasm(&body.wasm_base64).map_err(ApiError::from)?;
                let outcome = self
                    .observatory
                    .fingerprint(&wasm)
                    .map_err(ApiError::from)?;
                Ok(response(200, "fingerprint", &outcome))
            }
            ("POST", "/api/v1/contracts/security") => {
                self.require(principal, Permission::RunAnalysis)?;
                let body: WasmRequest = parse_json(request)?;
                let wasm = decode_wasm(&body.wasm_base64).map_err(ApiError::from)?;
                let outcome = self.observatory.security(&wasm).map_err(ApiError::from)?;
                Ok(response(200, "security", &outcome))
            }
            _ => {
                // Distinguish a known path with the wrong method from an unknown path.
                if ROUTES.iter().any(|(_, path)| *path == request.path) {
                    Err(ApiError::new(
                        405,
                        "method_not_allowed",
                        "method not allowed for this path",
                    ))
                } else {
                    Err(ApiError::new(404, "not_found", "unknown route"))
                }
            }
        }
    }
}

fn parse_json<T: serde::de::DeserializeOwned>(request: &Request) -> Result<T, ApiError> {
    if !request.body.is_empty() && request.header("content-type") != Some("application/json") {
        // Be lenient only about a missing content-type; reject explicit non-JSON.
        if let Some(value) = request.header("content-type") {
            if !value.contains("json") {
                return Err(ApiError::new(
                    415,
                    "unsupported_media_type",
                    "Content-Type must be application/json",
                ));
            }
        }
    }
    serde_json::from_slice(&request.body)
        .map_err(|error| ApiError::bad_request(format!("invalid JSON body: {error}")))
}

fn response<T: Serialize>(status: u16, kind: &str, data: &T) -> Response {
    match report(kind, data).to_json() {
        Ok(json) => {
            let value: Value = serde_json::from_str(&json).unwrap_or(Value::Null);
            Response::json(status, &value)
        }
        Err(_) => error_response(500, "internal", "report serialization failed"),
    }
}

fn error_response(status: u16, code: &str, message: &str) -> Response {
    let body = serde_json::json!({
        "schema_version": SCHEMA_VERSION,
        "kind": "error",
        "code": code,
        "message": message,
    });
    Response::json(status, &body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::Request;
    use base64::Engine as _;
    use observatory_testutil::module_with_spec;
    use observatory_testutil::spec::{spec_function, spec_struct};
    use std::collections::BTreeMap;
    use stellar_xdr::{ScSpecEntry, ScSpecTypeDef};

    fn post(path: &str, body: Value) -> Request {
        Request {
            method: "POST".to_string(),
            path: path.to_string(),
            query: None,
            headers: BTreeMap::new(),
            body: serde_json::to_vec(&body).unwrap(),
        }
    }

    fn get(path: &str) -> Request {
        Request {
            method: "GET".to_string(),
            path: path.to_string(),
            query: None,
            headers: BTreeMap::new(),
            body: Vec::new(),
        }
    }

    fn contract() -> String {
        let module = module_with_spec(&[
            spec_function("hello", &[], Some(ScSpecTypeDef::Bool)),
            spec_struct("S", &[("a", ScSpecTypeDef::U32)]),
        ]);
        base64::engine::general_purpose::STANDARD.encode(module)
    }

    fn empty_contract() -> String {
        base64::engine::general_purpose::STANDARD.encode(module_with_spec(&[] as &[ScSpecEntry]))
    }

    fn api_with_developer() -> (Api, String) {
        let api = Api::new(PlatformConfig::default());
        let key = api.generate_key(Role::Developer, "test").unwrap();
        (api, key)
    }

    fn with_key(mut request: Request, key: &str) -> Request {
        request
            .headers
            .insert("authorization".to_string(), format!("Bearer {key}"));
        request
    }

    #[test]
    fn public_health_and_version_do_not_require_auth() {
        let api = Api::new(PlatformConfig::default());
        assert_eq!(api.handle(&get("/api/v1/health"), "ip", 0).status, 200);
        let version = api.handle(&get("/api/v1/version"), "ip", 0);
        assert_eq!(version.status, 200);
        assert!(String::from_utf8_lossy(&version.body).contains("\"api_version\":\"v1\""));
    }

    #[test]
    fn analysis_requires_authentication() {
        let api = Api::new(PlatformConfig::default());
        let request = post(
            "/api/v1/contracts/inspect",
            serde_json::json!({ "wasm_base64": contract() }),
        );
        let response = api.handle(&request, "ip", 0);
        assert_eq!(response.status, 401);
    }

    #[test]
    fn developer_can_inspect() {
        let (api, key) = api_with_developer();
        let request = with_key(
            post(
                "/api/v1/contracts/inspect",
                serde_json::json!({ "wasm_base64": contract() }),
            ),
            &key,
        );
        let response = api.handle(&request, "ip", 0);
        assert_eq!(response.status, 200);
        assert!(response
            .headers
            .iter()
            .any(|(name, _)| name == "X-Request-Id"));
        assert!(String::from_utf8_lossy(&response.body).contains("\"has_contract_spec\":true"));
    }

    #[test]
    fn viewer_is_forbidden_from_analysis() {
        let api = Api::new(PlatformConfig::default());
        let key = api.generate_key(Role::Viewer, "readonly").unwrap();
        let request = with_key(
            post(
                "/api/v1/contracts/spec",
                serde_json::json!({ "wasm_base64": contract() }),
            ),
            &key,
        );
        assert_eq!(api.handle(&request, "ip", 0).status, 403);
    }

    #[test]
    fn compatibility_reports_incompatible() {
        let (api, key) = api_with_developer();
        let old = empty_contract();
        let new = contract();
        let request = with_key(
            post(
                "/api/v1/contracts/compatibility",
                serde_json::json!({ "old_wasm_base64": old, "new_wasm_base64": new, "policy": "strict" }),
            ),
            &key,
        );
        let response = api.handle(&request, "ip", 0);
        assert_eq!(response.status, 200);
        assert!(String::from_utf8_lossy(&response.body).contains("\"status\":\"unknown\""));
    }

    #[test]
    fn invalid_wasm_is_a_client_error() {
        let (api, key) = api_with_developer();
        let request = with_key(
            post(
                "/api/v1/contracts/inspect",
                serde_json::json!({ "wasm_base64": "AA==" }),
            ),
            &key,
        );
        assert_eq!(api.handle(&request, "ip", 0).status, 400);
    }

    #[test]
    fn unknown_route_and_wrong_method() {
        let api = Api::new(PlatformConfig::default());
        assert_eq!(api.handle(&get("/api/v1/nope"), "ip", 0).status, 404);
        assert_eq!(
            api.handle(&get("/api/v1/contracts/inspect"), "ip", 0)
                .status,
            405
        );
    }

    #[test]
    fn rate_limit_returns_429_with_headers() {
        let config = PlatformConfig {
            require_auth: false,
            rate_limit: observatory_platform::RateLimitConfig {
                limit: 1,
                window_secs: 60,
            },
        };
        let api = Api::new(config);
        assert_eq!(api.handle(&get("/api/v1/health"), "ip", 0).status, 200);
        let second = api.handle(&get("/api/v1/health"), "ip", 0);
        assert_eq!(second.status, 429);
        assert!(second
            .headers
            .iter()
            .any(|(name, value)| name == "X-RateLimit-Limit" && value == "1"));
    }

    #[test]
    fn missing_artifact_field_is_a_bad_request() {
        let (api, key) = api_with_developer();
        let request = with_key(post("/api/v1/contracts/spec", serde_json::json!({})), &key);
        assert_eq!(api.handle(&request, "ip", 0).status, 400);
    }
}
