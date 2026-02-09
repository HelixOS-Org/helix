// SPDX-License-Identifier: GPL-2.0
//! # Bridge Introspector
//!
//! Analyzes the bridge's own decision process. Every major decision —
//! routing, batching, caching — is recorded with a reasoning chain.
//! The introspector then analyzes decision quality, identifies systematic
//! biases, and calibrates confidence over time.
//!
//! This is not logging: it is the bridge examining *why* it decided what
//! it decided, and whether its confidence was justified by outcomes.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_DECISIONS: usize = 512;
const MAX_REASONING_DEPTH: usize = 16;
const EMA_ALPHA: f32 = 0.12;
const BIAS_THRESHOLD: f32 = 0.15;
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

// ============================================================================
// DECISION TYPES
// ============================================================================

/// Category of bridge decision
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DecisionCategory {
    Routing,
    Batching,
    Caching,
    Prefetching,
    Throttling,
    SecurityEscalation,
    FallbackActivation,
    OptimizationToggle,
}

/// A single reasoning step in a decision chain
#[derive(Debug, Clone)]
pub struct ReasoningStep {
    pub description: String,
    pub evidence_weight: f32,
    pub alternatives_considered: u8,
    pub confidence_at_step: f32,
}

/// Outcome of a decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionOutcome {
    Success,
    PartialSuccess,
    Failure,
    Pending,
}

/// A recorded decision with full reasoning chain
#[derive(Debug, Clone)]
pub struct Decision {
    pub id: u64,
    pub category: DecisionCategory,
    pub tick: u64,
    pub reasoning_chain: Vec<ReasoningStep>,
    pub final_confidence: f32,
    pub chosen_action: String,
    pub alternatives: Vec<String>,
    pub outcome: DecisionOutcome,
    pub outcome_score: f32,
}

/// Detected bias in decision-making
#[derive(Debug, Clone)]
pub struct DetectedBias {
    pub name: String,
    pub category: DecisionCategory,
    /// How strong the bias is (0.0 – 1.0)
    pub magnitude: f32,
    /// In which direction: positive = over-favors, negative = under-favors
    pub direction: f32,
    pub sample_count: u64,
    pub description: String,
}

// ============================================================================
// INTROSPECTION STATS
// ============================================================================

/// Aggregate statistics about introspection
#[derive(Debug, Clone, Copy, Default)]
pub struct IntrospectionStats {
    pub total_decisions: u64,
    pub avg_reasoning_depth: f32,
    pub avg_alternatives_considered: f32,
    pub confidence_calibration: f32,
    pub bias_count: usize,
    pub avg_decision_quality: f32,
    pub overconfidence_rate: f32,
    pub underconfidence_rate: f32,
}

// ============================================================================
// CATEGORY TRACKER
// ============================================================================

/// Per-category statistics for bias detection
#[derive(Debug, Clone)]
struct CategoryTracker {
    total_decisions: u64,
    total_confidence: f32,
    total_outcome: f32,
    success_count: u64,
    failure_count: u64,
    avg_confidence: f32,
    avg_outcome: f32,
}

impl CategoryTracker {
    fn new() -> Self {
        Self {
            total_decisions: 0,
            total_confidence: 0.0,
            total_outcome: 0.0,
            success_count: 0,
            failure_count: 0,
            avg_confidence: 0.5,
            avg_outcome: 0.5,
        }
    }

    fn record(&mut self, confidence: f32, outcome: f32, success: bool) {
        self.total_decisions += 1;
        self.total_confidence += confidence;
        self.total_outcome += outcome;
        if success {
            self.success_count += 1;
        } else {
            self.failure_count += 1;
        }
        self.avg_confidence = EMA_ALPHA * confidence + (1.0 - EMA_ALPHA) * self.avg_confidence;
        self.avg_outcome = EMA_ALPHA * outcome + (1.0 - EMA_ALPHA) * self.avg_outcome;
    }

    /// Calibration gap: how far confidence is from actual outcomes
    fn calibration_gap(&self) -> f32 {
        (self.avg_confidence - self.avg_outcome).abs()
    }
}

// ============================================================================
// BRIDGE INTROSPECTOR
// ============================================================================

/// Analyzes the bridge's own decision process, recording reasoning chains,
/// detecting biases, and calibrating confidence against outcomes.
#[derive(Debug)]
pub struct BridgeIntrospector {
    /// Ring buffer of recent decisions
    decisions: Vec<Decision>,
    write_idx: usize,
    /// Per-category trackers
    category_trackers: BTreeMap<u8, CategoryTracker>,
    /// Total decisions ever recorded
    total_decisions: u64,
    /// Monotonic tick
    tick: u64,
    /// Global EMA of decision quality
    global_quality_ema: f32,
    /// Detected biases (keyed by FNV hash of description)
    biases: BTreeMap<u64, DetectedBias>,
}

impl BridgeIntrospector {
    pub fn new() -> Self {
        Self {
            decisions: Vec::new(),
            write_idx: 0,
            category_trackers: BTreeMap::new(),
            total_decisions: 0,
            tick: 0,
            global_quality_ema: 0.5,
            biases: BTreeMap::new(),
        }
    }

    /// Record a decision with its full reasoning chain
    pub fn record_decision(
        &mut self,
        category: DecisionCategory,
        reasoning: Vec<ReasoningStep>,
        chosen_action: String,
        alternatives: Vec<String>,
        final_confidence: f32,
    ) -> u64 {
        self.tick += 1;
        self.total_decisions += 1;

        let id = fnv1a_hash(&self.total_decisions.to_le_bytes());
        let decision = Decision {
            id,
            category,
            tick: self.tick,
            reasoning_chain: reasoning,
            final_confidence: final_confidence.max(0.0).min(1.0),
            chosen_action,
            alternatives,
            outcome: DecisionOutcome::Pending,
            outcome_score: 0.0,
        };

        if self.decisions.len() < MAX_DECISIONS {
            self.decisions.push(decision);
        } else {
            self.decisions[self.write_idx] = decision;
        }
        self.write_idx = (self.write_idx + 1) % MAX_DECISIONS;
        id
    }

    /// Update a decision's outcome after the fact
    pub fn record_outcome(&mut self, decision_id: u64, outcome: DecisionOutcome, score: f32) {
        let clamped = score.max(0.0).min(1.0);
        for d in self.decisions.iter_mut() {
            if d.id == decision_id {
                d.outcome = outcome;
                d.outcome_score = clamped;

                let success = matches!(
                    outcome,
                    DecisionOutcome::Success | DecisionOutcome::PartialSuccess
                );
                let tracker = self
                    .category_trackers
                    .entry(d.category as u8)
                    .or_insert_with(CategoryTracker::new);
                tracker.record(d.final_confidence, clamped, success);

                self.global_quality_ema =
                    EMA_ALPHA * clamped + (1.0 - EMA_ALPHA) * self.global_quality_ema;
                break;
            }
        }
    }

    /// Analyze reasoning quality for a specific decision
    pub fn analyze_reasoning(&self, decision_id: u64) -> Option<ReasoningAnalysis> {
        self.decisions
            .iter()
            .find(|d| d.id == decision_id)
            .map(|d| {
                let depth = d.reasoning_chain.len();
                let total_alts: u32 = d
                    .reasoning_chain
                    .iter()
                    .map(|s| s.alternatives_considered as u32)
                    .sum();
                let avg_evidence: f32 = if depth > 0 {
                    d.reasoning_chain
                        .iter()
                        .map(|s| s.evidence_weight)
                        .sum::<f32>()
                        / depth as f32
                } else {
                    0.0
                };
                let confidence_trajectory: Vec<f32> = d
                    .reasoning_chain
                    .iter()
                    .map(|s| s.confidence_at_step)
                    .collect();
                let monotonic = confidence_trajectory
                    .windows(2)
                    .all(|w| w[1] >= w[0] || (w[0] - w[1]).abs() < 0.05);

                ReasoningAnalysis {
                    depth,
                    total_alternatives_considered: total_alts,
                    avg_evidence_weight: avg_evidence,
                    confidence_trajectory,
                    monotonic_confidence: monotonic,
                    calibration_error: (d.final_confidence - d.outcome_score).abs(),
                }
            })
    }

    /// Identify systematic biases across decision categories
    pub fn identify_bias(&mut self) -> Vec<DetectedBias> {
        let mut found = Vec::new();
        for (&cat_key, tracker) in self.category_trackers.iter() {
            if tracker.total_decisions < 10 {
                continue;
            }
            let gap = tracker.calibration_gap();
            if gap > BIAS_THRESHOLD {
                let direction = tracker.avg_confidence - tracker.avg_outcome;
                let bias_name = if direction > 0.0 {
                    String::from("overconfidence")
                } else {
                    String::from("underconfidence")
                };
                let desc = String::from(if direction > 0.0 {
                    "Bridge overestimates success likelihood"
                } else {
                    "Bridge underestimates success likelihood"
                });
                let bias = DetectedBias {
                    name: bias_name,
                    category: match cat_key {
                        0 => DecisionCategory::Routing,
                        1 => DecisionCategory::Batching,
                        2 => DecisionCategory::Caching,
                        3 => DecisionCategory::Prefetching,
                        4 => DecisionCategory::Throttling,
                        5 => DecisionCategory::SecurityEscalation,
                        6 => DecisionCategory::FallbackActivation,
                        _ => DecisionCategory::OptimizationToggle,
                    },
                    magnitude: gap,
                    direction,
                    sample_count: tracker.total_decisions,
                    description: desc,
                };
                let hash = fnv1a_hash(bias.description.as_bytes()) ^ (cat_key as u64);
                self.biases.insert(hash, bias.clone());
                found.push(bias);
            }
        }
        found
    }

    /// Average decision quality (EMA-smoothed outcome scores)
    pub fn decision_quality(&self) -> f32 {
        self.global_quality_ema
    }

    /// Average reasoning depth across all recorded decisions
    pub fn reasoning_depth(&self) -> f32 {
        if self.decisions.is_empty() {
            return 0.0;
        }
        let sum: usize = self.decisions.iter().map(|d| d.reasoning_chain.len()).sum();
        sum as f32 / self.decisions.len() as f32
    }

    /// Compute full introspection statistics
    pub fn stats(&self) -> IntrospectionStats {
        let n = self.decisions.len();
        let resolved: Vec<&Decision> = self
            .decisions
            .iter()
            .filter(|d| d.outcome != DecisionOutcome::Pending)
            .collect();
        let overconf = resolved
            .iter()
            .filter(|d| d.final_confidence > d.outcome_score + 0.1)
            .count();
        let underconf = resolved
            .iter()
            .filter(|d| d.final_confidence < d.outcome_score - 0.1)
            .count();
        let resolved_n = resolved.len().max(1) as f32;

        let avg_alts = if n > 0 {
            self.decisions
                .iter()
                .map(|d| d.alternatives.len() as f32)
                .sum::<f32>()
                / n as f32
        } else {
            0.0
        };

        let avg_cal: f32 = self
            .category_trackers
            .values()
            .map(|t| t.calibration_gap())
            .sum::<f32>()
            / self.category_trackers.len().max(1) as f32;

        IntrospectionStats {
            total_decisions: self.total_decisions,
            avg_reasoning_depth: self.reasoning_depth(),
            avg_alternatives_considered: avg_alts,
            confidence_calibration: 1.0 - avg_cal,
            bias_count: self.biases.len(),
            avg_decision_quality: self.global_quality_ema,
            overconfidence_rate: overconf as f32 / resolved_n,
            underconfidence_rate: underconf as f32 / resolved_n,
        }
    }
}

/// Analysis of a single decision's reasoning process
#[derive(Debug, Clone)]
pub struct ReasoningAnalysis {
    pub depth: usize,
    pub total_alternatives_considered: u32,
    pub avg_evidence_weight: f32,
    pub confidence_trajectory: Vec<f32>,
    pub monotonic_confidence: bool,
    pub calibration_error: f32,
}
