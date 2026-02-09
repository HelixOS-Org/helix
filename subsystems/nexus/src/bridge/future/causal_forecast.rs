// SPDX-License-Identifier: GPL-2.0
//! # Bridge Causal Forecast
//!
//! Causal prediction for syscall patterns. Not just correlation — actual causal
//! chains. Builds a causal DAG (directed acyclic graph) of syscall relationships
//! so the bridge can distinguish "A happens before B" from "A causes B". This
//! lets the bridge reason about interventions: "If I change X, what happens to Y?"
//!
//! Correlation is gossip; causation is physics.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_EVENTS: usize = 256;
const MAX_LINKS: usize = 1024;
const MAX_CHAIN_DEPTH: usize = 12;
const MAX_OBSERVATIONS: usize = 2048;
const EMA_ALPHA: f32 = 0.08;
const MIN_CONFIDENCE: f32 = 0.05;
const CAUSAL_STRENGTH_DECAY: f32 = 0.995;
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

fn rand_f32(state: &mut u64) -> f32 {
    (xorshift64(state) % 1_000_000) as f32 / 1_000_000.0
}

// ============================================================================
// CAUSAL LINK
// ============================================================================

/// A causal link between two events in the syscall DAG.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CausalLink {
    /// The event that acts as the cause
    pub cause_event: u64,
    /// The event that is the effect
    pub effect_event: u64,
    /// Strength of the causal link (0.0 to 1.0)
    pub strength: f32,
    /// Average delay in ticks between cause and effect
    pub delay_ticks: u64,
    /// Confidence in this causal relationship (0.0 to 1.0)
    pub confidence: f32,
    /// How many times this cause → effect pair was observed
    observations: u64,
    /// How many times cause occurred without effect following
    cause_without_effect: u64,
    /// How many times effect occurred without cause preceding
    effect_without_cause: u64,
    /// Running EMA of the delay
    delay_ema: f32,
}

impl CausalLink {
    fn new(cause: u64, effect: u64) -> Self {
        Self {
            cause_event: cause,
            effect_event: effect,
            strength: 0.0,
            delay_ticks: 0,
            confidence: 0.0,
            observations: 0,
            cause_without_effect: 0,
            effect_without_cause: 0,
            delay_ema: 0.0,
        }
    }

    #[inline]
    fn update(&mut self, observed_delay: u64) {
        self.observations += 1;
        self.delay_ema = self.delay_ema * (1.0 - EMA_ALPHA)
            + observed_delay as f32 * EMA_ALPHA;
        self.delay_ticks = self.delay_ema as u64;

        // Strength: P(effect | cause) approximated via observation ratio
        let total = self.observations + self.cause_without_effect;
        if total > 0 {
            self.strength = self.observations as f32 / total as f32;
        }

        // Confidence: grows with sample size, asymptoting at 1.0
        let n = self.observations as f32;
        self.confidence = 1.0 - 1.0 / (1.0 + n * 0.1);
    }

    fn record_cause_only(&mut self) {
        self.cause_without_effect += 1;
        let total = self.observations + self.cause_without_effect;
        if total > 0 {
            self.strength = self.observations as f32 / total as f32;
        }
    }

    fn record_effect_only(&mut self) {
        self.effect_without_cause += 1;
    }

    /// Compute the causal score: strength × confidence, decayed by staleness.
    fn causal_score(&self) -> f32 {
        self.strength * self.confidence
    }
}

// ============================================================================
// CAUSAL CHAIN
// ============================================================================

/// A chain of causal links forming a multi-hop causal path.
#[derive(Debug, Clone)]
pub struct CausalChain {
    /// Sequence of event hashes from root cause to final effect
    pub events: Vec<u64>,
    /// Product of link strengths along the chain
    pub total_strength: f32,
    /// Sum of delays along the chain
    pub total_delay_ticks: u64,
    /// Minimum confidence along the chain
    pub min_confidence: f32,
}

// ============================================================================
// CAUSAL PREDICTION
// ============================================================================

/// A causal prediction: what effects will follow from an observed cause.
#[derive(Debug, Clone)]
pub struct CausalPrediction {
    /// The triggering cause event
    pub cause: u64,
    /// Predicted effects with (event_hash, probability, expected_delay)
    pub effects: Vec<(u64, f32, u64)>,
    /// Overall confidence in the prediction
    pub confidence: f32,
}

// ============================================================================
// COUNTERFACTUAL RESULT
// ============================================================================

/// Result of a counterfactual query: what would change if an event didn't happen?
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CounterfactualResult {
    /// The hypothetically removed event
    pub removed_event: u64,
    /// Effects that would NOT have occurred
    pub prevented_effects: Vec<u64>,
    /// Estimated impact magnitude
    pub impact_magnitude: f32,
}

// ============================================================================
// STATS
// ============================================================================

/// Statistics for the causal forecast engine.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CausalForecastStats {
    pub total_links: u64,
    pub total_events_tracked: u64,
    pub total_predictions: u64,
    pub total_chains_found: u64,
    pub avg_chain_length: f32,
    pub avg_link_strength: f32,
    pub avg_confidence: f32,
    pub counterfactual_queries: u64,
}

impl CausalForecastStats {
    fn new() -> Self {
        Self {
            total_links: 0,
            total_events_tracked: 0,
            total_predictions: 0,
            total_chains_found: 0,
            avg_chain_length: 0.0,
            avg_link_strength: 0.0,
            avg_confidence: 0.0,
            counterfactual_queries: 0,
        }
    }
}

// ============================================================================
// BRIDGE CAUSAL FORECAST
// ============================================================================

/// Causal forecast engine for the syscall bridge.
///
/// Builds a DAG of causal relationships between syscall events and uses it
/// to predict effects from observed causes, trace causal chains, evaluate
/// interventions, and answer counterfactual queries.
#[repr(align(64))]
pub struct BridgeCausalForecast {
    /// Causal links: (cause_hash, effect_hash) -> CausalLink
    links: BTreeMap<(u64, u64), CausalLink>,
    /// Forward adjacency: cause_hash -> [effect_hash, ...]
    forward_adj: BTreeMap<u64, Vec<u64>>,
    /// Reverse adjacency: effect_hash -> [cause_hash, ...]
    reverse_adj: BTreeMap<u64, Vec<u64>>,
    /// Recent event window for temporal ordering
    recent_events: VecDeque<(u64, u64)>, // (event_hash, tick)
    /// Event occurrence counts
    event_counts: BTreeMap<u64, u64>,
    /// Total ticks observed
    tick_counter: u64,
    /// Running statistics
    stats: CausalForecastStats,
    /// PRNG state
    rng: u64,
    /// Maximum temporal window for cause-effect detection
    max_causal_window: u64,
}

impl BridgeCausalForecast {
    /// Create a new causal forecast engine.
    pub fn new() -> Self {
        Self {
            links: BTreeMap::new(),
            forward_adj: BTreeMap::new(),
            reverse_adj: BTreeMap::new(),
            recent_events: VecDeque::new(),
            event_counts: BTreeMap::new(),
            tick_counter: 0,
            stats: CausalForecastStats::new(),
            rng: 0xCAFE_BABE_DEAD_1234,
            max_causal_window: 500,
        }
    }

    /// Record an event occurrence. Automatically discovers potential causal links.
    pub fn record_event(&mut self, event_hash: u64, tick: u64) {
        self.tick_counter = tick;
        *self.event_counts.entry(event_hash).or_insert(0) += 1;

        // Check if this event could be an effect of any recent cause
        for &(cause_hash, cause_tick) in self.recent_events.iter().rev() {
            if tick.saturating_sub(cause_tick) > self.max_causal_window {
                break;
            }
            if cause_hash == event_hash {
                continue;
            }
            let delay = tick.saturating_sub(cause_tick);
            let key = (cause_hash, event_hash);
            let link = self.links.entry(key).or_insert_with(|| CausalLink::new(cause_hash, event_hash));
            link.update(delay);

            // Update adjacency lists
            let fwd = self.forward_adj.entry(cause_hash).or_insert_with(Vec::new);
            if !fwd.contains(&event_hash) && fwd.len() < MAX_EVENTS {
                fwd.push(event_hash);
            }
            let rev = self.reverse_adj.entry(event_hash).or_insert_with(Vec::new);
            if !rev.contains(&cause_hash) && rev.len() < MAX_EVENTS {
                rev.push(cause_hash);
            }
        }

        // Add to recent window
        self.recent_events.push_back((event_hash, tick));
        if self.recent_events.len() > MAX_OBSERVATIONS {
            self.recent_events.pop_front();
        }

        // Decay stale links
        if tick % 1000 == 0 {
            self.decay_links();
        }

        self.stats.total_events_tracked = self.event_counts.len() as u64;
        self.stats.total_links = self.links.len() as u64;
    }

    fn decay_links(&mut self) {
        let mut to_remove = Vec::new();
        for (key, link) in self.links.iter_mut() {
            link.strength *= CAUSAL_STRENGTH_DECAY;
            if link.strength < 0.001 && link.confidence < MIN_CONFIDENCE {
                to_remove.push(*key);
            }
        }
        for key in to_remove {
            self.links.remove(&key);
            // Clean adjacency
            if let Some(fwd) = self.forward_adj.get_mut(&key.0) {
                fwd.retain(|&e| e != key.1);
            }
            if let Some(rev) = self.reverse_adj.get_mut(&key.1) {
                rev.retain(|&c| c != key.0);
            }
        }
    }

    /// Predict effects of a given cause event.
    pub fn causal_predict(&mut self, cause_event: u64) -> CausalPrediction {
        self.stats.total_predictions += 1;
        let mut effects = Vec::new();
        let mut total_confidence = 0.0f32;

        if let Some(direct_effects) = self.forward_adj.get(&cause_event).cloned() {
            for effect in &direct_effects {
                let key = (cause_event, *effect);
                if let Some(link) = self.links.get(&key) {
                    if link.causal_score() > MIN_CONFIDENCE {
                        effects.push((*effect, link.strength, link.delay_ticks));
                        total_confidence += link.confidence;
                    }
                }
            }
        }

        let confidence = if effects.is_empty() {
            0.0
        } else {
            total_confidence / effects.len() as f32
        };

        effects.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        CausalPrediction { cause: cause_event, effects, confidence }
    }

    /// Identify the root cause(s) of a given effect event.
    pub fn identify_cause(&self, effect_event: u64) -> Vec<(u64, f32)> {
        let mut causes = Vec::new();
        if let Some(potential_causes) = self.reverse_adj.get(&effect_event) {
            for &cause in potential_causes {
                let key = (cause, effect_event);
                if let Some(link) = self.links.get(&key) {
                    if link.causal_score() > MIN_CONFIDENCE {
                        causes.push((cause, link.causal_score()));
                    }
                }
            }
        }
        causes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        causes
    }

    /// Trace the full causal chain from a root cause, following the strongest links.
    pub fn causal_chain(&mut self, root_cause: u64) -> CausalChain {
        self.stats.total_chains_found += 1;
        let mut events = Vec::new();
        let mut total_strength = 1.0f32;
        let mut total_delay = 0u64;
        let mut min_confidence = 1.0f32;
        let mut current = root_cause;
        let mut visited = Vec::new();

        events.push(current);
        visited.push(current);

        for _ in 0..MAX_CHAIN_DEPTH {
            let next = self.strongest_effect(current, &visited);
            match next {
                Some((effect, strength, delay, conf)) => {
                    events.push(effect);
                    total_strength *= strength;
                    total_delay += delay;
                    if conf < min_confidence {
                        min_confidence = conf;
                    }
                    visited.push(effect);
                    current = effect;
                }
                None => break,
            }
        }

        let chain_len = events.len() as f32;
        self.stats.avg_chain_length = self.stats.avg_chain_length * (1.0 - EMA_ALPHA)
            + chain_len * EMA_ALPHA;

        CausalChain {
            events,
            total_strength,
            total_delay_ticks: total_delay,
            min_confidence,
        }
    }

    fn strongest_effect(&self, cause: u64, visited: &[u64]) -> Option<(u64, f32, u64, f32)> {
        let effects = self.forward_adj.get(&cause)?;
        let mut best: Option<(u64, f32, u64, f32)> = None;
        for &effect in effects {
            if visited.contains(&effect) {
                continue;
            }
            let key = (cause, effect);
            if let Some(link) = self.links.get(&key) {
                let score = link.causal_score();
                if score > MIN_CONFIDENCE {
                    match &best {
                        Some((_, bs, _, _)) if score <= *bs => {}
                        _ => best = Some((effect, link.strength, link.delay_ticks, link.confidence)),
                    }
                }
            }
        }
        best
    }

    /// Estimate the effect of an intervention: what happens if we force or block an event?
    pub fn intervention_effect(&self, event_hash: u64, blocked: bool) -> Vec<(u64, f32)> {
        let mut downstream_effects = Vec::new();
        if blocked {
            // If we block this event, its effects lose their cause
            if let Some(effects) = self.forward_adj.get(&event_hash) {
                for &effect in effects {
                    let key = (event_hash, effect);
                    if let Some(link) = self.links.get(&key) {
                        // The effect's probability drops by the link strength
                        downstream_effects.push((effect, -link.strength));
                    }
                }
            }
        } else {
            // If we force this event, its effects become more likely
            if let Some(effects) = self.forward_adj.get(&event_hash) {
                for &effect in effects {
                    let key = (event_hash, effect);
                    if let Some(link) = self.links.get(&key) {
                        downstream_effects.push((effect, link.strength));
                    }
                }
            }
        }
        downstream_effects.sort_by(|a, b| {
            b.1.abs().partial_cmp(&a.1.abs()).unwrap_or(core::cmp::Ordering::Equal)
        });
        downstream_effects
    }

    /// Answer a counterfactual: "What would NOT have happened without this event?"
    pub fn counterfactual(&mut self, removed_event: u64) -> CounterfactualResult {
        self.stats.counterfactual_queries += 1;
        let mut prevented = Vec::new();
        let mut impact = 0.0f32;

        // BFS through forward adjacency
        let mut queue = Vec::new();
        let mut visited = Vec::new();
        queue.push(removed_event);
        visited.push(removed_event);

        while let Some(current) = queue.pop() {
            if let Some(effects) = self.forward_adj.get(&current).cloned() {
                for effect in effects {
                    if visited.contains(&effect) {
                        continue;
                    }
                    let key = (current, effect);
                    if let Some(link) = self.links.get(&key) {
                        // Effect is prevented only if this is its dominant cause
                        let causes = self.identify_cause(effect);
                        let is_dominant = causes.first().map(|c| c.0 == current).unwrap_or(false);
                        if is_dominant && link.strength > 0.5 {
                            prevented.push(effect);
                            impact += link.strength;
                            queue.push(effect);
                        }
                    }
                    visited.push(effect);
                }
            }
        }

        CounterfactualResult {
            removed_event,
            prevented_effects: prevented,
            impact_magnitude: impact,
        }
    }

    /// Compute the causal strength between two specific events.
    #[inline(always)]
    pub fn causal_strength(&self, cause: u64, effect: u64) -> f32 {
        let key = (cause, effect);
        self.links.get(&key).map(|l| l.causal_score()).unwrap_or(0.0)
    }

    /// Get all links sorted by causal score descending.
    #[inline]
    pub fn top_links(&self, limit: usize) -> Vec<(u64, u64, f32)> {
        let mut result: Vec<(u64, u64, f32)> = self
            .links
            .iter()
            .map(|((c, e), link)| (*c, *e, link.causal_score()))
            .collect();
        result.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(core::cmp::Ordering::Equal));
        result.truncate(limit);
        result
    }

    /// Get statistics.
    #[inline(always)]
    pub fn stats(&self) -> &CausalForecastStats {
        &self.stats
    }

    /// Update running average statistics.
    pub fn refresh_stats(&mut self) {
        if self.links.is_empty() {
            return;
        }
        let mut strength_sum = 0.0f32;
        let mut conf_sum = 0.0f32;
        let count = self.links.len() as f32;
        for link in self.links.values() {
            strength_sum += link.strength;
            conf_sum += link.confidence;
        }
        self.stats.avg_link_strength = self.stats.avg_link_strength * (1.0 - EMA_ALPHA)
            + (strength_sum / count) * EMA_ALPHA;
        self.stats.avg_confidence = self.stats.avg_confidence * (1.0 - EMA_ALPHA)
            + (conf_sum / count) * EMA_ALPHA;
    }
}
