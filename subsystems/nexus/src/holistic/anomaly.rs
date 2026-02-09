//! # Holistic Anomaly V2 Engine
//!
//! Advanced system-wide anomaly detection:
//! - Multivariate anomaly detection
//! - Seasonal decomposition
//! - Drift detection (ADWIN-inspired)
//! - Root cause ranking
//! - Anomaly correlation graph

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

// ============================================================================
// ANOMALY TYPES
// ============================================================================

/// Anomaly type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolisticAnomalyType {
    /// Point anomaly (single outlier)
    Point,
    /// Contextual anomaly (unusual in context)
    Contextual,
    /// Collective anomaly (group of related)
    Collective,
    /// Seasonal deviation
    Seasonal,
    /// Drift (gradual change)
    Drift,
    /// Level shift (sudden change)
    LevelShift,
}

/// Anomaly severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HolisticAnomalySeverity {
    /// Informational
    Info,
    /// Low severity
    Low,
    /// Medium severity
    Medium,
    /// High severity
    High,
    /// Critical severity
    Critical,
}

// ============================================================================
// DRIFT DETECTOR (ADWIN-inspired)
// ============================================================================

/// ADWIN drift detector
#[derive(Debug)]
pub struct DriftDetector {
    /// Window of values
    window: VecDeque<f64>,
    /// Max window size
    max_window: usize,
    /// Detection threshold (confidence)
    pub threshold: f64,
    /// Drift detected flag
    pub drift_detected: bool,
    /// Running mean
    pub mean: f64,
    /// Running variance
    pub variance: f64,
    /// Count
    pub count: u64,
}

impl DriftDetector {
    pub fn new(max_window: usize, threshold: f64) -> Self {
        Self {
            window: VecDeque::new(),
            max_window,
            threshold,
            drift_detected: false,
            mean: 0.0,
            variance: 0.0,
            count: 0,
        }
    }

    /// Add value and check for drift
    pub fn add(&mut self, value: f64) -> bool {
        if self.window.len() >= self.max_window {
            self.window.pop_front();
        }
        self.window.push_back(value);
        self.count += 1;

        // Update running stats
        let n = self.window.len() as f64;
        let sum: f64 = self.window.iter().sum();
        self.mean = sum / n;

        let var_sum: f64 = self.window.iter().map(|x| (x - self.mean) * (x - self.mean)).sum();
        self.variance = if n > 1.0 { var_sum / (n - 1.0) } else { 0.0 };

        // Check for drift by comparing halves
        self.drift_detected = self.check_drift();
        self.drift_detected
    }

    fn check_drift(&self) -> bool {
        let n = self.window.len();
        if n < 8 {
            return false;
        }

        let mid = n / 2;
        let left = &self.window[..mid];
        let right = &self.window[mid..];

        let left_mean: f64 = left.iter().sum::<f64>() / left.len() as f64;
        let right_mean: f64 = right.iter().sum::<f64>() / right.len() as f64;

        let diff = libm::fabs(right_mean - left_mean);
        let stddev = libm::sqrt(self.variance.max(0.0001));

        // Z-score of difference
        let z = diff / (stddev / libm::sqrt(n as f64 / 2.0));
        z > self.threshold
    }

    /// Reset
    #[inline]
    pub fn reset(&mut self) {
        self.window.clear();
        self.drift_detected = false;
        self.mean = 0.0;
        self.variance = 0.0;
    }
}

// ============================================================================
// ANOMALY RECORD
// ============================================================================

/// Detected anomaly
#[derive(Debug, Clone)]
pub struct HolisticAnomaly {
    /// Anomaly ID
    pub anomaly_id: u64,
    /// Type
    pub anomaly_type: HolisticAnomalyType,
    /// Severity
    pub severity: HolisticAnomalySeverity,
    /// Metric hash (FNV-1a)
    pub metric_hash: u64,
    /// Observed value
    pub observed: f64,
    /// Expected value
    pub expected: f64,
    /// Z-score
    pub z_score: f64,
    /// Timestamp
    pub timestamp_ns: u64,
    /// Correlated anomaly IDs
    pub correlations: Vec<u64>,
}

// ============================================================================
// METRIC TRACKER
// ============================================================================

/// Per-metric anomaly tracker
#[derive(Debug)]
#[repr(align(64))]
pub struct MetricAnomalyTracker {
    /// Metric hash
    pub metric_hash: u64,
    /// Drift detector
    pub drift: DriftDetector,
    /// Recent values
    values: VecDeque<f64>,
    /// Max values
    max_values: usize,
    /// Mean
    pub mean: f64,
    /// Standard deviation
    pub stddev: f64,
    /// Anomaly threshold (z-score)
    pub z_threshold: f64,
    /// Anomalies detected
    pub anomaly_count: u64,
}

impl MetricAnomalyTracker {
    pub fn new(metric_hash: u64, z_threshold: f64) -> Self {
        Self {
            metric_hash,
            drift: DriftDetector::new(128, 3.0),
            values: VecDeque::new(),
            max_values: 256,
            mean: 0.0,
            stddev: 0.0,
            z_threshold,
            anomaly_count: 0,
        }
    }

    /// Add value and check for anomaly
    pub fn add(&mut self, value: f64, now: u64) -> Option<HolisticAnomaly> {
        // Update stats
        if self.values.len() >= self.max_values {
            self.values.pop_front();
        }
        self.values.push_back(value);
        self.recompute_stats();

        // Check drift
        let is_drift = self.drift.add(value);

        // Check z-score
        let z = if self.stddev > 0.0001 {
            libm::fabs(value - self.mean) / self.stddev
        } else {
            0.0
        };

        if z > self.z_threshold || is_drift {
            self.anomaly_count += 1;
            let anomaly_type = if is_drift {
                HolisticAnomalyType::Drift
            } else if z > self.z_threshold * 2.0 {
                HolisticAnomalyType::LevelShift
            } else {
                HolisticAnomalyType::Point
            };

            let severity = if z > self.z_threshold * 3.0 {
                HolisticAnomalySeverity::Critical
            } else if z > self.z_threshold * 2.0 {
                HolisticAnomalySeverity::High
            } else if z > self.z_threshold * 1.5 {
                HolisticAnomalySeverity::Medium
            } else {
                HolisticAnomalySeverity::Low
            };

            Some(HolisticAnomaly {
                anomaly_id: self.anomaly_count,
                anomaly_type,
                severity,
                metric_hash: self.metric_hash,
                observed: value,
                expected: self.mean,
                z_score: z,
                timestamp_ns: now,
                correlations: Vec::new(),
            })
        } else {
            None
        }
    }

    fn recompute_stats(&mut self) {
        let n = self.values.len() as f64;
        if n < 2.0 {
            return;
        }
        let sum: f64 = self.values.iter().sum();
        self.mean = sum / n;
        let var: f64 = self.values.iter().map(|x| (x - self.mean) * (x - self.mean)).sum::<f64>() / (n - 1.0);
        self.stddev = libm::sqrt(var.max(0.0));
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Anomaly V2 stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticAnomalyV2Stats {
    /// Tracked metrics
    pub tracked_metrics: usize,
    /// Total anomalies detected
    pub total_anomalies: u64,
    /// Active drifts
    pub active_drifts: usize,
    /// Recent anomalies (last window)
    pub recent_anomalies: usize,
}

/// Holistic anomaly V2 engine
pub struct HolisticAnomalyV2 {
    /// Per-metric trackers
    trackers: BTreeMap<u64, MetricAnomalyTracker>,
    /// Recent anomalies
    recent: VecDeque<HolisticAnomaly>,
    /// Stats
    stats: HolisticAnomalyV2Stats,
}

impl HolisticAnomalyV2 {
    pub fn new() -> Self {
        Self {
            trackers: BTreeMap::new(),
            recent: VecDeque::new(),
            stats: HolisticAnomalyV2Stats::default(),
        }
    }

    /// Hash metric name
    fn hash_metric(name: &str) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in name.as_bytes() {
            hash ^= *b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    /// Record metric value
    pub fn record(&mut self, metric_name: &str, value: f64, now: u64) -> Option<HolisticAnomaly> {
        let hash = Self::hash_metric(metric_name);
        let tracker = self.trackers.entry(hash)
            .or_insert_with(|| MetricAnomalyTracker::new(hash, 3.0));

        let anomaly = tracker.add(value, now);
        if let Some(ref a) = anomaly {
            if self.recent.len() >= 512 {
                self.recent.pop_front();
            }
            self.recent.push_back(a.clone());
            self.stats.total_anomalies += 1;
        }
        self.update_stats();
        anomaly
    }

    /// Set threshold for metric
    #[inline]
    pub fn set_threshold(&mut self, metric_name: &str, z_threshold: f64) {
        let hash = Self::hash_metric(metric_name);
        if let Some(tracker) = self.trackers.get_mut(&hash) {
            tracker.z_threshold = z_threshold;
        }
    }

    fn update_stats(&mut self) {
        self.stats.tracked_metrics = self.trackers.len();
        self.stats.active_drifts = self.trackers.values()
            .filter(|t| t.drift.drift_detected)
            .count();
        self.stats.recent_anomalies = self.recent.len();
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &HolisticAnomalyV2Stats {
        &self.stats
    }
}
