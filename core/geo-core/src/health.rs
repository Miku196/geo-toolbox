//! Health probe with TTL caching — O(1) state reads, decoupled from network I/O.
//!
//! Each adapter holds a `CachedHealth` that stores the last known healthy state
//! and a TTL. Callers run `probe_ping()` periodically (server's background task),
//! and all `is_healthy()` reads return the cached value instantly.
//!
//! # Example
//!
//! ```rust,ignore
//! struct MyAdapter {
//!     health: CachedHealth,
//!     store: MyStore,
//! }
//!
//! impl ExternalAdapter for MyAdapter {
//!     fn health(&self) -> &CachedHealth { &self.health }
//!     fn is_healthy(&self) -> bool { self.health.is_ok() }
//!     async fn health_check(&self) -> GeoResult<bool> {
//!         let healthy = self.store.ping().is_ok();
//!         self.health.record(healthy);
//!         Ok(healthy)
//!     }
//! }
//! ```

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// A health probe with TTL caching. Thread-safe via atomics.
///
/// Writes allowed after creation; reads are lock-free.
#[derive(Debug)]
pub struct CachedHealth {
    /// Last successful check result.
    healthy: AtomicBool,
    /// Timestamp of the last successful check.
    last_ok_at: std::sync::Mutex<Option<Instant>>,
    /// TTL after which `is_ok()` considers the cached result stale.
    ttl: Duration,
    /// Timestamp of the last check (successful or not).
    last_checked_at: std::sync::Mutex<Option<Instant>>,
}

impl CachedHealth {
    /// Create a health probe with the given TTL. Initially unhealthy.
    pub fn new(ttl: Duration) -> Self {
        Self {
            healthy: AtomicBool::new(false),
            last_ok_at: std::sync::Mutex::new(None),
            ttl,
            last_checked_at: std::sync::Mutex::new(None),
        }
    }

    /// Default 30-second TTL (matching typical container health check interval).
    pub fn with_default_ttl() -> Self {
        Self::new(Duration::from_secs(30))
    }

    /// Returns `true` if the last recorded check was healthy AND within the TTL.
    pub fn is_ok(&self) -> bool {
        if !self.healthy.load(Ordering::Acquire) {
            return false;
        }
        let last_ok = self.last_ok_at.lock().unwrap();
        match *last_ok {
            Some(ts) => ts.elapsed() < self.ttl,
            None => false,
        }
    }

    /// Returns the elapsed time since the last check (None = never checked).
    pub fn elapsed_since_last_check(&self) -> Option<Duration> {
        self.last_checked_at.lock().unwrap().map(|ts| ts.elapsed())
    }

    /// Record a health check result. Called by the background probe task.
    pub fn record(&self, healthy: bool) {
        let now = Instant::now();
        if healthy {
            *self.last_ok_at.lock().unwrap() = Some(now);
        }
        *self.last_checked_at.lock().unwrap() = Some(now);
        self.healthy.store(healthy, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_unhealthy() {
        let h = CachedHealth::new(Duration::from_secs(30));
        assert!(!h.is_ok());
    }

    #[test]
    fn test_healthy_within_ttl() {
        let h = CachedHealth::new(Duration::from_secs(30));
        h.record(true);
        assert!(h.is_ok());
    }

    #[test]
    fn test_stale_after_ttl() {
        let h = CachedHealth::new(Duration::from_millis(1));
        h.record(true);
        std::thread::sleep(Duration::from_millis(2));
        assert!(!h.is_ok());
    }

    #[test]
    fn test_record_false() {
        let h = CachedHealth::new(Duration::from_secs(30));
        h.record(true);
        h.record(false);
        assert!(!h.is_ok());
    }
}
