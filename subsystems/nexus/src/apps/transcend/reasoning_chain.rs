// SPDX-License-Identifier: GPL-2.0
//! # Apps Reasoning Chain — Explainable Reasoning for App Management
//!
//! Every decision the kernel makes about application classification and
//! resource allocation carries a complete reasoning trace. This module
//! provides transparent, auditable chains of logic so that operators and
//! other subsystems can understand *why* the system chose a particular
//! strategy, classification, or allocation quantum.
//!
//! The reasoning engine builds multi-step inference chains, evaluates chain
//! quality, identifies the weakest link, and exposes depth metrics so that
//! downstream consumers can assess confidence and trigger deeper analysis
//! when reasoning is shallow.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x00000100000001B3;
const EMA_ALPHA_NUM: u64 = 2;
const EMA_ALPHA_DEN: u64 = 9;
const MAX_CHAIN_DEPTH: usize = 64;
const MAX_CHAINS: usize = 2048;
const MIN_STEP_CONFIDENCE: u64 = 10;
const QUALITY_EXCELLENT: u64 = 85;
const QUALITY_GOOD: u64 = 60;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn fnv1a(data: &[u8]) -> u64 {
    let mut h = FNV_OFFSET;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

fn xorshift64(state: &mut u64) -> u64 {
    let mut s = *state;
    s ^= s << 13;
    s ^= s >> 7;
    s ^= s << 17;
    *state = s;
    s
}

fn ema_update(prev: u64, sample: u64) -> u64 {
    (EMA_ALPHA_NUM * sample + (EMA_ALPHA_DEN - EMA_ALPHA_NUM) * prev) / EMA_ALPHA_DEN
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// A single step in a reasoning chain.
#[derive(Clone, Debug)]
pub struct ReasoningStep {
    pub step_id: u64,
    pub premise_hash: u64,
    pub conclusion_hash: u64,
    pub rule_label: String,
    pub confidence: u64,
    pub evidence_count: u64,
}

/// A complete reasoning chain from observation to decision.
#[derive(Clone, Debug)]
pub struct ReasoningChain {
    pub chain_id: u64,
    pub context_hash: u64,
    pub steps: Vec<ReasoningStep>,
    pub overall_confidence: u64,
    pub depth: usize,
    pub quality_score: u64,
    pub weakest_step_idx: usize,
    pub timestamp: u64,
}

/// Explanation record for an app classification decision.
#[derive(Clone, Debug)]
pub struct ClassificationExplanation {
    pub app_id: u64,
    pub chain_id: u64,
    pub assigned_class: u64,
    pub primary_features: Vec<u64>,
    pub deciding_step: u64,
    pub confidence: u64,
}

/// Explanation record for a resource allocation decision.
#[derive(Clone, Debug)]
pub struct AllocationExplanation {
    pub app_id: u64,
    pub chain_id: u64,
    pub resource_kind: String,
    pub allocated_amount: u64,
    pub demand_estimate: u64,
    pub priority_score: u64,
    pub justification_hash: u64,
}

/// Evidence record that feeds into a reasoning step.
#[derive(Clone, Debug)]
pub struct Evidence {
    pub evidence_id: u64,
    pub source_hash: u64,
    pub strength: u64,
    pub sample_count: u64,
    pub freshness: u64,
}

/// Aggregated statistics for the reasoning engine.
#[derive(Clone, Debug, Default)]
#[repr(align(64))]
pub struct ReasoningStats {
    pub total_chains_built: u64,
    pub avg_depth_ema: u64,
    pub avg_quality_ema: u64,
    pub classification_explanations: u64,
    pub allocation_explanations: u64,
    pub weak_chains_detected: u64,
    pub excellent_chains: u64,
    pub evidence_pool_size: u64,
}

// ---------------------------------------------------------------------------
// AppsReasoningChain
// ---------------------------------------------------------------------------

/// Engine for building and evaluating explainable reasoning chains that
/// justify every app management decision.
pub struct AppsReasoningChain {
    chains: BTreeMap<u64, ReasoningChain>,
    classification_expl: BTreeMap<u64, ClassificationExplanation>,
    allocation_expl: BTreeMap<u64, AllocationExplanation>,
    evidence_pool: BTreeMap<u64, Evidence>,
    stats: ReasoningStats,
    rng: u64,
    tick: u64,
}

impl AppsReasoningChain {
    /// Create a new reasoning chain engine.
    pub fn new(seed: u64) -> Self {
        Self {
            chains: BTreeMap::new(),
            classification_expl: BTreeMap::new(),
            allocation_expl: BTreeMap::new(),
            evidence_pool: BTreeMap::new(),
            stats: ReasoningStats::default(),
            rng: seed | 1,
            tick: 0,
        }
    }

    // -- evidence -----------------------------------------------------------

    /// Register a piece of evidence from an observation source.
    pub fn register_evidence(&mut self, source_hash: u64, strength: u64, samples: u64) -> u64 {
        let eid = fnv1a(&source_hash.to_le_bytes()) ^ xorshift64(&mut self.rng);
        let freshness = self.tick;
        self.evidence_pool.insert(eid, Evidence {
            evidence_id: eid,
            source_hash,
            strength: strength.min(100),
            sample_count: samples,
            freshness,
        });
        self.stats.evidence_pool_size = self.evidence_pool.len() as u64;
        eid
    }

    /// Update existing evidence with new samples.
    #[inline]
    pub fn update_evidence(&mut self, evidence_id: u64, strength: u64, samples: u64) {
        if let Some(ev) = self.evidence_pool.get_mut(&evidence_id) {
            ev.strength = ema_update(ev.strength, strength.min(100));
            ev.sample_count += samples;
            ev.freshness = self.tick;
        }
    }

    // -- chain building -----------------------------------------------------

    /// Build a reasoning chain from a set of evidence IDs and inference rules.
    ///
    /// Each `(rule_label, evidence_ids)` pair becomes one reasoning step.
    /// Returns the chain ID or `None` if no valid steps could be created.
    pub fn build_reasoning(
        &mut self,
        context_hash: u64,
        steps_input: &[(&str, &[u64])],
    ) -> Option<u64> {
        if steps_input.is_empty() || steps_input.len() > MAX_CHAIN_DEPTH {
            return None;
        }
        if self.chains.len() >= MAX_CHAINS {
            self.evict_oldest_chain();
        }

        self.tick += 1;
        let mut steps = Vec::new();
        let mut prev_conclusion: u64 = context_hash;

        for (rule_label, ev_ids) in steps_input {
            let (confidence, ev_count) = self.aggregate_evidence(ev_ids);
            if confidence < MIN_STEP_CONFIDENCE {
                continue;
            }
            let premise_hash = prev_conclusion;
            let conclusion_hash = self.derive_conclusion(premise_hash, rule_label, confidence);
            let step_id = fnv1a(&conclusion_hash.to_le_bytes()) ^ xorshift64(&mut self.rng);

            steps.push(ReasoningStep {
                step_id,
                premise_hash,
                conclusion_hash,
                rule_label: String::from(*rule_label),
                confidence,
                evidence_count: ev_count,
            });
            prev_conclusion = conclusion_hash;
        }

        if steps.is_empty() {
            return None;
        }

        let depth = steps.len();
        let overall_confidence = self.compute_chain_confidence(&steps);
        let quality_score = self.compute_chain_quality(&steps);
        let weakest_step_idx = self.find_weakest_step(&steps);

        let chain_id = fnv1a(&context_hash.to_le_bytes()) ^ xorshift64(&mut self.rng);
        let chain = ReasoningChain {
            chain_id,
            context_hash,
            steps,
            overall_confidence,
            depth,
            quality_score,
            weakest_step_idx,
            timestamp: self.tick,
        };

        self.chains.insert(chain_id, chain);
        self.stats.total_chains_built += 1;
        self.stats.avg_depth_ema = ema_update(self.stats.avg_depth_ema, depth as u64);
        self.stats.avg_quality_ema = ema_update(self.stats.avg_quality_ema, quality_score);

        if quality_score >= QUALITY_EXCELLENT {
            self.stats.excellent_chains += 1;
        }
        if quality_score < QUALITY_GOOD {
            self.stats.weak_chains_detected += 1;
        }

        Some(chain_id)
    }

    // -- explanation --------------------------------------------------------

    /// Explain why a particular app was given a specific classification.
    pub fn explain_classification(
        &mut self,
        app_id: u64,
        chain_id: u64,
        assigned_class: u64,
        features: &[u64],
    ) -> Option<ClassificationExplanation> {
        let chain = self.chains.get(&chain_id)?;
        let deciding_step = chain.steps.last()?.step_id;
        let confidence = chain.overall_confidence;
        let primary_features: Vec<u64> = features.iter().copied().take(8).collect();

        let expl = ClassificationExplanation {
            app_id,
            chain_id,
            assigned_class,
            primary_features,
            deciding_step,
            confidence,
        };
        self.classification_expl.insert(app_id, expl.clone());
        self.stats.classification_explanations += 1;
        Some(expl)
    }

    /// Explain a resource allocation decision for an application.
    pub fn explain_allocation(
        &mut self,
        app_id: u64,
        chain_id: u64,
        resource_kind: &str,
        allocated: u64,
        demand: u64,
        priority: u64,
    ) -> Option<AllocationExplanation> {
        let chain = self.chains.get(&chain_id)?;
        let justification_hash = chain.steps.last()?.conclusion_hash;

        let expl = AllocationExplanation {
            app_id,
            chain_id,
            resource_kind: String::from(resource_kind),
            allocated_amount: allocated,
            demand_estimate: demand,
            priority_score: priority,
            justification_hash,
        };
        self.allocation_expl.insert(
            fnv1a(&app_id.to_le_bytes()) ^ fnv1a(resource_kind.as_bytes()),
            expl.clone(),
        );
        self.stats.allocation_explanations += 1;
        Some(expl)
    }

    // -- analysis -----------------------------------------------------------

    /// Return the depth of a reasoning chain.
    #[inline(always)]
    pub fn reasoning_depth(&self, chain_id: u64) -> Option<usize> {
        self.chains.get(&chain_id).map(|c| c.depth)
    }

    /// Return the quality score of a reasoning chain (0–100).
    #[inline(always)]
    pub fn chain_quality(&self, chain_id: u64) -> Option<u64> {
        self.chains.get(&chain_id).map(|c| c.quality_score)
    }

    /// Identify the weakest reasoning step in a chain and return it.
    #[inline(always)]
    pub fn weakest_reasoning(&self, chain_id: u64) -> Option<&ReasoningStep> {
        let chain = self.chains.get(&chain_id)?;
        chain.steps.get(chain.weakest_step_idx)
    }

    /// Return a reference to the full chain.
    #[inline(always)]
    pub fn get_chain(&self, chain_id: u64) -> Option<&ReasoningChain> {
        self.chains.get(&chain_id)
    }

    /// Return classification explanation for an app.
    #[inline(always)]
    pub fn get_classification_explanation(&self, app_id: u64) -> Option<&ClassificationExplanation> {
        self.classification_expl.get(&app_id)
    }

    /// Return allocation explanations count.
    #[inline(always)]
    pub fn allocation_explanation_count(&self) -> u64 {
        self.allocation_expl.len() as u64
    }

    /// Return current statistics.
    #[inline(always)]
    pub fn stats(&self) -> &ReasoningStats {
        &self.stats
    }

    /// Advance the internal tick for freshness tracking.
    #[inline(always)]
    pub fn tick(&mut self) {
        self.tick += 1;
        self.decay_evidence_freshness();
    }

    // -- internal -----------------------------------------------------------

    fn aggregate_evidence(&self, ev_ids: &[u64]) -> (u64, u64) {
        if ev_ids.is_empty() {
            return (0, 0);
        }
        let mut total_strength: u64 = 0;
        let mut count: u64 = 0;
        for &eid in ev_ids {
            if let Some(ev) = self.evidence_pool.get(&eid) {
                let age_penalty = self.tick.saturating_sub(ev.freshness).min(20);
                let effective = ev.strength.saturating_sub(age_penalty);
                total_strength += effective;
                count += 1;
            }
        }
        if count == 0 {
            return (0, 0);
        }
        (total_strength / count, count)
    }

    fn derive_conclusion(&mut self, premise: u64, rule: &str, confidence: u64) -> u64 {
        let rule_hash = fnv1a(rule.as_bytes());
        let noise = xorshift64(&mut self.rng) % 256;
        premise.wrapping_add(rule_hash).wrapping_add(confidence).wrapping_add(noise)
    }

    fn compute_chain_confidence(&self, steps: &[ReasoningStep]) -> u64 {
        if steps.is_empty() {
            return 0;
        }
        // Confidence degrades multiplicatively: product / 100^(n-1)
        let mut product: u128 = 100;
        for step in steps {
            product = product * step.confidence as u128 / 100;
        }
        (product as u64).min(100)
    }

    fn compute_chain_quality(&self, steps: &[ReasoningStep]) -> u64 {
        if steps.is_empty() {
            return 0;
        }
        let depth_bonus = (steps.len() as u64).min(10) * 3;
        let avg_confidence = steps.iter().map(|s| s.confidence).sum::<u64>() / steps.len() as u64;
        let avg_evidence = steps.iter().map(|s| s.evidence_count).sum::<u64>() / steps.len() as u64;
        let evidence_bonus = avg_evidence.min(10) * 2;
        let min_conf = steps.iter().map(|s| s.confidence).min().unwrap_or(0);
        let weakness_penalty = if min_conf < 30 { 15 } else if min_conf < 50 { 5 } else { 0 };

        (avg_confidence + depth_bonus + evidence_bonus).saturating_sub(weakness_penalty).min(100)
    }

    fn find_weakest_step(&self, steps: &[ReasoningStep]) -> usize {
        if steps.is_empty() {
            return 0;
        }
        let mut weakest_idx = 0;
        let mut weakest_conf = u64::MAX;
        for (i, step) in steps.iter().enumerate() {
            if step.confidence < weakest_conf {
                weakest_conf = step.confidence;
                weakest_idx = i;
            }
        }
        weakest_idx
    }

    fn evict_oldest_chain(&mut self) {
        let oldest_key = self.chains.iter().min_by_key(|(_, c)| c.timestamp).map(|(k, _)| *k);
        if let Some(key) = oldest_key {
            self.chains.remove(&key);
        }
    }

    fn decay_evidence_freshness(&mut self) {
        let threshold = self.tick.saturating_sub(100);
        let stale_ids: Vec<u64> = self
            .evidence_pool
            .iter()
            .filter(|(_, ev)| ev.freshness < threshold && ev.sample_count < 5)
            .map(|(id, _)| *id)
            .collect();
        for id in stale_ids {
            self.evidence_pool.remove(&id);
        }
        self.stats.evidence_pool_size = self.evidence_pool.len() as u64;
    }
}
