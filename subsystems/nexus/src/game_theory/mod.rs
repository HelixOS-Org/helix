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

#![no_std]
#![allow(dead_code)]

extern crate alloc;

pub mod auction;
pub mod bargaining;
pub mod coalition;
pub mod evolutionary;
pub mod fair_division;
pub mod mechanism;
pub mod nash;
pub mod replicator;
pub mod voting;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::Ordering;

use crate::types::{NexusError, NexusResult};

/// Player (agent) identifier
pub type PlayerId = u32;

/// Strategy identifier
pub type StrategyId = u32;

/// Utility value
pub type Utility = f64;

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

/// Types of games
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameType {
    /// Zero-sum game (one player's gain is another's loss)
    ZeroSum,
    /// Coordination game (players benefit from matching)
    Coordination,
    /// Prisoner's dilemma
    PrisonersDilemma,
    /// Resource allocation game
    ResourceAllocation,
    /// Congestion game
    Congestion,
    /// Potential game
    Potential,
    /// General sum game
    GeneralSum,
}

/// Strategy representation
#[derive(Debug, Clone)]
pub struct Strategy {
    /// Strategy identifier
    pub id: StrategyId,
    /// Strategy name
    pub name: String,
    /// Action parameters
    pub params: Vec<f64>,
}

impl Strategy {
    /// Create a new strategy
    pub fn new(id: StrategyId, name: &str, params: Vec<f64>) -> Self {
        Self {
            id,
            name: String::from(name),
            params,
        }
    }

    /// Pure strategy for resource request
    pub fn resource_request(id: StrategyId, amount: f64) -> Self {
        Self::new(
            id,
            &alloc::format!("request_{}", amount as u64),
            alloc::vec![amount],
        )
    }

    /// Cooperation strategy
    pub fn cooperate(id: StrategyId) -> Self {
        Self::new(id, "cooperate", alloc::vec![1.0])
    }

    /// Defection strategy
    pub fn defect(id: StrategyId) -> Self {
        Self::new(id, "defect", alloc::vec![0.0])
    }
}

/// Mixed strategy (probability distribution over pure strategies)
#[derive(Debug, Clone)]
pub struct MixedStrategy {
    /// Probabilities for each pure strategy
    pub probabilities: Vec<f64>,
}

impl MixedStrategy {
    /// Create a pure strategy (probability 1 on one strategy)
    pub fn pure(strategy_idx: usize, num_strategies: usize) -> Self {
        let mut probs = alloc::vec![0.0; num_strategies];
        if strategy_idx < num_strategies {
            probs[strategy_idx] = 1.0;
        }
        Self {
            probabilities: probs,
        }
    }

    /// Create uniform mixed strategy
    pub fn uniform(num_strategies: usize) -> Self {
        let prob = 1.0 / num_strategies as f64;
        Self {
            probabilities: alloc::vec![prob; num_strategies],
        }
    }

    /// Create from probability vector
    pub fn from_probs(probabilities: Vec<f64>) -> Self {
        let mut probs = probabilities;
        // Normalize
        let sum: f64 = probs.iter().sum();
        if sum > 1e-10 {
            for p in &mut probs {
                *p /= sum;
            }
        }
        Self {
            probabilities: probs,
        }
    }

    /// Sample a pure strategy from mixed
    pub fn sample(&self, random: f64) -> usize {
        let mut cumulative = 0.0;
        for (i, &p) in self.probabilities.iter().enumerate() {
            cumulative += p;
            if random < cumulative {
                return i;
            }
        }
        self.probabilities.len() - 1
    }

    /// Get support (strategies with non-zero probability)
    pub fn support(&self) -> Vec<usize> {
        self.probabilities
            .iter()
            .enumerate()
            .filter(|(_, &p)| p > 1e-10)
            .map(|(i, _)| i)
            .collect()
    }
}

/// Payoff function type
#[derive(Debug, Clone)]
pub struct PayoffFunction {
    /// Payoff matrix for 2-player games
    matrix: Option<Vec<Vec<(Utility, Utility)>>>,
    /// Custom payoff calculator
    custom: Option<PayoffCalculator>,
}

/// Custom payoff calculator
#[derive(Debug, Clone)]
pub struct PayoffCalculator {
    /// Game parameters
    params: Vec<f64>,
    /// Calculation type
    calc_type: PayoffType,
}

#[derive(Debug, Clone, Copy)]
pub enum PayoffType {
    /// Linear resource allocation
    LinearResource,
    /// Congestion with delay
    CongestionDelay,
    /// Bandwidth sharing
    BandwidthShare,
    /// CPU time sharing
    CpuShare,
    /// Memory allocation
    MemoryAlloc,
}

impl PayoffFunction {
    /// Create from matrix (2-player games)
    pub fn from_matrix(matrix: Vec<Vec<(Utility, Utility)>>) -> Self {
        Self {
            matrix: Some(matrix),
            custom: None,
        }
    }

    /// Create for resource allocation
    pub fn resource_allocation(total_resource: f64) -> Self {
        Self {
            matrix: None,
            custom: Some(PayoffCalculator {
                params: alloc::vec![total_resource],
                calc_type: PayoffType::LinearResource,
            }),
        }
    }

    /// Calculate payoffs for strategy profile
    pub fn calculate(
        &self,
        strategies: &[StrategyId],
        all_strategies: &[Vec<Strategy>],
    ) -> Vec<Utility> {
        if let Some(ref matrix) = self.matrix {
            // 2-player matrix game
            if strategies.len() >= 2 {
                let (u1, u2) = matrix
                    .get(strategies[0] as usize)
                    .and_then(|row| row.get(strategies[1] as usize))
                    .copied()
                    .unwrap_or((0.0, 0.0));
                return alloc::vec![u1, u2];
            }
        }

        if let Some(ref calc) = self.custom {
            return self.calculate_custom(calc, strategies, all_strategies);
        }

        alloc::vec![0.0; strategies.len()]
    }

    fn calculate_custom(
        &self,
        calc: &PayoffCalculator,
        strategies: &[StrategyId],
        all_strategies: &[Vec<Strategy>],
    ) -> Vec<Utility> {
        match calc.calc_type {
            PayoffType::LinearResource => {
                let total = calc.params.get(0).copied().unwrap_or(100.0);
                let requests: Vec<f64> = strategies
                    .iter()
                    .zip(all_strategies.iter())
                    .map(|(&sid, strats)| {
                        strats
                            .get(sid as usize)
                            .and_then(|s| s.params.first())
                            .copied()
                            .unwrap_or(0.0)
                    })
                    .collect();

                let total_request: f64 = requests.iter().sum();
                if total_request <= total {
                    // All requests satisfied
                    requests
                } else {
                    // Proportional allocation
                    requests
                        .iter()
                        .map(|&r| r * total / total_request)
                        .collect()
                }
            },
            PayoffType::CongestionDelay => {
                // Higher congestion = lower utility
                let n = strategies.len() as f64;
                let base_utility = calc.params.get(0).copied().unwrap_or(10.0);
                alloc::vec![base_utility / n; strategies.len()]
            },
            _ => alloc::vec![0.0; strategies.len()],
        }
    }
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

/// Nash equilibrium solver
pub struct NashSolver {
    /// Maximum iterations
    max_iterations: usize,
    /// Convergence threshold
    tolerance: f64,
    /// RNG state
    rng: u64,
}

impl NashSolver {
    /// Create a new solver
    pub fn new() -> Self {
        Self {
            max_iterations: 10000,
            tolerance: 1e-8,
            rng: 0xDEADBEEF,
        }
    }

    /// Find Nash equilibrium using support enumeration (small games)
    pub fn find_nash_equilibrium(&mut self, game: &Game) -> NexusResult<StrategyProfile> {
        if game.num_players != 2 {
            return self.find_nash_n_player(game);
        }

        // For 2-player games, use Lemke-Howson style approach
        self.lemke_howson_2player(game)
    }

    /// Lemke-Howson for 2-player games
    fn lemke_howson_2player(&mut self, game: &Game) -> NexusResult<StrategyProfile> {
        let m = game.strategies[0].len();
        let n = game.strategies[1].len();

        // Try support enumeration for small games
        for support1_size in 1..=m {
            for support2_size in 1..=n {
                if let Some(equilibrium) =
                    self.check_support_equilibrium(game, support1_size, support2_size)
                {
                    return Ok(equilibrium);
                }
            }
        }

        // Fall back to fictitious play
        self.fictitious_play(game)
    }

    /// Check if given support sizes yield an equilibrium
    fn check_support_equilibrium(
        &mut self,
        game: &Game,
        support1_size: usize,
        support2_size: usize,
    ) -> Option<StrategyProfile> {
        // Simplified: check pure strategy equilibria first
        if support1_size == 1 && support2_size == 1 {
            for s1 in 0..game.strategies[0].len() {
                for s2 in 0..game.strategies[1].len() {
                    if self.is_pure_nash(game, s1 as StrategyId, s2 as StrategyId) {
                        return Some(StrategyProfile::pure(alloc::vec![s1 as u32, s2 as u32]));
                    }
                }
            }
        }
        None
    }

    /// Check if (s1, s2) is a pure Nash equilibrium
    fn is_pure_nash(&self, game: &Game, s1: StrategyId, s2: StrategyId) -> bool {
        let current_payoffs = game.payoff(&[s1, s2]);

        // Check player 1 deviations
        for alt_s1 in 0..game.strategies[0].len() as StrategyId {
            if alt_s1 != s1 {
                let alt_payoffs = game.payoff(&[alt_s1, s2]);
                if alt_payoffs[0] > current_payoffs[0] + self.tolerance {
                    return false;
                }
            }
        }

        // Check player 2 deviations
        for alt_s2 in 0..game.strategies[1].len() as StrategyId {
            if alt_s2 != s2 {
                let alt_payoffs = game.payoff(&[s1, alt_s2]);
                if alt_payoffs[1] > current_payoffs[1] + self.tolerance {
                    return false;
                }
            }
        }

        true
    }

    /// Fictitious play for finding mixed equilibrium
    fn fictitious_play(&mut self, game: &Game) -> NexusResult<StrategyProfile> {
        let n = game.num_players;

        // Initialize counts for each strategy
        let mut counts: Vec<Vec<f64>> = game
            .strategies
            .iter()
            .map(|s| alloc::vec![1.0; s.len()])
            .collect();

        for _ in 0..self.max_iterations {
            // Each player best responds to empirical distribution
            let mut new_strategies = Vec::with_capacity(n);

            for player in 0..n {
                let opponent_mixed: Vec<_> = counts
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != player)
                    .map(|(_, c)| {
                        let sum: f64 = c.iter().sum();
                        MixedStrategy::from_probs(c.iter().map(|&x| x / sum).collect())
                    })
                    .collect();

                let best_response = self.best_response(game, player, &opponent_mixed)?;
                new_strategies.push(best_response);
            }

            // Update counts
            for (player, strategy) in new_strategies.iter().enumerate() {
                counts[player][*strategy as usize] += 1.0;
            }
        }

        // Convert counts to mixed strategies
        let mixed: Vec<MixedStrategy> = counts
            .into_iter()
            .map(|c| {
                let sum: f64 = c.iter().sum();
                MixedStrategy::from_probs(c.into_iter().map(|x| x / sum).collect())
            })
            .collect();

        Ok(StrategyProfile::mixed(mixed))
    }

    /// Find best response for a player
    fn best_response(
        &self,
        game: &Game,
        player: usize,
        opponent_strategies: &[MixedStrategy],
    ) -> NexusResult<StrategyId> {
        let mut best_strategy = 0;
        let mut best_utility = f64::MIN;

        for (s, _) in game.strategies[player].iter().enumerate() {
            let utility = self.expected_utility(game, player, s as StrategyId, opponent_strategies);
            if utility > best_utility {
                best_utility = utility;
                best_strategy = s as StrategyId;
            }
        }

        Ok(best_strategy)
    }

    /// Calculate expected utility
    fn expected_utility(
        &self,
        game: &Game,
        player: usize,
        strategy: StrategyId,
        opponent_strategies: &[MixedStrategy],
    ) -> Utility {
        // Simplified: assume 2-player
        if opponent_strategies.is_empty() {
            return 0.0;
        }

        let opponent = &opponent_strategies[0];
        let mut expected = 0.0;

        for (opp_s, &prob) in opponent.probabilities.iter().enumerate() {
            let profile = if player == 0 {
                alloc::vec![strategy, opp_s as StrategyId]
            } else {
                alloc::vec![opp_s as StrategyId, strategy]
            };

            let payoffs = game.payoff(&profile);
            expected += prob * payoffs[player];
        }

        expected
    }

    /// Find Nash equilibrium for n-player games
    fn find_nash_n_player(&mut self, game: &Game) -> NexusResult<StrategyProfile> {
        // Use replicator dynamics approximation
        self.replicator_dynamics(game)
    }

    /// Replicator dynamics for equilibrium finding
    fn replicator_dynamics(&mut self, game: &Game) -> NexusResult<StrategyProfile> {
        let n = game.num_players;

        // Initialize with uniform strategies
        let mut populations: Vec<Vec<f64>> = game
            .strategies
            .iter()
            .map(|s| {
                let p = 1.0 / s.len() as f64;
                alloc::vec![p; s.len()]
            })
            .collect();

        let learning_rate = 0.01;

        for _ in 0..self.max_iterations {
            let mut new_populations = populations.clone();

            for player in 0..n {
                let num_strategies = game.strategies[player].len();

                // Calculate fitness for each strategy
                let mut fitnesses = Vec::with_capacity(num_strategies);
                let mut avg_fitness = 0.0;

                for s in 0..num_strategies {
                    let fitness =
                        self.strategy_fitness(game, player, s as StrategyId, &populations);
                    fitnesses.push(fitness);
                    avg_fitness += populations[player][s] * fitness;
                }

                // Update populations using replicator equation
                for s in 0..num_strategies {
                    let delta =
                        populations[player][s] * (fitnesses[s] - avg_fitness) * learning_rate;
                    new_populations[player][s] += delta;
                    new_populations[player][s] = new_populations[player][s].max(0.0);
                }

                // Normalize
                let sum: f64 = new_populations[player].iter().sum();
                if sum > 1e-10 {
                    for p in &mut new_populations[player] {
                        *p /= sum;
                    }
                }
            }

            populations = new_populations;
        }

        let mixed: Vec<MixedStrategy> = populations
            .into_iter()
            .map(MixedStrategy::from_probs)
            .collect();

        Ok(StrategyProfile::mixed(mixed))
    }

    /// Calculate fitness for a strategy
    fn strategy_fitness(
        &self,
        game: &Game,
        player: usize,
        strategy: StrategyId,
        populations: &[Vec<f64>],
    ) -> f64 {
        // Expected payoff when playing this strategy against population mix
        let mut expected = 0.0;

        // Simplified: enumerate opponent strategy combinations
        // For efficiency, use sampling for large games
        self.enumerate_expected_payoff(
            game,
            player,
            strategy,
            populations,
            0,
            alloc::vec![],
            &mut expected,
        );

        expected
    }

    fn enumerate_expected_payoff(
        &self,
        game: &Game,
        player: usize,
        strategy: StrategyId,
        populations: &[Vec<f64>],
        current_player: usize,
        mut profile: Vec<StrategyId>,
        expected: &mut f64,
    ) {
        if current_player == game.num_players {
            // Calculate payoff for this profile
            let mut probability = 1.0;
            for (p, &s) in profile.iter().enumerate() {
                if p != player {
                    probability *= populations[p].get(s as usize).copied().unwrap_or(0.0);
                }
            }

            let payoffs = game.payoff(&profile);
            *expected += probability * payoffs[player];
            return;
        }

        if current_player == player {
            profile.push(strategy);
            self.enumerate_expected_payoff(
                game,
                player,
                strategy,
                populations,
                current_player + 1,
                profile,
                expected,
            );
        } else {
            for s in 0..game.strategies[current_player].len() {
                let mut new_profile = profile.clone();
                new_profile.push(s as StrategyId);
                self.enumerate_expected_payoff(
                    game,
                    player,
                    strategy,
                    populations,
                    current_player + 1,
                    new_profile,
                    expected,
                );
            }
        }
    }
}

/// Evolutionary Stable Strategy (ESS) checker
pub struct EssChecker {
    /// Invasion threshold
    invasion_epsilon: f64,
}

impl EssChecker {
    pub fn new() -> Self {
        Self {
            invasion_epsilon: 0.01,
        }
    }

    /// Check if strategy is evolutionarily stable
    pub fn is_ess(&self, game: &Game, strategy: StrategyId) -> bool {
        // For symmetric 2-player games
        let payoff_ss = game.player_payoff(0, &[strategy, strategy]);

        // Check against all mutant strategies
        for mutant in 0..game.strategies[0].len() as StrategyId {
            if mutant == strategy {
                continue;
            }

            let payoff_ms = game.player_payoff(0, &[mutant, strategy]);
            let payoff_sm = game.player_payoff(0, &[strategy, mutant]);
            let payoff_mm = game.player_payoff(0, &[mutant, mutant]);

            // ESS conditions:
            // 1. u(S, S) > u(M, S), or
            // 2. u(S, S) = u(M, S) and u(S, M) > u(M, M)

            if payoff_ms > payoff_ss + self.invasion_epsilon {
                return false;
            }

            if (payoff_ms - payoff_ss).abs() < self.invasion_epsilon {
                if payoff_mm >= payoff_sm + self.invasion_epsilon {
                    return false;
                }
            }
        }

        true
    }

    /// Find all ESS strategies
    pub fn find_all_ess(&self, game: &Game) -> Vec<StrategyId> {
        (0..game.strategies[0].len() as StrategyId)
            .filter(|&s| self.is_ess(game, s))
            .collect()
    }
}

/// Mechanism design for kernel resource allocation
pub struct MechanismDesign {
    /// Mechanism type
    mechanism_type: MechanismType,
}

#[derive(Debug, Clone, Copy)]
pub enum MechanismType {
    /// Vickrey-Clarke-Groves (VCG) mechanism
    VCG,
    /// First-price auction
    FirstPrice,
    /// Second-price auction
    SecondPrice,
    /// Proportional share
    ProportionalShare,
    /// Max-min fairness
    MaxMinFair,
}

/// Bid in an auction
#[derive(Debug, Clone)]
pub struct Bid {
    pub player: PlayerId,
    pub valuation: f64,
    pub requested_amount: f64,
}

/// Allocation result
#[derive(Debug, Clone)]
pub struct Allocation {
    /// Amount allocated to each player
    pub allocations: BTreeMap<PlayerId, f64>,
    /// Payment from each player
    pub payments: BTreeMap<PlayerId, f64>,
    /// Total social welfare
    pub welfare: f64,
}

impl MechanismDesign {
    /// Create a new mechanism
    pub fn new(mechanism_type: MechanismType) -> Self {
        Self { mechanism_type }
    }

    /// Run the mechanism
    pub fn allocate(&self, bids: &[Bid], total_resource: f64) -> Allocation {
        match self.mechanism_type {
            MechanismType::VCG => self.vcg_allocation(bids, total_resource),
            MechanismType::ProportionalShare => self.proportional_allocation(bids, total_resource),
            MechanismType::MaxMinFair => self.max_min_fair_allocation(bids, total_resource),
            _ => self.proportional_allocation(bids, total_resource),
        }
    }

    /// VCG mechanism allocation
    fn vcg_allocation(&self, bids: &[Bid], total_resource: f64) -> Allocation {
        let mut allocations = BTreeMap::new();
        let mut payments = BTreeMap::new();

        // Sort by valuation density (value per unit)
        let mut sorted_bids: Vec<_> = bids
            .iter()
            .map(|b| {
                (
                    b.player,
                    b.valuation / b.requested_amount.max(1.0),
                    b.requested_amount,
                )
            })
            .collect();
        sorted_bids.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));

        // Allocate greedily
        let mut remaining = total_resource;
        let mut welfare = 0.0;

        for (player, density, requested) in &sorted_bids {
            let allocated = requested.min(remaining);
            allocations.insert(*player, allocated);
            welfare += density * allocated;
            remaining -= allocated;

            if remaining <= 0.0 {
                break;
            }
        }

        // Calculate VCG payments (externality)
        for bid in bids {
            // Welfare without this player
            let welfare_without =
                self.calculate_welfare_without(&sorted_bids, bid.player, total_resource);
            // Others' welfare with this player
            let others_welfare_with = welfare
                - (bid.valuation / bid.requested_amount.max(1.0))
                    * allocations.get(&bid.player).copied().unwrap_or(0.0);

            let payment = welfare_without - others_welfare_with;
            payments.insert(bid.player, payment.max(0.0));
        }

        Allocation {
            allocations,
            payments,
            welfare,
        }
    }

    fn calculate_welfare_without(
        &self,
        sorted_bids: &[(PlayerId, f64, f64)],
        exclude: PlayerId,
        total: f64,
    ) -> f64 {
        let mut remaining = total;
        let mut welfare = 0.0;

        for (player, density, requested) in sorted_bids {
            if *player == exclude {
                continue;
            }
            let allocated = requested.min(remaining);
            welfare += density * allocated;
            remaining -= allocated;
        }

        welfare
    }

    /// Proportional share allocation
    fn proportional_allocation(&self, bids: &[Bid], total_resource: f64) -> Allocation {
        let total_requested: f64 = bids.iter().map(|b| b.requested_amount).sum();

        let mut allocations = BTreeMap::new();
        let payments = BTreeMap::new();
        let mut welfare = 0.0;

        for bid in bids {
            let share = if total_requested > 0.0 {
                (bid.requested_amount / total_requested) * total_resource.min(total_requested)
            } else {
                0.0
            };
            allocations.insert(bid.player, share);
            welfare += (bid.valuation / bid.requested_amount.max(1.0)) * share;
        }

        Allocation {
            allocations,
            payments,
            welfare,
        }
    }

    /// Max-min fair allocation
    fn max_min_fair_allocation(&self, bids: &[Bid], total_resource: f64) -> Allocation {
        let mut allocations: BTreeMap<PlayerId, f64> =
            bids.iter().map(|b| (b.player, 0.0)).collect();

        let mut remaining = total_resource;
        let mut unsatisfied: Vec<_> = bids.iter().collect();

        while remaining > 1e-10 && !unsatisfied.is_empty() {
            let fair_share = remaining / unsatisfied.len() as f64;

            let mut newly_satisfied = Vec::new();

            for bid in &unsatisfied {
                let current = allocations.get(&bid.player).copied().unwrap_or(0.0);
                let needed = bid.requested_amount - current;

                let give = fair_share.min(needed);
                *allocations.entry(bid.player).or_insert(0.0) += give;
                remaining -= give;

                if current + give >= bid.requested_amount - 1e-10 {
                    newly_satisfied.push(bid.player);
                }
            }

            unsatisfied.retain(|b| !newly_satisfied.contains(&b.player));
        }

        let welfare = bids
            .iter()
            .map(|b| {
                let alloc = allocations.get(&b.player).copied().unwrap_or(0.0);
                (b.valuation / b.requested_amount.max(1.0)) * alloc
            })
            .sum();

        Allocation {
            allocations,
            payments: BTreeMap::new(),
            welfare,
        }
    }
}

/// Kernel resource game manager
pub struct KernelResourceGameManager {
    /// Nash solver
    nash_solver: NashSolver,
    /// ESS checker
    ess_checker: EssChecker,
    /// Current game
    current_game: Option<Game>,
    /// Mechanism designer
    mechanism: MechanismDesign,
}

impl KernelResourceGameManager {
    /// Create a new manager
    pub fn new() -> Self {
        Self {
            nash_solver: NashSolver::new(),
            ess_checker: EssChecker::new(),
            current_game: None,
            mechanism: MechanismDesign::new(MechanismType::MaxMinFair),
        }
    }

    /// Create game for CPU time allocation
    pub fn create_cpu_game(&mut self, num_processes: usize, total_time: u64) {
        self.current_game = Some(Game::resource_allocation(
            num_processes,
            total_time as f64,
            5, // 5 priority levels
        ));
    }

    /// Find stable resource allocation
    pub fn find_stable_allocation(&mut self) -> NexusResult<StrategyProfile> {
        let game = self
            .current_game
            .as_ref()
            .ok_or(NexusError::NotInitialized)?;
        self.nash_solver.find_nash_equilibrium(game)
    }

    /// Allocate resources using mechanism
    pub fn allocate_resources(&self, bids: &[Bid], total: f64) -> Allocation {
        self.mechanism.allocate(bids, total)
    }

    /// Check if allocation is stable
    pub fn is_stable(&self, profile: &StrategyProfile) -> bool {
        if let Some(ref game) = self.current_game {
            // Check if no player wants to deviate
            for player in 0..game.num_players {
                let current_payoff = game.player_payoff(player as u32, &profile.pure);

                for alt_strategy in 0..game.strategies[player].len() as StrategyId {
                    let mut alt_profile = profile.pure.clone();
                    alt_profile[player] = alt_strategy;
                    let alt_payoff = game.player_payoff(player as u32, &alt_profile);

                    if alt_payoff > current_payoff + 1e-6 {
                        return false;
                    }
                }
            }
            true
        } else {
            false
        }
    }
}

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
