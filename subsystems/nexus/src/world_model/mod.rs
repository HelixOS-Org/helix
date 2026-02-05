//! # World Model Engine for Helix OS Kernel
//!
//! Year 3 "EVOLUTION" - Revolutionary world modeling system that enables
//! the kernel to maintain an internal representation of its environment,
//! predict future states, and plan actions accordingly.
//!
//! ## Key Features
//!
//! - **Internal State Representation**: Latent space encoding of kernel state
//! - **Transition Dynamics**: Learning how actions affect state
//! - **Reward Prediction**: Estimating outcomes of different actions
//! - **Imagination/Planning**: Simulating future trajectories
//! - **Model-Based RL**: Using world model for efficient learning
//! - **Uncertainty Estimation**: Knowing what the model doesn't know
//!
//! ## Kernel Applications
//!
//! - Predictive resource allocation
//! - Proactive anomaly prevention
//! - Long-horizon planning for system optimization
//! - Understanding complex system dynamics
//!
//! ## Module Structure
//!
//! - [`types`] - Constants and type definitions
//! - [`latent`] - Latent state representation
//! - [`encoder`] - Observation encoder
//! - [`decoder`] - Observation decoder
//! - [`dynamics`] - Transition and reward models
//! - [`world`] - Complete world model
//! - [`ensemble`] - Ensemble world model for uncertainty
//! - [`rssm`] - Recurrent State-Space Model
//! - [`planning`] - Model-based planning (MPC)
//! - [`dreamer`] - Dreamer-style actor-critic
//! - [`kernel`] - Kernel-specific world model
//! - [`utils`] - Utility functions

#![no_std]

extern crate alloc;

// ============================================================================
// SUBMODULES
// ============================================================================

pub mod decoder;
pub mod dreamer;
pub mod dynamics;
pub mod encoder;
pub mod ensemble;
pub mod kernel;
pub mod latent;
pub mod planning;
pub mod rssm;
pub mod types;
pub mod utils;
pub mod world;

// ============================================================================
// RE-EXPORTS
// ============================================================================

// Types and constants
pub use types::{DEFAULT_HORIZON, ENSEMBLE_SIZE, MAX_ACTION_DIM, MAX_LATENT_DIM, MAX_OBS_DIM};

// Core types
pub use latent::LatentState;

// Encoder/Decoder
pub use decoder::Decoder;
pub use encoder::Encoder;

// Dynamics
pub use dynamics::{RecurrentCell, RewardModel, TransitionModel};

// World models
pub use ensemble::EnsembleWorldModel;
pub use rssm::RSSM;
pub use world::WorldModel;

// Planning
pub use planning::{ActionSequence, MPCPlanner};

// Dreamer agent
pub use dreamer::{Actor, Critic, DreamerAgent};

// Kernel-specific
pub use kernel::{KernelAction, KernelSystemState, KernelWorldModelManager};

// Utilities
pub use utils::{box_muller, lcg_next};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latent_state() {
        let state = LatentState::new(16);
        assert_eq!(state.dim(), 16);
        assert!(state.total_uncertainty() > 0.0);
    }

    #[test]
    fn test_latent_distance() {
        use alloc::vec;
        let s1 = LatentState::from_vec(vec![1.0, 0.0, 0.0]);
        let s2 = LatentState::from_vec(vec![0.0, 0.0, 0.0]);

        assert!((s1.distance(&s2) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_encoder() {
        use alloc::vec;
        let encoder = Encoder::new(16, 8, &[32, 16]);
        let obs = vec![0.5; 16];

        let state = encoder.encode(&obs);

        assert_eq!(state.z.len(), 8);
        assert_eq!(state.uncertainty.len(), 8);
    }

    #[test]
    fn test_decoder() {
        use alloc::vec;
        let decoder = Decoder::new(8, 16, &[16, 32]);
        let state = LatentState::from_vec(vec![0.5; 8]);

        let obs = decoder.decode(&state);

        assert_eq!(obs.len(), 16);
    }

    #[test]
    fn test_transition_model() {
        use alloc::vec;
        let model = TransitionModel::new(8, 4, &[16], true);
        let state = LatentState::from_vec(vec![0.5; 8]);
        let action = vec![0.1, 0.2, 0.3, 0.4];

        let next = model.predict(&state, &action);

        assert_eq!(next.z.len(), 8);
    }

    #[test]
    fn test_reward_model() {
        use alloc::vec;
        let model = RewardModel::new(8, 4, &[16]);
        let state = LatentState::from_vec(vec![0.5; 8]);
        let action = vec![0.1, 0.2, 0.3, 0.4];

        let reward = model.predict(&state, &action);

        assert!(reward.is_finite());
    }

    #[test]
    fn test_world_model() {
        let model = WorldModel::new(16, 4, 8, &[32]);

        assert_eq!(model.encoder.input_dim, 16);
        assert_eq!(model.decoder.output_dim, 16);
        assert_eq!(model.transition.latent_dim, 8);
    }

    #[test]
    fn test_world_model_observe() {
        use alloc::vec;
        let mut model = WorldModel::new(16, 4, 8, &[32]);
        let obs = vec![0.5; 16];

        let state = model.observe(&obs);

        assert_eq!(state.dim(), 8);
        assert_eq!(model.state_history.len(), 1);
    }

    #[test]
    fn test_world_model_step() {
        use alloc::vec;
        let mut model = WorldModel::new(16, 4, 8, &[32]);
        let obs = vec![0.5; 16];
        model.observe(&obs);

        let action = vec![0.1, 0.2, 0.3, 0.4];
        let (next_state, reward) = model.step(&action);

        assert_eq!(next_state.dim(), 8);
        assert!(reward.is_finite());
    }

    #[test]
    fn test_world_model_imagine() {
        use alloc::vec;
        let mut model = WorldModel::new(16, 4, 8, &[32]);
        let obs = vec![0.5; 16];
        model.observe(&obs);

        let actions = vec![vec![0.1, 0.2, 0.3, 0.4], vec![0.2, 0.3, 0.4, 0.5], vec![
            0.3, 0.4, 0.5, 0.6,
        ]];

        let trajectory = model.imagine(&actions, None);

        assert_eq!(trajectory.len(), 3);
    }

    #[test]
    fn test_ensemble_world_model() {
        let ensemble = EnsembleWorldModel::new(16, 4, 8, &[32], 3);

        assert_eq!(ensemble.models.len(), 3);
    }

    #[test]
    fn test_ensemble_observe() {
        use alloc::vec;
        let mut ensemble = EnsembleWorldModel::new(16, 4, 8, &[32], 3);
        let obs = vec![0.5; 16];

        ensemble.observe(&obs);

        assert!(ensemble.total_epistemic_uncertainty() >= 0.0);
    }

    #[test]
    fn test_recurrent_cell() {
        use alloc::vec;
        let cell = RecurrentCell::new(8, 16);
        let input = vec![0.5; 8];
        let hidden = vec![0.0; 16];

        let output = cell.forward(&input, &hidden);

        assert_eq!(output.len(), 16);
    }

    #[test]
    fn test_rssm() {
        use alloc::vec;
        let mut rssm = RSSM::new(16, 4, 8, 16);
        let obs = vec![0.5; 16];
        let action = vec![0.1, 0.2, 0.3, 0.4];

        let state = rssm.posterior_step(&obs, &action);

        assert!(!state.z.is_empty());
    }

    #[test]
    fn test_rssm_imagine() {
        use alloc::vec;
        let mut rssm = RSSM::new(16, 4, 8, 16);

        let actions = vec![vec![0.1, 0.2, 0.3, 0.4], vec![0.2, 0.3, 0.4, 0.5]];

        let trajectory = rssm.imagine(&actions);

        assert_eq!(trajectory.len(), 2);
    }

    #[test]
    fn test_mpc_planner() {
        let mut planner = MPCPlanner::new(4, 5, 10);
        let model = WorldModel::new(16, 4, 8, &[32]);

        let action = planner.plan(&model);

        assert_eq!(action.len(), 4);
    }

    #[test]
    fn test_actor() {
        use alloc::vec;
        let actor = Actor::new(8, 4, &[32]);
        let state = LatentState::from_vec(vec![0.5; 8]);

        let action = actor.get_action(&state, 0.1);

        assert_eq!(action.len(), 4);

        // Actions should be bounded by tanh
        for &a in &action {
            assert!(a >= -2.0 && a <= 2.0);
        }
    }

    #[test]
    fn test_critic() {
        use alloc::vec;
        let critic = Critic::new(8, &[32]);
        let state = LatentState::from_vec(vec![0.5; 8]);

        let value = critic.value(&state);

        assert!(value.is_finite());
    }

    #[test]
    fn test_dreamer_agent() {
        use alloc::vec;
        let agent = DreamerAgent::new(16, 4, 8);
        let obs = vec![0.5; 16];

        let action = agent.act(&obs, true);

        assert_eq!(action.len(), 4);
    }

    #[test]
    fn test_kernel_system_state() {
        let state = KernelSystemState::new();
        let obs = state.to_observation();
        let recovered = KernelSystemState::from_observation(&obs);

        assert_eq!(state.cpu_usage.len(), recovered.cpu_usage.len());
    }

    #[test]
    fn test_kernel_action() {
        let action = KernelAction::AdjustPriority;
        let vec = action.to_vector();
        let recovered = KernelAction::from_vector(&vec);

        assert_eq!(action, recovered);
    }

    #[test]
    fn test_kernel_world_model_manager() {
        let mut manager = KernelWorldModelManager::new();

        let state = KernelSystemState::new();
        manager.observe(state);

        let predictions = manager.predict_future(KernelAction::NoOp, 5);

        assert_eq!(predictions.len(), 5);
    }

    #[test]
    fn test_kernel_optimal_action() {
        let mut manager = KernelWorldModelManager::new();

        let state = KernelSystemState::new();
        manager.observe(state);

        let action = manager.get_optimal_action();

        // Should return some valid action
        assert!(matches!(
            action,
            KernelAction::AdjustPriority
                | KernelAction::MigrateProcess
                | KernelAction::AdjustMemory
                | KernelAction::ThrottleIO
                | KernelAction::ScaleResources
                | KernelAction::NoOp
        ));
    }

    #[test]
    fn test_compute_reward() {
        let manager = KernelWorldModelManager::new();

        let reward = manager.compute_reward();

        assert!(reward.is_finite());
    }
}
