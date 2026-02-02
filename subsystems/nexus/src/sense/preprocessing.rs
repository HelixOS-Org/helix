//! # Perception Preprocessing
//!
//! Preprocesses sensory data before analysis.
//! Normalization, feature extraction, and transformation.
//!
//! Part of Year 2 COGNITION - Perception Engine

#![allow(dead_code)]

extern crate alloc;
use alloc::format;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// PREPROCESSING TYPES
// ============================================================================

/// Raw input data
#[derive(Debug, Clone)]
pub struct RawInput {
    /// Input ID
    pub id: u64,
    /// Data type
    pub data_type: DataType,
    /// Data
    pub data: RawData,
    /// Metadata
    pub metadata: BTreeMap<String, String>,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Data type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    Numeric,
    Text,
    Categorical,
    Sequence,
    Structured,
}

/// Raw data
#[derive(Debug, Clone)]
pub enum RawData {
    Numbers(Vec<f64>),
    Text(String),
    Categories(Vec<String>),
    Sequence(Vec<u8>),
    Struct(BTreeMap<String, RawData>),
}

/// Processed data
#[derive(Debug, Clone)]
pub struct ProcessedData {
    /// Original ID
    pub source_id: u64,
    /// Features
    pub features: Vec<Feature>,
    /// Pipeline applied
    pub pipeline: Vec<String>,
    /// Processing time
    pub processing_time_ns: u64,
}

/// Feature
#[derive(Debug, Clone)]
pub struct Feature {
    /// Name
    pub name: String,
    /// Value
    pub value: FeatureValue,
    /// Importance
    pub importance: f64,
}

/// Feature value
#[derive(Debug, Clone)]
pub enum FeatureValue {
    Scalar(f64),
    Vector(Vec<f64>),
    Category(String),
    Boolean(bool),
}

/// Preprocessing step
#[derive(Debug, Clone)]
pub struct PreprocessStep {
    /// Step ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Operation
    pub operation: PreprocessOp,
    /// Parameters
    pub params: BTreeMap<String, f64>,
}

/// Preprocessing operation
#[derive(Debug, Clone)]
pub enum PreprocessOp {
    /// Normalize to 0-1 range
    Normalize,
    /// Standardize (z-score)
    Standardize,
    /// Scale by factor
    Scale { factor: f64 },
    /// Clip to range
    Clip { min: f64, max: f64 },
    /// Fill missing values
    FillMissing { value: f64 },
    /// One-hot encode
    OneHotEncode,
    /// Tokenize text
    Tokenize,
    /// Extract n-grams
    NGram { n: usize },
    /// Compute statistics
    Statistics,
    /// Apply log transform
    Log,
    /// Apply power transform
    Power { exponent: f64 },
    /// Difference
    Difference,
    /// Moving average
    MovingAverage { window: usize },
}

// ============================================================================
// PREPROCESSOR
// ============================================================================

/// Preprocessor
pub struct Preprocessor {
    /// Pipeline
    pipeline: Vec<PreprocessStep>,
    /// Statistics
    fit_stats: BTreeMap<String, FitStatistics>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: PreprocessConfig,
    /// Statistics
    stats: PreprocessStats,
}

/// Fit statistics (learned from data)
#[derive(Debug, Clone)]
pub struct FitStatistics {
    /// Mean
    pub mean: f64,
    /// Standard deviation
    pub std: f64,
    /// Min
    pub min: f64,
    /// Max
    pub max: f64,
    /// Categories
    pub categories: Vec<String>,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct PreprocessConfig {
    /// Handle missing values
    pub handle_missing: bool,
    /// Default missing value
    pub default_missing: f64,
    /// Epsilon for numerical stability
    pub epsilon: f64,
}

impl Default for PreprocessConfig {
    fn default() -> Self {
        Self {
            handle_missing: true,
            default_missing: 0.0,
            epsilon: 1e-8,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct PreprocessStats {
    /// Inputs processed
    pub inputs_processed: u64,
    /// Features extracted
    pub features_extracted: u64,
    /// Processing time ns
    pub total_processing_ns: u64,
}

impl Preprocessor {
    /// Create new preprocessor
    pub fn new(config: PreprocessConfig) -> Self {
        Self {
            pipeline: Vec::new(),
            fit_stats: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: PreprocessStats::default(),
        }
    }

    /// Add step to pipeline
    pub fn add_step(&mut self, name: &str, operation: PreprocessOp) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        self.pipeline.push(PreprocessStep {
            id,
            name: name.into(),
            operation,
            params: BTreeMap::new(),
        });

        id
    }

    /// Fit on data (learn statistics)
    pub fn fit(&mut self, inputs: &[RawInput]) {
        for input in inputs {
            let key = format!("input_{}", input.id);

            match &input.data {
                RawData::Numbers(nums) => {
                    let stats = self.compute_stats(nums);
                    self.fit_stats.insert(key, stats);
                },

                RawData::Categories(cats) => {
                    self.fit_stats.insert(key, FitStatistics {
                        mean: 0.0,
                        std: 0.0,
                        min: 0.0,
                        max: 0.0,
                        categories: cats.clone(),
                    });
                },

                _ => {},
            }
        }
    }

    fn compute_stats(&self, data: &[f64]) -> FitStatistics {
        if data.is_empty() {
            return FitStatistics {
                mean: 0.0,
                std: 1.0,
                min: 0.0,
                max: 1.0,
                categories: Vec::new(),
            };
        }

        let sum: f64 = data.iter().sum();
        let mean = sum / data.len() as f64;

        let variance: f64 =
            data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / data.len() as f64;
        let std = variance.sqrt();

        let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        FitStatistics {
            mean,
            std,
            min,
            max,
            categories: Vec::new(),
        }
    }

    /// Process input
    pub fn process(&mut self, input: RawInput) -> ProcessedData {
        let start = Timestamp::now();
        let source_id = input.id;

        let mut features = Vec::new();
        let mut pipeline_names = Vec::new();

        // Apply each step
        let mut current_data = input.data;

        for step in &self.pipeline {
            pipeline_names.push(step.name.clone());
            current_data = self.apply_step(&step.operation, current_data);
        }

        // Extract features from processed data
        match current_data {
            RawData::Numbers(nums) => {
                for (i, v) in nums.iter().enumerate() {
                    features.push(Feature {
                        name: format!("feature_{}", i),
                        value: FeatureValue::Scalar(*v),
                        importance: 1.0,
                    });
                }
            },

            RawData::Text(text) => {
                // Basic text features
                features.push(Feature {
                    name: "length".into(),
                    value: FeatureValue::Scalar(text.len() as f64),
                    importance: 0.5,
                });

                features.push(Feature {
                    name: "word_count".into(),
                    value: FeatureValue::Scalar(text.split_whitespace().count() as f64),
                    importance: 0.8,
                });
            },

            RawData::Categories(cats) => {
                for (i, cat) in cats.iter().enumerate() {
                    features.push(Feature {
                        name: format!("category_{}", i),
                        value: FeatureValue::Category(cat.clone()),
                        importance: 1.0,
                    });
                }
            },

            _ => {},
        }

        let end = Timestamp::now();
        let processing_time = end.0.saturating_sub(start.0);

        self.stats.inputs_processed += 1;
        self.stats.features_extracted += features.len() as u64;
        self.stats.total_processing_ns += processing_time;

        ProcessedData {
            source_id,
            features,
            pipeline: pipeline_names,
            processing_time_ns: processing_time,
        }
    }

    fn apply_step(&self, op: &PreprocessOp, data: RawData) -> RawData {
        match (op, data) {
            (PreprocessOp::Normalize, RawData::Numbers(nums)) => {
                let min = nums.iter().cloned().fold(f64::INFINITY, f64::min);
                let max = nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let range = (max - min).max(self.config.epsilon);

                RawData::Numbers(nums.iter().map(|x| (x - min) / range).collect())
            },

            (PreprocessOp::Standardize, RawData::Numbers(nums)) => {
                let stats = self.compute_stats(&nums);
                let std = stats.std.max(self.config.epsilon);

                RawData::Numbers(nums.iter().map(|x| (x - stats.mean) / std).collect())
            },

            (PreprocessOp::Scale { factor }, RawData::Numbers(nums)) => {
                RawData::Numbers(nums.iter().map(|x| x * factor).collect())
            },

            (PreprocessOp::Clip { min, max }, RawData::Numbers(nums)) => {
                RawData::Numbers(nums.iter().map(|x| x.max(*min).min(*max)).collect())
            },

            (PreprocessOp::FillMissing { value }, RawData::Numbers(nums)) => RawData::Numbers(
                nums.iter()
                    .map(|x| if x.is_nan() { *value } else { *x })
                    .collect(),
            ),

            (PreprocessOp::Log, RawData::Numbers(nums)) => RawData::Numbers(
                nums.iter()
                    .map(|x| (x.max(self.config.epsilon)).ln())
                    .collect(),
            ),

            (PreprocessOp::Power { exponent }, RawData::Numbers(nums)) => {
                RawData::Numbers(nums.iter().map(|x| x.powf(*exponent)).collect())
            },

            (PreprocessOp::Difference, RawData::Numbers(nums)) => {
                if nums.len() < 2 {
                    return RawData::Numbers(Vec::new());
                }

                let diff: Vec<f64> = nums.windows(2).map(|w| w[1] - w[0]).collect();

                RawData::Numbers(diff)
            },

            (PreprocessOp::MovingAverage { window }, RawData::Numbers(nums)) => {
                if nums.len() < *window || *window == 0 {
                    return RawData::Numbers(nums);
                }

                let mut result = Vec::new();

                for i in 0..=(nums.len() - window) {
                    let sum: f64 = nums[i..i + window].iter().sum();
                    result.push(sum / *window as f64);
                }

                RawData::Numbers(result)
            },

            (PreprocessOp::Tokenize, RawData::Text(text)) => {
                let tokens: Vec<String> =
                    text.split_whitespace().map(|s| s.to_lowercase()).collect();

                RawData::Categories(tokens)
            },

            (PreprocessOp::Statistics, RawData::Numbers(nums)) => {
                let stats = self.compute_stats(&nums);

                let mut result = BTreeMap::new();
                result.insert("mean".into(), RawData::Numbers(vec![stats.mean]));
                result.insert("std".into(), RawData::Numbers(vec![stats.std]));
                result.insert("min".into(), RawData::Numbers(vec![stats.min]));
                result.insert("max".into(), RawData::Numbers(vec![stats.max]));

                RawData::Struct(result)
            },

            (PreprocessOp::OneHotEncode, RawData::Categories(cats)) => {
                // Get unique categories
                let mut unique: Vec<String> = cats.clone();
                unique.sort();
                unique.dedup();

                // One-hot encode
                let mut encoded = Vec::new();
                for cat in &cats {
                    for u in &unique {
                        encoded.push(if cat == u { 1.0 } else { 0.0 });
                    }
                }

                RawData::Numbers(encoded)
            },

            (_, data) => data, // Passthrough for unsupported combinations
        }
    }

    /// Clear pipeline
    pub fn clear(&mut self) {
        self.pipeline.clear();
    }

    /// Get statistics
    pub fn stats(&self) -> &PreprocessStats {
        &self.stats
    }
}

impl Default for Preprocessor {
    fn default() -> Self {
        Self::new(PreprocessConfig::default())
    }
}

// ============================================================================
// PIPELINE BUILDER
// ============================================================================

/// Pipeline builder
pub struct PipelineBuilder {
    preprocessor: Preprocessor,
}

impl PipelineBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            preprocessor: Preprocessor::default(),
        }
    }

    /// Add normalization
    pub fn normalize(mut self) -> Self {
        self.preprocessor
            .add_step("normalize", PreprocessOp::Normalize);
        self
    }

    /// Add standardization
    pub fn standardize(mut self) -> Self {
        self.preprocessor
            .add_step("standardize", PreprocessOp::Standardize);
        self
    }

    /// Add scaling
    pub fn scale(mut self, factor: f64) -> Self {
        self.preprocessor
            .add_step("scale", PreprocessOp::Scale { factor });
        self
    }

    /// Add clipping
    pub fn clip(mut self, min: f64, max: f64) -> Self {
        self.preprocessor
            .add_step("clip", PreprocessOp::Clip { min, max });
        self
    }

    /// Add log transform
    pub fn log(mut self) -> Self {
        self.preprocessor.add_step("log", PreprocessOp::Log);
        self
    }

    /// Add moving average
    pub fn moving_average(mut self, window: usize) -> Self {
        self.preprocessor
            .add_step("moving_avg", PreprocessOp::MovingAverage { window });
        self
    }

    /// Build
    pub fn build(self) -> Preprocessor {
        self.preprocessor
    }
}

impl Default for PipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_numeric_input(data: Vec<f64>) -> RawInput {
        RawInput {
            id: 1,
            data_type: DataType::Numeric,
            data: RawData::Numbers(data),
            metadata: BTreeMap::new(),
            timestamp: Timestamp::now(),
        }
    }

    #[test]
    fn test_normalize() {
        let mut processor = Preprocessor::default();
        processor.add_step("normalize", PreprocessOp::Normalize);

        let input = create_numeric_input(vec![0.0, 50.0, 100.0]);
        let result = processor.process(input);

        // After normalization: [0.0, 0.5, 1.0]
        assert_eq!(result.features.len(), 3);
    }

    #[test]
    fn test_standardize() {
        let mut processor = Preprocessor::default();
        processor.add_step("standardize", PreprocessOp::Standardize);

        let input = create_numeric_input(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        let result = processor.process(input);

        // Mean should be near 0
        let mean: f64 = result
            .features
            .iter()
            .filter_map(|f| {
                if let FeatureValue::Scalar(v) = f.value {
                    Some(v)
                } else {
                    None
                }
            })
            .sum::<f64>()
            / result.features.len() as f64;

        assert!(mean.abs() < 0.01);
    }

    #[test]
    fn test_clip() {
        let mut processor = Preprocessor::default();
        processor.add_step("clip", PreprocessOp::Clip { min: 0.0, max: 1.0 });

        let input = create_numeric_input(vec![-1.0, 0.5, 2.0]);
        let result = processor.process(input);

        assert_eq!(result.features.len(), 3);
    }

    #[test]
    fn test_moving_average() {
        let mut processor = Preprocessor::default();
        processor.add_step("ma", PreprocessOp::MovingAverage { window: 3 });

        let input = create_numeric_input(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        let result = processor.process(input);

        // [1,2,3] -> 2, [2,3,4] -> 3, [3,4,5] -> 4
        assert_eq!(result.features.len(), 3);
    }

    #[test]
    fn test_pipeline_builder() {
        let processor = PipelineBuilder::new().normalize().clip(0.0, 1.0).build();

        assert_eq!(processor.pipeline.len(), 2);
    }

    #[test]
    fn test_tokenize() {
        let mut processor = Preprocessor::default();
        processor.add_step("tokenize", PreprocessOp::Tokenize);

        let input = RawInput {
            id: 1,
            data_type: DataType::Text,
            data: RawData::Text("Hello World Test".into()),
            metadata: BTreeMap::new(),
            timestamp: Timestamp::now(),
        };

        let result = processor.process(input);

        // Should have category features for tokens
        assert!(
            result
                .features
                .iter()
                .any(|f| f.name.starts_with("category_"))
        );
    }
}
