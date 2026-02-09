// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Wisdom — Strategic Cooperation Intelligence
//!
//! Knows WHEN to cooperate, WHEN to compete, WHEN to negotiate, and WHEN to
//! yield.  Encodes strategic patience, crowd-wisdom aggregation, and sage
//! arbitration heuristics.  Decisions are scored via EMA over historical
//! outcomes, indexed through FNV-1a, and adapted by xorshift64-driven
//! stochastic exploration of the strategy space.

extern crate alloc;

use crate::fast::linear_map::LinearMap;
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
const MAX_DECISIONS: usize = 2048;
const MAX_PATIENCE_RECORDS: usize = 1024;
const MAX_MEDIATIONS: usize = 512;
const MAX_CROWD_VOTES: usize = 4096;
const PATIENCE_BONUS_PER_TICK: u64 = 2;
const MAX_PATIENCE: u64 = 200;
const COOPERATION_THRESHOLD: u64 = 60;
const COMPETE_THRESHOLD: u64 = 30;

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

fn clamp(val: u64, lo: u64, hi: u64) -> u64 {
    if val < lo { lo } else if val > hi { hi } else { val }
}

fn abs_diff(a: u64, b: u64) -> u64 {
    if a > b { a - b } else { b - a }
}

// ---------------------------------------------------------------------------
// Strategy
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum Strategy {
    Cooperate,
    Compete,
    Negotiate,
    Yield,
    Wait,
}

// ---------------------------------------------------------------------------
// Wisdom decision
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct WisdomDecision {
    pub decision_id: u64,
    pub context_hash: u64,
    pub chosen_strategy: Strategy,
    pub confidence: u64,
    pub patience_invested: u64,
    pub outcome: u64,
    pub creation_tick: u64,
}

// ---------------------------------------------------------------------------
// Patience record
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct PatienceRecord {
    pub record_id: u64,
    pub situation_hash: u64,
    pub patience_level: u64,
    pub wait_ticks: u64,
    pub reward_gained: u64,
    pub start_tick: u64,
}

// ---------------------------------------------------------------------------
// Mediation record
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct MediationRecord {
    pub mediation_id: u64,
    pub party_a: u64,
    pub party_b: u64,
    pub dispute_hash: u64,
    pub resolution_quality: u64,
    pub rounds_needed: u64,
    pub mediator_patience: u64,
    pub creation_tick: u64,
}

// ---------------------------------------------------------------------------
// Crowd vote
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct CrowdVote {
    pub vote_id: u64,
    pub question_hash: u64,
    pub voter_id: u64,
    pub vote_value: u64,
    pub confidence: u64,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
#[repr(align(64))]
pub struct WisdomStats {
    pub total_decisions: u64,
    pub cooperations_chosen: u64,
    pub competitions_chosen: u64,
    pub negotiations_chosen: u64,
    pub yields_chosen: u64,
    pub avg_patience: u64,
    pub avg_outcome: u64,
    pub mediations_performed: u64,
    pub crowd_verdicts: u64,
    pub wisdom_index: u64,
}

// ---------------------------------------------------------------------------
// Manager
// ---------------------------------------------------------------------------

pub struct CoopWisdom {
    decisions: BTreeMap<u64, WisdomDecision>,
    patience_records: BTreeMap<u64, PatienceRecord>,
    mediations: BTreeMap<u64, MediationRecord>,
    crowd_votes: BTreeMap<u64, CrowdVote>,
    outcome_history: LinearMap<u64, 64>,
    strategy_scores: LinearMap<u64, 64>,
    stats: WisdomStats,
    rng_state: u64,
    current_tick: u64,
}

impl CoopWisdom {
    pub fn new() -> Self {
        Self {
            decisions: BTreeMap::new(),
            patience_records: BTreeMap::new(),
            mediations: BTreeMap::new(),
            crowd_votes: BTreeMap::new(),
            outcome_history: LinearMap::new(),
            strategy_scores: LinearMap::new(),
            stats: WisdomStats::default(),
            rng_state: 0x5A5A_A5A5_3C3C_C3C3u64,
            current_tick: 0,
        }
    }

    // -----------------------------------------------------------------------
    // cooperation_wisdom — decide the best cooperation strategy
    // -----------------------------------------------------------------------
    pub fn cooperation_wisdom(
        &mut self,
        context: &str,
        trust_level: u64,
        resource_pressure: u64,
        conflict_intensity: u64,
    ) -> Strategy {
        self.current_tick += 1;
        let ctx_hash = fnv1a(context.as_bytes());

        // Multi-factor decision
        let coop_score = trust_level.wrapping_mul(3)
            + (100u64.saturating_sub(conflict_intensity)).wrapping_mul(2);
        let compete_score = conflict_intensity.wrapping_mul(2)
            + resource_pressure.wrapping_mul(2);
        let negotiate_score = trust_level + resource_pressure + conflict_intensity;
        let yield_score = if trust_level > 80 && conflict_intensity > 70 {
            trust_level + 50
        } else {
            trust_level / 2
        };

        // Add stochastic exploration
        let noise = xorshift64(&mut self.rng_state) % 30;
        let exploration = xorshift64(&mut self.rng_state) % 100;

        let strategy = if exploration < 5 {
            // Random exploration
            match xorshift64(&mut self.rng_state) % 4 {
                0 => Strategy::Cooperate,
                1 => Strategy::Compete,
                2 => Strategy::Negotiate,
                _ => Strategy::Yield,
            }
        } else {
            let adjusted_coop = coop_score + noise;
            let adjusted_compete = compete_score + (xorshift64(&mut self.rng_state) % 20);
            let adjusted_negotiate = negotiate_score + (xorshift64(&mut self.rng_state) % 15);
            let adjusted_yield = yield_score + (xorshift64(&mut self.rng_state) % 10);

            let max_score = core::cmp::max(
                core::cmp::max(adjusted_coop, adjusted_compete),
                core::cmp::max(adjusted_negotiate, adjusted_yield),
            );

            if max_score == adjusted_coop {
                Strategy::Cooperate
            } else if max_score == adjusted_negotiate {
                Strategy::Negotiate
            } else if max_score == adjusted_yield {
                Strategy::Yield
            } else {
                Strategy::Compete
            }
        };

        // Record decision
        let confidence = match strategy {
            Strategy::Cooperate => clamp(coop_score / 5, 10, 100),
            Strategy::Compete => clamp(compete_score / 4, 10, 100),
            Strategy::Negotiate => clamp(negotiate_score / 3, 10, 100),
            Strategy::Yield => clamp(yield_score, 10, 100),
            Strategy::Wait => 50,
        };

        let did = ctx_hash ^ self.current_tick;
        let decision = WisdomDecision {
            decision_id: did,
            context_hash: ctx_hash,
            chosen_strategy: strategy.clone(),
            confidence,
            patience_invested: 0,
            outcome: 0,
            creation_tick: self.current_tick,
        };

        if self.decisions.len() >= MAX_DECISIONS {
            let oldest = self.decisions.keys().next().copied();
            if let Some(k) = oldest { self.decisions.remove(&k); }
        }
        self.decisions.insert(did, decision);

        match strategy {
            Strategy::Cooperate => self.stats.cooperations_chosen += 1,
            Strategy::Compete => self.stats.competitions_chosen += 1,
            Strategy::Negotiate => self.stats.negotiations_chosen += 1,
            Strategy::Yield => self.stats.yields_chosen += 1,
            Strategy::Wait => {}
        }
        self.stats.total_decisions += 1;

        strategy
    }

    // -----------------------------------------------------------------------
    // strategic_patience — invest patience to wait for a better outcome
    // -----------------------------------------------------------------------
    pub fn strategic_patience(
        &mut self,
        situation: &str,
        current_offer: u64,
        expected_future: u64,
    ) -> u64 {
        self.current_tick += 1;
        let sit_hash = fnv1a(situation.as_bytes());
        let rid = sit_hash ^ self.current_tick;

        let patience = if expected_future > current_offer {
            let gap = expected_future - current_offer;
            clamp(gap * PATIENCE_BONUS_PER_TICK, 1, MAX_PATIENCE)
        } else {
            0
        };

        let record = PatienceRecord {
            record_id: rid,
            situation_hash: sit_hash,
            patience_level: patience,
            wait_ticks: patience / PATIENCE_BONUS_PER_TICK,
            reward_gained: 0,
            start_tick: self.current_tick,
        };

        if self.patience_records.len() >= MAX_PATIENCE_RECORDS {
            let oldest = self.patience_records.keys().next().copied();
            if let Some(k) = oldest { self.patience_records.remove(&k); }
        }
        self.patience_records.insert(rid, record);
        self.stats.avg_patience = ema_update(self.stats.avg_patience, patience);

        patience
    }

    // -----------------------------------------------------------------------
    // yield_or_compete — binary decision: yield or compete
    // -----------------------------------------------------------------------
    pub fn yield_or_compete(
        &mut self,
        own_strength: u64,
        opponent_strength: u64,
        relationship_value: u64,
    ) -> Strategy {
        self.current_tick += 1;

        let strength_ratio = if opponent_strength > 0 {
            own_strength.wrapping_mul(100) / opponent_strength
        } else {
            200
        };

        let relationship_factor = relationship_value;
        let noise = xorshift64(&mut self.rng_state) % 15;

        let compete_incentive = strength_ratio + noise;
        let yield_incentive = relationship_factor + (100u64.saturating_sub(strength_ratio / 2));

        if compete_incentive > yield_incentive + COMPETE_THRESHOLD {
            Strategy::Compete
        } else if yield_incentive > compete_incentive + COOPERATION_THRESHOLD {
            Strategy::Yield
        } else {
            Strategy::Negotiate
        }
    }

    // -----------------------------------------------------------------------
    // wise_mediation — mediate a dispute between two parties
    // -----------------------------------------------------------------------
    pub fn wise_mediation(
        &mut self,
        party_a: u64,
        party_b: u64,
        dispute_context: &str,
        severity: u64,
    ) -> u64 {
        self.current_tick += 1;
        let dispute_hash = fnv1a(dispute_context.as_bytes());
        let mid = dispute_hash ^ party_a ^ party_b ^ self.current_tick;

        let rounds = clamp(severity / 10 + 1, 1, 20);
        let patience_needed = rounds * 5;

        let base_quality = 100u64.saturating_sub(severity / 2);
        let patience_bonus = clamp(patience_needed / 3, 0, 20);
        let noise = xorshift64(&mut self.rng_state) % 15;
        let resolution_quality = clamp(base_quality + patience_bonus + noise, 10, 100);

        let mediation = MediationRecord {
            mediation_id: mid,
            party_a,
            party_b,
            dispute_hash,
            resolution_quality,
            rounds_needed: rounds,
            mediator_patience: patience_needed,
            creation_tick: self.current_tick,
        };

        if self.mediations.len() >= MAX_MEDIATIONS {
            let oldest = self.mediations.keys().next().copied();
            if let Some(k) = oldest { self.mediations.remove(&k); }
        }
        self.mediations.insert(mid, mediation);
        self.stats.mediations_performed += 1;

        resolution_quality
    }

    // -----------------------------------------------------------------------
    // wisdom_of_crowds — aggregate multiple opinions for a decision
    // -----------------------------------------------------------------------
    pub fn wisdom_of_crowds(
        &mut self,
        question: &str,
        votes: &[(u64, u64, u64)], // (voter_id, value, confidence)
    ) -> u64 {
        self.current_tick += 1;
        let q_hash = fnv1a(question.as_bytes());

        let mut weighted_sum: u64 = 0;
        let mut weight_total: u64 = 0;

        for &(voter_id, value, confidence) in votes {
            let vid = q_hash ^ voter_id ^ self.current_tick;
            let vote = CrowdVote {
                vote_id: vid,
                question_hash: q_hash,
                voter_id,
                vote_value: value,
                confidence,
                tick: self.current_tick,
            };

            if self.crowd_votes.len() >= MAX_CROWD_VOTES {
                let oldest = self.crowd_votes.keys().next().copied();
                if let Some(k) = oldest { self.crowd_votes.remove(&k); }
            }
            self.crowd_votes.insert(vid, vote);

            let weight = clamp(confidence, 1, 100);
            weighted_sum += value.wrapping_mul(weight);
            weight_total += weight;
        }

        let verdict = if weight_total > 0 {
            weighted_sum / weight_total
        } else {
            50
        };

        self.outcome_history.insert(q_hash, verdict);
        self.stats.crowd_verdicts += 1;

        verdict
    }

    // -----------------------------------------------------------------------
    // sage_arbitration — authoritative arbitration based on accumulated wisdom
    // -----------------------------------------------------------------------
    pub fn sage_arbitration(
        &mut self,
        dispute_context: &str,
        party_claims: &[(u64, u64)], // (party_id, claim_strength)
    ) -> u64 {
        self.current_tick += 1;
        let ctx_hash = fnv1a(dispute_context.as_bytes());

        if party_claims.is_empty() {
            return 50;
        }

        // Historical outcome wisdom
        let historical = self.outcome_history.get(ctx_hash).copied().unwrap_or(50);

        // Weighted claim analysis
        let total_strength: u64 = party_claims.iter().map(|&(_, s)| s).sum();
        let mut fairness_scores: Vec<u64> = Vec::new();

        for &(party_id, claim_strength) in party_claims {
            let proportion = if total_strength > 0 {
                claim_strength.wrapping_mul(100) / total_strength
            } else {
                100 / party_claims.len() as u64
            };

            // Look at party's historical cooperation
            let party_history = self
                .decisions
                .values()
                .filter(|d| d.context_hash == party_id)
                .map(|d| d.outcome)
                .sum::<u64>();
            let history_bonus = clamp(party_history / 10, 0, 20);

            fairness_scores.push(proportion + history_bonus);
        }

        let avg_fairness = fairness_scores.iter().sum::<u64>()
            / core::cmp::max(fairness_scores.len() as u64, 1);

        let wisdom_factor = (historical + avg_fairness) / 2;
        let noise = xorshift64(&mut self.rng_state) % 5;
        let verdict = clamp(wisdom_factor + noise, 0, 100);

        self.outcome_history.insert(ctx_hash, verdict);
        self.stats.wisdom_index = ema_update(self.stats.wisdom_index, verdict);

        verdict
    }

    // -----------------------------------------------------------------------
    // Maintenance
    // -----------------------------------------------------------------------

    pub fn tick(&mut self) {
        self.current_tick += 1;

        // Decay strategy scores
        let keys: Vec<u64> = self.strategy_scores.keys().copied().collect();
        for k in keys {
            if let Some(v) = self.strategy_scores.get_mut(&k) {
                *v = (*v * 98) / 100;
            }
        }

        // Stochastic pruning of old patience records
        let r = xorshift64(&mut self.rng_state) % 100;
        if r < 3 && !self.patience_records.is_empty() {
            let oldest = self.patience_records.keys().next().copied();
            if let Some(k) = oldest {
                self.patience_records.remove(&k);
            }
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &WisdomStats {
        &self.stats
    }

    #[inline(always)]
    pub fn decision_count(&self) -> usize {
        self.decisions.len()
    }

    #[inline(always)]
    pub fn mediation_count(&self) -> usize {
        self.mediations.len()
    }
}
