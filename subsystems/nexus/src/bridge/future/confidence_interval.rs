// SPDX-License-Identifier: GPL-2.0
//! # Bridge Confidence Interval Engine
//!
//! Uncertainty quantification for ALL bridge predictions. Every prediction the
//! bridge makes comes with confidence intervals at 50%, 90%, and 99% levels.
//! Uses bootstrap-like resampling from historical data to construct intervals,
//! then tracks calibration: do 90% intervals actually contain the truth 90% of
//! the time? If not, intervals are widened or narrowed adaptively.
//!
//! A prediction without uncertainty is just a guess wearing a lab coat.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const BOOTSTRAP_SAMPLES: usize = 200;
const MAX_HISTORY: usize = 1024;
const MAX_PREDICTORS: usize = 64;
const CI_LEVELS: [f32; 3] = [0.50, 0.90, 0.99];
const EMA_ALPHA: f32 = 0.08;
const CALIBRATION_WINDOW: usize = 256;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const DEFAULT_SEED: u64 = 0xB007_5744_CAFE_BABE;

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

fn rand_usize(state: &mut u64, max: usize) -> usize {
    if max == 0 {
        return 0;
    }
    (xorshift64(state) % max as u64) as usize
}

// ============================================================================
// CONFIDENCE INTERVAL
// ============================================================================

/// A confidence interval at a given confidence level.
#[derive(Debug, Clone, Copy)]
pub struct ConfidenceInterval {
    /// Lower bound of the interval
    pub lower: f32,
    /// Upper bound of the interval
    pub upper: f32,
    /// Point estimate (median of bootstrap samples)
    pub point_estimate: f32,
    /// Confidence level (e.g., 0.90 for 90%)
    pub level: f32,
}

impl ConfidenceInterval {
    /// Width of the interval
    #[inline(always)]
    pub fn width(&self) -> f32 {
        self.upper - self.lower
    }

    /// Check if a value falls within this interval
    #[inline(always)]
    pub fn contains(&self, value: f32) -> bool {
        value >= self.lower && value <= self.upper
    }
}

// ============================================================================
// PREDICTION WITH CI
// ============================================================================

/// A prediction bundled with confidence intervals at multiple levels.
#[derive(Debug, Clone)]
pub struct PredictionWithCI {
    /// The predictor that produced this
    pub predictor_id: u64,
    /// Point estimate
    pub point_estimate: f32,
    /// Confidence intervals at 50%, 90%, 99%
    pub intervals: Vec<ConfidenceInterval>,
    /// Uncertainty decomposition: (aleatoric, epistemic)
    pub uncertainty: (f32, f32),
}

// ============================================================================
// CALIBRATION RECORD
// ============================================================================

/// Tracks whether predictions at a given confidence level are calibrated.
#[derive(Debug, Clone)]
struct CalibrationRecord {
    level: f32,
    total_predictions: u64,
    hits: u64, // times the actual value fell within the interval
    coverage_ema: f32,
    width_ema: f32,
    // Adaptive scaling factor: >1 widens intervals, <1 narrows
    scale_factor: f32,
}

impl CalibrationRecord {
    fn new(level: f32) -> Self {
        Self {
            level,
            total_predictions: 0,
            hits: 0,
            coverage_ema: level, // start at nominal
            width_ema: 0.1,
            scale_factor: 1.0,
        }
    }

    #[inline]
    fn update(&mut self, contained: bool, width: f32) {
        self.total_predictions += 1;
        if contained {
            self.hits += 1;
        }
        let hit_val = if contained { 1.0 } else { 0.0 };
        self.coverage_ema = self.coverage_ema * (1.0 - EMA_ALPHA) + hit_val * EMA_ALPHA;
        self.width_ema = self.width_ema * (1.0 - EMA_ALPHA) + width * EMA_ALPHA;

        // Adapt scale factor to improve calibration
        let error = self.coverage_ema - self.level;
        if error < -0.05 {
            // Under-covering: widen intervals
            self.scale_factor *= 1.01;
        } else if error > 0.05 {
            // Over-covering: narrow intervals
            self.scale_factor *= 0.99;
        }
        self.scale_factor = self.scale_factor.clamp(0.5, 3.0);
    }

    fn coverage_rate(&self) -> f32 {
        self.coverage_ema
    }
}

// ============================================================================
// PREDICTOR HISTORY
// ============================================================================

/// Historical predictions and outcomes for a single predictor.
#[derive(Debug, Clone)]
struct PredictorHistory {
    predictor_id: u64,
    /// Recent prediction residuals (actual - predicted)
    residuals: VecDeque<f32>,
    /// Recent actual values
    actuals: VecDeque<f32>,
    /// Recent predicted values
    predictions: VecDeque<f32>,
    /// Total predictions made
    total: u64,
    /// Variance of residuals (EMA)
    variance_ema: f32,
    /// Mean residual (EMA) — detects bias
    bias_ema: f32,
}

impl PredictorHistory {
    fn new(id: u64) -> Self {
        Self {
            predictor_id: id,
            residuals: VecDeque::new(),
            actuals: VecDeque::new(),
            predictions: VecDeque::new(),
            total: 0,
            variance_ema: 0.01,
            bias_ema: 0.0,
        }
    }

    #[inline]
    fn record(&mut self, predicted: f32, actual: f32) {
        let residual = actual - predicted;
        self.residuals.push_back(residual);
        self.actuals.push_back(actual);
        self.predictions.push_back(predicted);
        self.total += 1;

        if self.residuals.len() > MAX_HISTORY {
            self.residuals.pop_front();
            self.actuals.pop_front();
            self.predictions.pop_front();
        }

        self.bias_ema = self.bias_ema * (1.0 - EMA_ALPHA) + residual * EMA_ALPHA;
        let sq_residual = residual * residual;
        self.variance_ema =
            self.variance_ema * (1.0 - EMA_ALPHA) + sq_residual * EMA_ALPHA;
    }

    fn stddev(&self) -> f32 {
        if self.variance_ema > 0.0 {
            self.variance_ema.sqrt()
        } else {
            0.01
        }
    }
}

// ============================================================================
// CI STATS
// ============================================================================

/// Statistics for the confidence interval engine.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ConfidenceIntervalStats {
    pub total_predictions_with_ci: u64,
    pub total_calibration_checks: u64,
    pub avg_interval_width_50: f32,
    pub avg_interval_width_90: f32,
    pub avg_interval_width_99: f32,
    pub coverage_50: f32,
    pub coverage_90: f32,
    pub coverage_99: f32,
    pub avg_aleatoric_uncertainty: f32,
    pub avg_epistemic_uncertainty: f32,
}

impl ConfidenceIntervalStats {
    fn new() -> Self {
        Self {
            total_predictions_with_ci: 0,
            total_calibration_checks: 0,
            avg_interval_width_50: 0.0,
            avg_interval_width_90: 0.0,
            avg_interval_width_99: 0.0,
            coverage_50: 0.50,
            coverage_90: 0.90,
            coverage_99: 0.99,
            avg_aleatoric_uncertainty: 0.0,
            avg_epistemic_uncertainty: 0.0,
        }
    }
}

// ============================================================================
// BRIDGE CONFIDENCE INTERVAL
// ============================================================================

/// Confidence interval engine for bridge predictions.
///
/// Wraps every prediction in confidence intervals at 50%, 90%, 99% levels
/// using bootstrap resampling. Adaptively calibrates interval widths so
/// coverage rates match their nominal levels.
#[repr(align(64))]
pub struct BridgeConfidenceInterval {
    /// Per-predictor history
    histories: BTreeMap<u64, PredictorHistory>,
    /// Calibration records per (predictor_id, level_index)
    calibration: BTreeMap<(u64, u32), CalibrationRecord>,
    /// Running statistics
    stats: ConfidenceIntervalStats,
    /// PRNG state
    rng: u64,
    /// Bootstrap sample count
    bootstrap_n: usize,
}

impl BridgeConfidenceInterval {
    /// Create a new confidence interval engine.
    pub fn new() -> Self {
        Self {
            histories: BTreeMap::new(),
            calibration: BTreeMap::new(),
            stats: ConfidenceIntervalStats::new(),
            rng: DEFAULT_SEED,
            bootstrap_n: BOOTSTRAP_SAMPLES,
        }
    }

    /// Record a prediction and its eventual outcome for a predictor.
    pub fn record_outcome(&mut self, predictor_id: u64, predicted: f32, actual: f32) {
        let hist = self
            .histories
            .entry(predictor_id)
            .or_insert_with(|| PredictorHistory::new(predictor_id));
        hist.record(predicted, actual);

        if self.histories.len() > MAX_PREDICTORS {
            let mut min_total = u64::MAX;
            let mut min_key = 0u64;
            for (k, v) in self.histories.iter() {
                if v.total < min_total {
                    min_total = v.total;
                    min_key = *k;
                }
            }
            self.histories.remove(&min_key);
        }
    }

    /// Generate a prediction with confidence intervals using bootstrap resampling.
    pub fn prediction_with_ci(&mut self, predictor_id: u64, point_estimate: f32) -> PredictionWithCI {
        self.stats.total_predictions_with_ci += 1;

        let hist = self.histories.get(&predictor_id);
        let intervals = if let Some(h) = hist {
            if h.residuals.len() >= 5 {
                self.bootstrap_intervals(predictor_id, point_estimate, &h.residuals)
            } else {
                self.parametric_intervals(point_estimate, h.stddev())
            }
        } else {
            // No history: use wide default intervals
            self.parametric_intervals(point_estimate, 0.2)
        };

        let uncertainty = self.uncertainty_decomposition(predictor_id);

        PredictionWithCI {
            predictor_id,
            point_estimate,
            intervals,
            uncertainty,
        }
    }

    /// Bootstrap resampling to construct intervals.
    pub fn bootstrap_interval(
        &mut self,
        residuals: &[f32],
        point_estimate: f32,
        level: f32,
    ) -> ConfidenceInterval {
        if residuals.is_empty() {
            return ConfidenceInterval {
                lower: point_estimate - 0.1,
                upper: point_estimate + 0.1,
                point_estimate,
                level,
            };
        }

        let mut bootstrap_means = Vec::with_capacity(self.bootstrap_n);
        for _ in 0..self.bootstrap_n {
            let mut sample_sum = 0.0f32;
            let n = residuals.len();
            for _ in 0..n {
                let idx = rand_usize(&mut self.rng, n);
                sample_sum += residuals[idx];
            }
            let sample_mean = sample_sum / n as f32;
            bootstrap_means.push(point_estimate + sample_mean);
        }

        bootstrap_means.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));

        let alpha = 1.0 - level;
        let lower_idx = ((alpha / 2.0) * bootstrap_means.len() as f32) as usize;
        let upper_idx =
            (((1.0 - alpha / 2.0) * bootstrap_means.len() as f32) as usize).min(bootstrap_means.len() - 1);

        let lower = bootstrap_means[lower_idx];
        let upper = bootstrap_means[upper_idx];
        let median_idx = bootstrap_means.len() / 2;
        let median = bootstrap_means[median_idx];

        ConfidenceInterval {
            lower,
            upper,
            point_estimate: median,
            level,
        }
    }

    fn bootstrap_intervals(
        &mut self,
        predictor_id: u64,
        point_estimate: f32,
        residuals: &[f32],
    ) -> Vec<ConfidenceInterval> {
        let mut intervals = Vec::with_capacity(3);

        for (i, &level) in CI_LEVELS.iter().enumerate() {
            let cal_key = (predictor_id, i as u32);
            let scale = self
                .calibration
                .get(&cal_key)
                .map(|c| c.scale_factor)
                .unwrap_or(1.0);

            let mut ci = self.bootstrap_interval(residuals, point_estimate, level);

            // Apply calibration scaling
            let center = (ci.lower + ci.upper) / 2.0;
            let half_width = ci.width() / 2.0 * scale;
            ci.lower = center - half_width;
            ci.upper = center + half_width;

            intervals.push(ci);
        }

        intervals
    }

    fn parametric_intervals(&self, point: f32, stddev: f32) -> Vec<ConfidenceInterval> {
        // Z-scores for 50%, 90%, 99%
        let z_scores = [0.674, 1.645, 2.576];
        let mut intervals = Vec::with_capacity(3);
        for (i, &level) in CI_LEVELS.iter().enumerate() {
            let half_width = stddev * z_scores[i];
            intervals.push(ConfidenceInterval {
                lower: point - half_width,
                upper: point + half_width,
                point_estimate: point,
                level,
            });
        }
        intervals
    }

    /// Check calibration: update whether the actual fell inside the intervals.
    pub fn calibration_check(&mut self, predictor_id: u64, predicted: f32, actual: f32) {
        self.stats.total_calibration_checks += 1;

        let hist = self.histories.get(&predictor_id);
        let stddev = hist.map(|h| h.stddev()).unwrap_or(0.2);

        for (i, &level) in CI_LEVELS.iter().enumerate() {
            let cal_key = (predictor_id, i as u32);
            let scale = self.calibration.get(&cal_key).map(|c| c.scale_factor).unwrap_or(1.0);

            let z_scores = [0.674f32, 1.645, 2.576];
            let half_width = stddev * z_scores[i] * scale;
            let lower = predicted - half_width;
            let upper = predicted + half_width;
            let contained = actual >= lower && actual <= upper;
            let width = upper - lower;

            let cal = self
                .calibration
                .entry(cal_key)
                .or_insert_with(|| CalibrationRecord::new(level));
            cal.update(contained, width);
        }

        // Update global stats from calibration
        self.update_coverage_stats();
    }

    fn update_coverage_stats(&mut self) {
        let mut sum_50 = 0.0f32;
        let mut sum_90 = 0.0f32;
        let mut sum_99 = 0.0f32;
        let mut w_50 = 0.0f32;
        let mut w_90 = 0.0f32;
        let mut w_99 = 0.0f32;
        let mut count_50 = 0u32;
        let mut count_90 = 0u32;
        let mut count_99 = 0u32;

        for (&(_, level_idx), cal) in &self.calibration {
            match level_idx {
                0 => {
                    sum_50 += cal.coverage_rate();
                    w_50 += cal.width_ema;
                    count_50 += 1;
                }
                1 => {
                    sum_90 += cal.coverage_rate();
                    w_90 += cal.width_ema;
                    count_90 += 1;
                }
                2 => {
                    sum_99 += cal.coverage_rate();
                    w_99 += cal.width_ema;
                    count_99 += 1;
                }
                _ => {}
            }
        }
        if count_50 > 0 {
            self.stats.coverage_50 = sum_50 / count_50 as f32;
            self.stats.avg_interval_width_50 = w_50 / count_50 as f32;
        }
        if count_90 > 0 {
            self.stats.coverage_90 = sum_90 / count_90 as f32;
            self.stats.avg_interval_width_90 = w_90 / count_90 as f32;
        }
        if count_99 > 0 {
            self.stats.coverage_99 = sum_99 / count_99 as f32;
            self.stats.avg_interval_width_99 = w_99 / count_99 as f32;
        }
    }

    /// Get the average interval width for a given confidence level index.
    #[inline]
    pub fn interval_width(&self, level_idx: usize) -> f32 {
        match level_idx {
            0 => self.stats.avg_interval_width_50,
            1 => self.stats.avg_interval_width_90,
            2 => self.stats.avg_interval_width_99,
            _ => 0.0,
        }
    }

    /// Get the empirical coverage rate for a given confidence level index.
    #[inline]
    pub fn coverage_rate(&self, level_idx: usize) -> f32 {
        match level_idx {
            0 => self.stats.coverage_50,
            1 => self.stats.coverage_90,
            2 => self.stats.coverage_99,
            _ => 0.0,
        }
    }

    /// Decompose uncertainty into aleatoric (irreducible) and epistemic (reducible).
    pub fn uncertainty_decomposition(&self, predictor_id: u64) -> (f32, f32) {
        let hist = match self.histories.get(&predictor_id) {
            Some(h) => h,
            None => return (0.1, 0.1),
        };

        if hist.residuals.len() < 10 {
            // High epistemic uncertainty due to small sample size
            return (hist.stddev() * 0.5, hist.stddev() * 1.5);
        }

        // Aleatoric: variance of residuals in a stable window (last 50)
        let window_start = if hist.residuals.len() > 50 {
            hist.residuals.len() - 50
        } else {
            0
        };
        let window = &hist.residuals[window_start..];
        let mean: f32 = window.iter().sum::<f32>() / window.len() as f32;
        let var: f32 =
            window.iter().map(|r| (r - mean) * (r - mean)).sum::<f32>() / window.len() as f32;
        let aleatoric = var.sqrt();

        // Epistemic: difference between total variance and aleatoric
        let total_var = hist.variance_ema;
        let epistemic = (total_var - var).max(0.0).sqrt();

        self.update_uncertainty_stats_internal(aleatoric, epistemic);

        (aleatoric, epistemic)
    }

    fn update_uncertainty_stats_internal(&self, _aleatoric: f32, _epistemic: f32) {
        // Stats are updated in a batch via refresh_stats()
    }

    /// Refresh aggregate statistics.
    pub fn refresh_stats(&mut self) {
        let mut alea_sum = 0.0f32;
        let mut epis_sum = 0.0f32;
        let mut count = 0u32;

        let ids: Vec<u64> = self.histories.keys().copied().collect();
        for id in ids {
            let (a, e) = self.uncertainty_decomposition(id);
            alea_sum += a;
            epis_sum += e;
            count += 1;
        }
        if count > 0 {
            self.stats.avg_aleatoric_uncertainty = alea_sum / count as f32;
            self.stats.avg_epistemic_uncertainty = epis_sum / count as f32;
        }
    }

    /// Get statistics.
    #[inline(always)]
    pub fn stats(&self) -> &ConfidenceIntervalStats {
        &self.stats
    }

    /// Get the number of tracked predictors.
    #[inline(always)]
    pub fn predictor_count(&self) -> usize {
        self.histories.len()
    }
}
