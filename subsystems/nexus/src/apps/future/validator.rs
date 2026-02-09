// SPDX-License-Identifier: GPL-2.0
//! # Apps Prediction Validator
//!
//! Validates application behavior predictions against ground truth.
//! Computes standard forecasting metrics: MAPE (mean absolute percentage
//! error), directional accuracy (did we get the trend right?), and
//! prediction bias (do we systematically over- or under-estimate?).
//! When accuracy degrades, the validator triggers recalibration by
//! adjusting confidence scaling factors and trend weights.
//!
//! This is the kernel grading its own homework.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_VALIDATION_RECORDS: usize = 1024;
const MAX_RECALIBRATIONS: usize = 64;
const EMA_ALPHA: f32 = 0.10;
const MAPE_ALERT_THRESHOLD: f32 = 0.30;
const BIAS_ALERT_THRESHOLD: f32 = 0.15;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

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

// ============================================================================
// VALIDATION TYPES
// ============================================================================

/// Category of prediction being validated
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PredictionCategory {
    ResourceForecast,
    PhaseTransition,
    DemandSpike,
    LifetimeEstimate,
    ClassificationChange,
    InteractionEffect,
}

/// A single validation record
#[derive(Debug, Clone)]
pub struct ValidationRecord {
    pub id: u64,
    pub category: PredictionCategory,
    pub process_id: u64,
    pub tick: u64,
    pub predicted: f32,
    pub actual: f32,
    pub absolute_error: f32,
    pub percentage_error: f32,
    pub direction_correct: bool,
    pub prior_trend: f32,
}

/// Forecast validation result
#[derive(Debug, Clone)]
pub struct ForecastValidation {
    pub record_id: u64,
    pub mape: f32,
    pub direction_correct: bool,
    pub bias: f32,
    pub needs_recalibration: bool,
    pub description: String,
}

/// MAPE score breakdown
#[derive(Debug, Clone)]
pub struct MapeBreakdown {
    pub overall_mape: f32,
    pub per_category: Vec<(PredictionCategory, f32, u64)>,
    pub trend: f32,
    pub worst_category: PredictionCategory,
    pub best_category: PredictionCategory,
}

/// Directional accuracy report
#[derive(Debug, Clone)]
pub struct DirectionalAccuracy {
    pub overall_accuracy: f32,
    pub per_category: Vec<(PredictionCategory, f32, u64)>,
    pub streak_correct: u32,
    pub streak_incorrect: u32,
}

/// Prediction bias analysis
#[derive(Debug, Clone)]
pub struct BiasAnalysis {
    pub overall_bias: f32,
    pub per_category: Vec<(PredictionCategory, f32)>,
    pub is_overestimating: bool,
    pub magnitude: f32,
    pub correction_factor: f32,
}

/// Recalibration action
#[derive(Debug, Clone)]
pub struct RecalibrationAction {
    pub category: PredictionCategory,
    pub old_confidence_scale: f32,
    pub new_confidence_scale: f32,
    pub old_trend_weight: f32,
    pub new_trend_weight: f32,
    pub reason: String,
}

// ============================================================================
// PER-CATEGORY TRACKER
// ============================================================================

/// Tracks validation statistics per prediction category
#[derive(Debug, Clone)]
struct CategoryValidator {
    mape_ema: f32,
    bias_ema: f32,
    directional_ema: f32,
    record_count: u64,
    confidence_scale: f32,
    trend_weight: f32,
    recalibration_count: u32,
}

impl CategoryValidator {
    fn new() -> Self {
        Self {
            mape_ema: 0.2,
            bias_ema: 0.0,
            directional_ema: 0.5,
            record_count: 0,
            confidence_scale: 1.0,
            trend_weight: 1.0,
            recalibration_count: 0,
        }
    }

    #[inline]
    fn record(&mut self, pct_error: f32, bias: f32, direction_correct: bool) {
        self.record_count += 1;
        self.mape_ema = EMA_ALPHA * pct_error.abs() + (1.0 - EMA_ALPHA) * self.mape_ema;
        self.bias_ema = EMA_ALPHA * bias + (1.0 - EMA_ALPHA) * self.bias_ema;
        let dir_val = if direction_correct { 1.0 } else { 0.0 };
        self.directional_ema = EMA_ALPHA * dir_val + (1.0 - EMA_ALPHA) * self.directional_ema;
    }

    fn needs_recalibration(&self) -> bool {
        self.mape_ema > MAPE_ALERT_THRESHOLD || self.bias_ema.abs() > BIAS_ALERT_THRESHOLD
    }

    fn recalibrate(&mut self) -> (f32, f32, f32, f32) {
        let old_conf = self.confidence_scale;
        let old_trend = self.trend_weight;

        if self.mape_ema > MAPE_ALERT_THRESHOLD {
            self.confidence_scale *= 0.9;
        }
        if self.bias_ema > BIAS_ALERT_THRESHOLD {
            self.trend_weight *= 0.95;
        } else if self.bias_ema < -BIAS_ALERT_THRESHOLD {
            self.trend_weight *= 1.05;
        }
        self.confidence_scale = self.confidence_scale.clamp(0.3, 1.5);
        self.trend_weight = self.trend_weight.clamp(0.3, 2.0);
        self.recalibration_count += 1;

        (old_conf, self.confidence_scale, old_trend, self.trend_weight)
    }
}

// ============================================================================
// VALIDATOR STATS
// ============================================================================

/// Aggregate validation statistics
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct ValidatorStats {
    pub total_validations: u64,
    pub overall_mape: f32,
    pub overall_directional: f32,
    pub overall_bias: f32,
    pub recalibrations_performed: u64,
    pub categories_tracked: usize,
    pub alerts_active: u32,
}

// ============================================================================
// APPS PREDICTION VALIDATOR
// ============================================================================

/// Validates application predictions against actual behavior. Computes
/// MAPE, directional accuracy, and bias, triggering recalibration
/// when accuracy degrades.
#[derive(Debug)]
pub struct AppsPredictionValidator {
    records: Vec<ValidationRecord>,
    write_idx: usize,
    category_validators: BTreeMap<u8, CategoryValidator>,
    total_validations: u64,
    recalibrations: u64,
    tick: u64,
    rng_state: u64,
    overall_mape_ema: f32,
    overall_bias_ema: f32,
    overall_directional_ema: f32,
    streak_correct: u32,
    streak_incorrect: u32,
}

impl AppsPredictionValidator {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            write_idx: 0,
            category_validators: BTreeMap::new(),
            total_validations: 0,
            recalibrations: 0,
            tick: 0,
            rng_state: 0xDEAD_C0DE_FACE_B00C,
            overall_mape_ema: 0.2,
            overall_bias_ema: 0.0,
            overall_directional_ema: 0.5,
            streak_correct: 0,
            streak_incorrect: 0,
        }
    }

    /// Validate a forecast against actual value
    pub fn validate_forecast(
        &mut self,
        category: PredictionCategory,
        process_id: u64,
        predicted: f32,
        actual: f32,
        prior_trend: f32,
        tick: u64,
    ) -> ForecastValidation {
        self.tick = tick;
        self.total_validations += 1;

        let abs_error = (predicted - actual).abs();
        let pct_error = if actual.abs() > 0.001 {
            abs_error / actual.abs()
        } else {
            abs_error
        };
        let bias = predicted - actual;
        let direction_correct = (prior_trend >= 0.0 && actual >= predicted * 0.8)
            || (prior_trend < 0.0 && actual <= predicted * 1.2);

        if direction_correct {
            self.streak_correct += 1;
            self.streak_incorrect = 0;
        } else {
            self.streak_incorrect += 1;
            self.streak_correct = 0;
        }

        let id_bytes = [
            &process_id.to_le_bytes()[..],
            &tick.to_le_bytes()[..],
        ]
        .concat();
        let record_id = fnv1a_hash(&id_bytes);

        let record = ValidationRecord {
            id: record_id,
            category,
            process_id,
            tick,
            predicted,
            actual,
            absolute_error: abs_error,
            percentage_error: pct_error,
            direction_correct,
            prior_trend,
        };

        if self.records.len() < MAX_VALIDATION_RECORDS {
            self.records.push(record);
        } else {
            let idx = self.write_idx % MAX_VALIDATION_RECORDS;
            self.records[idx] = record;
            self.write_idx += 1;
        }

        let cat_key = category as u8;
        let validator = self.category_validators
            .entry(cat_key)
            .or_insert_with(CategoryValidator::new);
        validator.record(pct_error, bias, direction_correct);
        let needs_recal = validator.needs_recalibration();

        self.overall_mape_ema =
            EMA_ALPHA * pct_error + (1.0 - EMA_ALPHA) * self.overall_mape_ema;
        self.overall_bias_ema =
            EMA_ALPHA * bias + (1.0 - EMA_ALPHA) * self.overall_bias_ema;
        let dir_val = if direction_correct { 1.0 } else { 0.0 };
        self.overall_directional_ema =
            EMA_ALPHA * dir_val + (1.0 - EMA_ALPHA) * self.overall_directional_ema;

        let mut desc = String::new();
        if needs_recal {
            desc.push_str("recalibration_needed");
        } else if pct_error < 0.1 {
            desc.push_str("accurate");
        } else if pct_error < 0.3 {
            desc.push_str("acceptable");
        } else {
            desc.push_str("poor_accuracy");
        }

        ForecastValidation {
            record_id,
            mape: pct_error,
            direction_correct,
            bias,
            needs_recalibration: needs_recal,
            description: desc,
        }
    }

    /// Compute the MAPE score breakdown by category
    pub fn mape_score(&self) -> MapeBreakdown {
        let mut per_category = Vec::new();
        let mut worst_mape: f32 = 0.0;
        let mut best_mape: f32 = f32::MAX;
        let mut worst_cat = PredictionCategory::ResourceForecast;
        let mut best_cat = PredictionCategory::ResourceForecast;

        let categories = [
            PredictionCategory::ResourceForecast,
            PredictionCategory::PhaseTransition,
            PredictionCategory::DemandSpike,
            PredictionCategory::LifetimeEstimate,
            PredictionCategory::ClassificationChange,
            PredictionCategory::InteractionEffect,
        ];

        for &cat in &categories {
            let key = cat as u8;
            if let Some(v) = self.category_validators.get(&key) {
                per_category.push((cat, v.mape_ema, v.record_count));
                if v.mape_ema > worst_mape {
                    worst_mape = v.mape_ema;
                    worst_cat = cat;
                }
                if v.mape_ema < best_mape {
                    best_mape = v.mape_ema;
                    best_cat = cat;
                }
            }
        }

        MapeBreakdown {
            overall_mape: self.overall_mape_ema,
            per_category,
            trend: self.overall_bias_ema,
            worst_category: worst_cat,
            best_category: best_cat,
        }
    }

    /// Compute directional accuracy across categories
    pub fn directional_accuracy(&self) -> DirectionalAccuracy {
        let mut per_category = Vec::new();
        let categories = [
            PredictionCategory::ResourceForecast,
            PredictionCategory::PhaseTransition,
            PredictionCategory::DemandSpike,
            PredictionCategory::LifetimeEstimate,
            PredictionCategory::ClassificationChange,
            PredictionCategory::InteractionEffect,
        ];

        for &cat in &categories {
            let key = cat as u8;
            if let Some(v) = self.category_validators.get(&key) {
                per_category.push((cat, v.directional_ema, v.record_count));
            }
        }

        DirectionalAccuracy {
            overall_accuracy: self.overall_directional_ema,
            per_category,
            streak_correct: self.streak_correct,
            streak_incorrect: self.streak_incorrect,
        }
    }

    /// Analyze prediction bias
    pub fn prediction_bias(&self) -> BiasAnalysis {
        let mut per_category = Vec::new();
        let categories = [
            PredictionCategory::ResourceForecast,
            PredictionCategory::PhaseTransition,
            PredictionCategory::DemandSpike,
            PredictionCategory::LifetimeEstimate,
            PredictionCategory::ClassificationChange,
            PredictionCategory::InteractionEffect,
        ];

        for &cat in &categories {
            let key = cat as u8;
            if let Some(v) = self.category_validators.get(&key) {
                per_category.push((cat, v.bias_ema));
            }
        }

        let is_over = self.overall_bias_ema > 0.0;
        let correction = if is_over {
            1.0 - self.overall_bias_ema.abs().min(0.5)
        } else {
            1.0 + self.overall_bias_ema.abs().min(0.5)
        };

        BiasAnalysis {
            overall_bias: self.overall_bias_ema,
            per_category,
            is_overestimating: is_over,
            magnitude: self.overall_bias_ema.abs(),
            correction_factor: correction,
        }
    }

    /// Trigger recalibration for categories that need it
    pub fn recalibrate(&mut self) -> Vec<RecalibrationAction> {
        let mut actions = Vec::new();
        let keys: Vec<u8> = self.category_validators.keys().copied().collect();

        for key in keys {
            if let Some(v) = self.category_validators.get_mut(&key) {
                if v.needs_recalibration()
                    && (v.recalibration_count as usize) < MAX_RECALIBRATIONS
                {
                    let (old_conf, new_conf, old_trend, new_trend) = v.recalibrate();
                    self.recalibrations += 1;

                    let cat = match key {
                        0 => PredictionCategory::ResourceForecast,
                        1 => PredictionCategory::PhaseTransition,
                        2 => PredictionCategory::DemandSpike,
                        3 => PredictionCategory::LifetimeEstimate,
                        4 => PredictionCategory::ClassificationChange,
                        _ => PredictionCategory::InteractionEffect,
                    };

                    let mut reason = String::new();
                    if v.mape_ema > MAPE_ALERT_THRESHOLD {
                        reason.push_str("high_mape");
                    }
                    if v.bias_ema.abs() > BIAS_ALERT_THRESHOLD {
                        if !reason.is_empty() {
                            reason.push('+');
                        }
                        reason.push_str("bias");
                    }

                    actions.push(RecalibrationAction {
                        category: cat,
                        old_confidence_scale: old_conf,
                        new_confidence_scale: new_conf,
                        old_trend_weight: old_trend,
                        new_trend_weight: new_trend,
                        reason,
                    });
                }
            }
        }
        actions
    }

    /// Get aggregate statistics
    pub fn stats(&self) -> ValidatorStats {
        let mut alerts: u32 = 0;
        for v in self.category_validators.values() {
            if v.needs_recalibration() {
                alerts += 1;
            }
        }
        ValidatorStats {
            total_validations: self.total_validations,
            overall_mape: self.overall_mape_ema,
            overall_directional: self.overall_directional_ema,
            overall_bias: self.overall_bias_ema,
            recalibrations_performed: self.recalibrations,
            categories_tracked: self.category_validators.len(),
            alerts_active: alerts,
        }
    }
}
