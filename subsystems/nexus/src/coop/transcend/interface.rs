// SPDX-License-Identifier: GPL-2.0
//! # Advanced Cooperation Interface
//!
//! Provides human-readable and machine-consumable explanations of cooperation
//! decisions, fairness reports, negotiation insights, cooperation narratives,
//! and actionable recommendations.  Designed for both kernel introspection
//! tooling and autonomous decision-feedback loops.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const EMA_ALPHA_NUM: u64 = 3;
const EMA_ALPHA_DEN: u64 = 10;
const MAX_EXPLANATIONS: usize = 2048;
const MAX_REPORTS: usize = 512;
const MAX_RECOMMENDATIONS: usize = 1024;
const NARRATIVE_WINDOW: usize = 64;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn fnv1a(data: &[u8]) -> u64 {
    let mut h = FNV_OFFSET;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

fn xorshift64(state: &mut u64) -> u64 {
    let mut s = *state;
    s ^= s << 13;
    s ^= s >> 7;
    s ^= s << 17;
    *state = s;
    s
}

fn ema_update(prev: u64, sample: u64) -> u64 {
    (EMA_ALPHA_NUM * sample + (EMA_ALPHA_DEN - EMA_ALPHA_NUM) * prev) / EMA_ALPHA_DEN
}

fn clamp(v: u64, lo: u64, hi: u64) -> u64 {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}

// ---------------------------------------------------------------------------
// Allocation explanation
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct AllocationExplanation {
    pub explanation_id: u64,
    pub agent_id: u64,
    pub allocated: u64,
    pub demanded: u64,
    pub satisfaction_pct: u64,
    pub reason_code: ReasonCode,
    pub fairness_contribution: u64,
    pub tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReasonCode {
    FullSatisfaction,
    ProportionalShare,
    PriorityElevation,
    ContentionReduction,
    AnticipatoryCut,
    EmergencyReserve,
}

// ---------------------------------------------------------------------------
// Fairness report
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct FairnessReport {
    pub report_id: u64,
    pub tick: u64,
    pub jain_index: u64,
    pub max_envy: u64,
    pub min_satisfaction: u64,
    pub max_satisfaction: u64,
    pub gini_coefficient: u64,
    pub agents_assessed: usize,
}

// ---------------------------------------------------------------------------
// Negotiation insight
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct NegotiationInsight {
    pub insight_id: u64,
    pub parties: Vec<u64>,
    pub conflict_severity: u64,
    pub suggested_concession: u64,
    pub predicted_rounds: u64,
    pub ema_resolution_speed: u64,
}

// ---------------------------------------------------------------------------
// Recommendation
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Recommendation {
    pub rec_id: u64,
    pub target_agent: u64,
    pub action: RecAction,
    pub expected_improvement: u64,
    pub confidence: u64,
    pub priority: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecAction {
    IncreaseDemand,
    DecreaseDemand,
    SeekAlternatePool,
    FormCoalition,
    YieldToHigherPriority,
    WaitAndRetry,
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct InterfaceStats {
    pub total_explanations: usize,
    pub total_reports: usize,
    pub total_insights: usize,
    pub total_recommendations: usize,
    pub avg_satisfaction: u64,
    pub avg_fairness: u64,
    pub avg_confidence: u64,
    pub narrative_events: usize,
}

// ---------------------------------------------------------------------------
// CoopInterface
// ---------------------------------------------------------------------------

pub struct CoopInterface {
    explanations: BTreeMap<u64, AllocationExplanation>,
    reports: BTreeMap<u64, FairnessReport>,
    insights: BTreeMap<u64, NegotiationInsight>,
    recommendations: BTreeMap<u64, Recommendation>,
    narrative_log: Vec<u64>,
    rng_state: u64,
    tick: u64,
    stats: InterfaceStats,
    satisfaction_ema: u64,
    fairness_ema: u64,
}

impl CoopInterface {
    pub fn new(seed: u64) -> Self {
        Self {
            explanations: BTreeMap::new(),
            reports: BTreeMap::new(),
            insights: BTreeMap::new(),
            recommendations: BTreeMap::new(),
            narrative_log: Vec::new(),
            rng_state: seed | 1,
            tick: 0,
            stats: InterfaceStats {
                total_explanations: 0,
                total_reports: 0,
                total_insights: 0,
                total_recommendations: 0,
                avg_satisfaction: 50,
                avg_fairness: 50,
                avg_confidence: 50,
                narrative_events: 0,
            },
            satisfaction_ema: 50,
            fairness_ema: 50,
        }
    }

    // -- explain allocation -------------------------------------------------

    pub fn explain_allocation(
        &mut self,
        agent_id: u64,
        allocated: u64,
        demanded: u64,
        priority: u64,
        contention_level: u64,
    ) -> AllocationExplanation {
        let satisfaction = if demanded > 0 {
            allocated * 100 / demanded
        } else {
            100
        };
        let reason = self.infer_reason(satisfaction, priority, contention_level);
        let fairness_contrib = self.compute_fairness_contribution(agent_id, satisfaction);

        let eid = fnv1a(&[agent_id.to_le_bytes(), self.tick.to_le_bytes()].concat());

        let expl = AllocationExplanation {
            explanation_id: eid,
            agent_id,
            allocated,
            demanded,
            satisfaction_pct: satisfaction,
            reason_code: reason,
            fairness_contribution: fairness_contrib,
            tick: self.tick,
        };

        if self.explanations.len() < MAX_EXPLANATIONS {
            self.explanations.insert(eid, expl.clone());
        }
        self.satisfaction_ema = ema_update(self.satisfaction_ema, satisfaction);
        self.push_narrative_event(eid);
        expl
    }

    fn infer_reason(&self, satisfaction: u64, priority: u64, contention: u64) -> ReasonCode {
        if satisfaction >= 100 {
            ReasonCode::FullSatisfaction
        } else if priority > 80 {
            ReasonCode::PriorityElevation
        } else if contention > 70 {
            ReasonCode::ContentionReduction
        } else if satisfaction < 30 {
            ReasonCode::EmergencyReserve
        } else if satisfaction < 80 {
            ReasonCode::AnticipatoryCut
        } else {
            ReasonCode::ProportionalShare
        }
    }

    fn compute_fairness_contribution(&self, _agent_id: u64, satisfaction: u64) -> u64 {
        let global_avg = self.satisfaction_ema;
        let deviation = if satisfaction > global_avg {
            satisfaction - global_avg
        } else {
            global_avg - satisfaction
        };
        100u64.saturating_sub(deviation)
    }

    // -- fairness report ----------------------------------------------------

    pub fn fairness_report(&mut self, agent_satisfactions: &[(u64, u64)]) -> FairnessReport {
        let n = agent_satisfactions.len();
        if n == 0 {
            return FairnessReport {
                report_id: 0,
                tick: self.tick,
                jain_index: 100,
                max_envy: 0,
                min_satisfaction: 100,
                max_satisfaction: 100,
                gini_coefficient: 0,
                agents_assessed: 0,
            };
        }

        let sats: Vec<u64> = agent_satisfactions.iter().map(|&(_, s)| s).collect();
        let sum: u64 = sats.iter().sum();
        let sum_sq: u64 = sats.iter().map(|&s| s * s).sum();
        let n_u64 = n as u64;

        // Jain's fairness index
        let jain = if sum_sq > 0 && n_u64 > 0 {
            let numerator = (sum * sum) / n_u64;
            clamp(numerator * 100 / sum_sq, 0, 100)
        } else {
            100
        };

        let min_sat = *sats.iter().min().unwrap_or(&0);
        let max_sat = *sats.iter().max().unwrap_or(&100);
        let max_envy = max_sat.saturating_sub(min_sat);

        let gini = self.compute_gini(&sats);

        let rid = fnv1a(&self.tick.to_le_bytes());
        let report = FairnessReport {
            report_id: rid,
            tick: self.tick,
            jain_index: jain,
            max_envy,
            min_satisfaction: min_sat,
            max_satisfaction: max_sat,
            gini_coefficient: gini,
            agents_assessed: n,
        };

        if self.reports.len() < MAX_REPORTS {
            self.reports.insert(rid, report.clone());
        }
        self.fairness_ema = ema_update(self.fairness_ema, jain);
        self.push_narrative_event(rid);
        report
    }

    fn compute_gini(&self, values: &[u64]) -> u64 {
        let n = values.len();
        if n < 2 {
            return 0;
        }
        let mean = values.iter().sum::<u64>() / n as u64;
        if mean == 0 {
            return 0;
        }
        let mut abs_diff_sum = 0u64;
        for i in 0..n {
            for j in 0..n {
                abs_diff_sum += if values[i] > values[j] {
                    values[i] - values[j]
                } else {
                    values[j] - values[i]
                };
            }
        }
        let gini = abs_diff_sum * 100 / (2 * n as u64 * n as u64 * mean);
        clamp(gini, 0, 100)
    }

    // -- negotiation insight ------------------------------------------------

    pub fn negotiation_insight(&mut self, parties: &[u64], severity: u64) -> NegotiationInsight {
        let iid = fnv1a(
            &parties
                .iter()
                .flat_map(|p| p.to_le_bytes())
                .collect::<Vec<u8>>(),
        );

        let concession = self.suggest_concession(severity, parties.len() as u64);
        let rounds = self.predict_rounds(severity, parties.len() as u64);

        let existing_speed = self
            .insights
            .get(&iid)
            .map(|i| i.ema_resolution_speed)
            .unwrap_or(50);
        let resolution_speed = ema_update(existing_speed, 100u64.saturating_sub(rounds));

        let insight = NegotiationInsight {
            insight_id: iid,
            parties: parties.to_vec(),
            conflict_severity: severity,
            suggested_concession: concession,
            predicted_rounds: rounds,
            ema_resolution_speed: resolution_speed,
        };

        self.insights.insert(iid, insight.clone());
        self.push_narrative_event(iid);
        insight
    }

    fn suggest_concession(&self, severity: u64, num_parties: u64) -> u64 {
        let base = severity / 2;
        let party_factor = clamp(num_parties * 5, 5, 30);
        clamp(base + party_factor, 5, 80)
    }

    fn predict_rounds(&self, severity: u64, num_parties: u64) -> u64 {
        clamp(severity / 10 + num_parties, 1, 50)
    }

    // -- cooperation narrative ----------------------------------------------

    pub fn cooperation_narrative(&self) -> Vec<u64> {
        self.narrative_log.clone()
    }

    fn push_narrative_event(&mut self, event_id: u64) {
        self.narrative_log.push(event_id);
        if self.narrative_log.len() > NARRATIVE_WINDOW {
            self.narrative_log.remove(0);
        }
    }

    // -- recommendation -----------------------------------------------------

    pub fn recommendation(
        &mut self,
        agent_id: u64,
        satisfaction: u64,
        contention: u64,
    ) -> Recommendation {
        let action = self.decide_action(satisfaction, contention);
        let improvement = self.estimate_improvement(&action, satisfaction);
        let confidence = self.compute_rec_confidence(satisfaction, contention);
        let priority = self.compute_priority(satisfaction, contention);

        let rid = fnv1a(
            &[
                agent_id.to_le_bytes(),
                self.tick.to_le_bytes(),
                satisfaction.to_le_bytes(),
            ]
            .concat(),
        );

        let rec = Recommendation {
            rec_id: rid,
            target_agent: agent_id,
            action,
            expected_improvement: improvement,
            confidence,
            priority,
        };

        if self.recommendations.len() < MAX_RECOMMENDATIONS {
            self.recommendations.insert(rid, rec.clone());
        }
        self.push_narrative_event(rid);
        rec
    }

    fn decide_action(&self, satisfaction: u64, contention: u64) -> RecAction {
        if satisfaction >= 90 && contention < 20 {
            RecAction::IncreaseDemand
        } else if satisfaction < 30 && contention > 60 {
            RecAction::SeekAlternatePool
        } else if satisfaction < 50 && contention > 80 {
            RecAction::YieldToHigherPriority
        } else if contention > 50 {
            RecAction::FormCoalition
        } else if satisfaction < 60 {
            RecAction::WaitAndRetry
        } else {
            RecAction::DecreaseDemand
        }
    }

    fn estimate_improvement(&self, action: &RecAction, current_sat: u64) -> u64 {
        match action {
            RecAction::IncreaseDemand => 5,
            RecAction::DecreaseDemand => 10,
            RecAction::SeekAlternatePool => 30,
            RecAction::FormCoalition => 20,
            RecAction::YieldToHigherPriority => 15,
            RecAction::WaitAndRetry => clamp(100u64.saturating_sub(current_sat) / 3, 5, 25),
        }
    }

    fn compute_rec_confidence(&self, _satisfaction: u64, contention: u64) -> u64 {
        let data_factor = clamp(self.explanations.len() as u64, 0, 50);
        let stability = 100u64.saturating_sub(contention) / 2;
        clamp(data_factor + stability, 10, 95)
    }

    fn compute_priority(&self, satisfaction: u64, contention: u64) -> u64 {
        let urgency = 100u64.saturating_sub(satisfaction);
        let severity = contention;
        (urgency + severity) / 2
    }

    // -- tick ---------------------------------------------------------------

    pub fn tick(&mut self) {
        self.tick += 1;
        self.refresh_stats();
    }

    // -- stats --------------------------------------------------------------

    fn refresh_stats(&mut self) {
        let avg_conf = if self.recommendations.is_empty() {
            50
        } else {
            self.recommendations
                .values()
                .map(|r| r.confidence)
                .sum::<u64>()
                / self.recommendations.len() as u64
        };

        self.stats = InterfaceStats {
            total_explanations: self.explanations.len(),
            total_reports: self.reports.len(),
            total_insights: self.insights.len(),
            total_recommendations: self.recommendations.len(),
            avg_satisfaction: self.satisfaction_ema,
            avg_fairness: self.fairness_ema,
            avg_confidence: avg_conf,
            narrative_events: self.narrative_log.len(),
        };
    }

    pub fn stats(&self) -> InterfaceStats {
        self.stats.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explain_allocation() {
        let mut ci = CoopInterface::new(42);
        let expl = ci.explain_allocation(1, 80, 100, 50, 30);
        assert_eq!(expl.satisfaction_pct, 80);
        assert_eq!(expl.reason_code, ReasonCode::AnticipatoryCut);
    }

    #[test]
    fn test_fairness_report() {
        let mut ci = CoopInterface::new(7);
        let report = ci.fairness_report(&[(1, 80), (2, 85), (3, 78)]);
        assert!(report.jain_index >= 90);
        assert!(report.gini_coefficient < 50);
    }

    #[test]
    fn test_negotiation_insight() {
        let mut ci = CoopInterface::new(99);
        let insight = ci.negotiation_insight(&[1, 2, 3], 60);
        assert!(insight.suggested_concession > 0);
        assert!(insight.predicted_rounds >= 1);
    }

    #[test]
    fn test_recommendation() {
        let mut ci = CoopInterface::new(55);
        let rec = ci.recommendation(1, 40, 70);
        assert_eq!(rec.action, RecAction::SeekAlternatePool);
        assert!(rec.expected_improvement > 0);
    }
}
