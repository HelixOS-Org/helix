//! # Theorem Proving Engine
//!
//! Year 3 EVOLUTION - Automated theorem proving for code verification
//! Proves program properties using formal methods.

#![allow(dead_code)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// LOGIC TYPES
// ============================================================================

/// Term in first-order logic
#[derive(Debug, Clone)]
pub enum Term {
    /// Variable
    Var(String),
    /// Constant
    Const(String),
    /// Integer literal
    Int(i128),
    /// Boolean literal
    Bool(bool),
    /// Function application
    App(String, Vec<Term>),
    /// Let binding
    Let(String, Box<Term>, Box<Term>),
}

/// Formula in first-order logic
#[derive(Debug, Clone)]
pub enum Formula {
    /// True
    True,
    /// False
    False,
    /// Predicate application
    Pred(String, Vec<Term>),
    /// Equality
    Eq(Term, Term),
    /// Less than
    Lt(Term, Term),
    /// Less than or equal
    Le(Term, Term),
    /// Negation
    Not(Box<Formula>),
    /// Conjunction
    And(Box<Formula>, Box<Formula>),
    /// Disjunction
    Or(Box<Formula>, Box<Formula>),
    /// Implication
    Implies(Box<Formula>, Box<Formula>),
    /// Biconditional
    Iff(Box<Formula>, Box<Formula>),
    /// Universal quantifier
    ForAll(String, Sort, Box<Formula>),
    /// Existential quantifier
    Exists(String, Sort, Box<Formula>),
}

/// Sort (type) in logic
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sort {
    Bool,
    Int,
    Real,
    BitVec(u32),
    Array(Box<Sort>, Box<Sort>),
    Uninterpreted(String),
}

/// Proof
#[derive(Debug, Clone)]
pub struct Proof {
    /// Proof ID
    pub id: u64,
    /// Goal that was proved
    pub goal: Formula,
    /// Proof steps
    pub steps: Vec<ProofStep>,
    /// Valid
    pub valid: bool,
}

/// Proof step
#[derive(Debug, Clone)]
pub struct ProofStep {
    /// Step ID
    pub id: u64,
    /// Rule applied
    pub rule: ProofRule,
    /// Premises (previous step IDs)
    pub premises: Vec<u64>,
    /// Conclusion
    pub conclusion: Formula,
}

/// Proof rule
#[derive(Debug, Clone)]
pub enum ProofRule {
    /// Assumption
    Assume,
    /// Modus ponens
    ModusPonens,
    /// And introduction
    AndIntro,
    /// And elimination (left)
    AndElimL,
    /// And elimination (right)
    AndElimR,
    /// Or introduction (left)
    OrIntroL,
    /// Or introduction (right)
    OrIntroR,
    /// Or elimination
    OrElim,
    /// Implies introduction
    ImpliesIntro,
    /// Implies elimination
    ImpliesElim,
    /// Not introduction
    NotIntro,
    /// Not elimination
    NotElim,
    /// Double negation elimination
    DoubleNegElim,
    /// Forall introduction
    ForallIntro,
    /// Forall elimination
    ForallElim(Term),
    /// Exists introduction
    ExistsIntro(Term),
    /// Exists elimination
    ExistsElim,
    /// Equality reflexivity
    EqRefl,
    /// Equality symmetry
    EqSymm,
    /// Equality transitivity
    EqTrans,
    /// Substitution
    Subst(String, Term),
    /// Induction
    Induction(String),
    /// Arithmetic
    Arith,
    /// Simplification
    Simp,
    /// Axiom
    Axiom(String),
    /// Lemma
    Lemma(String),
}

/// Proof goal
#[derive(Debug, Clone)]
pub struct Goal {
    /// Goal ID
    pub id: u64,
    /// Hypotheses
    pub hyps: Vec<(String, Formula)>,
    /// Conclusion to prove
    pub conclusion: Formula,
    /// Solved
    pub solved: bool,
}

/// Proof tactic
#[derive(Debug, Clone)]
pub enum Tactic {
    /// Apply rule
    Apply(ProofRule),
    /// Introduce hypothesis
    Intro(String),
    /// Destruct hypothesis
    Destruct(String),
    /// Apply induction
    Induction(String),
    /// Split conjunction goal
    Split,
    /// Handle disjunction
    Left,
    Right,
    /// Assume the opposite
    ByContradiction,
    /// Case analysis
    Cases(String),
    /// Simplify
    Simp,
    /// Use arithmetic solver
    Arith,
    /// Auto solve
    Auto,
    /// Unfold definition
    Unfold(String),
    /// Rewrite with equality
    Rewrite(String),
    /// Apply lemma
    UseLemma(String),
}

// ============================================================================
// THEOREM PROVER
// ============================================================================

/// Theorem prover engine
pub struct TheoremProver {
    /// Axioms
    axioms: BTreeMap<String, Formula>,
    /// Lemmas (proved theorems)
    lemmas: BTreeMap<String, (Formula, Proof)>,
    /// Definitions
    definitions: BTreeMap<String, Term>,
    /// Current proof state
    goals: Vec<Goal>,
    /// Proof history
    history: Vec<ProofStep>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: ProverConfig,
    /// Statistics
    stats: ProverStats,
}

/// Prover configuration
#[derive(Debug, Clone)]
pub struct ProverConfig {
    /// Maximum proof depth
    pub max_depth: usize,
    /// Timeout (ms)
    pub timeout_ms: u64,
    /// Enable auto tactics
    pub auto_tactics: bool,
    /// Verbosity
    pub verbose: bool,
}

impl Default for ProverConfig {
    fn default() -> Self {
        Self {
            max_depth: 100,
            timeout_ms: 10000,
            auto_tactics: true,
            verbose: false,
        }
    }
}

/// Prover statistics
#[derive(Debug, Clone, Default)]
pub struct ProverStats {
    /// Total proofs attempted
    pub proofs_attempted: u64,
    /// Proofs succeeded
    pub proofs_succeeded: u64,
    /// Proofs failed
    pub proofs_failed: u64,
    /// Total steps
    pub total_steps: u64,
}

impl TheoremProver {
    /// Create new prover
    pub fn new(config: ProverConfig) -> Self {
        let mut prover = Self {
            axioms: BTreeMap::new(),
            lemmas: BTreeMap::new(),
            definitions: BTreeMap::new(),
            goals: Vec::new(),
            history: Vec::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: ProverStats::default(),
        };

        prover.load_standard_axioms();
        prover
    }

    fn load_standard_axioms(&mut self) {
        // Arithmetic axioms
        self.add_axiom(
            "zero_add",
            Formula::ForAll(
                "x".into(),
                Sort::Int,
                Box::new(Formula::Eq(
                    Term::App("add".into(), vec![Term::Int(0), Term::Var("x".into())]),
                    Term::Var("x".into()),
                )),
            ),
        );

        self.add_axiom(
            "add_comm",
            Formula::ForAll(
                "x".into(),
                Sort::Int,
                Box::new(Formula::ForAll(
                    "y".into(),
                    Sort::Int,
                    Box::new(Formula::Eq(
                        Term::App("add".into(), vec![
                            Term::Var("x".into()),
                            Term::Var("y".into()),
                        ]),
                        Term::App("add".into(), vec![
                            Term::Var("y".into()),
                            Term::Var("x".into()),
                        ]),
                    )),
                )),
            ),
        );

        self.add_axiom(
            "add_assoc",
            Formula::ForAll(
                "x".into(),
                Sort::Int,
                Box::new(Formula::ForAll(
                    "y".into(),
                    Sort::Int,
                    Box::new(Formula::ForAll(
                        "z".into(),
                        Sort::Int,
                        Box::new(Formula::Eq(
                            Term::App("add".into(), vec![
                                Term::App("add".into(), vec![
                                    Term::Var("x".into()),
                                    Term::Var("y".into()),
                                ]),
                                Term::Var("z".into()),
                            ]),
                            Term::App("add".into(), vec![
                                Term::Var("x".into()),
                                Term::App("add".into(), vec![
                                    Term::Var("y".into()),
                                    Term::Var("z".into()),
                                ]),
                            ]),
                        )),
                    )),
                )),
            ),
        );

        self.add_axiom(
            "mul_one",
            Formula::ForAll(
                "x".into(),
                Sort::Int,
                Box::new(Formula::Eq(
                    Term::App("mul".into(), vec![Term::Int(1), Term::Var("x".into())]),
                    Term::Var("x".into()),
                )),
            ),
        );

        self.add_axiom(
            "mul_zero",
            Formula::ForAll(
                "x".into(),
                Sort::Int,
                Box::new(Formula::Eq(
                    Term::App("mul".into(), vec![Term::Int(0), Term::Var("x".into())]),
                    Term::Int(0),
                )),
            ),
        );
    }

    /// Add axiom
    pub fn add_axiom(&mut self, name: &str, formula: Formula) {
        self.axioms.insert(name.into(), formula);
    }

    /// Add definition
    pub fn add_definition(&mut self, name: &str, term: Term) {
        self.definitions.insert(name.into(), term);
    }

    /// Start a proof
    pub fn start_proof(&mut self, name: &str, formula: Formula) -> u64 {
        self.stats.proofs_attempted += 1;

        let goal_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let goal = Goal {
            id: goal_id,
            hyps: Vec::new(),
            conclusion: formula,
            solved: false,
        };

        self.goals.clear();
        self.goals.push(goal);
        self.history.clear();

        goal_id
    }

    /// Apply tactic
    pub fn apply_tactic(&mut self, tactic: &Tactic) -> Result<(), String> {
        if self.goals.is_empty() {
            return Err("No goals to prove".into());
        }

        let goal = self.goals.last_mut().unwrap();

        match tactic {
            Tactic::Intro(name) => {
                self.intro(name)?;
            },
            Tactic::Split => {
                self.split()?;
            },
            Tactic::Left => {
                self.left()?;
            },
            Tactic::Right => {
                self.right()?;
            },
            Tactic::Apply(rule) => {
                self.apply_rule(rule)?;
            },
            Tactic::Destruct(hyp) => {
                self.destruct(hyp)?;
            },
            Tactic::Induction(var) => {
                self.induction(var)?;
            },
            Tactic::Simp => {
                self.simplify()?;
            },
            Tactic::Arith => {
                self.arith()?;
            },
            Tactic::Auto => {
                self.auto()?;
            },
            Tactic::Unfold(name) => {
                self.unfold(name)?;
            },
            Tactic::Rewrite(hyp) => {
                self.rewrite(hyp)?;
            },
            Tactic::UseLemma(name) => {
                self.use_lemma(name)?;
            },
            Tactic::ByContradiction => {
                self.by_contradiction()?;
            },
            Tactic::Cases(hyp) => {
                self.cases(hyp)?;
            },
        }

        self.stats.total_steps += 1;
        Ok(())
    }

    fn intro(&mut self, name: &str) -> Result<(), String> {
        let goal = self.goals.last_mut().ok_or("No goal")?;

        match &goal.conclusion {
            Formula::Implies(hyp, concl) => {
                goal.hyps.push((name.into(), (**hyp).clone()));
                goal.conclusion = (**concl).clone();
                Ok(())
            },
            Formula::ForAll(var, _sort, body) => {
                // Introduce universally quantified variable
                let new_var = name.to_string();
                let new_body = self.substitute_formula(body, var, &Term::Var(new_var.clone()));
                goal.conclusion = new_body;
                Ok(())
            },
            _ => Err("Cannot intro on this goal".into()),
        }
    }

    fn split(&mut self) -> Result<(), String> {
        let goal = self.goals.pop().ok_or("No goal")?;

        match &goal.conclusion {
            Formula::And(left, right) => {
                // Create two subgoals
                let goal1 = Goal {
                    id: self.next_id.fetch_add(1, Ordering::Relaxed),
                    hyps: goal.hyps.clone(),
                    conclusion: (**left).clone(),
                    solved: false,
                };
                let goal2 = Goal {
                    id: self.next_id.fetch_add(1, Ordering::Relaxed),
                    hyps: goal.hyps,
                    conclusion: (**right).clone(),
                    solved: false,
                };
                self.goals.push(goal2);
                self.goals.push(goal1);
                Ok(())
            },
            _ => {
                self.goals.push(goal);
                Err("Goal is not a conjunction".into())
            },
        }
    }

    fn left(&mut self) -> Result<(), String> {
        let goal = self.goals.last_mut().ok_or("No goal")?;

        match &goal.conclusion {
            Formula::Or(left, _) => {
                goal.conclusion = (**left).clone();
                Ok(())
            },
            _ => Err("Goal is not a disjunction".into()),
        }
    }

    fn right(&mut self) -> Result<(), String> {
        let goal = self.goals.last_mut().ok_or("No goal")?;

        match &goal.conclusion {
            Formula::Or(_, right) => {
                goal.conclusion = (**right).clone();
                Ok(())
            },
            _ => Err("Goal is not a disjunction".into()),
        }
    }

    fn apply_rule(&mut self, rule: &ProofRule) -> Result<(), String> {
        let step_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let goal = self.goals.last().ok_or("No goal")?;

        let step = ProofStep {
            id: step_id,
            rule: rule.clone(),
            premises: Vec::new(),
            conclusion: goal.conclusion.clone(),
        };

        self.history.push(step);
        Ok(())
    }

    fn destruct(&mut self, hyp_name: &str) -> Result<(), String> {
        let goal = self.goals.last_mut().ok_or("No goal")?;

        let hyp_idx = goal
            .hyps
            .iter()
            .position(|(n, _)| n == hyp_name)
            .ok_or("Hypothesis not found")?;

        let (_, hyp) = goal.hyps.remove(hyp_idx);

        match hyp {
            Formula::And(left, right) => {
                goal.hyps.push((format!("{}_l", hyp_name), *left));
                goal.hyps.push((format!("{}_r", hyp_name), *right));
                Ok(())
            },
            Formula::Exists(var, _sort, body) => {
                goal.hyps.push((var.clone(), *body));
                Ok(())
            },
            _ => {
                goal.hyps.push((hyp_name.into(), hyp));
                Err("Cannot destruct this hypothesis".into())
            },
        }
    }

    fn induction(&mut self, var: &str) -> Result<(), String> {
        let goal = self.goals.pop().ok_or("No goal")?;

        // Create base case and inductive case
        let base_case = Goal {
            id: self.next_id.fetch_add(1, Ordering::Relaxed),
            hyps: goal.hyps.clone(),
            conclusion: self.substitute_formula(&goal.conclusion, var, &Term::Int(0)),
            solved: false,
        };

        let ind_hyp = goal.conclusion.clone();
        let mut ind_hyps = goal.hyps.clone();
        ind_hyps.push(("IH".into(), ind_hyp));

        let ind_case = Goal {
            id: self.next_id.fetch_add(1, Ordering::Relaxed),
            hyps: ind_hyps,
            conclusion: self.substitute_formula(
                &goal.conclusion,
                var,
                &Term::App("succ".into(), vec![Term::Var(var.into())]),
            ),
            solved: false,
        };

        self.goals.push(ind_case);
        self.goals.push(base_case);

        Ok(())
    }

    fn simplify(&mut self) -> Result<(), String> {
        let goal = self.goals.last_mut().ok_or("No goal")?;
        goal.conclusion = self.simplify_formula(&goal.conclusion);

        // Check if simplified to True
        if matches!(goal.conclusion, Formula::True) {
            goal.solved = true;
            self.goals.pop();
        }

        Ok(())
    }

    fn simplify_formula(&self, formula: &Formula) -> Formula {
        match formula {
            Formula::And(left, right) => {
                let l = self.simplify_formula(left);
                let r = self.simplify_formula(right);
                match (&l, &r) {
                    (Formula::True, _) => r,
                    (_, Formula::True) => l,
                    (Formula::False, _) | (_, Formula::False) => Formula::False,
                    _ => Formula::And(Box::new(l), Box::new(r)),
                }
            },
            Formula::Or(left, right) => {
                let l = self.simplify_formula(left);
                let r = self.simplify_formula(right);
                match (&l, &r) {
                    (Formula::True, _) | (_, Formula::True) => Formula::True,
                    (Formula::False, _) => r,
                    (_, Formula::False) => l,
                    _ => Formula::Or(Box::new(l), Box::new(r)),
                }
            },
            Formula::Not(inner) => match self.simplify_formula(inner) {
                Formula::True => Formula::False,
                Formula::False => Formula::True,
                Formula::Not(x) => *x,
                other => Formula::Not(Box::new(other)),
            },
            Formula::Implies(left, right) => {
                let l = self.simplify_formula(left);
                let r = self.simplify_formula(right);
                match (&l, &r) {
                    (Formula::False, _) | (_, Formula::True) => Formula::True,
                    (Formula::True, _) => r,
                    _ => Formula::Implies(Box::new(l), Box::new(r)),
                }
            },
            Formula::Eq(t1, t2) if self.terms_equal(t1, t2) => Formula::True,
            _ => formula.clone(),
        }
    }

    fn terms_equal(&self, t1: &Term, t2: &Term) -> bool {
        match (t1, t2) {
            (Term::Var(a), Term::Var(b)) => a == b,
            (Term::Const(a), Term::Const(b)) => a == b,
            (Term::Int(a), Term::Int(b)) => a == b,
            (Term::Bool(a), Term::Bool(b)) => a == b,
            (Term::App(f1, args1), Term::App(f2, args2)) => {
                f1 == f2
                    && args1.len() == args2.len()
                    && args1
                        .iter()
                        .zip(args2.iter())
                        .all(|(a, b)| self.terms_equal(a, b))
            },
            _ => false,
        }
    }

    fn arith(&mut self) -> Result<(), String> {
        let goal = self.goals.last_mut().ok_or("No goal")?;

        // Try to solve arithmetic goal
        if self.is_arithmetic_tautology(&goal.conclusion) {
            goal.solved = true;
            self.goals.pop();
            Ok(())
        } else {
            Err("Could not solve by arithmetic".into())
        }
    }

    fn is_arithmetic_tautology(&self, formula: &Formula) -> bool {
        match formula {
            Formula::True => true,
            Formula::Eq(Term::Int(a), Term::Int(b)) => a == b,
            Formula::Lt(Term::Int(a), Term::Int(b)) => a < b,
            Formula::Le(Term::Int(a), Term::Int(b)) => a <= b,
            _ => false,
        }
    }

    fn auto(&mut self) -> Result<(), String> {
        // Try various tactics automatically
        let tactics = vec![Tactic::Simp, Tactic::Arith, Tactic::Split];

        for tactic in tactics {
            if self.apply_tactic(&tactic).is_ok() {
                if self.goals.is_empty() || self.goals.last().map(|g| g.solved).unwrap_or(false) {
                    return Ok(());
                }
            }
        }

        Err("Auto could not solve goal".into())
    }

    fn unfold(&mut self, name: &str) -> Result<(), String> {
        let definition = self
            .definitions
            .get(name)
            .cloned()
            .ok_or("Definition not found")?;

        let goal = self.goals.last_mut().ok_or("No goal")?;
        goal.conclusion = self.unfold_in_formula(&goal.conclusion, name, &definition);

        Ok(())
    }

    fn unfold_in_formula(&self, formula: &Formula, name: &str, definition: &Term) -> Formula {
        // Simplified unfolding
        formula.clone()
    }

    fn rewrite(&mut self, hyp_name: &str) -> Result<(), String> {
        let goal = self.goals.last_mut().ok_or("No goal")?;

        let hyp = goal
            .hyps
            .iter()
            .find(|(n, _)| n == hyp_name)
            .map(|(_, f)| f.clone())
            .ok_or("Hypothesis not found")?;

        if let Formula::Eq(lhs, rhs) = hyp {
            goal.conclusion = self.rewrite_formula(&goal.conclusion, &lhs, &rhs);
            Ok(())
        } else {
            Err("Hypothesis is not an equality".into())
        }
    }

    fn rewrite_formula(&self, formula: &Formula, from: &Term, to: &Term) -> Formula {
        // Simplified rewriting
        formula.clone()
    }

    fn use_lemma(&mut self, name: &str) -> Result<(), String> {
        let (lemma_formula, _) = self.lemmas.get(name).cloned().ok_or("Lemma not found")?;

        let goal = self.goals.last_mut().ok_or("No goal")?;
        goal.hyps.push((name.into(), lemma_formula));

        Ok(())
    }

    fn by_contradiction(&mut self) -> Result<(), String> {
        let goal = self.goals.last_mut().ok_or("No goal")?;

        // Add negation of goal as hypothesis
        let neg_goal = Formula::Not(Box::new(goal.conclusion.clone()));
        goal.hyps.push(("contra".into(), neg_goal));

        // New goal is False
        goal.conclusion = Formula::False;

        Ok(())
    }

    fn cases(&mut self, hyp_name: &str) -> Result<(), String> {
        let goal = self.goals.pop().ok_or("No goal")?;

        let hyp_idx = goal
            .hyps
            .iter()
            .position(|(n, _)| n == hyp_name)
            .ok_or("Hypothesis not found")?;

        let (_, hyp) = &goal.hyps[hyp_idx];

        match hyp {
            Formula::Or(left, right) => {
                let mut hyps1 = goal.hyps.clone();
                hyps1.remove(hyp_idx);
                hyps1.push((format!("{}_l", hyp_name), (**left).clone()));

                let mut hyps2 = goal.hyps.clone();
                hyps2.remove(hyp_idx);
                hyps2.push((format!("{}_r", hyp_name), (**right).clone()));

                let goal1 = Goal {
                    id: self.next_id.fetch_add(1, Ordering::Relaxed),
                    hyps: hyps1,
                    conclusion: goal.conclusion.clone(),
                    solved: false,
                };
                let goal2 = Goal {
                    id: self.next_id.fetch_add(1, Ordering::Relaxed),
                    hyps: hyps2,
                    conclusion: goal.conclusion,
                    solved: false,
                };

                self.goals.push(goal2);
                self.goals.push(goal1);
                Ok(())
            },
            _ => {
                self.goals.push(goal);
                Err("Hypothesis is not a disjunction".into())
            },
        }
    }

    fn substitute_formula(&self, formula: &Formula, var: &str, term: &Term) -> Formula {
        match formula {
            Formula::True | Formula::False => formula.clone(),
            Formula::Pred(name, args) => Formula::Pred(
                name.clone(),
                args.iter()
                    .map(|a| self.substitute_term(a, var, term))
                    .collect(),
            ),
            Formula::Eq(t1, t2) => Formula::Eq(
                self.substitute_term(t1, var, term),
                self.substitute_term(t2, var, term),
            ),
            Formula::Lt(t1, t2) => Formula::Lt(
                self.substitute_term(t1, var, term),
                self.substitute_term(t2, var, term),
            ),
            Formula::Le(t1, t2) => Formula::Le(
                self.substitute_term(t1, var, term),
                self.substitute_term(t2, var, term),
            ),
            Formula::Not(inner) => {
                Formula::Not(Box::new(self.substitute_formula(inner, var, term)))
            },
            Formula::And(left, right) => Formula::And(
                Box::new(self.substitute_formula(left, var, term)),
                Box::new(self.substitute_formula(right, var, term)),
            ),
            Formula::Or(left, right) => Formula::Or(
                Box::new(self.substitute_formula(left, var, term)),
                Box::new(self.substitute_formula(right, var, term)),
            ),
            Formula::Implies(left, right) => Formula::Implies(
                Box::new(self.substitute_formula(left, var, term)),
                Box::new(self.substitute_formula(right, var, term)),
            ),
            Formula::Iff(left, right) => Formula::Iff(
                Box::new(self.substitute_formula(left, var, term)),
                Box::new(self.substitute_formula(right, var, term)),
            ),
            Formula::ForAll(v, sort, body) if v != var => Formula::ForAll(
                v.clone(),
                sort.clone(),
                Box::new(self.substitute_formula(body, var, term)),
            ),
            Formula::Exists(v, sort, body) if v != var => Formula::Exists(
                v.clone(),
                sort.clone(),
                Box::new(self.substitute_formula(body, var, term)),
            ),
            _ => formula.clone(),
        }
    }

    fn substitute_term(&self, t: &Term, var: &str, replacement: &Term) -> Term {
        match t {
            Term::Var(name) if name == var => replacement.clone(),
            Term::Var(_) | Term::Const(_) | Term::Int(_) | Term::Bool(_) => t.clone(),
            Term::App(name, args) => Term::App(
                name.clone(),
                args.iter()
                    .map(|a| self.substitute_term(a, var, replacement))
                    .collect(),
            ),
            Term::Let(name, val, body) if name != var => Term::Let(
                name.clone(),
                Box::new(self.substitute_term(val, var, replacement)),
                Box::new(self.substitute_term(body, var, replacement)),
            ),
            _ => t.clone(),
        }
    }

    /// Finish proof
    pub fn finish_proof(&mut self, name: &str) -> Result<Proof, String> {
        if !self.goals.is_empty() && !self.goals.iter().all(|g| g.solved) {
            return Err("Proof not complete".into());
        }

        self.stats.proofs_succeeded += 1;

        let proof = Proof {
            id: self.next_id.fetch_add(1, Ordering::Relaxed),
            goal: if !self.history.is_empty() {
                self.history[0].conclusion.clone()
            } else {
                Formula::True
            },
            steps: self.history.clone(),
            valid: true,
        };

        // Store as lemma
        self.lemmas
            .insert(name.into(), (proof.goal.clone(), proof.clone()));

        Ok(proof)
    }

    /// Check if proof is complete
    pub fn is_complete(&self) -> bool {
        self.goals.is_empty() || self.goals.iter().all(|g| g.solved)
    }

    /// Get current goals
    pub fn current_goals(&self) -> &[Goal] {
        &self.goals
    }

    /// Get statistics
    pub fn stats(&self) -> &ProverStats {
        &self.stats
    }
}

impl Default for TheoremProver {
    fn default() -> Self {
        Self::new(ProverConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prover_creation() {
        let prover = TheoremProver::default();
        assert!(!prover.axioms.is_empty());
    }

    #[test]
    fn test_simple_proof() {
        let mut prover = TheoremProver::default();

        // Prove: True
        prover.start_proof("trivial", Formula::True);
        assert!(prover.is_complete());
    }

    #[test]
    fn test_simplify() {
        let mut prover = TheoremProver::default();

        // Prove: x = x
        prover.start_proof(
            "eq_refl",
            Formula::Eq(Term::Var("x".into()), Term::Var("x".into())),
        );

        let _ = prover.apply_tactic(&Tactic::Simp);
        assert!(prover.is_complete());
    }

    #[test]
    fn test_intro() {
        let mut prover = TheoremProver::default();

        // Prove: P -> P
        let p = Formula::Pred("P".into(), vec![]);
        prover.start_proof(
            "impl_refl",
            Formula::Implies(Box::new(p.clone()), Box::new(p)),
        );

        let _ = prover.apply_tactic(&Tactic::Intro("H".into()));
        assert!(!prover.is_complete()); // Still need to prove P from H
    }
}
