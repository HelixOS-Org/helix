// SPDX-License-Identifier: GPL-2.0
//! # Bridge Temporal Fusion
//!
//! Multi-horizon temporal fusion. Fuses predictions at different time horizons
//! (1ms, 10ms, 100ms, 1s, 10s) into a single coherent future view. Short-term
//! predictions are sharp but shallow; long-term predictions are blurry but deep.
//! This module reconciles them into a view that is both sharp and deep, weighting
//! each horizon by its empirical accuracy at that range.
//!
//! The future is not one thing — it's five things at five speeds.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const NUM_HORIZONS: usize = 5;
const HORIZON_MS: [u64; 5] = [1, 10, 100, 1000, 10000];
const MAX_HISTORY_PER_HORIZON: usize = 256;
const MAX_FUSION_SOURCES: usize = 32;
const EMA_ALPHA: f32 = 0.08;
const CONSISTENCY_WEIGHT: f32 = 0.15;
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
// TEMPORAL LAYER
// ============================================================================

/// A prediction at a specific time horizon.
#[derive(Debug, Clone)]
pub struct TemporalLayer {
    /// Time horizon in milliseconds
    pub horizon_ms: u64,
    /// Predicted value at this horizon
    pub prediction: f32,
    /// Confidence in this prediction (0.0 to 1.0)
    pub confidence: f32,
    /// Weight in the fusion (dynamically learned)
    pub weight: f32,
    /// Horizon index (0 = shortest, 4 = longest)
    pub index: usize,
}

// ============================================================================
// FUSED VIEW
// ============================================================================

/// The fused multi-horizon view of the future.
#[derive(Debug, Clone)]
pub struct FusedView {
    /// Fused prediction value
    pub value: f32,
    /// Overall confidence
    pub confidence: f32,
    /// Individual layer contributions
    pub layers: Vec<TemporalLayer>,
    /// Consistency score across horizons (0 = contradictory, 1 = aligned)
    pub consistency: f32,
    /// Quality score of the fusion
    pub quality: f32,
}

// ============================================================================
// HORIZON TRACKER
// ============================================================================

/// Tracks accuracy and calibration for a single time horizon.
#[derive(Debug, Clone)]
struct HorizonTracker {
    horizon_ms: u64,
    index: usize,
    /// Recent predictions and actuals
    predictions: VecDeque<f32>,
    actuals: VecDeque<f32>,
    /// Mean absolute error (EMA)
    mae_ema: f32,
    /// Accuracy as 1 - MAE (EMA)
    accuracy_ema: f32,
    /// Weight for fusion (learned)
    weight: f32,
    /// Bias detection: mean signed error
    bias_ema: f32,
    /// Total predictions at this horizon
    total: u64,
}

impl HorizonTracker {
    fn new(horizon_ms: u64, index: usize) -> Self {
        Self {
            horizon_ms,
            index,
            predictions: VecDeque::new(),
            actuals: VecDeque::new(),
            mae_ema: 0.1,
            accuracy_ema: 0.5,
            weight: 1.0 / NUM_HORIZONS as f32,
            bias_ema: 0.0,
            total: 0,
        }
    }

    #[inline]
    fn record(&mut self, predicted: f32, actual: f32) {
        self.predictions.push_back(predicted);
        self.actuals.push_back(actual);
        self.total += 1;

        if self.predictions.len() > MAX_HISTORY_PER_HORIZON {
            self.predictions.pop_front();
            self.actuals.pop_front();
        }

        let error = (actual - predicted).abs();
        self.mae_ema = self.mae_ema * (1.0 - EMA_ALPHA) + error * EMA_ALPHA;
        self.accuracy_ema = (1.0 - self.mae_ema).max(0.0);

        let signed_error = actual - predicted;
        self.bias_ema = self.bias_ema * (1.0 - EMA_ALPHA) + signed_error * EMA_ALPHA;
    }
}

// ============================================================================
// CONSISTENCY RECORD
// ============================================================================

/// Tracks pairwise consistency between horizon pairs.
#[derive(Debug, Clone)]
struct ConsistencyRecord {
    horizon_i: usize,
    horizon_j: usize,
    /// EMA of absolute difference between horizon predictions
    diff_ema: f32,
    /// EMA of signed difference (detects systematic divergence)
    signed_diff_ema: f32,
    /// Number of consistency checks
    checks: u64,
}

impl ConsistencyRecord {
    fn new(i: usize, j: usize) -> Self {
        Self {
            horizon_i: i,
            horizon_j: j,
            diff_ema: 0.0,
            signed_diff_ema: 0.0,
            checks: 0,
        }
    }

    #[inline]
    fn update(&mut self, pred_i: f32, pred_j: f32) {
        let diff = (pred_i - pred_j).abs();
        let signed_diff = pred_i - pred_j;
        self.diff_ema = self.diff_ema * (1.0 - EMA_ALPHA) + diff * EMA_ALPHA;
        self.signed_diff_ema =
            self.signed_diff_ema * (1.0 - EMA_ALPHA) + signed_diff * EMA_ALPHA;
        self.checks += 1;
    }
}

// ============================================================================
// TEMPORAL FUSION STATS
// ============================================================================

/// Statistics for the temporal fusion engine.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TemporalFusionStats {
    pub total_fusions: u64,
    pub avg_consistency: f32,
    pub avg_fusion_quality: f32,
    pub horizon_accuracies: [f32; NUM_HORIZONS],
    pub horizon_weights: [f32; NUM_HORIZONS],
    pub short_term_bias: f32,
    pub long_term_bias: f32,
}

impl TemporalFusionStats {
    fn new() -> Self {
        Self {
            total_fusions: 0,
            avg_consistency: 0.5,
            avg_fusion_quality: 0.5,
            horizon_accuracies: [0.5; NUM_HORIZONS],
            horizon_weights: [0.2; NUM_HORIZONS],
            short_term_bias: 0.0,
            long_term_bias: 0.0,
        }
    }
}

// ============================================================================
// BRIDGE TEMPORAL FUSION
// ============================================================================

/// Multi-horizon temporal fusion engine.
///
/// Fuses predictions at 5 time horizons (1ms, 10ms, 100ms, 1s, 10s) into
/// a coherent future view. Learns per-horizon weights from empirical accuracy
/// and enforces cross-horizon consistency.
#[repr(align(64))]
pub struct BridgeTemporalFusion {
    /// Per-horizon trackers
    horizons: [HorizonTracker; NUM_HORIZONS],
    /// Pairwise consistency records
    consistency: BTreeMap<(usize, usize), ConsistencyRecord>,
    /// Per-source tracking: source_id -> last predictions per horizon
    sources: BTreeMap<u64, [f32; NUM_HORIZONS]>,
    /// Running statistics
    stats: TemporalFusionStats,
    /// PRNG state
    rng: u64,
    /// Tick counter
    tick: u64,
}

impl BridgeTemporalFusion {
    /// Create a new temporal fusion engine.
    pub fn new() -> Self {
        let horizons = [
            HorizonTracker::new(HORIZON_MS[0], 0),
            HorizonTracker::new(HORIZON_MS[1], 1),
            HorizonTracker::new(HORIZON_MS[2], 2),
            HorizonTracker::new(HORIZON_MS[3], 3),
            HorizonTracker::new(HORIZON_MS[4], 4),
        ];

        let mut consistency = BTreeMap::new();
        for i in 0..NUM_HORIZONS {
            for j in (i + 1)..NUM_HORIZONS {
                consistency.insert((i, j), ConsistencyRecord::new(i, j));
            }
        }

        Self {
            horizons,
            consistency,
            sources: BTreeMap::new(),
            stats: TemporalFusionStats::new(),
            rng: 0xF05E_7E40_1234_5678,
            tick: 0,
        }
    }

    /// Submit predictions at all 5 horizons from a single source.
    #[inline]
    pub fn submit_predictions(&mut self, source_id: u64, predictions: &[f32; NUM_HORIZONS]) {
        self.sources.insert(source_id, *predictions);
        if self.sources.len() > MAX_FUSION_SOURCES {
            if let Some(&oldest) = self.sources.keys().next() {
                self.sources.remove(&oldest);
            }
        }
    }

    /// Record the actual outcome for a specific horizon.
    pub fn record_outcome(&mut self, horizon_idx: usize, actual: f32) {
        if horizon_idx >= NUM_HORIZONS {
            return;
        }

        // Update per-source predictions for this horizon
        for (_src_id, preds) in &self.sources {
            let predicted = preds[horizon_idx];
            self.horizons[horizon_idx].record(predicted, actual);
        }

        // Update weights based on accuracy
        self.recompute_weights();
    }

    fn recompute_weights(&mut self) {
        let mut inv_maes = [0.0f32; NUM_HORIZONS];
        let mut total = 0.0f32;
        for i in 0..NUM_HORIZONS {
            let inv = 1.0 / (self.horizons[i].mae_ema + 0.001);
            inv_maes[i] = inv;
            total += inv;
        }
        if total > 0.0 {
            for i in 0..NUM_HORIZONS {
                let new_w = inv_maes[i] / total;
                self.horizons[i].weight =
                    self.horizons[i].weight * (1.0 - EMA_ALPHA) + new_w * EMA_ALPHA;
            }
        }
    }

    /// Fuse predictions across all horizons into a single coherent view.
    pub fn fuse_horizons(&mut self, predictions: &[f32; NUM_HORIZONS]) -> FusedView {
        self.stats.total_fusions += 1;

        // Update pairwise consistency
        for i in 0..NUM_HORIZONS {
            for j in (i + 1)..NUM_HORIZONS {
                if let Some(cr) = self.consistency.get_mut(&(i, j)) {
                    cr.update(predictions[i], predictions[j]);
                }
            }
        }

        // Build temporal layers
        let mut layers = Vec::with_capacity(NUM_HORIZONS);
        let mut weighted_sum = 0.0f32;
        let mut total_weight = 0.0f32;

        for i in 0..NUM_HORIZONS {
            let layer = TemporalLayer {
                horizon_ms: HORIZON_MS[i],
                prediction: predictions[i],
                confidence: self.horizons[i].accuracy_ema,
                weight: self.horizons[i].weight,
                index: i,
            };
            weighted_sum += predictions[i] * self.horizons[i].weight;
            total_weight += self.horizons[i].weight;
            layers.push(layer);
        }

        let fused_value = if total_weight > 0.0 {
            weighted_sum / total_weight
        } else {
            predictions.iter().sum::<f32>() / NUM_HORIZONS as f32
        };

        // Compute consistency: 1 - normalized variance of predictions
        let mean = fused_value;
        let variance: f32 = predictions.iter().map(|p| (p - mean) * (p - mean)).sum::<f32>()
            / NUM_HORIZONS as f32;
        let consistency = 1.0 / (1.0 + variance * 10.0);

        // Incorporate consistency into confidence
        let base_confidence: f32 = layers.iter().map(|l| l.confidence * l.weight).sum::<f32>()
            / total_weight.max(0.001);
        let confidence = base_confidence * (1.0 - CONSISTENCY_WEIGHT)
            + consistency * CONSISTENCY_WEIGHT;

        let quality = confidence * consistency;

        // Update stats
        self.stats.avg_consistency = self.stats.avg_consistency * (1.0 - EMA_ALPHA)
            + consistency * EMA_ALPHA;
        self.stats.avg_fusion_quality = self.stats.avg_fusion_quality * (1.0 - EMA_ALPHA)
            + quality * EMA_ALPHA;

        for i in 0..NUM_HORIZONS {
            self.stats.horizon_accuracies[i] = self.horizons[i].accuracy_ema;
            self.stats.horizon_weights[i] = self.horizons[i].weight;
        }

        self.stats.short_term_bias = self.horizons[0].bias_ema;
        self.stats.long_term_bias = self.horizons[NUM_HORIZONS - 1].bias_ema;

        FusedView { value: fused_value, confidence, layers, consistency, quality }
    }

    /// Get the short-term view (1ms and 10ms horizons only).
    pub fn short_term_view(&self, predictions: &[f32; NUM_HORIZONS]) -> (f32, f32) {
        let w0 = self.horizons[0].weight;
        let w1 = self.horizons[1].weight;
        let total = w0 + w1;
        if total > 0.0 {
            let value = (predictions[0] * w0 + predictions[1] * w1) / total;
            let confidence = (self.horizons[0].accuracy_ema * w0
                + self.horizons[1].accuracy_ema * w1)
                / total;
            (value, confidence)
        } else {
            ((predictions[0] + predictions[1]) / 2.0, 0.5)
        }
    }

    /// Get the long-term view (1s and 10s horizons only).
    pub fn long_term_view(&self, predictions: &[f32; NUM_HORIZONS]) -> (f32, f32) {
        let w3 = self.horizons[3].weight;
        let w4 = self.horizons[4].weight;
        let total = w3 + w4;
        if total > 0.0 {
            let value = (predictions[3] * w3 + predictions[4] * w4) / total;
            let confidence = (self.horizons[3].accuracy_ema * w3
                + self.horizons[4].accuracy_ema * w4)
                / total;
            (value, confidence)
        } else {
            ((predictions[3] + predictions[4]) / 2.0, 0.3)
        }
    }

    /// Check cross-horizon consistency: do short and long predictions agree?
    pub fn horizon_consistency(&self) -> f32 {
        let mut total_diff = 0.0f32;
        let mut count = 0u32;
        for cr in self.consistency.values() {
            total_diff += cr.diff_ema;
            count += 1;
        }
        if count > 0 {
            1.0 / (1.0 + total_diff / count as f32 * 5.0)
        } else {
            0.5
        }
    }

    /// Get the temporal resolution: how many horizons have enough data to be useful.
    #[inline(always)]
    pub fn temporal_resolution(&self) -> usize {
        self.horizons.iter().filter(|h| h.total >= 10).count()
    }

    /// Get the fusion quality: how good is the fused prediction vs individual horizons.
    #[inline(always)]
    pub fn fusion_quality(&self) -> f32 {
        self.stats.avg_fusion_quality
    }

    /// Get statistics.
    #[inline(always)]
    pub fn stats(&self) -> &TemporalFusionStats {
        &self.stats
    }

    /// Get the current tick.
    #[inline(always)]
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Advance the tick counter.
    #[inline(always)]
    pub fn advance_tick(&mut self, tick: u64) {
        self.tick = tick;
    }
}
