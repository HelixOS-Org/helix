// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Causal Forecast
//!
//! Causal prediction of cooperation dynamics. Answers questions like
//! "Why does trust change?" and "What causes contention?" by maintaining
//! a causal graph of cooperation factors, effects, and intervention points.

extern crate alloc;

use alloc::collections::BTreeMap;
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

/// Mechanism through which a cause produces an effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CausalMechanism {
    DirectResource,
    TrustMediated,
    ContentionCascade,
    SchedulePressure,
    FairnessViolation,
    ReputationDecay,
    StarvationFeedback,
    CooperationBreakdown,
}

/// A single causal factor linking an event to its effect.
#[derive(Clone, Debug)]
pub struct CausalFactor {
    pub factor_id: u64,
    pub cause_event: u64,
    pub effect_event: u64,
    pub mechanism: CausalMechanism,
    pub strength: u64,
    pub confidence: u64,
    pub lag_ticks: u64,
    pub observation_count: u64,
}

/// Edge in the causal graph connecting two factors.
#[derive(Clone, Debug)]
pub struct CausalEdge {
    pub from_factor: u64,
    pub to_factor: u64,
    pub weight: u64,
    pub mechanism: CausalMechanism,
}

/// Result of a trust prediction via causal model.
#[derive(Clone, Debug)]
pub struct TrustCausalPrediction {
    pub partner_id: u64,
    pub predicted_trust: u64,
    pub primary_cause: u64,
    pub mechanism: CausalMechanism,
    pub causal_chain_length: u32,
    pub confidence: u64,
}

/// Root cause analysis result for contention.
#[derive(Clone, Debug)]
pub struct ContentionRootCause {
    pub resource_id: u64,
    pub root_event: u64,
    pub causal_path: Vec<u64>,
    pub mechanism_chain: Vec<CausalMechanism>,
    pub intervention_point: u64,
    pub severity: u64,
}

/// Result of an intervention effect analysis.
#[derive(Clone, Debug)]
pub struct InterventionEffect {
    pub intervention_id: u64,
    pub target_factor: u64,
    pub predicted_delta: i64,
    pub side_effects: Vec<(u64, i64)>,
    pub confidence: u64,
    pub cost_estimate: u64,
}

/// Causal explanation for a cooperation outcome.
#[derive(Clone, Debug)]
pub struct CausalExplanation {
    pub outcome_id: u64,
    pub explanation_chain: Vec<CausalFactor>,
    pub primary_mechanism: CausalMechanism,
    pub counterfactual_delta: i64,
    pub explanation_score: u64,
}

/// Cooperation forecast result from causal model.
#[derive(Clone, Debug)]
pub struct CooperationForecastResult {
    pub horizon_ticks: u64,
    pub predicted_cooperation_level: u64,
    pub trust_forecast: u64,
    pub contention_forecast: u64,
    pub dominant_mechanism: CausalMechanism,
    pub confidence: u64,
}

/// Rolling statistics for the causal forecast engine.
#[derive(Clone, Debug)]
pub struct CausalForecastStats {
    pub factors_recorded: u64,
    pub trust_predictions: u64,
    pub root_causes_found: u64,
    pub interventions_analyzed: u64,
    pub explanations_generated: u64,
    pub forecasts_produced: u64,
    pub avg_chain_length: u64,
    pub avg_confidence: u64,
}

impl CausalForecastStats {
    pub fn new() -> Self {
        Self {
            factors_recorded: 0,
            trust_predictions: 0,
            root_causes_found: 0,
            interventions_analyzed: 0,
            explanations_generated: 0,
            forecasts_produced: 0,
            avg_chain_length: 0,
            avg_confidence: 500,
        }
    }
}

/// Internal observation record for causal inference.
#[derive(Clone, Debug)]
struct ObservationRecord {
    event_id: u64,
    tick: u64,
    value: u64,
    preceding_events: Vec<u64>,
}

/// Internal state record for a causal variable.
#[derive(Clone, Debug)]
struct CausalVariable {
    variable_id: u64,
    ema_value: u64,
    history: Vec<u64>,
    incoming_edges: Vec<u64>,
    outgoing_edges: Vec<u64>,
}

/// Causal prediction engine for cooperation dynamics.
pub struct CoopCausalForecast {
    factors: BTreeMap<u64, CausalFactor>,
    edges: BTreeMap<u64, CausalEdge>,
    variables: BTreeMap<u64, CausalVariable>,
    observations: BTreeMap<u64, Vec<ObservationRecord>>,
    stats: CausalForecastStats,
    rng_state: u64,
    current_tick: u64,
    max_history: usize,
    min_observations: u64,
}

impl CoopCausalForecast {
    /// Create a new causal forecast engine.
    pub fn new(seed: u64) -> Self {
        Self {
            factors: BTreeMap::new(),
            edges: BTreeMap::new(),
            variables: BTreeMap::new(),
            observations: BTreeMap::new(),
            stats: CausalForecastStats::new(),
            rng_state: seed ^ 0xCAU5_A1F0_RECA_5700,
            current_tick: 0,
            max_history: 128,
            min_observations: 3,
        }
    }

    /// Record an observation for causal inference.
    pub fn record_observation(&mut self, event_id: u64, value: u64, preceding: &[u64]) {
        let obs = ObservationRecord {
            event_id,
            tick: self.current_tick,
            value,
            preceding_events: preceding.to_vec(),
        };

        let records = self.observations.entry(event_id).or_insert_with(Vec::new);
        records.push(obs);
        if records.len() > self.max_history {
            records.remove(0);
        }

        let var = self.variables.entry(event_id).or_insert_with(|| CausalVariable {
            variable_id: event_id,
            ema_value: value,
            history: Vec::new(),
            incoming_edges: Vec::new(),
            outgoing_edges: Vec::new(),
        });
        var.ema_value = ema_update(var.ema_value, value, 200, 1000);
        var.history.push(value);
        if var.history.len() > self.max_history {
            var.history.remove(0);
        }

        for &pred in preceding {
            self.infer_causal_link(pred, event_id, value);
        }
    }

    /// Predict trust change via causal model.
    pub fn causal_trust_predict(&mut self, partner_id: u64) -> TrustCausalPrediction {
        let trust_var_id = fnv1a_hash(&[b"trust_", &partner_id.to_le_bytes()[..]].concat());

        let (predicted, cause, mechanism, chain_len) = self.trace_causal_effect(trust_var_id);

        let confidence = self.compute_prediction_confidence(trust_var_id);

        self.stats.trust_predictions = self.stats.trust_predictions.saturating_add(1);
        self.stats.avg_confidence = ema_update(self.stats.avg_confidence, confidence, 150, 1000);

        TrustCausalPrediction {
            partner_id,
            predicted_trust: predicted,
            primary_cause: cause,
            mechanism,
            causal_chain_length: chain_len,
            confidence,
        }
    }

    /// Find root cause of contention for a resource.
    pub fn contention_root_cause(&mut self, resource_id: u64) -> ContentionRootCause {
        let contention_var = fnv1a_hash(&[b"cont_", &resource_id.to_le_bytes()[..]].concat());

        let mut causal_path: Vec<u64> = Vec::new();
        let mut mechanism_chain: Vec<CausalMechanism> = Vec::new();
        let mut current = contention_var;
        let mut visited: BTreeMap<u64, bool> = BTreeMap::new();
        let mut root_event = contention_var;

        for _ in 0..16 {
            if visited.contains_key(&current) {
                break;
            }
            visited.insert(current, true);
            causal_path.push(current);

            let incoming: Vec<(u64, CausalMechanism, u64)> = self.factors.values()
                .filter(|f| f.effect_event == current)
                .map(|f| (f.cause_event, f.mechanism.clone(), f.strength))
                .collect();

            if incoming.is_empty() {
                root_event = current;
                break;
            }

            let strongest = incoming.iter()
                .max_by_key(|&(_, _, s)| *s)
                .cloned();

            if let Some((cause, mech, _)) = strongest {
                mechanism_chain.push(mech);
                root_event = cause;
                current = cause;
            } else {
                break;
            }
        }

        let intervention_point = causal_path.get(causal_path.len() / 2).copied().unwrap_or(root_event);

        let severity = self.variables.get(&contention_var)
            .map(|v| v.ema_value)
            .unwrap_or(500);

        self.stats.root_causes_found = self.stats.root_causes_found.saturating_add(1);
        self.stats.avg_chain_length = ema_update(
            self.stats.avg_chain_length,
            causal_path.len() as u64,
            200,
            1000,
        );

        ContentionRootCause {
            resource_id,
            root_event,
            causal_path,
            mechanism_chain,
            intervention_point,
            severity,
        }
    }

    /// Build and return the cooperation causal graph as a list of edges.
    pub fn cooperation_causal_graph(&self) -> Vec<CausalEdge> {
        self.edges.values().cloned().collect()
    }

    /// Analyze the predicted effect of an intervention.
    pub fn intervention_effect(
        &mut self,
        target_factor: u64,
        intervention_magnitude: i64,
    ) -> InterventionEffect {
        let intervention_id = fnv1a_hash(&[
            target_factor.to_le_bytes().as_slice(),
            intervention_magnitude.to_le_bytes().as_slice(),
            self.current_tick.to_le_bytes().as_slice(),
        ].concat());

        let base_value = self.variables.get(&target_factor)
            .map(|v| v.ema_value)
            .unwrap_or(500);

        let predicted_delta = intervention_magnitude;

        let mut side_effects: Vec<(u64, i64)> = Vec::new();
        let downstream: Vec<(u64, u64)> = self.factors.values()
            .filter(|f| f.cause_event == target_factor)
            .map(|f| (f.effect_event, f.strength))
            .collect();

        for (effect_id, strength) in &downstream {
            let propagated = intervention_magnitude
                .saturating_mul(*strength as i64) / 1000;
            if propagated.unsigned_abs() > 10 {
                side_effects.push((*effect_id, propagated));
            }
        }

        let confidence = if self.observations.get(&target_factor)
            .map(|o| o.len() as u64)
            .unwrap_or(0) >= self.min_observations
        {
            700
        } else {
            400
        };

        let cost_estimate = intervention_magnitude.unsigned_abs()
            .saturating_mul(base_value) / 1000;

        self.stats.interventions_analyzed = self.stats.interventions_analyzed.saturating_add(1);

        InterventionEffect {
            intervention_id,
            target_factor,
            predicted_delta,
            side_effects,
            confidence,
            cost_estimate,
        }
    }

    /// Generate a causal explanation for a cooperation outcome.
    pub fn causal_explanation(&mut self, outcome_id: u64) -> CausalExplanation {
        let mut chain: Vec<CausalFactor> = Vec::new();
        let mut current = outcome_id;
        let mut visited: BTreeMap<u64, bool> = BTreeMap::new();
        let mut primary_mech = CausalMechanism::DirectResource;

        for _ in 0..12 {
            if visited.contains_key(&current) {
                break;
            }
            visited.insert(current, true);

            let incoming: Vec<&CausalFactor> = self.factors.values()
                .filter(|f| f.effect_event == current)
                .collect();

            if incoming.is_empty() {
                break;
            }

            let strongest = incoming.iter()
                .max_by_key(|f| f.strength)
                .cloned();

            if let Some(factor) = strongest {
                if chain.is_empty() {
                    primary_mech = factor.mechanism.clone();
                }
                chain.push(factor.clone());
                current = factor.cause_event;
            } else {
                break;
            }
        }

        let counterfactual_delta = chain.iter()
            .map(|f| f.strength as i64)
            .sum::<i64>()
            .saturating_neg();

        let explanation_score = if chain.is_empty() {
            200
        } else {
            let avg_conf: u64 = chain.iter().map(|f| f.confidence).sum::<u64>()
                / chain.len().max(1) as u64;
            avg_conf.min(1000)
        };

        self.stats.explanations_generated = self.stats.explanations_generated.saturating_add(1);

        CausalExplanation {
            outcome_id,
            explanation_chain: chain,
            primary_mechanism: primary_mech,
            counterfactual_delta,
            explanation_score,
        }
    }

    /// Produce a cooperation forecast for a given horizon.
    pub fn forecast_cooperation(&mut self, horizon_ticks: u64) -> CooperationForecastResult {
        let coop_var = fnv1a_hash(b"global_cooperation");
        let trust_var = fnv1a_hash(b"global_trust");
        let cont_var = fnv1a_hash(b"global_contention");

        let coop_level = self.project_variable(coop_var, horizon_ticks);
        let trust_level = self.project_variable(trust_var, horizon_ticks);
        let contention_level = self.project_variable(cont_var, horizon_ticks);

        let dominant = self.find_dominant_mechanism(coop_var);

        let confidence = self.compute_forecast_confidence(horizon_ticks);

        self.stats.forecasts_produced = self.stats.forecasts_produced.saturating_add(1);

        CooperationForecastResult {
            horizon_ticks,
            predicted_cooperation_level: coop_level,
            trust_forecast: trust_level,
            contention_forecast: contention_level,
            dominant_mechanism: dominant,
            confidence,
        }
    }

    /// Advance the internal tick.
    pub fn tick(&mut self) {
        self.current_tick = self.current_tick.wrapping_add(1);
    }

    /// Retrieve current statistics.
    pub fn stats(&self) -> &CausalForecastStats {
        &self.stats
    }

    // ── Private helpers ──────────────────────────────────────────────

    fn infer_causal_link(&mut self, cause: u64, effect: u64, effect_value: u64) {
        let factor_id = fnv1a_hash(&[
            cause.to_le_bytes().as_slice(),
            effect.to_le_bytes().as_slice(),
        ].concat());

        let mechanism = self.infer_mechanism(cause, effect);

        let factor = self.factors.entry(factor_id).or_insert_with(|| CausalFactor {
            factor_id,
            cause_event: cause,
            effect_event: effect,
            mechanism: mechanism.clone(),
            strength: 0,
            confidence: 0,
            lag_ticks: 0,
            observation_count: 0,
        });
        factor.observation_count = factor.observation_count.saturating_add(1);
        factor.strength = ema_update(factor.strength, effect_value, 200, 1000);
        factor.confidence = (factor.observation_count.min(20) * 50).min(1000);

        let edge_id = factor_id;
        self.edges.entry(edge_id).or_insert_with(|| CausalEdge {
            from_factor: cause,
            to_factor: effect,
            weight: factor.strength,
            mechanism,
        });

        if let Some(v) = self.variables.get_mut(&cause) {
            if !v.outgoing_edges.contains(&factor_id) {
                v.outgoing_edges.push(factor_id);
            }
        }
        if let Some(v) = self.variables.get_mut(&effect) {
            if !v.incoming_edges.contains(&factor_id) {
                v.incoming_edges.push(factor_id);
            }
        }

        self.stats.factors_recorded = self.stats.factors_recorded.saturating_add(1);
    }

    fn infer_mechanism(&self, cause: u64, effect: u64) -> CausalMechanism {
        let combined = cause ^ effect;
        match combined % 8 {
            0 => CausalMechanism::DirectResource,
            1 => CausalMechanism::TrustMediated,
            2 => CausalMechanism::ContentionCascade,
            3 => CausalMechanism::SchedulePressure,
            4 => CausalMechanism::FairnessViolation,
            5 => CausalMechanism::ReputationDecay,
            6 => CausalMechanism::StarvationFeedback,
            _ => CausalMechanism::CooperationBreakdown,
        }
    }

    fn trace_causal_effect(&self, target: u64) -> (u64, u64, CausalMechanism, u32) {
        let base = self.variables.get(&target).map(|v| v.ema_value).unwrap_or(500);

        let incoming: Vec<&CausalFactor> = self.factors.values()
            .filter(|f| f.effect_event == target)
            .collect();

        if incoming.is_empty() {
            return (base, 0, CausalMechanism::DirectResource, 0);
        }

        let strongest = incoming.iter()
            .max_by_key(|f| f.strength)
            .unwrap();

        let delta = strongest.strength.saturating_mul(200) / 1000;
        let predicted = if strongest.mechanism == CausalMechanism::ReputationDecay
            || strongest.mechanism == CausalMechanism::StarvationFeedback
        {
            base.saturating_sub(delta)
        } else {
            base.saturating_add(delta).min(1000)
        };

        (predicted, strongest.cause_event, strongest.mechanism.clone(), 1)
    }

    fn compute_prediction_confidence(&self, var_id: u64) -> u64 {
        let obs_count = self.observations.get(&var_id)
            .map(|o| o.len() as u64)
            .unwrap_or(0);
        let factor_count = self.factors.values()
            .filter(|f| f.effect_event == var_id)
            .count() as u64;

        let obs_conf = (obs_count.min(20) * 30).min(600);
        let factor_conf = (factor_count.min(10) * 40).min(400);

        obs_conf.saturating_add(factor_conf).min(1000)
    }

    fn project_variable(&self, var_id: u64, horizon: u64) -> u64 {
        let var = match self.variables.get(&var_id) {
            Some(v) => v,
            None => return 500,
        };

        let trend = if var.history.len() >= 2 {
            let recent = var.history[var.history.len() - 1];
            let older = var.history[var.history.len() - 2];
            recent as i64 - older as i64
        } else {
            0
        };

        let projected = var.ema_value as i64 + trend * (horizon as i64).min(10);
        (projected.max(0) as u64).min(1000)
    }

    fn find_dominant_mechanism(&self, var_id: u64) -> CausalMechanism {
        let incoming: Vec<&CausalFactor> = self.factors.values()
            .filter(|f| f.effect_event == var_id)
            .collect();

        incoming.iter()
            .max_by_key(|f| f.strength)
            .map(|f| f.mechanism.clone())
            .unwrap_or(CausalMechanism::DirectResource)
    }

    fn compute_forecast_confidence(&self, horizon: u64) -> u64 {
        let base_conf: u64 = 800;
        let decay_per_tick: u64 = 15;
        base_conf.saturating_sub(horizon.saturating_mul(decay_per_tick)).max(100)
    }
}
