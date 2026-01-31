//! # Decision Prioritization
//!
//! Prioritizes decisions based on urgency, importance, and context.
//! Implements priority queues and scheduling algorithms.
//!
//! Part of Year 2 COGNITION - Decision-Making Engine

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// PRIORITY TYPES
// ============================================================================

/// Decision item
#[derive(Debug, Clone)]
pub struct DecisionItem {
    /// Item ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Priority score
    pub priority: f64,
    /// Urgency
    pub urgency: f64,
    /// Importance
    pub importance: f64,
    /// Effort
    pub effort: f64,
    /// Deadline
    pub deadline: Option<Timestamp>,
    /// Dependencies
    pub dependencies: Vec<u64>,
    /// Status
    pub status: ItemStatus,
    /// Created
    pub created: Timestamp,
}

/// Item status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemStatus {
    Pending,
    Ready,
    InProgress,
    Blocked,
    Completed,
    Deferred,
}

/// Priority method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorityMethod {
    /// Urgency * Importance
    Eisenhower,
    /// Value / Effort
    WeightedShortestJob,
    /// Deadline-based
    EarliestDeadline,
    /// FIFO
    FirstComeFirstServed,
    /// Impact-based
    ImpactFirst,
    /// Custom weighted
    Weighted,
}

/// Priority matrix quadrant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EisenhowerQuadrant {
    /// Urgent & Important - Do first
    DoFirst,
    /// Not Urgent & Important - Schedule
    Schedule,
    /// Urgent & Not Important - Delegate
    Delegate,
    /// Not Urgent & Not Important - Eliminate
    Eliminate,
}

/// Prioritization result
#[derive(Debug, Clone)]
pub struct PrioritizationResult {
    /// Ordered items
    pub ordered: Vec<u64>,
    /// Priority scores
    pub scores: BTreeMap<u64, f64>,
    /// Method used
    pub method: PriorityMethod,
    /// Timestamp
    pub timestamp: Timestamp,
}

// ============================================================================
// PRIORITIZER
// ============================================================================

/// Decision prioritizer
pub struct DecisionPrioritizer {
    /// Items
    items: BTreeMap<u64, DecisionItem>,
    /// Weights
    weights: PriorityWeights,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: PrioritizerConfig,
    /// Statistics
    stats: PrioritizerStats,
}

/// Priority weights
#[derive(Debug, Clone)]
pub struct PriorityWeights {
    /// Urgency weight
    pub urgency: f64,
    /// Importance weight
    pub importance: f64,
    /// Deadline weight
    pub deadline: f64,
    /// Effort penalty
    pub effort_penalty: f64,
}

impl Default for PriorityWeights {
    fn default() -> Self {
        Self {
            urgency: 0.3,
            importance: 0.4,
            deadline: 0.2,
            effort_penalty: 0.1,
        }
    }
}

/// Configuration
#[derive(Debug, Clone)]
pub struct PrioritizerConfig {
    /// Default method
    pub default_method: PriorityMethod,
    /// Urgency threshold
    pub urgency_threshold: f64,
    /// Importance threshold
    pub importance_threshold: f64,
}

impl Default for PrioritizerConfig {
    fn default() -> Self {
        Self {
            default_method: PriorityMethod::Weighted,
            urgency_threshold: 0.7,
            importance_threshold: 0.7,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct PrioritizerStats {
    /// Items added
    pub items_added: u64,
    /// Prioritizations
    pub prioritizations: u64,
    /// Items completed
    pub items_completed: u64,
}

impl DecisionPrioritizer {
    /// Create new prioritizer
    pub fn new(config: PrioritizerConfig, weights: PriorityWeights) -> Self {
        Self {
            items: BTreeMap::new(),
            weights,
            next_id: AtomicU64::new(1),
            config,
            stats: PrioritizerStats::default(),
        }
    }

    /// Add item
    pub fn add(&mut self, name: &str, urgency: f64, importance: f64, effort: f64) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Timestamp::now();

        let item = DecisionItem {
            id,
            name: name.into(),
            priority: 0.0, // Will be calculated
            urgency: urgency.clamp(0.0, 1.0),
            importance: importance.clamp(0.0, 1.0),
            effort: effort.clamp(0.0, 1.0),
            deadline: None,
            dependencies: Vec::new(),
            status: ItemStatus::Pending,
            created: now,
        };

        self.items.insert(id, item);
        self.stats.items_added += 1;

        id
    }

    /// Set deadline
    pub fn set_deadline(&mut self, id: u64, deadline: Timestamp) {
        if let Some(item) = self.items.get_mut(&id) {
            item.deadline = Some(deadline);
        }
    }

    /// Add dependency
    pub fn add_dependency(&mut self, id: u64, dependency: u64) {
        if let Some(item) = self.items.get_mut(&id) {
            if !item.dependencies.contains(&dependency) {
                item.dependencies.push(dependency);
            }
        }
    }

    /// Update status
    pub fn set_status(&mut self, id: u64, status: ItemStatus) {
        if let Some(item) = self.items.get_mut(&id) {
            item.status = status;

            if status == ItemStatus::Completed {
                self.stats.items_completed += 1;

                // Unblock dependents
                self.update_blocked_items(id);
            }
        }
    }

    fn update_blocked_items(&mut self, completed_id: u64) {
        let to_update: Vec<u64> = self.items.iter()
            .filter(|(_, item)| {
                item.status == ItemStatus::Blocked &&
                item.dependencies.contains(&completed_id)
            })
            .map(|(&id, _)| id)
            .collect();

        for id in to_update {
            if let Some(item) = self.items.get_mut(&id) {
                // Check if all dependencies are completed
                let all_done = item.dependencies.iter()
                    .all(|dep_id| {
                        self.items.get(dep_id)
                            .map(|d| d.status == ItemStatus::Completed)
                            .unwrap_or(true)
                    });

                if all_done {
                    item.status = ItemStatus::Ready;
                }
            }
        }
    }

    /// Prioritize
    pub fn prioritize(&mut self, method: PriorityMethod) -> PrioritizationResult {
        self.stats.prioritizations += 1;

        let mut scores = BTreeMap::new();

        // Calculate scores
        for (&id, item) in &self.items {
            if item.status == ItemStatus::Completed || item.status == ItemStatus::Blocked {
                continue;
            }

            let score = self.calculate_score(item, method);
            scores.insert(id, score);
        }

        // Update items
        for (&id, &score) in &scores {
            if let Some(item) = self.items.get_mut(&id) {
                item.priority = score;
            }
        }

        // Sort
        let mut ordered: Vec<(u64, f64)> = scores.iter()
            .map(|(&id, &score)| (id, score))
            .collect();

        ordered.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));

        PrioritizationResult {
            ordered: ordered.into_iter().map(|(id, _)| id).collect(),
            scores,
            method,
            timestamp: Timestamp::now(),
        }
    }

    fn calculate_score(&self, item: &DecisionItem, method: PriorityMethod) -> f64 {
        match method {
            PriorityMethod::Eisenhower => {
                item.urgency * item.importance
            }

            PriorityMethod::WeightedShortestJob => {
                if item.effort > 0.0 {
                    item.importance / item.effort
                } else {
                    item.importance * 10.0
                }
            }

            PriorityMethod::EarliestDeadline => {
                if let Some(deadline) = item.deadline {
                    let now = Timestamp::now().0;
                    let remaining = deadline.0.saturating_sub(now) as f64;
                    1.0 / (remaining + 1.0) * 1e12
                } else {
                    0.0
                }
            }

            PriorityMethod::FirstComeFirstServed => {
                1.0 / (item.created.0 as f64 + 1.0) * 1e18
            }

            PriorityMethod::ImpactFirst => {
                item.importance * (1.0 + item.urgency)
            }

            PriorityMethod::Weighted => {
                let urgency_component = self.weights.urgency * item.urgency;
                let importance_component = self.weights.importance * item.importance;

                let deadline_component = if let Some(deadline) = item.deadline {
                    let now = Timestamp::now().0;
                    let remaining = deadline.0.saturating_sub(now) as f64;
                    self.weights.deadline / (remaining + 1.0) * 1e12
                } else {
                    0.0
                };

                let effort_penalty = self.weights.effort_penalty * item.effort;

                urgency_component + importance_component + deadline_component - effort_penalty
            }
        }
    }

    /// Classify into Eisenhower quadrant
    pub fn classify_eisenhower(&self, id: u64) -> Option<EisenhowerQuadrant> {
        let item = self.items.get(&id)?;

        let urgent = item.urgency >= self.config.urgency_threshold;
        let important = item.importance >= self.config.importance_threshold;

        Some(match (urgent, important) {
            (true, true) => EisenhowerQuadrant::DoFirst,
            (false, true) => EisenhowerQuadrant::Schedule,
            (true, false) => EisenhowerQuadrant::Delegate,
            (false, false) => EisenhowerQuadrant::Eliminate,
        })
    }

    /// Get by quadrant
    pub fn by_quadrant(&self, quadrant: EisenhowerQuadrant) -> Vec<&DecisionItem> {
        self.items.values()
            .filter(|item| {
                let urgent = item.urgency >= self.config.urgency_threshold;
                let important = item.importance >= self.config.importance_threshold;

                match quadrant {
                    EisenhowerQuadrant::DoFirst => urgent && important,
                    EisenhowerQuadrant::Schedule => !urgent && important,
                    EisenhowerQuadrant::Delegate => urgent && !important,
                    EisenhowerQuadrant::Eliminate => !urgent && !important,
                }
            })
            .collect()
    }

    /// Get next item
    pub fn next(&mut self) -> Option<&DecisionItem> {
        let result = self.prioritize(self.config.default_method);
        result.ordered.first().and_then(|id| self.items.get(id))
    }

    /// Get item
    pub fn get(&self, id: u64) -> Option<&DecisionItem> {
        self.items.get(&id)
    }

    /// Get pending items
    pub fn pending(&self) -> Vec<&DecisionItem> {
        self.items.values()
            .filter(|item| item.status == ItemStatus::Pending || item.status == ItemStatus::Ready)
            .collect()
    }

    /// Get statistics
    pub fn stats(&self) -> &PrioritizerStats {
        &self.stats
    }
}

impl Default for DecisionPrioritizer {
    fn default() -> Self {
        Self::new(PrioritizerConfig::default(), PriorityWeights::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_item() {
        let mut prioritizer = DecisionPrioritizer::default();

        let id = prioritizer.add("task", 0.8, 0.9, 0.3);
        assert!(prioritizer.get(id).is_some());
    }

    #[test]
    fn test_prioritize_eisenhower() {
        let mut prioritizer = DecisionPrioritizer::default();

        prioritizer.add("high_both", 0.9, 0.9, 0.5);
        prioritizer.add("low_both", 0.2, 0.2, 0.5);

        let result = prioritizer.prioritize(PriorityMethod::Eisenhower);

        assert!(!result.ordered.is_empty());
        // High urgency * importance should be first
    }

    #[test]
    fn test_prioritize_wsj() {
        let mut prioritizer = DecisionPrioritizer::default();

        prioritizer.add("high_value_low_effort", 0.5, 0.9, 0.1);
        prioritizer.add("low_value_high_effort", 0.5, 0.3, 0.9);

        let result = prioritizer.prioritize(PriorityMethod::WeightedShortestJob);

        assert!(!result.ordered.is_empty());
    }

    #[test]
    fn test_eisenhower_quadrant() {
        let mut prioritizer = DecisionPrioritizer::default();

        let id = prioritizer.add("urgent_important", 0.9, 0.9, 0.5);
        let quadrant = prioritizer.classify_eisenhower(id).unwrap();

        assert_eq!(quadrant, EisenhowerQuadrant::DoFirst);
    }

    #[test]
    fn test_by_quadrant() {
        let mut prioritizer = DecisionPrioritizer::default();

        prioritizer.add("do_first", 0.9, 0.9, 0.5);
        prioritizer.add("schedule", 0.2, 0.9, 0.5);
        prioritizer.add("delegate", 0.9, 0.2, 0.5);

        let do_first = prioritizer.by_quadrant(EisenhowerQuadrant::DoFirst);
        assert_eq!(do_first.len(), 1);

        let schedule = prioritizer.by_quadrant(EisenhowerQuadrant::Schedule);
        assert_eq!(schedule.len(), 1);
    }

    #[test]
    fn test_dependencies() {
        let mut prioritizer = DecisionPrioritizer::default();

        let a = prioritizer.add("first", 0.9, 0.9, 0.5);
        let b = prioritizer.add("second", 0.9, 0.9, 0.5);

        prioritizer.add_dependency(b, a);
        prioritizer.set_status(b, ItemStatus::Blocked);

        prioritizer.set_status(a, ItemStatus::Completed);

        let item_b = prioritizer.get(b).unwrap();
        assert_eq!(item_b.status, ItemStatus::Ready);
    }
}
