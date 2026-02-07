//! # Syscall Security & Rate Limiting
//!
//! Security layer in the syscall pipeline that enforces:
//! - Per-process syscall rate limits
//! - Syscall filtering (seccomp-like)
//! - Anomalous pattern detection
//! - Privilege escalation detection
//! - Resource abuse prevention

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::syscall::SyscallType;

// ============================================================================
// SECURITY POLICY
// ============================================================================

/// Action to take when a security rule matches
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityAction {
    /// Allow the syscall
    Allow,
    /// Deny the syscall with error
    Deny,
    /// Allow but log
    AllowLog,
    /// Allow but rate-limit
    RateLimit,
    /// Kill the process
    Kill,
    /// Redirect to sandbox
    Sandbox,
    /// Trap for inspection
    Trap,
}

/// A security rule for syscall filtering
#[derive(Debug, Clone)]
pub struct SecurityRule {
    /// Rule ID
    pub id: u32,
    /// Rule name
    pub name: &'static str,
    /// Syscall type to match (None = match all)
    pub syscall_type: Option<SyscallType>,
    /// Argument constraints (index → allowed range)
    pub arg_constraints: Vec<ArgConstraint>,
    /// Action when matched
    pub action: SecurityAction,
    /// Priority (lower = checked first)
    pub priority: u8,
    /// Whether this rule is enabled
    pub enabled: bool,
    /// Times this rule matched
    pub match_count: u64,
}

/// Constraint on a syscall argument
#[derive(Debug, Clone, Copy)]
pub struct ArgConstraint {
    /// Argument index (0-5)
    pub arg_index: u8,
    /// Constraint type
    pub constraint: ConstraintType,
}

/// Types of argument constraints
#[derive(Debug, Clone, Copy)]
pub enum ConstraintType {
    /// Must equal this value
    Equals(u64),
    /// Must not equal this value
    NotEquals(u64),
    /// Must be less than
    LessThan(u64),
    /// Must be greater than
    GreaterThan(u64),
    /// Must be within range [min, max]
    InRange(u64, u64),
    /// Must have these bits set (mask)
    BitMask(u64),
    /// Must NOT have these bits set
    NoBitMask(u64),
}

impl ConstraintType {
    /// Check if a value satisfies this constraint
    pub fn check(&self, value: u64) -> bool {
        match *self {
            Self::Equals(v) => value == v,
            Self::NotEquals(v) => value != v,
            Self::LessThan(v) => value < v,
            Self::GreaterThan(v) => value > v,
            Self::InRange(min, max) => value >= min && value <= max,
            Self::BitMask(mask) => value & mask == mask,
            Self::NoBitMask(mask) => value & mask == 0,
        }
    }
}

impl SecurityRule {
    pub fn new(
        id: u32,
        name: &'static str,
        syscall_type: Option<SyscallType>,
        action: SecurityAction,
    ) -> Self {
        Self {
            id,
            name,
            syscall_type,
            arg_constraints: Vec::new(),
            action,
            priority: 5,
            enabled: true,
            match_count: 0,
        }
    }

    pub fn with_constraint(mut self, constraint: ArgConstraint) -> Self {
        self.arg_constraints.push(constraint);
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Check if this rule matches a syscall
    pub fn matches(&self, syscall_type: SyscallType, args: &[u64; 6]) -> bool {
        if !self.enabled {
            return false;
        }

        // Check syscall type
        if let Some(expected) = self.syscall_type {
            if expected != syscall_type {
                return false;
            }
        }

        // Check argument constraints
        for constraint in &self.arg_constraints {
            let idx = constraint.arg_index as usize;
            if idx < 6 && !constraint.constraint.check(args[idx]) {
                return false;
            }
        }

        true
    }
}

// ============================================================================
// RATE LIMITER
// ============================================================================

/// Rate limit configuration
#[derive(Debug, Clone, Copy)]
pub struct RateLimitConfig {
    /// Maximum syscalls per window
    pub max_per_window: u64,
    /// Window size in milliseconds
    pub window_ms: u64,
    /// Burst allowance above the limit
    pub burst_allowance: u64,
    /// Penalty duration when exceeded (ms)
    pub penalty_ms: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_per_window: 100_000, // 100K syscalls per window
            window_ms: 1000,         // 1 second window
            burst_allowance: 10_000, // 10K burst
            penalty_ms: 100,         // 100ms penalty
        }
    }
}

/// Per-process rate limiter state
#[derive(Debug, Clone)]
struct ProcessRateState {
    /// Syscall count in current window
    count: u64,
    /// Window start timestamp
    window_start: u64,
    /// Whether currently penalized
    penalized: bool,
    /// Penalty end time
    penalty_until: u64,
    /// Total times rate-limited
    rate_limited_count: u64,
    /// Per-type counters
    per_type: BTreeMap<u8, u64>,
}

impl ProcessRateState {
    fn new(window_start: u64) -> Self {
        Self {
            count: 0,
            window_start,
            penalized: false,
            penalty_until: 0,
            rate_limited_count: 0,
            per_type: BTreeMap::new(),
        }
    }

    fn reset_window(&mut self, new_start: u64) {
        self.count = 0;
        self.window_start = new_start;
        self.per_type.clear();
    }
}

/// Rate limiter
pub struct RateLimiter {
    /// Configuration
    config: RateLimitConfig,
    /// Per-process state
    processes: BTreeMap<u64, ProcessRateState>,
    /// Per-type global rate limits
    type_limits: BTreeMap<u8, u64>,
    /// Global counters
    total_allowed: u64,
    total_rate_limited: u64,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            processes: BTreeMap::new(),
            type_limits: BTreeMap::new(),
            total_allowed: 0,
            total_rate_limited: 0,
        }
    }

    /// Set a per-type rate limit
    pub fn set_type_limit(&mut self, syscall_type: SyscallType, max_per_window: u64) {
        self.type_limits.insert(syscall_type as u8, max_per_window);
    }

    /// Check if a syscall should be rate-limited
    pub fn check(
        &mut self,
        pid: u64,
        syscall_type: SyscallType,
        current_time: u64,
    ) -> SecurityAction {
        let state = self
            .processes
            .entry(pid)
            .or_insert_with(|| ProcessRateState::new(current_time));

        // Check penalty
        if state.penalized && current_time < state.penalty_until {
            self.total_rate_limited += 1;
            return SecurityAction::Deny;
        }
        state.penalized = false;

        // Check window rollover
        if current_time.saturating_sub(state.window_start) >= self.config.window_ms {
            state.reset_window(current_time);
        }

        state.count += 1;
        *state.per_type.entry(syscall_type as u8).or_insert(0) += 1;

        // Check per-type limit
        if let Some(&type_limit) = self.type_limits.get(&(syscall_type as u8)) {
            if *state.per_type.get(&(syscall_type as u8)).unwrap_or(&0) > type_limit {
                state.penalized = true;
                state.penalty_until = current_time + self.config.penalty_ms;
                state.rate_limited_count += 1;
                self.total_rate_limited += 1;
                return SecurityAction::RateLimit;
            }
        }

        // Check global per-process limit
        let limit = self.config.max_per_window + self.config.burst_allowance;
        if state.count > limit {
            state.penalized = true;
            state.penalty_until = current_time + self.config.penalty_ms;
            state.rate_limited_count += 1;
            self.total_rate_limited += 1;
            return SecurityAction::RateLimit;
        }

        self.total_allowed += 1;
        SecurityAction::Allow
    }

    /// Remove process state
    pub fn remove_process(&mut self, pid: u64) {
        self.processes.remove(&pid);
    }

    /// Get stats
    pub fn stats(&self) -> (u64, u64) {
        (self.total_allowed, self.total_rate_limited)
    }
}

// ============================================================================
// ANOMALY DETECTOR (syscall-level)
// ============================================================================

/// Types of syscall-level anomalies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallAnomaly {
    /// Unusually high syscall rate
    HighRate,
    /// Unusual syscall type for this process
    UnusualType,
    /// Arguments outside normal range
    AbnormalArgs,
    /// Potential privilege escalation attempt
    PrivilegeEscalation,
    /// Potential sandbox escape
    SandboxEscape,
    /// Brute-force file access pattern
    BruteForceAccess,
    /// Unusual error rate
    HighErrorRate,
    /// Rapid file descriptor churn
    FdChurn,
}

/// An anomaly detection event
#[derive(Debug, Clone)]
pub struct AnomalyEvent {
    /// Anomaly type
    pub anomaly: SyscallAnomaly,
    /// Process ID
    pub pid: u64,
    /// Severity (0.0 - 1.0)
    pub severity: f64,
    /// Timestamp
    pub timestamp: u64,
    /// Recommended action
    pub recommended_action: SecurityAction,
    /// Evidence description
    pub evidence: &'static str,
}

/// Syscall anomaly detector
pub struct SyscallAnomalyDetector {
    /// Per-process normal behavior profiles
    profiles: BTreeMap<u64, SyscallBehaviorProfile>,
    /// Detected anomalies
    anomalies: Vec<AnomalyEvent>,
    /// Max anomalies to store
    max_anomalies: usize,
    /// Detection sensitivity (0.0 = lenient, 1.0 = strict)
    sensitivity: f64,
}

/// Normal behavior profile for a process
#[derive(Debug, Clone)]
struct SyscallBehaviorProfile {
    /// Expected syscall types (frequency)
    expected_types: BTreeMap<u8, f64>,
    /// Expected rate (per second)
    expected_rate: f64,
    /// Expected error rate
    expected_error_rate: f64,
    /// Sample count
    samples: u64,
    /// Last update timestamp
    last_update: u64,
}

impl SyscallBehaviorProfile {
    fn new() -> Self {
        Self {
            expected_types: BTreeMap::new(),
            expected_rate: 0.0,
            expected_error_rate: 0.0,
            samples: 0,
            last_update: 0,
        }
    }

    fn update(&mut self, syscall_type: SyscallType, rate: f64, timestamp: u64) {
        self.samples += 1;
        let alpha = 0.1; // EMA alpha

        // Update type frequency
        let entry = self.expected_types.entry(syscall_type as u8).or_insert(0.0);
        *entry = *entry * (1.0 - alpha) + alpha;

        // Decay others
        let key = syscall_type as u8;
        for (k, v) in self.expected_types.iter_mut() {
            if *k != key {
                *v *= 1.0 - alpha * 0.1;
            }
        }

        // Update rate
        self.expected_rate = self.expected_rate * (1.0 - alpha) + rate * alpha;
        self.last_update = timestamp;
    }
}

impl SyscallAnomalyDetector {
    pub fn new(sensitivity: f64) -> Self {
        Self {
            profiles: BTreeMap::new(),
            anomalies: Vec::new(),
            max_anomalies: 1024,
            sensitivity: sensitivity.clamp(0.0, 1.0),
        }
    }

    /// Check a syscall for anomalies
    pub fn check(
        &mut self,
        pid: u64,
        syscall_type: SyscallType,
        _args: &[u64; 6],
        rate: f64,
        timestamp: u64,
    ) -> Vec<AnomalyEvent> {
        let mut events = Vec::new();

        let profile = self
            .profiles
            .entry(pid)
            .or_insert_with(SyscallBehaviorProfile::new);

        // Need enough samples to detect anomalies
        if profile.samples > 100 {
            // High rate check
            let rate_threshold = profile.expected_rate * (3.0 - self.sensitivity * 2.0);
            if rate > rate_threshold && profile.expected_rate > 10.0 {
                let severity = ((rate / rate_threshold) - 1.0).min(1.0);
                events.push(AnomalyEvent {
                    anomaly: SyscallAnomaly::HighRate,
                    pid,
                    severity,
                    timestamp,
                    recommended_action: if severity > 0.8 {
                        SecurityAction::RateLimit
                    } else {
                        SecurityAction::AllowLog
                    },
                    evidence: "syscall rate significantly above baseline",
                });
            }

            // Unusual type check
            let type_freq = profile
                .expected_types
                .get(&(syscall_type as u8))
                .copied()
                .unwrap_or(0.0);
            if type_freq < 0.01 * self.sensitivity && profile.samples > 500 {
                events.push(AnomalyEvent {
                    anomaly: SyscallAnomaly::UnusualType,
                    pid,
                    severity: 0.5,
                    timestamp,
                    recommended_action: SecurityAction::AllowLog,
                    evidence: "rarely-seen syscall type for this process",
                });
            }

            // Privilege escalation heuristics
            if matches!(syscall_type, SyscallType::Exec) && rate > 10.0 {
                events.push(AnomalyEvent {
                    anomaly: SyscallAnomaly::PrivilegeEscalation,
                    pid,
                    severity: 0.7,
                    timestamp,
                    recommended_action: SecurityAction::Trap,
                    evidence: "high rate of exec syscalls may indicate exploitation",
                });
            }
        }

        // Update profile
        profile.update(syscall_type, rate, timestamp);

        // Store anomalies
        for event in &events {
            if self.anomalies.len() >= self.max_anomalies {
                self.anomalies.remove(0);
            }
            self.anomalies.push(event.clone());
        }

        events
    }

    /// Get recent anomalies for a process
    pub fn anomalies_for(&self, pid: u64) -> Vec<&AnomalyEvent> {
        self.anomalies.iter().filter(|a| a.pid == pid).collect()
    }

    /// Get all recent anomalies
    pub fn recent_anomalies(&self, count: usize) -> &[AnomalyEvent] {
        let start = self.anomalies.len().saturating_sub(count);
        &self.anomalies[start..]
    }

    /// Remove process profile
    pub fn remove_process(&mut self, pid: u64) {
        self.profiles.remove(&pid);
    }

    /// Total anomalies detected
    pub fn total_anomalies(&self) -> usize {
        self.anomalies.len()
    }
}

// ============================================================================
// SECURITY ENGINE (combines all security checks)
// ============================================================================

/// The syscall security engine
pub struct SecurityEngine {
    /// Security rules
    rules: Vec<SecurityRule>,
    /// Rate limiter
    rate_limiter: RateLimiter,
    /// Anomaly detector
    anomaly_detector: SyscallAnomalyDetector,
    /// Total checks performed
    total_checks: u64,
    /// Total blocks
    total_blocks: u64,
    /// Total alerts
    total_alerts: u64,
}

impl SecurityEngine {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            rate_limiter: RateLimiter::new(RateLimitConfig::default()),
            anomaly_detector: SyscallAnomalyDetector::new(0.5),
            total_checks: 0,
            total_blocks: 0,
            total_alerts: 0,
        }
    }

    /// Add a security rule
    pub fn add_rule(&mut self, rule: SecurityRule) {
        self.rules.push(rule);
        self.rules.sort_by_key(|r| r.priority);
    }

    /// Check a syscall against all security layers
    pub fn check(
        &mut self,
        pid: u64,
        syscall_type: SyscallType,
        args: &[u64; 6],
        rate: f64,
        timestamp: u64,
    ) -> SecurityAction {
        self.total_checks += 1;

        // Layer 1: Rule-based filtering
        for rule in &mut self.rules {
            if rule.matches(syscall_type, args) {
                rule.match_count += 1;
                match rule.action {
                    SecurityAction::Deny | SecurityAction::Kill => {
                        self.total_blocks += 1;
                        return rule.action;
                    },
                    SecurityAction::AllowLog | SecurityAction::Trap => {
                        self.total_alerts += 1;
                        return rule.action;
                    },
                    _ => {},
                }
            }
        }

        // Layer 2: Rate limiting
        let rate_result = self.rate_limiter.check(pid, syscall_type, timestamp);
        if rate_result != SecurityAction::Allow {
            self.total_blocks += 1;
            return rate_result;
        }

        // Layer 3: Anomaly detection
        let anomalies = self
            .anomaly_detector
            .check(pid, syscall_type, args, rate, timestamp);
        if let Some(worst) = anomalies.iter().max_by(|a, b| {
            a.severity
                .partial_cmp(&b.severity)
                .unwrap_or(core::cmp::Ordering::Equal)
        }) {
            if worst.severity > 0.8 {
                self.total_alerts += 1;
                return worst.recommended_action;
            }
        }

        SecurityAction::Allow
    }

    /// Get the rate limiter
    pub fn rate_limiter(&mut self) -> &mut RateLimiter {
        &mut self.rate_limiter
    }

    /// Get the anomaly detector
    pub fn anomaly_detector(&mut self) -> &mut SyscallAnomalyDetector {
        &mut self.anomaly_detector
    }

    /// Get statistics
    pub fn stats(&self) -> (u64, u64, u64) {
        (self.total_checks, self.total_blocks, self.total_alerts)
    }
}
