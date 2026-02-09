// SPDX-License-Identifier: GPL-2.0
//! # Apps Confidence Interval Engine
//!
//! Uncertainty quantification for application behavior predictions. Every
//! prediction the future engine makes — resource demand, phase transition
//! timing, anomaly probability — carries confidence bounds computed here.
//!
//! The engine maintains per-prediction-type error distributions, computes
//! bootstrap confidence intervals, tracks calibration quality (do 90% CI
//! actually contain the true value 90% of the time?), and identifies the
//! dominant sources of uncertainty.
//!
//! This is the apps engine knowing what it does not know.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_SAMPLES: usize = 1024;
const MAX_PREDICTION_TYPES: usize = 64;
const MAX_APPS: usize = 256;
const BOOTSTRAP_ITERATIONS: usize = 50;
const EMA_ALPHA: f64 = 0.10;
const DEFAULT_CI_LEVEL: f64 = 0.90;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const XORSHIFT_SEED: u64 = 0xbadc0ffee_0dd1ce;

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

#[inline]
fn ema_update(current: f64, sample: f64, alpha: f64) -> f64 {
    alpha * sample + (1.0 - alpha) * current
}

/// Sort a mutable slice of f64 in ascending order (insertion sort for no_std).
fn sort_f64(data: &mut [f64]) {
    for i in 1..data.len() {
        let mut j = i;
        while j > 0 && data[j] < data[j - 1] {
            data.swap(j, j - 1);
            j -= 1;
        }
    }
}

/// Compute percentile from a sorted slice.
fn percentile_sorted(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = (p * (sorted.len() - 1) as f64) as usize;
    let clamped = if idx >= sorted.len() { sorted.len() - 1 } else { idx };
    sorted[clamped]
}

// ============================================================================
// PREDICTION TYPE
// ============================================================================

/// Category of prediction being uncertainty-quantified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PredictionType {
    ResourceDemand,
    PhaseTransition,
    LifetimeEstimate,
    IoVolume,
    CpuUsage,
    MemoryPeak,
    ThreadCount,
    AnomalyProbability,
}

impl PredictionType {
    fn as_str(&self) -> &'static str {
        match self {
            Self::ResourceDemand => "resource_demand",
            Self::PhaseTransition => "phase_transition",
            Self::LifetimeEstimate => "lifetime",
            Self::IoVolume => "io_volume",
            Self::CpuUsage => "cpu_usage",
            Self::MemoryPeak => "mem_peak",
            Self::ThreadCount => "thread_count",
            Self::AnomalyProbability => "anomaly_prob",
        }
    }

    fn from_index(i: usize) -> Self {
        match i % 8 {
            0 => Self::ResourceDemand,
            1 => Self::PhaseTransition,
            2 => Self::LifetimeEstimate,
            3 => Self::IoVolume,
            4 => Self::CpuUsage,
            5 => Self::MemoryPeak,
            6 => Self::ThreadCount,
            _ => Self::AnomalyProbability,
        }
    }
}

// ============================================================================
// CONFIDENCE INTERVAL RESULT
// ============================================================================

/// A confidence interval around a point prediction.
#[derive(Debug, Clone)]
pub struct ConfidenceIntervalResult {
    pub point_estimate: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub confidence_level: f64,
    pub interval_width: f64,
    pub prediction_type: PredictionType,
    pub sample_count: usize,
}

impl ConfidenceIntervalResult {
    fn new(
        point: f64,
        lower: f64,
        upper: f64,
        level: f64,
        ptype: PredictionType,
        n: usize,
    ) -> Self {
        Self {
            point_estimate: point,
            lower_bound: lower,
            upper_bound: upper,
            confidence_level: level,
            interval_width: upper - lower,
            prediction_type: ptype,
            sample_count: n,
        }
    }

    /// Check whether an observed value falls within this interval.
    #[inline(always)]
    pub fn contains(&self, value: f64) -> bool {
        value >= self.lower_bound && value <= self.upper_bound
    }
}

// ============================================================================
// UNCERTAINTY SOURCE
// ============================================================================

/// Identified source of prediction uncertainty.
#[derive(Debug, Clone)]
pub struct UncertaintySource {
    pub source_name: String,
    pub contribution: f64,
    pub reducible: bool,
}

// ============================================================================
// PREDICTION ERROR TRACKER
// ============================================================================

#[derive(Debug, Clone)]
struct ErrorTracker {
    errors: VecDeque<f64>,
    absolute_errors: VecDeque<f64>,
    ema_error: f64,
    ema_abs_error: f64,
    coverage_hits: u64,
    coverage_total: u64,
}

impl ErrorTracker {
    fn new() -> Self {
        Self {
            errors: VecDeque::new(),
            absolute_errors: VecDeque::new(),
            ema_error: 0.0,
            ema_abs_error: 0.0,
            coverage_hits: 0,
            coverage_total: 0,
        }
    }

    fn record_error(&mut self, predicted: f64, actual: f64) {
        let error = actual - predicted;
        let abs_error = if error >= 0.0 { error } else { -error };

        self.ema_error = ema_update(self.ema_error, error, EMA_ALPHA);
        self.ema_abs_error = ema_update(self.ema_abs_error, abs_error, EMA_ALPHA);

        if self.errors.len() >= MAX_SAMPLES {
            self.errors.pop_front();
            self.absolute_errors.pop_front().unwrap();
        }
        self.errors.push_back(error);
        self.absolute_errors.push_back(abs_error);
    }

    fn record_coverage(&mut self, was_within: bool) {
        self.coverage_total += 1;
        if was_within {
            self.coverage_hits += 1;
        }
    }

    fn coverage_rate(&self) -> f64 {
        if self.coverage_total == 0 {
            return 0.0;
        }
        self.coverage_hits as f64 / self.coverage_total as f64
    }

    fn error_variance(&self) -> f64 {
        if self.errors.len() < 2 {
            return 0.0;
        }
        let mean = self.ema_error;
        let mut sum_sq = 0.0;
        for &e in &self.errors {
            let diff = e - mean;
            sum_sq += diff * diff;
        }
        sum_sq / (self.errors.len() - 1) as f64
    }

    fn error_std(&self) -> f64 {
        let var = self.error_variance();
        // Newton's method sqrt for no_std
        if var <= 0.0 {
            return 0.0;
        }
        let mut guess = var;
        for _ in 0..20 {
            guess = 0.5 * (guess + var / guess);
        }
        guess
    }
}

// ============================================================================
// PER-APP CI STATE
// ============================================================================

#[derive(Debug, Clone)]
struct AppCIState {
    app_id: u64,
    trackers: BTreeMap<u64, ErrorTracker>,
    total_predictions: u64,
    total_calibrations: u64,
}

impl AppCIState {
    fn new(app_id: u64) -> Self {
        Self {
            app_id,
            trackers: BTreeMap::new(),
            total_predictions: 0,
            total_calibrations: 0,
        }
    }

    fn get_tracker(&mut self, ptype: PredictionType) -> &mut ErrorTracker {
        let key = fnv1a_hash(ptype.as_str().as_bytes());
        self.trackers.entry(key).or_insert_with(ErrorTracker::new)
    }

    fn get_tracker_ref(&self, ptype: PredictionType) -> Option<&ErrorTracker> {
        let key = fnv1a_hash(ptype.as_str().as_bytes());
        self.trackers.get(&key)
    }
}

// ============================================================================
// CONFIDENCE INTERVAL STATS
// ============================================================================

/// Engine-level statistics for the confidence interval module.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ConfidenceIntervalStats {
    pub total_ci_computed: u64,
    pub total_bootstrap_runs: u64,
    pub average_interval_width: f64,
    pub average_coverage: f64,
    pub calibration_queries: u64,
    pub uncertainty_analyses: u64,
    pub best_calibrated_type: u64,
    pub worst_calibrated_type: u64,
}

impl ConfidenceIntervalStats {
    fn new() -> Self {
        Self {
            total_ci_computed: 0,
            total_bootstrap_runs: 0,
            average_interval_width: 0.0,
            average_coverage: 0.0,
            calibration_queries: 0,
            uncertainty_analyses: 0,
            best_calibrated_type: 0,
            worst_calibrated_type: 0,
        }
    }
}

// ============================================================================
// APPS CONFIDENCE INTERVAL ENGINE
// ============================================================================

/// Confidence interval engine for application behavior predictions.
///
/// Tracks prediction errors, computes bootstrap confidence intervals,
/// monitors calibration quality, and identifies uncertainty sources.
pub struct AppsConfidenceInterval {
    app_states: BTreeMap<u64, AppCIState>,
    stats: ConfidenceIntervalStats,
    rng_state: u64,
    tick: u64,
    ema_width: f64,
    ema_coverage: f64,
}

impl AppsConfidenceInterval {
    /// Create a new confidence interval engine.
    pub fn new() -> Self {
        Self {
            app_states: BTreeMap::new(),
            stats: ConfidenceIntervalStats::new(),
            rng_state: XORSHIFT_SEED,
            tick: 0,
            ema_width: 0.0,
            ema_coverage: 0.5,
        }
    }

    /// Record a prediction and its eventual observed value.
    #[inline]
    pub fn record_outcome(
        &mut self,
        app_id: u64,
        ptype: PredictionType,
        predicted: f64,
        actual: f64,
    ) {
        self.tick += 1;
        if self.app_states.len() >= MAX_APPS && !self.app_states.contains_key(&app_id) {
            return;
        }
        let state = self.app_states.entry(app_id).or_insert_with(|| AppCIState::new(app_id));
        state.total_predictions += 1;
        let tracker = state.get_tracker(ptype);
        tracker.record_error(predicted, actual);
    }

    /// Compute a confidence interval for a new prediction.
    ///
    /// Uses the error distribution from past predictions to place bounds
    /// around the point estimate.
    pub fn app_prediction_ci(
        &mut self,
        app_id: u64,
        ptype: PredictionType,
        point_estimate: f64,
        level: f64,
    ) -> ConfidenceIntervalResult {
        self.stats.total_ci_computed += 1;
        let ci_level = if level > 0.0 && level < 1.0 { level } else { DEFAULT_CI_LEVEL };

        let state = match self.app_states.get(&app_id) {
            Some(s) => s,
            None => {
                // No history — use wide default
                let half = point_estimate.abs() * 0.5 + 1.0;
                return ConfidenceIntervalResult::new(
                    point_estimate,
                    point_estimate - half,
                    point_estimate + half,
                    ci_level,
                    ptype,
                    0,
                );
            }
        };

        let tracker = match state.get_tracker_ref(ptype) {
            Some(t) => t,
            None => {
                let half = point_estimate.abs() * 0.3 + 0.5;
                return ConfidenceIntervalResult::new(
                    point_estimate,
                    point_estimate - half,
                    point_estimate + half,
                    ci_level,
                    ptype,
                    0,
                );
            }
        };

        let n = tracker.errors.len();
        if n < 3 {
            let half = point_estimate.abs() * 0.4 + 1.0;
            return ConfidenceIntervalResult::new(
                point_estimate,
                point_estimate - half,
                point_estimate + half,
                ci_level,
                ptype,
                n,
            );
        }

        let std = tracker.error_std();
        let bias = tracker.ema_error;
        let alpha_tail = (1.0 - ci_level) / 2.0;

        // Approximate z-score for common levels
        let z = if ci_level >= 0.99 {
            2.576
        } else if ci_level >= 0.95 {
            1.96
        } else if ci_level >= 0.90 {
            1.645
        } else {
            1.28
        };

        let lower = point_estimate + bias - z * std;
        let upper = point_estimate + bias + z * std;

        let result = ConfidenceIntervalResult::new(point_estimate, lower, upper, ci_level, ptype, n);
        self.ema_width = ema_update(self.ema_width, result.interval_width, EMA_ALPHA);
        self.stats.average_interval_width = self.ema_width;

        result
    }

    /// Bootstrap confidence interval estimation.
    ///
    /// Resamples the error distribution with replacement to estimate the
    /// sampling distribution of the mean error.
    pub fn bootstrap_estimate(
        &mut self,
        app_id: u64,
        ptype: PredictionType,
        point_estimate: f64,
    ) -> ConfidenceIntervalResult {
        self.stats.total_bootstrap_runs += 1;

        let state = match self.app_states.get(&app_id) {
            Some(s) => s,
            None => {
                let half = point_estimate.abs() * 0.5 + 1.0;
                return ConfidenceIntervalResult::new(
                    point_estimate,
                    point_estimate - half,
                    point_estimate + half,
                    DEFAULT_CI_LEVEL,
                    ptype,
                    0,
                );
            }
        };

        let tracker = match state.get_tracker_ref(ptype) {
            Some(t) if t.errors.len() >= 5 => t,
            _ => {
                let half = point_estimate.abs() * 0.4 + 1.0;
                return ConfidenceIntervalResult::new(
                    point_estimate,
                    point_estimate - half,
                    point_estimate + half,
                    DEFAULT_CI_LEVEL,
                    ptype,
                    0,
                );
            }
        };

        let n = tracker.errors.len();
        let mut bootstrap_means = Vec::with_capacity(BOOTSTRAP_ITERATIONS);

        for iter in 0..BOOTSTRAP_ITERATIONS {
            let mut sum = 0.0;
            let mut rng = self.rng_state.wrapping_add(iter as u64);
            for _ in 0..n {
                let idx = (xorshift64(&mut rng) % n as u64) as usize;
                sum += tracker.errors[idx];
            }
            bootstrap_means.push(sum / n as f64);
        }

        sort_f64(&mut bootstrap_means);

        let lower_pct = 0.05;
        let upper_pct = 0.95;
        let lower_correction = percentile_sorted(&bootstrap_means, lower_pct);
        let upper_correction = percentile_sorted(&bootstrap_means, upper_pct);

        let lower = point_estimate + lower_correction;
        let upper = point_estimate + upper_correction;

        ConfidenceIntervalResult::new(point_estimate, lower, upper, DEFAULT_CI_LEVEL, ptype, n)
    }

    /// Measure calibration quality: how often do 90% CIs contain the true value?
    pub fn calibration_quality(&mut self, app_id: u64, ptype: PredictionType) -> f64 {
        self.stats.calibration_queries += 1;
        let state = match self.app_states.get(&app_id) {
            Some(s) => s,
            None => return 0.0,
        };
        let tracker = match state.get_tracker_ref(ptype) {
            Some(t) => t,
            None => return 0.0,
        };

        let rate = tracker.coverage_rate();
        self.ema_coverage = ema_update(self.ema_coverage, rate, EMA_ALPHA);
        self.stats.average_coverage = self.ema_coverage;
        rate
    }

    /// Detailed interval analysis across all prediction types for an app.
    pub fn interval_analysis(&mut self, app_id: u64) -> Vec<(PredictionType, f64, f64, f64)> {
        let state = match self.app_states.get(&app_id) {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut results = Vec::new();
        for i in 0..8usize {
            let ptype = PredictionType::from_index(i);
            if let Some(tracker) = state.get_tracker_ref(ptype) {
                if tracker.errors.len() >= 2 {
                    let std = tracker.error_std();
                    let bias = tracker.ema_error;
                    let coverage = tracker.coverage_rate();
                    results.push((ptype, std, bias, coverage));
                }
            }
        }

        results
    }

    /// Track coverage: record whether a previously-computed CI contained the outcome.
    #[inline]
    pub fn coverage_tracking(
        &mut self,
        app_id: u64,
        ptype: PredictionType,
        ci: &ConfidenceIntervalResult,
        actual: f64,
    ) {
        if self.app_states.len() >= MAX_APPS && !self.app_states.contains_key(&app_id) {
            return;
        }
        let state = self.app_states.entry(app_id).or_insert_with(|| AppCIState::new(app_id));
        state.total_calibrations += 1;
        let within = ci.contains(actual);
        let tracker = state.get_tracker(ptype);
        tracker.record_coverage(within);
    }

    /// Identify the dominant sources of uncertainty for a prediction type.
    pub fn uncertainty_source(&mut self, app_id: u64, ptype: PredictionType) -> Vec<UncertaintySource> {
        self.stats.uncertainty_analyses += 1;
        let state = match self.app_states.get(&app_id) {
            Some(s) => s,
            None => return Vec::new(),
        };

        let tracker = match state.get_tracker_ref(ptype) {
            Some(t) if t.errors.len() >= 5 => t,
            _ => return Vec::new(),
        };

        let var = tracker.error_variance();
        let bias_sq = tracker.ema_error * tracker.ema_error;
        let total = var + bias_sq + 0.001;

        let mut sources = Vec::new();

        sources.push(UncertaintySource {
            source_name: String::from("variance"),
            contribution: var / total,
            reducible: true,
        });

        sources.push(UncertaintySource {
            source_name: String::from("bias"),
            contribution: bias_sq / total,
            reducible: true,
        });

        let n = tracker.errors.len();
        let sample_uncertainty = if n > 0 { 1.0 / n as f64 } else { 1.0 };
        sources.push(UncertaintySource {
            source_name: String::from("sample_size"),
            contribution: sample_uncertainty.min(1.0 - var / total - bias_sq / total).max(0.0),
            reducible: true,
        });

        let irreducible = 0.001 / total;
        sources.push(UncertaintySource {
            source_name: String::from("irreducible"),
            contribution: irreducible,
            reducible: false,
        });

        // Sort by contribution descending
        for i in 1..sources.len() {
            let mut j = i;
            while j > 0 && sources[j].contribution > sources[j - 1].contribution {
                sources.swap(j, j - 1);
                j -= 1;
            }
        }

        sources
    }

    /// Return a snapshot of engine statistics.
    #[inline(always)]
    pub fn stats(&self) -> &ConfidenceIntervalStats {
        &self.stats
    }

    /// Total number of tracked apps.
    #[inline(always)]
    pub fn tracked_apps(&self) -> usize {
        self.app_states.len()
    }

    /// Global EMA-smoothed average interval width.
    #[inline(always)]
    pub fn avg_interval_width(&self) -> f64 {
        self.ema_width
    }

    /// Global EMA-smoothed average coverage rate.
    #[inline(always)]
    pub fn avg_coverage(&self) -> f64 {
        self.ema_coverage
    }
}
