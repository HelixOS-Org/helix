// SPDX-License-Identifier: GPL-2.0
//! # Monte Carlo Cooperation Futures
//!
//! Simulates thousands of cooperation scenarios with random perturbations to
//! estimate failure probabilities, optimal strategies, and risk distributions
//! for cooperative kernel intelligence decisions.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// FNV-1a hash for deterministic identifier generation.
fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Xorshift64 PRNG for Monte Carlo randomness.
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

/// A single sampled scenario outcome.
#[derive(Clone, Debug)]
pub struct ScenarioSample {
    pub sample_id: u64,
    pub cooperation_success: bool,
    pub trust_outcome: u64,
    pub resource_utilization: u64,
    pub contention_level: u64,
    pub negotiation_rounds: u64,
    pub total_cost: u64,
}

/// Aggregate result from running many cooperation simulations.
#[derive(Clone, Debug)]
pub struct CooperationRunResult {
    pub run_id: u64,
    pub samples_count: u64,
    pub success_rate: u64,
    pub avg_trust: u64,
    pub avg_utilization: u64,
    pub avg_contention: u64,
    pub percentile_95_cost: u64,
    pub worst_case_trust: u64,
}

/// Failure probability estimate for a strategy.
#[derive(Clone, Debug)]
pub struct FailureProbability {
    pub strategy_hash: u64,
    pub failure_rate: u64,
    pub confidence_interval_low: u64,
    pub confidence_interval_high: u64,
    pub dominant_failure_mode: u64,
    pub samples_used: u64,
}

/// Optimal strategy recommendation.
#[derive(Clone, Debug)]
pub struct OptimalStrategy {
    pub strategy_hash: u64,
    pub expected_reward: u64,
    pub risk_adjusted_reward: u64,
    pub variance: u64,
    pub sharpe_ratio: u64,
    pub dominates_count: u64,
}

/// Risk distribution across cooperation outcomes.
#[derive(Clone, Debug)]
pub struct RiskDistribution {
    pub buckets: Vec<RiskBucket>,
    pub mean_risk: u64,
    pub median_risk: u64,
    pub tail_risk_5pct: u64,
    pub value_at_risk: u64,
}

/// A single bucket in the risk distribution.
#[derive(Clone, Debug)]
pub struct RiskBucket {
    pub lower_bound: u64,
    pub upper_bound: u64,
    pub count: u64,
    pub frequency: u64,
}

/// Rolling statistics for Monte Carlo analysis.
#[derive(Clone, Debug)]
pub struct MonteCarloStats {
    pub total_samples: u64,
    pub total_runs: u64,
    pub strategies_evaluated: u64,
    pub avg_success_rate: u64,
    pub avg_failure_rate: u64,
    pub risk_assessments: u64,
    pub computation_budget_used: u64,
}

impl MonteCarloStats {
    pub fn new() -> Self {
        Self {
            total_samples: 0,
            total_runs: 0,
            strategies_evaluated: 0,
            avg_success_rate: 500,
            avg_failure_rate: 500,
            risk_assessments: 0,
            computation_budget_used: 0,
        }
    }
}

/// Internal strategy definition for evaluation.
#[derive(Clone, Debug)]
struct StrategyDef {
    strategy_hash: u64,
    trust_threshold: u64,
    resource_commitment: u64,
    flexibility: u64,
    risk_tolerance: u64,
}

/// Internal scenario parameter set.
#[derive(Clone, Debug)]
struct ScenarioParams {
    base_trust: u64,
    base_contention: u64,
    resource_pool: u64,
    agent_count: u64,
    volatility: u64,
}

/// Monte Carlo cooperation futures engine.
pub struct CoopMonteCarlo {
    strategies: BTreeMap<u64, StrategyDef>,
    sample_cache: Vec<ScenarioSample>,
    reward_history: BTreeMap<u64, Vec<u64>>,
    stats: MonteCarloStats,
    rng_state: u64,
    max_cache: usize,
    max_reward_history: usize,
}

impl CoopMonteCarlo {
    /// Create a new Monte Carlo engine with the given seed.
    pub fn new(seed: u64) -> Self {
        Self {
            strategies: BTreeMap::new(),
            sample_cache: Vec::new(),
            reward_history: BTreeMap::new(),
            stats: MonteCarloStats::new(),
            rng_state: seed | 1,
            max_cache: 1024,
            max_reward_history: 128,
        }
    }

    /// Register a cooperation strategy for evaluation.
    pub fn register_strategy(
        &mut self,
        name: &str,
        trust_threshold: u64,
        resource_commitment: u64,
        flexibility: u64,
        risk_tolerance: u64,
    ) -> u64 {
        let hash = fnv1a_hash(name.as_bytes());
        self.strategies.insert(hash, StrategyDef {
            strategy_hash: hash,
            trust_threshold: trust_threshold.min(1000),
            resource_commitment: resource_commitment.min(1000),
            flexibility: flexibility.min(1000),
            risk_tolerance: risk_tolerance.min(1000),
        });
        hash
    }

    /// Sample a single cooperation scenario under a strategy.
    pub fn sample_scenario(&mut self, strategy_hash: u64, params: &[u64; 5]) -> ScenarioSample {
        self.stats.total_samples = self.stats.total_samples.saturating_add(1);
        self.stats.computation_budget_used = self.stats.computation_budget_used.saturating_add(1);

        let scenario = ScenarioParams {
            base_trust: params[0],
            base_contention: params[1],
            resource_pool: params[2],
            agent_count: params[3].max(1),
            volatility: params[4],
        };

        let strategy = self.strategies.get(&strategy_hash).cloned().unwrap_or(StrategyDef {
            strategy_hash,
            trust_threshold: 500,
            resource_commitment: 500,
            flexibility: 500,
            risk_tolerance: 500,
        });

        let trust_noise = (xorshift64(&mut self.rng_state) % (scenario.volatility.max(1) * 2)) as i64
            - scenario.volatility as i64;
        let trust_outcome = if trust_noise >= 0 {
            scenario.base_trust.saturating_add(trust_noise as u64)
        } else {
            scenario.base_trust.saturating_sub((-trust_noise) as u64)
        }.min(1000);

        let contention_noise = xorshift64(&mut self.rng_state) % scenario.volatility.max(1);
        let contention = scenario.base_contention.saturating_add(contention_noise)
            .saturating_sub(strategy.flexibility / 5).min(1000);

        let resource_demand = scenario.agent_count.saturating_mul(strategy.resource_commitment) / 10;
        let utilization = resource_demand.saturating_mul(1000) / scenario.resource_pool.max(1);
        let utilization = utilization.min(1000);

        let cooperation_success = trust_outcome >= strategy.trust_threshold
            && contention < strategy.risk_tolerance.saturating_mul(8) / 10;

        let rounds = if cooperation_success {
            3 + xorshift64(&mut self.rng_state) % 10
        } else {
            10 + xorshift64(&mut self.rng_state) % 20
        };

        let cost = rounds.saturating_mul(scenario.agent_count)
            .saturating_add(contention / 10)
            .saturating_add(if cooperation_success { 0 } else { 200 });

        let sample = ScenarioSample {
            sample_id: self.stats.total_samples,
            cooperation_success,
            trust_outcome,
            resource_utilization: utilization,
            contention_level: contention,
            negotiation_rounds: rounds,
            total_cost: cost,
        };

        if self.sample_cache.len() >= self.max_cache {
            self.sample_cache.remove(0);
        }
        self.sample_cache.push(sample.clone());
        sample
    }

    /// Run many cooperation simulations and aggregate results.
    pub fn run_cooperations(
        &mut self,
        strategy_hash: u64,
        params: &[u64; 5],
        num_samples: u64,
    ) -> CooperationRunResult {
        self.stats.total_runs = self.stats.total_runs.saturating_add(1);
        let run_id = fnv1a_hash(&self.stats.total_runs.to_le_bytes());

        let capped_samples = num_samples.min(2000);
        let mut successes: u64 = 0;
        let mut trust_sum: u64 = 0;
        let mut util_sum: u64 = 0;
        let mut contention_sum: u64 = 0;
        let mut costs: Vec<u64> = Vec::new();
        let mut worst_trust: u64 = 1000;

        for _ in 0..capped_samples {
            let sample = self.sample_scenario(strategy_hash, params);
            if sample.cooperation_success {
                successes = successes.saturating_add(1);
            }
            trust_sum = trust_sum.saturating_add(sample.trust_outcome);
            util_sum = util_sum.saturating_add(sample.resource_utilization);
            contention_sum = contention_sum.saturating_add(sample.contention_level);
            costs.push(sample.total_cost);
            if sample.trust_outcome < worst_trust {
                worst_trust = sample.trust_outcome;
            }
        }

        costs.sort();
        let p95_idx = (capped_samples as usize).saturating_mul(95) / 100;
        let p95_cost = costs.get(p95_idx.min(costs.len().saturating_sub(1)))
            .copied().unwrap_or(0);

        let success_rate = if capped_samples > 0 {
            successes.saturating_mul(1000) / capped_samples
        } else {
            0
        };

        self.stats.avg_success_rate = ema_update(self.stats.avg_success_rate, success_rate, 3, 10);

        let rewards = self.reward_history.entry(strategy_hash).or_insert_with(Vec::new);
        if rewards.len() >= self.max_reward_history {
            rewards.remove(0);
        }
        rewards.push(success_rate);

        CooperationRunResult {
            run_id,
            samples_count: capped_samples,
            success_rate,
            avg_trust: if capped_samples > 0 { trust_sum / capped_samples } else { 0 },
            avg_utilization: if capped_samples > 0 { util_sum / capped_samples } else { 0 },
            avg_contention: if capped_samples > 0 { contention_sum / capped_samples } else { 0 },
            percentile_95_cost: p95_cost,
            worst_case_trust: worst_trust,
        }
    }

    /// Estimate failure probability for a strategy under given conditions.
    pub fn failure_probability(&mut self, strategy_hash: u64, params: &[u64; 5], samples: u64) -> FailureProbability {
        let result = self.run_cooperations(strategy_hash, params, samples);
        let failure_rate = 1000u64.saturating_sub(result.success_rate);

        let se = if result.samples_count > 1 {
            let p = failure_rate;
            let q = 1000u64.saturating_sub(p);
            (p.saturating_mul(q) / result.samples_count).min(500)
        } else {
            500
        };
        let ci_low = failure_rate.saturating_sub(se.saturating_mul(2));
        let ci_high = failure_rate.saturating_add(se.saturating_mul(2)).min(1000);

        let dominant_mode = if result.worst_case_trust < 200 {
            1
        } else if result.avg_contention > 700 {
            2
        } else {
            0
        };

        self.stats.avg_failure_rate = ema_update(self.stats.avg_failure_rate, failure_rate, 3, 10);

        FailureProbability {
            strategy_hash,
            failure_rate,
            confidence_interval_low: ci_low,
            confidence_interval_high: ci_high,
            dominant_failure_mode: dominant_mode,
            samples_used: result.samples_count,
        }
    }

    /// Find the optimal strategy among all registered strategies.
    pub fn optimal_strategy(&mut self, params: &[u64; 5], samples_per_strategy: u64) -> OptimalStrategy {
        self.stats.strategies_evaluated = self.stats.strategies_evaluated.saturating_add(
            self.strategies.len() as u64,
        );

        let strategy_hashes: Vec<u64> = self.strategies.keys().copied().collect();
        let mut best_hash: u64 = 0;
        let mut best_risk_adj: u64 = 0;
        let mut best_reward: u64 = 0;
        let mut best_variance: u64 = 0;
        let mut dominates: u64 = 0;

        let mut all_results: Vec<(u64, CooperationRunResult)> = Vec::new();

        for &sh in &strategy_hashes {
            let result = self.run_cooperations(sh, params, samples_per_strategy);
            all_results.push((sh, result));
        }

        for &(sh, ref result) in &all_results {
            let reward = result.success_rate.saturating_mul(7) / 10
                + (1000u64.saturating_sub(result.avg_contention)).saturating_mul(2) / 10
                + result.avg_trust / 10;

            let variance = self.compute_reward_variance(sh);
            let risk_adj = if variance > 0 {
                reward.saturating_mul(1000) / (variance.max(1) + 100)
            } else {
                reward.saturating_mul(10)
            };

            if risk_adj > best_risk_adj {
                best_risk_adj = risk_adj;
                best_reward = reward;
                best_variance = variance;
                best_hash = sh;
                dominates = 0;
                for &(other_sh, ref other) in &all_results {
                    if other_sh != sh && result.success_rate >= other.success_rate
                        && result.avg_trust >= other.avg_trust
                    {
                        dominates = dominates.saturating_add(1);
                    }
                }
            }
        }

        let sharpe = if best_variance > 0 {
            best_reward.saturating_mul(1000) / best_variance.max(1)
        } else {
            best_reward.saturating_mul(10)
        };

        OptimalStrategy {
            strategy_hash: best_hash,
            expected_reward: best_reward,
            risk_adjusted_reward: best_risk_adj,
            variance: best_variance,
            sharpe_ratio: sharpe,
            dominates_count: dominates,
        }
    }

    /// Compute the risk distribution over recent samples.
    pub fn risk_distribution(&self, bucket_count: usize) -> RiskDistribution {
        let mut risk_values: Vec<u64> = self.sample_cache.iter()
            .map(|s| {
                let trust_risk = 1000u64.saturating_sub(s.trust_outcome);
                let contention_risk = s.contention_level;
                let cost_risk = s.total_cost.min(1000);
                trust_risk.saturating_mul(4) / 10
                    + contention_risk.saturating_mul(3) / 10
                    + cost_risk.saturating_mul(3) / 10
            })
            .collect();

        risk_values.sort();
        let n = risk_values.len();

        let mean_risk = if n > 0 {
            risk_values.iter().sum::<u64>() / n as u64
        } else {
            0
        };
        let median_risk = risk_values.get(n / 2).copied().unwrap_or(0);
        let tail_idx = n.saturating_mul(95) / 100;
        let tail_risk = risk_values.get(tail_idx.min(n.saturating_sub(1)))
            .copied().unwrap_or(0);
        let var_idx = n.saturating_mul(99) / 100;
        let var = risk_values.get(var_idx.min(n.saturating_sub(1)))
            .copied().unwrap_or(0);

        let actual_buckets = bucket_count.max(1).min(20);
        let bucket_size = 1000u64 / actual_buckets as u64;
        let mut buckets = Vec::with_capacity(actual_buckets);

        for i in 0..actual_buckets {
            let lower = (i as u64).saturating_mul(bucket_size);
            let upper = lower.saturating_add(bucket_size);
            let count = risk_values.iter()
                .filter(|&&v| v >= lower && v < upper)
                .count() as u64;
            let freq = if n > 0 { count.saturating_mul(1000) / n as u64 } else { 0 };
            buckets.push(RiskBucket {
                lower_bound: lower,
                upper_bound: upper,
                count,
                frequency: freq,
            });
        }

        RiskDistribution {
            buckets,
            mean_risk,
            median_risk,
            tail_risk_5pct: tail_risk,
            value_at_risk: var,
        }
    }

    /// Get the current statistics snapshot.
    pub fn stats(&self) -> &MonteCarloStats {
        &self.stats
    }

    /// Compute reward variance for a strategy from history.
    fn compute_reward_variance(&self, strategy_hash: u64) -> u64 {
        let rewards = match self.reward_history.get(&strategy_hash) {
            Some(r) if !r.is_empty() => r,
            _ => return 0,
        };
        let n = rewards.len() as u64;
        let mean = rewards.iter().sum::<u64>() / n.max(1);
        let variance: u64 = rewards.iter().map(|&r| {
            let diff = if r > mean { r - mean } else { mean - r };
            diff.saturating_mul(diff)
        }).sum::<u64>() / n.max(1);
        variance
    }
}
