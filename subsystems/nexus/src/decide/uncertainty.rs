//! # Uncertainty Handling
//!
//! Manages uncertainty in decision-making.
//! Implements uncertainty quantification and propagation.
//!
//! Part of Year 2 COGNITION - Q2: Causal Reasoning

#![allow(dead_code)]

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// UNCERTAINTY TYPES
// ============================================================================

/// Uncertain value
#[derive(Debug, Clone)]
pub struct UncertainValue {
    /// Value ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Central estimate
    pub estimate: f64,
    /// Uncertainty
    pub uncertainty: Uncertainty,
    /// Sources
    pub sources: Vec<UncertaintySource>,
}

/// Uncertainty representation
#[derive(Debug, Clone)]
pub enum Uncertainty {
    /// Standard deviation
    StdDev(f64),
    /// Confidence interval
    ConfidenceInterval {
        low: f64,
        high: f64,
        confidence: f64,
    },
    /// Probability distribution
    Distribution(Distribution),
    /// Credal set (imprecise probability)
    CredalSet { lower: f64, upper: f64 },
}

/// Distribution
#[derive(Debug, Clone)]
pub enum Distribution {
    Normal { mean: f64, std: f64 },
    Uniform { min: f64, max: f64 },
    Triangular { min: f64, mode: f64, max: f64 },
    Beta { alpha: f64, beta: f64 },
    Discrete { values: Vec<(f64, f64)> }, // (value, probability)
}

/// Uncertainty source
#[derive(Debug, Clone)]
pub struct UncertaintySource {
    /// Source type
    pub source_type: SourceType,
    /// Contribution
    pub contribution: f64,
    /// Description
    pub description: String,
}

/// Source type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceType {
    Measurement,
    Model,
    Parameter,
    Input,
    Epistemic,
    Aleatoric,
}

/// Propagation result
#[derive(Debug, Clone)]
pub struct PropagationResult {
    /// Output value
    pub output: UncertainValue,
    /// Sensitivity
    pub sensitivity: LinearMap<f64, 64>,
    /// Contribution
    pub contribution: LinearMap<f64, 64>,
}

/// Sensitivity info
#[derive(Debug, Clone)]
pub struct SensitivityInfo {
    /// Input ID
    pub input_id: u64,
    /// Sensitivity index
    pub sensitivity: f64,
    /// Contribution to variance
    pub variance_contribution: f64,
}

// ============================================================================
// UNCERTAINTY ENGINE
// ============================================================================

/// Uncertainty engine
pub struct UncertaintyEngine {
    /// Values
    values: BTreeMap<u64, UncertainValue>,
    /// Computed results
    results: BTreeMap<u64, PropagationResult>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: UncertaintyConfig,
    /// Statistics
    stats: UncertaintyStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct UncertaintyConfig {
    /// Monte Carlo samples
    pub monte_carlo_samples: usize,
    /// Confidence level
    pub confidence_level: f64,
    /// Epsilon for sensitivity
    pub sensitivity_epsilon: f64,
}

impl Default for UncertaintyConfig {
    fn default() -> Self {
        Self {
            monte_carlo_samples: 1000,
            confidence_level: 0.95,
            sensitivity_epsilon: 0.01,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct UncertaintyStats {
    /// Values created
    pub values_created: u64,
    /// Propagations performed
    pub propagations: u64,
    /// Sensitivities computed
    pub sensitivities_computed: u64,
}

impl UncertaintyEngine {
    /// Create new engine
    pub fn new(config: UncertaintyConfig) -> Self {
        Self {
            values: BTreeMap::new(),
            results: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: UncertaintyStats::default(),
        }
    }

    /// Create uncertain value with std dev
    pub fn create_normal(&mut self, name: &str, mean: f64, std: f64) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let value = UncertainValue {
            id,
            name: name.into(),
            estimate: mean,
            uncertainty: Uncertainty::StdDev(std),
            sources: Vec::new(),
        };

        self.values.insert(id, value);
        self.stats.values_created += 1;

        id
    }

    /// Create with confidence interval
    pub fn create_interval(
        &mut self,
        name: &str,
        estimate: f64,
        low: f64,
        high: f64,
        confidence: f64,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let value = UncertainValue {
            id,
            name: name.into(),
            estimate,
            uncertainty: Uncertainty::ConfidenceInterval {
                low,
                high,
                confidence,
            },
            sources: Vec::new(),
        };

        self.values.insert(id, value);
        self.stats.values_created += 1;

        id
    }

    /// Create uniform
    pub fn create_uniform(&mut self, name: &str, min: f64, max: f64) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let value = UncertainValue {
            id,
            name: name.into(),
            estimate: (min + max) / 2.0,
            uncertainty: Uncertainty::Distribution(Distribution::Uniform { min, max }),
            sources: Vec::new(),
        };

        self.values.insert(id, value);
        self.stats.values_created += 1;

        id
    }

    /// Create triangular
    pub fn create_triangular(&mut self, name: &str, min: f64, mode: f64, max: f64) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let value = UncertainValue {
            id,
            name: name.into(),
            estimate: mode,
            uncertainty: Uncertainty::Distribution(Distribution::Triangular { min, mode, max }),
            sources: Vec::new(),
        };

        self.values.insert(id, value);
        self.stats.values_created += 1;

        id
    }

    /// Add source
    #[inline]
    pub fn add_source(
        &mut self,
        value_id: u64,
        source_type: SourceType,
        contribution: f64,
        description: &str,
    ) {
        if let Some(value) = self.values.get_mut(&value_id) {
            value.sources.push(UncertaintySource {
                source_type,
                contribution: contribution.clamp(0.0, 1.0),
                description: description.into(),
            });
        }
    }

    /// Get variance
    pub fn variance(&self, value_id: u64) -> Option<f64> {
        let value = self.values.get(&value_id)?;

        Some(match &value.uncertainty {
            Uncertainty::StdDev(s) => s * s,
            Uncertainty::ConfidenceInterval {
                low,
                high,
                confidence,
            } => {
                // Approximate variance from CI
                let z = self.z_score(*confidence);
                let std = (high - low) / (2.0 * z);
                std * std
            },
            Uncertainty::Distribution(dist) => self.distribution_variance(dist),
            Uncertainty::CredalSet { lower, upper } => {
                // Maximum variance
                let range = upper - lower;
                range * range / 4.0
            },
        })
    }

    fn distribution_variance(&self, dist: &Distribution) -> f64 {
        match dist {
            Distribution::Normal { std, .. } => std * std,
            Distribution::Uniform { min, max } => (max - min) * (max - min) / 12.0,
            Distribution::Triangular { min, mode, max } => {
                (min * min + mode * mode + max * max - min * mode - min * max - mode * max) / 18.0
            },
            Distribution::Beta { alpha, beta } => {
                (alpha * beta) / ((alpha + beta) * (alpha + beta) * (alpha + beta + 1.0))
            },
            Distribution::Discrete { values } => {
                let mean: f64 = values.iter().map(|(v, p)| v * p).sum();
                values
                    .iter()
                    .map(|(v, p)| (v - mean) * (v - mean) * p)
                    .sum()
            },
        }
    }

    fn z_score(&self, confidence: f64) -> f64 {
        // Approximate z-score for common confidence levels
        if confidence >= 0.99 {
            2.576
        } else if confidence >= 0.95 {
            1.96
        } else if confidence >= 0.90 {
            1.645
        } else {
            1.28
        }
    }

    /// Propagate through linear combination
    pub fn propagate_linear(
        &mut self,
        inputs: &[(u64, f64)],
        name: &str,
    ) -> Option<PropagationResult> {
        let mut estimate = 0.0;
        let mut variance = 0.0;
        let mut sensitivity = BTreeMap::new();
        let mut contribution = BTreeMap::new();

        for &(id, coeff) in inputs {
            let value = self.values.get(&id)?;
            estimate += coeff * value.estimate;

            let input_var = self.variance(id)?;
            variance += coeff * coeff * input_var;

            sensitivity.insert(id, coeff);
        }

        // Calculate contributions
        for &(id, coeff) in inputs {
            if variance > 0.0 {
                let input_var = self.variance(id).unwrap_or(0.0);
                contribution.insert(id, coeff * coeff * input_var / variance);
            }
        }

        let output_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let std = variance.sqrt();

        let output = UncertainValue {
            id: output_id,
            name: name.into(),
            estimate,
            uncertainty: Uncertainty::StdDev(std),
            sources: Vec::new(),
        };

        self.values.insert(output_id, output.clone());
        self.stats.propagations += 1;

        let result = PropagationResult {
            output,
            sensitivity,
            contribution,
        };

        self.results.insert(output_id, result.clone());

        Some(result)
    }

    /// Propagate through product
    pub fn propagate_product(
        &mut self,
        input1: u64,
        input2: u64,
        name: &str,
    ) -> Option<PropagationResult> {
        let v1 = self.values.get(&input1)?;
        let v2 = self.values.get(&input2)?;

        let estimate = v1.estimate * v2.estimate;

        // For products, use relative variance
        let rel_var1 = self.variance(input1)? / (v1.estimate * v1.estimate);
        let rel_var2 = self.variance(input2)? / (v2.estimate * v2.estimate);

        let rel_var = rel_var1 + rel_var2;
        let variance = rel_var * estimate * estimate;

        let output_id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let output = UncertainValue {
            id: output_id,
            name: name.into(),
            estimate,
            uncertainty: Uncertainty::StdDev(variance.sqrt()),
            sources: Vec::new(),
        };

        let mut sensitivity = BTreeMap::new();
        sensitivity.insert(input1, v2.estimate);
        sensitivity.insert(input2, v1.estimate);

        let mut contribution = BTreeMap::new();
        contribution.insert(input1, rel_var1 / rel_var);
        contribution.insert(input2, rel_var2 / rel_var);

        self.values.insert(output_id, output.clone());
        self.stats.propagations += 1;

        let result = PropagationResult {
            output,
            sensitivity,
            contribution,
        };

        self.results.insert(output_id, result.clone());

        Some(result)
    }

    /// Compute sensitivity
    pub fn sensitivity_analysis(&mut self, output_id: u64) -> Vec<SensitivityInfo> {
        let result = match self.results.get(&output_id) {
            Some(r) => r.clone(),
            None => return Vec::new(),
        };

        let mut info = Vec::new();

        for (&input_id, &sens) in &result.sensitivity {
            let var_contrib = result.contribution.get(&input_id).copied().unwrap_or(0.0);

            info.push(SensitivityInfo {
                input_id,
                sensitivity: sens,
                variance_contribution: var_contrib,
            });

            self.stats.sensitivities_computed += 1;
        }

        info.sort_by(|a, b| {
            b.variance_contribution
                .partial_cmp(&a.variance_contribution)
                .unwrap_or(core::cmp::Ordering::Equal)
        });

        info
    }

    /// Get value
    #[inline(always)]
    pub fn get_value(&self, id: u64) -> Option<&UncertainValue> {
        self.values.get(&id)
    }

    /// Get confidence interval
    #[inline]
    pub fn confidence_interval(&self, id: u64, level: f64) -> Option<(f64, f64)> {
        let value = self.values.get(&id)?;
        let variance = self.variance(id)?;
        let std = variance.sqrt();
        let z = self.z_score(level);

        Some((value.estimate - z * std, value.estimate + z * std))
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &UncertaintyStats {
        &self.stats
    }
}

impl Default for UncertaintyEngine {
    fn default() -> Self {
        Self::new(UncertaintyConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_normal() {
        let mut engine = UncertaintyEngine::default();

        let id = engine.create_normal("x", 10.0, 2.0);
        let value = engine.get_value(id).unwrap();

        assert_eq!(value.estimate, 10.0);
    }

    #[test]
    fn test_variance() {
        let mut engine = UncertaintyEngine::default();

        let id = engine.create_normal("x", 0.0, 3.0);
        let variance = engine.variance(id).unwrap();

        assert!((variance - 9.0).abs() < 0.01);
    }

    #[test]
    fn test_propagate_linear() {
        let mut engine = UncertaintyEngine::default();

        let x = engine.create_normal("x", 10.0, 1.0);
        let y = engine.create_normal("y", 20.0, 2.0);

        let result = engine.propagate_linear(&[(x, 2.0), (y, 1.0)], "z").unwrap();

        // z = 2x + y = 2*10 + 20 = 40
        assert!((result.output.estimate - 40.0).abs() < 0.01);
    }

    #[test]
    fn test_propagate_product() {
        let mut engine = UncertaintyEngine::default();

        let x = engine.create_normal("x", 5.0, 0.5);
        let y = engine.create_normal("y", 4.0, 0.4);

        let result = engine.propagate_product(x, y, "xy").unwrap();

        // xy = 5 * 4 = 20
        assert!((result.output.estimate - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_confidence_interval() {
        let mut engine = UncertaintyEngine::default();

        let id = engine.create_normal("x", 100.0, 10.0);

        let (low, high) = engine.confidence_interval(id, 0.95).unwrap();

        // 95% CI: 100 ± 1.96 * 10
        assert!(low < 100.0 && high > 100.0);
        assert!((high - low - 39.2).abs() < 0.5);
    }

    #[test]
    fn test_uniform_variance() {
        let mut engine = UncertaintyEngine::default();

        let id = engine.create_uniform("u", 0.0, 12.0);
        let variance = engine.variance(id).unwrap();

        // Uniform variance = (b-a)^2 / 12 = 144/12 = 12
        assert!((variance - 12.0).abs() < 0.01);
    }

    #[test]
    fn test_sensitivity_analysis() {
        let mut engine = UncertaintyEngine::default();

        let x = engine.create_normal("x", 10.0, 2.0);
        let y = engine.create_normal("y", 20.0, 1.0);

        let result = engine.propagate_linear(&[(x, 1.0), (y, 1.0)], "z").unwrap();

        let sensitivity = engine.sensitivity_analysis(result.output.id);

        assert_eq!(sensitivity.len(), 2);
        // x contributes more variance (4 vs 1)
        assert!(sensitivity[0].input_id == x);
    }
}
