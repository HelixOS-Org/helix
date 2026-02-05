//! # Temporal Planning for NEXUS
//!
//! Planning with temporal constraints and durations.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::actions::ActionId;

// ============================================================================
// TIME TYPES
// ============================================================================

/// Time point identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimePoint(pub u32);

/// Time value (in time units)
pub type TimeValue = i64;

/// Temporal constraint type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintType {
    /// A must start before B
    Before,
    /// A must start after B ends
    After,
    /// A and B must start at same time
    Simultaneous,
    /// A must end before B starts
    Meets,
    /// A and B must overlap
    Overlaps,
    /// A contains B (B starts after A, B ends before A)
    Contains,
    /// A duration >= min and <= max
    Duration,
}

/// A temporal constraint
#[derive(Debug, Clone)]
pub struct TemporalConstraint {
    /// Constraint type
    pub constraint_type: ConstraintType,
    /// First time point (or action start)
    pub from: TimePoint,
    /// Second time point (or action start)
    pub to: TimePoint,
    /// Minimum distance (for duration constraints)
    pub min_distance: TimeValue,
    /// Maximum distance
    pub max_distance: TimeValue,
}

impl TemporalConstraint {
    /// Create new constraint
    pub fn new(constraint_type: ConstraintType, from: TimePoint, to: TimePoint) -> Self {
        Self {
            constraint_type,
            from,
            to,
            min_distance: 0,
            max_distance: TimeValue::MAX,
        }
    }

    /// Create "before" constraint
    pub fn before(from: TimePoint, to: TimePoint) -> Self {
        Self::new(ConstraintType::Before, from, to)
    }

    /// Create "after" constraint
    pub fn after(from: TimePoint, to: TimePoint) -> Self {
        Self::new(ConstraintType::After, from, to)
    }

    /// Create duration constraint
    pub fn duration(point: TimePoint, min: TimeValue, max: TimeValue) -> Self {
        let mut c = Self::new(ConstraintType::Duration, point, point);
        c.min_distance = min;
        c.max_distance = max;
        c
    }

    /// Set distance bounds
    pub fn with_distance(mut self, min: TimeValue, max: TimeValue) -> Self {
        self.min_distance = min;
        self.max_distance = max;
        self
    }
}

// ============================================================================
// TIMELINE
// ============================================================================

/// A scheduled action on timeline
#[derive(Debug, Clone)]
pub struct ScheduledAction {
    /// Action ID
    pub action: ActionId,
    /// Start time
    pub start: TimeValue,
    /// End time
    pub end: TimeValue,
    /// Duration
    pub duration: TimeValue,
}

impl ScheduledAction {
    /// Create new scheduled action
    pub fn new(action: ActionId, start: TimeValue, duration: TimeValue) -> Self {
        Self {
            action,
            start,
            end: start + duration,
            duration,
        }
    }

    /// Check if overlaps with another
    pub fn overlaps(&self, other: &ScheduledAction) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// Check if strictly before another
    pub fn before(&self, other: &ScheduledAction) -> bool {
        self.end <= other.start
    }
}

/// A timeline for scheduling actions
#[derive(Debug, Clone)]
pub struct Timeline {
    /// Scheduled actions
    actions: Vec<ScheduledAction>,
    /// Current time
    current_time: TimeValue,
    /// Makespan (total duration)
    makespan: TimeValue,
}

impl Timeline {
    /// Create empty timeline
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
            current_time: 0,
            makespan: 0,
        }
    }

    /// Schedule action
    pub fn schedule(&mut self, action: ActionId, start: TimeValue, duration: TimeValue) -> bool {
        let scheduled = ScheduledAction::new(action, start, duration);

        // Check for conflicts
        for existing in &self.actions {
            if scheduled.overlaps(existing) {
                return false;
            }
        }

        // Update makespan
        self.makespan = self.makespan.max(scheduled.end);

        self.actions.push(scheduled);
        true
    }

    /// Schedule action at earliest possible time
    pub fn schedule_earliest(&mut self, action: ActionId, duration: TimeValue) -> TimeValue {
        let start = self.find_earliest_slot(duration);
        self.schedule(action, start, duration);
        start
    }

    /// Find earliest available slot
    fn find_earliest_slot(&self, duration: TimeValue) -> TimeValue {
        if self.actions.is_empty() {
            return 0;
        }

        // Sort by start time
        let mut sorted: Vec<_> = self.actions.iter().collect();
        sorted.sort_by_key(|a| a.start);

        // Check before first action
        if sorted[0].start >= duration {
            return 0;
        }

        // Check gaps between actions
        for i in 0..sorted.len() - 1 {
            let gap_start = sorted[i].end;
            let gap_end = sorted[i + 1].start;
            if gap_end - gap_start >= duration {
                return gap_start;
            }
        }

        // Schedule after last action
        sorted.last().map(|a| a.end).unwrap_or(0)
    }

    /// Get actions in time order
    pub fn get_schedule(&self) -> Vec<&ScheduledAction> {
        let mut sorted: Vec<_> = self.actions.iter().collect();
        sorted.sort_by_key(|a| a.start);
        sorted
    }

    /// Get makespan
    pub fn makespan(&self) -> TimeValue {
        self.makespan
    }

    /// Advance current time
    pub fn advance(&mut self, delta: TimeValue) {
        self.current_time += delta;
    }

    /// Get current time
    pub fn current_time(&self) -> TimeValue {
        self.current_time
    }

    /// Get actions at current time
    pub fn current_actions(&self) -> Vec<&ScheduledAction> {
        self.actions
            .iter()
            .filter(|a| a.start <= self.current_time && self.current_time < a.end)
            .collect()
    }
}

impl Default for Timeline {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TEMPORAL PLANNER
// ============================================================================

/// Temporal planner configuration
#[derive(Debug, Clone)]
pub struct TemporalPlannerConfig {
    /// Maximum makespan
    pub max_makespan: TimeValue,
    /// Prefer compact schedules
    pub minimize_makespan: bool,
}

impl Default for TemporalPlannerConfig {
    fn default() -> Self {
        Self {
            max_makespan: 10000,
            minimize_makespan: true,
        }
    }
}

/// Temporal planner using Simple Temporal Network (STN)
pub struct TemporalPlanner {
    /// Configuration
    config: TemporalPlannerConfig,
    /// Time points
    time_points: BTreeMap<TimePoint, TimeValue>,
    /// Constraints
    constraints: Vec<TemporalConstraint>,
    /// Action to time point mapping
    action_starts: BTreeMap<ActionId, TimePoint>,
    /// Action durations
    action_durations: BTreeMap<ActionId, TimeValue>,
    /// Next time point ID
    next_tp: u32,
}

impl TemporalPlanner {
    /// Create new temporal planner
    pub fn new(config: TemporalPlannerConfig) -> Self {
        Self {
            config,
            time_points: BTreeMap::new(),
            constraints: Vec::new(),
            action_starts: BTreeMap::new(),
            action_durations: BTreeMap::new(),
            next_tp: 0,
        }
    }

    /// Create time point
    pub fn create_time_point(&mut self) -> TimePoint {
        let tp = TimePoint(self.next_tp);
        self.next_tp += 1;
        self.time_points.insert(tp, 0);
        tp
    }

    /// Register action with duration
    pub fn register_action(&mut self, action: ActionId, duration: TimeValue) -> TimePoint {
        let start = self.create_time_point();
        self.action_starts.insert(action, start);
        self.action_durations.insert(action, duration);
        start
    }

    /// Add constraint
    pub fn add_constraint(&mut self, constraint: TemporalConstraint) {
        self.constraints.push(constraint);
    }

    /// Add ordering: action1 before action2
    pub fn add_ordering(&mut self, before: ActionId, after: ActionId) {
        if let (Some(&tp1), Some(&tp2)) = (
            self.action_starts.get(&before),
            self.action_starts.get(&after),
        ) {
            let duration = self.action_durations.get(&before).copied().unwrap_or(1);
            let constraint =
                TemporalConstraint::before(tp1, tp2).with_distance(duration, TimeValue::MAX);
            self.add_constraint(constraint);
        }
    }

    /// Solve temporal constraints using Bellman-Ford
    pub fn solve(&mut self) -> Option<Timeline> {
        // Initialize distances
        let mut distances: BTreeMap<TimePoint, TimeValue> = BTreeMap::new();
        for &tp in self.time_points.keys() {
            distances.insert(tp, 0);
        }

        // Add origin time point
        let origin = TimePoint(u32::MAX);
        distances.insert(origin, 0);

        // Build edge list from constraints
        let mut edges = Vec::new();

        for constraint in &self.constraints {
            match constraint.constraint_type {
                ConstraintType::Before => {
                    // to - from >= min_distance
                    edges.push((constraint.from, constraint.to, constraint.min_distance));
                },
                ConstraintType::After => {
                    // from - to >= min_distance
                    edges.push((constraint.to, constraint.from, constraint.min_distance));
                },
                ConstraintType::Duration => {
                    // Already encoded in action durations
                },
                _ => {},
            }
        }

        // Add edges from origin to all time points (distance 0)
        for &tp in self.time_points.keys() {
            edges.push((origin, tp, 0));
        }

        // Bellman-Ford relaxation
        let n = distances.len();
        for _ in 0..n {
            for &(from, to, weight) in &edges {
                let dist_from = distances.get(&from).copied().unwrap_or(TimeValue::MAX);
                let dist_to = distances.get(&to).copied().unwrap_or(TimeValue::MAX);

                if dist_from != TimeValue::MAX {
                    let new_dist = dist_from + weight;
                    if new_dist > dist_to {
                        distances.insert(to, new_dist);
                    }
                }
            }
        }

        // Check for negative cycles
        for &(from, to, weight) in &edges {
            let dist_from = distances.get(&from).copied().unwrap_or(TimeValue::MAX);
            let dist_to = distances.get(&to).copied().unwrap_or(TimeValue::MAX);

            if dist_from != TimeValue::MAX && dist_from + weight > dist_to {
                return None; // Negative cycle detected (infeasible)
            }
        }

        // Build timeline
        let mut timeline = Timeline::new();

        for (&action, &start_tp) in &self.action_starts {
            let start_time = distances.get(&start_tp).copied().unwrap_or(0);
            let duration = self.action_durations.get(&action).copied().unwrap_or(1);

            if start_time > self.config.max_makespan {
                return None;
            }

            timeline.schedule(action, start_time, duration);
        }

        Some(timeline)
    }

    /// Get action start time
    pub fn get_start_time(&self, action: ActionId) -> Option<TimeValue> {
        self.action_starts
            .get(&action)
            .and_then(|tp| self.time_points.get(tp))
            .copied()
    }
}

impl Default for TemporalPlanner {
    fn default() -> Self {
        Self::new(TemporalPlannerConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeline_scheduling() {
        let mut timeline = Timeline::new();

        assert!(timeline.schedule(ActionId(0), 0, 5));
        assert!(timeline.schedule(ActionId(1), 5, 3));
        assert!(!timeline.schedule(ActionId(2), 2, 5)); // Overlaps with first

        assert_eq!(timeline.makespan(), 8);
    }

    #[test]
    fn test_timeline_earliest() {
        let mut timeline = Timeline::new();

        let t0 = timeline.schedule_earliest(ActionId(0), 5);
        assert_eq!(t0, 0);

        let t1 = timeline.schedule_earliest(ActionId(1), 3);
        assert_eq!(t1, 5); // After first action
    }

    #[test]
    fn test_scheduled_action_overlap() {
        let a1 = ScheduledAction::new(ActionId(0), 0, 5);
        let a2 = ScheduledAction::new(ActionId(1), 3, 4);
        let a3 = ScheduledAction::new(ActionId(2), 5, 3);

        assert!(a1.overlaps(&a2));
        assert!(!a1.overlaps(&a3));
        assert!(a1.before(&a3));
    }

    #[test]
    fn test_temporal_planner_ordering() {
        let mut planner = TemporalPlanner::default();

        planner.register_action(ActionId(0), 5);
        planner.register_action(ActionId(1), 3);
        planner.register_action(ActionId(2), 4);

        planner.add_ordering(ActionId(0), ActionId(1));
        planner.add_ordering(ActionId(1), ActionId(2));

        let timeline = planner.solve();
        assert!(timeline.is_some());

        let timeline = timeline.unwrap();
        let schedule = timeline.get_schedule();

        // Action 0 should be first, then 1, then 2
        assert_eq!(schedule.len(), 3);
    }

    #[test]
    fn test_temporal_constraint() {
        let c = TemporalConstraint::before(TimePoint(0), TimePoint(1)).with_distance(5, 10);

        assert_eq!(c.min_distance, 5);
        assert_eq!(c.max_distance, 10);
    }
}
