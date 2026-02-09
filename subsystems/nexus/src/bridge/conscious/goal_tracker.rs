// SPDX-License-Identifier: GPL-2.0
//! # Bridge Goal Tracker
//!
//! Goal-directed bridge optimization. Maintains a hierarchy of goals —
//! maximize throughput, minimize latency, reduce overhead — and tracks
//! achievement with progress metrics. Goals can conflict and the tracker
//! provides principled resolution through weighted priority scoring.
//!
//! This module gives the bridge *purpose*: it doesn't just react, it
//! works toward declared objectives.

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
pub struct Goal {
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
    /// Historical progress snapshots
    progress_history: Vec<f32>,
    write_idx: usize,
    /// Weight for conflict resolution (higher = more important)
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
    pub conflict_count: usize,
}

// ============================================================================
// BRIDGE GOAL TRACKER
// ============================================================================

/// Maintains a hierarchy of bridge goals, tracking progress and resolving
/// conflicts between competing objectives.
#[derive(Debug)]
pub struct BridgeGoalTracker {
    /// All goals keyed by FNV-1a hash
    goals: BTreeMap<u64, Goal>,
    /// Detected conflicts
    conflicts: Vec<GoalConflict>,
    /// Monotonic tick
    tick: u64,
    /// Total goals ever created
    total_created: u64,
    /// Total goals ever achieved
    total_achieved: u64,
}

impl BridgeGoalTracker {
    pub fn new() -> Self {
        Self {
            goals: BTreeMap::new(),
            conflicts: Vec::new(),
            tick: 0,
            total_created: 0,
            total_achieved: 0,
        }
    }

    /// Create a new goal in the hierarchy
    pub fn set_goal(
        &mut self,
        name: &str,
        priority: GoalPriority,
        target: f32,
        initial: f32,
        parent_name: Option<&str>,
        deadline_ticks: u64,
        weight: f32,
    ) -> u64 {
        self.tick += 1;
        self.total_created += 1;

        let id = fnv1a_hash(name.as_bytes());
        let parent_id = parent_name.map(|p| fnv1a_hash(p.as_bytes())).unwrap_or(0);

        let progress = if (target - initial).abs() < f32::EPSILON {
            1.0
        } else if target > initial {
            ((initial) / target).max(0.0).min(1.0)
        } else {
            // Target is lower than initial (e.g., minimize latency)
            (1.0 - (initial - target) / initial.max(0.001)).max(0.0).min(1.0)
        };

        let goal = Goal {
            id,
            name: String::from(name),
            priority,
            status: GoalStatus::Active,
            target,
            current: initial,
            progress,
            progress_rate: 0.0,
            parent_id,
            set_tick: self.tick,
            deadline_tick: if deadline_ticks > 0 { self.tick + deadline_ticks } else { 0 },
            progress_history: Vec::new(),
            write_idx: 0,
            weight: weight.max(0.1),
        };

        self.goals.insert(id, goal);
        id
    }

    /// Update a goal's current value and recompute progress
    pub fn evaluate_progress(&mut self, name: &str, current_value: f32) -> Option<f32> {
        self.tick += 1;
        let id = fnv1a_hash(name.as_bytes());

        let goal = self.goals.get_mut(&id)?;
        let old_progress = goal.progress;
        goal.current = current_value;

        // Recompute progress
        goal.progress = if (goal.target - goal.current).abs() < f32::EPSILON {
            1.0
        } else if goal.target > 0.0 && goal.target > goal.current {
            (goal.current / goal.target).max(0.0).min(1.0)
        } else if goal.target < goal.current {
            // Minimization goal
            let ratio = goal.current / goal.target.max(0.001);
            (1.0 / ratio).max(0.0).min(1.0)
        } else {
            0.0
        };

        // EMA-smooth progress rate
        let delta = goal.progress - old_progress;
        goal.progress_rate = EMA_ALPHA * delta + (1.0 - EMA_ALPHA) * goal.progress_rate;

        // Record history
        if goal.progress_history.len() < MAX_PROGRESS_HISTORY {
            goal.progress_history.push(goal.progress);
        } else {
            goal.progress_history[goal.write_idx] = goal.progress;
        }
        goal.write_idx = (goal.write_idx + 1) % MAX_PROGRESS_HISTORY;

        // Update status
        if goal.progress >= 0.99 {
            if goal.status != GoalStatus::Achieved {
                self.total_achieved += 1;
            }
            goal.status = GoalStatus::Achieved;
        } else if goal.progress_rate.abs() < 0.001 && goal.progress_history.len() > 10 {
            goal.status = GoalStatus::Stalled;
        } else {
            goal.status = GoalStatus::Active;
        }

        Some(goal.progress)
    }

    /// Return goals sorted by effective priority (priority × weight × urgency)
    pub fn prioritize_goals(&self) -> Vec<(u64, String, f32)> {
        let mut scored: Vec<(u64, String, f32)> = self.goals.values()
            .filter(|g| g.status == GoalStatus::Active || g.status == GoalStatus::Stalled)
            .map(|g| {
                let base = g.priority as u8 as f32;
                let urgency = if g.deadline_tick > 0 && g.deadline_tick > self.tick {
                    let remaining = (g.deadline_tick - self.tick) as f32;
                    1.0 + (1.0 / remaining.max(1.0))
                } else {
                    1.0
                };
                let remaining_work = 1.0 - g.progress;
                let score = base * g.weight * urgency * (1.0 + remaining_work);
                (g.id, g.name.clone(), score)
            })
            .collect();
        scored.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(core::cmp::Ordering::Equal));
        scored
    }

    /// Detect and resolve conflicts between competing goals
    pub fn goal_conflict_resolution(&mut self) -> Vec<GoalConflict> {
        self.conflicts.clear();
        let ids: Vec<u64> = self.goals.keys().copied().collect();

        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                let a = match self.goals.get(&ids[i]) {
                    Some(g) if g.status == GoalStatus::Active => g,
                    _ => continue,
                };
                let b = match self.goals.get(&ids[j]) {
                    Some(g) if g.status == GoalStatus::Active => g,
                    _ => continue,
                };

                // Detect conflict: both active, progress in one negatively
                // correlated with progress in the other (approximated by
                // opposing progress rates)
                let rate_product = a.progress_rate * b.progress_rate;
                if rate_product < -0.0001 {
                    let severity = (-rate_product).min(1.0);
                    if severity > CONFLICT_THRESHOLD {
                        let recommended = if a.priority as u8 * (a.weight * 100.0) as u8
                            >= b.priority as u8 * (b.weight * 100.0) as u8
                        {
                            a.id
                        } else {
                            b.id
                        };

                        self.conflicts.push(GoalConflict {
                            goal_a_id: a.id,
                            goal_b_id: b.id,
                            goal_a_name: a.name.clone(),
                            goal_b_name: b.name.clone(),
                            severity,
                            recommended_priority: recommended,
                        });
                    }
                }
            }
        }

        // Mark conflicted goals
        for conflict in &self.conflicts {
            if let Some(g) = self.goals.get_mut(&conflict.goal_a_id) {
                if g.id != conflict.recommended_priority {
                    g.status = GoalStatus::Conflicted;
                }
            }
            if let Some(g) = self.goals.get_mut(&conflict.goal_b_id) {
                if g.id != conflict.recommended_priority {
                    g.status = GoalStatus::Conflicted;
                }
            }
        }

        self.conflicts.clone()
    }

    /// Overall achievement rate (fraction of created goals that were achieved)
    pub fn achievement_rate(&self) -> f32 {
        if self.total_created == 0 {
            return 0.0;
        }
        self.total_achieved as f32 / self.total_created as f32
    }

    /// Compute aggregate statistics
    pub fn stats(&self) -> GoalTrackerStats {
        let active = self.goals.values().filter(|g| g.status == GoalStatus::Active).count();
        let achieved = self.goals.values().filter(|g| g.status == GoalStatus::Achieved).count();
        let stalled = self.goals.values().filter(|g| g.status == GoalStatus::Stalled).count();
        let conflicted = self.goals.values().filter(|g| g.status == GoalStatus::Conflicted).count();

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
            conflicted_goals: conflicted,
            avg_progress,
            avg_progress_rate: avg_rate,
            achievement_rate: self.achievement_rate(),
            conflict_count: self.conflicts.len(),
        }
    }

    /// Get a goal's progress by name
    pub fn goal_progress(&self, name: &str) -> Option<f32> {
        let id = fnv1a_hash(name.as_bytes());
        self.goals.get(&id).map(|g| g.progress)
    }

    /// Abandon a goal
    pub fn abandon_goal(&mut self, name: &str) {
        let id = fnv1a_hash(name.as_bytes());
        if let Some(g) = self.goals.get_mut(&id) {
            g.status = GoalStatus::Abandoned;
        }
    }
}
