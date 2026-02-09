// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Rehearsal Engine
//!
//! Dry-runs negotiations, tests protocol changes, and evaluates fairness of
//! proposed cooperation policies before they are deployed. Provides a safe
//! sandbox for exploring cooperation strategy changes.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// FNV-1a hash for deterministic identifiers.
fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Xorshift64 PRNG for simulation randomness.
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// EMA update for running averages.
fn ema_update(current: u64, sample: u64, alpha_num: u64, alpha_den: u64) -> u64 {
    let old_part = current.saturating_mul(alpha_den.saturating_sub(alpha_num));
    let new_part = sample.saturating_mul(alpha_num);
    old_part.saturating_add(new_part) / alpha_den.max(1)
}

/// Result of a negotiation rehearsal.
#[derive(Clone, Debug)]
pub struct NegotiationRehearsalResult {
    pub rehearsal_id: u64,
    pub participants: Vec<u64>,
    pub rounds_executed: u64,
    pub agreement_reached: bool,
    pub final_allocation: BTreeMap<u64, u64>,
    pub fairness_index: u64,
    pub negotiation_cost: u64,
}

/// Result of a protocol test.
#[derive(Clone, Debug)]
pub struct ProtocolTestResult {
    pub test_id: u64,
    pub protocol_hash: u64,
    pub messages_exchanged: u64,
    pub convergence_rounds: u64,
    pub overhead_ratio: u64,
    pub correctness_score: u64,
    pub regression_detected: bool,
}

/// Result of a fairness evaluation.
#[derive(Clone, Debug)]
pub struct FairnessResult {
    pub policy_hash: u64,
    pub gini_coefficient: u64,
    pub min_share: u64,
    pub max_share: u64,
    pub envy_free: bool,
    pub proportional: bool,
    pub overall_fairness: u64,
}

/// Outcome prediction from a rehearsal.
#[derive(Clone, Debug)]
pub struct OutcomePrediction {
    pub scenario_hash: u64,
    pub predicted_cooperation_rate: u64,
    pub predicted_contention: u64,
    pub predicted_throughput: u64,
    pub confidence: u64,
}

/// Rolling statistics for the rehearsal engine.
#[derive(Clone, Debug)]
pub struct RehearsalStats {
    pub rehearsals_run: u64,
    pub protocols_tested: u64,
    pub fairness_evaluations: u64,
    pub outcomes_predicted: u64,
    pub avg_fairness: u64,
    pub avg_quality: u64,
    pub agreements_reached: u64,
    pub regressions_detected: u64,
}

impl RehearsalStats {
    pub fn new() -> Self {
        Self {
            rehearsals_run: 0,
            protocols_tested: 0,
            fairness_evaluations: 0,
            outcomes_predicted: 0,
            avg_fairness: 500,
            avg_quality: 500,
            agreements_reached: 0,
            regressions_detected: 0,
        }
    }
}

/// Internal negotiation agent for rehearsal.
#[derive(Clone, Debug)]
struct RehearsalAgent {
    agent_id: u64,
    demand: u64,
    flexibility: u64,
    trust_bias: u64,
    current_offer: u64,
}

/// Internal protocol model for testing.
#[derive(Clone, Debug)]
struct ProtocolModel {
    protocol_hash: u64,
    message_overhead: u64,
    convergence_threshold: u64,
    max_rounds: u64,
    correctness_baseline: u64,
}

/// Internal quality record for rehearsal tracking.
#[derive(Clone, Debug)]
struct QualityRecord {
    rehearsal_id: u64,
    quality_score: u64,
    deviation_from_expected: u64,
}

/// Cooperation rehearsal engine.
pub struct CoopRehearsal {
    agents: BTreeMap<u64, RehearsalAgent>,
    protocols: BTreeMap<u64, ProtocolModel>,
    quality_history: Vec<QualityRecord>,
    fairness_history: Vec<u64>,
    stats: RehearsalStats,
    rng_state: u64,
    max_agents: usize,
    max_history: usize,
}

impl CoopRehearsal {
    /// Create a new rehearsal engine with the given seed.
    pub fn new(seed: u64) -> Self {
        Self {
            agents: BTreeMap::new(),
            protocols: BTreeMap::new(),
            quality_history: Vec::new(),
            fairness_history: Vec::new(),
            stats: RehearsalStats::new(),
            rng_state: seed | 1,
            max_agents: 64,
            max_history: 128,
        }
    }

    /// Register a negotiation agent for rehearsal.
    pub fn register_agent(&mut self, agent_id: u64, demand: u64, flexibility: u64, trust_bias: u64) {
        if self.agents.len() >= self.max_agents {
            return;
        }
        self.agents.insert(agent_id, RehearsalAgent {
            agent_id,
            demand,
            flexibility: flexibility.min(1000),
            trust_bias: trust_bias.min(1000),
            current_offer: demand,
        });
    }

    /// Register a protocol model for testing.
    pub fn register_protocol(&mut self, name: &str, overhead: u64, threshold: u64, max_rounds: u64) {
        let hash = fnv1a_hash(name.as_bytes());
        self.protocols.insert(hash, ProtocolModel {
            protocol_hash: hash,
            message_overhead: overhead,
            convergence_threshold: threshold,
            max_rounds,
            correctness_baseline: 900,
        });
    }

    /// Rehearse a negotiation between specified participants.
    pub fn rehearse_negotiation(
        &mut self,
        participant_ids: Vec<u64>,
        total_resource: u64,
        max_rounds: u64,
    ) -> NegotiationRehearsalResult {
        self.stats.rehearsals_run = self.stats.rehearsals_run.saturating_add(1);
        let rehearsal_id = fnv1a_hash(&self.stats.rehearsals_run.to_le_bytes());

        let mut allocation: BTreeMap<u64, u64> = BTreeMap::new();
        let mut agreement = false;
        let mut rounds_used: u64 = 0;
        let mut cost: u64 = 0;

        for &pid in &participant_ids {
            if let Some(agent) = self.agents.get_mut(&pid) {
                agent.current_offer = agent.demand;
            }
        }

        for round in 0..max_rounds {
            rounds_used = round + 1;
            cost = cost.saturating_add(participant_ids.len() as u64 * 5);

            let mut total_demand: u64 = 0;
            for &pid in &participant_ids {
                if let Some(agent) = self.agents.get(&pid) {
                    total_demand = total_demand.saturating_add(agent.current_offer);
                }
            }

            if total_demand <= total_resource {
                for &pid in &participant_ids {
                    if let Some(agent) = self.agents.get(&pid) {
                        allocation.insert(pid, agent.current_offer);
                    }
                }
                let remainder = total_resource.saturating_sub(total_demand);
                let share = remainder / participant_ids.len().max(1) as u64;
                for &pid in &participant_ids {
                    if let Some(val) = allocation.get_mut(&pid) {
                        *val = val.saturating_add(share);
                    }
                }
                agreement = true;
                break;
            }

            let excess = total_demand.saturating_sub(total_resource);
            for &pid in &participant_ids {
                if let Some(agent) = self.agents.get_mut(&pid) {
                    let noise = xorshift64(&mut self.rng_state) % 20;
                    let concession = (agent.flexibility.saturating_mul(excess))
                        / (total_demand.max(1).saturating_mul(10))
                        + noise;
                    agent.current_offer = agent.current_offer.saturating_sub(concession);
                    agent.current_offer = agent.current_offer.max(agent.demand / 4);
                }
            }
        }

        if !agreement {
            let fair_share = total_resource / participant_ids.len().max(1) as u64;
            for &pid in &participant_ids {
                allocation.insert(pid, fair_share);
            }
        }

        if agreement {
            self.stats.agreements_reached = self.stats.agreements_reached.saturating_add(1);
        }
        let fairness = self.compute_fairness_index(&allocation, total_resource);

        NegotiationRehearsalResult {
            rehearsal_id,
            participants: participant_ids,
            rounds_executed: rounds_used,
            agreement_reached: agreement,
            final_allocation: allocation,
            fairness_index: fairness,
            negotiation_cost: cost,
        }
    }

    /// Test a protocol by simulating message exchange and convergence.
    pub fn test_protocol(&mut self, protocol_name: &str, num_participants: u64) -> ProtocolTestResult {
        self.stats.protocols_tested = self.stats.protocols_tested.saturating_add(1);
        let hash = fnv1a_hash(protocol_name.as_bytes());
        let test_id = fnv1a_hash(&[
            hash.to_le_bytes(),
            self.stats.protocols_tested.to_le_bytes(),
        ].concat());

        let model = self.protocols.get(&hash).cloned().unwrap_or(ProtocolModel {
            protocol_hash: hash,
            message_overhead: 10,
            convergence_threshold: 50,
            max_rounds: 100,
            correctness_baseline: 800,
        });

        let mut messages: u64 = 0;
        let mut convergence_round: u64 = model.max_rounds;
        let mut state_values: Vec<u64> = (0..num_participants)
            .map(|i| 500 + (xorshift64(&mut self.rng_state) % 500) + i * 10)
            .collect();

        for round in 0..model.max_rounds {
            let msg_this_round = num_participants.saturating_mul(num_participants.saturating_sub(1));
            messages = messages.saturating_add(msg_this_round.saturating_add(model.message_overhead));

            let avg: u64 = state_values.iter().sum::<u64>() / state_values.len().max(1) as u64;
            for val in state_values.iter_mut() {
                let noise = (xorshift64(&mut self.rng_state) % 20) as i64 - 10;
                let diff = if *val > avg { *val - avg } else { avg - *val };
                let adjustment = diff / 3;
                if *val > avg {
                    *val = val.saturating_sub(adjustment);
                } else {
                    *val = val.saturating_add(adjustment);
                }
                *val = if noise >= 0 {
                    val.saturating_add(noise as u64)
                } else {
                    val.saturating_sub((-noise) as u64)
                };
            }

            let max_val = state_values.iter().max().copied().unwrap_or(0);
            let min_val = state_values.iter().min().copied().unwrap_or(0);
            if max_val.saturating_sub(min_val) <= model.convergence_threshold {
                convergence_round = round + 1;
                break;
            }
        }

        let overhead = if messages > 0 {
            model.message_overhead.saturating_mul(model.max_rounds).saturating_mul(1000) / messages
        } else {
            0
        };

        let noise_factor = xorshift64(&mut self.rng_state) % 50;
        let correctness = model.correctness_baseline.saturating_sub(
            convergence_round.saturating_mul(2)
        ).saturating_add(noise_factor).min(1000);

        let regression = correctness < model.correctness_baseline.saturating_mul(8) / 10;
        if regression {
            self.stats.regressions_detected = self.stats.regressions_detected.saturating_add(1);
        }

        ProtocolTestResult {
            test_id,
            protocol_hash: hash,
            messages_exchanged: messages,
            convergence_rounds: convergence_round,
            overhead_ratio: overhead,
            correctness_score: correctness,
            regression_detected: regression,
        }
    }

    /// Evaluate fairness of a proposed resource distribution policy.
    pub fn fairness_rehearsal(&mut self, shares: &BTreeMap<u64, u64>, total_resource: u64) -> FairnessResult {
        self.stats.fairness_evaluations = self.stats.fairness_evaluations.saturating_add(1);

        let policy_hash = {
            let mut combined: Vec<u8> = Vec::new();
            for (&k, &v) in shares.iter() {
                combined.extend_from_slice(&k.to_le_bytes());
                combined.extend_from_slice(&v.to_le_bytes());
            }
            fnv1a_hash(&combined)
        };

        let fairness = self.compute_fairness_index(shares, total_resource);
        let values: Vec<u64> = shares.values().copied().collect();
        let min_share = values.iter().min().copied().unwrap_or(0);
        let max_share = values.iter().max().copied().unwrap_or(0);

        let n = values.len() as u64;
        let fair_share = if n > 0 { total_resource / n } else { 0 };
        let proportional = values.iter().all(|&v| {
            let diff = if v > fair_share { v - fair_share } else { fair_share - v };
            diff <= fair_share / 4
        });

        let envy_free = {
            let mut no_envy = true;
            for (i, &vi) in values.iter().enumerate() {
                for (j, &vj) in values.iter().enumerate() {
                    if i != j && vj > vi.saturating_add(vi / 5) {
                        no_envy = false;
                        break;
                    }
                }
                if !no_envy {
                    break;
                }
            }
            no_envy
        };

        let gini = self.compute_gini(&values);
        if self.fairness_history.len() >= self.max_history {
            self.fairness_history.remove(0);
        }
        self.fairness_history.push(fairness);
        self.stats.avg_fairness = ema_update(self.stats.avg_fairness, fairness, 3, 10);

        FairnessResult {
            policy_hash,
            gini_coefficient: gini,
            min_share,
            max_share,
            envy_free,
            proportional,
            overall_fairness: fairness,
        }
    }

    /// Predict the outcome of a cooperation scenario based on rehearsal data.
    pub fn outcome_prediction(&mut self, participant_ids: &[u64], resource_pressure: u64) -> OutcomePrediction {
        self.stats.outcomes_predicted = self.stats.outcomes_predicted.saturating_add(1);

        let mut hash_input: Vec<u8> = Vec::new();
        for &pid in participant_ids {
            hash_input.extend_from_slice(&pid.to_le_bytes());
        }
        let scenario_hash = fnv1a_hash(&hash_input);

        let mut avg_flexibility: u64 = 0;
        let mut avg_trust: u64 = 0;
        let mut count: u64 = 0;
        for &pid in participant_ids {
            if let Some(agent) = self.agents.get(&pid) {
                avg_flexibility = avg_flexibility.saturating_add(agent.flexibility);
                avg_trust = avg_trust.saturating_add(agent.trust_bias);
                count += 1;
            }
        }
        if count > 0 {
            avg_flexibility /= count;
            avg_trust /= count;
        }

        let coop_rate = avg_trust.saturating_mul(6) / 10
            + avg_flexibility.saturating_mul(3) / 10
            + (1000u64.saturating_sub(resource_pressure.min(1000))) / 10;

        let contention = resource_pressure.saturating_mul(7) / 10
            + (1000u64.saturating_sub(avg_flexibility)).saturating_mul(2) / 10
            + (1000u64.saturating_sub(avg_trust)) / 10;

        let throughput = coop_rate.saturating_mul(8) / 10
            + (1000u64.saturating_sub(contention)).saturating_mul(2) / 10;

        let confidence = (count.min(10).saturating_mul(80))
            .saturating_add(self.stats.rehearsals_run.min(20).saturating_mul(10))
            .min(1000);

        OutcomePrediction {
            scenario_hash,
            predicted_cooperation_rate: coop_rate.min(1000),
            predicted_contention: contention.min(1000),
            predicted_throughput: throughput.min(1000),
            confidence,
        }
    }

    /// Compute an overall quality score for the last N rehearsals.
    pub fn rehearsal_quality(&self) -> u64 {
        if self.quality_history.is_empty() {
            return self.stats.avg_quality;
        }
        let recent: Vec<&QualityRecord> = self.quality_history.iter()
            .rev().take(16).collect();
        let sum: u64 = recent.iter().map(|r| r.quality_score).sum();
        sum / recent.len().max(1) as u64
    }

    /// Get the current statistics snapshot.
    pub fn stats(&self) -> &RehearsalStats {
        &self.stats
    }

    /// Compute the fairness index (Jain's fairness) for an allocation.
    fn compute_fairness_index(&self, allocation: &BTreeMap<u64, u64>, _total: u64) -> u64 {
        let values: Vec<u64> = allocation.values().copied().collect();
        if values.is_empty() {
            return 1000;
        }
        let n = values.len() as u64;
        let sum: u64 = values.iter().sum();
        let sum_sq: u64 = values.iter().map(|&v| v.saturating_mul(v)).sum();
        if sum_sq == 0 || n == 0 {
            return 1000;
        }
        let numerator = sum.saturating_mul(sum);
        let denominator = n.saturating_mul(sum_sq);
        numerator.saturating_mul(1000) / denominator.max(1)
    }

    /// Compute a simplified Gini coefficient (scaled 0-1000).
    fn compute_gini(&self, values: &[u64]) -> u64 {
        if values.is_empty() {
            return 0;
        }
        let n = values.len();
        let sum: u64 = values.iter().sum();
        if sum == 0 {
            return 0;
        }
        let mut abs_diff_sum: u64 = 0;
        for i in 0..n {
            for j in 0..n {
                abs_diff_sum = abs_diff_sum.saturating_add(
                    if values[i] > values[j] {
                        values[i] - values[j]
                    } else {
                        values[j] - values[i]
                    },
                );
            }
        }
        abs_diff_sum.saturating_mul(500) / (n as u64 * sum).max(1)
    }
}
