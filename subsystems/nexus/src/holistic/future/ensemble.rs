// SPDX-License-Identifier: GPL-2.0
//! # Holistic Ensemble — Master Multi-Model Ensemble
//!
//! The **meta-ensemble**: combines ALL prediction models from ALL subsystems
//! into a single, unified prediction. Where individual subsystems run their own
//! ensembles, this module ensembles the ensembles — a hierarchical fusion that
//! weighs every model by accuracy, recency, diversity, and cross-subsystem
//! coherence.
//!
//! ## Capabilities
//!
//! - Master ensemble that fuses predictions from every NEXUS subsystem
//! - Cross-subsystem model fusion with conflict resolution
//! - Hierarchical ensemble architecture (leaf → subsystem → master)
//! - Global model selection: dynamic weight adjustment per model
//! - Ensemble dominance analysis: which models actually matter?
//! - Meta-accuracy tracking: how good is the ensemble of ensembles?

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_MODELS: usize = 256;
const MAX_SUBSYSTEM_ENSEMBLES: usize = 16;
const MAX_HIERARCHY_LEVELS: usize = 8;
const MAX_FUSION_HISTORY: usize = 1024;
const MAX_DOMINANCE_ENTRIES: usize = 128;
const WEIGHT_DECAY: f32 = 0.98;
const MIN_MODEL_WEIGHT: f32 = 0.001;
const DIVERSITY_BONUS: f32 = 0.05;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

// ============================================================================
// HELPER FUNCTIONS
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
fn ema_update(current: f32, sample: f32) -> f32 {
    EMA_ALPHA * sample + (1.0 - EMA_ALPHA) * current
}

// ============================================================================
// DOMAIN TYPES
// ============================================================================

/// Subsystem that contributes an ensemble
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EnsembleSource {
    BridgePredictor,
    ApplicationPredictor,
    CooperativePredictor,
    SchedulerModel,
    MemoryModel,
    IoModel,
    NetworkModel,
    ThermalModel,
    PowerModel,
    SecurityModel,
    MonteCarloSampler,
    CausalModel,
    TimelineProjector,
    HorizonForecast,
}

/// A single model within an ensemble
#[derive(Debug, Clone)]
pub struct EnsembleMember {
    pub model_id: u64,
    pub source: EnsembleSource,
    pub weight: f32,
    pub accuracy: f32,
    pub prediction: f32,
    pub variance: f32,
    pub staleness_us: u64,
    pub diversity_score: f32,
    pub active: bool,
}

/// Subsystem-level ensemble result
#[derive(Debug, Clone)]
pub struct SubsystemEnsemble {
    pub source: EnsembleSource,
    pub members: Vec<u64>,
    pub fused_prediction: f32,
    pub fused_variance: f32,
    pub ensemble_accuracy: f32,
    pub member_count: usize,
}

/// Master ensemble fusion result
#[derive(Debug, Clone)]
pub struct MasterEnsembleResult {
    pub prediction: f32,
    pub variance: f32,
    pub confidence: f32,
    pub subsystem_contributions: LinearMap<f32, 64>,
    pub total_weight: f32,
    pub active_models: usize,
    pub diversity_index: f32,
    pub timestamp_us: u64,
}

/// Cross-subsystem fusion result
#[derive(Debug, Clone)]
pub struct CrossSubsystemFusion {
    pub from_source: EnsembleSource,
    pub to_source: EnsembleSource,
    pub agreement: f32,
    pub conflict: f32,
    pub resolution: FusionResolution,
    pub blended_prediction: f32,
}

/// How a cross-subsystem conflict was resolved
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FusionResolution {
    WeightedAverage,
    HighestAccuracy,
    MostRecent,
    MedianVote,
    ConflictFlagged,
}

/// Hierarchy level in the ensemble
#[derive(Debug, Clone)]
pub struct HierarchyLevel {
    pub level: usize,
    pub ensemble_count: usize,
    pub total_models: usize,
    pub fused_prediction: f32,
    pub fused_variance: f32,
    pub description: String,
}

/// Model selection state (weights, rankings)
#[derive(Debug, Clone)]
pub struct ModelSelection {
    pub rankings: Vec<ModelRanking>,
    pub total_weight: f32,
    pub active_count: usize,
    pub pruned_count: usize,
    pub weight_entropy: f32,
}

/// Ranking of a single model
#[derive(Debug, Clone)]
pub struct ModelRanking {
    pub model_id: u64,
    pub source: EnsembleSource,
    pub weight: f32,
    pub rank: usize,
    pub accuracy: f32,
    pub contribution: f32,
}

/// Ensemble dominance analysis: which models dominate?
#[derive(Debug, Clone)]
pub struct EnsembleDominance {
    pub dominant_model_id: u64,
    pub dominant_source: EnsembleSource,
    pub dominance_score: f32,
    pub concentration_index: f32,
    pub top_k_weight_share: f32,
    pub is_healthy: bool,
    pub diversity_warning: bool,
}

/// Meta-accuracy: how accurate is the master ensemble?
#[derive(Debug, Clone)]
pub struct MetaAccuracy {
    pub overall_accuracy: f32,
    pub accuracy_by_source: LinearMap<f32, 64>,
    pub recent_accuracy: f32,
    pub accuracy_trend: f32,
    pub total_evaluations: u64,
    pub improvement_over_best_single: f32,
}

// ============================================================================
// STATISTICS
// ============================================================================

/// Runtime statistics for the ensemble engine
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EnsembleStats {
    pub master_fusions: u64,
    pub cross_fusions: u64,
    pub hierarchy_builds: u64,
    pub model_selections: u64,
    pub dominance_analyses: u64,
    pub accuracy_evaluations: u64,
    pub avg_ensemble_size: f32,
    pub avg_diversity: f32,
    pub avg_meta_accuracy: f32,
    pub avg_dominance_score: f32,
}

impl EnsembleStats {
    fn new() -> Self {
        Self {
            master_fusions: 0,
            cross_fusions: 0,
            hierarchy_builds: 0,
            model_selections: 0,
            dominance_analyses: 0,
            accuracy_evaluations: 0,
            avg_ensemble_size: 0.0,
            avg_diversity: 0.0,
            avg_meta_accuracy: 0.0,
            avg_dominance_score: 0.0,
        }
    }
}

// ============================================================================
// HOLISTIC ENSEMBLE ENGINE
// ============================================================================

/// Master ensemble prediction engine
pub struct HolisticEnsemble {
    members: BTreeMap<u64, EnsembleMember>,
    subsystem_ensembles: BTreeMap<u64, SubsystemEnsemble>,
    fusion_history: Vec<MasterEnsembleResult>,
    accuracy_log: Vec<(u64, f32, f32)>,
    rng_state: u64,
    next_model_id: u64,
    stats: EnsembleStats,
    generation: u64,
}

impl HolisticEnsemble {
    /// Create a new holistic ensemble engine
    pub fn new(seed: u64) -> Self {
        Self {
            members: BTreeMap::new(),
            subsystem_ensembles: BTreeMap::new(),
            fusion_history: Vec::new(),
            accuracy_log: Vec::new(),
            rng_state: seed ^ 0xFACE_FEED_DEAD_CAFE,
            next_model_id: 1,
            stats: EnsembleStats::new(),
            generation: 0,
        }
    }

    /// Register a model member in the ensemble
    pub fn register_model(
        &mut self,
        source: EnsembleSource,
        initial_weight: f32,
        accuracy: f32,
    ) -> u64 {
        let id = self.next_model_id;
        self.next_model_id += 1;
        let member = EnsembleMember {
            model_id: id,
            source,
            weight: initial_weight.max(MIN_MODEL_WEIGHT),
            accuracy: accuracy.clamp(0.0, 1.0),
            prediction: 0.0,
            variance: 0.01,
            staleness_us: 0,
            diversity_score: 0.0,
            active: true,
        };
        self.members.insert(id, member);
        id
    }

    /// Submit a prediction from a model member
    #[inline]
    pub fn submit_prediction(&mut self, model_id: u64, prediction: f32, variance: f32) {
        if let Some(member) = self.members.get_mut(&model_id) {
            member.prediction = prediction;
            member.variance = variance.max(0.0001);
            member.staleness_us = 0;
        }
    }

    /// Perform master ensemble fusion across all models
    pub fn master_ensemble(&mut self, timestamp_us: u64) -> MasterEnsembleResult {
        self.stats.master_fusions += 1;
        self.generation += 1;

        let active_members: Vec<(u64, f32, f32, f32)> = self
            .members
            .values()
            .filter(|m| m.active && m.weight >= MIN_MODEL_WEIGHT)
            .map(|m| (m.model_id, m.prediction, m.weight, m.variance))
            .collect();

        let total_weight: f32 = active_members.iter().map(|(_, _, w, _)| *w).sum();
        let prediction = if total_weight > 0.0 {
            active_members
                .iter()
                .map(|(_, p, w, _)| p * w)
                .sum::<f32>()
                / total_weight
        } else {
            0.0
        };

        let variance = if total_weight > 0.0 {
            active_members
                .iter()
                .map(|(_, p, w, v)| {
                    let diff = p - prediction;
                    w * (v + diff * diff)
                })
                .sum::<f32>()
                / total_weight
        } else {
            1.0
        };

        let mut contributions: LinearMap<f32, 64> = BTreeMap::new();
        for (id, _, w, _) in &active_members {
            let contrib = if total_weight > 0.0 { w / total_weight } else { 0.0 };
            contributions.insert(*id, contrib);
        }

        let diversity = self.compute_diversity(&active_members);
        let confidence = (1.0 - variance.sqrt().min(1.0)) * 0.8 + diversity * 0.2;

        self.stats.avg_ensemble_size =
            ema_update(self.stats.avg_ensemble_size, active_members.len() as f32);
        self.stats.avg_diversity = ema_update(self.stats.avg_diversity, diversity);

        let result = MasterEnsembleResult {
            prediction,
            variance,
            confidence: confidence.clamp(0.0, 1.0),
            subsystem_contributions: contributions,
            total_weight,
            active_models: active_members.len(),
            diversity_index: diversity,
            timestamp_us,
        };

        if self.fusion_history.len() < MAX_FUSION_HISTORY {
            self.fusion_history.push(result.clone());
        }
        result
    }

    /// Fuse predictions across two subsystem ensembles
    pub fn cross_subsystem_fusion(
        &mut self,
        from: EnsembleSource,
        to: EnsembleSource,
    ) -> CrossSubsystemFusion {
        self.stats.cross_fusions += 1;
        let from_members: Vec<&EnsembleMember> = self
            .members
            .values()
            .filter(|m| m.source == from && m.active)
            .collect();
        let to_members: Vec<&EnsembleMember> = self
            .members
            .values()
            .filter(|m| m.source == to && m.active)
            .collect();

        let from_avg = Self::avg_prediction(&from_members);
        let to_avg = Self::avg_prediction(&to_members);
        let agreement = 1.0 - (from_avg - to_avg).abs();
        let conflict = 1.0 - agreement;

        let resolution = if agreement > 0.9 {
            FusionResolution::WeightedAverage
        } else if agreement > 0.7 {
            FusionResolution::HighestAccuracy
        } else if agreement > 0.5 {
            FusionResolution::MedianVote
        } else if agreement > 0.3 {
            FusionResolution::MostRecent
        } else {
            FusionResolution::ConflictFlagged
        };

        let from_weight = Self::total_weight(&from_members);
        let to_weight = Self::total_weight(&to_members);
        let total = from_weight + to_weight;
        let blended = if total > 0.0 {
            (from_avg * from_weight + to_avg * to_weight) / total
        } else {
            0.0
        };

        CrossSubsystemFusion {
            from_source: from,
            to_source: to,
            agreement,
            conflict,
            resolution,
            blended_prediction: blended,
        }
    }

    /// Build the ensemble hierarchy: leaf → subsystem → master
    pub fn ensemble_hierarchy(&mut self) -> Vec<HierarchyLevel> {
        self.stats.hierarchy_builds += 1;
        let mut levels: Vec<HierarchyLevel> = Vec::new();

        // Level 0: individual models
        levels.push(HierarchyLevel {
            level: 0,
            ensemble_count: self.members.len(),
            total_models: self.members.len(),
            fused_prediction: 0.0,
            fused_variance: 0.0,
            description: String::from("individual-models"),
        });

        // Level 1: subsystem ensembles
        let mut source_groups: BTreeMap<u8, Vec<&EnsembleMember>> = BTreeMap::new();
        for m in self.members.values().filter(|m| m.active) {
            source_groups.entry(m.source as u8).or_insert_with(Vec::new).push(m);
        }
        let subsys_count = source_groups.len();
        let mut subsys_pred = 0.0_f32;
        let mut subsys_var = 0.0_f32;
        for (_k, group) in &source_groups {
            let avg = Self::avg_prediction(group);
            subsys_pred += avg;
            subsys_var += group.iter().map(|m| m.variance).sum::<f32>() / group.len().max(1) as f32;
        }
        if subsys_count > 0 {
            subsys_pred /= subsys_count as f32;
            subsys_var /= subsys_count as f32;
        }
        levels.push(HierarchyLevel {
            level: 1,
            ensemble_count: subsys_count,
            total_models: self.members.values().filter(|m| m.active).count(),
            fused_prediction: subsys_pred,
            fused_variance: subsys_var,
            description: String::from("subsystem-ensembles"),
        });

        // Level 2: master ensemble
        let master_pred = subsys_pred;
        levels.push(HierarchyLevel {
            level: 2,
            ensemble_count: 1,
            total_models: self.members.len(),
            fused_prediction: master_pred,
            fused_variance: subsys_var * 0.7,
            description: String::from("master-ensemble"),
        });

        levels
    }

    /// Dynamic global model selection: adjust weights based on performance
    pub fn global_model_selection(&mut self) -> ModelSelection {
        self.stats.model_selections += 1;
        let mut rankings: Vec<ModelRanking> = Vec::new();
        let mut total_weight = 0.0_f32;
        let mut active = 0_usize;
        let mut pruned = 0_usize;

        let ids: Vec<u64> = self.members.keys().copied().collect();
        for id in &ids {
            if let Some(member) = self.members.get_mut(id) {
                member.weight *= WEIGHT_DECAY;
                member.weight += member.accuracy * 0.02 + member.diversity_score * DIVERSITY_BONUS;
                member.weight = member.weight.max(MIN_MODEL_WEIGHT);
                if member.weight < MIN_MODEL_WEIGHT * 10.0 {
                    member.active = false;
                    pruned += 1;
                } else {
                    active += 1;
                    total_weight += member.weight;
                }
            }
        }

        let mut sorted_members: Vec<(u64, f32, EnsembleSource, f32)> = self
            .members
            .values()
            .filter(|m| m.active)
            .map(|m| (m.model_id, m.weight, m.source, m.accuracy))
            .collect();
        sorted_members.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));

        for (rank, (id, weight, source, accuracy)) in sorted_members.iter().enumerate() {
            let contrib = if total_weight > 0.0 { weight / total_weight } else { 0.0 };
            rankings.push(ModelRanking {
                model_id: *id,
                source: *source,
                weight: *weight,
                rank: rank + 1,
                accuracy: *accuracy,
                contribution: contrib,
            });
        }

        let entropy = self.compute_weight_entropy(total_weight);

        ModelSelection {
            rankings,
            total_weight,
            active_count: active,
            pruned_count: pruned,
            weight_entropy: entropy,
        }
    }

    /// Analyze ensemble dominance: is one model dominating the ensemble?
    pub fn ensemble_dominance(&mut self) -> EnsembleDominance {
        self.stats.dominance_analyses += 1;
        let active: Vec<&EnsembleMember> = self.members.values().filter(|m| m.active).collect();
        let total_weight: f32 = active.iter().map(|m| m.weight).sum();

        let dominant = active
            .iter()
            .max_by(|a, b| a.weight.partial_cmp(&b.weight).unwrap_or(core::cmp::Ordering::Equal));

        let (dom_id, dom_source, dom_weight) = match dominant {
            Some(m) => (m.model_id, m.source, m.weight),
            None => (0, EnsembleSource::BridgePredictor, 0.0),
        };

        let dom_score = if total_weight > 0.0 { dom_weight / total_weight } else { 0.0 };
        let top_k = 3_usize;
        let mut sorted_weights: Vec<f32> = active.iter().map(|m| m.weight).collect();
        sorted_weights.sort_by(|a, b| b.partial_cmp(a).unwrap_or(core::cmp::Ordering::Equal));
        let top_k_share: f32 = sorted_weights.iter().take(top_k).sum::<f32>()
            / total_weight.max(0.001);

        let concentration = sorted_weights
            .iter()
            .map(|w| {
                let share = w / total_weight.max(0.001);
                share * share
            })
            .sum::<f32>();

        let healthy = dom_score < 0.5 && concentration < 0.3;
        let diversity_warning = dom_score > 0.6 || concentration > 0.4;

        self.stats.avg_dominance_score = ema_update(self.stats.avg_dominance_score, dom_score);

        EnsembleDominance {
            dominant_model_id: dom_id,
            dominant_source: dom_source,
            dominance_score: dom_score,
            concentration_index: concentration,
            top_k_weight_share: top_k_share,
            is_healthy: healthy,
            diversity_warning,
        }
    }

    /// Evaluate meta-accuracy: how good is the ensemble?
    pub fn meta_accuracy(&mut self) -> MetaAccuracy {
        self.stats.accuracy_evaluations += 1;
        let mut total_error = 0.0_f32;
        let mut count = 0_u64;
        let mut source_errors: BTreeMap<u64, (f32, u64)> = BTreeMap::new();
        let mut recent_error = 0.0_f32;
        let mut recent_count = 0_usize;

        for (pred_val, actual, source_key) in &self.accuracy_log {
            let err = (pred_val - actual).abs();
            total_error += err;
            count += 1;
            let entry = source_errors.entry(*source_key).or_insert((0.0, 0));
            entry.0 += err;
            entry.1 += 1;
        }

        let recent_window = self.accuracy_log.len().saturating_sub(100);
        for (pred_val, actual, _) in self.accuracy_log.iter().skip(recent_window) {
            recent_error += (pred_val - actual).abs();
            recent_count += 1;
        }

        let overall = if count > 0 { 1.0 - (total_error / count as f32).min(1.0) } else { 0.5 };
        let recent = if recent_count > 0 {
            1.0 - (recent_error / recent_count as f32).min(1.0)
        } else {
            0.5
        };

        let mut accuracy_by_source: LinearMap<f32, 64> = BTreeMap::new();
        for (k, (err, cnt)) in &source_errors {
            let acc = if *cnt > 0 { 1.0 - (err / *cnt as f32).min(1.0) } else { 0.5 };
            accuracy_by_source.insert(*k, acc);
        }

        let best_single = self
            .members
            .values()
            .filter(|m| m.active)
            .map(|m| m.accuracy)
            .fold(0.0_f32, f32::max);
        let improvement = overall - best_single;

        self.stats.avg_meta_accuracy = ema_update(self.stats.avg_meta_accuracy, overall);

        MetaAccuracy {
            overall_accuracy: overall,
            accuracy_by_source,
            recent_accuracy: recent,
            accuracy_trend: recent - overall,
            total_evaluations: count,
            improvement_over_best_single: improvement,
        }
    }

    /// Record an actual outcome for meta-accuracy tracking
    #[inline]
    pub fn record_outcome(&mut self, prediction: f32, actual: f32, source: EnsembleSource) {
        let key = fnv1a_hash(&[source as u8]);
        if self.accuracy_log.len() < MAX_FUSION_HISTORY {
            self.accuracy_log.push((prediction, actual, key));
        }
    }

    /// Get current statistics
    #[inline(always)]
    pub fn stats(&self) -> &EnsembleStats {
        &self.stats
    }

    // ========================================================================
    // PRIVATE HELPERS
    // ========================================================================

    fn avg_prediction(members: &[&EnsembleMember]) -> f32 {
        if members.is_empty() {
            return 0.0;
        }
        members.iter().map(|m| m.prediction).sum::<f32>() / members.len() as f32
    }

    fn total_weight(members: &[&EnsembleMember]) -> f32 {
        members.iter().map(|m| m.weight).sum()
    }

    fn compute_diversity(&self, members: &[(u64, f32, f32, f32)]) -> f32 {
        if members.len() < 2 {
            return 0.0;
        }
        let mean_pred = members.iter().map(|(_, p, _, _)| *p).sum::<f32>() / members.len() as f32;
        let variance = members
            .iter()
            .map(|(_, p, _, _)| (p - mean_pred) * (p - mean_pred))
            .sum::<f32>()
            / members.len() as f32;
        variance.sqrt().min(1.0)
    }

    fn compute_weight_entropy(&self, total_weight: f32) -> f32 {
        if total_weight <= 0.0 {
            return 0.0;
        }
        let mut entropy = 0.0_f32;
        for m in self.members.values().filter(|m| m.active) {
            let p = m.weight / total_weight;
            if p > 0.0 {
                entropy -= p * p.ln();
            }
        }
        entropy.max(0.0)
    }
}
