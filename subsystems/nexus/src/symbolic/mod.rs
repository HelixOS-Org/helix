//! # Symbolic AI Integration for Kernel Reasoning
//!
//! Revolutionary hybrid neural-symbolic reasoning system that combines
//! the pattern recognition of neural networks with the logical rigor
//! of symbolic AI for kernel-level decision making.
//!
//! ## Features
//!
//! - **Neural-Symbolic Bridge**: Seamless integration of neural and symbolic reasoning
//! - **Logical Rule Learning**: Automatically learn logical rules from data
//! - **Theorem Proving**: Kernel invariant verification
//! - **Constraint Satisfaction**: Solve complex kernel constraints
//! - **Abductive Reasoning**: Explain kernel behaviors
//! - **Causal Inference**: Understand cause-effect relationships
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                    NEURAL-SYMBOLIC HYBRID SYSTEM                        │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │  ┌──────────────────────────────────────────────────────────────────┐   │
//! │  │                    PERCEPTION LAYER (Neural)                      │   │
//! │  │   Raw Data → Features → Embeddings → Concept Vectors            │   │
//! │  └──────────────────────────────────────────────────────────────────┘   │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌──────────────────────────────────────────────────────────────────┐   │
//! │  │                    SYMBOL GROUNDING LAYER                         │   │
//! │  │   Concept Vectors ↔ Logical Symbols                              │   │
//! │  │   Neural Predicates ↔ Logical Predicates                         │   │
//! │  └──────────────────────────────────────────────────────────────────┘   │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌──────────────────────────────────────────────────────────────────┐   │
//! │  │                    REASONING LAYER (Symbolic)                     │   │
//! │  │   First-Order Logic │ Horn Clauses │ Constraint Solving          │   │
//! │  └──────────────────────────────────────────────────────────────────┘   │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌──────────────────────────────────────────────────────────────────┐   │
//! │  │                    DECISION LAYER (Hybrid)                        │   │
//! │  │   Neural Confidence + Logical Certainty → Action                 │   │
//! │  └──────────────────────────────────────────────────────────────────┘   │
//! │                                                                          │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

#![allow(dead_code)]

extern crate alloc;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// Year 2 COGNITION sub-modules
pub mod constraint;
pub mod knowledge_graph;
pub mod ontology;
pub mod theorem;

// Re-exports
use core::cmp::Ordering;

pub use constraint::{CspSolver, DpllSolver, MemoryAllocationCsp, SchedulerCsp};
pub use knowledge_graph::{
    Entity, KernelKnowledgeGraph, KnowledgeGraph, QueryEngine, Relation, Triple,
};
pub use ontology::{Individual, KernelOntology, Ontology, OntologyClass, OntologyProperty};
pub use theorem::{Atom as TheoremAtom, Clause as TheoremClause, Term as TheoremTerm};
// Re-export theorem types with aliases to avoid conflicts with local types
pub use theorem::{KernelInvariant, ProofTrace, TheoremProver};

use crate::math::F64Ext;

/// Logical term representation
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Term {
    /// Constant symbol (e.g., 'process_123')
    Constant(Symbol),
    /// Variable (e.g., X, Y, Z)
    Variable(Symbol),
    /// Integer value
    Integer(i64),
    /// Boolean value
    Boolean(bool),
    /// Compound term (functor with arguments)
    Compound(Symbol, Vec<Term>),
    /// List of terms
    List(Vec<Term>),
}

impl Term {
    /// Create a constant term
    pub fn constant(name: &str) -> Self {
        Term::Constant(Symbol::new(name))
    }

    /// Create a variable term
    pub fn variable(name: &str) -> Self {
        Term::Variable(Symbol::new(name))
    }

    /// Create a compound term
    pub fn compound(functor: &str, args: Vec<Term>) -> Self {
        Term::Compound(Symbol::new(functor), args)
    }

    /// Create an integer term
    pub fn integer(value: i64) -> Self {
        Term::Integer(value)
    }

    /// Check if term is a variable
    pub fn is_variable(&self) -> bool {
        matches!(self, Term::Variable(_))
    }

    /// Check if term is ground (no variables)
    pub fn is_ground(&self) -> bool {
        match self {
            Term::Variable(_) => false,
            Term::Constant(_) | Term::Integer(_) | Term::Boolean(_) => true,
            Term::Compound(_, args) => args.iter().all(|a| a.is_ground()),
            Term::List(items) => items.iter().all(|i| i.is_ground()),
        }
    }

    /// Get all variables in the term
    pub fn variables(&self) -> Vec<Symbol> {
        match self {
            Term::Variable(v) => alloc::vec![v.clone()],
            Term::Compound(_, args) => args.iter().flat_map(|a| a.variables()).collect(),
            Term::List(items) => items.iter().flat_map(|i| i.variables()).collect(),
            _ => Vec::new(),
        }
    }

    /// Apply substitution to term
    pub fn apply_substitution(&self, subst: &Substitution) -> Term {
        match self {
            Term::Variable(v) => subst.get(v).cloned().unwrap_or_else(|| self.clone()),
            Term::Compound(f, args) => {
                let new_args = args.iter().map(|a| a.apply_substitution(subst)).collect();
                Term::Compound(f.clone(), new_args)
            },
            Term::List(items) => {
                let new_items = items.iter().map(|i| i.apply_substitution(subst)).collect();
                Term::List(new_items)
            },
            _ => self.clone(),
        }
    }
}

/// Symbol (identifier)
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Symbol {
    pub name: String,
    pub id: u32,
}

impl Symbol {
    /// Create a new symbol
    pub fn new(name: &str) -> Self {
        // Simple hash for ID
        let id = name
            .bytes()
            .fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
        Self {
            name: String::from(name),
            id,
        }
    }
}

/// Substitution (variable bindings)
#[derive(Debug, Clone, Default)]
pub struct Substitution {
    bindings: BTreeMap<Symbol, Term>,
}

impl Substitution {
    /// Create empty substitution
    pub fn new() -> Self {
        Self {
            bindings: BTreeMap::new(),
        }
    }

    /// Bind variable to term
    pub fn bind(&mut self, var: Symbol, term: Term) {
        self.bindings.insert(var, term);
    }

    /// Get binding for variable
    pub fn get(&self, var: &Symbol) -> Option<&Term> {
        self.bindings.get(var)
    }

    /// Check if substitution is empty
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    /// Compose two substitutions
    pub fn compose(&self, other: &Substitution) -> Substitution {
        let mut result = self.clone();
        for (var, term) in &other.bindings {
            let applied = term.apply_substitution(self);
            result.bindings.insert(var.clone(), applied);
        }
        result
    }
}

/// Atomic formula (predicate with terms)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Atom {
    pub predicate: Symbol,
    pub args: Vec<Term>,
}

impl Atom {
    /// Create a new atom
    pub fn new(predicate: &str, args: Vec<Term>) -> Self {
        Self {
            predicate: Symbol::new(predicate),
            args,
        }
    }

    /// Apply substitution
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

    /// Get all variables
    pub fn variables(&self) -> Vec<Symbol> {
        self.args.iter().flat_map(|a| a.variables()).collect()
    }

    /// Check if ground
    pub fn is_ground(&self) -> bool {
        self.args.iter().all(|a| a.is_ground())
    }
}

/// Logical formula
#[derive(Debug, Clone, PartialEq)]
pub enum Formula {
    /// Atomic formula
    Atom(Atom),
    /// Negation
    Not(Box<Formula>),
    /// Conjunction
    And(Vec<Formula>),
    /// Disjunction
    Or(Vec<Formula>),
    /// Implication
    Implies(Box<Formula>, Box<Formula>),
    /// Universal quantification
    Forall(Symbol, Box<Formula>),
    /// Existential quantification
    Exists(Symbol, Box<Formula>),
    /// True constant
    True,
    /// False constant
    False,
    /// Comparison
    Compare(Term, CompareOp, Term),
}

/// Comparison operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

impl Formula {
    /// Create an atomic formula
    pub fn atom(predicate: &str, args: Vec<Term>) -> Self {
        Formula::Atom(Atom::new(predicate, args))
    }

    /// Create negation
    pub fn not(f: Formula) -> Self {
        Formula::Not(Box::new(f))
    }

    /// Create conjunction
    pub fn and(formulas: Vec<Formula>) -> Self {
        if formulas.is_empty() {
            Formula::True
        } else if formulas.len() == 1 {
            formulas.into_iter().next().unwrap()
        } else {
            Formula::And(formulas)
        }
    }

    /// Create disjunction
    pub fn or(formulas: Vec<Formula>) -> Self {
        if formulas.is_empty() {
            Formula::False
        } else if formulas.len() == 1 {
            formulas.into_iter().next().unwrap()
        } else {
            Formula::Or(formulas)
        }
    }

    /// Create implication
    pub fn implies(antecedent: Formula, consequent: Formula) -> Self {
        Formula::Implies(Box::new(antecedent), Box::new(consequent))
    }

    /// Convert to conjunctive normal form (CNF)
    pub fn to_cnf(&self) -> Formula {
        match self {
            Formula::Not(inner) => match inner.as_ref() {
                Formula::Not(f) => f.to_cnf(),
                Formula::And(fs) => Formula::or(
                    fs.iter()
                        .map(|f| Formula::not(f.clone()).to_cnf())
                        .collect(),
                ),
                Formula::Or(fs) => Formula::and(
                    fs.iter()
                        .map(|f| Formula::not(f.clone()).to_cnf())
                        .collect(),
                ),
                _ => self.clone(),
            },
            Formula::And(fs) => Formula::And(fs.iter().map(|f| f.to_cnf()).collect()),
            Formula::Or(fs) => {
                let cnfs: Vec<Formula> = fs.iter().map(|f| f.to_cnf()).collect();
                self.distribute_or(&cnfs)
            },
            Formula::Implies(a, b) => {
                Formula::or(alloc::vec![Formula::not(*a.clone()), *b.clone()]).to_cnf()
            },
            _ => self.clone(),
        }
    }

    fn distribute_or(&self, formulas: &[Formula]) -> Formula {
        if formulas.is_empty() {
            return Formula::False;
        }
        if formulas.len() == 1 {
            return formulas[0].clone();
        }

        // Simple distribution for two formulas
        let first = &formulas[0];
        let rest = self.distribute_or(&formulas[1..]);

        match (first, &rest) {
            (Formula::And(fs1), Formula::And(fs2)) => {
                let mut result = Vec::new();
                for f1 in fs1 {
                    for f2 in fs2 {
                        result.push(Formula::or(alloc::vec![f1.clone(), f2.clone()]));
                    }
                }
                Formula::And(result)
            },
            (Formula::And(fs), other) | (other, Formula::And(fs)) => {
                let result: Vec<Formula> = fs
                    .iter()
                    .map(|f| Formula::or(alloc::vec![f.clone(), other.clone()]))
                    .collect();
                Formula::And(result)
            },
            _ => Formula::Or(formulas.to_vec()),
        }
    }
}

/// Horn clause (rule)
#[derive(Debug, Clone)]
pub struct Clause {
    /// Head (conclusion)
    pub head: Atom,
    /// Body (conditions)
    pub body: Vec<Atom>,
    /// Confidence score (for soft rules)
    pub confidence: f64,
}

impl Clause {
    /// Create a fact (clause with no body)
    pub fn fact(head: Atom) -> Self {
        Self {
            head,
            body: Vec::new(),
            confidence: 1.0,
        }
    }

    /// Create a rule
    pub fn rule(head: Atom, body: Vec<Atom>) -> Self {
        Self {
            head,
            body,
            confidence: 1.0,
        }
    }

    /// Create a soft rule with confidence
    pub fn soft_rule(head: Atom, body: Vec<Atom>, confidence: f64) -> Self {
        Self {
            head,
            body,
            confidence,
        }
    }

    /// Check if this is a fact
    pub fn is_fact(&self) -> bool {
        self.body.is_empty()
    }
}

/// Knowledge base (set of clauses)
#[derive(Debug, Clone, Default)]
pub struct KnowledgeBase {
    /// Clauses indexed by predicate
    clauses: BTreeMap<Symbol, Vec<Clause>>,
    /// Constraint rules
    constraints: Vec<Constraint>,
    /// Integrity constraints
    integrity_constraints: Vec<Formula>,
}

impl KnowledgeBase {
    /// Create empty knowledge base
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a clause
    pub fn add_clause(&mut self, clause: Clause) {
        let pred = clause.head.predicate.clone();
        self.clauses.entry(pred).or_default().push(clause);
    }

    /// Add a fact
    pub fn add_fact(&mut self, predicate: &str, args: Vec<Term>) {
        self.add_clause(Clause::fact(Atom::new(predicate, args)));
    }

    /// Add a rule
    pub fn add_rule(&mut self, head: Atom, body: Vec<Atom>) {
        self.add_clause(Clause::rule(head, body));
    }

    /// Add integrity constraint
    pub fn add_integrity_constraint(&mut self, formula: Formula) {
        self.integrity_constraints.push(formula);
    }

    /// Get clauses for predicate
    pub fn get_clauses(&self, predicate: &Symbol) -> &[Clause] {
        self.clauses
            .get(predicate)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Count facts
    pub fn num_facts(&self) -> usize {
        self.clauses
            .values()
            .flat_map(|cs| cs.iter())
            .filter(|c| c.is_fact())
            .count()
    }

    /// Count rules
    pub fn num_rules(&self) -> usize {
        self.clauses
            .values()
            .flat_map(|cs| cs.iter())
            .filter(|c| !c.is_fact())
            .count()
    }
}

/// Constraint for constraint satisfaction
#[derive(Debug, Clone)]
pub struct Constraint {
    /// Variables involved
    pub variables: Vec<Symbol>,
    /// Constraint expression
    pub expression: ConstraintExpr,
}

/// Constraint expression
#[derive(Debug, Clone)]
pub enum ConstraintExpr {
    /// Linear constraint: a*x + b*y + ... <= c
    Linear(Vec<(f64, Symbol)>, CompareOp, f64),
    /// Equality constraint
    Equality(Term, Term),
    /// Domain constraint
    Domain(Symbol, Vec<Term>),
    /// All different constraint
    AllDifferent(Vec<Symbol>),
    /// Table constraint (allowed tuples)
    Table(Vec<Symbol>, Vec<Vec<Term>>),
}

/// Unification engine
pub struct UnificationEngine {
    /// Maximum unification depth
    max_depth: usize,
    /// Occurs check enabled
    occurs_check: bool,
}

impl UnificationEngine {
    /// Create a new unification engine
    pub fn new() -> Self {
        Self {
            max_depth: 1000,
            occurs_check: true,
        }
    }

    /// Unify two terms
    pub fn unify(&self, term1: &Term, term2: &Term) -> Option<Substitution> {
        self.unify_with_subst(term1, term2, Substitution::new(), 0)
    }

    fn unify_with_subst(
        &self,
        term1: &Term,
        term2: &Term,
        subst: Substitution,
        depth: usize,
    ) -> Option<Substitution> {
        if depth > self.max_depth {
            return None;
        }

        let t1 = term1.apply_substitution(&subst);
        let t2 = term2.apply_substitution(&subst);

        match (&t1, &t2) {
            // Same terms
            _ if t1 == t2 => Some(subst),

            // Variable cases
            (Term::Variable(v), term) | (term, Term::Variable(v)) => {
                if self.occurs_check && self.occurs(v, term) {
                    None
                } else {
                    let mut new_subst = subst;
                    new_subst.bind(v.clone(), term.clone());
                    Some(new_subst)
                }
            },

            // Compound terms
            (Term::Compound(f1, args1), Term::Compound(f2, args2)) => {
                if f1 != f2 || args1.len() != args2.len() {
                    return None;
                }

                let mut current_subst = subst;
                for (a1, a2) in args1.iter().zip(args2.iter()) {
                    current_subst = self.unify_with_subst(a1, a2, current_subst, depth + 1)?;
                }
                Some(current_subst)
            },

            // Lists
            (Term::List(items1), Term::List(items2)) => {
                if items1.len() != items2.len() {
                    return None;
                }

                let mut current_subst = subst;
                for (i1, i2) in items1.iter().zip(items2.iter()) {
                    current_subst = self.unify_with_subst(i1, i2, current_subst, depth + 1)?;
                }
                Some(current_subst)
            },

            _ => None,
        }
    }

    /// Check if variable occurs in term (occurs check)
    fn occurs(&self, var: &Symbol, term: &Term) -> bool {
        match term {
            Term::Variable(v) => v == var,
            Term::Compound(_, args) => args.iter().any(|a| self.occurs(var, a)),
            Term::List(items) => items.iter().any(|i| self.occurs(var, i)),
            _ => false,
        }
    }

    /// Unify two atoms
    pub fn unify_atoms(&self, atom1: &Atom, atom2: &Atom) -> Option<Substitution> {
        if atom1.predicate != atom2.predicate || atom1.args.len() != atom2.args.len() {
            return None;
        }

        let mut subst = Substitution::new();
        for (a1, a2) in atom1.args.iter().zip(atom2.args.iter()) {
            subst = self.unify_with_subst(a1, a2, subst, 0)?;
        }
        Some(subst)
    }
}

/// Prolog-style inference engine
pub struct InferenceEngine {
    /// Knowledge base
    kb: KnowledgeBase,
    /// Unification engine
    unify: UnificationEngine,
    /// Maximum proof depth
    max_depth: usize,
    /// Variable counter for renaming
    var_counter: u64,
    /// Proof trace
    trace: Vec<ProofStep>,
}

/// Proof step for explanation
#[derive(Debug, Clone)]
pub struct ProofStep {
    pub goal: Atom,
    pub clause_used: Option<Clause>,
    pub substitution: Substitution,
    pub depth: usize,
}

impl InferenceEngine {
    /// Create a new inference engine
    pub fn new(kb: KnowledgeBase) -> Self {
        Self {
            kb,
            unify: UnificationEngine::new(),
            max_depth: 100,
            var_counter: 0,
            trace: Vec::new(),
        }
    }

    /// Query the knowledge base
    pub fn query(&mut self, goal: &Atom) -> Vec<Substitution> {
        self.trace.clear();
        self.prove(goal, Substitution::new(), 0)
    }

    /// Query with limit on solutions
    pub fn query_first(&mut self, goal: &Atom) -> Option<Substitution> {
        self.query(goal).into_iter().next()
    }

    /// Prove a goal
    fn prove(&mut self, goal: &Atom, subst: Substitution, depth: usize) -> Vec<Substitution> {
        if depth > self.max_depth {
            return Vec::new();
        }

        let goal = goal.apply_substitution(&subst);
        let mut solutions = Vec::new();

        // Try each clause that might unify with the goal
        let clauses = self.kb.get_clauses(&goal.predicate).to_vec();

        for clause in clauses {
            // Rename variables to avoid capture
            let renamed = self.rename_clause(&clause);

            // Try to unify goal with clause head
            if let Some(new_subst) = self.unify.unify_atoms(&goal, &renamed.head) {
                let combined = subst.compose(&new_subst);

                // Record proof step
                self.trace.push(ProofStep {
                    goal: goal.clone(),
                    clause_used: Some(clause.clone()),
                    substitution: combined.clone(),
                    depth,
                });

                if renamed.body.is_empty() {
                    // Fact - we have a solution
                    solutions.push(combined);
                } else {
                    // Rule - need to prove body
                    let body_solutions = self.prove_body(&renamed.body, combined, depth + 1);
                    solutions.extend(body_solutions);
                }
            }
        }

        solutions
    }

    /// Prove all goals in body
    fn prove_body(
        &mut self,
        body: &[Atom],
        subst: Substitution,
        depth: usize,
    ) -> Vec<Substitution> {
        if body.is_empty() {
            return alloc::vec![subst];
        }

        let first = &body[0];
        let rest = &body[1..];

        let first_solutions = self.prove(first, subst, depth);

        let mut all_solutions = Vec::new();
        for sol in first_solutions {
            let rest_solutions = self.prove_body(rest, sol, depth);
            all_solutions.extend(rest_solutions);
        }

        all_solutions
    }

    /// Rename variables in clause to avoid capture
    fn rename_clause(&mut self, clause: &Clause) -> Clause {
        let mut renaming = BTreeMap::new();

        let head_vars = clause.head.variables();
        let body_vars: Vec<Symbol> = clause.body.iter().flat_map(|a| a.variables()).collect();

        for var in head_vars.into_iter().chain(body_vars) {
            if !renaming.contains_key(&var) {
                self.var_counter += 1;
                let new_name = alloc::format!("_G{}", self.var_counter);
                renaming.insert(var, Symbol::new(&new_name));
            }
        }

        let mut subst = Substitution::new();
        for (old, new) in renaming {
            subst.bind(old, Term::Variable(new));
        }

        Clause {
            head: clause.head.apply_substitution(&subst),
            body: clause
                .body
                .iter()
                .map(|a| a.apply_substitution(&subst))
                .collect(),
            confidence: clause.confidence,
        }
    }

    /// Get proof trace
    pub fn get_trace(&self) -> &[ProofStep] {
        &self.trace
    }

    /// Check if goal is provable
    pub fn is_provable(&mut self, goal: &Atom) -> bool {
        !self.query(goal).is_empty()
    }
}

/// Neural-symbolic bridge
pub struct NeuralSymbolicBridge {
    /// Concept embeddings (neural representations of symbols)
    concept_embeddings: BTreeMap<Symbol, Vec<f64>>,
    /// Predicate confidence scores
    predicate_confidence: BTreeMap<(Symbol, Vec<Term>), f64>,
    /// Embedding dimension
    embedding_dim: usize,
}

impl NeuralSymbolicBridge {
    /// Create a new bridge
    pub fn new(embedding_dim: usize) -> Self {
        Self {
            concept_embeddings: BTreeMap::new(),
            predicate_confidence: BTreeMap::new(),
            embedding_dim,
        }
    }

    /// Register a symbol with its embedding
    pub fn register_symbol(&mut self, symbol: Symbol, embedding: Vec<f64>) {
        self.concept_embeddings.insert(symbol, embedding);
    }

    /// Get embedding for symbol
    pub fn get_embedding(&self, symbol: &Symbol) -> Option<&Vec<f64>> {
        self.concept_embeddings.get(symbol)
    }

    /// Update predicate confidence from neural network output
    pub fn update_confidence(&mut self, predicate: Symbol, args: Vec<Term>, confidence: f64) {
        self.predicate_confidence
            .insert((predicate, args), confidence);
    }

    /// Get confidence for predicate
    pub fn get_confidence(&self, predicate: &Symbol, args: &[Term]) -> f64 {
        self.predicate_confidence
            .get(&(predicate.clone(), args.to_vec()))
            .copied()
            .unwrap_or(1.0)
    }

    /// Find similar concepts based on embedding similarity
    pub fn find_similar(&self, symbol: &Symbol, k: usize) -> Vec<(Symbol, f64)> {
        let embedding = match self.get_embedding(symbol) {
            Some(e) => e,
            None => return Vec::new(),
        };

        let mut similarities: Vec<(Symbol, f64)> = self
            .concept_embeddings
            .iter()
            .filter(|(s, _)| *s != symbol)
            .map(|(s, e)| {
                let sim = self.cosine_similarity(embedding, e);
                (s.clone(), sim)
            })
            .collect();

        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        similarities.into_iter().take(k).collect()
    }

    fn cosine_similarity(&self, a: &[f64], b: &[f64]) -> f64 {
        let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
        let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();

        if norm_a > 1e-10 && norm_b > 1e-10 {
            dot / (norm_a * norm_b)
        } else {
            0.0
        }
    }
}

/// Rule learning engine (ILP - Inductive Logic Programming)
pub struct RuleLearner {
    /// Positive examples
    positive_examples: Vec<Atom>,
    /// Negative examples
    negative_examples: Vec<Atom>,
    /// Background knowledge
    background: KnowledgeBase,
    /// Maximum rule complexity
    max_complexity: usize,
}

impl RuleLearner {
    /// Create a new rule learner
    pub fn new(background: KnowledgeBase) -> Self {
        Self {
            positive_examples: Vec::new(),
            negative_examples: Vec::new(),
            background,
            max_complexity: 5,
        }
    }

    /// Add positive example
    pub fn add_positive(&mut self, example: Atom) {
        self.positive_examples.push(example);
    }

    /// Add negative example
    pub fn add_negative(&mut self, example: Atom) {
        self.negative_examples.push(example);
    }

    /// Learn rules from examples
    pub fn learn(&self) -> Vec<Clause> {
        // Simplified rule learning using covering algorithm
        let mut learned_rules = Vec::new();
        let mut uncovered: Vec<_> = self.positive_examples.iter().cloned().collect();

        while !uncovered.is_empty() && learned_rules.len() < 10 {
            // Find best clause covering some positive examples
            if let Some(clause) = self.find_best_clause(&uncovered) {
                // Remove covered examples
                let engine = InferenceEngine::new(self.background.clone());
                uncovered.retain(|ex| !self.covers(&clause, ex, &engine));

                learned_rules.push(clause);
            } else {
                break;
            }
        }

        learned_rules
    }

    fn find_best_clause(&self, uncovered: &[Atom]) -> Option<Clause> {
        // Simplified: create a clause from the first uncovered example
        if let Some(example) = uncovered.first() {
            let mut body = Vec::new();

            // Generalize: replace constants with variables
            let generalized_head = self.generalize(example);

            // Add relevant background predicates as body
            for clause in self.background.clauses.values().flatten() {
                if clause.is_fact() {
                    // Check if this fact is relevant
                    let shared_vars = self.shared_variables(&generalized_head, &clause.head);
                    if !shared_vars.is_empty() {
                        body.push(clause.head.clone());
                        if body.len() >= self.max_complexity {
                            break;
                        }
                    }
                }
            }

            Some(Clause::rule(generalized_head, body))
        } else {
            None
        }
    }

    fn generalize(&self, atom: &Atom) -> Atom {
        let mut var_counter = 0;
        let args: Vec<Term> = atom
            .args
            .iter()
            .map(|arg| {
                if let Term::Constant(_) = arg {
                    var_counter += 1;
                    Term::variable(&alloc::format!("X{}", var_counter))
                } else {
                    arg.clone()
                }
            })
            .collect();

        Atom::new(&atom.predicate.name, args)
    }

    fn shared_variables(&self, a1: &Atom, a2: &Atom) -> Vec<Symbol> {
        let vars1: alloc::collections::BTreeSet<_> = a1.variables().into_iter().collect();
        let vars2: alloc::collections::BTreeSet<_> = a2.variables().into_iter().collect();
        vars1.intersection(&vars2).cloned().collect()
    }

    fn covers(&self, clause: &Clause, example: &Atom, _engine: &InferenceEngine) -> bool {
        // Simplified coverage check
        clause.head.predicate == example.predicate
    }
}

/// Kernel reasoning system using symbolic AI
pub struct KernelSymbolicReasoner {
    /// Knowledge base
    kb: KnowledgeBase,
    /// Inference engine
    engine: InferenceEngine,
    /// Neural-symbolic bridge
    bridge: NeuralSymbolicBridge,
    /// Rule learner
    learner: RuleLearner,
}

impl KernelSymbolicReasoner {
    /// Create a new kernel reasoner
    pub fn new() -> Self {
        let kb = Self::create_kernel_kb();
        let engine = InferenceEngine::new(kb.clone());
        let bridge = NeuralSymbolicBridge::new(64);
        let learner = RuleLearner::new(kb.clone());

        Self {
            kb,
            engine,
            bridge,
            learner,
        }
    }

    /// Create kernel domain knowledge base
    fn create_kernel_kb() -> KnowledgeBase {
        let mut kb = KnowledgeBase::new();

        // Process scheduling rules
        kb.add_rule(
            Atom::new("can_preempt", alloc::vec![
                Term::variable("P1"),
                Term::variable("P2"),
            ]),
            alloc::vec![
                Atom::new("priority", alloc::vec![
                    Term::variable("P1"),
                    Term::variable("Prio1")
                ]),
                Atom::new("priority", alloc::vec![
                    Term::variable("P2"),
                    Term::variable("Prio2")
                ]),
                Atom::new("greater", alloc::vec![
                    Term::variable("Prio1"),
                    Term::variable("Prio2")
                ]),
            ],
        );

        // Memory safety rules
        kb.add_rule(
            Atom::new("safe_access", alloc::vec![
                Term::variable("Proc"),
                Term::variable("Addr"),
            ]),
            alloc::vec![
                Atom::new("owns_page", alloc::vec![
                    Term::variable("Proc"),
                    Term::variable("Page")
                ]),
                Atom::new("in_page", alloc::vec![
                    Term::variable("Addr"),
                    Term::variable("Page")
                ]),
            ],
        );

        // Deadlock detection rules
        kb.add_rule(
            Atom::new("potential_deadlock", alloc::vec![
                Term::variable("P1"),
                Term::variable("P2"),
            ]),
            alloc::vec![
                Atom::new("holds_lock", alloc::vec![
                    Term::variable("P1"),
                    Term::variable("L1")
                ]),
                Atom::new("waits_lock", alloc::vec![
                    Term::variable("P1"),
                    Term::variable("L2")
                ]),
                Atom::new("holds_lock", alloc::vec![
                    Term::variable("P2"),
                    Term::variable("L2")
                ]),
                Atom::new("waits_lock", alloc::vec![
                    Term::variable("P2"),
                    Term::variable("L1")
                ]),
            ],
        );

        // Resource allocation rules
        kb.add_rule(
            Atom::new("can_allocate", alloc::vec![
                Term::variable("Proc"),
                Term::variable("Resource"),
                Term::variable("Amount"),
            ]),
            alloc::vec![
                Atom::new("available", alloc::vec![
                    Term::variable("Resource"),
                    Term::variable("Avail")
                ]),
                Atom::new("requested", alloc::vec![
                    Term::variable("Proc"),
                    Term::variable("Resource"),
                    Term::variable("Amount")
                ]),
                Atom::new("greater_equal", alloc::vec![
                    Term::variable("Avail"),
                    Term::variable("Amount")
                ]),
            ],
        );

        kb
    }

    /// Query if action is safe
    pub fn is_safe(&mut self, action: &str, args: Vec<Term>) -> bool {
        let goal = Atom::new(action, args);
        self.engine.is_provable(&goal)
    }

    /// Check for potential deadlock
    pub fn check_deadlock(&mut self, proc1: &str, proc2: &str) -> bool {
        let goal = Atom::new("potential_deadlock", alloc::vec![
            Term::constant(proc1),
            Term::constant(proc2),
        ]);
        self.engine.is_provable(&goal)
    }

    /// Explain a decision
    pub fn explain(&self, _goal: &Atom) -> Vec<ProofStep> {
        self.engine.get_trace().to_vec()
    }

    /// Learn new rules from observations
    pub fn learn_from_observations(&mut self, observations: Vec<(Atom, bool)>) {
        for (atom, is_positive) in observations {
            if is_positive {
                self.learner.add_positive(atom);
            } else {
                self.learner.add_negative(atom);
            }
        }

        let new_rules = self.learner.learn();
        for rule in new_rules {
            self.kb.add_clause(rule);
        }

        // Rebuild inference engine with updated KB
        self.engine = InferenceEngine::new(self.kb.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unification() {
        let engine = UnificationEngine::new();

        // Variable unifies with constant
        let t1 = Term::variable("X");
        let t2 = Term::constant("foo");
        let subst = engine.unify(&t1, &t2).unwrap();
        assert!(!subst.is_empty());

        // Compound terms
        let t3 = Term::compound("f", alloc::vec![Term::variable("X"), Term::constant("a")]);
        let t4 = Term::compound("f", alloc::vec![Term::constant("b"), Term::variable("Y")]);
        let subst = engine.unify(&t3, &t4).unwrap();

        let x = Symbol::new("X");
        let y = Symbol::new("Y");
        assert_eq!(subst.get(&x), Some(&Term::constant("b")));
        assert_eq!(subst.get(&y), Some(&Term::constant("a")));
    }

    #[test]
    fn test_inference() {
        let mut kb = KnowledgeBase::new();

        // Facts
        kb.add_fact("parent", alloc::vec![
            Term::constant("alice"),
            Term::constant("bob")
        ]);
        kb.add_fact("parent", alloc::vec![
            Term::constant("bob"),
            Term::constant("charlie")
        ]);

        // Rule: grandparent(X, Z) :- parent(X, Y), parent(Y, Z)
        kb.add_rule(
            Atom::new("grandparent", alloc::vec![
                Term::variable("X"),
                Term::variable("Z")
            ]),
            alloc::vec![
                Atom::new("parent", alloc::vec![
                    Term::variable("X"),
                    Term::variable("Y")
                ]),
                Atom::new("parent", alloc::vec![
                    Term::variable("Y"),
                    Term::variable("Z")
                ]),
            ],
        );

        let mut engine = InferenceEngine::new(kb);

        // Query: grandparent(alice, charlie)?
        let goal = Atom::new("grandparent", alloc::vec![
            Term::constant("alice"),
            Term::constant("charlie"),
        ]);

        assert!(engine.is_provable(&goal));
    }

    #[test]
    fn test_neural_symbolic_bridge() {
        let mut bridge = NeuralSymbolicBridge::new(3);

        bridge.register_symbol(Symbol::new("process"), alloc::vec![1.0, 0.0, 0.0]);
        bridge.register_symbol(Symbol::new("thread"), alloc::vec![0.9, 0.1, 0.0]);
        bridge.register_symbol(Symbol::new("memory"), alloc::vec![0.0, 1.0, 0.0]);

        let similar = bridge.find_similar(&Symbol::new("process"), 2);
        assert!(!similar.is_empty());
        assert_eq!(similar[0].0.name, "thread"); // Most similar
    }

    #[test]
    fn test_kernel_reasoner() {
        let mut reasoner = KernelSymbolicReasoner::new();

        // Add facts
        reasoner.kb.add_fact("holds_lock", alloc::vec![
            Term::constant("proc1"),
            Term::constant("lock_a"),
        ]);
        reasoner.kb.add_fact("waits_lock", alloc::vec![
            Term::constant("proc1"),
            Term::constant("lock_b"),
        ]);
        reasoner.kb.add_fact("holds_lock", alloc::vec![
            Term::constant("proc2"),
            Term::constant("lock_b"),
        ]);
        reasoner.kb.add_fact("waits_lock", alloc::vec![
            Term::constant("proc2"),
            Term::constant("lock_a"),
        ]);

        // Rebuild engine
        reasoner.engine = InferenceEngine::new(reasoner.kb.clone());

        // Check deadlock
        assert!(reasoner.check_deadlock("proc1", "proc2"));
    }
}
