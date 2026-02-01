//! Rate Limiter — Prevent action oscillation
//!
//! The rate limiter tracks action frequency and enforces cooldown
//! periods to prevent flip-flop behavior and system oscillation.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::*;
// ActionTarget is now in types/envelope.rs

// ============================================================================
// RATE LIMIT
// ============================================================================

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimit {
    /// Maximum actions per window
    pub max_actions: u32,
    /// Window duration
    pub window: Duration,
    /// Cooldown after action
    pub cooldown: Duration,
}

impl Default for RateLimit {
    fn default() -> Self {
        Self {
            max_actions: 10,
            window: Duration::from_secs(60),
            cooldown: Duration::from_secs(5),
        }
    }
}

impl RateLimit {
    /// Create new rate limit
    pub fn new(max_actions: u32, window: Duration, cooldown: Duration) -> Self {
        Self {
            max_actions,
            window,
            cooldown,
        }
    }

    /// Create strict limit
    pub fn strict() -> Self {
        Self {
            max_actions: 3,
            window: Duration::from_secs(60),
            cooldown: Duration::from_secs(30),
        }
    }

    /// Create lenient limit
    pub fn lenient() -> Self {
        Self {
            max_actions: 100,
            window: Duration::from_secs(60),
            cooldown: Duration::from_secs(1),
        }
    }
}

// ============================================================================
// RATE LIMIT RESULT
// ============================================================================

/// Rate limit result
#[derive(Debug, Clone)]
pub struct RateLimitResult {
    /// Is action allowed
    pub allowed: bool,
    /// Reason for denial
    pub reason: Option<RateLimitReason>,
    /// When to retry
    pub retry_after: Option<Duration>,
}

impl RateLimitResult {
    /// Create allowed result
    pub fn allowed() -> Self {
        Self {
            allowed: true,
            reason: None,
            retry_after: None,
        }
    }

    /// Create denied result
    pub fn denied(reason: RateLimitReason, retry_after: Duration) -> Self {
        Self {
            allowed: false,
            reason: Some(reason),
            retry_after: Some(retry_after),
        }
    }
}

/// Rate limit reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitReason {
    /// Rate limit exceeded
    RateExceeded,
    /// Cooldown period
    Cooldown,
    /// Flip-flop detected
    FlipFlop,
}

impl RateLimitReason {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::RateExceeded => "Rate Exceeded",
            Self::Cooldown => "Cooldown",
            Self::FlipFlop => "Flip-Flop Detected",
        }
    }
}

// ============================================================================
// ACTION HISTORY
// ============================================================================

/// Action history entry
#[derive(Debug, Clone)]
struct ActionHistoryEntry {
    action_type: ActionType,
    target: String,
    timestamp: Timestamp,
}

// ============================================================================
// RATE LIMITER
// ============================================================================

/// Rate limiter for actions
pub struct RateLimiter {
    /// Rate limits per action type
    limits: BTreeMap<ActionType, RateLimit>,
    /// Action history
    history: Vec<ActionHistoryEntry>,
    /// Maximum history size
    max_history: usize,
    /// Actions throttled
    throttled: AtomicU64,
}

impl RateLimiter {
    /// Create new rate limiter
    pub fn new() -> Self {
        let mut limits = BTreeMap::new();

        // Default rate limits
        limits.insert(
            ActionType::Restart,
            RateLimit {
                max_actions: 5,
                window: Duration::from_secs(60),
                cooldown: Duration::from_secs(10),
            },
        );

        limits.insert(
            ActionType::Kill,
            RateLimit {
                max_actions: 3,
                window: Duration::from_secs(60),
                cooldown: Duration::from_secs(30),
            },
        );

        limits.insert(
            ActionType::Migrate,
            RateLimit {
                max_actions: 2,
                window: Duration::from_secs(300),
                cooldown: Duration::from_secs(60),
            },
        );

        Self {
            limits,
            history: Vec::new(),
            max_history: 1000,
            throttled: AtomicU64::new(0),
        }
    }

    /// Create with custom history size
    pub fn with_history_size(max_history: usize) -> Self {
        let mut limiter = Self::new();
        limiter.max_history = max_history;
        limiter
    }

    /// Set rate limit for action type
    pub fn set_limit(&mut self, action_type: ActionType, limit: RateLimit) {
        self.limits.insert(action_type, limit);
    }

    /// Remove rate limit for action type
    pub fn remove_limit(&mut self, action_type: ActionType) -> Option<RateLimit> {
        self.limits.remove(&action_type)
    }

    /// Check if action is allowed
    pub fn check(&self, action_type: ActionType, target: &ActionTarget, now: Timestamp) -> RateLimitResult {
        let target_str = target_to_string(target);

        // Get rate limit for this action type
        let limit = self.limits.get(&action_type).cloned().unwrap_or_default();

        // Check cooldown
        let last_action = self
            .history
            .iter()
            .rev()
            .find(|e| e.action_type == action_type && e.target == target_str);

        if let Some(entry) = last_action {
            let elapsed = now.elapsed_since(entry.timestamp);
            if elapsed.as_nanos() < limit.cooldown.as_nanos() {
                let remaining = limit.cooldown.as_nanos() - elapsed.as_nanos();
                return RateLimitResult {
                    allowed: false,
                    reason: Some(RateLimitReason::Cooldown),
                    retry_after: Some(Duration::from_nanos(remaining)),
                };
            }
        }

        // Check rate limit
        let window_start = Timestamp::new(now.as_nanos().saturating_sub(limit.window.as_nanos()));
        let actions_in_window = self
            .history
            .iter()
            .filter(|e| e.action_type == action_type && e.timestamp.as_nanos() >= window_start.as_nanos())
            .count();

        if actions_in_window >= limit.max_actions as usize {
            return RateLimitResult {
                allowed: false,
                reason: Some(RateLimitReason::RateExceeded),
                retry_after: Some(limit.window),
            };
        }

        RateLimitResult {
            allowed: true,
            reason: None,
            retry_after: None,
        }
    }

    /// Record an action
    pub fn record(&mut self, action_type: ActionType, target: &ActionTarget, timestamp: Timestamp) {
        self.history.push(ActionHistoryEntry {
            action_type,
            target: target_to_string(target),
            timestamp,
        });

        // Trim history if too large
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }

    /// Clear history
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Get history size
    pub fn history_size(&self) -> usize {
        self.history.len()
    }

    /// Get actions in last N seconds
    pub fn recent_actions(&self, seconds: u64, now: Timestamp) -> usize {
        let window_start = Timestamp::new(now.as_nanos().saturating_sub(Duration::from_secs(seconds).as_nanos()));
        self.history
            .iter()
            .filter(|e| e.timestamp.as_nanos() >= window_start.as_nanos())
            .count()
    }

    /// Get statistics
    pub fn stats(&self) -> RateLimiterStats {
        RateLimiterStats {
            history_size: self.history.len(),
            throttled: self.throttled.load(Ordering::Relaxed),
            limits_configured: self.limits.len(),
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Rate limiter statistics
#[derive(Debug, Clone)]
pub struct RateLimiterStats {
    /// Current history size
    pub history_size: usize,
    /// Total actions throttled
    pub throttled: u64,
    /// Number of limits configured
    pub limits_configured: usize,
}

// ============================================================================
// HELPER
// ============================================================================

/// Convert target to string
pub fn target_to_string(target: &ActionTarget) -> String {
    match target {
        ActionTarget::System => String::from("system"),
        ActionTarget::Cpu(id) => format!("cpu:{}", id),
        ActionTarget::Process(pid) => format!("process:{}", pid),
        ActionTarget::Thread { pid, tid } => format!("thread:{}:{}", pid, tid),
        ActionTarget::Device(d) => format!("device:{}", d),
        ActionTarget::Network(n) => format!("network:{}", n),
        ActionTarget::Filesystem(f) => format!("fs:{}", f),
        ActionTarget::Module(m) => format!("module:{}", m),
        ActionTarget::Memory { start, size } => format!("memory:{}:{}", start, size),
        ActionTarget::Custom(c) => format!("custom:{}", c),
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new();
        let result = limiter.check(
            ActionType::Restart,
            &ActionTarget::System,
            Timestamp::now(),
        );
        assert!(result.allowed);
    }

    #[test]
    fn test_rate_limit_config() {
        let limit = RateLimit::strict();
        assert_eq!(limit.max_actions, 3);
    }

    #[test]
    fn test_record_action() {
        let mut limiter = RateLimiter::new();
        limiter.record(ActionType::Restart, &ActionTarget::System, Timestamp::now());
        assert_eq!(limiter.history_size(), 1);
    }

    #[test]
    fn test_target_to_string() {
        let target = ActionTarget::Process(123);
        let s = target_to_string(&target);
        assert_eq!(s, "process:123");
    }
}
