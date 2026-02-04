//! Game struct and related functionality.

extern crate alloc;
use alloc::vec::Vec;
use core::cmp::Ordering;

use super::payoff::PayoffFunction;
use super::strategy::{MixedStrategy, Strategy};
use super::types::{GameType, PlayerId, StrategyId, Utility};

/// Strategic game definition
#[derive(Debug, Clone)]
pub struct Game {
    /// Number of players
    pub num_players: usize,
    /// Strategy space for each player
    pub strategies: Vec<Vec<Strategy>>,
    /// Payoff function (returns utility for each player)
    payoffs: PayoffFunction,
    /// Game type
    pub game_type: GameType,
}

impl Game {
    /// Create a new game
    pub fn new(
        num_players: usize,
        strategies: Vec<Vec<Strategy>>,
        payoffs: PayoffFunction,
        game_type: GameType,
    ) -> Self {
        Self {
            num_players,
            strategies,
            payoffs,
            game_type,
        }
    }

    /// Create prisoner's dilemma
    pub fn prisoners_dilemma() -> Self {
        let strategies = alloc::vec![
            alloc::vec![Strategy::cooperate(0), Strategy::defect(1)],
            alloc::vec![Strategy::cooperate(0), Strategy::defect(1)],
        ];

        // (cooperate, cooperate) = (3, 3)
        // (cooperate, defect) = (0, 5)
        // (defect, cooperate) = (5, 0)
        // (defect, defect) = (1, 1)
        let matrix = alloc::vec![alloc::vec![(3.0, 3.0), (0.0, 5.0)], alloc::vec![
            (5.0, 0.0),
            (1.0, 1.0)
        ],];

        Self::new(
            2,
            strategies,
            PayoffFunction::from_matrix(matrix),
            GameType::PrisonersDilemma,
        )
    }

    /// Create coordination game
    pub fn coordination(reward: f64, penalty: f64) -> Self {
        let strategies = alloc::vec![
            alloc::vec![
                Strategy::new(0, "A", alloc::vec![]),
                Strategy::new(1, "B", alloc::vec![])
            ],
            alloc::vec![
                Strategy::new(0, "A", alloc::vec![]),
                Strategy::new(1, "B", alloc::vec![])
            ],
        ];

        let matrix = alloc::vec![
            alloc::vec![(reward, reward), (penalty, penalty)],
            alloc::vec![(penalty, penalty), (reward, reward)],
        ];

        Self::new(
            2,
            strategies,
            PayoffFunction::from_matrix(matrix),
            GameType::Coordination,
        )
    }

    /// Create resource allocation game
    pub fn resource_allocation(num_players: usize, total_resource: f64, num_levels: usize) -> Self {
        let strategies: Vec<Vec<Strategy>> = (0..num_players)
            .map(|_| {
                (0..num_levels)
                    .map(|i| {
                        let amount = (i + 1) as f64 * total_resource / num_levels as f64;
                        Strategy::resource_request(i as u32, amount)
                    })
                    .collect()
            })
            .collect();

        Self::new(
            num_players,
            strategies,
            PayoffFunction::resource_allocation(total_resource),
            GameType::ResourceAllocation,
        )
    }

    /// Get payoff for strategy profile
    pub fn payoff(&self, strategies: &[StrategyId]) -> Vec<Utility> {
        self.payoffs.calculate(strategies, &self.strategies)
    }

    /// Get payoff for player given strategy profile
    pub fn player_payoff(&self, player: PlayerId, strategies: &[StrategyId]) -> Utility {
        let payoffs = self.payoff(strategies);
        payoffs.get(player as usize).copied().unwrap_or(0.0)
    }
}

/// Strategy profile (combination of strategies for all players)
#[derive(Debug, Clone)]
pub struct StrategyProfile {
    /// Pure strategies for each player
    pub pure: Vec<StrategyId>,
    /// Mixed strategies for each player (if applicable)
    pub mixed: Option<Vec<MixedStrategy>>,
}

impl StrategyProfile {
    /// Create pure strategy profile
    pub fn pure(strategies: Vec<StrategyId>) -> Self {
        Self {
            pure: strategies,
            mixed: None,
        }
    }

    /// Create mixed strategy profile
    pub fn mixed(strategies: Vec<MixedStrategy>) -> Self {
        let pure = strategies
            .iter()
            .map(|m| {
                m.probabilities
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(Ordering::Equal))
                    .map(|(i, _)| i as StrategyId)
                    .unwrap_or(0)
            })
            .collect();
        Self {
            pure,
            mixed: Some(strategies),
        }
    }
}
