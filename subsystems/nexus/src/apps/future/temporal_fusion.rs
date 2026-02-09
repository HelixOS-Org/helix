// SPDX-License-Identifier: GPL-2.0
//! # Apps Temporal Fusion Engine
//!
//! Multi-horizon application prediction that fuses forecasts at five temporal
//! scales: next syscall (~microseconds), next 100 ms, next 1 s, next 10 s,
//! and next minute. Each horizon carries its own accuracy tracker, and the
//! fusion layer ensures consistency across horizons — a short-term prediction
//! of "heavy CPU" should not contradict a medium-term prediction of "idle".
//!
//! The engine merges these horizons into a single coherent view of the
//! application's future trajectory and measures fusion accuracy to learn
//! how much weight each horizon deserves.
//!
//! This is the apps engine seeing the future at every timescale simultaneously.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const NUM_HORIZONS: usize = 5;
const MAX_APPS: usize = 256;
const MAX_HISTORY: usize = 256;
const EMA_ALPHA: f64 = 0.10;
const CONSISTENCY_TOLERANCE: f64 = 0.25;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const XORSHIFT_SEED: u64 = 0x1234abcd_5678ef01;

// ============================================================================
// UTILITY FUNCTIONS
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

fn ema_update(current: f64, sample: f64, alpha: f64) -> f64 {
    alpha * sample + (1.0 - alpha) * current
}

fn abs_f64(v: f64) -> f64 {
    if v < 0.0 { -v } else { v }
}

// ============================================================================
// HORIZON SCALE
// ============================================================================

/// Temporal horizons for fusion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FusionHorizon {
    NextSyscall,
    HundredMs,
    OneSecond,
    TenSeconds,
    OneMinute,
}

impl FusionHorizon {
    fn as_str(&self) -> &'static str {
        match self {
            Self::NextSyscall => "next_syscall",
            Self::HundredMs => "100ms",
            Self::OneSecond => "1s",
            Self::TenSeconds => "10s",
            Self::OneMinute => "1min",
        }
    }

    fn index(&self) -> usize {
        match self {
            Self::NextSyscall => 0,
            Self::HundredMs => 1,
            Self::OneSecond => 2,
            Self::TenSeconds => 3,
            Self::OneMinute => 4,
        }
    }

    fn from_index(i: usize) -> Self {
        match i % NUM_HORIZONS {
            0 => Self::NextSyscall,
            1 => Self::HundredMs,
            2 => Self::OneSecond,
            3 => Self::TenSeconds,
            _ => Self::OneMinute,
        }
    }

    fn ticks(&self) -> u64 {
        match self {
            Self::NextSyscall => 1,
            Self::HundredMs => 100,
            Self::OneSecond => 1_000,
            Self::TenSeconds => 10_000,
            Self::OneMinute => 60_000,
        }
    }

    fn default_weight(&self) -> f64 {
        match self {
            Self::NextSyscall => 0.35,
            Self::HundredMs => 0.25,
            Self::OneSecond => 0.20,
            Self::TenSeconds => 0.12,
            Self::OneMinute => 0.08,
        }
    }
}

// ============================================================================
// HORIZON PREDICTION
// ============================================================================

/// A prediction at a specific temporal horizon.
#[derive(Debug, Clone)]
pub struct HorizonPrediction {
    pub horizon: FusionHorizon,
    pub predicted_value: f64,
    pub confidence: f64,
    pub tick_issued: u64,
}

// ============================================================================
// FUSED PREDICTION RESULT
// ============================================================================

/// The fused prediction combining all horizons.
#[derive(Debug, Clone)]
pub struct FusedPrediction {
    pub fused_value: f64,
    pub horizon_contributions: Vec<(FusionHorizon, f64, f64)>,
    pub consistency_score: f64,
    pub fused_confidence: f64,
    pub inconsistencies: Vec<(FusionHorizon, FusionHorizon, f64)>,
}

// ============================================================================
// PER-HORIZON TRACKER
// ============================================================================

#[derive(Debug, Clone)]
struct HorizonTracker {
    horizon: FusionHorizon,
    weight: f64,
    ema_error: f64,
    ema_abs_error: f64,
    ema_accuracy: f64,
    prediction_count: u64,
    error_history: Vec<f64>,
    last_prediction: f64,
    last_tick: u64,
}

impl HorizonTracker {
    fn new(horizon: FusionHorizon) -> Self {
        Self {
            horizon,
            weight: horizon.default_weight(),
            ema_error: 0.0,
            ema_abs_error: 0.5,
            ema_accuracy: 0.5,
            prediction_count: 0,
            error_history: Vec::new(),
            last_prediction: 0.0,
            last_tick: 0,
        }
    }

    fn record_prediction(&mut self, value: f64, tick: u64) {
        self.last_prediction = value;
        self.last_tick = tick;
    }

    fn record_outcome(&mut self, actual: f64) {
        let error = actual - self.last_prediction;
        let abs_err = abs_f64(error);
        self.ema_error = ema_update(self.ema_error, error, EMA_ALPHA);
        self.ema_abs_error = ema_update(self.ema_abs_error, abs_err, EMA_ALPHA);
        self.prediction_count += 1;

        let acc = 1.0 / (1.0 + abs_err);
        self.ema_accuracy = ema_update(self.ema_accuracy, acc, EMA_ALPHA);

        if self.error_history.len() >= MAX_HISTORY {
            self.error_history.remove(0);
        }
        self.error_history.push(abs_err);
    }

    fn update_weight(&mut self, inv_err_sum: f64) {
        if inv_err_sum <= 0.0 {
            self.weight = self.horizon.default_weight();
            return;
        }
        let inv = 1.0 / (self.ema_abs_error + 0.001);
        self.weight = inv / inv_err_sum;
    }
}

// ============================================================================
// PER-APP FUSION STATE
// ============================================================================

#[derive(Debug, Clone)]
struct AppFusionState {
    app_id: u64,
    trackers: [HorizonTracker; NUM_HORIZONS],
    consistency_history: Vec<f64>,
    ema_consistency: f64,
    total_fusions: u64,
    last_fused_value: f64,
}

impl AppFusionState {
    fn new(app_id: u64) -> Self {
        Self {
            app_id,
            trackers: [
                HorizonTracker::new(FusionHorizon::NextSyscall),
                HorizonTracker::new(FusionHorizon::HundredMs),
                HorizonTracker::new(FusionHorizon::OneSecond),
                HorizonTracker::new(FusionHorizon::TenSeconds),
                HorizonTracker::new(FusionHorizon::OneMinute),
            ],
            consistency_history: Vec::new(),
            ema_consistency: 1.0,
            total_fusions: 0,
            last_fused_value: 0.0,
        }
    }

    fn rebalance(&mut self) {
        let inv_sum: f64 = self
            .trackers
            .iter()
            .map(|t| 1.0 / (t.ema_abs_error + 0.001))
            .sum();
        for t in &mut self.trackers {
            t.update_weight(inv_sum);
        }
        let w_sum: f64 = self.trackers.iter().map(|t| t.weight).sum();
        if w_sum > 0.0 {
            for t in &mut self.trackers {
                t.weight /= w_sum;
            }
        }
    }
}

// ============================================================================
// TEMPORAL FUSION STATS
// ============================================================================

/// Engine-level statistics for temporal fusion.
#[derive(Debug, Clone)]
pub struct TemporalFusionStats {
    pub total_fusions: u64,
    pub total_outcomes_recorded: u64,
    pub average_consistency: f64,
    pub average_fusion_error: f64,
    pub immediate_accuracy: f64,
    pub medium_accuracy: f64,
    pub long_term_accuracy: f64,
    pub rebalance_count: u64,
}

impl TemporalFusionStats {
    fn new() -> Self {
        Self {
            total_fusions: 0,
            total_outcomes_recorded: 0,
            average_consistency: 1.0,
            average_fusion_error: 0.0,
            immediate_accuracy: 0.5,
            medium_accuracy: 0.5,
            long_term_accuracy: 0.5,
            rebalance_count: 0,
        }
    }
}

// ============================================================================
// APPS TEMPORAL FUSION ENGINE
// ============================================================================

/// Multi-horizon temporal fusion engine for application predictions.
///
/// Fuses predictions from five horizons into a single coherent forecast,
/// tracks consistency across scales, and adapts weights online.
pub struct AppsTemporalFusion {
    app_states: BTreeMap<u64, AppFusionState>,
    stats: TemporalFusionStats,
    rng_state: u64,
    tick: u64,
    ema_consistency_global: f64,
    ema_fusion_error: f64,
}

impl AppsTemporalFusion {
    /// Create a new temporal fusion engine.
    pub fn new() -> Self {
        Self {
            app_states: BTreeMap::new(),
            stats: TemporalFusionStats::new(),
            rng_state: XORSHIFT_SEED,
            tick: 0,
            ema_consistency_global: 1.0,
            ema_fusion_error: 0.5,
        }
    }

    /// Fuse predictions from all horizons into a single forecast.
    pub fn fuse_app_horizons(
        &mut self,
        app_id: u64,
        predictions: &[HorizonPrediction],
    ) -> FusedPrediction {
        self.tick += 1;
        self.stats.total_fusions += 1;

        if self.app_states.len() >= MAX_APPS && !self.app_states.contains_key(&app_id) {
            self.app_states.insert(app_id, AppFusionState::new(app_id));
        }
        let state = self.app_states.get_mut(&app_id).unwrap();
        state.total_fusions += 1;

        // Record each prediction
        for pred in predictions {
            let idx = pred.horizon.index();
            if idx < NUM_HORIZONS {
                state.trackers[idx].record_prediction(pred.predicted_value, self.tick);
            }
        }

        // Weighted combination
        let mut fused = 0.0;
        let mut w_sum = 0.0;
        let mut contributions = Vec::new();

        for pred in predictions {
            let idx = pred.horizon.index();
            let w = if idx < NUM_HORIZONS { state.trackers[idx].weight } else { 0.1 };
            fused += pred.predicted_value * w;
            w_sum += w;
            contributions.push((pred.horizon, pred.predicted_value, w));
        }

        if w_sum > 0.0 {
            fused /= w_sum;
        }

        // Consistency check among pairs
        let (consistency, inconsistencies) = self.check_consistency_internal(predictions);
        state.ema_consistency = ema_update(state.ema_consistency, consistency, EMA_ALPHA);
        if state.consistency_history.len() >= MAX_HISTORY {
            state.consistency_history.remove(0);
        }
        state.consistency_history.push(consistency);
        state.last_fused_value = fused;

        self.ema_consistency_global = ema_update(self.ema_consistency_global, consistency, EMA_ALPHA);
        self.stats.average_consistency = self.ema_consistency_global;

        let confidence = if predictions.is_empty() {
            0.0
        } else {
            let mean_conf: f64 = predictions.iter().map(|p| p.confidence).sum::<f64>() / predictions.len() as f64;
            mean_conf * consistency
        };

        FusedPrediction {
            fused_value: fused,
            horizon_contributions: contributions,
            consistency_score: consistency,
            fused_confidence: confidence,
            inconsistencies,
        }
    }

    fn check_consistency_internal(
        &self,
        predictions: &[HorizonPrediction],
    ) -> (f64, Vec<(FusionHorizon, FusionHorizon, f64)>) {
        let mut inconsistencies = Vec::new();
        let mut total_pairs = 0u64;
        let mut consistent_pairs = 0u64;

        for i in 0..predictions.len() {
            for j in (i + 1)..predictions.len() {
                total_pairs += 1;
                let diff = abs_f64(predictions[i].predicted_value - predictions[j].predicted_value);
                let scale = abs_f64(predictions[i].predicted_value).max(abs_f64(predictions[j].predicted_value)).max(0.001);
                let relative_diff = diff / scale;
                if relative_diff <= CONSISTENCY_TOLERANCE {
                    consistent_pairs += 1;
                } else {
                    inconsistencies.push((
                        predictions[i].horizon,
                        predictions[j].horizon,
                        relative_diff,
                    ));
                }
            }
        }

        let score = if total_pairs > 0 {
            consistent_pairs as f64 / total_pairs as f64
        } else {
            1.0
        };
        (score, inconsistencies)
    }

    /// Produce an immediate (next-syscall) prediction from the tracker.
    pub fn immediate_prediction(&self, app_id: u64) -> f64 {
        match self.app_states.get(&app_id) {
            Some(s) => s.trackers[FusionHorizon::NextSyscall.index()].last_prediction,
            None => 0.0,
        }
    }

    /// Produce a medium-term (1s) prediction from the tracker.
    pub fn medium_term(&self, app_id: u64) -> f64 {
        match self.app_states.get(&app_id) {
            Some(s) => s.trackers[FusionHorizon::OneSecond.index()].last_prediction,
            None => 0.0,
        }
    }

    /// Produce a long-term (1min) forecast from the tracker.
    pub fn long_term_forecast(&self, app_id: u64) -> f64 {
        match self.app_states.get(&app_id) {
            Some(s) => s.trackers[FusionHorizon::OneMinute.index()].last_prediction,
            None => 0.0,
        }
    }

    /// Check consistency across horizons for an app. Returns score in [0,1].
    pub fn consistency_check(&self, app_id: u64) -> f64 {
        match self.app_states.get(&app_id) {
            Some(s) => s.ema_consistency,
            None => 1.0,
        }
    }

    /// Record an actual outcome and update horizon trackers accordingly.
    pub fn record_outcome(&mut self, app_id: u64, actual: f64) {
        self.stats.total_outcomes_recorded += 1;
        let state = match self.app_states.get_mut(&app_id) {
            Some(s) => s,
            None => return,
        };

        for tracker in &mut state.trackers {
            tracker.record_outcome(actual);
        }

        state.rebalance();
        self.stats.rebalance_count += 1;

        // Update per-scale accuracy stats
        self.stats.immediate_accuracy = state.trackers[0].ema_accuracy;
        self.stats.medium_accuracy = state.trackers[2].ema_accuracy;
        self.stats.long_term_accuracy = state.trackers[4].ema_accuracy;

        let fusion_err = abs_f64(actual - state.last_fused_value);
        self.ema_fusion_error = ema_update(self.ema_fusion_error, fusion_err, EMA_ALPHA);
        self.stats.average_fusion_error = self.ema_fusion_error;
    }

    /// Measure fusion accuracy for an app: returns (fusion_accuracy, best_horizon_accuracy).
    pub fn fusion_accuracy(&self, app_id: u64) -> (f64, f64) {
        let state = match self.app_states.get(&app_id) {
            Some(s) => s,
            None => return (0.5, 0.5),
        };

        let best_horizon = state
            .trackers
            .iter()
            .map(|t| t.ema_accuracy)
            .fold(0.0_f64, |a, b| if b > a { b } else { a });

        // Approximate fusion accuracy as consistency-weighted mean
        let mean_acc: f64 = state.trackers.iter().map(|t| t.ema_accuracy * t.weight).sum();
        let fusion_acc = mean_acc * state.ema_consistency;

        (fusion_acc, best_horizon)
    }

    /// Return a snapshot of engine statistics.
    pub fn stats(&self) -> &TemporalFusionStats {
        &self.stats
    }

    /// Number of tracked apps.
    pub fn tracked_apps(&self) -> usize {
        self.app_states.len()
    }

    /// Global EMA-smoothed consistency score.
    pub fn avg_consistency(&self) -> f64 {
        self.ema_consistency_global
    }
}
