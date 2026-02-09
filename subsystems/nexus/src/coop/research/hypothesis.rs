// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Hypothesis Engine — Scientific Hypothesis Generation
//!
//! Observes anomalies and contention patterns in cooperation telemetry, then
//! formulates testable hypotheses about protocol improvements. "Auction-based
//! allocation may be fairer than fixed quotas under high contention." Each
//! hypothesis carries a statement, supporting evidence, a confidence level,
//! and explicit test criteria. The engine ranks hypotheses by expected
//! impact × testability and manages their lifecycle from formation through
//! confirmation or rejection.
//!
//! The engine that asks "what if?" about every cooperation pattern.

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
const HIGH_CONFIDENCE: f32 = 0.85;
const LOW_CONFIDENCE: f32 = 0.30;

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

/// Lifecycle phase of a cooperation hypothesis
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

/// Type of cooperation anomaly that triggered hypothesis generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CoopAnomaly {
    FairnessDrop,
    StarvationDetected,
    NegotiationTimeout,
    TrustDecaySpike,
    ContentionSurge,
    ResourceWaste,
    ProtocolInefficiency,
}

/// A single piece of evidence supporting or refuting a hypothesis
#[derive(Debug, Clone)]
pub struct Evidence {
    pub id: u64,
    pub hypothesis_id: u64,
    pub tick: u64,
    pub metric_name: String,
    pub observed_value: f32,
    pub expected_value: f32,
    pub weight: f32,
    pub supports: bool,
}

/// A cooperation hypothesis with lifecycle tracking
#[derive(Debug, Clone)]
pub struct CoopHypothesis {
    pub id: u64,
    pub statement: String,
    pub phase: HypothesisPhase,
    pub confidence: f32,
    pub evidence: Vec<Evidence>,
    pub anomaly_trigger: CoopAnomaly,
    pub created_tick: u64,
    pub last_updated_tick: u64,
    pub expected_impact: f32,
    pub testability: f32,
    pub priority_score: f32,
}

/// Test criteria for a hypothesis
#[derive(Debug, Clone)]
pub struct TestCriteria {
    pub hypothesis_id: u64,
    pub metric_name: String,
    pub success_threshold: f32,
    pub sample_count_required: usize,
    pub max_duration_ticks: u64,
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
    pub active_count: u64,
    pub avg_confidence_ema: f32,
    pub evidence_collected: u64,
    pub anomalies_observed: u64,
    pub highest_confidence: f32,
    pub confirmation_rate_ema: f32,
}

// ============================================================================
// ANOMALY TRACKER
// ============================================================================

#[derive(Debug, Clone)]
struct AnomalyRecord {
    anomaly: CoopAnomaly,
    tick: u64,
    severity: f32,
    hash: u64,
}

#[derive(Debug)]
struct AnomalyTracker {
    records: Vec<AnomalyRecord>,
    frequency: BTreeMap<u64, u32>,
    severity_ema: f32,
}

impl AnomalyTracker {
    fn new() -> Self {
        Self {
            records: Vec::new(),
            frequency: BTreeMap::new(),
            severity_ema: 0.0,
        }
    }

    fn record(&mut self, anomaly: CoopAnomaly, tick: u64, severity: f32) -> u64 {
        let hash = fnv1a_hash(&(anomaly as u64).to_le_bytes()) ^ fnv1a_hash(&tick.to_le_bytes());
        self.severity_ema = EMA_ALPHA * severity + (1.0 - EMA_ALPHA) * self.severity_ema;
        let key = anomaly as u64;
        let count = self.frequency.entry(key).or_insert(0);
        *count += 1;
        self.records.push(AnomalyRecord {
            anomaly,
            tick,
            severity,
            hash,
        });
        while self.records.len() > MAX_ANOMALIES {
            self.records.remove(0);
        }
        hash
    }
}

// ============================================================================
// COOPERATION HYPOTHESIS ENGINE
// ============================================================================

/// Autonomous cooperation hypothesis generation engine
#[derive(Debug)]
pub struct CoopHypothesisEngine {
    hypotheses: BTreeMap<u64, CoopHypothesis>,
    anomaly_tracker: AnomalyTracker,
    tick: u64,
    rng_state: u64,
    stats: HypothesisStats,
}

impl CoopHypothesisEngine {
    /// Create a new hypothesis engine with the given PRNG seed
    pub fn new(seed: u64) -> Self {
        Self {
            hypotheses: BTreeMap::new(),
            anomaly_tracker: AnomalyTracker::new(),
            tick: 0,
            rng_state: seed | 1,
            stats: HypothesisStats::default(),
        }
    }

    /// Generate a hypothesis from an observed cooperation anomaly
    pub fn generate_hypothesis(
        &mut self,
        anomaly: CoopAnomaly,
        statement: String,
        severity: f32,
    ) -> CoopHypothesis {
        self.tick += 1;
        let anom_hash = self.anomaly_tracker.record(anomaly, self.tick, severity);
        self.stats.anomalies_observed += 1;

        let id = fnv1a_hash(statement.as_bytes()) ^ anom_hash ^ xorshift64(&mut self.rng_state);

        // Novelty check — avoid duplicate hypotheses
        let existing_similar = self.hypotheses.values().any(|h| {
            let sim_hash = fnv1a_hash(h.statement.as_bytes());
            let new_hash = fnv1a_hash(statement.as_bytes());
            let xor_dist = (sim_hash ^ new_hash).count_ones();
            xor_dist < 8
        });

        let testability = if existing_similar {
            0.3
        } else {
            0.5 + xorshift64(&mut self.rng_state) as f32 % 50.0 / 100.0
        };
        let expected_impact = severity.clamp(0.0, 1.0) * 0.7
            + self.anomaly_tracker.severity_ema * 0.3;
        let priority = expected_impact * testability;

        let hyp = CoopHypothesis {
            id,
            statement,
            phase: HypothesisPhase::Formulated,
            confidence: 0.5,
            evidence: Vec::new(),
            anomaly_trigger: anomaly,
            created_tick: self.tick,
            last_updated_tick: self.tick,
            expected_impact,
            testability,
            priority_score: priority,
        };

        if self.hypotheses.len() < MAX_HYPOTHESES {
            self.hypotheses.insert(id, hyp.clone());
            self.stats.total_generated += 1;
            self.stats.active_count = self.hypotheses.len() as u64;
        }
        hyp
    }

    /// Formulate test criteria for a hypothesis
    pub fn formulate_test(
        &mut self,
        hypothesis_id: u64,
        metric_name: String,
        threshold: f32,
        sample_count: usize,
    ) -> Option<TestCriteria> {
        let hyp = self.hypotheses.get_mut(&hypothesis_id)?;
        hyp.phase = HypothesisPhase::ReadyForTest;
        hyp.last_updated_tick = self.tick;
        Some(TestCriteria {
            hypothesis_id,
            metric_name,
            success_threshold: threshold,
            sample_count_required: sample_count,
            max_duration_ticks: 10_000,
        })
    }

    /// Gather a piece of evidence for a hypothesis
    pub fn gather_evidence(
        &mut self,
        hypothesis_id: u64,
        metric_name: String,
        observed: f32,
        expected: f32,
    ) -> bool {
        self.tick += 1;
        let hyp = match self.hypotheses.get_mut(&hypothesis_id) {
            Some(h) => h,
            None => return false,
        };
        if hyp.evidence.len() >= MAX_EVIDENCE_PER_HYPOTHESIS {
            hyp.evidence.remove(0);
        }
        let supports = (observed - expected).abs() < expected * 0.15;
        let weight_raw = 1.0 - (observed - expected).abs() / (expected.abs() + 1.0);
        let weight = weight_raw.clamp(0.0, 1.0);

        let eid = fnv1a_hash(&hypothesis_id.to_le_bytes()) ^ fnv1a_hash(&self.tick.to_le_bytes());
        hyp.evidence.push(Evidence {
            id: eid,
            hypothesis_id,
            tick: self.tick,
            metric_name,
            observed_value: observed,
            expected_value: expected,
            weight,
            supports,
        });

        // Update confidence using EMA of evidence weights
        let supporting: f32 = hyp
            .evidence
            .iter()
            .filter(|e| e.supports)
            .map(|e| e.weight)
            .sum();
        let total: f32 = hyp.evidence.iter().map(|e| e.weight).sum();
        let ratio = if total > 0.0 {
            supporting / total
        } else {
            0.5
        };
        hyp.confidence = EVIDENCE_WEIGHT_EMA * ratio + (1.0 - EVIDENCE_WEIGHT_EMA) * hyp.confidence;
        hyp.last_updated_tick = self.tick;

        if hyp.phase == HypothesisPhase::ReadyForTest {
            hyp.phase = HypothesisPhase::UnderTest;
        }

        self.stats.evidence_collected += 1;
        if hyp.confidence > self.stats.highest_confidence {
            self.stats.highest_confidence = hyp.confidence;
        }
        self.stats.avg_confidence_ema =
            EMA_ALPHA * hyp.confidence + (1.0 - EMA_ALPHA) * self.stats.avg_confidence_ema;
        supports
    }

    /// Compute a confidence interval for a hypothesis
    pub fn confidence_interval(&self, hypothesis_id: u64) -> Option<(f32, f32, f32)> {
        let hyp = self.hypotheses.get(&hypothesis_id)?;
        if hyp.evidence.len() < 2 {
            return Some((hyp.confidence, 0.0, 1.0));
        }
        let n = hyp.evidence.len() as f32;
        let weights: Vec<f32> = hyp.evidence.iter().map(|e| e.weight).collect();
        let mean = weights.iter().sum::<f32>() / n;
        let variance = weights.iter().map(|w| (w - mean) * (w - mean)).sum::<f32>() / (n - 1.0);
        let std_err = if n > 1.0 {
            (variance / n).max(0.0)
        } else {
            0.0
        };
        let half_width = 1.96 * std_err; // ~95% CI
        let lower = (hyp.confidence - half_width).max(0.0);
        let upper = (hyp.confidence + half_width).min(1.0);
        Some((hyp.confidence, lower, upper))
    }

    /// Get the current status of a hypothesis
    pub fn hypothesis_status(&mut self, hypothesis_id: u64) -> Option<HypothesisPhase> {
        let hyp = self.hypotheses.get_mut(&hypothesis_id)?;

        // Auto-transition based on evidence count and confidence
        if hyp.phase == HypothesisPhase::UnderTest
            && hyp.evidence.len() >= MIN_EVIDENCE_FOR_RANKING
        {
            if hyp.confidence >= HIGH_CONFIDENCE {
                hyp.phase = HypothesisPhase::Confirmed;
                self.stats.total_confirmed += 1;
                let total = self.stats.total_confirmed + self.stats.total_rejected;
                let rate = self.stats.total_confirmed as f32 / total.max(1) as f32;
                self.stats.confirmation_rate_ema =
                    EMA_ALPHA * rate + (1.0 - EMA_ALPHA) * self.stats.confirmation_rate_ema;
            } else if hyp.confidence < LOW_CONFIDENCE {
                hyp.phase = HypothesisPhase::Rejected;
                self.stats.total_rejected += 1;
            }
        }

        // Decay confidence for stale hypotheses
        if self.tick > hyp.last_updated_tick + PRUNE_AGE_TICKS
            && hyp.phase != HypothesisPhase::Confirmed
            && hyp.phase != HypothesisPhase::Rejected
        {
            hyp.confidence =
                (hyp.confidence - CONFIDENCE_DECAY_RATE).max(0.0);
            if hyp.confidence < PRUNE_CONFIDENCE_MIN {
                hyp.phase = HypothesisPhase::Archived;
                self.stats.total_archived += 1;
            }
        }

        self.stats.active_count = self
            .hypotheses
            .values()
            .filter(|h| {
                h.phase != HypothesisPhase::Archived
                    && h.phase != HypothesisPhase::Confirmed
                    && h.phase != HypothesisPhase::Rejected
            })
            .count() as u64;

        Some(hyp.phase)
    }

    /// Get current hypothesis engine statistics
    pub fn stats(&self) -> &HypothesisStats {
        &self.stats
    }
}
