//! # Causal Reasoning Engine
//!
//! Causal inference and explanation generation for cognitive systems.
//! Implements causal graphs, interventions, and counterfactual reasoning.
//!
//! Part of Year 2 COGNITION - Q2: Causal Reasoning Engine

#![allow(dead_code)]

extern crate alloc;
use crate::fast::linear_map::LinearMap;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// CAUSAL TYPES
// ============================================================================

/// Causal variable
#[derive(Debug, Clone)]
pub struct CausalVariable {
    /// Variable ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Variable type
    pub var_type: VariableType,
    /// Current value
    pub value: CausalValue,
    /// Is observed
    pub observed: bool,
    /// Is intervention target
    pub intervened: bool,
    /// Domain constraints
    pub domain: VariableDomain,
}

/// Variable type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableType {
    /// Binary variable
    Binary,
    /// Discrete with finite values
    Discrete,
    /// Continuous
    Continuous,
    /// Categorical
    Categorical,
}

/// Causal value
#[derive(Debug, Clone, PartialEq)]
pub enum CausalValue {
    Unknown,
    Bool(bool),
    Int(i64),
    Float(f64),
    Category(String),
    Distribution(Distribution),
}

/// Probability distribution
#[derive(Debug, Clone, PartialEq)]
pub enum Distribution {
    /// Point mass
    Point(f64),
    /// Uniform
    Uniform { min: f64, max: f64 },
    /// Normal/Gaussian
    Normal { mean: f64, std: f64 },
    /// Discrete distribution
    Discrete(Vec<(CausalValue, f64)>),
}

/// Variable domain
#[derive(Debug, Clone)]
pub struct VariableDomain {
    /// Minimum value
    pub min: Option<f64>,
    /// Maximum value
    pub max: Option<f64>,
    /// Allowed values (for categorical)
    pub allowed: Option<Vec<String>>,
}

impl Default for VariableDomain {
    fn default() -> Self {
        Self {
            min: None,
            max: None,
            allowed: None,
        }
    }
}

/// Causal edge
#[derive(Debug, Clone)]
pub struct CausalEdge {
    /// Edge ID
    pub id: u64,
    /// Cause variable
    pub cause: u64,
    /// Effect variable
    pub effect: u64,
    /// Edge type
    pub edge_type: EdgeType,
    /// Causal strength (-1 to 1)
    pub strength: f64,
    /// Confidence (0 to 1)
    pub confidence: f64,
    /// Mechanism description
    pub mechanism: Option<String>,
}

/// Edge type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeType {
    /// Direct cause
    Direct,
    /// Mediated cause
    Mediated,
    /// Confounded (spurious)
    Confounded,
    /// Selection bias
    Selection,
    /// Unknown direction
    Undirected,
}

// ============================================================================
// CAUSAL GRAPH
// ============================================================================

/// Causal graph (DAG)
pub struct CausalGraph {
    /// Variables
    variables: BTreeMap<u64, CausalVariable>,
    /// Variables by name
    by_name: BTreeMap<String, u64>,
    /// Edges
    edges: BTreeMap<u64, CausalEdge>,
    /// Outgoing edges (cause -> effects)
    outgoing: BTreeMap<u64, Vec<u64>>,
    /// Incoming edges (effect <- causes)
    incoming: BTreeMap<u64, Vec<u64>>,
    /// Next ID
    next_id: AtomicU64,
}

impl CausalGraph {
    /// Create new causal graph
    pub fn new() -> Self {
        Self {
            variables: BTreeMap::new(),
            by_name: BTreeMap::new(),
            edges: BTreeMap::new(),
            outgoing: BTreeMap::new(),
            incoming: BTreeMap::new(),
            next_id: AtomicU64::new(1),
        }
    }

    /// Add variable
    pub fn add_variable(
        &mut self,
        name: &str,
        var_type: VariableType,
        domain: VariableDomain,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let variable = CausalVariable {
            id,
            name: name.into(),
            var_type,
            value: CausalValue::Unknown,
            observed: false,
            intervened: false,
            domain,
        };

        self.variables.insert(id, variable);
        self.by_name.insert(name.into(), id);

        id
    }

    /// Add causal edge
    pub fn add_edge(
        &mut self,
        cause: u64,
        effect: u64,
        edge_type: EdgeType,
        strength: f64,
    ) -> Result<u64, &'static str> {
        // Check variables exist
        if !self.variables.contains_key(&cause) {
            return Err("Cause variable not found");
        }
        if !self.variables.contains_key(&effect) {
            return Err("Effect variable not found");
        }

        // Check for cycle (simple check)
        if self.would_create_cycle(cause, effect) {
            return Err("Would create cycle");
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let edge = CausalEdge {
            id,
            cause,
            effect,
            edge_type,
            strength,
            confidence: 1.0,
            mechanism: None,
        };

        self.edges.insert(id, edge);
        self.outgoing.entry(cause).or_insert_with(Vec::new).push(id);
        self.incoming
            .entry(effect)
            .or_insert_with(Vec::new)
            .push(id);

        Ok(id)
    }

    fn would_create_cycle(&self, from: u64, to: u64) -> bool {
        // Check if 'to' can reach 'from' via existing edges
        let mut visited = Vec::new();
        let mut stack = vec![to];

        while let Some(current) = stack.pop() {
            if current == from {
                return true;
            }
            if visited.contains(&current) {
                continue;
            }
            visited.push(current);

            // Get outgoing edges from current
            if let Some(edges) = self.outgoing.get(&current) {
                for edge_id in edges {
                    if let Some(edge) = self.edges.get(edge_id) {
                        stack.push(edge.effect);
                    }
                }
            }
        }

        false
    }

    /// Get variable
    #[inline(always)]
    pub fn get_variable(&self, id: u64) -> Option<&CausalVariable> {
        self.variables.get(&id)
    }

    /// Get variable by name
    #[inline(always)]
    pub fn get_variable_by_name(&self, name: &str) -> Option<&CausalVariable> {
        let id = self.by_name.get(name)?;
        self.variables.get(id)
    }

    /// Set variable value
    #[inline]
    pub fn set_value(&mut self, id: u64, value: CausalValue) -> Result<(), &'static str> {
        let var = self.variables.get_mut(&id).ok_or("Variable not found")?;
        var.value = value;
        var.observed = true;
        Ok(())
    }

    /// Get parents (direct causes)
    pub fn get_parents(&self, id: u64) -> Vec<&CausalVariable> {
        self.incoming
            .get(&id)
            .map(|edge_ids| {
                edge_ids
                    .iter()
                    .filter_map(|eid| self.edges.get(eid))
                    .filter_map(|e| self.variables.get(&e.cause))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get children (direct effects)
    pub fn get_children(&self, id: u64) -> Vec<&CausalVariable> {
        self.outgoing
            .get(&id)
            .map(|edge_ids| {
                edge_ids
                    .iter()
                    .filter_map(|eid| self.edges.get(eid))
                    .filter_map(|e| self.variables.get(&e.effect))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get ancestors (all causes)
    pub fn get_ancestors(&self, id: u64) -> Vec<&CausalVariable> {
        let mut ancestors = Vec::new();
        let mut stack = vec![id];
        let mut visited = Vec::new();

        while let Some(current) = stack.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.push(current);

            for parent in self.get_parents(current) {
                if !ancestors
                    .iter()
                    .any(|a: &&CausalVariable| a.id == parent.id)
                {
                    ancestors.push(parent);
                    stack.push(parent.id);
                }
            }
        }

        ancestors
    }

    /// Get descendants (all effects)
    pub fn get_descendants(&self, id: u64) -> Vec<&CausalVariable> {
        let mut descendants = Vec::new();
        let mut stack = vec![id];
        let mut visited = Vec::new();

        while let Some(current) = stack.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.push(current);

            for child in self.get_children(current) {
                if !descendants
                    .iter()
                    .any(|d: &&CausalVariable| d.id == child.id)
                {
                    descendants.push(child);
                    stack.push(child.id);
                }
            }
        }

        descendants
    }

    /// Check d-separation
    pub fn is_d_separated(&self, x: u64, y: u64, z: &[u64]) -> bool {
        // Simplified d-separation check
        // Full implementation would use Bayes Ball algorithm

        // If there's a direct edge, not separated
        if let Some(edges) = self.outgoing.get(&x) {
            for edge_id in edges {
                if let Some(edge) = self.edges.get(edge_id) {
                    if edge.effect == y {
                        return false;
                    }
                }
            }
        }

        // If z blocks all paths, separated
        !z.is_empty() && self.paths_blocked(x, y, z)
    }

    fn paths_blocked(&self, from: u64, to: u64, blocking: &[u64]) -> bool {
        // Simplified path blocking check
        let mut stack = vec![from];
        let mut visited = Vec::new();

        while let Some(current) = stack.pop() {
            if current == to {
                return false; // Found unblocked path
            }
            if visited.contains(&current) || blocking.contains(&current) {
                continue;
            }
            visited.push(current);

            // Check outgoing
            if let Some(edges) = self.outgoing.get(&current) {
                for edge_id in edges {
                    if let Some(edge) = self.edges.get(edge_id) {
                        stack.push(edge.effect);
                    }
                }
            }
        }

        true // All paths blocked
    }

    /// Get topological order
    pub fn topological_order(&self) -> Vec<u64> {
        let mut order = Vec::new();
        let mut in_degree: LinearMap<usize, 64> = BTreeMap::new();

        // Calculate in-degrees
        for id in self.variables.keys() {
            let degree = self.incoming.get(id).map(|e| e.len()).unwrap_or(0);
            in_degree.insert(*id, degree);
        }

        // Find nodes with no incoming edges
        let mut queue: Vec<u64> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(&id, _)| id)
            .collect();

        while let Some(node) = queue.pop() {
            order.push(node);

            if let Some(edges) = self.outgoing.get(&node) {
                for edge_id in edges {
                    if let Some(edge) = self.edges.get(edge_id) {
                        if let Some(degree) = in_degree.get_mut(&edge.effect) {
                            *degree = degree.saturating_sub(1);
                            if *degree == 0 {
                                queue.push(edge.effect);
                            }
                        }
                    }
                }
            }
        }

        order
    }

    /// Variable count
    #[inline(always)]
    pub fn variable_count(&self) -> usize {
        self.variables.len()
    }

    /// Edge count
    #[inline(always)]
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

impl Default for CausalGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// INTERVENTION
// ============================================================================

/// Intervention (do-operator)
#[derive(Debug, Clone)]
pub struct Intervention {
    /// Target variable
    pub target: u64,
    /// Intervention value
    pub value: CausalValue,
    /// Intervention type
    pub intervention_type: InterventionType,
}

/// Intervention type
#[derive(Debug, Clone, Copy)]
pub enum InterventionType {
    /// Hard intervention (cut all incoming edges)
    Hard,
    /// Soft intervention (modify distribution)
    Soft,
    /// Conditional intervention
    Conditional,
}

/// Interventional query
#[derive(Debug, Clone)]
pub struct InterventionalQuery {
    /// Query variable
    pub query: u64,
    /// Interventions
    pub interventions: Vec<Intervention>,
    /// Conditioning variables
    pub conditions: Vec<(u64, CausalValue)>,
}

// ============================================================================
// COUNTERFACTUAL
// ============================================================================

/// Counterfactual query
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CounterfactualQuery {
    /// What would Y be...
    pub outcome: u64,
    /// If X had been...
    pub antecedent: (u64, CausalValue),
    /// Given that we observed...
    pub evidence: Vec<(u64, CausalValue)>,
}

/// Counterfactual result
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CounterfactualResult {
    /// Query
    pub query: CounterfactualQuery,
    /// Counterfactual value
    pub value: CausalValue,
    /// Actual value
    pub actual: CausalValue,
    /// Difference
    pub difference: Option<f64>,
    /// Confidence
    pub confidence: f64,
}

// ============================================================================
// CAUSAL EXPLANATION
// ============================================================================

/// Causal explanation
#[derive(Debug, Clone)]
pub struct CausalExplanation {
    /// Effect variable
    pub effect: u64,
    /// Cause chain
    pub cause_chain: Vec<CauseContribution>,
    /// Total explained variance
    pub explained_variance: f64,
    /// Confidence
    pub confidence: f64,
}

/// Cause contribution
#[derive(Debug, Clone)]
pub struct CauseContribution {
    /// Cause variable
    pub cause: u64,
    /// Contribution (how much this cause contributes)
    pub contribution: f64,
    /// Path from cause to effect
    pub path: Vec<u64>,
    /// Is necessary condition
    pub is_necessary: bool,
    /// Is sufficient condition
    pub is_sufficient: bool,
}

// ============================================================================
// CAUSAL REASONER
// ============================================================================

/// Causal reasoner
pub struct CausalReasoner {
    /// Causal graph
    graph: CausalGraph,
    /// Configuration
    config: ReasonerConfig,
    /// Statistics
    stats: ReasonerStats,
}

/// Reasoner configuration
#[derive(Debug, Clone)]
pub struct ReasonerConfig {
    /// Maximum path length for explanation
    pub max_path_length: usize,
    /// Minimum contribution threshold
    pub min_contribution: f64,
    /// Use Monte Carlo for estimation
    pub use_monte_carlo: bool,
    /// Monte Carlo samples
    pub mc_samples: usize,
}

impl Default for ReasonerConfig {
    fn default() -> Self {
        Self {
            max_path_length: 10,
            min_contribution: 0.01,
            use_monte_carlo: true,
            mc_samples: 1000,
        }
    }
}

/// Reasoner statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct ReasonerStats {
    /// Queries processed
    pub queries_processed: u64,
    /// Explanations generated
    pub explanations_generated: u64,
    /// Average query time (ns)
    pub avg_query_time_ns: f64,
}

impl CausalReasoner {
    /// Create new reasoner
    pub fn new(config: ReasonerConfig) -> Self {
        Self {
            graph: CausalGraph::new(),
            config,
            stats: ReasonerStats::default(),
        }
    }

    /// Get graph
    #[inline(always)]
    pub fn graph(&self) -> &CausalGraph {
        &self.graph
    }

    /// Get mutable graph
    #[inline(always)]
    pub fn graph_mut(&mut self) -> &mut CausalGraph {
        &mut self.graph
    }

    /// Explain effect
    pub fn explain(&mut self, effect: u64) -> CausalExplanation {
        self.stats.explanations_generated += 1;

        let causes = self.graph.get_ancestors(effect);
        let mut contributions = Vec::new();

        for cause in causes {
            let path = self.find_path(cause.id, effect);
            let contribution = self.estimate_contribution(cause.id, effect);

            if contribution >= self.config.min_contribution {
                contributions.push(CauseContribution {
                    cause: cause.id,
                    contribution,
                    path,
                    is_necessary: self.is_necessary(cause.id, effect),
                    is_sufficient: self.is_sufficient(cause.id, effect),
                });
            }
        }

        // Sort by contribution
        contributions.sort_by(|a, b| {
            b.contribution
                .partial_cmp(&a.contribution)
                .unwrap_or(core::cmp::Ordering::Equal)
        });

        let total_contribution = contributions.iter().map(|c| c.contribution).sum();

        CausalExplanation {
            effect,
            cause_chain: contributions,
            explained_variance: total_contribution,
            confidence: 0.8, // Would be computed based on data
        }
    }

    fn find_path(&self, from: u64, to: u64) -> Vec<u64> {
        // BFS to find shortest path
        let mut queue = vec![(from, vec![from])];
        let mut visited = Vec::new();

        while let Some((current, path)) = queue.pop() {
            if current == to {
                return path;
            }
            if visited.contains(&current) {
                continue;
            }
            visited.push(current);

            for child in self.graph.get_children(current) {
                let mut new_path = path.clone();
                new_path.push(child.id);
                queue.push((child.id, new_path));
            }
        }

        vec![]
    }

    fn estimate_contribution(&self, cause: u64, effect: u64) -> f64 {
        // Simplified contribution estimation
        // Would use do-calculus or regression in real implementation

        let path = self.find_path(cause, effect);
        if path.is_empty() {
            return 0.0;
        }

        // Multiply edge strengths along path
        let mut contribution = 1.0;
        for i in 0..path.len() - 1 {
            if let Some(edges) = self.graph.outgoing.get(&path[i]) {
                for edge_id in edges {
                    if let Some(edge) = self.graph.edges.get(edge_id) {
                        if edge.effect == path[i + 1] {
                            contribution *= edge.strength.abs();
                            break;
                        }
                    }
                }
            }
        }

        contribution
    }

    fn is_necessary(&self, cause: u64, effect: u64) -> bool {
        // A cause is necessary if without it, the effect wouldn't occur
        // Simplified check: if it's on the only path
        let path = self.find_path(cause, effect);
        !path.is_empty() && self.graph.get_parents(effect).len() == 1
    }

    fn is_sufficient(&self, cause: u64, effect: u64) -> bool {
        // A cause is sufficient if it alone can produce the effect
        // Simplified check: if there are no other parents
        self.graph.get_parents(effect).len() == 1
            && self.graph.get_parents(effect).iter().any(|p| p.id == cause)
    }

    /// Evaluate counterfactual
    pub fn counterfactual(&mut self, query: CounterfactualQuery) -> CounterfactualResult {
        self.stats.queries_processed += 1;

        // Get actual value
        let actual = self
            .graph
            .get_variable(query.outcome)
            .map(|v| v.value.clone())
            .unwrap_or(CausalValue::Unknown);

        // Estimate counterfactual value
        let (cf_value, confidence) = self.estimate_counterfactual(&query);

        // Calculate difference
        let difference = match (&actual, &cf_value) {
            (CausalValue::Float(a), CausalValue::Float(b)) => Some(b - a),
            (CausalValue::Int(a), CausalValue::Int(b)) => Some((*b - *a) as f64),
            _ => None,
        };

        CounterfactualResult {
            query,
            value: cf_value,
            actual,
            difference,
            confidence,
        }
    }

    fn estimate_counterfactual(&self, query: &CounterfactualQuery) -> (CausalValue, f64) {
        // Simplified counterfactual estimation
        // Full implementation would use twin networks or abduction-action-prediction

        let (var_id, cf_value) = &query.antecedent;

        // If the antecedent directly affects the outcome
        if self
            .graph
            .get_children(*var_id)
            .iter()
            .any(|c| c.id == query.outcome)
        {
            // Propagate the counterfactual value
            if let Some(edges) = self.graph.outgoing.get(var_id) {
                for edge_id in edges {
                    if let Some(edge) = self.graph.edges.get(edge_id) {
                        if edge.effect == query.outcome {
                            // Apply causal effect
                            if let CausalValue::Float(x) = cf_value {
                                return (CausalValue::Float(x * edge.strength), 0.7);
                            }
                        }
                    }
                }
            }
        }

        (CausalValue::Unknown, 0.5)
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &ReasonerStats {
        &self.stats
    }
}

impl Default for CausalReasoner {
    fn default() -> Self {
        Self::new(ReasonerConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_causal_graph() {
        let mut graph = CausalGraph::new();

        let x = graph.add_variable("X", VariableType::Continuous, VariableDomain::default());
        let y = graph.add_variable("Y", VariableType::Continuous, VariableDomain::default());
        let z = graph.add_variable("Z", VariableType::Continuous, VariableDomain::default());

        // X -> Y -> Z
        graph.add_edge(x, y, EdgeType::Direct, 0.8).unwrap();
        graph.add_edge(y, z, EdgeType::Direct, 0.6).unwrap();

        let ancestors = graph.get_ancestors(z);
        assert_eq!(ancestors.len(), 2);

        let order = graph.topological_order();
        assert_eq!(order[0], x);
        assert_eq!(order[2], z);
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = CausalGraph::new();

        let a = graph.add_variable("A", VariableType::Binary, VariableDomain::default());
        let b = graph.add_variable("B", VariableType::Binary, VariableDomain::default());
        let c = graph.add_variable("C", VariableType::Binary, VariableDomain::default());

        graph.add_edge(a, b, EdgeType::Direct, 1.0).unwrap();
        graph.add_edge(b, c, EdgeType::Direct, 1.0).unwrap();

        // This should fail - would create cycle C -> A while A -> B -> C exists
        let result = graph.add_edge(c, a, EdgeType::Direct, 1.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_explanation() {
        let mut reasoner = CausalReasoner::default();
        let graph = reasoner.graph_mut();

        let x = graph.add_variable("X", VariableType::Continuous, VariableDomain::default());
        let m = graph.add_variable("M", VariableType::Continuous, VariableDomain::default());
        let y = graph.add_variable("Y", VariableType::Continuous, VariableDomain::default());

        graph.add_edge(x, m, EdgeType::Direct, 0.7).unwrap();
        graph.add_edge(m, y, EdgeType::Direct, 0.8).unwrap();

        let explanation = reasoner.explain(y);
        assert!(!explanation.cause_chain.is_empty());
    }
}
