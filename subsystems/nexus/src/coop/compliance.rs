//! # Contract Compliance Monitoring
//!
//! Monitors and enforces cooperative contracts:
//! - SLA monitoring (latency, throughput, availability)
//! - Violation detection and classification
//! - Penalty calculation
//! - Grace period management
//! - Compliance history and trends
//! - Auto-adjustment recommendations

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// SLA METRICS
// ============================================================================

/// SLA metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SlaMetric {
    /// Maximum response latency (microseconds)
    MaxLatency,
    /// Minimum throughput (ops/sec)
    MinThroughput,
    /// Maximum CPU usage (percent * 100)
    MaxCpuUsage,
    /// Maximum memory usage (bytes)
    MaxMemoryUsage,
    /// Maximum I/O bandwidth (bytes/sec)
    MaxIoBandwidth,
    /// Minimum availability (percent * 100)
    MinAvailability,
    /// Maximum error rate (per 10000)
    MaxErrorRate,
    /// Maximum FD count
    MaxFdCount,
    /// Maximum syscall rate (per second)
    MaxSyscallRate,
    /// Custom metric
    Custom,
}

/// SLA bound
#[derive(Debug, Clone, Copy)]
pub struct SlaBound {
    /// Metric
    pub metric: SlaMetric,
    /// Warning threshold
    pub warning: u64,
    /// Violation threshold
    pub violation: u64,
    /// Critical threshold
    pub critical: u64,
}

impl SlaBound {
    pub fn new(metric: SlaMetric, warning: u64, violation: u64, critical: u64) -> Self {
        Self {
            metric,
            warning,
            violation,
            critical,
        }
    }

    /// Check level
    pub fn check(&self, value: u64) -> ComplianceLevel {
        match self.metric {
            // For "max" metrics, higher value = worse
            SlaMetric::MaxLatency
            | SlaMetric::MaxCpuUsage
            | SlaMetric::MaxMemoryUsage
            | SlaMetric::MaxIoBandwidth
            | SlaMetric::MaxErrorRate
            | SlaMetric::MaxFdCount
            | SlaMetric::MaxSyscallRate
            | SlaMetric::Custom => {
                if value >= self.critical {
                    ComplianceLevel::Critical
                } else if value >= self.violation {
                    ComplianceLevel::Violation
                } else if value >= self.warning {
                    ComplianceLevel::Warning
                } else {
                    ComplianceLevel::Compliant
                }
            }
            // For "min" metrics, lower value = worse
            SlaMetric::MinThroughput | SlaMetric::MinAvailability => {
                if value <= self.critical {
                    ComplianceLevel::Critical
                } else if value <= self.violation {
                    ComplianceLevel::Violation
                } else if value <= self.warning {
                    ComplianceLevel::Warning
                } else {
                    ComplianceLevel::Compliant
                }
            }
        }
    }
}

/// Compliance level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ComplianceLevel {
    /// Fully compliant
    Compliant,
    /// Warning — approaching limit
    Warning,
    /// Violation — limit exceeded
    Violation,
    /// Critical — severe violation
    Critical,
}

// ============================================================================
// VIOLATIONS
// ============================================================================

/// A detected violation
#[derive(Debug, Clone)]
pub struct Violation {
    /// Violation ID
    pub id: u64,
    /// Contract ID
    pub contract_id: u64,
    /// Process ID
    pub pid: u64,
    /// Metric violated
    pub metric: SlaMetric,
    /// Actual value
    pub actual: u64,
    /// Threshold exceeded
    pub threshold: u64,
    /// Severity
    pub level: ComplianceLevel,
    /// Detection time
    pub timestamp: u64,
    /// Duration of violation (ms), updated on check
    pub duration_ms: u64,
    /// Whether grace period is active
    pub in_grace_period: bool,
    /// Penalty points accumulated
    pub penalty_points: u32,
}

/// Grace period configuration
#[derive(Debug, Clone, Copy)]
pub struct GracePeriod {
    /// Grace period for warnings (ms)
    pub warning_ms: u64,
    /// Grace period for violations (ms)
    pub violation_ms: u64,
    /// Grace period for critical (ms)
    pub critical_ms: u64,
    /// Max grace periods per hour
    pub max_per_hour: u32,
}

impl Default for GracePeriod {
    fn default() -> Self {
        Self {
            warning_ms: 30_000,
            violation_ms: 10_000,
            critical_ms: 2_000,
            max_per_hour: 5,
        }
    }
}

impl GracePeriod {
    /// Get grace period for level
    pub fn for_level(&self, level: ComplianceLevel) -> u64 {
        match level {
            ComplianceLevel::Compliant => u64::MAX,
            ComplianceLevel::Warning => self.warning_ms,
            ComplianceLevel::Violation => self.violation_ms,
            ComplianceLevel::Critical => self.critical_ms,
        }
    }
}

// ============================================================================
// CONTRACT COMPLIANCE STATE
// ============================================================================

/// Penalty action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PenaltyAction {
    /// No penalty
    None,
    /// Reduce priority
    ReducePriority,
    /// Reduce resource allocation
    ReduceResources,
    /// Throttle operations
    Throttle,
    /// Suspend cooperation
    SuspendCooperation,
    /// Terminate contract
    TerminateContract,
}

/// Penalty schedule based on accumulated points
#[derive(Debug, Clone)]
pub struct PenaltySchedule {
    /// Thresholds: (points, action)
    pub thresholds: Vec<(u32, PenaltyAction)>,
    /// Points decay rate (points per second)
    pub decay_rate: u32,
}

impl Default for PenaltySchedule {
    fn default() -> Self {
        let mut thresholds = Vec::new();
        thresholds.push((10, PenaltyAction::ReducePriority));
        thresholds.push((25, PenaltyAction::ReduceResources));
        thresholds.push((50, PenaltyAction::Throttle));
        thresholds.push((100, PenaltyAction::SuspendCooperation));
        thresholds.push((200, PenaltyAction::TerminateContract));
        Self {
            thresholds,
            decay_rate: 1,
        }
    }
}

impl PenaltySchedule {
    /// Determine action for given points
    pub fn action_for(&self, points: u32) -> PenaltyAction {
        let mut action = PenaltyAction::None;
        for &(threshold, act) in &self.thresholds {
            if points >= threshold {
                action = act;
            }
        }
        action
    }
}

/// Per-contract compliance state
struct ContractCompliance {
    /// Contract ID
    contract_id: u64,
    /// PID
    pid: u64,
    /// SLA bounds
    bounds: Vec<SlaBound>,
    /// Grace period config
    grace: GracePeriod,
    /// Penalty schedule
    penalty: PenaltySchedule,
    /// Active violations
    active_violations: Vec<Violation>,
    /// Total violations detected
    total_violations: u64,
    /// Accumulated penalty points
    penalty_points: u32,
    /// Grace periods used this hour
    grace_used_this_hour: u32,
    /// Hour marker for grace period tracking
    grace_hour_marker: u64,
    /// Compliance history (last 100 checks: 1 = compliant, 0 = violation)
    history: Vec<u8>,
    /// Max history entries
    max_history: usize,
    /// Current penalty action
    current_action: PenaltyAction,
}

impl ContractCompliance {
    fn new(contract_id: u64, pid: u64) -> Self {
        Self {
            contract_id,
            pid,
            bounds: Vec::new(),
            grace: GracePeriod::default(),
            penalty: PenaltySchedule::default(),
            active_violations: Vec::new(),
            total_violations: 0,
            penalty_points: 0,
            grace_used_this_hour: 0,
            grace_hour_marker: 0,
            history: Vec::new(),
            max_history: 100,
            current_action: PenaltyAction::None,
        }
    }

    /// Add SLA bound
    fn add_bound(&mut self, bound: SlaBound) {
        self.bounds.push(bound);
    }

    /// Check a metric value against bounds
    fn check_metric(
        &mut self,
        metric: SlaMetric,
        value: u64,
        timestamp: u64,
        next_violation_id: &mut u64,
    ) -> ComplianceLevel {
        let mut worst = ComplianceLevel::Compliant;

        for bound in &self.bounds {
            if bound.metric != metric {
                continue;
            }

            let level = bound.check(value);

            if level > ComplianceLevel::Compliant {
                // Check if we already have an active violation for this metric
                let existing = self
                    .active_violations
                    .iter_mut()
                    .find(|v| v.metric == metric);

                if let Some(v) = existing {
                    v.actual = value;
                    v.level = level;
                    v.duration_ms = timestamp.saturating_sub(v.timestamp);

                    // Check grace period
                    let grace_ms = self.grace.for_level(level);
                    if v.duration_ms > grace_ms && v.in_grace_period {
                        v.in_grace_period = false;
                        // Assess penalty
                        let points = match level {
                            ComplianceLevel::Warning => 1,
                            ComplianceLevel::Violation => 5,
                            ComplianceLevel::Critical => 20,
                            ComplianceLevel::Compliant => 0,
                        };
                        v.penalty_points += points;
                        self.penalty_points += points;
                    }
                } else {
                    // New violation
                    let can_grace = self.grace_used_this_hour < self.grace.max_per_hour;
                    let violation = Violation {
                        id: *next_violation_id,
                        contract_id: self.contract_id,
                        pid: self.pid,
                        metric,
                        actual: value,
                        threshold: bound.violation,
                        level,
                        timestamp,
                        duration_ms: 0,
                        in_grace_period: can_grace,
                        penalty_points: 0,
                    };
                    *next_violation_id += 1;
                    self.active_violations.push(violation);
                    self.total_violations += 1;

                    if can_grace {
                        self.grace_used_this_hour += 1;
                    }
                }
            } else {
                // Metric is compliant - clear active violation
                self.active_violations.retain(|v| v.metric != metric);
            }

            if level > worst {
                worst = level;
            }
        }

        // Record history
        let compliant = if worst == ComplianceLevel::Compliant { 1u8 } else { 0u8 };
        self.history.push(compliant);
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }

        // Update penalty action
        self.current_action = self.penalty.action_for(self.penalty_points);

        worst
    }

    /// Decay penalty points
    fn decay_penalties(&mut self, elapsed_secs: u64) {
        let decay = (self.penalty.decay_rate as u64 * elapsed_secs) as u32;
        self.penalty_points = self.penalty_points.saturating_sub(decay);
        self.current_action = self.penalty.action_for(self.penalty_points);
    }

    /// Reset hourly grace counter
    fn maybe_reset_hour(&mut self, timestamp: u64) {
        let hour = timestamp / 3_600_000;
        if hour != self.grace_hour_marker {
            self.grace_hour_marker = hour;
            self.grace_used_this_hour = 0;
        }
    }

    /// Compliance percentage (from history)
    fn compliance_percent(&self) -> f64 {
        if self.history.is_empty() {
            return 100.0;
        }
        let sum: u64 = self.history.iter().map(|&v| v as u64).sum();
        (sum as f64 / self.history.len() as f64) * 100.0
    }
}

// ============================================================================
// COMPLIANCE MONITOR
// ============================================================================

/// Compliance check result
#[derive(Debug, Clone)]
pub struct ComplianceResult {
    /// Contract ID
    pub contract_id: u64,
    /// PID
    pub pid: u64,
    /// Overall level
    pub level: ComplianceLevel,
    /// Active violation count
    pub active_violations: u32,
    /// Current penalty action
    pub penalty_action: PenaltyAction,
    /// Compliance percentage
    pub compliance_percent: f64,
    /// Penalty points
    pub penalty_points: u32,
}

/// Global compliance monitor
pub struct ComplianceMonitor {
    /// Per-contract compliance state
    contracts: BTreeMap<u64, ContractCompliance>,
    /// Next violation ID
    next_violation_id: u64,
    /// Total checks performed
    pub total_checks: u64,
    /// Total violations detected
    pub total_violations: u64,
    /// Total penalties applied
    pub total_penalties: u64,
}

impl ComplianceMonitor {
    pub fn new() -> Self {
        Self {
            contracts: BTreeMap::new(),
            next_violation_id: 1,
            total_checks: 0,
            total_violations: 0,
            total_penalties: 0,
        }
    }

    /// Register a contract for monitoring
    pub fn register_contract(&mut self, contract_id: u64, pid: u64) {
        self.contracts.entry(contract_id)
            .or_insert_with(|| ContractCompliance::new(contract_id, pid));
    }

    /// Add SLA bound to contract
    pub fn add_bound(&mut self, contract_id: u64, bound: SlaBound) {
        if let Some(cc) = self.contracts.get_mut(&contract_id) {
            cc.add_bound(bound);
        }
    }

    /// Set grace period for contract
    pub fn set_grace(&mut self, contract_id: u64, grace: GracePeriod) {
        if let Some(cc) = self.contracts.get_mut(&contract_id) {
            cc.grace = grace;
        }
    }

    /// Check a metric value
    pub fn check(
        &mut self,
        contract_id: u64,
        metric: SlaMetric,
        value: u64,
        timestamp: u64,
    ) -> ComplianceLevel {
        self.total_checks += 1;

        if let Some(cc) = self.contracts.get_mut(&contract_id) {
            cc.maybe_reset_hour(timestamp);
            let level = cc.check_metric(metric, value, timestamp, &mut self.next_violation_id);
            if level > ComplianceLevel::Compliant {
                self.total_violations += 1;
            }
            level
        } else {
            ComplianceLevel::Compliant
        }
    }

    /// Get compliance result for contract
    pub fn get_result(&self, contract_id: u64) -> Option<ComplianceResult> {
        let cc = self.contracts.get(&contract_id)?;
        Some(ComplianceResult {
            contract_id,
            pid: cc.pid,
            level: cc.active_violations
                .iter()
                .map(|v| v.level)
                .max()
                .unwrap_or(ComplianceLevel::Compliant),
            active_violations: cc.active_violations.len() as u32,
            penalty_action: cc.current_action,
            compliance_percent: cc.compliance_percent(),
            penalty_points: cc.penalty_points,
        })
    }

    /// Unregister contract
    pub fn unregister_contract(&mut self, contract_id: u64) {
        self.contracts.remove(&contract_id);
    }

    /// Unregister all contracts for a PID
    pub fn unregister_pid(&mut self, pid: u64) {
        self.contracts.retain(|_, cc| cc.pid != pid);
    }

    /// Decay penalties for all contracts
    pub fn decay_all(&mut self, elapsed_secs: u64) {
        for cc in self.contracts.values_mut() {
            cc.decay_penalties(elapsed_secs);
        }
    }

    /// Monitored contract count
    pub fn contract_count(&self) -> usize {
        self.contracts.len()
    }
}
