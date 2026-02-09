// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Ensemble Predictor
//!
//! Multi-model ensemble for cooperation prediction. Combines a game theory
//! model, a trust model, a contention model, and a historical model —
//! weighting each by recent accuracy. Provides model selection, diversity
//! bonuses, and ensemble quality metrics.

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

/// FNV-1a hash for deterministic key hashing in no_std.
fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Xorshift64 PRNG for lightweight stochastic perturbation.
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// Exponential moving average update.
fn ema_update(current: u64, new_sample: u64, alpha_num: u64, alpha_den: u64) -> u64 {
    let weighted_old = current.saturating_mul(alpha_den.saturating_sub(alpha_num));
    let weighted_new = new_sample.saturating_mul(alpha_num);
    weighted_old.saturating_add(weighted_new) / alpha_den.max(1)
}

/// Identifier for the sub-model type in the ensemble.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModelType {
    GameTheory,
    TrustDynamics,
    ContentionPattern,
    HistoricalTrend,
}

/// A single sub-model's prediction.
#[derive(Clone, Debug)]
pub struct SubModelPrediction {
    pub model_type: ModelType,
    pub predicted_value: u64,
    pub confidence: u64,
    pub reasoning_hash: u64,
}

/// Combined ensemble prediction.
#[derive(Clone, Debug)]
pub struct EnsemblePrediction {
    pub target_id: u64,
    pub ensemble_value: u64,
    pub ensemble_confidence: u64,
    pub model_contributions: Vec<(ModelType, u64)>,
    pub diversity_score: u64,
    pub disagreement: u64,
}

/// Result of model selection (best model for a domain).
#[derive(Clone, Debug)]
pub struct ModelSelection {
    pub domain_hash: u64,
    pub best_model: ModelType,
    pub best_accuracy: u64,
    pub rankings: Vec<(ModelType, u64)>,
    pub selection_confidence: u64,
}

/// Fairness prediction via ensemble.
#[derive(Clone, Debug)]
pub struct EnsembleFairness {
    pub pair_hash: u64,
    pub predicted_fairness: u64,
    pub model_agreement: u64,
    pub confidence: u64,
    pub dominant_model: ModelType,
}

/// Diversity bonus measurement.
#[derive(Clone, Debug)]
pub struct DiversityBonus {
    pub domain_hash: u64,
    pub model_diversity: u64,
    pub prediction_spread: u64,
    pub ensemble_lift: u64,
    pub correlation_penalty: u64,
}

/// Ensemble quality report.
#[derive(Clone, Debug)]
pub struct EnsembleQuality {
    pub overall_accuracy: u64,
    pub ensemble_vs_best_single: i64,
    pub model_count: u32,
    pub active_models: u32,
    pub diversity_score: u64,
    pub calibration: u64,
}

/// Rolling statistics for the ensemble engine.
#[derive(Clone, Debug)]
#[repr(align(64))]
pub struct EnsembleStats {
    pub ensemble_predictions: u64,
    pub model_selections: u64,
    pub fairness_predictions: u64,
    pub diversity_checks: u64,
    pub quality_assessments: u64,
    pub avg_ensemble_accuracy: u64,
    pub avg_diversity: u64,
    pub best_model_wins: BTreeMap<ModelType, u64>,
}

impl EnsembleStats {
    pub fn new() -> Self {
        Self {
            ensemble_predictions: 0,
            model_selections: 0,
            fairness_predictions: 0,
            diversity_checks: 0,
            quality_assessments: 0,
            avg_ensemble_accuracy: 500,
            avg_diversity: 500,
            best_model_wins: BTreeMap::new(),
        }
    }
}

/// Internal tracking for a sub-model's accuracy.
#[derive(Clone, Debug)]
struct ModelTracker {
    model_type: ModelType,
    weight: u64,
    ema_accuracy: u64,
    prediction_count: u64,
    error_history: VecDeque<u64>,
    last_predictions: LinearMap<u64, 64>,
}

/// Internal record for an ensemble outcome.
#[derive(Clone, Debug)]
struct EnsembleOutcome {
    target_id: u64,
    ensemble_pred: u64,
    actual_value: u64,
    tick: u64,
    model_errors: BTreeMap<ModelType, u64>,
}

/// Multi-model ensemble engine for cooperation prediction.
pub struct CoopEnsemble {
    models: BTreeMap<ModelType, ModelTracker>,
    outcomes: VecDeque<EnsembleOutcome>,
    domain_best: BTreeMap<u64, ModelType>,
    stats: EnsembleStats,
    rng_state: u64,
    current_tick: u64,
    max_outcomes: usize,
    min_weight: u64,
}

impl CoopEnsemble {
    /// Create a new ensemble prediction engine.
    pub fn new(seed: u64) -> Self {
        let mut models = BTreeMap::new();

        for mt in &[
            ModelType::GameTheory,
            ModelType::TrustDynamics,
            ModelType::ContentionPattern,
            ModelType::HistoricalTrend,
        ] {
            models.insert(mt.clone(), ModelTracker {
                model_type: mt.clone(),
                weight: 250,
                ema_accuracy: 500,
                prediction_count: 0,
                error_history: VecDeque::new(),
                last_predictions: LinearMap::new(),
            });
        }

        Self {
            models,
            outcomes: VecDeque::new(),
            domain_best: BTreeMap::new(),
            stats: EnsembleStats::new(),
            rng_state: seed ^ 0xE75E_B1E0_C00P_0001,
            current_tick: 0,
            max_outcomes: 512,
            min_weight: 50,
        }
    }

    /// Feed a sub-model prediction for a target.
    #[inline]
    pub fn feed_prediction(
        &mut self,
        model_type: ModelType,
        target_id: u64,
        predicted: u64,
        confidence: u64,
    ) {
        if let Some(tracker) = self.models.get_mut(&model_type) {
            tracker.last_predictions.insert(target_id, predicted);
            tracker.prediction_count = tracker.prediction_count.saturating_add(1);
        }
    }

    /// Record actual outcome for model weight updates.
    pub fn record_outcome(&mut self, target_id: u64, actual_value: u64) {
        let mut model_errors: BTreeMap<ModelType, u64> = BTreeMap::new();
        let mut ensemble_pred: u64 = 0;
        let mut total_weight: u64 = 0;

        for (mt, tracker) in &self.models {
            if let Some(&predicted) = tracker.last_predictions.get(&target_id) {
                let error = if predicted > actual_value {
                    predicted - actual_value
                } else {
                    actual_value - predicted
                };
                model_errors.insert(mt.clone(), error);
                ensemble_pred = ensemble_pred
                    .saturating_add(predicted.saturating_mul(tracker.weight));
                total_weight = total_weight.saturating_add(tracker.weight);
            }
        }

        if total_weight > 0 {
            ensemble_pred /= total_weight;
        }

        for (mt, &error) in &model_errors {
            if let Some(tracker) = self.models.get_mut(mt) {
                let accuracy = 1000u64.saturating_sub(error.min(1000));
                tracker.ema_accuracy = ema_update(tracker.ema_accuracy, accuracy, 200, 1000);
                tracker.error_history.push(error);
                if tracker.error_history.len() > 128 {
                    tracker.error_history.pop_front().unwrap();
                }
            }
        }

        self.rebalance_weights();

        self.outcomes.push_back(EnsembleOutcome {
            target_id,
            ensemble_pred,
            actual_value,
            tick: self.current_tick,
            model_errors,
        });
        if self.outcomes.len() > self.max_outcomes {
            self.outcomes.pop_front();
        }
    }

    /// Produce an ensemble cooperation prediction for a target.
    pub fn ensemble_coop_predict(&mut self, target_id: u64) -> EnsemblePrediction {
        let mut weighted_sum: u64 = 0;
        let mut total_weight: u64 = 0;
        let mut contributions: Vec<(ModelType, u64)> = Vec::new();
        let mut predictions: Vec<u64> = Vec::new();

        for (mt, tracker) in &self.models {
            if let Some(&pred) = tracker.last_predictions.get(&target_id) {
                weighted_sum = weighted_sum
                    .saturating_add(pred.saturating_mul(tracker.weight));
                total_weight = total_weight.saturating_add(tracker.weight);
                contributions.push((mt.clone(), tracker.weight));
                predictions.push(pred);
            }
        }

        let ensemble_value = if total_weight > 0 {
            weighted_sum / total_weight
        } else {
            500
        };

        let diversity = self.compute_diversity(&predictions);
        let disagreement = self.compute_disagreement(&predictions, ensemble_value);

        let confidence = if disagreement < 100 {
            800u64.saturating_add(diversity / 10)
        } else {
            600u64.saturating_sub(disagreement / 5)
        }.min(1000);

        self.stats.ensemble_predictions = self.stats.ensemble_predictions.saturating_add(1);
        self.stats.avg_diversity = ema_update(self.stats.avg_diversity, diversity, 150, 1000);

        EnsemblePrediction {
            target_id,
            ensemble_value,
            ensemble_confidence: confidence,
            model_contributions: contributions,
            diversity_score: diversity,
            disagreement,
        }
    }

    /// Select the best model for a specific domain.
    pub fn model_selection(&mut self, domain_hash: u64) -> ModelSelection {
        let mut rankings: Vec<(ModelType, u64)> = self.models.iter()
            .map(|(mt, tracker)| (mt.clone(), tracker.ema_accuracy))
            .collect();
        rankings.sort_by(|a, b| b.1.cmp(&a.1));

        let best = rankings.first().cloned().unwrap_or((ModelType::HistoricalTrend, 0));

        let selection_confidence = if rankings.len() >= 2 {
            let gap = best.1.saturating_sub(rankings[1].1);
            (500u64.saturating_add(gap.saturating_mul(5))).min(1000)
        } else {
            500
        };

        self.domain_best.insert(domain_hash, best.0.clone());
        self.stats.model_selections = self.stats.model_selections.saturating_add(1);
        *self.stats.best_model_wins.entry(best.0.clone()).or_insert(0) += 1;

        ModelSelection {
            domain_hash,
            best_model: best.0,
            best_accuracy: best.1,
            rankings,
            selection_confidence,
        }
    }

    /// Predict fairness for a cooperation pair using ensemble.
    pub fn ensemble_fairness(&mut self, pair_hash: u64) -> EnsembleFairness {
        let mut weighted_sum: u64 = 0;
        let mut total_weight: u64 = 0;
        let mut preds: Vec<u64> = Vec::new();

        for (_, tracker) in &self.models {
            let fairness_key = fnv1a_hash(&[
                b"fair_",
                pair_hash.to_le_bytes().as_slice(),
            ].concat());

            let pred = tracker.last_predictions.get(&fairness_key)
                .copied()
                .unwrap_or(500);
            weighted_sum = weighted_sum
                .saturating_add(pred.saturating_mul(tracker.weight));
            total_weight = total_weight.saturating_add(tracker.weight);
            preds.push(pred);
        }

        let predicted = if total_weight > 0 {
            weighted_sum / total_weight
        } else {
            500
        };

        let agreement = self.compute_agreement(&preds, predicted);

        let dominant = self.domain_best.get(&pair_hash)
            .cloned()
            .unwrap_or(ModelType::TrustDynamics);

        let confidence = agreement.saturating_mul(800) / 1000;

        self.stats.fairness_predictions = self.stats.fairness_predictions.saturating_add(1);

        EnsembleFairness {
            pair_hash,
            predicted_fairness: predicted,
            model_agreement: agreement,
            confidence,
            dominant_model: dominant,
        }
    }

    /// Measure the diversity bonus of the ensemble.
    pub fn diversity_bonus(&mut self, domain_hash: u64) -> DiversityBonus {
        let predictions: Vec<u64> = self.models.values()
            .filter_map(|t| t.last_predictions.get(&domain_hash).copied())
            .collect();

        let diversity = self.compute_diversity(&predictions);
        let spread = self.compute_spread(&predictions);

        let solo_best = self.models.values()
            .map(|t| t.ema_accuracy)
            .max()
            .unwrap_or(0);

        let ensemble_acc = self.stats.avg_ensemble_accuracy;
        let lift = if ensemble_acc > solo_best {
            ensemble_acc - solo_best
        } else {
            0
        };

        let correlation = self.compute_model_correlation();
        let penalty = correlation.saturating_mul(100) / 1000;

        self.stats.diversity_checks = self.stats.diversity_checks.saturating_add(1);

        DiversityBonus {
            domain_hash,
            model_diversity: diversity,
            prediction_spread: spread,
            ensemble_lift: lift,
            correlation_penalty: penalty,
        }
    }

    /// Identify the best predictor overall.
    #[inline]
    pub fn best_predictor(&self) -> (ModelType, u64) {
        self.models.iter()
            .max_by_key(|(_, t)| t.ema_accuracy)
            .map(|(mt, t)| (mt.clone(), t.ema_accuracy))
            .unwrap_or((ModelType::HistoricalTrend, 0))
    }

    /// Assess overall ensemble quality.
    pub fn ensemble_quality(&mut self) -> EnsembleQuality {
        let overall_acc = self.compute_overall_accuracy();
        let (_, best_single_acc) = self.best_predictor();
        let diff = overall_acc as i64 - best_single_acc as i64;

        let active = self.models.values()
            .filter(|t| t.prediction_count > 0)
            .count() as u32;

        let diversity = self.stats.avg_diversity;

        let calibration = self.compute_calibration();

        self.stats.quality_assessments = self.stats.quality_assessments.saturating_add(1);
        self.stats.avg_ensemble_accuracy = ema_update(
            self.stats.avg_ensemble_accuracy,
            overall_acc,
            150,
            1000,
        );

        EnsembleQuality {
            overall_accuracy: overall_acc,
            ensemble_vs_best_single: diff,
            model_count: self.models.len() as u32,
            active_models: active,
            diversity_score: diversity,
            calibration,
        }
    }

    /// Advance the internal tick.
    #[inline(always)]
    pub fn tick(&mut self) {
        self.current_tick = self.current_tick.wrapping_add(1);
    }

    /// Retrieve current statistics.
    #[inline(always)]
    pub fn stats(&self) -> &EnsembleStats {
        &self.stats
    }

    // ── Private helpers ──────────────────────────────────────────────

    fn rebalance_weights(&mut self) {
        let total_accuracy: u64 = self.models.values()
            .map(|t| t.ema_accuracy)
            .sum();

        if total_accuracy == 0 {
            for tracker in self.models.values_mut() {
                tracker.weight = 250;
            }
            return;
        }

        for tracker in self.models.values_mut() {
            let raw_weight = tracker.ema_accuracy.saturating_mul(1000) / total_accuracy;
            tracker.weight = raw_weight.max(self.min_weight);
        }

        let total_weight: u64 = self.models.values().map(|t| t.weight).sum();
        if total_weight > 0 {
            for tracker in self.models.values_mut() {
                tracker.weight = tracker.weight.saturating_mul(1000) / total_weight;
            }
        }
    }

    fn compute_diversity(&self, predictions: &[u64]) -> u64 {
        if predictions.len() < 2 {
            return 0;
        }

        let n = predictions.len();
        let mut total_diff: u64 = 0;
        let mut pairs: u64 = 0;

        for i in 0..n {
            for j in (i + 1)..n {
                let diff = if predictions[i] > predictions[j] {
                    predictions[i] - predictions[j]
                } else {
                    predictions[j] - predictions[i]
                };
                total_diff = total_diff.saturating_add(diff);
                pairs += 1;
            }
        }

        if pairs > 0 {
            total_diff / pairs
        } else {
            0
        }
    }

    fn compute_disagreement(&self, predictions: &[u64], ensemble: u64) -> u64 {
        if predictions.is_empty() {
            return 0;
        }
        let total: u64 = predictions.iter()
            .map(|&p| if p > ensemble { p - ensemble } else { ensemble - p })
            .sum();
        total / predictions.len() as u64
    }

    fn compute_agreement(&self, predictions: &[u64], center: u64) -> u64 {
        let disagreement = self.compute_disagreement(predictions, center);
        1000u64.saturating_sub(disagreement.min(1000))
    }

    fn compute_spread(&self, predictions: &[u64]) -> u64 {
        let min_val = predictions.iter().copied().min().unwrap_or(0);
        let max_val = predictions.iter().copied().max().unwrap_or(0);
        max_val.saturating_sub(min_val)
    }

    fn compute_model_correlation(&self) -> u64 {
        let accuracies: Vec<u64> = self.models.values().map(|t| t.ema_accuracy).collect();
        if accuracies.len() < 2 {
            return 0;
        }

        let mean = accuracies.iter().sum::<u64>() / accuracies.len() as u64;
        let variance: u64 = accuracies.iter()
            .map(|&a| {
                let d = if a > mean { a - mean } else { mean - a };
                d.saturating_mul(d)
            })
            .sum::<u64>()
            / accuracies.len() as u64;

        1000u64.saturating_sub(variance.min(1000))
    }

    fn compute_overall_accuracy(&self) -> u64 {
        if self.outcomes.is_empty() {
            return 500;
        }

        let recent = if self.outcomes.len() > 50 {
            &self.outcomes[self.outcomes.len() - 50..]
        } else {
            &self.outcomes[..]
        };

        let total_error: u64 = recent.iter().map(|o| {
            if o.ensemble_pred > o.actual_value {
                o.ensemble_pred - o.actual_value
            } else {
                o.actual_value - o.ensemble_pred
            }
        }).sum();

        let avg_error = total_error / recent.len().max(1) as u64;
        1000u64.saturating_sub(avg_error.min(1000))
    }

    fn compute_calibration(&self) -> u64 {
        let acc_values: Vec<u64> = self.models.values().map(|t| t.ema_accuracy).collect();
        if acc_values.is_empty() {
            return 500;
        }
        let mean = acc_values.iter().sum::<u64>() / acc_values.len() as u64;
        let max_dev = acc_values.iter()
            .map(|&a| if a > mean { a - mean } else { mean - a })
            .max()
            .unwrap_or(0);
        1000u64.saturating_sub(max_dev.min(1000))
    }
}
