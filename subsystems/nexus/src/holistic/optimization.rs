//! # Holistic Global Optimization Engine
//!
//! System-wide multi-objective optimization:
//! - Pareto-optimal resource allocation
//! - Constraint satisfaction
//! - Gradient-free optimization
//! - Multi-objective trade-off analysis
//! - Objective function management

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// OPTIMIZATION TYPES
// ============================================================================

/// Optimization objective
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OptimizationObjective {
    /// Minimize latency
    MinLatency,
    /// Maximize throughput
    MaxThroughput,
    /// Minimize power
    MinPower,
    /// Maximize fairness
    MaxFairness,
    /// Minimize temperature
    MinTemperature,
    /// Maximize utilization
    MaxUtilization,
    /// Minimize jitter
    MinJitter,
    /// Minimize cost
    MinCost,
}

/// Optimization direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationDirection {
    /// Minimize
    Minimize,
    /// Maximize
    Maximize,
}

impl OptimizationObjective {
    /// Direction
    pub fn direction(&self) -> OptimizationDirection {
        match self {
            Self::MinLatency
            | Self::MinPower
            | Self::MinTemperature
            | Self::MinJitter
            | Self::MinCost => OptimizationDirection::Minimize,
            Self::MaxThroughput | Self::MaxFairness | Self::MaxUtilization => {
                OptimizationDirection::Maximize
            }
        }
    }
}

/// Constraint type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintType {
    /// Less than or equal
    LessEqual,
    /// Greater than or equal
    GreaterEqual,
    /// Equal
    Equal,
    /// Between range
    Range,
}

// ============================================================================
// OBJECTIVE & CONSTRAINT DEFINITIONS
// ============================================================================

/// Objective definition
#[derive(Debug, Clone)]
pub struct ObjectiveDef {
    /// Objective type
    pub objective: OptimizationObjective,
    /// Weight (importance)
    pub weight: f64,
    /// Current value
    pub current_value: f64,
    /// Target value
    pub target: f64,
    /// Direction
    pub direction: OptimizationDirection,
}

impl ObjectiveDef {
    pub fn new(objective: OptimizationObjective, weight: f64, target: f64) -> Self {
        Self {
            objective,
            weight,
            current_value: 0.0,
            target,
            direction: objective.direction(),
        }
    }

    /// Satisfaction (0.0-1.0)
    pub fn satisfaction(&self) -> f64 {
        if self.target == 0.0 {
            return 1.0;
        }
        match self.direction {
            OptimizationDirection::Minimize => {
                if self.current_value <= self.target {
                    1.0
                } else {
                    self.target / self.current_value
                }
            }
            OptimizationDirection::Maximize => {
                if self.current_value >= self.target {
                    1.0
                } else if self.target > 0.0 {
                    self.current_value / self.target
                } else {
                    0.0
                }
            }
        }
    }
}

/// Constraint definition
#[derive(Debug, Clone)]
pub struct ConstraintDef {
    /// Constraint name code
    pub code: u32,
    /// Variable index
    pub variable: usize,
    /// Type
    pub constraint_type: ConstraintType,
    /// Bound value
    pub bound: f64,
    /// Upper bound (for range)
    pub upper_bound: f64,
}

impl ConstraintDef {
    /// Check if value satisfies constraint
    #[inline]
    pub fn satisfied(&self, value: f64) -> bool {
        match self.constraint_type {
            ConstraintType::LessEqual => value <= self.bound,
            ConstraintType::GreaterEqual => value >= self.bound,
            ConstraintType::Equal => libm::fabs(value - self.bound) < 1e-6,
            ConstraintType::Range => value >= self.bound && value <= self.upper_bound,
        }
    }

    /// Violation amount
    pub fn violation(&self, value: f64) -> f64 {
        match self.constraint_type {
            ConstraintType::LessEqual => {
                if value > self.bound {
                    value - self.bound
                } else {
                    0.0
                }
            }
            ConstraintType::GreaterEqual => {
                if value < self.bound {
                    self.bound - value
                } else {
                    0.0
                }
            }
            ConstraintType::Equal => libm::fabs(value - self.bound),
            ConstraintType::Range => {
                if value < self.bound {
                    self.bound - value
                } else if value > self.upper_bound {
                    value - self.upper_bound
                } else {
                    0.0
                }
            }
        }
    }
}

// ============================================================================
// SOLUTION
// ============================================================================

/// Optimization solution
#[derive(Debug, Clone)]
pub struct OptSolution {
    /// Solution id
    pub id: u64,
    /// Variable values
    pub variables: Vec<f64>,
    /// Objective values
    pub objective_values: Vec<f64>,
    /// Total weighted score
    pub score: f64,
    /// All constraints satisfied?
    pub feasible: bool,
    /// Total constraint violation
    pub violation: f64,
}

impl OptSolution {
    /// Dominates another solution? (Pareto)
    pub fn dominates(&self, other: &OptSolution) -> bool {
        if self.objective_values.len() != other.objective_values.len() {
            return false;
        }
        let mut at_least_one_better = false;
        for (a, b) in self.objective_values.iter().zip(other.objective_values.iter()) {
            if a > b {
                return false;
            }
            if a < b {
                at_least_one_better = true;
            }
        }
        at_least_one_better
    }
}

/// Pareto front
#[derive(Debug, Clone)]
pub struct ParetoFront {
    /// Solutions on the front
    pub solutions: Vec<OptSolution>,
}

impl ParetoFront {
    pub fn new() -> Self {
        Self {
            solutions: Vec::new(),
        }
    }

    /// Add solution, maintaining non-dominated set
    #[inline]
    pub fn add(&mut self, solution: OptSolution) {
        // Remove dominated solutions
        self.solutions.retain(|s| !solution.dominates(s));
        // Add if not dominated by any existing
        let is_dominated = self.solutions.iter().any(|s| s.dominates(&solution));
        if !is_dominated {
            self.solutions.push(solution);
        }
    }

    /// Size
    #[inline(always)]
    pub fn size(&self) -> usize {
        self.solutions.len()
    }

    /// Best by weighted score
    #[inline]
    pub fn best_weighted(&self) -> Option<&OptSolution> {
        self.solutions
            .iter()
            .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(core::cmp::Ordering::Equal))
    }
}

// ============================================================================
// OPTIMIZATION ENGINE
// ============================================================================

/// Optimization stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticOptimizationStats {
    /// Objectives defined
    pub objectives: usize,
    /// Constraints defined
    pub constraints: usize,
    /// Solutions evaluated
    pub solutions_evaluated: u64,
    /// Pareto front size
    pub pareto_size: usize,
    /// Best score
    pub best_score: f64,
    /// Iterations
    pub iterations: u64,
}

/// Holistic optimization engine
pub struct HolisticOptimizationEngine {
    /// Objectives
    objectives: Vec<ObjectiveDef>,
    /// Constraints
    constraints: Vec<ConstraintDef>,
    /// Current best solution
    best_solution: Option<OptSolution>,
    /// Pareto front
    pareto: ParetoFront,
    /// Next solution id
    next_id: u64,
    /// Stats
    stats: HolisticOptimizationStats,
}

impl HolisticOptimizationEngine {
    pub fn new() -> Self {
        Self {
            objectives: Vec::new(),
            constraints: Vec::new(),
            best_solution: None,
            pareto: ParetoFront::new(),
            next_id: 1,
            stats: HolisticOptimizationStats::default(),
        }
    }

    /// Add objective
    #[inline(always)]
    pub fn add_objective(&mut self, objective: ObjectiveDef) {
        self.objectives.push(objective);
        self.stats.objectives = self.objectives.len();
    }

    /// Add constraint
    #[inline(always)]
    pub fn add_constraint(&mut self, constraint: ConstraintDef) {
        self.constraints.push(constraint);
        self.stats.constraints = self.constraints.len();
    }

    /// Update objective's current value
    #[inline]
    pub fn update_objective(&mut self, obj: OptimizationObjective, value: f64) {
        for o in &mut self.objectives {
            if o.objective == obj {
                o.current_value = value;
            }
        }
    }

    /// Evaluate a solution
    pub fn evaluate(&mut self, variables: Vec<f64>) -> OptSolution {
        // Compute constraint violations
        let mut total_violation = 0.0;
        let mut feasible = true;
        for c in &self.constraints {
            if c.variable < variables.len() {
                let v = c.violation(variables[c.variable]);
                if v > 0.0 {
                    feasible = false;
                    total_violation += v;
                }
            }
        }

        // Compute weighted score from objectives
        let mut obj_values = Vec::with_capacity(self.objectives.len());
        let mut weighted_score = 0.0;
        for obj in &self.objectives {
            let satisfaction = obj.satisfaction();
            obj_values.push(satisfaction);
            weighted_score += satisfaction * obj.weight;
        }
        let total_weight: f64 = self.objectives.iter().map(|o| o.weight).sum();
        if total_weight > 0.0 {
            weighted_score /= total_weight;
        }

        // Penalize infeasible
        if !feasible {
            weighted_score *= 0.5 / (1.0 + total_violation);
        }

        let solution = OptSolution {
            id: self.next_id,
            variables,
            objective_values: obj_values,
            score: weighted_score,
            feasible,
            violation: total_violation,
        };
        self.next_id += 1;
        self.stats.solutions_evaluated += 1;

        // Update best
        if let Some(ref best) = self.best_solution {
            if solution.score > best.score {
                self.best_solution = Some(solution.clone());
                self.stats.best_score = solution.score;
            }
        } else {
            self.stats.best_score = solution.score;
            self.best_solution = Some(solution.clone());
        }

        // Update pareto
        self.pareto.add(solution.clone());
        self.stats.pareto_size = self.pareto.size();

        solution
    }

    /// Run one iteration of coordinate descent
    pub fn iterate(&mut self, current: &[f64], step_size: f64) -> OptSolution {
        let n = current.len();
        let mut best = current.to_vec();
        let mut best_score = self.evaluate(best.clone()).score;

        for i in 0..n {
            // Try +step
            let mut candidate = best.clone();
            candidate[i] += step_size;
            let sol = self.evaluate(candidate.clone());
            if sol.score > best_score {
                best = candidate;
                best_score = sol.score;
            }

            // Try -step
            let mut candidate = best.clone();
            candidate[i] -= step_size;
            let sol = self.evaluate(candidate.clone());
            if sol.score > best_score {
                best = candidate;
                best_score = sol.score;
            }
        }

        self.stats.iterations += 1;
        self.evaluate(best)
    }

    /// Current objectives satisfaction
    #[inline]
    pub fn satisfaction_report(&self) -> Vec<(OptimizationObjective, f64)> {
        self.objectives
            .iter()
            .map(|o| (o.objective, o.satisfaction()))
            .collect()
    }

    /// Pareto front
    #[inline(always)]
    pub fn pareto_front(&self) -> &ParetoFront {
        &self.pareto
    }

    /// Best solution
    #[inline(always)]
    pub fn best(&self) -> Option<&OptSolution> {
        self.best_solution.as_ref()
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &HolisticOptimizationStats {
        &self.stats
    }
}
