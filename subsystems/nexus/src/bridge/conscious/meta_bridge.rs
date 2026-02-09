// SPDX-License-Identifier: GPL-2.0
//! # Bridge Meta-Cognition
//!
//! Thinks about bridge thinking. Analyzes how the bridge allocates its
//! attention across different decision types, identifies cognitive blind
//! spots where the bridge fails to gather sufficient evidence, and tracks
//! meta-learning rate — how fast the bridge learns to learn.
//!
//! This is the recursive layer: cognition observing cognition, allowing
//! the bridge to optimize its own reasoning process.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.10;
const BLIND_SPOT_THRESHOLD: f32 = 0.25;
const MAX_ATTENTION_CATEGORIES: usize = 32;
const MAX_BLIND_SPOTS: usize = 64;
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
// ATTENTION TRACKING
// ============================================================================

/// How the bridge allocates attention across decision types
#[derive(Debug, Clone)]
pub struct AttentionSlice {
    pub category: String,
    pub category_id: u64,
    /// Fraction of total reasoning cycles spent here (0.0 – 1.0)
    pub fraction: f32,
    /// EMA-smoothed time cost per decision in this category (microseconds)
    pub avg_cost_us: f32,
    /// Number of decisions in this category
    pub decision_count: u64,
    /// Average outcome quality for decisions here
    pub avg_quality: f32,
    /// Efficiency = quality / cost (higher is better)
    pub efficiency: f32,
}

/// A detected cognitive blind spot
#[derive(Debug, Clone)]
pub struct BlindSpot {
    pub id: u64,
    pub description: String,
    /// How severe the blind spot is (0.0 – 1.0)
    pub severity: f32,
    /// How often it goes unnoticed before detection
    pub latency_ticks: u64,
    /// Whether awareness has been raised
    pub acknowledged: bool,
    /// Tick of first detection
    pub detected_tick: u64,
    /// Number of times the blind spot has manifested
    pub occurrences: u64,
}

// ============================================================================
// META-LEARNING STATE
// ============================================================================

/// Tracks the bridge's learning-to-learn trajectory
#[derive(Debug, Clone)]
struct MetaLearningState {
    /// How fast the bridge improves at learning new patterns
    learning_rate: f32,
    /// Previous learning rate for computing meta-learning rate
    prev_learning_rate: f32,
    /// How fast the learning rate itself improves
    meta_rate: f32,
    /// Number of learning episodes
    episodes: u64,
    /// Historical learning rates for trend analysis
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
        self.learning_rate =
            EMA_ALPHA * quality_improvement.abs() + (1.0 - EMA_ALPHA) * self.learning_rate;
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
// META-COGNITION STATS
// ============================================================================

/// Aggregate statistics about the meta-cognitive layer
#[derive(Debug, Clone, Copy, Default)]
pub struct MetaCognitionStats {
    pub attention_categories: usize,
    pub blind_spots_detected: usize,
    pub blind_spots_acknowledged: usize,
    pub meta_learning_rate: f32,
    pub meta_learning_trend: f32,
    pub cognitive_load: f32,l
    pub attention_entropy: f32,
    pub reasoning_efficiency: f32,
}

// ============================================================================
// BRIDGE META-COGNITION ENGINE
// ============================================================================

/// Thinks about bridge thinking — attention allocation, blind spot detection,
/// meta-learning rate tracking, and reasoning optimization.
#[derive(Debug)]
pub struct BridgeMetaCognition {
    /// Attention allocation by category (keyed by FNV hash)
    attention: BTreeMap<u64, AttentionSlice>,
    /// Total reasoning cycles across all categories
    total_cycles: u64,
    /// Detected blind spots (keyed by FNV hash)
    blind_spots: BTreeMap<u64, BlindSpot>,
    /// Meta-learning state
    meta_learning: MetaLearningState,
    /// Current cognitive load (0.0 – 1.0)
    cognitive_load: f32,
    /// Monotonic tick
    tick: u64,
    /// Previous quality snapshot for computing deltas
    prev_quality: f32,
}

impl BridgeMetaCognition {
    pub fn new() -> Self {
        Self {
            attention: BTreeMap::new(),
            total_cycles: 0,
            blind_spots: BTreeMap::new(),
            meta_learning: MetaLearningState::new(),
            cognitive_load: 0.0,
            tick: 0,
            prev_quality: 0.5,
        }
    }

    /// Record attention spent on a category
    pub fn analyze_attention(&mut self, category: &str, cycles_spent: u64, decision_quality: f32) {
        self.tick += 1;
        self.total_cycles += cycles_spent;
        let id = fnv1a_hash(category.as_bytes());

        let slice = self.attention.entry(id).or_insert_with(|| AttentionSlice {
            category: String::from(category),
            category_id: id,
            fraction: 0.0,
            avg_cost_us: 0.0,
            avg_quality: 0.5,
            decision_count: 0,
            efficiency: 1.0,
        });

        slice.decision_count += 1;
        slice.avg_cost_us = EMA_ALPHA * cycles_spent as f32 + (1.0 - EMA_ALPHA) * slice.avg_cost_us;
        slice.avg_quality =
            EMA_ALPHA * decision_quality.max(0.0).min(1.0) + (1.0 - EMA_ALPHA) * slice.avg_quality;

        // Recompute fractions
        let total = self.total_cycles.max(1) as f32;
        for s in self.attention.values_mut() {
            s.fraction = (s.decision_count as f32 * s.avg_cost_us) / total;
            s.efficiency = if s.avg_cost_us > 0.0 {
                s.avg_quality / (s.avg_cost_us / 1000.0).max(0.001)
            } else {
                0.0
            };
        }

        // Track meta-learning: how fast quality is improving
        let delta = decision_quality - self.prev_quality;
        self.meta_learning.record_episode(delta);
        self.prev_quality = decision_quality;
    }

    /// Detect a blind spot — an area where the bridge lacks awareness
    pub fn detect_blind_spot(
        &mut self,
        description: &str,
        severity: f32,
        latency_ticks: u64,
    ) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(description.as_bytes());

        let spot = self.blind_spots.entry(id).or_insert_with(|| BlindSpot {
            id,
            description: String::from(description),
            severity: 0.0,
            latency_ticks: 0,
            acknowledged: false,
            detected_tick: self.tick,
            occurrences: 0,
        });

        spot.severity = EMA_ALPHA * severity.max(0.0).min(1.0) + (1.0 - EMA_ALPHA) * spot.severity;
        spot.latency_ticks = latency_ticks;
        spot.occurrences += 1;
        id
    }

    /// Acknowledge a blind spot (bridge is now aware of it)
    pub fn acknowledge_blind_spot(&mut self, id: u64) {
        if let Some(spot) = self.blind_spots.get_mut(&id) {
            spot.acknowledged = true;
            // Reduce severity over time once acknowledged
            spot.severity *= 0.8;
        }
    }

    /// Trigger a meta-learning step: the bridge reflects on its own learning
    pub fn meta_learn(&mut self, quality_improvement: f32) {
        self.meta_learning.record_episode(quality_improvement);
    }

    /// Suggest optimization: which category deserves more or less attention?
    pub fn optimize_reasoning(&self) -> Vec<(String, f32)> {
        let mut suggestions = Vec::new();
        for slice in self.attention.values() {
            if slice.decision_count < 5 {
                continue;
            }
            // High cost, low quality → reduce or improve
            if slice.avg_cost_us > 100.0 && slice.avg_quality < 0.4 {
                suggestions.push((slice.category.clone(), -0.2));
            }
            // Low cost, high quality → candidate for more investment
            if slice.avg_cost_us < 50.0 && slice.avg_quality > 0.7 {
                suggestions.push((slice.category.clone(), 0.1));
            }
            // Very high fraction but mediocre quality → attention imbalance
            if slice.fraction > 0.3 && slice.avg_quality < 0.6 {
                suggestions.push((slice.category.clone(), -0.15));
            }
        }
        suggestions
    }

    /// Current cognitive load (0.0 – 1.0)
    pub fn cognitive_load(&mut self) -> f32 {
        // Load = active blind spots weight + attention entropy pressure
        let blind_pressure: f32 = self
            .blind_spots
            .values()
            .filter(|b| !b.acknowledged)
            .map(|b| b.severity * 0.1)
            .sum();

        let entropy = self.attention_entropy();
        // High entropy = many categories competing = higher load
        let entropy_pressure = entropy / (self.attention.len().max(1) as f32).max(1.0);

        self.cognitive_load = COGNITIVE_LOAD_DECAY * self.cognitive_load
            + (1.0 - COGNITIVE_LOAD_DECAY) * (blind_pressure + entropy_pressure);
        self.cognitive_load.min(1.0)
    }

    /// Shannon entropy of attention distribution
    fn attention_entropy(&self) -> f32 {
        let total: f32 = self.attention.values().map(|s| s.fraction).sum();
        if total <= 0.0 {
            return 0.0;
        }
        let mut entropy: f32 = 0.0;
        for slice in self.attention.values() {
            let p = slice.fraction / total;
            if p > 0.0 {
                entropy -= p * libm::logf(p);
            }
        }
        entropy
    }

    /// Compute full meta-cognition statistics
    pub fn stats(&mut self) -> MetaCognitionStats {
        let load = self.cognitive_load();
        MetaCognitionStats {
            attention_categories: self.attention.len(),
            blind_spots_detected: self.blind_spots.len(),
            blind_spots_acknowledged: self.blind_spots.values().filter(|b| b.acknowledged).count(),
            meta_learning_rate: self.meta_learning.learning_rate,
            meta_learning_trend: self.meta_learning.trend(),
            cognitive_load: load,
            attention_entropy: self.attention_entropy(),
            reasoning_efficiency: self.attention.values().map(|s| s.efficiency).sum::<f32>()
                / self.attention.len().max(1) as f32,
        }
    }
}
