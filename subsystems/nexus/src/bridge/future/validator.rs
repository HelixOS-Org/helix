// SPDX-License-Identifier: GPL-2.0
//! # Bridge Prediction Validator
//!
//! Validates bridge predictions against reality. Tracks prediction accuracy,
//! calibration quality (are probabilities honest?), Brier score, log-loss,
//! and sharpness (are predictions decisive?). The reliability diagram reveals
//! systematic over- or under-confidence at each probability bucket.
//!
//! A prediction engine that doesn't validate itself is just a random number
//! generator with extra steps.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_PREDICTIONS: usize = 2048;
const CALIBRATION_BUCKETS: usize = 10;
const EMA_ALPHA: f32 = 0.08;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const MIN_SAMPLES_FOR_CALIBRATION: u64 = 5;
const LOG_LOSS_EPSILON: f32 = 0.0001;

fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Approximate natural log for no_std
fn ln_approx(x: f32) -> f32 {
    if x <= 0.0 {
        return -10.0;
    }
    let bits = x.to_bits();
    let exp = ((bits >> 23) & 0xFF) as f32 - 127.0;
    let mantissa = f32::from_bits((bits & 0x007F_FFFF) | 0x3F80_0000);
    let ln2 = 0.6931471805;
    exp * ln2 + (mantissa - 1.0) * (1.0 - 0.5 * (mantissa - 1.0))
}

// ============================================================================
// PREDICTION TYPES
// ============================================================================

/// Source of the prediction being validated
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PredictionSource {
    Horizon,
    Timeline,
    MonteCarlo,
    Simulator,
    Proactive,
    Rehearsal,
    Ensemble,
}

/// A single prediction record
#[derive(Debug, Clone)]
pub struct PredictionRecord {
    pub prediction_id: u64,
    pub source: PredictionSource,
    pub predicted_confidence: f32,
    pub predicted_outcome: u32,
    pub actual_outcome: Option<u32>,
    pub correct: Option<bool>,
    pub tick: u64,
    pub validated: bool,
}

/// A point on the calibration curve
#[derive(Debug, Clone, Copy)]
pub struct CalibrationPoint {
    pub bucket_center: f32,
    pub predicted_probability: f32,
    pub actual_frequency: f32,
    pub sample_count: u64,
    pub calibration_error: f32,
}

/// A point on the reliability diagram
#[derive(Debug, Clone, Copy)]
pub struct ReliabilityPoint {
    pub predicted_bin: f32,
    pub observed_frequency: f32,
    pub bin_count: u64,
    pub gap: f32,
}

// ============================================================================
// CALIBRATION BUCKET
// ============================================================================

/// Internal calibration bucket for tracking confidence vs outcomes
#[derive(Debug, Clone)]
struct CalibrationBucket {
    sum_predicted: f32,
    sum_actual: f32,
    count: u64,
    hits: u64,
}

impl CalibrationBucket {
    fn new() -> Self {
        Self {
            sum_predicted: 0.0,
            sum_actual: 0.0,
            count: 0,
            hits: 0,
        }
    }

    fn record(&mut self, predicted: f32, actual: bool) {
        self.count += 1;
        self.sum_predicted += predicted;
        if actual {
            self.hits += 1;
            self.sum_actual += 1.0;
        }
    }

    fn avg_predicted(&self) -> f32 {
        if self.count == 0 {
            0.0
        } else {
            self.sum_predicted / self.count as f32
        }
    }

    fn actual_frequency(&self) -> f32 {
        if self.count == 0 {
            0.0
        } else {
            self.hits as f32 / self.count as f32
        }
    }

    fn calibration_error(&self) -> f32 {
        (self.avg_predicted() - self.actual_frequency()).abs()
    }
}

// ============================================================================
// PER-SOURCE TRACKER
// ============================================================================

/// Per-source validation statistics
#[derive(Debug, Clone)]
struct SourceTracker {
    total: u64,
    correct: u64,
    brier_sum: f32,
    log_loss_sum: f32,
    accuracy_ema: f32,
    sharpness_sum: f32,
}

impl SourceTracker {
    fn new() -> Self {
        Self {
            total: 0,
            correct: 0,
            brier_sum: 0.0,
            log_loss_sum: 0.0,
            accuracy_ema: 0.5,
            sharpness_sum: 0.0,
        }
    }

    fn record(&mut self, predicted_conf: f32, correct: bool) {
        self.total += 1;
        if correct {
            self.correct += 1;
        }
        let outcome = if correct { 1.0 } else { 0.0 };

        // Brier score: (predicted - outcome)^2
        let brier = (predicted_conf - outcome) * (predicted_conf - outcome);
        self.brier_sum += brier;

        // Log loss: -(outcome * ln(p) + (1-outcome) * ln(1-p))
        let p = predicted_conf
            .max(LOG_LOSS_EPSILON)
            .min(1.0 - LOG_LOSS_EPSILON);
        let ll = -(outcome * ln_approx(p) + (1.0 - outcome) * ln_approx(1.0 - p));
        self.log_loss_sum += ll;

        // Sharpness: distance from 0.5 (more decisive = sharper)
        self.sharpness_sum += (predicted_conf - 0.5).abs();

        let acc_val = if correct { 1.0 } else { 0.0 };
        self.accuracy_ema = EMA_ALPHA * acc_val + (1.0 - EMA_ALPHA) * self.accuracy_ema;
    }

    fn brier_score(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            self.brier_sum / self.total as f32
        }
    }

    fn log_loss(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            self.log_loss_sum / self.total as f32
        }
    }

    fn sharpness(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            self.sharpness_sum / self.total as f32
        }
    }
}

// ============================================================================
// VALIDATION STATS
// ============================================================================

/// Aggregate validation statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidationStats {
    pub total_predictions: u64,
    pub validated_count: u64,
    pub overall_accuracy: f32,
    pub brier_score: f32,
    pub log_loss: f32,
    pub sharpness: f32,
    pub calibration_error: f32,
    pub overconfidence_rate: f32,
}

// ============================================================================
// BRIDGE PREDICTION VALIDATOR
// ============================================================================

/// Validates bridge predictions against actual outcomes. Tracks Brier score,
/// log-loss, calibration curves, sharpness, and reliability diagrams.
#[derive(Debug)]
pub struct BridgePredictionValidator {
    predictions: Vec<PredictionRecord>,
    write_idx: usize,
    calibration_buckets: [CalibrationBucket; CALIBRATION_BUCKETS],
    source_trackers: BTreeMap<u8, SourceTracker>,
    tick: u64,
    total_predictions: u64,
    validated_count: u64,
    global_brier_ema: f32,
    global_accuracy_ema: f32,
    global_sharpness_ema: f32,
}

impl BridgePredictionValidator {
    pub fn new() -> Self {
        Self {
            predictions: Vec::new(),
            write_idx: 0,
            calibration_buckets: [
                CalibrationBucket::new(),
                CalibrationBucket::new(),
                CalibrationBucket::new(),
                CalibrationBucket::new(),
                CalibrationBucket::new(),
                CalibrationBucket::new(),
                CalibrationBucket::new(),
                CalibrationBucket::new(),
                CalibrationBucket::new(),
                CalibrationBucket::new(),
            ],
            source_trackers: BTreeMap::new(),
            tick: 0,
            total_predictions: 0,
            validated_count: 0,
            global_brier_ema: 0.25,
            global_accuracy_ema: 0.5,
            global_sharpness_ema: 0.0,
        }
    }

    /// Register a new prediction for later validation
    pub fn register_prediction(
        &mut self,
        source: PredictionSource,
        predicted_confidence: f32,
        predicted_outcome: u32,
    ) -> u64 {
        self.tick += 1;
        self.total_predictions += 1;

        let id = fnv1a_hash(&self.total_predictions.to_le_bytes())
            ^ fnv1a_hash(&(source as u8).to_le_bytes());

        let record = PredictionRecord {
            prediction_id: id,
            source,
            predicted_confidence: predicted_confidence.max(0.0).min(1.0),
            predicted_outcome,
            actual_outcome: None,
            correct: None,
            tick: self.tick,
            validated: false,
        };

        if self.predictions.len() < MAX_PREDICTIONS {
            self.predictions.push(record);
        } else {
            self.predictions[self.write_idx] = record;
        }
        self.write_idx = (self.write_idx + 1) % MAX_PREDICTIONS;
        id
    }

    /// Validate a prediction against the actual outcome
    pub fn validate_prediction(&mut self, prediction_id: u64, actual_outcome: u32) -> Option<f32> {
        let pred = self
            .predictions
            .iter_mut()
            .find(|p| p.prediction_id == prediction_id && !p.validated)?;

        pred.actual_outcome = Some(actual_outcome);
        let correct = pred.predicted_outcome == actual_outcome;
        pred.correct = Some(correct);
        pred.validated = true;
        self.validated_count += 1;

        let conf = pred.predicted_confidence;
        let source_key = pred.source as u8;

        // Update calibration bucket
        let bucket_idx =
            ((conf * CALIBRATION_BUCKETS as f32) as usize).min(CALIBRATION_BUCKETS - 1);
        self.calibration_buckets[bucket_idx].record(conf, correct);

        // Update source tracker
        let tracker = self
            .source_trackers
            .entry(source_key)
            .or_insert_with(SourceTracker::new);
        tracker.record(conf, correct);

        // Update global EMAs
        let brier = (conf - if correct { 1.0 } else { 0.0 }).powi(2);
        self.global_brier_ema = EMA_ALPHA * brier + (1.0 - EMA_ALPHA) * self.global_brier_ema;

        let acc = if correct { 1.0 } else { 0.0 };
        self.global_accuracy_ema = EMA_ALPHA * acc + (1.0 - EMA_ALPHA) * self.global_accuracy_ema;

        let sharp = (conf - 0.5).abs();
        self.global_sharpness_ema =
            EMA_ALPHA * sharp + (1.0 - EMA_ALPHA) * self.global_sharpness_ema;

        Some(brier)
    }

    /// Compute the Brier score (lower is better, 0 = perfect)
    pub fn brier_score(&self) -> f32 {
        self.global_brier_ema
    }

    /// Compute per-source Brier score
    pub fn brier_score_by_source(&self, source: PredictionSource) -> f32 {
        self.source_trackers
            .get(&(source as u8))
            .map(|t| t.brier_score())
            .unwrap_or(0.25)
    }

    /// Generate the calibration curve: predicted probability vs actual frequency
    pub fn calibration_curve(&self) -> Vec<CalibrationPoint> {
        let mut points = Vec::new();
        for (i, bucket) in self.calibration_buckets.iter().enumerate() {
            if bucket.count < MIN_SAMPLES_FOR_CALIBRATION {
                continue;
            }
            let center = (i as f32 + 0.5) / CALIBRATION_BUCKETS as f32;
            points.push(CalibrationPoint {
                bucket_center: center,
                predicted_probability: bucket.avg_predicted(),
                actual_frequency: bucket.actual_frequency(),
                sample_count: bucket.count,
                calibration_error: bucket.calibration_error(),
            });
        }
        points
    }

    /// Compute sharpness: average distance from 0.5 (higher = more decisive)
    pub fn sharpness(&self) -> f32 {
        self.global_sharpness_ema
    }

    /// Generate reliability diagram data
    pub fn reliability_diagram(&self) -> Vec<ReliabilityPoint> {
        let mut points = Vec::new();
        for (i, bucket) in self.calibration_buckets.iter().enumerate() {
            if bucket.count == 0 {
                continue;
            }
            let bin_center = (i as f32 + 0.5) / CALIBRATION_BUCKETS as f32;
            let observed = bucket.actual_frequency();
            points.push(ReliabilityPoint {
                predicted_bin: bin_center,
                observed_frequency: observed,
                bin_count: bucket.count,
                gap: bucket.avg_predicted() - observed,
            });
        }
        points
    }

    /// Overall expected calibration error (weighted by bucket count)
    pub fn expected_calibration_error(&self) -> f32 {
        let total: u64 = self.calibration_buckets.iter().map(|b| b.count).sum();
        if total == 0 {
            return 0.0;
        }
        let weighted_error: f32 = self
            .calibration_buckets
            .iter()
            .filter(|b| b.count >= MIN_SAMPLES_FOR_CALIBRATION)
            .map(|b| b.calibration_error() * b.count as f32)
            .sum();
        weighted_error / total as f32
    }

    /// Log loss by source
    pub fn log_loss_by_source(&self, source: PredictionSource) -> f32 {
        self.source_trackers
            .get(&(source as u8))
            .map(|t| t.log_loss())
            .unwrap_or(0.693) // ln(2) ≈ random guessing
    }

    /// Aggregate validation statistics
    pub fn stats(&self) -> ValidationStats {
        let overall_acc = if self.validated_count == 0 {
            0.0
        } else {
            self.global_accuracy_ema
        };

        let overconf_count = self
            .calibration_buckets
            .iter()
            .filter(|b| {
                b.count >= MIN_SAMPLES_FOR_CALIBRATION
                    && b.avg_predicted() > b.actual_frequency() + 0.05
            })
            .count();
        let total_populated = self
            .calibration_buckets
            .iter()
            .filter(|b| b.count >= MIN_SAMPLES_FOR_CALIBRATION)
            .count()
            .max(1);

        ValidationStats {
            total_predictions: self.total_predictions,
            validated_count: self.validated_count,
            overall_accuracy: overall_acc,
            brier_score: self.global_brier_ema,
            log_loss: self
                .source_trackers
                .values()
                .map(|t| t.log_loss())
                .sum::<f32>()
                / self.source_trackers.len().max(1) as f32,
            sharpness: self.global_sharpness_ema,
            calibration_error: self.expected_calibration_error(),
            overconfidence_rate: overconf_count as f32 / total_populated as f32,
        }
    }
}
