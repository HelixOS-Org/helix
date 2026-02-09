// SPDX-License-Identifier: GPL-2.0
//! # Holistic Goal Tracker
//!
//! System-wide goal hierarchy. Root goals define the kernel's highest
//! aspirations: maximize uptime (99.9999%), minimize latency (p99 < 10ns),
//! zero security breaches. Each root goal decomposes into sub-goals for
//! individual subsystems, with alignment checks and synergy detection.
//!
//! Goals are not wishes — they are commitments tracked with precision.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_ROOT_GOALS: usize = 32;
const MAX_SUBGOALS: usize = 256;
const MAX_SYNERGIES: usize = 128;
const MAX_HISTORY: usize = 128;
const EMA_ALPHA: f32 = 0.10;
const ALIGNMENT_THRESHOLD: f32 = 0.70;
const PARETO_EPSILON: f32 = 0.02;
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
// GOAL TYPES
// ============================================================================

/// Priority tier for root goals
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GoalPriority {
    Critical = 0,
    High     = 1,
    Medium   = 2,
    Low      = 3,
}

/// Status of a goal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalStatus {
    NotStarted,
    InProgress,
    OnTrack,
    AtRisk,
    Achieved,
    Failed,
}

/// A root-level system goal
#[derive(Debug, Clone)]
pub struct RootGoal {
    pub name: String,
    pub id: u64,
    pub priority: GoalPriority,
    pub target_value: f32,
    pub current_value: f32,
    pub status: GoalStatus,
    pub progress: f32,
    pub subgoal_ids: Vec<u64>,
    pub tick_created: u64,
    pub tick_updated: u64,
    pub deadline_ticks: u64,
}

/// A decomposed sub-goal assigned to a subsystem
#[derive(Debug, Clone)]
pub struct SubGoal {
    pub name: String,
    pub id: u64,
    pub parent_id: u64,
    pub subsystem: String,
    pub target_value: f32,
    pub current_value: f32,
    pub weight: f32,
    pub progress: f32,
    pub status: GoalStatus,
}

/// A synergy or conflict between goals
#[derive(Debug, Clone)]
pub struct GoalSynergy {
    pub goal_a: u64,
    pub goal_b: u64,
    pub correlation: f32,
    pub synergy_type: SynergyType,
    pub sample_count: u64,
}

/// Type of goal interaction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SynergyType {
    Synergistic,
    Conflicting,
    Independent,
}

/// Pareto frontier point
#[derive(Debug, Clone)]
pub struct ParetoPoint {
    pub goal_scores: Vec<(u64, f32)>,
    pub is_dominant: bool,
    pub trade_off_cost: f32,
}

// ============================================================================
// STATS
// ============================================================================

/// Aggregate goal tracking statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct GoalTrackerStats {
    pub root_goal_count: usize,
    pub subgoal_count: usize,
    pub avg_progress: f32,
    pub goals_on_track: usize,
    pub goals_at_risk: usize,
    pub goals_achieved: usize,
    pub alignment_score: f32,
    pub synergy_score: f32,
    pub pareto_efficiency: f32,
}

// ============================================================================
// HOLISTIC GOAL TRACKER
// ============================================================================

/// System-wide goal hierarchy. Decomposes root goals into subsystem
/// sub-goals, checks alignment, detects synergies and conflicts,
/// and evaluates Pareto optimality of goal configurations.
#[derive(Debug)]
pub struct HolisticGoalTracker {
    root_goals: BTreeMap<u64, RootGoal>,
    subgoals: BTreeMap<u64, SubGoal>,
    synergies: BTreeMap<u64, GoalSynergy>,
    progress_history: Vec<(u64, f32)>,
    tick: u64,
    rng_state: u64,
    alignment_ema: f32,
    overall_progress_ema: f32,
}

impl HolisticGoalTracker {
    pub fn new() -> Self {
        Self {
            root_goals: BTreeMap::new(),
            subgoals: BTreeMap::new(),
            synergies: BTreeMap::new(),
            progress_history: Vec::new(),
            tick: 0,
            rng_state: 0x9876_5432_FEDC_BA10,
            alignment_ema: 0.5,
            overall_progress_ema: 0.0,
        }
    }

    /// Create a root-level goal for the entire system
    pub fn create_root_goal(
        &mut self,
        name: String,
        priority: GoalPriority,
        target: f32,
        deadline: u64,
    ) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(name.as_bytes()) ^ xorshift64(&mut self.rng_state);

        if self.root_goals.len() >= MAX_ROOT_GOALS {
            return id;
        }

        let goal = RootGoal {
            name,
            id,
            priority,
            target_value: target,
            current_value: 0.0,
            status: GoalStatus::NotStarted,
            progress: 0.0,
            subgoal_ids: Vec::new(),
            tick_created: self.tick,
            tick_updated: self.tick,
            deadline_ticks: deadline,
        };
        self.root_goals.insert(id, goal);
        id
    }

    /// Decompose a root goal into subsystem sub-goals
    pub fn decompose_goals(
        &mut self,
        root_id: u64,
        decompositions: Vec<(String, String, f32, f32)>,
    ) -> Vec<u64> {
        self.tick += 1;
        let mut ids = Vec::new();

        for (name, subsystem, target, weight) in decompositions {
            if self.subgoals.len() >= MAX_SUBGOALS {
                break;
            }
            let id = fnv1a_hash(name.as_bytes()) ^ xorshift64(&mut self.rng_state);
            let sub = SubGoal {
                name,
                id,
                parent_id: root_id,
                subsystem,
                target_value: target,
                current_value: 0.0,
                weight: weight.clamp(0.0, 1.0),
                progress: 0.0,
                status: GoalStatus::NotStarted,
            };
            self.subgoals.insert(id, sub);
            ids.push(id);

            if let Some(root) = self.root_goals.get_mut(&root_id) {
                root.subgoal_ids.push(id);
            }
        }
        ids
    }

    /// Update progress on a sub-goal
    pub fn update_subgoal(&mut self, subgoal_id: u64, current_value: f32) {
        self.tick += 1;
        if let Some(sub) = self.subgoals.get_mut(&subgoal_id) {
            sub.current_value = current_value;
            sub.progress = if sub.target_value > 0.0 {
                (current_value / sub.target_value).clamp(0.0, 1.0)
            } else {
                0.0
            };
            sub.status = if sub.progress >= 1.0 {
                GoalStatus::Achieved
            } else if sub.progress >= 0.7 {
                GoalStatus::OnTrack
            } else if sub.progress >= 0.3 {
                GoalStatus::InProgress
            } else {
                GoalStatus::AtRisk
            };

            self.propagate_to_root(sub.parent_id);
        }
    }

    /// Propagate sub-goal progress to the root goal
    fn propagate_to_root(&mut self, root_id: u64) {
        let children: Vec<(f32, f32)> = self
            .subgoals
            .values()
            .filter(|s| s.parent_id == root_id)
            .map(|s| (s.progress, s.weight))
            .collect();

        if children.is_empty() {
            return;
        }

        let total_weight: f32 = children.iter().map(|(_, w)| *w).sum();
        let weighted_progress: f32 = children.iter().map(|(p, w)| p * w).sum();
        let progress = if total_weight > 0.0 {
            weighted_progress / total_weight
        } else {
            0.0
        };

        if let Some(root) = self.root_goals.get_mut(&root_id) {
            root.progress = progress;
            root.tick_updated = self.tick;
            root.current_value = progress * root.target_value;
            root.status = if progress >= 1.0 {
                GoalStatus::Achieved
            } else if progress >= 0.7 {
                GoalStatus::OnTrack
            } else if progress >= 0.3 {
                GoalStatus::InProgress
            } else {
                GoalStatus::AtRisk
            };
        }
    }

    /// Get status of all root goals
    pub fn root_goal_status(&mut self) -> Vec<(u64, GoalStatus, f32)> {
        self.tick += 1;
        let status: Vec<(u64, GoalStatus, f32)> = self
            .root_goals
            .values()
            .map(|g| (g.id, g.status, g.progress))
            .collect();

        let avg_progress = if status.is_empty() {
            0.0
        } else {
            status.iter().map(|(_, _, p)| *p).sum::<f32>() / status.len() as f32
        };
        self.overall_progress_ema =
            EMA_ALPHA * avg_progress + (1.0 - EMA_ALPHA) * self.overall_progress_ema;

        if self.progress_history.len() < MAX_HISTORY {
            self.progress_history
                .push((self.tick, self.overall_progress_ema));
        }

        status
    }

    /// Check alignment between root goals and their sub-goals
    pub fn goal_alignment_check(&mut self) -> f32 {
        let mut alignment_sum = 0.0f32;
        let mut goal_count = 0u32;

        for root in self.root_goals.values() {
            let children: Vec<&SubGoal> = self
                .subgoals
                .values()
                .filter(|s| s.parent_id == root.id)
                .collect();
            if children.is_empty() {
                continue;
            }

            let child_progress_avg =
                children.iter().map(|c| c.progress).sum::<f32>() / children.len() as f32;
            let alignment = 1.0 - (root.progress - child_progress_avg).abs();
            alignment_sum += alignment.clamp(0.0, 1.0);
            goal_count += 1;
        }

        let alignment = if goal_count > 0 {
            alignment_sum / goal_count as f32
        } else {
            0.5
        };

        self.alignment_ema = EMA_ALPHA * alignment + (1.0 - EMA_ALPHA) * self.alignment_ema;
        self.alignment_ema
    }

    /// Detect synergies and conflicts between goals
    pub fn cross_module_synergy(&mut self) -> Vec<GoalSynergy> {
        self.tick += 1;
        let root_ids: Vec<u64> = self.root_goals.keys().copied().collect();
        let mut detected = Vec::new();

        for i in 0..root_ids.len() {
            for j in (i + 1)..root_ids.len() {
                let a = &self.root_goals[&root_ids[i]];
                let b = &self.root_goals[&root_ids[j]];

                let correlation = correlation_estimate(a.progress, b.progress);
                let synergy_type = if correlation > 0.3 {
                    SynergyType::Synergistic
                } else if correlation < -0.3 {
                    SynergyType::Conflicting
                } else {
                    SynergyType::Independent
                };

                let key = fnv1a_hash(&[root_ids[i] as u8, root_ids[j] as u8]);
                let synergy = self.synergies.entry(key).or_insert(GoalSynergy {
                    goal_a: root_ids[i],
                    goal_b: root_ids[j],
                    correlation: 0.0,
                    synergy_type,
                    sample_count: 0,
                });
                synergy.correlation =
                    EMA_ALPHA * correlation + (1.0 - EMA_ALPHA) * synergy.correlation;
                synergy.synergy_type = synergy_type;
                synergy.sample_count += 1;

                detected.push(synergy.clone());
            }
        }
        detected
    }

    /// Evaluate Pareto optimality of the current goal configuration
    pub fn pareto_optimal_goals(&self) -> Vec<ParetoPoint> {
        let goals: Vec<(u64, f32)> = self
            .root_goals
            .values()
            .map(|g| (g.id, g.progress))
            .collect();

        if goals.is_empty() {
            return Vec::new();
        }

        let mut points = Vec::new();
        for (i, (id_i, prog_i)) in goals.iter().enumerate() {
            let mut is_dominated = false;
            for (j, (_id_j, prog_j)) in goals.iter().enumerate() {
                if i != j && *prog_j > *prog_i + PARETO_EPSILON {
                    is_dominated = true;
                    break;
                }
            }
            let trade_off = goals
                .iter()
                .filter(|(gid, _)| *gid != *id_i)
                .map(|(_, p)| (prog_i - p).abs())
                .sum::<f32>()
                / (goals.len().max(1) - 1).max(1) as f32;

            points.push(ParetoPoint {
                goal_scores: alloc::vec![(*id_i, *prog_i)],
                is_dominant: !is_dominated,
                trade_off_cost: trade_off,
            });
        }
        points
    }

    /// Compute aggregate statistics
    pub fn stats(&self) -> GoalTrackerStats {
        let on_track = self
            .root_goals
            .values()
            .filter(|g| g.status == GoalStatus::OnTrack)
            .count();
        let at_risk = self
            .root_goals
            .values()
            .filter(|g| g.status == GoalStatus::AtRisk)
            .count();
        let achieved = self
            .root_goals
            .values()
            .filter(|g| g.status == GoalStatus::Achieved)
            .count();

        let avg_progress = if self.root_goals.is_empty() {
            0.0
        } else {
            self.root_goals.values().map(|g| g.progress).sum::<f32>() / self.root_goals.len() as f32
        };

        let synergy_score = if self.synergies.is_empty() {
            0.0
        } else {
            self.synergies.values().map(|s| s.correlation).sum::<f32>()
                / self.synergies.len() as f32
        };

        GoalTrackerStats {
            root_goal_count: self.root_goals.len(),
            subgoal_count: self.subgoals.len(),
            avg_progress,
            goals_on_track: on_track,
            goals_at_risk: at_risk,
            goals_achieved: achieved,
            alignment_score: self.alignment_ema,
            synergy_score,
            pareto_efficiency: avg_progress,
        }
    }
}

/// Simple correlation estimate from two progress values
fn correlation_estimate(a: f32, b: f32) -> f32 {
    let diff = (a - b).abs();
    if diff < 0.1 {
        0.8 - diff
    } else if diff < 0.3 {
        0.3 - diff
    } else {
        -0.2 - diff * 0.5
    }
}
