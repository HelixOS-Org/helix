// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Simulator
//!
//! Simulates future cooperation scenarios by modeling trust network evolution,
//! resource sharing dynamics, and protocol efficiency under varying load
//! conditions. Drives what-if analysis for cooperative kernel intelligence.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// FNV-1a hash for deterministic key derivation.
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

/// Result of a cooperation simulation run.
#[derive(Clone, Debug)]
pub struct SimulationResult {
    pub scenario_id: u64,
    pub final_trust_avg: u64,
    pub resource_utilization: u64,
    pub contention_events: u64,
    pub protocol_throughput: u64,
    pub cooperation_score: u64,
    pub steps_executed: u64,
}

/// Trust evolution snapshot at a given step.
#[derive(Clone, Debug)]
pub struct TrustSnapshot {
    pub step: u64,
    pub partner_id: u64,
    pub trust_level: u64,
    pub delta: i64,
}

/// Protocol stress test result.
#[derive(Clone, Debug)]
pub struct StressResult {
    pub protocol_hash: u64,
    pub max_throughput: u64,
    pub breakdown_point: u64,
    pub recovery_time: u64,
    pub efficiency_at_peak: u64,
}

/// Rolling statistics for the simulator.
#[derive(Clone, Debug)]
pub struct SimulatorStats {
    pub total_simulations: u64,
    pub total_steps: u64,
    pub avg_cooperation_score: u64,
    pub avg_trust_final: u64,
    pub stress_tests_run: u64,
    pub avg_divergence: u64,
    pub simulation_errors: u64,
}

impl SimulatorStats {
    pub fn new() -> Self {
        Self {
            total_simulations: 0,
            total_steps: 0,
            avg_cooperation_score: 500,
            avg_trust_final: 500,
            stress_tests_run: 0,
            avg_divergence: 0,
            simulation_errors: 0,
        }
    }
}

/// Internal node in the simulated trust network.
#[derive(Clone, Debug)]
struct SimNode {
    node_id: u64,
    trust_levels: BTreeMap<u64, u64>,
    resource_capacity: u64,
    resource_used: u64,
    cooperation_bias: u64,
}

/// Internal scenario configuration.
#[derive(Clone, Debug)]
struct ScenarioConfig {
    node_count: usize,
    duration_steps: u64,
    contention_rate: u64,
    trust_volatility: u64,
    protocol_overhead: u64,
}

/// Cooperation simulation engine.
pub struct CoopSimulator {
    nodes: BTreeMap<u64, SimNode>,
    trust_snapshots: Vec<TrustSnapshot>,
    divergence_history: Vec<u64>,
    stats: SimulatorStats,
    rng_state: u64,
    max_nodes: usize,
    max_snapshots: usize,
}

impl CoopSimulator {
    /// Create a new cooperation simulator with the given PRNG seed.
    pub fn new(seed: u64) -> Self {
        Self {
            nodes: BTreeMap::new(),
            trust_snapshots: Vec::new(),
            divergence_history: Vec::new(),
            stats: SimulatorStats::new(),
            rng_state: seed | 1,
            max_nodes: 128,
            max_snapshots: 256,
        }
    }

    /// Register a node in the simulation network.
    pub fn add_node(&mut self, node_id: u64, capacity: u64, cooperation_bias: u64) {
        if self.nodes.len() >= self.max_nodes {
            return;
        }
        self.nodes.insert(node_id, SimNode {
            node_id,
            trust_levels: BTreeMap::new(),
            resource_capacity: capacity,
            resource_used: 0,
            cooperation_bias: cooperation_bias.min(1000),
        });
    }

    /// Establish an initial trust link between two nodes.
    pub fn set_trust(&mut self, from: u64, to: u64, trust: u64) {
        let clamped = trust.min(1000);
        if let Some(node) = self.nodes.get_mut(&from) {
            node.trust_levels.insert(to, clamped);
        }
    }

    /// Simulate a full cooperation scenario and return the result.
    pub fn simulate_cooperation(&mut self, duration_steps: u64, contention_rate: u64) -> SimulationResult {
        self.stats.total_simulations = self.stats.total_simulations.saturating_add(1);
        let scenario_id = fnv1a_hash(&self.stats.total_simulations.to_le_bytes());

        let mut total_contention: u64 = 0;
        let mut step_trust_sum: u64 = 0;
        let mut step_count: u64 = 0;
        let mut throughput_acc: u64 = 0;

        let node_ids: Vec<u64> = self.nodes.keys().copied().collect();

        for step in 0..duration_steps {
            for &nid in &node_ids {
                let noise = xorshift64(&mut self.rng_state) % 100;
                let contention_event = noise < contention_rate;

                if contention_event {
                    total_contention = total_contention.saturating_add(1);
                    if let Some(node) = self.nodes.get_mut(&nid) {
                        let load_increase = xorshift64(&mut self.rng_state) % (node.resource_capacity / 4).max(1);
                        node.resource_used = node.resource_used.saturating_add(load_increase)
                            .min(node.resource_capacity);
                    }
                } else {
                    if let Some(node) = self.nodes.get_mut(&nid) {
                        let release = node.resource_used / 10;
                        node.resource_used = node.resource_used.saturating_sub(release);
                    }
                }

                let partner_idx = (xorshift64(&mut self.rng_state) as usize) % node_ids.len().max(1);
                let partner_id = node_ids[partner_idx];
                if partner_id != nid {
                    self.evolve_trust_pair(nid, partner_id, step);
                }
            }

            for &nid in &node_ids {
                if let Some(node) = self.nodes.get(&nid) {
                    let trust_sum: u64 = node.trust_levels.values().sum();
                    let trust_count = node.trust_levels.len() as u64;
                    if trust_count > 0 {
                        step_trust_sum = step_trust_sum.saturating_add(trust_sum / trust_count);
                        step_count = step_count.saturating_add(1);
                    }
                    let util = if node.resource_capacity > 0 {
                        node.resource_used.saturating_mul(1000) / node.resource_capacity
                    } else {
                        0
                    };
                    throughput_acc = throughput_acc.saturating_add(1000u64.saturating_sub(util));
                }
            }
        }

        self.stats.total_steps = self.stats.total_steps.saturating_add(duration_steps);
        let final_trust_avg = if step_count > 0 { step_trust_sum / step_count } else { 500 };
        let resource_util = self.compute_avg_utilization();
        let protocol_tp = if duration_steps > 0 {
            throughput_acc / (duration_steps.saturating_mul(node_ids.len() as u64).max(1))
        } else {
            0
        };
        let coop_score = final_trust_avg.saturating_mul(7) / 10
            + protocol_tp.saturating_mul(2) / 10
            + (1000u64.saturating_sub(total_contention.min(1000))) / 10;

        self.stats.avg_cooperation_score = ema_update(self.stats.avg_cooperation_score, coop_score, 3, 10);
        self.stats.avg_trust_final = ema_update(self.stats.avg_trust_final, final_trust_avg, 3, 10);

        SimulationResult {
            scenario_id,
            final_trust_avg,
            resource_utilization: resource_util,
            contention_events: total_contention,
            protocol_throughput: protocol_tp,
            cooperation_score: coop_score,
            steps_executed: duration_steps,
        }
    }

    /// Simulate trust evolution between all connected pairs for a number of steps.
    pub fn trust_evolution(&mut self, steps: u64) -> Vec<TrustSnapshot> {
        let node_ids: Vec<u64> = self.nodes.keys().copied().collect();
        let mut snapshots = Vec::new();

        for step in 0..steps {
            for i in 0..node_ids.len() {
                for j in (i + 1)..node_ids.len() {
                    let a = node_ids[i];
                    let b = node_ids[j];
                    let delta = self.evolve_trust_pair(a, b, step);
                    if step % 10 == 0 {
                        if let Some(node) = self.nodes.get(&a) {
                            if let Some(&trust) = node.trust_levels.get(&b) {
                                let snap = TrustSnapshot { step, partner_id: b, trust_level: trust, delta };
                                snapshots.push(snap);
                                if snapshots.len() >= self.max_snapshots {
                                    return snapshots;
                                }
                            }
                        }
                    }
                }
            }
        }
        snapshots
    }

    /// Run a contention scenario with a specified burst pattern.
    pub fn contention_scenario(&mut self, burst_size: u64, burst_interval: u64, total_steps: u64) -> u64 {
        let node_ids: Vec<u64> = self.nodes.keys().copied().collect();
        let mut total_contention_cost: u64 = 0;

        for step in 0..total_steps {
            let is_burst = burst_interval > 0 && step % burst_interval == 0;
            for &nid in &node_ids {
                if let Some(node) = self.nodes.get_mut(&nid) {
                    if is_burst {
                        let load = burst_size.min(node.resource_capacity);
                        node.resource_used = node.resource_used.saturating_add(load)
                            .min(node.resource_capacity);
                        total_contention_cost = total_contention_cost.saturating_add(load);
                    } else {
                        let decay = node.resource_used / 20;
                        node.resource_used = node.resource_used.saturating_sub(decay);
                    }
                }
            }
        }
        total_contention_cost
    }

    /// Stress test a protocol by ramping load and measuring throughput collapse.
    pub fn protocol_stress(&mut self, protocol_name: &str, max_load: u64, ramp_steps: u64) -> StressResult {
        self.stats.stress_tests_run = self.stats.stress_tests_run.saturating_add(1);
        let protocol_hash = fnv1a_hash(protocol_name.as_bytes());

        let mut max_throughput: u64 = 0;
        let mut breakdown_point: u64 = 0;
        let mut found_breakdown = false;
        let mut peak_efficiency: u64 = 0;
        let mut recovery_start: u64 = 0;

        for step in 0..ramp_steps {
            let load = if ramp_steps > 0 {
                max_load.saturating_mul(step) / ramp_steps
            } else {
                max_load
            };

            let noise = xorshift64(&mut self.rng_state) % (load / 10).max(1);
            let effective_load = load.saturating_add(noise);
            let overhead = effective_load.saturating_mul(effective_load) / max_load.max(1) / 10;
            let throughput = effective_load.saturating_sub(overhead);

            if throughput > max_throughput {
                max_throughput = throughput;
                peak_efficiency = if effective_load > 0 {
                    throughput.saturating_mul(1000) / effective_load
                } else {
                    1000
                };
            }

            if !found_breakdown && throughput < max_throughput.saturating_mul(8) / 10 && step > 2 {
                breakdown_point = load;
                found_breakdown = true;
                recovery_start = step;
            }
        }

        let recovery_time = if found_breakdown {
            ramp_steps.saturating_sub(recovery_start)
        } else {
            0
        };

        StressResult {
            protocol_hash,
            max_throughput,
            breakdown_point,
            recovery_time,
            efficiency_at_peak: peak_efficiency,
        }
    }

    /// Compute the divergence between the last two simulation results.
    pub fn simulation_divergence(&mut self, result_a: &SimulationResult, result_b: &SimulationResult) -> u64 {
        let trust_diff = if result_a.final_trust_avg > result_b.final_trust_avg {
            result_a.final_trust_avg - result_b.final_trust_avg
        } else {
            result_b.final_trust_avg - result_a.final_trust_avg
        };
        let util_diff = if result_a.resource_utilization > result_b.resource_utilization {
            result_a.resource_utilization - result_b.resource_utilization
        } else {
            result_b.resource_utilization - result_a.resource_utilization
        };
        let score_diff = if result_a.cooperation_score > result_b.cooperation_score {
            result_a.cooperation_score - result_b.cooperation_score
        } else {
            result_b.cooperation_score - result_a.cooperation_score
        };

        let divergence = trust_diff.saturating_mul(4) / 10
            + util_diff.saturating_mul(3) / 10
            + score_diff.saturating_mul(3) / 10;

        if self.divergence_history.len() >= 64 {
            self.divergence_history.remove(0);
        }
        self.divergence_history.push(divergence);
        self.stats.avg_divergence = ema_update(self.stats.avg_divergence, divergence, 3, 10);
        divergence
    }

    /// Get a reference to the current statistics.
    pub fn stats(&self) -> &SimulatorStats {
        &self.stats
    }

    /// Evolve trust between two nodes for a single step, returning the delta.
    fn evolve_trust_pair(&mut self, a: u64, b: u64, _step: u64) -> i64 {
        let bias_a = self.nodes.get(&a).map(|n| n.cooperation_bias).unwrap_or(500);
        let bias_b = self.nodes.get(&b).map(|n| n.cooperation_bias).unwrap_or(500);

        let combined_bias = (bias_a.saturating_add(bias_b)) / 2;
        let noise = (xorshift64(&mut self.rng_state) % 200) as i64 - 100;
        let delta = (combined_bias as i64 - 500) / 10 + noise / 20;

        if let Some(node) = self.nodes.get_mut(&a) {
            let current = node.trust_levels.get(&b).copied().unwrap_or(500);
            let new_val = if delta >= 0 {
                current.saturating_add(delta as u64)
            } else {
                current.saturating_sub((-delta) as u64)
            };
            node.trust_levels.insert(b, new_val.min(1000));
        }
        if let Some(node) = self.nodes.get_mut(&b) {
            let current = node.trust_levels.get(&a).copied().unwrap_or(500);
            let half_delta = delta / 2;
            let new_val = if half_delta >= 0 {
                current.saturating_add(half_delta as u64)
            } else {
                current.saturating_sub((-half_delta) as u64)
            };
            node.trust_levels.insert(a, new_val.min(1000));
        }
        delta
    }

    /// Compute average utilization across all nodes.
    fn compute_avg_utilization(&self) -> u64 {
        if self.nodes.is_empty() {
            return 0;
        }
        let mut total: u64 = 0;
        for node in self.nodes.values() {
            if node.resource_capacity > 0 {
                total = total.saturating_add(
                    node.resource_used.saturating_mul(1000) / node.resource_capacity,
                );
            }
        }
        total / self.nodes.len() as u64
    }
}
