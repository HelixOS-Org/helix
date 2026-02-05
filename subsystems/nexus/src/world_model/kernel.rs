//! Kernel-specific world model components for Helix OS.

use alloc::vec;
use alloc::vec::Vec;

use crate::world_model::planning::MPCPlanner;
use crate::world_model::world::WorldModel;

/// Kernel system state for world model
#[derive(Debug, Clone)]
pub struct KernelSystemState {
    /// CPU usage per core
    pub cpu_usage: Vec<f64>,
    /// Memory usage
    pub memory_usage: f64,
    /// I/O operations
    pub io_ops: f64,
    /// Network bandwidth
    pub network_bw: f64,
    /// Active processes
    pub process_count: usize,
    /// Queue lengths
    pub queue_lengths: Vec<f64>,
    /// Latency metrics
    pub latencies: Vec<f64>,
}

impl KernelSystemState {
    /// Create default state
    pub fn new() -> Self {
        Self {
            cpu_usage: vec![0.0; 4],
            memory_usage: 0.0,
            io_ops: 0.0,
            network_bw: 0.0,
            process_count: 0,
            queue_lengths: vec![0.0; 4],
            latencies: vec![0.0; 4],
        }
    }

    /// Convert to observation vector
    pub fn to_observation(&self) -> Vec<f64> {
        let mut obs = self.cpu_usage.clone();
        obs.push(self.memory_usage);
        obs.push(self.io_ops);
        obs.push(self.network_bw);
        obs.push(self.process_count as f64);
        obs.extend_from_slice(&self.queue_lengths);
        obs.extend_from_slice(&self.latencies);
        obs
    }

    /// Create from observation vector
    pub fn from_observation(obs: &[f64]) -> Self {
        let mut state = Self::new();

        if obs.len() >= 4 {
            state.cpu_usage = obs[..4].to_vec();
        }
        if obs.len() > 4 {
            state.memory_usage = obs[4];
        }
        if obs.len() > 5 {
            state.io_ops = obs[5];
        }
        if obs.len() > 6 {
            state.network_bw = obs[6];
        }
        if obs.len() > 7 {
            state.process_count = obs[7] as usize;
        }
        if obs.len() >= 12 {
            state.queue_lengths = obs[8..12].to_vec();
        }
        if obs.len() >= 16 {
            state.latencies = obs[12..16].to_vec();
        }

        state
    }
}

impl Default for KernelSystemState {
    fn default() -> Self {
        Self::new()
    }
}

/// Kernel action types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelAction {
    /// Adjust scheduler priority
    AdjustPriority,
    /// Migrate process
    MigrateProcess,
    /// Adjust memory limits
    AdjustMemory,
    /// Throttle I/O
    ThrottleIO,
    /// Scale resources
    ScaleResources,
    /// Do nothing
    NoOp,
}

impl KernelAction {
    /// Convert to action vector
    pub fn to_vector(&self) -> Vec<f64> {
        match self {
            KernelAction::AdjustPriority => vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            KernelAction::MigrateProcess => vec![0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
            KernelAction::AdjustMemory => vec![0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
            KernelAction::ThrottleIO => vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            KernelAction::ScaleResources => vec![0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            KernelAction::NoOp => vec![0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
        }
    }

    /// Create from action vector
    pub fn from_vector(v: &[f64]) -> Self {
        if v.is_empty() {
            return KernelAction::NoOp;
        }

        let (max_idx, _) = v
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal))
            .unwrap_or((5, &0.0));

        match max_idx {
            0 => KernelAction::AdjustPriority,
            1 => KernelAction::MigrateProcess,
            2 => KernelAction::AdjustMemory,
            3 => KernelAction::ThrottleIO,
            4 => KernelAction::ScaleResources,
            _ => KernelAction::NoOp,
        }
    }
}

/// Kernel world model manager
pub struct KernelWorldModelManager {
    /// World model
    pub model: WorldModel,
    /// MPC planner
    pub planner: MPCPlanner,
    /// Current system state
    pub current_state: KernelSystemState,
    /// Prediction buffer
    pub predictions: Vec<(KernelSystemState, f64)>,
    /// Action history
    pub action_history: Vec<(KernelAction, f64)>,
    /// Is model trained?
    pub is_trained: bool,
}

impl KernelWorldModelManager {
    /// Create a new kernel world model manager
    pub fn new() -> Self {
        let obs_dim = 16;
        let action_dim = 6;
        let latent_dim = 32;

        Self {
            model: WorldModel::new(obs_dim, action_dim, latent_dim, &[64, 64]),
            planner: MPCPlanner::new(action_dim, 10, 100),
            current_state: KernelSystemState::new(),
            predictions: Vec::new(),
            action_history: Vec::new(),
            is_trained: false,
        }
    }

    /// Update with new observation
    pub fn observe(&mut self, state: KernelSystemState) {
        let obs = state.to_observation();
        self.model.observe(&obs);
        self.current_state = state;
    }

    /// Predict future states
    pub fn predict_future(
        &mut self,
        action: KernelAction,
        horizon: usize,
    ) -> Vec<(KernelSystemState, f64)> {
        let action_vec = action.to_vector();
        let mut actions = Vec::new();

        for _ in 0..horizon {
            actions.push(action_vec.clone());
        }

        let trajectory = self.model.imagine(&actions, None);

        self.predictions = trajectory
            .iter()
            .map(|(state, reward)| {
                let obs = self.model.reconstruct(state);
                (KernelSystemState::from_observation(&obs), *reward)
            })
            .collect();

        self.predictions.clone()
    }

    /// Get optimal action
    pub fn get_optimal_action(&mut self) -> KernelAction {
        let action_vec = self.planner.plan(&self.model);
        let action = KernelAction::from_vector(&action_vec);

        self.action_history.push((action, 0.0));

        action
    }

    /// Compute reward for current state
    pub fn compute_reward(&self) -> f64 {
        // Lower is better for latency and CPU
        let latency_penalty: f64 = self.current_state.latencies.iter().sum();
        let cpu_penalty: f64 = self
            .current_state
            .cpu_usage
            .iter()
            .map(|&u| if u > 0.9 { (u - 0.9) * 10.0 } else { 0.0 })
            .sum();

        let memory_penalty = if self.current_state.memory_usage > 0.95 {
            (self.current_state.memory_usage - 0.95) * 100.0
        } else {
            0.0
        };

        // Throughput is good
        let throughput_reward = self.current_state.io_ops * 0.1;

        throughput_reward - latency_penalty - cpu_penalty - memory_penalty
    }

    /// Get model confidence for state
    pub fn get_confidence(&self) -> f64 {
        1.0 / (1.0 + self.model.current_state.total_uncertainty())
    }
}

impl Default for KernelWorldModelManager {
    fn default() -> Self {
        Self::new()
    }
}
