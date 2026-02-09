// SPDX-License-Identifier: GPL-2.0
//! # Holistic Introspector
//!
//! System-wide introspection engine. Analyzes ALL decisions across bridge,
//! application, and cooperative subsystems for coherence, quality, and
//! optimality. Builds a global reasoning graph and scans for blind spots
//! that no single subsystem can detect on its own.
//!
//! Where sub-introspectors see trees, this module sees the forest.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_DECISIONS: usize = 1024;
const MAX_COHERENCE_PAIRS: usize = 256;
const MAX_BLIND_SPOTS: usize = 64;
const EMA_ALPHA: f32 = 0.10;
const COHERENCE_THRESHOLD: f32 = 0.70;
const OPTIMALITY_THRESHOLD: f32 = 0.60;
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
// DECISION ORIGIN
// ============================================================================

/// Which subsystem originated a decision
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DecisionOrigin {
    Bridge,
    Application,
    Cooperative,
    Memory,
    Scheduler,
    Network,
    Security,
    Holistic,
}

/// Quality rating for a global decision
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QualityRating {
    Poor = 0,
    BelowAverage = 1,
    Average = 2,
    Good = 3,
    Excellent = 4,
}

// ============================================================================
// GLOBAL DECISION RECORD
// ============================================================================

/// A decision record visible across the entire system
#[derive(Debug, Clone)]
pub struct GlobalDecision {
    pub id: u64,
    pub origin: DecisionOrigin,
    pub tick: u64,
    pub description: String,
    pub confidence: f32,
    pub outcome_score: f32,
    pub resolved: bool,
    pub reasoning_depth: u32,
    pub alternatives_count: u32,
    pub cross_module_effects: Vec<u8>,
}

/// A coherence pair measuring alignment between two subsystems
#[derive(Debug, Clone)]
pub struct CoherencePair {
    pub origin_a: DecisionOrigin,
    pub origin_b: DecisionOrigin,
    pub alignment_score: f32,
    pub conflict_count: u64,
    pub synergy_count: u64,
    pub sample_count: u64,
}

/// A detected blind spot in the system's reasoning
#[derive(Debug, Clone)]
pub struct BlindSpot {
    pub id: u64,
    pub description: String,
    pub affected_origins: Vec<DecisionOrigin>,
    pub severity: f32,
    pub detection_tick: u64,
    pub evidence_strength: f32,
}

/// Reasoning graph node connecting decisions
#[derive(Debug, Clone)]
pub struct ReasoningNode {
    pub decision_id: u64,
    pub origin: DecisionOrigin,
    pub depends_on: Vec<u64>,
    pub influences: Vec<u64>,
    pub centrality: f32,
}

// ============================================================================
// STATS
// ============================================================================

/// Aggregate introspection statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct IntrospectionStats {
    pub total_decisions_audited: u64,
    pub avg_coherence: f32,
    pub avg_optimality: f32,
    pub avg_reasoning_depth: f32,
    pub blind_spot_count: usize,
    pub cross_module_conflict_rate: f32,
    pub decision_quality_ema: f32,
    pub graph_density: f32,
}

// ============================================================================
// HOLISTIC INTROSPECTOR
// ============================================================================

/// System-wide introspection engine. Audits decisions across all subsystems,
/// measures cross-module coherence, computes decision optimality, and scans
/// for blind spots invisible to individual modules.
#[derive(Debug)]
pub struct HolisticIntrospector {
    decisions: Vec<GlobalDecision>,
    write_idx: usize,
    coherence_pairs: BTreeMap<u16, CoherencePair>,
    blind_spots: BTreeMap<u64, BlindSpot>,
    reasoning_graph: BTreeMap<u64, ReasoningNode>,
    total_audited: u64,
    tick: u64,
    rng_state: u64,
    quality_ema: f32,
    coherence_ema: f32,
    optimality_ema: f32,
    depth_ema: f32,
}

impl HolisticIntrospector {
    pub fn new() -> Self {
        Self {
            decisions: Vec::new(),
            write_idx: 0,
            coherence_pairs: BTreeMap::new(),
            blind_spots: BTreeMap::new(),
            reasoning_graph: BTreeMap::new(),
            total_audited: 0,
            tick: 0,
            rng_state: 0xABCD_1234_5678_EF00,
            quality_ema: 0.5,
            coherence_ema: 0.5,
            optimality_ema: 0.5,
            depth_ema: 2.0,
        }
    }

    /// Record a decision from any subsystem for global auditing
    pub fn record_decision(
        &mut self,
        origin: DecisionOrigin,
        description: String,
        confidence: f32,
        reasoning_depth: u32,
        alternatives: u32,
        cross_effects: Vec<u8>,
    ) -> u64 {
        self.tick += 1;
        self.total_audited += 1;

        let id = fnv1a_hash(description.as_bytes()) ^ xorshift64(&mut self.rng_state);

        let decision = GlobalDecision {
            id,
            origin,
            tick: self.tick,
            description,
            confidence: confidence.clamp(0.0, 1.0),
            outcome_score: 0.0,
            resolved: false,
            reasoning_depth,
            alternatives_count: alternatives,
            cross_module_effects: cross_effects,
        };

        self.depth_ema =
            EMA_ALPHA * reasoning_depth as f32 + (1.0 - EMA_ALPHA) * self.depth_ema;

        if self.decisions.len() < MAX_DECISIONS {
            self.decisions.push(decision);
        } else {
            self.decisions[self.write_idx] = decision;
        }
        self.write_idx = (self.write_idx + 1) % MAX_DECISIONS;

        let node = ReasoningNode {
            decision_id: id,
            origin,
            depends_on: Vec::new(),
            influences: Vec::new(),
            centrality: 0.0,
        };
        if self.reasoning_graph.len() < MAX_DECISIONS {
            self.reasoning_graph.insert(id, node);
        }

        id
    }

    /// Resolve a decision outcome
    pub fn resolve_decision(&mut self, decision_id: u64, outcome: f32) {
        let clamped = outcome.clamp(0.0, 1.0);
        for d in self.decisions.iter_mut() {
            if d.id == decision_id {
                d.outcome_score = clamped;
                d.resolved = true;
                self.quality_ema =
                    EMA_ALPHA * clamped + (1.0 - EMA_ALPHA) * self.quality_ema;
                break;
            }
        }
    }

    /// Link decisions in the reasoning graph
    pub fn link_decisions(&mut self, cause_id: u64, effect_id: u64) {
        if let Some(cause) = self.reasoning_graph.get_mut(&cause_id) {
            if !cause.influences.contains(&effect_id) {
                cause.influences.push(effect_id);
            }
        }
        if let Some(effect) = self.reasoning_graph.get_mut(&effect_id) {
            if !effect.depends_on.contains(&cause_id) {
                effect.depends_on.push(cause_id);
            }
        }
    }

    /// Audit all decisions globally — returns quality score
    pub fn global_decision_audit(&mut self) -> f32 {
        if self.decisions.is_empty() {
            return 0.5;
        }

        let resolved: Vec<&GlobalDecision> =
            self.decisions.iter().filter(|d| d.resolved).collect();
        if resolved.is_empty() {
            return self.quality_ema;
        }

        let avg_outcome =
            resolved.iter().map(|d| d.outcome_score).sum::<f32>() / resolved.len() as f32;
        let avg_cal = resolved
            .iter()
            .map(|d| (d.confidence - d.outcome_score).abs())
            .sum::<f32>()
            / resolved.len() as f32;

        let audit_score = avg_outcome * 0.6 + (1.0 - avg_cal) * 0.4;
        self.quality_ema =
            EMA_ALPHA * audit_score + (1.0 - EMA_ALPHA) * self.quality_ema;
        self.quality_ema
    }

    /// Measure cross-module coherence between all subsystem pairs
    pub fn cross_module_coherence(&mut self) -> f32 {
        let mut origin_outcomes: BTreeMap<u8, Vec<f32>> = BTreeMap::new();
        for d in self.decisions.iter().filter(|d| d.resolved) {
            origin_outcomes
                .entry(d.origin as u8)
                .or_insert_with(Vec::new)
                .push(d.outcome_score);
        }

        let origins: Vec<u8> = origin_outcomes.keys().copied().collect();
        let mut total_alignment = 0.0;
        let mut pair_count = 0u32;

        for i in 0..origins.len() {
            for j in (i + 1)..origins.len() {
                let a_avg = avg_slice(origin_outcomes.get(&origins[i]).unwrap_or(&Vec::new()));
                let b_avg = avg_slice(origin_outcomes.get(&origins[j]).unwrap_or(&Vec::new()));
                let alignment = 1.0 - (a_avg - b_avg).abs();

                let pair_key = (origins[i] as u16) << 8 | origins[j] as u16;
                let pair = self.coherence_pairs.entry(pair_key).or_insert(CoherencePair {
                    origin_a: int_to_origin(origins[i]),
                    origin_b: int_to_origin(origins[j]),
                    alignment_score: 0.5,
                    conflict_count: 0,
                    synergy_count: 0,
                    sample_count: 0,
                });
                pair.sample_count += 1;
                pair.alignment_score =
                    EMA_ALPHA * alignment + (1.0 - EMA_ALPHA) * pair.alignment_score;
                if alignment < COHERENCE_THRESHOLD {
                    pair.conflict_count += 1;
                } else {
                    pair.synergy_count += 1;
                }

                total_alignment += pair.alignment_score;
                pair_count += 1;
            }
        }

        let coherence = if pair_count > 0 {
            total_alignment / pair_count as f32
        } else {
            0.5
        };
        self.coherence_ema =
            EMA_ALPHA * coherence + (1.0 - EMA_ALPHA) * self.coherence_ema;
        self.coherence_ema
    }

    /// Evaluate decision optimality: were the best alternatives chosen?
    pub fn decision_optimality(&self) -> f32 {
        let resolved: Vec<&GlobalDecision> =
            self.decisions.iter().filter(|d| d.resolved).collect();
        if resolved.is_empty() {
            return self.optimality_ema;
        }

        let score = resolved
            .iter()
            .map(|d| {
                let depth_factor = (d.reasoning_depth as f32 / 5.0).min(1.0);
                let alt_factor = (d.alternatives_count as f32 / 3.0).min(1.0);
                d.outcome_score * 0.5 + depth_factor * 0.25 + alt_factor * 0.25
            })
            .sum::<f32>()
            / resolved.len() as f32;
        score
    }

    /// Build the reasoning graph snapshot
    pub fn reasoning_graph(&self) -> Vec<ReasoningNode> {
        self.reasoning_graph.values().cloned().collect()
    }

    /// Current introspection depth: how deep the system looks into itself
    pub fn introspection_depth(&self) -> f32 {
        self.depth_ema
    }

    /// Scan for blind spots: domains with low coverage or poor quality
    pub fn blind_spot_scan(&mut self) -> Vec<BlindSpot> {
        let mut domain_counts: BTreeMap<u8, (u64, f32)> = BTreeMap::new();
        for d in self.decisions.iter() {
            let entry = domain_counts.entry(d.origin as u8).or_insert((0, 0.0));
            entry.0 += 1;
            entry.1 += d.outcome_score;
        }

        let total = self.decisions.len().max(1) as f32;
        let mut spots = Vec::new();

        for origin_val in 0..8u8 {
            let (count, score_sum) = domain_counts.get(&origin_val).copied().unwrap_or((0, 0.0));
            let coverage = count as f32 / total;
            let avg_quality = if count > 0 {
                score_sum / count as f32
            } else {
                0.0
            };

            if coverage < 0.05 || avg_quality < OPTIMALITY_THRESHOLD {
                let severity = (1.0 - coverage) * 0.5 + (1.0 - avg_quality) * 0.5;
                let origin = int_to_origin(origin_val);
                let desc = String::from("Under-represented or low-quality domain");
                let id = fnv1a_hash(&[origin_val]) ^ self.tick;
                let spot = BlindSpot {
                    id,
                    description: desc,
                    affected_origins: alloc::vec![origin],
                    severity: severity.clamp(0.0, 1.0),
                    detection_tick: self.tick,
                    evidence_strength: coverage.max(0.01),
                };
                if self.blind_spots.len() < MAX_BLIND_SPOTS {
                    self.blind_spots.insert(id, spot.clone());
                }
                spots.push(spot);
            }
        }
        spots
    }

    /// Compute aggregate introspection statistics
    pub fn stats(&self) -> IntrospectionStats {
        let edge_count: usize = self
            .reasoning_graph
            .values()
            .map(|n| n.influences.len())
            .sum();
        let node_count = self.reasoning_graph.len().max(1);
        let max_edges = node_count * (node_count - 1).max(1);
        let density = edge_count as f32 / max_edges as f32;

        let conflict_rate = if self.coherence_pairs.is_empty() {
            0.0
        } else {
            let total_conflicts: u64 =
                self.coherence_pairs.values().map(|p| p.conflict_count).sum();
            let total_samples: u64 =
                self.coherence_pairs.values().map(|p| p.sample_count).sum();
            if total_samples > 0 {
                total_conflicts as f32 / total_samples as f32
            } else {
                0.0
            }
        };

        IntrospectionStats {
            total_decisions_audited: self.total_audited,
            avg_coherence: self.coherence_ema,
            avg_optimality: self.optimality_ema,
            avg_reasoning_depth: self.depth_ema,
            blind_spot_count: self.blind_spots.len(),
            cross_module_conflict_rate: conflict_rate,
            decision_quality_ema: self.quality_ema,
            graph_density: density.clamp(0.0, 1.0),
        }
    }
}

fn avg_slice(v: &[f32]) -> f32 {
    if v.is_empty() {
        0.5
    } else {
        v.iter().sum::<f32>() / v.len() as f32
    }
}

fn int_to_origin(v: u8) -> DecisionOrigin {
    match v {
        0 => DecisionOrigin::Bridge,
        1 => DecisionOrigin::Application,
        2 => DecisionOrigin::Cooperative,
        3 => DecisionOrigin::Memory,
        4 => DecisionOrigin::Scheduler,
        5 => DecisionOrigin::Network,
        6 => DecisionOrigin::Security,
        _ => DecisionOrigin::Holistic,
    }
}
