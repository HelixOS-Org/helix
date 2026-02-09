//! # System-Level SLA Management
//!
//! End-to-end SLA tracking for the entire system:
//! - Multi-tier SLA definitions (latency, throughput, availability)
//! - Composite SLA scoring
//! - SLA violation root cause analysis
//! - Historical compliance tracking
//! - Degradation budget management

extern crate alloc;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::vec::Vec;

use crate::fast::linear_map::LinearMap;

// ============================================================================
// SLA DEFINITIONS
// ============================================================================

/// SLA tier levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SlaTier {
    /// Best effort
    BestEffort = 0,
    /// Standard
    Standard   = 1,
    /// Premium
    Premium    = 2,
    /// Critical (real-time)
    Critical   = 3,
    /// Safety-critical
    Safety     = 4,
}

impl SlaTier {
    /// Default latency target for tier (microseconds)
    #[inline]
    pub fn latency_target_us(&self) -> u64 {
        match self {
            Self::BestEffort => 100_000,
            Self::Standard => 10_000,
            Self::Premium => 1_000,
            Self::Critical => 100,
            Self::Safety => 10,
        }
    }

    /// Default availability target (nines)
    #[inline]
    pub fn availability_nines(&self) -> u32 {
        match self {
            Self::BestEffort => 2, // 99%
            Self::Standard => 3,   // 99.9%
            Self::Premium => 4,    // 99.99%
            Self::Critical => 5,   // 99.999%
            Self::Safety => 6,     // 99.9999%
        }
    }

    /// Max error budget (percent)
    #[inline]
    pub fn error_budget(&self) -> f64 {
        match self {
            Self::BestEffort => 1.0,
            Self::Standard => 0.1,
            Self::Premium => 0.01,
            Self::Critical => 0.001,
            Self::Safety => 0.0001,
        }
    }
}

/// SLA metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SlaMetricType {
    /// Response latency (P50)
    LatencyP50,
    /// Response latency (P95)
    LatencyP95,
    /// Response latency (P99)
    LatencyP99,
    /// Throughput (operations/sec)
    Throughput,
    /// Availability (percent)
    Availability,
    /// Error rate (percent)
    ErrorRate,
    /// Start latency
    StartLatency,
    /// Scheduling latency
    SchedulingLatency,
    /// Memory allocation latency
    AllocationLatency,
    /// I/O completion latency
    IoLatency,
}

/// SLA target
#[derive(Debug, Clone)]
pub struct SlaTarget {
    /// Metric type
    pub metric: SlaMetricType,
    /// Target value (lower is better for latency/error, higher for throughput/availability)
    pub target: f64,
    /// Warning threshold
    pub warning: f64,
    /// Critical threshold
    pub critical: f64,
    /// Whether higher is better
    pub higher_is_better: bool,
}

impl SlaTarget {
    #[inline]
    pub fn latency(metric: SlaMetricType, target_us: f64) -> Self {
        Self {
            metric,
            target: target_us,
            warning: target_us * 1.5,
            critical: target_us * 3.0,
            higher_is_better: false,
        }
    }

    #[inline]
    pub fn throughput(target_ops: f64) -> Self {
        Self {
            metric: SlaMetricType::Throughput,
            target: target_ops,
            warning: target_ops * 0.8,
            critical: target_ops * 0.5,
            higher_is_better: true,
        }
    }

    #[inline]
    pub fn availability(target_pct: f64) -> Self {
        Self {
            metric: SlaMetricType::Availability,
            target: target_pct,
            warning: target_pct - 0.5,
            critical: target_pct - 2.0,
            higher_is_better: true,
        }
    }

    /// Check if value meets target
    #[inline]
    pub fn meets_target(&self, value: f64) -> bool {
        if self.higher_is_better {
            value >= self.target
        } else {
            value <= self.target
        }
    }

    /// Check severity
    pub fn severity(&self, value: f64) -> SlaStatus {
        if self.higher_is_better {
            if value >= self.target {
                SlaStatus::Met
            } else if value >= self.warning {
                SlaStatus::Warning
            } else if value >= self.critical {
                SlaStatus::Critical
            } else {
                SlaStatus::Violated
            }
        } else {
            if value <= self.target {
                SlaStatus::Met
            } else if value <= self.warning {
                SlaStatus::Warning
            } else if value <= self.critical {
                SlaStatus::Critical
            } else {
                SlaStatus::Violated
            }
        }
    }
}

/// SLA status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlaStatus {
    /// Target met
    Met,
    /// Warning threshold exceeded
    Warning,
    /// Critical threshold exceeded
    Critical,
    /// SLA violated
    Violated,
    /// Unknown (no data)
    Unknown,
}

// ============================================================================
// SLA DEFINITION
// ============================================================================

/// Complete SLA definition
#[derive(Debug, Clone)]
pub struct SlaDefinition {
    /// SLA ID
    pub id: u32,
    /// Tier
    pub tier: SlaTier,
    /// Individual targets
    pub targets: Vec<SlaTarget>,
    /// Weight for composite score
    pub weights: BTreeMap<u8, f64>,
}

impl SlaDefinition {
    pub fn new(id: u32, tier: SlaTier) -> Self {
        Self {
            id,
            tier,
            targets: Vec::new(),
            weights: BTreeMap::new(),
        }
    }

    #[inline]
    pub fn add_target(&mut self, target: SlaTarget, weight: f64) {
        let metric_key = target.metric as u8;
        self.targets.push(target);
        self.weights.insert(metric_key, weight);
    }

    /// Create default SLA for tier
    pub fn default_for_tier(id: u32, tier: SlaTier) -> Self {
        let mut sla = Self::new(id, tier);

        let lat = tier.latency_target_us() as f64;
        sla.add_target(SlaTarget::latency(SlaMetricType::LatencyP50, lat), 0.2);
        sla.add_target(
            SlaTarget::latency(SlaMetricType::LatencyP95, lat * 2.0),
            0.3,
        );
        sla.add_target(
            SlaTarget::latency(SlaMetricType::LatencyP99, lat * 5.0),
            0.2,
        );

        let avail = match tier.availability_nines() {
            2 => 99.0,
            3 => 99.9,
            4 => 99.99,
            5 => 99.999,
            6 => 99.9999,
            _ => 99.0,
        };
        sla.add_target(SlaTarget::availability(avail), 0.3);

        sla
    }
}

// ============================================================================
// SLA EVALUATION
// ============================================================================

/// Single metric evaluation
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MetricEvaluation {
    pub metric: SlaMetricType,
    pub target: f64,
    pub actual: f64,
    pub status: SlaStatus,
    pub score: f64, // 0.0 = violated, 1.0 = perfect
}

/// Complete SLA evaluation result
#[derive(Debug, Clone)]
pub struct SlaEvaluation {
    pub sla_id: u32,
    pub tier: SlaTier,
    pub evaluations: Vec<MetricEvaluation>,
    pub composite_score: f64,
    pub overall_status: SlaStatus,
    pub timestamp: u64,
}

// ============================================================================
// ERROR BUDGET
// ============================================================================

/// Error budget tracking
#[derive(Debug, Clone)]
pub struct ErrorBudget {
    /// Total budget (percent)
    pub total: f64,
    /// Consumed so far
    pub consumed: f64,
    /// Budget window start
    pub window_start: u64,
    /// Budget window end
    pub window_end: u64,
    /// Burn rate (budget consumed per second)
    pub burn_rate: f64,
}

impl ErrorBudget {
    pub fn new(total: f64, window_start: u64, window_end: u64) -> Self {
        Self {
            total,
            consumed: 0.0,
            window_start,
            window_end,
            burn_rate: 0.0,
        }
    }

    /// Remaining budget
    #[inline(always)]
    pub fn remaining(&self) -> f64 {
        (self.total - self.consumed).max(0.0)
    }

    /// Budget utilization (0.0 - 1.0+)
    #[inline]
    pub fn utilization(&self) -> f64 {
        if self.total <= 0.0 {
            return 1.0;
        }
        self.consumed / self.total
    }

    /// Is budget exhausted?
    #[inline(always)]
    pub fn is_exhausted(&self) -> bool {
        self.consumed >= self.total
    }

    /// Consume budget
    #[inline(always)]
    pub fn consume(&mut self, amount: f64) {
        self.consumed += amount;
    }

    /// Update burn rate
    #[inline]
    pub fn update_burn_rate(&mut self, elapsed_secs: f64) {
        if elapsed_secs > 0.0 {
            self.burn_rate = self.consumed / elapsed_secs;
        }
    }

    /// Time until exhaustion (seconds)
    #[inline]
    pub fn time_to_exhaustion(&self) -> Option<u64> {
        if self.burn_rate <= 0.0 {
            return None;
        }
        let remaining = self.remaining();
        Some((remaining / self.burn_rate) as u64)
    }
}

// ============================================================================
// VIOLATION ROOT CAUSE
// ============================================================================

/// Root cause category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationCause {
    /// CPU saturation
    CpuSaturation,
    /// Memory pressure
    MemoryPressure,
    /// I/O bottleneck
    IoBottleneck,
    /// Network congestion
    NetworkCongestion,
    /// Noisy neighbor
    NoisyNeighbor,
    /// Resource starvation
    ResourceStarvation,
    /// Scheduling delay
    SchedulingDelay,
    /// Lock contention
    LockContention,
    /// Configuration issue
    Configuration,
    /// External dependency
    ExternalDependency,
    /// Unknown
    Unknown,
}

/// Violation record
#[derive(Debug, Clone)]
pub struct SlaViolation {
    pub sla_id: u32,
    pub metric: SlaMetricType,
    pub target: f64,
    pub actual: f64,
    pub cause: ViolationCause,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub affected_processes: Vec<u64>,
}

// ============================================================================
// SLA MANAGER
// ============================================================================

/// System SLA manager
pub struct SystemSlaManager {
    /// SLA definitions
    slas: BTreeMap<u32, SlaDefinition>,
    /// Process-to-SLA mapping
    process_sla: LinearMap<u32, 64>,
    /// Error budgets per SLA
    error_budgets: BTreeMap<u32, ErrorBudget>,
    /// Violation history
    violations: VecDeque<SlaViolation>,
    /// Max violation history
    max_violations: usize,
    /// Last metric values
    metric_values: BTreeMap<u32, BTreeMap<u8, f64>>,
    /// Total evaluations
    pub total_evaluations: u64,
    /// Total violations detected
    pub total_violations: u64,
}

impl SystemSlaManager {
    pub fn new() -> Self {
        Self {
            slas: BTreeMap::new(),
            process_sla: LinearMap::new(),
            error_budgets: BTreeMap::new(),
            violations: VecDeque::new(),
            max_violations: 1000,
            metric_values: BTreeMap::new(),
            total_evaluations: 0,
            total_violations: 0,
        }
    }

    /// Register SLA
    #[inline]
    pub fn register_sla(&mut self, sla: SlaDefinition) {
        let budget = ErrorBudget::new(sla.tier.error_budget(), 0, 86400);
        self.error_budgets.insert(sla.id, budget);
        self.slas.insert(sla.id, sla);
    }

    /// Assign process to SLA
    #[inline(always)]
    pub fn assign_process(&mut self, pid: u64, sla_id: u32) {
        self.process_sla.insert(pid, sla_id);
    }

    /// Update metric value for SLA
    #[inline]
    pub fn update_metric(&mut self, sla_id: u32, metric: SlaMetricType, value: f64) {
        self.metric_values
            .entry(sla_id)
            .or_insert_with(BTreeMap::new)
            .insert(metric as u8, value);
    }

    /// Evaluate single SLA
    pub fn evaluate_sla(&mut self, sla_id: u32, timestamp: u64) -> Option<SlaEvaluation> {
        let sla = self.slas.get(&sla_id)?;
        let metrics = self.metric_values.get(&sla_id)?;

        let mut evaluations = Vec::new();
        let mut worst_status = SlaStatus::Met;
        let mut total_score = 0.0;
        let mut total_weight = 0.0;

        for target in &sla.targets {
            let metric_key = target.metric as u8;
            let value = metrics.get(&metric_key).copied().unwrap_or(0.0);
            let status = target.severity(value);

            let score = if target.meets_target(value) {
                1.0
            } else if target.higher_is_better {
                if target.target > 0.0 {
                    (value / target.target).min(1.0)
                } else {
                    0.0
                }
            } else {
                if value > 0.0 {
                    (target.target / value).min(1.0)
                } else {
                    1.0
                }
            };

            let weight = sla.weights.get(&metric_key).copied().unwrap_or(1.0);

            total_score += score * weight;
            total_weight += weight;

            if (status as u8) > (worst_status as u8) {
                worst_status = status;
            }

            evaluations.push(MetricEvaluation {
                metric: target.metric,
                target: target.target,
                actual: value,
                status,
                score,
            });
        }

        let composite_score = if total_weight > 0.0 {
            total_score / total_weight
        } else {
            0.0
        };

        self.total_evaluations += 1;

        // Consume error budget for violations
        if worst_status == SlaStatus::Violated || worst_status == SlaStatus::Critical {
            if let Some(budget) = self.error_budgets.get_mut(&sla_id) {
                let consumption = if worst_status == SlaStatus::Violated {
                    0.01
                } else {
                    0.005
                };
                budget.consume(consumption);
            }
        }

        Some(SlaEvaluation {
            sla_id,
            tier: sla.tier,
            evaluations,
            composite_score,
            overall_status: worst_status,
            timestamp,
        })
    }

    /// Record violation
    #[inline]
    pub fn record_violation(&mut self, violation: SlaViolation) {
        self.total_violations += 1;
        self.violations.push_back(violation);
        if self.violations.len() > self.max_violations {
            self.violations.pop_front();
        }
    }

    /// Get violations for SLA
    #[inline]
    pub fn violations_for_sla(&self, sla_id: u32) -> Vec<&SlaViolation> {
        self.violations
            .iter()
            .filter(|v| v.sla_id == sla_id)
            .collect()
    }

    /// Get error budget
    #[inline(always)]
    pub fn error_budget(&self, sla_id: u32) -> Option<&ErrorBudget> {
        self.error_budgets.get(&sla_id)
    }

    /// SLA count
    #[inline(always)]
    pub fn sla_count(&self) -> usize {
        self.slas.len()
    }

    /// Get process SLA tier
    #[inline(always)]
    pub fn process_tier(&self, pid: u64) -> Option<SlaTier> {
        let sla_id = self.process_sla.get(pid)?;
        self.slas.get(sla_id).map(|s| s.tier)
    }
}
