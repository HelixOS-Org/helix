// SPDX-License-Identifier: GPL-2.0
//! # Apps Synthesis — Classifier/Predictor Synthesis from Research
//!
//! Translates validated app classification research into concrete classification
//! rules, prediction features, and optimization strategies. Each synthesized
//! artifact is versioned, can be applied atomically, and supports rollback to
//! the previous version. Integration testing verifies that a synthesized
//! improvement actually increases classification accuracy before committing.
//!
//! The engine that turns research papers into production classifiers.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_CLASSIFIERS: usize = 128;
const MAX_FEATURES: usize = 32;
const MAX_HISTORY: usize = 64;
const ROLLBACK_WINDOW: usize = 8;
const EMA_ALPHA: f32 = 0.12;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const MIN_IMPROVEMENT_FOR_COMMIT: f32 = 0.01;
const INTEGRATION_TRIALS: usize = 5;
const VALIDATION_THRESHOLD: f32 = 0.95;

fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

fn abs_f32(x: f32) -> f32 {
    if x < 0.0 { -x } else { x }
}

// ============================================================================
// SYNTHESIS TYPES
// ============================================================================

/// Status of a synthesized classifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClassifierStatus {
    Draft,
    Testing,
    Active,
    RolledBack,
    Retired,
}

/// A synthesized classification feature
#[derive(Debug, Clone)]
pub struct SynthesizedFeature {
    pub name: String,
    pub weight: f32,
    pub source_discovery: u64,
    pub confidence: f32,
}

/// A synthesized classification rule
#[derive(Debug, Clone)]
pub struct ClassificationRule {
    pub rule_id: u64,
    pub name: String,
    pub features: Vec<SynthesizedFeature>,
    pub accuracy: f32,
    pub status: ClassifierStatus,
    pub version: u32,
    pub source_experiment: u64,
    pub created_tick: u64,
}

/// Historical version of a classifier for rollback
#[derive(Debug, Clone)]
struct ClassifierSnapshot {
    rule_id: u64,
    version: u32,
    features: Vec<SynthesizedFeature>,
    accuracy: f32,
    tick: u64,
}

/// Integration test result
#[derive(Debug, Clone, Copy)]
pub struct IntegrationResult {
    pub trial: usize,
    pub baseline_accuracy: f32,
    pub new_accuracy: f32,
    pub improvement: f32,
    pub passed: bool,
}

/// Synthesis validation outcome
#[derive(Debug, Clone)]
pub struct SynthesisValidation {
    pub rule_id: u64,
    pub trials: Vec<IntegrationResult>,
    pub mean_improvement: f32,
    pub all_passed: bool,
    pub committed: bool,
}

// ============================================================================
// SYNTHESIS STATS
// ============================================================================

/// Aggregate synthesis statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct SynthesisStats {
    pub total_synthesized: u64,
    pub total_committed: u64,
    pub total_rolled_back: u64,
    pub total_retired: u64,
    pub total_features_generated: u64,
    pub avg_accuracy_ema: f32,
    pub avg_improvement_ema: f32,
    pub active_classifiers: u64,
    pub rollback_rate: f32,
}

// ============================================================================
// APPS SYNTHESIS ENGINE
// ============================================================================

/// Synthesizes new classifiers and prediction features from research
#[derive(Debug)]
pub struct AppsSynthesis {
    classifiers: BTreeMap<u64, ClassificationRule>,
    history: Vec<ClassifierSnapshot>,
    rng_state: u64,
    current_tick: u64,
    stats: SynthesisStats,
}

impl AppsSynthesis {
    /// Create a new synthesis engine with a seed
    pub fn new(seed: u64) -> Self {
        Self {
            classifiers: BTreeMap::new(),
            history: Vec::new(),
            rng_state: seed | 1,
            current_tick: 0,
            stats: SynthesisStats::default(),
        }
    }

    /// Synthesize a new classifier from research discovery
    pub fn synthesize_classifier(
        &mut self,
        name: String,
        source_experiment: u64,
        feature_names: &[String],
        feature_weights: &[f32],
        discovery_id: u64,
        tick: u64,
    ) -> u64 {
        self.current_tick = tick;
        let id = fnv1a_hash(name.as_bytes()) ^ xorshift64(&mut self.rng_state);

        let count = feature_names.len().min(feature_weights.len()).min(MAX_FEATURES);
        let mut features = Vec::with_capacity(count);
        for i in 0..count {
            features.push(SynthesizedFeature {
                name: feature_names[i].clone(),
                weight: feature_weights[i].clamp(0.0, 1.0),
                source_discovery: discovery_id,
                confidence: 0.5,
            });
            self.stats.total_features_generated += 1;
        }

        let rule = ClassificationRule {
            rule_id: id,
            name,
            features,
            accuracy: 0.0,
            status: ClassifierStatus::Draft,
            version: 1,
            source_experiment,
            created_tick: tick,
        };

        if self.classifiers.len() < MAX_CLASSIFIERS {
            self.classifiers.insert(id, rule);
            self.stats.total_synthesized += 1;
        }
        id
    }

    /// Generate a new feature for an existing classifier
    pub fn generate_feature(
        &mut self,
        rule_id: u64,
        feature_name: String,
        weight: f32,
        discovery_id: u64,
    ) -> bool {
        let rule = match self.classifiers.get_mut(&rule_id) {
            Some(r) => r,
            None => return false,
        };
        if rule.features.len() >= MAX_FEATURES {
            return false;
        }
        // Check for duplicate feature name
        let name_hash = fnv1a_hash(feature_name.as_bytes());
        for existing in &rule.features {
            if fnv1a_hash(existing.name.as_bytes()) == name_hash {
                return false;
            }
        }
        rule.features.push(SynthesizedFeature {
            name: feature_name,
            weight: weight.clamp(0.0, 1.0),
            source_discovery: discovery_id,
            confidence: 0.5,
        });
        self.stats.total_features_generated += 1;
        true
    }

    /// Integrate an improvement: run integration trials and commit if passing
    pub fn integrate_improvement(
        &mut self,
        rule_id: u64,
        baseline_accuracy: f32,
        trial_accuracies: &[f32],
        tick: u64,
    ) -> Option<SynthesisValidation> {
        self.current_tick = tick;
        let rule = match self.classifiers.get(&rule_id) {
            Some(r) => r,
            None => return None,
        };

        // Snapshot current state for potential rollback
        let snapshot = ClassifierSnapshot {
            rule_id,
            version: rule.version,
            features: rule.features.clone(),
            accuracy: rule.accuracy,
            tick,
        };

        let trials_count = trial_accuracies.len().min(INTEGRATION_TRIALS);
        let mut results: Vec<IntegrationResult> = Vec::new();
        let mut total_improvement: f32 = 0.0;
        let mut all_passed = true;

        for i in 0..trials_count {
            let new_acc = trial_accuracies[i];
            let improvement = new_acc - baseline_accuracy;
            let passed = improvement > MIN_IMPROVEMENT_FOR_COMMIT;
            if !passed {
                all_passed = false;
            }
            total_improvement += improvement;
            results.push(IntegrationResult {
                trial: i,
                baseline_accuracy,
                new_accuracy: new_acc,
                improvement,
                passed,
            });
        }

        let mean_improvement = if trials_count > 0 {
            total_improvement / trials_count as f32
        } else {
            0.0
        };

        let committed = all_passed && mean_improvement > MIN_IMPROVEMENT_FOR_COMMIT;

        if committed {
            // Save snapshot before committing
            if self.history.len() < MAX_HISTORY {
                self.history.push(snapshot);
            } else {
                self.history.remove(0);
                self.history.push(snapshot);
            }

            if let Some(rule) = self.classifiers.get_mut(&rule_id) {
                rule.accuracy = baseline_accuracy + mean_improvement;
                rule.status = ClassifierStatus::Active;
                rule.version += 1;
                self.stats.total_committed += 1;
                self.stats.active_classifiers = self
                    .classifiers
                    .values()
                    .filter(|r| r.status == ClassifierStatus::Active)
                    .count() as u64;
            }
        }

        self.stats.avg_improvement_ema =
            EMA_ALPHA * mean_improvement + (1.0 - EMA_ALPHA) * self.stats.avg_improvement_ema;

        Some(SynthesisValidation {
            rule_id,
            trials: results,
            mean_improvement,
            all_passed,
            committed,
        })
    }

    /// Validate a synthesized classifier against accuracy thresholds
    pub fn synthesis_validation(
        &mut self,
        rule_id: u64,
        test_accuracy: f32,
    ) -> bool {
        let rule = match self.classifiers.get_mut(&rule_id) {
            Some(r) => r,
            None => return false,
        };
        let passed = test_accuracy >= VALIDATION_THRESHOLD * rule.accuracy.max(0.5);
        if passed {
            rule.status = ClassifierStatus::Testing;
            // Update feature confidences based on test result
            for feat in rule.features.iter_mut() {
                feat.confidence = EMA_ALPHA * test_accuracy + (1.0 - EMA_ALPHA) * feat.confidence;
            }
        }
        self.stats.avg_accuracy_ema =
            EMA_ALPHA * test_accuracy + (1.0 - EMA_ALPHA) * self.stats.avg_accuracy_ema;
        passed
    }

    /// Rollback a classifier to a previous version
    pub fn rollback_change(&mut self, rule_id: u64, tick: u64) -> bool {
        self.current_tick = tick;

        // Find the most recent snapshot for this rule
        let snapshot_idx = self
            .history
            .iter()
            .rposition(|s| s.rule_id == rule_id);
        let snapshot = match snapshot_idx {
            Some(idx) => self.history.remove(idx),
            None => return false,
        };

        // Check rollback window
        let versions_back = self
            .classifiers
            .get(&rule_id)
            .map(|r| r.version.saturating_sub(snapshot.version))
            .unwrap_or(0);
        if versions_back as usize > ROLLBACK_WINDOW {
            return false;
        }

        if let Some(rule) = self.classifiers.get_mut(&rule_id) {
            rule.features = snapshot.features;
            rule.accuracy = snapshot.accuracy;
            rule.version = snapshot.version;
            rule.status = ClassifierStatus::RolledBack;
            self.stats.total_rolled_back += 1;

            let total = self.stats.total_committed.max(1) as f32;
            self.stats.rollback_rate = self.stats.total_rolled_back as f32 / total;
            self.stats.active_classifiers = self
                .classifiers
                .values()
                .filter(|r| r.status == ClassifierStatus::Active)
                .count() as u64;
            return true;
        }
        false
    }

    /// Retire a classifier
    pub fn retire_classifier(&mut self, rule_id: u64) {
        if let Some(rule) = self.classifiers.get_mut(&rule_id) {
            rule.status = ClassifierStatus::Retired;
            self.stats.total_retired += 1;
            self.stats.active_classifiers = self
                .classifiers
                .values()
                .filter(|r| r.status == ClassifierStatus::Active)
                .count() as u64;
        }
    }

    /// Get aggregate stats
    pub fn stats(&self) -> SynthesisStats {
        self.stats
    }

    /// Get a classifier by id
    pub fn classifier(&self, rule_id: u64) -> Option<&ClassificationRule> {
        self.classifiers.get(&rule_id)
    }

    /// List all active classifiers
    pub fn active_classifiers(&self) -> Vec<&ClassificationRule> {
        self.classifiers
            .values()
            .filter(|r| r.status == ClassifierStatus::Active)
            .collect()
    }
}
