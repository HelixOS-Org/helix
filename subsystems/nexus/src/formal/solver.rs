//! SAT and SMT solvers implementation.

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use super::assignment::Assignment;
use super::clause::Clause;
use super::formula::CnfFormula;
use super::types::{Literal, VarId};

/// SAT solver result
#[derive(Debug, Clone)]
pub enum SatResult {
    /// Satisfiable with assignment
    Sat(Assignment),
    /// Unsatisfiable
    Unsat,
    /// Unknown (timeout/resource limit)
    Unknown,
}

/// DPLL-based SAT solver with CDCL
#[derive(Debug)]
pub struct SatSolver {
    /// CNF formula
    formula: CnfFormula,
    /// Current assignment
    assignment: Assignment,
    /// Watched literals (for each literal, list of watching clauses)
    watches: BTreeMap<i32, Vec<usize>>,
    /// Learned clauses
    learned: Vec<Clause>,
    /// Implication graph
    implications: Vec<(VarId, Option<usize>)>, // (var, reason_clause)
    /// Conflict limit for restarts
    conflict_limit: u64,
    /// Current conflicts
    conflicts: u64,
}

impl SatSolver {
    /// Create a new SAT solver
    pub fn new(formula: CnfFormula) -> Self {
        let assignment = Assignment::new(formula.num_vars);
        let mut solver = Self {
            formula,
            assignment,
            watches: BTreeMap::new(),
            learned: Vec::new(),
            implications: Vec::new(),
            conflict_limit: 100,
            conflicts: 0,
        };
        solver.init_watches();
        solver
    }

    /// Initialize watched literals
    fn init_watches(&mut self) {
        for (clause_idx, clause) in self.formula.clauses.iter().enumerate() {
            if clause.size() >= 2 {
                let lit1 = clause.literals[0].to_dimacs();
                let lit2 = clause.literals[1].to_dimacs();
                self.watches.entry(lit1).or_default().push(clause_idx);
                self.watches.entry(lit2).or_default().push(clause_idx);
            }
        }
    }

    /// Unit propagation
    fn propagate(&mut self) -> Option<usize> {
        loop {
            let mut propagated = false;

            for (clause_idx, clause) in self.formula.clauses.iter().enumerate() {
                match self.assignment.eval_clause(clause) {
                    Some(false) => return Some(clause_idx), // Conflict
                    Some(true) => continue,                 // Already satisfied
                    None => {
                        // Check for unit clause
                        let unassigned: Vec<_> = clause
                            .literals
                            .iter()
                            .filter(|lit| self.assignment.eval_lit(**lit).is_none())
                            .collect();

                        if unassigned.len() == 1 {
                            let lit = unassigned[0];
                            let value = !lit.is_negated();
                            self.assignment.assign(lit.var(), value);
                            self.implications.push((lit.var(), Some(clause_idx)));
                            propagated = true;
                        }
                    },
                }
            }

            // Also check learned clauses
            for (clause_idx, clause) in self.learned.iter().enumerate() {
                let actual_idx = self.formula.clauses.len() + clause_idx;
                match self.assignment.eval_clause(clause) {
                    Some(false) => return Some(actual_idx),
                    Some(true) => continue,
                    None => {
                        let unassigned: Vec<_> = clause
                            .literals
                            .iter()
                            .filter(|lit| self.assignment.eval_lit(**lit).is_none())
                            .collect();

                        if unassigned.len() == 1 {
                            let lit = unassigned[0];
                            let value = !lit.is_negated();
                            self.assignment.assign(lit.var(), value);
                            self.implications.push((lit.var(), Some(actual_idx)));
                            propagated = true;
                        }
                    },
                }
            }

            if !propagated {
                break;
            }
        }

        None // No conflict
    }

    /// Analyze conflict and learn clause
    fn analyze_conflict(&mut self, _conflict_clause: usize) -> (Clause, usize) {
        // Simplified conflict analysis: learn negation of current decisions
        let mut learned_lits = Vec::new();
        let mut backtrack_level = 0;

        for &(var, _, level) in self.assignment.trail().iter().rev() {
            if level == 0 {
                break;
            }

            if let Some(value) = self.assignment.get(var) {
                let lit = if value {
                    Literal::neg(var)
                } else {
                    Literal::pos(var)
                };
                learned_lits.push(lit);

                if level > 0 {
                    backtrack_level = level - 1;
                    break;
                }
            }
        }

        if learned_lits.is_empty() {
            // Root level conflict - UNSAT
            (Clause::empty(), 0)
        } else {
            (Clause::from_lits(&learned_lits), backtrack_level)
        }
    }

    /// Pick branching variable (VSIDS-like)
    fn pick_branch_var(&self) -> Option<(VarId, bool)> {
        // Simple strategy: pick first unassigned, try true first
        self.assignment.pick_unassigned().map(|v| (v, true))
    }

    /// Solve the formula
    pub fn solve(&mut self) -> SatResult {
        // Initial propagation
        if let Some(_conflict) = self.propagate() {
            return SatResult::Unsat;
        }

        loop {
            // Check if complete
            if self.assignment.is_complete() {
                return SatResult::Sat(self.assignment.clone());
            }

            // Pick branching variable
            let (var, value) = match self.pick_branch_var() {
                Some(v) => v,
                None => return SatResult::Sat(self.assignment.clone()),
            };

            // Make decision
            self.assignment.decide(var, value);
            self.implications.push((var, None));

            // Propagate
            while let Some(conflict_clause) = self.propagate() {
                self.conflicts += 1;

                // Analyze conflict
                let (learned, backtrack_level) = self.analyze_conflict(conflict_clause);

                if learned.is_empty() || backtrack_level == 0 && self.assignment.decision_level == 1
                {
                    return SatResult::Unsat;
                }

                // Learn clause
                self.learned.push(learned);

                // Backtrack
                self.assignment.backtrack_to(backtrack_level);

                // Check for restart
                if self.conflicts > self.conflict_limit {
                    self.conflicts = 0;
                    self.conflict_limit = (self.conflict_limit as f64 * 1.1) as u64;
                    self.assignment.backtrack_to(0);
                    break;
                }
            }
        }
    }
}

/// SMT Sort (type)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SmtSort {
    Bool,
    Int,
    Real,
    BitVec(u32),
    Array(Box<SmtSort>, Box<SmtSort>),
    Uninterpreted(String),
}

/// SMT Term
#[derive(Debug, Clone)]
pub enum SmtTerm {
    /// Boolean constant
    BoolConst(bool),
    /// Integer constant
    IntConst(i64),
    /// Real constant
    RealConst(f64),
    /// Bitvector constant
    BvConst(u64, u32),
    /// Variable
    Var(String, SmtSort),
    /// Negation
    Not(Box<SmtTerm>),
    /// And
    And(Vec<SmtTerm>),
    /// Or
    Or(Vec<SmtTerm>),
    /// Implies
    Implies(Box<SmtTerm>, Box<SmtTerm>),
    /// Equals
    Eq(Box<SmtTerm>, Box<SmtTerm>),
    /// Less than
    Lt(Box<SmtTerm>, Box<SmtTerm>),
    /// Less or equal
    Le(Box<SmtTerm>, Box<SmtTerm>),
    /// Greater than
    Gt(Box<SmtTerm>, Box<SmtTerm>),
    /// Greater or equal
    Ge(Box<SmtTerm>, Box<SmtTerm>),
    /// Addition
    Add(Vec<SmtTerm>),
    /// Subtraction
    Sub(Box<SmtTerm>, Box<SmtTerm>),
    /// Multiplication
    Mul(Vec<SmtTerm>),
    /// Division
    Div(Box<SmtTerm>, Box<SmtTerm>),
    /// Modulo
    Mod(Box<SmtTerm>, Box<SmtTerm>),
    /// If-then-else
    Ite(Box<SmtTerm>, Box<SmtTerm>, Box<SmtTerm>),
    /// Bitvector operations
    BvAnd(Box<SmtTerm>, Box<SmtTerm>),
    BvOr(Box<SmtTerm>, Box<SmtTerm>),
    BvXor(Box<SmtTerm>, Box<SmtTerm>),
    BvAdd(Box<SmtTerm>, Box<SmtTerm>),
    BvSub(Box<SmtTerm>, Box<SmtTerm>),
    /// Array select
    Select(Box<SmtTerm>, Box<SmtTerm>),
    /// Array store
    Store(Box<SmtTerm>, Box<SmtTerm>, Box<SmtTerm>),
    /// Uninterpreted function application
    App(String, Vec<SmtTerm>),
}

impl SmtTerm {
    /// Create boolean variable
    #[inline(always)]
    pub fn bool_var(name: &str) -> Self {
        SmtTerm::Var(String::from(name), SmtSort::Bool)
    }

    /// Create integer variable
    #[inline(always)]
    pub fn int_var(name: &str) -> Self {
        SmtTerm::Var(String::from(name), SmtSort::Int)
    }

    /// Create bitvector variable
    #[inline(always)]
    pub fn bv_var(name: &str, width: u32) -> Self {
        SmtTerm::Var(String::from(name), SmtSort::BitVec(width))
    }

    /// Create and of multiple terms
    #[inline(always)]
    pub fn and(terms: Vec<SmtTerm>) -> Self {
        SmtTerm::And(terms)
    }

    /// Create or of multiple terms
    #[inline(always)]
    pub fn or(terms: Vec<SmtTerm>) -> Self {
        SmtTerm::Or(terms)
    }

    /// Create implication
    #[inline(always)]
    pub fn implies(a: SmtTerm, b: SmtTerm) -> Self {
        SmtTerm::Implies(Box::new(a), Box::new(b))
    }
}

/// SMT Solver (simplified)
#[derive(Debug)]
pub struct SmtSolver {
    /// Assertions
    assertions: Vec<SmtTerm>,
    /// Variable declarations
    variables: BTreeMap<String, SmtSort>,
}

impl SmtSolver {
    /// Create new SMT solver
    pub fn new() -> Self {
        Self {
            assertions: Vec::new(),
            variables: BTreeMap::new(),
        }
    }

    /// Declare a variable
    #[inline(always)]
    pub fn declare(&mut self, name: &str, sort: SmtSort) {
        self.variables.insert(String::from(name), sort);
    }

    /// Add assertion
    #[inline(always)]
    pub fn assert(&mut self, term: SmtTerm) {
        self.assertions.push(term);
    }

    /// Check satisfiability (converts to SAT)
    pub fn check(&self) -> SatResult {
        // Simplified: just convert boolean formulas to CNF
        let mut formula = CnfFormula::new();
        let mut var_map: BTreeMap<String, VarId> = BTreeMap::new();

        for assertion in &self.assertions {
            self.to_cnf(assertion, &mut formula, &mut var_map, false);
        }

        let mut solver = SatSolver::new(formula);
        solver.solve()
    }

    /// Convert term to CNF (Tseitin transformation)
    #[allow(clippy::only_used_in_recursion)]
    fn to_cnf(
        &self,
        term: &SmtTerm,
        formula: &mut CnfFormula,
        var_map: &mut BTreeMap<String, VarId>,
        negated: bool,
    ) -> Option<Literal> {
        match term {
            SmtTerm::BoolConst(b) => {
                let effective = if negated { !b } else { *b };
                if !effective {
                    // Add empty clause for false
                    formula.add_clause(Clause::empty());
                }
                None
            },
            SmtTerm::Var(name, SmtSort::Bool) => {
                let var = *var_map
                    .entry(name.clone())
                    .or_insert_with(|| formula.new_var());
                Some(if negated {
                    Literal::neg(var)
                } else {
                    Literal::pos(var)
                })
            },
            SmtTerm::Not(inner) => self.to_cnf(inner, formula, var_map, !negated),
            SmtTerm::And(terms) if !negated => {
                // ∧ terms: each term must be true
                for t in terms {
                    if let Some(lit) = self.to_cnf(t, formula, var_map, false) {
                        formula.add_unit(lit);
                    }
                }
                None
            },
            SmtTerm::And(terms) if negated => {
                // ¬(∧ terms) = ∨(¬terms)
                let mut clause_lits = Vec::new();
                for t in terms {
                    if let Some(lit) = self.to_cnf(t, formula, var_map, true) {
                        clause_lits.push(lit);
                    }
                }
                if !clause_lits.is_empty() {
                    formula.add_clause(Clause::from_lits(&clause_lits));
                }
                None
            },
            SmtTerm::Or(terms) if !negated => {
                // ∨ terms: at least one must be true
                let mut clause_lits = Vec::new();
                for t in terms {
                    if let Some(lit) = self.to_cnf(t, formula, var_map, false) {
                        clause_lits.push(lit);
                    }
                }
                if !clause_lits.is_empty() {
                    formula.add_clause(Clause::from_lits(&clause_lits));
                }
                None
            },
            SmtTerm::Or(terms) if negated => {
                // ¬(∨ terms) = ∧(¬terms)
                for t in terms {
                    if let Some(lit) = self.to_cnf(t, formula, var_map, true) {
                        formula.add_unit(lit);
                    }
                }
                None
            },
            SmtTerm::Implies(a, b) => {
                // a → b = ¬a ∨ b
                let or_term = SmtTerm::Or(vec![SmtTerm::Not(a.clone()), (**b).clone()]);
                self.to_cnf(&or_term, formula, var_map, negated)
            },
            SmtTerm::Eq(a, b) => {
                // For booleans: a = b is (a → b) ∧ (b → a)
                if let SmtTerm::Var(_, SmtSort::Bool) = **a {
                    let lit_a = self.to_cnf(a, formula, var_map, false);
                    let lit_b = self.to_cnf(b, formula, var_map, false);

                    if let (Some(la), Some(lb)) = (lit_a, lit_b) {
                        // a → b: ¬a ∨ b
                        formula.add_clause(Clause::binary(la.negate(), lb));
                        // b → a: ¬b ∨ a
                        formula.add_clause(Clause::binary(lb.negate(), la));
                    }
                }
                None
            },
            _ => None, // Other terms need theory solvers
        }
    }
}

impl Default for SmtSolver {
    fn default() -> Self {
        Self::new()
    }
}
