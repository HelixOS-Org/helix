//! # Sensor Fusion
//!
//! Combines multiple sensory inputs into unified perception.
//! Implements multi-modal integration and consensus building.
//!
//! Part of Year 2 COGNITION - Perception Engine

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// FUSION TYPES
// ============================================================================

/// Sensor input
#[derive(Debug, Clone)]
pub struct SensorInput {
    /// Input ID
    pub id: u64,
    /// Sensor identifier
    pub sensor_id: String,
    /// Modality
    pub modality: Modality,
    /// Data
    pub data: SensorData,
    /// Confidence
    pub confidence: f64,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Modality
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Modality {
    Visual,
    Textual,
    Numeric,
    Temporal,
    Spatial,
    Semantic,
    Behavioral,
}

/// Sensor data
#[derive(Debug, Clone)]
pub enum SensorData {
    Vector(Vec<f64>),
    Matrix(Vec<Vec<f64>>),
    Text(String),
    Structured(BTreeMap<String, f64>),
    Categorical(Vec<String>),
}

/// Fused perception
#[derive(Debug, Clone)]
pub struct FusedPerception {
    /// Perception ID
    pub id: u64,
    /// Source inputs
    pub sources: Vec<u64>,
    /// Fused features
    pub features: BTreeMap<String, FusedFeature>,
    /// Overall confidence
    pub confidence: f64,
    /// Fusion method used
    pub method: FusionMethod,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Fused feature
#[derive(Debug, Clone)]
pub struct FusedFeature {
    /// Name
    pub name: String,
    /// Value
    pub value: f64,
    /// Source contributions
    pub contributions: Vec<Contribution>,
    /// Agreement score
    pub agreement: f64,
}

/// Contribution
#[derive(Debug, Clone)]
pub struct Contribution {
    /// Source sensor
    pub sensor_id: String,
    /// Value
    pub value: f64,
    /// Weight
    pub weight: f64,
}

/// Fusion method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FusionMethod {
    /// Simple average
    Average,
    /// Weighted average
    WeightedAverage,
    /// Kalman filter
    Kalman,
    /// Voting
    Voting,
    /// Dempster-Shafer
    DempsterShafer,
    /// Bayesian
    Bayesian,
}

/// Sensor model
#[derive(Debug, Clone)]
pub struct SensorModel {
    /// Sensor ID
    pub sensor_id: String,
    /// Modality
    pub modality: Modality,
    /// Reliability
    pub reliability: f64,
    /// Bias
    pub bias: f64,
    /// Variance
    pub variance: f64,
}

// ============================================================================
// FUSION ENGINE
// ============================================================================

/// Sensor fusion engine
pub struct FusionEngine {
    /// Pending inputs
    inputs: BTreeMap<u64, SensorInput>,
    /// Sensor models
    sensors: BTreeMap<String, SensorModel>,
    /// Fused perceptions
    perceptions: BTreeMap<u64, FusedPerception>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: FusionConfig,
    /// Statistics
    stats: FusionStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct FusionConfig {
    /// Default fusion method
    pub default_method: FusionMethod,
    /// Minimum confidence
    pub min_confidence: f64,
    /// Time window for fusion (ns)
    pub time_window_ns: u64,
    /// Conflict threshold
    pub conflict_threshold: f64,
}

impl Default for FusionConfig {
    fn default() -> Self {
        Self {
            default_method: FusionMethod::WeightedAverage,
            min_confidence: 0.3,
            time_window_ns: 100_000_000, // 100ms
            conflict_threshold: 0.5,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct FusionStats {
    /// Inputs received
    pub inputs_received: u64,
    /// Fusions performed
    pub fusions_performed: u64,
    /// Conflicts detected
    pub conflicts_detected: u64,
}

impl FusionEngine {
    /// Create new engine
    pub fn new(config: FusionConfig) -> Self {
        Self {
            inputs: BTreeMap::new(),
            sensors: BTreeMap::new(),
            perceptions: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: FusionStats::default(),
        }
    }

    /// Register sensor
    pub fn register_sensor(&mut self, sensor: SensorModel) {
        self.sensors.insert(sensor.sensor_id.clone(), sensor);
    }

    /// Add input
    pub fn add_input(&mut self, input: SensorInput) -> u64 {
        let id = input.id;
        self.inputs.insert(id, input);
        self.stats.inputs_received += 1;
        id
    }

    /// Create input
    pub fn create_input(&mut self, sensor_id: &str, modality: Modality, data: SensorData, confidence: f64) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let input = SensorInput {
            id,
            sensor_id: sensor_id.into(),
            modality,
            data,
            confidence,
            timestamp: Timestamp::now(),
        };

        self.add_input(input)
    }

    /// Fuse inputs
    pub fn fuse(&mut self, input_ids: &[u64], method: Option<FusionMethod>) -> Option<u64> {
        let method = method.unwrap_or(self.config.default_method);

        let inputs: Vec<&SensorInput> = input_ids.iter()
            .filter_map(|id| self.inputs.get(id))
            .filter(|i| i.confidence >= self.config.min_confidence)
            .collect();

        if inputs.is_empty() {
            return None;
        }

        // Check time coherence
        if !self.check_time_coherence(&inputs) {
            return None;
        }

        // Extract features from each input
        let feature_sets: Vec<BTreeMap<String, f64>> = inputs.iter()
            .map(|i| self.extract_features(i))
            .collect();

        // Get all feature names
        let mut all_features: Vec<String> = feature_sets.iter()
            .flat_map(|fs| fs.keys().cloned())
            .collect();
        all_features.sort();
        all_features.dedup();

        // Fuse each feature
        let mut fused_features = BTreeMap::new();

        for feature_name in all_features {
            let contributions: Vec<Contribution> = inputs.iter()
                .zip(feature_sets.iter())
                .filter_map(|(input, fs)| {
                    fs.get(&feature_name).map(|v| Contribution {
                        sensor_id: input.sensor_id.clone(),
                        value: *v,
                        weight: self.get_sensor_weight(&input.sensor_id, input.confidence),
                    })
                })
                .collect();

            if let Some(fused) = self.fuse_feature(&feature_name, &contributions, method) {
                fused_features.insert(feature_name, fused);
            }
        }

        // Check for conflicts
        let conflicts = self.detect_conflicts(&fused_features);
        if conflicts > 0 {
            self.stats.conflicts_detected += conflicts as u64;
        }

        // Calculate overall confidence
        let confidence = self.calculate_confidence(&fused_features, &inputs);

        let perception_id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let perception = FusedPerception {
            id: perception_id,
            sources: input_ids.to_vec(),
            features: fused_features,
            confidence,
            method,
            timestamp: Timestamp::now(),
        };

        self.perceptions.insert(perception_id, perception);
        self.stats.fusions_performed += 1;

        Some(perception_id)
    }

    fn check_time_coherence(&self, inputs: &[&SensorInput]) -> bool {
        if inputs.len() < 2 {
            return true;
        }

        let min_time = inputs.iter().map(|i| i.timestamp.0).min().unwrap_or(0);
        let max_time = inputs.iter().map(|i| i.timestamp.0).max().unwrap_or(0);

        max_time - min_time <= self.config.time_window_ns
    }

    fn extract_features(&self, input: &SensorInput) -> BTreeMap<String, f64> {
        let mut features = BTreeMap::new();

        match &input.data {
            SensorData::Vector(v) => {
                for (i, val) in v.iter().enumerate() {
                    features.insert(format!("dim_{}", i), *val);
                }
            }

            SensorData::Structured(map) => {
                for (k, v) in map {
                    features.insert(k.clone(), *v);
                }
            }

            SensorData::Text(t) => {
                features.insert("length".into(), t.len() as f64);
                features.insert("word_count".into(), t.split_whitespace().count() as f64);
            }

            SensorData::Matrix(m) => {
                let rows = m.len() as f64;
                let cols = m.first().map(|r| r.len()).unwrap_or(0) as f64;
                features.insert("rows".into(), rows);
                features.insert("cols".into(), cols);

                // Mean
                let total: f64 = m.iter().flat_map(|r| r.iter()).sum();
                let count = (rows * cols) as f64;
                if count > 0.0 {
                    features.insert("mean".into(), total / count);
                }
            }

            SensorData::Categorical(cats) => {
                features.insert("count".into(), cats.len() as f64);
            }
        }

        features
    }

    fn get_sensor_weight(&self, sensor_id: &str, confidence: f64) -> f64 {
        self.sensors.get(sensor_id)
            .map(|s| s.reliability * confidence)
            .unwrap_or(confidence)
    }

    fn fuse_feature(
        &self,
        name: &str,
        contributions: &[Contribution],
        method: FusionMethod,
    ) -> Option<FusedFeature> {
        if contributions.is_empty() {
            return None;
        }

        let value = match method {
            FusionMethod::Average => {
                let sum: f64 = contributions.iter().map(|c| c.value).sum();
                sum / contributions.len() as f64
            }

            FusionMethod::WeightedAverage => {
                let weighted_sum: f64 = contributions.iter()
                    .map(|c| c.value * c.weight)
                    .sum();
                let weight_sum: f64 = contributions.iter()
                    .map(|c| c.weight)
                    .sum();

                if weight_sum > 0.0 {
                    weighted_sum / weight_sum
                } else {
                    contributions.iter().map(|c| c.value).sum::<f64>() / contributions.len() as f64
                }
            }

            FusionMethod::Voting => {
                // Mode-like voting
                let mut votes = BTreeMap::new();
                for c in contributions {
                    let key = (c.value * 100.0) as i64;
                    *votes.entry(key).or_insert(0) += 1;
                }

                votes.into_iter()
                    .max_by_key(|(_, count)| *count)
                    .map(|(key, _)| key as f64 / 100.0)
                    .unwrap_or(0.0)
            }

            _ => {
                // Default to weighted average for other methods
                let weighted_sum: f64 = contributions.iter()
                    .map(|c| c.value * c.weight)
                    .sum();
                let weight_sum: f64 = contributions.iter()
                    .map(|c| c.weight)
                    .sum();

                if weight_sum > 0.0 { weighted_sum / weight_sum } else { 0.0 }
            }
        };

        // Calculate agreement
        let mean = contributions.iter().map(|c| c.value).sum::<f64>() / contributions.len() as f64;
        let variance = contributions.iter()
            .map(|c| (c.value - mean).powi(2))
            .sum::<f64>() / contributions.len() as f64;

        let agreement = 1.0 / (1.0 + variance);

        Some(FusedFeature {
            name: name.into(),
            value,
            contributions: contributions.to_vec(),
            agreement,
        })
    }

    fn detect_conflicts(&self, features: &BTreeMap<String, FusedFeature>) -> usize {
        features.values()
            .filter(|f| f.agreement < self.config.conflict_threshold)
            .count()
    }

    fn calculate_confidence(
        &self,
        features: &BTreeMap<String, FusedFeature>,
        inputs: &[&SensorInput],
    ) -> f64 {
        if features.is_empty() {
            return 0.0;
        }

        // Average of input confidences weighted by agreement
        let input_confidence: f64 = inputs.iter().map(|i| i.confidence).sum::<f64>() / inputs.len() as f64;
        let agreement: f64 = features.values().map(|f| f.agreement).sum::<f64>() / features.len() as f64;

        input_confidence * agreement
    }

    /// Get perception
    pub fn get_perception(&self, id: u64) -> Option<&FusedPerception> {
        self.perceptions.get(&id)
    }

    /// Get statistics
    pub fn stats(&self) -> &FusionStats {
        &self.stats
    }
}

impl Default for FusionEngine {
    fn default() -> Self {
        Self::new(FusionConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_sensor() {
        let mut engine = FusionEngine::default();

        engine.register_sensor(SensorModel {
            sensor_id: "sensor1".into(),
            modality: Modality::Numeric,
            reliability: 0.9,
            bias: 0.0,
            variance: 0.1,
        });

        assert!(engine.sensors.contains_key("sensor1"));
    }

    #[test]
    fn test_create_input() {
        let mut engine = FusionEngine::default();

        let id = engine.create_input(
            "sensor1",
            Modality::Numeric,
            SensorData::Vector(vec![1.0, 2.0, 3.0]),
            0.9,
        );

        assert!(engine.inputs.contains_key(&id));
    }

    #[test]
    fn test_fuse() {
        let mut engine = FusionEngine::default();

        let id1 = engine.create_input(
            "sensor1",
            Modality::Numeric,
            SensorData::Vector(vec![1.0, 2.0]),
            0.9,
        );

        let id2 = engine.create_input(
            "sensor2",
            Modality::Numeric,
            SensorData::Vector(vec![1.1, 2.1]),
            0.85,
        );

        let perception_id = engine.fuse(&[id1, id2], None);
        assert!(perception_id.is_some());

        let perception = engine.get_perception(perception_id.unwrap()).unwrap();
        assert!(!perception.features.is_empty());
    }

    #[test]
    fn test_weighted_average() {
        let mut engine = FusionEngine::default();

        engine.register_sensor(SensorModel {
            sensor_id: "reliable".into(),
            modality: Modality::Numeric,
            reliability: 1.0,
            bias: 0.0,
            variance: 0.0,
        });

        engine.register_sensor(SensorModel {
            sensor_id: "unreliable".into(),
            modality: Modality::Numeric,
            reliability: 0.1,
            bias: 0.0,
            variance: 0.5,
        });

        let id1 = engine.create_input(
            "reliable",
            Modality::Numeric,
            SensorData::Vector(vec![10.0]),
            1.0,
        );

        let id2 = engine.create_input(
            "unreliable",
            Modality::Numeric,
            SensorData::Vector(vec![0.0]),
            0.5,
        );

        let perception_id = engine.fuse(&[id1, id2], Some(FusionMethod::WeightedAverage)).unwrap();
        let perception = engine.get_perception(perception_id).unwrap();

        // Fused value should be closer to reliable sensor's value
        let dim0 = perception.features.get("dim_0").unwrap();
        assert!(dim0.value > 5.0);
    }
}
