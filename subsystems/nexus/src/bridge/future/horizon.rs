// SPDX-License-Identifier: GPL-2.0
//! # Bridge Horizon Predictor
//!
//! Long-horizon syscall prediction engine operating at multiple time scales.
//! Maintains four temporal buckets — 1 second, 1 minute, 10 minutes, 1 hour —
//! each tracking independent pattern statistics. Predictions at longer horizons
//! naturally carry wider confidence intervals, and the engine tracks its own
//! error rate by horizon to learn where it can and cannot see clearly.
//!
//! This is the bridge looking an hour into the future and placing bets.

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_PATTERNS_PER_SCALE: usize = 256;
const MAX_OBSERVATIONS: usize = 1024;
const EMA_ALPHA: f32 = 0.10;
const CONFIDENCE_DECAY_PER_SCALE: f32 = 0.20;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const NUM_SCALES: usize = 4;

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

/// Time scale for predictions
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TimeScale {
    /// 1-second buckets — immediate future
    OneSecond = 0,
    /// 1-minute buckets — short-term
    OneMinute = 1,
    /// 10-minute buckets — medium-term
    TenMinutes = 2,
    /// 1-hour buckets — long horizon
    OneHour = 3,
}

impl TimeScale {
    fn bucket_width_ticks(&self) -> u64 {
        match self {
            TimeScale::OneSecond => 100,
            TimeScale::OneMinute => 6_000,
            TimeScale::TenMinutes => 60_000,
            TimeScale::OneHour => 360_000,
        }
    }

    fn from_index(i: usize) -> Self {
        match i {
            0 => TimeScale::OneSecond,
            1 => TimeScale::OneMinute,
            2 => TimeScale::TenMinutes,
            _ => TimeScale::OneHour,
        }
    }
}

// ============================================================================
// PATTERN TYPES
// ============================================================================

/// A temporal pattern observed at a specific time scale
#[derive(Debug, Clone)]
pub struct TemporalPattern {
    pub pattern_id: u64,
    pub syscall_class: u32,
    pub scale: TimeScale,
    pub frequency: f32,
    pub strength: f32,
    pub last_seen_tick: u64,
    pub occurrence_count: u64,
}

/// An observation of a syscall event
#[derive(Debug, Clone, Copy)]
pub struct Observation {
    pub syscall_nr: u32,
    pub process_id: u64,
    pub tick: u64,
}

/// A prediction for a specific future time
#[derive(Debug, Clone)]
pub struct HorizonPrediction {
    pub target_tick: u64,
    pub scale: TimeScale,
    pub predicted_syscall: u32,
    pub confidence: f32,
    pub supporting_patterns: u32,
}

/// Prediction error record for calibration
#[derive(Debug, Clone, Copy)]
struct ErrorRecord {
    scale: TimeScale,
    predicted: u32,
    actual: u32,
    confidence: f32,
    correct: bool,
}

// ============================================================================
// HORIZON STATS
// ============================================================================

/// Aggregate prediction statistics
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct HorizonStats {
    pub total_predictions: u64,
    pub total_observations: u64,
    pub accuracy_1s: f32,
    pub accuracy_1m: f32,
    pub accuracy_10m: f32,
    pub accuracy_1h: f32,
    pub avg_confidence: f32,
    pub calibration_error: f32,
}

// ============================================================================
// SCALE TRACKER
// ============================================================================

/// Per-scale pattern tracker
#[derive(Debug, Clone)]
struct ScaleTracker {
    patterns: BTreeMap<u64, TemporalPattern>,
    bucket_counts: LinearMap<u32, 64>,
    accuracy_ema: f32,
    total_predictions: u64,
    correct_predictions: u64,
    confidence_ema: f32,
}

impl ScaleTracker {
    fn new() -> Self {
        Self {
            patterns: BTreeMap::new(),
            bucket_counts: LinearMap::new(),
            accuracy_ema: 0.5,
            total_predictions: 0,
            correct_predictions: 0,
            confidence_ema: 0.5,
        }
    }

    #[inline]
    fn record_observation(&mut self, syscall_nr: u32, tick: u64, bucket_width: u64) {
        let bucket = tick / bucket_width.max(1);
        let count = self.bucket_counts.entry(bucket).or_insert(0);
        *count += 1;

        let key = fnv1a_hash(&syscall_nr.to_le_bytes()) ^ bucket;
        let pattern = self.patterns.entry(key).or_insert_with(|| TemporalPattern {
            pattern_id: key,
            syscall_class: syscall_nr,
            scale: TimeScale::OneSecond,
            frequency: 0.0,
            strength: 0.0,
            last_seen_tick: tick,
            occurrence_count: 0,
        });
        pattern.occurrence_count += 1;
        pattern.last_seen_tick = tick;
        pattern.frequency = EMA_ALPHA * 1.0 + (1.0 - EMA_ALPHA) * pattern.frequency;
        pattern.strength = pattern.occurrence_count as f32
            / self.bucket_counts.len().max(1) as f32;

        // Evict weakest if over capacity
        if self.patterns.len() > MAX_PATTERNS_PER_SCALE {
            let weakest = self.patterns.iter()
                .min_by(|a, b| a.1.strength.partial_cmp(&b.1.strength).unwrap_or(core::cmp::Ordering::Equal))
                .map(|(&k, _)| k);
            if let Some(k) = weakest {
                self.patterns.remove(&k);
            }
        }
    }

    fn best_prediction(&self, current_tick: u64, bucket_width: u64) -> Option<(u32, f32, u32)> {
        let target_bucket = current_tick / bucket_width.max(1) + 1;
        let mut best_syscall = 0u32;
        let mut best_score = 0.0f32;
        let mut supporting = 0u32;

        for pattern in self.patterns.values() {
            let recency = if current_tick > pattern.last_seen_tick {
                1.0 / (1.0 + (current_tick - pattern.last_seen_tick) as f32 / bucket_width as f32)
            } else {
                1.0
            };
            let score = pattern.frequency * pattern.strength * recency;
            if score > best_score {
                best_score = score;
                best_syscall = pattern.syscall_class;
                supporting = 0;
            }
            if pattern.syscall_class == best_syscall {
                supporting += 1;
            }
            let _ = target_bucket; // used for bucket alignment context
        }

        if best_score > 0.01 {
            Some((best_syscall, best_score.min(1.0), supporting))
        } else {
            None
        }
    }

    #[inline]
    fn record_accuracy(&mut self, correct: bool, confidence: f32) {
        self.total_predictions += 1;
        if correct {
            self.correct_predictions += 1;
        }
        let val = if correct { 1.0 } else { 0.0 };
        self.accuracy_ema = EMA_ALPHA * val + (1.0 - EMA_ALPHA) * self.accuracy_ema;
        self.confidence_ema = EMA_ALPHA * confidence + (1.0 - EMA_ALPHA) * self.confidence_ema;
    }
}

// ============================================================================
// BRIDGE HORIZON PREDICTOR
// ============================================================================

/// Long-horizon syscall prediction engine with multi-scale temporal patterns.
/// Maintains independent pattern trackers at 1s, 1min, 10min, and 1h scales.
#[derive(Debug)]
pub struct BridgeHorizonPredictor {
    scale_trackers: [ScaleTracker; NUM_SCALES],
    observations: Vec<Observation>,
    write_idx: usize,
    tick: u64,
    total_observations: u64,
    total_predictions: u64,
    error_log: Vec<ErrorRecord>,
    error_write_idx: usize,
    rng_state: u64,
}

impl BridgeHorizonPredictor {
    pub fn new() -> Self {
        Self {
            scale_trackers: [
                ScaleTracker::new(),
                ScaleTracker::new(),
                ScaleTracker::new(),
                ScaleTracker::new(),
            ],
            observations: Vec::new(),
            write_idx: 0,
            tick: 0,
            total_observations: 0,
            total_predictions: 0,
            error_log: Vec::new(),
            error_write_idx: 0,
            rng_state: 0xDEAD_BEEF_CAFE_1234,
        }
    }

    /// Record a new syscall observation, updating all time-scale trackers
    pub fn update_observation(&mut self, syscall_nr: u32, process_id: u64) {
        self.tick += 1;
        self.total_observations += 1;

        let obs = Observation { syscall_nr, process_id, tick: self.tick };
        if self.observations.len() < MAX_OBSERVATIONS {
            self.observations.push(obs);
        } else {
            self.observations[self.write_idx] = obs;
        }
        self.write_idx = (self.write_idx + 1) % MAX_OBSERVATIONS;

        for i in 0..NUM_SCALES {
            let scale = TimeScale::from_index(i);
            self.scale_trackers[i].record_observation(
                syscall_nr, self.tick, scale.bucket_width_ticks(),
            );
        }
    }

    /// Predict the most likely syscall at a future horizon
    pub fn predict_at_horizon(&mut self, scale: TimeScale) -> Option<HorizonPrediction> {
        self.total_predictions += 1;
        let idx = scale as usize;
        let bucket_width = scale.bucket_width_ticks();

        self.scale_trackers[idx].best_prediction(self.tick, bucket_width)
            .map(|(syscall, raw_conf, supporting)| {
                let scale_penalty = 1.0 - (idx as f32 * CONFIDENCE_DECAY_PER_SCALE);
                let accuracy_mod = self.scale_trackers[idx].accuracy_ema;
                let confidence = (raw_conf * scale_penalty * accuracy_mod).max(0.01).min(0.99);

                HorizonPrediction {
                    target_tick: self.tick + bucket_width,
                    scale,
                    predicted_syscall: syscall,
                    confidence,
                    supporting_patterns: supporting,
                }
            })
    }

    /// Get confidence for a prediction at a specific time scale
    #[inline]
    pub fn confidence_at_time(&self, scale: TimeScale) -> f32 {
        let idx = scale as usize;
        let base = self.scale_trackers[idx].accuracy_ema;
        let penalty = 1.0 - (idx as f32 * CONFIDENCE_DECAY_PER_SCALE);
        (base * penalty).max(0.0).min(1.0)
    }

    /// Extract multi-scale pattern for a syscall class across all time horizons
    pub fn multi_scale_pattern(&self, syscall_nr: u32) -> Vec<(TimeScale, f32, u64)> {
        let key_base = fnv1a_hash(&syscall_nr.to_le_bytes());
        let mut result = Vec::new();

        for i in 0..NUM_SCALES {
            let scale = TimeScale::from_index(i);
            let mut best_strength = 0.0f32;
            let mut total_occ = 0u64;

            for pattern in self.scale_trackers[i].patterns.values() {
                if pattern.syscall_class == syscall_nr {
                    if pattern.strength > best_strength {
                        best_strength = pattern.strength;
                    }
                    total_occ += pattern.occurrence_count;
                }
            }
            let _ = key_base;
            result.push((scale, best_strength, total_occ));
        }
        result
    }

    /// Compute horizon accuracy — how well predictions hold at each scale.
    /// Validates a previous prediction against actual observation.
    pub fn horizon_accuracy(&mut self, scale: TimeScale, predicted: u32, actual: u32) -> f32 {
        let correct = predicted == actual;
        let idx = scale as usize;
        let confidence = self.scale_trackers[idx].confidence_ema;

        self.scale_trackers[idx].record_accuracy(correct, confidence);

        let record = ErrorRecord {
            scale,
            predicted,
            actual,
            confidence,
            correct,
        };
        if self.error_log.len() < MAX_OBSERVATIONS {
            self.error_log.push(record);
        } else {
            self.error_log[self.error_write_idx] = record;
        }
        self.error_write_idx = (self.error_write_idx + 1) % MAX_OBSERVATIONS;

        self.scale_trackers[idx].accuracy_ema
    }

    /// Aggregate statistics across all horizons
    pub fn stats(&self) -> HorizonStats {
        let total_conf: f32 = self.scale_trackers.iter()
            .map(|t| t.confidence_ema)
            .sum::<f32>() / NUM_SCALES as f32;

        let cal_error: f32 = self.scale_trackers.iter()
            .map(|t| (t.confidence_ema - t.accuracy_ema).abs())
            .sum::<f32>() / NUM_SCALES as f32;

        HorizonStats {
            total_predictions: self.total_predictions,
            total_observations: self.total_observations,
            accuracy_1s: self.scale_trackers[0].accuracy_ema,
            accuracy_1m: self.scale_trackers[1].accuracy_ema,
            accuracy_10m: self.scale_trackers[2].accuracy_ema,
            accuracy_1h: self.scale_trackers[3].accuracy_ema,
            avg_confidence: total_conf,
            calibration_error: cal_error,
        }
    }
}
