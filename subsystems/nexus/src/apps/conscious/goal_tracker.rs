// SPDX-License-Identifier: GPL-2.0
//! # Apps Goal Tracker
//!
//! Goal-directed application optimization. Maintains a hierarchy of goals —
//! perfect classification, zero-lag adaptation, predictive resource allocation
//! — and tracks achievement with progress metrics. Goals can conflict (e.g.,
//! classification accuracy vs. speed) and the tracker provides principled
//! resolution through weighted priority scoring.
//!
//! This module gives the apps engine *purpose*: it doesn't just classify,
//! it works toward declared objectives for application understanding.

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
const CONFLICT_THRESHOLD: f32 = 0.3;
const STALL_RATE_THRESHOLD: f32 = 0.001;
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

/// Priority tier for a goal
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GoalPriority {
    /// Must be achieved — system correctness
    Critical = 4,
    /// Should be achieved — performance
    High = 3,
    /// Nice to have — optimization
    Medium = 2,
    /// Aspirational — long-term improvement
    Low = 1,
}

/// Current status of a goal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalStatus {
    Active,
    Achieved,
    Stalled,
    Conflicted,
    Abandoned,
}

/// A single goal in the hierarchy
#[derive(Debug, Clone)]
pub struct AppGoal {
    pub id: u64,
    pub name: String,
    pub priority: GoalPriority,
    pub status: GoalStatus,
    /// Target value (what we're aiming for)
    pub target: f32,
    /// Current value (where we are)
    pub current: f32,
    /// Progress as fraction (0.0 – 1.0)
    pub progress: f32,
    /// EMA-smoothed rate of progress per tick
    pub progress_rate: f32,
    /// Parent goal ID (0 = top-level)
    pub parent_id: u64,
    /// Tick when goal was set
    pub set_tick: u64,
    /// Deadline tick (0 = no deadline)
    pub deadline_tick: u64,
    /// Historical progress snapshots (ring buffer)
    progress_history: Vec<f32>,
    write_idx: usize,
    /// Weight for conflict resolution
    pub weight: f32,
}

/// A detected conflict between two goals
#[derive(Debug, Clone)]
pub struct GoalConflict {
    pub goal_a_id: u64,
    pub goal_b_id: u64,
    pub goal_a_name: String,
    pub goal_b_name: String,
    /// How severe the conflict is (0.0 – 1.0)
    pub severity: f32,
    /// Suggested resolution: which goal to prioritize
    pub recommended_priority: u64,
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
    pub conflicted_goals: usize,
    pub avg_progress: f32,
    pub avg_progress_rate: f32,
    pub achievement_rate: f32,
}

// ============================================================================
// APPS GOAL TRACKER
// ============================================================================

/// Goal-directed app optimization — tracks hierarchical goals with conflict
/// detection, progress monitoring, and achievement reporting.
#[derive(Debug)]
pub struct AppsGoalTracker {
    /// Goals keyed by FNV-1a hash of name
    goals: BTreeMap<u64, AppGoal>,
    /// Detected conflicts
    conflicts: Vec<GoalConflict>,
    /// Monotonic tick
    tick: u64,
    /// Total goals ever achieved
    total_achieved: u64,
    /// Total goals ever set
    total_set: u64,
}

impl AppsGoalTracker {
    pub fn new() -> Self {
        Self {
            goals: BTreeMap::new(),
            conflicts: Vec::new(),
            tick: 0,
            total_achieved: 0,
            total_set: 0,
        }
    }

    /// Set or update a goal
    pub fn set_goal(
        &mut self,
        name: &str,
        target: f32,
        priority: GoalPriority,
        parent_name: &str,
        deadline_tick: u64,
    ) -> u64 {
        self.tick += 1;
        self.total_set += 1;
        let id = fnv1a_hash(name.as_bytes());
        let parent_id = if parent_name.is_empty() {
            0
        } else {
            fnv1a_hash(parent_name.as_bytes())
        };

        let goal = self.goals.entry(id).or_insert_with(|| AppGoal {
            id,
            name: String::from(name),
            priority,
            status: GoalStatus::Active,
            target,
            current: 0.0,
            progress: 0.0,
            progress_rate: 0.0,
            parent_id,
            set_tick: self.tick,
            deadline_tick,
            progress_history: Vec::new(),
            write_idx: 0,
            weight: priority as u8 as f32 / 4.0,
        });

        goal.target = target;
        goal.priority = priority;
        goal.deadline_tick = deadline_tick;
        goal.weight = priority as u8 as f32 / 4.0;
        id
    }

    /// Measure progress toward a goal with a new observation
    pub fn measure_progress(&mut self, name: &str, current_value: f32) {
        self.tick += 1;
        let id = fnv1a_hash(name.as_bytes());

        if let Some(goal) = self.goals.get_mut(&id) {
            let clamped = current_value.max(0.0);
            goal.current = clamped;

            // Compute progress fraction
            let prev_progress = goal.progress;
            goal.progress = if goal.target > 0.0 {
                (clamped / goal.target).min(1.0)
            } else {
                0.0
            };

            // EMA-smoothed progress rate
            let delta = goal.progress - prev_progress;
            goal.progress_rate =
                EMA_ALPHA * delta + (1.0 - EMA_ALPHA) * goal.progress_rate;

            // Ring buffer history
            if goal.progress_history.len() < MAX_PROGRESS_HISTORY {
                goal.progress_history.push(goal.progress);
            } else {
                goal.progress_history[goal.write_idx] = goal.progress;
            }
            goal.write_idx = (goal.write_idx + 1) % MAX_PROGRESS_HISTORY;

            // Update status
            if goal.progress >= 1.0 && goal.status == GoalStatus::Active {
                goal.status = GoalStatus::Achieved;
                self.total_achieved += 1;
            } else if goal.progress_rate.abs() < STALL_RATE_THRESHOLD
                && goal.progress < 0.95
                && goal.progress_history.len() > 20
            {
                goal.status = GoalStatus::Stalled;
            }
        }
    }

    /// Get the priority ordering of all active goals
    pub fn goal_priority(&self) -> Vec<(String, GoalPriority, f32)> {
        let mut active: Vec<(String, GoalPriority, f32)> = self.goals.values()
            .filter(|g| g.status == GoalStatus::Active)
            .map(|g| (g.name.clone(), g.priority, g.progress))
            .collect();
        active.sort_by(|a, b| b.1.cmp(&a.1));
        active
    }

    /// Detect conflicts between goals that have negative correlation
    pub fn conflict_detect(&mut self) -> Vec<GoalConflict> {
        let mut new_conflicts = Vec::new();
        let ids: Vec<u64> = self.goals.keys().copied().collect();

        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                let a = match self.goals.get(&ids[i]) {
                    Some(g) => g,
                    None => continue,
                };
                let b = match self.goals.get(&ids[j]) {
                    Some(g) => g,
                    None => continue,
                };

                // Skip non-active goals
                if a.status != GoalStatus::Active || b.status != GoalStatus::Active {
                    continue;
                }

                // Detect conflict: if both have progress history, check negative
                // correlation via simple sign-of-rate comparison
                let rate_product = a.progress_rate * b.progress_rate;
                if rate_product < -CONFLICT_THRESHOLD * 0.01 {
                    let severity = (-rate_product).min(1.0);
                    let recommended = if a.weight >= b.weight { a.id } else { b.id };

                    let conflict = GoalConflict {
                        goal_a_id: a.id,
                        goal_b_id: b.id,
                        goal_a_name: a.name.clone(),
                        goal_b_name: b.name.clone(),
                        severity,
                        recommended_priority: recommended,
                    };
                    new_conflicts.push(conflict);
                }
            }
        }

        // Mark conflicted goals
        for conflict in &new_conflicts {
            if let Some(g) = self.goals.get_mut(&conflict.goal_a_id) {
                if g.status == GoalStatus::Active {
                    g.status = GoalStatus::Conflicted;
                }
            }
            if let Some(g) = self.goals.get_mut(&conflict.goal_b_id) {
                if g.status == GoalStatus::Active {
                    g.status = GoalStatus::Conflicted;
                }
            }
        }

        for c in &new_conflicts {
            self.conflicts.push(c.clone());
        }
        new_conflicts
    }

    /// Generate an achievement report for all goals
    pub fn achievement_report(&self) -> AchievementReport {
        let total = self.goals.len();
        let active = self.goals.values().filter(|g| g.status == GoalStatus::Active).count();
        let achieved = self.goals.values().filter(|g| g.status == GoalStatus::Achieved).count();
        let stalled = self.goals.values().filter(|g| g.status == GoalStatus::Stalled).count();
        let conflicted = self.goals.values().filter(|g| g.status == GoalStatus::Conflicted).count();

        let avg_progress = if total > 0 {
            self.goals.values().map(|g| g.progress).sum::<f32>() / total as f32
        } else {
            0.0
        };
        let avg_rate = if total > 0 {
            self.goals.values().map(|g| g.progress_rate).sum::<f32>() / total as f32
        } else {
            0.0
        };
        let achievement_rate = if self.total_set > 0 {
            self.total_achieved as f32 / self.total_set as f32
        } else {
            0.0
        };

        AchievementReport {
            total_goals: total,
            active_goals: active,
            achieved_goals: achieved,
            stalled_goals: stalled,
            conflicted_goals: conflicted,
            avg_progress,
            avg_progress_rate: avg_rate,
            achievement_rate,
            total_conflicts: self.conflicts.len(),
        }
    }

    /// Compute aggregate statistics
    pub fn stats(&self) -> GoalTrackerStats {
        let report = self.achievement_report();
        GoalTrackerStats {
            total_goals: report.total_goals,
            active_goals: report.active_goals,
            achieved_goals: report.achieved_goals,
            stalled_goals: report.stalled_goals,
            conflicted_goals: report.conflicted_goals,
            avg_progress: report.avg_progress,
            avg_progress_rate: report.avg_progress_rate,
            achievement_rate: report.achievement_rate,
        }
    }
}

// ============================================================================
// ACHIEVEMENT REPORT
// ============================================================================

/// Comprehensive achievement report
#[derive(Debug, Clone, Copy)]
pub struct AchievementReport {
    pub total_goals: usize,
    pub active_goals: usize,
    pub achieved_goals: usize,
    pub stalled_goals: usize,
    pub conflicted_goals: usize,
    pub avg_progress: f32,
    pub avg_progress_rate: f32,
    pub achievement_rate: f32,
    pub total_conflicts: usize,
}
