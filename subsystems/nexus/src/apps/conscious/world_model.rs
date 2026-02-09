// SPDX-License-Identifier: GPL-2.0
//! # Apps World Model
//!
//! The apps engine's model of the application ecosystem. Tracks global app
//! distribution by type, resource demand trends, inter-app interference
//! patterns, and overall ecosystem health. Detects demand surprises —
//! deviations from predicted workload distribution — and estimates ecosystem
//! entropy to gauge system stability.
//!
//! A kernel that models its application ecosystem can predict demand surges
//! and interference hotspots before they manifest as performance degradation.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.10;
const SURPRISE_THRESHOLD: f32 = 0.25;
const MAX_APP_TYPES: usize = 64;
const MAX_INTERFERENCE_PAIRS: usize = 128;
const MAX_DEMAND_HISTORY: usize = 128;
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

/// Xorshift64 PRNG for jitter injection in demand predictions
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

// ============================================================================
// APP TYPE DISTRIBUTION
// ============================================================================

/// Distribution tracking for an application type in the ecosystem
#[derive(Debug, Clone)]
pub struct AppTypeDistribution {
    pub name: String,
    pub id: u64,
    /// Fraction of total processes that are this type (0.0 – 1.0)
    pub fraction: f32,
    /// EMA-smoothed resource demand (0.0 – 1.0)
    pub demand: f32,
    /// Predicted demand at next tick
    pub predicted_demand: f32,
    /// Number of active instances
    pub instance_count: u32,
    /// Observations count
    pub observations: u64,
    /// Last observed surprise magnitude
    pub last_surprise: f32,
    /// Demand history for trend analysis (ring buffer)
    demand_history: Vec<f32>,
    write_idx: usize,
}

impl AppTypeDistribution {
    fn new(name: String) -> Self {
        let id = fnv1a_hash(name.as_bytes());
        Self {
            name,
            id,
            fraction: 0.0,
            demand: 0.0,
            predicted_demand: 0.0,
            instance_count: 0,
            observations: 0,
            last_surprise: 0.0,
            demand_history: Vec::new(),
            write_idx: 0,
        }
    }

    #[inline]
    fn update(&mut self, new_demand: f32, new_count: u32) {
        let clamped = new_demand.max(0.0).min(1.0);
        self.observations += 1;
        self.instance_count = new_count;

        // Surprise detection: how far is reality from prediction
        self.last_surprise = (clamped - self.predicted_demand).abs();

        // EMA update
        self.demand = EMA_ALPHA * clamped + (1.0 - EMA_ALPHA) * self.demand;

        // Simple linear prediction: current + momentum
        let momentum = clamped - self.demand;
        self.predicted_demand = (self.demand + momentum).max(0.0).min(1.0);

        // Ring buffer
        if self.demand_history.len() < MAX_DEMAND_HISTORY {
            self.demand_history.push(clamped);
        } else {
            self.demand_history[self.write_idx] = clamped;
        }
        self.write_idx = (self.write_idx + 1) % MAX_DEMAND_HISTORY;
    }

    /// Trend direction: positive = growing demand, negative = shrinking
    fn trend(&self) -> f32 {
        let len = self.demand_history.len();
        if len < 4 {
            return 0.0;
        }
        let mid = len / 2;
        let early: f32 = self.demand_history[..mid].iter().sum::<f32>() / mid as f32;
        let recent: f32 = self.demand_history[mid..].iter().sum::<f32>() / (len - mid) as f32;
        recent - early
    }
}

// ============================================================================
// INTERFERENCE TRACKING
// ============================================================================

/// Inter-app interference pattern between two app types
#[derive(Debug, Clone)]
pub struct InterferencePair {
    pub type_a_id: u64,
    pub type_b_id: u64,
    pub type_a_name: String,
    pub type_b_name: String,
    /// Interference magnitude (0.0 – 1.0), higher = more interference
    pub magnitude: f32,
    /// Observations of co-running
    pub observations: u64,
    /// Confidence in the interference measurement
    pub confidence: f32,
}

// ============================================================================
// WORLD MODEL STATS
// ============================================================================

/// Aggregate statistics about the app ecosystem world model
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct WorldModelStats {
    pub app_types_tracked: usize,
    pub total_instances: u32,
    pub avg_demand: f32,
    pub total_surprises: u64,
    pub avg_prediction_error: f32,
    pub ecosystem_entropy: f32,
    pub interference_pairs: usize,
    pub ecosystem_health: f32,
}

// ============================================================================
// APPS WORLD MODEL
// ============================================================================

/// The apps engine's model of the application ecosystem — type distribution,
/// demand prediction, interference mapping, and ecosystem health.
#[derive(Debug)]
pub struct AppsWorldModel {
    /// App type distributions keyed by FNV-1a hash
    app_types: BTreeMap<u64, AppTypeDistribution>,
    /// Interference pairs keyed by combined hash of both types
    interference: BTreeMap<u64, InterferencePair>,
    /// Monotonic tick
    tick: u64,
    /// Total surprise events
    total_surprises: u64,
    /// EMA-smoothed ecosystem health (0.0 – 1.0)
    ecosystem_health_ema: f32,
    /// PRNG state for jitter injection
    rng_state: u64,
    /// Demand prediction errors for accuracy tracking
    prediction_errors: Vec<f32>,
    prediction_write_idx: usize,
}

impl AppsWorldModel {
    pub fn new() -> Self {
        Self {
            app_types: BTreeMap::new(),
            interference: BTreeMap::new(),
            tick: 0,
            total_surprises: 0,
            ecosystem_health_ema: 0.5,
            rng_state: 0xDEAD_BEEF_CAFE_1234,
            prediction_errors: Vec::new(),
            prediction_write_idx: 0,
        }
    }

    /// Update the ecosystem with a new observation for an app type
    #[inline]
    pub fn update_ecosystem(&mut self, app_type_name: &str, demand: f32, instance_count: u32) {
        self.tick += 1;
        let id = fnv1a_hash(app_type_name.as_bytes());

        let dist = self
            .app_types
            .entry(id)
            .or_insert_with(|| AppTypeDistribution::new(String::from(app_type_name)));

        // Record prediction error before updating
        let pred_error = (demand.max(0.0).min(1.0) - dist.predicted_demand).abs();
        if self.prediction_errors.len() < MAX_DEMAND_HISTORY {
            self.prediction_errors.push(pred_error);
        } else {
            self.prediction_errors[self.prediction_write_idx] = pred_error;
        }
        self.prediction_write_idx = (self.prediction_write_idx + 1) % MAX_DEMAND_HISTORY;

        dist.update(demand, instance_count);

        if dist.last_surprise > SURPRISE_THRESHOLD {
            self.total_surprises += 1;
        }

        // Recompute fractions
        let total_instances: u32 = self.app_types.values().map(|d| d.instance_count).sum();
        if total_instances > 0 {
            for d in self.app_types.values_mut() {
                d.fraction = d.instance_count as f32 / total_instances as f32;
            }
        }

        // Update ecosystem health: lower surprise → healthier
        let avg_surprise: f32 = self
            .app_types
            .values()
            .map(|d| d.last_surprise)
            .sum::<f32>()
            / self.app_types.len().max(1) as f32;
        let health = 1.0 - avg_surprise.min(1.0);
        self.ecosystem_health_ema =
            EMA_ALPHA * health + (1.0 - EMA_ALPHA) * self.ecosystem_health_ema;
    }

    /// Predict demand for a specific app type with optional jitter
    pub fn predict_demand(&mut self, app_type_name: &str, add_jitter: bool) -> f32 {
        let id = fnv1a_hash(app_type_name.as_bytes());
        let base_prediction = self
            .app_types
            .get(&id)
            .map(|d| d.predicted_demand)
            .unwrap_or(0.0);

        if add_jitter {
            let raw = xorshift64(&mut self.rng_state);
            let jitter = (raw % 100) as f32 / 10000.0 - 0.005; // ±0.5% jitter
            (base_prediction + jitter).max(0.0).min(1.0)
        } else {
            base_prediction
        }
    }

    /// Record interference between two co-running app types
    #[inline]
    pub fn interference_map(&mut self, type_a: &str, type_b: &str, magnitude: f32) {
        self.tick += 1;
        let id_a = fnv1a_hash(type_a.as_bytes());
        let id_b = fnv1a_hash(type_b.as_bytes());
        // Canonical ordering for consistent key
        let (lo, hi) = if id_a <= id_b {
            (id_a, id_b)
        } else {
            (id_b, id_a)
        };
        let pair_key = lo ^ hi.wrapping_mul(FNV_PRIME);

        let pair = self
            .interference
            .entry(pair_key)
            .or_insert_with(|| InterferencePair {
                type_a_id: id_a,
                type_b_id: id_b,
                type_a_name: String::from(type_a),
                type_b_name: String::from(type_b),
                magnitude: 0.0,
                observations: 0,
                confidence: 0.0,
            });

        let clamped = magnitude.max(0.0).min(1.0);
        pair.magnitude = EMA_ALPHA * clamped + (1.0 - EMA_ALPHA) * pair.magnitude;
        pair.observations += 1;
        // Confidence grows with observations, saturates around 50
        pair.confidence = (pair.observations as f32 / 50.0).min(1.0);
    }

    /// Ecosystem entropy: Shannon entropy of the app type distribution
    pub fn ecosystem_entropy(&self) -> f32 {
        let total: f32 = self.app_types.values().map(|d| d.fraction).sum();
        if total <= 0.0 {
            return 0.0;
        }
        let mut entropy: f32 = 0.0;
        for dist in self.app_types.values() {
            let p = dist.fraction / total;
            if p > 0.0 {
                entropy -= p * libm::logf(p);
            }
        }
        entropy
    }

    /// Get the trend direction for a specific app type
    #[inline(always)]
    pub fn trend_direction(&self, app_type_name: &str) -> f32 {
        let id = fnv1a_hash(app_type_name.as_bytes());
        self.app_types.get(&id).map(|d| d.trend()).unwrap_or(0.0)
    }

    /// Get the top interference pairs sorted by magnitude
    #[inline]
    pub fn top_interference(&self, max_results: usize) -> Vec<(String, String, f32)> {
        let mut pairs: Vec<(String, String, f32)> = self
            .interference
            .values()
            .filter(|p| p.confidence > 0.3)
            .map(|p| (p.type_a_name.clone(), p.type_b_name.clone(), p.magnitude))
            .collect();
        pairs.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(core::cmp::Ordering::Equal));
        pairs.truncate(max_results);
        pairs
    }

    /// Compute aggregate world model statistics
    pub fn stats(&self) -> WorldModelStats {
        let total_inst: u32 = self.app_types.values().map(|d| d.instance_count).sum();
        let avg_demand = if self.app_types.is_empty() {
            0.0
        } else {
            self.app_types.values().map(|d| d.demand).sum::<f32>() / self.app_types.len() as f32
        };
        let avg_pred_err = if self.prediction_errors.is_empty() {
            1.0
        } else {
            self.prediction_errors.iter().sum::<f32>() / self.prediction_errors.len() as f32
        };

        WorldModelStats {
            app_types_tracked: self.app_types.len(),
            total_instances: total_inst,
            avg_demand,
            total_surprises: self.total_surprises,
            avg_prediction_error: avg_pred_err,
            ecosystem_entropy: self.ecosystem_entropy(),
            interference_pairs: self.interference.len(),
            ecosystem_health: self.ecosystem_health_ema,
        }
    }
}
