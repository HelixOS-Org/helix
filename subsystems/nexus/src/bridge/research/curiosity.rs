// SPDX-License-Identifier: GPL-2.0
//! # Bridge Curiosity Engine — Proactive Exploration of Syscall Space
//!
//! The bridge is *curious*: it deliberately ventures into untested syscall
//! routing paths seeking novel optimizations. Using a UCB1-inspired
//! exploration-vs-exploitation balance, the engine maintains a frontier map
//! of the optimization space, selects promising unexplored regions, assigns
//! novelty scores, and tracks curiosity satisfaction over time. The goal is
//! to prevent the bridge from getting stuck in local optima by continuously
//! injecting controlled experiments into less-explored dimensions.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_TARGETS: usize = 512;
const MAX_DIMENSIONS: usize = 64;
const MAX_FRONTIER_CELLS: usize = 1024;
const MAX_HISTORY: usize = 2048;
const UCB_C: f32 = 1.414; // sqrt(2) exploration constant
const NOVELTY_DECAY: f32 = 0.98;
const EMA_ALPHA: f32 = 0.12;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const MIN_EXPLORATION_RATIO: f32 = 0.15;
const MAX_EXPLORATION_RATIO: f32 = 0.85;
const REWARD_SCALE: f32 = 100.0;
const RISK_THRESHOLD: f32 = 0.7;
const SATISFACTION_THRESHOLD: f32 = 0.6;
const BOREDOM_THRESHOLD: u64 = 50;

// ============================================================================
// HELPERS
// ============================================================================

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

fn sqrt_approx(v: f32) -> f32 {
    if v <= 0.0 {
        return 0.0;
    }
    let mut g = v * 0.5;
    for _ in 0..6 {
        g = 0.5 * (g + v / g);
    }
    g
}

fn ln_approx(x: f32) -> f32 {
    if x <= 0.0 {
        return -10.0;
    }
    // ln(x) ≈ series around 1: use (x-1)/(x+1) series
    let y = (x - 1.0) / (x + 1.0);
    let y2 = y * y;
    2.0 * y * (1.0 + y2 / 3.0 + y2 * y2 / 5.0 + y2 * y2 * y2 / 7.0)
}

// ============================================================================
// TYPES
// ============================================================================

/// A dimension of the syscall optimization space.
#[derive(Clone)]
struct Dimension {
    id: u64,
    name: String,
    range_low: f32,
    range_high: f32,
    granularity: f32,
    visit_count: u64,
    best_reward: f32,
    total_reward: f32,
    last_visit_tick: u64,
}

/// A target for curiosity-driven exploration.
#[derive(Clone)]
pub struct CuriosityTarget {
    pub dimension: String,
    pub unexplored_region: (f32, f32),
    pub potential_reward: f32,
    pub risk: f32,
    pub novelty: f32,
    pub ucb_score: f32,
    pub times_explored: u64,
}

/// A cell in the frontier map.
#[derive(Clone)]
struct FrontierCell {
    dim_id: u64,
    region_low: f32,
    region_high: f32,
    visit_count: u64,
    total_reward: f32,
    avg_reward_ema: f32,
    novelty_score: f32,
    last_visit: u64,
}

/// Record of an exploration attempt.
#[derive(Clone)]
struct ExplorationRecord {
    target_dim: u64,
    region: (f32, f32),
    reward: f32,
    novelty: f32,
    tick: u64,
    was_exploit: bool,
}

/// Curiosity engine statistics.
#[derive(Clone)]
#[repr(align(64))]
pub struct CuriosityStats {
    pub total_explorations: u64,
    pub exploit_count: u64,
    pub explore_count: u64,
    pub avg_reward_ema: f32,
    pub avg_novelty_ema: f32,
    pub curiosity_satisfaction: f32,
    pub frontier_coverage: f32,
    pub breakthroughs_from_curiosity: u64,
    pub high_risk_attempts: u64,
    pub ticks_since_discovery: u64,
}

// ============================================================================
// BRIDGE CURIOSITY ENGINE
// ============================================================================

/// Proactive exploration engine for syscall optimization space.
#[repr(align(64))]
pub struct BridgeCuriosityEngine {
    dimensions: BTreeMap<u64, Dimension>,
    frontier: BTreeMap<u64, FrontierCell>,
    history: Vec<ExplorationRecord>,
    stats: CuriosityStats,
    rng_state: u64,
    tick: u64,
    total_global_visits: u64,
    boredom_counter: u64,
}

impl BridgeCuriosityEngine {
    /// Create a new curiosity engine.
    pub fn new(seed: u64) -> Self {
        Self {
            dimensions: BTreeMap::new(),
            frontier: BTreeMap::new(),
            history: Vec::new(),
            stats: CuriosityStats {
                total_explorations: 0,
                exploit_count: 0,
                explore_count: 0,
                avg_reward_ema: 0.0,
                avg_novelty_ema: 0.0,
                curiosity_satisfaction: 0.0,
                frontier_coverage: 0.0,
                breakthroughs_from_curiosity: 0,
                high_risk_attempts: 0,
                ticks_since_discovery: 0,
            },
            rng_state: seed ^ 0xC0410517AE0610E,
            tick: 0,
            total_global_visits: 0,
            boredom_counter: 0,
        }
    }

    /// Register an optimization dimension.
    pub fn register_dimension(&mut self, name: &str, low: f32, high: f32, granularity: f32) {
        if self.dimensions.len() >= MAX_DIMENSIONS {
            return;
        }
        let id = fnv1a_hash(name.as_bytes());
        self.dimensions.insert(
            id,
            Dimension {
                id,
                name: String::from(name),
                range_low: low,
                range_high: high,
                granularity: if granularity > 0.0 { granularity } else { 1.0 },
                visit_count: 0,
                best_reward: 0.0,
                total_reward: 0.0,
                last_visit_tick: 0,
            },
        );
        // Initialize frontier cells for this dimension
        self.init_frontier_for(id, low, high, granularity);
    }

    /// Curiosity-driven exploration: pick a target and explore it.
    pub fn curiosity_driven_explore(&mut self) -> CuriosityTarget {
        self.tick += 1;
        self.boredom_counter += 1;
        self.stats.total_explorations += 1;
        self.total_global_visits += 1;

        // Decide explore vs exploit
        let should_explore = self.exploration_vs_exploitation();

        let target = if should_explore {
            self.stats.explore_count += 1;
            self.select_explore_target()
        } else {
            self.stats.exploit_count += 1;
            self.select_exploit_target()
        };

        // Simulate reward for this exploration
        let reward = self.simulate_reward(&target);
        let novelty = self.novelty_score_internal(&target);

        // Record
        self.record_exploration(&target, reward, novelty, !should_explore);

        // Update frontier
        self.update_frontier(&target, reward, novelty);

        // Update stats
        self.stats.avg_reward_ema =
            self.stats.avg_reward_ema * (1.0 - EMA_ALPHA) + reward * EMA_ALPHA;
        self.stats.avg_novelty_ema =
            self.stats.avg_novelty_ema * (1.0 - EMA_ALPHA) + novelty * EMA_ALPHA;
        self.stats.curiosity_satisfaction = self.curiosity_satisfaction();
        self.stats.frontier_coverage = self.compute_coverage();

        if reward > self.stats.avg_reward_ema * 1.5 {
            self.stats.breakthroughs_from_curiosity += 1;
            self.boredom_counter = 0;
            self.stats.ticks_since_discovery = 0;
        } else {
            self.stats.ticks_since_discovery += 1;
        }

        target
    }

    /// Select the best experiment from current curiosity targets.
    pub fn select_experiment(&mut self) -> CuriosityTarget {
        // Find highest UCB score across frontier
        let mut best_score: f32 = -1.0;
        let mut best_cell_id: u64 = 0;

        for (&cid, cell) in self.frontier.iter() {
            let ucb = self.ucb1_score(cell.total_reward, cell.visit_count, self.total_global_visits);
            let combined = ucb + cell.novelty_score * 0.5;
            if combined > best_score {
                best_score = combined;
                best_cell_id = cid;
            }
        }

        self.build_target_from_cell(best_cell_id, best_score)
    }

    /// Compute novelty score for a given dimension and region.
    #[inline]
    pub fn novelty_score(&self, dim_name: &str, region_low: f32, region_high: f32) -> f32 {
        let dim_id = fnv1a_hash(dim_name.as_bytes());
        let cell_key = self.cell_key(dim_id, region_low);
        match self.frontier.get(&cell_key) {
            Some(cell) => cell.novelty_score,
            None => 1.0, // completely unknown = maximum novelty
        }
    }

    /// Decide exploration vs exploitation ratio.
    pub fn exploration_vs_exploitation(&mut self) -> bool {
        // UCB1-inspired: increase exploration when bored
        let base_explore_prob = MIN_EXPLORATION_RATIO;
        let boredom_bonus = if self.boredom_counter > BOREDOM_THRESHOLD {
            let excess = (self.boredom_counter - BOREDOM_THRESHOLD) as f32;
            (excess * 0.01).min(MAX_EXPLORATION_RATIO - MIN_EXPLORATION_RATIO)
        } else {
            0.0
        };
        let novelty_bonus = if self.stats.avg_novelty_ema > 0.5 {
            0.1
        } else {
            0.0
        };
        let explore_prob = (base_explore_prob + boredom_bonus + novelty_bonus)
            .min(MAX_EXPLORATION_RATIO);

        let r = xorshift_f32(&mut self.rng_state);
        r < explore_prob
    }

    /// Measure how satisfied the engine's curiosity is.
    pub fn curiosity_satisfaction(&self) -> f32 {
        let coverage = self.compute_coverage();
        let recent_novelty = self.stats.avg_novelty_ema;
        let discovery_recency = if self.stats.ticks_since_discovery < 10 {
            1.0
        } else if self.stats.ticks_since_discovery < 50 {
            0.5
        } else {
            0.2
        };
        (coverage * 0.3 + (1.0 - recent_novelty) * 0.4 + discovery_recency * 0.3)
            .min(1.0)
            .max(0.0)
    }

    /// Get the complete frontier map.
    pub fn frontier_map(&self) -> Vec<FrontierEntry> {
        let mut entries = Vec::new();
        for cell in self.frontier.values() {
            let dim_name = self
                .dimensions
                .get(&cell.dim_id)
                .map(|d| d.name.clone())
                .unwrap_or_else(|| String::from("unknown"));
            entries.push(FrontierEntry {
                dimension: dim_name,
                region_low: cell.region_low,
                region_high: cell.region_high,
                visit_count: cell.visit_count,
                avg_reward: cell.avg_reward_ema,
                novelty: cell.novelty_score,
            });
        }
        entries
    }

    /// Current stats.
    #[inline(always)]
    pub fn stats(&self) -> &CuriosityStats {
        &self.stats
    }

    /// Number of frontier cells.
    #[inline(always)]
    pub fn frontier_size(&self) -> usize {
        self.frontier.len()
    }

    /// Number of registered dimensions.
    #[inline(always)]
    pub fn dimension_count(&self) -> usize {
        self.dimensions.len()
    }

    // ========================================================================
    // INTERNAL HELPERS
    // ========================================================================

    fn init_frontier_for(&mut self, dim_id: u64, low: f32, high: f32, gran: f32) {
        let step = if gran > 0.0 { gran } else { (high - low) / 10.0 };
        let mut pos = low;
        while pos < high && self.frontier.len() < MAX_FRONTIER_CELLS {
            let cell_key = self.cell_key(dim_id, pos);
            self.frontier.insert(
                cell_key,
                FrontierCell {
                    dim_id,
                    region_low: pos,
                    region_high: (pos + step).min(high),
                    visit_count: 0,
                    total_reward: 0.0,
                    avg_reward_ema: 0.0,
                    novelty_score: 1.0,
                    last_visit: 0,
                },
            );
            pos += step;
        }
    }

    fn cell_key(&self, dim_id: u64, region_low: f32) -> u64 {
        let mut buf = [0u8; 12];
        let db = dim_id.to_le_bytes();
        let rb = region_low.to_bits().to_le_bytes();
        buf[..8].copy_from_slice(&db);
        buf[8..12].copy_from_slice(&rb);
        fnv1a_hash(&buf)
    }

    fn ucb1_score(&self, total_reward: f32, visits: u64, global_visits: u64) -> f32 {
        if visits == 0 {
            return f32::MAX;
        }
        let avg = total_reward / visits as f32;
        let exploration = UCB_C
            * sqrt_approx(ln_approx(global_visits.max(1) as f32) / visits as f32);
        avg + exploration
    }

    fn select_explore_target(&mut self) -> CuriosityTarget {
        // Pick least-visited frontier cell
        let mut min_visits: u64 = u64::MAX;
        let mut target_id: u64 = 0;
        for (&cid, cell) in self.frontier.iter() {
            if cell.visit_count < min_visits {
                min_visits = cell.visit_count;
                target_id = cid;
            }
        }
        self.build_target_from_cell(target_id, 0.0)
    }

    fn select_exploit_target(&mut self) -> CuriosityTarget {
        // Pick highest-reward frontier cell
        let mut max_reward: f32 = -1.0;
        let mut target_id: u64 = 0;
        for (&cid, cell) in self.frontier.iter() {
            if cell.avg_reward_ema > max_reward {
                max_reward = cell.avg_reward_ema;
                target_id = cid;
            }
        }
        self.build_target_from_cell(target_id, max_reward)
    }

    fn build_target_from_cell(&self, cell_id: u64, score: f32) -> CuriosityTarget {
        match self.frontier.get(&cell_id) {
            Some(cell) => {
                let dim_name = self
                    .dimensions
                    .get(&cell.dim_id)
                    .map(|d| d.name.clone())
                    .unwrap_or_else(|| String::from("unknown"));
                let risk = if cell.visit_count == 0 {
                    0.8
                } else {
                    (1.0 / (cell.visit_count as f32 + 1.0)).min(RISK_THRESHOLD)
                };
                CuriosityTarget {
                    dimension: dim_name,
                    unexplored_region: (cell.region_low, cell.region_high),
                    potential_reward: cell.avg_reward_ema * 1.2,
                    risk,
                    novelty: cell.novelty_score,
                    ucb_score: score,
                    times_explored: cell.visit_count,
                }
            }
            None => CuriosityTarget {
                dimension: String::from("none"),
                unexplored_region: (0.0, 0.0),
                potential_reward: 0.0,
                risk: 1.0,
                novelty: 0.0,
                ucb_score: 0.0,
                times_explored: 0,
            },
        }
    }

    fn simulate_reward(&mut self, target: &CuriosityTarget) -> f32 {
        let base = xorshift_f32(&mut self.rng_state) * REWARD_SCALE;
        let novelty_bonus = target.novelty * 20.0;
        let risk_penalty = target.risk * 15.0;
        ((base + novelty_bonus - risk_penalty) / REWARD_SCALE).max(0.0).min(1.0)
    }

    fn novelty_score_internal(&self, target: &CuriosityTarget) -> f32 {
        let dim_id = fnv1a_hash(target.dimension.as_bytes());
        let cell_key = self.cell_key(dim_id, target.unexplored_region.0);
        match self.frontier.get(&cell_key) {
            Some(cell) => cell.novelty_score,
            None => 1.0,
        }
    }

    fn record_exploration(
        &mut self,
        target: &CuriosityTarget,
        reward: f32,
        novelty: f32,
        was_exploit: bool,
    ) {
        if target.risk > RISK_THRESHOLD {
            self.stats.high_risk_attempts += 1;
        }
        let dim_id = fnv1a_hash(target.dimension.as_bytes());
        if self.history.len() < MAX_HISTORY {
            self.history.push(ExplorationRecord {
                target_dim: dim_id,
                region: target.unexplored_region,
                reward,
                novelty,
                tick: self.tick,
                was_exploit,
            });
        }
    }

    fn update_frontier(&mut self, target: &CuriosityTarget, reward: f32, _novelty: f32) {
        let dim_id = fnv1a_hash(target.dimension.as_bytes());
        let cell_key = self.cell_key(dim_id, target.unexplored_region.0);
        if let Some(cell) = self.frontier.get_mut(&cell_key) {
            cell.visit_count += 1;
            cell.total_reward += reward;
            cell.avg_reward_ema = cell.avg_reward_ema * (1.0 - EMA_ALPHA) + reward * EMA_ALPHA;
            cell.novelty_score *= NOVELTY_DECAY;
            cell.last_visit = self.tick;
        }
        // Update dimension stats
        if let Some(dim) = self.dimensions.get_mut(&dim_id) {
            dim.visit_count += 1;
            dim.total_reward += reward;
            if reward > dim.best_reward {
                dim.best_reward = reward;
            }
            dim.last_visit_tick = self.tick;
        }
    }

    fn compute_coverage(&self) -> f32 {
        if self.frontier.is_empty() {
            return 0.0;
        }
        let visited = self.frontier.values().filter(|c| c.visit_count > 0).count();
        visited as f32 / self.frontier.len() as f32
    }
}

// ============================================================================
// FRONTIER ENTRY (public)
// ============================================================================

/// Public view of a frontier cell.
#[derive(Clone)]
pub struct FrontierEntry {
    pub dimension: String,
    pub region_low: f32,
    pub region_high: f32,
    pub visit_count: u64,
    pub avg_reward: f32,
    pub novelty: f32,
}
