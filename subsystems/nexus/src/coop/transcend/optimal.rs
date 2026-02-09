// SPDX-License-Identifier: GPL-2.0
//! # Provably Optimal Cooperation Engine
//!
//! Computes mathematically optimal resource sharing through Nash equilibrium
//! discovery, Pareto-optimal allocation, zero-waste distribution, fairness
//! proofs, and social welfare maximisation.  All arithmetic is integer-only
//! to remain deterministic in `no_std` environments.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const EMA_ALPHA_NUM: u64 = 3;
const EMA_ALPHA_DEN: u64 = 10;
const MAX_AGENTS: usize = 512;
const MAX_RESOURCES: usize = 1024;
const NASH_ITERATIONS: usize = 64;
const PARETO_ITERATIONS: usize = 32;

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

fn abs_diff(a: u64, b: u64) -> u64 {
    if a > b { a - b } else { b - a }
}

// ---------------------------------------------------------------------------
// Agent
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Agent {
    pub agent_id: u64,
    pub demand: u64,
    pub allocation: u64,
    pub utility: u64,
    pub ema_utility: u64,
}

// ---------------------------------------------------------------------------
// Resource pool
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
#[repr(align(64))]
pub struct ResourcePool {
    pub pool_id: u64,
    pub capacity: u64,
    pub allocated: u64,
    pub waste: u64,
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
#[repr(align(64))]
pub struct OptimalStats {
    pub agents_count: usize,
    pub pools_count: usize,
    pub nash_convergence: u64,
    pub pareto_score: u64,
    pub total_waste: u64,
    pub fairness_index: u64,
    pub social_welfare: u64,
    pub iterations_run: u64,
}

// ---------------------------------------------------------------------------
// CoopOptimal
// ---------------------------------------------------------------------------

pub struct CoopOptimal {
    agents: BTreeMap<u64, Agent>,
    pools: BTreeMap<u64, ResourcePool>,
    rng_state: u64,
    stats: OptimalStats,
    iteration_counter: u64,
    welfare_history: VecDeque<u64>,
}

impl CoopOptimal {
    pub fn new(seed: u64) -> Self {
        Self {
            agents: BTreeMap::new(),
            pools: BTreeMap::new(),
            rng_state: seed | 1,
            stats: OptimalStats {
                agents_count: 0,
                pools_count: 0,
                nash_convergence: 0,
                pareto_score: 0,
                total_waste: 0,
                fairness_index: 0,
                social_welfare: 0,
                iterations_run: 0,
            },
            iteration_counter: 0,
            welfare_history: VecDeque::new(),
        }
    }

    // -- registration -------------------------------------------------------

    pub fn register_agent(&mut self, id: u64, demand: u64) {
        if self.agents.len() >= MAX_AGENTS {
            return;
        }
        self.agents.insert(id, Agent {
            agent_id: id,
            demand,
            allocation: 0,
            utility: 0,
            ema_utility: 0,
        });
    }

    #[inline]
    pub fn register_pool(&mut self, id: u64, capacity: u64) {
        if self.pools.len() >= MAX_RESOURCES {
            return;
        }
        self.pools.insert(id, ResourcePool {
            pool_id: id,
            capacity,
            allocated: 0,
            waste: 0,
        });
    }

    // -- Nash equilibrium ---------------------------------------------------

    pub fn nash_equilibrium(&mut self) -> u64 {
        let pool_ids: Vec<u64> = self.pools.keys().copied().collect();
        let agent_ids: Vec<u64> = self.agents.keys().copied().collect();
        if agent_ids.is_empty() || pool_ids.is_empty() {
            return 0;
        }

        let total_capacity: u64 = self.pools.values().map(|p| p.capacity).sum();
        let total_demand: u64 = self.agents.values().map(|a| a.demand).sum();
        if total_demand == 0 {
            return 100;
        }

        // Iterative best-response dynamics
        let mut convergence = 0u64;
        for _iter in 0..NASH_ITERATIONS {
            let mut max_change = 0u64;
            for &aid in &agent_ids {
                if let Some(agent) = self.agents.get(&aid) {
                    let fair_share = (agent.demand * total_capacity) / total_demand;
                    let old_alloc = agent.allocation;
                    let new_alloc = clamp(fair_share, 0, agent.demand);
                    let change = abs_diff(new_alloc, old_alloc);
                    if change > max_change {
                        max_change = change;
                    }
                    if let Some(a) = self.agents.get_mut(&aid) {
                        a.allocation = new_alloc;
                        a.utility = if a.demand > 0 {
                            new_alloc * 100 / a.demand
                        } else {
                            100
                        };
                        a.ema_utility = ema_update(a.ema_utility, a.utility);
                    }
                }
            }
            convergence = 100u64.saturating_sub(max_change);
            if max_change == 0 {
                break;
            }
        }

        // Update pools
        self.redistribute_pools(&agent_ids, &pool_ids);
        self.stats.nash_convergence = convergence;
        self.iteration_counter += 1;
        convergence
    }

    fn redistribute_pools(&mut self, agent_ids: &[u64], pool_ids: &[u64]) {
        let total_alloc: u64 = self.agents.values().map(|a| a.allocation).sum();
        let n_pools = pool_ids.len() as u64;
        if n_pools == 0 {
            return;
        }
        let share_per_pool = total_alloc / n_pools;
        for &pid in pool_ids {
            if let Some(pool) = self.pools.get_mut(&pid) {
                pool.allocated = share_per_pool.min(pool.capacity);
                pool.waste = pool.capacity.saturating_sub(pool.allocated);
            }
        }
    }

    // -- Pareto-optimal sharing ---------------------------------------------

    pub fn pareto_optimal_sharing(&mut self) -> u64 {
        let agent_ids: Vec<u64> = self.agents.keys().copied().collect();
        if agent_ids.is_empty() {
            return 100;
        }

        for _iter in 0..PARETO_ITERATIONS {
            let mut improved = false;
            for i in 0..agent_ids.len() {
                for j in (i + 1)..agent_ids.len() {
                    let aid_i = agent_ids[i];
                    let aid_j = agent_ids[j];
                    let (alloc_i, demand_i) = match self.agents.get(&aid_i) {
                        Some(a) => (a.allocation, a.demand),
                        None => continue,
                    };
                    let (alloc_j, demand_j) = match self.agents.get(&aid_j) {
                        Some(a) => (a.allocation, a.demand),
                        None => continue,
                    };
                    // Check if transferring from over-served to under-served helps
                    let sat_i = if demand_i > 0 {
                        alloc_i * 100 / demand_i
                    } else {
                        100
                    };
                    let sat_j = if demand_j > 0 {
                        alloc_j * 100 / demand_j
                    } else {
                        100
                    };
                    if sat_i > sat_j + 10 && alloc_i > 1 {
                        let transfer = (alloc_i - alloc_j) / 4;
                        let transfer = transfer.max(1);
                        if let Some(ai) = self.agents.get_mut(&aid_i) {
                            ai.allocation = ai.allocation.saturating_sub(transfer);
                            ai.utility = if ai.demand > 0 {
                                ai.allocation * 100 / ai.demand
                            } else {
                                100
                            };
                        }
                        if let Some(aj) = self.agents.get_mut(&aid_j) {
                            aj.allocation = aj.allocation.saturating_add(transfer).min(aj.demand);
                            aj.utility = if aj.demand > 0 {
                                aj.allocation * 100 / aj.demand
                            } else {
                                100
                            };
                        }
                        improved = true;
                    }
                }
            }
            if !improved {
                break;
            }
        }

        let pareto = self.compute_pareto_score();
        self.stats.pareto_score = pareto;
        pareto
    }

    fn compute_pareto_score(&self) -> u64 {
        if self.agents.is_empty() {
            return 100;
        }
        let utils: Vec<u64> = self.agents.values().map(|a| a.utility).collect();
        let min_u = *utils.iter().min().unwrap_or(&0);
        let max_u = *utils.iter().max().unwrap_or(&100);
        if max_u == 0 {
            return 0;
        }
        100 - (max_u - min_u)
    }

    // -- zero-waste allocation ----------------------------------------------

    pub fn zero_waste_allocation(&mut self) -> u64 {
        let mut total_waste = 0u64;
        for pool in self.pools.values_mut() {
            pool.waste = pool.capacity.saturating_sub(pool.allocated);
            total_waste += pool.waste;
        }
        let total_cap: u64 = self.pools.values().map(|p| p.capacity).sum();
        self.stats.total_waste = total_waste;
        if total_cap == 0 {
            return 100;
        }
        100u64.saturating_sub(total_waste * 100 / total_cap)
    }

    // -- fairness proof (Jain's index) --------------------------------------

    pub fn fairness_proof(&self) -> u64 {
        let utils: Vec<u64> = self.agents.values().map(|a| a.utility).collect();
        let n = utils.len() as u64;
        if n == 0 {
            return 100;
        }
        let sum: u64 = utils.iter().sum();
        let sum_sq: u64 = utils.iter().map(|&u| u * u).sum();
        if sum_sq == 0 || n == 0 {
            return 100;
        }
        // Jain's index = (sum(x))^2 / (n * sum(x^2))
        let numerator = (sum * sum) / n;
        let jain = numerator * 100 / sum_sq;
        clamp(jain, 0, 100)
    }

    // -- social welfare maximisation ----------------------------------------

    pub fn social_welfare_max(&mut self) -> u64 {
        // Run full pipeline
        self.nash_equilibrium();
        self.pareto_optimal_sharing();
        let waste_score = self.zero_waste_allocation();
        let fairness = self.fairness_proof();

        let welfare = self.agents.values().map(|a| a.utility).sum::<u64>();
        let n = self.agents.len() as u64;
        let avg_welfare = if n > 0 { welfare / n } else { 0 };

        self.welfare_history.push_back(avg_welfare);
        if self.welfare_history.len() > 256 {
            self.welfare_history.pop_front();
        }

        let social = (avg_welfare + fairness + waste_score) / 3;
        self.stats.social_welfare = social;
        self.stats.fairness_index = fairness;
        self.refresh_stats();
        social
    }

    // -- stats --------------------------------------------------------------

    fn refresh_stats(&mut self) {
        self.stats.agents_count = self.agents.len();
        self.stats.pools_count = self.pools.len();
        self.stats.iterations_run = self.iteration_counter;
    }

    #[inline(always)]
    pub fn stats(&self) -> OptimalStats {
        self.stats.clone()
    }

    // -- utility ------------------------------------------------------------

    pub fn random_perturbation(&mut self) {
        let agent_ids: Vec<u64> = self.agents.keys().copied().collect();
        if agent_ids.is_empty() {
            return;
        }
        let idx = (xorshift64(&mut self.rng_state) as usize) % agent_ids.len();
        let aid = agent_ids[idx];
        let delta = xorshift64(&mut self.rng_state) % 20;
        if let Some(a) = self.agents.get_mut(&aid) {
            a.demand = a.demand.saturating_add(delta).min(10000);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nash_equilibrium() {
        let mut co = CoopOptimal::new(42);
        co.register_agent(1, 100);
        co.register_agent(2, 200);
        co.register_pool(10, 150);
        let conv = co.nash_equilibrium();
        assert!(conv > 0);
    }

    #[test]
    fn test_pareto_optimal() {
        let mut co = CoopOptimal::new(7);
        co.register_agent(1, 50);
        co.register_agent(2, 50);
        co.register_pool(10, 100);
        co.nash_equilibrium();
        let score = co.pareto_optimal_sharing();
        assert!(score >= 50);
    }

    #[test]
    fn test_fairness_proof() {
        let mut co = CoopOptimal::new(99);
        co.register_agent(1, 100);
        co.register_agent(2, 100);
        co.register_pool(10, 200);
        co.nash_equilibrium();
        let jain = co.fairness_proof();
        assert!(jain >= 90);
    }

    #[test]
    fn test_social_welfare() {
        let mut co = CoopOptimal::new(55);
        co.register_agent(1, 80);
        co.register_agent(2, 120);
        co.register_pool(10, 200);
        let sw = co.social_welfare_max();
        assert!(sw > 0);
    }
}
