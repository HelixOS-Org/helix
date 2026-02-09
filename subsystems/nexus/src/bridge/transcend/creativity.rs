// SPDX-License-Identifier: GPL-2.0
//! # Bridge Creativity — Novel Solution Generation
//!
//! When standard optimisation fails, the bridge gets *creative*: it
//! combines strategies never combined before, inverts assumptions, tries
//! random mutations of known-good strategies, and evaluates the results
//! against a novelty metric.
//!
//! `CreativeSolution` records the approach, its novelty score, risk, and
//! expected reward. FNV-1a hashing indexes the solution vault;
//! xorshift64 drives stochastic mutation and combination; EMA tracks the
//! running creativity score.

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_SOLUTIONS: usize = 512;
const MAX_STRATEGY_POOL: usize = 128;
const MAX_COMBINATION_DEPTH: usize = 4;
const MUTATION_RATE: f32 = 0.15;
const INVERSION_BONUS: f32 = 0.20;
const NOVELTY_THRESHOLD: f32 = 0.60;
const EMA_ALPHA: f32 = 0.10;
const RISK_CEILING: f32 = 0.90;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

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
// CREATIVITY TYPES
// ============================================================================

/// How a creative solution was produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CreativeMethod {
    RandomMutation,
    StrategyInversion,
    CrossCombination,
    AnalogicalTransfer,
    ConstraintRelaxation,
    SerendipitousDiscovery,
    BiasedExploration,
    TabuViolation,
}

/// Risk category for a creative solution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Negligible,
    Low,
    Moderate,
    High,
    Extreme,
}

/// A known strategy that can be mutated or combined.
#[derive(Debug, Clone)]
pub struct KnownStrategy {
    pub strategy_id: u64,
    pub name: String,
    pub parameters: Vec<f32>,
    pub fitness: f32,
    pub usage_count: u64,
}

/// A creative solution produced by the engine.
#[derive(Debug, Clone)]
pub struct CreativeSolution {
    pub solution_id: u64,
    pub approach: String,
    pub method: CreativeMethod,
    pub novelty: f32,
    pub risk: f32,
    pub risk_level: RiskLevel,
    pub expected_reward: f32,
    pub parent_strategies: Vec<u64>,
    pub tick: u64,
    pub accepted: bool,
}

/// Novelty assessment report.
#[derive(Debug, Clone)]
pub struct NoveltyReport {
    pub solution_id: u64,
    pub novelty_score: f32,
    pub distance_from_known: f32,
    pub similar_solutions: usize,
    pub is_novel: bool,
}

// ============================================================================
// CREATIVITY STATS
// ============================================================================

/// Aggregate statistics for the creativity engine.
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct CreativityStats {
    pub total_solutions: u64,
    pub novel_solutions: u64,
    pub mutations_tried: u64,
    pub inversions_tried: u64,
    pub combinations_tried: u64,
    pub avg_novelty: f32,
    pub avg_risk: f32,
    pub avg_reward: f32,
    pub acceptance_rate: f32,
    pub creativity_ema: f32,
}

// ============================================================================
// STRATEGY VAULT
// ============================================================================

#[derive(Debug, Clone)]
struct StrategyVault {
    strategies: BTreeMap<u64, KnownStrategy>,
    param_hash_index: LinearMap<u64, 64>, // param_hash -> strategy_id
}

impl StrategyVault {
    fn new() -> Self {
        Self { strategies: BTreeMap::new(), param_hash_index: BTreeMap::new() }
    }

    fn add(&mut self, strategy: KnownStrategy) {
        if self.strategies.len() >= MAX_STRATEGY_POOL {
            // Evict lowest-fitness
            if let Some((&worst_id, _)) = self.strategies.iter().min_by(|(_, a), (_, b)| {
                a.fitness.partial_cmp(&b.fitness).unwrap_or(core::cmp::Ordering::Equal)
            }) {
                self.strategies.remove(&worst_id);
            }
        }
        let param_hash = self.hash_params(&strategy.parameters);
        self.param_hash_index.insert(param_hash, strategy.strategy_id);
        self.strategies.insert(strategy.strategy_id, strategy);
    }

    fn hash_params(&self, params: &[f32]) -> u64 {
        let mut data = Vec::new();
        for p in params {
            data.extend_from_slice(&p.to_le_bytes());
        }
        fnv1a_hash(&data)
    }

    fn get(&self, id: u64) -> Option<&KnownStrategy> {
        self.strategies.get(&id)
    }

    fn random_pair(&self, rng: &mut u64) -> Option<(u64, u64)> {
        let keys: Vec<u64> = self.strategies.keys().copied().collect();
        if keys.len() < 2 {
            return None;
        }
        let i = (xorshift64(rng) as usize) % keys.len();
        let mut j = (xorshift64(rng) as usize) % keys.len();
        if j == i {
            j = (j + 1) % keys.len();
        }
        Some((keys[i], keys[j]))
    }
}

// ============================================================================
// BRIDGE CREATIVITY ENGINE
// ============================================================================

/// Novel solution generation engine. Combines mutation, inversion,
/// cross-combination, and serendipity to solve optimisation problems
/// that standard approaches cannot crack.
#[derive(Debug)]
#[repr(align(64))]
pub struct BridgeCreativity {
    solutions: BTreeMap<u64, CreativeSolution>,
    vault: StrategyVault,
    mutations_tried: u64,
    inversions_tried: u64,
    combinations_tried: u64,
    novel_count: u64,
    accepted_count: u64,
    tick: u64,
    rng_state: u64,
    novelty_ema: f32,
    risk_ema: f32,
    reward_ema: f32,
}

impl BridgeCreativity {
    /// Create a new creativity engine.
    pub fn new(seed: u64) -> Self {
        Self {
            solutions: BTreeMap::new(),
            vault: StrategyVault::new(),
            mutations_tried: 0,
            inversions_tried: 0,
            combinations_tried: 0,
            novel_count: 0,
            accepted_count: 0,
            tick: 0,
            rng_state: seed ^ 0xCREA_7E00_DEAD,
            novelty_ema: 0.0,
            risk_ema: 0.0,
            reward_ema: 0.0,
        }
    }

    /// Register a known strategy in the vault.
    #[inline]
    pub fn register_strategy(&mut self, name: &str, parameters: &[f32], fitness: f32) -> u64 {
        let sid = fnv1a_hash(name.as_bytes()) ^ (self.tick.wrapping_add(1));
        self.vault.add(KnownStrategy {
            strategy_id: sid,
            name: String::from(name),
            parameters: parameters.to_vec(),
            fitness,
            usage_count: 0,
        });
        sid
    }

    /// Attempt a creative solve: tries mutation, inversion, and combination
    /// in sequence, returning the best novel solution found.
    pub fn creative_solve(&mut self, problem_context: &str) -> Option<CreativeSolution> {
        self.tick += 1;

        let mut candidates = Vec::new();

        // Try random mutation
        if let Some(sol) = self.random_mutation(problem_context) {
            candidates.push(sol);
        }

        // Try strategy inversion
        if let Some(sol) = self.strategy_inversion(problem_context) {
            candidates.push(sol);
        }

        // Try combination search
        if let Some(sol) = self.combination_search(problem_context) {
            candidates.push(sol);
        }

        // Pick the one with highest (reward - risk) * novelty
        candidates.sort_by(|a, b| {
            let score_a = (a.expected_reward - a.risk) * a.novelty;
            let score_b = (b.expected_reward - b.risk) * b.novelty;
            score_b.partial_cmp(&score_a).unwrap_or(core::cmp::Ordering::Equal)
        });

        if let Some(best) = candidates.into_iter().next() {
            let sol_id = best.solution_id;
            if self.solutions.len() >= MAX_SOLUTIONS {
                if let Some((&oldest, _)) = self.solutions.iter().next() {
                    self.solutions.remove(&oldest);
                }
            }
            self.solutions.insert(sol_id, best.clone());
            Some(best)
        } else {
            None
        }
    }

    /// Randomly mutate a known-good strategy.
    pub fn random_mutation(&mut self, context: &str) -> Option<CreativeSolution> {
        self.mutations_tried += 1;
        let keys: Vec<u64> = self.vault.strategies.keys().copied().collect();
        if keys.is_empty() {
            return None;
        }

        let idx = (xorshift64(&mut self.rng_state) as usize) % keys.len();
        let base = self.vault.get(keys[idx])?.clone();

        let mut new_params = base.parameters.clone();
        for p in new_params.iter_mut() {
            let roll = (xorshift64(&mut self.rng_state) % 1000) as f32 / 1000.0;
            if roll < MUTATION_RATE {
                let delta = ((xorshift64(&mut self.rng_state) % 200) as f32 - 100.0) / 100.0;
                *p += delta * 0.3;
            }
        }

        let novelty = self.compute_param_novelty(&new_params);
        let risk = (MUTATION_RATE + novelty * 0.3).min(RISK_CEILING);
        let reward = base.fitness * (1.0 + novelty * 0.2);

        let sol_id = fnv1a_hash(context.as_bytes()) ^ xorshift64(&mut self.rng_state);

        self.update_emas(novelty, risk, reward);

        Some(CreativeSolution {
            solution_id: sol_id,
            approach: String::from(context),
            method: CreativeMethod::RandomMutation,
            novelty,
            risk,
            risk_level: classify_risk(risk),
            expected_reward: reward,
            parent_strategies: alloc::vec![base.strategy_id],
            tick: self.tick,
            accepted: false,
        })
    }

    /// Invert a strategy's parameters to explore the opposite design space.
    pub fn strategy_inversion(&mut self, context: &str) -> Option<CreativeSolution> {
        self.inversions_tried += 1;
        let keys: Vec<u64> = self.vault.strategies.keys().copied().collect();
        if keys.is_empty() {
            return None;
        }

        let idx = (xorshift64(&mut self.rng_state) as usize) % keys.len();
        let base = self.vault.get(keys[idx])?.clone();

        let inverted: Vec<f32> = base.parameters.iter().map(|p| -(*p)).collect();

        let novelty = (self.compute_param_novelty(&inverted) + INVERSION_BONUS).min(1.0);
        let risk = (0.4 + novelty * 0.3).min(RISK_CEILING);
        let reward = base.fitness * (0.5 + novelty * 0.8);

        let sol_id = fnv1a_hash(context.as_bytes()) ^ xorshift64(&mut self.rng_state);

        self.update_emas(novelty, risk, reward);

        Some(CreativeSolution {
            solution_id: sol_id,
            approach: String::from(context),
            method: CreativeMethod::StrategyInversion,
            novelty,
            risk,
            risk_level: classify_risk(risk),
            expected_reward: reward,
            parent_strategies: alloc::vec![base.strategy_id],
            tick: self.tick,
            accepted: false,
        })
    }

    /// Search for novel combinations of two or more strategies.
    pub fn combination_search(&mut self, context: &str) -> Option<CreativeSolution> {
        self.combinations_tried += 1;

        let (id_a, id_b) = self.vault.random_pair(&mut self.rng_state)?;
        let strat_a = self.vault.get(id_a)?.clone();
        let strat_b = self.vault.get(id_b)?.clone();

        let max_len = strat_a.parameters.len().max(strat_b.parameters.len());
        let mut combined = Vec::with_capacity(max_len.min(MAX_COMBINATION_DEPTH * 8));
        for i in 0..max_len {
            let a = strat_a.parameters.get(i).copied().unwrap_or(0.0);
            let b = strat_b.parameters.get(i).copied().unwrap_or(0.0);
            let weight = (xorshift64(&mut self.rng_state) % 100) as f32 / 100.0;
            combined.push(a * weight + b * (1.0 - weight));
        }

        let novelty = self.compute_param_novelty(&combined);
        let avg_fitness = (strat_a.fitness + strat_b.fitness) / 2.0;
        let risk = (0.25 + novelty * 0.25).min(RISK_CEILING);
        let reward = avg_fitness * (1.0 + novelty * 0.3);

        let sol_id = fnv1a_hash(context.as_bytes()) ^ xorshift64(&mut self.rng_state);

        self.update_emas(novelty, risk, reward);

        Some(CreativeSolution {
            solution_id: sol_id,
            approach: String::from(context),
            method: CreativeMethod::CrossCombination,
            novelty,
            risk,
            risk_level: classify_risk(risk),
            expected_reward: reward,
            parent_strategies: alloc::vec![id_a, id_b],
            tick: self.tick,
            accepted: false,
        })
    }

    /// Compute a creativity score: weighted mix of novelty, diversity, and reward.
    pub fn creativity_score(&self) -> f32 {
        let diversity = if self.solutions.is_empty() {
            0.0
        } else {
            let mut method_counts: BTreeMap<u8, u64> = BTreeMap::new();
            for (_, sol) in &self.solutions {
                *method_counts.entry(sol.method as u8).or_insert(0) += 1;
            }
            method_counts.len() as f32 / 8.0
        };

        0.40 * self.novelty_ema + 0.30 * diversity + 0.30 * self.reward_ema.min(1.0)
    }

    /// Assess novelty of a specific solution.
    pub fn novelty_assessment(&self, solution_id: u64) -> Option<NoveltyReport> {
        let sol = self.solutions.get(&solution_id)?;
        let mut similar = 0usize;
        let mut min_distance = f32::MAX;

        for (_, other) in &self.solutions {
            if other.solution_id == solution_id {
                continue;
            }
            let dist = (sol.novelty - other.novelty).abs()
                + (sol.risk - other.risk).abs()
                + (sol.expected_reward - other.expected_reward).abs();
            let normalised = dist / 3.0;
            if normalised < 0.15 {
                similar += 1;
            }
            if normalised < min_distance {
                min_distance = normalised;
            }
        }

        Some(NoveltyReport {
            solution_id,
            novelty_score: sol.novelty,
            distance_from_known: if min_distance == f32::MAX { 1.0 } else { min_distance },
            similar_solutions: similar,
            is_novel: sol.novelty >= NOVELTY_THRESHOLD,
        })
    }

    /// Accept a solution, marking it as proven.
    pub fn accept_solution(&mut self, solution_id: u64) -> bool {
        if let Some(sol) = self.solutions.get_mut(&solution_id) {
            if !sol.accepted {
                sol.accepted = true;
                self.accepted_count += 1;
                if sol.novelty >= NOVELTY_THRESHOLD {
                    self.novel_count += 1;
                }
                return true;
            }
        }
        false
    }

    /// Aggregate statistics.
    pub fn stats(&self) -> CreativityStats {
        let n = self.solutions.len().max(1) as f32;
        let acceptance_rate = if self.solutions.is_empty() {
            0.0
        } else {
            self.accepted_count as f32 / self.solutions.len() as f32
        };

        CreativityStats {
            total_solutions: self.solutions.len() as u64,
            novel_solutions: self.novel_count,
            mutations_tried: self.mutations_tried,
            inversions_tried: self.inversions_tried,
            combinations_tried: self.combinations_tried,
            avg_novelty: self.novelty_ema,
            avg_risk: self.risk_ema,
            avg_reward: self.reward_ema,
            acceptance_rate,
            creativity_ema: self.creativity_score(),
        }
    }

    /// Number of solutions stored.
    #[inline(always)]
    pub fn solution_count(&self) -> usize {
        self.solutions.len()
    }

    /// Current tick.
    #[inline(always)]
    pub fn tick(&self) -> u64 {
        self.tick
    }

    // --- private helpers ---

    fn compute_param_novelty(&self, params: &[f32]) -> f32 {
        if self.vault.strategies.is_empty() {
            return 1.0;
        }
        let mut total_dist = 0.0_f32;
        let mut count = 0u64;
        for (_, strat) in &self.vault.strategies {
            let dist = param_distance(params, &strat.parameters);
            total_dist += dist;
            count += 1;
        }
        let avg_dist = if count > 0 { total_dist / count as f32 } else { 1.0 };
        (avg_dist / 2.0).min(1.0)
    }

    #[inline]
    fn update_emas(&mut self, novelty: f32, risk: f32, reward: f32) {
        self.novelty_ema = EMA_ALPHA * novelty + (1.0 - EMA_ALPHA) * self.novelty_ema;
        self.risk_ema = EMA_ALPHA * risk + (1.0 - EMA_ALPHA) * self.risk_ema;
        self.reward_ema = EMA_ALPHA * reward + (1.0 - EMA_ALPHA) * self.reward_ema;
    }
}

// ============================================================================
// FREE FUNCTIONS
// ============================================================================

fn param_distance(a: &[f32], b: &[f32]) -> f32 {
    let max_len = a.len().max(b.len());
    let mut sum_sq = 0.0_f32;
    for i in 0..max_len {
        let va = a.get(i).copied().unwrap_or(0.0);
        let vb = b.get(i).copied().unwrap_or(0.0);
        let diff = va - vb;
        sum_sq += diff * diff;
    }
    sqrt_approx(sum_sq)
}

fn sqrt_approx(x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    let mut guess = x;
    for _ in 0..8 {
        guess = 0.5 * (guess + x / guess);
    }
    guess
}

fn classify_risk(r: f32) -> RiskLevel {
    if r < 0.15 {
        RiskLevel::Negligible
    } else if r < 0.35 {
        RiskLevel::Low
    } else if r < 0.55 {
        RiskLevel::Moderate
    } else if r < 0.75 {
        RiskLevel::High
    } else {
        RiskLevel::Extreme
    }
}
