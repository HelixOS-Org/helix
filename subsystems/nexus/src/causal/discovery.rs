//! # Causal Discovery Engine for NEXUS
//!
//! Year 2 "COGNITION" - Advanced causal discovery algorithms that enable
//! the kernel AI to automatically discover causal relationships from
//! observational data and intervene intelligently.
//!
//! ## Features
//!
//! - PC Algorithm for constraint-based discovery
//! - Conditional independence testing
//! - Intervention analysis (do-calculus)
//! - Counterfactual reasoning
//! - Causal effect estimation
//! - Structure learning from data

#![allow(dead_code)]
#![allow(clippy::excessive_nesting)]
#![allow(clippy::only_used_in_recursion)]
#![allow(clippy::needless_range_loop)]

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::math::F64Ext;

// ============================================================================
// CONSTANTS
// ============================================================================

/// Default significance level for independence tests
const DEFAULT_ALPHA: f64 = 0.05;

/// Maximum conditioning set size
const MAX_CONDITIONING_SIZE: usize = 5;

/// Minimum samples for valid test
const MIN_SAMPLES: usize = 30;

// ============================================================================
// CORE TYPES
// ============================================================================

/// Variable identifier
pub type VarId = u32;

/// Observation: a set of (variable, value) pairs
#[derive(Debug, Clone)]
pub struct Observation {
    /// Variable values
    pub values: BTreeMap<VarId, f64>,
    /// Timestamp (if temporal)
    pub timestamp: Option<u64>,
    /// Weight (for weighted samples)
    pub weight: f64,
}

impl Observation {
    /// Create a new observation
    pub fn new() -> Self {
        Self {
            values: BTreeMap::new(),
            timestamp: None,
            weight: 1.0,
        }
    }

    /// Set a value
    pub fn set(&mut self, var: VarId, value: f64) -> &mut Self {
        self.values.insert(var, value);
        self
    }

    /// Get a value
    pub fn get(&self, var: VarId) -> Option<f64> {
        self.values.get(&var).copied()
    }

    /// Check if has variable
    pub fn has(&self, var: VarId) -> bool {
        self.values.contains_key(&var)
    }
}

impl Default for Observation {
    fn default() -> Self {
        Self::new()
    }
}

/// Dataset of observations
#[derive(Debug, Clone)]
pub struct Dataset {
    /// All observations
    observations: Vec<Observation>,
    /// Variable names
    var_names: BTreeMap<VarId, String>,
    /// All variables
    variables: BTreeSet<VarId>,
}

impl Dataset {
    /// Create empty dataset
    pub fn new() -> Self {
        Self {
            observations: Vec::new(),
            var_names: BTreeMap::new(),
            variables: BTreeSet::new(),
        }
    }

    /// Add observation
    pub fn add(&mut self, obs: Observation) {
        for &var in obs.values.keys() {
            self.variables.insert(var);
        }
        self.observations.push(obs);
    }

    /// Set variable name
    pub fn set_var_name(&mut self, var: VarId, name: String) {
        self.var_names.insert(var, name);
    }

    /// Get variable name
    pub fn get_var_name(&self, var: VarId) -> Option<&String> {
        self.var_names.get(&var)
    }

    /// Get all variables
    pub fn variables(&self) -> &BTreeSet<VarId> {
        &self.variables
    }

    /// Get sample count
    pub fn len(&self) -> usize {
        self.observations.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.observations.is_empty()
    }

    /// Get values for a variable
    pub fn get_values(&self, var: VarId) -> Vec<f64> {
        self.observations
            .iter()
            .filter_map(|o| o.get(var))
            .collect()
    }

    /// Get subset with conditioning
    pub fn subset_where(&self, conditions: &[(VarId, f64, f64)]) -> Vec<&Observation> {
        self.observations
            .iter()
            .filter(|obs| {
                conditions.iter().all(|(var, low, high)| {
                    obs.get(*var)
                        .map(|v| v >= *low && v <= *high)
                        .unwrap_or(false)
                })
            })
            .collect()
    }

    /// Calculate mean for variable
    pub fn mean(&self, var: VarId) -> Option<f64> {
        let values = self.get_values(var);
        if values.is_empty() {
            return None;
        }
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }

    /// Calculate variance for variable
    pub fn variance(&self, var: VarId) -> Option<f64> {
        let values = self.get_values(var);
        if values.len() < 2 {
            return None;
        }
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let sq_diff: f64 = values.iter().map(|v| (v - mean).powi(2)).sum();
        Some(sq_diff / (values.len() - 1) as f64)
    }

    /// Calculate covariance between two variables
    pub fn covariance(&self, x: VarId, y: VarId) -> Option<f64> {
        let pairs: Vec<(f64, f64)> = self
            .observations
            .iter()
            .filter_map(|obs| {
                let vx = obs.get(x)?;
                let vy = obs.get(y)?;
                Some((vx, vy))
            })
            .collect();

        if pairs.len() < 2 {
            return None;
        }

        let n = pairs.len() as f64;
        let mean_x: f64 = pairs.iter().map(|(x, _)| x).sum::<f64>() / n;
        let mean_y: f64 = pairs.iter().map(|(_, y)| y).sum::<f64>() / n;

        let cov: f64 = pairs.iter().map(|(x, y)| (x - mean_x) * (y - mean_y)).sum();

        Some(cov / (n - 1.0))
    }

    /// Calculate Pearson correlation
    pub fn correlation(&self, x: VarId, y: VarId) -> Option<f64> {
        let cov = self.covariance(x, y)?;
        let var_x = self.variance(x)?;
        let var_y = self.variance(y)?;

        if var_x < 1e-10 || var_y < 1e-10 {
            return None;
        }

        Some(cov / (var_x.sqrt() * var_y.sqrt()))
    }
}

impl Default for Dataset {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// INDEPENDENCE TESTING
// ============================================================================

/// Result of an independence test
#[derive(Debug, Clone)]
pub struct IndependenceTestResult {
    /// Test statistic
    pub statistic: f64,
    /// P-value
    pub p_value: f64,
    /// Are they independent?
    pub independent: bool,
    /// Degrees of freedom
    pub df: usize,
    /// Sample size used
    pub n: usize,
}

/// Independence test type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndependenceTest {
    /// Partial correlation test (Fisher's Z)
    PartialCorrelation,
    /// Conditional mutual information
    ConditionalMI,
    /// G-test for discrete data
    GTest,
}

/// Independence tester
pub struct IndependenceTester {
    /// Significance level (alpha)
    alpha: f64,
    /// Test type
    test_type: IndependenceTest,
}

impl IndependenceTester {
    /// Create new tester
    pub fn new(alpha: f64) -> Self {
        Self {
            alpha,
            test_type: IndependenceTest::PartialCorrelation,
        }
    }

    /// Set test type
    pub fn with_test(mut self, test_type: IndependenceTest) -> Self {
        self.test_type = test_type;
        self
    }

    /// Test if X and Y are conditionally independent given Z
    pub fn test(
        &self,
        data: &Dataset,
        x: VarId,
        y: VarId,
        conditioning: &[VarId],
    ) -> Option<IndependenceTestResult> {
        match self.test_type {
            IndependenceTest::PartialCorrelation => {
                self.partial_correlation_test(data, x, y, conditioning)
            },
            IndependenceTest::ConditionalMI => self.conditional_mi_test(data, x, y, conditioning),
            IndependenceTest::GTest => self.g_test(data, x, y, conditioning),
        }
    }

    /// Partial correlation test using Fisher's Z transformation
    fn partial_correlation_test(
        &self,
        data: &Dataset,
        x: VarId,
        y: VarId,
        conditioning: &[VarId],
    ) -> Option<IndependenceTestResult> {
        let n = data.len();
        if n < MIN_SAMPLES {
            return None;
        }

        let partial_corr = if conditioning.is_empty() {
            data.correlation(x, y)?
        } else {
            self.compute_partial_correlation(data, x, y, conditioning)?
        };

        // Fisher's Z transformation
        let r_clamped = partial_corr.clamp(-0.999, 0.999);
        let fisher_z = 0.5 * ((1.0 + r_clamped) / (1.0 - r_clamped)).ln();

        // Standard error
        let k = conditioning.len();
        let se = 1.0 / ((n - k - 3) as f64).sqrt();

        // Z-statistic
        let z_stat = fisher_z / se;

        // Two-tailed p-value (approximation using normal CDF)
        let p_value = 2.0 * (1.0 - normal_cdf(z_stat.abs()));

        Some(IndependenceTestResult {
            statistic: z_stat,
            p_value,
            independent: p_value > self.alpha,
            df: n - k - 3,
            n,
        })
    }

    /// Compute partial correlation controlling for Z
    fn compute_partial_correlation(
        &self,
        data: &Dataset,
        x: VarId,
        y: VarId,
        conditioning: &[VarId],
    ) -> Option<f64> {
        if conditioning.is_empty() {
            return data.correlation(x, y);
        }

        // Recursive formula for partial correlation
        if conditioning.len() == 1 {
            let z = conditioning[0];
            let r_xy = data.correlation(x, y)?;
            let r_xz = data.correlation(x, z)?;
            let r_yz = data.correlation(y, z)?;

            let numerator = r_xy - r_xz * r_yz;
            let denominator = ((1.0 - r_xz.powi(2)) * (1.0 - r_yz.powi(2))).sqrt();

            if denominator < 1e-10 {
                return None;
            }

            return Some(numerator / denominator);
        }

        // For larger conditioning sets, use regression residuals approach
        // (simplified approximation)
        let last = conditioning.last()?;
        let rest: Vec<VarId> = conditioning[..conditioning.len() - 1].to_vec();

        let r_xy_rest = self.compute_partial_correlation(data, x, y, &rest)?;
        let r_xz_rest = self.compute_partial_correlation(data, x, *last, &rest)?;
        let r_yz_rest = self.compute_partial_correlation(data, y, *last, &rest)?;

        let numerator = r_xy_rest - r_xz_rest * r_yz_rest;
        let denominator = ((1.0 - r_xz_rest.powi(2)) * (1.0 - r_yz_rest.powi(2))).sqrt();

        if denominator < 1e-10 {
            return None;
        }

        Some(numerator / denominator)
    }

    /// Conditional mutual information test
    fn conditional_mi_test(
        &self,
        data: &Dataset,
        x: VarId,
        y: VarId,
        conditioning: &[VarId],
    ) -> Option<IndependenceTestResult> {
        let n = data.len();
        if n < MIN_SAMPLES {
            return None;
        }

        // Compute conditional mutual information (binned approximation)
        let cmi = self.compute_conditional_mi(data, x, y, conditioning)?;

        // Chi-squared approximation: 2n * CMI ~ chi-squared
        let chi_sq = 2.0 * n as f64 * cmi;

        // Degrees of freedom (simplified)
        let df = 1; // Approximation

        // P-value from chi-squared (approximation)
        let p_value = chi_squared_pvalue(chi_sq, df);

        Some(IndependenceTestResult {
            statistic: chi_sq,
            p_value,
            independent: p_value > self.alpha,
            df,
            n,
        })
    }

    /// Compute conditional mutual information
    fn compute_conditional_mi(
        &self,
        data: &Dataset,
        x: VarId,
        y: VarId,
        conditioning: &[VarId],
    ) -> Option<f64> {
        // Discretize data into bins for MI estimation
        const NUM_BINS: usize = 5;

        let x_vals = data.get_values(x);
        let y_vals = data.get_values(y);

        if x_vals.len() < MIN_SAMPLES {
            return None;
        }

        // Simple binning
        let x_bins = discretize(&x_vals, NUM_BINS);
        let y_bins = discretize(&y_vals, NUM_BINS);

        // Compute MI (simplified - ignoring conditioning for now)
        let _ = conditioning; // TODO: full conditional MI

        let mi = compute_mutual_information(&x_bins, &y_bins)?;
        Some(mi)
    }

    /// G-test for discrete variables
    fn g_test(
        &self,
        data: &Dataset,
        x: VarId,
        y: VarId,
        conditioning: &[VarId],
    ) -> Option<IndependenceTestResult> {
        let n = data.len();
        if n < MIN_SAMPLES {
            return None;
        }

        // Discretize if continuous
        const NUM_BINS: usize = 5;

        let x_vals = data.get_values(x);
        let y_vals = data.get_values(y);

        let x_bins = discretize(&x_vals, NUM_BINS);
        let y_bins = discretize(&y_vals, NUM_BINS);

        // Build contingency table
        let mut contingency = [[0u64; 5]; 5];
        let mut x_margin = [0u64; 5];
        let mut y_margin = [0u64; 5];

        for (xi, yi) in x_bins.iter().zip(y_bins.iter()) {
            if *xi < 5 && *yi < 5 {
                contingency[*xi][*yi] += 1;
                x_margin[*xi] += 1;
                y_margin[*yi] += 1;
            }
        }

        // G statistic: 2 * sum(O * ln(O/E))
        let mut g = 0.0;
        let total = n as f64;

        for i in 0..NUM_BINS {
            for j in 0..NUM_BINS {
                let observed = contingency[i][j] as f64;
                if observed > 0.0 {
                    let expected = (x_margin[i] as f64 * y_margin[j] as f64) / total;
                    if expected > 0.0 {
                        g += observed * (observed / expected).ln();
                    }
                }
            }
        }
        g *= 2.0;

        // Degrees of freedom
        let df = (NUM_BINS - 1) * (NUM_BINS - 1);

        // P-value
        let _ = conditioning; // TODO: conditional G-test
        let p_value = chi_squared_pvalue(g, df);

        Some(IndependenceTestResult {
            statistic: g,
            p_value,
            independent: p_value > self.alpha,
            df,
            n,
        })
    }
}

impl Default for IndependenceTester {
    fn default() -> Self {
        Self::new(DEFAULT_ALPHA)
    }
}

// ============================================================================
// PC ALGORITHM
// ============================================================================

/// Skeleton edge (undirected)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SkeletonEdge {
    /// First variable (always smaller)
    pub a: VarId,
    /// Second variable (always larger)
    pub b: VarId,
}

impl SkeletonEdge {
    /// Create edge (normalizes order)
    pub fn new(x: VarId, y: VarId) -> Self {
        if x < y {
            Self { a: x, b: y }
        } else {
            Self { a: y, b: x }
        }
    }
}

/// Separation set (conditioning variables that made X, Y independent)
pub type SepSet = BTreeSet<VarId>;

/// PC Algorithm result
#[derive(Debug, Clone)]
pub struct PCResult {
    /// Skeleton (undirected edges)
    pub skeleton: BTreeSet<SkeletonEdge>,
    /// Separation sets
    pub sep_sets: BTreeMap<(VarId, VarId), SepSet>,
    /// Oriented edges (from, to)
    pub oriented: BTreeSet<(VarId, VarId)>,
    /// Variables
    pub variables: BTreeSet<VarId>,
}

impl PCResult {
    fn new(variables: BTreeSet<VarId>) -> Self {
        Self {
            skeleton: BTreeSet::new(),
            sep_sets: BTreeMap::new(),
            oriented: BTreeSet::new(),
            variables,
        }
    }

    /// Check if there's an edge between x and y
    pub fn has_edge(&self, x: VarId, y: VarId) -> bool {
        self.skeleton.contains(&SkeletonEdge::new(x, y))
    }

    /// Check if x -> y is oriented
    pub fn is_directed(&self, from: VarId, to: VarId) -> bool {
        self.oriented.contains(&(from, to))
    }

    /// Get neighbors of a variable
    pub fn neighbors(&self, x: VarId) -> Vec<VarId> {
        self.skeleton
            .iter()
            .filter_map(|edge| {
                if edge.a == x {
                    Some(edge.b)
                } else if edge.b == x {
                    Some(edge.a)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get parents of a variable (oriented edges pointing to it)
    pub fn parents(&self, x: VarId) -> Vec<VarId> {
        self.oriented
            .iter()
            .filter_map(|(from, to)| if *to == x { Some(*from) } else { None })
            .collect()
    }

    /// Get children of a variable
    pub fn children(&self, x: VarId) -> Vec<VarId> {
        self.oriented
            .iter()
            .filter_map(|(from, to)| if *from == x { Some(*to) } else { None })
            .collect()
    }
}

/// PC Algorithm implementation
pub struct PCAlgorithm {
    /// Independence tester
    tester: IndependenceTester,
    /// Maximum conditioning set size
    max_cond_size: usize,
    /// Verbose mode
    verbose: bool,
}

impl PCAlgorithm {
    /// Create new PC algorithm
    pub fn new(alpha: f64) -> Self {
        Self {
            tester: IndependenceTester::new(alpha),
            max_cond_size: MAX_CONDITIONING_SIZE,
            verbose: false,
        }
    }

    /// Set maximum conditioning size
    pub fn with_max_cond_size(mut self, size: usize) -> Self {
        self.max_cond_size = size;
        self
    }

    /// Run PC algorithm on dataset
    pub fn run(&self, data: &Dataset) -> PCResult {
        let vars: BTreeSet<VarId> = data.variables().clone();
        let mut result = PCResult::new(vars.clone());

        // Phase 1: Build complete skeleton
        for &x in &vars {
            for &y in &vars {
                if x < y {
                    result.skeleton.insert(SkeletonEdge::new(x, y));
                }
            }
        }

        // Phase 2: Remove edges via conditional independence testing
        for cond_size in 0..=self.max_cond_size {
            let edges_to_check: Vec<SkeletonEdge> = result.skeleton.iter().copied().collect();

            for edge in edges_to_check {
                if !result.skeleton.contains(&edge) {
                    continue;
                }

                let x = edge.a;
                let y = edge.b;

                // Get potential conditioning sets from neighbors
                let neighbors_x: BTreeSet<VarId> = result
                    .neighbors(x)
                    .into_iter()
                    .filter(|&n| n != y)
                    .collect();

                let neighbors_y: BTreeSet<VarId> = result
                    .neighbors(y)
                    .into_iter()
                    .filter(|&n| n != x)
                    .collect();

                // Try conditioning sets from neighbors of X
                for subset in subsets_of_size(&neighbors_x, cond_size) {
                    if let Some(test_result) = self.tester.test(data, x, y, &subset) {
                        if test_result.independent {
                            result.skeleton.remove(&edge);
                            result
                                .sep_sets
                                .insert((x, y), subset.iter().copied().collect());
                            result
                                .sep_sets
                                .insert((y, x), subset.iter().copied().collect());
                            break;
                        }
                    }
                }

                // If not removed, try neighbors of Y
                if result.skeleton.contains(&edge) {
                    for subset in subsets_of_size(&neighbors_y, cond_size) {
                        if let Some(test_result) = self.tester.test(data, x, y, &subset) {
                            if test_result.independent {
                                result.skeleton.remove(&edge);
                                result
                                    .sep_sets
                                    .insert((x, y), subset.iter().copied().collect());
                                result
                                    .sep_sets
                                    .insert((y, x), subset.iter().copied().collect());
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Phase 3: Orient edges (v-structures and rules)
        self.orient_edges(&mut result);

        result
    }

    /// Orient edges based on v-structures and Meek's rules
    fn orient_edges(&self, result: &mut PCResult) {
        // Step 1: Orient v-structures (X - Z - Y becomes X -> Z <- Y if Z not in sep(X,Y))
        let vars: Vec<VarId> = result.variables.iter().copied().collect();

        for &z in &vars {
            let neighbors: Vec<VarId> = result.neighbors(z);

            for i in 0..neighbors.len() {
                for j in (i + 1)..neighbors.len() {
                    let x = neighbors[i];
                    let y = neighbors[j];

                    // Check if X and Y are not adjacent
                    if !result.has_edge(x, y) {
                        // Check if Z is not in sep_set(X, Y)
                        let sep = result
                            .sep_sets
                            .get(&(x.min(y), x.max(y)))
                            .cloned()
                            .unwrap_or_default();

                        if !sep.contains(&z) {
                            // Orient as v-structure: X -> Z <- Y
                            result.oriented.insert((x, z));
                            result.oriented.insert((y, z));
                        }
                    }
                }
            }
        }

        // Step 2: Apply Meek's rules until no more changes
        let mut changed = true;
        while changed {
            changed = false;

            // Rule 1: If X -> Y - Z and X, Z not adjacent, orient Y -> Z
            for edge in result.skeleton.iter() {
                let (a, b) = (edge.a, edge.b);

                // Check both directions of the undirected edge
                for (y, z) in [(a, b), (b, a)] {
                    if result.is_directed(y, z) || result.is_directed(z, y) {
                        continue;
                    }

                    // Find X such that X -> Y and X not adjacent to Z
                    for &x in &result.parents(y) {
                        if !result.has_edge(x, z) {
                            result.oriented.insert((y, z));
                            changed = true;
                            break;
                        }
                    }
                }
            }

            // Rule 2: If X -> Y -> Z and X - Z, orient X -> Z
            for edge in result.skeleton.iter() {
                let (x, z) = (edge.a, edge.b);

                if result.is_directed(x, z) || result.is_directed(z, x) {
                    continue;
                }

                // Find Y such that X -> Y -> Z
                for &y in &result.children(x) {
                    if result.is_directed(y, z) {
                        result.oriented.insert((x, z));
                        changed = true;
                        break;
                    }
                }

                // And check reverse: Z -> Y -> X means orient Z -> X
                for &y in &result.children(z) {
                    if result.is_directed(y, x) {
                        result.oriented.insert((z, x));
                        changed = true;
                        break;
                    }
                }
            }
        }
    }
}

impl Default for PCAlgorithm {
    fn default() -> Self {
        Self::new(DEFAULT_ALPHA)
    }
}

// ============================================================================
// INTERVENTION ANALYSIS (DO-CALCULUS)
// ============================================================================

/// An intervention on a variable
#[derive(Debug, Clone)]
pub struct Intervention {
    /// Variable being intervened on
    pub variable: VarId,
    /// Intervention value
    pub value: f64,
}

impl Intervention {
    /// Create new intervention
    pub fn new(variable: VarId, value: f64) -> Self {
        Self { variable, value }
    }
}

/// Interventional query result
#[derive(Debug, Clone)]
pub struct InterventionResult {
    /// Target variable
    pub target: VarId,
    /// Interventions applied
    pub interventions: Vec<Intervention>,
    /// Estimated effect
    pub effect: f64,
    /// Confidence interval (if available)
    pub confidence: Option<(f64, f64)>,
    /// Method used
    pub method: String,
}

/// Causal effect estimator using do-calculus
pub struct DoCalculus<'a> {
    /// Causal graph (from PC algorithm)
    graph: &'a PCResult,
    /// Observational data
    data: &'a Dataset,
}

impl<'a> DoCalculus<'a> {
    /// Create new do-calculus estimator
    pub fn new(graph: &'a PCResult, data: &'a Dataset) -> Self {
        Self { graph, data }
    }

    /// Estimate P(Y | do(X = x))
    pub fn estimate_effect(
        &self,
        target: VarId,
        intervention: &Intervention,
    ) -> Option<InterventionResult> {
        // Check if we can use backdoor adjustment
        if let Some(adjustment_set) = self.find_backdoor_adjustment(intervention.variable, target) {
            return self.backdoor_adjustment(target, intervention, &adjustment_set);
        }

        // Try front-door adjustment
        if let Some(mediators) = self.find_frontdoor_adjustment(intervention.variable, target) {
            return self.frontdoor_adjustment(target, intervention, &mediators);
        }

        None
    }

    /// Find a valid backdoor adjustment set
    fn find_backdoor_adjustment(&self, x: VarId, _y: VarId) -> Option<Vec<VarId>> {
        // A set Z satisfies the backdoor criterion if:
        // 1. No node in Z is a descendant of X
        // 2. Z blocks all backdoor paths from X to Y

        let descendants_x = self.get_descendants(x);
        let parents_x = self.graph.parents(x);

        // Simple heuristic: use parents of X that are not descendants
        let adjustment: Vec<VarId> = parents_x
            .into_iter()
            .filter(|&z| !descendants_x.contains(&z))
            .collect();

        if !adjustment.is_empty() {
            Some(adjustment)
        } else if self.graph.parents(x).is_empty() {
            // If X has no parents, we can use empty adjustment set
            // (no confounding)
            Some(Vec::new())
        } else {
            None
        }
    }

    /// Find front-door adjustment mediators
    fn find_frontdoor_adjustment(&self, x: VarId, y: VarId) -> Option<Vec<VarId>> {
        // M satisfies front-door criterion if:
        // 1. M intercepts all directed paths from X to Y
        // 2. No backdoor path from X to M
        // 3. All backdoor paths from M to Y are blocked by X

        let children_x = self.graph.children(x);

        // Find mediator that satisfies conditions
        for &m in &children_x {
            if self.graph.children(m).contains(&y) {
                // M is on path X -> M -> Y
                return Some(vec![m]);
            }
        }

        None
    }

    /// Apply backdoor adjustment formula
    fn backdoor_adjustment(
        &self,
        target: VarId,
        intervention: &Intervention,
        adjustment_set: &[VarId],
    ) -> Option<InterventionResult> {
        // P(Y | do(X=x)) = sum_z P(Y | X=x, Z=z) P(Z=z)

        if adjustment_set.is_empty() {
            // No confounding - just condition on X
            let effect = self.estimate_conditional_mean(target, &[(
                intervention.variable,
                intervention.value,
            )])?;

            return Some(InterventionResult {
                target,
                interventions: vec![intervention.clone()],
                effect,
                confidence: None,
                method: String::from("backdoor_adjustment"),
            });
        }

        // For non-empty adjustment set, use stratification
        // This is a simplified version
        let mut weighted_sum = 0.0;
        let mut weight_total = 0.0;

        // Discretize adjustment variables
        for z_value in 0..5 {
            let z = adjustment_set[0]; // Simplified: use first variable
            let z_obs = self.data.get_values(z);
            let z_range = self.get_bin_range(&z_obs, z_value);

            // P(Z = z)
            let z_count = self.data.subset_where(&[(z, z_range.0, z_range.1)]).len();
            let p_z = z_count as f64 / self.data.len() as f64;

            if p_z > 0.0 {
                // E[Y | X=x, Z=z]
                if let Some(cond_mean) = self.estimate_conditional_mean_with_z(
                    target,
                    intervention.variable,
                    intervention.value,
                    z,
                    z_range,
                ) {
                    weighted_sum += cond_mean * p_z;
                    weight_total += p_z;
                }
            }
        }

        if weight_total < 1e-10 {
            return None;
        }

        Some(InterventionResult {
            target,
            interventions: vec![intervention.clone()],
            effect: weighted_sum / weight_total,
            confidence: None,
            method: String::from("backdoor_adjustment"),
        })
    }

    /// Apply front-door adjustment formula
    fn frontdoor_adjustment(
        &self,
        target: VarId,
        intervention: &Intervention,
        mediators: &[VarId],
    ) -> Option<InterventionResult> {
        // P(Y | do(X=x)) = sum_m P(M=m | X=x) sum_x' P(Y | X=x', M=m) P(X=x')

        let m = mediators[0]; // Use first mediator

        let mut effect = 0.0;
        let m_values = self.data.get_values(m);

        for m_bin in 0..5 {
            let m_range = self.get_bin_range(&m_values, m_bin);

            // P(M=m | X=x)
            let p_m_given_x = self.estimate_conditional_prob(m, m_range, &[(
                intervention.variable,
                intervention.value,
            )])?;

            // sum_x' P(Y | X=x', M=m) P(X=x')
            let mut inner_sum = 0.0;
            let x_values = self.data.get_values(intervention.variable);

            for x_bin in 0..5 {
                let x_range = self.get_bin_range(&x_values, x_bin);
                let p_x = self.estimate_marginal_prob(intervention.variable, x_range)?;

                if p_x > 0.0 {
                    if let Some(y_mean) = self.estimate_conditional_mean_with_z(
                        target,
                        m,
                        (m_range.0 + m_range.1) / 2.0,
                        intervention.variable,
                        x_range,
                    ) {
                        inner_sum += y_mean * p_x;
                    }
                }
            }

            effect += p_m_given_x * inner_sum;
        }

        Some(InterventionResult {
            target,
            interventions: vec![intervention.clone()],
            effect,
            confidence: None,
            method: String::from("frontdoor_adjustment"),
        })
    }

    /// Get descendants of a variable
    fn get_descendants(&self, x: VarId) -> BTreeSet<VarId> {
        let mut descendants = BTreeSet::new();
        let mut queue = vec![x];

        while let Some(node) = queue.pop() {
            for &child in &self.graph.children(node) {
                if descendants.insert(child) {
                    queue.push(child);
                }
            }
        }

        descendants
    }

    /// Estimate conditional mean E[Y | X=x]
    fn estimate_conditional_mean(&self, target: VarId, conditions: &[(VarId, f64)]) -> Option<f64> {
        // Find observations close to condition values
        let matching: Vec<&Observation> = self
            .data
            .observations
            .iter()
            .filter(|obs| {
                conditions.iter().all(|(var, value)| {
                    obs.get(*var)
                        .map(|v| (v - value).abs() < 0.5) // Tolerance
                        .unwrap_or(false)
                })
            })
            .collect();

        if matching.is_empty() {
            return None;
        }

        let sum: f64 = matching.iter().filter_map(|obs| obs.get(target)).sum();
        Some(sum / matching.len() as f64)
    }

    /// Estimate conditional mean with range for Z
    fn estimate_conditional_mean_with_z(
        &self,
        target: VarId,
        x: VarId,
        x_value: f64,
        z: VarId,
        z_range: (f64, f64),
    ) -> Option<f64> {
        let matching: Vec<&Observation> = self
            .data
            .observations
            .iter()
            .filter(|obs| {
                let x_ok = obs
                    .get(x)
                    .map(|v| (v - x_value).abs() < 0.5)
                    .unwrap_or(false);
                let z_ok = obs
                    .get(z)
                    .map(|v| v >= z_range.0 && v <= z_range.1)
                    .unwrap_or(false);
                x_ok && z_ok
            })
            .collect();

        if matching.is_empty() {
            return None;
        }

        let sum: f64 = matching.iter().filter_map(|obs| obs.get(target)).sum();
        Some(sum / matching.len() as f64)
    }

    /// Estimate conditional probability
    fn estimate_conditional_prob(
        &self,
        var: VarId,
        range: (f64, f64),
        conditions: &[(VarId, f64)],
    ) -> Option<f64> {
        let matching: Vec<&Observation> = self
            .data
            .observations
            .iter()
            .filter(|obs| {
                conditions.iter().all(|(v, value)| {
                    obs.get(*v)
                        .map(|val| (val - value).abs() < 0.5)
                        .unwrap_or(false)
                })
            })
            .collect();

        if matching.is_empty() {
            return None;
        }

        let in_range = matching
            .iter()
            .filter(|obs| {
                obs.get(var)
                    .map(|v| v >= range.0 && v <= range.1)
                    .unwrap_or(false)
            })
            .count();

        Some(in_range as f64 / matching.len() as f64)
    }

    /// Estimate marginal probability
    fn estimate_marginal_prob(&self, var: VarId, range: (f64, f64)) -> Option<f64> {
        let values = self.data.get_values(var);
        if values.is_empty() {
            return None;
        }

        let in_range = values
            .iter()
            .filter(|&&v| v >= range.0 && v <= range.1)
            .count();

        Some(in_range as f64 / values.len() as f64)
    }

    /// Get bin range for discretization
    fn get_bin_range(&self, values: &[f64], bin: usize) -> (f64, f64) {
        if values.is_empty() {
            return (0.0, 1.0);
        }

        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        let bin_width = (max - min) / 5.0;
        let low = min + bin as f64 * bin_width;
        let high = low + bin_width;

        (low, high)
    }
}

// ============================================================================
// COUNTERFACTUAL REASONING
// ============================================================================

/// A counterfactual query
#[derive(Debug, Clone)]
pub struct CounterfactualQuery {
    /// What we observed (evidence)
    pub evidence: Observation,
    /// What intervention we're considering
    pub intervention: Intervention,
    /// What variable we want to know about
    pub target: VarId,
}

/// Counterfactual result
#[derive(Debug, Clone)]
pub struct CounterfactualResult {
    /// Query
    pub query: CounterfactualQuery,
    /// Counterfactual value
    pub value: f64,
    /// Actual (factual) value
    pub actual: Option<f64>,
    /// Difference (counterfactual - actual)
    pub difference: Option<f64>,
}

/// Counterfactual reasoner
pub struct CounterfactualReasoner<'a> {
    /// Causal graph
    graph: &'a PCResult,
    /// Data
    data: &'a Dataset,
}

impl<'a> CounterfactualReasoner<'a> {
    /// Create new reasoner
    pub fn new(graph: &'a PCResult, data: &'a Dataset) -> Self {
        Self { graph, data }
    }

    /// Answer a counterfactual query
    pub fn query(&self, query: CounterfactualQuery) -> Option<CounterfactualResult> {
        // Step 1: Abduction - infer exogenous variables from evidence
        // Step 2: Action - modify the model with intervention
        // Step 3: Prediction - compute target under modified model

        // Simplified: Use matching to find similar units
        let actual = query.evidence.get(query.target);

        // Find units with similar evidence
        let similar = self.find_similar(&query.evidence);

        if similar.is_empty() {
            return None;
        }

        // Among similar units, find those with intervention value
        let intervened: Vec<&&Observation> = similar
            .iter()
            .filter(|obs| {
                obs.get(query.intervention.variable)
                    .map(|v| (v - query.intervention.value).abs() < 0.5)
                    .unwrap_or(false)
            })
            .collect();

        if intervened.is_empty() {
            return None;
        }

        // Estimate counterfactual value
        let cf_sum: f64 = intervened
            .iter()
            .filter_map(|obs| obs.get(query.target))
            .sum();

        let counterfactual = cf_sum / intervened.len() as f64;

        Some(CounterfactualResult {
            query: query.clone(),
            value: counterfactual,
            actual,
            difference: actual.map(|a| counterfactual - a),
        })
    }

    /// Find similar observations based on covariates
    fn find_similar(&self, evidence: &Observation) -> Vec<&Observation> {
        // Get parents of the intervention and target as matching variables
        self.data
            .observations
            .iter()
            .filter(|obs| {
                // Match on all evidence variables (except target and intervention)
                evidence.values.iter().all(|(var, value)| {
                    obs.get(*var)
                        .map(|v| (v - value).abs() < 1.0)
                        .unwrap_or(true)
                })
            })
            .collect()
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Discretize continuous values into bins
fn discretize(values: &[f64], num_bins: usize) -> Vec<usize> {
    if values.is_empty() {
        return Vec::new();
    }

    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    let bin_width = (max - min + 1e-10) / num_bins as f64;

    values
        .iter()
        .map(|v| {
            let bin = ((v - min) / bin_width) as usize;
            bin.min(num_bins - 1)
        })
        .collect()
}

/// Compute mutual information between two discretized variables
fn compute_mutual_information(x: &[usize], y: &[usize]) -> Option<f64> {
    if x.len() != y.len() || x.is_empty() {
        return None;
    }

    let n = x.len() as f64;
    let mut joint = [[0u64; 10]; 10];
    let mut px = [0u64; 10];
    let mut py = [0u64; 10];

    for (&xi, &yi) in x.iter().zip(y.iter()) {
        if xi < 10 && yi < 10 {
            joint[xi][yi] += 1;
            px[xi] += 1;
            py[yi] += 1;
        }
    }

    let mut mi = 0.0;

    for i in 0..10 {
        for j in 0..10 {
            if joint[i][j] > 0 && px[i] > 0 && py[j] > 0 {
                let pxy = joint[i][j] as f64 / n;
                let pxi = px[i] as f64 / n;
                let pyj = py[j] as f64 / n;
                mi += pxy * (pxy / (pxi * pyj)).ln();
            }
        }
    }

    Some(mi.max(0.0))
}

/// Generate all subsets of a given size
fn subsets_of_size(set: &BTreeSet<VarId>, size: usize) -> Vec<Vec<VarId>> {
    if size == 0 {
        return vec![vec![]];
    }

    let elements: Vec<VarId> = set.iter().copied().collect();
    if size > elements.len() {
        return Vec::new();
    }

    let mut result = Vec::new();
    let mut indices: Vec<usize> = (0..size).collect();

    loop {
        let subset: Vec<VarId> = indices.iter().map(|&i| elements[i]).collect();
        result.push(subset);

        // Find rightmost index that can be incremented
        let mut i = size;
        while i > 0 {
            i -= 1;
            if indices[i] < elements.len() - size + i {
                indices[i] += 1;
                for j in (i + 1)..size {
                    indices[j] = indices[j - 1] + 1;
                }
                break;
            }
        }

        if i == 0 && indices[0] > elements.len() - size {
            break;
        }
    }

    result
}

/// Standard normal CDF approximation
fn normal_cdf(x: f64) -> f64 {
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs() / core::f64::consts::SQRT_2;

    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();

    0.5 * (1.0 + sign * y)
}

/// Chi-squared p-value approximation
fn chi_squared_pvalue(chi_sq: f64, df: usize) -> f64 {
    // Wilson-Hilferty approximation for chi-squared CDF
    if df == 0 || chi_sq <= 0.0 {
        return 1.0;
    }

    let k = df as f64;
    let z = ((chi_sq / k).powf(1.0 / 3.0) - (1.0 - 2.0 / (9.0 * k))) / (2.0 / (9.0 * k)).sqrt();

    1.0 - normal_cdf(z)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_dataset() -> Dataset {
        let mut data = Dataset::new();

        // X -> Y -> Z chain
        for i in 0..100 {
            let mut obs = Observation::new();
            let x = (i as f64) / 10.0;
            let noise_y = ((i * 7) % 10) as f64 / 20.0;
            let noise_z = ((i * 13) % 10) as f64 / 20.0;
            let y = 2.0 * x + noise_y;
            let z = 1.5 * y + noise_z;

            obs.set(0, x);
            obs.set(1, y);
            obs.set(2, z);
            data.add(obs);
        }

        data.set_var_name(0, String::from("X"));
        data.set_var_name(1, String::from("Y"));
        data.set_var_name(2, String::from("Z"));

        data
    }

    #[test]
    fn test_dataset() {
        let data = create_test_dataset();

        assert_eq!(data.len(), 100);
        assert!(data.mean(0).is_some());
        assert!(data.variance(0).is_some());
        assert!(data.correlation(0, 1).is_some());
    }

    #[test]
    fn test_independence_tester() {
        let data = create_test_dataset();
        let tester = IndependenceTester::new(0.05);

        // X and Y should be dependent
        let result = tester.test(&data, 0, 1, &[]);
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(!r.independent);
    }

    #[test]
    fn test_pc_algorithm() {
        let data = create_test_dataset();
        let pc = PCAlgorithm::new(0.05);

        let result = pc.run(&data);

        // Should find edges
        assert!(!result.skeleton.is_empty());
    }

    #[test]
    fn test_subsets() {
        let set: BTreeSet<VarId> = [1, 2, 3].iter().copied().collect();

        let subsets_0 = subsets_of_size(&set, 0);
        assert_eq!(subsets_0.len(), 1);
        assert!(subsets_0[0].is_empty());

        let subsets_1 = subsets_of_size(&set, 1);
        assert_eq!(subsets_1.len(), 3);

        let subsets_2 = subsets_of_size(&set, 2);
        assert_eq!(subsets_2.len(), 3);
    }

    #[test]
    fn test_observation() {
        let mut obs = Observation::new();
        obs.set(0, 1.5).set(1, 2.5);

        assert_eq!(obs.get(0), Some(1.5));
        assert_eq!(obs.get(1), Some(2.5));
        assert!(obs.has(0));
        assert!(!obs.has(2));
    }

    #[test]
    fn test_intervention() {
        let int = Intervention::new(0, 5.0);
        assert_eq!(int.variable, 0);
        assert_eq!(int.value, 5.0);
    }
}
