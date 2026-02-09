// SPDX-License-Identifier: GPL-2.0
//! # Bridge Paradigm — Paradigm Shift Detection
//!
//! When accumulated research evidence overwhelmingly contradicts the current
//! bridge optimization model, a paradigm shift is needed. This module detects
//! when the weight of evidence requires abandoning the old model in favour of
//! a fundamentally new one. It tracks evidence accumulation, compares model
//! fitness, plans the transition from old to new, and maintains a history
//! of all paradigm shifts. Kuhn meets kernel engineering.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_MODELS: usize = 64;
const MAX_EVIDENCE: usize = 2048;
const MAX_SHIFTS: usize = 128;
const EVIDENCE_THRESHOLD: f32 = 0.75;
const MODEL_SUPERIORITY_THRESHOLD: f32 = 0.20;
const TRANSITION_STEPS: usize = 8;
const EVIDENCE_DECAY: f32 = 0.997;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const MIN_EVIDENCE_FOR_SHIFT: usize = 10;
const PARADIGM_AGE_WARNING: u64 = 500;
const MODEL_FITNESS_SAMPLES: usize = 20;
const ANOMALY_WEIGHT: f32 = 1.5;

// ============================================================================
// HELPERS
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

fn xorshift_f32(state: &mut u64) -> f32 {
    (xorshift64(state) % 10000) as f32 / 10000.0
}

fn abs_f32(v: f32) -> f32 {
    if v < 0.0 { -v } else { v }
}

fn sqrt_approx(v: f32) -> f32 {
    if v <= 0.0 {
        return 0.0;
    }
    let mut g = v * 0.5;
    for _ in 0..6 {
        g = 0.5 * (g + v / g);
    }
    g
}

// ============================================================================
// TYPES
// ============================================================================

/// An optimization model (paradigm).
#[derive(Clone)]
struct OptimizationModel {
    id: u64,
    name: String,
    description: String,
    fitness: f32,
    fitness_history: Vec<f32>,
    created_tick: u64,
    anomaly_count: u64,
    total_predictions: u64,
    correct_predictions: u64,
    is_active: bool,
}

/// A piece of evidence for or against a model.
#[derive(Clone)]
struct Evidence {
    id: u64,
    model_id: u64,
    supports: bool,
    weight: f32,
    observation: String,
    tick: u64,
}

/// A paradigm shift event.
#[derive(Clone)]
pub struct ParadigmShift {
    pub id: u64,
    pub old_model: String,
    pub new_model: String,
    pub evidence_weight: f32,
    pub transition_plan: Vec<String>,
    pub old_fitness: f32,
    pub new_fitness: f32,
    pub evidence_count: usize,
    pub tick: u64,
}

/// Evidence accumulation state for a model.
#[derive(Clone)]
pub struct EvidenceState {
    pub model_id: u64,
    pub model_name: String,
    pub supporting_evidence: u64,
    pub contradicting_evidence: u64,
    pub net_evidence_weight: f32,
    pub anomaly_rate: f32,
    pub overall_health: f32,
}

/// Model comparison result.
#[derive(Clone)]
pub struct ModelComparison {
    pub model_a: String,
    pub model_b: String,
    pub fitness_a: f32,
    pub fitness_b: f32,
    pub fitness_difference: f32,
    pub superior_model: String,
    pub evidence_ratio: f32,
    pub shift_recommended: bool,
}

/// Paradigm engine statistics.
#[derive(Clone)]
pub struct ParadigmStats {
    pub total_models: u64,
    pub active_model_age: u64,
    pub total_evidence: u64,
    pub total_shifts: u64,
    pub avg_paradigm_age_ema: f32,
    pub avg_evidence_weight_ema: f32,
    pub current_model_fitness: f32,
    pub anomaly_rate_ema: f32,
    pub shift_frequency_ema: f32,
    pub model_stability: f32,
}

/// Transition plan.
#[derive(Clone)]
pub struct TransitionPlan {
    pub old_model: String,
    pub new_model: String,
    pub steps: Vec<TransitionStep>,
    pub estimated_risk: f32,
    pub estimated_benefit: f32,
    pub rollback_possible: bool,
}

/// A single step in the transition plan.
#[derive(Clone)]
pub struct TransitionStep {
    pub order: usize,
    pub action: String,
    pub risk: f32,
    pub reversible: bool,
}

// ============================================================================
// BRIDGE PARADIGM ENGINE
// ============================================================================

/// Paradigm shift detection and management engine.
pub struct BridgeParadigm {
    models: BTreeMap<u64, OptimizationModel>,
    evidence: Vec<Evidence>,
    shifts: Vec<ParadigmShift>,
    active_model_id: u64,
    stats: ParadigmStats,
    rng_state: u64,
    tick: u64,
}

impl BridgeParadigm {
    /// Create a new paradigm engine with an initial model.
    pub fn new(seed: u64, initial_model_name: &str, initial_model_desc: &str) -> Self {
        let id = fnv1a_hash(initial_model_name.as_bytes());
        let mut models = BTreeMap::new();
        models.insert(
            id,
            OptimizationModel {
                id,
                name: String::from(initial_model_name),
                description: String::from(initial_model_desc),
                fitness: 0.5,
                fitness_history: Vec::new(),
                created_tick: 0,
                anomaly_count: 0,
                total_predictions: 0,
                correct_predictions: 0,
                is_active: true,
            },
        );

        Self {
            models,
            evidence: Vec::new(),
            shifts: Vec::new(),
            active_model_id: id,
            stats: ParadigmStats {
                total_models: 1,
                active_model_age: 0,
                total_evidence: 0,
                total_shifts: 0,
                avg_paradigm_age_ema: 0.0,
                avg_evidence_weight_ema: 0.0,
                current_model_fitness: 0.5,
                anomaly_rate_ema: 0.0,
                shift_frequency_ema: 0.0,
                model_stability: 1.0,
            },
            rng_state: seed ^ 0xDA4AD165010F01,
            tick: 0,
        }
    }

    /// Register a new candidate model.
    pub fn register_model(&mut self, name: &str, description: &str) -> u64 {
        let id = fnv1a_hash(name.as_bytes());
        if self.models.len() >= MAX_MODELS {
            self.evict_worst_model();
        }
        self.models.insert(
            id,
            OptimizationModel {
                id,
                name: String::from(name),
                description: String::from(description),
                fitness: 0.0,
                fitness_history: Vec::new(),
                created_tick: self.tick,
                anomaly_count: 0,
                total_predictions: 0,
                correct_predictions: 0,
                is_active: false,
            },
        );
        self.stats.total_models += 1;
        id
    }

    /// Submit evidence for or against a model.
    pub fn submit_evidence(
        &mut self,
        model_name: &str,
        supports: bool,
        weight: f32,
        observation: &str,
    ) {
        self.tick += 1;
        let model_id = fnv1a_hash(model_name.as_bytes());
        let clamped_weight = weight.max(0.0).min(2.0);

        if self.evidence.len() >= MAX_EVIDENCE {
            self.evidence.remove(0);
        }

        let eid = fnv1a_hash(observation.as_bytes()) ^ self.tick;
        self.evidence.push(Evidence {
            id: eid,
            model_id,
            supports,
            weight: clamped_weight,
            observation: String::from(observation),
            tick: self.tick,
        });

        self.stats.total_evidence += 1;
        self.stats.avg_evidence_weight_ema = self.stats.avg_evidence_weight_ema
            * (1.0 - EMA_ALPHA)
            + clamped_weight * EMA_ALPHA;

        // Update model anomaly tracking
        if !supports {
            if let Some(model) = self.models.get_mut(&model_id) {
                model.anomaly_count += 1;
            }
        }
    }

    /// Record a prediction result for a model.
    pub fn record_prediction(&mut self, model_name: &str, correct: bool) {
        let model_id = fnv1a_hash(model_name.as_bytes());
        if let Some(model) = self.models.get_mut(&model_id) {
            model.total_predictions += 1;
            if correct {
                model.correct_predictions += 1;
            }
            // Update fitness
            let accuracy = model.correct_predictions as f32
                / model.total_predictions.max(1) as f32;
            model.fitness = model.fitness * (1.0 - EMA_ALPHA) + accuracy * EMA_ALPHA;
            if model.fitness_history.len() < MODEL_FITNESS_SAMPLES * 4 {
                model.fitness_history.push(model.fitness);
            }
        }
    }

    /// Check if a paradigm shift is needed.
    pub fn detect_paradigm_shift(&mut self) -> Option<ParadigmShift> {
        self.tick += 1;
        self.stats.active_model_age += 1;

        let active_fitness = self
            .models
            .get(&self.active_model_id)
            .map(|m| m.fitness)
            .unwrap_or(0.0);
        self.stats.current_model_fitness = active_fitness;

        // Find the best alternative model
        let mut best_alt_id: u64 = 0;
        let mut best_alt_fitness: f32 = 0.0;
        for (&mid, model) in &self.models {
            if mid != self.active_model_id && model.fitness > best_alt_fitness {
                best_alt_fitness = model.fitness;
                best_alt_id = mid;
            }
        }

        // Check evidence accumulation
        let evidence_state = self.evidence_accumulation_for(self.active_model_id);
        let anomaly_rate = evidence_state.anomaly_rate;
        self.stats.anomaly_rate_ema =
            self.stats.anomaly_rate_ema * (1.0 - EMA_ALPHA) + anomaly_rate * EMA_ALPHA;

        // Paradigm shift conditions:
        // 1. Alternative model is significantly superior
        // 2. Evidence against current model exceeds threshold
        // 3. Sufficient evidence has been gathered
        let fitness_gap = best_alt_fitness - active_fitness;
        let evidence_against = evidence_state.contradicting_evidence as f32
            / (evidence_state.supporting_evidence + evidence_state.contradicting_evidence).max(1)
                as f32;
        let enough_evidence = self
            .evidence
            .iter()
            .filter(|e| e.model_id == self.active_model_id)
            .count()
            >= MIN_EVIDENCE_FOR_SHIFT;

        let should_shift = fitness_gap >= MODEL_SUPERIORITY_THRESHOLD
            && evidence_against >= EVIDENCE_THRESHOLD
            && enough_evidence;

        if should_shift && best_alt_id != 0 {
            let old_name = self
                .models
                .get(&self.active_model_id)
                .map(|m| m.name.clone())
                .unwrap_or_else(|| String::from("unknown"));
            let new_name = self
                .models
                .get(&best_alt_id)
                .map(|m| m.name.clone())
                .unwrap_or_else(|| String::from("unknown"));

            let plan = self.build_transition_steps(&old_name, &new_name);

            let shift = ParadigmShift {
                id: self.tick,
                old_model: old_name,
                new_model: new_name,
                evidence_weight: evidence_state.net_evidence_weight,
                transition_plan: plan,
                old_fitness: active_fitness,
                new_fitness: best_alt_fitness,
                evidence_count: self
                    .evidence
                    .iter()
                    .filter(|e| e.model_id == self.active_model_id)
                    .count(),
                tick: self.tick,
            };

            // Execute shift
            if let Some(old_model) = self.models.get_mut(&self.active_model_id) {
                old_model.is_active = false;
            }
            if let Some(new_model) = self.models.get_mut(&best_alt_id) {
                new_model.is_active = true;
            }
            self.active_model_id = best_alt_id;

            if self.shifts.len() < MAX_SHIFTS {
                self.shifts.push(shift.clone());
            }

            self.stats.total_shifts += 1;
            self.stats.avg_paradigm_age_ema = self.stats.avg_paradigm_age_ema
                * (1.0 - EMA_ALPHA)
                + self.stats.active_model_age as f32 * EMA_ALPHA;
            self.stats.active_model_age = 0;
            self.stats.shift_frequency_ema =
                self.stats.shift_frequency_ema * (1.0 - EMA_ALPHA) + 1.0 * EMA_ALPHA;
            self.stats.model_stability =
                (self.stats.model_stability * (1.0 - EMA_ALPHA)).max(0.0);

            Some(shift)
        } else {
            self.stats.model_stability =
                (self.stats.model_stability * (1.0 - EMA_ALPHA) + 1.0 * EMA_ALPHA).min(1.0);
            self.stats.shift_frequency_ema *= 1.0 - EMA_ALPHA;
            None
        }
    }

    /// Get the evidence accumulation state for the active model.
    pub fn evidence_accumulation(&self) -> EvidenceState {
        self.evidence_accumulation_for(self.active_model_id)
    }

    /// Compare two models.
    pub fn model_comparison(&self, model_a: &str, model_b: &str) -> ModelComparison {
        let id_a = fnv1a_hash(model_a.as_bytes());
        let id_b = fnv1a_hash(model_b.as_bytes());
        let fit_a = self.models.get(&id_a).map(|m| m.fitness).unwrap_or(0.0);
        let fit_b = self.models.get(&id_b).map(|m| m.fitness).unwrap_or(0.0);
        let diff = fit_a - fit_b;

        let ev_a = self.evidence.iter().filter(|e| e.model_id == id_a && e.supports).count();
        let ev_b = self.evidence.iter().filter(|e| e.model_id == id_b && e.supports).count();
        let ev_ratio = ev_a as f32 / (ev_a + ev_b).max(1) as f32;

        let superior = if diff > 0.0 {
            String::from(model_a)
        } else {
            String::from(model_b)
        };

        ModelComparison {
            model_a: String::from(model_a),
            model_b: String::from(model_b),
            fitness_a: fit_a,
            fitness_b: fit_b,
            fitness_difference: abs_f32(diff),
            superior_model: superior,
            evidence_ratio: ev_ratio,
            shift_recommended: abs_f32(diff) >= MODEL_SUPERIORITY_THRESHOLD,
        }
    }

    /// Generate a transition plan from one model to another.
    pub fn transition_plan(&self, old_name: &str, new_name: &str) -> TransitionPlan {
        let steps = self.build_transition_plan_detailed(old_name, new_name);
        let total_risk: f32 = steps.iter().map(|s| s.risk).sum::<f32>() / steps.len().max(1) as f32;
        let old_fit = self
            .models
            .get(&fnv1a_hash(old_name.as_bytes()))
            .map(|m| m.fitness)
            .unwrap_or(0.0);
        let new_fit = self
            .models
            .get(&fnv1a_hash(new_name.as_bytes()))
            .map(|m| m.fitness)
            .unwrap_or(0.0);

        TransitionPlan {
            old_model: String::from(old_name),
            new_model: String::from(new_name),
            steps,
            estimated_risk: total_risk,
            estimated_benefit: (new_fit - old_fit).max(0.0),
            rollback_possible: true,
        }
    }

    /// Get the age of the current paradigm (ticks since last shift).
    pub fn paradigm_age(&self) -> u64 {
        self.stats.active_model_age
    }

    /// Get the history of all paradigm shifts.
    pub fn shift_history(&self) -> &[ParadigmShift] {
        &self.shifts
    }

    /// Get stats.
    pub fn stats(&self) -> &ParadigmStats {
        &self.stats
    }

    /// Number of registered models.
    pub fn model_count(&self) -> usize {
        self.models.len()
    }

    /// Name of the active model.
    pub fn active_model_name(&self) -> String {
        self.models
            .get(&self.active_model_id)
            .map(|m| m.name.clone())
            .unwrap_or_else(|| String::from("none"))
    }

    // ========================================================================
    // INTERNAL HELPERS
    // ========================================================================

    fn evidence_accumulation_for(&self, model_id: u64) -> EvidenceState {
        let model_evidence: Vec<&Evidence> =
            self.evidence.iter().filter(|e| e.model_id == model_id).collect();
        let supporting = model_evidence.iter().filter(|e| e.supports).count() as u64;
        let contradicting = model_evidence.iter().filter(|e| !e.supports).count() as u64;
        let net_weight: f32 = model_evidence
            .iter()
            .map(|e| if e.supports { e.weight } else { -e.weight * ANOMALY_WEIGHT })
            .sum();

        let total_preds = self
            .models
            .get(&model_id)
            .map(|m| m.total_predictions)
            .unwrap_or(0);
        let anomalies = self
            .models
            .get(&model_id)
            .map(|m| m.anomaly_count)
            .unwrap_or(0);
        let anomaly_rate = if total_preds > 0 {
            anomalies as f32 / total_preds as f32
        } else {
            contradicting as f32 / (supporting + contradicting).max(1) as f32
        };

        let fitness = self.models.get(&model_id).map(|m| m.fitness).unwrap_or(0.0);
        let health = fitness * 0.5 + (1.0 - anomaly_rate) * 0.3
            + (supporting as f32 / (supporting + contradicting).max(1) as f32) * 0.2;

        let name = self
            .models
            .get(&model_id)
            .map(|m| m.name.clone())
            .unwrap_or_else(|| String::from("unknown"));

        EvidenceState {
            model_id,
            model_name: name,
            supporting_evidence: supporting,
            contradicting_evidence: contradicting,
            net_evidence_weight: net_weight,
            anomaly_rate,
            overall_health: health.max(0.0).min(1.0),
        }
    }

    fn build_transition_steps(&self, old: &str, new: &str) -> Vec<String> {
        let mut steps = Vec::new();
        steps.push(String::from("1. Snapshot current model state"));
        steps.push(String::from("2. Enable shadow-mode for new model"));
        steps.push(String::from("3. Run parallel predictions on 10% traffic"));
        steps.push(String::from("4. Compare prediction accuracy"));
        steps.push(String::from("5. Gradually increase new model traffic to 50%"));
        steps.push(String::from("6. Validate no regression in bridge latency"));
        steps.push(String::from("7. Complete cutover to new model"));
        steps.push(String::from("8. Retain old model for emergency rollback"));
        steps
    }

    fn build_transition_plan_detailed(&self, old: &str, new: &str) -> Vec<TransitionStep> {
        let mut steps = Vec::new();
        let actions = [
            ("Snapshot current state and create rollback point", 0.05, true),
            ("Enable shadow-mode for new model", 0.10, true),
            ("Route 10% traffic to new model", 0.15, true),
            ("Validate prediction accuracy matches expectations", 0.10, true),
            ("Increase to 50% traffic split", 0.25, true),
            ("Monitor for performance regression", 0.05, true),
            ("Complete cutover to new model", 0.30, false),
            ("Retain old model for 100 ticks as rollback", 0.05, true),
        ];
        for (i, (action, risk, rev)) in actions.iter().enumerate() {
            steps.push(TransitionStep {
                order: i + 1,
                action: String::from(*action),
                risk: *risk,
                reversible: *rev,
            });
        }
        steps
    }

    fn evict_worst_model(&mut self) {
        let worst = self
            .models
            .values()
            .filter(|m| !m.is_active)
            .min_by(|a, b| {
                a.fitness
                    .partial_cmp(&b.fitness)
                    .unwrap_or(core::cmp::Ordering::Equal)
            })
            .map(|m| m.id);
        if let Some(wid) = worst {
            self.models.remove(&wid);
        }
    }
}
