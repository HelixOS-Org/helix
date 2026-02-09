//! Generalization for rule learning
//!
//! This module provides capabilities to generalize from specific experiences.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::feedback::FeedbackEntry;
use super::types::{ExperienceId, RuleId, Timestamp};

/// Generalization strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneralizationStrategy {
    /// Inductive generalization (specific to general)
    Inductive,
    /// Abductive generalization (inference to best explanation)
    Abductive,
    /// Analogical generalization (similar cases)
    Analogical,
}

/// Condition operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Contains,
    StartsWith,
    EndsWith,
    Matches,
    In,
    NotIn,
}

impl ConditionOp {
    /// Evaluate condition
    pub fn evaluate(&self, actual: &str, expected: &str) -> bool {
        match self {
            Self::Eq => actual == expected,
            Self::Ne => actual != expected,
            Self::Lt => actual < expected,
            Self::Le => actual <= expected,
            Self::Gt => actual > expected,
            Self::Ge => actual >= expected,
            Self::Contains => actual.contains(expected),
            Self::StartsWith => actual.starts_with(expected),
            Self::EndsWith => actual.ends_with(expected),
            Self::Matches => actual == expected, // Simplified
            Self::In => expected.split(',').any(|v| v.trim() == actual),
            Self::NotIn => !expected.split(',').any(|v| v.trim() == actual),
        }
    }
}

/// Rule condition
#[derive(Debug, Clone)]
pub struct RuleCondition {
    /// Variable name
    pub variable: String,
    /// Operator
    pub operator: ConditionOp,
    /// Value
    pub value: String,
}

/// Learned rule
#[derive(Debug, Clone)]
pub struct LearnedRule {
    /// Rule ID
    pub id: RuleId,
    /// Name
    pub name: String,
    /// Conditions (AND)
    pub conditions: Vec<RuleCondition>,
    /// Action to take when conditions match
    pub action: String,
    /// Confidence (0-1)
    pub confidence: f32,
    /// Support (number of examples)
    pub support: u64,
    /// Created from experiences
    pub source_experiences: Vec<ExperienceId>,
    /// Created timestamp
    pub created: Timestamp,
    /// Last validated
    pub last_validated: Timestamp,
    /// Validation count
    pub validation_count: u64,
    /// Success count
    pub success_count: u64,
}

impl LearnedRule {
    /// Create new rule
    pub fn new(id: RuleId, name: String) -> Self {
        Self {
            id,
            name,
            conditions: Vec::new(),
            action: String::new(),
            confidence: 0.0,
            support: 0,
            source_experiences: Vec::new(),
            created: Timestamp::new(0),
            last_validated: Timestamp::new(0),
            validation_count: 0,
            success_count: 0,
        }
    }

    /// Add condition
    #[inline(always)]
    pub fn add_condition(&mut self, condition: RuleCondition) {
        self.conditions.push(condition);
    }

    /// Set action
    #[inline(always)]
    pub fn set_action(&mut self, action: String) {
        self.action = action;
    }

    /// Evaluate against context
    #[inline]
    pub fn evaluate(&self, context: &BTreeMap<String, String>) -> bool {
        self.conditions.iter().all(|cond| {
            context
                .get(&cond.variable)
                .map(|v| cond.operator.evaluate(v, &cond.value))
                .unwrap_or(false)
        })
    }

    /// Record validation
    pub fn record_validation(&mut self, success: bool, timestamp: Timestamp) {
        self.validation_count += 1;
        if success {
            self.success_count += 1;
        }
        self.last_validated = timestamp;

        // Update confidence
        if self.validation_count > 0 {
            self.confidence = self.success_count as f32 / self.validation_count as f32;
        }
    }

    /// Is reliable
    #[inline(always)]
    pub fn is_reliable(&self) -> bool {
        self.confidence >= 0.7 && self.validation_count >= 10
    }
}

/// Generalizer
pub struct Generalizer {
    /// Learned rules
    rules: BTreeMap<RuleId, LearnedRule>,
    /// Rules by action
    by_action: BTreeMap<String, Vec<RuleId>>,
    /// Rule counter
    counter: AtomicU64,
    /// Strategy
    strategy: GeneralizationStrategy,
    /// Minimum support for rule
    min_support: u64,
    /// Minimum confidence
    min_confidence: f32,
}

impl Generalizer {
    /// Create new generalizer
    pub fn new() -> Self {
        Self {
            rules: BTreeMap::new(),
            by_action: BTreeMap::new(),
            counter: AtomicU64::new(0),
            strategy: GeneralizationStrategy::Inductive,
            min_support: 3,
            min_confidence: 0.5,
        }
    }

    /// Set strategy
    #[inline(always)]
    pub fn set_strategy(&mut self, strategy: GeneralizationStrategy) {
        self.strategy = strategy;
    }

    /// Generalize from experiences
    pub fn generalize(&mut self, experiences: &[FeedbackEntry]) -> Vec<RuleId> {
        use super::feedback::FeedbackType;

        let mut created = Vec::new();

        // Group by action
        let mut by_action: BTreeMap<String, Vec<&FeedbackEntry>> = BTreeMap::new();
        for exp in experiences {
            by_action.entry(exp.action.clone()).or_default().push(exp);
        }

        // For each action, find common patterns in positive outcomes
        for (action, entries) in by_action {
            let positive: Vec<_> = entries
                .iter()
                .filter(|e| e.feedback_type == FeedbackType::Positive)
                .collect();

            if positive.len() < self.min_support as usize {
                continue;
            }

            // Find common context variables
            let common = self.find_common_context(&positive);

            if !common.is_empty() {
                let id = self.create_rule(&action, &common, &positive);
                created.push(id);
            }
        }

        created
    }

    /// Find common context variables
    fn find_common_context(&self, entries: &[&&FeedbackEntry]) -> Vec<RuleCondition> {
        if entries.is_empty() {
            return Vec::new();
        }

        let mut common = Vec::new();
        let first = &entries[0].context;

        // For each variable in first entry, check if common
        for (key, value) in first {
            let all_same = entries
                .iter()
                .all(|e| e.context.get(key).map(|v| v == value).unwrap_or(false));

            if all_same {
                common.push(RuleCondition {
                    variable: key.clone(),
                    operator: ConditionOp::Eq,
                    value: value.clone(),
                });
            }
        }

        common
    }

    /// Create rule
    fn create_rule(
        &mut self,
        action: &str,
        conditions: &[RuleCondition],
        sources: &[&&FeedbackEntry],
    ) -> RuleId {
        let id = RuleId(self.counter.fetch_add(1, Ordering::Relaxed));
        let name = alloc::format!("rule_{}", id.0);

        let mut rule = LearnedRule::new(id, name);
        for cond in conditions {
            rule.add_condition(cond.clone());
        }
        rule.set_action(String::from(action));
        rule.support = sources.len() as u64;
        rule.confidence = self.min_confidence;
        rule.source_experiences = sources.iter().map(|e| e.id).collect();

        self.by_action
            .entry(String::from(action))
            .or_default()
            .push(id);

        self.rules.insert(id, rule);

        id
    }

    /// Get rule
    #[inline(always)]
    pub fn get(&self, id: RuleId) -> Option<&LearnedRule> {
        self.rules.get(&id)
    }

    /// Get rule mutably
    #[inline(always)]
    pub fn get_mut(&mut self, id: RuleId) -> Option<&mut LearnedRule> {
        self.rules.get_mut(&id)
    }

    /// Find matching rules
    #[inline]
    pub fn find_matching(&self, context: &BTreeMap<String, String>) -> Vec<&LearnedRule> {
        self.rules
            .values()
            .filter(|r| r.evaluate(context))
            .collect()
    }

    /// Find reliable rules
    #[inline(always)]
    pub fn find_reliable(&self) -> Vec<&LearnedRule> {
        self.rules.values().filter(|r| r.is_reliable()).collect()
    }

    /// Rule count
    #[inline(always)]
    pub fn count(&self) -> usize {
        self.rules.len()
    }
}

impl Default for Generalizer {
    fn default() -> Self {
        Self::new()
    }
}
