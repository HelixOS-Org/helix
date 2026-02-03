//! # Explainable AI (XAI) Engine for Helix OS Kernel
//!
//! Year 3 "EVOLUTION" - Revolutionary explainability system that makes
//! kernel-level AI decisions transparent, interpretable, and auditable.
//!
//! ## Key Features
//!
//! - **LIME (Local Interpretable Model-agnostic Explanations)**: Local linear approximations
//! - **SHAP (SHapley Additive exPlanations)**: Game-theoretic feature attribution
//! - **Attention Visualization**: Highlighting important input features
//! - **Counterfactual Explanations**: "What would change the decision?"
//! - **Rule Extraction**: Extracting symbolic rules from neural networks
//! - **Concept Activation Vectors**: Understanding high-level concepts
//!
//! ## Kernel Applications
//!
//! - Explain why a process was killed
//! - Justify scheduler decisions
//! - Audit security decisions
//! - Debug anomaly detection
//! - Visualize memory allocation reasoning

#![no_std]

extern crate alloc;
use alloc::format;
use crate::math::F64Ext;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

// Year 2 COGNITION sub-modules
pub mod natural_explanation;

// Re-exports
pub use natural_explanation::{
    NaturalExplanationGenerator, KernelExplainer, DecisionData, Factor, Alternative,
    CausalStep, ExplanationContext, DetailLevel, Audience, DecisionType,
};

// ============================================================================
// CORE EXPLANATION TYPES
// ============================================================================

/// Types of explanations supported
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplanationType {
    /// Feature importance ranking
    FeatureImportance,
    /// Local linear approximation (LIME)
    LocalLinear,
    /// SHAP values
    Shapley,
    /// Attention weights
    Attention,
    /// Counterfactual examples
    Counterfactual,
    /// Extracted rules
    Rules,
    /// Concept activations
    Concepts,
    /// Prototype comparison
    Prototype,
    /// Influence functions
    Influence,
    /// Saliency maps
    Saliency,
}

/// Feature attribution for a single prediction
#[derive(Debug, Clone)]
pub struct FeatureAttribution {
    /// Feature name/identifier
    pub feature_id: u32,
    /// Feature name (optional)
    pub name: Option<String>,
    /// Attribution value (positive = supports prediction, negative = opposes)
    pub attribution: f64,
    /// Baseline value for comparison
    pub baseline: f64,
    /// Current value
    pub value: f64,
    /// Confidence in this attribution
    pub confidence: f64,
}

impl FeatureAttribution {
    /// Create a new feature attribution
    pub fn new(feature_id: u32, attribution: f64, value: f64) -> Self {
        Self {
            feature_id,
            name: None,
            attribution,
            baseline: 0.0,
            value,
            confidence: 1.0,
        }
    }

    /// Set the feature name
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// Check if this feature supports the prediction
    pub fn supports_prediction(&self) -> bool {
        self.attribution > 0.0
    }

    /// Get absolute importance
    pub fn importance(&self) -> f64 {
        libm::fabs(self.attribution)
    }
}

/// A complete explanation for a prediction
#[derive(Debug, Clone)]
pub struct Explanation {
    /// Type of explanation
    pub explanation_type: ExplanationType,
    /// Feature attributions
    pub attributions: Vec<FeatureAttribution>,
    /// Predicted class/value
    pub prediction: f64,
    /// Confidence in prediction
    pub prediction_confidence: f64,
    /// Base value (expected prediction without features)
    pub base_value: f64,
    /// Human-readable summary
    pub summary: Option<String>,
    /// Timestamp
    pub timestamp: u64,
    /// Additional metadata
    pub metadata: BTreeMap<String, f64>,
}

impl Explanation {
    /// Create a new explanation
    pub fn new(explanation_type: ExplanationType, prediction: f64) -> Self {
        Self {
            explanation_type,
            attributions: Vec::new(),
            prediction,
            prediction_confidence: 1.0,
            base_value: 0.0,
            summary: None,
            timestamp: 0,
            metadata: BTreeMap::new(),
        }
    }

    /// Add an attribution
    pub fn add_attribution(&mut self, attr: FeatureAttribution) {
        self.attributions.push(attr);
    }

    /// Sort attributions by importance (descending)
    pub fn sort_by_importance(&mut self) {
        self.attributions
            .sort_by(|a, b| b.importance().partial_cmp(&a.importance()).unwrap());
    }

    /// Get top K most important features
    pub fn top_k_features(&self, k: usize) -> Vec<&FeatureAttribution> {
        let mut sorted = self.attributions.clone();
        sorted.sort_by(|a, b| b.importance().partial_cmp(&a.importance()).unwrap());
        self.attributions.iter().take(k).collect()
    }

    /// Check if explanation is faithful (attributions sum to prediction - base)
    pub fn check_faithfulness(&self, tolerance: f64) -> bool {
        let sum: f64 = self.attributions.iter().map(|a| a.attribution).sum();
        let expected = self.prediction - self.base_value;
        libm::fabs(sum - expected) < tolerance
    }

    /// Generate a human-readable summary
    pub fn generate_summary(&mut self) {
        let mut parts = Vec::new();
        parts.push(alloc::format!("Prediction: {:.4}", self.prediction));

        // Top 3 supporting features
        let supporting: Vec<_> = self
            .attributions
            .iter()
            .filter(|a| a.supports_prediction())
            .take(3)
            .collect();

        if !supporting.is_empty() {
            parts.push(alloc::format!("Top factors: "));
            for attr in supporting {
                let name = attr
                    .name
                    .clone()
                    .unwrap_or_else(|| alloc::format!("F{}", attr.feature_id));
                parts.push(alloc::format!("{} (+{:.3})", name, attr.attribution));
            }
        }

        self.summary = Some(parts.join(" "));
    }
}

// ============================================================================
// LIME: LOCAL INTERPRETABLE MODEL-AGNOSTIC EXPLANATIONS
// ============================================================================

/// Configuration for LIME
#[derive(Debug, Clone)]
pub struct LimeConfig {
    /// Number of perturbation samples
    pub num_samples: usize,
    /// Kernel width for locality
    pub kernel_width: f64,
    /// Number of features in explanation
    pub num_features: usize,
    /// Discretize continuous features
    pub discretize: bool,
    /// Random seed
    pub seed: u64,
}

impl Default for LimeConfig {
    fn default() -> Self {
        Self {
            num_samples: 1000,
            kernel_width: 0.75,
            num_features: 10,
            discretize: false,
            seed: 42,
        }
    }
}

/// A perturbation sample for LIME
#[derive(Debug, Clone)]
struct PerturbedSample {
    /// Binary mask indicating which features are present
    mask: Vec<bool>,
    /// Feature values
    features: Vec<f64>,
    /// Model prediction on this sample
    prediction: f64,
    /// Weight based on similarity to original
    weight: f64,
}

/// LIME Explainer
pub struct LimeExplainer {
    /// Configuration
    config: LimeConfig,
    /// Stored samples for analysis
    samples: Vec<PerturbedSample>,
    /// Random state
    rng_state: u64,
}

impl LimeExplainer {
    /// Create a new LIME explainer
    pub fn new(config: LimeConfig) -> Self {
        let rng_state = config.seed;
        Self {
            config,
            samples: Vec::new(),
            rng_state,
        }
    }

    /// Generate perturbation samples
    fn generate_perturbations(&mut self, original: &[f64]) -> Vec<Vec<f64>> {
        let n_features = original.len();
        let mut perturbations = Vec::with_capacity(self.config.num_samples);

        for _ in 0..self.config.num_samples {
            let mut sample = original.to_vec();
            let mut mask = vec![true; n_features];

            for i in 0..n_features {
                self.rng_state = lcg_next(self.rng_state);
                if self.rng_state % 100 < 50 {
                    // Perturb this feature
                    mask[i] = false;
                    self.rng_state = lcg_next(self.rng_state);
                    let noise = ((self.rng_state as f64 / u64::MAX as f64) * 2.0 - 1.0) * 0.5;
                    sample[i] = original[i] + noise * libm::fabs(original[i]).max(1.0);
                }
            }

            perturbations.push(sample);
        }

        perturbations
    }

    /// Calculate similarity weight
    fn calculate_weight(&self, original: &[f64], perturbed: &[f64]) -> f64 {
        let distance: f64 = original
            .iter()
            .zip(perturbed.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt();

        let sigma = self.config.kernel_width;
        libm::exp(-(distance * distance) / (2.0 * sigma * sigma))
    }

    /// Fit a weighted linear model
    fn fit_linear_model(&self, samples: &[PerturbedSample]) -> Vec<f64> {
        if samples.is_empty() {
            return Vec::new();
        }

        let n_features = samples[0].features.len();
        let n_samples = samples.len();

        // Weighted linear regression using normal equations
        // X^T W X beta = X^T W y

        // For simplicity, use correlation-based importance
        let mut coefficients = vec![0.0; n_features];

        // Calculate weighted mean of predictions
        let total_weight: f64 = samples.iter().map(|s| s.weight).sum();
        let mean_pred: f64 =
            samples.iter().map(|s| s.prediction * s.weight).sum::<f64>() / total_weight.max(1e-10);

        for feat_idx in 0..n_features {
            // Weighted mean of feature
            let mean_feat: f64 = samples
                .iter()
                .map(|s| s.features[feat_idx] * s.weight)
                .sum::<f64>()
                / total_weight.max(1e-10);

            // Weighted covariance
            let mut cov = 0.0;
            let mut var_feat = 0.0;

            for sample in samples {
                let feat_diff = sample.features[feat_idx] - mean_feat;
                let pred_diff = sample.prediction - mean_pred;
                cov += sample.weight * feat_diff * pred_diff;
                var_feat += sample.weight * feat_diff * feat_diff;
            }

            // Coefficient
            if var_feat.abs() > 1e-10 {
                coefficients[feat_idx] = cov / var_feat;
            }
        }

        coefficients
    }

    /// Explain a prediction using LIME
    pub fn explain<F>(&mut self, input: &[f64], predict_fn: F) -> Explanation
    where
        F: Fn(&[f64]) -> f64,
    {
        let original_pred = predict_fn(input);

        // Generate perturbations
        let perturbations = self.generate_perturbations(input);

        // Evaluate model on perturbations and calculate weights
        self.samples.clear();
        for perturbed in perturbations {
            let pred = predict_fn(&perturbed);
            let weight = self.calculate_weight(input, &perturbed);

            let mask = input
                .iter()
                .zip(perturbed.iter())
                .map(|(a, b)| (a - b).abs() < 1e-10)
                .collect();

            self.samples.push(PerturbedSample {
                mask,
                features: perturbed,
                prediction: pred,
                weight,
            });
        }

        // Fit interpretable model
        let coefficients = self.fit_linear_model(&self.samples);

        // Create explanation
        let mut explanation = Explanation::new(ExplanationType::LocalLinear, original_pred);
        explanation.base_value = self
            .samples
            .iter()
            .map(|s| s.prediction * s.weight)
            .sum::<f64>()
            / self
                .samples
                .iter()
                .map(|s| s.weight)
                .sum::<f64>()
                .max(1e-10);

        for (i, &coef) in coefficients.iter().enumerate() {
            let attr = FeatureAttribution::new(
                i as u32,
                coef * (input[i] - 0.0), // Attribution = coef * (value - baseline)
                input[i],
            );
            explanation.add_attribution(attr);
        }

        explanation.sort_by_importance();
        explanation.generate_summary();

        explanation
    }
}

// ============================================================================
// SHAP: SHAPLEY ADDITIVE EXPLANATIONS
// ============================================================================

/// Configuration for SHAP
#[derive(Debug, Clone)]
pub struct ShapConfig {
    /// Number of samples for Monte Carlo estimation
    pub num_samples: usize,
    /// Use fast kernel SHAP approximation
    pub use_kernel_shap: bool,
    /// Random seed
    pub seed: u64,
}

impl Default for ShapConfig {
    fn default() -> Self {
        Self {
            num_samples: 100,
            use_kernel_shap: true,
            seed: 42,
        }
    }
}

/// SHAP Explainer using Shapley values
pub struct ShapExplainer {
    /// Configuration
    config: ShapConfig,
    /// Background dataset for expectations
    background: Vec<Vec<f64>>,
    /// Precomputed coalition weights
    coalition_weights: BTreeMap<usize, f64>,
    /// Random state
    rng_state: u64,
}

impl ShapExplainer {
    /// Create a new SHAP explainer
    pub fn new(config: ShapConfig, background: Vec<Vec<f64>>) -> Self {
        Self {
            coalition_weights: BTreeMap::new(),
            config,
            background,
            rng_state: config.seed,
        }
    }

    /// Compute factorial
    fn factorial(n: usize) -> f64 {
        (1..=n).fold(1.0, |acc, x| acc * x as f64)
    }

    /// Compute Shapley kernel weight for a coalition of size |S|
    fn shapley_kernel_weight(n_features: usize, coalition_size: usize) -> f64 {
        if coalition_size == 0 || coalition_size == n_features {
            return 0.0; // Undefined, will use limit
        }

        let n = n_features as f64;
        let s = coalition_size as f64;

        (n - 1.0)
            / (Self::factorial(coalition_size)
                * Self::factorial(n_features - coalition_size - 1)
                * s
                * (n - s))
    }

    /// Generate random coalition mask
    fn random_coalition(&mut self, n_features: usize) -> Vec<bool> {
        let mut mask = vec![false; n_features];

        for i in 0..n_features {
            self.rng_state = lcg_next(self.rng_state);
            mask[i] = self.rng_state % 2 == 0;
        }

        mask
    }

    /// Evaluate model with coalition (replace absent features with background)
    fn evaluate_coalition<F>(&self, input: &[f64], coalition: &[bool], predict_fn: &F) -> f64
    where
        F: Fn(&[f64]) -> f64,
    {
        if self.background.is_empty() {
            // No background - use zeros for absent features
            let masked: Vec<f64> = input
                .iter()
                .zip(coalition.iter())
                .map(|(&v, &present)| if present { v } else { 0.0 })
                .collect();
            return predict_fn(&masked);
        }

        // Average over background samples
        let mut sum = 0.0;
        let count = self.background.len().min(10); // Limit for efficiency

        for bg in self.background.iter().take(count) {
            let masked: Vec<f64> = input
                .iter()
                .zip(coalition.iter())
                .zip(bg.iter())
                .map(|((&v, &present), &bg_v)| if present { v } else { bg_v })
                .collect();
            sum += predict_fn(&masked);
        }

        sum / count as f64
    }

    /// Compute SHAP values using Kernel SHAP
    pub fn explain<F>(&mut self, input: &[f64], predict_fn: F) -> Explanation
    where
        F: Fn(&[f64]) -> f64,
    {
        let n_features = input.len();
        let original_pred = predict_fn(input);

        // Baseline: all features absent
        let empty_coalition = vec![false; n_features];
        let base_value = self.evaluate_coalition(input, &empty_coalition, &predict_fn);

        // Monte Carlo estimation of Shapley values
        let mut shap_values = vec![0.0; n_features];
        let mut counts = vec![0usize; n_features];

        for _ in 0..self.config.num_samples {
            // Generate random permutation by repeated random swaps
            let mut perm: Vec<usize> = (0..n_features).collect();
            for i in (1..n_features).rev() {
                self.rng_state = lcg_next(self.rng_state);
                let j = self.rng_state as usize % (i + 1);
                perm.swap(i, j);
            }

            // Compute marginal contribution for each feature
            let mut coalition = vec![false; n_features];
            let mut prev_value = self.evaluate_coalition(input, &coalition, &predict_fn);

            for &feat_idx in &perm {
                coalition[feat_idx] = true;
                let new_value = self.evaluate_coalition(input, &coalition, &predict_fn);

                shap_values[feat_idx] += new_value - prev_value;
                counts[feat_idx] += 1;

                prev_value = new_value;
            }
        }

        // Average the contributions
        for i in 0..n_features {
            if counts[i] > 0 {
                shap_values[i] /= counts[i] as f64;
            }
        }

        // Create explanation
        let mut explanation = Explanation::new(ExplanationType::Shapley, original_pred);
        explanation.base_value = base_value;

        for (i, &shap_val) in shap_values.iter().enumerate() {
            let attr = FeatureAttribution::new(i as u32, shap_val, input[i]);
            explanation.add_attribution(attr);
        }

        explanation.sort_by_importance();
        explanation.generate_summary();

        explanation
    }
}

// ============================================================================
// ATTENTION VISUALIZATION
// ============================================================================

/// Attention head visualization
#[derive(Debug, Clone)]
pub struct AttentionHead {
    /// Head index
    pub head_id: usize,
    /// Layer index
    pub layer_id: usize,
    /// Attention weights (from_idx -> to_idx -> weight)
    pub weights: Vec<Vec<f64>>,
}

impl AttentionHead {
    /// Create a new attention head
    pub fn new(head_id: usize, layer_id: usize, seq_len: usize) -> Self {
        Self {
            head_id,
            layer_id,
            weights: vec![vec![0.0; seq_len]; seq_len],
        }
    }

    /// Set attention weight
    pub fn set_weight(&mut self, from: usize, to: usize, weight: f64) {
        if from < self.weights.len() && to < self.weights[from].len() {
            self.weights[from][to] = weight;
        }
    }

    /// Get attention from a position
    pub fn get_attention_from(&self, pos: usize) -> Option<&Vec<f64>> {
        self.weights.get(pos)
    }

    /// Get total attention received by a position
    pub fn attention_received(&self, pos: usize) -> f64 {
        self.weights
            .iter()
            .map(|row| row.get(pos).copied().unwrap_or(0.0))
            .sum()
    }

    /// Find the most attended positions from a given position
    pub fn top_attended(&self, from: usize, k: usize) -> Vec<(usize, f64)> {
        if from >= self.weights.len() {
            return Vec::new();
        }

        let mut indexed: Vec<(usize, f64)> = self.weights[from]
            .iter()
            .enumerate()
            .map(|(i, &w)| (i, w))
            .collect();

        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        indexed.truncate(k);
        indexed
    }
}

/// Attention visualization for a model
pub struct AttentionVisualizer {
    /// All attention heads
    pub heads: Vec<AttentionHead>,
    /// Input sequence/features
    pub input_tokens: Vec<u64>,
    /// Feature names
    pub feature_names: Vec<String>,
}

impl AttentionVisualizer {
    /// Create a new visualizer
    pub fn new(num_layers: usize, heads_per_layer: usize, seq_len: usize) -> Self {
        let mut heads = Vec::new();

        for layer in 0..num_layers {
            for head in 0..heads_per_layer {
                heads.push(AttentionHead::new(head, layer, seq_len));
            }
        }

        Self {
            heads,
            input_tokens: vec![0; seq_len],
            feature_names: Vec::new(),
        }
    }

    /// Get average attention across all heads for a position
    pub fn average_attention(&self, from: usize) -> Vec<f64> {
        if self.heads.is_empty() {
            return Vec::new();
        }

        let seq_len = self.heads[0].weights.len();
        let mut avg = vec![0.0; seq_len];

        for head in &self.heads {
            if let Some(weights) = head.get_attention_from(from) {
                for (i, &w) in weights.iter().enumerate() {
                    avg[i] += w;
                }
            }
        }

        let n_heads = self.heads.len() as f64;
        for v in &mut avg {
            *v /= n_heads;
        }

        avg
    }

    /// Get attention flow to output
    pub fn attention_to_output(&self) -> Vec<f64> {
        if self.heads.is_empty() {
            return Vec::new();
        }

        let seq_len = self.heads[0].weights.len();
        let mut importance = vec![0.0; seq_len];

        // Sum attention received by each position
        for head in &self.heads {
            for pos in 0..seq_len {
                importance[pos] += head.attention_received(pos);
            }
        }

        // Normalize
        let sum: f64 = importance.iter().sum();
        if sum > 0.0 {
            for v in &mut importance {
                *v /= sum;
            }
        }

        importance
    }

    /// Generate explanation from attention
    pub fn to_explanation(&self, prediction: f64) -> Explanation {
        let importance = self.attention_to_output();

        let mut explanation = Explanation::new(ExplanationType::Attention, prediction);

        for (i, &imp) in importance.iter().enumerate() {
            let name = self.feature_names.get(i).cloned();
            let mut attr = FeatureAttribution::new(i as u32, imp, imp);
            if let Some(n) = name {
                attr = attr.with_name(n);
            }
            explanation.add_attribution(attr);
        }

        explanation.sort_by_importance();
        explanation
    }
}

// ============================================================================
// COUNTERFACTUAL EXPLANATIONS
// ============================================================================

/// A counterfactual example
#[derive(Debug, Clone)]
pub struct Counterfactual {
    /// Original input
    pub original: Vec<f64>,
    /// Counterfactual input
    pub counterfactual: Vec<f64>,
    /// Original prediction
    pub original_pred: f64,
    /// Counterfactual prediction
    pub counterfactual_pred: f64,
    /// Features that changed
    pub changed_features: Vec<(u32, f64, f64)>, // (index, from, to)
    /// Distance from original
    pub distance: f64,
    /// Validity (prediction changed sufficiently)
    pub is_valid: bool,
}

impl Counterfactual {
    /// Create a new counterfactual
    pub fn new(original: Vec<f64>, counterfactual: Vec<f64>, orig_pred: f64, cf_pred: f64) -> Self {
        let mut changed = Vec::new();
        let mut dist = 0.0;

        for (i, (o, c)) in original.iter().zip(counterfactual.iter()).enumerate() {
            let diff = c - o;
            if diff.abs() > 1e-6 {
                changed.push((i as u32, *o, *c));
                dist += diff * diff;
            }
        }

        Self {
            original,
            counterfactual,
            original_pred: orig_pred,
            counterfactual_pred: cf_pred,
            changed_features: changed,
            distance: libm::sqrt(dist),
            is_valid: (orig_pred - cf_pred).abs() > 0.1,
        }
    }

    /// Get the most important change
    pub fn primary_change(&self) -> Option<(u32, f64, f64)> {
        self.changed_features
            .iter()
            .max_by(|a, b| (a.2 - a.1).abs().partial_cmp(&(b.2 - b.1).abs()).unwrap())
            .copied()
    }

    /// Get number of changes
    pub fn num_changes(&self) -> usize {
        self.changed_features.len()
    }
}

/// Configuration for counterfactual search
#[derive(Debug, Clone)]
pub struct CounterfactualConfig {
    /// Maximum number of features to change
    pub max_changes: usize,
    /// Step size for gradient-based search
    pub step_size: f64,
    /// Maximum iterations
    pub max_iterations: usize,
    /// Target prediction change
    pub target_change: f64,
    /// Random seed
    pub seed: u64,
}

impl Default for CounterfactualConfig {
    fn default() -> Self {
        Self {
            max_changes: 5,
            step_size: 0.1,
            max_iterations: 100,
            target_change: 0.5,
            seed: 42,
        }
    }
}

/// Counterfactual explanation generator
pub struct CounterfactualGenerator {
    /// Configuration
    config: CounterfactualConfig,
    /// Feature constraints (min, max)
    constraints: Vec<(f64, f64)>,
    /// Random state
    rng_state: u64,
}

impl CounterfactualGenerator {
    /// Create a new generator
    pub fn new(config: CounterfactualConfig) -> Self {
        Self {
            rng_state: config.seed,
            config,
            constraints: Vec::new(),
        }
    }

    /// Set feature constraints
    pub fn set_constraints(&mut self, constraints: Vec<(f64, f64)>) {
        self.constraints = constraints;
    }

    /// Generate counterfactual using gradient-based search
    pub fn generate<F>(
        &mut self,
        input: &[f64],
        predict_fn: F,
        direction: f64,
    ) -> Option<Counterfactual>
    where
        F: Fn(&[f64]) -> f64,
    {
        let original_pred = predict_fn(input);
        let target_pred = original_pred + direction * self.config.target_change;

        let mut cf = input.to_vec();
        let n_features = input.len();

        // Extend constraints if needed
        while self.constraints.len() < n_features {
            self.constraints.push((f64::NEG_INFINITY, f64::INFINITY));
        }

        for _ in 0..self.config.max_iterations {
            let current_pred = predict_fn(&cf);

            // Check if we've reached target
            if (direction > 0.0 && current_pred >= target_pred)
                || (direction < 0.0 && current_pred <= target_pred)
            {
                return Some(Counterfactual::new(
                    input.to_vec(),
                    cf,
                    original_pred,
                    current_pred,
                ));
            }

            // Estimate gradient numerically
            let mut gradient = vec![0.0; n_features];
            let eps = 0.01;

            for i in 0..n_features {
                let mut cf_plus = cf.clone();
                cf_plus[i] += eps;
                let pred_plus = predict_fn(&cf_plus);
                gradient[i] = (pred_plus - current_pred) / eps;
            }

            // Select features to modify (sparsity)
            let mut indices: Vec<usize> = (0..n_features).collect();
            indices.sort_by(|&a, &b| gradient[b].abs().partial_cmp(&gradient[a].abs()).unwrap());

            // Update only top features
            for &i in indices.iter().take(self.config.max_changes) {
                let update = direction * gradient[i].signum() * self.config.step_size;
                cf[i] += update;

                // Apply constraints
                let (min_val, max_val) = self.constraints[i];
                cf[i] = cf[i].clamp(min_val, max_val);
            }
        }

        // Return best found even if not reached target
        let final_pred = predict_fn(&cf);
        Some(Counterfactual::new(
            input.to_vec(),
            cf,
            original_pred,
            final_pred,
        ))
    }

    /// Generate diverse counterfactuals
    pub fn generate_diverse<F>(
        &mut self,
        input: &[f64],
        predict_fn: F,
        count: usize,
    ) -> Vec<Counterfactual>
    where
        F: Fn(&[f64]) -> f64 + Copy,
    {
        let mut counterfactuals = Vec::new();

        // Try both directions
        for direction in [-1.0, 1.0].iter() {
            for _ in 0..count / 2 {
                self.rng_state = lcg_next(self.rng_state);
                let mut config = self.config.clone();
                config.seed = self.rng_state;

                let mut generator = CounterfactualGenerator::new(config);
                generator.constraints = self.constraints.clone();

                if let Some(cf) = generator.generate(input, predict_fn, *direction) {
                    if cf.is_valid {
                        counterfactuals.push(cf);
                    }
                }
            }
        }

        // Sort by distance (prefer minimal changes)
        counterfactuals.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
        counterfactuals.truncate(count);

        counterfactuals
    }
}

// ============================================================================
// RULE EXTRACTION
// ============================================================================

/// A condition in a rule
#[derive(Debug, Clone)]
pub struct RuleCondition {
    /// Feature index
    pub feature_id: u32,
    /// Operator type
    pub operator: ComparisonOp,
    /// Threshold value
    pub threshold: f64,
    /// Feature name (optional)
    pub feature_name: Option<String>,
}

/// Comparison operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOp {
    LessThan,
    LessEqual,
    GreaterThan,
    GreaterEqual,
    Equal,
    NotEqual,
}

impl RuleCondition {
    /// Create a new condition
    pub fn new(feature_id: u32, operator: ComparisonOp, threshold: f64) -> Self {
        Self {
            feature_id,
            operator,
            threshold,
            feature_name: None,
        }
    }

    /// Check if condition is satisfied
    pub fn evaluate(&self, value: f64) -> bool {
        match self.operator {
            ComparisonOp::LessThan => value < self.threshold,
            ComparisonOp::LessEqual => value <= self.threshold,
            ComparisonOp::GreaterThan => value > self.threshold,
            ComparisonOp::GreaterEqual => value >= self.threshold,
            ComparisonOp::Equal => (value - self.threshold).abs() < 1e-10,
            ComparisonOp::NotEqual => (value - self.threshold).abs() >= 1e-10,
        }
    }

    /// Format as string
    pub fn to_string(&self) -> String {
        let feature = self
            .feature_name
            .clone()
            .unwrap_or_else(|| alloc::format!("x{}", self.feature_id));

        let op = match self.operator {
            ComparisonOp::LessThan => "<",
            ComparisonOp::LessEqual => "<=",
            ComparisonOp::GreaterThan => ">",
            ComparisonOp::GreaterEqual => ">=",
            ComparisonOp::Equal => "==",
            ComparisonOp::NotEqual => "!=",
        };

        alloc::format!("{} {} {:.3}", feature, op, self.threshold)
    }
}

/// A decision rule
#[derive(Debug, Clone)]
pub struct Rule {
    /// Conditions (AND)
    pub conditions: Vec<RuleCondition>,
    /// Predicted class/value when rule fires
    pub prediction: f64,
    /// Confidence/support
    pub confidence: f64,
    /// Coverage (fraction of data covered)
    pub coverage: f64,
}

impl Rule {
    /// Create a new rule
    pub fn new(conditions: Vec<RuleCondition>, prediction: f64) -> Self {
        Self {
            conditions,
            prediction,
            confidence: 1.0,
            coverage: 0.0,
        }
    }

    /// Check if rule fires on input
    pub fn fires(&self, input: &[f64]) -> bool {
        self.conditions.iter().all(|c| {
            input
                .get(c.feature_id as usize)
                .map(|&v| c.evaluate(v))
                .unwrap_or(false)
        })
    }

    /// Format rule as string
    pub fn to_string(&self) -> String {
        if self.conditions.is_empty() {
            return alloc::format!("IF true THEN {:.3}", self.prediction);
        }

        let conditions: Vec<String> = self.conditions.iter().map(|c| c.to_string()).collect();

        alloc::format!(
            "IF {} THEN {:.3} (conf: {:.2}, cov: {:.2})",
            conditions.join(" AND "),
            self.prediction,
            self.confidence,
            self.coverage
        )
    }
}

/// Rule extractor from decision trees
pub struct RuleExtractor {
    /// Extracted rules
    pub rules: Vec<Rule>,
    /// Maximum rule length
    pub max_depth: usize,
    /// Minimum confidence
    pub min_confidence: f64,
    /// Minimum coverage
    pub min_coverage: f64,
}

impl RuleExtractor {
    /// Create a new rule extractor
    pub fn new(max_depth: usize, min_confidence: f64, min_coverage: f64) -> Self {
        Self {
            rules: Vec::new(),
            max_depth,
            min_confidence,
            min_coverage,
        }
    }

    /// Extract rules from a decision tree
    pub fn extract_from_tree(&mut self, tree: &DecisionTree, feature_names: &[String]) {
        self.rules.clear();
        self.extract_recursive(tree, 0, Vec::new(), feature_names);
    }

    fn extract_recursive(
        &mut self,
        tree: &DecisionTree,
        node: usize,
        mut conditions: Vec<RuleCondition>,
        feature_names: &[String],
    ) {
        if conditions.len() > self.max_depth {
            return;
        }

        if let Some(leaf) = tree.get_leaf(node) {
            if leaf.confidence >= self.min_confidence && leaf.coverage >= self.min_coverage {
                let mut rule = Rule::new(conditions, leaf.prediction);
                rule.confidence = leaf.confidence;
                rule.coverage = leaf.coverage;
                self.rules.push(rule);
            }
            return;
        }

        if let Some(split) = tree.get_split(node) {
            // Left branch (< threshold)
            let mut left_cond = RuleCondition::new(
                split.feature as u32,
                ComparisonOp::LessThan,
                split.threshold,
            );
            if split.feature < feature_names.len() {
                left_cond.feature_name = Some(feature_names[split.feature].clone());
            }
            let mut left_conditions = conditions.clone();
            left_conditions.push(left_cond);
            self.extract_recursive(tree, split.left_child, left_conditions, feature_names);

            // Right branch (>= threshold)
            let mut right_cond = RuleCondition::new(
                split.feature as u32,
                ComparisonOp::GreaterEqual,
                split.threshold,
            );
            if split.feature < feature_names.len() {
                right_cond.feature_name = Some(feature_names[split.feature].clone());
            }
            conditions.push(right_cond);
            self.extract_recursive(tree, split.right_child, conditions, feature_names);
        }
    }

    /// Simplify rules by removing redundant conditions
    pub fn simplify_rules(&mut self) {
        for rule in &mut self.rules {
            // Sort conditions by feature
            rule.conditions.sort_by_key(|c| c.feature_id);

            // Merge overlapping conditions on same feature
            let mut simplified = Vec::new();
            let mut prev: Option<RuleCondition> = None;

            for cond in &rule.conditions {
                if let Some(ref p) = prev {
                    if p.feature_id == cond.feature_id {
                        // Try to merge
                        // For now, keep the more restrictive one
                        match (p.operator, cond.operator) {
                            (ComparisonOp::GreaterEqual, ComparisonOp::GreaterEqual)
                            | (ComparisonOp::GreaterThan, ComparisonOp::GreaterThan) => {
                                if cond.threshold > p.threshold {
                                    prev = Some(cond.clone());
                                }
                            },
                            (ComparisonOp::LessEqual, ComparisonOp::LessEqual)
                            | (ComparisonOp::LessThan, ComparisonOp::LessThan) => {
                                if cond.threshold < p.threshold {
                                    prev = Some(cond.clone());
                                }
                            },
                            _ => {
                                simplified.push(prev.take().unwrap());
                                prev = Some(cond.clone());
                            },
                        }
                    } else {
                        simplified.push(prev.take().unwrap());
                        prev = Some(cond.clone());
                    }
                } else {
                    prev = Some(cond.clone());
                }
            }

            if let Some(p) = prev {
                simplified.push(p);
            }

            rule.conditions = simplified;
        }
    }

    /// Get rules sorted by confidence
    pub fn get_rules_by_confidence(&self) -> Vec<&Rule> {
        let mut rules: Vec<&Rule> = self.rules.iter().collect();
        rules.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        rules
    }
}

/// Simple decision tree structure for rule extraction
pub struct DecisionTree {
    /// Split nodes: (feature, threshold, left_child, right_child)
    splits: Vec<Option<TreeSplit>>,
    /// Leaf nodes
    leaves: Vec<Option<TreeLeaf>>,
}

#[derive(Debug, Clone)]
pub struct TreeSplit {
    pub feature: usize,
    pub threshold: f64,
    pub left_child: usize,
    pub right_child: usize,
}

#[derive(Debug, Clone)]
pub struct TreeLeaf {
    pub prediction: f64,
    pub confidence: f64,
    pub coverage: f64,
}

impl DecisionTree {
    /// Create a new tree with capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            splits: vec![None; capacity],
            leaves: vec![None; capacity],
        }
    }

    /// Add a split node
    pub fn add_split(&mut self, node: usize, split: TreeSplit) {
        if node < self.splits.len() {
            self.splits[node] = Some(split);
        }
    }

    /// Add a leaf node
    pub fn add_leaf(&mut self, node: usize, leaf: TreeLeaf) {
        if node < self.leaves.len() {
            self.leaves[node] = Some(leaf);
        }
    }

    /// Get split node
    pub fn get_split(&self, node: usize) -> Option<&TreeSplit> {
        self.splits.get(node).and_then(|s| s.as_ref())
    }

    /// Get leaf node
    pub fn get_leaf(&self, node: usize) -> Option<&TreeLeaf> {
        self.leaves.get(node).and_then(|l| l.as_ref())
    }

    /// Predict for input
    pub fn predict(&self, input: &[f64]) -> Option<f64> {
        let mut node = 0;

        loop {
            if let Some(leaf) = self.get_leaf(node) {
                return Some(leaf.prediction);
            }

            if let Some(split) = self.get_split(node) {
                let value = input.get(split.feature).copied().unwrap_or(0.0);
                node = if value < split.threshold {
                    split.left_child
                } else {
                    split.right_child
                };
            } else {
                return None;
            }
        }
    }
}

// ============================================================================
// CONCEPT ACTIVATION VECTORS
// ============================================================================

/// A concept defined by example activations
#[derive(Debug, Clone)]
pub struct Concept {
    /// Concept name
    pub name: String,
    /// Direction vector in activation space
    pub direction: Vec<f64>,
    /// Positive examples
    pub positive_examples: Vec<Vec<f64>>,
    /// Negative examples
    pub negative_examples: Vec<Vec<f64>>,
    /// Accuracy of the concept classifier
    pub accuracy: f64,
}

impl Concept {
    /// Create a new concept
    pub fn new(name: String, activation_size: usize) -> Self {
        Self {
            name,
            direction: vec![0.0; activation_size],
            positive_examples: Vec::new(),
            negative_examples: Vec::new(),
            accuracy: 0.0,
        }
    }

    /// Add positive example
    pub fn add_positive(&mut self, activation: Vec<f64>) {
        self.positive_examples.push(activation);
    }

    /// Add negative example
    pub fn add_negative(&mut self, activation: Vec<f64>) {
        self.negative_examples.push(activation);
    }

    /// Train the concept direction (simple linear classifier)
    pub fn train(&mut self) {
        if self.positive_examples.is_empty() || self.negative_examples.is_empty() {
            return;
        }

        // Compute mean of positive and negative examples
        let dim = self.positive_examples[0].len();
        let mut pos_mean = vec![0.0; dim];
        let mut neg_mean = vec![0.0; dim];

        for ex in &self.positive_examples {
            for (i, &v) in ex.iter().enumerate() {
                pos_mean[i] += v;
            }
        }
        for v in &mut pos_mean {
            *v /= self.positive_examples.len() as f64;
        }

        for ex in &self.negative_examples {
            for (i, &v) in ex.iter().enumerate() {
                neg_mean[i] += v;
            }
        }
        for v in &mut neg_mean {
            *v /= self.negative_examples.len() as f64;
        }

        // Direction: positive - negative
        self.direction = pos_mean
            .iter()
            .zip(neg_mean.iter())
            .map(|(p, n)| p - n)
            .collect();

        // Normalize
        let norm: f64 = self.direction.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm > 1e-10 {
            for v in &mut self.direction {
                *v /= norm;
            }
        }

        // Calculate accuracy
        let mut correct = 0;
        let total = self.positive_examples.len() + self.negative_examples.len();

        for ex in &self.positive_examples {
            if self.score(ex) > 0.0 {
                correct += 1;
            }
        }
        for ex in &self.negative_examples {
            if self.score(ex) <= 0.0 {
                correct += 1;
            }
        }

        self.accuracy = correct as f64 / total as f64;
    }

    /// Score an activation (dot product with direction)
    pub fn score(&self, activation: &[f64]) -> f64 {
        self.direction
            .iter()
            .zip(activation.iter())
            .map(|(d, a)| d * a)
            .sum()
    }
}

/// Concept Activation Vector (CAV) Explainer
pub struct CavExplainer {
    /// Trained concepts
    pub concepts: Vec<Concept>,
    /// Layer to analyze
    pub layer: usize,
}

impl CavExplainer {
    /// Create a new CAV explainer
    pub fn new(layer: usize) -> Self {
        Self {
            concepts: Vec::new(),
            layer,
        }
    }

    /// Add a concept
    pub fn add_concept(&mut self, concept: Concept) {
        self.concepts.push(concept);
    }

    /// Compute TCAV score (Testing with CAVs)
    pub fn tcav_score<F>(&self, concept_idx: usize, inputs: &[Vec<f64>], get_gradients: F) -> f64
    where
        F: Fn(&[f64]) -> Vec<f64>,
    {
        if concept_idx >= self.concepts.len() || inputs.is_empty() {
            return 0.0;
        }

        let concept = &self.concepts[concept_idx];
        let mut positive_count = 0;

        for input in inputs {
            let gradient = get_gradients(input);

            // Directional derivative: dot(gradient, CAV direction)
            let directional: f64 = gradient
                .iter()
                .zip(concept.direction.iter())
                .map(|(g, d)| g * d)
                .sum();

            if directional > 0.0 {
                positive_count += 1;
            }
        }

        positive_count as f64 / inputs.len() as f64
    }

    /// Explain which concepts are activated
    pub fn explain_concepts(&self, activation: &[f64]) -> Vec<(String, f64)> {
        let mut scores: Vec<(String, f64)> = self
            .concepts
            .iter()
            .map(|c| (c.name.clone(), c.score(activation)))
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scores
    }
}

// ============================================================================
// INTEGRATED XAI MANAGER
// ============================================================================

/// Types of kernel events to explain
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelEventType {
    ProcessScheduled,
    ProcessKilled,
    MemoryAllocated,
    MemoryDeallocated,
    InterruptHandled,
    SyscallCompleted,
    AnomalyDetected,
    SecurityBlocked,
    ResourceLimitHit,
}

/// An explained kernel decision
#[derive(Debug, Clone)]
pub struct ExplainedDecision {
    /// Event type
    pub event_type: KernelEventType,
    /// Decision made
    pub decision: f64,
    /// Feature values
    pub features: Vec<(String, f64)>,
    /// Explanations (multiple methods)
    pub explanations: Vec<Explanation>,
    /// Counterfactuals
    pub counterfactuals: Vec<Counterfactual>,
    /// Applied rules
    pub rules_applied: Vec<Rule>,
    /// Timestamp
    pub timestamp: u64,
}

impl ExplainedDecision {
    /// Create a new explained decision
    pub fn new(event_type: KernelEventType, decision: f64) -> Self {
        Self {
            event_type,
            features: Vec::new(),
            explanations: Vec::new(),
            counterfactuals: Vec::new(),
            rules_applied: Vec::new(),
            timestamp: 0,
            decision,
        }
    }

    /// Get primary explanation
    pub fn primary_explanation(&self) -> Option<&Explanation> {
        self.explanations.first()
    }

    /// Get top contributing features
    pub fn top_features(&self, k: usize) -> Vec<&FeatureAttribution> {
        self.explanations
            .first()
            .map(|e| e.top_k_features(k))
            .unwrap_or_default()
    }

    /// Get minimal counterfactual
    pub fn minimal_counterfactual(&self) -> Option<&Counterfactual> {
        self.counterfactuals.first()
    }

    /// Generate human-readable summary
    pub fn summarize(&self) -> String {
        let mut parts = Vec::new();

        parts.push(alloc::format!("Event: {:?}", self.event_type));
        parts.push(alloc::format!("Decision: {:.4}", self.decision));

        if let Some(expl) = self.primary_explanation() {
            if let Some(ref summary) = expl.summary {
                parts.push(summary.clone());
            }
        }

        if let Some(cf) = self.minimal_counterfactual() {
            if let Some((feat, from, to)) = cf.primary_change() {
                let feat_name = self
                    .features
                    .get(feat as usize)
                    .map(|(n, _)| n.as_str())
                    .unwrap_or("?");
                parts.push(alloc::format!(
                    "To change: set {} from {:.2} to {:.2}",
                    feat_name,
                    from,
                    to
                ));
            }
        }

        parts.join(" | ")
    }
}

/// Main XAI Manager for kernel
pub struct KernelXaiManager {
    /// LIME explainer
    pub lime: LimeExplainer,
    /// SHAP explainer
    pub shap: Option<ShapExplainer>,
    /// Attention visualizer
    pub attention: Option<AttentionVisualizer>,
    /// Counterfactual generator
    pub counterfactual: CounterfactualGenerator,
    /// Rule extractor
    pub rule_extractor: RuleExtractor,
    /// CAV explainer
    pub cav: Option<CavExplainer>,
    /// History of explained decisions
    pub history: Vec<ExplainedDecision>,
    /// Maximum history size
    pub max_history: usize,
    /// Feature names
    pub feature_names: Vec<String>,
}

impl KernelXaiManager {
    /// Create a new XAI manager
    pub fn new(lime_config: LimeConfig) -> Self {
        Self {
            lime: LimeExplainer::new(lime_config),
            shap: None,
            attention: None,
            counterfactual: CounterfactualGenerator::new(CounterfactualConfig::default()),
            rule_extractor: RuleExtractor::new(10, 0.5, 0.01),
            cav: None,
            history: Vec::new(),
            max_history: 1000,
            feature_names: Vec::new(),
        }
    }

    /// Set feature names
    pub fn set_feature_names(&mut self, names: Vec<String>) {
        self.feature_names = names;
    }

    /// Initialize SHAP with background data
    pub fn init_shap(&mut self, background: Vec<Vec<f64>>) {
        self.shap = Some(ShapExplainer::new(ShapConfig::default(), background));
    }

    /// Initialize CAV explainer
    pub fn init_cav(&mut self, layer: usize) {
        self.cav = Some(CavExplainer::new(layer));
    }

    /// Add a concept for CAV
    pub fn add_concept(&mut self, concept: Concept) {
        if let Some(ref mut cav) = self.cav {
            cav.add_concept(concept);
        }
    }

    /// Explain a kernel decision
    pub fn explain<F>(
        &mut self,
        event_type: KernelEventType,
        input: &[f64],
        predict_fn: F,
    ) -> ExplainedDecision
    where
        F: Fn(&[f64]) -> f64 + Copy,
    {
        let decision = predict_fn(input);
        let mut explained = ExplainedDecision::new(event_type, decision);

        // Add feature values
        for (i, &val) in input.iter().enumerate() {
            let name = self
                .feature_names
                .get(i)
                .cloned()
                .unwrap_or_else(|| alloc::format!("feature_{}", i));
            explained.features.push((name, val));
        }

        // LIME explanation
        let lime_expl = self.lime.explain(input, predict_fn);
        explained.explanations.push(lime_expl);

        // SHAP explanation if available
        if let Some(ref mut shap) = self.shap {
            let shap_expl = shap.explain(input, predict_fn);
            explained.explanations.push(shap_expl);
        }

        // Generate counterfactuals
        let cfs = self.counterfactual.generate_diverse(input, predict_fn, 3);
        explained.counterfactuals = cfs;

        // Store in history
        self.history.push(explained.clone());
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }

        explained
    }

    /// Get explanation statistics
    pub fn get_stats(&self) -> XaiStats {
        XaiStats {
            total_explanations: self.history.len(),
            by_event_type: self.count_by_event_type(),
            avg_features_used: self.average_features_used(),
            counterfactual_success_rate: self.counterfactual_success_rate(),
        }
    }

    fn count_by_event_type(&self) -> BTreeMap<u8, usize> {
        let mut counts = BTreeMap::new();
        for expl in &self.history {
            *counts.entry(expl.event_type as u8).or_insert(0) += 1;
        }
        counts
    }

    fn average_features_used(&self) -> f64 {
        if self.history.is_empty() {
            return 0.0;
        }

        let total: usize = self
            .history
            .iter()
            .flat_map(|e| e.explanations.first())
            .map(|e| {
                e.attributions
                    .iter()
                    .filter(|a| a.importance() > 0.01)
                    .count()
            })
            .sum();

        total as f64 / self.history.len() as f64
    }

    fn counterfactual_success_rate(&self) -> f64 {
        if self.history.is_empty() {
            return 0.0;
        }

        let valid: usize = self
            .history
            .iter()
            .flat_map(|e| e.counterfactuals.first())
            .filter(|cf| cf.is_valid)
            .count();

        let total: usize = self
            .history
            .iter()
            .filter(|e| !e.counterfactuals.is_empty())
            .count();

        if total == 0 {
            0.0
        } else {
            valid as f64 / total as f64
        }
    }
}

/// XAI statistics
#[derive(Debug, Clone)]
pub struct XaiStats {
    pub total_explanations: usize,
    pub by_event_type: BTreeMap<u8, usize>,
    pub avg_features_used: f64,
    pub counterfactual_success_rate: f64,
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

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_attribution() {
        let attr = FeatureAttribution::new(0, 0.5, 1.0);
        assert!(attr.supports_prediction());
        assert!((attr.importance() - 0.5).abs() < 1e-10);

        let attr_neg = FeatureAttribution::new(1, -0.3, 0.5);
        assert!(!attr_neg.supports_prediction());
    }

    #[test]
    fn test_explanation() {
        let mut expl = Explanation::new(ExplanationType::FeatureImportance, 0.8);
        expl.base_value = 0.5;

        expl.add_attribution(FeatureAttribution::new(0, 0.2, 1.0));
        expl.add_attribution(FeatureAttribution::new(1, 0.1, 0.5));

        expl.sort_by_importance();
        assert_eq!(expl.attributions[0].feature_id, 0);

        let top = expl.top_k_features(1);
        assert_eq!(top.len(), 1);
    }

    #[test]
    fn test_lime_explainer() {
        let config = LimeConfig {
            num_samples: 100,
            num_features: 3,
            ..Default::default()
        };

        let mut lime = LimeExplainer::new(config);

        // Simple linear model
        let predict = |x: &[f64]| x[0] * 0.5 + x[1] * 0.3 + x[2] * 0.2;

        let input = vec![1.0, 2.0, 3.0];
        let expl = lime.explain(&input, predict);

        assert_eq!(expl.attributions.len(), 3);
    }

    #[test]
    fn test_shap_explainer() {
        let background = vec![vec![0.0, 0.0, 0.0]];
        let config = ShapConfig {
            num_samples: 20,
            ..Default::default()
        };

        let mut shap = ShapExplainer::new(config, background);

        let predict = |x: &[f64]| x[0] + x[1] * 2.0;
        let input = vec![1.0, 1.0, 0.0];

        let expl = shap.explain(&input, predict);

        assert_eq!(expl.attributions.len(), 3);
    }

    #[test]
    fn test_attention_head() {
        let mut head = AttentionHead::new(0, 0, 4);

        head.set_weight(0, 1, 0.5);
        head.set_weight(0, 2, 0.3);
        head.set_weight(0, 3, 0.2);

        let top = head.top_attended(0, 2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, 1);
        assert!((top[0].1 - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_counterfactual() {
        let original = vec![1.0, 2.0, 3.0];
        let cf = vec![1.0, 3.0, 3.0];

        let counterfactual = Counterfactual::new(original, cf, 0.3, 0.7);

        assert_eq!(counterfactual.num_changes(), 1);
        assert!(counterfactual.is_valid);

        let primary = counterfactual.primary_change().unwrap();
        assert_eq!(primary.0, 1); // Feature 1 changed
    }

    #[test]
    fn test_counterfactual_generator() {
        let config = CounterfactualConfig {
            max_iterations: 50,
            ..Default::default()
        };

        let mut generator = CounterfactualGenerator::new(config);

        // Simple threshold model
        let predict = |x: &[f64]| if x[0] > 0.5 { 1.0 } else { 0.0 };

        let input = vec![0.3, 0.5];
        let cf = generator.generate(&input, predict, 1.0);

        assert!(cf.is_some());
    }

    #[test]
    fn test_rule_condition() {
        let cond = RuleCondition::new(0, ComparisonOp::GreaterThan, 0.5);

        assert!(cond.evaluate(0.6));
        assert!(!cond.evaluate(0.4));
    }

    #[test]
    fn test_rule() {
        let conditions = vec![
            RuleCondition::new(0, ComparisonOp::GreaterThan, 0.5),
            RuleCondition::new(1, ComparisonOp::LessThan, 0.3),
        ];

        let rule = Rule::new(conditions, 1.0);

        assert!(rule.fires(&[0.6, 0.2]));
        assert!(!rule.fires(&[0.4, 0.2]));
        assert!(!rule.fires(&[0.6, 0.4]));
    }

    #[test]
    fn test_decision_tree() {
        let mut tree = DecisionTree::new(10);

        tree.add_split(0, TreeSplit {
            feature: 0,
            threshold: 0.5,
            left_child: 1,
            right_child: 2,
        });

        tree.add_leaf(1, TreeLeaf {
            prediction: 0.0,
            confidence: 0.9,
            coverage: 0.5,
        });

        tree.add_leaf(2, TreeLeaf {
            prediction: 1.0,
            confidence: 0.95,
            coverage: 0.5,
        });

        assert_eq!(tree.predict(&[0.3]), Some(0.0));
        assert_eq!(tree.predict(&[0.7]), Some(1.0));
    }

    #[test]
    fn test_concept() {
        let mut concept = Concept::new(alloc::string::String::from("high_value"), 3);

        concept.add_positive(vec![1.0, 0.0, 0.0]);
        concept.add_positive(vec![0.9, 0.1, 0.0]);
        concept.add_negative(vec![0.0, 1.0, 0.0]);
        concept.add_negative(vec![0.1, 0.9, 0.0]);

        concept.train();

        assert!(concept.accuracy >= 0.5);
        assert!(concept.score(&[1.0, 0.0, 0.0]) > concept.score(&[0.0, 1.0, 0.0]));
    }

    #[test]
    fn test_explained_decision() {
        let mut decision = ExplainedDecision::new(KernelEventType::ProcessScheduled, 0.8);
        decision
            .features
            .push((alloc::string::String::from("priority"), 5.0));
        decision
            .features
            .push((alloc::string::String::from("cpu_usage"), 0.7));

        let mut expl = Explanation::new(ExplanationType::FeatureImportance, 0.8);
        expl.add_attribution(FeatureAttribution::new(0, 0.5, 5.0));
        decision.explanations.push(expl);

        assert!(decision.primary_explanation().is_some());
        let summary = decision.summarize();
        assert!(!summary.is_empty());
    }

    #[test]
    fn test_kernel_xai_manager() {
        let config = LimeConfig {
            num_samples: 50,
            ..Default::default()
        };

        let mut manager = KernelXaiManager::new(config);
        manager.set_feature_names(vec![
            alloc::string::String::from("cpu"),
            alloc::string::String::from("memory"),
        ]);

        let predict = |x: &[f64]| x[0] * 0.7 + x[1] * 0.3;

        let decision = manager.explain(KernelEventType::ProcessScheduled, &[0.8, 0.5], predict);

        assert_eq!(decision.features.len(), 2);
        assert!(!decision.explanations.is_empty());

        let stats = manager.get_stats();
        assert_eq!(stats.total_explanations, 1);
    }
}
