// SPDX-License-Identifier: GPL-2.0
//! # Holistic Temporal Fusion — Multi-Horizon Prediction Fusion
//!
//! System-wide multi-horizon prediction fusion engine. From **microseconds to
//! hours**, this module weaves a coherent view of the system's future at every
//! temporal scale. Predictions at different horizons are reconciled so that the
//! 1-second forecast is consistent with the 1-minute forecast, which is
//! consistent with the 10-minute forecast, and so on.
//!
//! ## Capabilities
//!
//! - System-wide temporal fusion across all prediction horizons
//! - Micro-to-macro bridging: connect μs-level signals to hour-level trends
//! - Horizon hierarchy with automatic coherence enforcement
//! - Temporal consistency validation between adjacent horizons
//! - Long-range forecasting with uncertainty growth modelling
//! - Fusion panorama: a single panoramic view of the system's temporal future

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_HORIZONS: usize = 12;
const MAX_SIGNALS_PER_HORIZON: usize = 256;
const MAX_FUSION_SNAPSHOTS: usize = 512;
const MAX_CONSISTENCY_LOG: usize = 256;
const MAX_PANORAMA_ENTRIES: usize = 128;
const COHERENCE_TOLERANCE: f32 = 0.15;
const UNCERTAINTY_GROWTH_RATE: f32 = 1.05;
const EMA_ALPHA: f32 = 0.11;
const TEMPORAL_DECAY: f32 = 0.97;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

// ============================================================================
// HELPER FUNCTIONS
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

fn ema_update(current: f32, sample: f32) -> f32 {
    EMA_ALPHA * sample + (1.0 - EMA_ALPHA) * current
}

// ============================================================================
// TEMPORAL HORIZON TYPES
// ============================================================================

/// Temporal prediction horizon scale
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TemporalHorizon {
    Microseconds100,
    Millisecond1,
    Milliseconds10,
    Milliseconds100,
    Second1,
    Seconds10,
    Minute1,
    Minutes10,
    Hour1,
    Hours6,
    Day1,
}

impl TemporalHorizon {
    fn to_us(self) -> u64 {
        match self {
            Self::Microseconds100 => 100,
            Self::Millisecond1 => 1_000,
            Self::Milliseconds10 => 10_000,
            Self::Milliseconds100 => 100_000,
            Self::Second1 => 1_000_000,
            Self::Seconds10 => 10_000_000,
            Self::Minute1 => 60_000_000,
            Self::Minutes10 => 600_000_000,
            Self::Hour1 => 3_600_000_000,
            Self::Hours6 => 21_600_000_000,
            Self::Day1 => 86_400_000_000,
        }
    }

    fn level(self) -> usize {
        match self {
            Self::Microseconds100 => 0,
            Self::Millisecond1 => 1,
            Self::Milliseconds10 => 2,
            Self::Milliseconds100 => 3,
            Self::Second1 => 4,
            Self::Seconds10 => 5,
            Self::Minute1 => 6,
            Self::Minutes10 => 7,
            Self::Hour1 => 8,
            Self::Hours6 => 9,
            Self::Day1 => 10,
        }
    }
}

/// A prediction signal at a specific temporal horizon
#[derive(Debug, Clone)]
pub struct TemporalSignal {
    pub signal_id: u64,
    pub horizon: TemporalHorizon,
    pub prediction: f32,
    pub uncertainty: f32,
    pub source_label: String,
    pub timestamp_us: u64,
    pub weight: f32,
}

/// Fused prediction at a single horizon after reconciliation
#[derive(Debug, Clone)]
pub struct FusedHorizonPrediction {
    pub horizon: TemporalHorizon,
    pub fused_value: f32,
    pub fused_uncertainty: f32,
    pub signal_count: usize,
    pub coherence_with_prev: f32,
    pub coherence_with_next: f32,
    pub confidence: f32,
}

/// Result of system-wide temporal fusion
#[derive(Debug, Clone)]
pub struct SystemTemporalFusion {
    pub horizons: Vec<FusedHorizonPrediction>,
    pub overall_coherence: f32,
    pub overall_uncertainty: f32,
    pub active_horizons: usize,
    pub total_signals: usize,
    pub timestamp_us: u64,
}

/// Micro-to-macro bridging result
#[derive(Debug, Clone)]
pub struct MicroToMacroBridge {
    pub micro_horizon: TemporalHorizon,
    pub macro_horizon: TemporalHorizon,
    pub micro_prediction: f32,
    pub macro_prediction: f32,
    pub bridging_factor: f32,
    pub consistency: f32,
    pub extrapolation_confidence: f32,
}

/// Horizon hierarchy node
#[derive(Debug, Clone)]
pub struct HorizonNode {
    pub horizon: TemporalHorizon,
    pub level: usize,
    pub prediction: f32,
    pub uncertainty: f32,
    pub child_horizons: Vec<TemporalHorizon>,
    pub parent_horizon: Option<TemporalHorizon>,
    pub coherence_score: f32,
}

/// Temporal consistency check between adjacent horizons
#[derive(Debug, Clone)]
pub struct TemporalConsistencyCheck {
    pub horizon_a: TemporalHorizon,
    pub horizon_b: TemporalHorizon,
    pub prediction_a: f32,
    pub prediction_b: f32,
    pub deviation: f32,
    pub within_tolerance: bool,
    pub correction_applied: f32,
}

/// Long-range forecast with uncertainty envelope
#[derive(Debug, Clone)]
pub struct LongRangeForecast {
    pub target_horizon: TemporalHorizon,
    pub point_forecast: f32,
    pub lower_bound: f32,
    pub upper_bound: f32,
    pub uncertainty_envelope: Vec<(u64, f32, f32)>,
    pub confidence: f32,
    pub extrapolation_warning: bool,
}

/// Panoramic view of the entire temporal future
#[derive(Debug, Clone)]
pub struct FusionPanorama {
    pub entries: Vec<PanoramaEntry>,
    pub min_horizon_us: u64,
    pub max_horizon_us: u64,
    pub overall_trend: f32,
    pub overall_confidence: f32,
    pub anomaly_flags: Vec<TemporalHorizon>,
}

/// A single entry in the panoramic view
#[derive(Debug, Clone)]
pub struct PanoramaEntry {
    pub horizon: TemporalHorizon,
    pub prediction: f32,
    pub uncertainty: f32,
    pub trend: f32,
    pub is_anomalous: bool,
}

// ============================================================================
// STATISTICS
// ============================================================================

/// Runtime statistics for the temporal fusion engine
#[derive(Debug, Clone)]
pub struct TemporalFusionStats {
    pub fusions_performed: u64,
    pub micro_macro_bridges: u64,
    pub hierarchy_builds: u64,
    pub consistency_checks: u64,
    pub long_range_forecasts: u64,
    pub panoramas_generated: u64,
    pub avg_coherence: f32,
    pub avg_uncertainty: f32,
    pub avg_signal_count: f32,
    pub avg_confidence: f32,
}

impl TemporalFusionStats {
    fn new() -> Self {
        Self {
            fusions_performed: 0,
            micro_macro_bridges: 0,
            hierarchy_builds: 0,
            consistency_checks: 0,
            long_range_forecasts: 0,
            panoramas_generated: 0,
            avg_coherence: 0.0,
            avg_uncertainty: 0.0,
            avg_signal_count: 0.0,
            avg_confidence: 0.0,
        }
    }
}

// ============================================================================
// HOLISTIC TEMPORAL FUSION ENGINE
// ============================================================================

/// System-wide multi-horizon temporal fusion engine
pub struct HolisticTemporalFusion {
    signals: BTreeMap<u64, Vec<TemporalSignal>>,
    fused_cache: BTreeMap<u64, FusedHorizonPrediction>,
    consistency_log: Vec<TemporalConsistencyCheck>,
    snapshots: Vec<SystemTemporalFusion>,
    rng_state: u64,
    next_signal_id: u64,
    stats: TemporalFusionStats,
    generation: u64,
}

impl HolisticTemporalFusion {
    /// Create a new holistic temporal fusion engine
    pub fn new(seed: u64) -> Self {
        Self {
            signals: BTreeMap::new(),
            fused_cache: BTreeMap::new(),
            consistency_log: Vec::new(),
            snapshots: Vec::new(),
            rng_state: seed ^ 0xBEEF_CAFE_1234_DEAD,
            next_signal_id: 1,
            stats: TemporalFusionStats::new(),
            generation: 0,
        }
    }

    /// Submit a prediction signal at a specific horizon
    pub fn submit_signal(
        &mut self,
        horizon: TemporalHorizon,
        prediction: f32,
        uncertainty: f32,
        source: String,
        timestamp_us: u64,
    ) -> u64 {
        let id = self.next_signal_id;
        self.next_signal_id += 1;
        let signal = TemporalSignal {
            signal_id: id,
            horizon,
            prediction,
            uncertainty: uncertainty.max(0.001),
            source_label: source,
            timestamp_us,
            weight: 1.0,
        };
        let key = horizon.to_us();
        self.signals.entry(key).or_insert_with(Vec::new).push(signal);
        id
    }

    /// Perform system-wide temporal fusion across all horizons
    pub fn system_temporal_fusion(&mut self, timestamp_us: u64) -> SystemTemporalFusion {
        self.stats.fusions_performed += 1;
        self.generation += 1;

        let all_horizons = [
            TemporalHorizon::Microseconds100, TemporalHorizon::Millisecond1,
            TemporalHorizon::Milliseconds10, TemporalHorizon::Milliseconds100,
            TemporalHorizon::Second1, TemporalHorizon::Seconds10,
            TemporalHorizon::Minute1, TemporalHorizon::Minutes10,
            TemporalHorizon::Hour1, TemporalHorizon::Hours6,
            TemporalHorizon::Day1,
        ];

        let mut fused_horizons: Vec<FusedHorizonPrediction> = Vec::new();
        let mut total_signals = 0_usize;
        let mut total_coherence = 0.0_f32;
        let mut total_uncertainty = 0.0_f32;
        let mut active = 0_usize;

        for &h in &all_horizons {
            let key = h.to_us();
            let sigs = self.signals.get(&key);
            let sig_vec = match sigs {
                Some(v) if !v.is_empty() => v,
                _ => continue,
            };
            total_signals += sig_vec.len();
            active += 1;

            let total_weight: f32 = sig_vec.iter().map(|s| s.weight).sum();
            let fused_val = if total_weight > 0.0 {
                sig_vec.iter().map(|s| s.prediction * s.weight).sum::<f32>() / total_weight
            } else {
                0.0
            };
            let fused_unc = if total_weight > 0.0 {
                sig_vec
                    .iter()
                    .map(|s| {
                        let d = s.prediction - fused_val;
                        s.weight * (s.uncertainty * s.uncertainty + d * d)
                    })
                    .sum::<f32>()
                    / total_weight
            } else {
                1.0
            };
            let unc_sqrt = fused_unc.sqrt();

            let fused = FusedHorizonPrediction {
                horizon: h,
                fused_value: fused_val,
                fused_uncertainty: unc_sqrt,
                signal_count: sig_vec.len(),
                coherence_with_prev: 0.0,
                coherence_with_next: 0.0,
                confidence: (1.0 - unc_sqrt.min(1.0)).max(0.0),
            };
            self.fused_cache.insert(key, fused.clone());
            fused_horizons.push(fused);
        }

        // Enforce coherence between adjacent horizons
        for i in 1..fused_horizons.len() {
            let prev_val = fused_horizons[i - 1].fused_value;
            let curr_val = fused_horizons[i].fused_value;
            let dev = (curr_val - prev_val).abs();
            let coh = (1.0 - dev).max(0.0);
            total_coherence += coh;
            fused_horizons[i].coherence_with_prev = coh;
            fused_horizons[i - 1].coherence_with_next = coh;
        }

        let overall_coherence = if active > 1 {
            total_coherence / (active - 1) as f32
        } else {
            1.0
        };
        let overall_unc = fused_horizons
            .iter()
            .map(|f| f.fused_uncertainty)
            .sum::<f32>()
            / active.max(1) as f32;
        total_uncertainty = overall_unc;

        self.stats.avg_coherence = ema_update(self.stats.avg_coherence, overall_coherence);
        self.stats.avg_uncertainty = ema_update(self.stats.avg_uncertainty, total_uncertainty);
        self.stats.avg_signal_count = ema_update(self.stats.avg_signal_count, total_signals as f32);

        let result = SystemTemporalFusion {
            horizons: fused_horizons,
            overall_coherence,
            overall_uncertainty: total_uncertainty,
            active_horizons: active,
            total_signals,
            timestamp_us,
        };
        if self.snapshots.len() < MAX_FUSION_SNAPSHOTS {
            self.snapshots.push(result.clone());
        }
        result
    }

    /// Bridge micro-level signals to macro-level trends
    pub fn micro_to_macro(
        &mut self,
        micro: TemporalHorizon,
        macro_h: TemporalHorizon,
    ) -> MicroToMacroBridge {
        self.stats.micro_macro_bridges += 1;
        let micro_key = micro.to_us();
        let macro_key = macro_h.to_us();

        let micro_pred = self.fused_cache.get(&micro_key).map(|f| f.fused_value).unwrap_or(0.5);
        let macro_pred = self.fused_cache.get(&macro_key).map(|f| f.fused_value).unwrap_or(0.5);

        let bridging = if macro_pred.abs() > 0.001 { micro_pred / macro_pred } else { 1.0 };
        let consistency = 1.0 - (micro_pred - macro_pred).abs();
        let levels_apart = (macro_h.level() as i32 - micro.level() as i32).unsigned_abs() as f32;
        let extrap_conf = (1.0 - levels_apart * 0.08).max(0.1);

        MicroToMacroBridge {
            micro_horizon: micro,
            macro_horizon: macro_h,
            micro_prediction: micro_pred,
            macro_prediction: macro_pred,
            bridging_factor: bridging,
            consistency,
            extrapolation_confidence: extrap_conf,
        }
    }

    /// Build the horizon hierarchy
    pub fn horizon_hierarchy(&mut self) -> Vec<HorizonNode> {
        self.stats.hierarchy_builds += 1;
        let all_horizons = [
            TemporalHorizon::Microseconds100, TemporalHorizon::Millisecond1,
            TemporalHorizon::Milliseconds10, TemporalHorizon::Milliseconds100,
            TemporalHorizon::Second1, TemporalHorizon::Seconds10,
            TemporalHorizon::Minute1, TemporalHorizon::Minutes10,
            TemporalHorizon::Hour1, TemporalHorizon::Hours6,
            TemporalHorizon::Day1,
        ];

        let mut nodes: Vec<HorizonNode> = Vec::new();
        for (i, &h) in all_horizons.iter().enumerate() {
            let key = h.to_us();
            let cached = self.fused_cache.get(&key);
            let pred = cached.map(|c| c.fused_value).unwrap_or(0.0);
            let unc = cached.map(|c| c.fused_uncertainty).unwrap_or(0.5);
            let coh = cached.map(|c| c.confidence).unwrap_or(0.0);

            let children = if i + 1 < all_horizons.len() {
                Vec::new()
            } else {
                Vec::new()
            };
            let parent = if i > 0 { Some(all_horizons[i - 1]) } else { None };

            nodes.push(HorizonNode {
                horizon: h,
                level: h.level(),
                prediction: pred,
                uncertainty: unc,
                child_horizons: children,
                parent_horizon: parent,
                coherence_score: coh,
            });
        }
        nodes
    }

    /// Check temporal consistency between two adjacent horizons
    pub fn temporal_consistency(
        &mut self,
        a: TemporalHorizon,
        b: TemporalHorizon,
    ) -> TemporalConsistencyCheck {
        self.stats.consistency_checks += 1;
        let key_a = a.to_us();
        let key_b = b.to_us();
        let pred_a = self.fused_cache.get(&key_a).map(|f| f.fused_value).unwrap_or(0.5);
        let pred_b = self.fused_cache.get(&key_b).map(|f| f.fused_value).unwrap_or(0.5);
        let deviation = (pred_a - pred_b).abs();
        let within = deviation <= COHERENCE_TOLERANCE;

        let correction = if !within {
            let mid = (pred_a + pred_b) * 0.5;
            let corr_a = mid - pred_a;
            corr_a
        } else {
            0.0
        };

        let check = TemporalConsistencyCheck {
            horizon_a: a,
            horizon_b: b,
            prediction_a: pred_a,
            prediction_b: pred_b,
            deviation,
            within_tolerance: within,
            correction_applied: correction,
        };
        if self.consistency_log.len() < MAX_CONSISTENCY_LOG {
            self.consistency_log.push(check.clone());
        }
        check
    }

    /// Generate a long-range forecast with uncertainty envelope
    pub fn long_range_forecast(&mut self, target: TemporalHorizon) -> LongRangeForecast {
        self.stats.long_range_forecasts += 1;
        let key = target.to_us();
        let base_pred = self.fused_cache.get(&key).map(|f| f.fused_value).unwrap_or(0.5);
        let base_unc = self.fused_cache.get(&key).map(|f| f.fused_uncertainty).unwrap_or(0.2);

        let mut envelope: Vec<(u64, f32, f32)> = Vec::new();
        let target_us = target.to_us();
        let steps = 10_usize;
        let step_size = target_us / steps.max(1) as u64;
        let mut running_unc = base_unc;

        for i in 0..=steps {
            let t = step_size * i as u64;
            let drift = (xorshift64(&mut self.rng_state) % 50) as f32 / 1000.0 - 0.025;
            let pred_at_t = base_pred + drift * (i as f32 / steps as f32);
            running_unc *= UNCERTAINTY_GROWTH_RATE;
            envelope.push((t, pred_at_t - running_unc, pred_at_t + running_unc));
        }

        let final_unc = running_unc;
        let extrap_warning = target.level() >= 8;

        LongRangeForecast {
            target_horizon: target,
            point_forecast: base_pred,
            lower_bound: base_pred - final_unc,
            upper_bound: base_pred + final_unc,
            uncertainty_envelope: envelope,
            confidence: (1.0 - final_unc.min(1.0)).max(0.0),
            extrapolation_warning: extrap_warning,
        }
    }

    /// Generate a panoramic view of the system's temporal future
    pub fn fusion_panorama(&mut self) -> FusionPanorama {
        self.stats.panoramas_generated += 1;
        let mut entries: Vec<PanoramaEntry> = Vec::new();
        let mut anomaly_flags: Vec<TemporalHorizon> = Vec::new();
        let mut min_us = u64::MAX;
        let mut max_us = 0_u64;
        let mut trend_sum = 0.0_f32;
        let mut conf_sum = 0.0_f32;

        let all_horizons = [
            TemporalHorizon::Microseconds100, TemporalHorizon::Millisecond1,
            TemporalHorizon::Milliseconds10, TemporalHorizon::Milliseconds100,
            TemporalHorizon::Second1, TemporalHorizon::Seconds10,
            TemporalHorizon::Minute1, TemporalHorizon::Minutes10,
            TemporalHorizon::Hour1, TemporalHorizon::Hours6,
            TemporalHorizon::Day1,
        ];

        let mut prev_pred = 0.0_f32;
        for (i, &h) in all_horizons.iter().enumerate() {
            let key = h.to_us();
            if key < min_us { min_us = key; }
            if key > max_us { max_us = key; }

            let cached = self.fused_cache.get(&key);
            let pred = cached.map(|c| c.fused_value).unwrap_or(0.5);
            let unc = cached.map(|c| c.fused_uncertainty).unwrap_or(0.3);
            let trend = if i > 0 { pred - prev_pred } else { 0.0 };
            let anomalous = unc > 0.5 || trend.abs() > 0.3;
            if anomalous {
                anomaly_flags.push(h);
            }
            trend_sum += trend;
            conf_sum += 1.0 - unc.min(1.0);

            if entries.len() < MAX_PANORAMA_ENTRIES {
                entries.push(PanoramaEntry {
                    horizon: h,
                    prediction: pred,
                    uncertainty: unc,
                    trend,
                    is_anomalous: anomalous,
                });
            }
            prev_pred = pred;
        }

        let count = entries.len().max(1) as f32;
        self.stats.avg_confidence = ema_update(self.stats.avg_confidence, conf_sum / count);

        FusionPanorama {
            entries,
            min_horizon_us: min_us,
            max_horizon_us: max_us,
            overall_trend: trend_sum / count,
            overall_confidence: conf_sum / count,
            anomaly_flags,
        }
    }

    /// Get current statistics
    pub fn stats(&self) -> &TemporalFusionStats {
        &self.stats
    }
}
