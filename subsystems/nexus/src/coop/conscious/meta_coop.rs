// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Meta-Cognition
//!
//! Meta-reasoning about cooperation protocols. Evaluates whether current
//! protocols are optimal, whether fairness is genuinely achieved rather
//! than merely reported, and whether negotiation strategies could be improved.
//!
//! This is the recursive layer: the cooperation engine reasoning about its
//! own cooperation reasoning, optimizing the optimizer, and detecting blind
//! spots in its fairness algorithms.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.10;
const MAX_PROTOCOLS: usize = 64;
const MAX_EVALUATIONS: usize = 256;
const OPTIMIZATION_THRESHOLD: f32 = 0.70;
const COGNITIVE_LOAD_DECAY: f32 = 0.95;
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

// ============================================================================
// PROTOCOL EVALUATION TYPES
// ============================================================================

/// A cooperation protocol under evaluation
#[derive(Debug, Clone)]
pub struct ProtocolModel {
    pub name: String,
    pub id: u64,
    /// Effectiveness score (0.0 – 1.0)
    pub effectiveness: f32,
    /// Fairness score (0.0 – 1.0)
    pub fairness: f32,
    /// Efficiency: overhead vs. value delivered
    pub efficiency: f32,
    /// Negotiation success rate under this protocol
    pub success_rate: f32,
    /// Number of evaluations
    pub evaluations: u64,
    /// Variance in effectiveness
    pub variance: f32,
    /// Is this protocol currently optimal?
    pub is_optimal: bool,
}

/// A snapshot of protocol evaluation
#[derive(Debug, Clone)]
pub struct ProtocolEvaluation {
    pub protocol_id: u64,
    pub tick: u64,
    pub effectiveness: f32,
    pub fairness: f32,
    pub efficiency: f32,
    pub recommendation: OptimizationAction,
}

/// Possible optimization actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationAction {
    /// Protocol is performing well, keep it
    Retain,
    /// Tune parameters for better performance
    Tune,
    /// Replace with alternative protocol
    Replace,
    /// Merge with another protocol
    Merge,
    /// Deprecate — not providing value
    Deprecate,
}

/// Meta-negotiation insight: can we negotiate the negotiation process better?
#[derive(Debug, Clone)]
pub struct MetaNegotiationInsight {
    pub id: u64,
    pub description: String,
    /// Potential improvement (0.0 – 1.0)
    pub improvement_potential: f32,
    /// Confidence in this insight
    pub confidence: f32,
    /// Number of supporting observations
    pub evidence_count: u64,
}

// ============================================================================
// META-COGNITION STATS
// ============================================================================

/// Aggregate meta-cognition statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct MetaCognitionStats {
    pub protocols_tracked: usize,
    pub optimal_protocols: usize,
    pub avg_effectiveness: f32,
    pub avg_fairness: f32,
    pub cognitive_load: f32,
    pub cognitive_efficiency: f32,
    pub meta_insights: usize,
    pub optimization_actions_pending: usize,
}

// ============================================================================
// COOPERATION META-COGNITION ENGINE
// ============================================================================

/// Meta-reasoning about cooperation protocols — evaluating optimality,
/// analyzing fairness at a meta-level, and optimizing the cooperation engine.
#[derive(Debug)]
pub struct CoopMetaCognition {
    /// Protocol models (keyed by FNV hash)
    protocols: BTreeMap<u64, ProtocolModel>,
    /// Evaluation history ring buffer
    evaluations: Vec<ProtocolEvaluation>,
    eval_write_idx: usize,
    /// Meta-negotiation insights (keyed by FNV hash)
    insights: BTreeMap<u64, MetaNegotiationInsight>,
    /// Current cognitive load (0.0 – 1.0)
    cognitive_load: f32,
    /// Monotonic tick
    tick: u64,
    /// PRNG state for exploration
    rng_state: u64,
    /// Previous global effectiveness for computing deltas
    prev_effectiveness: f32,
    /// EMA of cognitive efficiency
    efficiency_ema: f32,
}

impl CoopMetaCognition {
    pub fn new() -> Self {
        Self {
            protocols: BTreeMap::new(),
            evaluations: Vec::new(),
            eval_write_idx: 0,
            insights: BTreeMap::new(),
            cognitive_load: 0.0,
            tick: 0,
            rng_state: 0xAE7A_C00B_CAFE_BABE,
            prev_effectiveness: 0.5,
            efficiency_ema: 0.5,
        }
    }

    /// Evaluate all tracked protocols and update optimality flags
    pub fn evaluate_protocols(&mut self) -> usize {
        self.tick += 1;
        self.cognitive_load = COGNITIVE_LOAD_DECAY * self.cognitive_load + 0.05;
        let mut actions = 0_usize;

        let ids: Vec<u64> = self.protocols.keys().copied().collect();
        for id in ids {
            if let Some(proto) = self.protocols.get(&id) {
                let composite = proto.effectiveness * 0.35
                    + proto.fairness * 0.35
                    + proto.efficiency * 0.15
                    + proto.success_rate * 0.15;

                let action = if composite >= 0.85 {
                    OptimizationAction::Retain
                } else if composite >= OPTIMIZATION_THRESHOLD {
                    OptimizationAction::Tune
                } else if composite >= 0.50 {
                    OptimizationAction::Replace
                } else if composite >= 0.30 {
                    OptimizationAction::Merge
                } else {
                    OptimizationAction::Deprecate
                };

                if action != OptimizationAction::Retain {
                    actions += 1;
                }

                let evaluation = ProtocolEvaluation {
                    protocol_id: id,
                    tick: self.tick,
                    effectiveness: proto.effectiveness,
                    fairness: proto.fairness,
                    efficiency: proto.efficiency,
                    recommendation: action,
                };

                if self.evaluations.len() < MAX_EVALUATIONS {
                    self.evaluations.push(evaluation);
                } else {
                    self.evaluations[self.eval_write_idx] = evaluation;
                }
                self.eval_write_idx = (self.eval_write_idx + 1) % MAX_EVALUATIONS;

                // Update optimality flag
                if let Some(proto) = self.protocols.get_mut(&id) {
                    proto.is_optimal = action == OptimizationAction::Retain;
                }
            }
        }
        actions
    }

    /// Meta-analysis of fairness: is our fairness measurement itself fair?
    pub fn fairness_meta_analysis(&self) -> f32 {
        if self.protocols.is_empty() {
            return 0.5;
        }

        let fairness_scores: Vec<f32> = self.protocols.values().map(|p| p.fairness).collect();
        let n = fairness_scores.len() as f32;
        let mean = fairness_scores.iter().sum::<f32>() / n;

        // Variance of fairness scores — low variance means consistent fairness
        let variance = fairness_scores
            .iter()
            .map(|f| (f - mean) * (f - mean))
            .sum::<f32>()
            / n;

        // Meta-fairness: high mean fairness + low variance = genuinely fair
        let consistency = 1.0 - libm::sqrtf(variance).min(1.0);
        mean * 0.60 + consistency * 0.40
    }

    /// Suggest protocol optimizations based on evaluation history
    pub fn protocol_optimization(&mut self, name: &str, effectiveness: f32, fairness: f32, efficiency: f32, success_rate: f32) {
        self.tick += 1;
        let id = fnv1a_hash(name.as_bytes());

        let proto = self.protocols.entry(id).or_insert_with(|| ProtocolModel {
            name: String::from(name),
            id,
            effectiveness: 0.5,
            fairness: 0.5,
            efficiency: 0.5,
            success_rate: 0.5,
            evaluations: 0,
            variance: 0.0,
            is_optimal: false,
        });

        let diff = effectiveness - proto.effectiveness;
        proto.variance = EMA_ALPHA * diff * diff + (1.0 - EMA_ALPHA) * proto.variance;

        proto.effectiveness = EMA_ALPHA * effectiveness.max(0.0).min(1.0)
            + (1.0 - EMA_ALPHA) * proto.effectiveness;
        proto.fairness =
            EMA_ALPHA * fairness.max(0.0).min(1.0) + (1.0 - EMA_ALPHA) * proto.fairness;
        proto.efficiency =
            EMA_ALPHA * efficiency.max(0.0).min(1.0) + (1.0 - EMA_ALPHA) * proto.efficiency;
        proto.success_rate = EMA_ALPHA * success_rate.max(0.0).min(1.0)
            + (1.0 - EMA_ALPHA) * proto.success_rate;
        proto.evaluations += 1;
    }

    /// Meta-negotiate: reason about how to improve the negotiation process itself
    pub fn meta_negotiate(&mut self) -> usize {
        self.tick += 1;
        let mut new_insights = 0_usize;

        // Insight 1: Are protocols converging on fairness?
        let fairness_scores: Vec<f32> = self.protocols.values().map(|p| p.fairness).collect();
        if !fairness_scores.is_empty() {
            let mean_fairness = fairness_scores.iter().sum::<f32>() / fairness_scores.len() as f32;
            if mean_fairness < 0.7 {
                let id = fnv1a_hash(b"fairness_convergence_gap");
                let insight = self.insights.entry(id).or_insert_with(|| {
                    new_insights += 1;
                    MetaNegotiationInsight {
                        id,
                        description: String::from("Fairness below target across protocols"),
                        improvement_potential: 1.0 - mean_fairness,
                        confidence: 0.5,
                        evidence_count: 0,
                    }
                });
                insight.evidence_count += 1;
                insight.confidence = EMA_ALPHA * 0.8 + (1.0 - EMA_ALPHA) * insight.confidence;
            }
        }

        // Insight 2: Are there protocols that never improve?
        for proto in self.protocols.values() {
            if proto.evaluations > 10 && proto.effectiveness < 0.5 {
                let id = fnv1a_hash(proto.name.as_bytes()) ^ fnv1a_hash(b"stagnant");
                let insight = self.insights.entry(id).or_insert_with(|| {
                    new_insights += 1;
                    MetaNegotiationInsight {
                        id,
                        description: String::from("Stagnant protocol detected"),
                        improvement_potential: 0.8,
                        confidence: 0.6,
                        evidence_count: 0,
                    }
                });
                insight.evidence_count += 1;
            }
        }

        // Insight 3: Cognitive overload check
        if self.cognitive_load > 0.8 {
            let id = fnv1a_hash(b"cognitive_overload");
            let insight = self.insights.entry(id).or_insert_with(|| {
                new_insights += 1;
                MetaNegotiationInsight {
                    id,
                    description: String::from("Meta-cognitive overload risk"),
                    improvement_potential: self.cognitive_load - 0.6,
                    confidence: 0.7,
                    evidence_count: 0,
                }
            });
            insight.evidence_count += 1;
        }

        new_insights
    }

    /// Cognitive efficiency: value produced per unit of meta-cognitive effort
    pub fn cognitive_efficiency(&mut self) -> f32 {
        if self.protocols.is_empty() {
            return 0.0;
        }

        let total_effectiveness: f32 = self.protocols.values().map(|p| p.effectiveness).sum();
        let avg_effectiveness = total_effectiveness / self.protocols.len() as f32;

        let delta = avg_effectiveness - self.prev_effectiveness;
        self.prev_effectiveness = avg_effectiveness;

        // Efficiency = improvement per cognitive load unit
        let raw_efficiency = if self.cognitive_load > f32::EPSILON {
            (delta.max(0.0) / self.cognitive_load).min(1.0)
        } else {
            0.5
        };

        self.efficiency_ema = EMA_ALPHA * raw_efficiency + (1.0 - EMA_ALPHA) * self.efficiency_ema;
        self.efficiency_ema
    }

    /// Get aggregate statistics
    pub fn stats(&self) -> MetaCognitionStats {
        let optimal = self.protocols.values().filter(|p| p.is_optimal).count();
        let avg_eff = if self.protocols.is_empty() {
            0.0
        } else {
            self.protocols.values().map(|p| p.effectiveness).sum::<f32>()
                / self.protocols.len() as f32
        };
        let avg_fair = if self.protocols.is_empty() {
            0.0
        } else {
            self.protocols.values().map(|p| p.fairness).sum::<f32>()
                / self.protocols.len() as f32
        };

        let pending = self
            .evaluations
            .iter()
            .filter(|e| e.recommendation != OptimizationAction::Retain)
            .count();

        MetaCognitionStats {
            protocols_tracked: self.protocols.len(),
            optimal_protocols: optimal,
            avg_effectiveness: avg_eff,
            avg_fairness: avg_fair,
            cognitive_load: self.cognitive_load,
            cognitive_efficiency: self.efficiency_ema,
            meta_insights: self.insights.len(),
            optimization_actions_pending: pending,
        }
    }
}
