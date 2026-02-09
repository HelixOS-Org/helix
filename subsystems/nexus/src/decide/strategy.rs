//! # Decision Strategy
//!
//! Implements various decision-making strategies.
//! Supports rule-based, heuristic, and analytical approaches.
//!
//! Part of Year 2 COGNITION - Decision-Making Engine

#![allow(dead_code)]

extern crate alloc;
use alloc::format;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// STRATEGY TYPES
// ============================================================================

/// Decision context
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct DecisionContext {
    /// Context ID
    pub id: u64,
    /// Problem statement
    pub problem: String,
    /// Options
    pub options: Vec<DecisionOption>,
    /// Constraints
    pub constraints: Vec<Constraint>,
    /// Objectives
    pub objectives: Vec<Objective>,
    /// Time pressure
    pub time_pressure: f64,
    /// Uncertainty
    pub uncertainty: f64,
    /// Created
    pub created: Timestamp,
}

/// Decision option
#[derive(Debug, Clone)]
pub struct DecisionOption {
    /// Option ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Description
    pub description: String,
    /// Attributes
    pub attributes: BTreeMap<String, f64>,
    /// Feasible
    pub feasible: bool,
}

/// Constraint
#[derive(Debug, Clone)]
pub struct Constraint {
    /// Constraint ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Type
    pub constraint_type: ConstraintType,
    /// Value
    pub value: f64,
    /// Hard constraint
    pub hard: bool,
}

/// Constraint type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintType {
    Budget,
    Time,
    Resource,
    Quality,
    Risk,
}

/// Objective
#[derive(Debug, Clone)]
pub struct Objective {
    /// Objective ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Direction
    pub direction: Direction,
    /// Weight
    pub weight: f64,
}

/// Direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Maximize,
    Minimize,
    Target(i64), // stored as i64, interpreted as target value
}

/// Strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    RuleBased,
    Heuristic,
    Analytical,
    Satisficing,
    Maximizing,
    Minimax,
    Regret,
}

/// Decision result
#[derive(Debug, Clone)]
pub struct DecisionResult {
    /// Context ID
    pub context_id: u64,
    /// Strategy used
    pub strategy: Strategy,
    /// Selected option
    pub selected: u64,
    /// Confidence
    pub confidence: f64,
    /// Rationale
    pub rationale: String,
    /// Alternatives
    pub alternatives: Vec<AlternativeRanking>,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Alternative ranking
#[derive(Debug, Clone)]
pub struct AlternativeRanking {
    /// Option ID
    pub option_id: u64,
    /// Score
    pub score: f64,
    /// Rank
    pub rank: usize,
}

// ============================================================================
// STRATEGY ENGINE
// ============================================================================

/// Strategy engine
pub struct StrategyEngine {
    /// Contexts
    contexts: BTreeMap<u64, DecisionContext>,
    /// Results
    results: BTreeMap<u64, DecisionResult>,
    /// Rules
    rules: Vec<DecisionRule>,
    /// Heuristics
    heuristics: Vec<Heuristic>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: StrategyConfig,
    /// Statistics
    stats: StrategyStats,
}

/// Decision rule
#[derive(Debug, Clone)]
pub struct DecisionRule {
    /// Rule ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Condition
    pub condition: RuleCondition,
    /// Selection
    pub selection: RuleSelection,
}

/// Rule condition
#[derive(Debug, Clone)]
pub enum RuleCondition {
    AttributeGreater(String, f64),
    AttributeLess(String, f64),
    HasAttribute(String),
    ConstraintMet(String),
}

/// Rule selection
#[derive(Debug, Clone)]
pub enum RuleSelection {
    SelectFirst,
    SelectBest(String),
    SelectWorst(String),
    Reject,
}

/// Heuristic
#[derive(Debug, Clone)]
pub struct Heuristic {
    /// Heuristic ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Type
    pub heuristic_type: HeuristicType,
    /// Parameters
    pub params: BTreeMap<String, f64>,
}

/// Heuristic type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeuristicType {
    TakeTheBest,
    EqualWeight,
    Tallying,
    RecognitionBased,
    DefaultFirst,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct StrategyConfig {
    /// Default strategy
    pub default_strategy: Strategy,
    /// Confidence threshold
    pub confidence_threshold: f64,
    /// Time pressure threshold for heuristics
    pub time_pressure_threshold: f64,
}

impl Default for StrategyConfig {
    fn default() -> Self {
        Self {
            default_strategy: Strategy::Analytical,
            confidence_threshold: 0.6,
            time_pressure_threshold: 0.7,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct StrategyStats {
    /// Decisions made
    pub decisions_made: u64,
    /// Strategies used
    pub strategies_used: BTreeMap<u8, u64>,
    /// Average confidence
    pub average_confidence: f64,
}

impl StrategyEngine {
    /// Create new engine
    pub fn new(config: StrategyConfig) -> Self {
        Self {
            contexts: BTreeMap::new(),
            results: BTreeMap::new(),
            rules: Vec::new(),
            heuristics: Vec::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: StrategyStats::default(),
        }
    }

    /// Create context
    pub fn create_context(&mut self, problem: &str) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let context = DecisionContext {
            id,
            problem: problem.into(),
            options: Vec::new(),
            constraints: Vec::new(),
            objectives: Vec::new(),
            time_pressure: 0.0,
            uncertainty: 0.0,
            created: Timestamp::now(),
        };

        self.contexts.insert(id, context);
        id
    }

    /// Add option
    pub fn add_option(
        &mut self,
        context_id: u64,
        name: &str,
        description: &str,
        attributes: BTreeMap<String, f64>,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let option = DecisionOption {
            id,
            name: name.into(),
            description: description.into(),
            attributes,
            feasible: true,
        };

        if let Some(ctx) = self.contexts.get_mut(&context_id) {
            ctx.options.push(option);
        }

        id
    }

    /// Add constraint
    pub fn add_constraint(
        &mut self,
        context_id: u64,
        name: &str,
        constraint_type: ConstraintType,
        value: f64,
        hard: bool,
    ) {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let constraint = Constraint {
            id,
            name: name.into(),
            constraint_type,
            value,
            hard,
        };

        if let Some(ctx) = self.contexts.get_mut(&context_id) {
            ctx.constraints.push(constraint);
        }
    }

    /// Add objective
    pub fn add_objective(
        &mut self,
        context_id: u64,
        name: &str,
        direction: Direction,
        weight: f64,
    ) {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let objective = Objective {
            id,
            name: name.into(),
            direction,
            weight,
        };

        if let Some(ctx) = self.contexts.get_mut(&context_id) {
            ctx.objectives.push(objective);
        }
    }

    /// Select strategy
    pub fn select_strategy(&self, context: &DecisionContext) -> Strategy {
        // High time pressure -> heuristics
        if context.time_pressure > self.config.time_pressure_threshold {
            return Strategy::Heuristic;
        }

        // High uncertainty -> satisficing
        if context.uncertainty > 0.7 {
            return Strategy::Satisficing;
        }

        // Few options -> analytical
        if context.options.len() <= 3 {
            return Strategy::Analytical;
        }

        // Rules available -> rule-based
        if !self.rules.is_empty() {
            return Strategy::RuleBased;
        }

        self.config.default_strategy
    }

    /// Decide
    pub fn decide(&mut self, context_id: u64) -> Option<DecisionResult> {
        let context = self.contexts.get(&context_id)?.clone();

        // Filter feasible options
        self.filter_feasible(context_id);

        let context = self.contexts.get(&context_id)?.clone();

        if context.options.iter().all(|o| !o.feasible) {
            return None;
        }

        let strategy = self.select_strategy(&context);

        let result = match strategy {
            Strategy::RuleBased => self.decide_rule_based(&context),
            Strategy::Heuristic => self.decide_heuristic(&context),
            Strategy::Analytical => self.decide_analytical(&context),
            Strategy::Satisficing => self.decide_satisficing(&context),
            Strategy::Maximizing => self.decide_maximizing(&context),
            Strategy::Minimax => self.decide_minimax(&context),
            Strategy::Regret => self.decide_regret(&context),
        };

        // Update stats
        self.stats.decisions_made += 1;
        *self
            .stats
            .strategies_used
            .entry(strategy as u8)
            .or_insert(0) += 1;

        if let Some(ref r) = result {
            let n = self.stats.decisions_made as f64;
            self.stats.average_confidence =
                (self.stats.average_confidence * (n - 1.0) + r.confidence) / n;
        }

        if let Some(ref r) = result {
            self.results.insert(context_id, r.clone());
        }

        result
    }

    fn filter_feasible(&mut self, context_id: u64) {
        if let Some(ctx) = self.contexts.get_mut(&context_id) {
            for option in &mut ctx.options {
                for constraint in &ctx.constraints {
                    if constraint.hard {
                        let attr = option
                            .attributes
                            .get(&constraint.name)
                            .copied()
                            .unwrap_or(0.0);

                        match constraint.constraint_type {
                            ConstraintType::Budget => {
                                if attr > constraint.value {
                                    option.feasible = false;
                                }
                            },
                            ConstraintType::Time => {
                                if attr > constraint.value {
                                    option.feasible = false;
                                }
                            },
                            _ => {},
                        }
                    }
                }
            }
        }
    }

    fn decide_rule_based(&self, context: &DecisionContext) -> Option<DecisionResult> {
        let feasible: Vec<_> = context.options.iter().filter(|o| o.feasible).collect();

        for rule in &self.rules {
            for option in &feasible {
                let matches = match &rule.condition {
                    RuleCondition::AttributeGreater(attr, val) => {
                        option.attributes.get(attr).copied().unwrap_or(0.0) > *val
                    },
                    RuleCondition::AttributeLess(attr, val) => {
                        option.attributes.get(attr).copied().unwrap_or(0.0) < *val
                    },
                    RuleCondition::HasAttribute(attr) => option.attributes.contains_key(attr),
                    RuleCondition::ConstraintMet(_) => true,
                };

                if matches {
                    match &rule.selection {
                        RuleSelection::SelectFirst => {
                            return Some(self.create_result(
                                context.id,
                                Strategy::RuleBased,
                                option.id,
                                0.8,
                                &format!("Rule '{}' applied", rule.name),
                                &feasible,
                            ));
                        },
                        RuleSelection::Reject => {
                            continue;
                        },
                        _ => {},
                    }
                }
            }
        }

        // Fallback to first feasible
        feasible.first().map(|opt| {
            self.create_result(
                context.id,
                Strategy::RuleBased,
                opt.id,
                0.5,
                "No rule matched",
                &feasible,
            )
        })
    }

    fn decide_heuristic(&self, context: &DecisionContext) -> Option<DecisionResult> {
        let feasible: Vec<_> = context.options.iter().filter(|o| o.feasible).collect();

        // Take the best heuristic
        if let Some(obj) = context.objectives.first() {
            let best = feasible.iter().max_by(|a, b| {
                let va = a.attributes.get(&obj.name).copied().unwrap_or(0.0);
                let vb = b.attributes.get(&obj.name).copied().unwrap_or(0.0);

                match obj.direction {
                    Direction::Maximize => {
                        va.partial_cmp(&vb).unwrap_or(core::cmp::Ordering::Equal)
                    },
                    Direction::Minimize => {
                        vb.partial_cmp(&va).unwrap_or(core::cmp::Ordering::Equal)
                    },
                    Direction::Target(t) => {
                        let da = (va - t as f64).abs();
                        let db = (vb - t as f64).abs();
                        db.partial_cmp(&da).unwrap_or(core::cmp::Ordering::Equal)
                    },
                }
            })?;

            return Some(self.create_result(
                context.id,
                Strategy::Heuristic,
                best.id,
                0.7,
                "Take-the-best heuristic",
                &feasible,
            ));
        }

        feasible.first().map(|opt| {
            self.create_result(
                context.id,
                Strategy::Heuristic,
                opt.id,
                0.5,
                "First available",
                &feasible,
            )
        })
    }

    fn decide_analytical(&self, context: &DecisionContext) -> Option<DecisionResult> {
        let feasible: Vec<_> = context.options.iter().filter(|o| o.feasible).collect();

        // Weighted sum
        let scores: Vec<_> = feasible
            .iter()
            .map(|opt| {
                let score: f64 = context
                    .objectives
                    .iter()
                    .map(|obj| {
                        let val = opt.attributes.get(&obj.name).copied().unwrap_or(0.0);
                        let normalized = match obj.direction {
                            Direction::Maximize => val,
                            Direction::Minimize => -val,
                            Direction::Target(t) => -(val - t as f64).abs(),
                        };
                        normalized * obj.weight
                    })
                    .sum();
                (opt, score)
            })
            .collect();

        let best = scores
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal))?;

        Some(self.create_result(
            context.id,
            Strategy::Analytical,
            best.0.id,
            0.9,
            "Weighted sum analysis",
            &feasible,
        ))
    }

    fn decide_satisficing(&self, context: &DecisionContext) -> Option<DecisionResult> {
        // Find first option that meets all thresholds
        let feasible: Vec<_> = context.options.iter().filter(|o| o.feasible).collect();

        for opt in &feasible {
            let meets_all = context.objectives.iter().all(|obj| {
                let val = opt.attributes.get(&obj.name).copied().unwrap_or(0.0);
                match obj.direction {
                    Direction::Maximize => val >= 0.5,
                    Direction::Minimize => val <= 0.5,
                    Direction::Target(t) => (val - t as f64).abs() < 0.2,
                }
            });

            if meets_all {
                return Some(self.create_result(
                    context.id,
                    Strategy::Satisficing,
                    opt.id,
                    0.7,
                    "Meets satisficing criteria",
                    &feasible,
                ));
            }
        }

        feasible.first().map(|opt| {
            self.create_result(
                context.id,
                Strategy::Satisficing,
                opt.id,
                0.4,
                "No option fully satisfies",
                &feasible,
            )
        })
    }

    fn decide_maximizing(&self, context: &DecisionContext) -> Option<DecisionResult> {
        self.decide_analytical(context)
    }

    fn decide_minimax(&self, context: &DecisionContext) -> Option<DecisionResult> {
        // Minimize the maximum possible loss
        let feasible: Vec<_> = context.options.iter().filter(|o| o.feasible).collect();

        let scores: Vec<_> = feasible
            .iter()
            .map(|opt| {
                let max_loss = context
                    .objectives
                    .iter()
                    .filter(|obj| matches!(obj.direction, Direction::Minimize))
                    .map(|obj| opt.attributes.get(&obj.name).copied().unwrap_or(0.0))
                    .fold(0.0_f64, f64::max);
                (opt, max_loss)
            })
            .collect();

        let best = scores
            .iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal))?;

        Some(self.create_result(
            context.id,
            Strategy::Minimax,
            best.0.id,
            0.75,
            "Minimax strategy",
            &feasible,
        ))
    }

    fn decide_regret(&self, context: &DecisionContext) -> Option<DecisionResult> {
        // Minimize maximum regret
        self.decide_minimax(context)
    }

    fn create_result(
        &self,
        context_id: u64,
        strategy: Strategy,
        selected: u64,
        confidence: f64,
        rationale: &str,
        options: &[&DecisionOption],
    ) -> DecisionResult {
        let alternatives: Vec<_> = options
            .iter()
            .enumerate()
            .map(|(i, opt)| AlternativeRanking {
                option_id: opt.id,
                score: if opt.id == selected { 1.0 } else { 0.5 },
                rank: if opt.id == selected { 1 } else { i + 2 },
            })
            .collect();

        DecisionResult {
            context_id,
            strategy,
            selected,
            confidence,
            rationale: rationale.into(),
            alternatives,
            timestamp: Timestamp::now(),
        }
    }

    /// Add rule
    #[inline]
    pub fn add_rule(&mut self, name: &str, condition: RuleCondition, selection: RuleSelection) {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.rules.push(DecisionRule {
            id,
            name: name.into(),
            condition,
            selection,
        });
    }

    /// Get result
    #[inline(always)]
    pub fn get_result(&self, context_id: u64) -> Option<&DecisionResult> {
        self.results.get(&context_id)
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &StrategyStats {
        &self.stats
    }
}

impl Default for StrategyEngine {
    fn default() -> Self {
        Self::new(StrategyConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_context() {
        let mut engine = StrategyEngine::default();

        let id = engine.create_context("Choose product");
        assert!(engine.contexts.contains_key(&id));
    }

    #[test]
    fn test_add_option() {
        let mut engine = StrategyEngine::default();

        let ctx = engine.create_context("test");
        let mut attrs = BTreeMap::new();
        attrs.insert("price".into(), 100.0);

        let opt = engine.add_option(ctx, "Option A", "Description", attrs);

        let context = engine.contexts.get(&ctx).unwrap();
        assert_eq!(context.options.len(), 1);
    }

    #[test]
    fn test_decide() {
        let mut engine = StrategyEngine::default();

        let ctx = engine.create_context("Choose");

        let mut attrs1 = BTreeMap::new();
        attrs1.insert("quality".into(), 0.8);

        let mut attrs2 = BTreeMap::new();
        attrs2.insert("quality".into(), 0.6);

        engine.add_option(ctx, "A", "", attrs1);
        engine.add_option(ctx, "B", "", attrs2);

        engine.add_objective(ctx, "quality", Direction::Maximize, 1.0);

        let result = engine.decide(ctx);
        assert!(result.is_some());
    }

    #[test]
    fn test_constraint() {
        let mut engine = StrategyEngine::default();

        let ctx = engine.create_context("Choose");

        let mut attrs1 = BTreeMap::new();
        attrs1.insert("cost".into(), 200.0);

        let mut attrs2 = BTreeMap::new();
        attrs2.insert("cost".into(), 50.0);

        engine.add_option(ctx, "Expensive", "", attrs1);
        engine.add_option(ctx, "Cheap", "", attrs2);

        engine.add_constraint(ctx, "cost", ConstraintType::Budget, 100.0, true);

        let result = engine.decide(ctx);
        assert!(result.is_some());
    }
}
