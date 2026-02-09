// SPDX-License-Identifier: GPL-2.0
//! # Bridge Simulator
//!
//! Simulates future syscall scenarios by maintaining lightweight models of
//! process behavior, resource availability, and contention. The simulator
//! can fork state to explore multiple parallel branches, applying hypothetical
//! actions and identifying divergence points where outcomes dramatically differ.
//!
//! Think of it as a chess engine for the kernel: exploring move trees before
//! committing to a strategy.

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use crate::fast::array_map::ArrayMap;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_PROCESSES: usize = 128;
const MAX_BRANCHES: usize = 64;
const MAX_ACTIONS_PER_BRANCH: usize = 256;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const DIVERGENCE_THRESHOLD: f32 = 0.25;

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
// BEHAVIOR & RESOURCE MODELS
// ============================================================================

/// A lightweight model of a single process's syscall behavior
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ProcessBehaviorModel {
    pub process_id: u64,
    pub syscall_rates: ArrayMap<f32, 32>,
    pub avg_latency: f32,
    pub resource_appetite: f32,
    pub contention_sensitivity: f32,
    pub observation_count: u64,
}

impl ProcessBehaviorModel {
    fn new(process_id: u64) -> Self {
        Self {
            process_id,
            syscall_rates: ArrayMap::new(0.0),
            avg_latency: 1.0,
            resource_appetite: 0.5,
            contention_sensitivity: 0.5,
            observation_count: 0,
        }
    }

    #[inline]
    fn observe_syscall(&mut self, syscall_nr: u32, latency: f32) {
        self.observation_count += 1;
        let rate = self.syscall_rates.entry(syscall_nr).or_insert(0.0);
        *rate = EMA_ALPHA + (1.0 - EMA_ALPHA) * *rate;
        self.avg_latency = EMA_ALPHA * latency + (1.0 - EMA_ALPHA) * self.avg_latency;

        // Decay other rates
        let keys: Vec<u32> = self.syscall_rates.keys().copied().collect();
        for k in keys {
            if k != syscall_nr {
                if let Some(r) = self.syscall_rates.get_mut(&k) {
                    *r *= 1.0 - EMA_ALPHA;
                }
            }
        }
    }
}

/// Resource availability projection
#[derive(Debug, Clone)]
pub struct ResourceProjection {
    pub memory_available: f32,
    pub cpu_available: f32,
    pub io_bandwidth: f32,
    pub fd_available: u32,
    pub tick: u64,
}

/// Contention model between processes
#[derive(Debug, Clone)]
struct ContentionModel {
    process_pairs: LinearMap<f32, 64>,
    global_contention: f32,
    total_events: u64,
}

impl ContentionModel {
    fn new() -> Self {
        Self {
            process_pairs: LinearMap::new(),
            global_contention: 0.0,
            total_events: 0,
        }
    }

    #[inline]
    fn record_contention(&mut self, pid_a: u64, pid_b: u64, severity: f32) {
        self.total_events += 1;
        let pair_key = if pid_a < pid_b {
            fnv1a_hash(&[pid_a.to_le_bytes(), pid_b.to_le_bytes()].concat())
        } else {
            fnv1a_hash(&[pid_b.to_le_bytes(), pid_a.to_le_bytes()].concat())
        };
        let level = self.process_pairs.entry(pair_key).or_insert(0.0);
        *level = EMA_ALPHA * severity + (1.0 - EMA_ALPHA) * *level;
        self.global_contention =
            EMA_ALPHA * severity + (1.0 - EMA_ALPHA) * self.global_contention;
    }
}

// ============================================================================
// SIMULATION TYPES
// ============================================================================

/// An action that can be applied to a simulation state
#[derive(Debug, Clone)]
pub struct SimAction {
    pub action_id: u64,
    pub description: String,
    pub resource_delta_memory: f32,
    pub resource_delta_cpu: f32,
    pub syscall_nr: u32,
    pub target_process: u64,
}

/// A single simulation branch
#[derive(Debug, Clone)]
pub struct SimBranch {
    pub branch_id: u64,
    pub parent_id: u64,
    pub actions: Vec<SimAction>,
    pub resource_state: ResourceProjection,
    pub score: f32,
    pub divergence_tick: u64,
    pub likelihood: f32,
}

/// Result of a scenario simulation
#[derive(Debug, Clone)]
pub struct ScenarioResult {
    pub scenario_id: u64,
    pub branches_explored: u32,
    pub best_score: f32,
    pub worst_score: f32,
    pub avg_score: f32,
    pub divergence_points: Vec<u64>,
    pub recommended_branch: u64,
}

// ============================================================================
// SIMULATOR STATS
// ============================================================================

/// Aggregate simulation statistics
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct SimulatorStats {
    pub total_simulations: u64,
    pub total_branches: u64,
    pub avg_branches_per_sim: f32,
    pub avg_score: f32,
    pub best_score_ever: f32,
    pub divergence_rate: f32,
    pub avg_actions_per_branch: f32,
}

// ============================================================================
// BRIDGE SIMULATOR
// ============================================================================

/// Simulates future syscall scenarios, maintaining process behavior models,
/// resource projections, and contention data to explore outcome branches.
#[derive(Debug)]
#[repr(align(64))]
pub struct BridgeSimulator {
    process_models: BTreeMap<u64, ProcessBehaviorModel>,
    contention: ContentionModel,
    branches: Vec<SimBranch>,
    current_resources: ResourceProjection,
    tick: u64,
    total_simulations: u64,
    total_branches: u64,
    best_score_ever: f32,
    avg_score_ema: f32,
    rng_state: u64,
}

impl BridgeSimulator {
    pub fn new() -> Self {
        Self {
            process_models: BTreeMap::new(),
            contention: ContentionModel::new(),
            branches: Vec::new(),
            current_resources: ResourceProjection {
                memory_available: 1.0,
                cpu_available: 1.0,
                io_bandwidth: 1.0,
                fd_available: 1024,
                tick: 0,
            },
            tick: 0,
            total_simulations: 0,
            total_branches: 0,
            best_score_ever: 0.0,
            avg_score_ema: 0.5,
            rng_state: 0xBEEF_CAFE_DEAD_F00D,
        }
    }

    /// Update a process behavior model with a new observation
    pub fn observe_process(&mut self, process_id: u64, syscall_nr: u32, latency: f32) {
        self.tick += 1;
        let model = self.process_models.entry(process_id)
            .or_insert_with(|| ProcessBehaviorModel::new(process_id));
        model.observe_syscall(syscall_nr, latency);

        if self.process_models.len() > MAX_PROCESSES {
            let oldest = self.process_models.iter()
                .min_by_key(|(_, m)| m.observation_count)
                .map(|(&k, _)| k);
            if let Some(k) = oldest {
                self.process_models.remove(&k);
            }
        }
    }

    /// Record contention between two processes
    #[inline(always)]
    pub fn record_contention(&mut self, pid_a: u64, pid_b: u64, severity: f32) {
        self.contention.record_contention(pid_a, pid_b, severity.max(0.0).min(1.0));
    }

    /// Update current resource availability
    #[inline]
    pub fn update_resources(&mut self, memory: f32, cpu: f32, io: f32, fds: u32) {
        self.current_resources = ResourceProjection {
            memory_available: memory.max(0.0).min(1.0),
            cpu_available: cpu.max(0.0).min(1.0),
            io_bandwidth: io.max(0.0).min(1.0),
            fd_available: fds,
            tick: self.tick,
        };
    }

    /// Simulate a complete scenario with branching
    #[inline]
    pub fn simulate_scenario(&mut self, actions: Vec<SimAction>) -> ScenarioResult {
        self.total_simulations += 1;
        let scenario_id = fnv1a_hash(&self.total_simulations.to_le_bytes());

        let root_branch = SimBranch {
            branch_id: scenario_id,
            parent_id: 0,
            actions: Vec::new(),
            resource_state: self.current_resources.clone(),
            score: 0.5,
            divergence_tick: self.tick,
            likelihood: 1.0,
        };

        let mut branches = Vec::new();
        branches.push(root_branch);
        let mut divergence_points = Vec::new();

        for action in actions.iter() {
            let n = branches.len();
            let mut new_branches = Vec::new();

            for i in 0..n {
                let branch = &branches[i];
                let effect = self.evaluate_action(action, &branch.resource_state);
                let mut updated = branch.clone();
                updated.actions.push(action.clone());
                updated.resource_state.memory_available =
                    (updated.resource_state.memory_available + action.resource_delta_memory)
                        .max(0.0).min(1.0);
                updated.resource_state.cpu_available =
                    (updated.resource_state.cpu_available + action.resource_delta_cpu)
                        .max(0.0).min(1.0);
                updated.score = EMA_ALPHA * effect + (1.0 - EMA_ALPHA) * updated.score;

                if (effect - branch.score).abs() > DIVERGENCE_THRESHOLD
                    && new_branches.len() < MAX_BRANCHES
                {
                    divergence_points.push(self.tick);
                    let alt = self.fork_state(&updated);
                    new_branches.push(alt);
                }
                branches[i] = updated;
            }
            branches.extend(new_branches);
        }

        self.total_branches += branches.len() as u64;
        let scores: Vec<f32> = branches.iter().map(|b| b.score).collect();
        let best = scores.iter().cloned().fold(0.0f32, f32::max);
        let worst = scores.iter().cloned().fold(1.0f32, f32::min);
        let avg = scores.iter().sum::<f32>() / scores.len().max(1) as f32;
        let best_id = branches.iter().max_by(|a, b|
            a.score.partial_cmp(&b.score).unwrap_or(core::cmp::Ordering::Equal)
        ).map(|b| b.branch_id).unwrap_or(0);

        if best > self.best_score_ever {
            self.best_score_ever = best;
        }
        self.avg_score_ema = EMA_ALPHA * avg + (1.0 - EMA_ALPHA) * self.avg_score_ema;
        self.branches = branches;

        ScenarioResult {
            scenario_id,
            branches_explored: self.branches.len() as u32,
            best_score: best,
            worst_score: worst,
            avg_score: avg,
            divergence_points,
            recommended_branch: best_id,
        }
    }

    /// Fork a simulation branch into an alternative timeline
    pub fn fork_state(&mut self, parent: &SimBranch) -> SimBranch {
        let noise = (xorshift64(&mut self.rng_state) % 100) as f32 / 1000.0;
        let new_id = fnv1a_hash(&parent.branch_id.to_le_bytes())
            ^ xorshift64(&mut self.rng_state);

        SimBranch {
            branch_id: new_id,
            parent_id: parent.branch_id,
            actions: parent.actions.clone(),
            resource_state: ResourceProjection {
                memory_available: (parent.resource_state.memory_available + noise - 0.05)
                    .max(0.0).min(1.0),
                cpu_available: (parent.resource_state.cpu_available + noise - 0.05)
                    .max(0.0).min(1.0),
                io_bandwidth: parent.resource_state.io_bandwidth,
                fd_available: parent.resource_state.fd_available,
                tick: self.tick,
            },
            score: parent.score * (0.9 + noise),
            divergence_tick: self.tick,
            likelihood: parent.likelihood * 0.6,
        }
    }

    /// Apply a single action and return the score delta
    pub fn apply_action(&mut self, action: &SimAction) -> f32 {
        let before = self.current_resources.memory_available
            + self.current_resources.cpu_available;
        self.current_resources.memory_available =
            (self.current_resources.memory_available + action.resource_delta_memory)
                .max(0.0).min(1.0);
        self.current_resources.cpu_available =
            (self.current_resources.cpu_available + action.resource_delta_cpu)
                .max(0.0).min(1.0);
        let after = self.current_resources.memory_available
            + self.current_resources.cpu_available;
        after - before
    }

    /// Find the first divergence point between two branches
    pub fn divergence_point(&self, branch_a: u64, branch_b: u64) -> Option<u64> {
        let a = self.branches.iter().find(|b| b.branch_id == branch_a);
        let b = self.branches.iter().find(|b| b.branch_id == branch_b);
        match (a, b) {
            (Some(ba), Some(bb)) => {
                let min_len = ba.actions.len().min(bb.actions.len());
                for i in 0..min_len {
                    if ba.actions[i].action_id != bb.actions[i].action_id {
                        return Some(i as u64);
                    }
                }
                if ba.actions.len() != bb.actions.len() {
                    Some(min_len as u64)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Compute likelihood of a specific branch outcome
    #[inline]
    pub fn scenario_likelihood(&self, branch_id: u64) -> f32 {
        self.branches.iter()
            .find(|b| b.branch_id == branch_id)
            .map(|b| b.likelihood)
            .unwrap_or(0.0)
    }

    fn evaluate_action(&self, action: &SimAction, resources: &ResourceProjection) -> f32 {
        let resource_fit = if action.resource_delta_memory < 0.0 {
            resources.memory_available / (-action.resource_delta_memory).max(0.001)
        } else {
            1.0
        };
        let cpu_fit = if action.resource_delta_cpu < 0.0 {
            resources.cpu_available / (-action.resource_delta_cpu).max(0.001)
        } else {
            1.0
        };
        let contention_penalty = self.contention.global_contention * 0.3;
        let process_fit = self.process_models.get(&action.target_process)
            .map(|m| 1.0 - m.contention_sensitivity * contention_penalty)
            .unwrap_or(0.5);
        (resource_fit.min(1.0) * cpu_fit.min(1.0) * process_fit).max(0.0).min(1.0)
    }

    /// Aggregate simulation statistics
    pub fn stats(&self) -> SimulatorStats {
        let avg_branches = if self.total_simulations > 0 {
            self.total_branches as f32 / self.total_simulations as f32
        } else { 0.0 };
        let avg_actions = if self.branches.is_empty() { 0.0 } else {
            self.branches.iter().map(|b| b.actions.len() as f32).sum::<f32>()
                / self.branches.len() as f32
        };
        let div_rate = self.branches.iter()
            .filter(|b| b.parent_id != 0).count() as f32
            / self.branches.len().max(1) as f32;

        SimulatorStats {
            total_simulations: self.total_simulations,
            total_branches: self.total_branches,
            avg_branches_per_sim: avg_branches,
            avg_score: self.avg_score_ema,
            best_score_ever: self.best_score_ever,
            divergence_rate: div_rate,
            avg_actions_per_branch: avg_actions,
        }
    }
}
