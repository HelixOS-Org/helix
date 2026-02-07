//! # Bridge Gateway System
//!
//! Syscall gateway and entry point management:
//! - API versioning for syscall interfaces
//! - Feature gating per caller
//! - Rate limiting at entry
//! - Request normalization
//! - Protocol negotiation

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// GATEWAY TYPES
// ============================================================================

/// API version
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ApiVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl ApiVersion {
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self { major, minor, patch }
    }

    /// Is compatible with (same major, >= minor)
    pub fn is_compatible_with(&self, other: &ApiVersion) -> bool {
        self.major == other.major && self.minor >= other.minor
    }
}

/// Gateway state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewayState {
    /// Accepting requests
    Open,
    /// Rate limited
    Throttled,
    /// Maintenance mode
    Maintenance,
    /// Closed
    Closed,
}

/// Feature flag
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FeatureFlag {
    /// Async I/O
    AsyncIo,
    /// Io_uring style
    IoUring,
    /// Extended attributes
    ExtendedAttrs,
    /// Memory mapping extensions
    MmapExtended,
    /// Security sandboxing
    Sandboxing,
    /// Performance monitoring
    PerfMonitor,
    /// Hot reload
    HotReload,
    /// Debug tracing
    DebugTrace,
}

// ============================================================================
// RATE LIMITER
// ============================================================================

/// Token bucket rate limiter
#[derive(Debug, Clone)]
pub struct GatewayRateLimiter {
    /// Tokens available
    pub tokens: f64,
    /// Max tokens
    pub max_tokens: f64,
    /// Refill rate (tokens per ns)
    pub refill_rate: f64,
    /// Last refill timestamp
    pub last_refill: u64,
    /// Total allowed
    pub total_allowed: u64,
    /// Total denied
    pub total_denied: u64,
}

impl GatewayRateLimiter {
    pub fn new(max_tokens: f64, rate_per_sec: f64) -> Self {
        Self {
            tokens: max_tokens,
            max_tokens,
            refill_rate: rate_per_sec / 1_000_000_000.0,
            last_refill: 0,
            total_allowed: 0,
            total_denied: 0,
        }
    }

    /// Try to consume a token
    pub fn try_consume(&mut self, now: u64) -> bool {
        self.refill(now);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            self.total_allowed += 1;
            true
        } else {
            self.total_denied += 1;
            false
        }
    }

    /// Refill tokens
    fn refill(&mut self, now: u64) {
        let elapsed = now.saturating_sub(self.last_refill);
        if elapsed > 0 {
            self.tokens += elapsed as f64 * self.refill_rate;
            if self.tokens > self.max_tokens {
                self.tokens = self.max_tokens;
            }
            self.last_refill = now;
        }
    }

    /// Utilization (denied / total)
    pub fn denial_rate(&self) -> f64 {
        let total = self.total_allowed + self.total_denied;
        if total == 0 {
            return 0.0;
        }
        self.total_denied as f64 / total as f64
    }
}

// ============================================================================
// CALLER PROFILE
// ============================================================================

/// Caller capabilities/features
#[derive(Debug)]
pub struct CallerProfile {
    /// Process id
    pub pid: u64,
    /// API version
    pub api_version: ApiVersion,
    /// Enabled features (bitmask)
    pub features: u64,
    /// Rate limiter
    pub rate_limiter: GatewayRateLimiter,
    /// Registered at
    pub registered_at: u64,
    /// Total requests
    pub total_requests: u64,
    /// Blocked requests
    pub blocked_requests: u64,
}

impl CallerProfile {
    pub fn new(pid: u64, api_version: ApiVersion, rate_per_sec: f64, now: u64) -> Self {
        Self {
            pid,
            api_version,
            features: 0,
            rate_limiter: GatewayRateLimiter::new(rate_per_sec * 2.0, rate_per_sec),
            registered_at: now,
            total_requests: 0,
            blocked_requests: 0,
        }
    }

    /// Enable feature
    pub fn enable_feature(&mut self, feature: FeatureFlag) {
        self.features |= 1u64 << (feature as u8);
    }

    /// Disable feature
    pub fn disable_feature(&mut self, feature: FeatureFlag) {
        self.features &= !(1u64 << (feature as u8));
    }

    /// Has feature?
    pub fn has_feature(&self, feature: FeatureFlag) -> bool {
        self.features & (1u64 << (feature as u8)) != 0
    }

    /// Process request
    pub fn process_request(&mut self, now: u64) -> bool {
        self.total_requests += 1;
        if self.rate_limiter.try_consume(now) {
            true
        } else {
            self.blocked_requests += 1;
            false
        }
    }
}

// ============================================================================
// GATEWAY ENGINE
// ============================================================================

/// Gateway stats
#[derive(Debug, Clone, Default)]
pub struct BridgeGatewayStats {
    /// Registered callers
    pub registered_callers: usize,
    /// Total requests
    pub total_requests: u64,
    /// Total blocked
    pub total_blocked: u64,
    /// Current state
    pub is_open: bool,
}

/// Bridge gateway manager
pub struct BridgeGatewayManager {
    /// Current state
    pub state: GatewayState,
    /// Supported API version
    pub current_version: ApiVersion,
    /// Min supported version
    pub min_version: ApiVersion,
    /// Caller profiles
    callers: BTreeMap<u64, CallerProfile>,
    /// Global rate limiter
    global_limiter: GatewayRateLimiter,
    /// Stats
    stats: BridgeGatewayStats,
}

impl BridgeGatewayManager {
    pub fn new(current_version: ApiVersion, global_rate_per_sec: f64) -> Self {
        Self {
            state: GatewayState::Open,
            current_version,
            min_version: ApiVersion::new(1, 0, 0),
            callers: BTreeMap::new(),
            global_limiter: GatewayRateLimiter::new(global_rate_per_sec * 2.0, global_rate_per_sec),
            stats: BridgeGatewayStats::default(),
        }
    }

    /// Register caller
    pub fn register(&mut self, pid: u64, version: ApiVersion, rate_per_sec: f64, now: u64) -> bool {
        if !version.is_compatible_with(&self.min_version) {
            return false;
        }
        let profile = CallerProfile::new(pid, version, rate_per_sec, now);
        self.callers.insert(pid, profile);
        self.update_stats();
        true
    }

    /// Unregister caller
    pub fn unregister(&mut self, pid: u64) {
        self.callers.remove(&pid);
        self.update_stats();
    }

    /// Process incoming request
    pub fn process_request(&mut self, pid: u64, now: u64) -> bool {
        // Check gateway state
        if self.state != GatewayState::Open && self.state != GatewayState::Throttled {
            self.stats.total_blocked += 1;
            return false;
        }

        // Check global limit
        if !self.global_limiter.try_consume(now) {
            self.state = GatewayState::Throttled;
            self.stats.total_blocked += 1;
            return false;
        }

        // Check per-caller limit
        if let Some(caller) = self.callers.get_mut(&pid) {
            self.stats.total_requests += 1;
            let result = caller.process_request(now);
            if !result {
                self.stats.total_blocked += 1;
            }
            result
        } else {
            // Unregistered caller
            self.stats.total_blocked += 1;
            false
        }
    }

    /// Enable feature for caller
    pub fn enable_feature(&mut self, pid: u64, feature: FeatureFlag) {
        if let Some(caller) = self.callers.get_mut(&pid) {
            caller.enable_feature(feature);
        }
    }

    /// Set gateway state
    pub fn set_state(&mut self, state: GatewayState) {
        self.state = state;
        self.stats.is_open = state == GatewayState::Open;
    }

    /// Get caller profile
    pub fn caller(&self, pid: u64) -> Option<&CallerProfile> {
        self.callers.get(&pid)
    }

    fn update_stats(&mut self) {
        self.stats.registered_callers = self.callers.len();
        self.stats.is_open = self.state == GatewayState::Open;
    }

    /// Stats
    pub fn stats(&self) -> &BridgeGatewayStats {
        &self.stats
    }
}
