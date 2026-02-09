// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Prediction Validator
//!
//! Validates cooperation predictions by comparing predicted trust, contention,
//! and other cooperation metrics against actual observed values. Tracks
//! calibration, drift, and triggers model recalibration when accuracy degrades.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// FNV-1a hash for deterministic key generation.
fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Xorshift64 PRNG for perturbation and jitter.
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// EMA update for running averages.
fn ema_update(current: u64, sample: u64, alpha_num: u64, alpha_den: u64) -> u64 {
    let old_part = current.saturating_mul(alpha_den.saturating_sub(alpha_num));
    let new_part = sample.saturating_mul(alpha_num);
    old_part.saturating_add(new_part) / alpha_den.max(1)
}

/// Trust prediction validation result.
#[derive(Clone, Debug)]
pub struct TrustValidation {
    pub partner_id: u64,
    pub predicted_trust: u64,
    pub actual_trust: u64,
    pub absolute_error: u64,
    pub relative_error: u64,
    pub within_tolerance: bool,
}

/// Contention accuracy assessment.
#[derive(Clone, Debug)]
pub struct ContentionAccuracy {
    pub resource_id: u64,
    pub predicted_contention: u64,
    pub actual_contention: u64,
    pub accuracy_score: u64,
    pub bias_direction: i64,
    pub samples_count: u64,
}

/// Calibration check result.
#[derive(Clone, Debug)]
pub struct CalibrationResult {
    pub model_hash: u64,
    pub overall_calibration: u64,
    pub overestimate_ratio: u64,
    pub underestimate_ratio: u64,
    pub mean_absolute_error: u64,
    pub needs_recalibration: bool,
}

/// Prediction drift measurement.
#[derive(Clone, Debug)]
pub struct DriftMeasurement {
    pub metric_hash: u64,
    pub drift_magnitude: u64,
    pub drift_direction: i64,
    pub drift_velocity: u64,
    pub stable: bool,
    pub time_since_stable: u64,
}

/// Recalibration action result.
#[derive(Clone, Debug)]
pub struct RecalibrationResult {
    pub model_hash: u64,
    pub old_bias: i64,
    pub new_bias: i64,
    pub old_scale: u64,
    pub new_scale: u64,
    pub improvement_estimate: u64,
    pub parameters_adjusted: u64,
}

/// Rolling statistics for the validator.
#[derive(Clone, Debug)]
pub struct ValidatorStats {
    pub trust_validations: u64,
    pub contention_validations: u64,
    pub calibration_checks: u64,
    pub drift_measurements: u64,
    pub recalibrations: u64,
    pub avg_trust_accuracy: u64,
    pub avg_contention_accuracy: u64,
    pub avg_calibration: u64,
}

impl ValidatorStats {
    pub fn new() -> Self {
        Self {
            trust_validations: 0,
            contention_validations: 0,
            calibration_checks: 0,
            drift_measurements: 0,
            recalibrations: 0,
            avg_trust_accuracy: 500,
            avg_contention_accuracy: 500,
            avg_calibration: 500,
        }
    }
}

/// Internal pair of predicted and actual values.
#[derive(Clone, Debug)]
struct PredictionPair {
    predicted: u64,
    actual: u64,
    tick: u64,
}

/// Internal model calibration state.
#[derive(Clone, Debug)]
struct CalibrationState {
    model_hash: u64,
    bias: i64,
    scale_factor: u64,
    error_history: Vec<u64>,
    overestimates: u64,
    underestimates: u64,
    last_calibration_tick: u64,
}

/// Internal drift tracker for a specific metric.
#[derive(Clone, Debug)]
struct DriftTracker {
    metric_hash: u64,
    error_trend: Vec<i64>,
    ema_drift: i64,
    last_stable_tick: u64,
    stable_threshold: u64,
}

/// Cooperation prediction validator engine.
pub struct CoopPredictionValidator {
    trust_pairs: BTreeMap<u64, Vec<PredictionPair>>,
    contention_pairs: BTreeMap<u64, Vec<PredictionPair>>,
    calibrations: BTreeMap<u64, CalibrationState>,
    drift_trackers: BTreeMap<u64, DriftTracker>,
    stats: ValidatorStats,
    rng_state: u64,
    current_tick: u64,
    max_history: usize,
    tolerance: u64,
}

impl CoopPredictionValidator {
    /// Create a new prediction validator with the given seed.
    pub fn new(seed: u64) -> Self {
        Self {
            trust_pairs: BTreeMap::new(),
            contention_pairs: BTreeMap::new(),
            calibrations: BTreeMap::new(),
            drift_trackers: BTreeMap::new(),
            stats: ValidatorStats::new(),
            rng_state: seed | 1,
            current_tick: 0,
            max_history: 128,
            tolerance: 100,
        }
    }

    /// Advance the internal tick counter.
    pub fn tick(&mut self, now: u64) {
        self.current_tick = now;
    }

    /// Set the tolerance for within-tolerance checks (0-1000 scale).
    pub fn set_tolerance(&mut self, tol: u64) {
        self.tolerance = tol.min(500);
    }

    /// Validate a trust prediction against the actual observed value.
    pub fn validate_trust_prediction(
        &mut self,
        partner_id: u64,
        predicted: u64,
        actual: u64,
    ) -> TrustValidation {
        self.stats.trust_validations = self.stats.trust_validations.saturating_add(1);

        let key = fnv1a_hash(&partner_id.to_le_bytes());
        let pairs = self.trust_pairs.entry(key).or_insert_with(Vec::new);
        if pairs.len() >= self.max_history {
            pairs.remove(0);
        }
        pairs.push(PredictionPair {
            predicted,
            actual,
            tick: self.current_tick,
        });

        let abs_error = if predicted > actual {
            predicted - actual
        } else {
            actual - predicted
        };
        let rel_error = abs_error.saturating_mul(1000) / actual.max(1);
        let within_tol = abs_error <= self.tolerance;

        let accuracy = 1000u64.saturating_sub(rel_error.min(1000));
        self.stats.avg_trust_accuracy = ema_update(self.stats.avg_trust_accuracy, accuracy, 3, 10);

        self.update_calibration(key, predicted, actual);
        self.update_drift(key, predicted, actual);

        TrustValidation {
            partner_id,
            predicted_trust: predicted,
            actual_trust: actual,
            absolute_error: abs_error,
            relative_error: rel_error,
            within_tolerance: within_tol,
        }
    }

    /// Assess contention prediction accuracy for a resource.
    pub fn contention_accuracy(
        &mut self,
        resource_id: u64,
        predicted: u64,
        actual: u64,
    ) -> ContentionAccuracy {
        self.stats.contention_validations = self.stats.contention_validations.saturating_add(1);

        let key = fnv1a_hash(&resource_id.to_le_bytes());
        let pairs = self.contention_pairs.entry(key).or_insert_with(Vec::new);
        if pairs.len() >= self.max_history {
            pairs.remove(0);
        }
        pairs.push(PredictionPair {
            predicted,
            actual,
            tick: self.current_tick,
        });

        let abs_error = if predicted > actual {
            predicted - actual
        } else {
            actual - predicted
        };
        let accuracy = 1000u64
            .saturating_sub(abs_error.saturating_mul(1000) / actual.max(1))
            .min(1000);

        let bias = predicted as i64 - actual as i64;
        let sample_count = pairs.len() as u64;

        self.stats.avg_contention_accuracy =
            ema_update(self.stats.avg_contention_accuracy, accuracy, 3, 10);

        self.update_calibration(key, predicted, actual);
        self.update_drift(key, predicted, actual);

        ContentionAccuracy {
            resource_id,
            predicted_contention: predicted,
            actual_contention: actual,
            accuracy_score: accuracy,
            bias_direction: bias.signum(),
            samples_count: sample_count,
        }
    }

    /// Perform a calibration check across all tracked models.
    pub fn calibration_check(&mut self) -> Vec<CalibrationResult> {
        self.stats.calibration_checks = self.stats.calibration_checks.saturating_add(1);

        let keys: Vec<u64> = self.calibrations.keys().copied().collect();
        let mut results = Vec::new();

        for key in keys {
            if let Some(cal) = self.calibrations.get(&key) {
                let total = cal.overestimates.saturating_add(cal.underestimates);
                let over_ratio = if total > 0 {
                    cal.overestimates.saturating_mul(1000) / total
                } else {
                    500
                };
                let under_ratio = 1000u64.saturating_sub(over_ratio);

                let mae = if cal.error_history.is_empty() {
                    0
                } else {
                    cal.error_history.iter().sum::<u64>() / cal.error_history.len() as u64
                };

                let balance = if over_ratio > under_ratio {
                    over_ratio - under_ratio
                } else {
                    under_ratio - over_ratio
                };
                let calibration_score = 1000u64.saturating_sub(balance).saturating_sub(mae / 2);
                let needs_recal = calibration_score < 500 || mae > 200;

                self.stats.avg_calibration =
                    ema_update(self.stats.avg_calibration, calibration_score, 2, 10);

                results.push(CalibrationResult {
                    model_hash: key,
                    overall_calibration: calibration_score.min(1000),
                    overestimate_ratio: over_ratio,
                    underestimate_ratio: under_ratio,
                    mean_absolute_error: mae,
                    needs_recalibration: needs_recal,
                });
            }
        }
        results
    }

    /// Measure prediction drift for a specific metric.
    pub fn prediction_drift(&mut self, metric_name: &str) -> DriftMeasurement {
        self.stats.drift_measurements = self.stats.drift_measurements.saturating_add(1);

        let key = fnv1a_hash(metric_name.as_bytes());
        let tracker = self
            .drift_trackers
            .entry(key)
            .or_insert_with(|| DriftTracker {
                metric_hash: key,
                error_trend: Vec::new(),
                ema_drift: 0,
                last_stable_tick: 0,
                stable_threshold: 50,
            });

        let magnitude = if tracker.ema_drift >= 0 {
            tracker.ema_drift as u64
        } else {
            (-tracker.ema_drift) as u64
        };
        let direction = if tracker.ema_drift > 0 {
            1i64
        } else if tracker.ema_drift < 0 {
            -1i64
        } else {
            0i64
        };

        let velocity = if tracker.error_trend.len() >= 2 {
            let recent = tracker.error_trend.len();
            let last = tracker.error_trend[recent - 1];
            let prev = tracker.error_trend[recent - 2];
            let diff = last - prev;
            if diff >= 0 {
                diff as u64
            } else {
                (-diff) as u64
            }
        } else {
            0
        };

        let stable = magnitude < tracker.stable_threshold as u64 && velocity < 20;
        let time_since = if stable {
            0
        } else {
            self.current_tick.saturating_sub(tracker.last_stable_tick)
        };
        if stable {
            tracker.last_stable_tick = self.current_tick;
        }

        DriftMeasurement {
            metric_hash: key,
            drift_magnitude: magnitude,
            drift_direction: direction,
            drift_velocity: velocity,
            stable,
            time_since_stable: time_since,
        }
    }

    /// Recalibrate a prediction model based on accumulated error data.
    pub fn recalibrate_model(&mut self, model_name: &str) -> RecalibrationResult {
        self.stats.recalibrations = self.stats.recalibrations.saturating_add(1);

        let key = fnv1a_hash(model_name.as_bytes());
        let cal = self
            .calibrations
            .entry(key)
            .or_insert_with(|| CalibrationState {
                model_hash: key,
                bias: 0,
                scale_factor: 1000,
                error_history: Vec::new(),
                overestimates: 0,
                underestimates: 0,
                last_calibration_tick: 0,
            });

        let old_bias = cal.bias;
        let old_scale = cal.scale_factor;

        let total = cal.overestimates.saturating_add(cal.underestimates);
        if total > 0 {
            let over_frac = cal.overestimates as i64 * 1000 / total as i64;
            let bias_correction = (over_frac - 500) / 5;
            cal.bias = cal.bias - bias_correction;
        }

        let mae = if cal.error_history.is_empty() {
            0
        } else {
            cal.error_history.iter().sum::<u64>() / cal.error_history.len() as u64
        };

        if mae > 100 {
            let scale_adjustment = mae / 10;
            if cal.overestimates > cal.underestimates {
                cal.scale_factor = cal.scale_factor.saturating_sub(scale_adjustment).max(500);
            } else {
                cal.scale_factor = cal.scale_factor.saturating_add(scale_adjustment).min(1500);
            }
        }

        let noise = xorshift64(&mut self.rng_state) % 20;
        let improvement = mae.saturating_mul(3) / 10 + noise;

        let params_adjusted = if old_bias != cal.bias { 1u64 } else { 0u64 }
            + if old_scale != cal.scale_factor {
                1u64
            } else {
                0u64
            };

        cal.error_history.clear();
        cal.overestimates = 0;
        cal.underestimates = 0;
        cal.last_calibration_tick = self.current_tick;

        RecalibrationResult {
            model_hash: key,
            old_bias,
            new_bias: cal.bias,
            old_scale,
            new_scale: cal.scale_factor,
            improvement_estimate: improvement,
            parameters_adjusted: params_adjusted,
        }
    }

    /// Get the current statistics snapshot.
    pub fn stats(&self) -> &ValidatorStats {
        &self.stats
    }

    /// Update calibration state for a model given a prediction pair.
    fn update_calibration(&mut self, key: u64, predicted: u64, actual: u64) {
        let cal = self
            .calibrations
            .entry(key)
            .or_insert_with(|| CalibrationState {
                model_hash: key,
                bias: 0,
                scale_factor: 1000,
                error_history: Vec::new(),
                overestimates: 0,
                underestimates: 0,
                last_calibration_tick: 0,
            });

        let error = if predicted > actual {
            cal.overestimates = cal.overestimates.saturating_add(1);
            predicted - actual
        } else {
            cal.underestimates = cal.underestimates.saturating_add(1);
            actual - predicted
        };

        if cal.error_history.len() >= self.max_history {
            cal.error_history.remove(0);
        }
        cal.error_history.push(error);

        let signed_error = predicted as i64 - actual as i64;
        cal.bias = (cal.bias * 9 + signed_error) / 10;
    }

    /// Update drift tracking for a metric.
    fn update_drift(&mut self, key: u64, predicted: u64, actual: u64) {
        let tracker = self
            .drift_trackers
            .entry(key)
            .or_insert_with(|| DriftTracker {
                metric_hash: key,
                error_trend: Vec::new(),
                ema_drift: 0,
                last_stable_tick: self.current_tick,
                stable_threshold: 50,
            });

        let signed_error = predicted as i64 - actual as i64;
        if tracker.error_trend.len() >= self.max_history {
            tracker.error_trend.remove(0);
        }
        tracker.error_trend.push(signed_error);

        tracker.ema_drift = (tracker.ema_drift * 7 + signed_error * 3) / 10;
    }
}
