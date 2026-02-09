//! # Probabilistic Reasoning
//!
//! Implements probabilistic inference and Bayesian reasoning.
//! Handles uncertainty in reasoning with probability theory.
//!
//! Part of Year 2 COGNITION - Q2: Causal Reasoning

#![allow(dead_code)]

extern crate alloc;
use crate::fast::linear_map::LinearMap;
use alloc::format;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// PROBABILISTIC TYPES
// ============================================================================

/// Random variable
#[derive(Debug, Clone)]
pub struct Variable {
    /// Variable ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Domain
    pub domain: Domain,
    /// Observed value
    pub observed: Option<DomainValue>,
}

/// Domain
#[derive(Debug, Clone)]
pub enum Domain {
    Binary,
    Discrete(Vec<String>),
    Continuous { min: f64, max: f64 },
}

/// Domain value
#[derive(Debug, Clone)]
pub enum DomainValue {
    Bool(bool),
    Category(String),
    Number(f64),
}

/// Probability distribution
#[derive(Debug, Clone)]
pub struct Distribution {
    /// Distribution ID
    pub id: u64,
    /// Variable ID
    pub variable: u64,
    /// Type
    pub dist_type: DistType,
    /// Parameters
    pub params: BTreeMap<String, f64>,
}

/// Distribution type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistType {
    Bernoulli,
    Categorical,
    Normal,
    Beta,
    Uniform,
    Conditional,
}

/// Bayesian network node
#[derive(Debug, Clone)]
pub struct BayesNode {
    /// Variable ID
    pub variable: u64,
    /// Parents
    pub parents: Vec<u64>,
    /// CPT (Conditional Probability Table)
    pub cpt: BTreeMap<String, f64>,
}

/// Inference result
#[derive(Debug, Clone)]
pub struct InferenceResult {
    /// Query variable
    pub variable: u64,
    /// Probability distribution
    pub distribution: BTreeMap<String, f64>,
    /// Most likely value
    pub map_value: String,
    /// Confidence
    pub confidence: f64,
}

/// Evidence
#[derive(Debug, Clone)]
pub struct ProbEvidence {
    /// Variable ID
    pub variable: u64,
    /// Observed value
    pub value: DomainValue,
}

// ============================================================================
// PROBABILISTIC ENGINE
// ============================================================================

/// Probabilistic reasoning engine
pub struct ProbabilisticEngine {
    /// Variables
    variables: BTreeMap<u64, Variable>,
    /// Distributions
    distributions: BTreeMap<u64, Distribution>,
    /// Bayesian network
    network: BTreeMap<u64, BayesNode>,
    /// Priors
    priors: LinearMap<f64, 64>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: ProbConfig,
    /// Statistics
    stats: ProbStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct ProbConfig {
    /// Number of samples for Monte Carlo
    pub num_samples: usize,
    /// Convergence threshold
    pub convergence: f64,
    /// Maximum iterations
    pub max_iterations: usize,
}

impl Default for ProbConfig {
    fn default() -> Self {
        Self {
            num_samples: 1000,
            convergence: 0.001,
            max_iterations: 100,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct ProbStats {
    /// Variables defined
    pub variables_defined: u64,
    /// Inferences performed
    pub inferences: u64,
    /// Updates performed
    pub updates: u64,
}

impl ProbabilisticEngine {
    /// Create new engine
    pub fn new(config: ProbConfig) -> Self {
        Self {
            variables: BTreeMap::new(),
            distributions: BTreeMap::new(),
            network: BTreeMap::new(),
            priors: LinearMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: ProbStats::default(),
        }
    }

    /// Define binary variable
    pub fn define_binary(&mut self, name: &str, prior: f64) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let var = Variable {
            id,
            name: name.into(),
            domain: Domain::Binary,
            observed: None,
        };

        self.variables.insert(id, var);
        self.priors.insert(id, prior.clamp(0.0, 1.0));
        self.stats.variables_defined += 1;

        id
    }

    /// Define discrete variable
    pub fn define_discrete(&mut self, name: &str, values: Vec<String>) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let var = Variable {
            id,
            name: name.into(),
            domain: Domain::Discrete(values),
            observed: None,
        };

        self.variables.insert(id, var);
        self.stats.variables_defined += 1;

        id
    }

    /// Define continuous variable
    pub fn define_continuous(&mut self, name: &str, min: f64, max: f64) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let var = Variable {
            id,
            name: name.into(),
            domain: Domain::Continuous { min, max },
            observed: None,
        };

        self.variables.insert(id, var);
        self.stats.variables_defined += 1;

        id
    }

    /// Add dependency
    #[inline]
    pub fn add_dependency(&mut self, child: u64, parents: Vec<u64>, cpt: BTreeMap<String, f64>) {
        let node = BayesNode {
            variable: child,
            parents,
            cpt,
        };

        self.network.insert(child, node);
    }

    /// Set evidence
    #[inline]
    pub fn set_evidence(&mut self, variable_id: u64, value: DomainValue) {
        if let Some(var) = self.variables.get_mut(&variable_id) {
            var.observed = Some(value);
        }
    }

    /// Clear evidence
    #[inline]
    pub fn clear_evidence(&mut self, variable_id: u64) {
        if let Some(var) = self.variables.get_mut(&variable_id) {
            var.observed = None;
        }
    }

    /// Query probability
    #[inline]
    pub fn query(&mut self, variable_id: u64) -> Option<InferenceResult> {
        let var = self.variables.get(&variable_id)?;

        self.stats.inferences += 1;

        match &var.domain {
            Domain::Binary => self.query_binary(variable_id),
            Domain::Discrete(values) => self.query_discrete(variable_id, values.clone()),
            Domain::Continuous { .. } => self.query_continuous(variable_id),
        }
    }

    fn query_binary(&self, variable_id: u64) -> Option<InferenceResult> {
        // Get prior or computed probability
        let prob_true = if let Some(node) = self.network.get(&variable_id) {
            self.compute_conditional_prob(node)
        } else {
            *self.priors.get(variable_id).unwrap_or(&0.5)
        };

        let mut distribution = BTreeMap::new();
        distribution.insert("true".into(), prob_true);
        distribution.insert("false".into(), 1.0 - prob_true);

        let map_value = if prob_true > 0.5 { "true" } else { "false" };
        let confidence = (prob_true - 0.5).abs() * 2.0;

        Some(InferenceResult {
            variable: variable_id,
            distribution,
            map_value: map_value.into(),
            confidence,
        })
    }

    fn query_discrete(&self, variable_id: u64, values: Vec<String>) -> Option<InferenceResult> {
        let mut distribution = BTreeMap::new();

        // Uniform if no CPT
        let uniform_prob = 1.0 / values.len() as f64;

        for value in &values {
            let prob = if let Some(node) = self.network.get(&variable_id) {
                node.cpt.get(value).copied().unwrap_or(uniform_prob)
            } else {
                uniform_prob
            };
            distribution.insert(value.clone(), prob);
        }

        // Normalize
        let total: f64 = distribution.values().sum();
        if total > 0.0 {
            for prob in distribution.values_mut() {
                *prob /= total;
            }
        }

        // Find MAP
        let map_value = distribution
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(core::cmp::Ordering::Equal))
            .map(|(k, _)| k.clone())
            .unwrap_or_default();

        let max_prob = *distribution.get(&map_value).unwrap_or(&0.0);
        let confidence = max_prob;

        Some(InferenceResult {
            variable: variable_id,
            distribution,
            map_value,
            confidence,
        })
    }

    fn query_continuous(&self, variable_id: u64) -> Option<InferenceResult> {
        // Simplified: return mean and variance
        let var = self.variables.get(&variable_id)?;

        if let Domain::Continuous { min, max } = &var.domain {
            let mean = (min + max) / 2.0;

            let mut distribution = BTreeMap::new();
            distribution.insert("mean".into(), mean);
            distribution.insert("min".into(), *min);
            distribution.insert("max".into(), *max);

            Some(InferenceResult {
                variable: variable_id,
                distribution,
                map_value: format!("{:.4}", mean),
                confidence: 0.5,
            })
        } else {
            None
        }
    }

    fn compute_conditional_prob(&self, node: &BayesNode) -> f64 {
        // Build parent configuration key
        let parent_config = self.build_parent_config(&node.parents);

        // Look up in CPT
        node.cpt.get(&parent_config).copied().unwrap_or(0.5)
    }

    fn build_parent_config(&self, parents: &[u64]) -> String {
        parents
            .iter()
            .filter_map(|&p| {
                self.variables
                    .get(&p)
                    .and_then(|v| v.observed.as_ref().map(|val| self.value_to_string(val)))
            })
            .collect::<Vec<_>>()
            .join(",")
    }

    fn value_to_string(&self, value: &DomainValue) -> String {
        match value {
            DomainValue::Bool(b) => if *b { "T" } else { "F" }.into(),
            DomainValue::Category(c) => c.clone(),
            DomainValue::Number(n) => format!("{:.2}", n),
        }
    }

    /// Bayesian update
    pub fn bayesian_update(&mut self, variable_id: u64, evidence: DomainValue, likelihood: f64) {
        self.stats.updates += 1;

        // Get current prior
        let prior = *self.priors.get(variable_id).unwrap_or(&0.5);

        // Calculate posterior using Bayes rule
        // P(H|E) = P(E|H) * P(H) / P(E)
        // P(E) ≈ P(E|H) * P(H) + P(E|¬H) * P(¬H)

        let p_e_given_h = likelihood;
        let p_e_given_not_h = 1.0 - likelihood; // Simplified
        let p_h = prior;

        let p_e = p_e_given_h * p_h + p_e_given_not_h * (1.0 - p_h);

        let posterior = if p_e > 0.0 {
            (p_e_given_h * p_h) / p_e
        } else {
            prior
        };

        self.priors
            .insert(variable_id, posterior.clamp(0.001, 0.999));
        self.set_evidence(variable_id, evidence);
    }

    /// Joint probability
    pub fn joint_probability(&self, assignments: &[(u64, DomainValue)]) -> f64 {
        let mut prob = 1.0;

        for (var_id, value) in assignments {
            if let Some(node) = self.network.get(var_id) {
                // Get conditional probability
                let key = self.build_assignment_key(&node.parents, assignments);
                prob *= node.cpt.get(&key).copied().unwrap_or(0.5);
            } else {
                // Use prior
                prob *= self.priors.get(var_id).copied().unwrap_or(0.5);
            }
        }

        prob
    }

    fn build_assignment_key(&self, parents: &[u64], assignments: &[(u64, DomainValue)]) -> String {
        parents
            .iter()
            .filter_map(|&p| {
                assignments
                    .iter()
                    .find(|(id, _)| *id == p)
                    .map(|(_, v)| self.value_to_string(v))
            })
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Get variable
    #[inline(always)]
    pub fn get_variable(&self, id: u64) -> Option<&Variable> {
        self.variables.get(&id)
    }

    /// Get prior
    #[inline(always)]
    pub fn get_prior(&self, id: u64) -> f64 {
        *self.priors.get(id).unwrap_or(&0.5)
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &ProbStats {
        &self.stats
    }
}

impl Default for ProbabilisticEngine {
    fn default() -> Self {
        Self::new(ProbConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_define_binary() {
        let mut engine = ProbabilisticEngine::default();

        let id = engine.define_binary("rain", 0.3);
        assert!(engine.get_variable(id).is_some());
        assert!((engine.get_prior(id) - 0.3).abs() < 0.001);
    }

    #[test]
    fn test_define_discrete() {
        let mut engine = ProbabilisticEngine::default();

        let id = engine.define_discrete("weather", vec![
            "sunny".into(),
            "rainy".into(),
            "cloudy".into(),
        ]);
        let var = engine.get_variable(id).unwrap();

        if let Domain::Discrete(values) = &var.domain {
            assert_eq!(values.len(), 3);
        }
    }

    #[test]
    fn test_query_binary() {
        let mut engine = ProbabilisticEngine::default();

        let id = engine.define_binary("test", 0.7);
        let result = engine.query(id).unwrap();

        assert!((result.distribution.get("true").unwrap() - 0.7).abs() < 0.001);
    }

    #[test]
    fn test_bayesian_update() {
        let mut engine = ProbabilisticEngine::default();

        let id = engine.define_binary("hypothesis", 0.5);

        engine.bayesian_update(id, DomainValue::Bool(true), 0.9);

        let posterior = engine.get_prior(id);
        // With strong evidence, posterior should increase
        assert!(posterior > 0.5);
    }

    #[test]
    fn test_dependency() {
        let mut engine = ProbabilisticEngine::default();

        let rain = engine.define_binary("rain", 0.3);
        let wet = engine.define_binary("wet_grass", 0.0);

        let mut cpt = BTreeMap::new();
        cpt.insert("T".into(), 0.9); // P(wet|rain) = 0.9
        cpt.insert("F".into(), 0.1); // P(wet|¬rain) = 0.1

        engine.add_dependency(wet, vec![rain], cpt);

        // Set evidence
        engine.set_evidence(rain, DomainValue::Bool(true));

        let result = engine.query(wet).unwrap();
        assert!(*result.distribution.get("true").unwrap() > 0.5);
    }

    #[test]
    fn test_joint_probability() {
        let mut engine = ProbabilisticEngine::default();

        let a = engine.define_binary("A", 0.5);
        let b = engine.define_binary("B", 0.5);

        let assignments = vec![(a, DomainValue::Bool(true)), (b, DomainValue::Bool(true))];

        let prob = engine.joint_probability(&assignments);
        // P(A=T, B=T) = 0.5 * 0.5 = 0.25
        assert!((prob - 0.25).abs() < 0.01);
    }
}
