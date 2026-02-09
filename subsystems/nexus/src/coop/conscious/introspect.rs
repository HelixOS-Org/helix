// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Introspector
//!
//! Introspects on cooperation decisions. Every negotiation outcome,
//! arbitration decision, and resource allocation is recorded with reasoning.
//! The introspector analyzes fairness across decisions, detects systematic
//! bias toward or against specific participants, and audits the decision
//! trail for accountability.
//!
//! This is not logging — it is the cooperation engine examining *why* it
//! decided what it decided, and whether fairness was truly achieved.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_NEGOTIATIONS: usize = 512;
const EMA_ALPHA: f32 = 0.12;
const BIAS_THRESHOLD: f32 = 0.15;
const FAIRNESS_TOLERANCE: f32 = 0.10;
const MAX_PARTICIPANTS: usize = 128;
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
// NEGOTIATION TYPES
// ============================================================================

/// Category of cooperation decision
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NegotiationCategory {
    ResourceAllocation,
    ContractFormation,
    DisputeArbitration,
    PriorityAssignment,
    HintProcessing,
    FeedbackRouting,
    TrustAdjustment,
    CoalitionMerge,
}

/// Outcome of a negotiation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NegotiationOutcome {
    Agreed,
    PartialAgreement,
    Rejected,
    Timeout,
    Escalated,
}

/// A recorded negotiation with full context
#[derive(Debug, Clone)]
pub struct NegotiationRecord {
    pub id: u64,
    pub category: NegotiationCategory,
    pub tick: u64,
    /// Participants (FNV hashes of participant IDs)
    pub participants: Vec<u64>,
    /// Resources at stake (arbitrary units)
    pub resources_at_stake: f32,
    /// Fairness score of the outcome (0.0 – 1.0)
    pub fairness_score: f32,
    /// Confidence in the decision
    pub confidence: f32,
    /// Outcome
    pub outcome: NegotiationOutcome,
    /// Satisfaction scores per participant
    pub satisfaction: Vec<f32>,
}

/// Detected bias in cooperation decisions
#[derive(Debug, Clone)]
pub struct CoopBias {
    pub name: String,
    pub id: u64,
    pub category: NegotiationCategory,
    /// How strong the bias is (0.0 – 1.0)
    pub magnitude: f32,
    /// Positive = favors certain participants, negative = penalizes
    pub direction: f32,
    /// Affected participant IDs
    pub affected_participants: Vec<u64>,
    pub sample_count: u64,
}

// ============================================================================
// PER-PARTICIPANT TRACKER
// ============================================================================

/// Tracks cooperation fairness per participant
#[derive(Debug, Clone)]
struct ParticipantTracker {
    participant_id: u64,
    total_negotiations: u64,
    total_resources_received: f32,
    total_resources_requested: f32,
    avg_satisfaction: f32,
    avg_fairness: f32,
    agreements: u64,
    rejections: u64,
}

impl ParticipantTracker {
    fn new(participant_id: u64) -> Self {
        Self {
            participant_id,
            total_negotiations: 0,
            total_resources_received: 0.0,
            total_resources_requested: 0.0,
            avg_satisfaction: 0.5,
            avg_fairness: 0.5,
            agreements: 0,
            rejections: 0,
        }
    }

    fn record(&mut self, satisfaction: f32, fairness: f32, agreed: bool, resources: f32) {
        self.total_negotiations += 1;
        self.total_resources_requested += resources;
        if agreed {
            self.agreements += 1;
            self.total_resources_received += resources * satisfaction;
        } else {
            self.rejections += 1;
        }
        self.avg_satisfaction =
            EMA_ALPHA * satisfaction + (1.0 - EMA_ALPHA) * self.avg_satisfaction;
        self.avg_fairness = EMA_ALPHA * fairness + (1.0 - EMA_ALPHA) * self.avg_fairness;
    }

    fn fulfillment_ratio(&self) -> f32 {
        if self.total_resources_requested < f32::EPSILON {
            return 1.0;
        }
        (self.total_resources_received / self.total_resources_requested).min(1.0)
    }
}

// ============================================================================
// INTROSPECTION STATS
// ============================================================================

/// Aggregate cooperation introspection statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct IntrospectionStats {
    pub total_negotiations: u64,
    pub avg_fairness: f32,
    pub avg_satisfaction: f32,
    pub bias_count: usize,
    pub agreement_rate: f32,
    pub decision_quality: f32,
    pub participant_count: usize,
    pub fairness_variance: f32,
}

// ============================================================================
// COOPERATION INTROSPECTOR
// ============================================================================

/// Analyzes cooperation decisions, recording negotiation outcomes, detecting
/// bias, and auditing fairness across all participants.
#[derive(Debug)]
pub struct CoopIntrospector {
    /// Ring buffer of recent negotiations
    negotiations: Vec<NegotiationRecord>,
    write_idx: usize,
    /// Per-participant trackers (keyed by participant FNV hash)
    participant_trackers: BTreeMap<u64, ParticipantTracker>,
    /// Total negotiations ever recorded
    total_negotiations: u64,
    /// Total agreements
    total_agreements: u64,
    /// Monotonic tick
    tick: u64,
    /// Global EMA of fairness
    global_fairness_ema: f32,
    /// Global EMA of satisfaction
    global_satisfaction_ema: f32,
    /// Detected biases (keyed by FNV hash of description)
    biases: BTreeMap<u64, CoopBias>,
}

impl CoopIntrospector {
    pub fn new() -> Self {
        Self {
            negotiations: Vec::new(),
            write_idx: 0,
            participant_trackers: BTreeMap::new(),
            total_negotiations: 0,
            total_agreements: 0,
            tick: 0,
            global_fairness_ema: 0.5,
            global_satisfaction_ema: 0.5,
            biases: BTreeMap::new(),
        }
    }

    /// Record a completed negotiation with per-participant satisfaction
    pub fn record_negotiation(
        &mut self,
        category: NegotiationCategory,
        participants: &[u64],
        resources_at_stake: f32,
        fairness_score: f32,
        confidence: f32,
        outcome: NegotiationOutcome,
        satisfaction: &[f32],
    ) {
        self.tick += 1;
        self.total_negotiations += 1;

        let agreed = matches!(
            outcome,
            NegotiationOutcome::Agreed | NegotiationOutcome::PartialAgreement
        );
        if agreed {
            self.total_agreements += 1;
        }

        let clamped_fairness = fairness_score.max(0.0).min(1.0);
        self.global_fairness_ema =
            EMA_ALPHA * clamped_fairness + (1.0 - EMA_ALPHA) * self.global_fairness_ema;

        // Per-participant tracking
        for (i, &pid) in participants.iter().enumerate() {
            let sat = satisfaction.get(i).copied().unwrap_or(0.5);
            self.global_satisfaction_ema =
                EMA_ALPHA * sat + (1.0 - EMA_ALPHA) * self.global_satisfaction_ema;

            let tracker = self
                .participant_trackers
                .entry(pid)
                .or_insert_with(|| ParticipantTracker::new(pid));
            tracker.record(sat, clamped_fairness, agreed, resources_at_stake);
        }

        let id = self.total_negotiations;
        let record = NegotiationRecord {
            id,
            category,
            tick: self.tick,
            participants: Vec::from(participants),
            resources_at_stake,
            fairness_score: clamped_fairness,
            confidence: confidence.max(0.0).min(1.0),
            outcome,
            satisfaction: Vec::from(satisfaction),
        };

        if self.negotiations.len() < MAX_NEGOTIATIONS {
            self.negotiations.push(record);
        } else {
            self.negotiations[self.write_idx] = record;
        }
        self.write_idx = (self.write_idx + 1) % MAX_NEGOTIATIONS;
    }

    /// Analyze fairness across all participants — returns Gini-like coefficient
    pub fn analyze_fairness(&self) -> f32 {
        let trackers: Vec<&ParticipantTracker> = self.participant_trackers.values().collect();
        let n = trackers.len();
        if n < 2 {
            return 1.0;
        }

        let ratios: Vec<f32> = trackers.iter().map(|t| t.fulfillment_ratio()).collect();
        let mean = ratios.iter().sum::<f32>() / n as f32;
        if mean < f32::EPSILON {
            return 0.0;
        }

        // Gini coefficient: mean absolute difference / (2 * mean)
        let mut diff_sum = 0.0_f32;
        for i in 0..n {
            for j in 0..n {
                diff_sum += (ratios[i] - ratios[j]).abs();
            }
        }
        let gini = diff_sum / (2.0 * n as f32 * n as f32 * mean);
        1.0 - gini.min(1.0) // 1.0 = perfectly fair
    }

    /// Detect systematic bias: participants consistently over- or under-served
    pub fn bias_detection(&mut self) -> usize {
        self.biases.clear();
        let trackers: Vec<&ParticipantTracker> = self.participant_trackers.values().collect();
        let n = trackers.len();
        if n < 2 {
            return 0;
        }

        let global_avg_sat = self.global_satisfaction_ema;

        for tracker in &trackers {
            let deviation = tracker.avg_satisfaction - global_avg_sat;
            if deviation.abs() > BIAS_THRESHOLD {
                let bias_name = if deviation > 0.0 {
                    "over_favored_participant"
                } else {
                    "under_served_participant"
                };
                let bias_id = fnv1a_hash(bias_name.as_bytes()) ^ tracker.participant_id;
                self.biases.insert(bias_id, CoopBias {
                    name: String::from(bias_name),
                    id: bias_id,
                    category: NegotiationCategory::ResourceAllocation,
                    magnitude: deviation.abs(),
                    direction: deviation,
                    affected_participants: Vec::from([tracker.participant_id]),
                    sample_count: tracker.total_negotiations,
                });
            }
        }
        self.biases.len()
    }

    /// Audit a decision trail: check if outcomes matched stated confidence
    pub fn decision_audit(&self) -> f32 {
        if self.negotiations.is_empty() {
            return 0.5;
        }

        let mut calibration_sum = 0.0_f32;
        let mut count = 0_usize;

        for neg in &self.negotiations {
            let expected_quality = neg.confidence;
            let actual_quality = neg.fairness_score;
            let gap = (expected_quality - actual_quality).abs();
            calibration_sum += 1.0 - gap.min(1.0);
            count += 1;
        }

        if count == 0 {
            return 0.5;
        }
        calibration_sum / count as f32
    }

    /// Overall cooperation quality score
    pub fn cooperation_quality(&self) -> f32 {
        let fairness = self.analyze_fairness();
        let calibration = self.decision_audit();
        let agreement_rate = if self.total_negotiations > 0 {
            self.total_agreements as f32 / self.total_negotiations as f32
        } else {
            0.5
        };

        fairness * 0.35
            + calibration * 0.30
            + agreement_rate * 0.20
            + self.global_satisfaction_ema * 0.15
    }

    /// Get aggregate statistics
    pub fn stats(&self) -> IntrospectionStats {
        let fairness_values: Vec<f32> = self
            .participant_trackers
            .values()
            .map(|t| t.avg_fairness)
            .collect();
        let fairness_var = if fairness_values.len() > 1 {
            let mean = fairness_values.iter().sum::<f32>() / fairness_values.len() as f32;
            fairness_values
                .iter()
                .map(|v| (v - mean) * (v - mean))
                .sum::<f32>()
                / fairness_values.len() as f32
        } else {
            0.0
        };

        IntrospectionStats {
            total_negotiations: self.total_negotiations,
            avg_fairness: self.global_fairness_ema,
            avg_satisfaction: self.global_satisfaction_ema,
            bias_count: self.biases.len(),
            agreement_rate: if self.total_negotiations > 0 {
                self.total_agreements as f32 / self.total_negotiations as f32
            } else {
                0.0
            },
            decision_quality: self.cooperation_quality(),
            participant_count: self.participant_trackers.len(),
            fairness_variance: fairness_var,
        }
    }
}
