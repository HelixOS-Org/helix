// SPDX-License-Identifier: GPL-2.0
//! # Holistic Emotion Engine
//!
//! **SYSTEM-WIDE emotional state.** Aggregates every subsystem's emotional
//! signals into a single, unified emotional landscape that influences ALL
//! kernel decisions. Where per-subsystem emotion engines see local feelings,
//! this engine perceives the *gestalt* — the combined emotional reality of
//! the entire operating system.
//!
//! ## System Emotions
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │                 EMOTIONAL LANDSCAPE                          │
//! ├──────────────────────────────────────────────────────────────┤
//! │  SystemStress ◄──── aggregate pressure across all modules   │
//! │  SystemConfidence ◄ belief in ability to meet all goals     │
//! │  SystemCuriosity ◄─ drive to explore and optimize           │
//! │  SystemSatisfaction ◄ all objectives met, resources balanced│
//! │  SystemAlarm ◄───── critical failure imminent               │
//! │  SystemSerenity ◄── everything running harmoniously         │
//! │  SystemDetermination ◄ persistence under adversity          │
//! │  SystemAwe ◄─────── unexpected emergent beauty detected     │
//! └──────────────────────────────────────────────────────────────┘
//! ```
//!
//! The dominant emotion plus its intensity directly shapes scheduling
//! policy, memory reclamation aggressiveness, and I/O priority tuning.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.12;
const EMOTION_DECAY: f32 = 0.991;
const MAX_SUBSYSTEMS: usize = 64;
const MAX_EMOTION_HISTORY: usize = 256;
const MAX_FORECAST_HORIZON: usize = 16;
const STRESS_CRITICAL: f32 = 0.85;
const ALARM_CRITICAL: f32 = 0.90;
const SERENITY_THRESHOLD: f32 = 0.75;
const CONFIDENCE_BASELINE: f32 = 0.50;
const CURIOSITY_BASELINE: f32 = 0.30;
const FUSION_WEIGHT_SUM: f32 = 1.0;
const TREND_WINDOW: usize = 32;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const LANDSCAPE_BLEND: f32 = 0.25;

// ============================================================================
// FNV-1a HASHING
// ============================================================================

fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Xorshift64 PRNG for stochastic jitter
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

// ============================================================================
// SYSTEM EMOTION ENUM
// ============================================================================

/// The eight fundamental system-wide emotions
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SystemEmotion {
    /// Aggregate pressure across all subsystems
    SystemStress,
    /// Belief the system can meet all its goals
    SystemConfidence,
    /// Drive to explore, optimize, and learn
    SystemCuriosity,
    /// All objectives met, resources well-balanced
    SystemSatisfaction,
    /// Critical failure pathway detected
    SystemAlarm,
    /// Everything running in perfect harmony
    SystemSerenity,
    /// Persistence and resilience under adversity
    SystemDetermination,
    /// Unexpected emergent beauty or synergy detected
    SystemAwe,
}

impl SystemEmotion {
    /// All variants for iteration
    pub fn all() -> &'static [SystemEmotion] {
        &[
            SystemEmotion::SystemStress,
            SystemEmotion::SystemConfidence,
            SystemEmotion::SystemCuriosity,
            SystemEmotion::SystemSatisfaction,
            SystemEmotion::SystemAlarm,
            SystemEmotion::SystemSerenity,
            SystemEmotion::SystemDetermination,
            SystemEmotion::SystemAwe,
        ]
    }

    /// Base priority weight for decision influence
    pub fn decision_weight(&self) -> f32 {
        match self {
            SystemEmotion::SystemStress => 0.95,
            SystemEmotion::SystemConfidence => 0.60,
            SystemEmotion::SystemCuriosity => 0.40,
            SystemEmotion::SystemSatisfaction => 0.35,
            SystemEmotion::SystemAlarm => 1.00,
            SystemEmotion::SystemSerenity => 0.30,
            SystemEmotion::SystemDetermination => 0.80,
            SystemEmotion::SystemAwe => 0.20,
        }
    }

    /// Whether this emotion is negative / stress-inducing
    #[inline(always)]
    pub fn is_negative(&self) -> bool {
        matches!(self, SystemEmotion::SystemStress | SystemEmotion::SystemAlarm)
    }

    /// Whether this emotion is positive / restorative
    #[inline]
    pub fn is_positive(&self) -> bool {
        matches!(
            self,
            SystemEmotion::SystemConfidence
                | SystemEmotion::SystemSatisfaction
                | SystemEmotion::SystemSerenity
                | SystemEmotion::SystemAwe
        )
    }
}

// ============================================================================
// EMOTION SIGNAL
// ============================================================================

/// A single system-wide emotional signal with EMA smoothing
#[derive(Debug, Clone)]
pub struct SystemEmotionSignal {
    pub kind: SystemEmotion,
    /// EMA-smoothed intensity (0.0 – 1.0)
    pub intensity: f32,
    /// Raw intensity before smoothing
    pub raw_intensity: f32,
    /// Number of subsystem contributions aggregated
    pub source_count: u32,
    /// Trigger count
    pub trigger_count: u64,
    /// Last update tick
    pub last_tick: u64,
    /// Peak intensity ever recorded
    pub peak: f32,
    /// Variance accumulator for confidence bands
    pub variance_accum: f32,
    /// Ring buffer of historical intensities
    history: Vec<f32>,
    write_idx: usize,
}

impl SystemEmotionSignal {
    pub fn new(kind: SystemEmotion) -> Self {
        let mut history = Vec::with_capacity(MAX_EMOTION_HISTORY);
        for _ in 0..MAX_EMOTION_HISTORY {
            history.push(0.0);
        }
        Self {
            kind,
            intensity: 0.0,
            raw_intensity: 0.0,
            source_count: 0,
            trigger_count: 0,
            last_tick: 0,
            peak: 0.0,
            variance_accum: 0.0,
            history,
            write_idx: 0,
        }
    }

    /// Observe a new raw value and update EMA
    #[inline]
    pub fn observe(&mut self, raw: f32, tick: u64) {
        let clamped = if raw < 0.0 { 0.0 } else if raw > 1.0 { 1.0 } else { raw };
        self.raw_intensity = clamped;
        let delta = clamped - self.intensity;
        self.intensity += EMA_ALPHA * delta;
        self.variance_accum += EMA_ALPHA * (delta * delta - self.variance_accum);
        if self.intensity > self.peak {
            self.peak = self.intensity;
        }
        self.history[self.write_idx] = clamped;
        self.write_idx = (self.write_idx + 1) % MAX_EMOTION_HISTORY;
        self.trigger_count += 1;
        self.last_tick = tick;
    }

    /// Decay this emotion toward baseline with stochastic jitter
    #[inline]
    pub fn decay(&mut self, rng: &mut u64) {
        let jitter_raw = xorshift64(rng);
        let jitter = (jitter_raw % 100) as f32 / 100_000.0;
        self.intensity *= EMOTION_DECAY - jitter;
        if self.intensity < 0.001 {
            self.intensity = 0.0;
        }
    }

    /// Compute trend from recent history (positive = rising)
    pub fn trend(&self) -> f32 {
        if self.trigger_count < 2 {
            return 0.0;
        }
        let end = self.write_idx;
        let start = if end >= TREND_WINDOW { end - TREND_WINDOW } else { 0 };
        let window = &self.history[start..end.max(1)];
        if window.len() < 2 {
            return 0.0;
        }
        let first_half: f32 = window[..window.len() / 2].iter().sum::<f32>()
            / (window.len() / 2) as f32;
        let second_half: f32 = window[window.len() / 2..].iter().sum::<f32>()
            / (window.len() - window.len() / 2) as f32;
        second_half - first_half
    }
}

// ============================================================================
// EMOTIONAL LANDSCAPE
// ============================================================================

/// The unified emotional landscape of the entire system
#[derive(Debug, Clone)]
pub struct EmotionalLandscape {
    /// Dominant emotion right now
    pub dominant: SystemEmotion,
    /// Secondary (runner-up) emotion
    pub secondary: SystemEmotion,
    /// Overall emotional intensity (0.0 – 1.0)
    pub intensity: f32,
    /// Trend direction: positive = improving, negative = worsening
    pub trend: f32,
    /// Valence: -1.0 (entirely negative) to 1.0 (entirely positive)
    pub valence: f32,
    /// Arousal: 0.0 (calm) to 1.0 (highly activated)
    pub arousal: f32,
    /// Tick when landscape was last computed
    pub tick: u64,
}

// ============================================================================
// SUBSYSTEM EMOTION INPUT
// ============================================================================

/// Emotion input from a single subsystem
#[derive(Debug, Clone)]
pub struct SubsystemEmotionInput {
    pub subsystem_name: String,
    pub subsystem_id: u64,
    /// Per-emotion-kind raw intensities from this subsystem
    pub emotion_values: BTreeMap<u8, f32>,
    /// Trust weight: how much to trust this subsystem's emotion reports
    pub trust_weight: f32,
    pub tick: u64,
}

// ============================================================================
// EMOTION FORECAST
// ============================================================================

/// Predicted future emotional state
#[derive(Debug, Clone, Copy)]
pub struct EmotionForecast {
    pub horizon_ticks: u64,
    pub predicted_dominant: SystemEmotion,
    pub predicted_intensity: f32,
    pub predicted_valence: f32,
    pub confidence: f32,
}

// ============================================================================
// STATS
// ============================================================================

/// Statistics for the holistic emotion engine
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct HolisticEmotionStats {
    pub total_observations: u64,
    pub total_fusions: u64,
    pub landscape_updates: u64,
    pub forecasts_generated: u64,
    pub alarm_events: u64,
    pub serenity_episodes: u64,
    pub average_valence: f32,
    pub average_arousal: f32,
    pub dominant_distribution: BTreeMap<u8, u64>,
}

// ============================================================================
// HOLISTIC EMOTION ENGINE
// ============================================================================

/// The system-wide emotion engine — aggregates all subsystem emotions
/// into a unified emotional landscape that influences every kernel decision.
pub struct HolisticEmotionEngine {
    /// Per-emotion-kind signal accumulators
    signals: BTreeMap<u8, SystemEmotionSignal>,
    /// Per-subsystem latest inputs
    subsystem_inputs: BTreeMap<u64, SubsystemEmotionInput>,
    /// Current emotional landscape
    landscape: EmotionalLandscape,
    /// Landscape history ring buffer
    landscape_history: Vec<EmotionalLandscape>,
    landscape_write_idx: usize,
    /// Forecast cache
    forecasts: Vec<EmotionForecast>,
    /// Running stats
    stats: HolisticEmotionStats,
    /// PRNG state
    rng: u64,
    /// Current tick
    tick: u64,
}

impl HolisticEmotionEngine {
    /// Create a new holistic emotion engine
    pub fn new(seed: u64) -> Self {
        let mut signals = BTreeMap::new();
        for (i, kind) in SystemEmotion::all().iter().enumerate() {
            signals.insert(i as u8, SystemEmotionSignal::new(*kind));
        }
        let mut landscape_history = Vec::with_capacity(MAX_EMOTION_HISTORY);
        for _ in 0..MAX_EMOTION_HISTORY {
            landscape_history.push(EmotionalLandscape {
                dominant: SystemEmotion::SystemSerenity,
                secondary: SystemEmotion::SystemConfidence,
                intensity: 0.0,
                trend: 0.0,
                valence: 0.0,
                arousal: 0.0,
                tick: 0,
            });
        }
        let mut dominant_distribution = BTreeMap::new();
        for i in 0..SystemEmotion::all().len() {
            dominant_distribution.insert(i as u8, 0);
        }
        Self {
            signals,
            subsystem_inputs: BTreeMap::new(),
            landscape: EmotionalLandscape {
                dominant: SystemEmotion::SystemSerenity,
                secondary: SystemEmotion::SystemConfidence,
                intensity: 0.0,
                trend: 0.0,
                valence: 0.0,
                arousal: 0.0,
                tick: 0,
            },
            landscape_history,
            landscape_write_idx: 0,
            forecasts: Vec::with_capacity(MAX_FORECAST_HORIZON),
            stats: HolisticEmotionStats {
                total_observations: 0,
                total_fusions: 0,
                landscape_updates: 0,
                forecasts_generated: 0,
                alarm_events: 0,
                serenity_episodes: 0,
                average_valence: 0.0,
                average_arousal: 0.0,
                dominant_distribution,
            },
            rng: seed ^ 0xDEAD_CAFE_BABE_F00D,
            tick: 0,
        }
    }

    /// Retrieve the current dominant system emotion
    #[inline(always)]
    pub fn system_emotion(&self) -> SystemEmotion {
        self.landscape.dominant
    }

    /// Compute and return the full emotional landscape
    #[inline]
    pub fn emotional_landscape(&mut self, tick: u64) -> &EmotionalLandscape {
        self.tick = tick;
        self.recompute_landscape();
        &self.landscape
    }

    /// Fuse all subsystem emotion inputs into unified signals
    pub fn emotion_fusion(&mut self, tick: u64) {
        self.tick = tick;
        let mut per_kind_accum: BTreeMap<u8, (f32, f32)> = BTreeMap::new();
        for (kind_idx, _signal) in self.signals.iter() {
            per_kind_accum.insert(*kind_idx, (0.0, 0.0));
        }
        for (_sub_id, input) in self.subsystem_inputs.iter() {
            for (kind_idx, raw_val) in input.emotion_values.iter() {
                if let Some(accum) = per_kind_accum.get_mut(kind_idx) {
                    accum.0 += raw_val * input.trust_weight;
                    accum.1 += input.trust_weight;
                }
            }
        }
        for (kind_idx, (weighted_sum, weight_total)) in per_kind_accum.iter() {
            if *weight_total > 0.0 {
                let fused = weighted_sum / weight_total;
                if let Some(signal) = self.signals.get_mut(kind_idx) {
                    signal.observe(fused, tick);
                    signal.source_count = self.subsystem_inputs.len() as u32;
                }
            }
        }
        self.stats.total_fusions += 1;
    }

    /// Aggregate stress across all subsystems
    #[inline(always)]
    pub fn stress_aggregate(&self) -> f32 {
        let stress_idx = 0u8; // SystemStress is index 0
        self.signals.get(&stress_idx).map_or(0.0, |s| s.intensity)
    }

    /// Aggregate confidence across all subsystems
    #[inline]
    pub fn confidence_aggregate(&self) -> f32 {
        let confidence_idx = 1u8; // SystemConfidence is index 1
        self.signals
            .get(&confidence_idx)
            .map_or(CONFIDENCE_BASELINE, |s| s.intensity)
    }

    /// Generate emotional forecast for the next horizon ticks
    pub fn emotional_forecast(&mut self, horizon: u64) -> Vec<EmotionForecast> {
        self.forecasts.clear();
        let steps = (horizon as usize).min(MAX_FORECAST_HORIZON);
        for step in 1..=steps {
            let decay_factor = EMOTION_DECAY.powi(step as i32);
            let mut best_kind = SystemEmotion::SystemSerenity;
            let mut best_intensity: f32 = 0.0;
            for (_idx, signal) in self.signals.iter() {
                let projected = signal.intensity * decay_factor + signal.trend() * step as f32 * 0.1;
                let clamped = if projected < 0.0 { 0.0 } else if projected > 1.0 { 1.0 } else { projected };
                if clamped > best_intensity {
                    best_intensity = clamped;
                    best_kind = signal.kind;
                }
            }
            let valence = if best_kind.is_positive() { best_intensity } else { -best_intensity };
            let confidence = 1.0 / (1.0 + step as f32 * 0.15);
            self.forecasts.push(EmotionForecast {
                horizon_ticks: step as u64,
                predicted_dominant: best_kind,
                predicted_intensity: best_intensity,
                predicted_valence: valence,
                confidence,
            });
        }
        self.stats.forecasts_generated += 1;
        self.forecasts.clone()
    }

    /// Return snapshot of emotion history for a given emotion kind
    #[inline]
    pub fn emotion_history(&self, kind_idx: u8) -> Vec<f32> {
        self.signals
            .get(&kind_idx)
            .map(|s| s.history.clone())
            .unwrap_or_default()
    }

    /// Ingest a subsystem's emotion report
    #[inline]
    pub fn ingest_subsystem(&mut self, input: SubsystemEmotionInput) {
        let id = input.subsystem_id;
        self.stats.total_observations += 1;
        self.subsystem_inputs.insert(id, input);
    }

    /// Decay all emotion signals
    #[inline]
    pub fn decay_all(&mut self) {
        for (_idx, signal) in self.signals.iter_mut() {
            signal.decay(&mut self.rng);
        }
    }

    /// Get engine statistics
    #[inline(always)]
    pub fn stats(&self) -> &HolisticEmotionStats {
        &self.stats
    }

    /// Get intensity of a specific emotion
    #[inline(always)]
    pub fn emotion_intensity(&self, kind_idx: u8) -> f32 {
        self.signals.get(&kind_idx).map_or(0.0, |s| s.intensity)
    }

    /// Check if system is in alarm state
    #[inline(always)]
    pub fn is_alarm_active(&self) -> bool {
        let alarm_idx = 4u8; // SystemAlarm
        self.signals.get(&alarm_idx).map_or(false, |s| s.intensity > ALARM_CRITICAL)
    }

    /// Check if system is serene
    #[inline(always)]
    pub fn is_serene(&self) -> bool {
        let serenity_idx = 5u8; // SystemSerenity
        self.signals.get(&serenity_idx).map_or(false, |s| s.intensity > SERENITY_THRESHOLD)
    }

    // ========================================================================
    // INTERNAL
    // ========================================================================

    fn recompute_landscape(&mut self) {
        let mut best_idx: u8 = 0;
        let mut best_intensity: f32 = 0.0;
        let mut second_idx: u8 = 0;
        let mut second_intensity: f32 = 0.0;
        let mut total_positive: f32 = 0.0;
        let mut total_negative: f32 = 0.0;
        let mut total_arousal: f32 = 0.0;
        let mut count: f32 = 0.0;

        for (idx, signal) in self.signals.iter() {
            count += 1.0;
            total_arousal += signal.intensity;
            if signal.kind.is_positive() {
                total_positive += signal.intensity;
            }
            if signal.kind.is_negative() {
                total_negative += signal.intensity;
            }
            if signal.intensity > best_intensity {
                second_idx = best_idx;
                second_intensity = best_intensity;
                best_idx = *idx;
                best_intensity = signal.intensity;
            } else if signal.intensity > second_intensity {
                second_idx = *idx;
                second_intensity = signal.intensity;
            }
        }

        let dominant = self.signals.get(&best_idx).map_or(SystemEmotion::SystemSerenity, |s| s.kind);
        let secondary = self.signals.get(&second_idx).map_or(SystemEmotion::SystemConfidence, |s| s.kind);
        let valence = if (total_positive + total_negative) > 0.0 {
            (total_positive - total_negative) / (total_positive + total_negative)
        } else {
            0.0
        };
        let arousal = if count > 0.0 { total_arousal / count } else { 0.0 };

        let trend = self.signals.get(&best_idx).map_or(0.0, |s| s.trend());

        self.landscape = EmotionalLandscape {
            dominant,
            secondary,
            intensity: best_intensity,
            trend,
            valence,
            arousal,
            tick: self.tick,
        };

        self.landscape_history[self.landscape_write_idx] = self.landscape.clone();
        self.landscape_write_idx = (self.landscape_write_idx + 1) % MAX_EMOTION_HISTORY;
        self.stats.landscape_updates += 1;

        if let Some(cnt) = self.stats.dominant_distribution.get_mut(&best_idx) {
            *cnt += 1;
        }

        if self.is_alarm_active() {
            self.stats.alarm_events += 1;
        }
        if self.is_serene() {
            self.stats.serenity_episodes += 1;
        }

        let blend = LANDSCAPE_BLEND;
        self.stats.average_valence += blend * (valence - self.stats.average_valence);
        self.stats.average_arousal += blend * (arousal - self.stats.average_arousal);
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emotion_signal_ema() {
        let mut sig = SystemEmotionSignal::new(SystemEmotion::SystemStress);
        sig.observe(0.8, 1);
        assert!(sig.intensity > 0.0);
        assert!(sig.intensity < 0.8);
    }

    #[test]
    fn test_engine_creation() {
        let engine = HolisticEmotionEngine::new(42);
        assert_eq!(engine.system_emotion(), SystemEmotion::SystemSerenity);
        assert_eq!(engine.stress_aggregate(), 0.0);
    }

    #[test]
    fn test_landscape_computation() {
        let mut engine = HolisticEmotionEngine::new(123);
        let _ = engine.emotional_landscape(1);
        assert_eq!(engine.stats().landscape_updates, 1);
    }

    #[test]
    fn test_forecast_generation() {
        let mut engine = HolisticEmotionEngine::new(456);
        let forecasts = engine.emotional_forecast(8);
        assert_eq!(forecasts.len(), 8);
        for fc in &forecasts {
            assert!(fc.confidence > 0.0 && fc.confidence <= 1.0);
        }
    }

    #[test]
    fn test_fnv1a_deterministic() {
        let h1 = fnv1a_hash(b"SystemStress");
        let h2 = fnv1a_hash(b"SystemStress");
        assert_eq!(h1, h2);
        assert_ne!(h1, fnv1a_hash(b"SystemConfidence"));
    }
}
