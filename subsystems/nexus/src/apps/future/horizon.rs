// SPDX-License-Identifier: GPL-2.0
//! # Apps Horizon Predictor
//!
//! Long-horizon application behavior prediction engine operating at multiple
//! time scales. Maintains four temporal buckets — 1 second, 1 minute,
//! 10 minutes, 1 hour — each tracking independent resource and phase
//! statistics per process. Predictions at longer horizons carry wider
//! confidence intervals, and the engine tracks its own forecast error
//! by horizon to learn where it can and cannot see clearly.
//!
//! This is the apps engine looking hours ahead and placing resource bets.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_PROCESSES: usize = 512;
const MAX_OBSERVATIONS: usize = 1024;
const NUM_SCALES: usize = 4;
const EMA_ALPHA: f32 = 0.10;
const CONFIDENCE_DECAY_PER_SCALE: f32 = 0.18;
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

fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

// ============================================================================
// TIME SCALES
// ============================================================================

/// Time scale for multi-horizon predictions
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HorizonScale {
    Seconds    = 0,
    Minutes    = 1,
    TenMinutes = 2,
    Hours      = 3,
}

impl HorizonScale {
    fn bucket_ticks(&self) -> u64 {
        match self {
            HorizonScale::Seconds => 100,
            HorizonScale::Minutes => 6_000,
            HorizonScale::TenMinutes => 60_000,
            HorizonScale::Hours => 360_000,
        }
    }

    fn from_index(i: usize) -> Self {
        match i {
            0 => HorizonScale::Seconds,
            1 => HorizonScale::Minutes,
            2 => HorizonScale::TenMinutes,
            _ => HorizonScale::Hours,
        }
    }
}

// ============================================================================
// RESOURCE TYPES
// ============================================================================

/// Resource category tracked per process
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResourceKind {
    CpuCycles,
    MemoryPages,
    IoOps,
    NetworkBytes,
    FileDescriptors,
    ThreadCount,
}

/// Phase of an application lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AppPhase {
    Startup,
    Warming,
    SteadyState,
    BurstLoad,
    WindDown,
    Idle,
    Exiting,
}

// ============================================================================
// OBSERVATION AND PREDICTION TYPES
// ============================================================================

/// An observed resource sample from a process
#[derive(Debug, Clone, Copy)]
pub struct ResourceObservation {
    pub process_id: u64,
    pub tick: u64,
    pub resource: ResourceKind,
    pub value: f32,
    pub phase: AppPhase,
}

/// Per-scale resource pattern for a single process
#[derive(Debug, Clone)]
struct ScalePattern {
    avg_value: f32,
    peak_value: f32,
    trend: f32,
    variance: f32,
    sample_count: u64,
    last_tick: u64,
}

impl ScalePattern {
    fn new() -> Self {
        Self {
            avg_value: 0.0,
            peak_value: 0.0,
            trend: 0.0,
            variance: 0.0,
            sample_count: 0,
            last_tick: 0,
        }
    }

    fn update(&mut self, value: f32, tick: u64) {
        let old_avg = self.avg_value;
        self.avg_value = EMA_ALPHA * value + (1.0 - EMA_ALPHA) * self.avg_value;
        if value > self.peak_value {
            self.peak_value = value;
        }
        let delta = value - old_avg;
        self.variance = EMA_ALPHA * delta * delta + (1.0 - EMA_ALPHA) * self.variance;
        if self.sample_count > 0 && tick > self.last_tick {
            let dt = (tick - self.last_tick) as f32;
            let new_trend = (value - old_avg) / dt;
            self.trend = EMA_ALPHA * new_trend + (1.0 - EMA_ALPHA) * self.trend;
        }
        self.sample_count += 1;
        self.last_tick = tick;
    }

    fn forecast(&self, ticks_ahead: u64) -> f32 {
        self.avg_value + self.trend * ticks_ahead as f32
    }
}

/// Process-level tracker across all scales and resources
#[derive(Debug, Clone)]
struct ProcessHorizon {
    patterns: BTreeMap<u32, Vec<ScalePattern>>,
    phase_history: Vec<(u64, AppPhase)>,
    current_phase: AppPhase,
    phase_confidence: f32,
}

impl ProcessHorizon {
    fn new() -> Self {
        Self {
            patterns: BTreeMap::new(),
            phase_history: Vec::new(),
            current_phase: AppPhase::Startup,
            phase_confidence: 0.5,
        }
    }

    fn resource_key(resource: ResourceKind, scale: HorizonScale) -> u32 {
        (resource as u32) << 8 | (scale as u32)
    }

    fn record(&mut self, obs: &ResourceObservation) {
        for si in 0..NUM_SCALES {
            let key = Self::resource_key(obs.resource, HorizonScale::from_index(si));
            let scales = self.patterns.entry(key).or_insert_with(|| {
                let mut v = Vec::new();
                for _ in 0..NUM_SCALES {
                    v.push(ScalePattern::new());
                }
                v
            });
            if si < scales.len() {
                scales[si].update(obs.value, obs.tick);
            }
        }
        if obs.phase != self.current_phase {
            self.phase_history.push((obs.tick, obs.phase));
            self.current_phase = obs.phase;
            self.phase_confidence = 0.6;
        } else {
            self.phase_confidence = EMA_ALPHA * 1.0 + (1.0 - EMA_ALPHA) * self.phase_confidence;
            if self.phase_confidence > 1.0 {
                self.phase_confidence = 1.0;
            }
        }
    }
}

/// Predicted future application state
#[derive(Debug, Clone)]
pub struct AppStatePrediction {
    pub process_id: u64,
    pub target_tick: u64,
    pub scale: HorizonScale,
    pub predicted_phase: AppPhase,
    pub confidence: f32,
    pub resource_forecasts: Vec<(ResourceKind, f32)>,
}

/// Resource forecast for a specific horizon
#[derive(Debug, Clone)]
pub struct ResourceForecast {
    pub resource: ResourceKind,
    pub scale: HorizonScale,
    pub predicted_value: f32,
    pub lower_bound: f32,
    pub upper_bound: f32,
    pub confidence: f32,
}

/// Demand curve point
#[derive(Debug, Clone, Copy)]
pub struct DemandPoint {
    pub tick_offset: u64,
    pub predicted_demand: f32,
    pub confidence: f32,
}

// ============================================================================
// HORIZON STATS
// ============================================================================

/// Aggregate horizon prediction statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct HorizonStats {
    pub total_observations: u64,
    pub total_predictions: u64,
    pub tracked_processes: usize,
    pub avg_confidence: f32,
    pub avg_forecast_error: f32,
    pub phase_accuracy: f32,
    pub best_scale_accuracy: f32,
}

// ============================================================================
// APPS HORIZON PREDICTOR
// ============================================================================

/// Long-horizon application behavior prediction engine.
/// Maintains multi-scale temporal models per process and predicts
/// future resource needs, phase transitions, and workload shifts.
#[derive(Debug)]
pub struct AppsHorizonPredictor {
    processes: BTreeMap<u64, ProcessHorizon>,
    total_observations: u64,
    total_predictions: u64,
    tick: u64,
    rng_state: u64,
    error_by_scale: [f32; NUM_SCALES],
    prediction_count_by_scale: [u64; NUM_SCALES],
    global_confidence_ema: f32,
    phase_accuracy_ema: f32,
}

impl AppsHorizonPredictor {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            total_observations: 0,
            total_predictions: 0,
            tick: 0,
            rng_state: 0xDEAD_BEEF_CAFE_1234,
            error_by_scale: [0.5; NUM_SCALES],
            prediction_count_by_scale: [0; NUM_SCALES],
            global_confidence_ema: 0.5,
            phase_accuracy_ema: 0.5,
        }
    }

    /// Ingest a resource observation for a process
    pub fn observe(&mut self, obs: ResourceObservation) {
        self.tick = obs.tick;
        self.total_observations += 1;
        if self.processes.len() >= MAX_PROCESSES && !self.processes.contains_key(&obs.process_id) {
            return;
        }
        let proc = self
            .processes
            .entry(obs.process_id)
            .or_insert_with(ProcessHorizon::new);
        proc.record(&obs);
    }

    /// Predict future application state at a target tick
    pub fn predict_app_state(&mut self, process_id: u64, target_tick: u64) -> AppStatePrediction {
        self.total_predictions += 1;
        let ticks_ahead = target_tick.saturating_sub(self.tick);
        let scale_idx = if ticks_ahead < 6_000 {
            0
        } else if ticks_ahead < 60_000 {
            1
        } else if ticks_ahead < 360_000 {
            2
        } else {
            3
        };
        let scale = HorizonScale::from_index(scale_idx);
        self.prediction_count_by_scale[scale_idx] += 1;

        let (predicted_phase, confidence, resource_forecasts) =
            if let Some(proc) = self.processes.get(&process_id) {
                let phase = self.extrapolate_phase(proc, ticks_ahead);
                let base_conf =
                    proc.phase_confidence * (1.0 - CONFIDENCE_DECAY_PER_SCALE * scale_idx as f32);
                let conf = base_conf.max(0.05).min(0.99);
                let mut forecasts = Vec::new();
                let resources = [
                    ResourceKind::CpuCycles,
                    ResourceKind::MemoryPages,
                    ResourceKind::IoOps,
                ];
                for &res in &resources {
                    let key = ProcessHorizon::resource_key(res, scale);
                    if let Some(scales) = proc.patterns.get(&key) {
                        if scale_idx < scales.len() {
                            let val = scales[scale_idx].forecast(ticks_ahead);
                            forecasts.push((res, val.max(0.0)));
                        }
                    }
                }
                (phase, conf, forecasts)
            } else {
                (AppPhase::Startup, 0.1, Vec::new())
            };

        self.global_confidence_ema =
            EMA_ALPHA * confidence + (1.0 - EMA_ALPHA) * self.global_confidence_ema;

        AppStatePrediction {
            process_id,
            target_tick,
            scale,
            predicted_phase,
            confidence,
            resource_forecasts,
        }
    }

    /// Forecast a specific resource for a process across a horizon
    pub fn resource_forecast(
        &self,
        process_id: u64,
        resource: ResourceKind,
        scale: HorizonScale,
        ticks_ahead: u64,
    ) -> ResourceForecast {
        let (predicted, variance) = if let Some(proc) = self.processes.get(&process_id) {
            let key = ProcessHorizon::resource_key(resource, scale);
            if let Some(scales) = proc.patterns.get(&key) {
                let si = scale as usize;
                if si < scales.len() {
                    let p = &scales[si];
                    (p.forecast(ticks_ahead), p.variance)
                } else {
                    (0.0, 1.0)
                }
            } else {
                (0.0, 1.0)
            }
        } else {
            (0.0, 1.0)
        };

        let stddev = if variance > 0.0 { variance.sqrt() } else { 0.1 };
        let conf = (1.0 - CONFIDENCE_DECAY_PER_SCALE * scale as usize as f32).max(0.1);

        ResourceForecast {
            resource,
            scale,
            predicted_value: predicted.max(0.0),
            lower_bound: (predicted - 2.0 * stddev).max(0.0),
            upper_bound: predicted + 2.0 * stddev,
            confidence: conf,
        }
    }

    /// Predict the phase transition for a process
    pub fn phase_prediction(&self, process_id: u64) -> (AppPhase, f32) {
        if let Some(proc) = self.processes.get(&process_id) {
            let next = self.extrapolate_phase(proc, 6_000);
            (next, proc.phase_confidence)
        } else {
            (AppPhase::Startup, 0.1)
        }
    }

    /// Generate a demand curve: sequence of predicted demand points over time
    pub fn demand_curve(
        &self,
        process_id: u64,
        resource: ResourceKind,
        steps: usize,
        step_ticks: u64,
    ) -> Vec<DemandPoint> {
        let mut curve = Vec::new();
        if let Some(proc) = self.processes.get(&process_id) {
            let key = ProcessHorizon::resource_key(resource, HorizonScale::Seconds);
            if let Some(scales) = proc.patterns.get(&key) {
                if !scales.is_empty() {
                    let p = &scales[0];
                    for i in 0..steps {
                        let offset = (i as u64 + 1) * step_ticks;
                        let demand = p.forecast(offset).max(0.0);
                        let conf = (1.0 - 0.02 * i as f32).max(0.05);
                        curve.push(DemandPoint {
                            tick_offset: offset,
                            predicted_demand: demand,
                            confidence: conf,
                        });
                    }
                }
            }
        }
        curve
    }

    /// Compute overall horizon confidence by scale
    pub fn horizon_confidence(&self) -> Vec<(HorizonScale, f32, u64)> {
        let mut result = Vec::new();
        for si in 0..NUM_SCALES {
            let scale = HorizonScale::from_index(si);
            let conf = (1.0 - self.error_by_scale[si]).max(0.0).min(1.0);
            result.push((scale, conf, self.prediction_count_by_scale[si]));
        }
        result
    }

    /// Validate a past prediction against actual observation
    pub fn validate_prediction(&mut self, scale_idx: usize, predicted: f32, actual: f32) {
        if scale_idx < NUM_SCALES {
            let error = (predicted - actual).abs() / (actual.abs() + 1.0);
            self.error_by_scale[scale_idx] =
                EMA_ALPHA * error + (1.0 - EMA_ALPHA) * self.error_by_scale[scale_idx];
        }
    }

    /// Validate a phase prediction against actual phase
    pub fn validate_phase(&mut self, predicted: AppPhase, actual: AppPhase) {
        let correct = if predicted == actual { 1.0 } else { 0.0 };
        self.phase_accuracy_ema = EMA_ALPHA * correct + (1.0 - EMA_ALPHA) * self.phase_accuracy_ema;
    }

    /// Get aggregate statistics
    pub fn stats(&self) -> HorizonStats {
        HorizonStats {
            total_observations: self.total_observations,
            total_predictions: self.total_predictions,
            tracked_processes: self.processes.len(),
            avg_confidence: self.global_confidence_ema,
            avg_forecast_error: self.error_by_scale.iter().sum::<f32>() / NUM_SCALES as f32,
            phase_accuracy: self.phase_accuracy_ema,
            best_scale_accuracy: self
                .error_by_scale
                .iter()
                .copied()
                .map(|e| 1.0 - e)
                .fold(0.0_f32, f32::max),
        }
    }

    // ========================================================================
    // INTERNAL HELPERS
    // ========================================================================

    fn extrapolate_phase(&self, proc: &ProcessHorizon, ticks_ahead: u64) -> AppPhase {
        if proc.phase_history.len() < 2 {
            return proc.current_phase;
        }
        let len = proc.phase_history.len();
        let (t1, _p1) = proc.phase_history[len - 2];
        let (t2, p2) = proc.phase_history[len - 1];
        let phase_duration = t2.saturating_sub(t1);
        if phase_duration == 0 {
            return p2;
        }
        let phases_ahead = ticks_ahead / phase_duration.max(1);
        let next_phase = match p2 {
            AppPhase::Startup if phases_ahead >= 1 => AppPhase::Warming,
            AppPhase::Warming if phases_ahead >= 1 => AppPhase::SteadyState,
            AppPhase::SteadyState if phases_ahead >= 2 => AppPhase::BurstLoad,
            AppPhase::BurstLoad if phases_ahead >= 1 => AppPhase::WindDown,
            AppPhase::WindDown if phases_ahead >= 1 => AppPhase::Exiting,
            other => other,
        };
        next_phase
    }
}
