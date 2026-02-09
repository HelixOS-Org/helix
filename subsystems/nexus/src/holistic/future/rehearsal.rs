// SPDX-License-Identifier: GPL-2.0
//! # Holistic Rehearsal Engine
//!
//! System-wide decision rehearsal engine. Before executing major system
//! decisions — OOM kills, process migrations, scaling events, resource
//! reclamation — this module **rehearses** them: simulates their impact,
//! evaluates risk, runs counterfactual analysis, and produces a go/no-go
//! recommendation with quantified confidence.
//!
//! Think of it as the kernel's "imagination": it plays out consequences
//! before committing to irreversible actions.

extern crate alloc;

use crate::fast::fast_hash::FastHasher;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_REHEARSALS: usize = 256;
const MAX_COUNTERFACTUALS: usize = 128;
const MAX_IMPACT_ENTRIES: usize = 256;
const MAX_RISK_ENTRIES: usize = 128;
const EMA_ALPHA: f32 = 0.10;
const RISK_THRESHOLD: f32 = 0.50;
const CONFIDENCE_MIN: f32 = 0.10;
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
// DECISION KIND
// ============================================================================

/// Kind of major system decision being rehearsed
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DecisionKind {
    OomKill,
    ProcessMigration,
    ScaleUp,
    ScaleDown,
    MemoryReclaim,
    IoReroute,
    ThermalThrottle,
    LoadShed,
}

/// Rehearsal recommendation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Recommendation {
    StrongProceed,
    Proceed,
    ProceedWithCaution,
    Defer,
    Abort,
}

/// Risk category
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskCategory {
    Negligible,
    Low,
    Moderate,
    High,
    Extreme,
}

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// A rehearsed decision with simulated outcomes
#[derive(Debug, Clone)]
pub struct RehearsalRecord {
    pub id: u64,
    pub kind: DecisionKind,
    pub description: String,
    pub rehearsal_ticks: u64,
    pub simulated_benefit: f32,
    pub simulated_cost: f32,
    pub net_outcome: f32,
    pub risk_score: f32,
    pub recommendation: Recommendation,
    pub confidence: f32,
    pub tick: u64,
}

/// Impact assessment for a rehearsed decision
#[derive(Debug, Clone)]
pub struct ImpactAssessment {
    pub decision_id: u64,
    pub cpu_impact: f32,
    pub memory_impact: f32,
    pub io_impact: f32,
    pub latency_impact_us: i64,
    pub process_count_delta: i32,
    pub stability_impact: f32,
    pub recovery_ticks: u64,
}

/// Risk evaluation for a rehearsed decision
#[derive(Debug, Clone)]
pub struct RiskEvaluation {
    pub decision_id: u64,
    pub category: RiskCategory,
    pub probability_of_harm: f32,
    pub worst_case_impact: f32,
    pub expected_impact: f32,
    pub reversibility: f32,
    pub cascading_risk: f32,
}

/// Counterfactual analysis — "what if we did something else?"
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CounterfactualResult {
    pub id: u64,
    pub original_decision: u64,
    pub alternative_kind: DecisionKind,
    pub alternative_outcome: f32,
    pub original_outcome: f32,
    pub regret: f32,
    pub description: String,
}

/// Final recommendation with confidence interval
#[derive(Debug, Clone)]
pub struct RehearsalRecommendation {
    pub decision_id: u64,
    pub recommendation: Recommendation,
    pub confidence: f32,
    pub confidence_lower: f32,
    pub confidence_upper: f32,
    pub supporting_evidence: u32,
    pub contradicting_evidence: u32,
}

// ============================================================================
// STATS
// ============================================================================

/// Aggregate rehearsal statistics
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct RehearsalStats {
    pub total_rehearsals: u64,
    pub total_counterfactuals: u64,
    pub avg_net_outcome: f32,
    pub avg_risk: f32,
    pub avg_confidence: f32,
    pub proceed_rate: f32,
    pub abort_rate: f32,
    pub regret_ema: f32,
}

// ============================================================================
// HOLISTIC REHEARSAL ENGINE
// ============================================================================

/// System-wide decision rehearsal engine. Simulates, evaluates, and
/// recommends before irreversible system actions are taken.
#[derive(Debug)]
pub struct HolisticRehearsal {
    rehearsals: BTreeMap<u64, RehearsalRecord>,
    impacts: BTreeMap<u64, ImpactAssessment>,
    risks: BTreeMap<u64, RiskEvaluation>,
    counterfactuals: BTreeMap<u64, CounterfactualResult>,
    total_rehearsals: u64,
    total_counterfactuals: u64,
    proceed_count: u64,
    abort_count: u64,
    tick: u64,
    rng_state: u64,
    outcome_ema: f32,
    risk_ema: f32,
    confidence_ema: f32,
    regret_ema: f32,
}

impl HolisticRehearsal {
    pub fn new() -> Self {
        Self {
            rehearsals: BTreeMap::new(),
            impacts: BTreeMap::new(),
            risks: BTreeMap::new(),
            counterfactuals: BTreeMap::new(),
            total_rehearsals: 0,
            total_counterfactuals: 0,
            proceed_count: 0,
            abort_count: 0,
            tick: 0,
            rng_state: 0xAEBE_AA5A_1E06_1BE0,
            outcome_ema: 0.0,
            risk_ema: 0.3,
            confidence_ema: 0.5,
            regret_ema: 0.0,
        }
    }

    /// Rehearse a major system decision
    #[inline]
    pub fn rehearse_decision(
        &mut self,
        kind: DecisionKind,
        description: String,
        estimated_benefit: f32,
        estimated_cost: f32,
    ) -> RehearsalRecord {
        self.tick += 1;
        self.total_rehearsals += 1;

        let noise = (xorshift64(&mut self.rng_state) % 100) as f32 / 500.0 - 0.1;
        let sim_benefit = (estimated_benefit + noise).clamp(0.0, 1.0);
        let sim_cost = (estimated_cost + noise * 0.5).clamp(0.0, 1.0);
        let net = sim_benefit - sim_cost;

        let risk = self.compute_risk_score(kind, sim_cost);

        let recommendation = if net > 0.4 && risk < 0.3 {
            self.proceed_count += 1;
            Recommendation::StrongProceed
        } else if net > 0.2 && risk < 0.5 {
            self.proceed_count += 1;
            Recommendation::Proceed
        } else if net > 0.0 && risk < 0.6 {
            self.proceed_count += 1;
            Recommendation::ProceedWithCaution
        } else if net > -0.1 {
            Recommendation::Defer
        } else {
            self.abort_count += 1;
            Recommendation::Abort
        };

        let base_conf = (0.5 + (self.total_rehearsals as f32).sqrt() * 0.01).clamp(0.0, 1.0);
        let confidence = (base_conf * (1.0 - risk * 0.3)).clamp(CONFIDENCE_MIN, 1.0);

        let id = fnv1a_hash(description.as_bytes()) ^ xorshift64(&mut self.rng_state);

        self.outcome_ema = EMA_ALPHA * net + (1.0 - EMA_ALPHA) * self.outcome_ema;
        self.risk_ema = EMA_ALPHA * risk + (1.0 - EMA_ALPHA) * self.risk_ema;
        self.confidence_ema = EMA_ALPHA * confidence + (1.0 - EMA_ALPHA) * self.confidence_ema;

        let rehearsal_ticks = match kind {
            DecisionKind::OomKill => 5,
            DecisionKind::ProcessMigration => 10,
            DecisionKind::ScaleUp | DecisionKind::ScaleDown => 8,
            DecisionKind::MemoryReclaim => 3,
            DecisionKind::IoReroute => 6,
            DecisionKind::ThermalThrottle => 4,
            DecisionKind::LoadShed => 7,
        };

        let record = RehearsalRecord {
            id,
            kind,
            description,
            rehearsal_ticks,
            simulated_benefit: sim_benefit,
            simulated_cost: sim_cost,
            net_outcome: net,
            risk_score: risk,
            recommendation,
            confidence,
            tick: self.tick,
        };

        self.rehearsals.insert(id, record.clone());
        if self.rehearsals.len() > MAX_REHEARSALS {
            if let Some((&oldest, _)) = self.rehearsals.iter().next() {
                self.rehearsals.remove(&oldest);
            }
        }

        record
    }

    /// Compute risk score based on decision kind and cost
    fn compute_risk_score(&self, kind: DecisionKind, cost: f32) -> f32 {
        let base_risk = match kind {
            DecisionKind::OomKill => 0.7,
            DecisionKind::ProcessMigration => 0.3,
            DecisionKind::ScaleUp => 0.2,
            DecisionKind::ScaleDown => 0.4,
            DecisionKind::MemoryReclaim => 0.25,
            DecisionKind::IoReroute => 0.35,
            DecisionKind::ThermalThrottle => 0.15,
            DecisionKind::LoadShed => 0.5,
        };
        (base_risk * 0.6 + cost * 0.4).clamp(0.0, 1.0)
    }

    /// Full impact assessment for a rehearsed decision
    pub fn impact_assessment(&mut self, decision_id: u64) -> ImpactAssessment {
        let record = self.rehearsals.get(&decision_id);

        let (kind, net) = record
            .map(|r| (r.kind, r.net_outcome))
            .unwrap_or((DecisionKind::OomKill, 0.0));

        let (cpu_impact, mem_impact, io_impact) = match kind {
            DecisionKind::OomKill => (-10.0, 0.3, -5.0),
            DecisionKind::ProcessMigration => (5.0, -0.05, 2.0),
            DecisionKind::ScaleUp => (-15.0, -0.1, -3.0),
            DecisionKind::ScaleDown => (8.0, 0.05, 1.0),
            DecisionKind::MemoryReclaim => (2.0, -0.2, 3.0),
            DecisionKind::IoReroute => (1.0, 0.0, -10.0),
            DecisionKind::ThermalThrottle => (10.0, 0.0, 0.0),
            DecisionKind::LoadShed => (-20.0, -0.15, -8.0),
        };

        let lat_impact = (net * -1000.0) as i64;
        let proc_delta = match kind {
            DecisionKind::OomKill => -1,
            DecisionKind::ScaleUp => 2,
            DecisionKind::ScaleDown => -1,
            _ => 0,
        };

        let stability = (0.5 + net * 0.5).clamp(0.0, 1.0);
        let recovery = match kind {
            DecisionKind::OomKill => 20,
            DecisionKind::ProcessMigration => 10,
            DecisionKind::ScaleUp | DecisionKind::ScaleDown => 15,
            _ => 5,
        };

        let impact = ImpactAssessment {
            decision_id,
            cpu_impact,
            memory_impact: mem_impact,
            io_impact,
            latency_impact_us: lat_impact,
            process_count_delta: proc_delta,
            stability_impact: stability,
            recovery_ticks: recovery,
        };

        self.impacts.insert(decision_id, impact.clone());
        if self.impacts.len() > MAX_IMPACT_ENTRIES {
            if let Some((&oldest, _)) = self.impacts.iter().next() {
                self.impacts.remove(&oldest);
            }
        }

        impact
    }

    /// Detailed risk evaluation
    pub fn risk_evaluation(&mut self, decision_id: u64) -> RiskEvaluation {
        let record = self.rehearsals.get(&decision_id);
        let (kind, risk_score) = record
            .map(|r| (r.kind, r.risk_score))
            .unwrap_or((DecisionKind::OomKill, 0.5));

        let category = if risk_score < 0.1 {
            RiskCategory::Negligible
        } else if risk_score < 0.3 {
            RiskCategory::Low
        } else if risk_score < 0.5 {
            RiskCategory::Moderate
        } else if risk_score < 0.7 {
            RiskCategory::High
        } else {
            RiskCategory::Extreme
        };

        let reversibility = match kind {
            DecisionKind::OomKill => 0.0,
            DecisionKind::ProcessMigration => 0.8,
            DecisionKind::ScaleUp => 0.9,
            DecisionKind::ScaleDown => 0.7,
            DecisionKind::MemoryReclaim => 0.4,
            DecisionKind::IoReroute => 0.85,
            DecisionKind::ThermalThrottle => 0.95,
            DecisionKind::LoadShed => 0.3,
        };

        let cascading = (risk_score * (1.0 - reversibility)).clamp(0.0, 1.0);

        let eval = RiskEvaluation {
            decision_id,
            category,
            probability_of_harm: risk_score,
            worst_case_impact: (risk_score * 1.5).clamp(0.0, 1.0),
            expected_impact: risk_score * 0.6,
            reversibility,
            cascading_risk: cascading,
        };

        self.risks.insert(decision_id, eval.clone());
        if self.risks.len() > MAX_RISK_ENTRIES {
            if let Some((&oldest, _)) = self.risks.iter().next() {
                self.risks.remove(&oldest);
            }
        }

        eval
    }

    /// Counterfactual analysis: what if a different decision was made?
    #[inline]
    pub fn counterfactual_analysis(
        &mut self,
        original_id: u64,
        alternative: DecisionKind,
    ) -> CounterfactualResult {
        self.total_counterfactuals += 1;

        let original_outcome = self
            .rehearsals
            .get(&original_id)
            .map(|r| r.net_outcome)
            .unwrap_or(0.0);

        let noise = (xorshift64(&mut self.rng_state) % 100) as f32 / 500.0 - 0.1;
        let alt_base = match alternative {
            DecisionKind::OomKill => -0.2,
            DecisionKind::ProcessMigration => 0.1,
            DecisionKind::ScaleUp => 0.15,
            DecisionKind::ScaleDown => -0.05,
            DecisionKind::MemoryReclaim => 0.1,
            DecisionKind::IoReroute => 0.05,
            DecisionKind::ThermalThrottle => 0.0,
            DecisionKind::LoadShed => -0.15,
        };
        let alt_outcome = (alt_base + noise).clamp(-1.0, 1.0);
        let regret = (alt_outcome - original_outcome).max(0.0);

        self.regret_ema = EMA_ALPHA * regret + (1.0 - EMA_ALPHA) * self.regret_ema;

        let id = FastHasher::new().feed_str("cf-").feed_u64(original_id as u64).feed_str("-").feed_u64(alternative as u64).finish()
            ^ xorshift64(&mut self.rng_state);

        let result = CounterfactualResult {
            id,
            original_decision: original_id,
            alternative_kind: alternative,
            alternative_outcome: alt_outcome,
            original_outcome,
            regret,
            description: String::from("counterfactual scenario"),
        };

        self.counterfactuals.insert(id, result.clone());
        if self.counterfactuals.len() > MAX_COUNTERFACTUALS {
            if let Some((&oldest, _)) = self.counterfactuals.iter().next() {
                self.counterfactuals.remove(&oldest);
            }
        }

        result
    }

    /// Get the final rehearsal recommendation with confidence bounds
    pub fn rehearsal_recommendation(&self, decision_id: u64) -> RehearsalRecommendation {
        let record = self.rehearsals.get(&decision_id);
        let (recommendation, confidence) = record
            .map(|r| (r.recommendation, r.confidence))
            .unwrap_or((Recommendation::Defer, 0.1));

        let supporting = self
            .counterfactuals
            .values()
            .filter(|cf| cf.original_decision == decision_id && cf.regret < 0.1)
            .count() as u32;

        let contradicting = self
            .counterfactuals
            .values()
            .filter(|cf| cf.original_decision == decision_id && cf.regret > 0.2)
            .count() as u32;

        let spread = 0.1 + contradicting as f32 * 0.05;

        RehearsalRecommendation {
            decision_id,
            recommendation,
            confidence,
            confidence_lower: (confidence - spread).clamp(0.0, 1.0),
            confidence_upper: (confidence + spread).clamp(0.0, 1.0),
            supporting_evidence: supporting,
            contradicting_evidence: contradicting,
        }
    }

    /// Decision confidence score combining all evidence
    pub fn decision_confidence(&self, decision_id: u64) -> f32 {
        let rehearsal_conf = self
            .rehearsals
            .get(&decision_id)
            .map(|r| r.confidence)
            .unwrap_or(0.0);

        let risk_factor = self
            .risks
            .get(&decision_id)
            .map(|r| 1.0 - r.probability_of_harm)
            .unwrap_or(0.5);

        let cf_factor = {
            let related: Vec<&CounterfactualResult> = self
                .counterfactuals
                .values()
                .filter(|cf| cf.original_decision == decision_id)
                .collect();
            if related.is_empty() {
                0.5
            } else {
                let avg_regret: f32 =
                    related.iter().map(|cf| cf.regret).sum::<f32>() / related.len() as f32;
                (1.0 - avg_regret).clamp(0.0, 1.0)
            }
        };

        (rehearsal_conf * 0.4 + risk_factor * 0.3 + cf_factor * 0.3).clamp(0.0, 1.0)
    }

    /// Gather aggregate statistics
    pub fn stats(&self) -> RehearsalStats {
        let total = self.proceed_count + self.abort_count;
        let proceed_rate = if total > 0 {
            self.proceed_count as f32 / total as f32
        } else {
            0.0
        };

        RehearsalStats {
            total_rehearsals: self.total_rehearsals,
            total_counterfactuals: self.total_counterfactuals,
            avg_net_outcome: self.outcome_ema,
            avg_risk: self.risk_ema,
            avg_confidence: self.confidence_ema,
            proceed_rate,
            abort_rate: 1.0 - proceed_rate,
            regret_ema: self.regret_ema,
        }
    }
}
