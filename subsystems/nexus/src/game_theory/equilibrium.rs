//! Nash equilibrium solvers and ESS checkers.

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::cmp::Ordering;

use super::game::{Game, StrategyProfile};
use super::strategy::MixedStrategy;
use super::types::{PlayerId, StrategyId, Utility};
use crate::types::{NexusError, NexusResult};

/// Nash equilibrium solver
pub struct NashSolver {
    /// Maximum iterations
    max_iterations: usize,
    /// Convergence threshold
    tolerance: f64,
    /// RNG state
    rng: u64,
}

impl Default for NashSolver {
    fn default() -> Self {
        Self {
            max_iterations: 10000,
            tolerance: 1e-8,
            rng: 0xDEADBEEF,
        }
    }
}

impl NashSolver {
    /// Create a new solver
    pub fn new() -> Self {
        Self::default()
    }

    /// Find Nash equilibrium using support enumeration (small games)
    #[inline]
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
                for (s, &fitness) in fitnesses.iter().enumerate().take(num_strategies) {
                    let delta = populations[player][s] * (fitness - avg_fitness) * learning_rate;
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

    #[allow(clippy::only_used_in_recursion)]
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

impl Default for EssChecker {
    fn default() -> Self {
        Self {
            invasion_epsilon: 0.01,
        }
    }
}

impl EssChecker {
    pub fn new() -> Self {
        Self::default()
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

            if (payoff_ms - payoff_ss).abs() < self.invasion_epsilon
                && payoff_mm >= payoff_sm + self.invasion_epsilon
            {
                return false;
            }
        }

        true
    }

    /// Find all ESS strategies
    #[inline]
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
    #[inline]
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

impl Default for KernelResourceGameManager {
    fn default() -> Self {
        Self {
            nash_solver: NashSolver::new(),
            ess_checker: EssChecker::new(),
            current_game: None,
            mechanism: MechanismDesign::new(MechanismType::MaxMinFair),
        }
    }
}

impl KernelResourceGameManager {
    /// Create a new manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Create game for CPU time allocation
    #[inline]
    pub fn create_cpu_game(&mut self, num_processes: usize, total_time: u64) {
        self.current_game = Some(Game::resource_allocation(
            num_processes,
            total_time as f64,
            5, // 5 priority levels
        ));
    }

    /// Find stable resource allocation
    #[inline]
    pub fn find_stable_allocation(&mut self) -> NexusResult<StrategyProfile> {
        let game = self
            .current_game
            .as_ref()
            .ok_or(NexusError::not_initialized())?;
        self.nash_solver.find_nash_equilibrium(game)
    }

    /// Allocate resources using mechanism
    #[inline(always)]
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
