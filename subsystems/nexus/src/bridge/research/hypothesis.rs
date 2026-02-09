// SPDX-License-Identifier: GPL-2.0
//! # Bridge Hypothesis Engine — Scientific Hypothesis Generation
//!
//! Observes anomalies and statistical patterns in bridge telemetry, then
//! formulates testable hypotheses about potential optimizations. Each
//! hypothesis carries a statement, supporting evidence, a confidence level,
//! and explicit test criteria. The engine ranks hypotheses by expected
//! impact × testability and manages their lifecycle from formation through
//! confirmation or rejection.
//!
//! The bridge that asks "what if?" and means it.

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
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const NOVELTY_THRESHOLD: f32 = 0.70;

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
    /// Just formulated, not yet tested
    Formulated,
    /// Gathering supporting evidence
    EvidenceCollection,
    /// Ready for experimental testing
    ReadyForTest,
    /// Currently being tested experimentally
    UnderTest,
    /// Confirmed by experiment
    Confirmed,
    /// Rejected by experiment
    Rejected,
    /// Archived — superseded or stale
    Archived,
}

/// Type of anomaly that triggered hypothesis generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnomalyType {
    LatencySpike,
    ThroughputDrop,
    CacheMissRate,
    BatchInefficiency,
    RoutingImbalance,
    ResourceContention,
    UnexpectedPattern,
}

/// A piece of evidence supporting or refuting a hypothesis
#[derive(Debug, Clone)]
pub struct Evidence {
    pub evidence_id: u64,
    pub anomaly_type: AnomalyType,
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

/// A single hypothesis
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

/// Anomaly observation that feeds hypothesis generation
#[derive(Debug, Clone)]
pub struct Anomaly {
    pub anomaly_id: u64,
    pub anomaly_type: AnomalyType,
    pub severity: f32,
    pub tick: u64,
    pub context_hash: u64,
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
}

// ============================================================================
// ANOMALY TRACKER
// ============================================================================

/// Tracks recent anomalies and detects recurring patterns
#[derive(Debug)]
struct AnomalyTracker {
    anomalies: Vec<Anomaly>,
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

    fn record(&mut self, anomaly: Anomaly) {
        let type_key = anomaly.anomaly_type as u64;
        let count = self.type_counts.entry(type_key).or_insert(0);
        *count += 1;

        let pattern = fnv1a_hash(&anomaly.context_hash.to_le_bytes()) ^ type_key;
        let freq = self.pattern_hashes.entry(pattern).or_insert(0);
        *freq += 1;

        self.anomalies.push(anomaly);
        if self.anomalies.len() > MAX_ANOMALIES {
            self.anomalies.remove(0);
        }
    }

    fn recurring_patterns(&self) -> Vec<(u64, u32)> {
        let mut patterns: Vec<(u64, u32)> = self
            .pattern_hashes
            .iter()
            .filter(|(_, &count)| count > 2)
            .map(|(&k, &v)| (k, v))
            .collect();
        patterns.sort_by(|a, b| b.1.cmp(&a.1));
        patterns
    }
}

// ============================================================================
// BRIDGE HYPOTHESIS ENGINE
// ============================================================================

/// Autonomous hypothesis generation and lifecycle management
#[derive(Debug)]
pub struct BridgeHypothesisEngine {
    hypotheses: BTreeMap<u64, Hypothesis>,
    anomaly_tracker: AnomalyTracker,
    rng_state: u64,
    current_tick: u64,
    stats: HypothesisStats,
}

impl BridgeHypothesisEngine {
    /// Create a new hypothesis engine
    pub fn new(seed: u64) -> Self {
        Self {
            hypotheses: BTreeMap::new(),
            anomaly_tracker: AnomalyTracker::new(),
            rng_state: seed | 1,
            current_tick: 0,
            stats: HypothesisStats::default(),
        }
    }

    /// Generate a hypothesis from an observed anomaly
    pub fn generate_hypothesis(
        &mut self,
        anomaly_type: AnomalyType,
        severity: f32,
        context_hash: u64,
        statement: String,
        expected_impact: f32,
        tick: u64,
    ) -> Hypothesis {
        self.current_tick = tick;
        let anomaly_id = fnv1a_hash(&tick.to_le_bytes()) ^ xorshift64(&mut self.rng_state);
        let anomaly = Anomaly {
            anomaly_id,
            anomaly_type,
            severity,
            tick,
            context_hash,
        };
        self.anomaly_tracker.record(anomaly);
        self.stats.anomalies_observed += 1;

        let hyp_id = fnv1a_hash(statement.as_bytes()) ^ xorshift64(&mut self.rng_state);
        let domain_hash = fnv1a_hash(&context_hash.to_le_bytes());
        let initial_confidence = (severity * 0.3).clamp(0.05, 0.5);

        let hypothesis = Hypothesis {
            hypothesis_id: hyp_id,
            statement,
            phase: HypothesisPhase::Formulated,
            confidence: initial_confidence,
            evidence: Vec::new(),
            test_criteria: None,
            expected_impact: expected_impact.clamp(0.0, 1.0),
            created_tick: tick,
            last_updated_tick: tick,
            domain_hash,
        };

        self.hypotheses.insert(hyp_id, hypothesis.clone());
        self.stats.total_generated += 1;

        // Evict oldest if over capacity
        while self.hypotheses.len() > MAX_HYPOTHESES {
            let oldest = self
                .hypotheses
                .iter()
                .filter(|(_, h)| {
                    h.phase == HypothesisPhase::Archived || h.phase == HypothesisPhase::Rejected
                })
                .min_by_key(|(_, h)| h.created_tick)
                .or_else(|| self.hypotheses.iter().min_by_key(|(_, h)| h.created_tick))
                .map(|(&k, _)| k);
            if let Some(k) = oldest {
                self.hypotheses.remove(&k);
            } else {
                break;
            }
        }

        hypothesis
    }

    /// Collect evidence for a hypothesis
    pub fn evidence_collect(
        &mut self,
        hypothesis_id: u64,
        anomaly_type: AnomalyType,
        strength: f32,
        supports: bool,
        description: String,
        tick: u64,
    ) -> bool {
        self.current_tick = tick;
        let hyp = match self.hypotheses.get_mut(&hypothesis_id) {
            Some(h) => h,
            None => return false,
        };

        if hyp.evidence.len() >= MAX_EVIDENCE_PER_HYPOTHESIS {
            // Remove weakest evidence
            hyp.evidence
                .sort_by(|a, b| b.strength.partial_cmp(&a.strength).unwrap_or(core::cmp::Ordering::Equal));
            hyp.evidence.truncate(MAX_EVIDENCE_PER_HYPOTHESIS - 1);
        }

        let eid = fnv1a_hash(&tick.to_le_bytes()) ^ fnv1a_hash(&hypothesis_id.to_le_bytes());
        hyp.evidence.push(Evidence {
            evidence_id: eid,
            anomaly_type,
            strength: strength.clamp(0.0, 1.0),
            tick,
            description,
            supports,
        });
        hyp.last_updated_tick = tick;

        // Update confidence based on evidence
        let support_strength: f32 = hyp
            .evidence
            .iter()
            .filter(|e| e.supports)
            .map(|e| e.strength)
            .sum::<f32>();
        let refute_strength: f32 = hyp
            .evidence
            .iter()
            .filter(|e| !e.supports)
            .map(|e| e.strength)
            .sum::<f32>();
        let total = support_strength + refute_strength;
        if total > 0.0 {
            let new_conf = support_strength / total;
            hyp.confidence = EVIDENCE_WEIGHT_EMA * new_conf
                + (1.0 - EVIDENCE_WEIGHT_EMA) * hyp.confidence;
        }

        // Advance phase if enough evidence
        if hyp.phase == HypothesisPhase::Formulated
            && hyp.evidence.len() >= MIN_EVIDENCE_FOR_RANKING
        {
            hyp.phase = HypothesisPhase::EvidenceCollection;
        }

        self.stats.total_evidence_collected += 1;
        true
    }

    /// Rank all active hypotheses by score = impact × confidence × testability
    pub fn rank_hypotheses(&self) -> Vec<RankedHypothesis> {
        let mut rankings: Vec<RankedHypothesis> = self
            .hypotheses
            .values()
            .filter(|h| {
                h.phase != HypothesisPhase::Archived && h.phase != HypothesisPhase::Rejected
            })
            .map(|h| {
                let testability = self.testability_score_inner(h);
                let score = h.expected_impact * h.confidence * testability;
                RankedHypothesis {
                    hypothesis_id: h.hypothesis_id,
                    rank: 0,
                    score,
                    confidence: h.confidence,
                    testability,
                    expected_impact: h.expected_impact,
                }
            })
            .collect();

        rankings.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(core::cmp::Ordering::Equal));
        for (i, r) in rankings.iter_mut().enumerate() {
            r.rank = i as u32 + 1;
        }
        rankings
    }

    /// Compute testability score for a hypothesis
    pub fn testability_score(&self, hypothesis_id: u64) -> f32 {
        self.hypotheses
            .get(&hypothesis_id)
            .map_or(0.0, |h| self.testability_score_inner(h))
    }

    fn testability_score_inner(&self, h: &Hypothesis) -> f32 {
        let evidence_factor =
            (h.evidence.len() as f32 / MIN_EVIDENCE_FOR_RANKING as f32).min(1.0);
        let criteria_factor = if h.test_criteria.is_some() {
            1.0
        } else {
            0.5
        };
        let recency_factor = if self.current_tick > h.last_updated_tick {
            let age = self.current_tick - h.last_updated_tick;
            1.0 / (1.0 + age as f32 / 10000.0)
        } else {
            1.0
        };
        (evidence_factor * criteria_factor * recency_factor).clamp(0.0, 1.0)
    }

    /// Advance hypothesis lifecycle: decay confidence, archive stale, transition phases
    pub fn hypothesis_lifecycle(&mut self, tick: u64) {
        self.current_tick = tick;
        let mut to_archive: Vec<u64> = Vec::new();

        for (id, hyp) in self.hypotheses.iter_mut() {
            // Decay confidence over time
            let age_ticks = tick.saturating_sub(hyp.last_updated_tick);
            let decay = CONFIDENCE_DECAY_RATE * (age_ticks as f32 / 1000.0);
            hyp.confidence = (hyp.confidence - decay).max(0.0);

            // Archive if confidence collapsed
            if hyp.confidence < 0.01
                && hyp.phase != HypothesisPhase::Confirmed
                && hyp.phase != HypothesisPhase::Archived
            {
                to_archive.push(*id);
            }

            // Transition: EvidenceCollection → ReadyForTest if confidence > threshold
            if hyp.phase == HypothesisPhase::EvidenceCollection
                && hyp.confidence > 0.6
                && hyp.evidence.len() >= MIN_EVIDENCE_FOR_RANKING
            {
                hyp.phase = HypothesisPhase::ReadyForTest;
            }
        }

        for id in to_archive {
            if let Some(h) = self.hypotheses.get_mut(&id) {
                h.phase = HypothesisPhase::Archived;
                self.stats.total_archived += 1;
            }
        }

        // Update aggregate stats
        let active: Vec<&Hypothesis> = self
            .hypotheses
            .values()
            .filter(|h| h.phase != HypothesisPhase::Archived)
            .collect();
        if !active.is_empty() {
            let avg_conf =
                active.iter().map(|h| h.confidence).sum::<f32>() / active.len() as f32;
            self.stats.avg_confidence_ema =
                EVIDENCE_WEIGHT_EMA * avg_conf
                    + (1.0 - EVIDENCE_WEIGHT_EMA) * self.stats.avg_confidence_ema;
        }

        let total_resolved = self.stats.total_confirmed + self.stats.total_rejected;
        if total_resolved > 0 {
            self.stats.confirmation_rate =
                self.stats.total_confirmed as f32 / total_resolved as f32;
        }
    }

    /// Mark a hypothesis as confirmed or rejected
    pub fn resolve_hypothesis(&mut self, hypothesis_id: u64, confirmed: bool) {
        if let Some(h) = self.hypotheses.get_mut(&hypothesis_id) {
            if confirmed {
                h.phase = HypothesisPhase::Confirmed;
                h.confidence = 1.0;
                self.stats.total_confirmed += 1;
            } else {
                h.phase = HypothesisPhase::Rejected;
                h.confidence = 0.0;
                self.stats.total_rejected += 1;
            }
        }
    }

    /// Set test criteria for a hypothesis
    pub fn set_test_criteria(&mut self, hypothesis_id: u64, criteria: TestCriteria) -> bool {
        if let Some(h) = self.hypotheses.get_mut(&hypothesis_id) {
            h.test_criteria = Some(criteria);
            true
        } else {
            false
        }
    }

    /// Get a hypothesis by ID
    pub fn get_hypothesis(&self, hypothesis_id: u64) -> Option<&Hypothesis> {
        self.hypotheses.get(&hypothesis_id)
    }

    /// Get aggregate stats
    pub fn stats(&self) -> HypothesisStats {
        self.stats
    }
}
