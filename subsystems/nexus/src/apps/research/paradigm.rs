// SPDX-License-Identifier: GPL-2.0
//! # Apps Paradigm — Paradigm Shift Detection in App Understanding
//!
//! When accumulated evidence invalidates the current classification model,
//! a paradigm shift has occurred. This engine detects when the existing model
//! is becoming obsolete, proposes new paradigms, plans transitions, and
//! records the history of paradigm shifts for the app understanding subsystem.
//!
//! The engine that knows when to throw out the old model and start fresh.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_EVIDENCE: usize = 1024;
const MAX_PARADIGMS: usize = 64;
const MAX_CHRONICLE: usize = 128;
const OBSOLESCENCE_THRESHOLD: f32 = 0.70;
const SHIFT_THRESHOLD: f32 = 0.75;
const EVIDENCE_DECAY: f32 = 0.998;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const MIN_EVIDENCE_FOR_SHIFT: usize = 10;
const WEIGHT_RECENCY: f32 = 0.30;
const WEIGHT_STRENGTH: f32 = 0.40;
const WEIGHT_CONSISTENCY: f32 = 0.30;
const TRANSITION_PHASES: usize = 5;
const PROPOSAL_MATURITY_TICKS: u64 = 500;

// ============================================================================
// HELPERS
// ============================================================================

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

fn abs_f32(x: f32) -> f32 {
    if x < 0.0 { -x } else { x }
}

// ============================================================================
// TYPES
// ============================================================================

/// Status of the current paradigm.
#[derive(Clone, Copy, PartialEq)]
pub enum ParadigmStatus {
    Stable,
    Questioned,
    Challenged,
    Shifting,
    Replaced,
}

/// Direction of model invalidation evidence.
#[derive(Clone, Copy, PartialEq)]
pub enum EvidenceDirection {
    SupportsModel,
    Neutral,
    ContradictsModel,
    InvalidatesModel,
}

/// A piece of evidence for or against the current paradigm.
#[derive(Clone)]
pub struct ParadigmEvidence {
    pub evidence_id: u64,
    pub description: String,
    pub direction: EvidenceDirection,
    pub strength: f32,
    pub weight: f32,
    pub submitted_tick: u64,
    pub decayed_weight: f32,
}

/// A proposed new paradigm.
#[derive(Clone)]
pub struct ParadigmProposal {
    pub proposal_id: u64,
    pub name: String,
    pub description: String,
    pub supporting_evidence: Vec<u64>,
    pub viability_score: f32,
    pub improvement_estimate: f32,
    pub proposed_tick: u64,
    pub maturity: f32,
}

/// Model obsolescence assessment.
#[derive(Clone)]
pub struct ObsolescenceReport {
    pub current_paradigm: String,
    pub obsolescence_score: f32,
    pub contradicting_evidence: usize,
    pub supporting_evidence: usize,
    pub status: ParadigmStatus,
    pub decay_rate: f32,
    pub time_since_last_shift: u64,
}

/// Transition strategy for a paradigm shift.
#[derive(Clone)]
pub struct TransitionStrategy {
    pub from_paradigm: String,
    pub to_paradigm: String,
    pub phases: Vec<TransitionPhase>,
    pub total_risk: f32,
    pub estimated_duration: u64,
    pub rollback_possible: bool,
}

/// A single phase in a paradigm transition.
#[derive(Clone)]
pub struct TransitionPhase {
    pub phase_number: usize,
    pub name: String,
    pub description: String,
    pub risk_level: f32,
    pub completion_criteria: String,
}

/// A chronicle entry recording a paradigm shift.
#[derive(Clone)]
pub struct ChronicleEntry {
    pub entry_id: u64,
    pub old_paradigm: String,
    pub new_paradigm: String,
    pub trigger_evidence: usize,
    pub shift_magnitude: f32,
    pub transition_success: bool,
    pub shift_tick: u64,
}

/// Engine-level stats.
#[derive(Clone)]
pub struct ParadigmStats {
    pub evidence_collected: u64,
    pub shifts_detected: u64,
    pub proposals_made: u64,
    pub transitions_completed: u64,
    pub ema_obsolescence: f32,
    pub ema_evidence_strength: f32,
    pub ema_shift_magnitude: f32,
}

// ============================================================================
// ENGINE
// ============================================================================

/// Paradigm shift detection engine for app understanding.
pub struct AppsParadigm {
    current_paradigm: String,
    evidence: Vec<ParadigmEvidence>,
    proposals: BTreeMap<u64, ParadigmProposal>,
    chronicle: Vec<ChronicleEntry>,
    status: ParadigmStatus,
    last_shift_tick: u64,
    stats: ParadigmStats,
    rng_state: u64,
    tick: u64,
}

impl AppsParadigm {
    /// Create a new paradigm detection engine.
    pub fn new(seed: u64, initial_paradigm: &str) -> Self {
        Self {
            current_paradigm: String::from(initial_paradigm),
            evidence: Vec::new(),
            proposals: BTreeMap::new(),
            chronicle: Vec::new(),
            status: ParadigmStatus::Stable,
            last_shift_tick: 0,
            stats: ParadigmStats {
                evidence_collected: 0,
                shifts_detected: 0,
                proposals_made: 0,
                transitions_completed: 0,
                ema_obsolescence: 0.0,
                ema_evidence_strength: 0.0,
                ema_shift_magnitude: 0.0,
            },
            rng_state: seed ^ 0xc83a41de6f520b97,
            tick: 0,
        }
    }

    // ── Primary API ────────────────────────────────────────────────────

    /// Detect whether a paradigm shift is occurring based on accumulated evidence.
    pub fn detect_shift(&mut self) -> (bool, ParadigmStatus) {
        self.tick += 1;

        // Decay all evidence weights
        for ev in self.evidence.iter_mut() {
            ev.decayed_weight = ev.weight * EVIDENCE_DECAY;
            ev.weight = ev.decayed_weight;
        }

        // Tally evidence
        let mut contra_sum = 0.0f32;
        let mut support_sum = 0.0f32;
        let mut contra_count = 0usize;
        let mut support_count = 0usize;

        for ev in &self.evidence {
            match ev.direction {
                EvidenceDirection::ContradictsModel | EvidenceDirection::InvalidatesModel => {
                    contra_sum += ev.strength * ev.weight;
                    contra_count += 1;
                }
                EvidenceDirection::SupportsModel => {
                    support_sum += ev.strength * ev.weight;
                    support_count += 1;
                }
                EvidenceDirection::Neutral => {}
            }
        }

        let total_weight = (contra_sum + support_sum).max(0.01);
        let contra_ratio = contra_sum / total_weight;
        let enough_evidence = contra_count >= MIN_EVIDENCE_FOR_SHIFT;

        // Determine new status
        let old_status = self.status;
        self.status = if contra_ratio >= SHIFT_THRESHOLD && enough_evidence {
            ParadigmStatus::Shifting
        } else if contra_ratio >= OBSOLESCENCE_THRESHOLD && enough_evidence {
            ParadigmStatus::Challenged
        } else if contra_ratio >= 0.40 {
            ParadigmStatus::Questioned
        } else {
            ParadigmStatus::Stable
        };

        let shift_detected = self.status == ParadigmStatus::Shifting
            && old_status != ParadigmStatus::Shifting;

        if shift_detected {
            self.stats.shifts_detected += 1;
        }

        self.stats.ema_obsolescence =
            EMA_ALPHA * contra_ratio + (1.0 - EMA_ALPHA) * self.stats.ema_obsolescence;

        (shift_detected, self.status)
    }

    /// Submit a piece of evidence for or against the current paradigm.
    pub fn evidence_weight(
        &mut self,
        description: &str,
        direction: EvidenceDirection,
        strength: f32,
    ) -> u64 {
        self.tick += 1;
        self.stats.evidence_collected += 1;

        let id = fnv1a_hash(description.as_bytes()) ^ self.tick;
        let clamped = strength.min(1.0).max(0.0);

        // Weight accounts for recency
        let recency_factor = 1.0; // freshly added evidence has full weight
        let weight = clamped * recency_factor;

        let evidence = ParadigmEvidence {
            evidence_id: id,
            description: String::from(description),
            direction,
            strength: clamped,
            weight,
            submitted_tick: self.tick,
            decayed_weight: weight,
        };

        if self.evidence.len() >= MAX_EVIDENCE {
            self.evidence.remove(0);
        }
        self.evidence.push(evidence);

        self.stats.ema_evidence_strength =
            EMA_ALPHA * clamped + (1.0 - EMA_ALPHA) * self.stats.ema_evidence_strength;

        id
    }

    /// Assess the obsolescence level of the current model.
    pub fn model_obsolescence(&self) -> ObsolescenceReport {
        let mut contra = 0usize;
        let mut support = 0usize;
        let mut contra_weight = 0.0f32;
        let mut support_weight = 0.0f32;

        for ev in &self.evidence {
            match ev.direction {
                EvidenceDirection::ContradictsModel | EvidenceDirection::InvalidatesModel => {
                    contra += 1;
                    contra_weight += ev.weight;
                }
                EvidenceDirection::SupportsModel => {
                    support += 1;
                    support_weight += ev.weight;
                }
                _ => {}
            }
        }

        let total = (contra_weight + support_weight).max(0.01);
        let obs_score = contra_weight / total;
        let time_since = self.tick.saturating_sub(self.last_shift_tick);

        ObsolescenceReport {
            current_paradigm: self.current_paradigm.clone(),
            obsolescence_score: obs_score,
            contradicting_evidence: contra,
            supporting_evidence: support,
            status: self.status,
            decay_rate: EVIDENCE_DECAY,
            time_since_last_shift: time_since,
        }
    }

    /// Propose a new paradigm to replace the current one.
    pub fn new_paradigm_proposal(
        &mut self,
        name: &str,
        description: &str,
        supporting_evidence_ids: &[u64],
        improvement_estimate: f32,
    ) -> u64 {
        self.tick += 1;
        self.stats.proposals_made += 1;

        let id = fnv1a_hash(name.as_bytes()) ^ self.tick;

        // Compute viability from supporting evidence
        let mut viability = 0.0f32;
        let mut match_count = 0u32;
        for &eid in supporting_evidence_ids {
            for ev in &self.evidence {
                if ev.evidence_id == eid {
                    viability += ev.strength * ev.weight;
                    match_count += 1;
                }
            }
        }
        viability = if match_count > 0 {
            (viability / match_count as f32).min(1.0)
        } else {
            0.2
        };

        let proposal = ParadigmProposal {
            proposal_id: id,
            name: String::from(name),
            description: String::from(description),
            supporting_evidence: Vec::from(supporting_evidence_ids),
            viability_score: viability,
            improvement_estimate: improvement_estimate.min(1.0).max(0.0),
            proposed_tick: self.tick,
            maturity: 0.0,
        };

        if self.proposals.len() >= MAX_PARADIGMS {
            let mut min_id = 0u64;
            let mut min_via = f32::MAX;
            for (pid, p) in self.proposals.iter() {
                if p.viability_score < min_via {
                    min_via = p.viability_score;
                    min_id = *pid;
                }
            }
            self.proposals.remove(&min_id);
        }
        self.proposals.insert(id, proposal);
        id
    }

    /// Generate a transition strategy from the current paradigm to a new one.
    pub fn transition_strategy(&self, proposal_id: u64) -> Option<TransitionStrategy> {
        let proposal = self.proposals.get(&proposal_id)?;

        let mut phases = Vec::new();
        let phase_names = [
            ("Assessment", "Evaluate new paradigm against current workloads"),
            ("Dual-Run", "Run both paradigms in parallel and compare"),
            ("Gradual Migration", "Shift classification weight toward new paradigm"),
            ("Validation", "Verify new paradigm meets quality bar"),
            ("Cutover", "Fully switch to new paradigm with rollback ready"),
        ];

        let base_risk = 1.0 - proposal.viability_score;
        for (i, (name, desc)) in phase_names.iter().enumerate() {
            let phase_risk = base_risk * (0.5 + 0.1 * i as f32);
            phases.push(TransitionPhase {
                phase_number: i + 1,
                name: String::from(*name),
                description: String::from(*desc),
                risk_level: phase_risk.min(1.0),
                completion_criteria: String::from("Metrics meet threshold"),
            });
        }

        let total_risk = base_risk * 0.8;
        let duration = ((1.0 - proposal.viability_score) * 2000.0) as u64 + 500;

        Some(TransitionStrategy {
            from_paradigm: self.current_paradigm.clone(),
            to_paradigm: proposal.name.clone(),
            phases,
            total_risk,
            estimated_duration: duration,
            rollback_possible: true,
        })
    }

    /// Execute a paradigm shift — record it and update the current paradigm.
    pub fn execute_shift(&mut self, proposal_id: u64) -> bool {
        let proposal = match self.proposals.get(&proposal_id) {
            Some(p) => p.clone(),
            None => return false,
        };

        let entry = ChronicleEntry {
            entry_id: fnv1a_hash(&self.stats.shifts_detected.to_le_bytes()) ^ self.tick,
            old_paradigm: self.current_paradigm.clone(),
            new_paradigm: proposal.name.clone(),
            trigger_evidence: proposal.supporting_evidence.len(),
            shift_magnitude: proposal.viability_score,
            transition_success: true,
            shift_tick: self.tick,
        };

        self.stats.transitions_completed += 1;
        self.stats.ema_shift_magnitude =
            EMA_ALPHA * proposal.viability_score + (1.0 - EMA_ALPHA) * self.stats.ema_shift_magnitude;

        self.current_paradigm = proposal.name.clone();
        self.status = ParadigmStatus::Stable;
        self.last_shift_tick = self.tick;

        // Clear contradicting evidence — new paradigm starts fresh
        self.evidence.retain(|ev| {
            ev.direction == EvidenceDirection::SupportsModel
                || ev.direction == EvidenceDirection::Neutral
        });

        if self.chronicle.len() >= MAX_CHRONICLE {
            self.chronicle.remove(0);
        }
        self.chronicle.push(entry);

        // Remove the executed proposal
        self.proposals.remove(&proposal_id);
        true
    }

    /// Get the full paradigm chronicle — history of all shifts.
    pub fn paradigm_chronicle(&self) -> Vec<ChronicleEntry> {
        self.chronicle.clone()
    }

    /// Get the current paradigm name.
    pub fn current_paradigm(&self) -> &str {
        &self.current_paradigm
    }

    /// Get the current paradigm status.
    pub fn current_status(&self) -> ParadigmStatus {
        self.status
    }

    /// List all active proposals ranked by viability.
    pub fn active_proposals(&self) -> Vec<ParadigmProposal> {
        let mut proposals: Vec<ParadigmProposal> = self.proposals.values().cloned().collect();
        for i in 0..proposals.len() {
            for j in (i + 1)..proposals.len() {
                if proposals[j].viability_score > proposals[i].viability_score {
                    proposals.swap(i, j);
                }
            }
        }
        proposals
    }

    /// Return engine stats.
    pub fn stats(&self) -> &ParadigmStats {
        &self.stats
    }
}
