// SPDX-License-Identifier: GPL-2.0
//! # Apps Self-Model
//!
//! The apps engine's complete model of its own classification and prediction
//! capabilities. Tracks classification accuracy per workload type, prediction
//! hit rates, adaptation success rates, and calibration quality. All metrics
//! are smoothed with exponential moving averages and bounded by confidence
//! intervals derived from observed variance.
//!
//! A kernel that models its own classification fidelity can predict when
//! it is likely to misclassify and proactively seek more evidence.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.14;
const CONFIDENCE_Z: f32 = 1.96; // 95% CI
const MAX_ACCURACY_HISTORY: usize = 256;
const MAX_WORKLOAD_TYPES: usize = 64;
const CALIBRATION_BINS: usize = 10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

// ============================================================================
// FNV-1a HASHING
// ============================================================================

fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

// ============================================================================
// WORKLOAD ACCURACY TRACKER
// ============================================================================

/// Per-workload classification accuracy tracking
#[derive(Debug, Clone)]
pub struct WorkloadAccuracy {
    /// Human-readable workload name
    pub name: String,
    /// Hashed identifier for fast lookup
    pub id: u64,
    /// EMA-smoothed classification accuracy (0.0 – 1.0)
    pub accuracy: f32,
    /// EMA-smoothed prediction hit rate (0.0 – 1.0)
    pub prediction_hit_rate: f32,
    /// EMA-smoothed adaptation success rate (0.0 – 1.0)
    pub adaptation_success: f32,
    /// Variance accumulator for confidence intervals
    pub variance_accum: f32,
    /// Number of classification attempts
    pub attempts: u64,
    /// Number of correct classifications
    pub correct: u64,
    /// Raw accuracy sample history (ring buffer)
    history: Vec<f32>,
    /// Write index into history ring
    write_idx: usize,
    /// Peak accuracy ever observed
    pub peak_accuracy: f32,
    /// Tick of last update
    pub last_update_tick: u64,
}

impl WorkloadAccuracy {
    pub fn new(name: String) -> Self {
        let id = fnv1a_hash(name.as_bytes());
        Self {
            name,
            id,
            accuracy: 0.5,
            prediction_hit_rate: 0.5,
            adaptation_success: 0.5,
            variance_accum: 0.0,
            attempts: 0,
            correct: 0,
            history: Vec::new(),
            write_idx: 0,
            peak_accuracy: 0.5,
            last_update_tick: 0,
        }
    }

    /// Record a classification result for this workload
    pub fn record(&mut self, was_correct: bool, confidence: f32, tick: u64) {
        let raw = if was_correct { 1.0 } else { 0.0 };
        self.attempts += 1;
        if was_correct {
            self.correct += 1;
        }
        self.last_update_tick = tick;

        // EMA update
        self.accuracy = EMA_ALPHA * raw + (1.0 - EMA_ALPHA) * self.accuracy;
        if self.accuracy > self.peak_accuracy {
            self.peak_accuracy = self.accuracy;
        }

        // Online variance (Welford-like with EMA weighting)
        let diff = raw - self.accuracy;
        self.variance_accum = EMA_ALPHA * diff * diff + (1.0 - EMA_ALPHA) * self.variance_accum;

        // Ring buffer
        let _ = confidence; // captured in calibration layer
        if self.history.len() < MAX_ACCURACY_HISTORY {
            self.history.push(raw);
        } else {
            self.history[self.write_idx] = raw;
        }
        self.write_idx = (self.write_idx + 1) % MAX_ACCURACY_HISTORY;
    }

    /// Update prediction hit rate with a new outcome
    pub fn update_prediction(&mut self, hit: bool) {
        let raw = if hit { 1.0 } else { 0.0 };
        self.prediction_hit_rate = EMA_ALPHA * raw + (1.0 - EMA_ALPHA) * self.prediction_hit_rate;
    }

    /// Update adaptation success rate
    pub fn update_adaptation(&mut self, success: bool) {
        let raw = if success { 1.0 } else { 0.0 };
        self.adaptation_success = EMA_ALPHA * raw + (1.0 - EMA_ALPHA) * self.adaptation_success;
    }

    /// 95% confidence interval half-width around accuracy
    pub fn confidence_half_width(&self) -> f32 {
        if self.attempts < 2 {
            return 0.5;
        }
        let std_dev = libm::sqrtf(self.variance_accum);
        let n_sqrt = libm::sqrtf(self.attempts.min(MAX_ACCURACY_HISTORY as u64) as f32);
        CONFIDENCE_Z * std_dev / n_sqrt
    }

    /// Improvement trajectory: slope of recent accuracy trend
    pub fn improvement_trajectory(&self) -> f32 {
        let len = self.history.len();
        if len < 4 {
            return 0.0;
        }
        let mid = len / 2;
        let first_avg: f32 = self.history[..mid].iter().sum::<f32>() / mid as f32;
        let second_avg: f32 = self.history[mid..].iter().sum::<f32>() / (len - mid) as f32;
        second_avg - first_avg
    }
}

// ============================================================================
// CALIBRATION TRACKER
// ============================================================================

/// Tracks calibration quality — how well confidence matches actual accuracy.
/// Uses binned approach: predictions are grouped by confidence decile and
/// compared with actual hit rates.
#[derive(Debug, Clone)]
struct CalibrationBin {
    confidence_sum: f32,
    accuracy_sum: f32,
    count: u64,
}

impl CalibrationBin {
    fn new() -> Self {
        Self { confidence_sum: 0.0, accuracy_sum: 0.0, count: 0 }
    }

    fn record(&mut self, confidence: f32, correct: bool) {
        self.confidence_sum += confidence;
        self.accuracy_sum += if correct { 1.0 } else { 0.0 };
        self.count += 1;
    }

    fn expected_confidence(&self) -> f32 {
        if self.count == 0 { return 0.0; }
        self.confidence_sum / self.count as f32
    }

    fn actual_accuracy(&self) -> f32 {
        if self.count == 0 { return 0.0; }
        self.accuracy_sum / self.count as f32
    }
}

// ============================================================================
// SELF-MODEL STATS
// ============================================================================

/// Aggregate statistics about the apps self-model
#[derive(Debug, Clone, Copy, Default)]
pub struct SelfModelStats {
    pub workload_types_tracked: usize,
    pub avg_classification_accuracy: f32,
    pub avg_prediction_hit_rate: f32,
    pub avg_adaptation_success: f32,
    pub avg_confidence_width: f32,
    pub overall_calibration_error: f32,
    pub total_classification_attempts: u64,
    pub overall_improvement_rate: f32,
}

// ============================================================================
// APPS SELF-MODEL
// ============================================================================

/// The apps engine's model of its own classification and prediction
/// capabilities — per-workload accuracy tracking with EMA smoothing,
/// calibration bins, and confidence intervals.
#[derive(Debug)]
pub struct AppsSelfModel {
    /// Per-workload accuracy trackers keyed by FNV-1a hash
    workloads: BTreeMap<u64, WorkloadAccuracy>,
    /// Calibration bins for ECE computation
    calibration_bins: Vec<CalibrationBin>,
    /// Monotonic tick counter
    tick: u64,
    /// Global EMA-smoothed classification accuracy
    global_accuracy_ema: f32,
    /// Cached aggregate stats
    cached_stats: SelfModelStats,
    /// Tick at which stats were last computed
    stats_tick: u64,
}

impl AppsSelfModel {
    pub fn new() -> Self {
        let mut bins = Vec::with_capacity(CALIBRATION_BINS);
        for _ in 0..CALIBRATION_BINS {
            bins.push(CalibrationBin::new());
        }
        Self {
            workloads: BTreeMap::new(),
            calibration_bins: bins,
            tick: 0,
            global_accuracy_ema: 0.5,
            cached_stats: SelfModelStats::default(),
            stats_tick: 0,
        }
    }

    /// Update classification accuracy for a workload type
    pub fn update_accuracy(
        &mut self,
        workload_name: &str,
        was_correct: bool,
        confidence: f32,
    ) {
        self.tick += 1;
        let id = fnv1a_hash(workload_name.as_bytes());
        let tick = self.tick;
        let tracker = self.workloads.entry(id).or_insert_with(|| {
            WorkloadAccuracy::new(String::from(workload_name))
        });
        let clamped_conf = confidence.max(0.0).min(1.0);
        tracker.record(was_correct, clamped_conf, tick);

        // Update calibration bins
        let bin_idx = ((clamped_conf * CALIBRATION_BINS as f32) as usize)
            .min(CALIBRATION_BINS - 1);
        self.calibration_bins[bin_idx].record(clamped_conf, was_correct);

        // Update global EMA
        let raw = if was_correct { 1.0 } else { 0.0 };
        self.global_accuracy_ema =
            EMA_ALPHA * raw + (1.0 - EMA_ALPHA) * self.global_accuracy_ema;
    }

    /// Assess classification quality for a specific workload
    pub fn assess_classification(&self, workload_name: &str) -> Option<(f32, f32, f32)> {
        let id = fnv1a_hash(workload_name.as_bytes());
        self.workloads.get(&id).map(|w| {
            let hw = w.confidence_half_width();
            (w.accuracy, (w.accuracy - hw).max(0.0), (w.accuracy + hw).min(1.0))
        })
    }

    /// Model confidence: how confident the self-model is about its own accuracy
    pub fn model_confidence(&self) -> f32 {
        if self.workloads.is_empty() {
            return 0.0;
        }
        // Inverse of average confidence interval width
        let total_width: f32 = self.workloads.values()
            .map(|w| w.confidence_half_width() * 2.0)
            .sum();
        let avg_width = total_width / self.workloads.len() as f32;
        (1.0 - avg_width).max(0.0).min(1.0)
    }

    /// Expected Calibration Error — how well confidence matches accuracy
    pub fn calibration_error(&self) -> f32 {
        let total_samples: u64 = self.calibration_bins.iter().map(|b| b.count).sum();
        if total_samples == 0 {
            return 1.0;
        }
        let mut ece: f32 = 0.0;
        for bin in &self.calibration_bins {
            if bin.count == 0 {
                continue;
            }
            let weight = bin.count as f32 / total_samples as f32;
            let gap = (bin.expected_confidence() - bin.actual_accuracy()).abs();
            ece += weight * gap;
        }
        ece
    }

    /// Overall improvement trajectory across all workloads
    pub fn improvement_trajectory(&self) -> f32 {
        if self.workloads.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.workloads.values()
            .map(|w| w.improvement_trajectory())
            .sum();
        sum / self.workloads.len() as f32
    }

    /// Update prediction hit rate for a workload
    pub fn record_prediction(&mut self, workload_name: &str, hit: bool) {
        let id = fnv1a_hash(workload_name.as_bytes());
        if let Some(w) = self.workloads.get_mut(&id) {
            w.update_prediction(hit);
        }
    }

    /// Update adaptation success for a workload
    pub fn record_adaptation(&mut self, workload_name: &str, success: bool) {
        let id = fnv1a_hash(workload_name.as_bytes());
        if let Some(w) = self.workloads.get_mut(&id) {
            w.update_adaptation(success);
        }
    }

    /// Compute and return aggregate statistics
    pub fn stats(&mut self) -> SelfModelStats {
        if self.tick == self.stats_tick && self.cached_stats.total_classification_attempts > 0 {
            return self.cached_stats;
        }
        let n = self.workloads.len();
        let (avg_acc, avg_pred, avg_adapt, avg_ci, total_att) = if n > 0 {
            let acc: f32 = self.workloads.values().map(|w| w.accuracy).sum::<f32>() / n as f32;
            let pred: f32 = self.workloads.values().map(|w| w.prediction_hit_rate).sum::<f32>() / n as f32;
            let adapt: f32 = self.workloads.values().map(|w| w.adaptation_success).sum::<f32>() / n as f32;
            let ci: f32 = self.workloads.values().map(|w| w.confidence_half_width() * 2.0).sum::<f32>() / n as f32;
            let att: u64 = self.workloads.values().map(|w| w.attempts).sum();
            (acc, pred, adapt, ci, att)
        } else {
            (0.0, 0.0, 0.0, 1.0, 0)
        };

        self.cached_stats = SelfModelStats {
            workload_types_tracked: n,
            avg_classification_accuracy: avg_acc,
            avg_prediction_hit_rate: avg_pred,
            avg_adaptation_success: avg_adapt,
            avg_confidence_width: avg_ci,
            overall_calibration_error: self.calibration_error(),
            total_classification_attempts: total_att,
            overall_improvement_rate: self.improvement_trajectory(),
        };
        self.stats_tick = self.tick;
        self.cached_stats
    }

    /// Report per-workload accuracy with confidence intervals
    pub fn workload_report(&self) -> Vec<(String, f32, f32, f32)> {
        self.workloads.values().map(|w| {
            let hw = w.confidence_half_width();
            (w.name.clone(), w.accuracy, (w.accuracy - hw).max(0.0), (w.accuracy + hw).min(1.0))
        }).collect()
    }
}
