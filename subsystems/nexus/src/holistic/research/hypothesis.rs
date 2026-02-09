// SPDX-License-Identifier: GPL-2.0
//! # Holistic Hypothesis Engine — System-Wide Hypothesis Generation
//!
//! Formulates cross-subsystem hypotheses that no single research module can
//! conceive. While the bridge hypothesis engine wonders about prediction
//! accuracy and the coop engine wonders about fairness, *this* engine
//! formulates span-hypotheses: "Combining bridge traffic prediction with
//! cooperative scheduling reduces tail latency by 20%", or "Application
//! classification feedback improves memory tiering hit rates."
//!
//! Each hypothesis tracks a statement, cross-module evidence chains,
//! testability score, potential impact, and breakthrough probability.
//! A directed hypothesis graph encodes dependency and reinforcement
//! relationships between active hypotheses.
//!
//! The engine that asks "what if?" across the entire kernel.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_HYPOTHESES: usize = 512;
const MAX_EVIDENCE_CHAINS: usize = 64;
const MAX_GRAPH_EDGES: usize = 1024;
const CONFIDENCE_DECAY: f32 = 0.003;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const NOVELTY_THRESHOLD: f32 = 0.65;
const PRUNE_CONFIDENCE_MIN: f32 = 0.08;
const HIGH_CONFIDENCE: f32 = 0.85;
const BREAKTHROUGH_THRESHOLD: f32 = 0.90;
const MIN_EVIDENCE_FOR_RANK: usize = 3;
const CORRELATION_WINDOW: usize = 128;
const IMPACT_DECAY: f32 = 0.98;

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

// ============================================================================
// TYPES
// ============================================================================

/// Lifecycle phase of a holistic hypothesis
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HypothesisPhase {
    Conceived,
    EvidenceGathering,
    Correlated,
    ReadyForExperiment,
    UnderExperiment,
    Confirmed,
    Rejected,
    Archived,
}

/// Origin subsystem that contributed evidence
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceOrigin {
    Bridge,
    Application,
    Cooperation,
    Memory,
    Scheduler,
    Ipc,
    Trust,
    Energy,
}

/// A single piece of cross-module evidence
#[derive(Debug, Clone)]
pub struct CrossEvidence {
    pub id: u64,
    pub origin: EvidenceOrigin,
    pub metric_name: String,
    pub value: f32,
    pub weight: f32,
    pub tick: u64,
}

/// A holistic hypothesis spanning multiple subsystems
#[derive(Debug, Clone)]
pub struct GlobalHypothesis {
    pub id: u64,
    pub statement: String,
    pub phase: HypothesisPhase,
    pub confidence: f32,
    pub evidence: Vec<CrossEvidence>,
    pub origins_involved: Vec<EvidenceOrigin>,
    pub impact_estimate: f32,
    pub testability: f32,
    pub breakthrough_prob: f32,
    pub created_tick: u64,
    pub last_updated: u64,
}

/// Edge in the hypothesis dependency graph
#[derive(Debug, Clone)]
pub struct HypothesisEdge {
    pub from_id: u64,
    pub to_id: u64,
    pub relationship: EdgeRelation,
    pub strength: f32,
}

/// Type of relationship between hypotheses
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EdgeRelation {
    Reinforces,
    Contradicts,
    DependsOn,
    Generalises,
    Specialises,
}

/// Cross-module correlation result
#[derive(Debug, Clone)]
pub struct CorrelationResult {
    pub origin_a: EvidenceOrigin,
    pub origin_b: EvidenceOrigin,
    pub metric_a: String,
    pub metric_b: String,
    pub pearson_r: f32,
    pub samples: usize,
}

/// Hypothesis engine statistics
#[derive(Debug, Clone)]
pub struct HypothesisStats {
    pub total_hypotheses: u64,
    pub active_count: u64,
    pub confirmed_count: u64,
    pub rejected_count: u64,
    pub avg_confidence_ema: f32,
    pub breakthrough_candidates: u64,
    pub graph_edges: u64,
    pub correlations_found: u64,
    pub evidence_items: u64,
    pub pruned_total: u64,
}

// ============================================================================
// HOLISTIC HYPOTHESIS ENGINE
// ============================================================================

/// System-wide hypothesis generation and management engine
pub struct HolisticHypothesisEngine {
    hypotheses: BTreeMap<u64, GlobalHypothesis>,
    graph_edges: Vec<HypothesisEdge>,
    correlations: Vec<CorrelationResult>,
    evidence_buffer: Vec<CrossEvidence>,
    rng_state: u64,
    stats: HypothesisStats,
}

impl HolisticHypothesisEngine {
    /// Create a new holistic hypothesis engine
    pub fn new(seed: u64) -> Self {
        Self {
            hypotheses: BTreeMap::new(),
            graph_edges: Vec::new(),
            correlations: Vec::new(),
            evidence_buffer: Vec::new(),
            rng_state: seed | 1,
            stats: HypothesisStats {
                total_hypotheses: 0,
                active_count: 0,
                confirmed_count: 0,
                rejected_count: 0,
                avg_confidence_ema: 0.0,
                breakthrough_candidates: 0,
                graph_edges: 0,
                correlations_found: 0,
                evidence_items: 0,
                pruned_total: 0,
            },
        }
    }

    /// Generate a new system-wide hypothesis from cross-module evidence
    pub fn generate_global_hypothesis(
        &mut self,
        statement: String,
        origins: Vec<EvidenceOrigin>,
        tick: u64,
    ) -> u64 {
        let id = fnv1a_hash(statement.as_bytes()) ^ fnv1a_hash(&tick.to_le_bytes());
        let testability = xorshift_f32(&mut self.rng_state) * 0.5 + 0.4;
        let impact = xorshift_f32(&mut self.rng_state) * 0.6 + 0.3;
        let hyp = GlobalHypothesis {
            id,
            statement,
            phase: HypothesisPhase::Conceived,
            confidence: 0.50,
            evidence: Vec::new(),
            origins_involved: origins,
            impact_estimate: impact,
            testability,
            breakthrough_prob: 0.0,
            created_tick: tick,
            last_updated: tick,
        };
        self.hypotheses.insert(id, hyp);
        self.stats.total_hypotheses += 1;
        self.refresh_counts();
        id
    }

    /// Detect correlations across module evidence streams
    pub fn cross_module_correlation(&mut self, tick: u64) -> Vec<CorrelationResult> {
        let mut results = Vec::new();
        let origins = [
            EvidenceOrigin::Bridge,
            EvidenceOrigin::Application,
            EvidenceOrigin::Cooperation,
            EvidenceOrigin::Memory,
            EvidenceOrigin::Scheduler,
        ];
        let window = self.evidence_buffer.len().min(CORRELATION_WINDOW);
        if window < 4 {
            return results;
        }
        let recent = &self.evidence_buffer[self.evidence_buffer.len() - window..];
        for i in 0..origins.len() {
            for j in (i + 1)..origins.len() {
                let vals_a: Vec<f32> = recent
                    .iter()
                    .filter(|e| e.origin == origins[i])
                    .map(|e| e.value)
                    .collect();
                let vals_b: Vec<f32> = recent
                    .iter()
                    .filter(|e| e.origin == origins[j])
                    .map(|e| e.value)
                    .collect();
                let n = vals_a.len().min(vals_b.len());
                if n < 3 {
                    continue;
                }
                let r = self.pearson(&vals_a[..n], &vals_b[..n]);
                if r.abs() > 0.3 {
                    let cr = CorrelationResult {
                        origin_a: origins[i],
                        origin_b: origins[j],
                        metric_a: String::from("agg"),
                        metric_b: String::from("agg"),
                        pearson_r: r,
                        samples: n,
                    };
                    results.push(cr.clone());
                    self.correlations.push(cr);
                }
            }
        }
        self.stats.correlations_found = self.correlations.len() as u64;
        let _ = tick;
        results
    }

    /// Fuse evidence from multiple subsystems into active hypotheses
    pub fn evidence_fusion(&mut self, evidence: CrossEvidence) {
        let eid = evidence.id;
        self.evidence_buffer.push(evidence.clone());
        self.stats.evidence_items = self.evidence_buffer.len() as u64;
        for (_, hyp) in self.hypotheses.iter_mut() {
            if hyp.phase == HypothesisPhase::Archived {
                continue;
            }
            if hyp.origins_involved.contains(&evidence.origin) {
                hyp.evidence.push(evidence.clone());
                let new_conf = hyp.confidence + evidence.weight * 0.05;
                hyp.confidence = new_conf.min(1.0);
                hyp.last_updated = evidence.tick;
                if hyp.evidence.len() >= MIN_EVIDENCE_FOR_RANK
                    && hyp.phase == HypothesisPhase::Conceived
                {
                    hyp.phase = HypothesisPhase::EvidenceGathering;
                }
            }
        }
        let _ = eid;
    }

    /// Rank hypotheses by testability × expected impact
    pub fn testability_rank(&self) -> Vec<(u64, f32)> {
        let mut ranked: Vec<(u64, f32)> = self
            .hypotheses
            .iter()
            .filter(|(_, h)| {
                h.phase != HypothesisPhase::Archived && h.phase != HypothesisPhase::Rejected
            })
            .map(|(&id, h)| {
                let score = h.testability * h.impact_estimate * h.confidence;
                (id, score)
            })
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        ranked
    }

    /// Build the directed hypothesis dependency graph
    pub fn hypothesis_graph(&mut self) -> &[HypothesisEdge] {
        self.graph_edges.clear();
        let ids: Vec<u64> = self.hypotheses.keys().copied().collect();
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                if self.graph_edges.len() >= MAX_GRAPH_EDGES {
                    break;
                }
                let ha = &self.hypotheses[&ids[i]];
                let hb = &self.hypotheses[&ids[j]];
                let overlap = ha
                    .origins_involved
                    .iter()
                    .filter(|o| hb.origins_involved.contains(o))
                    .count();
                if overlap > 0 {
                    let strength = overlap as f32 / ha.origins_involved.len().max(1) as f32;
                    let rel = if (ha.confidence - hb.confidence).abs() < 0.1 {
                        EdgeRelation::Reinforces
                    } else {
                        EdgeRelation::DependsOn
                    };
                    self.graph_edges.push(HypothesisEdge {
                        from_id: ids[i],
                        to_id: ids[j],
                        relationship: rel,
                        strength,
                    });
                }
            }
        }
        self.stats.graph_edges = self.graph_edges.len() as u64;
        &self.graph_edges
    }

    /// Estimate breakthrough potential for each hypothesis
    pub fn breakthrough_potential(&mut self) -> Vec<(u64, f32)> {
        let mut results = Vec::new();
        let mut candidates = 0u64;
        for (&id, hyp) in self.hypotheses.iter_mut() {
            let novelty = if hyp.origins_involved.len() >= 3 {
                0.3
            } else {
                0.0
            };
            let bp = hyp.impact_estimate * hyp.confidence * (1.0 + novelty);
            hyp.breakthrough_prob = bp.min(1.0);
            if bp > BREAKTHROUGH_THRESHOLD {
                candidates += 1;
            }
            results.push((id, bp));
        }
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        self.stats.breakthrough_candidates = candidates;
        results
    }

    /// Current statistics snapshot
    pub fn stats(&self) -> &HypothesisStats {
        &self.stats
    }

    /// Decay confidence on stale hypotheses and prune dead ones
    pub fn decay_and_prune(&mut self, tick: u64) {
        let mut to_prune = Vec::new();
        for (&id, hyp) in self.hypotheses.iter_mut() {
            let age = tick.saturating_sub(hyp.last_updated);
            hyp.confidence -= CONFIDENCE_DECAY * age as f32 * 0.001;
            hyp.confidence = hyp.confidence.max(0.0);
            if hyp.confidence < PRUNE_CONFIDENCE_MIN && hyp.phase != HypothesisPhase::Confirmed {
                to_prune.push(id);
            }
        }
        for id in &to_prune {
            self.hypotheses.remove(id);
            self.stats.pruned_total += 1;
        }
        self.refresh_counts();
    }

    // ── private helpers ─────────────────────────────────────────────────

    fn refresh_counts(&mut self) {
        let mut active = 0u64;
        let mut confirmed = 0u64;
        let mut rejected = 0u64;
        let mut conf_sum = 0.0f32;
        for (_, h) in &self.hypotheses {
            match h.phase {
                HypothesisPhase::Confirmed => confirmed += 1,
                HypothesisPhase::Rejected => rejected += 1,
                HypothesisPhase::Archived => {},
                _ => active += 1,
            }
            conf_sum += h.confidence;
        }
        self.stats.active_count = active;
        self.stats.confirmed_count = confirmed;
        self.stats.rejected_count = rejected;
        let total = self.hypotheses.len().max(1) as f32;
        let avg = conf_sum / total;
        self.stats.avg_confidence_ema =
            EMA_ALPHA * avg + (1.0 - EMA_ALPHA) * self.stats.avg_confidence_ema;
    }

    fn pearson(&self, a: &[f32], b: &[f32]) -> f32 {
        let n = a.len().min(b.len()) as f32;
        if n < 2.0 {
            return 0.0;
        }
        let mean_a: f32 = a.iter().sum::<f32>() / n;
        let mean_b: f32 = b.iter().sum::<f32>() / n;
        let mut cov = 0.0f32;
        let mut var_a = 0.0f32;
        let mut var_b = 0.0f32;
        for i in 0..a.len().min(b.len()) {
            let da = a[i] - mean_a;
            let db = b[i] - mean_b;
            cov += da * db;
            var_a += da * da;
            var_b += db * db;
        }
        let denom = (var_a * var_b).sqrt();
        if denom < 1e-9 { 0.0 } else { cov / denom }
    }
}
