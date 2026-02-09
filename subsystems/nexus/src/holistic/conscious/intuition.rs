// SPDX-License-Identifier: GPL-2.0
//! # Holistic Intuition Engine
//!
//! **SYSTEM-WIDE fast pattern matching.** The most powerful intuition engine:
//! recognizes system-level patterns instantly without deliberate analysis.
//! "This workload mix ALWAYS causes memory pressure in 30 seconds." These
//! snap judgments bypass slow analytical paths and provide immediate guidance
//! that would take full analysis cycles to derive.
//!
//! ## Intuition Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │              HOLISTIC INTUITION                              │
//! ├──────────────────────────────────────────────────────────────┤
//! │  Pattern Library ──▶ Rapid Assessment ──▶ Confidence Check  │
//! │       │                     │                    │           │
//! │       ▼                     ▼                    ▼           │
//! │  Historical patterns   Instant match       "How sure        │
//! │  stored & indexed      against current      am I?"          │
//! │                        system state                         │
//! │                                                             │
//! │  Meta-Intuition: "My intuitions about X are usually right"  │
//! │  Accuracy Tracking: validate intuitions against outcomes    │
//! └──────────────────────────────────────────────────────────────┘
//! ```
//!
//! Intuition is not guessing — it is compressed experience. Each pattern has
//! a validated accuracy score from historical predictions.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.10;
const MAX_PATTERNS: usize = 512;
const MAX_ASSESSMENTS: usize = 128;
const MAX_HISTORY: usize = 256;
const MIN_OBSERVATIONS_FOR_PATTERN: u32 = 3;
const CONFIDENCE_DECAY: f32 = 0.998;
const MATCH_THRESHOLD: f32 = 0.60;
const HIGH_CONFIDENCE: f32 = 0.80;
const META_ACCURACY_WEIGHT: f32 = 0.15;
const PATTERN_SIMILARITY_THRESHOLD: f32 = 0.70;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

// ============================================================================
// FNV-1a HASHING & PRNG
// ============================================================================

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

// ============================================================================
// PATTERN CATEGORY
// ============================================================================

/// Category of system-level pattern recognized by intuition
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PatternCategory {
    /// Workload composition patterns
    WorkloadMix,
    /// Resource pressure patterns
    ResourcePressure,
    /// Failure cascade patterns
    FailureCascade,
    /// Performance anomaly patterns
    PerformanceAnomaly,
    /// Scheduling pattern
    SchedulingPattern,
    /// Memory access pattern
    MemoryAccess,
    /// I/O behavior pattern
    IoBehavior,
    /// Cross-subsystem interaction pattern
    CrossSubsystem,
}

impl PatternCategory {
    pub fn all() -> &'static [PatternCategory] {
        &[
            PatternCategory::WorkloadMix,
            PatternCategory::ResourcePressure,
            PatternCategory::FailureCascade,
            PatternCategory::PerformanceAnomaly,
            PatternCategory::SchedulingPattern,
            PatternCategory::MemoryAccess,
            PatternCategory::IoBehavior,
            PatternCategory::CrossSubsystem,
        ]
    }
}

// ============================================================================
// SYSTEM PATTERN
// ============================================================================

/// A stored system-level pattern that intuition can match against
#[derive(Debug, Clone)]
pub struct SystemPattern {
    pub id: u64,
    pub name: String,
    pub category: PatternCategory,
    /// Feature vector — normalized signature of this pattern
    pub signature: Vec<f32>,
    /// What typically happens when this pattern is detected
    pub typical_outcome: String,
    /// Predicted severity (0.0 = benign, 1.0 = catastrophic)
    pub predicted_severity: f32,
    /// How many times this pattern was observed
    pub observation_count: u32,
    /// How many times the prediction was correct
    pub correct_predictions: u32,
    /// EMA-smoothed accuracy
    pub accuracy: f32,
    /// Time horizon: how far in advance this predicts (in ticks)
    pub prediction_horizon: u64,
    /// Tick when first learned
    pub first_learned_tick: u64,
    /// Tick when last matched
    pub last_matched_tick: u64,
}

impl SystemPattern {
    pub fn new(name: String, category: PatternCategory, signature: Vec<f32>, tick: u64) -> Self {
        let id = fnv1a_hash(name.as_bytes());
        Self {
            id,
            name,
            category,
            signature,
            typical_outcome: String::new(),
            predicted_severity: 0.5,
            observation_count: 0,
            correct_predictions: 0,
            accuracy: 0.5,
            prediction_horizon: 10,
            first_learned_tick: tick,
            last_matched_tick: 0,
        }
    }

    /// Compute similarity between this pattern and a state vector
    pub fn similarity(&self, state: &[f32]) -> f32 {
        if self.signature.len() != state.len() || self.signature.is_empty() {
            return 0.0;
        }
        let mut dot = 0.0f32;
        let mut mag_a = 0.0f32;
        let mut mag_b = 0.0f32;
        for i in 0..self.signature.len() {
            dot += self.signature[i] * state[i];
            mag_a += self.signature[i] * self.signature[i];
            mag_b += state[i] * state[i];
        }
        let denom = mag_a.sqrt() * mag_b.sqrt();
        if denom > 0.0 { dot / denom } else { 0.0 }
    }

    /// Record a prediction outcome
    pub fn record_outcome(&mut self, correct: bool) {
        self.observation_count += 1;
        if correct {
            self.correct_predictions += 1;
        }
        let outcome_val = if correct { 1.0 } else { 0.0 };
        self.accuracy += EMA_ALPHA * (outcome_val - self.accuracy);
    }

    /// Decay confidence over time
    pub fn decay_confidence(&mut self) {
        self.accuracy *= CONFIDENCE_DECAY;
    }
}

// ============================================================================
// RAPID ASSESSMENT
// ============================================================================

/// Result of a rapid intuitive assessment
#[derive(Debug, Clone)]
pub struct RapidAssessment {
    pub matched_pattern_id: u64,
    pub pattern_name: String,
    pub category: PatternCategory,
    pub similarity_score: f32,
    pub confidence: f32,
    pub predicted_outcome: String,
    pub predicted_severity: f32,
    pub horizon_ticks: u64,
    pub tick: u64,
}

// ============================================================================
// META-INTUITION
// ============================================================================

/// Meta-intuition: how accurate are our intuitions per category?
#[derive(Debug, Clone)]
pub struct MetaIntuitionEntry {
    pub category: PatternCategory,
    pub total_predictions: u64,
    pub correct_predictions: u64,
    pub ema_accuracy: f32,
    pub trust_level: f32,
}

impl MetaIntuitionEntry {
    pub fn new(category: PatternCategory) -> Self {
        Self {
            category,
            total_predictions: 0,
            correct_predictions: 0,
            ema_accuracy: 0.5,
            trust_level: 0.5,
        }
    }

    pub fn record(&mut self, correct: bool) {
        self.total_predictions += 1;
        if correct {
            self.correct_predictions += 1;
        }
        let val = if correct { 1.0 } else { 0.0 };
        self.ema_accuracy += META_ACCURACY_WEIGHT * (val - self.ema_accuracy);
        self.trust_level = self.ema_accuracy;
    }
}

// ============================================================================
// STATS
// ============================================================================

/// Intuition engine statistics
#[derive(Debug, Clone)]
pub struct HolisticIntuitionStats {
    pub patterns_stored: u64,
    pub total_assessments: u64,
    pub high_confidence_matches: u64,
    pub average_accuracy: f32,
    pub average_confidence: f32,
    pub best_category_accuracy: f32,
    pub worst_category_accuracy: f32,
    pub meta_evaluations: u64,
}

// ============================================================================
// HOLISTIC INTUITION ENGINE
// ============================================================================

/// System-wide fast pattern matching engine. Recognizes system-level
/// patterns instantly and provides rapid assessments with accuracy tracking.
pub struct HolisticIntuitionEngine {
    /// Pattern library
    patterns: BTreeMap<u64, SystemPattern>,
    /// Recent assessments ring buffer
    assessments: Vec<RapidAssessment>,
    assessment_write_idx: usize,
    /// Meta-intuition per category
    meta_intuition: BTreeMap<u8, MetaIntuitionEntry>,
    /// Stats
    stats: HolisticIntuitionStats,
    /// PRNG
    rng: u64,
    /// Tick
    tick: u64,
}

impl HolisticIntuitionEngine {
    /// Create a new holistic intuition engine
    pub fn new(seed: u64) -> Self {
        let mut meta_intuition = BTreeMap::new();
        for (i, cat) in PatternCategory::all().iter().enumerate() {
            meta_intuition.insert(i as u8, MetaIntuitionEntry::new(*cat));
        }
        let mut assessments = Vec::with_capacity(MAX_ASSESSMENTS);
        for _ in 0..MAX_ASSESSMENTS {
            assessments.push(RapidAssessment {
                matched_pattern_id: 0,
                pattern_name: String::new(),
                category: PatternCategory::WorkloadMix,
                similarity_score: 0.0,
                confidence: 0.0,
                predicted_outcome: String::new(),
                predicted_severity: 0.0,
                horizon_ticks: 0,
                tick: 0,
            });
        }
        Self {
            patterns: BTreeMap::new(),
            assessments,
            assessment_write_idx: 0,
            meta_intuition,
            stats: HolisticIntuitionStats {
                patterns_stored: 0,
                total_assessments: 0,
                high_confidence_matches: 0,
                average_accuracy: 0.5,
                average_confidence: 0.5,
                best_category_accuracy: 0.5,
                worst_category_accuracy: 0.5,
                meta_evaluations: 0,
            },
            rng: seed ^ 0x1A7B_1710_CAFE_BABE,
            tick: 0,
        }
    }

    /// Run the complete system intuition cycle against current state
    pub fn system_intuition(&mut self, state: &[f32], tick: u64) -> Vec<RapidAssessment> {
        self.tick = tick;
        let matches = self.global_pattern_match(state);
        matches
    }

    /// Match state against all patterns in the library
    pub fn global_pattern_match(&mut self, state: &[f32]) -> Vec<RapidAssessment> {
        let mut results = Vec::new();
        let pattern_ids: Vec<u64> = self.patterns.keys().copied().collect();
        for pid in pattern_ids {
            if let Some(pattern) = self.patterns.get_mut(&pid) {
                let sim = pattern.similarity(state);
                if sim >= MATCH_THRESHOLD {
                    let cat_idx = PatternCategory::all()
                        .iter()
                        .position(|c| *c == pattern.category)
                        .unwrap_or(0) as u8;
                    let meta_trust = self
                        .meta_intuition
                        .get(&cat_idx)
                        .map_or(0.5, |m| m.trust_level);
                    let confidence = sim * pattern.accuracy * meta_trust;
                    pattern.last_matched_tick = self.tick;
                    let assessment = RapidAssessment {
                        matched_pattern_id: pattern.id,
                        pattern_name: pattern.name.clone(),
                        category: pattern.category,
                        similarity_score: sim,
                        confidence,
                        predicted_outcome: pattern.typical_outcome.clone(),
                        predicted_severity: pattern.predicted_severity,
                        horizon_ticks: pattern.prediction_horizon,
                        tick: self.tick,
                    };
                    if confidence >= HIGH_CONFIDENCE {
                        self.stats.high_confidence_matches += 1;
                    }
                    results.push(assessment);
                }
            }
        }
        self.stats.total_assessments += 1;
        results
    }

    /// Get overall intuition confidence (average pattern accuracy)
    pub fn intuition_confidence(&self) -> f32 {
        if self.patterns.is_empty() {
            return 0.0;
        }
        let total: f32 = self.patterns.values().map(|p| p.accuracy).sum();
        total / self.patterns.len() as f32
    }

    /// Perform a rapid assessment — snap judgment on a named scenario
    pub fn rapid_assessment(&mut self, scenario: &str, state: &[f32], tick: u64) -> Option<RapidAssessment> {
        self.tick = tick;
        let scenario_hash = fnv1a_hash(scenario.as_bytes());
        // First try exact pattern match
        if let Some(pattern) = self.patterns.get(&scenario_hash) {
            let sim = pattern.similarity(state);
            if sim >= MATCH_THRESHOLD {
                return Some(RapidAssessment {
                    matched_pattern_id: pattern.id,
                    pattern_name: pattern.name.clone(),
                    category: pattern.category,
                    similarity_score: sim,
                    confidence: sim * pattern.accuracy,
                    predicted_outcome: pattern.typical_outcome.clone(),
                    predicted_severity: pattern.predicted_severity,
                    horizon_ticks: pattern.prediction_horizon,
                    tick: self.tick,
                });
            }
        }
        // Fall back to best match across all patterns
        let matches = self.global_pattern_match(state);
        matches.into_iter().max_by(|a, b| {
            a.confidence.partial_cmp(&b.confidence).unwrap_or(core::cmp::Ordering::Equal)
        })
    }

    /// Get overall intuition accuracy
    pub fn intuition_accuracy(&self) -> f32 {
        self.stats.average_accuracy
    }

    /// Get the pattern library summary
    pub fn pattern_library(&self) -> Vec<(u64, String, f32)> {
        self.patterns
            .values()
            .map(|p| (p.id, p.name.clone(), p.accuracy))
            .collect()
    }

    /// Evaluate meta-intuition: how good are we at each category?
    pub fn meta_intuition(&mut self) -> BTreeMap<u8, f32> {
        self.stats.meta_evaluations += 1;
        let mut best: f32 = 0.0;
        let mut worst: f32 = 1.0;
        let mut result = BTreeMap::new();
        for (idx, entry) in &self.meta_intuition {
            result.insert(*idx, entry.ema_accuracy);
            if entry.ema_accuracy > best {
                best = entry.ema_accuracy;
            }
            if entry.ema_accuracy < worst {
                worst = entry.ema_accuracy;
            }
        }
        self.stats.best_category_accuracy = best;
        self.stats.worst_category_accuracy = worst;
        result
    }

    /// Store a new pattern in the library
    pub fn learn_pattern(&mut self, pattern: SystemPattern) {
        if self.patterns.len() < MAX_PATTERNS {
            self.patterns.insert(pattern.id, pattern);
            self.stats.patterns_stored = self.patterns.len() as u64;
        }
    }

    /// Record outcome for a specific pattern
    pub fn record_outcome(&mut self, pattern_id: u64, correct: bool) {
        if let Some(pattern) = self.patterns.get_mut(&pattern_id) {
            let cat_idx = PatternCategory::all()
                .iter()
                .position(|c| *c == pattern.category)
                .unwrap_or(0) as u8;
            pattern.record_outcome(correct);
            if let Some(meta) = self.meta_intuition.get_mut(&cat_idx) {
                meta.record(correct);
            }
            // Update running average accuracy
            let total_acc: f32 = self.patterns.values().map(|p| p.accuracy).sum();
            self.stats.average_accuracy = total_acc / self.patterns.len().max(1) as f32;
        }
    }

    /// Decay all pattern confidences
    pub fn decay_all(&mut self) {
        for (_id, pattern) in self.patterns.iter_mut() {
            pattern.decay_confidence();
        }
    }

    /// Number of stored patterns
    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }

    /// Stats
    pub fn stats(&self) -> &HolisticIntuitionStats {
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
    fn test_pattern_similarity() {
        let p = SystemPattern::new(
            String::from("mem_pressure"),
            PatternCategory::ResourcePressure,
            alloc::vec![0.8, 0.2, 0.9, 0.1],
            1,
        );
        let state = alloc::vec![0.8, 0.2, 0.9, 0.1];
        assert!((p.similarity(&state) - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_engine_creation() {
        let engine = HolisticIntuitionEngine::new(42);
        assert_eq!(engine.pattern_count(), 0);
        assert_eq!(engine.intuition_confidence(), 0.0);
    }

    #[test]
    fn test_learn_and_count() {
        let mut engine = HolisticIntuitionEngine::new(99);
        let p = SystemPattern::new(
            String::from("test_pattern"),
            PatternCategory::WorkloadMix,
            alloc::vec![0.5, 0.5],
            1,
        );
        engine.learn_pattern(p);
        assert_eq!(engine.pattern_count(), 1);
    }

    #[test]
    fn test_fnv1a() {
        assert_eq!(fnv1a_hash(b"intuition"), fnv1a_hash(b"intuition"));
    }
}
