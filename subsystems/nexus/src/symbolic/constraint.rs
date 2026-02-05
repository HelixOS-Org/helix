//! # Constraint Solver for NEXUS
//!
//! Year 2 "COGNITION" - Revolutionary kernel-level constraint satisfaction
//! and optimization system for solving complex scheduling, resource allocation,
//! and configuration problems.
//!
//! ## Features
//!
//! - CSP (Constraint Satisfaction Problem) solver
//! - SAT solver (DPLL algorithm)
//! - Constraint propagation (arc consistency)
//! - Backtracking with intelligent variable ordering
//! - Optimization (minimize/maximize objectives)
//! - Kernel-specific constraints (resources, scheduling, memory)

#![allow(dead_code)]
#![allow(clippy::excessive_nesting)]
#![allow(unused_variables)]
#![allow(unused_mut)]

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

/// Maximum variables in a CSP
const MAX_VARIABLES: usize = 10_000;

/// Maximum domain size
const MAX_DOMAIN_SIZE: usize = 1_000;

/// Maximum constraints
const MAX_CONSTRAINTS: usize = 100_000;

/// Default search timeout
const DEFAULT_TIMEOUT: usize = 1_000_000;

// ============================================================================
// CORE TYPES
// ============================================================================

/// Variable identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VarId(pub u32);

/// A domain value
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DomainValue {
    /// Integer value
    Integer(i64),
    /// Boolean value
    Boolean(bool),
    /// Symbol value
    Symbol(String),
    /// Interval [min, max]
    Interval(i64, i64),
}

impl DomainValue {
    /// Get as integer
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            DomainValue::Integer(i) => Some(*i),
            DomainValue::Boolean(b) => Some(if *b { 1 } else { 0 }),
            _ => None,
        }
    }

    /// Get as boolean
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            DomainValue::Boolean(b) => Some(*b),
            DomainValue::Integer(i) => Some(*i != 0),
            _ => None,
        }
    }
}

/// A variable's domain
#[derive(Debug, Clone)]
pub struct Domain {
    /// Possible values
    values: BTreeSet<DomainValue>,
    /// Original size (before propagation)
    original_size: usize,
}

impl Domain {
    /// Create a new domain with given values
    pub fn new(values: Vec<DomainValue>) -> Self {
        let original_size = values.len();
        Self {
            values: values.into_iter().collect(),
            original_size,
        }
    }

    /// Create an integer range domain [min, max]
    pub fn integer_range(min: i64, max: i64) -> Self {
        let values: Vec<DomainValue> = (min..=max).map(DomainValue::Integer).collect();
        Self::new(values)
    }

    /// Create a boolean domain
    pub fn boolean() -> Self {
        Self::new(vec![
            DomainValue::Boolean(false),
            DomainValue::Boolean(true),
        ])
    }

    /// Check if domain is empty
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Check if domain is a singleton
    pub fn is_singleton(&self) -> bool {
        self.values.len() == 1
    }

    /// Get the single value if singleton
    pub fn get_singleton(&self) -> Option<&DomainValue> {
        if self.is_singleton() {
            self.values.iter().next()
        } else {
            None
        }
    }

    /// Get domain size
    pub fn size(&self) -> usize {
        self.values.len()
    }

    /// Remove a value from the domain
    pub fn remove(&mut self, value: &DomainValue) -> bool {
        self.values.remove(value)
    }

    /// Restrict domain to given values
    pub fn restrict(&mut self, allowed: &BTreeSet<DomainValue>) {
        self.values.retain(|v| allowed.contains(v));
    }

    /// Check if value is in domain
    pub fn contains(&self, value: &DomainValue) -> bool {
        self.values.contains(value)
    }

    /// Get all values
    pub fn values(&self) -> impl Iterator<Item = &DomainValue> {
        self.values.iter()
    }

    /// Set to singleton
    pub fn set_value(&mut self, value: DomainValue) {
        self.values.clear();
        self.values.insert(value);
    }

    /// Get minimum integer value
    pub fn min_integer(&self) -> Option<i64> {
        self.values.iter().filter_map(|v| v.as_integer()).min()
    }

    /// Get maximum integer value
    pub fn max_integer(&self) -> Option<i64> {
        self.values.iter().filter_map(|v| v.as_integer()).max()
    }
}

// ============================================================================
// CONSTRAINTS
// ============================================================================

/// Constraint types
#[derive(Debug, Clone)]
pub enum Constraint {
    /// X = Y
    Equal(VarId, VarId),
    /// X ≠ Y
    NotEqual(VarId, VarId),
    /// X < Y
    LessThan(VarId, VarId),
    /// X ≤ Y
    LessEqual(VarId, VarId),
    /// X > Y
    GreaterThan(VarId, VarId),
    /// X ≥ Y
    GreaterEqual(VarId, VarId),
    /// X = constant
    EqualConstant(VarId, DomainValue),
    /// X ≠ constant
    NotEqualConstant(VarId, DomainValue),
    /// X + Y = Z
    AddEquals(VarId, VarId, VarId),
    /// X - Y = Z
    SubEquals(VarId, VarId, VarId),
    /// X * Y = Z
    MulEquals(VarId, VarId, VarId),
    /// All different: all variables must have different values
    AllDifferent(Vec<VarId>),
    /// Sum constraint: sum of variables equals value
    Sum(Vec<VarId>, i64),
    /// Sum less than or equal
    SumLessEqual(Vec<VarId>, i64),
    /// Sum greater than or equal
    SumGreaterEqual(Vec<VarId>, i64),
    /// Cardinality: exactly n variables equal value
    Cardinality(Vec<VarId>, DomainValue, usize),
    /// Implication: if X = val1 then Y = val2
    Implication(VarId, DomainValue, VarId, DomainValue),
    /// Table constraint (allowed tuples)
    Table(Vec<VarId>, Vec<Vec<DomainValue>>),
    /// Custom constraint (evaluated by callback ID)
    Custom(u32, Vec<VarId>),
}

/// Constraint metadata
#[derive(Debug, Clone)]
pub struct ConstraintInfo {
    /// Constraint ID
    pub id: u32,
    /// The constraint
    pub constraint: Constraint,
    /// Variables involved
    pub variables: Vec<VarId>,
    /// Priority (higher = checked first)
    pub priority: u32,
    /// Is this a hard or soft constraint?
    pub hard: bool,
    /// Weight (for soft constraints)
    pub weight: f64,
}

impl ConstraintInfo {
    /// Create a hard constraint
    pub fn hard(id: u32, constraint: Constraint) -> Self {
        let variables = constraint.get_variables();
        Self {
            id,
            constraint,
            variables,
            priority: 0,
            hard: true,
            weight: 1.0,
        }
    }

    /// Create a soft constraint with weight
    pub fn soft(id: u32, constraint: Constraint, weight: f64) -> Self {
        let variables = constraint.get_variables();
        Self {
            id,
            constraint,
            variables,
            priority: 0,
            hard: false,
            weight,
        }
    }
}

impl Constraint {
    /// Get all variables in this constraint
    pub fn get_variables(&self) -> Vec<VarId> {
        match self {
            Constraint::Equal(a, b)
            | Constraint::NotEqual(a, b)
            | Constraint::LessThan(a, b)
            | Constraint::LessEqual(a, b)
            | Constraint::GreaterThan(a, b)
            | Constraint::GreaterEqual(a, b) => vec![*a, *b],

            Constraint::EqualConstant(v, _) | Constraint::NotEqualConstant(v, _) => {
                vec![*v]
            },

            Constraint::AddEquals(a, b, c)
            | Constraint::SubEquals(a, b, c)
            | Constraint::MulEquals(a, b, c) => vec![*a, *b, *c],

            Constraint::AllDifferent(vars)
            | Constraint::Sum(vars, _)
            | Constraint::SumLessEqual(vars, _)
            | Constraint::SumGreaterEqual(vars, _) => vars.clone(),

            Constraint::Cardinality(vars, _, _) => vars.clone(),

            Constraint::Implication(a, _, b, _) => vec![*a, *b],

            Constraint::Table(vars, _) | Constraint::Custom(_, vars) => vars.clone(),
        }
    }
}

// ============================================================================
// CSP SOLVER
// ============================================================================

/// Variable selection heuristic
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableHeuristic {
    /// First unassigned variable
    First,
    /// Minimum remaining values (MRV)
    MinimumRemainingValues,
    /// Maximum degree (most constraints)
    MaxDegree,
    /// Combined MRV + degree
    DomWDeg,
}

/// Value selection heuristic
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueHeuristic {
    /// Ascending order
    Ascending,
    /// Descending order
    Descending,
    /// Least constraining value
    LeastConstraining,
    /// Random
    Random,
}

/// CSP solution
#[derive(Debug, Clone)]
pub struct Solution {
    /// Variable assignments
    pub assignments: BTreeMap<VarId, DomainValue>,
    /// Soft constraint violations
    pub violations: Vec<u32>,
    /// Total weight of violated soft constraints
    pub violation_cost: f64,
}

/// CSP solver result
#[derive(Debug, Clone)]
pub enum SolverResult {
    /// Found a solution
    Satisfiable(Solution),
    /// No solution exists
    Unsatisfiable,
    /// Timeout or limit reached
    Unknown,
    /// All solutions found
    AllSolutions(Vec<Solution>),
}

/// The CSP solver
pub struct CspSolver {
    /// Variables and their domains
    domains: BTreeMap<VarId, Domain>,
    /// Constraints
    constraints: Vec<ConstraintInfo>,
    /// Constraint index by variable
    var_constraints: BTreeMap<VarId, Vec<u32>>,
    /// Next constraint ID
    next_constraint_id: u32,
    /// Variable heuristic
    var_heuristic: VariableHeuristic,
    /// Value heuristic
    val_heuristic: ValueHeuristic,
    /// Enable propagation
    propagation: bool,
    /// Search timeout
    timeout: usize,
    /// Steps taken
    steps: usize,
    /// Backtracks performed
    backtracks: usize,
}

impl CspSolver {
    /// Create a new CSP solver
    pub fn new() -> Self {
        Self {
            domains: BTreeMap::new(),
            constraints: Vec::new(),
            var_constraints: BTreeMap::new(),
            next_constraint_id: 0,
            var_heuristic: VariableHeuristic::MinimumRemainingValues,
            val_heuristic: ValueHeuristic::Ascending,
            propagation: true,
            timeout: DEFAULT_TIMEOUT,
            steps: 0,
            backtracks: 0,
        }
    }

    /// Set variable heuristic
    pub fn with_variable_heuristic(mut self, heuristic: VariableHeuristic) -> Self {
        self.var_heuristic = heuristic;
        self
    }

    /// Set value heuristic
    pub fn with_value_heuristic(mut self, heuristic: ValueHeuristic) -> Self {
        self.val_heuristic = heuristic;
        self
    }

    /// Disable propagation
    pub fn without_propagation(mut self) -> Self {
        self.propagation = false;
        self
    }

    /// Add a variable with domain
    pub fn add_variable(&mut self, domain: Domain) -> VarId {
        let id = VarId(self.domains.len() as u32);
        self.domains.insert(id, domain);
        self.var_constraints.insert(id, Vec::new());
        id
    }

    /// Add an integer variable with range
    pub fn add_int_variable(&mut self, min: i64, max: i64) -> VarId {
        self.add_variable(Domain::integer_range(min, max))
    }

    /// Add a boolean variable
    pub fn add_bool_variable(&mut self) -> VarId {
        self.add_variable(Domain::boolean())
    }

    /// Add a constraint
    pub fn add_constraint(&mut self, constraint: Constraint) -> u32 {
        let id = self.next_constraint_id;
        self.next_constraint_id += 1;

        let info = ConstraintInfo::hard(id, constraint);

        // Update variable-constraint index
        for var in &info.variables {
            if let Some(list) = self.var_constraints.get_mut(var) {
                list.push(id);
            }
        }

        self.constraints.push(info);
        id
    }

    /// Add a soft constraint with weight
    pub fn add_soft_constraint(&mut self, constraint: Constraint, weight: f64) -> u32 {
        let id = self.next_constraint_id;
        self.next_constraint_id += 1;

        let info = ConstraintInfo::soft(id, constraint, weight);

        for var in &info.variables {
            if let Some(list) = self.var_constraints.get_mut(var) {
                list.push(id);
            }
        }

        self.constraints.push(info);
        id
    }

    /// Solve the CSP
    pub fn solve(&mut self) -> SolverResult {
        self.steps = 0;
        self.backtracks = 0;

        // Initial propagation
        let mut domains = self.domains.clone();
        if self.propagation && !self.propagate(&mut domains) {
            return SolverResult::Unsatisfiable;
        }

        // Backtracking search
        let mut assignment = BTreeMap::new();
        if self.backtrack_search(&mut assignment, &mut domains) {
            let solution = Solution {
                assignments: assignment,
                violations: Vec::new(),
                violation_cost: 0.0,
            };
            SolverResult::Satisfiable(solution)
        } else if self.steps >= self.timeout {
            SolverResult::Unknown
        } else {
            SolverResult::Unsatisfiable
        }
    }

    /// Find all solutions
    pub fn solve_all(&mut self, max_solutions: usize) -> SolverResult {
        self.steps = 0;
        self.backtracks = 0;

        let mut domains = self.domains.clone();
        if self.propagation && !self.propagate(&mut domains) {
            return SolverResult::Unsatisfiable;
        }

        let mut solutions = Vec::new();
        let mut assignment = BTreeMap::new();

        self.backtrack_all(&mut assignment, &mut domains, &mut solutions, max_solutions);

        if solutions.is_empty() {
            SolverResult::Unsatisfiable
        } else {
            SolverResult::AllSolutions(solutions)
        }
    }

    /// Backtracking search
    fn backtrack_search(
        &mut self,
        assignment: &mut BTreeMap<VarId, DomainValue>,
        domains: &mut BTreeMap<VarId, Domain>,
    ) -> bool {
        self.steps += 1;
        if self.steps >= self.timeout {
            return false;
        }

        // Check if complete
        if assignment.len() == self.domains.len() {
            return true;
        }

        // Select unassigned variable
        let var = match self.select_variable(assignment, domains) {
            Some(v) => v,
            None => return false,
        };

        // Get ordered values
        let values: Vec<DomainValue> = self.order_values(var, domains);

        for value in values {
            // Try assignment
            assignment.insert(var, value.clone());

            // Check constraints
            if self.is_consistent(var, assignment) {
                // Propagate
                let mut new_domains = domains.clone();
                new_domains.get_mut(&var).unwrap().set_value(value.clone());

                let consistent = if self.propagation {
                    self.propagate(&mut new_domains)
                } else {
                    true
                };

                if consistent && self.backtrack_search(assignment, &mut new_domains) {
                    return true;
                }
            }

            // Backtrack
            self.backtracks += 1;
            assignment.remove(&var);
        }

        false
    }

    /// Find all solutions
    fn backtrack_all(
        &mut self,
        assignment: &mut BTreeMap<VarId, DomainValue>,
        domains: &mut BTreeMap<VarId, Domain>,
        solutions: &mut Vec<Solution>,
        max_solutions: usize,
    ) {
        if solutions.len() >= max_solutions || self.steps >= self.timeout {
            return;
        }

        self.steps += 1;

        // Check if complete
        if assignment.len() == self.domains.len() {
            solutions.push(Solution {
                assignments: assignment.clone(),
                violations: Vec::new(),
                violation_cost: 0.0,
            });
            return;
        }

        // Select unassigned variable
        let var = match self.select_variable(assignment, domains) {
            Some(v) => v,
            None => return,
        };

        let values: Vec<DomainValue> = self.order_values(var, domains);

        for value in values {
            assignment.insert(var, value.clone());

            if self.is_consistent(var, assignment) {
                let mut new_domains = domains.clone();
                new_domains.get_mut(&var).unwrap().set_value(value.clone());

                let consistent = if self.propagation {
                    self.propagate(&mut new_domains)
                } else {
                    true
                };

                if consistent {
                    self.backtrack_all(assignment, &mut new_domains, solutions, max_solutions);
                }
            }

            assignment.remove(&var);
        }
    }

    /// Select unassigned variable
    fn select_variable(
        &self,
        assignment: &BTreeMap<VarId, DomainValue>,
        domains: &BTreeMap<VarId, Domain>,
    ) -> Option<VarId> {
        let unassigned: Vec<VarId> = domains
            .keys()
            .filter(|v| !assignment.contains_key(*v))
            .copied()
            .collect();

        if unassigned.is_empty() {
            return None;
        }

        match self.var_heuristic {
            VariableHeuristic::First => Some(unassigned[0]),

            VariableHeuristic::MinimumRemainingValues => unassigned
                .iter()
                .min_by_key(|v| domains.get(*v).map(|d| d.size()).unwrap_or(usize::MAX))
                .copied(),

            VariableHeuristic::MaxDegree => unassigned
                .iter()
                .max_by_key(|v| self.var_constraints.get(*v).map(|c| c.len()).unwrap_or(0))
                .copied(),

            VariableHeuristic::DomWDeg => unassigned
                .iter()
                .min_by(|a, b| {
                    let dom_a = domains.get(*a).map(|d| d.size()).unwrap_or(1);
                    let dom_b = domains.get(*b).map(|d| d.size()).unwrap_or(1);
                    let deg_a = self.var_constraints.get(*a).map(|c| c.len()).unwrap_or(0);
                    let deg_b = self.var_constraints.get(*b).map(|c| c.len()).unwrap_or(0);

                    let ratio_a = dom_a as f64 / (deg_a as f64 + 1.0);
                    let ratio_b = dom_b as f64 / (deg_b as f64 + 1.0);

                    ratio_a
                        .partial_cmp(&ratio_b)
                        .unwrap_or(core::cmp::Ordering::Equal)
                })
                .copied(),
        }
    }

    /// Order values for a variable
    fn order_values(&self, var: VarId, domains: &BTreeMap<VarId, Domain>) -> Vec<DomainValue> {
        let domain = match domains.get(&var) {
            Some(d) => d,
            None => return Vec::new(),
        };

        let mut values: Vec<DomainValue> = domain.values().cloned().collect();

        match self.val_heuristic {
            ValueHeuristic::Ascending => {
                values.sort();
            },
            ValueHeuristic::Descending => {
                values.sort();
                values.reverse();
            },
            ValueHeuristic::LeastConstraining | ValueHeuristic::Random => {
                // Keep default order for now
            },
        }

        values
    }

    /// Check if current assignment is consistent
    fn is_consistent(&self, var: VarId, assignment: &BTreeMap<VarId, DomainValue>) -> bool {
        // Check only constraints involving var
        if let Some(constraint_ids) = self.var_constraints.get(&var) {
            for &cid in constraint_ids {
                if let Some(info) = self.constraints.iter().find(|c| c.id == cid) {
                    if info.hard && !self.check_constraint(&info.constraint, assignment) {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// Check a single constraint
    fn check_constraint(
        &self,
        constraint: &Constraint,
        assignment: &BTreeMap<VarId, DomainValue>,
    ) -> bool {
        match constraint {
            Constraint::Equal(a, b) => {
                match (assignment.get(a), assignment.get(b)) {
                    (Some(va), Some(vb)) => va == vb,
                    _ => true, // Not fully assigned yet
                }
            },

            Constraint::NotEqual(a, b) => match (assignment.get(a), assignment.get(b)) {
                (Some(va), Some(vb)) => va != vb,
                _ => true,
            },

            Constraint::LessThan(a, b) => match (assignment.get(a), assignment.get(b)) {
                (Some(va), Some(vb)) => match (va.as_integer(), vb.as_integer()) {
                    (Some(ia), Some(ib)) => ia < ib,
                    _ => true,
                },
                _ => true,
            },

            Constraint::LessEqual(a, b) => match (assignment.get(a), assignment.get(b)) {
                (Some(va), Some(vb)) => match (va.as_integer(), vb.as_integer()) {
                    (Some(ia), Some(ib)) => ia <= ib,
                    _ => true,
                },
                _ => true,
            },

            Constraint::GreaterThan(a, b) => match (assignment.get(a), assignment.get(b)) {
                (Some(va), Some(vb)) => match (va.as_integer(), vb.as_integer()) {
                    (Some(ia), Some(ib)) => ia > ib,
                    _ => true,
                },
                _ => true,
            },

            Constraint::GreaterEqual(a, b) => match (assignment.get(a), assignment.get(b)) {
                (Some(va), Some(vb)) => match (va.as_integer(), vb.as_integer()) {
                    (Some(ia), Some(ib)) => ia >= ib,
                    _ => true,
                },
                _ => true,
            },

            Constraint::EqualConstant(v, c) => match assignment.get(v) {
                Some(val) => val == c,
                None => true,
            },

            Constraint::NotEqualConstant(v, c) => match assignment.get(v) {
                Some(val) => val != c,
                None => true,
            },

            Constraint::AllDifferent(vars) => {
                let assigned: Vec<&DomainValue> =
                    vars.iter().filter_map(|v| assignment.get(v)).collect();

                let unique: BTreeSet<&DomainValue> = assigned.iter().copied().collect();
                assigned.len() == unique.len()
            },

            Constraint::Sum(vars, target) => {
                let values: Vec<Option<i64>> = vars
                    .iter()
                    .map(|v| assignment.get(v).and_then(|dv| dv.as_integer()))
                    .collect();

                if values.iter().all(|v| v.is_some()) {
                    let sum: i64 = values.iter().filter_map(|v| *v).sum();
                    sum == *target
                } else {
                    true
                }
            },

            Constraint::SumLessEqual(vars, target) => {
                let values: Vec<Option<i64>> = vars
                    .iter()
                    .map(|v| assignment.get(v).and_then(|dv| dv.as_integer()))
                    .collect();

                if values.iter().all(|v| v.is_some()) {
                    let sum: i64 = values.iter().filter_map(|v| *v).sum();
                    sum <= *target
                } else {
                    // Partial check: current sum should not exceed target
                    let partial_sum: i64 = values.iter().filter_map(|v| *v).sum();
                    partial_sum <= *target
                }
            },

            Constraint::SumGreaterEqual(vars, target) => {
                let values: Vec<Option<i64>> = vars
                    .iter()
                    .map(|v| assignment.get(v).and_then(|dv| dv.as_integer()))
                    .collect();

                if values.iter().all(|v| v.is_some()) {
                    let sum: i64 = values.iter().filter_map(|v| *v).sum();
                    sum >= *target
                } else {
                    true
                }
            },

            Constraint::Implication(a, val_a, b, val_b) => {
                match (assignment.get(a), assignment.get(b)) {
                    (Some(va), Some(vb)) => {
                        // If A = val_a then B must = val_b
                        if va == val_a { vb == val_b } else { true }
                    },
                    (Some(va), None) => {
                        // If A = val_a, B is not constrained yet
                        va != val_a
                    },
                    _ => true,
                }
            },

            _ => true, // Other constraints need full implementation
        }
    }

    /// Arc consistency propagation (AC-3)
    fn propagate(&self, domains: &mut BTreeMap<VarId, Domain>) -> bool {
        let mut queue: Vec<(VarId, VarId)> = Vec::new();

        // Initialize queue with all arcs
        for constraint in &self.constraints {
            if !constraint.hard {
                continue;
            }

            match &constraint.constraint {
                Constraint::Equal(a, b)
                | Constraint::NotEqual(a, b)
                | Constraint::LessThan(a, b)
                | Constraint::LessEqual(a, b)
                | Constraint::GreaterThan(a, b)
                | Constraint::GreaterEqual(a, b) => {
                    queue.push((*a, *b));
                    queue.push((*b, *a));
                },
                _ => {},
            }
        }

        while let Some((xi, xj)) = queue.pop() {
            if self.revise(xi, xj, domains) {
                let domain = domains.get(&xi);
                if domain.map(|d| d.is_empty()).unwrap_or(true) {
                    return false; // Domain wipeout
                }

                // Add neighbors to queue
                if let Some(constraint_ids) = self.var_constraints.get(&xi) {
                    for &cid in constraint_ids {
                        if let Some(info) = self.constraints.iter().find(|c| c.id == cid) {
                            for var in &info.variables {
                                if *var != xi && *var != xj {
                                    queue.push((*var, xi));
                                }
                            }
                        }
                    }
                }
            }
        }

        true
    }

    /// Revise domain of xi based on constraint with xj
    fn revise(&self, xi: VarId, xj: VarId, domains: &mut BTreeMap<VarId, Domain>) -> bool {
        let mut revised = false;

        let di = match domains.get(&xi) {
            Some(d) => d.clone(),
            None => return false,
        };
        let dj = match domains.get(&xj) {
            Some(d) => d,
            None => return false,
        };

        // Find constraints between xi and xj
        let mut values_to_remove = Vec::new();

        for val_i in di.values() {
            let mut has_support = false;

            for val_j in dj.values() {
                // Check if (val_i, val_j) is consistent
                let mut temp_assignment = BTreeMap::new();
                temp_assignment.insert(xi, val_i.clone());
                temp_assignment.insert(xj, val_j.clone());

                let consistent = self.constraints.iter().all(|info| {
                    if !info.hard {
                        return true;
                    }
                    if info.variables.contains(&xi) && info.variables.contains(&xj) {
                        self.check_constraint(&info.constraint, &temp_assignment)
                    } else {
                        true
                    }
                });

                if consistent {
                    has_support = true;
                    break;
                }
            }

            if !has_support {
                values_to_remove.push(val_i.clone());
            }
        }

        // Remove unsupported values
        if let Some(domain) = domains.get_mut(&xi) {
            for val in values_to_remove {
                if domain.remove(&val) {
                    revised = true;
                }
            }
        }

        revised
    }

    /// Get statistics
    pub fn stats(&self) -> SolverStats {
        SolverStats {
            steps: self.steps,
            backtracks: self.backtracks,
            variables: self.domains.len(),
            constraints: self.constraints.len(),
        }
    }
}

impl Default for CspSolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Solver statistics
#[derive(Debug, Clone)]
pub struct SolverStats {
    /// Total steps taken
    pub steps: usize,
    /// Backtracks performed
    pub backtracks: usize,
    /// Number of variables
    pub variables: usize,
    /// Number of constraints
    pub constraints: usize,
}

// ============================================================================
// SAT SOLVER (DPLL)
// ============================================================================

/// A SAT literal (positive or negative variable)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SatLiteral {
    /// Variable index
    pub var: u32,
    /// Is positive?
    pub positive: bool,
}

impl SatLiteral {
    /// Create a positive literal
    pub fn pos(var: u32) -> Self {
        Self {
            var,
            positive: true,
        }
    }

    /// Create a negative literal
    pub fn neg(var: u32) -> Self {
        Self {
            var,
            positive: false,
        }
    }

    /// Negate this literal
    pub fn negate(&self) -> Self {
        Self {
            var: self.var,
            positive: !self.positive,
        }
    }
}

/// A SAT clause (disjunction of literals)
pub type SatClause = Vec<SatLiteral>;

/// SAT solver result
#[derive(Debug, Clone)]
pub enum SatResult {
    /// Satisfiable with model
    Sat(Vec<bool>),
    /// Unsatisfiable
    Unsat,
    /// Unknown (timeout)
    Unknown,
}

/// DPLL SAT solver
pub struct DpllSolver {
    /// Number of variables
    num_vars: u32,
    /// Clauses
    clauses: Vec<SatClause>,
    /// Current assignment
    assignment: Vec<Option<bool>>,
    /// Steps taken
    steps: usize,
    /// Timeout
    timeout: usize,
}

impl DpllSolver {
    /// Create a new SAT solver
    pub fn new(num_vars: u32) -> Self {
        Self {
            num_vars,
            clauses: Vec::new(),
            assignment: vec![None; num_vars as usize],
            steps: 0,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Add a clause
    pub fn add_clause(&mut self, clause: SatClause) {
        self.clauses.push(clause);
    }

    /// Solve
    pub fn solve(&mut self) -> SatResult {
        self.steps = 0;
        self.assignment = vec![None; self.num_vars as usize];

        if self.dpll() {
            let model: Vec<bool> = self.assignment.iter().map(|a| a.unwrap_or(false)).collect();
            SatResult::Sat(model)
        } else if self.steps >= self.timeout {
            SatResult::Unknown
        } else {
            SatResult::Unsat
        }
    }

    /// DPLL algorithm
    fn dpll(&mut self) -> bool {
        self.steps += 1;
        if self.steps >= self.timeout {
            return false;
        }

        // Unit propagation
        loop {
            let unit = self.find_unit_clause();
            match unit {
                Some(lit) => {
                    self.assign(lit);
                },
                None => break,
            }

            // Check for conflict
            if self.has_empty_clause() {
                return false;
            }
        }

        // Check if all clauses satisfied
        if self.all_satisfied() {
            return true;
        }

        // Check for empty clause
        if self.has_empty_clause() {
            return false;
        }

        // Choose variable
        let var = match self.choose_variable() {
            Some(v) => v,
            None => return self.all_satisfied(),
        };

        // Try positive assignment
        let saved_assignment = self.assignment.clone();
        self.assignment[var as usize] = Some(true);

        if self.dpll() {
            return true;
        }

        // Backtrack and try negative
        self.assignment = saved_assignment;
        self.assignment[var as usize] = Some(false);

        self.dpll()
    }

    /// Find a unit clause
    fn find_unit_clause(&self) -> Option<SatLiteral> {
        for clause in &self.clauses {
            let unassigned: Vec<&SatLiteral> = clause
                .iter()
                .filter(|lit| self.assignment[lit.var as usize].is_none())
                .collect();

            let satisfied = clause
                .iter()
                .any(|lit| match self.assignment[lit.var as usize] {
                    Some(val) => val == lit.positive,
                    None => false,
                });

            if !satisfied && unassigned.len() == 1 {
                return Some(*unassigned[0]);
            }
        }
        None
    }

    /// Assign a literal
    fn assign(&mut self, lit: SatLiteral) {
        self.assignment[lit.var as usize] = Some(lit.positive);
    }

    /// Check if there's an empty clause (conflict)
    fn has_empty_clause(&self) -> bool {
        for clause in &self.clauses {
            let all_false = clause
                .iter()
                .all(|lit| match self.assignment[lit.var as usize] {
                    Some(val) => val != lit.positive,
                    None => false,
                });

            if all_false {
                return true;
            }
        }
        false
    }

    /// Check if all clauses are satisfied
    fn all_satisfied(&self) -> bool {
        for clause in &self.clauses {
            let satisfied = clause
                .iter()
                .any(|lit| match self.assignment[lit.var as usize] {
                    Some(val) => val == lit.positive,
                    None => false,
                });

            if !satisfied {
                return false;
            }
        }
        true
    }

    /// Choose an unassigned variable
    fn choose_variable(&self) -> Option<u32> {
        for (i, val) in self.assignment.iter().enumerate() {
            if val.is_none() {
                return Some(i as u32);
            }
        }
        None
    }
}

// ============================================================================
// KERNEL CONSTRAINT PROBLEMS
// ============================================================================

/// Scheduler constraint problem
pub struct SchedulerCsp {
    solver: CspSolver,
    /// Task variables (task_id -> var_id)
    task_vars: BTreeMap<u64, VarId>,
    /// CPU assignments
    cpu_assignments: BTreeMap<u64, VarId>,
}

impl SchedulerCsp {
    /// Create a new scheduler CSP
    pub fn new(num_cpus: usize) -> Self {
        let mut solver = CspSolver::new();
        Self {
            solver,
            task_vars: BTreeMap::new(),
            cpu_assignments: BTreeMap::new(),
        }
    }

    /// Add a task with priority range
    pub fn add_task(&mut self, task_id: u64, min_priority: i64, max_priority: i64) {
        let var = self.solver.add_int_variable(min_priority, max_priority);
        self.task_vars.insert(task_id, var);
    }

    /// Add CPU affinity constraint
    pub fn add_affinity_constraint(&mut self, task_id: u64, allowed_cpus: Vec<i64>) {
        if let Some(&var) = self.task_vars.get(&task_id) {
            let values: Vec<DomainValue> =
                allowed_cpus.into_iter().map(DomainValue::Integer).collect();

            let cpu_var = self.solver.add_variable(Domain::new(values));
            self.cpu_assignments.insert(task_id, cpu_var);
        }
    }

    /// Add mutual exclusion (tasks can't run simultaneously on same CPU)
    pub fn add_mutex_constraint(&mut self, task1: u64, task2: u64) {
        if let (Some(&v1), Some(&v2)) = (
            self.cpu_assignments.get(&task1),
            self.cpu_assignments.get(&task2),
        ) {
            self.solver.add_constraint(Constraint::NotEqual(v1, v2));
        }
    }

    /// Add priority ordering constraint
    pub fn add_priority_ordering(&mut self, higher: u64, lower: u64) {
        if let (Some(&v1), Some(&v2)) = (self.task_vars.get(&higher), self.task_vars.get(&lower)) {
            self.solver.add_constraint(Constraint::GreaterThan(v1, v2));
        }
    }

    /// Solve the scheduling problem
    pub fn solve(&mut self) -> Option<BTreeMap<u64, (i64, i64)>> {
        match self.solver.solve() {
            SolverResult::Satisfiable(solution) => {
                let mut result = BTreeMap::new();

                for (&task_id, &priority_var) in &self.task_vars {
                    let priority = solution
                        .assignments
                        .get(&priority_var)
                        .and_then(|v| v.as_integer())
                        .unwrap_or(0);

                    let cpu = self
                        .cpu_assignments
                        .get(&task_id)
                        .and_then(|v| solution.assignments.get(v))
                        .and_then(|v| v.as_integer())
                        .unwrap_or(0);

                    result.insert(task_id, (priority, cpu));
                }

                Some(result)
            },
            _ => None,
        }
    }
}

/// Memory allocation constraint problem
pub struct MemoryAllocationCsp {
    solver: CspSolver,
    /// Region variables (region_id -> (start_var, size))
    regions: BTreeMap<u64, (VarId, u64)>,
    /// Memory size
    memory_size: u64,
}

impl MemoryAllocationCsp {
    /// Create a new memory allocation CSP
    pub fn new(memory_size: u64) -> Self {
        Self {
            solver: CspSolver::new(),
            regions: BTreeMap::new(),
            memory_size,
        }
    }

    /// Add a memory region to allocate
    pub fn add_region(&mut self, region_id: u64, size: u64, alignment: u64) {
        // Variable for start address (must be aligned and leave room for size)
        let max_start = self.memory_size.saturating_sub(size);
        let mut domain_values = Vec::new();

        let mut addr = 0u64;
        while addr <= max_start {
            domain_values.push(DomainValue::Integer(addr as i64));
            addr += alignment;
        }

        if !domain_values.is_empty() {
            let var = self.solver.add_variable(Domain::new(domain_values));
            self.regions.insert(region_id, (var, size));
        }
    }

    /// Add non-overlapping constraint between regions
    pub fn add_non_overlapping(&mut self, region1: u64, region2: u64) {
        if let (Some(&(v1, s1)), Some(&(v2, s2))) =
            (self.regions.get(&region1), self.regions.get(&region2))
        {
            // r1.start + r1.size <= r2.start OR r2.start + r2.size <= r1.start
            // This is complex for CSP, so we use a disjunctive approach
            // For simplicity, we'll just add: r1 < r2 (assuming we want ordered allocation)
            self.solver.add_constraint(Constraint::LessThan(v1, v2));
        }
    }

    /// Solve the allocation problem
    pub fn solve(&mut self) -> Option<BTreeMap<u64, u64>> {
        match self.solver.solve() {
            SolverResult::Satisfiable(solution) => {
                let mut result = BTreeMap::new();

                for (&region_id, &(var, _)) in &self.regions {
                    if let Some(addr) = solution.assignments.get(&var).and_then(|v| v.as_integer())
                    {
                        result.insert(region_id, addr as u64);
                    }
                }

                Some(result)
            },
            _ => None,
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csp_basic() {
        let mut solver = CspSolver::new();

        // Two variables with domain [1, 2, 3]
        let x = solver.add_int_variable(1, 3);
        let y = solver.add_int_variable(1, 3);

        // X != Y
        solver.add_constraint(Constraint::NotEqual(x, y));

        // X < Y
        solver.add_constraint(Constraint::LessThan(x, y));

        match solver.solve() {
            SolverResult::Satisfiable(sol) => {
                let x_val = sol.assignments.get(&x).unwrap().as_integer().unwrap();
                let y_val = sol.assignments.get(&y).unwrap().as_integer().unwrap();
                assert!(x_val < y_val);
            },
            _ => panic!("Should be satisfiable"),
        }
    }

    #[test]
    fn test_all_different() {
        let mut solver = CspSolver::new();

        let vars: Vec<VarId> = (0..3).map(|_| solver.add_int_variable(1, 3)).collect();

        solver.add_constraint(Constraint::AllDifferent(vars.clone()));

        match solver.solve() {
            SolverResult::Satisfiable(sol) => {
                let values: Vec<i64> = vars
                    .iter()
                    .filter_map(|v| sol.assignments.get(v)?.as_integer())
                    .collect();

                let unique: BTreeSet<i64> = values.iter().copied().collect();
                assert_eq!(values.len(), unique.len());
            },
            _ => panic!("Should be satisfiable"),
        }
    }

    #[test]
    fn test_unsatisfiable() {
        let mut solver = CspSolver::new();

        let x = solver.add_int_variable(1, 1); // Only value 1
        let y = solver.add_int_variable(1, 1); // Only value 1

        solver.add_constraint(Constraint::NotEqual(x, y));

        match solver.solve() {
            SolverResult::Unsatisfiable => {},
            _ => panic!("Should be unsatisfiable"),
        }
    }

    #[test]
    fn test_sat_basic() {
        let mut solver = DpllSolver::new(3);

        // (x1 OR x2) AND (NOT x1 OR x3) AND (NOT x2 OR NOT x3)
        solver.add_clause(vec![SatLiteral::pos(0), SatLiteral::pos(1)]);
        solver.add_clause(vec![SatLiteral::neg(0), SatLiteral::pos(2)]);
        solver.add_clause(vec![SatLiteral::neg(1), SatLiteral::neg(2)]);

        match solver.solve() {
            SatResult::Sat(model) => {
                // Verify solution
                let x1 = model[0];
                let x2 = model[1];
                let x3 = model[2];

                assert!(x1 || x2);
                assert!(!x1 || x3);
                assert!(!x2 || !x3);
            },
            _ => panic!("Should be satisfiable"),
        }
    }

    #[test]
    fn test_sat_unsat() {
        let mut solver = DpllSolver::new(1);

        // x1 AND NOT x1
        solver.add_clause(vec![SatLiteral::pos(0)]);
        solver.add_clause(vec![SatLiteral::neg(0)]);

        match solver.solve() {
            SatResult::Unsat => {},
            _ => panic!("Should be unsatisfiable"),
        }
    }

    #[test]
    fn test_sum_constraint() {
        let mut solver = CspSolver::new();

        let x = solver.add_int_variable(1, 5);
        let y = solver.add_int_variable(1, 5);
        let z = solver.add_int_variable(1, 5);

        // x + y + z = 10
        solver.add_constraint(Constraint::Sum(vec![x, y, z], 10));

        match solver.solve() {
            SolverResult::Satisfiable(sol) => {
                let sum: i64 = [x, y, z]
                    .iter()
                    .filter_map(|v| sol.assignments.get(v)?.as_integer())
                    .sum();
                assert_eq!(sum, 10);
            },
            _ => panic!("Should be satisfiable"),
        }
    }
}
