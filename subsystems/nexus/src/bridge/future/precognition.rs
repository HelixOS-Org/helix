// SPDX-License-Identifier: GPL-2.0
//! # Bridge Precognition Engine
//!
//! Pre-cognitive pattern detection for bridge operations. The bridge "senses"
//! what's coming before there's statistical evidence — detecting subtle shifts
//! in syscall distributions that precede major changes. Works by tracking
//! higher-order statistics (kurtosis, skewness) and distribution fingerprints
//! that shift before the mean does. Like feeling the tremor before the quake.
//!
//! The signal is always there. You just have to listen softer.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_SIGNALS: usize = 128;
const MAX_HISTORY: usize = 512;
const DISTRIBUTION_BINS: usize = 32;
const DRIFT_WINDOW: usize = 64;
const FAST_EMA_ALPHA: f32 = 0.15;
const SLOW_EMA_ALPHA: f32 = 0.03;
const DRIFT_SENSITIVITY: f32 = 0.05;
const SHIFT_DETECTION_THRESHOLD: f32 = 0.10;
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
// EARLY SIGNAL
// ============================================================================

/// An early signal detected before statistical significance is reached.
#[derive(Debug, Clone)]
pub struct EarlySignal {
    /// Hash identifying the feature that shifted
    pub feature_hash: u64,
    /// Type of shift detected
    pub shift_type: ShiftType,
    /// Magnitude of the shift (0.0 to 1.0)
    pub magnitude: f32,
    /// Confidence in the signal (0.0 to 1.0)
    pub confidence: f32,
    /// Estimated ticks until the shift becomes statistically significant
    pub estimated_lead_ticks: u64,
    /// Tick when the signal was first detected
    pub detection_tick: u64,
}

/// Type of subtle shift detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShiftType {
    /// Distribution mean is drifting
    MeanDrift,
    /// Variance is changing (volatility shift)
    VarianceShift,
    /// Skewness is changing (asymmetry shift)
    SkewnessShift,
    /// Kurtosis is changing (tail behavior shift)
    KurtosisShift,
    /// Entire distribution fingerprint is changing
    DistributionDrift,
    /// New modes appearing in the distribution
    ModalShift,
}

// ============================================================================
// DISTRIBUTION FINGERPRINT
// ============================================================================

/// Lightweight fingerprint of a distribution using histogram bins.
#[derive(Debug, Clone)]
struct DistributionFingerprint {
    /// Histogram bin counts (normalized to fractions)
    bins: [f32; DISTRIBUTION_BINS],
    /// Range covered: [min_val, max_val)
    min_val: f32,
    max_val: f32,
    /// Total samples in this fingerprint
    total_samples: u64,
}

impl DistributionFingerprint {
    fn new(min_val: f32, max_val: f32) -> Self {
        Self {
            bins: [0.0; DISTRIBUTION_BINS],
            min_val,
            max_val,
            total_samples: 0,
        }
    }

    fn add_sample(&mut self, value: f32) {
        if self.max_val <= self.min_val {
            return;
        }
        let normalized = (value - self.min_val) / (self.max_val - self.min_val);
        let bin_idx = (normalized * DISTRIBUTION_BINS as f32) as usize;
        let bin_idx = bin_idx.min(DISTRIBUTION_BINS - 1);
        self.total_samples += 1;

        // Incremental update: add 1/total to this bin, decay all bins slightly
        let decay = 1.0 - 1.0 / (self.total_samples as f32 + 1.0);
        for b in self.bins.iter_mut() {
            *b *= decay;
        }
        self.bins[bin_idx] += 1.0 / (self.total_samples as f32 + 1.0);

        // Normalize
        let sum: f32 = self.bins.iter().sum();
        if sum > 0.0 {
            for b in self.bins.iter_mut() {
                *b /= sum;
            }
        }
    }

    /// Jensen-Shannon divergence between two fingerprints
    fn js_divergence(&self, other: &DistributionFingerprint) -> f32 {
        let mut divergence = 0.0f32;
        for i in 0..DISTRIBUTION_BINS {
            let p = self.bins[i].max(1e-10);
            let q = other.bins[i].max(1e-10);
            let m = (p + q) / 2.0;
            // KL(P||M) + KL(Q||M)
            if p > 1e-10 {
                divergence += p * (p / m).ln();
            }
            if q > 1e-10 {
                divergence += q * (q / m).ln();
            }
        }
        (divergence / 2.0).max(0.0)
    }
}

// ============================================================================
// FEATURE TRACKER
// ============================================================================

/// Tracks a single feature's distribution over time.
#[derive(Debug, Clone)]
struct FeatureTracker {
    feature_hash: u64,
    /// Fast EMA of the feature value
    fast_ema: f32,
    /// Slow EMA of the feature value
    slow_ema: f32,
    /// EMA of variance
    variance_fast: f32,
    variance_slow: f32,
    /// EMA of skewness (third central moment)
    skewness_ema: f32,
    /// EMA of kurtosis (fourth central moment)
    kurtosis_ema: f32,
    /// Recent values for windowed statistics
    recent_values: Vec<f32>,
    /// Reference distribution (the "normal" state)
    reference_fingerprint: DistributionFingerprint,
    /// Current distribution (sliding window)
    current_fingerprint: DistributionFingerprint,
    /// Total observations
    total: u64,
    /// Whether a shift has been flagged
    shift_detected: bool,
    /// Signal history for accuracy tracking
    signal_count: u64,
    confirmed_signals: u64,
}

impl FeatureTracker {
    fn new(feature_hash: u64) -> Self {
        Self {
            feature_hash,
            fast_ema: 0.0,
            slow_ema: 0.0,
            variance_fast: 0.0,
            variance_slow: 0.0,
            skewness_ema: 0.0,
            kurtosis_ema: 0.0,
            recent_values: Vec::new(),
            reference_fingerprint: DistributionFingerprint::new(0.0, 1.0),
            current_fingerprint: DistributionFingerprint::new(0.0, 1.0),
            total: 0,
            shift_detected: false,
            signal_count: 0,
            confirmed_signals: 0,
        }
    }

    fn observe(&mut self, value: f32) {
        self.total += 1;

        // Update EMAs
        if self.total == 1 {
            self.fast_ema = value;
            self.slow_ema = value;
        } else {
            self.fast_ema = self.fast_ema * (1.0 - FAST_EMA_ALPHA) + value * FAST_EMA_ALPHA;
            self.slow_ema = self.slow_ema * (1.0 - SLOW_EMA_ALPHA) + value * SLOW_EMA_ALPHA;
        }

        // Update variance
        let diff_fast = value - self.fast_ema;
        let diff_slow = value - self.slow_ema;
        self.variance_fast = self.variance_fast * (1.0 - FAST_EMA_ALPHA)
            + diff_fast * diff_fast * FAST_EMA_ALPHA;
        self.variance_slow = self.variance_slow * (1.0 - SLOW_EMA_ALPHA)
            + diff_slow * diff_slow * SLOW_EMA_ALPHA;

        // Update skewness: E[(X-μ)^3] / σ^3
        let std_fast = self.variance_fast.sqrt().max(0.001);
        let normalized = diff_fast / std_fast;
        let cubed = normalized * normalized * normalized;
        self.skewness_ema = self.skewness_ema * (1.0 - FAST_EMA_ALPHA) + cubed * FAST_EMA_ALPHA;

        // Update kurtosis: E[(X-μ)^4] / σ^4
        let quartic = normalized * normalized * normalized * normalized;
        self.kurtosis_ema = self.kurtosis_ema * (1.0 - FAST_EMA_ALPHA) + quartic * FAST_EMA_ALPHA;

        // Maintain recent values window
        self.recent_values.push(value);
        if self.recent_values.len() > MAX_HISTORY {
            self.recent_values.remove(0);
        }

        // Update fingerprints
        if self.total <= DRIFT_WINDOW as u64 {
            // Still building reference
            self.reference_fingerprint.add_sample(value);
        }
        self.current_fingerprint.add_sample(value);
    }

    fn detect_shifts(&mut self, tick: u64) -> Vec<EarlySignal> {
        let mut signals = Vec::new();

        if self.total < (DRIFT_WINDOW as u64) * 2 {
            return signals;
        }

        // 1. Mean drift: fast EMA diverging from slow EMA
        let mean_diff = (self.fast_ema - self.slow_ema).abs();
        let std_slow = self.variance_slow.sqrt().max(0.001);
        let mean_drift_z = mean_diff / std_slow;

        if mean_drift_z > 1.5 && !self.shift_detected {
            let magnitude = (mean_drift_z / 5.0).min(1.0);
            let confidence = 1.0 - 1.0 / (1.0 + self.total as f32 * 0.01);
            signals.push(EarlySignal {
                feature_hash: self.feature_hash,
                shift_type: ShiftType::MeanDrift,
                magnitude,
                confidence: confidence * magnitude,
                estimated_lead_ticks: (100.0 / mean_drift_z.max(0.1)) as u64,
                detection_tick: tick,
            });
        }

        // 2. Variance shift: fast variance diverging from slow variance
        let var_ratio = if self.variance_slow > 0.001 {
            self.variance_fast / self.variance_slow
        } else {
            1.0
        };
        if (var_ratio - 1.0).abs() > 0.3 {
            let magnitude = ((var_ratio - 1.0).abs() / 2.0).min(1.0);
            signals.push(EarlySignal {
                feature_hash: self.feature_hash,
                shift_type: ShiftType::VarianceShift,
                magnitude,
                confidence: magnitude * 0.7,
                estimated_lead_ticks: 200,
                detection_tick: tick,
            });
        }

        // 3. Skewness shift: distribution becoming asymmetric
        if self.skewness_ema.abs() > 0.5 {
            let magnitude = (self.skewness_ema.abs() / 3.0).min(1.0);
            signals.push(EarlySignal {
                feature_hash: self.feature_hash,
                shift_type: ShiftType::SkewnessShift,
                magnitude,
                confidence: magnitude * 0.6,
                estimated_lead_ticks: 300,
                detection_tick: tick,
            });
        }

        // 4. Kurtosis shift: tail behavior changing (excess kurtosis > 0 = heavy tails)
        let excess_kurtosis = self.kurtosis_ema - 3.0;
        if excess_kurtosis.abs() > 1.0 {
            let magnitude = (excess_kurtosis.abs() / 5.0).min(1.0);
            signals.push(EarlySignal {
                feature_hash: self.feature_hash,
                shift_type: ShiftType::KurtosisShift,
                magnitude,
                confidence: magnitude * 0.5,
                estimated_lead_ticks: 400,
                detection_tick: tick,
            });
        }

        // 5. Distribution drift: JS divergence between reference and current
        let js_div = self.reference_fingerprint.js_divergence(&self.current_fingerprint);
        if js_div > SHIFT_DETECTION_THRESHOLD {
            let magnitude = (js_div / 0.5).min(1.0);
            signals.push(EarlySignal {
                feature_hash: self.feature_hash,
                shift_type: ShiftType::DistributionDrift,
                magnitude,
                confidence: magnitude * 0.8,
                estimated_lead_ticks: 150,
                detection_tick: tick,
            });
        }

        if !signals.is_empty() {
            self.signal_count += signals.len() as u64;
            self.shift_detected = true;
        } else {
            self.shift_detected = false;
        }

        signals
    }
}

// ============================================================================
// PRECOGNITION STATS
// ============================================================================

/// Statistics for the precognition engine.
#[derive(Debug, Clone)]
pub struct PrecognitionStats {
    pub total_features_tracked: u32,
    pub total_signals_detected: u64,
    pub total_signals_confirmed: u64,
    pub precognition_accuracy: f32,
    pub false_positive_rate: f32,
    pub avg_lead_ticks: f32,
    pub avg_signal_magnitude: f32,
    pub active_drifts: u32,
}

impl PrecognitionStats {
    fn new() -> Self {
        Self {
            total_features_tracked: 0,
            total_signals_detected: 0,
            total_signals_confirmed: 0,
            precognition_accuracy: 0.0,
            false_positive_rate: 1.0,
            avg_lead_ticks: 0.0,
            avg_signal_magnitude: 0.0,
            active_drifts: 0,
        }
    }
}

// ============================================================================
// BRIDGE PRECOGNITION
// ============================================================================

/// Pre-cognitive pattern detection engine for bridge operations.
///
/// Detects subtle shifts in syscall distributions that precede major changes.
/// Tracks higher-order statistics (variance, skewness, kurtosis) and
/// distribution fingerprints to sense changes before they become significant.
pub struct BridgePrecognition {
    /// Per-feature trackers
    features: BTreeMap<u64, FeatureTracker>,
    /// Recent signals for deduplication
    recent_signals: Vec<EarlySignal>,
    /// Running statistics
    stats: PrecognitionStats,
    /// PRNG state
    rng: u64,
    /// Current tick
    tick: u64,
    /// Confirmed signal count for accuracy
    total_signals: u64,
    confirmed_signals: u64,
    false_positives: u64,
}

impl BridgePrecognition {
    /// Create a new precognition engine.
    pub fn new() -> Self {
        Self {
            features: BTreeMap::new(),
            recent_signals: Vec::new(),
            stats: PrecognitionStats::new(),
            rng: 0x09EC_06A1_CAFE_BABE,
            tick: 0,
            total_signals: 0,
            confirmed_signals: 0,
            false_positives: 0,
        }
    }

    /// Observe a feature value for a given feature hash.
    pub fn observe(&mut self, feature_hash: u64, value: f32, tick: u64) {
        self.tick = tick;

        let tracker = self.features.entry(feature_hash).or_insert_with(|| {
            FeatureTracker::new(feature_hash)
        });
        tracker.observe(value);

        if self.features.len() > MAX_SIGNALS {
            // Evict least-observed feature
            let mut min_total = u64::MAX;
            let mut min_key = 0u64;
            for (k, v) in self.features.iter() {
                if v.total < min_total {
                    min_total = v.total;
                    min_key = *k;
                }
            }
            self.features.remove(&min_key);
        }

        self.stats.total_features_tracked = self.features.len() as u32;
    }

    /// Run pre-cognitive sensing across all tracked features.
    pub fn precognitive_sense(&mut self) -> Vec<EarlySignal> {
        let tick = self.tick;
        let mut all_signals = Vec::new();

        let keys: Vec<u64> = self.features.keys().copied().collect();
        for key in keys {
            if let Some(tracker) = self.features.get_mut(&key) {
                let signals = tracker.detect_shifts(tick);
                for s in signals {
                    all_signals.push(s);
                }
            }
        }

        self.total_signals += all_signals.len() as u64;
        self.stats.total_signals_detected = self.total_signals;

        // Update average magnitude
        if !all_signals.is_empty() {
            let avg_mag: f32 =
                all_signals.iter().map(|s| s.magnitude).sum::<f32>() / all_signals.len() as f32;
            self.stats.avg_signal_magnitude = self.stats.avg_signal_magnitude * (1.0 - EMA_ALPHA)
                + avg_mag * EMA_ALPHA;

            let avg_lead: f32 = all_signals
                .iter()
                .map(|s| s.estimated_lead_ticks as f32)
                .sum::<f32>()
                / all_signals.len() as f32;
            self.stats.avg_lead_ticks =
                self.stats.avg_lead_ticks * (1.0 - EMA_ALPHA) + avg_lead * EMA_ALPHA;
        }

        // Update active drifts
        self.stats.active_drifts = self
            .features
            .values()
            .filter(|f| f.shift_detected)
            .count() as u32;

        // Store recent signals
        self.recent_signals.extend(all_signals.iter().cloned());
        if self.recent_signals.len() > MAX_HISTORY {
            let drain_count = self.recent_signals.len() - MAX_HISTORY;
            self.recent_signals.drain(0..drain_count);
        }

        all_signals
    }

    /// Detect subtle shifts for a specific feature.
    pub fn subtle_shift_detection(&mut self, feature_hash: u64) -> Vec<EarlySignal> {
        let tick = self.tick;
        if let Some(tracker) = self.features.get_mut(&feature_hash) {
            tracker.detect_shifts(tick)
        } else {
            Vec::new()
        }
    }

    /// Measure distribution drift for a specific feature.
    pub fn distribution_drift(&self, feature_hash: u64) -> f32 {
        if let Some(tracker) = self.features.get(&feature_hash) {
            tracker
                .reference_fingerprint
                .js_divergence(&tracker.current_fingerprint)
        } else {
            0.0
        }
    }

    /// Get the most recent early signal across all features.
    pub fn early_signal(&self) -> Option<&EarlySignal> {
        self.recent_signals.last()
    }

    /// Confirm that a precognitive signal was correct (the predicted change materialized).
    pub fn confirm_signal(&mut self, feature_hash: u64) {
        self.confirmed_signals += 1;
        self.stats.total_signals_confirmed = self.confirmed_signals;

        if let Some(tracker) = self.features.get_mut(&feature_hash) {
            tracker.confirmed_signals += 1;
        }

        self.update_accuracy();
    }

    /// Record a false positive: signal was raised but no change materialized.
    pub fn record_false_positive(&mut self, _feature_hash: u64) {
        self.false_positives += 1;
        self.update_accuracy();
    }

    fn update_accuracy(&mut self) {
        let total = self.confirmed_signals + self.false_positives;
        if total > 0 {
            self.stats.precognition_accuracy =
                self.confirmed_signals as f32 / total as f32;
            self.stats.false_positive_rate =
                self.false_positives as f32 / total as f32;
        }
    }

    /// Get overall precognition accuracy.
    pub fn precognition_accuracy(&self) -> f32 {
        self.stats.precognition_accuracy
    }

    /// Get false positive rate.
    pub fn false_positive_rate(&self) -> f32 {
        self.stats.false_positive_rate
    }

    /// Get statistics.
    pub fn stats(&self) -> &PrecognitionStats {
        &self.stats
    }

    /// Get the number of tracked features.
    pub fn feature_count(&self) -> usize {
        self.features.len()
    }
}
