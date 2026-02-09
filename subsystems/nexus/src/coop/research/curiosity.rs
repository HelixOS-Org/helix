// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Curiosity Engine — Proactive Exploration of Cooperation Space
//!
//! Drives autonomous exploration of untested cooperation strategies. Rather
//! than waiting for anomalies, the curiosity engine proactively tries novel
//! sharing strategies, untested fairness algorithms, and experimental trust
//! models. Maintains a curiosity budget that allocates exploration effort
//! across different cooperation dimensions, rewarding discoveries of high
//! novelty and penalizing well-trodden territory. This is the engine that
//! asks "what haven't we tried yet?" and goes looking for surprises.
//!
//! The engine that pushes the boundaries of known cooperation.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_EXPLORATIONS: usize = 512;
const MAX_STRATEGIES: usize = 128;
const MAX_TERRITORY_MAP: usize = 256;
const CURIOSITY_BUDGET_INITIAL: f32 = 100.0;
const CURIOSITY_REGEN_RATE: f32 = 0.5;
const NOVELTY_REWARD_BASE: f32 = 10.0;
const FAMILIARITY_PENALTY: f32 = 0.8;
const EMA_ALPHA: f32 = 0.12;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const EXPLORATION_COST_BASE: f32 = 5.0;
const MIN_NOVELTY_THRESHOLD: f32 = 0.30;
const REWARD_DECAY: f32 = 0.95;
const TERRITORY_RESOLUTION: f32 = 0.05;
const UNCHARTED_BONUS: f32 = 2.0;
const MAX_EXPLORATION_HISTORY: usize = 1024;

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

fn xorshift_f32(state: &mut u64) -> f32 {
    (xorshift64(state) % 10000) as f32 / 10000.0
}

// ============================================================================
// CURIOSITY TYPES
// ============================================================================

/// Dimension of the cooperation space being explored
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CuriosityDimension {
    FairnessAlgorithm,
    SharingStrategy,
    TrustModel,
    NegotiationTactic,
    AuctionDesign,
    ConflictMediation,
    CoalitionFormation,
}

/// Status of an exploration attempt
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExplorationStatus {
    Proposed,
    InProgress,
    Rewarded,
    Exhausted,
    Abandoned,
}

/// A novel strategy being explored
#[derive(Debug, Clone)]
pub struct NovelStrategy {
    pub id: u64,
    pub dimension: CuriosityDimension,
    pub parameters: Vec<f32>,
    pub description: String,
    pub novelty_score: f32,
    pub estimated_reward: f32,
    pub cost: f32,
    pub status: ExplorationStatus,
    pub created_tick: u64,
}

/// Record of an exploration attempt
#[derive(Debug, Clone)]
pub struct ExplorationRecord {
    pub id: u64,
    pub strategy_id: u64,
    pub dimension: CuriosityDimension,
    pub novelty_at_start: f32,
    pub novelty_at_end: f32,
    pub reward_earned: f32,
    pub cost_spent: f32,
    pub tick: u64,
    pub success: bool,
}

/// A territory cell in the cooperation space map
#[derive(Debug, Clone)]
pub struct TerritoryCell {
    pub hash: u64,
    pub visit_count: u32,
    pub best_reward: f32,
    pub last_visit_tick: u64,
    pub dimension: CuriosityDimension,
    pub center: Vec<f32>,
}

/// Budget allocation across dimensions
#[derive(Debug, Clone)]
pub struct BudgetAllocation {
    pub dimension: CuriosityDimension,
    pub allocated: f32,
    pub spent: f32,
    pub reward_earned: f32,
    pub roi: f32,
}

// ============================================================================
// CURIOSITY STATS
// ============================================================================

/// Aggregate statistics for the curiosity engine
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct CuriosityStats {
    pub total_explorations: u64,
    pub successful_explorations: u64,
    pub total_reward_earned: f32,
    pub total_cost_spent: f32,
    pub current_budget: f32,
    pub avg_novelty_ema: f32,
    pub avg_reward_ema: f32,
    pub territory_coverage: f32,
    pub uncharted_cells: u64,
    pub dimension_count: u64,
}

// ============================================================================
// COOPERATION CURIOSITY ENGINE
// ============================================================================

/// Autonomous curiosity-driven cooperation space explorer
#[derive(Debug)]
pub struct CoopCuriosityEngine {
    strategies: VecDeque<NovelStrategy>,
    explorations: VecDeque<ExplorationRecord>,
    territory_map: BTreeMap<u64, TerritoryCell>,
    budget_allocations: BTreeMap<u64, BudgetAllocation>,
    exploration_history: Vec<Vec<f32>>,
    current_budget: f32,
    rng_state: u64,
    tick: u64,
    stats: CuriosityStats,
}

impl CoopCuriosityEngine {
    /// Create a new curiosity engine with the given PRNG seed
    pub fn new(seed: u64) -> Self {
        Self {
            strategies: VecDeque::new(),
            explorations: VecDeque::new(),
            territory_map: BTreeMap::new(),
            budget_allocations: BTreeMap::new(),
            exploration_history: Vec::new(),
            current_budget: CURIOSITY_BUDGET_INITIAL,
            rng_state: seed | 1,
            tick: 0,
            stats: CuriosityStats::default(),
        }
    }

    /// Explore a novel cooperation strategy in the given dimension
    pub fn explore_cooperation(
        &mut self,
        dimension: CuriosityDimension,
        param_count: usize,
    ) -> Option<ExplorationRecord> {
        self.tick += 1;
        let cost = EXPLORATION_COST_BASE + xorshift_f32(&mut self.rng_state) * 2.0;
        if self.current_budget < cost {
            self.regenerate_budget();
            if self.current_budget < cost {
                return None;
            }
        }
        let mut params = Vec::with_capacity(param_count);
        for _ in 0..param_count {
            params.push(xorshift_f32(&mut self.rng_state));
        }
        let novelty = self.compute_novelty(&params, dimension);
        let strategy_id = fnv1a_hash(&self.tick.to_le_bytes()) ^ xorshift64(&mut self.rng_state);
        let mut desc = String::from("Novel ");
        match dimension {
            CuriosityDimension::FairnessAlgorithm => desc.push_str("fairness algorithm"),
            CuriosityDimension::SharingStrategy => desc.push_str("sharing strategy"),
            CuriosityDimension::TrustModel => desc.push_str("trust model"),
            CuriosityDimension::NegotiationTactic => desc.push_str("negotiation tactic"),
            CuriosityDimension::AuctionDesign => desc.push_str("auction design"),
            CuriosityDimension::ConflictMediation => desc.push_str("conflict mediation"),
            CuriosityDimension::CoalitionFormation => desc.push_str("coalition formation"),
        }
        let reward = if novelty >= MIN_NOVELTY_THRESHOLD {
            NOVELTY_REWARD_BASE * novelty * UNCHARTED_BONUS
        } else {
            NOVELTY_REWARD_BASE * novelty * FAMILIARITY_PENALTY
        };
        let strategy = NovelStrategy {
            id: strategy_id,
            dimension,
            parameters: params.clone(),
            description: desc,
            novelty_score: novelty,
            estimated_reward: reward,
            cost,
            status: ExplorationStatus::Rewarded,
            created_tick: self.tick,
        };
        self.current_budget -= cost;
        self.update_territory(&params, dimension, reward);
        if self.exploration_history.len() < MAX_EXPLORATION_HISTORY {
            self.exploration_history.push(params);
        }
        let record_id = xorshift64(&mut self.rng_state);
        let success = novelty >= MIN_NOVELTY_THRESHOLD;
        let record = ExplorationRecord {
            id: record_id,
            strategy_id,
            dimension,
            novelty_at_start: novelty,
            novelty_at_end: novelty * REWARD_DECAY,
            reward_earned: reward,
            cost_spent: cost,
            tick: self.tick,
            success,
        };
        self.stats.total_explorations += 1;
        if success {
            self.stats.successful_explorations += 1;
        }
        self.stats.total_reward_earned += reward;
        self.stats.total_cost_spent += cost;
        self.stats.current_budget = self.current_budget;
        self.stats.avg_novelty_ema = EMA_ALPHA * novelty + (1.0 - EMA_ALPHA) * self.stats.avg_novelty_ema;
        self.stats.avg_reward_ema = EMA_ALPHA * reward + (1.0 - EMA_ALPHA) * self.stats.avg_reward_ema;
        if self.strategies.len() >= MAX_STRATEGIES {
            self.strategies.pop_front();
        }
        self.strategies.push_back(strategy);
        if self.explorations.len() >= MAX_EXPLORATIONS {
            self.explorations.pop_front();
        }
        self.explorations.push_back(record.clone());
        self.update_budget_allocation(dimension, cost, reward);
        Some(record)
    }

    /// Try a novel fairness algorithm variant
    pub fn novel_fairness(&mut self, variant_params: &[f32]) -> Option<NovelStrategy> {
        self.tick += 1;
        let novelty = self.compute_novelty(variant_params, CuriosityDimension::FairnessAlgorithm);
        if novelty < MIN_NOVELTY_THRESHOLD * 0.5 {
            return None;
        }
        let id = fnv1a_hash(&self.tick.to_le_bytes()) ^ xorshift64(&mut self.rng_state);
        let cost = EXPLORATION_COST_BASE * 0.8;
        let reward = NOVELTY_REWARD_BASE * novelty;
        let strategy = NovelStrategy {
            id,
            dimension: CuriosityDimension::FairnessAlgorithm,
            parameters: variant_params.to_vec(),
            description: String::from("Novel fairness variant exploration"),
            novelty_score: novelty,
            estimated_reward: reward,
            cost,
            status: if novelty > MIN_NOVELTY_THRESHOLD {
                ExplorationStatus::Rewarded
            } else {
                ExplorationStatus::Proposed
            },
            created_tick: self.tick,
        };
        self.update_territory(variant_params, CuriosityDimension::FairnessAlgorithm, reward);
        if self.strategies.len() >= MAX_STRATEGIES {
            self.strategies.pop_front();
        }
        self.strategies.push_back(strategy.clone());
        Some(strategy)
    }

    /// Try an experimental resource sharing approach
    pub fn experimental_sharing(&mut self, share_params: &[f32]) -> Option<NovelStrategy> {
        self.tick += 1;
        let novelty = self.compute_novelty(share_params, CuriosityDimension::SharingStrategy);
        if novelty < MIN_NOVELTY_THRESHOLD * 0.3 {
            return None;
        }
        let id = fnv1a_hash(&self.tick.to_le_bytes()) ^ xorshift64(&mut self.rng_state);
        let cost = EXPLORATION_COST_BASE * 1.2;
        let reward = NOVELTY_REWARD_BASE * novelty * 1.5;
        let strategy = NovelStrategy {
            id,
            dimension: CuriosityDimension::SharingStrategy,
            parameters: share_params.to_vec(),
            description: String::from("Experimental sharing strategy probe"),
            novelty_score: novelty,
            estimated_reward: reward,
            cost,
            status: ExplorationStatus::InProgress,
            created_tick: self.tick,
        };
        self.update_territory(share_params, CuriosityDimension::SharingStrategy, reward);
        if self.strategies.len() >= MAX_STRATEGIES {
            self.strategies.pop_front();
        }
        self.strategies.push_back(strategy.clone());
        Some(strategy)
    }

    /// Get the current curiosity budget status
    #[inline(always)]
    pub fn curiosity_budget(&self) -> (f32, f32) {
        (self.current_budget, CURIOSITY_BUDGET_INITIAL)
    }

    /// Compute exploration reward for a given novelty and cost
    pub fn exploration_reward(&self, novelty: f32, cost: f32) -> f32 {
        if cost <= 0.0 {
            return 0.0;
        }
        let base_reward = NOVELTY_REWARD_BASE * novelty;
        let bonus = if novelty >= MIN_NOVELTY_THRESHOLD {
            UNCHARTED_BONUS
        } else {
            FAMILIARITY_PENALTY
        };
        let roi = (base_reward * bonus - cost) / cost;
        roi.max(0.0)
    }

    /// Find uncharted territory in the cooperation space
    pub fn uncharted_territory(&self, dimension: CuriosityDimension) -> Vec<Vec<f32>> {
        let mut uncharted: Vec<Vec<f32>> = Vec::new();
        let dim_key = dimension as u64;
        let visited_hashes: Vec<u64> = self
            .territory_map
            .values()
            .filter(|c| c.dimension == dimension)
            .map(|c| c.hash)
            .collect();
        // Generate candidate points and check if they fall in visited territory
        let steps = (1.0 / TERRITORY_RESOLUTION) as usize;
        let step_limit = steps.min(20); // Keep bounded
        for i in 0..step_limit {
            for j in 0..step_limit {
                let x = i as f32 * TERRITORY_RESOLUTION;
                let y = j as f32 * TERRITORY_RESOLUTION;
                let cell_hash = fnv1a_hash(&x.to_le_bytes()) ^ fnv1a_hash(&y.to_le_bytes()) ^ dim_key;
                if !visited_hashes.contains(&cell_hash) {
                    let mut point = Vec::new();
                    point.push(x);
                    point.push(y);
                    uncharted.push(point);
                    if uncharted.len() >= 32 {
                        return uncharted;
                    }
                }
            }
        }
        uncharted
    }

    /// Get current curiosity statistics
    #[inline(always)]
    pub fn stats(&self) -> &CuriosityStats {
        &self.stats
    }

    /// Number of strategies generated so far
    #[inline(always)]
    pub fn strategy_count(&self) -> usize {
        self.strategies.len()
    }

    /// Get budget allocations per dimension
    #[inline(always)]
    pub fn budget_report(&self) -> Vec<&BudgetAllocation> {
        self.budget_allocations.values().collect()
    }

    // ========================================================================
    // INTERNAL HELPERS
    // ========================================================================

    fn compute_novelty(&self, params: &[f32], dimension: CuriosityDimension) -> f32 {
        if self.exploration_history.is_empty() {
            return 1.0;
        }
        let dim_filter = dimension as u64;
        let relevant: Vec<&Vec<f32>> = self
            .exploration_history
            .iter()
            .collect();
        if relevant.is_empty() {
            return 1.0;
        }
        let mut min_dist = f32::MAX;
        let mut total_dist = 0.0f32;
        let mut count = 0u32;
        for archived in &relevant {
            let len = params.len().min(archived.len());
            let mut dist_sq = 0.0f32;
            for i in 0..len {
                let d = params[i] - archived[i];
                dist_sq += d * d;
            }
            let dist = self.fast_sqrt(dist_sq);
            if dist < min_dist {
                min_dist = dist;
            }
            total_dist += dist;
            count += 1;
        }
        let _ = dim_filter;
        let avg_dist = if count > 0 { total_dist / count as f32 } else { 1.0 };
        let novelty = (min_dist * 0.4 + avg_dist * 0.6).min(1.0);
        novelty
    }

    fn update_territory(&mut self, params: &[f32], dimension: CuriosityDimension, reward: f32) {
        let dim_key = dimension as u64;
        let cell_hash = if params.len() >= 2 {
            fnv1a_hash(&params[0].to_le_bytes()) ^ fnv1a_hash(&params[1].to_le_bytes()) ^ dim_key
        } else if !params.is_empty() {
            fnv1a_hash(&params[0].to_le_bytes()) ^ dim_key
        } else {
            dim_key
        };
        if let Some(cell) = self.territory_map.get_mut(&cell_hash) {
            cell.visit_count += 1;
            if reward > cell.best_reward {
                cell.best_reward = reward;
            }
            cell.last_visit_tick = self.tick;
        } else {
            if self.territory_map.len() < MAX_TERRITORY_MAP {
                self.territory_map.insert(cell_hash, TerritoryCell {
                    hash: cell_hash,
                    visit_count: 1,
                    best_reward: reward,
                    last_visit_tick: self.tick,
                    dimension,
                    center: params.to_vec(),
                });
            }
        }
        let visited = self.territory_map.len() as f32;
        let total = MAX_TERRITORY_MAP as f32;
        self.stats.territory_coverage = visited / total;
        self.stats.uncharted_cells = (MAX_TERRITORY_MAP - self.territory_map.len()) as u64;
    }

    fn update_budget_allocation(&mut self, dimension: CuriosityDimension, cost: f32, reward: f32) {
        let key = dimension as u64;
        if let Some(alloc) = self.budget_allocations.get_mut(&key) {
            alloc.spent += cost;
            alloc.reward_earned += reward;
            alloc.roi = if alloc.spent > 0.0 {
                (alloc.reward_earned - alloc.spent) / alloc.spent
            } else {
                0.0
            };
        } else {
            let alloc = BudgetAllocation {
                dimension,
                allocated: CURIOSITY_BUDGET_INITIAL / 7.0,
                spent: cost,
                reward_earned: reward,
                roi: if cost > 0.0 { (reward - cost) / cost } else { 0.0 },
            };
            self.budget_allocations.insert(key, alloc);
        }
        self.stats.dimension_count = self.budget_allocations.len() as u64;
    }

    fn regenerate_budget(&mut self) {
        self.current_budget += CURIOSITY_REGEN_RATE;
        if self.current_budget > CURIOSITY_BUDGET_INITIAL {
            self.current_budget = CURIOSITY_BUDGET_INITIAL;
        }
        self.stats.current_budget = self.current_budget;
    }

    fn fast_sqrt(&self, x: f32) -> f32 {
        if x <= 0.0 {
            return 0.0;
        }
        let mut guess = x * 0.5;
        for _ in 0..8 {
            if guess > 0.0 {
                guess = (guess + x / guess) * 0.5;
            }
        }
        guess
    }
}
