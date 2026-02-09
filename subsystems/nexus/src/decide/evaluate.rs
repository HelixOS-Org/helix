//! # Decision Evaluation
//!
//! Evaluates decision options against criteria and constraints.
//! Supports multi-criteria decision analysis (MCDA).
//!
//! Part of Year 2 COGNITION - Decision-Making Engine

#![allow(dead_code)]

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// EVALUATION TYPES
// ============================================================================

/// Decision context
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct DecisionContext {
    /// Context ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Options to evaluate
    pub options: Vec<Option_>,
    /// Criteria
    pub criteria: Vec<Criterion>,
    /// Constraints
    pub constraints: Vec<Constraint>,
    /// Decision type
    pub decision_type: DecisionType,
    /// Created
    pub created: Timestamp,
}

/// Decision type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionType {
    /// Select best option
    Selection,
    /// Rank all options
    Ranking,
    /// Classify options
    Classification,
    /// Filter options
    Filtering,
}

/// Option to evaluate
#[derive(Debug, Clone)]
pub struct Option_ {
    /// Option ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Description
    pub description: String,
    /// Attribute values
    pub attributes: BTreeMap<String, Value>,
    /// Feasible
    pub feasible: bool,
}

/// Value
#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    Boolean(bool),
    Text(String),
    List(Vec<Value>),
}

/// Criterion
#[derive(Debug, Clone)]
pub struct Criterion {
    /// Criterion ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Weight
    pub weight: f64,
    /// Direction
    pub direction: Direction,
    /// Value type
    pub value_type: ValueType,
    /// Normalization
    pub normalization: Normalization,
}

/// Direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Maximize,
    Minimize,
}

/// Value type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    Numeric,
    Boolean,
    Categorical,
}

/// Normalization method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Normalization {
    MinMax,
    ZScore,
    Sum,
    Max,
    None,
}

/// Constraint
#[derive(Debug, Clone)]
pub struct Constraint {
    /// Constraint ID
    pub id: u64,
    /// Criterion name
    pub criterion: String,
    /// Operator
    pub operator: ConstraintOp,
    /// Threshold
    pub threshold: Value,
    /// Hard constraint (must satisfy)
    pub hard: bool,
}

/// Constraint operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintOp {
    GreaterThan,
    LessThan,
    GreaterEqual,
    LessEqual,
    Equal,
    NotEqual,
}

// ============================================================================
// EVALUATION RESULT
// ============================================================================

/// Evaluation result
#[derive(Debug, Clone)]
pub struct EvaluationResult {
    /// Context ID
    pub context_id: u64,
    /// Scores
    pub scores: Vec<OptionScore>,
    /// Recommended option
    pub recommended: Option<u64>,
    /// Method used
    pub method: EvaluationMethod,
    /// Sensitivity
    pub sensitivity: Option<SensitivityAnalysis>,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Option score
#[derive(Debug, Clone)]
pub struct OptionScore {
    /// Option ID
    pub option_id: u64,
    /// Overall score
    pub overall: f64,
    /// Score by criterion
    pub by_criterion: LinearMap<f64, 64>,
    /// Rank
    pub rank: usize,
    /// Feasible
    pub feasible: bool,
}

/// Evaluation method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvaluationMethod {
    /// Weighted Sum
    WeightedSum,
    /// TOPSIS (Technique for Order of Preference by Similarity to Ideal Solution)
    Topsis,
    /// AHP (Analytic Hierarchy Process)
    Ahp,
    /// Satisficing
    Satisficing,
}

/// Sensitivity analysis
#[derive(Debug, Clone)]
pub struct SensitivityAnalysis {
    /// Weight sensitivity
    pub weight_sensitivity: LinearMap<f64, 64>,
    /// Stable range
    pub stable_ranges: BTreeMap<u64, (f64, f64)>,
}

// ============================================================================
// EVALUATOR
// ============================================================================

/// Decision evaluator
pub struct DecisionEvaluator {
    /// Contexts
    contexts: BTreeMap<u64, DecisionContext>,
    /// Results
    results: BTreeMap<u64, EvaluationResult>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: EvaluatorConfig,
    /// Statistics
    stats: EvaluatorStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct EvaluatorConfig {
    /// Default method
    pub default_method: EvaluationMethod,
    /// Enable sensitivity analysis
    pub sensitivity_analysis: bool,
    /// Tolerance for equality
    pub tolerance: f64,
}

impl Default for EvaluatorConfig {
    fn default() -> Self {
        Self {
            default_method: EvaluationMethod::WeightedSum,
            sensitivity_analysis: true,
            tolerance: 1e-6,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct EvaluatorStats {
    /// Evaluations performed
    pub evaluations: u64,
    /// Options evaluated
    pub options_evaluated: u64,
    /// Criteria processed
    pub criteria_processed: u64,
}

impl DecisionEvaluator {
    /// Create new evaluator
    pub fn new(config: EvaluatorConfig) -> Self {
        Self {
            contexts: BTreeMap::new(),
            results: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: EvaluatorStats::default(),
        }
    }

    /// Create context
    pub fn create_context(&mut self, name: &str, decision_type: DecisionType) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let context = DecisionContext {
            id,
            name: name.into(),
            options: Vec::new(),
            criteria: Vec::new(),
            constraints: Vec::new(),
            decision_type,
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
        attributes: BTreeMap<String, Value>,
    ) -> Option<u64> {
        let context = self.contexts.get_mut(&context_id)?;
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        context.options.push(Option_ {
            id,
            name: name.into(),
            description: description.into(),
            attributes,
            feasible: true,
        });

        Some(id)
    }

    /// Add criterion
    pub fn add_criterion(
        &mut self,
        context_id: u64,
        name: &str,
        weight: f64,
        direction: Direction,
    ) -> Option<u64> {
        let context = self.contexts.get_mut(&context_id)?;
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        context.criteria.push(Criterion {
            id,
            name: name.into(),
            weight: weight.clamp(0.0, 1.0),
            direction,
            value_type: ValueType::Numeric,
            normalization: Normalization::MinMax,
        });

        Some(id)
    }

    /// Add constraint
    pub fn add_constraint(
        &mut self,
        context_id: u64,
        criterion: &str,
        operator: ConstraintOp,
        threshold: Value,
        hard: bool,
    ) {
        if let Some(context) = self.contexts.get_mut(&context_id) {
            let id = self.next_id.fetch_add(1, Ordering::Relaxed);

            context.constraints.push(Constraint {
                id,
                criterion: criterion.into(),
                operator,
                threshold,
                hard,
            });
        }
    }

    /// Evaluate
    #[inline(always)]
    pub fn evaluate(&mut self, context_id: u64) -> Option<EvaluationResult> {
        self.evaluate_with_method(context_id, self.config.default_method)
    }

    /// Evaluate with method
    pub fn evaluate_with_method(
        &mut self,
        context_id: u64,
        method: EvaluationMethod,
    ) -> Option<EvaluationResult> {
        let context = self.contexts.get(&context_id)?.clone();

        // Check constraints
        let mut options = context.options.clone();
        self.apply_constraints(&mut options, &context.constraints);

        // Calculate scores
        let scores = match method {
            EvaluationMethod::WeightedSum => self.weighted_sum(&options, &context.criteria),
            EvaluationMethod::Topsis => self.topsis(&options, &context.criteria),
            EvaluationMethod::Satisficing => self.satisficing(&options, &context.criteria),
            _ => self.weighted_sum(&options, &context.criteria),
        };

        // Find recommended
        let recommended = scores
            .iter()
            .filter(|s| s.feasible)
            .max_by(|a, b| a.overall.partial_cmp(&b.overall).unwrap())
            .map(|s| s.option_id);

        // Sensitivity analysis
        let sensitivity = if self.config.sensitivity_analysis {
            Some(self.sensitivity_analysis(&options, &context.criteria))
        } else {
            None
        };

        self.stats.evaluations += 1;
        self.stats.options_evaluated += options.len() as u64;
        self.stats.criteria_processed += context.criteria.len() as u64;

        let result = EvaluationResult {
            context_id,
            scores,
            recommended,
            method,
            sensitivity,
            timestamp: Timestamp::now(),
        };

        self.results.insert(context_id, result.clone());
        Some(result)
    }

    fn apply_constraints(&self, options: &mut [Option_], constraints: &[Constraint]) {
        for option in options.iter_mut() {
            for constraint in constraints {
                if !constraint.hard {
                    continue;
                }

                if let Some(value) = option.attributes.get(&constraint.criterion) {
                    if !self.satisfies_constraint(value, constraint) {
                        option.feasible = false;
                        break;
                    }
                }
            }
        }
    }

    fn satisfies_constraint(&self, value: &Value, constraint: &Constraint) -> bool {
        match (value, &constraint.threshold) {
            (Value::Number(v), Value::Number(t)) => match constraint.operator {
                ConstraintOp::GreaterThan => *v > *t,
                ConstraintOp::LessThan => *v < *t,
                ConstraintOp::GreaterEqual => *v >= *t,
                ConstraintOp::LessEqual => *v <= *t,
                ConstraintOp::Equal => (*v - *t).abs() < self.config.tolerance,
                ConstraintOp::NotEqual => (*v - *t).abs() >= self.config.tolerance,
            },
            (Value::Boolean(v), Value::Boolean(t)) => match constraint.operator {
                ConstraintOp::Equal => *v == *t,
                ConstraintOp::NotEqual => *v != *t,
                _ => true,
            },
            _ => true,
        }
    }

    fn weighted_sum(&self, options: &[Option_], criteria: &[Criterion]) -> Vec<OptionScore> {
        let normalized = self.normalize_values(options, criteria);

        let mut scores: Vec<OptionScore> = options
            .iter()
            .map(|option| {
                let mut overall = 0.0;
                let mut by_criterion = BTreeMap::new();

                for criterion in criteria {
                    let key = (option.id, criterion.id);
                    let value = normalized.get(&key).copied().unwrap_or(0.0);
                    let weighted = value * criterion.weight;

                    by_criterion.insert(criterion.id, weighted);
                    overall += weighted;
                }

                OptionScore {
                    option_id: option.id,
                    overall,
                    by_criterion,
                    rank: 0,
                    feasible: option.feasible,
                }
            })
            .collect();

        // Assign ranks
        scores.sort_by(|a, b| b.overall.partial_cmp(&a.overall).unwrap());
        for (i, score) in scores.iter_mut().enumerate() {
            score.rank = i + 1;
        }

        scores
    }

    fn topsis(&self, options: &[Option_], criteria: &[Criterion]) -> Vec<OptionScore> {
        let normalized = self.normalize_values(options, criteria);

        // Find ideal and anti-ideal
        let mut ideal: LinearMap<f64, 64> = BTreeMap::new();
        let mut anti_ideal: LinearMap<f64, 64> = BTreeMap::new();

        for criterion in criteria {
            let values: Vec<f64> = options
                .iter()
                .map(|o| {
                    normalized
                        .get(&(o.id, criterion.id))
                        .copied()
                        .unwrap_or(0.0)
                })
                .collect();

            let best = match criterion.direction {
                Direction::Maximize => values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                Direction::Minimize => values.iter().cloned().fold(f64::INFINITY, f64::min),
            };
            let worst = match criterion.direction {
                Direction::Maximize => values.iter().cloned().fold(f64::INFINITY, f64::min),
                Direction::Minimize => values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            };

            ideal.insert(criterion.id, best);
            anti_ideal.insert(criterion.id, worst);
        }

        // Calculate distances
        let mut scores: Vec<OptionScore> = options
            .iter()
            .map(|option| {
                let mut dist_ideal = 0.0;
                let mut dist_anti = 0.0;
                let mut by_criterion = BTreeMap::new();

                for criterion in criteria {
                    let value = normalized
                        .get(&(option.id, criterion.id))
                        .copied()
                        .unwrap_or(0.0);
                    let ideal_val = ideal.get(&criterion.id).copied().unwrap_or(0.0);
                    let anti_val = anti_ideal.get(&criterion.id).copied().unwrap_or(0.0);

                    let weighted = value * criterion.weight;
                    dist_ideal += (weighted - ideal_val * criterion.weight).powi(2);
                    dist_anti += (weighted - anti_val * criterion.weight).powi(2);

                    by_criterion.insert(criterion.id, value);
                }

                dist_ideal = dist_ideal.sqrt();
                dist_anti = dist_anti.sqrt();

                let overall = if dist_ideal + dist_anti > 0.0 {
                    dist_anti / (dist_ideal + dist_anti)
                } else {
                    0.5
                };

                OptionScore {
                    option_id: option.id,
                    overall,
                    by_criterion,
                    rank: 0,
                    feasible: option.feasible,
                }
            })
            .collect();

        // Assign ranks
        scores.sort_by(|a, b| b.overall.partial_cmp(&a.overall).unwrap());
        for (i, score) in scores.iter_mut().enumerate() {
            score.rank = i + 1;
        }

        scores
    }

    fn satisficing(&self, options: &[Option_], criteria: &[Criterion]) -> Vec<OptionScore> {
        // Simple satisficing: first feasible option that meets all criteria
        options
            .iter()
            .enumerate()
            .map(|(i, option)| {
                let feasible = option.feasible;
                let overall = if feasible { 1.0 } else { 0.0 };

                OptionScore {
                    option_id: option.id,
                    overall,
                    by_criterion: LinearMap::new(),
                    rank: i + 1,
                    feasible,
                }
            })
            .collect()
    }

    fn normalize_values(
        &self,
        options: &[Option_],
        criteria: &[Criterion],
    ) -> BTreeMap<(u64, u64), f64> {
        let mut normalized = BTreeMap::new();

        for criterion in criteria {
            // Collect values for this criterion
            let values: Vec<(u64, f64)> = options
                .iter()
                .filter_map(|o| {
                    o.attributes.get(&criterion.name).and_then(|v| match v {
                        Value::Number(n) => Some((o.id, *n)),
                        _ => None,
                    })
                })
                .collect();

            if values.is_empty() {
                continue;
            }

            let nums: Vec<f64> = values.iter().map(|(_, v)| *v).collect();
            let min = nums.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

            for (option_id, value) in values {
                let norm = match criterion.normalization {
                    Normalization::MinMax => {
                        if (max - min).abs() > self.config.tolerance {
                            (value - min) / (max - min)
                        } else {
                            0.5
                        }
                    },
                    Normalization::Max => {
                        if max.abs() > self.config.tolerance {
                            value / max
                        } else {
                            0.0
                        }
                    },
                    Normalization::None => value,
                    _ => (value - min) / (max - min).max(1.0),
                };

                // Invert if minimizing
                let final_val = match criterion.direction {
                    Direction::Maximize => norm,
                    Direction::Minimize => 1.0 - norm,
                };

                normalized.insert((option_id, criterion.id), final_val);
            }
        }

        normalized
    }

    fn sensitivity_analysis(
        &self,
        _options: &[Option_],
        criteria: &[Criterion],
    ) -> SensitivityAnalysis {
        // Simplified sensitivity analysis
        let weight_sensitivity: LinearMap<f64, 64> =
            criteria.iter().map(|c| (c.id, c.weight)).collect();

        let stable_ranges: BTreeMap<u64, (f64, f64)> = criteria
            .iter()
            .map(|c| {
                let lower = (c.weight - 0.1).max(0.0);
                let upper = (c.weight + 0.1).min(1.0);
                (c.id, (lower, upper))
            })
            .collect();

        SensitivityAnalysis {
            weight_sensitivity,
            stable_ranges,
        }
    }

    /// Get context
    #[inline(always)]
    pub fn get_context(&self, id: u64) -> Option<&DecisionContext> {
        self.contexts.get(&id)
    }

    /// Get result
    #[inline(always)]
    pub fn get_result(&self, context_id: u64) -> Option<&EvaluationResult> {
        self.results.get(&context_id)
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &EvaluatorStats {
        &self.stats
    }
}

impl Default for DecisionEvaluator {
    fn default() -> Self {
        Self::new(EvaluatorConfig::default())
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
        let mut eval = DecisionEvaluator::default();
        let id = eval.create_context("test", DecisionType::Selection);

        assert!(eval.get_context(id).is_some());
    }

    #[test]
    fn test_evaluate() {
        let mut eval = DecisionEvaluator::default();
        let ctx = eval.create_context("car", DecisionType::Selection);

        let mut attr1 = BTreeMap::new();
        attr1.insert("price".into(), Value::Number(20000.0));
        attr1.insert("mpg".into(), Value::Number(30.0));
        eval.add_option(ctx, "Car A", "Affordable", attr1);

        let mut attr2 = BTreeMap::new();
        attr2.insert("price".into(), Value::Number(35000.0));
        attr2.insert("mpg".into(), Value::Number(25.0));
        eval.add_option(ctx, "Car B", "Luxury", attr2);

        eval.add_criterion(ctx, "price", 0.6, Direction::Minimize);
        eval.add_criterion(ctx, "mpg", 0.4, Direction::Maximize);

        let result = eval.evaluate(ctx).unwrap();
        assert!(result.recommended.is_some());
    }

    #[test]
    fn test_constraints() {
        let mut eval = DecisionEvaluator::default();
        let ctx = eval.create_context("test", DecisionType::Selection);

        let mut attr = BTreeMap::new();
        attr.insert("budget".into(), Value::Number(50000.0));
        eval.add_option(ctx, "Opt1", "Over budget", attr);

        eval.add_constraint(
            ctx,
            "budget",
            ConstraintOp::LessEqual,
            Value::Number(30000.0),
            true,
        );

        let result = eval.evaluate(ctx).unwrap();

        // Option should be infeasible
        assert!(!result.scores[0].feasible);
    }

    #[test]
    fn test_topsis() {
        let mut eval = DecisionEvaluator::default();
        let ctx = eval.create_context("test", DecisionType::Selection);

        let mut attr1 = BTreeMap::new();
        attr1.insert("score".into(), Value::Number(80.0));
        eval.add_option(ctx, "A", "", attr1);

        let mut attr2 = BTreeMap::new();
        attr2.insert("score".into(), Value::Number(90.0));
        eval.add_option(ctx, "B", "", attr2);

        eval.add_criterion(ctx, "score", 1.0, Direction::Maximize);

        let result = eval
            .evaluate_with_method(ctx, EvaluationMethod::Topsis)
            .unwrap();
        assert!(!result.scores.is_empty());
    }
}
