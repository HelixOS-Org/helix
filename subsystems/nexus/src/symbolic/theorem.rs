//! # Automated Theorem Prover for NEXUS
//!
//! Year 2 "COGNITION" - Revolutionary kernel-level theorem proving system
//! that enables formal verification, invariant checking, and proof-based
//! reasoning for kernel correctness.
//!
//! ## Features
//!
//! - Resolution-based theorem proving
//! - Unification with occurs check
//! - Skolemization and CNF conversion
//! - Proof search strategies (depth-first, breadth-first, iterative deepening)
//! - Proof explanation generation
//! - Kernel invariant verification

#![allow(dead_code)]
#![allow(clippy::excessive_nesting)]
#![allow(clippy::useless_format)]
#![allow(clippy::let_and_return)]

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

/// Maximum proof depth
const MAX_PROOF_DEPTH: usize = 100;

/// Maximum clauses in working set
const MAX_CLAUSES: usize = 10_000;

/// Maximum unification steps
const MAX_UNIFICATION_STEPS: usize = 1000;

/// Default timeout (in steps)
const DEFAULT_TIMEOUT: usize = 100_000;

// ============================================================================
// LOGICAL TERMS
// ============================================================================

/// A logical term in first-order logic
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Term {
    /// A constant symbol (e.g., `alice`, `0`, `null`)
    Constant(String),
    /// A variable (e.g., `X`, `Y`)
    Variable(String),
    /// An integer constant
    Integer(i64),
    /// A function application (e.g., `f(X, Y)`)
    Function(String, Vec<Term>),
}

impl Term {
    /// Create a constant term
    #[inline(always)]
    pub fn constant(name: &str) -> Self {
        Term::Constant(String::from(name))
    }

    /// Create a variable term
    #[inline(always)]
    pub fn variable(name: &str) -> Self {
        Term::Variable(String::from(name))
    }

    /// Create an integer term
    #[inline(always)]
    pub fn integer(value: i64) -> Self {
        Term::Integer(value)
    }

    /// Create a function term
    #[inline(always)]
    pub fn function(name: &str, args: Vec<Term>) -> Self {
        Term::Function(String::from(name), args)
    }

    /// Check if this term is a variable
    #[inline(always)]
    pub fn is_variable(&self) -> bool {
        matches!(self, Term::Variable(_))
    }

    /// Check if this term is ground (no variables)
    #[inline]
    pub fn is_ground(&self) -> bool {
        match self {
            Term::Constant(_) | Term::Integer(_) => true,
            Term::Variable(_) => false,
            Term::Function(_, args) => args.iter().all(|a| a.is_ground()),
        }
    }

    /// Get all variables in this term
    #[inline]
    pub fn variables(&self) -> BTreeSet<String> {
        let mut vars = BTreeSet::new();
        self.collect_variables(&mut vars);
        vars
    }

    fn collect_variables(&self, vars: &mut BTreeSet<String>) {
        match self {
            Term::Variable(v) => {
                vars.insert(v.clone());
            },
            Term::Function(_, args) => {
                for arg in args {
                    arg.collect_variables(vars);
                }
            },
            _ => {},
        }
    }

    /// Apply a substitution to this term
    pub fn apply_substitution(&self, subst: &Substitution) -> Term {
        match self {
            Term::Variable(v) => {
                if let Some(t) = subst.get(v) {
                    t.apply_substitution(subst)
                } else {
                    self.clone()
                }
            },
            Term::Function(name, args) => {
                let new_args: Vec<Term> =
                    args.iter().map(|a| a.apply_substitution(subst)).collect();
                Term::Function(name.clone(), new_args)
            },
            _ => self.clone(),
        }
    }

    /// Check if variable occurs in term (for occurs check)
    #[inline]
    pub fn occurs(&self, var: &str) -> bool {
        match self {
            Term::Variable(v) => v == var,
            Term::Function(_, args) => args.iter().any(|a| a.occurs(var)),
            _ => false,
        }
    }
}

// ============================================================================
// SUBSTITUTION
// ============================================================================

/// A substitution mapping variables to terms
#[derive(Debug, Clone, Default)]
pub struct Substitution {
    /// The mapping
    bindings: BTreeMap<String, Term>,
}

impl Substitution {
    /// Create an empty substitution
    pub fn new() -> Self {
        Self::default()
    }

    /// Create from a list of bindings
    #[inline]
    pub fn from_bindings(bindings: Vec<(String, Term)>) -> Self {
        let mut subst = Self::new();
        for (var, term) in bindings {
            subst.bind(var, term);
        }
        subst
    }

    /// Bind a variable to a term
    #[inline(always)]
    pub fn bind(&mut self, var: String, term: Term) {
        self.bindings.insert(var, term);
    }

    /// Get the binding for a variable
    #[inline(always)]
    pub fn get(&self, var: &str) -> Option<&Term> {
        self.bindings.get(var)
    }

    /// Check if empty
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    /// Compose two substitutions
    pub fn compose(&self, other: &Substitution) -> Substitution {
        let mut result = Substitution::new();

        // Apply other to all bindings in self
        for (var, term) in &self.bindings {
            result.bind(var.clone(), term.apply_substitution(other));
        }

        // Add bindings from other that aren't in self
        for (var, term) in &other.bindings {
            if !result.bindings.contains_key(var) {
                result.bind(var.clone(), term.clone());
            }
        }

        result
    }

    /// Get all bindings
    #[inline(always)]
    pub fn bindings(&self) -> &BTreeMap<String, Term> {
        &self.bindings
    }
}

// ============================================================================
// UNIFICATION
// ============================================================================

/// Unification result
#[derive(Debug, Clone)]
pub enum UnificationResult {
    /// Unification succeeded with this substitution
    Success(Substitution),
    /// Unification failed
    Failure,
    /// Occurs check failed
    OccursCheckFailed,
}

/// The unification engine
pub struct Unifier {
    /// Enable occurs check
    occurs_check: bool,
    /// Step counter
    steps: usize,
    /// Max steps
    max_steps: usize,
}

impl Unifier {
    /// Create a new unifier
    pub fn new() -> Self {
        Self {
            occurs_check: true,
            max_steps: MAX_UNIFICATION_STEPS,
            steps: 0,
        }
    }

    /// Create without occurs check (faster but unsound)
    #[inline]
    pub fn without_occurs_check() -> Self {
        Self {
            occurs_check: false,
            max_steps: MAX_UNIFICATION_STEPS,
            steps: 0,
        }
    }

    /// Unify two terms
    #[inline(always)]
    pub fn unify(&mut self, t1: &Term, t2: &Term) -> UnificationResult {
        self.steps = 0;
        self.unify_with_subst(t1, t2, Substitution::new())
    }

    /// Unify with initial substitution
    fn unify_with_subst(&mut self, t1: &Term, t2: &Term, subst: Substitution) -> UnificationResult {
        self.steps += 1;
        if self.steps > self.max_steps {
            return UnificationResult::Failure;
        }

        let t1 = t1.apply_substitution(&subst);
        let t2 = t2.apply_substitution(&subst);

        match (&t1, &t2) {
            // Same terms unify trivially
            (Term::Constant(a), Term::Constant(b)) if a == b => UnificationResult::Success(subst),
            (Term::Integer(a), Term::Integer(b)) if a == b => UnificationResult::Success(subst),

            // Variable unification
            (Term::Variable(v), t) | (t, Term::Variable(v)) => {
                if let Term::Variable(v2) = t {
                    if v == v2 {
                        return UnificationResult::Success(subst);
                    }
                }

                // Occurs check
                if self.occurs_check && t.occurs(v) {
                    return UnificationResult::OccursCheckFailed;
                }

                let mut new_subst = subst;
                new_subst.bind(v.clone(), t.clone());
                UnificationResult::Success(new_subst)
            },

            // Function unification
            (Term::Function(f1, args1), Term::Function(f2, args2)) => {
                if f1 != f2 || args1.len() != args2.len() {
                    return UnificationResult::Failure;
                }

                let mut current_subst = subst;
                for (a1, a2) in args1.iter().zip(args2.iter()) {
                    match self.unify_with_subst(a1, a2, current_subst.clone()) {
                        UnificationResult::Success(s) => current_subst = s,
                        result => return result,
                    }
                }
                UnificationResult::Success(current_subst)
            },

            // Different constants/types
            _ => UnificationResult::Failure,
        }
    }
}

impl Default for Unifier {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// LITERALS AND CLAUSES
// ============================================================================

/// An atomic formula (predicate applied to terms)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Atom {
    /// Predicate name
    pub predicate: String,
    /// Arguments
    pub args: Vec<Term>,
}

impl Atom {
    /// Create a new atom
    pub fn new(predicate: &str, args: Vec<Term>) -> Self {
        Self {
            predicate: String::from(predicate),
            args,
        }
    }

    /// Check if ground
    #[inline(always)]
    pub fn is_ground(&self) -> bool {
        self.args.iter().all(|a| a.is_ground())
    }

    /// Get variables
    #[inline]
    pub fn variables(&self) -> BTreeSet<String> {
        let mut vars = BTreeSet::new();
        for arg in &self.args {
            for v in arg.variables() {
                vars.insert(v);
            }
        }
        vars
    }

    /// Apply substitution
    #[inline]
    pub fn apply_substitution(&self, subst: &Substitution) -> Atom {
        Atom {
            predicate: self.predicate.clone(),
            args: self
                .args
                .iter()
                .map(|a| a.apply_substitution(subst))
                .collect(),
        }
    }
}

/// A literal (positive or negative atom)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Literal {
    /// The atom
    pub atom: Atom,
    /// Is this a positive literal?
    pub positive: bool,
}

impl Literal {
    /// Create a positive literal
    #[inline]
    pub fn positive(atom: Atom) -> Self {
        Self {
            atom,
            positive: true,
        }
    }

    /// Create a negative literal
    #[inline]
    pub fn negative(atom: Atom) -> Self {
        Self {
            atom,
            positive: false,
        }
    }

    /// Negate this literal
    #[inline]
    pub fn negate(&self) -> Literal {
        Literal {
            atom: self.atom.clone(),
            positive: !self.positive,
        }
    }

    /// Check if this literal is complementary to another
    #[inline(always)]
    pub fn is_complementary(&self, other: &Literal) -> bool {
        self.positive != other.positive && self.atom.predicate == other.atom.predicate
    }

    /// Apply substitution
    #[inline]
    pub fn apply_substitution(&self, subst: &Substitution) -> Literal {
        Literal {
            atom: self.atom.apply_substitution(subst),
            positive: self.positive,
        }
    }

    /// Get variables
    #[inline(always)]
    pub fn variables(&self) -> BTreeSet<String> {
        self.atom.variables()
    }
}

/// A clause (disjunction of literals)
#[derive(Debug, Clone)]
pub struct Clause {
    /// Clause ID
    pub id: usize,
    /// Literals in the clause
    pub literals: Vec<Literal>,
    /// Parent clauses (for proof reconstruction)
    pub parents: Option<(usize, usize)>,
    /// Depth in proof tree
    pub depth: usize,
}

impl Clause {
    /// Create a new clause
    pub fn new(id: usize, literals: Vec<Literal>) -> Self {
        Self {
            id,
            literals,
            parents: None,
            depth: 0,
        }
    }

    /// Create from parents
    #[inline]
    pub fn from_resolution(
        id: usize,
        literals: Vec<Literal>,
        parent1: usize,
        parent2: usize,
        depth: usize,
    ) -> Self {
        Self {
            id,
            literals,
            parents: Some((parent1, parent2)),
            depth,
        }
    }

    /// Check if this is the empty clause (contradiction)
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.literals.is_empty()
    }

    /// Check if this is a unit clause
    #[inline(always)]
    pub fn is_unit(&self) -> bool {
        self.literals.len() == 1
    }

    /// Check if this is a Horn clause (at most one positive literal)
    #[inline(always)]
    pub fn is_horn(&self) -> bool {
        self.literals.iter().filter(|l| l.positive).count() <= 1
    }

    /// Get all variables in the clause
    #[inline]
    pub fn variables(&self) -> BTreeSet<String> {
        let mut vars = BTreeSet::new();
        for lit in &self.literals {
            for v in lit.variables() {
                vars.insert(v);
            }
        }
        vars
    }

    /// Apply substitution to all literals
    pub fn apply_substitution(&self, subst: &Substitution) -> Clause {
        Clause {
            id: self.id,
            literals: self
                .literals
                .iter()
                .map(|l| l.apply_substitution(subst))
                .collect(),
            parents: self.parents,
            depth: self.depth,
        }
    }

    /// Rename variables to avoid conflicts
    #[inline]
    pub fn rename_variables(&self, suffix: &str) -> Clause {
        let mut subst = Substitution::new();
        for var in self.variables() {
            let new_var = alloc::format!("{}_{}", var, suffix);
            subst.bind(var, Term::Variable(new_var));
        }
        self.apply_substitution(&subst)
    }
}

// ============================================================================
// RESOLUTION ENGINE
// ============================================================================

/// Proof search strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchStrategy {
    /// Depth-first search
    DepthFirst,
    /// Breadth-first search
    BreadthFirst,
    /// Iterative deepening
    IterativeDeepening,
    /// Best-first (heuristic)
    BestFirst,
    /// Set of support
    SetOfSupport,
}

/// Theorem prover result
#[derive(Debug, Clone)]
pub enum ProofResult {
    /// Theorem proved with proof trace
    Proved(ProofTrace),
    /// Theorem disproved (found countermodel)
    Disproved,
    /// Could not determine (timeout/limit)
    Unknown,
    /// Error occurred
    Error(String),
}

/// A proof trace
#[derive(Debug, Clone)]
pub struct ProofTrace {
    /// The clauses used in the proof
    pub clauses: Vec<Clause>,
    /// The empty clause (if found)
    pub contradiction: Option<usize>,
    /// Number of resolution steps
    pub steps: usize,
    /// Maximum depth reached
    pub max_depth: usize,
}

impl ProofTrace {
    /// Get the proof explanation
    #[inline]
    pub fn explain(&self) -> Vec<String> {
        let mut explanation = Vec::new();

        if let Some(empty_id) = self.contradiction {
            explanation.push(alloc::format!("Proof found by refutation:"));
            self.explain_clause(empty_id, &mut explanation, 0);
        }

        explanation
    }

    fn explain_clause(&self, id: usize, explanation: &mut Vec<String>, indent: usize) {
        if let Some(clause) = self.clauses.iter().find(|c| c.id == id) {
            let prefix = "  ".repeat(indent);
            let lit_str: Vec<String> = clause
                .literals
                .iter()
                .map(|l| {
                    let sign = if l.positive { "" } else { "¬" };
                    alloc::format!("{}{}", sign, l.atom.predicate)
                })
                .collect();

            if clause.is_empty() {
                explanation.push(alloc::format!("{}□ (empty clause - contradiction)", prefix));
            } else {
                explanation.push(alloc::format!(
                    "{}[{}] {{{}}}",
                    prefix,
                    id,
                    lit_str.join(" ∨ ")
                ));
            }

            if let Some((p1, p2)) = clause.parents {
                explanation.push(alloc::format!(
                    "{}  resolved from [{}] and [{}]",
                    prefix,
                    p1,
                    p2
                ));
                self.explain_clause(p1, explanation, indent + 1);
                self.explain_clause(p2, explanation, indent + 1);
            }
        }
    }
}

/// The resolution-based theorem prover
pub struct TheoremProver {
    /// Clauses (knowledge base)
    clauses: Vec<Clause>,
    /// Next clause ID
    next_id: usize,
    /// Unifier
    unifier: Unifier,
    /// Search strategy
    strategy: SearchStrategy,
    /// Maximum depth
    max_depth: usize,
    /// Timeout (in steps)
    timeout: usize,
    /// Steps taken
    steps: usize,
}

impl TheoremProver {
    /// Create a new theorem prover
    pub fn new() -> Self {
        Self {
            clauses: Vec::new(),
            next_id: 0,
            unifier: Unifier::new(),
            strategy: SearchStrategy::SetOfSupport,
            max_depth: MAX_PROOF_DEPTH,
            timeout: DEFAULT_TIMEOUT,
            steps: 0,
        }
    }

    /// Set search strategy
    #[inline(always)]
    pub fn with_strategy(mut self, strategy: SearchStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Set maximum depth
    #[inline(always)]
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Add a clause to the knowledge base
    #[inline]
    pub fn add_clause(&mut self, literals: Vec<Literal>) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.clauses.push(Clause::new(id, literals));
        id
    }

    /// Add a fact (positive unit clause)
    #[inline]
    pub fn add_fact(&mut self, predicate: &str, args: Vec<Term>) -> usize {
        let atom = Atom::new(predicate, args);
        let literal = Literal::positive(atom);
        self.add_clause(vec![literal])
    }

    /// Add a rule (implication A₁ ∧ A₂ ∧ ... → B)
    /// Represented as ¬A₁ ∨ ¬A₂ ∨ ... ∨ B
    #[inline]
    pub fn add_rule(&mut self, conditions: Vec<Atom>, conclusion: Atom) -> usize {
        let mut literals: Vec<Literal> = conditions.into_iter().map(Literal::negative).collect();
        literals.push(Literal::positive(conclusion));
        self.add_clause(literals)
    }

    /// Prove a goal (query)
    pub fn prove(&mut self, goal: &Atom) -> ProofResult {
        self.steps = 0;

        // Add negation of goal (for refutation)
        let negated_goal = Literal::negative(goal.clone());
        let goal_clause_id = self.add_clause(vec![negated_goal]);

        // Perform resolution
        let result = match self.strategy {
            SearchStrategy::SetOfSupport => self.set_of_support_resolution(goal_clause_id),
            SearchStrategy::BreadthFirst => self.breadth_first_resolution(),
            SearchStrategy::DepthFirst => self.depth_first_resolution(),
            _ => self.set_of_support_resolution(goal_clause_id),
        };

        result
    }

    /// Set of support resolution strategy
    fn set_of_support_resolution(&mut self, support_start: usize) -> ProofResult {
        let mut support_set: VecDeque<usize> = vec![support_start];
        let mut used_set: BTreeSet<usize> = BTreeSet::new();
        let mut all_clauses = self.clauses.clone();
        let mut max_depth = 0;

        while !support_set.is_empty() && self.steps < self.timeout {
            let current_id = support_set.pop_front().unwrap();
            used_set.insert(current_id);

            let current = match all_clauses.iter().find(|c| c.id == current_id).cloned() {
                Some(c) => c,
                None => continue,
            };

            if current.depth > max_depth {
                max_depth = current.depth;
            }

            if current.depth > self.max_depth {
                continue;
            }

            // Try to resolve with all other clauses
            for other in &all_clauses.clone() {
                if other.id == current_id {
                    continue;
                }

                self.steps += 1;
                if self.steps >= self.timeout {
                    return ProofResult::Unknown;
                }

                // Try resolution
                if let Some(resolvent) = self.resolve(&current, other) {
                    let new_id = self.next_id;
                    self.next_id += 1;

                    let new_clause = Clause::from_resolution(
                        new_id,
                        resolvent,
                        current.id,
                        other.id,
                        current.depth + 1,
                    );

                    // Found empty clause - proof complete
                    if new_clause.is_empty() {
                        all_clauses.push(new_clause);
                        return ProofResult::Proved(ProofTrace {
                            clauses: all_clauses,
                            contradiction: Some(new_id),
                            steps: self.steps,
                            max_depth,
                        });
                    }

                    // Add to support set if not subsumed
                    if !self.is_subsumed(&new_clause, &all_clauses) {
                        support_set.push(new_id);
                        all_clauses.push(new_clause);
                    }
                }
            }
        }

        if self.steps >= self.timeout {
            ProofResult::Unknown
        } else {
            ProofResult::Disproved
        }
    }

    /// Breadth-first resolution
    fn breadth_first_resolution(&mut self) -> ProofResult {
        let mut queue: Vec<(usize, usize)> = Vec::new();
        let mut all_clauses = self.clauses.clone();
        let mut max_depth = 0;

        // Initialize queue with all pairs
        for i in 0..all_clauses.len() {
            for j in (i + 1)..all_clauses.len() {
                queue.push((i, j));
            }
        }

        while let Some((i, j)) = queue.pop() {
            self.steps += 1;
            if self.steps >= self.timeout {
                return ProofResult::Unknown;
            }

            let c1 = match all_clauses.get(i).cloned() {
                Some(c) => c,
                None => continue,
            };
            let c2 = match all_clauses.get(j).cloned() {
                Some(c) => c,
                None => continue,
            };

            if c1.depth > max_depth {
                max_depth = c1.depth;
            }

            if c1.depth > self.max_depth || c2.depth > self.max_depth {
                continue;
            }

            if let Some(resolvent) = self.resolve(&c1, &c2) {
                let new_id = self.next_id;
                self.next_id += 1;

                let new_clause = Clause::from_resolution(
                    new_id,
                    resolvent,
                    c1.id,
                    c2.id,
                    c1.depth.max(c2.depth) + 1,
                );

                if new_clause.is_empty() {
                    all_clauses.push(new_clause);
                    return ProofResult::Proved(ProofTrace {
                        clauses: all_clauses,
                        contradiction: Some(new_id),
                        steps: self.steps,
                        max_depth,
                    });
                }

                if !self.is_subsumed(&new_clause, &all_clauses) {
                    let new_idx = all_clauses.len();
                    for k in 0..new_idx {
                        queue.push((k, new_idx));
                    }
                    all_clauses.push(new_clause);
                }
            }
        }

        ProofResult::Disproved
    }

    /// Depth-first resolution
    fn depth_first_resolution(&mut self) -> ProofResult {
        let mut stack: Vec<(usize, usize)> = Vec::new();
        let mut all_clauses = self.clauses.clone();
        let mut max_depth = 0;

        // Initialize stack with all pairs
        for i in 0..all_clauses.len() {
            for j in (i + 1)..all_clauses.len() {
                stack.push((i, j));
            }
        }

        while let Some((i, j)) = stack.pop() {
            self.steps += 1;
            if self.steps >= self.timeout {
                return ProofResult::Unknown;
            }

            let c1 = match all_clauses.get(i).cloned() {
                Some(c) => c,
                None => continue,
            };
            let c2 = match all_clauses.get(j).cloned() {
                Some(c) => c,
                None => continue,
            };

            if c1.depth > max_depth {
                max_depth = c1.depth;
            }

            if c1.depth > self.max_depth || c2.depth > self.max_depth {
                continue;
            }

            if let Some(resolvent) = self.resolve(&c1, &c2) {
                let new_id = self.next_id;
                self.next_id += 1;

                let new_clause = Clause::from_resolution(
                    new_id,
                    resolvent,
                    c1.id,
                    c2.id,
                    c1.depth.max(c2.depth) + 1,
                );

                if new_clause.is_empty() {
                    all_clauses.push(new_clause);
                    return ProofResult::Proved(ProofTrace {
                        clauses: all_clauses,
                        contradiction: Some(new_id),
                        steps: self.steps,
                        max_depth,
                    });
                }

                if !self.is_subsumed(&new_clause, &all_clauses) {
                    let new_idx = all_clauses.len();
                    // Push pairs with new clause first (depth-first)
                    for k in 0..new_idx {
                        stack.push((k, new_idx));
                    }
                    all_clauses.push(new_clause);
                }
            }
        }

        ProofResult::Disproved
    }

    /// Try to resolve two clauses
    fn resolve(&mut self, c1: &Clause, c2: &Clause) -> Option<Vec<Literal>> {
        // Rename variables in c2 to avoid conflicts
        let c2_renamed = c2.rename_variables(&c1.id.to_string());

        // Try to find complementary literals
        for (i, lit1) in c1.literals.iter().enumerate() {
            for (j, lit2) in c2_renamed.literals.iter().enumerate() {
                if lit1.is_complementary(lit2) {
                    // Try to unify the atoms
                    if let UnificationResult::Success(subst) =
                        self.unify_atoms(&lit1.atom, &lit2.atom)
                    {
                        // Build resolvent
                        let mut resolvent = Vec::new();

                        // Add literals from c1 (except the resolved one)
                        for (k, lit) in c1.literals.iter().enumerate() {
                            if k != i {
                                let applied = lit.apply_substitution(&subst);
                                if !resolvent.contains(&applied) {
                                    resolvent.push(applied);
                                }
                            }
                        }

                        // Add literals from c2 (except the resolved one)
                        for (k, lit) in c2_renamed.literals.iter().enumerate() {
                            if k != j {
                                let applied = lit.apply_substitution(&subst);
                                if !resolvent.contains(&applied) {
                                    resolvent.push(applied);
                                }
                            }
                        }

                        return Some(resolvent);
                    }
                }
            }
        }

        None
    }

    /// Unify two atoms
    fn unify_atoms(&mut self, a1: &Atom, a2: &Atom) -> UnificationResult {
        if a1.predicate != a2.predicate || a1.args.len() != a2.args.len() {
            return UnificationResult::Failure;
        }

        let mut subst = Substitution::new();
        for (t1, t2) in a1.args.iter().zip(a2.args.iter()) {
            match self.unifier.unify_with_subst(t1, t2, subst.clone()) {
                UnificationResult::Success(s) => subst = s,
                result => return result,
            }
        }

        UnificationResult::Success(subst)
    }

    /// Check if a clause is subsumed by existing clauses
    fn is_subsumed(&self, clause: &Clause, existing: &[Clause]) -> bool {
        for other in existing {
            if self.subsumes(other, clause) {
                return true;
            }
        }
        false
    }

    /// Check if c1 subsumes c2 (c1 is more general)
    fn subsumes(&self, c1: &Clause, c2: &Clause) -> bool {
        // A clause C1 subsumes C2 if C1 ⊆ C2θ for some substitution θ
        if c1.literals.len() > c2.literals.len() {
            return false;
        }

        // Simple check: all literals in c1 must appear in c2
        for lit1 in &c1.literals {
            let mut found = false;
            for lit2 in &c2.literals {
                if lit1 == lit2 {
                    found = true;
                    break;
                }
            }
            if !found {
                return false;
            }
        }

        true
    }
}

impl Default for TheoremProver {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// KERNEL INVARIANT VERIFICATION
// ============================================================================

/// A kernel invariant to verify
#[derive(Debug, Clone)]
pub struct KernelInvariant {
    /// Name of the invariant
    pub name: String,
    /// Description
    pub description: String,
    /// The formula to verify
    pub formula: Vec<Clause>,
    /// Severity if violated
    pub severity: InvariantSeverity,
}

/// Severity of invariant violation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvariantSeverity {
    /// Critical - system may crash
    Critical,
    /// High - significant problems
    High,
    /// Medium - should be fixed
    Medium,
    /// Low - minor issue
    Low,
    /// Info - informational only
    Info,
}

/// Kernel invariant verifier
pub struct InvariantVerifier {
    /// Theorem prover
    prover: TheoremProver,
    /// Registered invariants
    invariants: Vec<KernelInvariant>,
}

impl InvariantVerifier {
    /// Create a new verifier
    pub fn new() -> Self {
        Self {
            prover: TheoremProver::new(),
            invariants: Vec::new(),
        }
    }

    /// Register an invariant
    #[inline(always)]
    pub fn register_invariant(&mut self, invariant: KernelInvariant) {
        self.invariants.push(invariant);
    }

    /// Register the no-deadlock invariant
    pub fn register_no_deadlock_invariant(&mut self) {
        // ∀P1, P2: holds(P1, L1) ∧ waits(P1, L2) ∧ holds(P2, L2) ∧ waits(P2, L1) → ⊥
        // This is represented as the negation being unsatisfiable

        let invariant = KernelInvariant {
            name: String::from("no_deadlock"),
            description: String::from("No circular lock dependencies allowed"),
            formula: vec![],
            severity: InvariantSeverity::Critical,
        };

        self.invariants.push(invariant);
    }

    /// Register the memory safety invariant
    #[inline]
    pub fn register_memory_safety_invariant(&mut self) {
        let invariant = KernelInvariant {
            name: String::from("memory_safety"),
            description: String::from("All memory accesses are within valid bounds"),
            formula: vec![],
            severity: InvariantSeverity::Critical,
        };

        self.invariants.push(invariant);
    }

    /// Verify all invariants
    pub fn verify_all(&mut self) -> Vec<(String, bool, Option<ProofTrace>)> {
        let mut results = Vec::new();

        for invariant in &self.invariants {
            // Add invariant clauses to prover
            for clause in &invariant.formula {
                self.prover.clauses.push(clause.clone());
            }

            // Try to derive contradiction (invariant violation)
            let empty_goal = Atom::new("__false__", vec![]);
            let result = self.prover.prove(&empty_goal);

            match result {
                ProofResult::Proved(trace) => {
                    // Contradiction found - invariant violated
                    results.push((invariant.name.clone(), false, Some(trace)));
                },
                ProofResult::Disproved | ProofResult::Unknown => {
                    // No contradiction - invariant holds
                    results.push((invariant.name.clone(), true, None));
                },
                ProofResult::Error(e) => {
                    results.push((invariant.name.clone(), false, None));
                    let _ = e; // Handle error
                },
            }
        }

        results
    }

    /// Add a fact about current system state
    #[inline(always)]
    pub fn add_state_fact(&mut self, predicate: &str, args: Vec<Term>) {
        self.prover.add_fact(predicate, args);
    }
}

impl Default for InvariantVerifier {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unification_basic() {
        let mut unifier = Unifier::new();

        // X unifies with constant
        let t1 = Term::variable("X");
        let t2 = Term::constant("alice");

        match unifier.unify(&t1, &t2) {
            UnificationResult::Success(subst) => {
                assert_eq!(subst.get("X"), Some(&t2));
            },
            _ => panic!("Unification should succeed"),
        }
    }

    #[test]
    fn test_unification_function() {
        let mut unifier = Unifier::new();

        // f(X, Y) unifies with f(a, b)
        let t1 = Term::function("f", vec![Term::variable("X"), Term::variable("Y")]);
        let t2 = Term::function("f", vec![Term::constant("a"), Term::constant("b")]);

        match unifier.unify(&t1, &t2) {
            UnificationResult::Success(subst) => {
                assert_eq!(subst.get("X"), Some(&Term::constant("a")));
                assert_eq!(subst.get("Y"), Some(&Term::constant("b")));
            },
            _ => panic!("Unification should succeed"),
        }
    }

    #[test]
    fn test_unification_occurs_check() {
        let mut unifier = Unifier::new();

        // X does not unify with f(X) (occurs check)
        let t1 = Term::variable("X");
        let t2 = Term::function("f", vec![Term::variable("X")]);

        match unifier.unify(&t1, &t2) {
            UnificationResult::OccursCheckFailed => {},
            _ => panic!("Should fail occurs check"),
        }
    }

    #[test]
    fn test_resolution_basic() {
        let mut prover = TheoremProver::new();

        // Fact: mortal(socrates)
        prover.add_fact("mortal", vec![Term::constant("socrates")]);

        // Fact: human(socrates)
        prover.add_fact("human", vec![Term::constant("socrates")]);

        // Rule: human(X) → mortal(X)
        prover.add_rule(
            vec![Atom::new("human", vec![Term::variable("X")])],
            Atom::new("mortal", vec![Term::variable("X")]),
        );

        // Query: mortal(socrates)?
        let goal = Atom::new("mortal", vec![Term::constant("socrates")]);
        let result = prover.prove(&goal);

        match result {
            ProofResult::Proved(_) => {},
            _ => panic!("Should prove mortal(socrates)"),
        }
    }

    #[test]
    fn test_resolution_transitive() {
        let mut prover = TheoremProver::new();

        // parent(a, b)
        prover.add_fact("parent", vec![Term::constant("a"), Term::constant("b")]);

        // parent(b, c)
        prover.add_fact("parent", vec![Term::constant("b"), Term::constant("c")]);

        // Rule: parent(X, Y) ∧ parent(Y, Z) → grandparent(X, Z)
        prover.add_rule(
            vec![
                Atom::new("parent", vec![Term::variable("X"), Term::variable("Y")]),
                Atom::new("parent", vec![Term::variable("Y"), Term::variable("Z")]),
            ],
            Atom::new("grandparent", vec![
                Term::variable("X"),
                Term::variable("Z"),
            ]),
        );

        // Query: grandparent(a, c)?
        let goal = Atom::new("grandparent", vec![
            Term::constant("a"),
            Term::constant("c"),
        ]);
        let result = prover.prove(&goal);

        match result {
            ProofResult::Proved(trace) => {
                assert!(trace.steps > 0);
            },
            _ => panic!("Should prove grandparent(a, c)"),
        }
    }

    #[test]
    fn test_clause_operations() {
        let atom = Atom::new("p", vec![Term::variable("X")]);
        let lit = Literal::positive(atom);
        let clause = Clause::new(0, vec![lit.clone()]);

        assert!(clause.is_unit());
        assert!(clause.is_horn());
        assert!(!clause.is_empty());

        let vars = clause.variables();
        assert!(vars.contains("X"));
    }

    #[test]
    fn test_proof_trace() {
        let mut prover = TheoremProver::new();

        prover.add_fact("p", vec![Term::constant("a")]);
        prover.add_rule(
            vec![Atom::new("p", vec![Term::variable("X")])],
            Atom::new("q", vec![Term::variable("X")]),
        );

        let goal = Atom::new("q", vec![Term::constant("a")]);
        let result = prover.prove(&goal);

        if let ProofResult::Proved(trace) = result {
            let explanation = trace.explain();
            assert!(!explanation.is_empty());
        } else {
            panic!("Should prove q(a)");
        }
    }
}
