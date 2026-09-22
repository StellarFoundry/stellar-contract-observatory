//! OpenAPI document generation and synchronization checks.
//!
//! The document is generated from the single source of truth for routes
//! ([`crate::api::ROUTES`]) so it cannot drift from the implementation. A test
//! asserts the two stay in sync.

use serde_json::{json, Value};

use crate::api::ROUTES;

/// The OpenAPI version implemented by this document.
pub const OPENAPI_VERSION: &str = "3.0.3";

/// Build the OpenAPI document for the v1 API.
#[must_use]
pub fn document() -> Value {
    let mut paths = serde_json::Map::new();
    for (method, path) in ROUTES {
        let operation = operation_for(method, path);
        let entry = paths
            .entry((*path).to_string())
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
        if let Value::Object(map) = entry {
            map.insert(method.to_ascii_lowercase(), operation);
        }
    }

    json!({
        "openapi": OPENAPI_VERSION,
        "info": {
            "title": "Stellar Contract Observatory API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Contract intelligence, inspection, compatibility, and verification for Stellar/Soroban contracts."
        },
        "servers": [{ "url": "/", "description": "Same-origin" }],
        "components": {
            "securitySchemes": {
                "ApiKey": {
                    "type": "apiKey",
                    "in": "header",
                    "name": "X-API-Key"
                },
                "Bearer": {
                    "type": "http",
                    "scheme": "bearer"
                }
            },
            "schemas": {
                "Error": {
                    "type": "object",
                    "required": ["schema_version", "kind", "code", "message"],
                    "properties": {
                        "schema_version": { "type": "string" },
                        "kind": { "type": "string", "enum": ["error"] },
                        "code": { "type": "string" },
                        "message": { "type": "string" }
                    }
                },
                "Report": {
                    "type": "object",
                    "required": ["schema_version", "kind", "tool", "version", "data"],
                    "properties": {
                        "schema_version": { "type": "string" },
                        "kind": { "type": "string" },
                        "tool": { "type": "string" },
                        "version": { "type": "string" },
                        "data": { "type": "object" }
                    }
                }
            },
            "responses": {
                "Unauthorized": { "$ref": "#/components/responses/ErrorResponse" },
                "Forbidden": { "$ref": "#/components/responses/ErrorResponse" },
                "TooManyRequests": { "$ref": "#/components/responses/ErrorResponse" }
            }
        },
        "paths": Value::Object(paths)
    })
}

/// Whether the API path is public (no authentication required).
#[must_use]
pub fn is_public(path: &str) -> bool {
    matches!(
        path,
        "/api/v1/health" | "/api/v1/readiness" | "/api/v1/version"
    )
}

fn operation_for(method: &str, path: &str) -> Value {
    let public = is_public(path);
    let mut operation = json!({
        "operationId": operation_id(method, path),
        "summary": summary_for(path),
        "responses": {
            "200": {
                "description": "Success",
                "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Report" } } }
            },
            "400": { "description": "Bad request", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Error" } } } },
            "429": { "description": "Rate limited", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Error" } } } }
        }
    });
    if !public {
        operation["security"] = json!([{ "ApiKey": [] }, { "Bearer": [] }]);
    }
    if method == "POST" {
        operation["requestBody"] = json!({
            "required": true,
            "content": {
                "application/json": {
                    "schema": { "type": "object", "additionalProperties": true }
                }
            }
        });
    }
    operation
}

fn operation_id(method: &str, path: &str) -> String {
    let suffix: String = path
        .trim_start_matches("/api/v1/")
        .split('/')
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect();
    format!("{}{}", method.to_ascii_lowercase(), suffix)
}

fn summary_for(path: &str) -> &'static str {
    match path {
        "/api/v1/health" => "Liveness probe",
        "/api/v1/readiness" => "Readiness probe",
        "/api/v1/version" => "Version information",
        "/api/v1/contracts/inspect" => "Inspect a WASM artifact",
        "/api/v1/contracts/spec" => "Read the contract specification",
        "/api/v1/contracts/diff" => "Diff two interfaces",
        "/api/v1/contracts/compatibility" => "Assess interface compatibility",
        "/api/v1/contracts/fingerprint" => "Compute fingerprints",
        "/api/v1/contracts/security" => "Run security heuristics",
        _ => "Operation",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_route_is_documented_and_vice_versa() {
        let document = document();
        let paths = document["paths"].as_object().unwrap();
        for (method, path) in ROUTES {
            let entry = paths
                .get(*path)
                .unwrap_or_else(|| panic!("route {path} missing from OpenAPI document"));
            assert!(
                entry.get(method.to_ascii_lowercase()).is_some(),
                "method {method} missing for {path}"
            );
        }
        let documented = paths
            .values()
            .map(|entry| entry.as_object().map_or(0, |map| map.len()))
            .sum::<usize>();
        assert_eq!(documented, ROUTES.len());
    }

    #[test]
    fn analysis_routes_require_security() {
        let document = document();
        let inspect = &document["paths"]["/api/v1/contracts/inspect"]["post"];
        assert!(inspect.get("security").is_some());
        let health = &document["paths"]["/api/v1/health"]["get"];
        assert!(health.get("security").is_none());
    }
}
