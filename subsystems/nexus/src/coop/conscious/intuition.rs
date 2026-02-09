// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Intuition Engine
//!
//! Fast cooperation decisions without full negotiation. When the system
//! encounters a cooperation scenario it has seen many times before, the
//! intuition engine bypasses expensive negotiation and applies a cached
//! template. Each template records the scenario hash, the action taken,
//! and its historical success rate.
//!
//! Intuition is built through experience: as the cooperation protocol
//! handles more scenarios, it builds an increasingly rich library of
//! templates that enable near-instant mediation for common patterns.
//!
//! ## Key Methods
//!
//! - `intuitive_cooperate()` — Attempt fast cooperation via cached template
//! - `build_template()` — Create a new intuition template from experience
//! - `template_hit_rate()` — How often templates match real scenarios
//! - `fast_mediation()` — Use intuition for rapid conflict resolution
//! - `intuition_vs_negotiation()` — Compare intuition vs full negotiation outcomes
//! - `cache_optimization()` — Optimize template cache for hit rate

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.12;
const MAX_TEMPLATES: usize = 512;
const MAX_SCENARIO_HISTORY: usize = 128;
const CONFIDENCE_THRESHOLD: f32 = 0.7;
const SUCCESS_RATE_GOOD: f32 = 0.8;
const EVICTION_THRESHOLD: f32 = 0.3;
const TEMPLATE_DECAY: f32 = 0.998;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

// ============================================================================
// FNV-1a HASHING
// ============================================================================

fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Xorshift64 PRNG for exploration vs exploitation
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

// ============================================================================
// COOPERATION ACTION
// ============================================================================

/// Possible cooperation actions the intuition engine can recommend
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CoopAction {
    /// Share resources equally
    ShareEqually,
    /// Give priority to the process with tighter deadline
    PrioritizeUrgent,
    /// Split proportionally by declared need
    ProportionalSplit,
    /// Queue and batch for efficiency
    QueueAndBatch,
    /// Isolate contending processes
    IsolateContenders,
    /// Defer to full negotiation (intuition insufficient)
    DeferToNegotiation,
}

// ============================================================================
// INTUITION TEMPLATE
// ============================================================================

/// A cached cooperation template for a recognized scenario
#[derive(Debug, Clone)]
pub struct IntuitionTemplate {
    pub scenario_hash: u64,
    pub scenario_name: String,
    pub action: CoopAction,
    /// Success rate when this template was applied (EMA-smoothed)
    pub success_rate: f32,
    /// Number of times this template was used
    pub use_count: u64,
    /// Number of successes
    pub success_count: u64,
    /// Confidence in this template (grows with use)
    pub confidence: f32,
    /// Tick when template was created
    pub created_tick: u64,
    /// Tick of last use
    pub last_use_tick: u64,
    /// Average outcome quality when applied
    pub avg_outcome: f32,
    /// Variance in outcomes
    pub outcome_variance: f32,
}

impl IntuitionTemplate {
    pub fn new(scenario_name: String, action: CoopAction, tick: u64) -> Self {
        let scenario_hash = fnv1a_hash(scenario_name.as_bytes());
        Self {
            scenario_hash,
            scenario_name,
            action,
            success_rate: 0.5,
            use_count: 0,
            success_count: 0,
            confidence: 0.0,
            created_tick: tick,
            last_use_tick: tick,
            avg_outcome: 0.5,
            outcome_variance: 0.0,
        }
    }

    /// Record a use outcome
    #[inline]
    pub fn record_outcome(&mut self, success: bool, quality: f32, tick: u64) {
        self.use_count += 1;
        if success {
            self.success_count += 1;
        }
        let outcome = if success { 1.0 } else { 0.0 };
        self.success_rate += EMA_ALPHA * (outcome - self.success_rate);
        let quality_clamped = if quality < 0.0 {
            0.0
        } else if quality > 1.0 {
            1.0
        } else {
            quality
        };
        let delta = quality_clamped - self.avg_outcome;
        self.avg_outcome += EMA_ALPHA * delta;
        self.outcome_variance += EMA_ALPHA * (delta * delta - self.outcome_variance);
        // Confidence grows with successful uses
        let conf_raw = (self.success_count as f32 / (self.use_count as f32 + 1.0)).min(1.0);
        self.confidence += EMA_ALPHA * (conf_raw - self.confidence);
        self.last_use_tick = tick;
    }

    /// Decay template relevance over time
    #[inline(always)]
    pub fn decay(&mut self) {
        self.confidence *= TEMPLATE_DECAY;
        self.success_rate *= TEMPLATE_DECAY;
    }

    /// Is this template reliable enough for use?
    #[inline(always)]
    pub fn is_reliable(&self) -> bool {
        self.confidence >= CONFIDENCE_THRESHOLD && self.success_rate >= SUCCESS_RATE_GOOD
    }
}

// ============================================================================
// SCENARIO RECORD
// ============================================================================

/// Record of a cooperation scenario for comparison
#[derive(Debug, Clone)]
pub struct ScenarioRecord {
    pub scenario_hash: u64,
    pub used_intuition: bool,
    pub intuition_quality: f32,
    pub negotiation_quality: f32,
    pub tick: u64,
}

// ============================================================================
// INTUITION STATS
// ============================================================================

#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CoopIntuitionStats {
    pub total_templates: usize,
    pub reliable_templates: usize,
    pub total_intuitive_decisions: u64,
    pub total_deferred_to_negotiation: u64,
    pub hit_rate: f32,
    pub avg_success_rate: f32,
    pub avg_template_confidence: f32,
    pub cache_evictions: u64,
    pub intuition_advantage: f32,
}

impl CoopIntuitionStats {
    pub fn new() -> Self {
        Self {
            total_templates: 0,
            reliable_templates: 0,
            total_intuitive_decisions: 0,
            total_deferred_to_negotiation: 0,
            hit_rate: 0.0,
            avg_success_rate: 0.0,
            avg_template_confidence: 0.0,
            cache_evictions: 0,
            intuition_advantage: 0.0,
        }
    }
}

// ============================================================================
// COOPERATION INTUITION ENGINE
// ============================================================================

/// Engine for fast cooperation decisions via cached templates
pub struct CoopIntuitionEngine {
    templates: BTreeMap<u64, IntuitionTemplate>,
    /// History of scenario comparisons (intuition vs negotiation)
    scenario_history: Vec<ScenarioRecord>,
    scenario_write_idx: usize,
    pub stats: CoopIntuitionStats,
    rng_state: u64,
    tick: u64,
    /// EMA-smoothed hit rate
    hit_rate_ema: f32,
    /// EMA-smoothed intuition advantage over negotiation
    advantage_ema: f32,
    /// Total lookup attempts
    total_lookups: u64,
    /// Total hits
    total_hits: u64,
}

impl CoopIntuitionEngine {
    pub fn new(seed: u64) -> Self {
        let mut scenario_history = Vec::with_capacity(MAX_SCENARIO_HISTORY);
        for _ in 0..MAX_SCENARIO_HISTORY {
            scenario_history.push(ScenarioRecord {
                scenario_hash: 0,
                used_intuition: false,
                intuition_quality: 0.0,
                negotiation_quality: 0.0,
                tick: 0,
            });
        }
        Self {
            templates: BTreeMap::new(),
            scenario_history,
            scenario_write_idx: 0,
            stats: CoopIntuitionStats::new(),
            rng_state: seed | 1,
            tick: 0,
            hit_rate_ema: 0.0,
            advantage_ema: 0.0,
            total_lookups: 0,
            total_hits: 0,
        }
    }

    // ========================================================================
    // INTUITIVE COOPERATE
    // ========================================================================

    /// Attempt fast cooperation using a cached template.
    ///
    /// Returns the recommended action if a reliable template is found,
    /// or `DeferToNegotiation` if no suitable template exists.
    #[inline]
    pub fn intuitive_cooperate(&mut self, scenario_description: &str) -> CoopAction {
        self.tick += 1;
        self.total_lookups += 1;

        let hash = fnv1a_hash(scenario_description.as_bytes());

        if let Some(template) = self.templates.get(&hash) {
            if template.is_reliable() {
                self.total_hits += 1;
                let hit = 1.0;
                self.hit_rate_ema += EMA_ALPHA * (hit - self.hit_rate_ema);
                self.stats.total_intuitive_decisions += 1;
                return template.action;
            }
        }

        // Miss — no reliable template
        self.hit_rate_ema += EMA_ALPHA * (0.0 - self.hit_rate_ema);
        self.stats.total_deferred_to_negotiation += 1;
        CoopAction::DeferToNegotiation
    }

    /// Record the outcome of an intuitive decision
    #[inline]
    pub fn record_intuitive_outcome(
        &mut self,
        scenario_description: &str,
        success: bool,
        quality: f32,
    ) {
        let hash = fnv1a_hash(scenario_description.as_bytes());
        let tick = self.tick;
        if let Some(template) = self.templates.get_mut(&hash) {
            template.record_outcome(success, quality, tick);
        }
    }

    // ========================================================================
    // BUILD TEMPLATE
    // ========================================================================

    /// Create a new intuition template from a successful cooperation experience
    pub fn build_template(
        &mut self,
        scenario_name: String,
        action: CoopAction,
        initial_quality: f32,
    ) -> u64 {
        self.tick += 1;
        let hash = fnv1a_hash(scenario_name.as_bytes());

        if self.templates.contains_key(&hash) {
            // Update existing template
            let tick = self.tick;
            if let Some(t) = self.templates.get_mut(&hash) {
                t.record_outcome(true, initial_quality, tick);
            }
            return hash;
        }

        if self.templates.len() >= MAX_TEMPLATES {
            self.evict_worst_template();
        }

        let mut template = IntuitionTemplate::new(scenario_name, action, self.tick);
        template.record_outcome(true, initial_quality, self.tick);
        self.templates.insert(hash, template);
        self.update_stats();
        hash
    }

    // ========================================================================
    // TEMPLATE HIT RATE
    // ========================================================================

    /// How often do templates match incoming cooperation scenarios?
    #[inline]
    pub fn template_hit_rate(&self) -> f32 {
        if self.total_lookups == 0 {
            return 0.0;
        }
        self.hit_rate_ema
    }

    /// Raw hit rate (not EMA-smoothed)
    #[inline]
    pub fn raw_hit_rate(&self) -> f32 {
        if self.total_lookups == 0 {
            return 0.0;
        }
        self.total_hits as f32 / self.total_lookups as f32
    }

    // ========================================================================
    // FAST MEDIATION
    // ========================================================================

    /// Use intuition for rapid conflict resolution between two processes
    ///
    /// Hashes the conflict fingerprint and looks up a template. Returns
    /// the action and confidence level, or defers if no template fits.
    #[inline]
    pub fn fast_mediation(
        &mut self,
        process_a: u64,
        process_b: u64,
        contention_type: u8,
    ) -> (CoopAction, f32) {
        self.tick += 1;

        let mut buf = [0u8; 17];
        buf[..8].copy_from_slice(&process_a.to_le_bytes());
        buf[8..16].copy_from_slice(&process_b.to_le_bytes());
        buf[16] = contention_type;
        let hash = fnv1a_hash(&buf);

        self.total_lookups += 1;

        if let Some(template) = self.templates.get(&hash) {
            if template.confidence >= CONFIDENCE_THRESHOLD * 0.8 {
                self.total_hits += 1;
                self.hit_rate_ema += EMA_ALPHA * (1.0 - self.hit_rate_ema);
                self.stats.total_intuitive_decisions += 1;
                return (template.action, template.confidence);
            }
        }

        self.hit_rate_ema += EMA_ALPHA * (0.0 - self.hit_rate_ema);
        self.stats.total_deferred_to_negotiation += 1;
        (CoopAction::DeferToNegotiation, 0.0)
    }

    // ========================================================================
    // INTUITION VS NEGOTIATION
    // ========================================================================

    /// Compare intuition outcome against full negotiation for the same scenario
    #[inline]
    pub fn intuition_vs_negotiation(
        &mut self,
        scenario_hash: u64,
        intuition_quality: f32,
        negotiation_quality: f32,
    ) {
        self.tick += 1;

        let record = ScenarioRecord {
            scenario_hash,
            used_intuition: true,
            intuition_quality,
            negotiation_quality,
            tick: self.tick,
        };
        self.scenario_history[self.scenario_write_idx] = record;
        self.scenario_write_idx = (self.scenario_write_idx + 1) % MAX_SCENARIO_HISTORY;

        let advantage = intuition_quality - negotiation_quality;
        self.advantage_ema += EMA_ALPHA * (advantage - self.advantage_ema);
        self.stats.intuition_advantage = self.advantage_ema;
    }

    /// Average intuition advantage across recorded comparisons
    #[inline(always)]
    pub fn avg_intuition_advantage(&self) -> f32 {
        self.advantage_ema
    }

    // ========================================================================
    // CACHE OPTIMIZATION
    // ========================================================================

    /// Optimize the template cache for better hit rate
    ///
    /// Evicts low-performing templates, decays stale ones, and
    /// computes updated statistics.
    pub fn cache_optimization(&mut self) -> u32 {
        self.tick += 1;
        let mut evicted = 0u32;

        // Decay all templates
        let ids: Vec<u64> = self.templates.keys().copied().collect();
        for id in ids.iter() {
            if let Some(t) = self.templates.get_mut(id) {
                t.decay();
            }
        }

        // Evict templates below threshold
        let to_evict: Vec<u64> = self
            .templates
            .iter()
            .filter(|(_, t)| t.use_count > 5 && t.success_rate < EVICTION_THRESHOLD)
            .map(|(k, _)| *k)
            .collect();
        for id in to_evict {
            self.templates.remove(&id);
            evicted += 1;
        }

        self.stats.cache_evictions += evicted as u64;
        self.update_stats();
        evicted
    }

    // ========================================================================
    // INTERNAL
    // ========================================================================

    fn evict_worst_template(&mut self) {
        let mut worst_id: Option<u64> = None;
        let mut worst_score = f32::MAX;
        for (id, t) in self.templates.iter() {
            let score =
                t.success_rate * 0.5 + t.confidence * 0.3 + (t.use_count as f32 / 100.0).min(0.2);
            if score < worst_score {
                worst_score = score;
                worst_id = Some(*id);
            }
        }
        if let Some(id) = worst_id {
            self.templates.remove(&id);
            self.stats.cache_evictions += 1;
        }
    }

    fn update_stats(&mut self) {
        let count = self.templates.len();
        self.stats.total_templates = count;
        if count == 0 {
            return;
        }
        let mut reliable = 0usize;
        let mut total_success = 0.0f32;
        let mut total_conf = 0.0f32;
        for (_, t) in self.templates.iter() {
            if t.is_reliable() {
                reliable += 1;
            }
            total_success += t.success_rate;
            total_conf += t.confidence;
        }
        self.stats.reliable_templates = reliable;
        self.stats.avg_success_rate = total_success / count as f32;
        self.stats.avg_template_confidence = total_conf / count as f32;
        self.stats.hit_rate = self.hit_rate_ema;
    }

    // ========================================================================
    // QUERIES
    // ========================================================================

    #[inline(always)]
    pub fn template(&self, hash: u64) -> Option<&IntuitionTemplate> {
        self.templates.get(&hash)
    }

    #[inline(always)]
    pub fn template_count(&self) -> usize {
        self.templates.len()
    }

    #[inline(always)]
    pub fn reliable_template_count(&self) -> usize {
        self.templates.values().filter(|t| t.is_reliable()).count()
    }

    #[inline(always)]
    pub fn snapshot_stats(&self) -> CoopIntuitionStats {
        self.stats.clone()
    }
}
