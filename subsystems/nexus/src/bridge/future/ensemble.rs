// SPDX-License-Identifier: GPL-2.0
//! # Bridge Ensemble Predictor
//!
//! Multi-model ensemble for robust prediction. Combines predictions from
//! multiple models (EMA, trend, seasonal, causal) with dynamically learned
//! weights. Each model's weight is proportional to its recent accuracy, so
//! the ensemble automatically shifts trust toward whichever model is
//! performing best in the current regime.
//!
//! One model is a thesis; an ensemble is peer review.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_MEMBERS: usize = 16;
const MAX_HISTORY: usize = 512;
const WEIGHT_EMA_ALPHA: f32 = 0.10;
const ACCURACY_EMA_ALPHA: f32 = 0.08;
const MIN_WEIGHT: f32 = 0.01;
const DIVERSITY_BONUS: f32 = 0.05;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const DEFAULT_SEED: u64 = 0xE45E_1234_DEAD_BEEF;

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

fn rand_f32(state: &mut u64) -> f32 {
    (xorshift64(state) % 1_000_000) as f32 / 1_000_000.0
}

// ============================================================================
// ENSEMBLE MEMBER
// ============================================================================

/// A single model in the ensemble.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EnsembleMember {
    /// Unique identifier for this model
    pub model_id: u64,
    /// Current weight in the ensemble (0.0 to 1.0, sums to 1.0 across all)
    pub weight: f32,
    /// Recent accuracy measured as 1.0 - MAE (EMA)
    pub recent_accuracy: f32,
    /// Total predictions contributed
    pub total_predictions: u64,
    /// Running mean absolute error (EMA)
    mae_ema: f32,
    /// Running mean squared error (EMA)
    mse_ema: f32,
    /// Recent predictions for diversity computation
    recent_predictions: VecDeque<f32>,
    /// Streak of being the best model
    best_streak: u32,
}

impl EnsembleMember {
    fn new(model_id: u64) -> Self {
        Self {
            model_id,
            weight: 1.0,
            recent_accuracy: 0.5,
            total_predictions: 0,
            mae_ema: 0.1,
            mse_ema: 0.01,
            recent_predictions: VecDeque::new(),
            best_streak: 0,
        }
    }

    #[inline]
    fn record_prediction(&mut self, predicted: f32, actual: f32) {
        self.total_predictions += 1;
        let error = (actual - predicted).abs();
        let sq_error = error * error;

        self.mae_ema = self.mae_ema * (1.0 - ACCURACY_EMA_ALPHA) + error * ACCURACY_EMA_ALPHA;
        self.mse_ema = self.mse_ema * (1.0 - ACCURACY_EMA_ALPHA) + sq_error * ACCURACY_EMA_ALPHA;
        self.recent_accuracy = (1.0 - self.mae_ema).max(0.0);

        self.recent_predictions.push_back(predicted);
        if self.recent_predictions.len() > MAX_HISTORY {
            self.recent_predictions.pop_front();
        }
    }
}

// ============================================================================
// ENSEMBLE PREDICTION
// ============================================================================

/// Result of an ensemble prediction.
#[derive(Debug, Clone)]
pub struct EnsemblePrediction {
    /// Weighted ensemble prediction
    pub value: f32,
    /// Individual member predictions with weights
    pub member_values: Vec<(u64, f32, f32)>, // (model_id, prediction, weight)
    /// Ensemble disagreement (variance among members)
    pub disagreement: f32,
    /// Confidence based on member agreement
    pub confidence: f32,
}

// ============================================================================
// ENSEMBLE STATS
// ============================================================================

/// Statistics for the ensemble engine.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EnsembleStats {
    pub total_ensemble_predictions: u64,
    pub total_weight_updates: u64,
    pub member_count: u32,
    pub avg_diversity: f32,
    pub ensemble_mae: f32,
    pub best_member_mae: f32,
    pub ensemble_vs_best_ratio: f32,
    pub avg_disagreement: f32,
}

impl EnsembleStats {
    fn new() -> Self {
        Self {
            total_ensemble_predictions: 0,
            total_weight_updates: 0,
            member_count: 0,
            avg_diversity: 0.0,
            ensemble_mae: 0.1,
            best_member_mae: 0.1,
            ensemble_vs_best_ratio: 1.0,
            avg_disagreement: 0.0,
        }
    }
}

// ============================================================================
// WEIGHT HISTORY
// ============================================================================

/// Tracks weight evolution over time for analysis.
#[derive(Debug, Clone)]
struct WeightSnapshot {
    tick: u64,
    weights: Vec<(u64, f32)>,
}

// ============================================================================
// BRIDGE ENSEMBLE
// ============================================================================

/// Multi-model ensemble for robust bridge prediction.
///
/// Combines multiple prediction models with dynamically learned weights.
/// Automatically shifts trust toward better-performing models and tracks
/// diversity to avoid degenerate ensembles.
#[repr(align(64))]
pub struct BridgeEnsemble {
    /// Ensemble members
    members: BTreeMap<u64, EnsembleMember>,
    /// Ensemble-level error tracking
    ensemble_mae_ema: f32,
    ensemble_mse_ema: f32,
    /// History of ensemble predictions vs actuals
    prediction_history: VecDeque<(f32, f32)>, // (ensemble_pred, actual)
    /// Weight snapshots for evolution tracking
    weight_history: VecDeque<WeightSnapshot>,
    /// Running statistics
    stats: EnsembleStats,
    /// PRNG state
    rng: u64,
    /// Tick counter
    tick: u64,
}

impl BridgeEnsemble {
    /// Create a new ensemble engine.
    pub fn new() -> Self {
        Self {
            members: BTreeMap::new(),
            ensemble_mae_ema: 0.1,
            ensemble_mse_ema: 0.01,
            prediction_history: VecDeque::new(),
            weight_history: VecDeque::new(),
            stats: EnsembleStats::new(),
            rng: DEFAULT_SEED,
            tick: 0,
        }
    }

    /// Register a new model in the ensemble.
    #[inline]
    pub fn add_member(&mut self, model_id: u64) {
        if self.members.len() >= MAX_MEMBERS {
            return;
        }
        self.members.entry(model_id).or_insert_with(|| {
            EnsembleMember::new(model_id)
        });
        self.normalize_weights();
        self.stats.member_count = self.members.len() as u32;
    }

    /// Remove a model from the ensemble.
    #[inline]
    pub fn remove_member(&mut self, model_id: u64) {
        self.members.remove(&model_id);
        self.normalize_weights();
        self.stats.member_count = self.members.len() as u32;
    }

    fn normalize_weights(&mut self) {
        let total: f32 = self.members.values().map(|m| m.weight).sum();
        if total > 0.0 {
            for member in self.members.values_mut() {
                member.weight /= total;
                member.weight = member.weight.max(MIN_WEIGHT);
            }
            // Re-normalize after applying min
            let new_total: f32 = self.members.values().map(|m| m.weight).sum();
            if new_total > 0.0 {
                for member in self.members.values_mut() {
                    member.weight /= new_total;
                }
            }
        } else if !self.members.is_empty() {
            let uniform = 1.0 / self.members.len() as f32;
            for member in self.members.values_mut() {
                member.weight = uniform;
            }
        }
    }

    /// Generate an ensemble prediction from individual model predictions.
    pub fn ensemble_predict(
        &mut self,
        predictions: &[(u64, f32)], // (model_id, prediction)
    ) -> EnsemblePrediction {
        self.stats.total_ensemble_predictions += 1;

        let mut weighted_sum = 0.0f32;
        let mut total_weight = 0.0f32;
        let mut member_values = Vec::new();

        for &(model_id, pred) in predictions {
            let weight = self.members.get(&model_id).map(|m| m.weight).unwrap_or(0.0);
            if weight > 0.0 {
                weighted_sum += pred * weight;
                total_weight += weight;
                member_values.push((model_id, pred, weight));
            }
        }

        let ensemble_value = if total_weight > 0.0 {
            weighted_sum / total_weight
        } else if !predictions.is_empty() {
            // Fallback: simple average
            predictions.iter().map(|(_, p)| p).sum::<f32>() / predictions.len() as f32
        } else {
            0.0
        };

        // Compute disagreement (weighted variance of predictions)
        let mut disagreement = 0.0f32;
        for &(_, pred, weight) in &member_values {
            let diff = pred - ensemble_value;
            disagreement += weight * diff * diff;
        }

        let confidence = 1.0 / (1.0 + disagreement * 10.0);

        self.stats.avg_disagreement = self.stats.avg_disagreement * (1.0 - ACCURACY_EMA_ALPHA)
            + disagreement * ACCURACY_EMA_ALPHA;

        EnsemblePrediction {
            value: ensemble_value,
            member_values,
            disagreement,
            confidence,
        }
    }

    /// Update member weights based on observed actual value.
    pub fn update_weights(
        &mut self,
        predictions: &[(u64, f32)],
        actual: f32,
        tick: u64,
    ) {
        self.tick = tick;
        self.stats.total_weight_updates += 1;

        // Record individual member accuracy
        let mut best_error = f32::INFINITY;
        let mut best_id = 0u64;

        for &(model_id, pred) in predictions {
            if let Some(member) = self.members.get_mut(&model_id) {
                member.record_prediction(pred, actual);
                let error = (pred - actual).abs();
                if error < best_error {
                    best_error = error;
                    best_id = model_id;
                }
            }
        }

        // Update best streak
        for (id, member) in self.members.iter_mut() {
            if *id == best_id {
                member.best_streak += 1;
            } else {
                member.best_streak = 0;
            }
        }

        // Compute new weights: inverse-MAE weighted
        let mut weight_map: Vec<(u64, f32)> = Vec::new();
        for (id, member) in &self.members {
            let inverse_mae = 1.0 / (member.mae_ema + 0.001);
            weight_map.push((*id, inverse_mae));
        }
        let total_inv: f32 = weight_map.iter().map(|(_, w)| w).sum();
        if total_inv > 0.0 {
            for (id, new_w) in &weight_map {
                let normalized = new_w / total_inv;
                if let Some(member) = self.members.get_mut(id) {
                    member.weight = member.weight * (1.0 - WEIGHT_EMA_ALPHA)
                        + normalized * WEIGHT_EMA_ALPHA;
                }
            }
        }
        self.normalize_weights();

        // Update ensemble-level error
        let ensemble_pred = self.ensemble_predict(predictions).value;
        let ensemble_error = (ensemble_pred - actual).abs();
        self.ensemble_mae_ema = self.ensemble_mae_ema * (1.0 - ACCURACY_EMA_ALPHA)
            + ensemble_error * ACCURACY_EMA_ALPHA;
        self.ensemble_mse_ema = self.ensemble_mse_ema * (1.0 - ACCURACY_EMA_ALPHA)
            + ensemble_error * ensemble_error * ACCURACY_EMA_ALPHA;

        self.stats.ensemble_mae = self.ensemble_mae_ema;
        self.stats.best_member_mae = best_error;

        self.prediction_history.push_back((ensemble_pred, actual));
        if self.prediction_history.len() > MAX_HISTORY {
            self.prediction_history.pop_front();
        }

        // Snapshot weights periodically
        if tick % 100 == 0 {
            let weights: Vec<(u64, f32)> = self.members.iter().map(|(id, m)| (*id, m.weight)).collect();
            self.weight_history.push_back(WeightSnapshot { tick, weights });
            if self.weight_history.len() > 100 {
                self.weight_history.pop_front();
            }
        }
    }

    /// Get the accuracy of a specific member.
    #[inline(always)]
    pub fn member_accuracy(&self, model_id: u64) -> Option<f32> {
        self.members.get(&model_id).map(|m| m.recent_accuracy)
    }

    /// Compute the diversity score of the ensemble.
    ///
    /// High diversity means members disagree often — which is good for ensemble
    /// robustness. Measured as average pairwise correlation of prediction errors.
    pub fn diversity_score(&self) -> f32 {
        let members: Vec<&EnsembleMember> = self.members.values().collect();
        if members.len() < 2 {
            return 0.0;
        }

        let mut pairwise_correlation_sum = 0.0f32;
        let mut pair_count = 0u32;

        for i in 0..members.len() {
            for j in (i + 1)..members.len() {
                let a_preds = &members[i].recent_predictions;
                let b_preds = &members[j].recent_predictions;
                let min_len = a_preds.len().min(b_preds.len());
                if min_len < 5 {
                    continue;
                }

                // Compute Pearson correlation of last `min_len` predictions
                let start_a = a_preds.len() - min_len;
                let start_b = b_preds.len() - min_len;
                let a_slice = &a_preds[start_a..];
                let b_slice = &b_preds[start_b..];

                let mean_a: f32 = a_slice.iter().sum::<f32>() / min_len as f32;
                let mean_b: f32 = b_slice.iter().sum::<f32>() / min_len as f32;

                let mut cov = 0.0f32;
                let mut var_a = 0.0f32;
                let mut var_b = 0.0f32;

                for k in 0..min_len {
                    let da = a_slice[k] - mean_a;
                    let db = b_slice[k] - mean_b;
                    cov += da * db;
                    var_a += da * da;
                    var_b += db * db;
                }

                let denom = (var_a * var_b).sqrt();
                let corr = if denom > 0.001 { cov / denom } else { 0.0 };
                pairwise_correlation_sum += corr;
                pair_count += 1;
            }
        }

        if pair_count == 0 {
            return 0.5;
        }

        // Diversity = 1 - avg_correlation (low correlation = high diversity)
        let avg_corr = pairwise_correlation_sum / pair_count as f32;
        let diversity = (1.0 - avg_corr) / 2.0; // Scale to [0, 1]
        diversity.clamp(0.0, 1.0)
    }

    /// Compute the optimal linear combination weights (Bates-Granger style).
    pub fn optimal_combination(&self) -> Vec<(u64, f32)> {
        // Bates-Granger: weight inversely proportional to MSE
        let mut result = Vec::new();
        let mut total_inv_mse = 0.0f32;

        for (id, member) in &self.members {
            let inv_mse = 1.0 / (member.mse_ema + 0.0001);
            result.push((*id, inv_mse));
            total_inv_mse += inv_mse;
        }

        if total_inv_mse > 0.0 {
            for (_, w) in result.iter_mut() {
                *w /= total_inv_mse;
            }
        }
        result
    }

    /// Compare ensemble performance against the single best member.
    pub fn ensemble_vs_best(&self) -> f32 {
        let best_mae = self
            .members
            .values()
            .map(|m| m.mae_ema)
            .fold(f32::INFINITY, f32::min);

        if best_mae > 0.0001 {
            let ratio = self.ensemble_mae_ema / best_mae;
            ratio
        } else {
            1.0
        }
    }

    /// Get statistics.
    #[inline(always)]
    pub fn stats(&self) -> &EnsembleStats {
        &self.stats
    }

    /// Refresh computed statistics fields.
    pub fn refresh_stats(&mut self) {
        self.stats.member_count = self.members.len() as u32;
        self.stats.ensemble_vs_best_ratio = self.ensemble_vs_best();
        self.stats.avg_diversity = self.diversity_score();

        let best_mae = self
            .members
            .values()
            .map(|m| m.mae_ema)
            .fold(f32::INFINITY, f32::min);
        self.stats.best_member_mae = if best_mae.is_infinite() { 0.0 } else { best_mae };
    }

    /// Get member count.
    #[inline(always)]
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// Get weight of a specific member.
    #[inline(always)]
    pub fn member_weight(&self, model_id: u64) -> Option<f32> {
        self.members.get(&model_id).map(|m| m.weight)
    }
}
