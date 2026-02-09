// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Self-Model
//!
//! The cooperation protocol's complete model of itself. Tracks negotiation
//! success rate, contract fulfillment percentage, hint accuracy, and fairness
//! score. All metrics are smoothed with exponential moving averages and
//! bounded by confidence intervals derived from observed variance.
//!
//! A cooperation engine that knows its own strengths and weaknesses can
//! negotiate more honestly and allocate resources more fairly.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.15;
const CONFIDENCE_Z: f32 = 1.96;
const MAX_METRIC_HISTORY: usize = 256;
const MAX_METRICS: usize = 64;
const FAIRNESS_IDEAL: f32 = 1.0;
const TRUST_BASELINE: f32 = 0.5;
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
// COOPERATION METRIC
// ============================================================================

/// A single tracked cooperation metric with EMA smoothing
#[derive(Debug, Clone)]
pub struct CoopMetric {
    pub name: String,
    pub id: u64,
    /// Current EMA-smoothed score (0.0 – 1.0)
    pub score: f32,
    /// Variance accumulator for confidence intervals
    pub variance_accum: f32,
    /// Number of observations
    pub observations: u64,
    /// Ring buffer of raw samples
    history: Vec<f32>,
    write_idx: usize,
    /// Last raw sample
    pub last_raw: f32,
    /// Peak score ever observed
    pub peak_score: f32,
    /// Tick of last update
    pub last_update_tick: u64,
}

impl CoopMetric {
    pub fn new(name: String) -> Self {
        let id = fnv1a_hash(name.as_bytes());
        Self {
            name,
            id,
            score: 0.5,
            variance_accum: 0.0,
            observations: 0,
            history: Vec::new(),
            write_idx: 0,
            last_raw: 0.5,
            peak_score: 0.5,
            last_update_tick: 0,
        }
    }

    /// Push a new observation, update EMA and variance
    pub fn observe(&mut self, raw: f32, tick: u64) {
        let clamped = raw.max(0.0).min(1.0);
        self.last_raw = clamped;
        self.observations += 1;
        self.last_update_tick = tick;

        self.score = EMA_ALPHA * clamped + (1.0 - EMA_ALPHA) * self.score;
        if self.score > self.peak_score {
            self.peak_score = self.score;
        }

        let diff = clamped - self.score;
        self.variance_accum = EMA_ALPHA * diff * diff + (1.0 - EMA_ALPHA) * self.variance_accum;

        if self.history.len() < MAX_METRIC_HISTORY {
            self.history.push(clamped);
        } else {
            self.history[self.write_idx] = clamped;
        }
        self.write_idx = (self.write_idx + 1) % MAX_METRIC_HISTORY;
    }

    /// 95% confidence interval half-width
    pub fn confidence_half_width(&self) -> f32 {
        if self.observations < 2 {
            return 0.5;
        }
        let std_dev = libm::sqrtf(self.variance_accum);
        let n_sqrt = libm::sqrtf(self.observations.min(MAX_METRIC_HISTORY as u64) as f32);
        CONFIDENCE_Z * std_dev / n_sqrt
    }

    /// Improvement rate: slope of recent trend
    pub fn improvement_rate(&self) -> f32 {
        let len = self.history.len();
        if len < 4 {
            return 0.0;
        }
        let mid = len / 2;
        let first_avg: f32 = self.history[..mid].iter().sum::<f32>() / mid as f32;
        let second_avg: f32 = self.history[mid..].iter().sum::<f32>() / (len - mid) as f32;
        second_avg - first_avg
    }
}

// ============================================================================
// SELF-MODEL STATS
// ============================================================================

/// Aggregate cooperation self-model statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct SelfModelStats {
    pub total_metrics: usize,
    pub negotiation_success_rate: f32,
    pub contract_fulfillment: f32,
    pub hint_accuracy: f32,
    pub fairness_score: f32,
    pub overall_cooperation_score: f32,
    pub trust_self_score: f32,
    pub avg_confidence_width: f32,
    pub total_observations: u64,
}

// ============================================================================
// COOPERATION SELF-MODEL
// ============================================================================

/// The cooperation protocol's complete model of itself — negotiation success,
/// contract fulfillment, hint accuracy, fairness with EMA smoothing.
#[derive(Debug)]
pub struct CoopSelfModel {
    /// All tracked metrics keyed by FNV-1a hash of name
    metrics: BTreeMap<u64, CoopMetric>,
    /// Fairness calibration offset applied per-metric
    fairness_offsets: BTreeMap<u64, f32>,
    /// Monotonic tick counter
    tick: u64,
    /// Cached aggregate stats
    cached_stats: SelfModelStats,
    /// Tick at which stats were last computed
    stats_tick: u64,
    /// PRNG state for stochastic probes
    rng_state: u64,
}

impl CoopSelfModel {
    pub fn new() -> Self {
        Self {
            metrics: BTreeMap::new(),
            fairness_offsets: BTreeMap::new(),
            tick: 0,
            cached_stats: SelfModelStats::default(),
            stats_tick: 0,
            rng_state: 0xC00B_5E1F_A0DE_1A1A,
        }
    }

    /// Register or update a cooperation metric with a new observation
    pub fn update_metric(&mut self, name: &str, raw_score: f32) {
        self.tick += 1;
        let id = fnv1a_hash(name.as_bytes());
        let tick = self.tick;
        let metric = self
            .metrics
            .entry(id)
            .or_insert_with(|| CoopMetric::new(String::from(name)));
        metric.observe(raw_score, tick);
    }

    /// Comprehensive self-assessment across all tracked metrics
    pub fn self_assess(&self) -> SelfModelStats {
        if self.metrics.is_empty() {
            return SelfModelStats::default();
        }

        let mut total_score = 0.0_f32;
        let mut total_confidence_width = 0.0_f32;
        let mut total_obs = 0_u64;
        let mut neg_success = 0.5_f32;
        let mut fulfillment = 0.5_f32;
        let mut accuracy = 0.5_f32;
        let mut fairness = 0.5_f32;

        let neg_id = fnv1a_hash(b"negotiation_success");
        let ful_id = fnv1a_hash(b"contract_fulfillment");
        let acc_id = fnv1a_hash(b"hint_accuracy");
        let fair_id = fnv1a_hash(b"fairness");

        for (_, metric) in self.metrics.iter() {
            total_score += metric.score;
            total_confidence_width += metric.confidence_half_width();
            total_obs += metric.observations;
            if metric.id == neg_id {
                neg_success = metric.score;
            } else if metric.id == ful_id {
                fulfillment = metric.score;
            } else if metric.id == acc_id {
                accuracy = metric.score;
            } else if metric.id == fair_id {
                fairness = metric.score;
            }
        }

        let count = self.metrics.len() as f32;
        SelfModelStats {
            total_metrics: self.metrics.len(),
            negotiation_success_rate: neg_success,
            contract_fulfillment: fulfillment,
            hint_accuracy: accuracy,
            fairness_score: fairness,
            overall_cooperation_score: total_score / count,
            trust_self_score: self.trust_self_evaluation(),
            avg_confidence_width: total_confidence_width / count,
            total_observations: total_obs,
        }
    }

    /// Composite cooperation score: weighted blend of core metrics
    pub fn cooperation_score(&self) -> f32 {
        let neg_id = fnv1a_hash(b"negotiation_success");
        let ful_id = fnv1a_hash(b"contract_fulfillment");
        let acc_id = fnv1a_hash(b"hint_accuracy");
        let fair_id = fnv1a_hash(b"fairness");

        let neg = self.metrics.get(&neg_id).map(|m| m.score).unwrap_or(0.5);
        let ful = self.metrics.get(&ful_id).map(|m| m.score).unwrap_or(0.5);
        let acc = self.metrics.get(&acc_id).map(|m| m.score).unwrap_or(0.5);
        let fair = self.metrics.get(&fair_id).map(|m| m.score).unwrap_or(0.5);

        neg * 0.25 + ful * 0.30 + acc * 0.20 + fair * 0.25
    }

    /// Calibrate fairness by computing offset between observed and ideal
    pub fn fairness_calibration(&mut self) -> f32 {
        self.tick += 1;
        let mut total_offset = 0.0_f32;
        let mut count = 0_usize;

        for (&id, metric) in self.metrics.iter() {
            let deviation = FAIRNESS_IDEAL - metric.score;
            let offset = deviation * 0.1;
            self.fairness_offsets.insert(id, offset);
            total_offset += deviation.abs();
            count += 1;
        }

        if count == 0 {
            return 0.0;
        }
        let avg_deviation = total_offset / count as f32;
        1.0 - avg_deviation.min(1.0)
    }

    /// Self-trust evaluation: how reliable are our own metrics?
    pub fn trust_self_evaluation(&self) -> f32 {
        if self.metrics.is_empty() {
            return TRUST_BASELINE;
        }

        let mut consistency_score = 0.0_f32;
        let mut maturity_score = 0.0_f32;
        let count = self.metrics.len() as f32;

        for metric in self.metrics.values() {
            // Consistency: narrow confidence = more trustworthy
            let hw = metric.confidence_half_width();
            consistency_score += 1.0 - hw.min(1.0);

            // Maturity: more observations = more trustworthy
            let maturity = (metric.observations as f32 / 100.0).min(1.0);
            maturity_score += maturity;
        }

        let consistency = consistency_score / count;
        let maturity = maturity_score / count;

        // Improvement trend contributes positively
        let mut trend_score = 0.0_f32;
        for metric in self.metrics.values() {
            trend_score += metric.improvement_rate().max(-0.5).min(0.5) + 0.5;
        }
        let trend = trend_score / count;

        consistency * 0.40 + maturity * 0.35 + trend * 0.25
    }

    /// Get a specific metric by name
    pub fn get_metric(&self, name: &str) -> Option<&CoopMetric> {
        let id = fnv1a_hash(name.as_bytes());
        self.metrics.get(&id)
    }

    /// Number of tracked metrics
    pub fn metric_count(&self) -> usize {
        self.metrics.len()
    }

    /// Stochastic self-probe: inject a random check to verify internal consistency
    pub fn stochastic_probe(&mut self) -> f32 {
        let nonce = xorshift64(&mut self.rng_state);
        let probe_idx = (nonce % self.metrics.len().max(1) as u64) as usize;

        if let Some((_, metric)) = self.metrics.iter().nth(probe_idx) {
            let expected = metric.score;
            let hw = metric.confidence_half_width();
            if metric.last_raw >= expected - hw && metric.last_raw <= expected + hw {
                1.0
            } else {
                0.5
            }
        } else {
            0.0
        }
    }
}
