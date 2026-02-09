// SPDX-License-Identifier: GPL-2.0
//! # Apps Meta-Cognition
//!
//! Meta-reasoning about application understanding. Asks the fundamental
//! questions: "Am I classifying correctly? Are my features good? Should I
//! add new behavioral dimensions?" Evaluates feature importance, detects
//! classification drift where previously reliable features lose predictive
//! power, and tracks cognitive overhead — how much reasoning effort the
//! apps engine spends relative to the quality it achieves.
//!
//! This is the recursive layer: the apps engine thinking about how it
//! thinks about applications.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.10;
const DRIFT_THRESHOLD: f32 = 0.20;
const MAX_FEATURES: usize = 64;
const MAX_DRIFT_HISTORY: usize = 128;
const COGNITIVE_LOAD_DECAY: f32 = 0.95;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

// ============================================================================
// FEATURE TRACKING
// ============================================================================

/// A behavioral feature used for classification
#[derive(Debug, Clone)]
pub struct BehavioralFeature {
    pub name: String,
    pub id: u64,
    /// EMA-smoothed importance score (0.0 – 1.0)
    pub importance: f32,
    /// EMA-smoothed predictive accuracy when this feature is used
    pub predictive_accuracy: f32,
    /// Previous predictive accuracy for drift detection
    pub prev_accuracy: f32,
    /// Number of times this feature has been evaluated
    pub evaluations: u64,
    /// EMA-smoothed computation cost (microseconds)
    pub cost_us: f32,
    /// Efficiency = importance / cost
    pub efficiency: f32,
    /// Historical accuracy for trend analysis
    accuracy_history: Vec<f32>,
    write_idx: usize,
}

impl BehavioralFeature {
    fn new(name: String) -> Self {
        let id = fnv1a_hash(name.as_bytes());
        Self {
            name,
            id,
            importance: 0.5,
            predictive_accuracy: 0.5,
            prev_accuracy: 0.5,
            evaluations: 0,
            cost_us: 1.0,
            efficiency: 0.5,
            accuracy_history: Vec::new(),
            write_idx: 0,
        }
    }

    fn evaluate(&mut self, accuracy: f32, cost_us: f32) {
        let clamped = accuracy.max(0.0).min(1.0);
        self.evaluations += 1;
        self.prev_accuracy = self.predictive_accuracy;
        self.predictive_accuracy =
            EMA_ALPHA * clamped + (1.0 - EMA_ALPHA) * self.predictive_accuracy;
        self.cost_us = EMA_ALPHA * cost_us.max(0.0) + (1.0 - EMA_ALPHA) * self.cost_us;
        self.efficiency = if self.cost_us > 0.001 {
            self.importance / (self.cost_us / 1000.0).max(0.001)
        } else {
            self.importance * 100.0
        };

        if self.accuracy_history.len() < MAX_DRIFT_HISTORY {
            self.accuracy_history.push(clamped);
        } else {
            self.accuracy_history[self.write_idx] = clamped;
        }
        self.write_idx = (self.write_idx + 1) % MAX_DRIFT_HISTORY;
    }

    /// Detect if this feature's predictive power is drifting
    fn drift_magnitude(&self) -> f32 {
        let len = self.accuracy_history.len();
        if len < 8 {
            return 0.0;
        }
        let mid = len / 2;
        let early_avg: f32 =
            self.accuracy_history[..mid].iter().sum::<f32>() / mid as f32;
        let recent_avg: f32 =
            self.accuracy_history[mid..].iter().sum::<f32>() / (len - mid) as f32;
        (early_avg - recent_avg).abs()
    }
}

// ============================================================================
// META-LEARNING STATE
// ============================================================================

/// Tracks the apps engine's learning-to-learn trajectory
#[derive(Debug, Clone)]
struct MetaLearningState {
    learning_rate: f32,
    prev_learning_rate: f32,
    meta_rate: f32,
    episodes: u64,
    rate_history: Vec<f32>,
    max_history: usize,
    write_idx: usize,
}

impl MetaLearningState {
    fn new() -> Self {
        Self {
            learning_rate: 0.01,
            prev_learning_rate: 0.01,
            meta_rate: 0.0,
            episodes: 0,
            rate_history: Vec::new(),
            max_history: 128,
            write_idx: 0,
        }
    }

    fn record_episode(&mut self, quality_improvement: f32) {
        self.episodes += 1;
        self.prev_learning_rate = self.learning_rate;
        self.learning_rate = EMA_ALPHA * quality_improvement.abs()
            + (1.0 - EMA_ALPHA) * self.learning_rate;
        self.meta_rate = self.learning_rate - self.prev_learning_rate;

        if self.rate_history.len() < self.max_history {
            self.rate_history.push(self.learning_rate);
        } else {
            self.rate_history[self.write_idx] = self.learning_rate;
        }
        self.write_idx = (self.write_idx + 1) % self.max_history;
    }

    fn trend(&self) -> f32 {
        let len = self.rate_history.len();
        if len < 4 {
            return 0.0;
        }
        let mid = len / 2;
        let first: f32 = self.rate_history[..mid].iter().sum::<f32>() / mid as f32;
        let second: f32 = self.rate_history[mid..].iter().sum::<f32>() / (len - mid) as f32;
        second - first
    }
}

// ============================================================================
// DRIFT DETECTION
// ============================================================================

/// A detected classification drift event
#[derive(Debug, Clone)]
pub struct DriftEvent {
    pub feature_id: u64,
    pub feature_name: String,
    pub drift_magnitude: f32,
    pub old_accuracy: f32,
    pub new_accuracy: f32,
    pub detected_tick: u64,
}

// ============================================================================
// META-COGNITION STATS
// ============================================================================

/// Aggregate meta-cognition statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct MetaCognitionStats {
    pub features_tracked: usize,
    pub avg_feature_importance: f32,
    pub avg_predictive_accuracy: f32,
    pub drift_events_detected: usize,
    pub meta_learning_rate: f32,
    pub meta_learning_trend: f32,
    pub cognitive_load: f32,
    pub feature_efficiency: f32,
}

// ============================================================================
// APPS META-COGNITION ENGINE
// ============================================================================

/// Thinks about app classification thinking — feature evaluation,
/// drift detection, meta-learning rate, and cognitive overhead tracking.
#[derive(Debug)]
pub struct AppsMetaCognition {
    /// Behavioral features keyed by FNV hash
    features: BTreeMap<u64, BehavioralFeature>,
    /// Detected drift events
    drift_events: Vec<DriftEvent>,
    /// Meta-learning state
    meta_learning: MetaLearningState,
    /// Current cognitive load (0.0 – 1.0)
    cognitive_load: f32,
    /// Monotonic tick
    tick: u64,
    /// Previous overall quality for delta computation
    prev_quality: f32,
    /// Total reasoning cycles spent
    total_cycles: u64,
}

impl AppsMetaCognition {
    pub fn new() -> Self {
        Self {
            features: BTreeMap::new(),
            drift_events: Vec::new(),
            meta_learning: MetaLearningState::new(),
            cognitive_load: 0.0,
            tick: 0,
            prev_quality: 0.5,
            total_cycles: 0,
        }
    }

    /// Evaluate a feature's contribution to classification accuracy
    pub fn evaluate_features(
        &mut self,
        feature_name: &str,
        accuracy: f32,
        importance: f32,
        cost_us: f32,
    ) {
        self.tick += 1;
        let id = fnv1a_hash(feature_name.as_bytes());

        let feature = self.features.entry(id).or_insert_with(|| {
            BehavioralFeature::new(String::from(feature_name))
        });
        feature.importance = EMA_ALPHA * importance.max(0.0).min(1.0)
            + (1.0 - EMA_ALPHA) * feature.importance;
        feature.evaluate(accuracy, cost_us);
    }

    /// Detect classification drift across all features
    pub fn detect_classification_drift(&mut self) -> Vec<DriftEvent> {
        let mut new_drifts = Vec::new();
        for feature in self.features.values() {
            let magnitude = feature.drift_magnitude();
            if magnitude > DRIFT_THRESHOLD && feature.evaluations > 20 {
                let event = DriftEvent {
                    feature_id: feature.id,
                    feature_name: feature.name.clone(),
                    drift_magnitude: magnitude,
                    old_accuracy: feature.prev_accuracy,
                    new_accuracy: feature.predictive_accuracy,
                    detected_tick: self.tick,
                };
                new_drifts.push(event);
            }
        }
        for event in &new_drifts {
            self.drift_events.push(event.clone());
        }
        new_drifts
    }

    /// Trigger a meta-learning step from observed quality improvement
    pub fn meta_learn(&mut self, quality_improvement: f32) {
        let delta = quality_improvement - self.prev_quality;
        self.meta_learning.record_episode(delta);
        self.prev_quality = quality_improvement;
    }

    /// Get feature importance ranking (sorted by importance descending)
    pub fn feature_importance(&self) -> Vec<(String, f32, f32)> {
        let mut ranked: Vec<(String, f32, f32)> = self.features.values()
            .map(|f| (f.name.clone(), f.importance, f.predictive_accuracy))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        ranked
    }

    /// Current cognitive overhead — how much reasoning effort is expended
    pub fn cognitive_overhead(&mut self, cycles_spent: u64) -> f32 {
        self.total_cycles += cycles_spent;

        // Load from drift events + feature complexity
        let drift_pressure: f32 = self.drift_events.iter()
            .filter(|d| self.tick.saturating_sub(d.detected_tick) < 100)
            .map(|d| d.drift_magnitude * 0.1)
            .sum();

        let feature_pressure = (self.features.len() as f32 / MAX_FEATURES as f32).min(1.0) * 0.3;

        self.cognitive_load = COGNITIVE_LOAD_DECAY * self.cognitive_load
            + (1.0 - COGNITIVE_LOAD_DECAY) * (drift_pressure + feature_pressure);
        self.cognitive_load.min(1.0)
    }

    /// Suggest which features to add or remove based on efficiency
    pub fn optimization_suggestions(&self) -> Vec<(String, f32)> {
        let mut suggestions = Vec::new();
        for feature in self.features.values() {
            if feature.evaluations < 10 {
                continue;
            }
            // Low importance, high cost → candidate for removal
            if feature.importance < 0.2 && feature.cost_us > 50.0 {
                suggestions.push((feature.name.clone(), -0.3));
            }
            // High importance, low cost → underutilized
            if feature.importance > 0.7 && feature.cost_us < 20.0 {
                suggestions.push((feature.name.clone(), 0.2));
            }
            // Drifting accuracy → needs investigation
            if feature.drift_magnitude() > DRIFT_THRESHOLD {
                suggestions.push((feature.name.clone(), -0.15));
            }
        }
        suggestions
    }

    /// Compute full meta-cognition statistics
    pub fn stats(&mut self) -> MetaCognitionStats {
        let load = self.cognitive_overhead(0);
        let n = self.features.len().max(1) as f32;
        MetaCognitionStats {
            features_tracked: self.features.len(),
            avg_feature_importance: self.features.values()
                .map(|f| f.importance).sum::<f32>() / n,
            avg_predictive_accuracy: self.features.values()
                .map(|f| f.predictive_accuracy).sum::<f32>() / n,
            drift_events_detected: self.drift_events.len(),
            meta_learning_rate: self.meta_learning.learning_rate,
            meta_learning_trend: self.meta_learning.trend(),
            cognitive_load: load,
            feature_efficiency: self.features.values()
                .map(|f| f.efficiency).sum::<f32>() / n,
        }
    }
}
