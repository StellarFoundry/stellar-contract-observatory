//! Application/service layer and platform concerns for the Observatory.
//!
//! This crate introduces no analysis logic of its own. [`service::Observatory`]
//! composes the contract-intelligence crates into coarse application services,
//! and the remaining modules provide authentication, authorization, and rate
//! limiting that a transport layer (for example `observatory-api`) can apply.
//!
//! Keeping these concerns out of the intelligence crates preserves the
//! dependency direction: the platform depends on the engine, never the reverse.

pub mod auth;
pub mod config;
pub mod ratelimit;
pub mod rbac;
pub mod service;

pub use auth::{ApiKeyRecord, ApiKeyStore, Principal};
pub use config::PlatformConfig;
pub use observatory_compat::CompatibilityPolicy;
pub use ratelimit::{RateLimitConfig, RateLimitDecision, RateLimiter};
pub use rbac::{authorize, Permission, Role};
pub use service::Observatory;
