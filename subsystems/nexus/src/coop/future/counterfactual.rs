// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Counterfactual Analysis
//!
//! "What if we had shared differently?" engine. Evaluates alternative sharing
//! strategies, computes fairness regret, finds optimal alternatives, and
//! measures hindsight fairness for cooperative subsystem interactions.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

/// FNV-1a hash for deterministic key hashing in no_std.
fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Xorshift64 PRNG for lightweight stochastic perturbation.
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// Exponential moving average update.
fn ema_update(current: u64, new_sample: u64, alpha_num: u64, alpha_den: u64) -> u64 {
    let weighted_old = current.saturating_mul(alpha_den.saturating_sub(alpha_num));
    let weighted_new = new_sample.saturating_mul(alpha_num);
    weighted_old.saturating_add(weighted_new) / alpha_den.max(1)
}

/// An actual sharing decision that was made.
#[derive(Clone, Debug)]
pub struct SharingDecision {
    pub decision_id: u64,
    pub resource_id: u64,
    pub sharer_id: u64,
    pub receiver_id: u64,
    pub amount_shared: u64,
    pub tick: u64,
    pub actual_utility: i64,
    pub actual_fairness: u64,
}

/// An alternative sharing strategy to evaluate counterfactually.
#[derive(Clone, Debug)]
pub struct AlternativeStrategy {
    pub strategy_id: u64,
    pub description_hash: u64,
    pub sharing_ratio: u64,
    pub priority_weight: u64,
    pub cooperation_bonus: i64,
}

/// Result of a "what if" sharing analysis.
#[derive(Clone, Debug)]
pub struct WhatIfResult {
    pub decision_id: u64,
    pub alternative_id: u64,
    pub counterfactual_utility: i64,
    pub utility_delta: i64,
    pub counterfactual_fairness: u64,
    pub fairness_delta: i64,
    pub trust_impact: i64,
    pub would_have_improved: bool,
}

/// Fairness regret measurement for a period.
#[derive(Clone, Debug)]
pub struct FairnessRegret {
    pub period_start: u64,
    pub period_end: u64,
    pub actual_gini: u64,
    pub optimal_gini: u64,
    pub regret_score: u64,
    pub worst_decision: u64,
    pub improvement_potential: u64,
}

/// An identified optimal alternative for a past decision.
#[derive(Clone, Debug)]
pub struct OptimalAlternative {
    pub decision_id: u64,
    pub best_strategy_id: u64,
    pub best_utility: i64,
    pub best_fairness: u64,
    pub margin_over_actual: i64,
    pub implementation_cost: u64,
}

/// Counterfactual trust analysis result.
#[derive(Clone, Debug)]
#[repr(align(64))]
pub struct CounterfactualTrust {
    pub partner_id: u64,
    pub actual_trust: u64,
    pub counterfactual_trust: u64,
    pub trust_delta: i64,
    pub key_decision: u64,
    pub missed_opportunity: bool,
}

/// Hindsight fairness analysis for a cooperation period.
#[derive(Clone, Debug)]
pub struct HindsightFairness {
    pub period_ticks: u64,
    pub decisions_analyzed: u64,
    pub improvements_found: u64,
    pub avg_improvement: u64,
    pub fairness_score_actual: u64,
    pub fairness_score_optimal: u64,
    pub hindsight_regret: u64,
}

/// Rolling statistics for the counterfactual engine.
#[derive(Clone, Debug)]
#[repr(align(64))]
pub struct CounterfactualStats {
    pub what_if_analyses: u64,
    pub regret_computations: u64,
    pub optimal_alternatives_found: u64,
    pub trust_counterfactuals: u64,
    pub improvements_identified: u64,
    pub hindsight_analyses: u64,
    pub avg_regret: u64,
    pub avg_improvement_potential: u64,
}

impl CounterfactualStats {
    pub fn new() -> Self {
        Self {
            what_if_analyses: 0,
            regret_computations: 0,
            optimal_alternatives_found: 0,
            trust_counterfactuals: 0,
            improvements_identified: 0,
            hindsight_analyses: 0,
            avg_regret: 0,
            avg_improvement_potential: 0,
        }
    }
}

/// Internal record tracking a historical sharing decision and outcomes.
#[derive(Clone, Debug)]
struct DecisionRecord {
    decision: SharingDecision,
    context_hash: u64,
    resource_pressure: u64,
    trust_at_time: u64,
    partners_involved: Vec<u64>,
}

/// Internal record for tracking process sharing history.
#[derive(Clone, Debug)]
struct ProcessSharingHistory {
    process_id: u64,
    total_shared: u64,
    total_received: u64,
    ema_utility: i64,
    fairness_history: VecDeque<u64>,
}

/// Counterfactual analysis engine for cooperation decisions.
#[repr(align(64))]
pub struct CoopCounterfactual {
    decisions: BTreeMap<u64, DecisionRecord>,
    process_history: BTreeMap<u64, ProcessSharingHistory>,
    alternatives: BTreeMap<u64, AlternativeStrategy>,
    regret_history: Vec<u64>,
    stats: CounterfactualStats,
    rng_state: u64,
    current_tick: u64,
    max_decisions: usize,
}

impl CoopCounterfactual {
    /// Create a new counterfactual analysis engine.
    pub fn new(seed: u64) -> Self {
        let mut engine = Self {
            decisions: BTreeMap::new(),
            process_history: BTreeMap::new(),
            alternatives: BTreeMap::new(),
            regret_history: Vec::new(),
            stats: CounterfactualStats::new(),
            rng_state: seed ^ 0xC0F4_C7F4_C7F4_0001,
            current_tick: 0,
            max_decisions: 512,
        };
        engine.seed_default_alternatives();
        engine
    }

    /// Record a sharing decision that was made.
    pub fn record_decision(
        &mut self,
        resource_id: u64,
        sharer_id: u64,
        receiver_id: u64,
        amount: u64,
        utility: i64,
        fairness: u64,
        trust_level: u64,
        pressure: u64,
    ) {
        let decision_id = fnv1a_hash(&[
            resource_id.to_le_bytes().as_slice(),
            sharer_id.to_le_bytes().as_slice(),
            self.current_tick.to_le_bytes().as_slice(),
        ].concat());

        let decision = SharingDecision {
            decision_id,
            resource_id,
            sharer_id,
            receiver_id,
            amount_shared: amount,
            tick: self.current_tick,
            actual_utility: utility,
            actual_fairness: fairness,
        };

        let record = DecisionRecord {
            decision,
            context_hash: fnv1a_hash(&pressure.to_le_bytes()),
            resource_pressure: pressure,
            trust_at_time: trust_level,
            partners_involved: alloc::vec![sharer_id, receiver_id],
        };

        self.decisions.insert(decision_id, record);
        self.update_process_history(sharer_id, amount, 0, utility, fairness);
        self.update_process_history(receiver_id, 0, amount, utility, fairness);
        self.prune_decisions();
    }

    /// Analyze "what if" a different sharing strategy had been used.
    pub fn what_if_sharing(
        &mut self,
        decision_id: u64,
        alternative_id: u64,
    ) -> Option<WhatIfResult> {
        let record = self.decisions.get(&decision_id)?.clone();
        let alt = self.alternatives.get(&alternative_id)?.clone();

        let alt_amount = record.decision.amount_shared
            .saturating_mul(alt.sharing_ratio) / 1000;

        let base_utility = record.decision.actual_utility;
        let alt_utility = self.simulate_utility(
            alt_amount,
            record.resource_pressure,
            record.trust_at_time,
            alt.cooperation_bonus,
        );

        let alt_fairness = self.simulate_fairness(
            alt_amount,
            record.decision.amount_shared,
            record.decision.actual_fairness,
            alt.priority_weight,
        );

        let trust_impact = if alt_utility > base_utility {
            (alt_utility - base_utility).min(100)
        } else {
            (alt_utility - base_utility).max(-100)
        };

        let utility_delta = alt_utility - base_utility;
        let fairness_delta = alt_fairness as i64 - record.decision.actual_fairness as i64;

        self.stats.what_if_analyses = self.stats.what_if_analyses.saturating_add(1);
        if utility_delta > 0 || fairness_delta > 0 {
            self.stats.improvements_identified = self.stats.improvements_identified.saturating_add(1);
        }

        Some(WhatIfResult {
            decision_id,
            alternative_id,
            counterfactual_utility: alt_utility,
            utility_delta,
            counterfactual_fairness: alt_fairness,
            fairness_delta,
            trust_impact,
            would_have_improved: utility_delta > 0 && fairness_delta >= 0,
        })
    }

    /// Compute fairness regret over a period.
    pub fn fairness_regret(&mut self, period_start: u64, period_end: u64) -> FairnessRegret {
        let period_decisions: Vec<&DecisionRecord> = self.decisions.values()
            .filter(|d| d.decision.tick >= period_start && d.decision.tick <= period_end)
            .collect();

        let actual_gini = self.compute_gini(&period_decisions);

        let mut optimal_gini = actual_gini;
        let mut worst_decision: u64 = 0;
        let mut worst_regret: u64 = 0;

        for dec in &period_decisions {
            for alt in self.alternatives.values() {
                let alt_fairness = self.simulate_fairness(
                    dec.decision.amount_shared.saturating_mul(alt.sharing_ratio) / 1000,
                    dec.decision.amount_shared,
                    dec.decision.actual_fairness,
                    alt.priority_weight,
                );
                if alt_fairness < optimal_gini {
                    optimal_gini = alt_fairness;
                }
            }

            let decision_regret = actual_gini.saturating_sub(optimal_gini);
            if decision_regret > worst_regret {
                worst_regret = decision_regret;
                worst_decision = dec.decision.decision_id;
            }
        }

        let regret_score = actual_gini.saturating_sub(optimal_gini);
        let improvement_potential = regret_score.saturating_mul(100)
            / actual_gini.max(1);

        self.stats.regret_computations = self.stats.regret_computations.saturating_add(1);
        self.stats.avg_regret = ema_update(self.stats.avg_regret, regret_score, 200, 1000);
        self.regret_history.push(regret_score);

        FairnessRegret {
            period_start,
            period_end,
            actual_gini,
            optimal_gini,
            regret_score,
            worst_decision,
            improvement_potential,
        }
    }

    /// Find the optimal alternative for a past decision.
    pub fn optimal_alternative(&mut self, decision_id: u64) -> Option<OptimalAlternative> {
        let record = self.decisions.get(&decision_id)?.clone();

        let mut best_strategy: u64 = 0;
        let mut best_utility: i64 = i64::MIN;
        let mut best_fairness: u64 = 0;

        for alt in self.alternatives.values() {
            let alt_amount = record.decision.amount_shared
                .saturating_mul(alt.sharing_ratio) / 1000;
            let utility = self.simulate_utility(
                alt_amount,
                record.resource_pressure,
                record.trust_at_time,
                alt.cooperation_bonus,
            );
            let fairness = self.simulate_fairness(
                alt_amount,
                record.decision.amount_shared,
                record.decision.actual_fairness,
                alt.priority_weight,
            );

            let combined = utility.saturating_add(fairness as i64);
            let actual_combined = record.decision.actual_utility
                .saturating_add(record.decision.actual_fairness as i64);

            if combined > best_utility.saturating_add(best_fairness as i64) {
                best_strategy = alt.strategy_id;
                best_utility = utility;
                best_fairness = fairness;
            }
        }

        let margin = best_utility - record.decision.actual_utility;
        let cost = if margin > 0 { margin as u64 / 10 } else { 0 };

        self.stats.optimal_alternatives_found = self.stats.optimal_alternatives_found.saturating_add(1);
        self.stats.avg_improvement_potential = ema_update(
            self.stats.avg_improvement_potential,
            margin.max(0) as u64,
            150,
            1000,
        );

        Some(OptimalAlternative {
            decision_id,
            best_strategy_id: best_strategy,
            best_utility,
            best_fairness,
            margin_over_actual: margin,
            implementation_cost: cost,
        })
    }

    /// Analyze counterfactual trust with a partner.
    pub fn counterfactual_trust(&mut self, partner_id: u64) -> CounterfactualTrust {
        let history = self.process_history.get(&partner_id);
        let actual_trust = history
            .and_then(|h| h.fairness_history.last())
            .copied()
            .unwrap_or(500);

        let partner_decisions: Vec<&DecisionRecord> = self.decisions.values()
            .filter(|d| d.partners_involved.contains(&partner_id))
            .collect();

        let mut trust_delta_sum: i64 = 0;
        let mut key_decision: u64 = 0;
        let mut max_impact: i64 = 0;

        for dec in &partner_decisions {
            for alt in self.alternatives.values() {
                let alt_utility = self.simulate_utility(
                    dec.decision.amount_shared.saturating_mul(alt.sharing_ratio) / 1000,
                    dec.resource_pressure,
                    dec.trust_at_time,
                    alt.cooperation_bonus,
                );
                let delta = alt_utility - dec.decision.actual_utility;
                trust_delta_sum = trust_delta_sum.saturating_add(delta);

                if delta.abs() > max_impact.abs() {
                    max_impact = delta;
                    key_decision = dec.decision.decision_id;
                }
            }
        }

        let avg_delta = if !partner_decisions.is_empty() {
            trust_delta_sum / (partner_decisions.len() as i64 * self.alternatives.len().max(1) as i64).max(1)
        } else {
            0
        };

        let counterfactual_trust = if avg_delta > 0 {
            actual_trust.saturating_add(avg_delta as u64).min(1000)
        } else {
            actual_trust.saturating_sub(avg_delta.unsigned_abs())
        };

        self.stats.trust_counterfactuals = self.stats.trust_counterfactuals.saturating_add(1);

        CounterfactualTrust {
            partner_id,
            actual_trust,
            counterfactual_trust,
            trust_delta: avg_delta,
            key_decision,
            missed_opportunity: avg_delta > 50,
        }
    }

    /// Compute potential sharing improvement for recent decisions.
    pub fn sharing_improvement(&mut self, window: u64) -> u64 {
        let cutoff = self.current_tick.saturating_sub(window);
        let recent: Vec<u64> = self.decisions.values()
            .filter(|d| d.decision.tick >= cutoff)
            .map(|d| d.decision.decision_id)
            .collect();

        let mut total_improvement: u64 = 0;
        let count = recent.len().max(1) as u64;

        for did in recent {
            if let Some(opt) = self.optimal_alternative(did) {
                if opt.margin_over_actual > 0 {
                    total_improvement = total_improvement
                        .saturating_add(opt.margin_over_actual as u64);
                }
            }
        }

        total_improvement / count
    }

    /// Run hindsight fairness analysis for a cooperation period.
    pub fn hindsight_fairness(&mut self, period_ticks: u64) -> HindsightFairness {
        let cutoff = self.current_tick.saturating_sub(period_ticks);
        let period_decisions: Vec<u64> = self.decisions.values()
            .filter(|d| d.decision.tick >= cutoff)
            .map(|d| d.decision.decision_id)
            .collect();

        let decisions_analyzed = period_decisions.len() as u64;
        let mut improvements_found: u64 = 0;
        let mut total_improvement: u64 = 0;

        let mut fairness_actual_sum: u64 = 0;
        let mut fairness_optimal_sum: u64 = 0;

        for &did in &period_decisions {
            if let Some(record) = self.decisions.get(&did) {
                fairness_actual_sum = fairness_actual_sum
                    .saturating_add(record.decision.actual_fairness);
            }

            if let Some(opt) = self.optimal_alternative(did) {
                fairness_optimal_sum = fairness_optimal_sum.saturating_add(opt.best_fairness);
                if opt.margin_over_actual > 0 {
                    improvements_found += 1;
                    total_improvement = total_improvement
                        .saturating_add(opt.margin_over_actual as u64);
                }
            }
        }

        let count = decisions_analyzed.max(1);
        let avg_improvement = total_improvement / count;
        let fairness_actual = fairness_actual_sum / count;
        let fairness_optimal = fairness_optimal_sum / count;
        let hindsight_regret = fairness_optimal.saturating_sub(fairness_actual);

        self.stats.hindsight_analyses = self.stats.hindsight_analyses.saturating_add(1);

        HindsightFairness {
            period_ticks,
            decisions_analyzed,
            improvements_found,
            avg_improvement,
            fairness_score_actual: fairness_actual,
            fairness_score_optimal: fairness_optimal,
            hindsight_regret,
        }
    }

    /// Advance the internal tick.
    #[inline(always)]
    pub fn tick(&mut self) {
        self.current_tick = self.current_tick.wrapping_add(1);
    }

    /// Retrieve current statistics.
    #[inline(always)]
    pub fn stats(&self) -> &CounterfactualStats {
        &self.stats
    }

    // ── Private helpers ──────────────────────────────────────────────

    fn seed_default_alternatives(&mut self) {
        let configs: [(u64, u64, i64); 5] = [
            (500, 500, 0),    // equal split
            (800, 300, 20),   // generous
            (300, 800, -10),  // selfish
            (600, 600, 10),   // balanced cooperative
            (400, 400, -5),   // cautious
        ];

        for (ratio, weight, bonus) in &configs {
            let sid = fnv1a_hash(&[
                ratio.to_le_bytes().as_slice(),
                weight.to_le_bytes().as_slice(),
            ].concat());
            self.alternatives.insert(sid, AlternativeStrategy {
                strategy_id: sid,
                description_hash: fnv1a_hash(&sid.to_le_bytes()),
                sharing_ratio: *ratio,
                priority_weight: *weight,
                cooperation_bonus: *bonus,
            });
        }
    }

    fn simulate_utility(
        &self,
        amount: u64,
        pressure: u64,
        trust: u64,
        bonus: i64,
    ) -> i64 {
        let base = amount as i64;
        let pressure_factor = (1000u64.saturating_sub(pressure)) as i64;
        let trust_factor = trust as i64;
        let utility = base.saturating_mul(pressure_factor) / 1000
            + base.saturating_mul(trust_factor) / 2000
            + bonus;
        utility
    }

    fn simulate_fairness(
        &self,
        alt_amount: u64,
        actual_amount: u64,
        actual_fairness: u64,
        priority_weight: u64,
    ) -> u64 {
        let balance = if alt_amount > actual_amount {
            actual_fairness.saturating_add(
                (alt_amount - actual_amount).saturating_mul(priority_weight) / 1000,
            )
        } else {
            actual_fairness.saturating_sub(
                (actual_amount - alt_amount).saturating_mul(priority_weight) / 1000,
            )
        };
        balance.min(1000)
    }

    fn compute_gini(&self, decisions: &[&DecisionRecord]) -> u64 {
        if decisions.is_empty() {
            return 0;
        }

        let mut utilities: Vec<u64> = decisions.iter()
            .map(|d| d.decision.actual_utility.max(0) as u64)
            .collect();
        utilities.sort();

        let n = utilities.len() as u64;
        let total: u64 = utilities.iter().sum();
        if total == 0 {
            return 0;
        }

        let mut numerator: u64 = 0;
        for (i, &val) in utilities.iter().enumerate() {
            numerator = numerator.saturating_add(
                val.saturating_mul((2 * (i as u64) + 1).saturating_sub(n)),
            );
        }

        numerator.saturating_mul(1000) / (n.saturating_mul(total)).max(1)
    }

    fn update_process_history(
        &mut self,
        process_id: u64,
        shared: u64,
        received: u64,
        utility: i64,
        fairness: u64,
    ) {
        let record = self.process_history.entry(process_id)
            .or_insert_with(|| ProcessSharingHistory {
                process_id,
                total_shared: 0,
                total_received: 0,
                ema_utility: 0,
                fairness_history: VecDeque::new(),
            });
        record.total_shared = record.total_shared.saturating_add(shared);
        record.total_received = record.total_received.saturating_add(received);
        record.ema_utility = (record.ema_utility.saturating_mul(800)
            + utility.saturating_mul(200)) / 1000;
        record.fairness_history.push(fairness);
        if record.fairness_history.len() > 128 {
            record.fairness_history.pop_front().unwrap();
        }
    }

    fn prune_decisions(&mut self) {
        while self.decisions.len() > self.max_decisions {
            let oldest = self.decisions.iter()
                .min_by_key(|(_, v)| v.decision.tick)
                .map(|(&k, _)| k);
            if let Some(key) = oldest {
                self.decisions.remove(&key);
            } else {
                break;
            }
        }
    }
}
