//! # Evolutionary Game Theory for Kernel Resource Allocation
//!
//! Revolutionary game-theoretic approach to multi-agent kernel optimization.
//! Processes, subsystems, and resources are modeled as strategic agents
//! competing and cooperating for optimal system performance.
//!
//! ## Features
//!
//! - **Nash Equilibrium Finding**: Optimal stable states for resource sharing
//! - **Evolutionary Stable Strategies**: Self-correcting allocation patterns
//! - **Mechanism Design**: Incentive-compatible resource distribution
//! - **Auction Mechanisms**: Fair and efficient resource auctions
//! - **Coalition Formation**: Cooperative optimization between subsystems
//! - **Replicator Dynamics**: Population-level strategy evolution
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                  EVOLUTIONARY GAME THEORY ENGINE                        │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐            │
//! │  │    Process     │  │   Subsystem    │  │   Resource     │            │
//! │  │    Agents      │  │    Agents      │  │    Agents      │            │
//! │  └───────┬────────┘  └───────┬────────┘  └───────┬────────┘            │
//! │          │                   │                   │                      │
//! │          └───────────────────┼───────────────────┘                      │
//! │                              ▼                                          │
//! │                 ┌─────────────────────────┐                             │
//! │                 │     Strategy Space      │                             │
//! │                 │  • Allocation Bids      │                             │
//! │                 │  • Priority Claims      │                             │
//! │                 │  • Cooperation Offers   │                             │
//! │                 └────────────┬────────────┘                             │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                    EQUILIBRIUM SOLVER                            │   │
//! │  │   Nash │ Pareto │ ESS │ Correlated │ Coalition-Proof            │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                          │
//! │                              ▼                                          │
//! │                 ┌─────────────────────────┐                             │
//! │                 │   Optimal Allocation    │                             │
//! │                 │   + Stability Proof     │                             │
//! │                 └─────────────────────────┘                             │
//! │                                                                          │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

#![allow(dead_code)]

extern crate alloc;

// Submodules
mod equilibrium;
mod game;
mod payoff;
mod strategy;
mod types;

// Re-export all public types
pub use equilibrium::{
    Allocation, Bid, EssChecker, KernelResourceGameManager, MechanismDesign, MechanismType,
    NashSolver,
};
pub use game::{Game, StrategyProfile};
pub use payoff::{PayoffCalculator, PayoffFunction, PayoffType};
pub use strategy::{MixedStrategy, Strategy};
pub use types::{GameType, PlayerId, StrategyId, Utility};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prisoners_dilemma() {
        let game = Game::prisoners_dilemma();
        let mut solver = NashSolver::new();

        let equilibrium = solver.find_nash_equilibrium(&game).unwrap();

        // Nash equilibrium is (defect, defect)
        assert_eq!(equilibrium.pure, alloc::vec![1, 1]);
    }

    #[test]
    fn test_coordination_game() {
        let game = Game::coordination(10.0, 0.0);
        let mut solver = NashSolver::new();

        let equilibrium = solver.find_nash_equilibrium(&game).unwrap();

        // Both (A,A) and (B,B) are Nash equilibria
        assert!(equilibrium.pure == alloc::vec![0, 0] || equilibrium.pure == alloc::vec![1, 1]);
    }

    #[test]
    fn test_vcg_mechanism() {
        let mechanism = MechanismDesign::new(MechanismType::VCG);

        let bids = alloc::vec![
            Bid {
                player: 0,
                valuation: 100.0,
                requested_amount: 50.0
            },
            Bid {
                player: 1,
                valuation: 80.0,
                requested_amount: 60.0
            },
            Bid {
                player: 2,
                valuation: 60.0,
                requested_amount: 40.0
            },
        ];

        let allocation = mechanism.allocate(&bids, 100.0);

        // Total allocated should not exceed 100
        let total_allocated: f64 = allocation.allocations.values().sum();
        assert!(total_allocated <= 100.0 + 1e-6);
    }

    #[test]
    fn test_max_min_fair() {
        let mechanism = MechanismDesign::new(MechanismType::MaxMinFair);

        let bids = alloc::vec![
            Bid {
                player: 0,
                valuation: 10.0,
                requested_amount: 30.0
            },
            Bid {
                player: 1,
                valuation: 10.0,
                requested_amount: 30.0
            },
            Bid {
                player: 2,
                valuation: 10.0,
                requested_amount: 30.0
            },
        ];

        let allocation = mechanism.allocate(&bids, 60.0);

        // Each should get 20 (fair share limited by request)
        for (_, &amount) in &allocation.allocations {
            assert!((amount - 20.0).abs() < 1e-6);
        }
    }

    #[test]
    fn test_ess_checker() {
        let game = Game::prisoners_dilemma();
        let checker = EssChecker::new();

        // Defect is ESS in prisoner's dilemma
        assert!(checker.is_ess(&game, 1));
        assert!(!checker.is_ess(&game, 0));
    }
}
