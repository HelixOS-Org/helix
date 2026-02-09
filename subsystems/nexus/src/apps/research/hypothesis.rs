// SPDX-License-Identifier: GPL-2.0
//! # Apps Hypothesis Engine — Scientific Hypothesis Generation
//!
//! Observes anomalies and statistical patterns in application telemetry, then
//! formulates testable hypotheses about potential classification improvements.
//! "If we classify by I/O pattern instead of CPU profile, accuracy improves."
//! Each hypothesis carries a statement, supporting evidence, a confidence
//! level, and explicit test criteria. The engine ranks hypotheses by expected
//! impact × testability and manages their lifecycle from formation through
//! confirmation or rejection.
//!
//! The engine that asks "what if?" about every app behavior pattern.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_HYPOTHESES: usize = 256;
const MAX_EVIDENCE_PER_HYPOTHESIS: usize = 32;
const MAX_ANOMALIES: usize = 512;
const CONFIDENCE_DECAY_RATE: f32 = 0.005;
const EVIDENCE_WEIGHT_EMA: f32 = 0.15;
const MIN_EVIDENCE_FOR_RANKING: usize = 3;
const EMA_ALPHA: f32 = 0.12;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const NOVELTY_THRESHOLD: f32 = 0.70;
const PRUNE_CONFIDENCE_MIN: f32 = 0.10;
const PRUNE_AGE_TICKS: u64 = 50_000;

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
// HYPOTHESIS TYPES
// ============================================================================

/// Lifecycle phase of a hypothesis
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HypothesisPhase {
    Formulated,
    EvidenceCollection,
    ReadyForTest,
    UnderTest,
    Confirmed,
    Rejected,
    Archived,
}

/// Type of classification anomaly that triggered hypothesis generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClassificationAnomaly {
    AccuracyDrop,
    MisclassifiedBurst,
    PredictionFailure,
    FeatureDrift,
    NovelWorkload,
    ResourceMismatch,
    StrategyInefficiency,
}

/// A piece of evidence supporting or refuting a hypothesis
#[derive(Debug, Clone)]
pub struct Evidence {
    pub evidence_id: u64,
    pub anomaly_type: ClassificationAnomaly,
    pub strength: f32,
    pub tick: u64,
    pub description: String,
    pub supports: bool,
}

/// Test criteria for a hypothesis
#[derive(Debug, Clone)]
pub struct TestCriteria {
    pub metric_name: String,
    pub expected_improvement: f32,
    pub min_sample_size: u32,
    pub max_p_value: f32,
    pub duration_ticks: u64,
}

/// A single hypothesis about app behavior or classification
#[derive(Debug, Clone)]
pub struct Hypothesis {
    pub hypothesis_id: u64,
    pub statement: String,
    pub phase: HypothesisPhase,
    pub confidence: f32,
    pub evidence: Vec<Evidence>,
    pub test_criteria: Option<TestCriteria>,
    pub expected_impact: f32,
    pub created_tick: u64,
    pub last_updated_tick: u64,
    pub domain_hash: u64,
}

/// Ranked hypothesis result
#[derive(Debug, Clone)]
pub struct RankedHypothesis {
    pub hypothesis_id: u64,
    pub rank: u32,
    pub score: f32,
    pub confidence: f32,
    pub testability: f32,
    pub expected_impact: f32,
}

// ============================================================================
// HYPOTHESIS STATS
// ============================================================================

/// Aggregate hypothesis engine statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct HypothesisStats {
    pub total_generated: u64,
    pub total_confirmed: u64,
    pub total_rejected: u64,
    pub total_archived: u64,
    pub total_evidence_collected: u64,
    pub avg_confidence_ema: f32,
    pub confirmation_rate: f32,
    pub avg_testability: f32,
    pub anomalies_observed: u64,
    pub pruned_count: u64,
}

// ============================================================================
// ANOMALY TRACKER
// ============================================================================

/// Tracks recent classification anomalies and detects recurring patterns
#[derive(Debug)]
struct AnomalyTracker {
    anomalies: Vec<(u64, ClassificationAnomaly, f32, u64)>,
    type_counts: BTreeMap<u64, u64>,
    pattern_hashes: BTreeMap<u64, u32>,
}

impl AnomalyTracker {
    fn new() -> Self {
        Self {
            anomalies: Vec::new(),
            type_counts: BTreeMap::new(),
            pattern_hashes: BTreeMap::new(),
        }
    }

    fn record(&mut self, anomaly_type: ClassificationAnomaly, severity: f32, tick: u64, ctx_hash: u64) {
        let type_key = anomaly_type as u64;
        let count = self.type_counts.entry(type_key).or_insert(0);
        *count += 1;
        let pattern = fnv1a_hash(&ctx_hash.to_le_bytes()) ^ type_key;
        let freq = self.pattern_hashes.entry(pattern).or_insert(0);
        *freq += 1;
        self.anomalies.push((tick, anomaly_type, severity, ctx_hash));
        if self.anomalies.len() > MAX_ANOMALIES {
            self.anomalies.remove(0);
        }
    }

    fn recurring_patterns(&self) -> Vec<(u64, u32)> {
        let mut patterns: Vec<(u64, u32)> = self
            .pattern_hashes
            .iter()
            .filter(|(_, &count)| count > 2)
            .map(|(&hash, &count)| (hash, count))
            .collect();
        patterns.sort_by(|a, b| b.1.cmp(&a.1));
        patterns
    }
}

// ============================================================================
// APPS HYPOTHESIS ENGINE
// ============================================================================

/// Autonomous hypothesis generation engine for app classification research
#[derive(Debug)]
pub struct AppsHypothesisEngine {
    hypotheses: BTreeMap<u64, Hypothesis>,
    anomaly_tracker: AnomalyTracker,
    rng_state: u64,
    current_tick: u64,
    stats: HypothesisStats,
}

impl AppsHypothesisEngine {
    /// Create a new hypothesis engine with a seed
    pub fn new(seed: u64) -> Self {
        Self {
            hypotheses: BTreeMap::new(),
            anomaly_tracker: AnomalyTracker::new(),
            rng_state: seed | 1,
            current_tick: 0,
            stats: HypothesisStats::default(),
        }
    }

    /// Generate a hypothesis from observed classification anomaly
    pub fn generate_hypothesis(
        &mut self,
        statement: String,
        anomaly_type: ClassificationAnomaly,
        severity: f32,
        expected_impact: f32,
        tick: u64,
    ) -> u64 {
        self.current_tick = tick;
        self.anomaly_tracker.record(anomaly_type, severity, tick, 0);
        self.stats.anomalies_observed += 1;

        let id = fnv1a_hash(statement.as_bytes()) ^ xorshift64(&mut self.rng_state);
        let domain_hash = fnv1a_hash(&(anomaly_type as u64).to_le_bytes());

        // Check novelty against existing hypotheses
        let mut most_similar: f32 = 0.0;
        for existing in self.hypotheses.values() {
            if existing.domain_hash == domain_hash {
                let statement_hash_a = fnv1a_hash(existing.statement.as_bytes());
                let statement_hash_b = fnv1a_hash(statement.as_bytes());
                let similarity = if statement_hash_a == statement_hash_b {
                    1.0
                } else {
                    let xor = statement_hash_a ^ statement_hash_b;
                    1.0 - (xor.count_ones() as f32 / 64.0)
                };
                if similarity > most_similar {
                    most_similar = similarity;
                }
            }
        }

        let initial_confidence = if most_similar > NOVELTY_THRESHOLD {
            0.3 * severity
        } else {
            0.5 * severity + 0.2
        };

        let hypothesis = Hypothesis {
            hypothesis_id: id,
            statement,
            phase: HypothesisPhase::Formulated,
            confidence: initial_confidence.clamp(0.0, 1.0),
            evidence: Vec::new(),
            test_criteria: None,
            expected_impact: expected_impact.clamp(0.0, 1.0),
            created_tick: tick,
            last_updated_tick: tick,
            domain_hash,
        };

        if self.hypotheses.len() < MAX_HYPOTHESES {
            self.hypotheses.insert(id, hypothesis);
            self.stats.total_generated += 1;
        }
        id
    }

    /// Compute evidence strength for a hypothesis
    pub fn evidence_strength(&self, hypothesis_id: u64) -> f32 {
        let hypothesis = match self.hypotheses.get(&hypothesis_id) {
            Some(h) => h,
            None => return 0.0,
        };
        if hypothesis.evidence.is_empty() {
            return 0.0;
        }
        let mut supporting_weight: f32 = 0.0;
        let mut opposing_weight: f32 = 0.0;
        let mut ema_strength: f32 = 0.0;
        for ev in &hypothesis.evidence {
            ema_strength = EVIDENCE_WEIGHT_EMA * ev.strength + (1.0 - EVIDENCE_WEIGHT_EMA) * ema_strength;
            if ev.supports {
                supporting_weight += ev.strength;
            } else {
                opposing_weight += ev.strength;
            }
        }
        let total = supporting_weight + opposing_weight;
        if total < 0.001 {
            return 0.0;
        }
        let net_support = (supporting_weight - opposing_weight) / total;
        let recency_factor = ema_strength.clamp(0.0, 1.0);
        (net_support * 0.7 + recency_factor * 0.3).clamp(0.0, 1.0)
    }

    /// Test a hypothesis by adding evidence and transitioning phase
    pub fn hypothesis_test(
        &mut self,
        hypothesis_id: u64,
        evidence_desc: String,
        anomaly_type: ClassificationAnomaly,
        strength: f32,
        supports: bool,
        tick: u64,
    ) -> bool {
        self.current_tick = tick;
        let hypothesis = match self.hypotheses.get_mut(&hypothesis_id) {
            Some(h) => h,
            None => return false,
        };
        if hypothesis.evidence.len() >= MAX_EVIDENCE_PER_HYPOTHESIS {
            hypothesis.evidence.remove(0);
        }
        let ev_id = fnv1a_hash(evidence_desc.as_bytes()) ^ xorshift64(&mut self.rng_state);
        hypothesis.evidence.push(Evidence {
            evidence_id: ev_id,
            anomaly_type,
            strength: strength.clamp(0.0, 1.0),
            tick,
            description: evidence_desc,
            supports,
        });
        hypothesis.last_updated_tick = tick;
        self.stats.total_evidence_collected += 1;

        // Phase transitions
        if hypothesis.phase == HypothesisPhase::Formulated
            && hypothesis.evidence.len() >= MIN_EVIDENCE_FOR_RANKING
        {
            hypothesis.phase = HypothesisPhase::EvidenceCollection;
        }

        let ev_str = self.evidence_strength(hypothesis_id);
        let hypothesis = self.hypotheses.get_mut(&hypothesis_id).unwrap();
        if hypothesis.phase == HypothesisPhase::EvidenceCollection && ev_str > 0.6 {
            hypothesis.phase = HypothesisPhase::ReadyForTest;
        }
        true
    }

    /// Update confidence of a hypothesis based on accumulated evidence
    pub fn confidence_update(&mut self, hypothesis_id: u64, tick: u64) -> f32 {
        self.current_tick = tick;
        let ev_str = self.evidence_strength(hypothesis_id);
        let hypothesis = match self.hypotheses.get_mut(&hypothesis_id) {
            Some(h) => h,
            None => return 0.0,
        };
        let age_ticks = tick.saturating_sub(hypothesis.created_tick);
        let decay = CONFIDENCE_DECAY_RATE * (age_ticks as f32 / 1000.0);
        let new_confidence = (ev_str * 0.6 + hypothesis.confidence * 0.4 - decay).clamp(0.0, 1.0);
        hypothesis.confidence = new_confidence;
        hypothesis.last_updated_tick = tick;
        self.stats.avg_confidence_ema =
            EMA_ALPHA * new_confidence + (1.0 - EMA_ALPHA) * self.stats.avg_confidence_ema;
        new_confidence
    }

    /// Prune stale or low-confidence hypotheses
    pub fn hypothesis_prune(&mut self, tick: u64) -> u64 {
        self.current_tick = tick;
        let mut to_archive: Vec<u64> = Vec::new();
        for (id, h) in self.hypotheses.iter() {
            let stale = tick.saturating_sub(h.last_updated_tick) > PRUNE_AGE_TICKS;
            let low_confidence = h.confidence < PRUNE_CONFIDENCE_MIN;
            let rejectable = h.phase == HypothesisPhase::Formulated
                || h.phase == HypothesisPhase::EvidenceCollection;
            if (stale || low_confidence) && rejectable {
                to_archive.push(*id);
            }
        }
        let pruned = to_archive.len() as u64;
        for id in &to_archive {
            if let Some(h) = self.hypotheses.get_mut(id) {
                h.phase = HypothesisPhase::Archived;
            }
        }
        self.stats.pruned_count += pruned;
        self.stats.total_archived += pruned;

        // Update confirmation rate
        let total = self.stats.total_confirmed + self.stats.total_rejected;
        if total > 0 {
            self.stats.confirmation_rate = self.stats.total_confirmed as f32 / total as f32;
        }
        pruned
    }

    /// Mark a hypothesis as confirmed
    pub fn confirm(&mut self, hypothesis_id: u64) {
        if let Some(h) = self.hypotheses.get_mut(&hypothesis_id) {
            h.phase = HypothesisPhase::Confirmed;
            self.stats.total_confirmed += 1;
        }
    }

    /// Mark a hypothesis as rejected
    pub fn reject(&mut self, hypothesis_id: u64) {
        if let Some(h) = self.hypotheses.get_mut(&hypothesis_id) {
            h.phase = HypothesisPhase::Rejected;
            self.stats.total_rejected += 1;
        }
    }

    /// Rank all testable hypotheses by score = impact × confidence × testability
    pub fn rank_hypotheses(&self) -> Vec<RankedHypothesis> {
        let mut ranked: Vec<RankedHypothesis> = Vec::new();
        for h in self.hypotheses.values() {
            if h.phase != HypothesisPhase::ReadyForTest
                && h.phase != HypothesisPhase::EvidenceCollection
            {
                continue;
            }
            let testability = if h.test_criteria.is_some() { 0.9 } else { 0.5 };
            let score = h.expected_impact * h.confidence * testability;
            ranked.push(RankedHypothesis {
                hypothesis_id: h.hypothesis_id,
                rank: 0,
                score,
                confidence: h.confidence,
                testability,
                expected_impact: h.expected_impact,
            });
        }
        ranked.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(core::cmp::Ordering::Equal));
        for (i, r) in ranked.iter_mut().enumerate() {
            r.rank = i as u32 + 1;
        }
        ranked
    }

    /// Get aggregate stats
    pub fn stats(&self) -> HypothesisStats {
        self.stats
    }

    /// Get recurring anomaly patterns
    pub fn recurring_patterns(&self) -> Vec<(u64, u32)> {
        self.anomaly_tracker.recurring_patterns()
    }
}
