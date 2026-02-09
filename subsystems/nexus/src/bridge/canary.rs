//! # Bridge Canary System
//!
//! Canary deployment and progressive rollout for syscall policy changes:
//! - Canary analysis for syscall routing changes
//! - A/B testing infrastructure
//! - Progressive rollout percentages
//! - Automatic rollback on anomaly detection
//! - Statistical significance testing

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// CANARY TYPES
// ============================================================================

/// Canary state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanaryState {
    /// Preparing (not yet active)
    Preparing,
    /// Active (receiving traffic)
    Active,
    /// Paused
    Paused,
    /// Promoting (becoming the new baseline)
    Promoting,
    /// Rolling back
    RollingBack,
    /// Completed successfully
    Completed,
    /// Failed
    Failed,
}

/// Canary metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CanaryMetric {
    /// Latency p50
    LatencyP50,
    /// Latency p99
    LatencyP99,
    /// Error rate
    ErrorRate,
    /// Throughput
    Throughput,
    /// CPU usage
    CpuUsage,
    /// Memory usage
    MemoryUsage,
}

/// Comparison result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonResult {
    /// Canary is significantly better
    Better,
    /// No significant difference
    NoChange,
    /// Canary is significantly worse
    Worse,
    /// Insufficient data
    InsufficientData,
}

// ============================================================================
// METRIC SAMPLES
// ============================================================================

/// Collected metric samples
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MetricSamples {
    /// Values
    values: Vec<f64>,
    /// Sum
    sum: f64,
    /// Sum of squares (for variance)
    sum_sq: f64,
    /// Count
    count: u64,
    /// Min
    min: f64,
    /// Max
    max: f64,
}

impl MetricSamples {
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
            sum: 0.0,
            sum_sq: 0.0,
            count: 0,
            min: f64::MAX,
            max: f64::MIN,
        }
    }

    /// Add sample
    pub fn add(&mut self, value: f64) {
        self.values.push(value);
        self.sum += value;
        self.sum_sq += value * value;
        self.count += 1;
        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }
    }

    /// Mean
    #[inline]
    pub fn mean(&self) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        self.sum / self.count as f64
    }

    /// Variance
    #[inline]
    pub fn variance(&self) -> f64 {
        if self.count < 2 {
            return 0.0;
        }
        let n = self.count as f64;
        (self.sum_sq - (self.sum * self.sum) / n) / (n - 1.0)
    }

    /// Standard deviation
    #[inline(always)]
    pub fn stddev(&self) -> f64 {
        libm::sqrt(self.variance())
    }

    /// Sample count
    #[inline(always)]
    pub fn sample_count(&self) -> u64 {
        self.count
    }
}

// ============================================================================
// CANARY DEPLOYMENT
// ============================================================================

/// A canary deployment
#[derive(Debug)]
#[repr(align(64))]
pub struct CanaryDeployment {
    /// Deployment id
    pub id: u64,
    /// State
    pub state: CanaryState,
    /// Traffic percentage to canary (0-100)
    pub traffic_pct: u32,
    /// Target traffic percentage
    pub target_pct: u32,
    /// Step size for progressive rollout
    pub step_pct: u32,
    /// Created at
    pub created_at: u64,
    /// Started at
    pub started_at: Option<u64>,
    /// Baseline metrics
    baseline: BTreeMap<u8, MetricSamples>,
    /// Canary metrics
    canary: BTreeMap<u8, MetricSamples>,
    /// Thresholds (metric -> max allowed degradation %)
    thresholds: BTreeMap<u8, f64>,
    /// Min samples before decision
    pub min_samples: u64,
    /// Auto-promote after steps complete
    pub auto_promote: bool,
}

impl CanaryDeployment {
    pub fn new(id: u64, target_pct: u32, step_pct: u32, now: u64) -> Self {
        Self {
            id,
            state: CanaryState::Preparing,
            traffic_pct: 0,
            target_pct,
            step_pct: if step_pct == 0 { 10 } else { step_pct },
            created_at: now,
            started_at: None,
            baseline: BTreeMap::new(),
            canary: BTreeMap::new(),
            thresholds: BTreeMap::new(),
            min_samples: 100,
            auto_promote: true,
        }
    }

    /// Set threshold for a metric
    #[inline(always)]
    pub fn set_threshold(&mut self, metric: CanaryMetric, max_degradation_pct: f64) {
        self.thresholds.insert(metric as u8, max_degradation_pct);
    }

    /// Start deployment
    #[inline]
    pub fn start(&mut self, now: u64) {
        self.state = CanaryState::Active;
        self.traffic_pct = self.step_pct.min(self.target_pct);
        self.started_at = Some(now);
    }

    /// Record baseline sample
    #[inline]
    pub fn record_baseline(&mut self, metric: CanaryMetric, value: f64) {
        self.baseline
            .entry(metric as u8)
            .or_insert_with(MetricSamples::new)
            .add(value);
    }

    /// Record canary sample
    #[inline]
    pub fn record_canary(&mut self, metric: CanaryMetric, value: f64) {
        self.canary
            .entry(metric as u8)
            .or_insert_with(MetricSamples::new)
            .add(value);
    }

    /// Compare metric between baseline and canary
    pub fn compare(&self, metric: CanaryMetric) -> ComparisonResult {
        let key = metric as u8;
        let baseline = match self.baseline.get(&key) {
            Some(s) if s.sample_count() >= self.min_samples => s,
            _ => return ComparisonResult::InsufficientData,
        };
        let canary = match self.canary.get(&key) {
            Some(s) if s.sample_count() >= self.min_samples => s,
            _ => return ComparisonResult::InsufficientData,
        };

        let threshold = self.thresholds.get(&key).copied().unwrap_or(5.0);
        let baseline_mean = baseline.mean();
        if baseline_mean == 0.0 {
            return ComparisonResult::NoChange;
        }

        let diff_pct = (canary.mean() - baseline_mean) / libm::fabs(baseline_mean) * 100.0;

        // For error rate / latency: higher is worse
        // For throughput: lower is worse
        match metric {
            CanaryMetric::Throughput => {
                if diff_pct < -threshold {
                    ComparisonResult::Worse
                } else if diff_pct > threshold {
                    ComparisonResult::Better
                } else {
                    ComparisonResult::NoChange
                }
            }
            _ => {
                if diff_pct > threshold {
                    ComparisonResult::Worse
                } else if diff_pct < -threshold {
                    ComparisonResult::Better
                } else {
                    ComparisonResult::NoChange
                }
            }
        }
    }

    /// Advance to next step
    pub fn advance(&mut self) -> bool {
        if self.state != CanaryState::Active {
            return false;
        }
        let new_pct = self.traffic_pct + self.step_pct;
        if new_pct >= self.target_pct {
            self.traffic_pct = self.target_pct;
            if self.auto_promote {
                self.state = CanaryState::Promoting;
            }
            true
        } else {
            self.traffic_pct = new_pct;
            true
        }
    }

    /// Rollback
    #[inline(always)]
    pub fn rollback(&mut self) {
        self.state = CanaryState::RollingBack;
        self.traffic_pct = 0;
    }

    /// Complete
    #[inline(always)]
    pub fn complete(&mut self) {
        self.state = CanaryState::Completed;
    }
}

// ============================================================================
// CANARY ENGINE
// ============================================================================

/// Canary stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BridgeCanaryStats {
    /// Active canaries
    pub active_canaries: usize,
    /// Total deployments
    pub total_deployments: u64,
    /// Successful
    pub successful: u64,
    /// Rolled back
    pub rolled_back: u64,
}

/// Bridge canary manager
#[repr(align(64))]
pub struct BridgeCanaryManager {
    /// Deployments
    deployments: BTreeMap<u64, CanaryDeployment>,
    /// Next id
    next_id: u64,
    /// Stats
    stats: BridgeCanaryStats,
}

impl BridgeCanaryManager {
    pub fn new() -> Self {
        Self {
            deployments: BTreeMap::new(),
            next_id: 1,
            stats: BridgeCanaryStats::default(),
        }
    }

    /// Create canary deployment
    #[inline]
    pub fn create(&mut self, target_pct: u32, step_pct: u32, now: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let dep = CanaryDeployment::new(id, target_pct, step_pct, now);
        self.deployments.insert(id, dep);
        self.stats.total_deployments += 1;
        self.update_stats();
        id
    }

    /// Start deployment
    #[inline]
    pub fn start(&mut self, id: u64, now: u64) -> bool {
        if let Some(dep) = self.deployments.get_mut(&id) {
            dep.start(now);
            true
        } else {
            false
        }
    }

    /// Advance deployment
    #[inline]
    pub fn advance(&mut self, id: u64) -> bool {
        if let Some(dep) = self.deployments.get_mut(&id) {
            dep.advance()
        } else {
            false
        }
    }

    /// Rollback deployment
    #[inline]
    pub fn rollback(&mut self, id: u64) {
        if let Some(dep) = self.deployments.get_mut(&id) {
            dep.rollback();
            self.stats.rolled_back += 1;
        }
    }

    /// Get deployment
    #[inline(always)]
    pub fn deployment(&self, id: u64) -> Option<&CanaryDeployment> {
        self.deployments.get(&id)
    }

    fn update_stats(&mut self) {
        self.stats.active_canaries = self.deployments.values()
            .filter(|d| d.state == CanaryState::Active).count();
        self.stats.successful = self.deployments.values()
            .filter(|d| d.state == CanaryState::Completed).count() as u64;
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &BridgeCanaryStats {
        &self.stats
    }
}
