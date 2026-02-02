//! # Cognitive Latency Manager
//!
//! Latency tracking and optimization for cognitive operations.
//! Provides SLA monitoring and performance optimization.

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// LATENCY TYPES
// ============================================================================

/// Latency measurement
#[derive(Debug, Clone)]
pub struct LatencyMeasurement {
    /// Operation name
    pub operation: String,
    /// Start time
    pub start: Timestamp,
    /// End time
    pub end: Timestamp,
    /// Duration (ns)
    pub duration_ns: u64,
    /// Source domain
    pub source: DomainId,
    /// Target domain
    pub target: Option<DomainId>,
    /// Success
    pub success: bool,
    /// Tags
    pub tags: Vec<String>,
}

/// Latency percentile
#[derive(Debug, Clone, Copy)]
pub struct LatencyPercentiles {
    pub p50: u64,
    pub p75: u64,
    pub p90: u64,
    pub p95: u64,
    pub p99: u64,
    pub p999: u64,
}

impl LatencyPercentiles {
    /// Calculate from samples
    pub fn from_samples(samples: &[u64]) -> Option<Self> {
        if samples.is_empty() {
            return None;
        }

        let mut sorted = samples.to_vec();
        sorted.sort_unstable();

        let len = sorted.len();

        Some(Self {
            p50: sorted[len * 50 / 100],
            p75: sorted[len * 75 / 100],
            p90: sorted[len * 90 / 100],
            p95: sorted[len * 95 / 100],
            p99: sorted[len * 99 / 100],
            p999: sorted[(len * 999 / 1000).min(len - 1)],
        })
    }
}

/// Service Level Objective
#[derive(Debug, Clone)]
pub struct Slo {
    /// SLO name
    pub name: String,
    /// Operation pattern
    pub operation: String,
    /// Target latency (ns)
    pub target_ns: u64,
    /// Target percentile
    pub percentile: f64,
    /// Error budget (0-1)
    pub error_budget: f64,
}

impl Slo {
    /// Check if SLO is met
    pub fn is_met(&self, percentiles: &LatencyPercentiles) -> bool {
        let actual = match self.percentile as u32 {
            50 => percentiles.p50,
            75 => percentiles.p75,
            90 => percentiles.p90,
            95 => percentiles.p95,
            99 => percentiles.p99,
            _ => percentiles.p99,
        };

        actual <= self.target_ns
    }
}

/// SLO status
#[derive(Debug, Clone)]
pub struct SloStatus {
    /// SLO
    pub slo: Slo,
    /// Current value
    pub current_ns: u64,
    /// Is met
    pub is_met: bool,
    /// Error budget consumed (0-1)
    pub budget_consumed: f64,
    /// Violations count
    pub violations: u64,
    /// Total measurements
    pub total: u64,
}

// ============================================================================
// HISTOGRAM
// ============================================================================

/// Latency histogram
#[derive(Debug, Clone)]
pub struct LatencyHistogram {
    /// Bucket boundaries (ns)
    buckets: Vec<u64>,
    /// Counts per bucket
    counts: Vec<u64>,
    /// Total count
    total: u64,
    /// Sum of all values
    sum: u64,
    /// Minimum value
    min: u64,
    /// Maximum value
    max: u64,
}

impl LatencyHistogram {
    /// Create new histogram with default buckets
    pub fn new() -> Self {
        // Buckets from 1us to 60s in exponential steps
        let buckets = vec![
            1_000,          // 1us
            10_000,         // 10us
            100_000,        // 100us
            1_000_000,      // 1ms
            5_000_000,      // 5ms
            10_000_000,     // 10ms
            25_000_000,     // 25ms
            50_000_000,     // 50ms
            100_000_000,    // 100ms
            250_000_000,    // 250ms
            500_000_000,    // 500ms
            1_000_000_000,  // 1s
            5_000_000_000,  // 5s
            10_000_000_000, // 10s
            60_000_000_000, // 60s
        ];

        let counts = vec![0; buckets.len() + 1];

        Self {
            buckets,
            counts,
            total: 0,
            sum: 0,
            min: u64::MAX,
            max: 0,
        }
    }

    /// Record a value
    pub fn record(&mut self, value_ns: u64) {
        let bucket = self
            .buckets
            .iter()
            .position(|&b| value_ns < b)
            .unwrap_or(self.buckets.len());

        self.counts[bucket] += 1;
        self.total += 1;
        self.sum += value_ns;
        self.min = self.min.min(value_ns);
        self.max = self.max.max(value_ns);
    }

    /// Get mean
    pub fn mean(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.sum as f64 / self.total as f64
        }
    }

    /// Get percentile
    pub fn percentile(&self, p: f64) -> u64 {
        if self.total == 0 {
            return 0;
        }

        let target = (self.total as f64 * p / 100.0) as u64;
        let mut cumulative = 0u64;

        for (i, &count) in self.counts.iter().enumerate() {
            cumulative += count;
            if cumulative >= target {
                return if i < self.buckets.len() {
                    self.buckets[i]
                } else {
                    self.max
                };
            }
        }

        self.max
    }

    /// Get all percentiles
    pub fn percentiles(&self) -> LatencyPercentiles {
        LatencyPercentiles {
            p50: self.percentile(50.0),
            p75: self.percentile(75.0),
            p90: self.percentile(90.0),
            p95: self.percentile(95.0),
            p99: self.percentile(99.0),
            p999: self.percentile(99.9),
        }
    }

    /// Get count
    pub fn count(&self) -> u64 {
        self.total
    }

    /// Get min
    pub fn min(&self) -> u64 {
        if self.total == 0 { 0 } else { self.min }
    }

    /// Get max
    pub fn max(&self) -> u64 {
        self.max
    }

    /// Reset
    pub fn reset(&mut self) {
        for count in &mut self.counts {
            *count = 0;
        }
        self.total = 0;
        self.sum = 0;
        self.min = u64::MAX;
        self.max = 0;
    }
}

impl Default for LatencyHistogram {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// LATENCY TRACKER
// ============================================================================

/// Active latency measurement
#[derive(Debug)]
pub struct ActiveMeasurement {
    /// Measurement ID
    pub id: u64,
    /// Operation
    pub operation: String,
    /// Start time
    pub start: Timestamp,
    /// Source
    pub source: DomainId,
    /// Target
    pub target: Option<DomainId>,
    /// Tags
    pub tags: Vec<String>,
}

/// Latency tracker for a specific operation
#[derive(Debug)]
pub struct OperationTracker {
    /// Operation name
    pub operation: String,
    /// Histogram
    histogram: LatencyHistogram,
    /// Recent samples (for percentile calculation)
    samples: Vec<u64>,
    /// Maximum samples
    max_samples: usize,
    /// Success count
    success_count: u64,
    /// Failure count
    failure_count: u64,
    /// SLOs for this operation
    slos: Vec<Slo>,
    /// SLO violations
    slo_violations: u64,
}

impl OperationTracker {
    /// Create new tracker
    pub fn new(operation: &str, max_samples: usize) -> Self {
        Self {
            operation: operation.into(),
            histogram: LatencyHistogram::new(),
            samples: Vec::new(),
            max_samples,
            success_count: 0,
            failure_count: 0,
            slos: Vec::new(),
            slo_violations: 0,
        }
    }

    /// Record measurement
    pub fn record(&mut self, duration_ns: u64, success: bool) {
        self.histogram.record(duration_ns);

        // Store sample
        if self.samples.len() >= self.max_samples {
            self.samples.remove(0);
        }
        self.samples.push(duration_ns);

        if success {
            self.success_count += 1;
        } else {
            self.failure_count += 1;
        }

        // Check SLOs
        for slo in &self.slos {
            if duration_ns > slo.target_ns {
                self.slo_violations += 1;
            }
        }
    }

    /// Add SLO
    pub fn add_slo(&mut self, slo: Slo) {
        self.slos.push(slo);
    }

    /// Get SLO status
    pub fn slo_status(&self) -> Vec<SloStatus> {
        let percentiles = self.histogram.percentiles();
        let total = self.success_count + self.failure_count;

        self.slos
            .iter()
            .map(|slo| {
                let is_met = slo.is_met(&percentiles);
                let current = match slo.percentile as u32 {
                    50 => percentiles.p50,
                    75 => percentiles.p75,
                    90 => percentiles.p90,
                    95 => percentiles.p95,
                    99 => percentiles.p99,
                    _ => percentiles.p99,
                };

                let budget_consumed = if total > 0 {
                    self.slo_violations as f64 / total as f64 / slo.error_budget
                } else {
                    0.0
                };

                SloStatus {
                    slo: slo.clone(),
                    current_ns: current,
                    is_met,
                    budget_consumed: budget_consumed.min(1.0),
                    violations: self.slo_violations,
                    total,
                }
            })
            .collect()
    }

    /// Get percentiles
    pub fn percentiles(&self) -> LatencyPercentiles {
        self.histogram.percentiles()
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        let total = self.success_count + self.failure_count;
        if total == 0 {
            1.0
        } else {
            self.success_count as f64 / total as f64
        }
    }
}

// ============================================================================
// LATENCY MANAGER
// ============================================================================

/// Latency manager
pub struct LatencyManager {
    /// Operation trackers
    trackers: BTreeMap<String, OperationTracker>,
    /// Active measurements
    active: BTreeMap<u64, ActiveMeasurement>,
    /// Next measurement ID
    next_id: AtomicU64,
    /// Configuration
    config: LatencyConfig,
    /// Global histogram
    global: LatencyHistogram,
    /// Statistics
    stats: LatencyStats,
}

/// Latency configuration
#[derive(Debug, Clone)]
pub struct LatencyConfig {
    /// Maximum samples per operation
    pub max_samples: usize,
    /// Maximum active measurements
    pub max_active: usize,
    /// Measurement timeout (ns)
    pub timeout_ns: u64,
    /// Alert threshold (ns)
    pub alert_threshold_ns: u64,
}

impl Default for LatencyConfig {
    fn default() -> Self {
        Self {
            max_samples: 10000,
            max_active: 10000,
            timeout_ns: 60_000_000_000,        // 60 seconds
            alert_threshold_ns: 1_000_000_000, // 1 second
        }
    }
}

/// Latency statistics
#[derive(Debug, Clone, Default)]
pub struct LatencyStats {
    /// Total measurements
    pub total_measurements: u64,
    /// Active measurements
    pub active_measurements: u64,
    /// Timed out measurements
    pub timed_out: u64,
    /// High latency alerts
    pub high_latency_alerts: u64,
    /// Average latency (ns)
    pub avg_latency_ns: f64,
}

impl LatencyManager {
    /// Create new latency manager
    pub fn new(config: LatencyConfig) -> Self {
        Self {
            trackers: BTreeMap::new(),
            active: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            global: LatencyHistogram::new(),
            stats: LatencyStats::default(),
        }
    }

    /// Start a measurement
    pub fn start(
        &mut self,
        operation: &str,
        source: DomainId,
        target: Option<DomainId>,
        tags: Vec<String>,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let measurement = ActiveMeasurement {
            id,
            operation: operation.into(),
            start: Timestamp::now(),
            source,
            target,
            tags,
        };

        self.active.insert(id, measurement);
        self.stats.active_measurements = self.active.len() as u64;

        id
    }

    /// End a measurement
    pub fn end(&mut self, id: u64, success: bool) -> Option<LatencyMeasurement> {
        let active = self.active.remove(&id)?;
        let end = Timestamp::now();
        let duration_ns = end.elapsed_since(active.start);

        // Update tracker
        let tracker = self
            .trackers
            .entry(active.operation.clone())
            .or_insert_with(|| OperationTracker::new(&active.operation, self.config.max_samples));
        tracker.record(duration_ns, success);

        // Update global
        self.global.record(duration_ns);

        // Update stats
        self.stats.total_measurements += 1;
        self.stats.active_measurements = self.active.len() as u64;
        self.stats.avg_latency_ns = (self.stats.avg_latency_ns
            * (self.stats.total_measurements - 1) as f64
            + duration_ns as f64)
            / self.stats.total_measurements as f64;

        // Check alert threshold
        if duration_ns > self.config.alert_threshold_ns {
            self.stats.high_latency_alerts += 1;
        }

        Some(LatencyMeasurement {
            operation: active.operation,
            start: active.start,
            end,
            duration_ns,
            source: active.source,
            target: active.target,
            success,
            tags: active.tags,
        })
    }

    /// Record a complete measurement
    pub fn record(&mut self, measurement: LatencyMeasurement) {
        let tracker = self
            .trackers
            .entry(measurement.operation.clone())
            .or_insert_with(|| {
                OperationTracker::new(&measurement.operation, self.config.max_samples)
            });
        tracker.record(measurement.duration_ns, measurement.success);

        self.global.record(measurement.duration_ns);
        self.stats.total_measurements += 1;
    }

    /// Add SLO for operation
    pub fn add_slo(&mut self, operation: &str, slo: Slo) {
        let tracker = self
            .trackers
            .entry(operation.into())
            .or_insert_with(|| OperationTracker::new(operation, self.config.max_samples));
        tracker.add_slo(slo);
    }

    /// Get tracker for operation
    pub fn get_tracker(&self, operation: &str) -> Option<&OperationTracker> {
        self.trackers.get(operation)
    }

    /// Get all SLO statuses
    pub fn all_slo_status(&self) -> Vec<SloStatus> {
        self.trackers
            .values()
            .flat_map(|t| t.slo_status())
            .collect()
    }

    /// Get global percentiles
    pub fn global_percentiles(&self) -> LatencyPercentiles {
        self.global.percentiles()
    }

    /// Cleanup timed out measurements
    pub fn cleanup(&mut self) {
        let now = Timestamp::now();
        let timeout = self.config.timeout_ns;

        let timed_out: Vec<u64> = self
            .active
            .iter()
            .filter(|(_, m)| now.elapsed_since(m.start) > timeout)
            .map(|(id, _)| *id)
            .collect();

        for id in timed_out {
            self.active.remove(&id);
            self.stats.timed_out += 1;
        }

        self.stats.active_measurements = self.active.len() as u64;
    }

    /// Get statistics
    pub fn stats(&self) -> &LatencyStats {
        &self.stats
    }

    /// Get operations
    pub fn operations(&self) -> Vec<&str> {
        self.trackers.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for LatencyManager {
    fn default() -> Self {
        Self::new(LatencyConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_histogram() {
        let mut hist = LatencyHistogram::new();

        for i in 0..100 {
            hist.record(i * 10_000); // 0-990us
        }

        assert_eq!(hist.count(), 100);
        assert!(hist.min() < 100_000);
        assert!(hist.max() > 900_000);
        assert!(hist.percentile(50.0) > 0);
    }

    #[test]
    fn test_latency_measurement() {
        let mut manager = LatencyManager::default();
        let domain = DomainId::new(1);

        let id = manager.start("test_op", domain, None, vec![]);
        let measurement = manager.end(id, true).unwrap();

        assert_eq!(measurement.operation, "test_op");
        assert!(measurement.success);
        assert!(measurement.duration_ns > 0);
    }

    #[test]
    fn test_slo() {
        let mut manager = LatencyManager::default();
        let domain = DomainId::new(1);

        manager.add_slo("test_op", Slo {
            name: "test_slo".into(),
            operation: "test_op".into(),
            target_ns: 100_000_000, // 100ms
            percentile: 95.0,
            error_budget: 0.01,
        });

        // Record some measurements
        for _ in 0..100 {
            let measurement = LatencyMeasurement {
                operation: "test_op".into(),
                start: Timestamp::now(),
                end: Timestamp::now(),
                duration_ns: 50_000_000, // 50ms
                source: domain,
                target: None,
                success: true,
                tags: vec![],
            };
            manager.record(measurement);
        }

        let slo_status = manager.all_slo_status();
        assert!(!slo_status.is_empty());
        assert!(slo_status[0].is_met);
    }

    #[test]
    fn test_percentiles_calculation() {
        let samples: Vec<u64> = (0..100).map(|i| i * 1000).collect();
        let percentiles = LatencyPercentiles::from_samples(&samples).unwrap();

        assert!(percentiles.p50 > 0);
        assert!(percentiles.p99 > percentiles.p50);
    }

    #[test]
    fn test_operation_tracker() {
        let mut tracker = OperationTracker::new("test", 1000);

        for i in 0..100 {
            tracker.record(i * 1000, true);
        }
        tracker.record(1_000_000, false);

        assert!(tracker.success_rate() > 0.99);
        assert!(tracker.percentiles().p50 > 0);
    }
}
