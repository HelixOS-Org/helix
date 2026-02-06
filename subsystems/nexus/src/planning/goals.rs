//! # Goal Management for NEXUS Planning
//!
//! Goal representation, prioritization, and lifecycle management.

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// GOAL TYPES
// ============================================================================

/// Goal identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GoalId(pub u32);

/// Goal priority level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GoalPriority {
    /// Critical - must achieve
    Critical = 100,
    /// High priority
    High     = 75,
    /// Medium priority
    Medium   = 50,
    /// Low priority
    Low      = 25,
    /// Optional - nice to have
    Optional = 10,
}

impl GoalPriority {
    /// Convert to numeric value
    pub fn value(&self) -> u32 {
        *self as u32
    }
}

/// Goal status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalStatus {
    /// Goal is pending (not yet started)
    Pending,
    /// Goal is active (being pursued)
    Active,
    /// Goal is suspended (temporarily paused)
    Suspended,
    /// Goal is achieved
    Achieved,
    /// Goal has failed
    Failed,
    /// Goal was abandoned
    Abandoned,
}

/// Goal type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalType {
    /// Achieve a state
    Achievement,
    /// Maintain a state
    Maintenance,
    /// Avoid a state
    Avoidance,
    /// Perform an action
    Performance,
    /// Query/gather information
    Query,
}

/// A goal to be achieved
#[derive(Debug, Clone)]
pub struct Goal {
    /// Goal ID
    pub id: GoalId,
    /// Goal name
    pub name: String,
    /// Description
    pub description: String,
    /// Goal type
    pub goal_type: GoalType,
    /// Priority
    pub priority: GoalPriority,
    /// Current status
    pub status: GoalStatus,
    /// Parent goal (for subgoals)
    pub parent: Option<GoalId>,
    /// Subgoals
    pub subgoals: Vec<GoalId>,
    /// Prerequisites (goals that must be achieved first)
    pub prerequisites: BTreeSet<GoalId>,
    /// Conflicts (mutually exclusive goals)
    pub conflicts: BTreeSet<GoalId>,
    /// Deadline (optional timestamp)
    pub deadline: Option<u64>,
    /// Progress (0.0 to 1.0)
    pub progress: f64,
    /// Utility value (for decision making)
    pub utility: f64,
    /// Cost estimate
    pub cost: f64,
    /// Attempts made
    pub attempts: u32,
    /// Maximum attempts allowed
    pub max_attempts: u32,
    /// Creation timestamp
    pub created_at: u64,
    /// Last update timestamp
    pub updated_at: u64,
}

impl Goal {
    /// Create new goal
    pub fn new(id: GoalId, name: String, goal_type: GoalType) -> Self {
        Self {
            id,
            name,
            description: String::new(),
            goal_type,
            priority: GoalPriority::Medium,
            status: GoalStatus::Pending,
            parent: None,
            subgoals: Vec::new(),
            prerequisites: BTreeSet::new(),
            conflicts: BTreeSet::new(),
            deadline: None,
            progress: 0.0,
            utility: 1.0,
            cost: 1.0,
            attempts: 0,
            max_attempts: 3,
            created_at: 0,
            updated_at: 0,
        }
    }

    /// Set priority
    pub fn with_priority(mut self, priority: GoalPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set deadline
    pub fn with_deadline(mut self, deadline: u64) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// Add prerequisite
    pub fn with_prerequisite(mut self, prereq: GoalId) -> Self {
        self.prerequisites.insert(prereq);
        self
    }

    /// Set utility
    pub fn with_utility(mut self, utility: f64) -> Self {
        self.utility = utility.max(0.0);
        self
    }

    /// Set cost
    pub fn with_cost(mut self, cost: f64) -> Self {
        self.cost = cost.max(0.0);
        self
    }

    /// Check if goal is terminal (achieved, failed, or abandoned)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            GoalStatus::Achieved | GoalStatus::Failed | GoalStatus::Abandoned
        )
    }

    /// Check if goal is actionable (can be pursued)
    pub fn is_actionable(&self) -> bool {
        matches!(self.status, GoalStatus::Pending | GoalStatus::Active)
    }

    /// Update progress
    pub fn update_progress(&mut self, progress: f64) {
        self.progress = progress.clamp(0.0, 1.0);
        if self.progress >= 1.0 {
            self.status = GoalStatus::Achieved;
        }
    }

    /// Increment attempt counter
    pub fn attempt(&mut self) -> bool {
        self.attempts += 1;
        if self.attempts >= self.max_attempts {
            self.status = GoalStatus::Failed;
            false
        } else {
            true
        }
    }

    /// Calculate priority score (combining priority, urgency, utility)
    pub fn priority_score(&self, current_time: u64) -> f64 {
        let base_priority = self.priority.value() as f64 / 100.0;

        // Urgency based on deadline
        let urgency = if let Some(deadline) = self.deadline {
            if current_time >= deadline {
                2.0 // Overdue - highest urgency
            } else {
                let time_left = deadline - current_time;
                let urgency_factor = 1.0 / (1.0 + time_left as f64 / 1000.0);
                1.0 + urgency_factor
            }
        } else {
            1.0
        };

        // Combine factors
        base_priority * urgency * self.utility / (1.0 + self.cost)
    }
}

// ============================================================================
// GOAL MANAGER
// ============================================================================

/// Goal manager for tracking and prioritizing goals
pub struct GoalManager {
    /// All goals
    goals: BTreeMap<GoalId, Goal>,
    /// Active goals
    active: BTreeSet<GoalId>,
    /// Goal hierarchy (parent -> children)
    hierarchy: BTreeMap<GoalId, Vec<GoalId>>,
    /// Next goal ID
    next_id: u32,
    /// Current timestamp
    current_time: u64,
}

impl GoalManager {
    /// Create new goal manager
    pub fn new() -> Self {
        Self {
            goals: BTreeMap::new(),
            active: BTreeSet::new(),
            hierarchy: BTreeMap::new(),
            next_id: 0,
            current_time: 0,
        }
    }

    /// Add goal
    pub fn add_goal(&mut self, mut goal: Goal) -> GoalId {
        goal.created_at = self.current_time;
        goal.updated_at = self.current_time;

        let id = goal.id;

        // Update hierarchy if parent exists
        if let Some(parent_id) = goal.parent {
            self.hierarchy.entry(parent_id).or_default().push(id);

            // Add to parent's subgoals
            if let Some(parent) = self.goals.get_mut(&parent_id) {
                parent.subgoals.push(id);
            }
        }

        self.goals.insert(id, goal);
        id
    }

    /// Create and add goal
    pub fn create_goal(&mut self, name: String, goal_type: GoalType) -> GoalId {
        let id = GoalId(self.next_id);
        self.next_id += 1;
        let goal = Goal::new(id, name, goal_type);
        self.add_goal(goal)
    }

    /// Create subgoal
    pub fn create_subgoal(&mut self, parent: GoalId, name: String, goal_type: GoalType) -> GoalId {
        let id = GoalId(self.next_id);
        self.next_id += 1;
        let mut goal = Goal::new(id, name, goal_type);
        goal.parent = Some(parent);

        // Inherit priority from parent
        if let Some(parent_goal) = self.goals.get(&parent) {
            goal.priority = parent_goal.priority;
        }

        self.add_goal(goal)
    }

    /// Activate goal
    pub fn activate(&mut self, id: GoalId) -> bool {
        // Pre-check prerequisites and conflicts
        let prereqs_met = self.prerequisites_met(id);
        if !prereqs_met {
            return false;
        }

        // Get goal data first
        let (is_actionable, conflicts) = if let Some(goal) = self.goals.get(&id) {
            (goal.is_actionable(), goal.conflicts.clone())
        } else {
            return false;
        };

        if !is_actionable {
            return false;
        }

        // Check for conflicts with active goals
        for &active_id in &self.active {
            if conflicts.contains(&active_id) {
                return false;
            }
        }

        // Now we can mutably borrow and update
        if let Some(goal) = self.goals.get_mut(&id) {
            goal.status = GoalStatus::Active;
            goal.updated_at = self.current_time;
            self.active.insert(id);
            return true;
        }
        false
    }

    /// Suspend goal
    pub fn suspend(&mut self, id: GoalId) {
        if let Some(goal) = self.goals.get_mut(&id) {
            goal.status = GoalStatus::Suspended;
            goal.updated_at = self.current_time;
            self.active.remove(&id);
        }
    }

    /// Mark goal as achieved
    pub fn achieve(&mut self, id: GoalId) {
        if let Some(goal) = self.goals.get_mut(&id) {
            goal.status = GoalStatus::Achieved;
            goal.progress = 1.0;
            goal.updated_at = self.current_time;
            self.active.remove(&id);

            // Check if parent can be achieved
            if let Some(parent_id) = goal.parent {
                self.update_parent_progress(parent_id);
            }
        }
    }

    /// Mark goal as failed
    pub fn fail(&mut self, id: GoalId) {
        if let Some(goal) = self.goals.get_mut(&id) {
            goal.status = GoalStatus::Failed;
            goal.updated_at = self.current_time;
            self.active.remove(&id);
        }
    }

    /// Update goal progress
    pub fn update_progress(&mut self, id: GoalId, progress: f64) {
        if let Some(goal) = self.goals.get_mut(&id) {
            goal.update_progress(progress);
            goal.updated_at = self.current_time;

            if goal.status == GoalStatus::Achieved {
                self.active.remove(&id);
            }
        }
    }

    /// Update parent progress based on subgoals
    fn update_parent_progress(&mut self, parent_id: GoalId) {
        let subgoals = self.hierarchy.get(&parent_id).cloned().unwrap_or_default();
        if subgoals.is_empty() {
            return;
        }

        let achieved = subgoals
            .iter()
            .filter(|id| {
                self.goals
                    .get(id)
                    .map(|g| g.status == GoalStatus::Achieved)
                    .unwrap_or(false)
            })
            .count();

        let progress = achieved as f64 / subgoals.len() as f64;

        if let Some(parent) = self.goals.get_mut(&parent_id) {
            parent.progress = progress;
            parent.updated_at = self.current_time;

            if progress >= 1.0 {
                parent.status = GoalStatus::Achieved;
                self.active.remove(&parent_id);
            }
        }
    }

    /// Check if prerequisites are met
    pub fn prerequisites_met(&self, id: GoalId) -> bool {
        let goal = match self.goals.get(&id) {
            Some(g) => g,
            None => return false,
        };

        goal.prerequisites.iter().all(|prereq_id| {
            self.goals
                .get(prereq_id)
                .map(|g| g.status == GoalStatus::Achieved)
                .unwrap_or(true)
        })
    }

    /// Get goal
    pub fn get_goal(&self, id: GoalId) -> Option<&Goal> {
        self.goals.get(&id)
    }

    /// Get goal mut
    pub fn get_goal_mut(&mut self, id: GoalId) -> Option<&mut Goal> {
        self.goals.get_mut(&id)
    }

    /// Get active goals sorted by priority
    pub fn get_active_sorted(&self) -> Vec<&Goal> {
        let mut active: Vec<&Goal> = self
            .active
            .iter()
            .filter_map(|id| self.goals.get(id))
            .collect();

        active.sort_by(|a, b| {
            let score_a = a.priority_score(self.current_time);
            let score_b = b.priority_score(self.current_time);
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(core::cmp::Ordering::Equal)
        });

        active
    }

    /// Get highest priority active goal
    pub fn get_top_goal(&self) -> Option<&Goal> {
        self.get_active_sorted().into_iter().next()
    }

    /// Get pending goals that can be activated
    pub fn get_activatable(&self) -> Vec<&Goal> {
        self.goals
            .values()
            .filter(|g| g.status == GoalStatus::Pending && self.prerequisites_met(g.id))
            .collect()
    }

    /// Update current time
    pub fn update_time(&mut self, time: u64) {
        self.current_time = time;
    }

    /// Get goal count
    pub fn goal_count(&self) -> usize {
        self.goals.len()
    }

    /// Get active count
    pub fn active_count(&self) -> usize {
        self.active.len()
    }

    /// Get achieved count
    pub fn achieved_count(&self) -> usize {
        self.goals
            .values()
            .filter(|g| g.status == GoalStatus::Achieved)
            .count()
    }
}

impl Default for GoalManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// GOAL SELECTION
// ============================================================================

/// Goal selection strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionStrategy {
    /// Highest priority first
    HighestPriority,
    /// Most urgent (closest deadline)
    MostUrgent,
    /// Best utility/cost ratio
    BestUtility,
    /// Weighted combination
    Weighted,
}

/// Goal selector
pub struct GoalSelector {
    /// Selection strategy
    strategy: SelectionStrategy,
    /// Weights for weighted strategy
    priority_weight: f64,
    urgency_weight: f64,
    utility_weight: f64,
}

impl GoalSelector {
    /// Create new selector
    pub fn new(strategy: SelectionStrategy) -> Self {
        Self {
            strategy,
            priority_weight: 0.4,
            urgency_weight: 0.3,
            utility_weight: 0.3,
        }
    }

    /// Select best goal from candidates
    pub fn select<'a>(&self, goals: &[&'a Goal], current_time: u64) -> Option<&'a Goal> {
        if goals.is_empty() {
            return None;
        }

        match self.strategy {
            SelectionStrategy::HighestPriority => {
                goals.iter().max_by_key(|g| g.priority.value()).copied()
            },
            SelectionStrategy::MostUrgent => goals
                .iter()
                .filter(|g| g.deadline.is_some())
                .min_by_key(|g| g.deadline.unwrap())
                .copied()
                .or_else(|| goals.first().copied()),
            SelectionStrategy::BestUtility => goals
                .iter()
                .max_by(|a, b| {
                    let ratio_a = a.utility / (1.0 + a.cost);
                    let ratio_b = b.utility / (1.0 + b.cost);
                    ratio_a
                        .partial_cmp(&ratio_b)
                        .unwrap_or(core::cmp::Ordering::Equal)
                })
                .copied(),
            SelectionStrategy::Weighted => goals
                .iter()
                .max_by(|a, b| {
                    let score_a = self.weighted_score(a, current_time);
                    let score_b = self.weighted_score(b, current_time);
                    score_a
                        .partial_cmp(&score_b)
                        .unwrap_or(core::cmp::Ordering::Equal)
                })
                .copied(),
        }
    }

    /// Calculate weighted score
    fn weighted_score(&self, goal: &Goal, current_time: u64) -> f64 {
        let priority = goal.priority.value() as f64 / 100.0;

        let urgency = if let Some(deadline) = goal.deadline {
            if current_time >= deadline {
                1.0
            } else {
                1.0 / (1.0 + (deadline - current_time) as f64 / 1000.0)
            }
        } else {
            0.5
        };

        let utility_ratio = goal.utility / (1.0 + goal.cost);

        self.priority_weight * priority
            + self.urgency_weight * urgency
            + self.utility_weight * utility_ratio
    }
}

impl Default for GoalSelector {
    fn default() -> Self {
        Self::new(SelectionStrategy::Weighted)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_goal_creation() {
        let goal = Goal::new(GoalId(0), String::from("test"), GoalType::Achievement)
            .with_priority(GoalPriority::High)
            .with_utility(2.0);

        assert_eq!(goal.priority, GoalPriority::High);
        assert_eq!(goal.utility, 2.0);
        assert_eq!(goal.status, GoalStatus::Pending);
    }

    #[test]
    fn test_goal_progress() {
        let mut goal = Goal::new(GoalId(0), String::from("test"), GoalType::Achievement);

        goal.update_progress(0.5);
        assert_eq!(goal.progress, 0.5);
        assert_eq!(goal.status, GoalStatus::Pending);

        goal.update_progress(1.0);
        assert_eq!(goal.progress, 1.0);
        assert_eq!(goal.status, GoalStatus::Achieved);
    }

    #[test]
    fn test_goal_manager() {
        let mut manager = GoalManager::new();

        let id1 = manager.create_goal(String::from("goal1"), GoalType::Achievement);
        let id2 = manager.create_goal(String::from("goal2"), GoalType::Achievement);

        assert_eq!(manager.goal_count(), 2);

        manager.activate(id1);
        assert_eq!(manager.active_count(), 1);

        manager.achieve(id1);
        assert_eq!(manager.achieved_count(), 1);
        assert_eq!(manager.active_count(), 0);

        // id2 should still be pending
        assert_eq!(manager.get_goal(id2).unwrap().status, GoalStatus::Pending);
    }

    #[test]
    fn test_goal_hierarchy() {
        let mut manager = GoalManager::new();

        let parent = manager.create_goal(String::from("parent"), GoalType::Achievement);
        let child1 = manager.create_subgoal(parent, String::from("child1"), GoalType::Achievement);
        let child2 = manager.create_subgoal(parent, String::from("child2"), GoalType::Achievement);

        // Achieve children
        manager.achieve(child1);
        assert!(manager.get_goal(parent).unwrap().progress < 1.0);

        manager.achieve(child2);
        assert_eq!(manager.get_goal(parent).unwrap().progress, 1.0);
    }

    #[test]
    fn test_goal_selector() {
        let selector = GoalSelector::new(SelectionStrategy::HighestPriority);

        let goal1 = Goal::new(GoalId(0), String::from("low"), GoalType::Achievement)
            .with_priority(GoalPriority::Low);
        let goal2 = Goal::new(GoalId(1), String::from("high"), GoalType::Achievement)
            .with_priority(GoalPriority::High);

        let goals = vec![&goal1, &goal2];
        let selected = selector.select(&goals, 0);

        assert!(selected.is_some());
        assert_eq!(selected.unwrap().id, GoalId(1)); // High priority
    }
}
