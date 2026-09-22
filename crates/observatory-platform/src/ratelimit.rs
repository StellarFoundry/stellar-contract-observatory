//! Fixed-window rate limiting.
//!
//! The limiter is in-memory and process-local. It is deterministic given an
//! injected clock reading, which keeps tests offline and reproducible. A
//! distributed limiter is intentionally out of scope until a real workload
//! requires it.

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// Rate-limit configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per window.
    pub limit: u32,
    /// Window length in seconds.
    pub window_secs: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        RateLimitConfig {
            limit: 120,
            window_secs: 60,
        }
    }
}

/// The outcome of a rate-limit check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RateLimitDecision {
    /// Whether the request is allowed.
    pub allowed: bool,
    /// The configured limit.
    pub limit: u32,
    /// Remaining requests in the window.
    pub remaining: u32,
    /// Seconds until the window resets.
    pub reset_after_secs: u64,
}

#[derive(Debug, Clone, Copy)]
struct Window {
    count: u32,
    start: u64,
}

/// An in-memory fixed-window rate limiter.
#[derive(Debug)]
pub struct RateLimiter {
    config: RateLimitConfig,
    windows: Mutex<BTreeMap<String, Window>>,
}

impl RateLimiter {
    /// Create a limiter with the given configuration.
    #[must_use]
    pub fn new(config: RateLimitConfig) -> Self {
        RateLimiter {
            config,
            windows: Mutex::new(BTreeMap::new()),
        }
    }

    /// The active configuration.
    #[must_use]
    pub fn config(&self) -> RateLimitConfig {
        self.config
    }

    /// Check and consume one request for `key` at time `now_secs`.
    pub fn check(&self, key: &str, now_secs: u64) -> RateLimitDecision {
        let mut windows = self.windows.lock().expect("rate limiter poisoned");
        let window = windows.entry(key.to_string()).or_insert(Window {
            count: 0,
            start: now_secs,
        });
        if now_secs.saturating_sub(window.start) >= self.config.window_secs {
            window.count = 0;
            window.start = now_secs;
        }
        let elapsed = now_secs.saturating_sub(window.start);
        let reset_after_secs = self.config.window_secs.saturating_sub(elapsed);
        let allowed = window.count < self.config.limit;
        if allowed {
            window.count += 1;
        }
        let remaining = self.config.limit.saturating_sub(window.count);
        RateLimitDecision {
            allowed,
            limit: self.config.limit,
            remaining,
            reset_after_secs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_the_limit_then_blocks() {
        let limiter = RateLimiter::new(RateLimitConfig {
            limit: 2,
            window_secs: 10,
        });
        assert!(limiter.check("k", 0).allowed);
        let second = limiter.check("k", 0);
        assert!(second.allowed);
        assert_eq!(second.remaining, 0);
        let third = limiter.check("k", 0);
        assert!(!third.allowed);
        assert_eq!(third.reset_after_secs, 10);
    }

    #[test]
    fn resets_after_the_window() {
        let limiter = RateLimiter::new(RateLimitConfig {
            limit: 1,
            window_secs: 10,
        });
        assert!(limiter.check("k", 0).allowed);
        assert!(!limiter.check("k", 5).allowed);
        assert!(limiter.check("k", 10).allowed);
    }

    #[test]
    fn keys_are_independent() {
        let limiter = RateLimiter::new(RateLimitConfig {
            limit: 1,
            window_secs: 10,
        });
        assert!(limiter.check("a", 0).allowed);
        assert!(limiter.check("b", 0).allowed);
        assert!(!limiter.check("a", 0).allowed);
    }
}
