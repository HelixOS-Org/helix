//! # Holistic Anomaly Detection
//!
//! System-wide anomaly detection across all subsystems:
//! - Statistical anomaly detection (Z-score, IQR)
//! - Pattern-based anomalies
//! - Correlation anomalies (unusual combinations)
//! - Cascading failure detection
//! - Anomaly clustering
//! - Root cause inference

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

// ============================================================================
// ANOMALY TYPES
// ============================================================================

/// Anomaly source subsystem
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnomalySource {
    /// CPU scheduler
    Scheduler,
    /// Memory management
    Memory,
    /// I/O subsystem
    Io,
    /// Network
    Network,
    /// Thermal
    Thermal,
    /// Power
    Power,
    /// IPC
    Ipc,
    /// Filesystem
    Filesystem,
    /// Cross-subsystem
    CrossSubsystem,
}

/// Anomaly category
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HolisticAnomalyType {
    /// Value exceeds threshold
    Threshold,
    /// Statistical outlier
    StatisticalOutlier,
    /// Rate of change anomaly
    RateOfChange,
    /// Pattern break
    PatternBreak,
    /// Correlation anomaly
    CorrelationAnomaly,
    /// Cascading failure
    CascadingFailure,
    /// Resource exhaustion
    ResourceExhaustion,
    /// Performance degradation
    PerformanceDegradation,
    /// Oscillation
    Oscillation,
}

/// Anomaly severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HolisticAnomalySeverity {
    /// Informational
    Info = 0,
    /// Warning
    Warning = 1,
    /// Error
    Error = 2,
    /// Critical
    Critical = 3,
}

// ============================================================================
// ANOMALY DETECTION
// ============================================================================

/// Detected anomaly
#[derive(Debug, Clone)]
pub struct HolisticAnomaly {
    /// Anomaly ID
    pub id: u64,
    /// Source
    pub source: AnomalySource,
    /// Type
    pub anomaly_type: HolisticAnomalyType,
    /// Severity
    pub severity: HolisticAnomalySeverity,
    /// Metric name/ID
    pub metric_id: u64,
    /// Observed value
    pub observed: f64,
    /// Expected value (or threshold)
    pub expected: f64,
    /// Deviation (how far from expected)
    pub deviation: f64,
    /// Confidence (0.0-1.0)
    pub confidence: f64,
    /// Timestamp
    pub timestamp: u64,
    /// Related process (if any)
    pub related_pid: Option<u64>,
    /// Correlated anomalies
    pub correlated: Vec<u64>,
}

// ============================================================================
// METRIC TRACKER
// ============================================================================

/// Sliding window metric tracker for anomaly detection
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MetricTracker {
    /// Metric ID
    pub metric_id: u64,
    /// Source
    pub source: AnomalySource,
    /// Recent values
    values: VecDeque<f64>,
    /// Max window size
    max_window: usize,
    /// Running sum
    sum: f64,
    /// Running sum of squares
    sum_sq: f64,
    /// Count
    count: u64,
    /// Z-score threshold
    z_threshold: f64,
    /// Rate-of-change threshold (per second)
    roc_threshold: f64,
}

impl MetricTracker {
    pub fn new(metric_id: u64, source: AnomalySource) -> Self {
        Self {
            metric_id,
            source,
            values: VecDeque::new(),
            max_window: 200,
            sum: 0.0,
            sum_sq: 0.0,
            count: 0,
            z_threshold: 3.0,
            roc_threshold: f64::MAX,
        }
    }

    #[inline(always)]
    pub fn with_z_threshold(mut self, z: f64) -> Self {
        self.z_threshold = z;
        self
    }

    #[inline(always)]
    pub fn with_roc_threshold(mut self, roc: f64) -> Self {
        self.roc_threshold = roc;
        self
    }

    /// Mean
    #[inline]
    pub fn mean(&self) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        self.sum / self.count as f64
    }

    /// Standard deviation
    pub fn stddev(&self) -> f64 {
        if self.count < 2 {
            return 0.0;
        }
        let n = self.count as f64;
        let variance = (self.sum_sq - (self.sum * self.sum) / n) / (n - 1.0);
        if variance < 0.0 {
            0.0
        } else {
            libm::sqrt(variance)
        }
    }

    /// Add value and check for anomalies
    pub fn observe(&mut self, value: f64, timestamp: u64) -> Option<HolisticAnomaly> {
        let old_mean = self.mean();
        let old_stddev = self.stddev();

        self.values.push_back(value);
        if self.values.len() > self.max_window {
            let removed = self.values.pop_front().unwrap();
            self.sum -= removed;
            self.sum_sq -= removed * removed;
            self.count -= 1;
        }

        self.sum += value;
        self.sum_sq += value * value;
        self.count += 1;

        // Z-score check
        if old_stddev > 0.0 && self.count > 10 {
            let z = libm::fabs(value - old_mean) / old_stddev;
            if z > self.z_threshold {
                return Some(HolisticAnomaly {
                    id: 0,
                    source: self.source,
                    anomaly_type: HolisticAnomalyType::StatisticalOutlier,
                    severity: if z > self.z_threshold * 2.0 {
                        HolisticAnomalySeverity::Critical
                    } else if z > self.z_threshold * 1.5 {
                        HolisticAnomalySeverity::Error
                    } else {
                        HolisticAnomalySeverity::Warning
                    },
                    metric_id: self.metric_id,
                    observed: value,
                    expected: old_mean,
                    deviation: z,
                    confidence: 1.0 - 1.0 / (z * z),
                    timestamp,
                    related_pid: None,
                    correlated: Vec::new(),
                });
            }
        }

        // Rate-of-change check
        if self.values.len() >= 2 {
            let prev = self.values[self.values.len() - 2];
            let roc = libm::fabs(value - prev);
            if roc > self.roc_threshold {
                return Some(HolisticAnomaly {
                    id: 0,
                    source: self.source,
                    anomaly_type: HolisticAnomalyType::RateOfChange,
                    severity: HolisticAnomalySeverity::Warning,
                    metric_id: self.metric_id,
                    observed: roc,
                    expected: self.roc_threshold,
                    deviation: roc / self.roc_threshold,
                    confidence: 0.8,
                    timestamp,
                    related_pid: None,
                    correlated: Vec::new(),
                });
            }
        }

        None
    }
}

// ============================================================================
// CORRELATION DETECTOR
// ============================================================================

/// Correlation between two metrics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MetricCorrelation {
    /// Metric A
    pub metric_a: u64,
    /// Metric B
    pub metric_b: u64,
    /// Pearson correlation coefficient
    pub coefficient: f64,
    /// Sample count
    pub samples: u64,
    /// Is anomalous (deviation from historical correlation)
    pub anomalous: bool,
}

// ============================================================================
// CASCADE DETECTOR
// ============================================================================

/// Cascade event
#[derive(Debug, Clone)]
pub struct CascadeEvent {
    /// Root anomaly
    pub root_anomaly_id: u64,
    /// Downstream anomaly IDs
    pub downstream: Vec<u64>,
    /// Total affected subsystems
    pub affected_subsystems: Vec<AnomalySource>,
    /// Cascade depth
    pub depth: u32,
    /// Detection timestamp
    pub detected_at: u64,
}

// ============================================================================
// HOLISTIC ANOMALY MANAGER
// ============================================================================

/// Anomaly manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticAnomalyStats {
    /// Total anomalies detected
    pub total_detected: u64,
    /// Active anomalies
    pub active: usize,
    /// By severity
    pub by_severity: BTreeMap<u8, u64>,
    /// By source
    pub by_source: BTreeMap<u8, u64>,
    /// Cascades detected
    pub cascades: u64,
    /// Metric trackers
    pub trackers: usize,
}

/// Holistic anomaly manager
pub struct HolisticAnomalyManager {
    /// Metric trackers
    trackers: BTreeMap<u64, MetricTracker>,
    /// Detected anomalies
    anomalies: VecDeque<HolisticAnomaly>,
    /// Cascade events
    cascades: Vec<CascadeEvent>,
    /// Next anomaly ID
    next_id: u64,
    /// Stats
    stats: HolisticAnomalyStats,
    /// Max anomalies
    max_anomalies: usize,
    /// Cascade window (ms) — anomalies within this window may be related
    cascade_window_ms: u64,
}

impl HolisticAnomalyManager {
    pub fn new() -> Self {
        Self {
            trackers: BTreeMap::new(),
            anomalies: VecDeque::new(),
            cascades: Vec::new(),
            next_id: 1,
            stats: HolisticAnomalyStats::default(),
            max_anomalies: 2048,
            cascade_window_ms: 1000,
        }
    }

    /// Register metric tracker
    #[inline(always)]
    pub fn register_tracker(&mut self, tracker: MetricTracker) {
        self.trackers.insert(tracker.metric_id, tracker);
        self.stats.trackers = self.trackers.len();
    }

    /// Observe metric value
    pub fn observe(&mut self, metric_id: u64, value: f64, timestamp: u64) -> Option<u64> {
        let anomaly = {
            let tracker = self.trackers.get_mut(&metric_id)?;
            tracker.observe(value, timestamp)?
        };

        let id = self.next_id;
        self.next_id += 1;

        let mut anomaly = anomaly;
        anomaly.id = id;

        // Check for cascade
        self.check_cascade(&anomaly, timestamp);

        *self.stats.by_severity.entry(anomaly.severity as u8).or_insert(0) += 1;
        *self.stats.by_source.entry(anomaly.source as u8).or_insert(0) += 1;
        self.stats.total_detected += 1;

        self.anomalies.push_back(anomaly);
        if self.anomalies.len() > self.max_anomalies {
            self.anomalies.pop_front();
        }

        self.stats.active = self.anomalies.len();
        Some(id)
    }

    /// Check for cascade patterns
    fn check_cascade(&mut self, new_anomaly: &HolisticAnomaly, now: u64) {
        let window = self.cascade_window_ms;
        let recent: Vec<&HolisticAnomaly> = self
            .anomalies
            .iter()
            .filter(|a| now.saturating_sub(a.timestamp) < window)
            .collect();

        if recent.len() >= 3 {
            // Multiple anomalies in different subsystems → possible cascade
            let mut sources = Vec::new();
            let mut ids = Vec::new();
            for a in &recent {
                if !sources.contains(&a.source) {
                    sources.push(a.source);
                }
                ids.push(a.id);
            }

            if sources.len() >= 2 {
                self.cascades.push(CascadeEvent {
                    root_anomaly_id: ids.first().copied().unwrap_or(0),
                    downstream: ids,
                    affected_subsystems: sources,
                    depth: recent.len() as u32,
                    detected_at: now,
                });
                self.stats.cascades += 1;
            }
        }
    }

    /// Get recent anomalies
    #[inline(always)]
    pub fn recent_anomalies(&self, count: usize) -> &[HolisticAnomaly] {
        let start = self.anomalies.len().saturating_sub(count);
        &self.anomalies[start..]
    }

    /// Get anomaly
    #[inline(always)]
    pub fn anomaly(&self, id: u64) -> Option<&HolisticAnomaly> {
        self.anomalies.iter().find(|a| a.id == id)
    }

    /// Get stats
    #[inline(always)]
    pub fn stats(&self) -> &HolisticAnomalyStats {
        &self.stats
    }
}
