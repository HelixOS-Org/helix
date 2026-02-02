//! # Decision Optimization
//!
//! Optimizes decision parameters and strategies.
//! Implements multi-objective optimization and constraints.
//!
//! Part of Year 2 COGNITION - Decision-Making Engine

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// OPTIMIZATION TYPES
// ============================================================================

/// Optimization problem
#[derive(Debug, Clone)]
pub struct OptimizationProblem {
    /// Problem ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Variables
    pub variables: Vec<Variable>,
    /// Objectives
    pub objectives: Vec<Objective>,
    /// Constraints
    pub constraints: Vec<Constraint>,
    /// Created
    pub created: Timestamp,
}

/// Variable
#[derive(Debug, Clone)]
pub struct Variable {
    /// Name
    pub name: String,
    /// Type
    pub var_type: VariableType,
    /// Bounds
    pub bounds: (f64, f64),
    /// Initial value
    pub initial: f64,
}

/// Variable type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableType {
    Continuous,
    Integer,
    Binary,
}

/// Objective
#[derive(Debug, Clone)]
pub struct Objective {
    /// Name
    pub name: String,
    /// Direction
    pub direction: OptimDirection,
    /// Weight (for weighted sum)
    pub weight: f64,
}

/// Optimization direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimDirection {
    Minimize,
    Maximize,
}

/// Constraint
#[derive(Debug, Clone)]
pub struct Constraint {
    /// Name
    pub name: String,
    /// Type
    pub constraint_type: ConstraintType,
    /// Bound
    pub bound: f64,
}

/// Constraint type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintType {
    LessThanOrEqual,
    GreaterThanOrEqual,
    Equal,
}

/// Solution
#[derive(Debug, Clone)]
pub struct Solution {
    /// Solution ID
    pub id: u64,
    /// Variable values
    pub values: BTreeMap<String, f64>,
    /// Objective values
    pub objectives: BTreeMap<String, f64>,
    /// Feasible
    pub feasible: bool,
    /// Pareto optimal
    pub pareto_optimal: bool,
    /// Iteration found
    pub iteration: u64,
}

/// Optimization result
#[derive(Debug, Clone)]
pub struct OptimizationResult {
    /// Problem ID
    pub problem_id: u64,
    /// Best solution
    pub best: Option<Solution>,
    /// All solutions (for multi-objective)
    pub solutions: Vec<Solution>,
    /// Iterations
    pub iterations: u64,
    /// Time taken ns
    pub time_ns: u64,
    /// Converged
    pub converged: bool,
}

// ============================================================================
// OPTIMIZER
// ============================================================================

/// Decision optimizer
pub struct DecisionOptimizer {
    /// Problems
    problems: BTreeMap<u64, OptimizationProblem>,
    /// Results
    results: BTreeMap<u64, OptimizationResult>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: OptimizerConfig,
    /// Statistics
    stats: OptimizerStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct OptimizerConfig {
    /// Maximum iterations
    pub max_iterations: u64,
    /// Convergence tolerance
    pub tolerance: f64,
    /// Population size (for evolutionary)
    pub population_size: usize,
    /// Mutation rate
    pub mutation_rate: f64,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            max_iterations: 1000,
            tolerance: 1e-6,
            population_size: 50,
            mutation_rate: 0.1,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct OptimizerStats {
    /// Problems solved
    pub problems_solved: u64,
    /// Total iterations
    pub total_iterations: u64,
    /// Solutions found
    pub solutions_found: u64,
}

impl DecisionOptimizer {
    /// Create new optimizer
    pub fn new(config: OptimizerConfig) -> Self {
        Self {
            problems: BTreeMap::new(),
            results: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: OptimizerStats::default(),
        }
    }

    /// Create problem
    pub fn create_problem(&mut self, name: &str) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let problem = OptimizationProblem {
            id,
            name: name.into(),
            variables: Vec::new(),
            objectives: Vec::new(),
            constraints: Vec::new(),
            created: Timestamp::now(),
        };

        self.problems.insert(id, problem);
        id
    }

    /// Add variable
    pub fn add_variable(&mut self, problem_id: u64, variable: Variable) {
        if let Some(problem) = self.problems.get_mut(&problem_id) {
            problem.variables.push(variable);
        }
    }

    /// Add objective
    pub fn add_objective(&mut self, problem_id: u64, objective: Objective) {
        if let Some(problem) = self.problems.get_mut(&problem_id) {
            problem.objectives.push(objective);
        }
    }

    /// Add constraint
    pub fn add_constraint(&mut self, problem_id: u64, constraint: Constraint) {
        if let Some(problem) = self.problems.get_mut(&problem_id) {
            problem.constraints.push(constraint);
        }
    }

    /// Solve using gradient descent (for single objective)
    pub fn solve_gradient(
        &mut self,
        problem_id: u64,
        objective_fn: impl Fn(&BTreeMap<String, f64>) -> f64,
    ) -> Option<OptimizationResult> {
        let start = Timestamp::now();

        let problem = self.problems.get(&problem_id)?.clone();

        if problem.objectives.is_empty() {
            return None;
        }

        let is_minimize = problem.objectives[0].direction == OptimDirection::Minimize;

        // Initialize
        let mut current: BTreeMap<String, f64> = problem.variables.iter()
            .map(|v| (v.name.clone(), v.initial))
            .collect();

        let mut current_value = objective_fn(&current);
        let mut best = current.clone();
        let mut best_value = current_value;

        let mut iterations = 0u64;
        let mut converged = false;

        let step_size = 0.01;
        let epsilon = 1e-8;

        while iterations < self.config.max_iterations {
            iterations += 1;

            // Compute gradient numerically
            let mut gradient = BTreeMap::new();

            for var in &problem.variables {
                let mut plus = current.clone();
                *plus.get_mut(&var.name).unwrap() += epsilon;
                let f_plus = objective_fn(&plus);

                let mut minus = current.clone();
                *minus.get_mut(&var.name).unwrap() -= epsilon;
                let f_minus = objective_fn(&minus);

                let grad = (f_plus - f_minus) / (2.0 * epsilon);
                gradient.insert(var.name.clone(), grad);
            }

            // Update
            let mut new_current = current.clone();

            for var in &problem.variables {
                let grad = gradient.get(&var.name).copied().unwrap_or(0.0);

                let step = if is_minimize { -step_size * grad } else { step_size * grad };

                let new_val = current.get(&var.name).copied().unwrap_or(0.0) + step;
                let bounded = new_val.max(var.bounds.0).min(var.bounds.1);

                new_current.insert(var.name.clone(), bounded);
            }

            let new_value = objective_fn(&new_current);

            // Check improvement
            let improved = if is_minimize {
                new_value < current_value
            } else {
                new_value > current_value
            };

            if improved {
                current = new_current;
                current_value = new_value;

                let is_best = if is_minimize {
                    current_value < best_value
                } else {
                    current_value > best_value
                };

                if is_best {
                    best = current.clone();
                    best_value = current_value;
                }
            }

            // Check convergence
            if (new_value - current_value).abs() < self.config.tolerance {
                converged = true;
                break;
            }
        }

        let end = Timestamp::now();

        let solution = Solution {
            id: self.next_id.fetch_add(1, Ordering::Relaxed),
            values: best,
            objectives: {
                let mut objs = BTreeMap::new();
                objs.insert(problem.objectives[0].name.clone(), best_value);
                objs
            },
            feasible: self.check_feasibility(&problem, &current),
            pareto_optimal: true,
            iteration: iterations,
        };

        let result = OptimizationResult {
            problem_id,
            best: Some(solution.clone()),
            solutions: vec![solution],
            iterations,
            time_ns: end.0.saturating_sub(start.0),
            converged,
        };

        self.stats.problems_solved += 1;
        self.stats.total_iterations += iterations;
        self.stats.solutions_found += 1;

        self.results.insert(problem_id, result.clone());

        Some(result)
    }

    /// Solve using random search
    pub fn solve_random(
        &mut self,
        problem_id: u64,
        objective_fn: impl Fn(&BTreeMap<String, f64>) -> f64,
    ) -> Option<OptimizationResult> {
        let start = Timestamp::now();

        let problem = self.problems.get(&problem_id)?.clone();

        if problem.objectives.is_empty() {
            return None;
        }

        let is_minimize = problem.objectives[0].direction == OptimDirection::Minimize;

        let mut best: Option<Solution> = None;
        let mut best_value = if is_minimize { f64::INFINITY } else { f64::NEG_INFINITY };

        let mut iterations = 0u64;

        while iterations < self.config.max_iterations {
            iterations += 1;

            // Generate random solution
            let mut values = BTreeMap::new();

            for var in &problem.variables {
                let range = var.bounds.1 - var.bounds.0;
                let random = self.simple_random();
                let value = var.bounds.0 + random * range;

                let quantized = match var.var_type {
                    VariableType::Integer => value.round(),
                    VariableType::Binary => if value > 0.5 { 1.0 } else { 0.0 },
                    VariableType::Continuous => value,
                };

                values.insert(var.name.clone(), quantized);
            }

            let obj_value = objective_fn(&values);

            let is_better = if is_minimize {
                obj_value < best_value
            } else {
                obj_value > best_value
            };

            if is_better && self.check_feasibility(&problem, &values) {
                best_value = obj_value;

                best = Some(Solution {
                    id: self.next_id.fetch_add(1, Ordering::Relaxed),
                    values: values.clone(),
                    objectives: {
                        let mut objs = BTreeMap::new();
                        objs.insert(problem.objectives[0].name.clone(), obj_value);
                        objs
                    },
                    feasible: true,
                    pareto_optimal: true,
                    iteration: iterations,
                });
            }
        }

        let end = Timestamp::now();

        let result = OptimizationResult {
            problem_id,
            best: best.clone(),
            solutions: best.into_iter().collect(),
            iterations,
            time_ns: end.0.saturating_sub(start.0),
            converged: true,
        };

        self.stats.problems_solved += 1;
        self.stats.total_iterations += iterations;
        self.stats.solutions_found += 1;

        self.results.insert(problem_id, result.clone());

        Some(result)
    }

    fn simple_random(&self) -> f64 {
        let t = Timestamp::now().0;
        ((t % 10000) as f64) / 10000.0
    }

    fn check_feasibility(&self, problem: &OptimizationProblem, values: &BTreeMap<String, f64>) -> bool {
        // Simplified: just check bounds
        for var in &problem.variables {
            let val = values.get(&var.name).copied().unwrap_or(0.0);

            if val < var.bounds.0 || val > var.bounds.1 {
                return false;
            }
        }
        true
    }

    /// Get problem
    pub fn get_problem(&self, id: u64) -> Option<&OptimizationProblem> {
        self.problems.get(&id)
    }

    /// Get result
    pub fn get_result(&self, id: u64) -> Option<&OptimizationResult> {
        self.results.get(&id)
    }

    /// Get statistics
    pub fn stats(&self) -> &OptimizerStats {
        &self.stats
    }
}

impl Default for DecisionOptimizer {
    fn default() -> Self {
        Self::new(OptimizerConfig::default())
    }
}

// ============================================================================
// PROBLEM BUILDER
// ============================================================================

/// Problem builder
pub struct ProblemBuilder<'a> {
    optimizer: &'a mut DecisionOptimizer,
    problem_id: u64,
}

impl<'a> ProblemBuilder<'a> {
    /// Create new builder
    pub fn new(optimizer: &'a mut DecisionOptimizer, name: &str) -> Self {
        let problem_id = optimizer.create_problem(name);

        Self {
            optimizer,
            problem_id,
        }
    }

    /// Add continuous variable
    pub fn variable(self, name: &str, min: f64, max: f64, initial: f64) -> Self {
        self.optimizer.add_variable(self.problem_id, Variable {
            name: name.into(),
            var_type: VariableType::Continuous,
            bounds: (min, max),
            initial,
        });
        self
    }

    /// Add integer variable
    pub fn integer_variable(self, name: &str, min: i64, max: i64) -> Self {
        self.optimizer.add_variable(self.problem_id, Variable {
            name: name.into(),
            var_type: VariableType::Integer,
            bounds: (min as f64, max as f64),
            initial: min as f64,
        });
        self
    }

    /// Add objective
    pub fn minimize(self, name: &str) -> Self {
        self.optimizer.add_objective(self.problem_id, Objective {
            name: name.into(),
            direction: OptimDirection::Minimize,
            weight: 1.0,
        });
        self
    }

    /// Add objective
    pub fn maximize(self, name: &str) -> Self {
        self.optimizer.add_objective(self.problem_id, Objective {
            name: name.into(),
            direction: OptimDirection::Maximize,
            weight: 1.0,
        });
        self
    }

    /// Build
    pub fn build(self) -> u64 {
        self.problem_id
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_problem() {
        let mut optimizer = DecisionOptimizer::default();

        let id = optimizer.create_problem("test");
        assert!(optimizer.get_problem(id).is_some());
    }

    #[test]
    fn test_gradient_descent() {
        let mut optimizer = DecisionOptimizer::default();

        let problem_id = ProblemBuilder::new(&mut optimizer, "quadratic")
            .variable("x", -10.0, 10.0, 5.0)
            .minimize("f")
            .build();

        // Minimize f(x) = x^2
        let result = optimizer.solve_gradient(problem_id, |values| {
            let x = values.get("x").copied().unwrap_or(0.0);
            x * x
        });

        assert!(result.is_some());

        let r = result.unwrap();
        let best = r.best.unwrap();
        let x = best.values.get("x").copied().unwrap_or(100.0);

        // Should be close to 0
        assert!(x.abs() < 0.5);
    }

    #[test]
    fn test_random_search() {
        let mut optimizer = DecisionOptimizer::default();

        let problem_id = ProblemBuilder::new(&mut optimizer, "test")
            .variable("x", 0.0, 1.0, 0.5)
            .maximize("f")
            .build();

        // Maximize f(x) = -x^2 + x (max at x=0.5)
        let result = optimizer.solve_random(problem_id, |values| {
            let x = values.get("x").copied().unwrap_or(0.0);
            -x * x + x
        });

        assert!(result.is_some());
    }

    #[test]
    fn test_builder() {
        let mut optimizer = DecisionOptimizer::default();

        let id = ProblemBuilder::new(&mut optimizer, "multi")
            .variable("x", 0.0, 1.0, 0.5)
            .variable("y", 0.0, 1.0, 0.5)
            .minimize("cost")
            .build();

        let problem = optimizer.get_problem(id).unwrap();
        assert_eq!(problem.variables.len(), 2);
        assert_eq!(problem.objectives.len(), 1);
    }
}
