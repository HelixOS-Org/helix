//! # Decision Selection
//!
//! Selects optimal decisions from evaluated options.
//! Implements various selection strategies.
//!
//! Part of Year 2 COGNITION - Decision-Making Engine

#![allow(dead_code)]

extern crate alloc;
use alloc::format;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// SELECTION TYPES
// ============================================================================

/// Decision option
#[derive(Debug, Clone)]
pub struct DecisionOption {
    /// Option ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Description
    pub description: String,
    /// Score
    pub score: f64,
    /// Confidence
    pub confidence: f64,
    /// Risk level
    pub risk: RiskLevel,
    /// Attributes
    pub attributes: BTreeMap<String, f64>,
}

/// Risk level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    VeryLow,
    Low,
    Medium,
    High,
    VeryHigh,
}

/// Selection result
#[derive(Debug, Clone)]
pub struct SelectionResult {
    /// Selected option ID
    pub selected_id: u64,
    /// Strategy used
    pub strategy: SelectionStrategy,
    /// Confidence
    pub confidence: f64,
    /// Reasoning
    pub reasoning: String,
    /// Alternatives
    pub alternatives: Vec<Alternative>,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Alternative
#[derive(Debug, Clone)]
pub struct Alternative {
    /// Option ID
    pub option_id: u64,
    /// Rank
    pub rank: usize,
    /// Score difference from selected
    pub score_diff: f64,
}

/// Selection strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionStrategy {
    /// Maximize score
    Maximize,
    /// Minimize score
    Minimize,
    /// Satisfice (first acceptable)
    Satisfice,
    /// Minimax (minimize worst case)
    Minimax,
    /// Maximin (maximize minimum)
    Maximin,
    /// Risk-adjusted
    RiskAdjusted,
    /// Pareto optimal
    Pareto,
    /// Weighted random
    WeightedRandom,
}

/// Selection constraint
#[derive(Debug, Clone)]
pub struct SelectionConstraint {
    /// Constraint type
    pub constraint_type: ConstraintType,
    /// Attribute
    pub attribute: String,
    /// Threshold
    pub threshold: f64,
}

/// Constraint type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintType {
    Minimum,
    Maximum,
    Equal,
    NotEqual,
}

// ============================================================================
// DECISION SELECTOR
// ============================================================================

/// Decision selector
pub struct DecisionSelector {
    /// Options
    options: BTreeMap<u64, DecisionOption>,
    /// Constraints
    constraints: Vec<SelectionConstraint>,
    /// History
    history: VecDeque<SelectionResult>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: SelectorConfig,
    /// Statistics
    stats: SelectorStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct SelectorConfig {
    /// Default strategy
    pub default_strategy: SelectionStrategy,
    /// Risk tolerance
    pub risk_tolerance: f64,
    /// Satisficing threshold
    pub satisfice_threshold: f64,
    /// Keep history size
    pub history_size: usize,
}

impl Default for SelectorConfig {
    fn default() -> Self {
        Self {
            default_strategy: SelectionStrategy::Maximize,
            risk_tolerance: 0.5,
            satisfice_threshold: 0.7,
            history_size: 100,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct SelectorStats {
    /// Selections made
    pub selections_made: u64,
    /// By strategy
    pub by_strategy: BTreeMap<String, u64>,
}

impl DecisionSelector {
    /// Create new selector
    pub fn new(config: SelectorConfig) -> Self {
        Self {
            options: BTreeMap::new(),
            constraints: Vec::new(),
            history: VecDeque::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: SelectorStats::default(),
        }
    }

    /// Add option
    #[inline(always)]
    pub fn add_option(&mut self, option: DecisionOption) {
        self.options.insert(option.id, option);
    }

    /// Create option
    pub fn create_option(&mut self, name: &str, score: f64) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let option = DecisionOption {
            id,
            name: name.into(),
            description: String::new(),
            score,
            confidence: 1.0,
            risk: RiskLevel::Medium,
            attributes: BTreeMap::new(),
        };

        self.options.insert(id, option);
        id
    }

    /// Add constraint
    #[inline(always)]
    pub fn add_constraint(&mut self, constraint: SelectionConstraint) {
        self.constraints.push(constraint);
    }

    /// Select
    pub fn select(&mut self, strategy: Option<SelectionStrategy>) -> Option<SelectionResult> {
        let strategy = strategy.unwrap_or(self.config.default_strategy);

        // Filter by constraints
        let valid_options: Vec<&DecisionOption> = self
            .options
            .values()
            .filter(|opt| self.satisfies_constraints(opt))
            .collect();

        if valid_options.is_empty() {
            return None;
        }

        let selected = match strategy {
            SelectionStrategy::Maximize => self.select_maximize(&valid_options),
            SelectionStrategy::Minimize => self.select_minimize(&valid_options),
            SelectionStrategy::Satisfice => self.select_satisfice(&valid_options),
            SelectionStrategy::Minimax => self.select_minimax(&valid_options),
            SelectionStrategy::Maximin => self.select_maximin(&valid_options),
            SelectionStrategy::RiskAdjusted => self.select_risk_adjusted(&valid_options),
            SelectionStrategy::Pareto => self.select_pareto(&valid_options),
            SelectionStrategy::WeightedRandom => self.select_weighted_random(&valid_options),
        };

        if let Some(selected_opt) = selected {
            let result = self.create_result(selected_opt, &valid_options, strategy);
            self.record_selection(&result);
            return Some(result);
        }

        None
    }

    fn satisfies_constraints(&self, option: &DecisionOption) -> bool {
        for constraint in &self.constraints {
            let value = option
                .attributes
                .get(&constraint.attribute)
                .copied()
                .unwrap_or(option.score);

            let satisfied = match constraint.constraint_type {
                ConstraintType::Minimum => value >= constraint.threshold,
                ConstraintType::Maximum => value <= constraint.threshold,
                ConstraintType::Equal => (value - constraint.threshold).abs() < f64::EPSILON,
                ConstraintType::NotEqual => (value - constraint.threshold).abs() >= f64::EPSILON,
            };

            if !satisfied {
                return false;
            }
        }
        true
    }

    fn select_maximize<'a>(&self, options: &[&'a DecisionOption]) -> Option<&'a DecisionOption> {
        options
            .iter()
            .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap())
            .copied()
    }

    fn select_minimize<'a>(&self, options: &[&'a DecisionOption]) -> Option<&'a DecisionOption> {
        options
            .iter()
            .min_by(|a, b| a.score.partial_cmp(&b.score).unwrap())
            .copied()
    }

    fn select_satisfice<'a>(&self, options: &[&'a DecisionOption]) -> Option<&'a DecisionOption> {
        options
            .iter()
            .find(|opt| opt.score >= self.config.satisfice_threshold)
            .copied()
            .or_else(|| self.select_maximize(options))
    }

    fn select_minimax<'a>(&self, options: &[&'a DecisionOption]) -> Option<&'a DecisionOption> {
        // Minimize the maximum risk
        options.iter().min_by(|a, b| a.risk.cmp(&b.risk)).copied()
    }

    fn select_maximin<'a>(&self, options: &[&'a DecisionOption]) -> Option<&'a DecisionOption> {
        // Maximize the minimum value across attributes
        options
            .iter()
            .max_by(|a, b| {
                let min_a = a.attributes.values().copied().fold(a.score, f64::min);
                let min_b = b.attributes.values().copied().fold(b.score, f64::min);
                min_a.partial_cmp(&min_b).unwrap()
            })
            .copied()
    }

    fn select_risk_adjusted<'a>(
        &self,
        options: &[&'a DecisionOption],
    ) -> Option<&'a DecisionOption> {
        options
            .iter()
            .max_by(|a, b| {
                let adj_a = self.risk_adjusted_score(a);
                let adj_b = self.risk_adjusted_score(b);
                adj_a.partial_cmp(&adj_b).unwrap()
            })
            .copied()
    }

    fn risk_adjusted_score(&self, option: &DecisionOption) -> f64 {
        let risk_penalty = match option.risk {
            RiskLevel::VeryLow => 0.0,
            RiskLevel::Low => 0.1,
            RiskLevel::Medium => 0.2,
            RiskLevel::High => 0.4,
            RiskLevel::VeryHigh => 0.6,
        };

        option.score * (1.0 - risk_penalty * (1.0 - self.config.risk_tolerance))
    }

    fn select_pareto<'a>(&self, options: &[&'a DecisionOption]) -> Option<&'a DecisionOption> {
        // Find Pareto optimal (not dominated by any other)
        let pareto: Vec<_> = options
            .iter()
            .filter(|opt| {
                !options
                    .iter()
                    .any(|other| other.id != opt.id && self.dominates(other, opt))
            })
            .copied()
            .collect();

        // From Pareto set, select highest score
        pareto
            .iter()
            .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap())
            .copied()
    }

    fn dominates(&self, a: &DecisionOption, b: &DecisionOption) -> bool {
        // a dominates b if a is better in all dimensions
        let a_better_score = a.score >= b.score;
        let a_better_confidence = a.confidence >= b.confidence;
        let a_better_risk = a.risk <= b.risk;

        let a_strictly_better = a.score > b.score || a.confidence > b.confidence || a.risk < b.risk;

        a_better_score && a_better_confidence && a_better_risk && a_strictly_better
    }

    fn select_weighted_random<'a>(
        &self,
        options: &[&'a DecisionOption],
    ) -> Option<&'a DecisionOption> {
        let total: f64 = options.iter().map(|o| o.score.max(0.0)).sum();

        if total <= 0.0 {
            return options.first().copied();
        }

        let r = self.simple_random() * total;
        let mut cumsum = 0.0;

        for opt in options {
            cumsum += opt.score.max(0.0);
            if r <= cumsum {
                return Some(opt);
            }
        }

        options.last().copied()
    }

    fn simple_random(&self) -> f64 {
        let t = Timestamp::now().0;
        ((t % 1000) as f64) / 1000.0
    }

    fn create_result(
        &self,
        selected: &DecisionOption,
        all: &[&DecisionOption],
        strategy: SelectionStrategy,
    ) -> SelectionResult {
        let mut alternatives: Vec<Alternative> = all
            .iter()
            .filter(|opt| opt.id != selected.id)
            .enumerate()
            .map(|(i, opt)| Alternative {
                option_id: opt.id,
                rank: i + 2,
                score_diff: selected.score - opt.score,
            })
            .collect();

        alternatives.sort_by(|a, b| a.score_diff.partial_cmp(&b.score_diff).unwrap());

        SelectionResult {
            selected_id: selected.id,
            strategy,
            confidence: selected.confidence,
            reasoning: format!("Selected {} using {:?} strategy", selected.name, strategy),
            alternatives,
            timestamp: Timestamp::now(),
        }
    }

    fn record_selection(&mut self, result: &SelectionResult) {
        self.history.push_back(result.clone());

        if self.history.len() > self.config.history_size {
            self.history.pop_front();
        }

        self.stats.selections_made += 1;

        let strategy_key = format!("{:?}", result.strategy);
        *self.stats.by_strategy.entry(strategy_key).or_insert(0) += 1;
    }

    /// Clear options
    #[inline(always)]
    pub fn clear(&mut self) {
        self.options.clear();
        self.constraints.clear();
    }

    /// Get option
    #[inline(always)]
    pub fn get_option(&self, id: u64) -> Option<&DecisionOption> {
        self.options.get(&id)
    }

    /// Get history
    #[inline(always)]
    pub fn history(&self) -> &[SelectionResult] {
        &self.history
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &SelectorStats {
        &self.stats
    }
}

impl Default for DecisionSelector {
    fn default() -> Self {
        Self::new(SelectorConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_option() {
        let mut selector = DecisionSelector::default();

        let id = selector.create_option("Option A", 0.8);
        assert!(selector.get_option(id).is_some());
    }

    #[test]
    fn test_maximize() {
        let mut selector = DecisionSelector::default();

        selector.create_option("Low", 0.3);
        selector.create_option("High", 0.9);
        selector.create_option("Mid", 0.5);

        let result = selector.select(Some(SelectionStrategy::Maximize)).unwrap();

        let selected = selector.get_option(result.selected_id).unwrap();
        assert_eq!(selected.score, 0.9);
    }

    #[test]
    fn test_minimize() {
        let mut selector = DecisionSelector::default();

        selector.create_option("High", 0.9);
        selector.create_option("Low", 0.1);
        selector.create_option("Mid", 0.5);

        let result = selector.select(Some(SelectionStrategy::Minimize)).unwrap();

        let selected = selector.get_option(result.selected_id).unwrap();
        assert_eq!(selected.score, 0.1);
    }

    #[test]
    fn test_constraint() {
        let mut selector = DecisionSelector::default();

        selector.create_option("Good", 0.9);
        selector.create_option("Bad", 0.1);

        selector.add_constraint(SelectionConstraint {
            constraint_type: ConstraintType::Minimum,
            attribute: "score".into(),
            threshold: 0.5,
        });

        let result = selector.select(Some(SelectionStrategy::Maximize)).unwrap();

        let selected = selector.get_option(result.selected_id).unwrap();
        assert!(selected.score >= 0.5);
    }

    #[test]
    fn test_satisfice() {
        let mut config = SelectorConfig::default();
        config.satisfice_threshold = 0.7;

        let mut selector = DecisionSelector::new(config);

        selector.create_option("First good enough", 0.75);
        selector.create_option("Best but later", 0.95);

        let result = selector.select(Some(SelectionStrategy::Satisfice)).unwrap();

        let selected = selector.get_option(result.selected_id).unwrap();
        assert_eq!(selected.score, 0.75);
    }

    #[test]
    fn test_risk_adjusted() {
        let mut selector = DecisionSelector::default();

        let opt1 = DecisionOption {
            id: 1,
            name: "High risk".into(),
            description: String::new(),
            score: 0.9,
            confidence: 1.0,
            risk: RiskLevel::VeryHigh,
            attributes: BTreeMap::new(),
        };

        let opt2 = DecisionOption {
            id: 2,
            name: "Low risk".into(),
            description: String::new(),
            score: 0.7,
            confidence: 1.0,
            risk: RiskLevel::Low,
            attributes: BTreeMap::new(),
        };

        selector.add_option(opt1);
        selector.add_option(opt2);

        let result = selector
            .select(Some(SelectionStrategy::RiskAdjusted))
            .unwrap();

        // With default risk tolerance, low risk option should be selected
        let selected = selector.get_option(result.selected_id).unwrap();
        assert_eq!(selected.name, "Low risk");
    }
}
