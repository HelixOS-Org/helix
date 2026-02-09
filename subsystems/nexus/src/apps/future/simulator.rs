// SPDX-License-Identifier: GPL-2.0
//! # Apps Simulator
//!
//! Simulates future application behavior by modeling lifecycle phases,
//! resource consumption trajectories, and inter-app interaction effects.
//! Each simulation operates on a lightweight process model that tracks
//! phase transitions, memory/CPU/IO trajectories, and interference from
//! colocated applications. The simulator generates branching futures
//! when phase transitions are uncertain.
//!
//! This is the kernel running the future before it happens.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_SIM_STEPS: usize = 256;
const MAX_SIM_PROCESSES: usize = 128;
const MAX_INTERACTIONS: usize = 64;
const EMA_ALPHA: f32 = 0.12;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const DEFAULT_PHASE_TICKS: u64 = 5_000;

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
// SIMULATION TYPES
// ============================================================================

/// Simulated lifecycle phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SimPhase {
    Init,
    Loading,
    SteadyState,
    Peak,
    Declining,
    Terminating,
}

impl SimPhase {
    fn next(self) -> Self {
        match self {
            SimPhase::Init => SimPhase::Loading,
            SimPhase::Loading => SimPhase::SteadyState,
            SimPhase::SteadyState => SimPhase::Peak,
            SimPhase::Peak => SimPhase::Declining,
            SimPhase::Declining => SimPhase::Terminating,
            SimPhase::Terminating => SimPhase::Terminating,
        }
    }

    fn resource_multiplier(self) -> f32 {
        match self {
            SimPhase::Init => 0.3,
            SimPhase::Loading => 0.7,
            SimPhase::SteadyState => 1.0,
            SimPhase::Peak => 1.6,
            SimPhase::Declining => 0.5,
            SimPhase::Terminating => 0.1,
        }
    }
}

/// Resource trajectory point at a simulation step
#[derive(Debug, Clone, Copy)]
pub struct TrajectoryPoint {
    pub step: u32,
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub io_rate: f32,
    pub phase: SimPhase,
}

/// Inter-app interaction effect
#[derive(Debug, Clone)]
pub struct InteractionEffect {
    pub source_id: u64,
    pub target_id: u64,
    pub effect_kind: InteractionKind,
    pub magnitude: f32,
    pub description: String,
}

/// Kind of inter-app interaction
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InteractionKind {
    CacheContention,
    MemoryPressure,
    IoBandwidthSteal,
    CpuThrottling,
    NetworkContention,
    LockContention,
}

/// Simulated process model
#[derive(Debug, Clone)]
struct SimProcess {
    process_id: u64,
    name_hash: u64,
    phase: SimPhase,
    phase_ticks_remaining: u64,
    base_cpu: f32,
    base_memory: f32,
    base_io: f32,
    cpu_trend: f32,
    memory_trend: f32,
    io_trend: f32,
    interaction_penalty: f32,
}

impl SimProcess {
    fn new(process_id: u64, base_cpu: f32, base_memory: f32, base_io: f32) -> Self {
        let name_bytes = process_id.to_le_bytes();
        Self {
            process_id,
            name_hash: fnv1a_hash(&name_bytes),
            phase: SimPhase::Init,
            phase_ticks_remaining: DEFAULT_PHASE_TICKS,
            base_cpu,
            base_memory,
            base_io,
            cpu_trend: 0.0,
            memory_trend: 0.0,
            io_trend: 0.0,
            interaction_penalty: 0.0,
        }
    }

    fn step(&mut self, rng: &mut u64) -> TrajectoryPoint {
        if self.phase_ticks_remaining == 0 {
            self.phase = self.phase.next();
            let noise = (xorshift64(rng) % 3000) as u64 + 2000;
            self.phase_ticks_remaining = noise;
        } else {
            self.phase_ticks_remaining -= 1;
        }

        let mult = self.phase.resource_multiplier();
        let interaction_scale = 1.0 - self.interaction_penalty.min(0.5);

        let cpu = (self.base_cpu * mult + self.cpu_trend) * interaction_scale;
        let mem = (self.base_memory * mult + self.memory_trend) * interaction_scale;
        let io = (self.base_io * mult + self.io_trend) * interaction_scale;

        self.cpu_trend = EMA_ALPHA * 0.0 + (1.0 - EMA_ALPHA) * self.cpu_trend;
        self.memory_trend = EMA_ALPHA * 0.0 + (1.0 - EMA_ALPHA) * self.memory_trend;
        self.io_trend = EMA_ALPHA * 0.0 + (1.0 - EMA_ALPHA) * self.io_trend;

        TrajectoryPoint {
            step: 0,
            cpu_usage: cpu.max(0.0),
            memory_usage: mem.max(0.0),
            io_rate: io.max(0.0),
            phase: self.phase,
        }
    }
}

/// Result of a full app simulation
#[derive(Debug, Clone)]
pub struct SimulationResult {
    pub process_id: u64,
    pub trajectory: Vec<TrajectoryPoint>,
    pub final_phase: SimPhase,
    pub peak_cpu: f32,
    pub peak_memory: f32,
    pub avg_cpu: f32,
    pub avg_memory: f32,
    pub fidelity: f32,
}

/// Result of lifecycle prediction
#[derive(Debug, Clone)]
pub struct LifecyclePrediction {
    pub process_id: u64,
    pub phases: Vec<(SimPhase, u64)>,
    pub estimated_lifetime_ticks: u64,
    pub confidence: f32,
}

/// Interaction simulation result
#[derive(Debug, Clone)]
pub struct InteractionSimResult {
    pub effects: Vec<InteractionEffect>,
    pub total_penalty: f32,
    pub worst_pair: (u64, u64),
    pub mitigation_potential: f32,
}

// ============================================================================
// SIMULATOR STATS
// ============================================================================

/// Aggregate simulation statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct SimulatorStats {
    pub total_simulations: u64,
    pub total_steps_computed: u64,
    pub avg_trajectory_length: f32,
    pub avg_fidelity: f32,
    pub interaction_sims: u64,
    pub lifecycle_predictions: u64,
    pub tracked_processes: usize,
}

// ============================================================================
// APPS SIMULATOR
// ============================================================================

/// Simulates future application behavior including lifecycle phases,
/// resource consumption trajectories, and inter-app interaction effects.
#[derive(Debug)]
pub struct AppsSimulator {
    models: BTreeMap<u64, SimProcess>,
    interaction_history: Vec<InteractionEffect>,
    total_simulations: u64,
    total_steps: u64,
    lifecycle_predictions: u64,
    interaction_sims: u64,
    tick: u64,
    rng_state: u64,
    fidelity_ema: f32,
}

impl AppsSimulator {
    pub fn new() -> Self {
        Self {
            models: BTreeMap::new(),
            interaction_history: Vec::new(),
            total_simulations: 0,
            total_steps: 0,
            lifecycle_predictions: 0,
            interaction_sims: 0,
            tick: 0,
            rng_state: 0xA1B2_C3D4_E5F6_7890,
            fidelity_ema: 0.5,
        }
    }

    /// Register a process for simulation
    pub fn register_process(
        &mut self,
        process_id: u64,
        base_cpu: f32,
        base_memory: f32,
        base_io: f32,
    ) {
        if self.models.len() < MAX_SIM_PROCESSES {
            self.models.insert(
                process_id,
                SimProcess::new(process_id, base_cpu, base_memory, base_io),
            );
        }
    }

    /// Simulate the future of a specific app for `num_steps` steps
    pub fn simulate_app_future(
        &mut self,
        process_id: u64,
        num_steps: usize,
    ) -> SimulationResult {
        self.total_simulations += 1;
        let steps = num_steps.min(MAX_SIM_STEPS);

        let mut trajectory = Vec::new();
        let mut peak_cpu: f32 = 0.0;
        let mut peak_memory: f32 = 0.0;
        let mut sum_cpu: f32 = 0.0;
        let mut sum_memory: f32 = 0.0;
        let mut final_phase = SimPhase::Init;

        if let Some(proc) = self.models.get(&process_id) {
            let mut sim_proc = proc.clone();
            for i in 0..steps {
                let mut pt = sim_proc.step(&mut self.rng_state);
                pt.step = i as u32;
                if pt.cpu_usage > peak_cpu {
                    peak_cpu = pt.cpu_usage;
                }
                if pt.memory_usage > peak_memory {
                    peak_memory = pt.memory_usage;
                }
                sum_cpu += pt.cpu_usage;
                sum_memory += pt.memory_usage;
                final_phase = pt.phase;
                trajectory.push(pt);
            }
            self.total_steps += steps as u64;
        }

        let n = trajectory.len().max(1) as f32;
        let fidelity = if trajectory.len() > 10 { 0.75 } else { 0.3 };
        self.fidelity_ema = EMA_ALPHA * fidelity + (1.0 - EMA_ALPHA) * self.fidelity_ema;

        SimulationResult {
            process_id,
            trajectory,
            final_phase,
            peak_cpu,
            peak_memory,
            avg_cpu: sum_cpu / n,
            avg_memory: sum_memory / n,
            fidelity,
        }
    }

    /// Predict the lifecycle phases of a process
    pub fn lifecycle_prediction(&mut self, process_id: u64) -> LifecyclePrediction {
        self.lifecycle_predictions += 1;
        let phases = [
            SimPhase::Init,
            SimPhase::Loading,
            SimPhase::SteadyState,
            SimPhase::Peak,
            SimPhase::Declining,
            SimPhase::Terminating,
        ];

        let mut phase_durations = Vec::new();
        let mut total_ticks: u64 = 0;
        let confidence;

        if let Some(proc) = self.models.get(&process_id) {
            let base = proc.name_hash;
            for (i, &phase) in phases.iter().enumerate() {
                let seed = base.wrapping_add(i as u64 * 997);
                let duration = (seed % 10_000) + 2_000;
                phase_durations.push((phase, duration));
                total_ticks += duration;
            }
            confidence = 0.6;
        } else {
            for &phase in &phases {
                let dur = DEFAULT_PHASE_TICKS;
                phase_durations.push((phase, dur));
                total_ticks += dur;
            }
            confidence = 0.2;
        }

        LifecyclePrediction {
            process_id,
            phases: phase_durations,
            estimated_lifetime_ticks: total_ticks,
            confidence,
        }
    }

    /// Generate the resource trajectory for a process
    pub fn resource_trajectory(
        &mut self,
        process_id: u64,
        num_steps: usize,
    ) -> Vec<TrajectoryPoint> {
        let result = self.simulate_app_future(process_id, num_steps);
        result.trajectory
    }

    /// Simulate interactions between colocated processes
    pub fn interaction_simulation(&mut self, process_ids: &[u64]) -> InteractionSimResult {
        self.interaction_sims += 1;
        let mut effects = Vec::new();
        let mut total_penalty: f32 = 0.0;
        let mut worst_penalty: f32 = 0.0;
        let mut worst_pair = (0u64, 0u64);

        let len = process_ids.len().min(MAX_INTERACTIONS);
        for i in 0..len {
            for j in (i + 1)..len {
                let pid_a = process_ids[i];
                let pid_b = process_ids[j];
                let hash_a = fnv1a_hash(&pid_a.to_le_bytes());
                let hash_b = fnv1a_hash(&pid_b.to_le_bytes());
                let combined = hash_a ^ hash_b;

                let kind = match combined % 6 {
                    0 => InteractionKind::CacheContention,
                    1 => InteractionKind::MemoryPressure,
                    2 => InteractionKind::IoBandwidthSteal,
                    3 => InteractionKind::CpuThrottling,
                    4 => InteractionKind::NetworkContention,
                    _ => InteractionKind::LockContention,
                };

                let magnitude = (combined % 100) as f32 / 200.0;
                total_penalty += magnitude;

                if magnitude > worst_penalty {
                    worst_penalty = magnitude;
                    worst_pair = (pid_a, pid_b);
                }

                let mut desc = String::new();
                desc.push_str("interaction:");
                let kind_str = match kind {
                    InteractionKind::CacheContention => "cache",
                    InteractionKind::MemoryPressure => "memory",
                    InteractionKind::IoBandwidthSteal => "io",
                    InteractionKind::CpuThrottling => "cpu",
                    InteractionKind::NetworkContention => "net",
                    InteractionKind::LockContention => "lock",
                };
                desc.push_str(kind_str);

                effects.push(InteractionEffect {
                    source_id: pid_a,
                    target_id: pid_b,
                    effect_kind: kind,
                    magnitude,
                    description: desc,
                });
            }
        }

        if effects.len() > MAX_INTERACTIONS {
            effects.truncate(MAX_INTERACTIONS);
        }
        if self.interaction_history.len() + effects.len() > MAX_INTERACTIONS * 4 {
            self.interaction_history.clear();
        }
        for e in &effects {
            self.interaction_history.push(e.clone());
        }

        let mitigation = if total_penalty > 0.0 {
            (total_penalty * 0.4).min(1.0)
        } else {
            0.0
        };

        InteractionSimResult {
            effects,
            total_penalty,
            worst_pair,
            mitigation_potential: mitigation,
        }
    }

    /// Get the current fidelity estimate for the simulator
    pub fn simulation_fidelity(&self) -> f32 {
        self.fidelity_ema
    }

    /// Update fidelity from actual observation
    pub fn update_fidelity(&mut self, predicted: f32, actual: f32) {
        let error = (predicted - actual).abs() / (actual.abs() + 1.0);
        let fidelity = (1.0 - error).max(0.0);
        self.fidelity_ema = EMA_ALPHA * fidelity + (1.0 - EMA_ALPHA) * self.fidelity_ema;
    }

    /// Remove a process model
    pub fn deregister_process(&mut self, process_id: u64) {
        self.models.remove(&process_id);
    }

    /// Get aggregate statistics
    pub fn stats(&self) -> SimulatorStats {
        let avg_traj = if self.total_simulations > 0 {
            self.total_steps as f32 / self.total_simulations as f32
        } else {
            0.0
        };
        SimulatorStats {
            total_simulations: self.total_simulations,
            total_steps_computed: self.total_steps,
            avg_trajectory_length: avg_traj,
            avg_fidelity: self.fidelity_ema,
            interaction_sims: self.interaction_sims,
            lifecycle_predictions: self.lifecycle_predictions,
            tracked_processes: self.models.len(),
        }
    }
}
