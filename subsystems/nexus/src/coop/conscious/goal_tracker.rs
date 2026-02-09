// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Goal Tracker
//!
//! Goal-directed cooperation optimization. Maintains a hierarchy of goals
//! focused on perfect fairness, zero-waste resource allocation, and optimal
//! contract formation. Tracks progress toward each goal, measures fairness
//! convergence, and monitors resource waste over time.
//!
//! This module gives the cooperation engine *purpose*: it doesn't just
//! cooperate, it works toward declared fairness and efficiency objectives.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.12;
const MAX_GOALS: usize = 64;
const MAX_PROGRESS_HISTORY: usize = 128;
const CONVERGENCE_WINDOW: usize = 16;
const CONVERGENCE_THRESHOLD: f32 = 0.02;
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

// ============================================================================
// GOAL TYPES
// ============================================================================

/// Priority tier for a cooperation goal
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CoopGoalPriority {
    /// Must be achieved — fairness invariants
    Critical   = 4,
    /// Should be achieved — resource efficiency
    High       = 3,
    /// Nice to have — protocol optimization
    Medium     = 2,
    /// Aspirational — long-term cooperation quality
    Low        = 1,
}

/// Current status of a cooperation goal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopGoalStatus {
    Active,
    Achieved,
    Stalled,
    Regressing,
    Abandoned,
}

/// A single cooperation goal
#[derive(Debug, Clone)]
pub struct CoopGoal {
    pub id: u64,
    pub name: String,
    pub priority: CoopGoalPriority,
    pub status: CoopGoalStatus,
    /// Target value (what we're aiming for)
    pub target: f32,
    /// Current value (where we are)
    pub current: f32,
    /// Progress as fraction (0.0 – 1.0)
    pub progress: f32,
    /// EMA-smoothed progress rate per tick
    pub progress_rate: f32,
    /// Tick when goal was set
    pub set_tick: u64,
    /// Historical progress snapshots
    progress_history: Vec<f32>,
    write_idx: usize,
    /// Weight for conflict resolution
    pub weight: f32,
}

/// Resource waste measurement
#[derive(Debug, Clone)]
pub struct WasteRecord {
    pub tick: u64,
    /// Fraction of resources wasted (0.0 – 1.0)
    pub waste_fraction: f32,
    /// Absolute resources wasted (arbitrary units)
    pub wasted_units: f32,
    /// Total resources available
    pub total_units: f32,
    /// Source of waste (hashed identifier)
    pub source_id: u64,
}

/// Contract quality measurement
#[derive(Debug, Clone)]
pub struct ContractQuality {
    pub contract_id: u64,
    /// How well the contract was fulfilled (0.0 – 1.0)
    pub fulfillment: f32,
    /// Fairness of terms (0.0 – 1.0)
    pub fairness: f32,
    /// Duration efficiency: actual vs expected
    pub duration_ratio: f32,
    /// Number of renegotiations required
    pub renegotiations: u32,
}

// ============================================================================
// GOAL TRACKER STATS
// ============================================================================

/// Aggregate goal tracking statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct GoalTrackerStats {
    pub total_goals: usize,
    pub active_goals: usize,
    pub achieved_goals: usize,
    pub stalled_goals: usize,
    pub avg_progress: f32,
    pub avg_progress_rate: f32,
    pub waste_rate: f32,
    pub avg_contract_quality: f32,
    pub convergence_score: f32,
}

// ============================================================================
// COOPERATION GOAL TRACKER
// ============================================================================

/// Maintains a hierarchy of cooperation goals, tracking fairness progress,
/// waste minimization, and contract quality over time.
#[derive(Debug)]
pub struct CoopGoalTracker {
    /// All goals keyed by FNV-1a hash
    goals: BTreeMap<u64, CoopGoal>,
    /// Waste history ring buffer
    waste_history: Vec<WasteRecord>,
    waste_write_idx: usize,
    /// Contract quality records (keyed by contract ID)
    contract_quality: BTreeMap<u64, ContractQuality>,
    /// EMA of waste rate
    waste_rate_ema: f32,
    /// EMA of contract quality
    contract_quality_ema: f32,
    /// Monotonic tick
    tick: u64,
    /// Total goals ever created
    total_created: u64,
    /// Total goals ever achieved
    total_achieved: u64,
}

impl CoopGoalTracker {
    pub fn new() -> Self {
        Self {
            goals: BTreeMap::new(),
            waste_history: Vec::new(),
            waste_write_idx: 0,
            contract_quality: BTreeMap::new(),
            waste_rate_ema: 0.0,
            contract_quality_ema: 0.5,
            tick: 0,
            total_created: 0,
            total_achieved: 0,
        }
    }

    /// Create a new cooperation goal
    pub fn set_goal(
        &mut self,
        name: &str,
        priority: CoopGoalPriority,
        target: f32,
        initial: f32,
        weight: f32,
    ) -> u64 {
        self.tick += 1;
        self.total_created += 1;

        let id = fnv1a_hash(name.as_bytes());
        let progress = if (target - initial).abs() < f32::EPSILON {
            1.0
        } else if target > initial {
            (initial / target).max(0.0).min(1.0)
        } else {
            (1.0 - (initial - target) / initial.max(0.001))
                .max(0.0)
                .min(1.0)
        };

        let goal = CoopGoal {
            id,
            name: String::from(name),
            priority,
            status: CoopGoalStatus::Active,
            target,
            current: initial,
            progress,
            progress_rate: 0.0,
            set_tick: self.tick,
            progress_history: Vec::new(),
            write_idx: 0,
            weight,
        };

        self.goals.insert(id, goal);
        id
    }

    /// Update a goal with a new measurement and compute progress
    fn update_goal(&mut self, name: &str, current: f32) {
        self.tick += 1;
        let id = fnv1a_hash(name.as_bytes());

        if let Some(goal) = self.goals.get_mut(&id) {
            let prev_progress = goal.progress;
            goal.current = current;

            goal.progress = if (goal.target - goal.current).abs() < f32::EPSILON {
                1.0
            } else if goal.target > goal.current {
                (goal.current / goal.target).max(0.0).min(1.0)
            } else {
                (1.0 - (goal.current - goal.target) / goal.current.max(0.001))
                    .max(0.0)
                    .min(1.0)
            };

            let delta = goal.progress - prev_progress;
            goal.progress_rate = EMA_ALPHA * delta + (1.0 - EMA_ALPHA) * goal.progress_rate;

            // Update status
            if goal.progress >= 0.99 {
                if goal.status != CoopGoalStatus::Achieved {
                    self.total_achieved += 1;
                }
                goal.status = CoopGoalStatus::Achieved;
            } else if goal.progress_rate < -0.01 {
                goal.status = CoopGoalStatus::Regressing;
            } else if goal.progress_rate.abs() < 0.001 && goal.progress < 0.99 {
                goal.status = CoopGoalStatus::Stalled;
            } else {
                goal.status = CoopGoalStatus::Active;
            }

            if goal.progress_history.len() < MAX_PROGRESS_HISTORY {
                goal.progress_history.push(goal.progress);
            } else {
                goal.progress_history[goal.write_idx] = goal.progress;
            }
            goal.write_idx = (goal.write_idx + 1) % MAX_PROGRESS_HISTORY;
        }
    }

    /// Measure fairness toward a goal — shorthand for fairness-type goals
    pub fn measure_fairness(&mut self, fairness_value: f32) {
        self.update_goal("perfect_fairness", fairness_value);
    }

    /// Record resource waste observation
    pub fn waste_tracking(&mut self, wasted_units: f32, total_units: f32, source: &str) {
        self.tick += 1;
        let waste_fraction = if total_units > f32::EPSILON {
            (wasted_units / total_units).max(0.0).min(1.0)
        } else {
            0.0
        };

        self.waste_rate_ema =
            EMA_ALPHA * waste_fraction + (1.0 - EMA_ALPHA) * self.waste_rate_ema;

        let source_id = fnv1a_hash(source.as_bytes());
        let record = WasteRecord {
            tick: self.tick,
            waste_fraction,
            wasted_units,
            total_units,
            source_id,
        };

        if self.waste_history.len() < MAX_PROGRESS_HISTORY {
            self.waste_history.push(record);
        } else {
            self.waste_history[self.waste_write_idx] = record;
        }
        self.waste_write_idx = (self.waste_write_idx + 1) % MAX_PROGRESS_HISTORY;

        // Update zero-waste goal if it exists
        self.update_goal("zero_waste", 1.0 - waste_fraction);
    }

    /// Record contract quality measurement
    pub fn contract_quality(
        &mut self,
        contract_id: u64,
        fulfillment: f32,
        fairness: f32,
        duration_ratio: f32,
        renegotiations: u32,
    ) {
        self.tick += 1;
        let quality = ContractQuality {
            contract_id,
            fulfillment: fulfillment.max(0.0).min(1.0),
            fairness: fairness.max(0.0).min(1.0),
            duration_ratio: duration_ratio.max(0.0).min(2.0),
            renegotiations,
        };

        let composite = quality.fulfillment * 0.40
            + quality.fairness * 0.30
            + (1.0 - (quality.duration_ratio - 1.0).abs().min(1.0)) * 0.20
            + (1.0 - (quality.renegotiations as f32 / 5.0).min(1.0)) * 0.10;

        self.contract_quality_ema =
            EMA_ALPHA * composite + (1.0 - EMA_ALPHA) * self.contract_quality_ema;
        self.contract_quality.insert(contract_id, quality);

        // Update optimal contracts goal if it exists
        self.update_goal("optimal_contracts", composite);
    }

    /// Measure convergence: are goals converging toward their targets?
    pub fn goal_convergence(&self) -> f32 {
        if self.goals.is_empty() {
            return 0.0;
        }

        let mut converging = 0_usize;
        let mut total = 0_usize;

        for goal in self.goals.values() {
            if goal.status == CoopGoalStatus::Abandoned {
                continue;
            }
            total += 1;

            let len = goal.progress_history.len();
            if len < CONVERGENCE_WINDOW {
                continue;
            }

            // Check if the recent window shows convergence (small variance)
            let window_start = len.saturating_sub(CONVERGENCE_WINDOW);
            let window = &goal.progress_history[window_start..];
            let mean = window.iter().sum::<f32>() / window.len() as f32;
            let variance = window
                .iter()
                .map(|v| (v - mean) * (v - mean))
                .sum::<f32>()
                / window.len() as f32;

            if libm::sqrtf(variance) < CONVERGENCE_THRESHOLD && goal.progress_rate >= 0.0 {
                converging += 1;
            }
        }

        if total == 0 {
            return 0.0;
        }
        converging as f32 / total as f32
    }

    /// Get aggregate statistics
    pub fn stats(&self) -> GoalTrackerStats {
        let active = self
            .goals
            .values()
            .filter(|g| g.status == CoopGoalStatus::Active)
            .count();
        let achieved = self
            .goals
            .values()
            .filter(|g| g.status == CoopGoalStatus::Achieved)
            .count();
        let stalled = self
            .goals
            .values()
            .filter(|g| g.status == CoopGoalStatus::Stalled)
            .count();

        let avg_progress = if self.goals.is_empty() {
            0.0
        } else {
            self.goals.values().map(|g| g.progress).sum::<f32>() / self.goals.len() as f32
        };
        let avg_rate = if self.goals.is_empty() {
            0.0
        } else {
            self.goals.values().map(|g| g.progress_rate).sum::<f32>() / self.goals.len() as f32
        };

        GoalTrackerStats {
            total_goals: self.goals.len(),
            active_goals: active,
            achieved_goals: achieved,
            stalled_goals: stalled,
            avg_progress,
            avg_progress_rate: avg_rate,
            waste_rate: self.waste_rate_ema,
            avg_contract_quality: self.contract_quality_ema,
            convergence_score: self.goal_convergence(),
        }
    }
}
