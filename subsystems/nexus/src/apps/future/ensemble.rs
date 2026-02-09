// SPDX-License-Identifier: GPL-2.0
//! # Apps Ensemble Prediction Engine
//!
//! Multi-model ensemble for application behavior prediction. Combines four
//! independent sub-models — behavioral, resource, pattern, and causal — into
//! a single weighted prediction that outperforms any individual model.
//!
//! Model weights are adapted online via EMA-tracked per-model accuracy.
//! The engine also measures ensemble diversity (disagreement among models)
//! which is the key driver of ensemble advantage over single models.
//!
//! This is the apps engine combining many weak predictors into one strong one.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const NUM_MODELS: usize = 4;
const MAX_APPS: usize = 256;
const MAX_HISTORY: usize = 512;
const EMA_ALPHA: f64 = 0.12;
const MIN_WEIGHT: f64 = 0.05;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const XORSHIFT_SEED: u64 = 0xc0ffee_b00b1e5;

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

fn ema_update(current: f64, sample: f64, alpha: f64) -> f64 {
    alpha * sample + (1.0 - alpha) * current
}

fn abs_f64(v: f64) -> f64 {
    if v < 0.0 { -v } else { v }
}

// ============================================================================
// MODEL IDENTITY
// ============================================================================

/// Identifiers for the sub-models in the ensemble.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModelId {
    Behavioral,
    Resource,
    Pattern,
    Causal,
}

impl ModelId {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Behavioral => "behavioral",
            Self::Resource => "resource",
            Self::Pattern => "pattern",
            Self::Causal => "causal",
        }
    }

    fn index(&self) -> usize {
        match self {
            Self::Behavioral => 0,
            Self::Resource => 1,
            Self::Pattern => 2,
            Self::Causal => 3,
        }
    }

    fn from_index(i: usize) -> Self {
        match i % NUM_MODELS {
            0 => Self::Behavioral,
            1 => Self::Resource,
            2 => Self::Pattern,
            _ => Self::Causal,
        }
    }
}

// ============================================================================
// MODEL PREDICTION
// ============================================================================

/// A single model's prediction for a given app and metric.
#[derive(Debug, Clone)]
pub struct ModelPrediction {
    pub model: ModelId,
    pub predicted_value: f64,
    pub confidence: f64,
    pub model_specific_features: u64,
}

// ============================================================================
// ENSEMBLE PREDICTION RESULT
// ============================================================================

/// The combined ensemble prediction.
#[derive(Debug, Clone)]
pub struct EnsemblePrediction {
    pub combined_value: f64,
    pub individual_predictions: Vec<(ModelId, f64, f64)>,
    pub weights_used: Vec<(ModelId, f64)>,
    pub diversity: f64,
    pub ensemble_confidence: f64,
}

// ============================================================================
// PER-MODEL TRACKER
// ============================================================================

#[derive(Debug, Clone)]
struct ModelTracker {
    model: ModelId,
    ema_error: f64,
    ema_abs_error: f64,
    ema_accuracy: f64,
    weight: f64,
    prediction_count: u64,
    correct_direction_count: u64,
    error_history: Vec<f64>,
}

impl ModelTracker {
    fn new(model: ModelId) -> Self {
        Self {
            model,
            ema_error: 0.0,
            ema_abs_error: 0.5,
            ema_accuracy: 0.5,
            weight: 1.0 / NUM_MODELS as f64,
            prediction_count: 0,
            correct_direction_count: 0,
            error_history: Vec::new(),
        }
    }

    fn record_outcome(&mut self, predicted: f64, actual: f64, prev_actual: f64) {
        let error = actual - predicted;
        let abs_error = abs_f64(error);
        self.ema_error = ema_update(self.ema_error, error, EMA_ALPHA);
        self.ema_abs_error = ema_update(self.ema_abs_error, abs_error, EMA_ALPHA);

        self.prediction_count += 1;
        let predicted_direction = predicted - prev_actual;
        let actual_direction = actual - prev_actual;
        if (predicted_direction >= 0.0) == (actual_direction >= 0.0) {
            self.correct_direction_count += 1;
        }

        let dir_acc = if self.prediction_count > 0 {
            self.correct_direction_count as f64 / self.prediction_count as f64
        } else {
            0.5
        };
        self.ema_accuracy = ema_update(self.ema_accuracy, dir_acc, EMA_ALPHA);

        if self.error_history.len() >= MAX_HISTORY {
            self.error_history.remove(0);
        }
        self.error_history.push(abs_error);
    }

    fn update_weight(&mut self, inv_error_sum: f64) {
        if inv_error_sum <= 0.0 {
            self.weight = 1.0 / NUM_MODELS as f64;
            return;
        }
        let inv_err = 1.0 / (self.ema_abs_error + 0.001);
        self.weight = (inv_err / inv_error_sum).max(MIN_WEIGHT);
    }

    fn recent_mean_abs_error(&self, n: usize) -> f64 {
        if self.error_history.is_empty() {
            return self.ema_abs_error;
        }
        let start = if self.error_history.len() > n {
            self.error_history.len() - n
        } else {
            0
        };
        let slice = &self.error_history[start..];
        let sum: f64 = slice.iter().sum();
        if slice.is_empty() { 0.0 } else { sum / slice.len() as f64 }
    }
}

// ============================================================================
// PER-APP ENSEMBLE STATE
// ============================================================================

#[derive(Debug, Clone)]
struct AppEnsembleState {
    app_id: u64,
    models: [ModelTracker; NUM_MODELS],
    last_actual: f64,
    ensemble_error_history: Vec<f64>,
    ensemble_ema_error: f64,
    total_ensembles: u64,
}

impl AppEnsembleState {
    fn new(app_id: u64) -> Self {
        Self {
            app_id,
            models: [
                ModelTracker::new(ModelId::Behavioral),
                ModelTracker::new(ModelId::Resource),
                ModelTracker::new(ModelId::Pattern),
                ModelTracker::new(ModelId::Causal),
            ],
            last_actual: 0.0,
            ensemble_error_history: Vec::new(),
            ensemble_ema_error: 0.5,
            total_ensembles: 0,
        }
    }

    fn rebalance_weights(&mut self) {
        let inv_sum: f64 = self
            .models
            .iter()
            .map(|m| 1.0 / (m.ema_abs_error + 0.001))
            .sum();
        for m in &mut self.models {
            m.update_weight(inv_sum);
        }
        // Normalize weights to sum to 1.0
        let w_sum: f64 = self.models.iter().map(|m| m.weight).sum();
        if w_sum > 0.0 {
            for m in &mut self.models {
                m.weight /= w_sum;
            }
        }
    }
}

// ============================================================================
// ENSEMBLE STATS
// ============================================================================

/// Engine-level statistics for the ensemble module.
#[derive(Debug, Clone)]
pub struct EnsembleStats {
    pub total_ensemble_predictions: u64,
    pub total_model_outcomes: u64,
    pub average_diversity: f64,
    pub average_ensemble_error: f64,
    pub average_best_single_error: f64,
    pub ensemble_advantage_ratio: f64,
    pub weight_rebalances: u64,
}

impl EnsembleStats {
    fn new() -> Self {
        Self {
            total_ensemble_predictions: 0,
            total_model_outcomes: 0,
            average_diversity: 0.0,
            average_ensemble_error: 0.0,
            average_best_single_error: 0.0,
            ensemble_advantage_ratio: 1.0,
            weight_rebalances: 0,
        }
    }
}

// ============================================================================
// APPS ENSEMBLE ENGINE
// ============================================================================

/// Multi-model ensemble engine for application behavior prediction.
///
/// Combines behavioral, resource, pattern, and causal sub-models using
/// adaptive weighting based on recent prediction accuracy.
pub struct AppsEnsemble {
    app_states: BTreeMap<u64, AppEnsembleState>,
    stats: EnsembleStats,
    rng_state: u64,
    tick: u64,
    ema_diversity: f64,
    ema_ensemble_err: f64,
    ema_best_single_err: f64,
}

impl AppsEnsemble {
    /// Create a new ensemble prediction engine.
    pub fn new() -> Self {
        Self {
            app_states: BTreeMap::new(),
            stats: EnsembleStats::new(),
            rng_state: XORSHIFT_SEED,
            tick: 0,
            ema_diversity: 0.0,
            ema_ensemble_err: 0.5,
            ema_best_single_err: 0.5,
        }
    }

    /// Record an observed outcome to update all model trackers.
    pub fn record_outcome(
        &mut self,
        app_id: u64,
        model_predictions: &[(ModelId, f64)],
        actual: f64,
    ) {
        self.tick += 1;
        self.stats.total_model_outcomes += 1;

        if self.app_states.len() >= MAX_APPS && !self.app_states.contains_key(&app_id) {
            return;
        }
        let state = self.app_states.entry(app_id).or_insert_with(|| AppEnsembleState::new(app_id));
        let prev = state.last_actual;
        state.last_actual = actual;

        for &(model, predicted) in model_predictions {
            let idx = model.index();
            if idx < NUM_MODELS {
                state.models[idx].record_outcome(predicted, actual, prev);
            }
        }

        state.rebalance_weights();
        self.stats.weight_rebalances += 1;
    }

    /// Produce an ensemble prediction by combining sub-model predictions.
    pub fn ensemble_app_predict(
        &mut self,
        app_id: u64,
        predictions: &[ModelPrediction],
    ) -> EnsemblePrediction {
        self.stats.total_ensemble_predictions += 1;

        let state = match self.app_states.get(&app_id) {
            Some(s) => s,
            None => {
                // Equal weighting fallback
                let combined = if predictions.is_empty() {
                    0.0
                } else {
                    predictions.iter().map(|p| p.predicted_value).sum::<f64>() / predictions.len() as f64
                };
                return EnsemblePrediction {
                    combined_value: combined,
                    individual_predictions: predictions.iter().map(|p| (p.model, p.predicted_value, p.confidence)).collect(),
                    weights_used: predictions.iter().map(|p| (p.model, 1.0 / predictions.len().max(1) as f64)).collect(),
                    diversity: 0.0,
                    ensemble_confidence: 0.5,
                };
            }
        };

        let mut combined = 0.0;
        let mut weight_sum = 0.0;
        let mut individual = Vec::new();
        let mut weights_used = Vec::new();

        for pred in predictions {
            let idx = pred.model.index();
            let w = if idx < NUM_MODELS { state.models[idx].weight } else { MIN_WEIGHT };
            combined += pred.predicted_value * w;
            weight_sum += w;
            individual.push((pred.model, pred.predicted_value, pred.confidence));
            weights_used.push((pred.model, w));
        }

        if weight_sum > 0.0 {
            combined /= weight_sum;
        }

        // Compute diversity: mean pairwise disagreement
        let diversity = self.compute_diversity(predictions);
        self.ema_diversity = ema_update(self.ema_diversity, diversity, EMA_ALPHA);
        self.stats.average_diversity = self.ema_diversity;

        let confidence = if predictions.is_empty() {
            0.0
        } else {
            let mean_conf: f64 = predictions.iter().map(|p| p.confidence).sum::<f64>() / predictions.len() as f64;
            mean_conf * (1.0 + diversity * 0.1).min(1.0)
        };

        EnsemblePrediction {
            combined_value: combined,
            individual_predictions: individual,
            weights_used,
            diversity,
            ensemble_confidence: confidence,
        }
    }

    /// Compute current model weights for an app.
    pub fn model_weighting(&self, app_id: u64) -> Vec<(ModelId, f64)> {
        let state = match self.app_states.get(&app_id) {
            Some(s) => s,
            None => {
                let default_w = 1.0 / NUM_MODELS as f64;
                return (0..NUM_MODELS).map(|i| (ModelId::from_index(i), default_w)).collect();
            }
        };

        state.models.iter().map(|m| (m.model, m.weight)).collect()
    }

    /// Measure diversity among model predictions.
    fn compute_diversity(&self, predictions: &[ModelPrediction]) -> f64 {
        if predictions.len() < 2 {
            return 0.0;
        }

        let mean: f64 = predictions.iter().map(|p| p.predicted_value).sum::<f64>() / predictions.len() as f64;
        let variance: f64 = predictions
            .iter()
            .map(|p| {
                let d = p.predicted_value - mean;
                d * d
            })
            .sum::<f64>()
            / predictions.len() as f64;

        // Normalize by mean magnitude
        let norm = abs_f64(mean) + 0.001;
        let var_sqrt = {
            let mut g = variance;
            if g > 0.0 {
                for _ in 0..20 {
                    g = 0.5 * (g + variance / g);
                }
            }
            g
        };
        var_sqrt / norm
    }

    /// Public diversity measure for external callers.
    pub fn diversity_measure(&self, predictions: &[ModelPrediction]) -> f64 {
        self.compute_diversity(predictions)
    }

    /// Get ensemble accuracy vs best single model accuracy for an app.
    pub fn ensemble_accuracy(&self, app_id: u64) -> (f64, f64) {
        let state = match self.app_states.get(&app_id) {
            Some(s) => s,
            None => return (0.5, 0.5),
        };

        let ensemble_err = state.ensemble_ema_error;
        let best_single_err = state
            .models
            .iter()
            .map(|m| m.ema_abs_error)
            .fold(f64::MAX, |a, b| if b < a { b } else { a });

        let ens_acc = 1.0 / (1.0 + ensemble_err);
        let single_acc = 1.0 / (1.0 + best_single_err);
        (ens_acc, single_acc)
    }

    /// Identify the best single model for an app.
    pub fn best_single_model(&self, app_id: u64) -> (ModelId, f64) {
        let state = match self.app_states.get(&app_id) {
            Some(s) => s,
            None => return (ModelId::Behavioral, 0.5),
        };

        let mut best = ModelId::Behavioral;
        let mut best_err = f64::MAX;

        for m in &state.models {
            if m.ema_abs_error < best_err {
                best_err = m.ema_abs_error;
                best = m.model;
            }
        }

        let accuracy = 1.0 / (1.0 + best_err);
        (best, accuracy)
    }

    /// Compute the ensemble advantage: ratio of ensemble accuracy to best single model.
    pub fn ensemble_advantage(&mut self, app_id: u64) -> f64 {
        let (ens_acc, single_acc) = self.ensemble_accuracy(app_id);
        let advantage = if single_acc > 0.0 {
            ens_acc / single_acc
        } else {
            1.0
        };

        self.ema_ensemble_err = ema_update(self.ema_ensemble_err, 1.0 - ens_acc, EMA_ALPHA);
        self.ema_best_single_err = ema_update(self.ema_best_single_err, 1.0 - single_acc, EMA_ALPHA);
        self.stats.average_ensemble_error = self.ema_ensemble_err;
        self.stats.average_best_single_error = self.ema_best_single_err;
        self.stats.ensemble_advantage_ratio = advantage;

        advantage
    }

    /// Return a snapshot of engine statistics.
    pub fn stats(&self) -> &EnsembleStats {
        &self.stats
    }

    /// Number of tracked apps.
    pub fn tracked_apps(&self) -> usize {
        self.app_states.len()
    }

    /// Global EMA-smoothed diversity.
    pub fn avg_diversity(&self) -> f64 {
        self.ema_diversity
    }
}
