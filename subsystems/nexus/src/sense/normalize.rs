//! # Normalization Engine
//!
//! Normalizes and standardizes sensory input.
//! Converts diverse data formats to unified representations.
//!
//! Part of Year 2 COGNITION - Sense/Normalize

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// NORMALIZATION TYPES
// ============================================================================

/// Raw input
#[derive(Debug, Clone)]
pub struct RawInput {
    /// Input ID
    pub id: u64,
    /// Source
    pub source: String,
    /// Type
    pub input_type: InputType,
    /// Data
    pub data: InputData,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Input type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputType {
    Numeric,
    Categorical,
    Temporal,
    Spatial,
    Textual,
    Binary,
    Mixed,
}

/// Input data
#[derive(Debug, Clone)]
pub enum InputData {
    Number(f64),
    Integer(i64),
    Text(String),
    Vector(Vec<f64>),
    Bytes(Vec<u8>),
    Map(BTreeMap<String, f64>),
}

/// Normalized output
#[derive(Debug, Clone)]
pub struct NormalizedOutput {
    /// Output ID
    pub id: u64,
    /// Original ID
    pub original: u64,
    /// Data
    pub data: NormalizedData,
    /// Method used
    pub method: NormalizationMethod,
    /// Quality score
    pub quality: f64,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Normalized data
#[derive(Debug, Clone)]
pub enum NormalizedData {
    Scalar(f64),
    Vector(Vec<f64>),
    Embedding(Vec<f64>),
    Sparse(BTreeMap<usize, f64>),
}

/// Normalization method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizationMethod {
    MinMax,
    ZScore,
    Robust,
    Log,
    Quantile,
    UnitVector,
    None,
}

/// Normalization parameters
#[derive(Debug, Clone)]
pub struct NormParams {
    /// Method
    pub method: NormalizationMethod,
    /// Statistics
    pub stats: NormStats,
    /// Target range
    pub target_range: (f64, f64),
}

/// Normalization statistics
#[derive(Debug, Clone, Default)]
pub struct NormStats {
    /// Minimum
    pub min: f64,
    /// Maximum
    pub max: f64,
    /// Mean
    pub mean: f64,
    /// Standard deviation
    pub std: f64,
    /// Median
    pub median: f64,
    /// IQR
    pub iqr: f64,
    /// Count
    pub count: u64,
}

// ============================================================================
// NORMALIZER
// ============================================================================

/// Normalizer
pub struct Normalizer {
    /// Inputs
    inputs: BTreeMap<u64, RawInput>,
    /// Outputs
    outputs: BTreeMap<u64, NormalizedOutput>,
    /// Parameters per source
    params: BTreeMap<String, NormParams>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: NormConfig,
    /// Statistics
    stats: NormalizerStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct NormConfig {
    /// Default method
    pub default_method: NormalizationMethod,
    /// Target range
    pub target_range: (f64, f64),
    /// Clip outliers
    pub clip_outliers: bool,
    /// Outlier threshold (std devs)
    pub outlier_threshold: f64,
}

impl Default for NormConfig {
    fn default() -> Self {
        Self {
            default_method: NormalizationMethod::MinMax,
            target_range: (0.0, 1.0),
            clip_outliers: true,
            outlier_threshold: 3.0,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct NormalizerStats {
    /// Inputs processed
    pub inputs_processed: u64,
    /// Outputs generated
    pub outputs_generated: u64,
    /// Outliers clipped
    pub outliers_clipped: u64,
}

impl Normalizer {
    /// Create new normalizer
    pub fn new(config: NormConfig) -> Self {
        Self {
            inputs: BTreeMap::new(),
            outputs: BTreeMap::new(),
            params: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: NormalizerStats::default(),
        }
    }

    /// Submit input
    pub fn submit(&mut self, source: &str, input_type: InputType, data: InputData) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let input = RawInput {
            id,
            source: source.into(),
            input_type,
            data,
            timestamp: Timestamp::now(),
        };

        self.inputs.insert(id, input);
        self.stats.inputs_processed += 1;

        id
    }

    /// Normalize input
    pub fn normalize(&mut self, input_id: u64) -> Option<NormalizedOutput> {
        let input = self.inputs.get(&input_id)?.clone();

        let method = self.select_method(&input);
        let data = self.apply_normalization(&input, method);

        let output = NormalizedOutput {
            id: self.next_id.fetch_add(1, Ordering::Relaxed),
            original: input_id,
            data,
            method,
            quality: self.assess_quality(&input),
            timestamp: Timestamp::now(),
        };

        self.stats.outputs_generated += 1;
        self.outputs.insert(output.id, output.clone());

        Some(output)
    }

    fn select_method(&self, input: &RawInput) -> NormalizationMethod {
        match &input.data {
            InputData::Number(_) | InputData::Integer(_) => self.config.default_method,
            InputData::Vector(_) => NormalizationMethod::ZScore,
            InputData::Map(_) => NormalizationMethod::MinMax,
            InputData::Text(_) => NormalizationMethod::None,
            InputData::Bytes(_) => NormalizationMethod::None,
        }
    }

    fn apply_normalization(
        &mut self,
        input: &RawInput,
        method: NormalizationMethod,
    ) -> NormalizedData {
        match &input.data {
            InputData::Number(n) => {
                let normalized = self.normalize_scalar(*n, &input.source, method);
                NormalizedData::Scalar(normalized)
            },
            InputData::Integer(n) => {
                let normalized = self.normalize_scalar(*n as f64, &input.source, method);
                NormalizedData::Scalar(normalized)
            },
            InputData::Vector(v) => {
                let normalized = self.normalize_vector(v, &input.source, method);
                NormalizedData::Vector(normalized)
            },
            InputData::Map(m) => {
                let values: Vec<f64> = m.values().copied().collect();
                let normalized = self.normalize_vector(&values, &input.source, method);
                NormalizedData::Vector(normalized)
            },
            InputData::Text(t) => {
                // Simple character encoding
                let vec: Vec<f64> = t
                    .chars()
                    .take(100)
                    .map(|c| c as u32 as f64 / 128.0)
                    .collect();
                NormalizedData::Embedding(vec)
            },
            InputData::Bytes(b) => {
                let vec: Vec<f64> = b.iter().map(|&byte| byte as f64 / 255.0).collect();
                NormalizedData::Vector(vec)
            },
        }
    }

    fn normalize_scalar(&mut self, value: f64, source: &str, method: NormalizationMethod) -> f64 {
        // Get or create params
        let params = self
            .params
            .entry(source.into())
            .or_insert_with(|| NormParams {
                method,
                stats: NormStats::default(),
                target_range: self.config.target_range,
            });

        // Update stats
        self.update_stats(&mut params.stats, value);

        let stats = &params.stats;
        let (min_out, max_out) = self.config.target_range;

        let normalized = match method {
            NormalizationMethod::MinMax => {
                if stats.max - stats.min > 0.0 {
                    (value - stats.min) / (stats.max - stats.min) * (max_out - min_out) + min_out
                } else {
                    (min_out + max_out) / 2.0
                }
            },
            NormalizationMethod::ZScore => {
                if stats.std > 0.0 {
                    (value - stats.mean) / stats.std
                } else {
                    0.0
                }
            },
            NormalizationMethod::Robust => {
                if stats.iqr > 0.0 {
                    (value - stats.median) / stats.iqr
                } else {
                    0.0
                }
            },
            NormalizationMethod::Log => {
                if value > 0.0 {
                    value.ln()
                } else {
                    0.0
                }
            },
            _ => value,
        };

        // Clip outliers
        if self.config.clip_outliers {
            let threshold = self.config.outlier_threshold;
            if normalized.abs() > threshold {
                self.stats.outliers_clipped += 1;
                return normalized.signum() * threshold;
            }
        }

        normalized
    }

    fn normalize_vector(
        &mut self,
        values: &[f64],
        source: &str,
        method: NormalizationMethod,
    ) -> Vec<f64> {
        match method {
            NormalizationMethod::UnitVector => {
                let norm: f64 = values.iter().map(|v| v * v).sum::<f64>().sqrt();
                if norm > 0.0 {
                    values.iter().map(|v| v / norm).collect()
                } else {
                    values.to_vec()
                }
            },
            _ => values
                .iter()
                .map(|&v| self.normalize_scalar(v, source, method))
                .collect(),
        }
    }

    fn update_stats(&self, stats: &mut NormStats, value: f64) {
        stats.count += 1;

        if stats.count == 1 {
            stats.min = value;
            stats.max = value;
            stats.mean = value;
            stats.median = value;
        } else {
            stats.min = stats.min.min(value);
            stats.max = stats.max.max(value);

            // Running mean
            let n = stats.count as f64;
            stats.mean = stats.mean + (value - stats.mean) / n;

            // Running variance (Welford's algorithm simplified)
            let delta = value - stats.mean;
            stats.std = ((stats.std * stats.std * (n - 1.0) + delta * delta) / n).sqrt();
        }
    }

    fn assess_quality(&self, input: &RawInput) -> f64 {
        match &input.data {
            InputData::Number(n) if n.is_finite() => 1.0,
            InputData::Number(_) => 0.0,
            InputData::Integer(_) => 1.0,
            InputData::Vector(v) => {
                let finite_count = v.iter().filter(|x| x.is_finite()).count();
                finite_count as f64 / v.len().max(1) as f64
            },
            InputData::Map(m) => {
                let finite_count = m.values().filter(|x| x.is_finite()).count();
                finite_count as f64 / m.len().max(1) as f64
            },
            InputData::Text(t) => {
                if t.is_empty() {
                    0.0
                } else {
                    1.0
                }
            },
            InputData::Bytes(b) => {
                if b.is_empty() {
                    0.0
                } else {
                    1.0
                }
            },
        }
    }

    /// Fit parameters from data
    pub fn fit(&mut self, source: &str, values: &[f64], method: NormalizationMethod) {
        if values.is_empty() {
            return;
        }

        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));

        let n = values.len();
        let min = sorted[0];
        let max = sorted[n - 1];
        let mean: f64 = values.iter().sum::<f64>() / n as f64;

        let variance: f64 = values.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / n as f64;
        let std = variance.sqrt();

        let median = if n % 2 == 0 {
            (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
        } else {
            sorted[n / 2]
        };

        let q1 = sorted[n / 4];
        let q3 = sorted[3 * n / 4];
        let iqr = q3 - q1;

        let stats = NormStats {
            min,
            max,
            mean,
            std,
            median,
            iqr,
            count: n as u64,
        };

        self.params.insert(source.into(), NormParams {
            method,
            stats,
            target_range: self.config.target_range,
        });
    }

    /// Get parameters
    pub fn params(&self, source: &str) -> Option<&NormParams> {
        self.params.get(source)
    }

    /// Get output
    pub fn get(&self, id: u64) -> Option<&NormalizedOutput> {
        self.outputs.get(&id)
    }

    /// Get statistics
    pub fn stats(&self) -> &NormalizerStats {
        &self.stats
    }
}

impl Default for Normalizer {
    fn default() -> Self {
        Self::new(NormConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_submit() {
        let mut normalizer = Normalizer::default();

        let id = normalizer.submit("sensor", InputType::Numeric, InputData::Number(42.0));
        assert!(normalizer.inputs.contains_key(&id));
    }

    #[test]
    fn test_normalize_scalar() {
        let mut normalizer = Normalizer::default();

        // Fit with sample data
        normalizer.fit("test", &[0.0, 50.0, 100.0], NormalizationMethod::MinMax);

        let id = normalizer.submit("test", InputType::Numeric, InputData::Number(50.0));
        let output = normalizer.normalize(id).unwrap();

        if let NormalizedData::Scalar(v) = output.data {
            assert!(v >= 0.0 && v <= 1.0);
        }
    }

    #[test]
    fn test_normalize_vector() {
        let mut normalizer = Normalizer::default();

        let id = normalizer.submit(
            "vec",
            InputType::Numeric,
            InputData::Vector(vec![1.0, 2.0, 3.0, 4.0]),
        );

        let output = normalizer.normalize(id).unwrap();

        if let NormalizedData::Vector(v) = output.data {
            assert_eq!(v.len(), 4);
        }
    }

    #[test]
    fn test_unit_vector() {
        let mut normalizer = Normalizer::new(NormConfig {
            default_method: NormalizationMethod::UnitVector,
            ..Default::default()
        });

        let id = normalizer.submit("vec", InputType::Numeric, InputData::Vector(vec![3.0, 4.0]));

        let output = normalizer.normalize(id).unwrap();

        if let NormalizedData::Vector(v) = output.data {
            let norm: f64 = v.iter().map(|x| x * x).sum::<f64>().sqrt();
            assert!((norm - 1.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_fit() {
        let mut normalizer = Normalizer::default();

        normalizer.fit(
            "source",
            &[10.0, 20.0, 30.0, 40.0, 50.0],
            NormalizationMethod::ZScore,
        );

        let params = normalizer.params("source").unwrap();
        assert_eq!(params.stats.min, 10.0);
        assert_eq!(params.stats.max, 50.0);
    }

    #[test]
    fn test_text_normalization() {
        let mut normalizer = Normalizer::default();

        let id = normalizer.submit("text", InputType::Textual, InputData::Text("hello".into()));

        let output = normalizer.normalize(id).unwrap();

        if let NormalizedData::Embedding(v) = output.data {
            assert!(!v.is_empty());
        }
    }
}
