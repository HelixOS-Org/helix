// SPDX-License-Identifier: GPL-2.0
//! # Bridge World Model
//!
//! The bridge's understanding of the OS world. Models global syscall patterns,
//! process ecosystem state, resource availability, and kernel subsystem health.
//! Detects environmental surprises — deviations from predicted state — and
//! estimates overall entropy of the system.
//!
//! A kernel that models its own environment can predict what will happen next
//! and prepare accordingly.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.10;
const SURPRISE_THRESHOLD: f32 = 0.3;
const MAX_SUBSYSTEMS: usize = 64;
const MAX_PREDICTION_HISTORY: usize = 128;
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

/// Xorshift64 PRNG for jitter injection in predictions
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

// ============================================================================
// SUBSYSTEM HEALTH MODEL
// ============================================================================

/// Health state of a kernel subsystem as observed by the bridge
#[derive(Debug, Clone)]
pub struct SubsystemHealth {
    pub name: String,
    pub id: u64,
    /// EMA-smoothed health score (0.0 – 1.0)
    pub health: f32,
    /// Variance in health observations
    pub variance: f32,
    /// Predicted health at next tick
    pub predicted_health: f32,
    /// Number of observations
    pub observations: u64,
    /// Last observed anomaly magnitude
    pub last_surprise: f32,
    /// Cumulative surprise (higher = more unpredictable)
    pub cumulative_surprise: f32,
}

/// Global resource availability snapshot
#[derive(Debug, Clone, Copy, Default)]
pub struct ResourceState {
    /// Fraction of memory available (0.0 – 1.0)
    pub memory_available: f32,
    /// CPU utilization (0.0 – 1.0)
    pub cpu_utilization: f32,
    /// I/O bandwidth saturation (0.0 – 1.0)
    pub io_saturation: f32,
    /// Number of active processes
    pub active_processes: u32,
    /// Syscall rate (per tick)
    pub syscall_rate: f32,
    /// Tick of this snapshot
    pub tick: u64,
}

/// Prediction with confidence for a single dimension
#[derive(Debug, Clone, Copy)]
pub struct Prediction {
    pub predicted: f32,
    pub actual: f32,
    pub error: f32,
    pub confidence: f32,
    pub tick: u64,
}

// ============================================================================
// WORLD MODEL STATS
// ============================================================================

/// Aggregate statistics about the world model
#[derive(Debug, Clone, Copy, Default)]
pub struct WorldModelStats {
    pub subsystems_tracked: usize,
    pub avg_subsystem_health: f32,
    pub total_surprises: u64,
    pub avg_prediction_error: f32,
    pub model_accuracy: f32,
    pub system_entropy: f32,
    pub resource_pressure: f32,
    pub prediction_count: u64,
}

// ============================================================================
// BRIDGE WORLD MODEL
// ============================================================================

/// The bridge's model of the OS environment — subsystem health, resource
/// state, prediction accuracy, and surprise detection.
#[derive(Debug)]
pub struct BridgeWorldModel {
    /// Subsystem health models (keyed by FNV hash)
    subsystems: BTreeMap<u64, SubsystemHealth>,
    /// Current resource state
    resources: ResourceState,
    /// Previous resource state for delta computation
    prev_resources: ResourceState,
    /// Prediction history for accuracy tracking
    predictions: Vec<Prediction>,
    pred_write_idx: usize,
    /// EMA of prediction error
    avg_prediction_error: f32,
    /// Total surprise events detected
    total_surprises: u64,
    /// Monotonic tick
    tick: u64,
    /// PRNG state for prediction jitter
    rng_state: u64,
}

impl BridgeWorldModel {
    pub fn new() -> Self {
        Self {
            subsystems: BTreeMap::new(),
            resources: ResourceState::default(),
            prev_resources: ResourceState::default(),
            predictions: Vec::new(),
            pred_write_idx: 0,
            avg_prediction_error: 0.5,
            total_surprises: 0,
            tick: 0,
            rng_state: 0xDEAD_BEEF_CAFE_F00D,
        }
    }

    /// Update the health state of a kernel subsystem
    pub fn update_state(&mut self, subsystem_name: &str, observed_health: f32) {
        self.tick += 1;
        let clamped = observed_health.max(0.0).min(1.0);
        let id = fnv1a_hash(subsystem_name.as_bytes());

        let sub = self.subsystems.entry(id).or_insert_with(|| SubsystemHealth {
            name: String::from(subsystem_name),
            id,
            health: 0.5,
            variance: 0.0,
            predicted_health: 0.5,
            observations: 0,
            last_surprise: 0.0,
            cumulative_surprise: 0.0,
        });

        // Compute surprise before updating
        let surprise = (clamped - sub.predicted_health).abs();
        sub.last_surprise = surprise;
        if surprise > SURPRISE_THRESHOLD {
            sub.cumulative_surprise += surprise;
            self.total_surprises += 1;
        }

        // Record prediction accuracy
        let pred = Prediction {
            predicted: sub.predicted_health,
            actual: clamped,
            error: surprise,
            confidence: 1.0 - sub.variance.min(1.0),
            tick: self.tick,
        };
        if self.predictions.len() < MAX_PREDICTION_HISTORY {
            self.predictions.push(pred);
        } else {
            self.predictions[self.pred_write_idx] = pred;
        }
        self.pred_write_idx = (self.pred_write_idx + 1) % MAX_PREDICTION_HISTORY;

        // EMA update of global prediction error
        self.avg_prediction_error = EMA_ALPHA * surprise
            + (1.0 - EMA_ALPHA) * self.avg_prediction_error;

        // Update subsystem state
        let diff = clamped - sub.health;
        sub.variance = EMA_ALPHA * diff * diff + (1.0 - EMA_ALPHA) * sub.variance;
        sub.health = EMA_ALPHA * clamped + (1.0 - EMA_ALPHA) * sub.health;
        sub.observations += 1;

        // Predict next health: linear extrapolation + noise
        let trend = clamped - sub.health; // post-EMA residual
        let jitter_raw = xorshift64(&mut self.rng_state);
        let jitter = ((jitter_raw % 100) as f32 / 10000.0) - 0.005; // ±0.005
        sub.predicted_health = (sub.health + trend * 0.5 + jitter).max(0.0).min(1.0);
    }

    /// Update global resource state
    pub fn update_resources(&mut self, resources: ResourceState) {
        self.prev_resources = self.resources;
        self.resources = resources;
        self.resources.tick = self.tick;
    }

    /// Predict the environment state for the next N ticks
    pub fn predict_environment(&mut self, ticks_ahead: u32) -> ResourceState {
        let delta_mem = self.resources.memory_available - self.prev_resources.memory_available;
        let delta_cpu = self.resources.cpu_utilization - self.prev_resources.cpu_utilization;
        let delta_io = self.resources.io_saturation - self.prev_resources.io_saturation;
        let delta_rate = self.resources.syscall_rate - self.prev_resources.syscall_rate;

        let scale = ticks_ahead as f32;
        let jitter_raw = xorshift64(&mut self.rng_state);
        let jitter = ((jitter_raw % 200) as f32 / 10000.0) - 0.01;

        ResourceState {
            memory_available: (self.resources.memory_available + delta_mem * scale * 0.3 + jitter)
                .max(0.0).min(1.0),
            cpu_utilization: (self.resources.cpu_utilization + delta_cpu * scale * 0.3 + jitter)
                .max(0.0).min(1.0),
            io_saturation: (self.resources.io_saturation + delta_io * scale * 0.3 + jitter)
                .max(0.0).min(1.0),
            active_processes: self.resources.active_processes,
            syscall_rate: (self.resources.syscall_rate + delta_rate * scale * 0.3).max(0.0),
            tick: self.tick + ticks_ahead as u64,
        }
    }

    /// Overall model accuracy (1.0 − average prediction error)
    pub fn model_accuracy(&self) -> f32 {
        (1.0 - self.avg_prediction_error).max(0.0)
    }

    /// Detect surprise events: subsystems whose last observation deviated
    /// significantly from prediction
    pub fn surprise_detection(&self) -> Vec<(String, f32)> {
        self.subsystems.values()
            .filter(|s| s.last_surprise > SURPRISE_THRESHOLD)
            .map(|s| (s.name.clone(), s.last_surprise))
            .collect()
    }

    /// Estimate overall system entropy: how unpredictable the OS world is
    pub fn entropy_estimate(&self) -> f32 {
        if self.subsystems.is_empty() {
            return 0.0;
        }

        // Entropy from subsystem variance
        let variance_sum: f32 = self.subsystems.values()
            .map(|s| s.variance)
            .sum();
        let avg_variance = variance_sum / self.subsystems.len() as f32;

        // Entropy from resource volatility
        let resource_delta = (self.resources.memory_available - self.prev_resources.memory_available).abs()
            + (self.resources.cpu_utilization - self.prev_resources.cpu_utilization).abs()
            + (self.resources.io_saturation - self.prev_resources.io_saturation).abs();
        let resource_entropy = resource_delta / 3.0;

        // Combined: weighted average with surprise rate
        let surprise_rate = if self.predictions.is_empty() {
            0.0
        } else {
            self.predictions.iter().filter(|p| p.error > SURPRISE_THRESHOLD).count() as f32
                / self.predictions.len() as f32
        };

        (avg_variance * 0.4 + resource_entropy * 0.3 + surprise_rate * 0.3).min(1.0)
    }

    /// Compute aggregate statistics
    pub fn stats(&self) -> WorldModelStats {
        let avg_health = if self.subsystems.is_empty() {
            0.0
        } else {
            self.subsystems.values().map(|s| s.health).sum::<f32>()
                / self.subsystems.len() as f32
        };

        let resource_pressure = self.resources.cpu_utilization * 0.4
            + self.resources.io_saturation * 0.3
            + (1.0 - self.resources.memory_available) * 0.3;

        WorldModelStats {
            subsystems_tracked: self.subsystems.len(),
            avg_subsystem_health: avg_health,
            total_surprises: self.total_surprises,
            avg_prediction_error: self.avg_prediction_error,
            model_accuracy: self.model_accuracy(),
            system_entropy: self.entropy_estimate(),
            resource_pressure,
            prediction_count: self.predictions.len() as u64,
        }
    }

    /// Get health of a specific subsystem
    pub fn subsystem_health(&self, name: &str) -> Option<f32> {
        let id = fnv1a_hash(name.as_bytes());
        self.subsystems.get(&id).map(|s| s.health)
    }

    /// List all subsystems sorted by health (worst first)
    pub fn subsystem_ranking(&self) -> Vec<(String, f32)> {
        let mut ranking: Vec<(String, f32)> = self.subsystems.values()
            .map(|s| (s.name.clone(), s.health))
            .collect();
        ranking.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal));
        ranking
    }
}
