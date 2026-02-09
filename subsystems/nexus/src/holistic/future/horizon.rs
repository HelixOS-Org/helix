// SPDX-License-Identifier: GPL-2.0
//! # Holistic Horizon Predictor
//!
//! System-wide long-horizon prediction engine. Predicts the **complete** system
//! state at multiple time horizons: 1 second, 1 minute, 10 minutes, 1 hour.
//! Fuses predictions from bridge, application, and cooperative subsystems into
//! a single, unified forecast that no individual subsystem could produce.
//!
//! Where sub-predictors forecast their own narrow domain, this module predicts
//! *everything at once* — CPU, memory, I/O, network, process lifecycle — and
//! captures cross-domain interactions that only manifest at the holistic level.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_HORIZONS: usize = 8;
const MAX_SUBSYSTEM_SOURCES: usize = 16;
const MAX_TRAJECTORY_POINTS: usize = 256;
const MAX_INFLECTION_EVENTS: usize = 64;
const MAX_CONFIDENCE_ENTRIES: usize = 512;
const EMA_ALPHA: f32 = 0.12;
const CONFIDENCE_DECAY: f32 = 0.95;
const INFLECTION_SENSITIVITY: f32 = 0.15;
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
// HORIZON SCALE
// ============================================================================

/// Prediction time horizon
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HorizonScale {
    OneSecond,
    OneMinute,
    TenMinutes,
    OneHour,
}

/// Subsystem prediction source
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PredictionSource {
    Bridge,
    Application,
    Cooperative,
    Memory,
    Scheduler,
    Network,
    IO,
    Thermal,
}

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// Predicted state for a single subsystem at a given horizon
#[derive(Debug, Clone)]
pub struct SubsystemForecast {
    pub source: PredictionSource,
    pub horizon: HorizonScale,
    pub cpu_util_pct: f32,
    pub memory_pressure: f32,
    pub io_bandwidth_pct: f32,
    pub network_load_pct: f32,
    pub process_count_delta: i32,
    pub confidence: f32,
    pub tick: u64,
}

/// Unified system state prediction at a particular horizon
#[derive(Debug, Clone)]
pub struct SystemStatePrediction {
    pub id: u64,
    pub horizon: HorizonScale,
    pub fused_cpu_util: f32,
    pub fused_memory_pressure: f32,
    pub fused_io_load: f32,
    pub fused_network_load: f32,
    pub fused_process_delta: i32,
    pub overall_confidence: f32,
    pub source_count: u32,
    pub tick: u64,
}

/// Point on the system trajectory through state-space
#[derive(Debug, Clone)]
pub struct TrajectoryPoint {
    pub tick_offset: u64,
    pub cpu_util: f32,
    pub mem_pressure: f32,
    pub io_load: f32,
    pub net_load: f32,
    pub stability: f32,
}

/// Detected inflection point where system behavior changes qualitatively
#[derive(Debug, Clone)]
pub struct InflectionEvent {
    pub id: u64,
    pub estimated_tick: u64,
    pub dimension: String,
    pub magnitude: f32,
    pub direction_change: f32,
    pub confidence: f32,
}

/// Confidence map entry for a specific prediction dimension
#[derive(Debug, Clone)]
pub struct ConfidenceEntry {
    pub dimension: String,
    pub horizon: HorizonScale,
    pub confidence: f32,
    pub historical_accuracy: f32,
    pub sample_count: u64,
}

/// Decomposition of a horizon prediction into contributing factors
#[derive(Debug, Clone)]
pub struct HorizonDecomposition {
    pub horizon: HorizonScale,
    pub trend_component: f32,
    pub cyclic_component: f32,
    pub noise_component: f32,
    pub cross_domain_component: f32,
    pub residual: f32,
}

// ============================================================================
// STATS
// ============================================================================

/// Aggregate horizon prediction statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct HorizonStats {
    pub total_predictions: u64,
    pub total_fusions: u64,
    pub avg_confidence: f32,
    pub avg_accuracy: f32,
    pub inflection_count: u64,
    pub trajectory_length: usize,
    pub best_horizon_accuracy: f32,
    pub worst_horizon_accuracy: f32,
}

// ============================================================================
// HOLISTIC HORIZON PREDICTOR
// ============================================================================

/// Master long-horizon prediction engine. Fuses all subsystem forecasts into
/// unified system-wide predictions at multiple time horizons.
#[derive(Debug)]
pub struct HolisticHorizonPredictor {
    forecasts: BTreeMap<u64, SubsystemForecast>,
    predictions: BTreeMap<u64, SystemStatePrediction>,
    trajectory: Vec<TrajectoryPoint>,
    inflections: BTreeMap<u64, InflectionEvent>,
    confidence_map: BTreeMap<u64, ConfidenceEntry>,
    decompositions: BTreeMap<u8, HorizonDecomposition>,
    accuracy_by_horizon: BTreeMap<u8, f32>,
    total_predictions: u64,
    total_fusions: u64,
    tick: u64,
    rng_state: u64,
    confidence_ema: f32,
    accuracy_ema: f32,
}

impl HolisticHorizonPredictor {
    pub fn new() -> Self {
        Self {
            forecasts: BTreeMap::new(),
            predictions: BTreeMap::new(),
            trajectory: Vec::new(),
            inflections: BTreeMap::new(),
            confidence_map: BTreeMap::new(),
            decompositions: BTreeMap::new(),
            accuracy_by_horizon: BTreeMap::new(),
            total_predictions: 0,
            total_fusions: 0,
            tick: 0,
            rng_state: 0xA0E1_20B5_CDED_1C70,
            confidence_ema: 0.5,
            accuracy_ema: 0.5,
        }
    }

    /// Predict complete system state at a given horizon by fusing all sources
    pub fn predict_system_state(
        &mut self,
        horizon: HorizonScale,
        source_forecasts: &[SubsystemForecast],
    ) -> SystemStatePrediction {
        self.tick += 1;
        self.total_predictions += 1;

        let mut cpu_sum = 0.0_f32;
        let mut mem_sum = 0.0_f32;
        let mut io_sum = 0.0_f32;
        let mut net_sum = 0.0_f32;
        let mut proc_sum = 0_i32;
        let mut weight_sum = 0.0_f32;
        let mut count = 0_u32;

        for fc in source_forecasts.iter() {
            if fc.horizon != horizon {
                continue;
            }
            let w = fc.confidence.clamp(0.01, 1.0);
            cpu_sum += fc.cpu_util_pct * w;
            mem_sum += fc.memory_pressure * w;
            io_sum += fc.io_bandwidth_pct * w;
            net_sum += fc.network_load_pct * w;
            proc_sum += (fc.process_count_delta as f32 * w) as i32;
            weight_sum += w;
            count += 1;

            let key = fnv1a_hash(format!("{:?}-{:?}", fc.source, fc.horizon).as_bytes());
            self.forecasts.insert(key, fc.clone());
        }

        let denom = if weight_sum > 0.0 { weight_sum } else { 1.0 };
        let overall_conf = if count > 0 {
            (weight_sum / count as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let id = fnv1a_hash(format!("{:?}-{}", horizon, self.tick).as_bytes())
            ^ xorshift64(&mut self.rng_state);

        let prediction = SystemStatePrediction {
            id,
            horizon,
            fused_cpu_util: (cpu_sum / denom).clamp(0.0, 100.0),
            fused_memory_pressure: (mem_sum / denom).clamp(0.0, 1.0),
            fused_io_load: (io_sum / denom).clamp(0.0, 100.0),
            fused_network_load: (net_sum / denom).clamp(0.0, 100.0),
            fused_process_delta: proc_sum,
            overall_confidence: overall_conf,
            source_count: count,
            tick: self.tick,
        };

        self.confidence_ema = EMA_ALPHA * overall_conf + (1.0 - EMA_ALPHA) * self.confidence_ema;
        self.predictions.insert(id, prediction.clone());

        if self.predictions.len() > MAX_CONFIDENCE_ENTRIES {
            if let Some((&oldest, _)) = self.predictions.iter().next() {
                self.predictions.remove(&oldest);
            }
        }

        prediction
    }

    /// Generate a fused forecast across ALL horizons simultaneously
    pub fn fused_forecast(
        &mut self,
        source_forecasts: &[SubsystemForecast],
    ) -> Vec<SystemStatePrediction> {
        self.total_fusions += 1;
        let horizons = [
            HorizonScale::OneSecond,
            HorizonScale::OneMinute,
            HorizonScale::TenMinutes,
            HorizonScale::OneHour,
        ];
        let mut results = Vec::new();
        for &h in &horizons {
            results.push(self.predict_system_state(h, source_forecasts));
        }
        results
    }

    /// Decompose a horizon prediction into trend, cyclic, noise, and cross-domain
    pub fn horizon_decomposition(&mut self, horizon: HorizonScale) -> HorizonDecomposition {
        let horizon_key = horizon as u8;

        let predictions_at_horizon: Vec<&SystemStatePrediction> = self
            .predictions
            .values()
            .filter(|p| p.horizon == horizon)
            .collect();

        let n = predictions_at_horizon.len().max(1) as f32;
        let cpu_vals: Vec<f32> = predictions_at_horizon
            .iter()
            .map(|p| p.fused_cpu_util)
            .collect();

        let mean = cpu_vals.iter().sum::<f32>() / n;
        let trend = if cpu_vals.len() >= 2 {
            let last = *cpu_vals.last().unwrap_or(&mean);
            let first = *cpu_vals.first().unwrap_or(&mean);
            (last - first) / n
        } else {
            0.0
        };

        let cyclic = if cpu_vals.len() >= 4 {
            let mid = cpu_vals.len() / 2;
            let first_half: f32 = cpu_vals[..mid].iter().sum::<f32>() / mid as f32;
            let second_half: f32 =
                cpu_vals[mid..].iter().sum::<f32>() / (cpu_vals.len() - mid) as f32;
            (first_half - second_half).abs() / mean.max(1.0)
        } else {
            0.0
        };

        let variance = cpu_vals.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / n;
        let noise = variance.sqrt() / mean.max(1.0);

        let cross = self.forecasts.len() as f32 / MAX_SUBSYSTEM_SOURCES as f32;

        let decomp = HorizonDecomposition {
            horizon,
            trend_component: trend.clamp(-1.0, 1.0),
            cyclic_component: cyclic.clamp(0.0, 1.0),
            noise_component: noise.clamp(0.0, 1.0),
            cross_domain_component: cross.clamp(0.0, 1.0),
            residual: (1.0 - trend.abs() - cyclic - noise - cross).clamp(0.0, 1.0),
        };

        self.decompositions.insert(horizon_key, decomp.clone());
        decomp
    }

    /// Build a confidence map across all dimensions and horizons
    pub fn prediction_confidence_map(&mut self) -> Vec<ConfidenceEntry> {
        let dimensions = ["cpu", "memory", "io", "network"];
        let horizons = [
            HorizonScale::OneSecond,
            HorizonScale::OneMinute,
            HorizonScale::TenMinutes,
            HorizonScale::OneHour,
        ];

        let mut entries = Vec::new();
        for dim in &dimensions {
            for &h in &horizons {
                let preds: Vec<&SystemStatePrediction> = self
                    .predictions
                    .values()
                    .filter(|p| p.horizon == h)
                    .collect();

                let avg_conf = if preds.is_empty() {
                    0.0
                } else {
                    preds.iter().map(|p| p.overall_confidence).sum::<f32>() / preds.len() as f32
                };

                let decay_factor = match h {
                    HorizonScale::OneSecond => 1.0,
                    HorizonScale::OneMinute => CONFIDENCE_DECAY,
                    HorizonScale::TenMinutes => CONFIDENCE_DECAY * CONFIDENCE_DECAY,
                    HorizonScale::OneHour => CONFIDENCE_DECAY.powi(3),
                };

                let key = fnv1a_hash(format!("{}-{:?}", dim, h).as_bytes());
                let entry = ConfidenceEntry {
                    dimension: String::from(*dim),
                    horizon: h,
                    confidence: (avg_conf * decay_factor).clamp(0.0, 1.0),
                    historical_accuracy: self.accuracy_ema,
                    sample_count: preds.len() as u64,
                };

                self.confidence_map.insert(key, entry.clone());
                entries.push(entry);
            }
        }

        while self.confidence_map.len() > MAX_CONFIDENCE_ENTRIES {
            if let Some((&oldest, _)) = self.confidence_map.iter().next() {
                self.confidence_map.remove(&oldest);
            }
        }

        entries
    }

    /// Compute the system trajectory — a path through state-space over time
    pub fn system_trajectory(&mut self) -> Vec<TrajectoryPoint> {
        let mut sorted: Vec<&SystemStatePrediction> = self.predictions.values().collect();
        sorted.sort_by(|a, b| a.tick.cmp(&b.tick));

        self.trajectory.clear();
        let mut prev_cpu = 50.0_f32;
        let mut prev_mem = 0.5_f32;

        for pred in sorted.iter().take(MAX_TRAJECTORY_POINTS) {
            let stability = 1.0
                - ((pred.fused_cpu_util - prev_cpu).abs() / 100.0
                    + (pred.fused_memory_pressure - prev_mem).abs())
                    / 2.0;

            let point = TrajectoryPoint {
                tick_offset: pred.tick,
                cpu_util: pred.fused_cpu_util,
                mem_pressure: pred.fused_memory_pressure,
                io_load: pred.fused_io_load,
                net_load: pred.fused_network_load,
                stability: stability.clamp(0.0, 1.0),
            };

            prev_cpu = pred.fused_cpu_util;
            prev_mem = pred.fused_memory_pressure;
            self.trajectory.push(point);
        }

        self.trajectory.clone()
    }

    /// Detect inflection points where system trajectory changes qualitatively
    pub fn inflection_detection(&mut self) -> Vec<InflectionEvent> {
        let traj = &self.trajectory;
        let mut events = Vec::new();

        if traj.len() < 3 {
            return events;
        }

        for i in 1..traj.len() - 1 {
            let prev = &traj[i - 1];
            let curr = &traj[i];
            let next = &traj[i + 1];

            let cpu_accel = (next.cpu_util - curr.cpu_util) - (curr.cpu_util - prev.cpu_util);
            let mem_accel =
                (next.mem_pressure - curr.mem_pressure) - (curr.mem_pressure - prev.mem_pressure);

            if cpu_accel.abs() > INFLECTION_SENSITIVITY * 100.0 {
                let id = fnv1a_hash(format!("cpu-inflect-{}", i).as_bytes());
                events.push(InflectionEvent {
                    id,
                    estimated_tick: curr.tick_offset,
                    dimension: String::from("cpu"),
                    magnitude: cpu_accel.abs(),
                    direction_change: cpu_accel,
                    confidence: (1.0 - (1.0 / (cpu_accel.abs() + 1.0))).clamp(0.0, 1.0),
                });
            }

            if mem_accel.abs() > INFLECTION_SENSITIVITY {
                let id = fnv1a_hash(format!("mem-inflect-{}", i).as_bytes());
                events.push(InflectionEvent {
                    id,
                    estimated_tick: curr.tick_offset,
                    dimension: String::from("memory"),
                    magnitude: mem_accel.abs(),
                    direction_change: mem_accel,
                    confidence: (1.0 - (1.0 / (mem_accel.abs() + 1.0))).clamp(0.0, 1.0),
                });
            }
        }

        for ev in &events {
            self.inflections.insert(ev.id, ev.clone());
        }

        while self.inflections.len() > MAX_INFLECTION_EVENTS {
            if let Some((&oldest, _)) = self.inflections.iter().next() {
                self.inflections.remove(&oldest);
            }
        }

        events
    }

    /// Record actual system state for accuracy tracking
    pub fn record_actual(&mut self, horizon: HorizonScale, actual_cpu: f32, actual_mem: f32) {
        let preds: Vec<&SystemStatePrediction> = self
            .predictions
            .values()
            .filter(|p| p.horizon == horizon)
            .collect();

        if let Some(last) = preds.last() {
            let cpu_err = (last.fused_cpu_util - actual_cpu).abs() / 100.0;
            let mem_err = (last.fused_memory_pressure - actual_mem).abs();
            let accuracy = 1.0 - (cpu_err + mem_err) / 2.0;
            self.accuracy_ema =
                EMA_ALPHA * accuracy.clamp(0.0, 1.0) + (1.0 - EMA_ALPHA) * self.accuracy_ema;

            self.accuracy_by_horizon
                .insert(horizon as u8, self.accuracy_ema);
        }
    }

    /// Gather aggregate statistics
    pub fn stats(&self) -> HorizonStats {
        let best = self
            .accuracy_by_horizon
            .values()
            .cloned()
            .fold(0.0_f32, f32::max);
        let worst = self
            .accuracy_by_horizon
            .values()
            .cloned()
            .fold(1.0_f32, f32::min);

        HorizonStats {
            total_predictions: self.total_predictions,
            total_fusions: self.total_fusions,
            avg_confidence: self.confidence_ema,
            avg_accuracy: self.accuracy_ema,
            inflection_count: self.inflections.len() as u64,
            trajectory_length: self.trajectory.len(),
            best_horizon_accuracy: best,
            worst_horizon_accuracy: worst,
        }
    }
}
