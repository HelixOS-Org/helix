//! # Data Fusion Engine
//!
//! Fuses data from multiple cognitive domains into coherent insights.
//! Implements multi-source data integration algorithms.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// FUSION TYPES
// ============================================================================

/// Source of data for fusion
#[derive(Debug, Clone)]
pub struct DataSource {
    /// Source ID
    pub id: u64,
    /// Source domain
    pub domain: DomainId,
    /// Data type
    pub data_type: DataType,
    /// Reliability score (0.0 - 1.0)
    pub reliability: f32,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Data
    pub data: FusionData,
}

/// Data type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataType {
    /// Signal measurement
    Signal,
    /// Pattern match
    Pattern,
    /// Causal relationship
    Causal,
    /// Decision option
    Option,
    /// Action effect
    Effect,
    /// Memory recall
    Memory,
    /// Insight
    Insight,
    /// Learning update
    Learning,
}

/// Data for fusion
#[derive(Debug, Clone)]
pub enum FusionData {
    /// Numeric value
    Numeric(NumericData),
    /// Categorical value
    Categorical(CategoricalData),
    /// Temporal data
    Temporal(TemporalData),
    /// Graph data
    Graph(GraphData),
    /// Vector data
    Vector(VectorData),
}

/// Numeric data
#[derive(Debug, Clone)]
pub struct NumericData {
    /// Value
    pub value: f64,
    /// Unit
    pub unit: String,
    /// Uncertainty
    pub uncertainty: f64,
    /// Bounds
    pub bounds: Option<(f64, f64)>,
}

/// Categorical data
#[derive(Debug, Clone)]
pub struct CategoricalData {
    /// Category
    pub category: String,
    /// Confidence
    pub confidence: f32,
    /// Alternatives with probabilities
    pub alternatives: Vec<(String, f32)>,
}

/// Temporal data
#[derive(Debug, Clone)]
pub struct TemporalData {
    /// Start time
    pub start: Timestamp,
    /// End time
    pub end: Option<Timestamp>,
    /// Values over time
    pub values: Vec<(u64, f64)>,
}

/// Graph data
#[derive(Debug, Clone)]
pub struct GraphData {
    /// Nodes
    pub nodes: Vec<u64>,
    /// Edges (from, to, weight)
    pub edges: Vec<(u64, u64, f64)>,
}

/// Vector data
#[derive(Debug, Clone)]
pub struct VectorData {
    /// Dimensions
    pub dimensions: u32,
    /// Values
    pub values: Vec<f64>,
}

/// Fused result
#[derive(Debug, Clone)]
pub struct FusedResult {
    /// Result ID
    pub id: u64,
    /// Contributing sources
    pub sources: Vec<u64>,
    /// Fusion method used
    pub method: FusionMethod,
    /// Result data
    pub data: FusionData,
    /// Confidence
    pub confidence: f32,
    /// Conflict indicator
    pub has_conflict: bool,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Fusion method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FusionMethod {
    /// Weighted average
    WeightedAverage,
    /// Voting
    Voting,
    /// Bayesian update
    Bayesian,
    /// Dempster-Shafer
    DempsterShafer,
    /// Kalman filter
    Kalman,
    /// Neural fusion
    Neural,
    /// Custom
    Custom,
}

// ============================================================================
// FUSION ENGINE
// ============================================================================

/// Fuses data from multiple sources
pub struct FusionEngine {
    /// Pending data sources
    sources: BTreeMap<u64, DataSource>,
    /// Fused results
    results: BTreeMap<u64, FusedResult>,
    /// Next source ID
    next_source_id: AtomicU64,
    /// Next result ID
    next_result_id: AtomicU64,
    /// Configuration
    config: FusionConfig,
    /// Statistics
    stats: FusionStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct FusionConfig {
    /// Minimum sources for fusion
    pub min_sources: usize,
    /// Conflict threshold
    pub conflict_threshold: f64,
    /// Default reliability
    pub default_reliability: f32,
    /// Decay factor for old data
    pub decay_factor: f64,
    /// Maximum age (cycles)
    pub max_age: u64,
}

impl Default for FusionConfig {
    fn default() -> Self {
        Self {
            min_sources: 2,
            conflict_threshold: 0.5,
            default_reliability: 0.8,
            decay_factor: 0.95,
            max_age: 100,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct FusionStats {
    /// Total sources received
    pub total_sources: u64,
    /// Total fusions performed
    pub total_fusions: u64,
    /// Conflicts detected
    pub conflicts: u64,
    /// Average confidence
    pub avg_confidence: f32,
    /// Average sources per fusion
    pub avg_sources: f32,
}

impl FusionEngine {
    /// Create a new fusion engine
    pub fn new(config: FusionConfig) -> Self {
        Self {
            sources: BTreeMap::new(),
            results: BTreeMap::new(),
            next_source_id: AtomicU64::new(1),
            next_result_id: AtomicU64::new(1),
            config,
            stats: FusionStats::default(),
        }
    }

    /// Add a data source
    pub fn add_source(&mut self, source: DataSource) -> u64 {
        let id = if source.id == 0 {
            self.next_source_id.fetch_add(1, Ordering::Relaxed)
        } else {
            source.id
        };

        let mut source = source;
        source.id = id;

        self.sources.insert(id, source);
        self.stats.total_sources += 1;

        id
    }

    /// Fuse sources of a specific type
    pub fn fuse_by_type(
        &mut self,
        data_type: DataType,
        method: FusionMethod,
    ) -> Option<FusedResult> {
        let matching: Vec<_> = self
            .sources
            .values()
            .filter(|s| s.data_type == data_type)
            .cloned()
            .collect();

        if matching.len() < self.config.min_sources {
            return None;
        }

        self.fuse_sources(&matching, method)
    }

    /// Fuse specific sources
    pub fn fuse_sources(
        &mut self,
        sources: &[DataSource],
        method: FusionMethod,
    ) -> Option<FusedResult> {
        if sources.len() < self.config.min_sources {
            return None;
        }

        let result = match method {
            FusionMethod::WeightedAverage => self.fuse_weighted_average(sources),
            FusionMethod::Voting => self.fuse_voting(sources),
            FusionMethod::Bayesian => self.fuse_bayesian(sources),
            FusionMethod::DempsterShafer => self.fuse_dempster_shafer(sources),
            FusionMethod::Kalman => self.fuse_kalman(sources),
            _ => self.fuse_weighted_average(sources),
        };

        if let Some(mut result) = result {
            result.id = self.next_result_id.fetch_add(1, Ordering::Relaxed);
            result.method = method;
            result.timestamp = Timestamp::now();

            // Check for conflicts
            result.has_conflict = self.detect_conflict(sources);
            if result.has_conflict {
                self.stats.conflicts += 1;
            }

            // Update stats
            self.stats.total_fusions += 1;
            self.stats.avg_confidence = (self.stats.avg_confidence
                * (self.stats.total_fusions - 1) as f32
                + result.confidence)
                / self.stats.total_fusions as f32;
            self.stats.avg_sources = (self.stats.avg_sources
                * (self.stats.total_fusions - 1) as f32
                + sources.len() as f32)
                / self.stats.total_fusions as f32;

            self.results.insert(result.id, result.clone());
            Some(result)
        } else {
            None
        }
    }

    /// Weighted average fusion
    fn fuse_weighted_average(&self, sources: &[DataSource]) -> Option<FusedResult> {
        // Get numeric values
        let numerics: Vec<_> = sources
            .iter()
            .filter_map(|s| match &s.data {
                FusionData::Numeric(n) => Some((s.reliability, n.value, n.uncertainty)),
                _ => None,
            })
            .collect();

        if numerics.is_empty() {
            return None;
        }

        let total_weight: f32 = numerics.iter().map(|(r, _, _)| r).sum();
        if total_weight == 0.0 {
            return None;
        }

        let weighted_sum: f64 = numerics.iter().map(|(r, v, _)| *r as f64 * *v).sum();
        let fused_value = weighted_sum / total_weight as f64;

        // Propagate uncertainty
        let weighted_uncertainty: f64 = numerics
            .iter()
            .map(|(r, _, u)| (*r as f64).powi(2) * u.powi(2))
            .sum();
        let fused_uncertainty = (weighted_uncertainty / (total_weight as f64).powi(2)).sqrt();

        Some(FusedResult {
            id: 0,
            sources: sources.iter().map(|s| s.id).collect(),
            method: FusionMethod::WeightedAverage,
            data: FusionData::Numeric(NumericData {
                value: fused_value,
                unit: numerics.first().map(|_| String::new()).unwrap_or_default(),
                uncertainty: fused_uncertainty,
                bounds: None,
            }),
            confidence: total_weight / sources.len() as f32,
            has_conflict: false,
            timestamp: Timestamp::now(),
        })
    }

    /// Voting fusion
    fn fuse_voting(&self, sources: &[DataSource]) -> Option<FusedResult> {
        let categoricals: Vec<_> = sources
            .iter()
            .filter_map(|s| match &s.data {
                FusionData::Categorical(c) => Some((s.reliability, &c.category)),
                _ => None,
            })
            .collect();

        if categoricals.is_empty() {
            return None;
        }

        // Count votes
        let mut votes: BTreeMap<&String, f32> = BTreeMap::new();
        for (reliability, category) in &categoricals {
            *votes.entry(*category).or_default() += reliability;
        }

        // Find winner
        let winner = votes
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(core::cmp::Ordering::Equal))
            .map(|(k, v)| (*k, *v));

        winner.map(|(category, score)| {
            let total: f32 = votes.values().sum();
            let confidence = if total > 0.0 { score / total } else { 0.0 };

            FusedResult {
                id: 0,
                sources: sources.iter().map(|s| s.id).collect(),
                method: FusionMethod::Voting,
                data: FusionData::Categorical(CategoricalData {
                    category: category.clone(),
                    confidence,
                    alternatives: votes
                        .iter()
                        .filter(|(k, _)| **k != category)
                        .map(|(k, v)| ((*k).clone(), *v / total))
                        .collect(),
                }),
                confidence,
                has_conflict: false,
                timestamp: Timestamp::now(),
            }
        })
    }

    /// Bayesian fusion
    fn fuse_bayesian(&self, sources: &[DataSource]) -> Option<FusedResult> {
        // Simplified Bayesian update for numeric data
        self.fuse_weighted_average(sources)
    }

    /// Dempster-Shafer fusion
    fn fuse_dempster_shafer(&self, sources: &[DataSource]) -> Option<FusedResult> {
        // Simplified D-S for categorical data
        self.fuse_voting(sources)
    }

    /// Kalman filter fusion
    fn fuse_kalman(&self, sources: &[DataSource]) -> Option<FusedResult> {
        // Simplified Kalman for temporal numeric data
        let numerics: Vec<_> = sources
            .iter()
            .filter_map(|s| match &s.data {
                FusionData::Numeric(n) => Some((s.reliability, n.value, n.uncertainty)),
                _ => None,
            })
            .collect();

        if numerics.is_empty() {
            return None;
        }

        // Kalman-style update
        let mut estimate = numerics[0].1;
        let mut variance = numerics[0].2.powi(2);

        for (_, measurement, uncertainty) in numerics.iter().skip(1) {
            let measurement_var = uncertainty.powi(2);
            let kalman_gain = variance / (variance + measurement_var);
            estimate = estimate + kalman_gain * (measurement - estimate);
            variance = (1.0 - kalman_gain) * variance;
        }

        Some(FusedResult {
            id: 0,
            sources: sources.iter().map(|s| s.id).collect(),
            method: FusionMethod::Kalman,
            data: FusionData::Numeric(NumericData {
                value: estimate,
                unit: String::new(),
                uncertainty: variance.sqrt(),
                bounds: None,
            }),
            confidence: 1.0 - (variance.sqrt() / estimate.abs().max(1.0)) as f32,
            has_conflict: false,
            timestamp: Timestamp::now(),
        })
    }

    /// Detect conflicts between sources
    fn detect_conflict(&self, sources: &[DataSource]) -> bool {
        // Check for numeric conflicts
        let numerics: Vec<_> = sources
            .iter()
            .filter_map(|s| match &s.data {
                FusionData::Numeric(n) => Some(n.value),
                _ => None,
            })
            .collect();

        if numerics.len() >= 2 {
            let mean: f64 = numerics.iter().sum::<f64>() / numerics.len() as f64;
            let variance: f64 =
                numerics.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / numerics.len() as f64;
            let std_dev = variance.sqrt();

            if std_dev / mean.abs().max(1.0) > self.config.conflict_threshold {
                return true;
            }
        }

        // Check for categorical conflicts
        let categories: Vec<_> = sources
            .iter()
            .filter_map(|s| match &s.data {
                FusionData::Categorical(c) => Some(&c.category),
                _ => None,
            })
            .collect();

        if categories.len() >= 2 {
            let first = categories[0];
            if categories.iter().any(|c| *c != first) {
                return true;
            }
        }

        false
    }

    /// Get fused result
    pub fn get_result(&self, result_id: u64) -> Option<&FusedResult> {
        self.results.get(&result_id)
    }

    /// Clear old sources
    pub fn cleanup(&mut self, current_cycle: u64) {
        let threshold = current_cycle.saturating_sub(self.config.max_age);
        self.sources
            .retain(|_, s| s.timestamp.as_cycles() >= threshold);
    }

    /// Get statistics
    pub fn stats(&self) -> &FusionStats {
        &self.stats
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weighted_average_fusion() {
        let config = FusionConfig::default();
        let mut engine = FusionEngine::new(config);

        let sources = vec![
            DataSource {
                id: 0,
                domain: DomainId::new(1),
                data_type: DataType::Signal,
                reliability: 0.8,
                timestamp: Timestamp::now(),
                data: FusionData::Numeric(NumericData {
                    value: 100.0,
                    unit: "Hz".into(),
                    uncertainty: 5.0,
                    bounds: None,
                }),
            },
            DataSource {
                id: 0,
                domain: DomainId::new(2),
                data_type: DataType::Signal,
                reliability: 0.9,
                timestamp: Timestamp::now(),
                data: FusionData::Numeric(NumericData {
                    value: 105.0,
                    unit: "Hz".into(),
                    uncertainty: 3.0,
                    bounds: None,
                }),
            },
        ];

        for s in &sources {
            engine.add_source(s.clone());
        }

        let result = engine.fuse_sources(&sources, FusionMethod::WeightedAverage);
        assert!(result.is_some());
    }

    #[test]
    fn test_voting_fusion() {
        let config = FusionConfig::default();
        let mut engine = FusionEngine::new(config);

        let sources = vec![
            DataSource {
                id: 0,
                domain: DomainId::new(1),
                data_type: DataType::Pattern,
                reliability: 0.8,
                timestamp: Timestamp::now(),
                data: FusionData::Categorical(CategoricalData {
                    category: "anomaly".into(),
                    confidence: 0.9,
                    alternatives: Vec::new(),
                }),
            },
            DataSource {
                id: 0,
                domain: DomainId::new(2),
                data_type: DataType::Pattern,
                reliability: 0.9,
                timestamp: Timestamp::now(),
                data: FusionData::Categorical(CategoricalData {
                    category: "anomaly".into(),
                    confidence: 0.85,
                    alternatives: Vec::new(),
                }),
            },
        ];

        let result = engine.fuse_sources(&sources, FusionMethod::Voting);
        assert!(result.is_some());
    }

    #[test]
    fn test_conflict_detection() {
        let config = FusionConfig::default();
        let engine = FusionEngine::new(config);

        let conflicting = vec![
            DataSource {
                id: 1,
                domain: DomainId::new(1),
                data_type: DataType::Signal,
                reliability: 0.9,
                timestamp: Timestamp::now(),
                data: FusionData::Numeric(NumericData {
                    value: 100.0,
                    unit: "Hz".into(),
                    uncertainty: 1.0,
                    bounds: None,
                }),
            },
            DataSource {
                id: 2,
                domain: DomainId::new(2),
                data_type: DataType::Signal,
                reliability: 0.9,
                timestamp: Timestamp::now(),
                data: FusionData::Numeric(NumericData {
                    value: 200.0, // Very different!
                    unit: "Hz".into(),
                    uncertainty: 1.0,
                    bounds: None,
                }),
            },
        ];

        assert!(engine.detect_conflict(&conflicting));
    }
}
