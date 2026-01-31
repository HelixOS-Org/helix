//! # Inference Engine
//!
//! Performs logical inference and deduction.
//! Supports forward and backward chaining.
//!
//! Part of Year 2 COGNITION - Q2: Causal Reasoning

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// INFERENCE TYPES
// ============================================================================

/// Fact
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fact {
    /// Fact ID
    pub id: u64,
    /// Predicate
    pub predicate: String,
    /// Arguments
    pub arguments: Vec<Term>,
    /// Truth value
    pub truth: TruthValue,
    /// Confidence
    pub confidence: u64, // Fixed point, 0-1000 = 0.0-1.0
    /// Source
    pub source: FactSource,
}

/// Term
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Term {
    /// Constant
    Constant(String),
    /// Variable
    Variable(String),
    /// Number
    Number(i64),
}

/// Truth value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TruthValue {
    True,
    False,
    Unknown,
}

/// Fact source
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactSource {
    Given,
    Inferred { rule_id: u64, from: Vec<u64> },
    Observed,
    Assumed,
}

/// Rule
#[derive(Debug, Clone)]
pub struct Rule {
    /// Rule ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Antecedents (conditions)
    pub antecedents: Vec<Pattern>,
    /// Consequent (conclusion)
    pub consequent: Pattern,
    /// Priority
    pub priority: u32,
    /// Enabled
    pub enabled: bool,
}

/// Pattern
#[derive(Debug, Clone)]
pub struct Pattern {
    /// Predicate
    pub predicate: String,
    /// Arguments
    pub arguments: Vec<Term>,
    /// Negated
    pub negated: bool,
}

/// Inference result
#[derive(Debug, Clone)]
pub struct InferenceResult {
    /// New facts
    pub new_facts: Vec<Fact>,
    /// Rules fired
    pub rules_fired: Vec<u64>,
    /// Iterations
    pub iterations: usize,
    /// Conflicts
    pub conflicts: Vec<Conflict>,
}

/// Conflict
#[derive(Debug, Clone)]
pub struct Conflict {
    /// Fact 1
    pub fact1: u64,
    /// Fact 2
    pub fact2: u64,
    /// Description
    pub description: String,
}

// ============================================================================
// BINDING
// ============================================================================

/// Variable binding
#[derive(Debug, Clone, Default)]
pub struct Binding {
    /// Mappings
    pub mappings: BTreeMap<String, Term>,
}

impl Binding {
    /// Create new binding
    pub fn new() -> Self {
        Self::default()
    }

    /// Add binding
    pub fn bind(&mut self, var: &str, term: Term) -> bool {
        if let Some(existing) = self.mappings.get(var) {
            return *existing == term;
        }
        self.mappings.insert(var.into(), term);
        true
    }

    /// Get binding
    pub fn get(&self, var: &str) -> Option<&Term> {
        self.mappings.get(var)
    }

    /// Apply binding to term
    pub fn apply(&self, term: &Term) -> Term {
        match term {
            Term::Variable(v) => {
                self.mappings.get(v).cloned().unwrap_or(term.clone())
            }
            _ => term.clone(),
        }
    }

    /// Apply binding to pattern
    pub fn apply_pattern(&self, pattern: &Pattern) -> Pattern {
        Pattern {
            predicate: pattern.predicate.clone(),
            arguments: pattern.arguments.iter().map(|t| self.apply(t)).collect(),
            negated: pattern.negated,
        }
    }

    /// Merge bindings
    pub fn merge(&self, other: &Binding) -> Option<Binding> {
        let mut result = self.clone();
        for (var, term) in &other.mappings {
            if !result.bind(var, term.clone()) {
                return None;
            }
        }
        Some(result)
    }
}

// ============================================================================
// INFERENCE ENGINE
// ============================================================================

/// Inference engine
pub struct InferenceEngine {
    /// Facts
    facts: BTreeMap<u64, Fact>,
    /// Rules
    rules: BTreeMap<u64, Rule>,
    /// Fact index by predicate
    predicate_index: BTreeMap<String, BTreeSet<u64>>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: InferenceConfig,
    /// Statistics
    stats: InferenceStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct InferenceConfig {
    /// Maximum iterations
    pub max_iterations: usize,
    /// Enable forward chaining
    pub forward_chaining: bool,
    /// Enable backward chaining
    pub backward_chaining: bool,
    /// Minimum confidence
    pub min_confidence: u64,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            forward_chaining: true,
            backward_chaining: true,
            min_confidence: 500, // 0.5
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct InferenceStats {
    /// Facts added
    pub facts_added: u64,
    /// Rules fired
    pub rules_fired: u64,
    /// Inferences made
    pub inferences_made: u64,
}

impl InferenceEngine {
    /// Create new engine
    pub fn new(config: InferenceConfig) -> Self {
        Self {
            facts: BTreeMap::new(),
            rules: BTreeMap::new(),
            predicate_index: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: InferenceStats::default(),
        }
    }

    /// Assert fact
    pub fn assert_fact(&mut self, predicate: &str, arguments: Vec<Term>) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let fact = Fact {
            id,
            predicate: predicate.into(),
            arguments,
            truth: TruthValue::True,
            confidence: 1000,
            source: FactSource::Given,
        };

        self.predicate_index.entry(predicate.into())
            .or_insert_with(BTreeSet::new)
            .insert(id);

        self.facts.insert(id, fact);
        self.stats.facts_added += 1;

        id
    }

    /// Add rule
    pub fn add_rule(&mut self, name: &str, antecedents: Vec<Pattern>, consequent: Pattern) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let rule = Rule {
            id,
            name: name.into(),
            antecedents,
            consequent,
            priority: 1,
            enabled: true,
        };

        self.rules.insert(id, rule);
        id
    }

    /// Forward chain
    pub fn forward_chain(&mut self) -> InferenceResult {
        let mut result = InferenceResult {
            new_facts: Vec::new(),
            rules_fired: Vec::new(),
            iterations: 0,
            conflicts: Vec::new(),
        };

        if !self.config.forward_chaining {
            return result;
        }

        for _ in 0..self.config.max_iterations {
            result.iterations += 1;
            let mut new_this_round = Vec::new();

            // Get rules sorted by priority
            let mut rules: Vec<_> = self.rules.values()
                .filter(|r| r.enabled)
                .collect();
            rules.sort_by(|a, b| b.priority.cmp(&a.priority));

            for rule in rules {
                let bindings = self.match_antecedents(&rule.antecedents);

                for binding in bindings {
                    let conclusion = binding.apply_pattern(&rule.consequent);

                    if !self.fact_exists(&conclusion) {
                        let fact_id = self.next_id.fetch_add(1, Ordering::Relaxed);

                        let fact = Fact {
                            id: fact_id,
                            predicate: conclusion.predicate.clone(),
                            arguments: conclusion.arguments.clone(),
                            truth: TruthValue::True,
                            confidence: 900,
                            source: FactSource::Inferred { rule_id: rule.id, from: vec![] },
                        };

                        new_this_round.push(fact.clone());

                        if !result.rules_fired.contains(&rule.id) {
                            result.rules_fired.push(rule.id);
                            self.stats.rules_fired += 1;
                        }
                    }
                }
            }

            if new_this_round.is_empty() {
                break;
            }

            // Add new facts
            for fact in new_this_round {
                self.predicate_index.entry(fact.predicate.clone())
                    .or_insert_with(BTreeSet::new)
                    .insert(fact.id);

                result.new_facts.push(fact.clone());
                self.facts.insert(fact.id, fact);
                self.stats.inferences_made += 1;
            }
        }

        result
    }

    fn match_antecedents(&self, antecedents: &[Pattern]) -> Vec<Binding> {
        if antecedents.is_empty() {
            return vec![Binding::new()];
        }

        let mut current_bindings = vec![Binding::new()];

        for pattern in antecedents {
            let mut new_bindings = Vec::new();

            for binding in &current_bindings {
                let matches = self.match_pattern(pattern, binding);
                for new_binding in matches {
                    if let Some(merged) = binding.merge(&new_binding) {
                        new_bindings.push(merged);
                    }
                }
            }

            current_bindings = new_bindings;

            if current_bindings.is_empty() {
                break;
            }
        }

        current_bindings
    }

    fn match_pattern(&self, pattern: &Pattern, binding: &Binding) -> Vec<Binding> {
        let fact_ids = self.predicate_index.get(&pattern.predicate)
            .cloned()
            .unwrap_or_default();

        let mut results = Vec::new();

        for fact_id in fact_ids {
            if let Some(fact) = self.facts.get(&fact_id) {
                if fact.truth == TruthValue::True && fact.arguments.len() == pattern.arguments.len() {
                    if let Some(new_binding) = self.unify(&pattern.arguments, &fact.arguments, binding) {
                        if pattern.negated {
                            // Negated pattern matched - fail
                        } else {
                            results.push(new_binding);
                        }
                    }
                }
            }
        }

        // Handle negated patterns
        if pattern.negated && results.is_empty() {
            results.push(binding.clone());
        }

        results
    }

    fn unify(&self, pattern_args: &[Term], fact_args: &[Term], binding: &Binding) -> Option<Binding> {
        let mut result = binding.clone();

        for (p, f) in pattern_args.iter().zip(fact_args.iter()) {
            match p {
                Term::Variable(v) => {
                    if !result.bind(v, f.clone()) {
                        return None;
                    }
                }
                Term::Constant(c) => {
                    if let Term::Constant(fc) = f {
                        if c != fc {
                            return None;
                        }
                    } else {
                        return None;
                    }
                }
                Term::Number(n) => {
                    if let Term::Number(fn_) = f {
                        if n != fn_ {
                            return None;
                        }
                    } else {
                        return None;
                    }
                }
            }
        }

        Some(result)
    }

    fn fact_exists(&self, pattern: &Pattern) -> bool {
        if let Some(fact_ids) = self.predicate_index.get(&pattern.predicate) {
            for &fact_id in fact_ids {
                if let Some(fact) = self.facts.get(&fact_id) {
                    if fact.arguments == pattern.arguments {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Query facts
    pub fn query(&self, predicate: &str, args: &[Term]) -> Vec<&Fact> {
        let fact_ids = self.predicate_index.get(predicate)
            .cloned()
            .unwrap_or_default();

        fact_ids.iter()
            .filter_map(|&id| self.facts.get(&id))
            .filter(|fact| {
                if args.is_empty() {
                    return true;
                }
                self.pattern_matches(&fact.arguments, args)
            })
            .collect()
    }

    fn pattern_matches(&self, fact_args: &[Term], pattern_args: &[Term]) -> bool {
        if fact_args.len() != pattern_args.len() {
            return false;
        }

        fact_args.iter().zip(pattern_args.iter()).all(|(f, p)| {
            match p {
                Term::Variable(_) => true,
                _ => f == p,
            }
        })
    }

    /// Retract fact
    pub fn retract(&mut self, fact_id: u64) {
        if let Some(fact) = self.facts.remove(&fact_id) {
            if let Some(index) = self.predicate_index.get_mut(&fact.predicate) {
                index.remove(&fact_id);
            }
        }
    }

    /// Get fact
    pub fn get_fact(&self, id: u64) -> Option<&Fact> {
        self.facts.get(&id)
    }

    /// Get statistics
    pub fn stats(&self) -> &InferenceStats {
        &self.stats
    }
}

impl Default for InferenceEngine {
    fn default() -> Self {
        Self::new(InferenceConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assert_fact() {
        let mut engine = InferenceEngine::default();

        let id = engine.assert_fact("parent", vec![
            Term::Constant("alice".into()),
            Term::Constant("bob".into()),
        ]);

        assert!(engine.get_fact(id).is_some());
    }

    #[test]
    fn test_query() {
        let mut engine = InferenceEngine::default();

        engine.assert_fact("color", vec![
            Term::Constant("apple".into()),
            Term::Constant("red".into()),
        ]);

        engine.assert_fact("color", vec![
            Term::Constant("banana".into()),
            Term::Constant("yellow".into()),
        ]);

        let results = engine.query("color", &[]);
        assert_eq!(results.len(), 2);

        let results = engine.query("color", &[
            Term::Constant("apple".into()),
            Term::Variable("X".into()),
        ]);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_forward_chain() {
        let mut engine = InferenceEngine::default();

        // Facts
        engine.assert_fact("parent", vec![
            Term::Constant("alice".into()),
            Term::Constant("bob".into()),
        ]);

        engine.assert_fact("parent", vec![
            Term::Constant("bob".into()),
            Term::Constant("carol".into()),
        ]);

        // Rule: parent(X, Y) & parent(Y, Z) => grandparent(X, Z)
        engine.add_rule(
            "grandparent",
            vec![
                Pattern {
                    predicate: "parent".into(),
                    arguments: vec![Term::Variable("X".into()), Term::Variable("Y".into())],
                    negated: false,
                },
                Pattern {
                    predicate: "parent".into(),
                    arguments: vec![Term::Variable("Y".into()), Term::Variable("Z".into())],
                    negated: false,
                },
            ],
            Pattern {
                predicate: "grandparent".into(),
                arguments: vec![Term::Variable("X".into()), Term::Variable("Z".into())],
                negated: false,
            },
        );

        let result = engine.forward_chain();
        assert!(!result.new_facts.is_empty());

        let grandparents = engine.query("grandparent", &[]);
        assert_eq!(grandparents.len(), 1);
    }

    #[test]
    fn test_binding() {
        let mut binding = Binding::new();

        assert!(binding.bind("X", Term::Constant("value".into())));
        assert!(binding.bind("X", Term::Constant("value".into())));
        assert!(!binding.bind("X", Term::Constant("other".into())));

        let term = binding.apply(&Term::Variable("X".into()));
        assert_eq!(term, Term::Constant("value".into()));
    }
}
