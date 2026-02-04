//! # Causal Inference Engine for Helix OS Kernel
//!
//! Year 3 "EVOLUTION" - Revolutionary causal reasoning system that enables
//! the kernel to understand cause-and-effect relationships, not just correlations.
//!
//! ## Key Features
//!
//! - **Structural Causal Models (SCM)**: Formal representation of causal mechanisms
//! - **Do-Calculus**: Interventional reasoning (what happens if we do X?)
//! - **Counterfactual Reasoning**: What would have happened if...?
//! - **Causal Discovery**: Learning causal structure from data
//! - **Instrumental Variables**: Handling unobserved confounders
//! - **Causal Mediation**: Understanding causal pathways
//!
//! ## Kernel Applications
//!
//! - Root cause analysis for system failures
//! - Understanding performance bottlenecks
//! - Predicting effects of configuration changes
//! - Explaining why anomalies occurred

#![allow(dead_code)]
#![allow(clippy::excessive_nesting)]

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::math::F64Ext;

// ============================================================================
// CAUSAL GRAPH STRUCTURES
// ============================================================================

/// A node in the causal graph
#[derive(Debug, Clone)]
pub struct CausalNode {
    /// Node identifier
    pub id: u32,
    /// Node name
    pub name: String,
    /// Node type
    pub node_type: CausalNodeType,
    /// Observed values
    pub values: Vec<f64>,
    /// Is this an intervention target?
    pub is_intervention: bool,
    /// Node attributes
    pub attributes: BTreeMap<String, f64>,
}

/// Types of causal nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CausalNodeType {
    /// Observed variable
    Observed,
    /// Unobserved/latent variable
    Latent,
    /// Exogenous variable (external cause)
    Exogenous,
    /// Treatment/intervention variable
    Treatment,
    /// Outcome variable
    Outcome,
}

impl CausalNode {
    /// Create a new causal node
    pub fn new(id: u32, name: String, node_type: CausalNodeType) -> Self {
        Self {
            id,
            name,
            node_type,
            values: Vec::new(),
            is_intervention: false,
            attributes: BTreeMap::new(),
        }
    }

    /// Add an observation
    pub fn add_observation(&mut self, value: f64) {
        self.values.push(value);
    }

    /// Get mean value
    pub fn mean(&self) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }
        self.values.iter().sum::<f64>() / self.values.len() as f64
    }

    /// Get variance
    pub fn variance(&self) -> f64 {
        if self.values.len() < 2 {
            return 0.0;
        }

        let mean = self.mean();
        let sum_sq: f64 = self.values.iter().map(|v| (v - mean).powi(2)).sum();

        sum_sq / (self.values.len() - 1) as f64
    }
}

/// A directed edge in the causal graph
#[derive(Debug, Clone)]
pub struct CausalEdge {
    /// Source node ID
    pub from: u32,
    /// Target node ID
    pub to: u32,
    /// Edge type
    pub edge_type: CausalEdgeType,
    /// Causal effect strength (if known)
    pub effect: Option<f64>,
    /// Confidence in this edge
    pub confidence: f64,
}

/// Types of causal edges
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CausalEdgeType {
    /// Direct causal effect
    Direct,
    /// Bidirectional (confounded)
    Bidirectional,
    /// Induced by selection
    Selection,
}

impl CausalEdge {
    /// Create a new direct causal edge
    pub fn direct(from: u32, to: u32) -> Self {
        Self {
            from,
            to,
            edge_type: CausalEdgeType::Direct,
            effect: None,
            confidence: 1.0,
        }
    }

    /// Create a bidirectional edge (unobserved confounder)
    pub fn bidirectional(from: u32, to: u32) -> Self {
        Self {
            from,
            to,
            edge_type: CausalEdgeType::Bidirectional,
            effect: None,
            confidence: 1.0,
        }
    }

    /// Set causal effect
    pub fn with_effect(mut self, effect: f64) -> Self {
        self.effect = Some(effect);
        self
    }
}

/// A causal graph (DAG + bidirectional edges)
#[derive(Debug, Clone)]
pub struct CausalGraph {
    /// Nodes
    pub nodes: BTreeMap<u32, CausalNode>,
    /// Directed edges
    pub edges: Vec<CausalEdge>,
    /// Bidirectional edges (confounders)
    pub bidirectional: Vec<(u32, u32)>,
    /// Next node ID
    next_id: u32,
}

impl CausalGraph {
    /// Create an empty causal graph
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            edges: Vec::new(),
            bidirectional: Vec::new(),
            next_id: 0,
        }
    }

    /// Add a node
    pub fn add_node(&mut self, name: String, node_type: CausalNodeType) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.nodes.insert(id, CausalNode::new(id, name, node_type));
        id
    }

    /// Add a direct causal edge
    pub fn add_edge(&mut self, from: u32, to: u32) {
        self.edges.push(CausalEdge::direct(from, to));
    }

    /// Add a bidirectional edge (unobserved confounder)
    pub fn add_confounder(&mut self, node1: u32, node2: u32) {
        self.bidirectional.push((node1, node2));
        self.edges.push(CausalEdge::bidirectional(node1, node2));
    }

    /// Get parents of a node
    pub fn parents(&self, node: u32) -> Vec<u32> {
        self.edges
            .iter()
            .filter(|e| e.to == node && e.edge_type == CausalEdgeType::Direct)
            .map(|e| e.from)
            .collect()
    }

    /// Get children of a node
    pub fn children(&self, node: u32) -> Vec<u32> {
        self.edges
            .iter()
            .filter(|e| e.from == node && e.edge_type == CausalEdgeType::Direct)
            .map(|e| e.to)
            .collect()
    }

    /// Get ancestors of a node
    pub fn ancestors(&self, node: u32) -> BTreeSet<u32> {
        let mut ancestors = BTreeSet::new();
        let mut to_visit: Vec<u32> = self.parents(node);

        while let Some(n) = to_visit.pop() {
            if ancestors.insert(n) {
                to_visit.extend(self.parents(n));
            }
        }

        ancestors
    }

    /// Get descendants of a node
    pub fn descendants(&self, node: u32) -> BTreeSet<u32> {
        let mut descendants = BTreeSet::new();
        let mut to_visit: Vec<u32> = self.children(node);

        while let Some(n) = to_visit.pop() {
            if descendants.insert(n) {
                to_visit.extend(self.children(n));
            }
        }

        descendants
    }

    /// Check if there's a path from source to target
    pub fn has_path(&self, from: u32, to: u32) -> bool {
        self.descendants(from).contains(&to)
    }

    /// Get all nodes on paths between two nodes
    pub fn nodes_between(&self, from: u32, to: u32) -> BTreeSet<u32> {
        let from_descendants = self.descendants(from);
        let to_ancestors = self.ancestors(to);

        from_descendants
            .intersection(&to_ancestors)
            .cloned()
            .collect()
    }

    /// Topological sort
    pub fn topological_sort(&self) -> Vec<u32> {
        let mut in_degree: BTreeMap<u32, usize> = BTreeMap::new();

        for &id in self.nodes.keys() {
            in_degree.insert(id, 0);
        }

        for edge in &self.edges {
            if edge.edge_type == CausalEdgeType::Direct {
                *in_degree.entry(edge.to).or_insert(0) += 1;
            }
        }

        let mut queue: Vec<u32> = in_degree
            .iter()
            .filter(|&(_, &d)| d == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut sorted = Vec::new();

        while let Some(node) = queue.pop() {
            sorted.push(node);

            for child in self.children(node) {
                if let Some(d) = in_degree.get_mut(&child) {
                    *d = d.saturating_sub(1);
                    if *d == 0 {
                        queue.push(child);
                    }
                }
            }
        }

        sorted
    }

    /// Check if graph is acyclic
    pub fn is_acyclic(&self) -> bool {
        self.topological_sort().len() == self.nodes.len()
    }
}

impl Default for CausalGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// STRUCTURAL CAUSAL MODEL (SCM)
// ============================================================================

/// A structural equation in an SCM
#[derive(Debug, Clone)]
pub struct StructuralEquation {
    /// Target variable
    pub target: u32,
    /// Parent variables
    pub parents: Vec<u32>,
    /// Coefficients (linear model)
    pub coefficients: Vec<f64>,
    /// Intercept
    pub intercept: f64,
    /// Noise variance
    pub noise_variance: f64,
}

impl StructuralEquation {
    /// Create a new structural equation
    pub fn new(target: u32, parents: Vec<u32>, coefficients: Vec<f64>) -> Self {
        Self {
            target,
            parents,
            coefficients,
            intercept: 0.0,
            noise_variance: 1.0,
        }
    }

    /// Evaluate the equation
    pub fn evaluate(&self, parent_values: &BTreeMap<u32, f64>, noise: f64) -> f64 {
        let mut result = self.intercept;

        for (&parent, &coef) in self.parents.iter().zip(self.coefficients.iter()) {
            if let Some(&value) = parent_values.get(&parent) {
                result += coef * value;
            }
        }

        result + noise * libm::sqrt(self.noise_variance)
    }

    /// Compute partial derivative with respect to a parent
    pub fn partial_derivative(&self, parent: u32) -> f64 {
        self.parents
            .iter()
            .zip(self.coefficients.iter())
            .find(|&(&p, _)| p == parent)
            .map(|(_, &c)| c)
            .unwrap_or(0.0)
    }
}

/// Structural Causal Model
pub struct StructuralCausalModel {
    /// Underlying causal graph
    pub graph: CausalGraph,
    /// Structural equations for each node
    pub equations: BTreeMap<u32, StructuralEquation>,
    /// Current variable values
    pub values: BTreeMap<u32, f64>,
    /// Intervention targets
    pub interventions: BTreeMap<u32, f64>,
}

impl StructuralCausalModel {
    /// Create a new SCM
    pub fn new(graph: CausalGraph) -> Self {
        Self {
            graph,
            equations: BTreeMap::new(),
            values: BTreeMap::new(),
            interventions: BTreeMap::new(),
        }
    }

    /// Add a structural equation
    pub fn add_equation(&mut self, equation: StructuralEquation) {
        self.equations.insert(equation.target, equation);
    }

    /// Set an intervention do(X = x)
    pub fn do_intervention(&mut self, node: u32, value: f64) {
        self.interventions.insert(node, value);
    }

    /// Clear all interventions
    pub fn clear_interventions(&mut self) {
        self.interventions.clear();
    }

    /// Sample from the model (forward simulation)
    pub fn sample(&mut self, exogenous_noise: &BTreeMap<u32, f64>) -> BTreeMap<u32, f64> {
        let order = self.graph.topological_sort();
        self.values.clear();

        for node in order {
            if let Some(&intervention_value) = self.interventions.get(&node) {
                // Intervention: ignore parents
                self.values.insert(node, intervention_value);
            } else if let Some(equation) = self.equations.get(&node) {
                let noise = exogenous_noise.get(&node).copied().unwrap_or(0.0);
                let value = equation.evaluate(&self.values, noise);
                self.values.insert(node, value);
            } else {
                // Exogenous: use noise directly
                let value = exogenous_noise.get(&node).copied().unwrap_or(0.0);
                self.values.insert(node, value);
            }
        }

        self.values.clone()
    }

    /// Compute interventional distribution E[Y | do(X=x)]
    pub fn interventional_expectation(
        &mut self,
        target: u32,
        treatment: u32,
        treatment_value: f64,
        num_samples: usize,
        seed: u64,
    ) -> f64 {
        self.do_intervention(treatment, treatment_value);

        let mut rng = seed;
        let mut sum = 0.0;

        for _ in 0..num_samples {
            // Generate exogenous noise
            let mut noise = BTreeMap::new();
            for &node in self.graph.nodes.keys() {
                rng = lcg_next(rng);
                let z = box_muller(rng);
                noise.insert(node, z);
            }

            let values = self.sample(&noise);
            sum += values.get(&target).copied().unwrap_or(0.0);
        }

        self.clear_interventions();

        sum / num_samples as f64
    }

    /// Compute average treatment effect (ATE)
    pub fn average_treatment_effect(
        &mut self,
        target: u32,
        treatment: u32,
        treatment_high: f64,
        treatment_low: f64,
        num_samples: usize,
        seed: u64,
    ) -> f64 {
        let e_high =
            self.interventional_expectation(target, treatment, treatment_high, num_samples, seed);
        let e_low = self.interventional_expectation(
            target,
            treatment,
            treatment_low,
            num_samples,
            seed + 1,
        );

        e_high - e_low
    }
}

// ============================================================================
// COUNTERFACTUAL REASONING
// ============================================================================

/// A counterfactual query
#[derive(Debug, Clone)]
pub struct CounterfactualQuery {
    /// Observed evidence
    pub evidence: BTreeMap<u32, f64>,
    /// Hypothetical intervention
    pub intervention: BTreeMap<u32, f64>,
    /// Target variable(s)
    pub targets: Vec<u32>,
}

impl CounterfactualQuery {
    /// Create a new counterfactual query
    pub fn new() -> Self {
        Self {
            evidence: BTreeMap::new(),
            intervention: BTreeMap::new(),
            targets: Vec::new(),
        }
    }

    /// Add observed evidence
    pub fn given(mut self, node: u32, value: f64) -> Self {
        self.evidence.insert(node, value);
        self
    }

    /// Add hypothetical intervention
    pub fn had_been(mut self, node: u32, value: f64) -> Self {
        self.intervention.insert(node, value);
        self
    }

    /// Set target
    pub fn query(mut self, node: u32) -> Self {
        self.targets.push(node);
        self
    }
}

impl Default for CounterfactualQuery {
    fn default() -> Self {
        Self::new()
    }
}

/// Counterfactual reasoning engine
pub struct CounterfactualEngine {
    /// Underlying SCM
    pub scm: StructuralCausalModel,
    /// Abduced noise values
    abduced_noise: BTreeMap<u32, f64>,
}

impl CounterfactualEngine {
    /// Create a new counterfactual engine
    pub fn new(scm: StructuralCausalModel) -> Self {
        Self {
            scm,
            abduced_noise: BTreeMap::new(),
        }
    }

    /// Step 1: Abduction - infer noise from evidence
    pub fn abduce(&mut self, evidence: &BTreeMap<u32, f64>) {
        self.abduced_noise.clear();

        // Simple abduction for linear models: solve for noise
        // For more complex models, would use MCMC or variational inference

        let order = self.scm.graph.topological_sort();
        let mut current_values = evidence.clone();

        for node in order {
            if let Some(equation) = self.scm.equations.get(&node) {
                // Compute expected value without noise
                let mut expected = equation.intercept;
                for (&parent, &coef) in equation.parents.iter().zip(equation.coefficients.iter()) {
                    if let Some(&v) = current_values.get(&parent) {
                        expected += coef * v;
                    }
                }

                // If we have evidence for this node, compute noise
                if let Some(&observed) = evidence.get(&node) {
                    let noise =
                        (observed - expected) / libm::sqrt(equation.noise_variance.max(0.01));
                    self.abduced_noise.insert(node, noise);
                    current_values.insert(node, observed);
                } else {
                    // Use zero noise for unobserved nodes
                    self.abduced_noise.insert(node, 0.0);
                    current_values.insert(node, expected);
                }
            } else if let Some(&v) = evidence.get(&node) {
                // Exogenous node
                self.abduced_noise.insert(node, v);
                current_values.insert(node, v);
            }
        }
    }

    /// Step 2 & 3: Action and Prediction
    pub fn predict_counterfactual(&mut self, query: &CounterfactualQuery) -> BTreeMap<u32, f64> {
        // Step 1: Abduce noise from evidence
        self.abduce(&query.evidence);

        // Step 2: Apply interventions
        for (&node, &value) in &query.intervention {
            self.scm.do_intervention(node, value);
        }

        // Step 3: Predict under interventions with abduced noise
        let result = self.scm.sample(&self.abduced_noise);

        self.scm.clear_interventions();

        // Return only target values
        query
            .targets
            .iter()
            .filter_map(|&t| result.get(&t).map(|&v| (t, v)))
            .collect()
    }

    /// Compute probability of necessity: P(Y=0 | do(X=0), X=1, Y=1)
    /// "Would Y have been 0 if X had been 0, given that X was 1 and Y was 1?"
    pub fn probability_of_necessity(
        &mut self,
        treatment: u32,
        outcome: u32,
        treatment_observed: f64,
        outcome_observed: f64,
        treatment_counterfactual: f64,
        num_samples: usize,
        seed: u64,
    ) -> f64 {
        // Filter: P(X=x, Y=y)
        let mut count_filtered = 0;
        let mut count_necessary = 0;
        let mut rng = seed;

        for _ in 0..num_samples {
            // Generate noise
            let mut noise = BTreeMap::new();
            for &node in self.scm.graph.nodes.keys() {
                rng = lcg_next(rng);
                noise.insert(node, box_muller(rng));
            }

            // Observational sample
            self.scm.clear_interventions();
            let obs_values = self.scm.sample(&noise);

            let x = obs_values.get(&treatment).copied().unwrap_or(0.0);
            let y = obs_values.get(&outcome).copied().unwrap_or(0.0);

            // Check if matches filter (approximately)
            if (x - treatment_observed).abs() < 0.5 && (y - outcome_observed).abs() < 0.5 {
                count_filtered += 1;

                // Counterfactual: what if X had been different?
                self.scm
                    .do_intervention(treatment, treatment_counterfactual);
                let cf_values = self.scm.sample(&noise);
                let y_cf = cf_values.get(&outcome).copied().unwrap_or(0.0);

                // Check if outcome would have been different
                if (y_cf - outcome_observed).abs() >= 0.5 {
                    count_necessary += 1;
                }
            }
        }

        self.scm.clear_interventions();

        if count_filtered == 0 {
            0.0
        } else {
            count_necessary as f64 / count_filtered as f64
        }
    }

    /// Compute probability of sufficiency: P(Y=1 | do(X=1), X=0, Y=0)
    pub fn probability_of_sufficiency(
        &mut self,
        treatment: u32,
        outcome: u32,
        treatment_counterfactual: f64,
        num_samples: usize,
        seed: u64,
    ) -> f64 {
        self.probability_of_necessity(
            treatment,
            outcome,
            0.0,
            0.0,
            treatment_counterfactual,
            num_samples,
            seed,
        )
    }
}

// ============================================================================
// CAUSAL DISCOVERY
// ============================================================================

/// Causal discovery algorithm types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryAlgorithm {
    /// PC algorithm (constraint-based)
    PC,
    /// GES (score-based)
    GES,
    /// LiNGAM (functional model)
    LiNGAM,
    /// NOTEARS (continuous optimization)
    NOTEARS,
}

/// Result of conditional independence test
#[derive(Debug, Clone)]
pub struct IndependenceTest {
    pub x: u32,
    pub y: u32,
    pub conditioning_set: Vec<u32>,
    pub independent: bool,
    pub p_value: f64,
    pub statistic: f64,
}

/// Causal discovery from data
pub struct CausalDiscovery {
    /// Data (variable -> observations)
    pub data: BTreeMap<u32, Vec<f64>>,
    /// Variable names
    pub names: BTreeMap<u32, String>,
    /// Discovered edges
    pub discovered_edges: Vec<(u32, u32, f64)>,
    /// Independence tests performed
    pub tests: Vec<IndependenceTest>,
    /// Significance level
    pub alpha: f64,
}

impl CausalDiscovery {
    /// Create a new discovery instance
    pub fn new(alpha: f64) -> Self {
        Self {
            data: BTreeMap::new(),
            names: BTreeMap::new(),
            discovered_edges: Vec::new(),
            tests: Vec::new(),
            alpha,
        }
    }

    /// Add variable data
    pub fn add_variable(&mut self, id: u32, name: String, observations: Vec<f64>) {
        self.names.insert(id, name);
        self.data.insert(id, observations);
    }

    /// Compute correlation between two variables
    fn correlation(&self, x: u32, y: u32) -> f64 {
        let x_data = match self.data.get(&x) {
            Some(d) => d,
            None => return 0.0,
        };
        let y_data = match self.data.get(&y) {
            Some(d) => d,
            None => return 0.0,
        };

        if x_data.len() != y_data.len() || x_data.is_empty() {
            return 0.0;
        }

        let n = x_data.len() as f64;
        let mean_x: f64 = x_data.iter().sum::<f64>() / n;
        let mean_y: f64 = y_data.iter().sum::<f64>() / n;

        let mut cov = 0.0;
        let mut var_x = 0.0;
        let mut var_y = 0.0;

        for (xi, yi) in x_data.iter().zip(y_data.iter()) {
            let dx = xi - mean_x;
            let dy = yi - mean_y;
            cov += dx * dy;
            var_x += dx * dx;
            var_y += dy * dy;
        }

        if var_x < 1e-10 || var_y < 1e-10 {
            return 0.0;
        }

        cov / (libm::sqrt(var_x) * libm::sqrt(var_y))
    }

    /// Compute partial correlation (conditioned on one variable)
    fn partial_correlation(&self, x: u32, y: u32, z: u32) -> f64 {
        let r_xy = self.correlation(x, y);
        let r_xz = self.correlation(x, z);
        let r_yz = self.correlation(y, z);

        let denom = libm::sqrt((1.0 - r_xz * r_xz) * (1.0 - r_yz * r_yz));
        if denom < 1e-10 {
            return 0.0;
        }

        (r_xy - r_xz * r_yz) / denom
    }

    /// Test conditional independence using Fisher's z-test
    fn test_conditional_independence(&mut self, x: u32, y: u32, conditioning: &[u32]) -> bool {
        let n = self.data.get(&x).map(|d| d.len()).unwrap_or(0);
        if n < 5 {
            return true; // Too little data
        }

        // Compute partial correlation
        let r = if conditioning.is_empty() {
            self.correlation(x, y)
        } else if conditioning.len() == 1 {
            self.partial_correlation(x, y, conditioning[0])
        } else {
            // Higher-order partial correlation: simplified
            let mut r = self.correlation(x, y);
            for &z in conditioning {
                let r_xz = self.correlation(x, z);
                let r_yz = self.correlation(y, z);
                let denom = libm::sqrt((1.0 - r_xz * r_xz) * (1.0 - r_yz * r_yz));
                if denom > 1e-10 {
                    r = (r - r_xz * r_yz) / denom;
                }
            }
            r
        };

        // Fisher's z-transformation
        let z = 0.5 * libm::log((1.0 + r + 1e-10) / (1.0 - r + 1e-10));
        let se = 1.0 / libm::sqrt((n - conditioning.len() - 3).max(1) as f64);
        let statistic = libm::fabs(z / se);

        // Compare to normal distribution (approximation)
        // For alpha = 0.05, critical value ≈ 1.96
        let critical = 1.96 / self.alpha.sqrt();
        let independent = statistic < critical;

        // P-value approximation using normal CDF
        let p_value = 2.0 * (1.0 - normal_cdf(statistic));

        self.tests.push(IndependenceTest {
            x,
            y,
            conditioning_set: conditioning.to_vec(),
            independent,
            p_value,
            statistic,
        });

        independent
    }

    /// Run PC algorithm
    pub fn run_pc(&mut self) -> CausalGraph {
        let variables: Vec<u32> = self.data.keys().copied().collect();
        let n = variables.len();

        // Initialize complete undirected graph
        let mut adjacency: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
        for &v in &variables {
            let mut neighbors = BTreeSet::new();
            for &u in &variables {
                if u != v {
                    neighbors.insert(u);
                }
            }
            adjacency.insert(v, neighbors);
        }

        // Skeleton discovery
        let mut depth = 0;
        loop {
            let mut removed = false;

            for &x in &variables {
                let neighbors: Vec<u32> = adjacency
                    .get(&x)
                    .map(|s| s.iter().copied().collect())
                    .unwrap_or_default();

                for &y in &neighbors {
                    if x >= y {
                        continue; // Only check once
                    }

                    // Get potential conditioning sets
                    let adj_x: Vec<u32> = adjacency
                        .get(&x)
                        .map(|s| s.iter().copied().filter(|&n| n != y).collect())
                        .unwrap_or_default();

                    // Test all subsets of size depth
                    let subsets = combinations(&adj_x, depth);

                    for subset in subsets {
                        if self.test_conditional_independence(x, y, &subset) {
                            // Remove edge
                            if let Some(s) = adjacency.get_mut(&x) {
                                s.remove(&y);
                            }
                            if let Some(s) = adjacency.get_mut(&y) {
                                s.remove(&x);
                            }
                            removed = true;
                            break;
                        }
                    }
                }
            }

            if !removed || depth > n {
                break;
            }
            depth += 1;
        }

        // Create graph from skeleton
        let mut graph = CausalGraph::new();

        for (&_id, name) in &self.names {
            graph.add_node(name.clone(), CausalNodeType::Observed);
        }

        // Add edges (undirected as bidirectional for now)
        for (&x, neighbors) in &adjacency {
            for &y in neighbors {
                if x < y {
                    // Store as discovered edge
                    let corr = libm::fabs(self.correlation(x, y));
                    self.discovered_edges.push((x, y, corr));
                    graph.add_edge(x, y);
                }
            }
        }

        graph
    }

    /// Get discovery summary
    pub fn summary(&self) -> DiscoverySummary {
        DiscoverySummary {
            num_variables: self.data.len(),
            num_edges: self.discovered_edges.len(),
            tests_performed: self.tests.len(),
            avg_correlation: if self.discovered_edges.is_empty() {
                0.0
            } else {
                self.discovered_edges.iter().map(|(_, _, c)| c).sum::<f64>()
                    / self.discovered_edges.len() as f64
            },
        }
    }
}

/// Discovery summary
#[derive(Debug, Clone)]
pub struct DiscoverySummary {
    pub num_variables: usize,
    pub num_edges: usize,
    pub tests_performed: usize,
    pub avg_correlation: f64,
}

// ============================================================================
// CAUSAL EFFECT ESTIMATION
// ============================================================================

/// Methods for estimating causal effects
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstimationMethod {
    /// Backdoor adjustment
    Backdoor,
    /// Inverse probability weighting
    IPW,
    /// Doubly robust estimation
    DoublyRobust,
    /// Instrumental variables
    IV,
    /// Regression discontinuity
    RDD,
    /// Difference in differences
    DID,
}

/// Causal effect estimate
#[derive(Debug, Clone)]
pub struct CausalEffect {
    /// Point estimate
    pub estimate: f64,
    /// Standard error
    pub std_error: f64,
    /// Confidence interval (lower, upper)
    pub confidence_interval: (f64, f64),
    /// Method used
    pub method: EstimationMethod,
    /// Number of samples
    pub n_samples: usize,
}

impl CausalEffect {
    /// Create a new effect estimate
    pub fn new(estimate: f64, std_error: f64, method: EstimationMethod, n: usize) -> Self {
        // 95% CI
        let z = 1.96;
        let ci = (estimate - z * std_error, estimate + z * std_error);

        Self {
            estimate,
            std_error,
            confidence_interval: ci,
            method,
            n_samples: n,
        }
    }

    /// Check if effect is statistically significant
    pub fn is_significant(&self, alpha: f64) -> bool {
        // Two-tailed test
        let z = libm::fabs(self.estimate / self.std_error.max(1e-10));
        let critical = if alpha <= 0.01 {
            2.576
        } else if alpha <= 0.05 {
            1.96
        } else {
            1.645
        };
        z > critical
    }
}

/// Causal effect estimator
pub struct CausalEstimator {
    /// Treatment variable
    pub treatment: u32,
    /// Outcome variable
    pub outcome: u32,
    /// Adjustment set
    pub adjustment_set: Vec<u32>,
    /// Data
    pub data: BTreeMap<u32, Vec<f64>>,
}

impl CausalEstimator {
    /// Create a new estimator
    pub fn new(treatment: u32, outcome: u32) -> Self {
        Self {
            treatment,
            outcome,
            adjustment_set: Vec::new(),
            data: BTreeMap::new(),
        }
    }

    /// Set adjustment variables
    pub fn adjust_for(&mut self, variables: Vec<u32>) {
        self.adjustment_set = variables;
    }

    /// Add data
    pub fn add_data(&mut self, variable: u32, observations: Vec<f64>) {
        self.data.insert(variable, observations);
    }

    /// Estimate using backdoor adjustment
    pub fn estimate_backdoor(&self) -> Option<CausalEffect> {
        let t_data = self.data.get(&self.treatment)?;
        let y_data = self.data.get(&self.outcome)?;

        if t_data.len() != y_data.len() || t_data.is_empty() {
            return None;
        }

        let n = t_data.len();

        // Simple regression adjustment
        // Y ~ T + adjustments

        // For simplicity, compute adjusted correlation
        let mean_t: f64 = t_data.iter().sum::<f64>() / n as f64;
        let mean_y: f64 = y_data.iter().sum::<f64>() / n as f64;

        let mut cov_ty = 0.0;
        let mut var_t = 0.0;

        for i in 0..n {
            let dt = t_data[i] - mean_t;
            let dy = y_data[i] - mean_y;
            cov_ty += dt * dy;
            var_t += dt * dt;
        }

        if var_t < 1e-10 {
            return None;
        }

        let beta = cov_ty / var_t;

        // Residual standard error
        let mut sse = 0.0;
        for i in 0..n {
            let pred = mean_y + beta * (t_data[i] - mean_t);
            let residual = y_data[i] - pred;
            sse += residual * residual;
        }

        let mse = sse / (n - 2).max(1) as f64;
        let se = libm::sqrt(mse / var_t);

        Some(CausalEffect::new(beta, se, EstimationMethod::Backdoor, n))
    }

    /// Estimate using inverse probability weighting
    pub fn estimate_ipw(&self) -> Option<CausalEffect> {
        let t_data = self.data.get(&self.treatment)?;
        let y_data = self.data.get(&self.outcome)?;

        if t_data.len() != y_data.len() || t_data.is_empty() {
            return None;
        }

        let n = t_data.len();

        // Estimate propensity scores (simplified: logistic regression)
        let mean_t: f64 = t_data.iter().sum::<f64>() / n as f64;
        let p_treat = mean_t.clamp(0.01, 0.99);

        // Weighted outcomes
        let mut sum_treated = 0.0;
        let mut sum_control = 0.0;
        let mut n_treated = 0.0;
        let mut n_control = 0.0;

        for i in 0..n {
            if t_data[i] > 0.5 {
                // Treated
                let weight = 1.0 / p_treat;
                sum_treated += weight * y_data[i];
                n_treated += weight;
            } else {
                // Control
                let weight = 1.0 / (1.0 - p_treat);
                sum_control += weight * y_data[i];
                n_control += weight;
            }
        }

        if n_treated < 1e-10 || n_control < 1e-10 {
            return None;
        }

        let ate = sum_treated / n_treated - sum_control / n_control;

        // Approximate SE
        let se = libm::sqrt(1.0 / n as f64);

        Some(CausalEffect::new(ate, se, EstimationMethod::IPW, n))
    }
}

// ============================================================================
// KERNEL CAUSAL INFERENCE
// ============================================================================

/// Types of kernel events for causal analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KernelCausalEvent {
    /// Process started
    ProcessStart,
    /// Process ended
    ProcessEnd,
    /// Memory allocation
    MemAlloc,
    /// Memory free
    MemFree,
    /// CPU spike
    CpuSpike,
    /// I/O wait
    IoWait,
    /// Context switch
    ContextSwitch,
    /// System call
    Syscall,
    /// Interrupt
    Interrupt,
    /// Error occurred
    Error,
}

/// Causal chain for root cause analysis
#[derive(Debug, Clone)]
pub struct CausalChain {
    /// Events in causal order
    pub events: Vec<(u64, KernelCausalEvent, f64)>, // (timestamp, event, value)
    /// Causal relationships
    pub links: Vec<(usize, usize, f64)>, // (from_idx, to_idx, strength)
    /// Root cause index
    pub root_cause: Option<usize>,
    /// Final effect index
    pub effect: Option<usize>,
}

impl CausalChain {
    /// Create an empty chain
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            links: Vec::new(),
            root_cause: None,
            effect: None,
        }
    }

    /// Add an event
    pub fn add_event(&mut self, timestamp: u64, event: KernelCausalEvent, value: f64) -> usize {
        let idx = self.events.len();
        self.events.push((timestamp, event, value));
        idx
    }

    /// Add a causal link
    pub fn add_link(&mut self, from: usize, to: usize, strength: f64) {
        self.links.push((from, to, strength));
    }

    /// Find root cause (earliest event with outgoing links)
    pub fn find_root_cause(&mut self) {
        let sources: BTreeSet<usize> = self.links.iter().map(|(f, _, _)| *f).collect();
        let targets: BTreeSet<usize> = self.links.iter().map(|(_, t, _)| *t).collect();

        // Root cause: has outgoing links but no incoming
        let roots: Vec<usize> = sources.difference(&targets).copied().collect();

        if let Some(&root) = roots
            .iter()
            .min_by_key(|&&i| self.events.get(i).map(|(t, _, _)| *t).unwrap_or(u64::MAX))
        {
            self.root_cause = Some(root);
        }
    }

    /// Get chain length
    pub fn length(&self) -> usize {
        // Find longest path
        let mut max_length = 0;
        let mut lengths: BTreeMap<usize, usize> = BTreeMap::new();

        for i in 0..self.events.len() {
            lengths.insert(i, 1);
        }

        for (from, to, _) in &self.links {
            let from_len = lengths.get(from).copied().unwrap_or(1);
            let to_len = lengths.get(to).copied().unwrap_or(1);
            if from_len + 1 > to_len {
                lengths.insert(*to, from_len + 1);
                max_length = max_length.max(from_len + 1);
            }
        }

        max_length
    }
}

impl Default for CausalChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Kernel causal inference manager
pub struct KernelCausalManager {
    /// Causal graph of kernel events
    pub graph: CausalGraph,
    /// Event history
    pub event_history: Vec<(u64, u32, f64)>, // (timestamp, node_id, value)
    /// SCM for kernel
    pub scm: Option<StructuralCausalModel>,
    /// Discovered chains
    pub chains: Vec<CausalChain>,
    /// Event type to node ID mapping
    pub event_nodes: BTreeMap<KernelCausalEvent, u32>,
}

impl KernelCausalManager {
    /// Create a new kernel causal manager
    pub fn new() -> Self {
        Self {
            graph: CausalGraph::new(),
            event_history: Vec::new(),
            scm: None,
            chains: Vec::new(),
            event_nodes: BTreeMap::new(),
        }
    }

    /// Initialize kernel causal graph
    pub fn init_kernel_graph(&mut self) {
        // Add kernel event nodes
        let cpu = self
            .graph
            .add_node(String::from("CPU_Usage"), CausalNodeType::Observed);
        let mem = self
            .graph
            .add_node(String::from("Memory_Usage"), CausalNodeType::Observed);
        let io = self
            .graph
            .add_node(String::from("IO_Wait"), CausalNodeType::Observed);
        let context = self
            .graph
            .add_node(String::from("Context_Switches"), CausalNodeType::Observed);
        let latency = self
            .graph
            .add_node(String::from("Latency"), CausalNodeType::Outcome);
        let error = self
            .graph
            .add_node(String::from("Error_Rate"), CausalNodeType::Outcome);

        // Add causal edges
        self.graph.add_edge(cpu, latency);
        self.graph.add_edge(mem, latency);
        self.graph.add_edge(io, latency);
        self.graph.add_edge(context, latency);
        self.graph.add_edge(cpu, context);
        self.graph.add_edge(mem, io);
        self.graph.add_edge(latency, error);

        // Map events
        self.event_nodes.insert(KernelCausalEvent::CpuSpike, cpu);
        self.event_nodes.insert(KernelCausalEvent::MemAlloc, mem);
        self.event_nodes.insert(KernelCausalEvent::IoWait, io);
        self.event_nodes
            .insert(KernelCausalEvent::ContextSwitch, context);
        self.event_nodes.insert(KernelCausalEvent::Error, error);
    }

    /// Record an event
    pub fn record_event(&mut self, event: KernelCausalEvent, value: f64, timestamp: u64) {
        if let Some(&node_id) = self.event_nodes.get(&event) {
            self.event_history.push((timestamp, node_id, value));

            if let Some(node) = self.graph.nodes.get_mut(&node_id) {
                node.add_observation(value);
            }
        }
    }

    /// Build SCM from graph
    pub fn build_scm(&mut self) {
        let scm = StructuralCausalModel::new(self.graph.clone());
        self.scm = Some(scm);
    }

    /// Perform root cause analysis
    pub fn root_cause_analysis(
        &mut self,
        effect_event: KernelCausalEvent,
        time_window: u64,
    ) -> Option<CausalChain> {
        let effect_node = *self.event_nodes.get(&effect_event)?;

        // Get recent events
        let recent: Vec<_> = self
            .event_history
            .iter()
            .filter(|(t, _, _)| *t >= time_window)
            .cloned()
            .collect();

        if recent.is_empty() {
            return None;
        }

        let mut chain = CausalChain::new();

        // Add events to chain
        let mut node_to_idx: BTreeMap<u32, usize> = BTreeMap::new();

        for (ts, node_id, value) in &recent {
            let event_type = self
                .event_nodes
                .iter()
                .find(|&(_, &n)| n == *node_id)
                .map(|(e, _)| *e)
                .unwrap_or(KernelCausalEvent::Syscall);

            let idx = chain.add_event(*ts, event_type, *value);
            node_to_idx.insert(*node_id, idx);
        }

        // Add causal links based on graph
        for edge in &self.graph.edges {
            if edge.edge_type == CausalEdgeType::Direct {
                if let (Some(&from_idx), Some(&to_idx)) =
                    (node_to_idx.get(&edge.from), node_to_idx.get(&edge.to))
                {
                    let strength = edge.effect.unwrap_or(1.0);
                    chain.add_link(from_idx, to_idx, strength);
                }
            }
        }

        // Find root cause
        chain.find_root_cause();
        chain.effect = node_to_idx.get(&effect_node).copied();

        self.chains.push(chain.clone());

        Some(chain)
    }

    /// Get intervention recommendation
    pub fn recommend_intervention(
        &self,
        target: KernelCausalEvent,
    ) -> Option<(KernelCausalEvent, f64)> {
        let target_node = *self.event_nodes.get(&target)?;

        // Find the strongest parent
        let parents = self.graph.parents(target_node);

        let mut best_parent = None;
        let mut best_effect = 0.0;

        for parent in parents {
            if let Some(edge) = self
                .graph
                .edges
                .iter()
                .find(|e| e.from == parent && e.to == target_node)
            {
                let effect = edge.effect.unwrap_or(1.0);
                if effect.abs() > best_effect {
                    best_effect = effect;
                    best_parent = Some(parent);
                }
            }
        }

        let parent = best_parent?;
        let event = self
            .event_nodes
            .iter()
            .find(|&(_, &n)| n == parent)
            .map(|(e, _)| *e)?;

        Some((event, best_effect))
    }
}

impl Default for KernelCausalManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Linear congruential generator
fn lcg_next(state: u64) -> u64 {
    state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
}

/// Box-Muller transform for normal distribution
fn box_muller(seed: u64) -> f64 {
    let u1 = (seed as f64 / u64::MAX as f64).max(1e-10);
    let seed2 = lcg_next(seed);
    let u2 = seed2 as f64 / u64::MAX as f64;

    libm::sqrt(-2.0 * libm::log(u1)) * libm::cos(2.0 * core::f64::consts::PI * u2)
}

/// Approximate normal CDF
fn normal_cdf(x: f64) -> f64 {
    // Approximation using error function
    0.5 * (1.0 + libm::erf(x / libm::sqrt(2.0)))
}

/// Generate combinations of size k from a set
fn combinations(set: &[u32], k: usize) -> Vec<Vec<u32>> {
    if k == 0 {
        return vec![Vec::new()];
    }
    if set.is_empty() || k > set.len() {
        return Vec::new();
    }

    let mut result = Vec::new();

    for (i, &elem) in set.iter().enumerate() {
        let rest = &set[i + 1..];
        for mut combo in combinations(rest, k - 1) {
            combo.insert(0, elem);
            result.push(combo);
        }
    }

    result
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_causal_node() {
        let mut node = CausalNode::new(0, String::from("X"), CausalNodeType::Observed);

        node.add_observation(1.0);
        node.add_observation(2.0);
        node.add_observation(3.0);

        assert!((node.mean() - 2.0).abs() < 1e-10);
        assert!(node.variance() > 0.0);
    }

    #[test]
    fn test_causal_graph() {
        let mut graph = CausalGraph::new();

        let x = graph.add_node(String::from("X"), CausalNodeType::Treatment);
        let y = graph.add_node(String::from("Y"), CausalNodeType::Outcome);
        let z = graph.add_node(String::from("Z"), CausalNodeType::Observed);

        graph.add_edge(x, z);
        graph.add_edge(z, y);

        assert_eq!(graph.parents(z), vec![x]);
        assert_eq!(graph.children(z), vec![y]);
        assert!(graph.has_path(x, y));
        assert!(graph.is_acyclic());
    }

    #[test]
    fn test_topological_sort() {
        let mut graph = CausalGraph::new();

        let a = graph.add_node(String::from("A"), CausalNodeType::Exogenous);
        let b = graph.add_node(String::from("B"), CausalNodeType::Observed);
        let c = graph.add_node(String::from("C"), CausalNodeType::Outcome);

        graph.add_edge(a, b);
        graph.add_edge(b, c);

        let sorted = graph.topological_sort();
        assert_eq!(sorted.len(), 3);

        // A should come before B, B before C
        let pos_a = sorted.iter().position(|&x| x == a).unwrap();
        let pos_b = sorted.iter().position(|&x| x == b).unwrap();
        let pos_c = sorted.iter().position(|&x| x == c).unwrap();

        assert!(pos_a < pos_b);
        assert!(pos_b < pos_c);
    }

    #[test]
    fn test_structural_equation() {
        let eq = StructuralEquation::new(2, vec![0, 1], vec![0.5, 0.3]);

        let mut values = BTreeMap::new();
        values.insert(0, 2.0);
        values.insert(1, 3.0);

        let result = eq.evaluate(&values, 0.0);
        // 0.5 * 2.0 + 0.3 * 3.0 = 1.0 + 0.9 = 1.9
        assert!((result - 1.9).abs() < 1e-10);

        assert!((eq.partial_derivative(0) - 0.5).abs() < 1e-10);
        assert!((eq.partial_derivative(1) - 0.3).abs() < 1e-10);
    }

    #[test]
    fn test_scm_sampling() {
        let mut graph = CausalGraph::new();
        let x = graph.add_node(String::from("X"), CausalNodeType::Exogenous);
        let y = graph.add_node(String::from("Y"), CausalNodeType::Outcome);

        graph.add_edge(x, y);

        let mut scm = StructuralCausalModel::new(graph);

        // Y = 2*X + noise
        let eq = StructuralEquation::new(y, vec![x], vec![2.0]);
        scm.add_equation(eq);

        let mut noise = BTreeMap::new();
        noise.insert(x, 1.0);
        noise.insert(y, 0.0);

        let values = scm.sample(&noise);

        assert!((values.get(&x).copied().unwrap_or(0.0) - 1.0).abs() < 1e-10);
        assert!((values.get(&y).copied().unwrap_or(0.0) - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_intervention() {
        let mut graph = CausalGraph::new();
        let x = graph.add_node(String::from("X"), CausalNodeType::Treatment);
        let y = graph.add_node(String::from("Y"), CausalNodeType::Outcome);

        graph.add_edge(x, y);

        let mut scm = StructuralCausalModel::new(graph);
        let eq = StructuralEquation::new(y, vec![x], vec![2.0]);
        scm.add_equation(eq);

        // Intervention do(X = 5)
        scm.do_intervention(x, 5.0);

        let mut noise = BTreeMap::new();
        noise.insert(x, 1.0); // This should be ignored
        noise.insert(y, 0.0);

        let values = scm.sample(&noise);

        assert!((values.get(&x).copied().unwrap_or(0.0) - 5.0).abs() < 1e-10);
        assert!((values.get(&y).copied().unwrap_or(0.0) - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_counterfactual_query() {
        let query = CounterfactualQuery::new()
            .given(0, 1.0)
            .given(1, 2.0)
            .had_been(0, 0.0)
            .query(1);

        assert_eq!(query.evidence.len(), 2);
        assert_eq!(query.intervention.len(), 1);
        assert_eq!(query.targets.len(), 1);
    }

    #[test]
    fn test_counterfactual_engine() {
        let mut graph = CausalGraph::new();
        let x = graph.add_node(String::from("X"), CausalNodeType::Treatment);
        let y = graph.add_node(String::from("Y"), CausalNodeType::Outcome);
        graph.add_edge(x, y);

        let mut scm = StructuralCausalModel::new(graph);
        let eq = StructuralEquation::new(y, vec![x], vec![2.0]);
        scm.add_equation(eq);

        let mut engine = CounterfactualEngine::new(scm);

        // Evidence: X=1, Y=2
        // Counterfactual: what if X had been 0?
        let query = CounterfactualQuery::new()
            .given(x, 1.0)
            .given(y, 2.0)
            .had_been(x, 0.0)
            .query(y);

        let result = engine.predict_counterfactual(&query);

        // Y would have been 0 if X had been 0
        assert!((result.get(&y).copied().unwrap_or(999.0) - 0.0).abs() < 0.5);
    }

    #[test]
    fn test_causal_discovery() {
        let mut discovery = CausalDiscovery::new(0.05);

        // Create correlated data
        let x: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|&xi| 2.0 * xi + 0.1).collect();
        let z: Vec<f64> = (0..100).map(|i| (i as f64 * 0.1).sin()).collect();

        discovery.add_variable(0, String::from("X"), x);
        discovery.add_variable(1, String::from("Y"), y);
        discovery.add_variable(2, String::from("Z"), z);

        let graph = discovery.run_pc();

        assert!(!graph.nodes.is_empty());
        let summary = discovery.summary();
        assert!(summary.num_variables == 3);
    }

    #[test]
    fn test_causal_effect() {
        let effect = CausalEffect::new(0.5, 0.1, EstimationMethod::Backdoor, 100);

        assert!(effect.is_significant(0.05));
        assert!(effect.confidence_interval.0 < 0.5);
        assert!(effect.confidence_interval.1 > 0.5);
    }

    #[test]
    fn test_causal_estimator() {
        let mut estimator = CausalEstimator::new(0, 1);

        // Treatment: binary
        let t: Vec<f64> = vec![0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        // Outcome: treatment effect of 2.0
        let y: Vec<f64> = vec![1.0, 1.2, 0.8, 1.1, 0.9, 3.0, 3.2, 2.8, 3.1, 2.9];

        estimator.add_data(0, t);
        estimator.add_data(1, y);

        let effect = estimator.estimate_backdoor().unwrap();

        // Effect should be approximately 2.0
        assert!(effect.estimate > 1.5 && effect.estimate < 2.5);
    }

    #[test]
    fn test_causal_chain() {
        let mut chain = CausalChain::new();

        let e0 = chain.add_event(100, KernelCausalEvent::CpuSpike, 0.9);
        let e1 = chain.add_event(110, KernelCausalEvent::IoWait, 0.5);
        let e2 = chain.add_event(120, KernelCausalEvent::Error, 1.0);

        chain.add_link(e0, e1, 0.8);
        chain.add_link(e1, e2, 0.7);

        chain.find_root_cause();
        chain.effect = Some(e2);

        assert_eq!(chain.root_cause, Some(e0));
        assert_eq!(chain.length(), 3);
    }

    #[test]
    fn test_kernel_causal_manager() {
        let mut manager = KernelCausalManager::new();
        manager.init_kernel_graph();

        // Record events
        manager.record_event(KernelCausalEvent::CpuSpike, 0.9, 100);
        manager.record_event(KernelCausalEvent::IoWait, 0.5, 110);
        manager.record_event(KernelCausalEvent::Error, 1.0, 120);

        // Root cause analysis
        let chain = manager.root_cause_analysis(KernelCausalEvent::Error, 90);

        assert!(chain.is_some());
    }

    #[test]
    fn test_combinations() {
        let set = vec![1, 2, 3, 4];

        let c2 = combinations(&set, 2);
        assert_eq!(c2.len(), 6); // C(4,2) = 6

        let c3 = combinations(&set, 3);
        assert_eq!(c3.len(), 4); // C(4,3) = 4
    }

    #[test]
    fn test_box_muller() {
        let mut sum = 0.0;
        let n = 1000;

        for i in 0..n {
            let seed = lcg_next((i * 12345) as u64);
            sum += box_muller(seed);
        }

        // Mean should be approximately 0
        let mean = sum / n as f64;
        assert!(mean.abs() < 0.5);
    }
}
