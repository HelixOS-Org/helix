// SPDX-License-Identifier: GPL-2.0
//! # Apps Introspector
//!
//! Introspects on application analysis decisions. Every classification choice
//! is recorded with the alternatives considered, the profile building quality,
//! and the resource recommendation reasoning chain. The introspector then
//! audits decision quality, identifies systematic classification biases,
//! and calibrates confidence over time.
//!
//! This is not logging: it is the apps engine examining *why* it classified
//! an application the way it did, and whether a different classification
//! would have yielded better resource outcomes.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_CLASSIFICATIONS: usize = 512;
const MAX_REASONING_DEPTH: usize = 12;
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
// CLASSIFICATION DECISION TYPES
// ============================================================================

/// Category of classification decision
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClassificationCategory {
    WorkloadType,
    ResourceProfile,
    BehaviorPattern,
    InterferenceRisk,
    ScalabilityClass,
    LifecyclePhase,
    ThermalImpact,
    PriorityAssignment,
}

/// A single reasoning step in a classification chain
#[derive(Debug, Clone)]
pub struct ClassificationReason {
    pub feature_name: String,
    pub feature_weight: f32,
    pub alternatives_considered: u8,
    pub confidence_at_step: f32,
}

/// Outcome of a classification decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassificationOutcome {
    Correct,
    PartiallyCorrect,
    Incorrect,
    Pending,
}

/// A recorded classification with full reasoning chain
#[derive(Debug, Clone)]
pub struct ClassificationRecord {
    pub id: u64,
    pub category: ClassificationCategory,
    pub tick: u64,
    pub process_id: u64,
    pub reasoning_chain: Vec<ClassificationReason>,
    pub final_confidence: f32,
    pub chosen_class: String,
    pub alternatives: Vec<String>,
    pub outcome: ClassificationOutcome,
    pub outcome_score: f32,
}

/// A detected classification bias
#[derive(Debug, Clone)]
pub struct ClassificationBias {
    pub name: String,
    pub category: ClassificationCategory,
    pub magnitude: f32,
    pub direction: f32,
    pub sample_count: u64,
    pub description: String,
}

// ============================================================================
// INTROSPECTION STATS
// ============================================================================

/// Aggregate statistics about classification introspection
#[derive(Debug, Clone, Copy, Default)]
pub struct IntrospectionStats {
    pub total_classifications: u64,
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
    avg_confidence: f32,
    avg_outcome: f32,
    success_count: u64,
    failure_count: u64,
}

impl CategoryTracker {
    fn new() -> Self {
        Self {
            total_decisions: 0,
            avg_confidence: 0.5,
            avg_outcome: 0.5,
            success_count: 0,
            failure_count: 0,
        }
    }

    fn record(&mut self, confidence: f32, outcome: f32, success: bool) {
        self.total_decisions += 1;
        if success {
            self.success_count += 1;
        } else {
            self.failure_count += 1;
        }
        self.avg_confidence = EMA_ALPHA * confidence + (1.0 - EMA_ALPHA) * self.avg_confidence;
        self.avg_outcome = EMA_ALPHA * outcome + (1.0 - EMA_ALPHA) * self.avg_outcome;
    }

    fn calibration_gap(&self) -> f32 {
        (self.avg_confidence - self.avg_outcome).abs()
    }
}

// ============================================================================
// APPS INTROSPECTOR
// ============================================================================

/// Analyzes the apps engine's own classification decisions, recording
/// reasoning chains, detecting biases, and auditing recommendation quality.
#[derive(Debug)]
pub struct AppsIntrospector {
    /// Ring buffer of recent classification records
    records: Vec<ClassificationRecord>,
    write_idx: usize,
    /// Per-category trackers
    category_trackers: BTreeMap<u8, CategoryTracker>,
    /// Total classifications ever recorded
    total_classifications: u64,
    /// Monotonic tick
    tick: u64,
    /// Global EMA of classification quality
    global_quality_ema: f32,
    /// Detected biases keyed by FNV hash
    biases: BTreeMap<u64, ClassificationBias>,
}

impl AppsIntrospector {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            write_idx: 0,
            category_trackers: BTreeMap::new(),
            total_classifications: 0,
            tick: 0,
            global_quality_ema: 0.5,
            biases: BTreeMap::new(),
        }
    }

    /// Record a classification with its full reasoning chain
    pub fn record_classification(
        &mut self,
        category: ClassificationCategory,
        process_id: u64,
        reasoning: Vec<ClassificationReason>,
        chosen_class: String,
        alternatives: Vec<String>,
        final_confidence: f32,
    ) -> u64 {
        self.tick += 1;
        self.total_classifications += 1;

        let id = fnv1a_hash(&self.total_classifications.to_le_bytes());
        let record = ClassificationRecord {
            id,
            category,
            tick: self.tick,
            process_id,
            reasoning_chain: reasoning,
            final_confidence: final_confidence.max(0.0).min(1.0),
            chosen_class,
            alternatives,
            outcome: ClassificationOutcome::Pending,
            outcome_score: 0.0,
        };

        if self.records.len() < MAX_CLASSIFICATIONS {
            self.records.push(record);
        } else {
            self.records[self.write_idx] = record;
        }
        self.write_idx = (self.write_idx + 1) % MAX_CLASSIFICATIONS;
        id
    }

    /// Analyze alternatives that were considered for a classification
    pub fn analyze_alternatives(&self, record_id: u64) -> Option<AlternativeAnalysis> {
        self.records.iter().find(|r| r.id == record_id).map(|r| {
            let depth = r.reasoning_chain.len().min(MAX_REASONING_DEPTH);
            let total_alts: u32 = r
                .reasoning_chain
                .iter()
                .map(|s| s.alternatives_considered as u32)
                .sum();
            let avg_feature_weight: f32 = if depth > 0 {
                r.reasoning_chain
                    .iter()
                    .map(|s| s.feature_weight)
                    .sum::<f32>()
                    / depth as f32
            } else {
                0.0
            };
            let confidence_path: Vec<f32> = r
                .reasoning_chain
                .iter()
                .map(|s| s.confidence_at_step)
                .collect();
            let monotonic = confidence_path
                .windows(2)
                .all(|w| w[1] >= w[0] || (w[0] - w[1]).abs() < 0.05);

            AlternativeAnalysis {
                depth,
                total_alternatives_considered: total_alts,
                avg_feature_weight,
                confidence_path,
                monotonic_confidence: monotonic,
                calibration_error: (r.final_confidence - r.outcome_score).abs(),
                alternative_count: r.alternatives.len(),
            }
        })
    }

    /// Assess quality of a classification after outcome is known
    pub fn quality_assessment(
        &mut self,
        record_id: u64,
        outcome: ClassificationOutcome,
        score: f32,
    ) {
        let clamped = score.max(0.0).min(1.0);
        for r in self.records.iter_mut() {
            if r.id == record_id {
                r.outcome = outcome;
                r.outcome_score = clamped;

                let success = matches!(
                    outcome,
                    ClassificationOutcome::Correct | ClassificationOutcome::PartiallyCorrect
                );
                let tracker = self
                    .category_trackers
                    .entry(r.category as u8)
                    .or_insert_with(CategoryTracker::new);
                tracker.record(r.final_confidence, clamped, success);

                self.global_quality_ema =
                    EMA_ALPHA * clamped + (1.0 - EMA_ALPHA) * self.global_quality_ema;
                break;
            }
        }
    }

    /// Audit resource recommendations: how often did the classification
    /// lead to the right resource allocation?
    pub fn recommendation_audit(&self) -> RecommendationAudit {
        let resolved: Vec<&ClassificationRecord> = self
            .records
            .iter()
            .filter(|r| r.outcome != ClassificationOutcome::Pending)
            .collect();
        let n = resolved.len().max(1) as f32;
        let correct = resolved
            .iter()
            .filter(|r| r.outcome == ClassificationOutcome::Correct)
            .count();
        let partial = resolved
            .iter()
            .filter(|r| r.outcome == ClassificationOutcome::PartiallyCorrect)
            .count();
        let incorrect = resolved
            .iter()
            .filter(|r| r.outcome == ClassificationOutcome::Incorrect)
            .count();

        let avg_conf: f32 = resolved.iter().map(|r| r.final_confidence).sum::<f32>() / n;
        let avg_score: f32 = resolved.iter().map(|r| r.outcome_score).sum::<f32>() / n;

        RecommendationAudit {
            total_audited: resolved.len() as u64,
            correct_rate: correct as f32 / n,
            partial_rate: partial as f32 / n,
            incorrect_rate: incorrect as f32 / n,
            avg_confidence: avg_conf,
            avg_outcome_score: avg_score,
            calibration_gap: (avg_conf - avg_score).abs(),
        }
    }

    /// Produce a decision trace for debugging a specific classification
    pub fn decision_trace(&self, record_id: u64) -> Option<Vec<String>> {
        self.records.iter().find(|r| r.id == record_id).map(|r| {
            let mut trace = Vec::new();
            for (i, step) in r.reasoning_chain.iter().enumerate() {
                let line = alloc::format!(
                    "Step {}: feature={} weight={:.3} alts={} conf={:.3}",
                    i,
                    step.feature_name,
                    step.feature_weight,
                    step.alternatives_considered,
                    step.confidence_at_step
                );
                trace.push(line);
            }
            trace.push(alloc::format!(
                "Final: class={} conf={:.3} outcome_score={:.3}",
                r.chosen_class,
                r.final_confidence,
                r.outcome_score
            ));
            trace
        })
    }

    /// Compute full introspection statistics
    pub fn stats(&self) -> IntrospectionStats {
        let n = self.records.len();
        let resolved: Vec<&ClassificationRecord> = self
            .records
            .iter()
            .filter(|r| r.outcome != ClassificationOutcome::Pending)
            .collect();
        let overconf = resolved
            .iter()
            .filter(|r| r.final_confidence > r.outcome_score + 0.1)
            .count();
        let underconf = resolved
            .iter()
            .filter(|r| r.final_confidence < r.outcome_score - 0.1)
            .count();
        let resolved_n = resolved.len().max(1) as f32;

        let avg_depth = if n > 0 {
            self.records
                .iter()
                .map(|r| r.reasoning_chain.len() as f32)
                .sum::<f32>()
                / n as f32
        } else {
            0.0
        };
        let avg_alts = if n > 0 {
            self.records
                .iter()
                .map(|r| r.alternatives.len() as f32)
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
            total_classifications: self.total_classifications,
            avg_reasoning_depth: avg_depth,
            avg_alternatives_considered: avg_alts,
            confidence_calibration: 1.0 - avg_cal,
            bias_count: self.biases.len(),
            avg_decision_quality: self.global_quality_ema,
            overconfidence_rate: overconf as f32 / resolved_n,
            underconfidence_rate: underconf as f32 / resolved_n,
        }
    }
}

// ============================================================================
// ANALYSIS OUTPUT TYPES
// ============================================================================

/// Analysis of alternatives considered during a classification
#[derive(Debug, Clone)]
pub struct AlternativeAnalysis {
    pub depth: usize,
    pub total_alternatives_considered: u32,
    pub avg_feature_weight: f32,
    pub confidence_path: Vec<f32>,
    pub monotonic_confidence: bool,
    pub calibration_error: f32,
    pub alternative_count: usize,
}

/// Audit of resource recommendations derived from classifications
#[derive(Debug, Clone, Copy)]
pub struct RecommendationAudit {
    pub total_audited: u64,
    pub correct_rate: f32,
    pub partial_rate: f32,
    pub incorrect_rate: f32,
    pub avg_confidence: f32,
    pub avg_outcome_score: f32,
    pub calibration_gap: f32,
}
