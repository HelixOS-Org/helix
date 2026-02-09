//! Safe learning with constraints
//!
//! This module provides bounded exploration with safety constraints.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::types::Timestamp;

/// Safety constraint type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintType {
    /// Resource limit
    ResourceLimit,
    /// Action prohibition
    ActionProhibition,
    /// State boundary
    StateBoundary,
    /// Timing constraint
    TimingConstraint,
    /// Invariant
    Invariant,
}

/// Safety constraint
#[derive(Debug, Clone)]
pub struct SafetyConstraint {
    /// Constraint name
    pub name: String,
    /// Constraint type
    pub constraint_type: ConstraintType,
    /// Condition expression
    pub condition: String,
    /// Violation severity (1-10)
    pub severity: u8,
    /// Is hard constraint (cannot be violated)
    pub is_hard: bool,
    /// Violation count
    pub violations: u64,
}

impl SafetyConstraint {
    /// Create new constraint
    pub fn new(name: String, constraint_type: ConstraintType) -> Self {
        Self {
            name,
            constraint_type,
            condition: String::new(),
            severity: 5,
            is_hard: false,
            violations: 0,
        }
    }

    /// With condition
    #[inline(always)]
    pub fn with_condition(mut self, condition: String) -> Self {
        self.condition = condition;
        self
    }

    /// As hard constraint
    #[inline]
    pub fn as_hard(mut self) -> Self {
        self.is_hard = true;
        self.severity = 10;
        self
    }
}

/// Exploration policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplorationPolicy {
    /// Epsilon-greedy
    EpsilonGreedy,
    /// UCB (Upper Confidence Bound)
    UCB,
    /// Thompson sampling
    ThompsonSampling,
    /// Safe exploration
    SafeExploration,
}

/// Safety violation record
#[derive(Debug, Clone)]
pub struct SafetyViolation {
    /// Constraint that was violated
    pub constraint_name: String,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Action that caused violation
    pub action: String,
    /// Severity
    pub severity: u8,
}

/// Safe learning manager
pub struct SafeLearner {
    /// Safety constraints
    constraints: Vec<SafetyConstraint>,
    /// Exploration policy
    policy: ExplorationPolicy,
    /// Epsilon for epsilon-greedy
    epsilon: f32,
    /// Safety violations
    violations: Vec<SafetyViolation>,
    /// Is learning enabled
    learning_enabled: AtomicBool,
    /// Exploration budget
    exploration_budget: u64,
    /// Explorations performed
    explorations: AtomicU64,
}

impl SafeLearner {
    /// Create new safe learner
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
            policy: ExplorationPolicy::SafeExploration,
            epsilon: 0.1,
            violations: Vec::new(),
            learning_enabled: AtomicBool::new(true),
            exploration_budget: 1000,
            explorations: AtomicU64::new(0),
        }
    }

    /// Add constraint
    #[inline(always)]
    pub fn add_constraint(&mut self, constraint: SafetyConstraint) {
        self.constraints.push(constraint);
    }

    /// Set policy
    #[inline(always)]
    pub fn set_policy(&mut self, policy: ExplorationPolicy) {
        self.policy = policy;
    }

    /// Set epsilon
    #[inline(always)]
    pub fn set_epsilon(&mut self, epsilon: f32) {
        self.epsilon = epsilon.clamp(0.0, 1.0);
    }

    /// Check if action is safe
    pub fn is_safe(&self, action: &str, context: &BTreeMap<String, String>) -> bool {
        // Check all hard constraints
        for constraint in &self.constraints {
            if constraint.is_hard {
                // Simplified check - would evaluate condition properly
                if !self.evaluate_constraint(constraint, action, context) {
                    return false;
                }
            }
        }
        true
    }

    /// Evaluate constraint
    fn evaluate_constraint(
        &self,
        constraint: &SafetyConstraint,
        action: &str,
        _context: &BTreeMap<String, String>,
    ) -> bool {
        // Simplified evaluation
        match constraint.constraint_type {
            ConstraintType::ActionProhibition => {
                // Check if action is prohibited
                !constraint.condition.contains(action)
            },
            ConstraintType::ResourceLimit => {
                // Would check resource usage
                true
            },
            _ => true,
        }
    }

    /// Record violation
    pub fn record_violation(&mut self, constraint_name: &str, action: &str, timestamp: u64) {
        if let Some(c) = self
            .constraints
            .iter_mut()
            .find(|c| c.name == constraint_name)
        {
            c.violations += 1;
        }

        self.violations.push(SafetyViolation {
            constraint_name: String::from(constraint_name),
            timestamp: Timestamp::new(timestamp),
            action: String::from(action),
            severity: self
                .constraints
                .iter()
                .find(|c| c.name == constraint_name)
                .map(|c| c.severity)
                .unwrap_or(5),
        });

        // Disable learning if too many violations
        if self.violations.len() > 100 {
            self.learning_enabled.store(false, Ordering::Relaxed);
        }
    }

    /// Should explore
    pub fn should_explore(&self) -> bool {
        if !self.learning_enabled.load(Ordering::Relaxed) {
            return false;
        }

        if self.explorations.load(Ordering::Relaxed) >= self.exploration_budget {
            return false;
        }

        match self.policy {
            ExplorationPolicy::EpsilonGreedy => {
                // Would use random here
                false
            },
            ExplorationPolicy::SafeExploration => {
                // More conservative
                self.violations.is_empty()
            },
            _ => true,
        }
    }

    /// Record exploration
    #[inline(always)]
    pub fn record_exploration(&self) {
        self.explorations.fetch_add(1, Ordering::Relaxed);
    }

    /// Constraint count
    #[inline(always)]
    pub fn constraint_count(&self) -> usize {
        self.constraints.len()
    }

    /// Violation count
    #[inline(always)]
    pub fn violation_count(&self) -> usize {
        self.violations.len()
    }
}

impl Default for SafeLearner {
    fn default() -> Self {
        Self::new()
    }
}
