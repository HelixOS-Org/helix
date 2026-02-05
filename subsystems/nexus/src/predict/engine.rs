//! Prediction engine
//!
//! This module provides the main PredictionEngine that coordinates
//! feature tracking, decision tree evaluation, and prediction management.

#![allow(dead_code)]
#![allow(clippy::excessive_nesting)]

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::feature::{Feature, FeatureCategory};
use super::prediction::{CrashPrediction, PredictionFactor};
use super::tree::DecisionNode;
use super::types::PredictionKind;

/// The main prediction engine
pub struct PredictionEngine {
    /// Features being tracked
    features: Vec<Feature>,
    /// Decision trees for different prediction kinds
    trees: Vec<(PredictionKind, DecisionNode)>,
    /// Recent predictions
    predictions: Vec<CrashPrediction>,
    /// Maximum predictions to keep
    max_predictions: usize,
    /// Minimum confidence threshold
    min_confidence: f32,
    /// Prediction horizon in milliseconds
    horizon_ms: u64,
    /// Total predictions made
    total_predictions: AtomicU64,
    /// Correct predictions
    correct_predictions: AtomicU64,
    /// Is engine enabled
    enabled: bool,
}

impl PredictionEngine {
    /// Create a new prediction engine
    pub fn new() -> Self {
        Self {
            features: Vec::new(),
            trees: Vec::new(),
            predictions: Vec::new(),
            max_predictions: 1000,
            min_confidence: 0.7,
            horizon_ms: 30_000,
            total_predictions: AtomicU64::new(0),
            correct_predictions: AtomicU64::new(0),
            enabled: true,
        }
    }

    /// Initialize with default features
    pub fn init_default_features(&mut self) {
        // Memory features
        self.add_feature(Feature::new(
            1,
            "memory_free_pages",
            FeatureCategory::Memory,
            100,
        ));
        self.add_feature(Feature::new(
            2,
            "memory_pressure",
            FeatureCategory::Memory,
            100,
        ));
        self.add_feature(Feature::new(
            3,
            "page_fault_rate",
            FeatureCategory::Memory,
            100,
        ));
        self.add_feature(Feature::new(4, "swap_usage", FeatureCategory::Memory, 100));

        // CPU features
        self.add_feature(Feature::new(
            10,
            "cpu_utilization",
            FeatureCategory::Cpu,
            100,
        ));
        self.add_feature(Feature::new(
            11,
            "runqueue_length",
            FeatureCategory::Cpu,
            100,
        ));
        self.add_feature(Feature::new(
            12,
            "context_switch_rate",
            FeatureCategory::Cpu,
            100,
        ));

        // I/O features
        self.add_feature(Feature::new(20, "io_pending", FeatureCategory::Io, 100));
        self.add_feature(Feature::new(21, "io_latency_avg", FeatureCategory::Io, 100));

        // Timing features
        self.add_feature(Feature::new(
            30,
            "interrupt_latency",
            FeatureCategory::Timing,
            100,
        ));
        self.add_feature(Feature::new(
            31,
            "scheduler_latency",
            FeatureCategory::Timing,
            100,
        ));

        // Lock features
        self.add_feature(Feature::new(
            40,
            "lock_contention",
            FeatureCategory::Lock,
            100,
        ));
        self.add_feature(Feature::new(
            41,
            "lock_wait_time",
            FeatureCategory::Lock,
            100,
        ));
    }

    /// Initialize with default decision trees
    pub fn init_default_trees(&mut self) {
        // OOM prediction tree
        let oom_tree = DecisionNode::split(
            2, // memory_pressure
            0.8,
            DecisionNode::split(
                2,
                0.5,
                DecisionNode::leaf(PredictionKind::OutOfMemory, 0.3, 60_000),
                DecisionNode::leaf(PredictionKind::OutOfMemory, 0.6, 30_000),
            ),
            DecisionNode::split(
                3, // page_fault_rate
                1000.0,
                DecisionNode::leaf(PredictionKind::OutOfMemory, 0.7, 15_000),
                DecisionNode::leaf(PredictionKind::OutOfMemory, 0.95, 5_000),
            ),
        );
        self.add_tree(PredictionKind::OutOfMemory, oom_tree);

        // Deadlock prediction tree
        let deadlock_tree = DecisionNode::split(
            40, // lock_contention
            0.9,
            DecisionNode::leaf(PredictionKind::Deadlock, 0.2, 0),
            DecisionNode::split(
                41,       // lock_wait_time
                10_000.0, // 10ms
                DecisionNode::leaf(PredictionKind::Deadlock, 0.5, 30_000),
                DecisionNode::leaf(PredictionKind::Deadlock, 0.85, 10_000),
            ),
        );
        self.add_tree(PredictionKind::Deadlock, deadlock_tree);
    }

    /// Add a feature
    pub fn add_feature(&mut self, feature: Feature) {
        self.features.push(feature);
    }

    /// Add a decision tree
    pub fn add_tree(&mut self, kind: PredictionKind, tree: DecisionNode) {
        self.trees.push((kind, tree));
    }

    /// Update a feature value
    pub fn update_feature(&mut self, feature_id: u16, value: f64) {
        if let Some(feature) = self.features.iter_mut().find(|f| f.id == feature_id) {
            feature.update(value);
        }
    }

    /// Run prediction
    pub fn predict(&mut self) -> Vec<CrashPrediction> {
        if !self.enabled {
            return Vec::new();
        }

        let mut new_predictions = Vec::new();

        for (_kind, tree) in &self.trees {
            if let Some((pred_kind, confidence, ttf)) = tree.evaluate(&self.features) {
                if confidence >= self.min_confidence {
                    let mut prediction = CrashPrediction::new(pred_kind, confidence, ttf);

                    // Add contributing factors
                    for feature in &self.features {
                        if feature.is_anomalous() || feature.trend().is_concerning() {
                            prediction.factors.push(PredictionFactor {
                                name: feature.name.into(),
                                current_value: feature.value,
                                threshold: feature.mean() + 2.0 * feature.std_dev(),
                                trend: feature.trend(),
                                weight: feature.z_score().abs() as f32 / 5.0,
                            });
                        }
                    }

                    self.total_predictions.fetch_add(1, Ordering::Relaxed);
                    new_predictions.push(prediction.clone());

                    // Store prediction
                    if self.predictions.len() >= self.max_predictions {
                        self.predictions.remove(0);
                    }
                    self.predictions.push(prediction);
                }
            }
        }

        new_predictions
    }

    /// Get prediction accuracy
    pub fn accuracy(&self) -> f32 {
        let total = self.total_predictions.load(Ordering::Relaxed);
        let correct = self.correct_predictions.load(Ordering::Relaxed);

        if total == 0 {
            0.0
        } else {
            correct as f32 / total as f32
        }
    }

    /// Validate a prediction (after the fact)
    pub fn validate_prediction(&mut self, prediction_id: u64, correct: bool) {
        if let Some(pred) = self.predictions.iter_mut().find(|p| p.id == prediction_id) {
            pred.validate(correct);
            if correct {
                self.correct_predictions.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Enable/disable engine
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Is engine enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get recent predictions
    pub fn recent_predictions(&self) -> &[CrashPrediction] {
        &self.predictions
    }

    /// Get feature by ID
    pub fn feature(&self, id: u16) -> Option<&Feature> {
        self.features.iter().find(|f| f.id == id)
    }

    /// Get mutable feature by ID
    pub fn feature_mut(&mut self, id: u16) -> Option<&mut Feature> {
        self.features.iter_mut().find(|f| f.id == id)
    }

    /// Get all features
    pub fn features(&self) -> &[Feature] {
        &self.features
    }

    /// Get total predictions count
    pub fn total_predictions(&self) -> u64 {
        self.total_predictions.load(Ordering::Relaxed)
    }

    /// Get correct predictions count
    pub fn correct_predictions(&self) -> u64 {
        self.correct_predictions.load(Ordering::Relaxed)
    }

    /// Set minimum confidence threshold
    pub fn set_min_confidence(&mut self, threshold: f32) {
        self.min_confidence = threshold.clamp(0.0, 1.0);
    }

    /// Get prediction horizon
    pub fn horizon_ms(&self) -> u64 {
        self.horizon_ms
    }
}

impl Default for PredictionEngine {
    fn default() -> Self {
        let mut engine = Self::new();
        engine.init_default_features();
        engine.init_default_trees();
        engine
    }
}
