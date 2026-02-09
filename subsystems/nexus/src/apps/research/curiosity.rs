// SPDX-License-Identifier: GPL-2.0
//! # Apps Curiosity Engine — Proactive Exploration of App Behavior Space
//!
//! The engine is curious about unknown app patterns, tries novel classification
//! approaches, and explores untested allocation strategies. Uses UCB1-style
//! exploration-exploitation balancing to decide where to invest its limited
//! exploration budget. High-curiosity areas are those where the model is most
//! uncertain or where surprising results have previously been found.
//!
//! The engine that never stops asking "what if?" about applications.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_FRONTIERS: usize = 128;
const MAX_STRATEGIES: usize = 64;
const MAX_CLASSIFICATIONS: usize = 64;
const UCB_C: f32 = 1.414;
const EMA_ALPHA: f32 = 0.12;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const DEFAULT_BUDGET: u32 = 100;
const MIN_EXPLORATION_RATIO: f32 = 0.10;
const MAX_EXPLORATION_RATIO: f32 = 0.70;
const NOVELTY_THRESHOLD: f32 = 0.60;
const REWARD_DECAY: f32 = 0.95;
const SURPRISE_BOOST: f32 = 2.0;
const MAX_HISTORY: usize = 1024;

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

fn sqrt_approx(x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    let mut guess = x * 0.5;
    for _ in 0..12 {
        guess = 0.5 * (guess + x / guess);
    }
    guess
}

fn ln_approx(x: f32) -> f32 {
    if x <= 0.0 {
        return -10.0;
    }
    let mut val = x;
    let mut count = 0.0f32;
    while val > 2.0 {
        val *= 0.5;
        count += 0.6931;
    }
    while val < 0.5 {
        val *= 2.0;
        count -= 0.6931;
    }
    let y = val - 1.0;
    count + y - 0.5 * y * y + 0.333 * y * y * y
}

// ============================================================================
// TYPES
// ============================================================================

/// An exploration frontier — an area of app behavior space to investigate.
#[derive(Clone)]
pub struct ExplorationFrontier {
    pub frontier_id: u64,
    pub label: String,
    pub uncertainty: f32,
    pub surprise_history: f32,
    pub visits: u32,
    pub total_reward: f32,
    pub ucb_score: f32,
    pub last_visit_tick: u64,
}

/// A novel classification approach to try.
#[derive(Clone)]
pub struct NovelClassification {
    pub class_id: u64,
    pub description: String,
    pub novelty_score: f32,
    pub trial_count: u32,
    pub success_rate: f32,
    pub ema_reward: f32,
}

/// An untested allocation strategy.
#[derive(Clone)]
pub struct UntestedStrategy {
    pub strategy_id: u64,
    pub description: String,
    pub estimated_novelty: f32,
    pub risk_score: f32,
    pub trials: u32,
    pub cumulative_reward: f32,
}

/// Budget tracking for exploration.
#[derive(Clone)]
pub struct ExplorationBudget {
    pub total_budget: u32,
    pub spent: u32,
    pub remaining: u32,
    pub efficiency: f32,
    pub roi: f32,
}

/// Reward record for a curiosity exploration.
#[derive(Clone)]
pub struct CuriosityReward {
    pub exploration_id: u64,
    pub raw_reward: f32,
    pub surprise_bonus: f32,
    pub total_reward: f32,
    pub tick: u64,
}

/// Engine stats for the curiosity engine.
#[derive(Clone)]
pub struct CuriosityStats {
    pub total_explorations: u64,
    pub frontiers_discovered: u64,
    pub novel_classifications_tried: u64,
    pub strategies_tested: u64,
    pub ema_reward: f32,
    pub ema_novelty: f32,
    pub ema_surprise: f32,
    pub budget_utilization: f32,
}

// ============================================================================
// ENGINE
// ============================================================================

/// Proactive exploration engine for app behavior space.
pub struct AppsCuriosityEngine {
    frontiers: BTreeMap<u64, ExplorationFrontier>,
    classifications: BTreeMap<u64, NovelClassification>,
    strategies: BTreeMap<u64, UntestedStrategy>,
    reward_history: Vec<CuriosityReward>,
    budget_total: u32,
    budget_spent: u32,
    stats: CuriosityStats,
    rng_state: u64,
    tick: u64,
    total_visits: u32,
}

impl AppsCuriosityEngine {
    /// Create a new curiosity engine with a given RNG seed.
    pub fn new(seed: u64) -> Self {
        Self {
            frontiers: BTreeMap::new(),
            classifications: BTreeMap::new(),
            strategies: BTreeMap::new(),
            reward_history: Vec::new(),
            budget_total: DEFAULT_BUDGET,
            budget_spent: 0,
            stats: CuriosityStats {
                total_explorations: 0,
                frontiers_discovered: 0,
                novel_classifications_tried: 0,
                strategies_tested: 0,
                ema_reward: 0.0,
                ema_novelty: 0.0,
                ema_surprise: 0.0,
                budget_utilization: 0.0,
            },
            rng_state: seed ^ 0x3d7e92ab41cf085e,
            tick: 0,
            total_visits: 0,
        }
    }

    // ── Primary API ────────────────────────────────────────────────────

    /// Explore the next most promising frontier using UCB1 selection.
    pub fn curiosity_explore(&mut self) -> Option<ExplorationFrontier> {
        self.tick += 1;
        if self.frontiers.is_empty() || self.budget_spent >= self.budget_total {
            return None;
        }
        self.stats.total_explorations += 1;

        // UCB1: select frontier with highest UCB score
        let total_n = self.total_visits.max(1) as f32;
        let mut best_id = 0u64;
        let mut best_ucb = f32::MIN;

        for (id, frontier) in self.frontiers.iter() {
            let avg_reward = if frontier.visits > 0 {
                frontier.total_reward / frontier.visits as f32
            } else {
                f32::MAX // unexplored → always selected
            };
            let exploration_bonus = UCB_C * sqrt_approx(ln_approx(total_n) / (frontier.visits.max(1) as f32));
            let surprise_factor = 1.0 + frontier.surprise_history * SURPRISE_BOOST;
            let ucb = avg_reward * surprise_factor + exploration_bonus;

            if ucb > best_ucb {
                best_ucb = ucb;
                best_id = *id;
            }
        }

        if let Some(frontier) = self.frontiers.get_mut(&best_id) {
            frontier.visits += 1;
            frontier.ucb_score = best_ucb;
            frontier.last_visit_tick = self.tick;
            self.total_visits += 1;
            self.budget_spent += 1;

            let util = self.budget_spent as f32 / self.budget_total.max(1) as f32;
            self.stats.budget_utilization = EMA_ALPHA * util + (1.0 - EMA_ALPHA) * self.stats.budget_utilization;

            Some(frontier.clone())
        } else {
            None
        }
    }

    /// Try a novel classification approach for the given app pattern.
    pub fn novel_classification(&mut self, description: &str) -> NovelClassification {
        self.tick += 1;
        let id = fnv1a_hash(description.as_bytes()) ^ self.tick;
        self.stats.novel_classifications_tried += 1;

        let novelty = 0.5 + xorshift_f32(&mut self.rng_state) * 0.5;
        let cls = NovelClassification {
            class_id: id,
            description: String::from(description),
            novelty_score: novelty,
            trial_count: 1,
            success_rate: 0.0,
            ema_reward: 0.0,
        };

        if self.classifications.len() >= MAX_CLASSIFICATIONS {
            // Evict lowest-novelty classification
            let mut min_id = 0u64;
            let mut min_score = f32::MAX;
            for (cid, c) in self.classifications.iter() {
                if c.novelty_score < min_score {
                    min_score = c.novelty_score;
                    min_id = *cid;
                }
            }
            self.classifications.remove(&min_id);
        }
        self.classifications.insert(id, cls.clone());

        self.stats.ema_novelty = EMA_ALPHA * novelty + (1.0 - EMA_ALPHA) * self.stats.ema_novelty;
        cls
    }

    /// Test an untested allocation strategy.
    pub fn untested_strategy(&mut self, description: &str, risk: f32) -> UntestedStrategy {
        self.tick += 1;
        let id = fnv1a_hash(description.as_bytes()) ^ self.tick;
        self.stats.strategies_tested += 1;

        let est_novelty = xorshift_f32(&mut self.rng_state) * 0.8 + 0.2;
        let strategy = UntestedStrategy {
            strategy_id: id,
            description: String::from(description),
            estimated_novelty: est_novelty,
            risk_score: risk.min(1.0).max(0.0),
            trials: 0,
            cumulative_reward: 0.0,
        };

        if self.strategies.len() >= MAX_STRATEGIES {
            let mut min_id = 0u64;
            let mut min_nov = f32::MAX;
            for (sid, s) in self.strategies.iter() {
                if s.estimated_novelty < min_nov {
                    min_nov = s.estimated_novelty;
                    min_id = *sid;
                }
            }
            self.strategies.remove(&min_id);
        }
        self.strategies.insert(id, strategy.clone());
        strategy
    }

    /// Report the current exploration budget status.
    pub fn exploration_budget(&self) -> ExplorationBudget {
        let remaining = self.budget_total.saturating_sub(self.budget_spent);
        let efficiency = if self.budget_spent > 0 {
            self.stats.ema_reward / self.budget_spent as f32
        } else {
            0.0
        };
        let total_reward: f32 = self.reward_history.iter().map(|r| r.total_reward).sum();
        let roi = if self.budget_spent > 0 {
            total_reward / self.budget_spent as f32
        } else {
            0.0
        };

        ExplorationBudget {
            total_budget: self.budget_total,
            spent: self.budget_spent,
            remaining,
            efficiency,
            roi,
        }
    }

    /// Record a curiosity reward from a completed exploration.
    pub fn curiosity_reward(&mut self, frontier_id: u64, raw_reward: f32, surprise: f32) -> CuriosityReward {
        self.tick += 1;
        let surprise_bonus = surprise * SURPRISE_BOOST;
        let total = raw_reward + surprise_bonus;

        // Update frontier reward tracking
        if let Some(frontier) = self.frontiers.get_mut(&frontier_id) {
            frontier.total_reward += total;
            frontier.surprise_history =
                REWARD_DECAY * frontier.surprise_history + (1.0 - REWARD_DECAY) * surprise;
        }

        self.stats.ema_reward = EMA_ALPHA * total + (1.0 - EMA_ALPHA) * self.stats.ema_reward;
        self.stats.ema_surprise = EMA_ALPHA * surprise + (1.0 - EMA_ALPHA) * self.stats.ema_surprise;

        let record = CuriosityReward {
            exploration_id: frontier_id,
            raw_reward,
            surprise_bonus,
            total_reward: total,
            tick: self.tick,
        };

        if self.reward_history.len() >= MAX_HISTORY {
            self.reward_history.remove(0);
        }
        self.reward_history.push(record.clone());
        record
    }

    /// List the top frontiers ranked by UCB score.
    pub fn frontier_apps(&mut self, max_count: usize) -> Vec<ExplorationFrontier> {
        let total_n = self.total_visits.max(1) as f32;

        // Recompute UCB for all frontiers
        for frontier in self.frontiers.values_mut() {
            let avg_reward = if frontier.visits > 0 {
                frontier.total_reward / frontier.visits as f32
            } else {
                0.5 // prior for unseen
            };
            let bonus = UCB_C * sqrt_approx(ln_approx(total_n) / frontier.visits.max(1) as f32);
            let surprise_factor = 1.0 + frontier.surprise_history * SURPRISE_BOOST;
            frontier.ucb_score = avg_reward * surprise_factor + bonus;
        }

        let mut ranked: Vec<ExplorationFrontier> = self.frontiers.values().cloned().collect();
        // Sort descending by ucb_score (manual since no std sort_by_float)
        for i in 0..ranked.len() {
            for j in (i + 1)..ranked.len() {
                if ranked[j].ucb_score > ranked[i].ucb_score {
                    ranked.swap(i, j);
                }
            }
        }
        ranked.truncate(max_count.min(MAX_FRONTIERS));
        ranked
    }

    /// Register a new exploration frontier.
    pub fn register_frontier(&mut self, label: &str, uncertainty: f32) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(label.as_bytes()) ^ self.tick;
        self.stats.frontiers_discovered += 1;

        let frontier = ExplorationFrontier {
            frontier_id: id,
            label: String::from(label),
            uncertainty: uncertainty.min(1.0).max(0.0),
            surprise_history: 0.0,
            visits: 0,
            total_reward: 0.0,
            ucb_score: 0.0,
            last_visit_tick: 0,
        };

        if self.frontiers.len() >= MAX_FRONTIERS {
            // Evict least-visited frontier
            let mut min_id = 0u64;
            let mut min_visits = u32::MAX;
            for (fid, f) in self.frontiers.iter() {
                if f.visits < min_visits {
                    min_visits = f.visits;
                    min_id = *fid;
                }
            }
            self.frontiers.remove(&min_id);
        }
        self.frontiers.insert(id, frontier);
        id
    }

    /// Set the exploration budget.
    pub fn set_budget(&mut self, total: u32) {
        self.budget_total = total;
    }

    /// Reset the budget for a new exploration cycle.
    pub fn reset_budget(&mut self) {
        self.budget_spent = 0;
    }

    /// Return engine stats.
    pub fn stats(&self) -> &CuriosityStats {
        &self.stats
    }
}
