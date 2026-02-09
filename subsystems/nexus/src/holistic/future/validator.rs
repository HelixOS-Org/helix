// SPDX-License-Identifier: GPL-2.0
//! # Holistic Prediction Validator
//!
//! System-wide prediction validation engine. Validates ALL predictions from
//! ALL subsystems — bridge, application, cooperative, memory, scheduler,
//! network — against actual outcomes. Computes global prediction scores,
//! per-subsystem accuracy, systematic error analysis, prediction decomposition,
//! model selection signals, and recalibration signals.
//!
//! This is the prediction framework's quality controller: it ensures that
//! the kernel's foresight continuously improves.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_VALIDATION_ENTRIES: usize = 512;
const MAX_SUBSYSTEM_ENTRIES: usize = 16;
const MAX_ERROR_RECORDS: usize = 256;
const MAX_MODEL_ENTRIES: usize = 32;
const MAX_RECAL_SIGNALS: usize = 128;
const EMA_ALPHA: f32 = 0.10;
const ACCURACY_THRESHOLD: f32 = 0.70;
const SYSTEMATIC_ERROR_THRESHOLD: f32 = 0.10;
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
// SUBSYSTEM IDENTITY
// ============================================================================

/// Identity of a prediction-producing subsystem
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SubsystemId {
    Bridge,
    Application,
    Cooperative,
    Memory,
    Scheduler,
    Network,
    IO,
    Thermal,
    Holistic,
}

/// Error type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorType {
    Bias,
    Variance,
    Lag,
    Overshoot,
    Undershoot,
    Noise,
}

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// A single validation entry: prediction vs actual
#[derive(Debug, Clone)]
pub struct ValidationEntry {
    pub id: u64,
    pub subsystem: SubsystemId,
    pub dimension: String,
    pub predicted: f32,
    pub actual: f32,
    pub error: f32,
    pub absolute_error: f32,
    pub accuracy: f32,
    pub tick: u64,
}

/// Per-subsystem accuracy report
#[derive(Debug, Clone)]
pub struct SubsystemAccuracy {
    pub subsystem: SubsystemId,
    pub mean_accuracy: f32,
    pub mean_error: f32,
    pub error_std_dev: f32,
    pub sample_count: u64,
    pub trend: f32,
    pub rank: u32,
}

/// Systematic error detected in predictions
#[derive(Debug, Clone)]
pub struct SystematicError {
    pub id: u64,
    pub subsystem: SubsystemId,
    pub error_type: ErrorType,
    pub magnitude: f32,
    pub persistence: f32,
    pub description: String,
    pub correction_applied: bool,
}

/// Decomposition of prediction error into components
#[derive(Debug, Clone)]
pub struct PredictionDecomposition {
    pub subsystem: SubsystemId,
    pub bias_component: f32,
    pub variance_component: f32,
    pub irreducible_noise: f32,
    pub total_mse: f32,
    pub bias_fraction: f32,
    pub variance_fraction: f32,
}

/// Model selection signal — which model is best for which domain
#[derive(Debug, Clone)]
pub struct ModelSelectionSignal {
    pub id: u64,
    pub dimension: String,
    pub best_subsystem: SubsystemId,
    pub best_accuracy: f32,
    pub second_best: SubsystemId,
    pub second_accuracy: f32,
    pub margin: f32,
}

/// Recalibration signal sent to a subsystem
#[derive(Debug, Clone)]
pub struct RecalibrationSignal {
    pub id: u64,
    pub target: SubsystemId,
    pub dimension: String,
    pub current_bias: f32,
    pub suggested_correction: f32,
    pub confidence: f32,
    pub urgency: f32,
}

/// Global validation summary
#[derive(Debug, Clone)]
pub struct GlobalValidation {
    pub global_accuracy: f32,
    pub global_bias: f32,
    pub global_variance: f32,
    pub subsystem_count: usize,
    pub best_subsystem: SubsystemId,
    pub worst_subsystem: SubsystemId,
    pub total_validations: u64,
    pub systematic_error_count: usize,
}

// ============================================================================
// STATS
// ============================================================================

/// Aggregate validation statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatorStats {
    pub total_validations: u64,
    pub global_accuracy_ema: f32,
    pub global_bias_ema: f32,
    pub systematic_errors_found: u64,
    pub recalibrations_sent: u64,
    pub model_selections: u64,
    pub best_accuracy: f32,
    pub worst_accuracy: f32,
}

// ============================================================================
// HOLISTIC PREDICTION VALIDATOR
// ============================================================================

/// System-wide prediction validation engine. Validates predictions from
/// all subsystems, computes accuracy, detects systematic errors, and
/// sends recalibration signals.
#[derive(Debug)]
pub struct HolisticPredictionValidator {
    validations: BTreeMap<u64, ValidationEntry>,
    subsystem_stats: BTreeMap<u8, Vec<ValidationEntry>>,
    systematic_errors: BTreeMap<u64, SystematicError>,
    model_signals: BTreeMap<u64, ModelSelectionSignal>,
    recalibrations: BTreeMap<u64, RecalibrationSignal>,
    total_validations: u64,
    systematic_count: u64,
    recal_count: u64,
    model_count: u64,
    tick: u64,
    rng_state: u64,
    accuracy_ema: f32,
    bias_ema: f32,
}

impl HolisticPredictionValidator {
    pub fn new() -> Self {
        Self {
            validations: BTreeMap::new(),
            subsystem_stats: BTreeMap::new(),
            systematic_errors: BTreeMap::new(),
            model_signals: BTreeMap::new(),
            recalibrations: BTreeMap::new(),
            total_validations: 0,
            systematic_count: 0,
            recal_count: 0,
            model_count: 0,
            tick: 0,
            rng_state: 0xFA11_DA70_AE06_1BE0,
            accuracy_ema: 0.5,
            bias_ema: 0.0,
        }
    }

    /// Record a prediction and its actual outcome for validation
    pub fn record_validation(
        &mut self,
        subsystem: SubsystemId,
        dimension: String,
        predicted: f32,
        actual: f32,
    ) -> ValidationEntry {
        self.tick += 1;
        self.total_validations += 1;

        let error = predicted - actual;
        let abs_error = error.abs();
        let scale = actual.abs().max(1.0);
        let accuracy = (1.0 - abs_error / scale).clamp(0.0, 1.0);

        let id = fnv1a_hash(format!("{:?}-{}-{}", subsystem, dimension, self.tick).as_bytes())
            ^ xorshift64(&mut self.rng_state);

        self.accuracy_ema = EMA_ALPHA * accuracy + (1.0 - EMA_ALPHA) * self.accuracy_ema;
        self.bias_ema = EMA_ALPHA * error + (1.0 - EMA_ALPHA) * self.bias_ema;

        let entry = ValidationEntry {
            id,
            subsystem,
            dimension,
            predicted,
            actual,
            error,
            absolute_error: abs_error,
            accuracy,
            tick: self.tick,
        };

        self.validations.insert(id, entry.clone());
        if self.validations.len() > MAX_VALIDATION_ENTRIES {
            if let Some((&oldest, _)) = self.validations.iter().next() {
                self.validations.remove(&oldest);
            }
        }

        let sub_key = subsystem as u8;
        let sub_entries = self.subsystem_stats.entry(sub_key).or_insert_with(Vec::new);
        sub_entries.push(entry.clone());
        if sub_entries.len() > MAX_VALIDATION_ENTRIES / MAX_SUBSYSTEM_ENTRIES {
            sub_entries.remove(0);
        }

        entry
    }

    /// Compute global validation across all subsystems
    pub fn global_validation(&self) -> GlobalValidation {
        let mut best_acc = 0.0_f32;
        let mut worst_acc = 1.0_f32;
        let mut best_sub = SubsystemId::Holistic;
        let mut worst_sub = SubsystemId::Holistic;
        let mut total_bias = 0.0_f32;
        let mut total_variance = 0.0_f32;
        let mut sub_count = 0_usize;

        for (&sub_key, entries) in &self.subsystem_stats {
            if entries.is_empty() {
                continue;
            }
            sub_count += 1;

            let n = entries.len() as f32;
            let acc = entries.iter().map(|e| e.accuracy).sum::<f32>() / n;
            let mean_err = entries.iter().map(|e| e.error).sum::<f32>() / n;
            let var = entries
                .iter()
                .map(|e| (e.error - mean_err).powi(2))
                .sum::<f32>()
                / n;

            total_bias += mean_err.abs();
            total_variance += var;

            let sub = Self::subsystem_from_key(sub_key);
            if acc > best_acc {
                best_acc = acc;
                best_sub = sub;
            }
            if acc < worst_acc {
                worst_acc = acc;
                worst_sub = sub;
            }
        }

        let sc = sub_count.max(1) as f32;

        GlobalValidation {
            global_accuracy: self.accuracy_ema,
            global_bias: total_bias / sc,
            global_variance: total_variance / sc,
            subsystem_count: sub_count,
            best_subsystem: best_sub,
            worst_subsystem: worst_sub,
            total_validations: self.total_validations,
            systematic_error_count: self.systematic_errors.len(),
        }
    }

    /// Compute per-subsystem accuracy
    pub fn subsystem_accuracy(&self, subsystem: SubsystemId) -> SubsystemAccuracy {
        let sub_key = subsystem as u8;
        let entries = self.subsystem_stats.get(&sub_key);

        match entries {
            Some(entries) if !entries.is_empty() => {
                let n = entries.len() as f32;
                let mean_acc = entries.iter().map(|e| e.accuracy).sum::<f32>() / n;
                let mean_err = entries.iter().map(|e| e.error).sum::<f32>() / n;
                let var = entries
                    .iter()
                    .map(|e| (e.error - mean_err).powi(2))
                    .sum::<f32>()
                    / n;

                let trend = if entries.len() >= 4 {
                    let half = entries.len() / 2;
                    let first: f32 =
                        entries[..half].iter().map(|e| e.accuracy).sum::<f32>() / half as f32;
                    let second: f32 = entries[half..].iter().map(|e| e.accuracy).sum::<f32>()
                        / (entries.len() - half) as f32;
                    second - first
                } else {
                    0.0
                };

                SubsystemAccuracy {
                    subsystem,
                    mean_accuracy: mean_acc,
                    mean_error: mean_err,
                    error_std_dev: var.sqrt(),
                    sample_count: entries.len() as u64,
                    trend,
                    rank: 0,
                }
            },
            _ => SubsystemAccuracy {
                subsystem,
                mean_accuracy: 0.0,
                mean_error: 0.0,
                error_std_dev: 0.0,
                sample_count: 0,
                trend: 0.0,
                rank: 0,
            },
        }
    }

    /// Detect systematic errors in a subsystem's predictions
    pub fn systematic_error(&mut self, subsystem: SubsystemId) -> Vec<SystematicError> {
        let sub_key = subsystem as u8;
        let entries = self
            .subsystem_stats
            .get(&sub_key)
            .cloned()
            .unwrap_or_default();
        let mut errors = Vec::new();

        if entries.len() < 5 {
            return errors;
        }

        let n = entries.len() as f32;
        let mean_err = entries.iter().map(|e| e.error).sum::<f32>() / n;

        // Bias detection
        if mean_err.abs() > SYSTEMATIC_ERROR_THRESHOLD {
            let error_type = if mean_err > 0.0 {
                ErrorType::Overshoot
            } else {
                ErrorType::Undershoot
            };
            let id = fnv1a_hash(format!("syserr-{:?}-bias", subsystem).as_bytes());
            errors.push(SystematicError {
                id,
                subsystem,
                error_type,
                magnitude: mean_err.abs(),
                persistence: (entries.len() as f32 / MAX_VALIDATION_ENTRIES as f32).clamp(0.0, 1.0),
                description: String::from("persistent directional bias"),
                correction_applied: false,
            });
            self.systematic_errors
                .insert(id, errors.last().unwrap().clone());
            self.systematic_count += 1;
        }

        // Variance detection (high noise)
        let variance = entries
            .iter()
            .map(|e| (e.error - mean_err).powi(2))
            .sum::<f32>()
            / n;
        if variance.sqrt() > SYSTEMATIC_ERROR_THRESHOLD * 2.0 {
            let id = fnv1a_hash(format!("syserr-{:?}-variance", subsystem).as_bytes());
            errors.push(SystematicError {
                id,
                subsystem,
                error_type: ErrorType::Variance,
                magnitude: variance.sqrt(),
                persistence: 0.5,
                description: String::from("high prediction variance / noise"),
                correction_applied: false,
            });
            self.systematic_errors
                .insert(id, errors.last().unwrap().clone());
            self.systematic_count += 1;
        }

        // Lag detection (errors correlate with time)
        if entries.len() >= 10 {
            let recent = &entries[entries.len() - 5..];
            let older = &entries[entries.len() - 10..entries.len() - 5];
            let recent_err: f32 = recent.iter().map(|e| e.error).sum::<f32>() / 5.0;
            let older_err: f32 = older.iter().map(|e| e.error).sum::<f32>() / 5.0;

            if (recent_err - older_err).abs() > SYSTEMATIC_ERROR_THRESHOLD {
                let id = fnv1a_hash(format!("syserr-{:?}-lag", subsystem).as_bytes());
                errors.push(SystematicError {
                    id,
                    subsystem,
                    error_type: ErrorType::Lag,
                    magnitude: (recent_err - older_err).abs(),
                    persistence: 0.3,
                    description: String::from("prediction lag — errors shift over time"),
                    correction_applied: false,
                });
                self.systematic_errors
                    .insert(id, errors.last().unwrap().clone());
                self.systematic_count += 1;
            }
        }

        while self.systematic_errors.len() > MAX_ERROR_RECORDS {
            if let Some((&oldest, _)) = self.systematic_errors.iter().next() {
                self.systematic_errors.remove(&oldest);
            }
        }

        errors
    }

    /// Decompose prediction error into bias, variance, and noise
    pub fn prediction_decomposition(&self, subsystem: SubsystemId) -> PredictionDecomposition {
        let sub_key = subsystem as u8;
        let entries = self.subsystem_stats.get(&sub_key);

        match entries {
            Some(entries) if !entries.is_empty() => {
                let n = entries.len() as f32;
                let mean_pred = entries.iter().map(|e| e.predicted).sum::<f32>() / n;
                let mean_actual = entries.iter().map(|e| e.actual).sum::<f32>() / n;
                let bias = mean_pred - mean_actual;

                let variance = entries
                    .iter()
                    .map(|e| (e.predicted - mean_pred).powi(2))
                    .sum::<f32>()
                    / n;

                let mse = entries
                    .iter()
                    .map(|e| (e.predicted - e.actual).powi(2))
                    .sum::<f32>()
                    / n;

                let noise = (mse - bias.powi(2) - variance).max(0.0);
                let total = (bias.powi(2) + variance + noise).max(0.001);

                PredictionDecomposition {
                    subsystem,
                    bias_component: bias.powi(2),
                    variance_component: variance,
                    irreducible_noise: noise,
                    total_mse: mse,
                    bias_fraction: bias.powi(2) / total,
                    variance_fraction: variance / total,
                }
            },
            _ => PredictionDecomposition {
                subsystem,
                bias_component: 0.0,
                variance_component: 0.0,
                irreducible_noise: 0.0,
                total_mse: 0.0,
                bias_fraction: 0.0,
                variance_fraction: 0.0,
            },
        }
    }

    /// Model selection: identify which subsystem predicts each dimension best
    pub fn model_selection(&mut self, dimension: &str) -> ModelSelectionSignal {
        self.model_count += 1;

        let subsystems = [
            SubsystemId::Bridge,
            SubsystemId::Application,
            SubsystemId::Cooperative,
            SubsystemId::Memory,
            SubsystemId::Scheduler,
            SubsystemId::Network,
            SubsystemId::IO,
            SubsystemId::Thermal,
            SubsystemId::Holistic,
        ];

        let mut ranked: Vec<(SubsystemId, f32)> = Vec::new();
        for &sub in &subsystems {
            let acc = self.subsystem_accuracy(sub);
            if acc.sample_count > 0 {
                ranked.push((sub, acc.mean_accuracy));
            }
        }

        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));

        let (best, best_acc) = ranked
            .first()
            .copied()
            .unwrap_or((SubsystemId::Holistic, 0.0));
        let (second, second_acc) = ranked
            .get(1)
            .copied()
            .unwrap_or((SubsystemId::Holistic, 0.0));

        let id = fnv1a_hash(format!("model-sel-{}", dimension).as_bytes())
            ^ xorshift64(&mut self.rng_state);

        let signal = ModelSelectionSignal {
            id,
            dimension: String::from(dimension),
            best_subsystem: best,
            best_accuracy: best_acc,
            second_best: second,
            second_accuracy: second_acc,
            margin: best_acc - second_acc,
        };

        self.model_signals.insert(id, signal.clone());
        if self.model_signals.len() > MAX_MODEL_ENTRIES {
            if let Some((&oldest, _)) = self.model_signals.iter().next() {
                self.model_signals.remove(&oldest);
            }
        }

        signal
    }

    /// Generate recalibration signal for a subsystem
    pub fn recalibration_signal(&mut self, subsystem: SubsystemId) -> RecalibrationSignal {
        self.recal_count += 1;

        let acc = self.subsystem_accuracy(subsystem);
        let bias = acc.mean_error;
        let correction = -bias * 0.5;
        let urgency = if acc.mean_accuracy < ACCURACY_THRESHOLD {
            0.8
        } else if bias.abs() > SYSTEMATIC_ERROR_THRESHOLD {
            0.5
        } else {
            0.2
        };

        let confidence = (acc.sample_count as f32 / 50.0).clamp(0.1, 1.0);

        let id = fnv1a_hash(format!("recal-{:?}-{}", subsystem, self.tick).as_bytes())
            ^ xorshift64(&mut self.rng_state);

        let signal = RecalibrationSignal {
            id,
            target: subsystem,
            dimension: String::from("composite"),
            current_bias: bias,
            suggested_correction: correction,
            confidence,
            urgency,
        };

        self.recalibrations.insert(id, signal.clone());
        if self.recalibrations.len() > MAX_RECAL_SIGNALS {
            if let Some((&oldest, _)) = self.recalibrations.iter().next() {
                self.recalibrations.remove(&oldest);
            }
        }

        signal
    }

    /// Helper: convert subsystem key back to enum
    fn subsystem_from_key(key: u8) -> SubsystemId {
        match key {
            0 => SubsystemId::Bridge,
            1 => SubsystemId::Application,
            2 => SubsystemId::Cooperative,
            3 => SubsystemId::Memory,
            4 => SubsystemId::Scheduler,
            5 => SubsystemId::Network,
            6 => SubsystemId::IO,
            7 => SubsystemId::Thermal,
            _ => SubsystemId::Holistic,
        }
    }

    /// Gather aggregate statistics
    pub fn stats(&self) -> ValidatorStats {
        let mut best = 0.0_f32;
        let mut worst = 1.0_f32;
        for entries in self.subsystem_stats.values() {
            if entries.is_empty() {
                continue;
            }
            let acc = entries.iter().map(|e| e.accuracy).sum::<f32>() / entries.len() as f32;
            if acc > best {
                best = acc;
            }
            if acc < worst {
                worst = acc;
            }
        }

        ValidatorStats {
            total_validations: self.total_validations,
            global_accuracy_ema: self.accuracy_ema,
            global_bias_ema: self.bias_ema,
            systematic_errors_found: self.systematic_count,
            recalibrations_sent: self.recal_count,
            model_selections: self.model_count,
            best_accuracy: best,
            worst_accuracy: worst,
        }
    }
}
