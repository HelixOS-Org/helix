// SPDX-License-Identifier: GPL-2.0
//! # Holistic Precognition — System-Wide Pre-Cognitive Sensing
//!
//! The kernel **knows what's coming** before any statistical model can detect
//! it. This module detects **regime changes**, **phase transitions**, and
//! **paradigm shifts** — fundamental changes in the system's operating mode
//! that invalidate all existing models and require a complete recalibration.
//!
//! Where anomaly detection catches individual deviations and trend analysis
//! catches gradual shifts, precognition catches the *moments of discontinuity*:
//! when the system transitions from one stable mode to another entirely
//! different one.
//!
//! ## Capabilities
//!
//! - System-wide precognitive sensing via multi-signal correlation
//! - Regime change detection: is the system entering a new operating mode?
//! - Phase transition sensing: detecting critical points and bifurcations
//! - Paradigm shift alerting: when the rules of the system change
//! - Precognitive accuracy tracking: how reliable is our foresight?
//! - Transcendent foresight: meta-prediction of prediction capability

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_SIGNAL_WINDOW: usize = 4096;
const MAX_REGIME_HISTORY: usize = 128;
const MAX_PHASE_DETECTORS: usize = 64;
const MAX_PARADIGM_ALERTS: usize = 32;
const MAX_ACCURACY_LOG: usize = 1024;
const MAX_FORESIGHT_ENTRIES: usize = 256;
const REGIME_SENSITIVITY: f32 = 0.20;
const PHASE_CRITICAL_EXPONENT: f32 = 0.50;
const PARADIGM_THRESHOLD: f32 = 0.70;
const CORRELATION_WINDOW: usize = 64;
const EMA_ALPHA: f32 = 0.10;
const EMA_SLOW: f32 = 0.03;
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

#[inline]
fn ema_update(current: f32, sample: f32) -> f32 {
    EMA_ALPHA * sample + (1.0 - EMA_ALPHA) * current
}

fn ema_slow_update(current: f32, sample: f32) -> f32 {
    EMA_SLOW * sample + (1.0 - EMA_SLOW) * current
}

// ============================================================================
// DOMAIN TYPES
// ============================================================================

/// Subsystem signal source for precognition
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrecogSource {
    Scheduler,
    Memory,
    Io,
    Network,
    Thermal,
    Power,
    FileSystem,
    Ipc,
    Security,
    Driver,
    Userspace,
    SystemWide,
}

/// Operating regime: a stable mode of system behaviour
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OperatingRegime {
    Idle,
    LightLoad,
    SteadyState,
    HighThroughput,
    Overloaded,
    Degraded,
    Recovery,
    CriticalState,
    Unknown,
}

/// Phase transition type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PhaseTransitionKind {
    FirstOrder,
    SecondOrder,
    CriticalPoint,
    Bifurcation,
    Catastrophic,
    Gradual,
}

/// Paradigm shift type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParadigmShiftKind {
    WorkloadClassChange,
    ResourceTopologyChange,
    FailureModeChange,
    PerformanceModelBreak,
    SecurityPostureChange,
    ScalingRegimeChange,
}

// ============================================================================
// PRECOGNITIVE SIGNAL
// ============================================================================

/// A raw signal used for precognitive sensing
#[derive(Debug, Clone)]
pub struct PrecogSignal {
    pub signal_id: u64,
    pub source: PrecogSource,
    pub value: f32,
    pub timestamp_us: u64,
    pub ema_fast: f32,
    pub ema_slow: f32,
    pub derivative: f32,
}

/// Regime change detection result
#[derive(Debug, Clone)]
pub struct RegimeChangeDetection {
    pub previous_regime: OperatingRegime,
    pub detected_regime: OperatingRegime,
    pub confidence: f32,
    pub transition_sharpness: f32,
    pub signals_contributing: Vec<PrecogSource>,
    pub timestamp_us: u64,
    pub duration_estimate_us: u64,
    pub reversion_probability: f32,
}

/// Phase transition detection result
#[derive(Debug, Clone)]
pub struct PhaseTransitionSense {
    pub transition_kind: PhaseTransitionKind,
    pub order_parameter: f32,
    pub critical_exponent: f32,
    pub distance_to_critical: f32,
    pub susceptibility: f32,
    pub correlation_length: f32,
    pub confidence: f32,
    pub sources_involved: Vec<PrecogSource>,
    pub timestamp_us: u64,
}

/// Paradigm shift alert
#[derive(Debug, Clone)]
pub struct ParadigmShiftAlert {
    pub shift_kind: ParadigmShiftKind,
    pub severity: f32,
    pub confidence: f32,
    pub affected_models: Vec<String>,
    pub recommended_recalibration: Vec<PrecogSource>,
    pub timestamp_us: u64,
    pub description: String,
}

/// Precognitive accuracy record
#[derive(Debug, Clone)]
pub struct PrecogAccuracyRecord {
    pub prediction_type: String,
    pub predicted_at_us: u64,
    pub validated_at_us: u64,
    pub was_correct: bool,
    pub lead_time_us: u64,
    pub confidence_at_prediction: f32,
}

/// Precognitive accuracy report
#[derive(Debug, Clone)]
pub struct PrecogAccuracyReport {
    pub total_predictions: usize,
    pub correct_predictions: usize,
    pub accuracy: f32,
    pub average_lead_time_us: u64,
    pub regime_detection_accuracy: f32,
    pub phase_detection_accuracy: f32,
    pub paradigm_detection_accuracy: f32,
    pub running_accuracy: f32,
}

/// Transcendent foresight: meta-prediction of precognitive capability
#[derive(Debug, Clone)]
pub struct TranscendentForesight {
    pub current_capability: f32,
    pub capability_trend: f32,
    pub signal_quality: f32,
    pub model_coherence: f32,
    pub blind_spots: Vec<PrecogSource>,
    pub strongest_domains: Vec<PrecogSource>,
    pub overall_foresight_score: f32,
    pub recommendation: ForesightRecommendation,
}

/// Recommendation from transcendent foresight
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForesightRecommendation {
    MaintainCourse,
    IncreaseMonitoring,
    RecalibrateModels,
    ExpandSignalSources,
    ReduceNoise,
    FocusDomain,
}

// ============================================================================
// STATISTICS
// ============================================================================

/// Runtime statistics for the precognition engine
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PrecognitionStats {
    pub precog_cycles: u64,
    pub regime_changes_detected: u64,
    pub phase_transitions_sensed: u64,
    pub paradigm_shifts_alerted: u64,
    pub accuracy_evaluations: u64,
    pub foresight_reports: u64,
    pub avg_confidence: f32,
    pub avg_lead_time_us: f32,
    pub avg_accuracy: f32,
    pub avg_foresight_score: f32,
}

impl PrecognitionStats {
    fn new() -> Self {
        Self {
            precog_cycles: 0,
            regime_changes_detected: 0,
            phase_transitions_sensed: 0,
            paradigm_shifts_alerted: 0,
            accuracy_evaluations: 0,
            foresight_reports: 0,
            avg_confidence: 0.0,
            avg_lead_time_us: 0.0,
            avg_accuracy: 0.0,
            avg_foresight_score: 0.0,
        }
    }
}

// ============================================================================
// HOLISTIC PRECOGNITION ENGINE
// ============================================================================

/// System-wide precognitive sensing engine
pub struct HolisticPrecognition {
    signal_window: BTreeMap<u64, Vec<PrecogSignal>>,
    current_regime: OperatingRegime,
    regime_history: Vec<RegimeChangeDetection>,
    phase_history: Vec<PhaseTransitionSense>,
    paradigm_alerts: Vec<ParadigmShiftAlert>,
    accuracy_log: Vec<PrecogAccuracyRecord>,
    source_ema_fast: LinearMap<f32, 64>,
    source_ema_slow: LinearMap<f32, 64>,
    rng_state: u64,
    next_signal_id: u64,
    stats: PrecognitionStats,
    generation: u64,
}

impl HolisticPrecognition {
    /// Create a new holistic precognition engine
    pub fn new(seed: u64) -> Self {
        Self {
            signal_window: BTreeMap::new(),
            current_regime: OperatingRegime::Unknown,
            regime_history: Vec::new(),
            phase_history: Vec::new(),
            paradigm_alerts: Vec::new(),
            accuracy_log: Vec::new(),
            source_ema_fast: LinearMap::new(),
            source_ema_slow: LinearMap::new(),
            rng_state: seed ^ 0xC0FFEE_DEAD_BEEF_42,
            next_signal_id: 1,
            stats: PrecognitionStats::new(),
            generation: 0,
        }
    }

    /// Ingest a raw signal for precognitive processing
    pub fn ingest_signal(
        &mut self,
        source: PrecogSource,
        value: f32,
        timestamp_us: u64,
    ) -> u64 {
        let id = self.next_signal_id;
        self.next_signal_id += 1;
        let sk = fnv1a_hash(&[source as u8]);

        let prev_fast = self.source_ema_fast.get(sk).copied().unwrap_or(value);
        let prev_slow = self.source_ema_slow.get(sk).copied().unwrap_or(value);
        let new_fast = ema_update(prev_fast, value);
        let new_slow = ema_slow_update(prev_slow, value);
        let derivative = new_fast - prev_fast;

        self.source_ema_fast.insert(sk, new_fast);
        self.source_ema_slow.insert(sk, new_slow);

        let signal = PrecogSignal {
            signal_id: id,
            source,
            value,
            timestamp_us,
            ema_fast: new_fast,
            ema_slow: new_slow,
            derivative,
        };

        let window = self.signal_window.entry(sk).or_insert_with(Vec::new);
        window.push(signal);
        if window.len() > MAX_SIGNAL_WINDOW {
            window.pop_front();
        }
        id
    }

    /// Main precognitive sensing cycle: detect regime changes, phase
    /// transitions, and paradigm shifts
    #[inline]
    pub fn system_precognition(&mut self, timestamp_us: u64) -> (
        Option<RegimeChangeDetection>,
        Option<PhaseTransitionSense>,
        Option<ParadigmShiftAlert>,
    ) {
        self.stats.precog_cycles += 1;
        self.generation += 1;

        let regime = self.detect_regime(timestamp_us);
        let phase = self.sense_phase_transition(timestamp_us);
        let paradigm = self.detect_paradigm_shift(timestamp_us);

        (regime, phase, paradigm)
    }

    /// Detect regime changes in system operating mode
    #[inline(always)]
    pub fn regime_change_detection(&mut self, timestamp_us: u64) -> Option<RegimeChangeDetection> {
        self.detect_regime(timestamp_us)
    }

    /// Sense phase transitions and critical points
    #[inline(always)]
    pub fn phase_transition_sense(&mut self, timestamp_us: u64) -> Option<PhaseTransitionSense> {
        self.sense_phase_transition(timestamp_us)
    }

    /// Alert on paradigm shifts
    #[inline(always)]
    pub fn paradigm_shift_alert(&mut self, timestamp_us: u64) -> Option<ParadigmShiftAlert> {
        self.detect_paradigm_shift(timestamp_us)
    }

    /// Evaluate precognitive accuracy over time
    pub fn precognitive_accuracy(&mut self) -> PrecogAccuracyReport {
        self.stats.accuracy_evaluations += 1;
        let total = self.accuracy_log.len();
        let correct = self.accuracy_log.iter().filter(|r| r.was_correct).count();
        let accuracy = if total > 0 { correct as f32 / total as f32 } else { 0.0 };

        let avg_lead = if total > 0 {
            self.accuracy_log.iter().map(|r| r.lead_time_us).sum::<u64>() / total as u64
        } else {
            0
        };

        let regime_correct = self
            .accuracy_log
            .iter()
            .filter(|r| r.prediction_type == "regime" && r.was_correct)
            .count();
        let regime_total = self
            .accuracy_log
            .iter()
            .filter(|r| r.prediction_type == "regime")
            .count();
        let regime_acc = if regime_total > 0 {
            regime_correct as f32 / regime_total as f32
        } else {
            0.0
        };

        let phase_correct = self
            .accuracy_log
            .iter()
            .filter(|r| r.prediction_type == "phase" && r.was_correct)
            .count();
        let phase_total = self
            .accuracy_log
            .iter()
            .filter(|r| r.prediction_type == "phase")
            .count();
        let phase_acc = if phase_total > 0 {
            phase_correct as f32 / phase_total as f32
        } else {
            0.0
        };

        let paradigm_correct = self
            .accuracy_log
            .iter()
            .filter(|r| r.prediction_type == "paradigm" && r.was_correct)
            .count();
        let paradigm_total = self
            .accuracy_log
            .iter()
            .filter(|r| r.prediction_type == "paradigm")
            .count();
        let paradigm_acc = if paradigm_total > 0 {
            paradigm_correct as f32 / paradigm_total as f32
        } else {
            0.0
        };

        let recent_window = total.saturating_sub(100);
        let recent_correct = self
            .accuracy_log
            .iter()
            .skip(recent_window)
            .filter(|r| r.was_correct)
            .count();
        let recent_total = total - recent_window;
        let running = if recent_total > 0 {
            recent_correct as f32 / recent_total as f32
        } else {
            0.0
        };

        self.stats.avg_accuracy = ema_update(self.stats.avg_accuracy, accuracy);
        self.stats.avg_lead_time_us = ema_update(self.stats.avg_lead_time_us, avg_lead as f32);

        PrecogAccuracyReport {
            total_predictions: total,
            correct_predictions: correct,
            accuracy,
            average_lead_time_us: avg_lead,
            regime_detection_accuracy: regime_acc,
            phase_detection_accuracy: phase_acc,
            paradigm_detection_accuracy: paradigm_acc,
            running_accuracy: running,
        }
    }

    /// Record validation of a past precognitive prediction
    pub fn record_validation(
        &mut self,
        prediction_type: String,
        predicted_at: u64,
        validated_at: u64,
        correct: bool,
        confidence: f32,
    ) {
        if self.accuracy_log.len() < MAX_ACCURACY_LOG {
            self.accuracy_log.push(PrecogAccuracyRecord {
                prediction_type,
                predicted_at_us: predicted_at,
                validated_at_us: validated_at,
                was_correct: correct,
                lead_time_us: validated_at.saturating_sub(predicted_at),
                confidence_at_prediction: confidence,
            });
        }
    }

    /// Generate transcendent foresight: meta-analysis of precognitive ability
    pub fn transcendent_foresight(&mut self) -> TranscendentForesight {
        self.stats.foresight_reports += 1;
        let accuracy = self.stats.avg_accuracy;
        let trend = self.compute_accuracy_trend();

        let all_sources = [
            PrecogSource::Scheduler, PrecogSource::Memory, PrecogSource::Io,
            PrecogSource::Network, PrecogSource::Thermal, PrecogSource::Power,
            PrecogSource::FileSystem, PrecogSource::Ipc, PrecogSource::Security,
            PrecogSource::Driver, PrecogSource::Userspace, PrecogSource::SystemWide,
        ];

        let mut source_quality: LinearMap<f32, 64> = BTreeMap::new();
        let mut blind_spots: Vec<PrecogSource> = Vec::new();
        let mut strongest: Vec<(PrecogSource, f32)> = Vec::new();

        for &src in &all_sources {
            let sk = fnv1a_hash(&[src as u8]);
            let window = self.signal_window.get(&sk);
            let count = window.map(|w| w.len()).unwrap_or(0);
            let quality = if count > CORRELATION_WINDOW {
                let variance = self.compute_signal_variance(sk);
                (1.0 - variance.min(1.0)).max(0.0)
            } else if count > 0 {
                0.3
            } else {
                0.0
            };
            source_quality.insert(sk, quality);
            if quality < 0.1 {
                blind_spots.push(src);
            } else {
                strongest.push((src, quality));
            }
        }

        strongest.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        let top_sources: Vec<PrecogSource> = strongest.iter().take(3).map(|(s, _)| *s).collect();

        let signal_quality = source_quality.values().sum::<f32>()
            / source_quality.len().max(1) as f32;
        let model_coherence = self.compute_model_coherence();
        let overall = accuracy * 0.4 + signal_quality * 0.3 + model_coherence * 0.3;

        let recommendation = if overall > 0.8 {
            ForesightRecommendation::MaintainCourse
        } else if signal_quality < 0.3 {
            ForesightRecommendation::ExpandSignalSources
        } else if model_coherence < 0.3 {
            ForesightRecommendation::RecalibrateModels
        } else if blind_spots.len() > 4 {
            ForesightRecommendation::IncreaseMonitoring
        } else if accuracy < 0.5 {
            ForesightRecommendation::ReduceNoise
        } else {
            ForesightRecommendation::FocusDomain
        };

        self.stats.avg_foresight_score = ema_update(self.stats.avg_foresight_score, overall);

        TranscendentForesight {
            current_capability: accuracy,
            capability_trend: trend,
            signal_quality,
            model_coherence,
            blind_spots,
            strongest_domains: top_sources,
            overall_foresight_score: overall,
            recommendation,
        }
    }

    /// Get current statistics
    #[inline(always)]
    pub fn stats(&self) -> &PrecognitionStats {
        &self.stats
    }

    // ========================================================================
    // PRIVATE HELPERS
    // ========================================================================

    fn detect_regime(&mut self, timestamp_us: u64) -> Option<RegimeChangeDetection> {
        let mut fast_sum = 0.0_f32;
        let mut slow_sum = 0.0_f32;
        let mut source_count = 0_usize;
        let mut contributing: Vec<PrecogSource> = Vec::new();

        let all_sources = [
            PrecogSource::Scheduler, PrecogSource::Memory, PrecogSource::Io,
            PrecogSource::Network, PrecogSource::Thermal, PrecogSource::Power,
        ];

        for &src in &all_sources {
            let sk = fnv1a_hash(&[src as u8]);
            let fast = self.source_ema_fast.get(sk).copied().unwrap_or(0.0);
            let slow = self.source_ema_slow.get(sk).copied().unwrap_or(0.0);
            fast_sum += fast;
            slow_sum += slow;
            source_count += 1;
            if (fast - slow).abs() > REGIME_SENSITIVITY {
                contributing.push(src);
            }
        }

        if source_count == 0 || contributing.is_empty() {
            return None;
        }

        let avg_fast = fast_sum / source_count as f32;
        let avg_slow = slow_sum / source_count as f32;
        let sharpness = (avg_fast - avg_slow).abs();

        if sharpness < REGIME_SENSITIVITY {
            return None;
        }

        let new_regime = self.classify_regime(avg_fast);
        if new_regime == self.current_regime {
            return None;
        }

        let prev = self.current_regime;
        self.current_regime = new_regime;
        self.stats.regime_changes_detected += 1;

        let confidence = (sharpness / 0.5).min(0.95);
        self.stats.avg_confidence = ema_update(self.stats.avg_confidence, confidence);

        let detection = RegimeChangeDetection {
            previous_regime: prev,
            detected_regime: new_regime,
            confidence,
            transition_sharpness: sharpness,
            signals_contributing: contributing,
            timestamp_us,
            duration_estimate_us: (1_000_000.0 / sharpness.max(0.01)) as u64,
            reversion_probability: (1.0 - sharpness).max(0.05),
        };

        if self.regime_history.len() < MAX_REGIME_HISTORY {
            self.regime_history.push(detection.clone());
        }
        Some(detection)
    }

    fn sense_phase_transition(&mut self, timestamp_us: u64) -> Option<PhaseTransitionSense> {
        let mut max_susceptibility = 0.0_f32;
        let mut involved: Vec<PrecogSource> = Vec::new();

        let all_sources = [
            PrecogSource::Scheduler, PrecogSource::Memory, PrecogSource::Io,
            PrecogSource::Network, PrecogSource::Thermal, PrecogSource::Power,
        ];

        for &src in &all_sources {
            let sk = fnv1a_hash(&[src as u8]);
            let variance = self.compute_signal_variance(sk);
            if variance > PHASE_CRITICAL_EXPONENT {
                max_susceptibility = max_susceptibility.max(variance);
                involved.push(src);
            }
        }

        if involved.is_empty() {
            return None;
        }

        self.stats.phase_transitions_sensed += 1;
        let order_param = max_susceptibility;
        let distance = (1.0 - order_param).max(0.0);
        let corr_length = order_param * CORRELATION_WINDOW as f32;

        let kind = if order_param > 0.9 {
            PhaseTransitionKind::Catastrophic
        } else if order_param > 0.7 {
            PhaseTransitionKind::FirstOrder
        } else if order_param > 0.6 {
            PhaseTransitionKind::CriticalPoint
        } else if order_param > 0.5 {
            PhaseTransitionKind::SecondOrder
        } else {
            PhaseTransitionKind::Gradual
        };

        let confidence = (order_param * 0.8).min(0.95);
        let sense = PhaseTransitionSense {
            transition_kind: kind,
            order_parameter: order_param,
            critical_exponent: PHASE_CRITICAL_EXPONENT,
            distance_to_critical: distance,
            susceptibility: max_susceptibility,
            correlation_length: corr_length,
            confidence,
            sources_involved: involved,
            timestamp_us,
        };

        if self.phase_history.len() < MAX_PHASE_DETECTORS {
            self.phase_history.push(sense.clone());
        }
        Some(sense)
    }

    fn detect_paradigm_shift(&mut self, timestamp_us: u64) -> Option<ParadigmShiftAlert> {
        let mut divergence_sum = 0.0_f32;
        let mut source_count = 0_usize;

        for (sk, _window) in &self.signal_window {
            let fast = self.source_ema_fast.get(sk).copied().unwrap_or(0.0);
            let slow = self.source_ema_slow.get(sk).copied().unwrap_or(0.0);
            let div = (fast - slow).abs();
            divergence_sum += div;
            source_count += 1;
        }

        if source_count == 0 {
            return None;
        }

        let avg_divergence = divergence_sum / source_count as f32;
        if avg_divergence < PARADIGM_THRESHOLD {
            return None;
        }

        self.stats.paradigm_shifts_alerted += 1;

        let kind = if avg_divergence > 0.9 {
            ParadigmShiftKind::PerformanceModelBreak
        } else if avg_divergence > 0.85 {
            ParadigmShiftKind::WorkloadClassChange
        } else if avg_divergence > 0.80 {
            ParadigmShiftKind::ResourceTopologyChange
        } else {
            ParadigmShiftKind::ScalingRegimeChange
        };

        let alert = ParadigmShiftAlert {
            shift_kind: kind,
            severity: avg_divergence,
            confidence: (avg_divergence * 0.9).min(0.95),
            affected_models: Vec::new(),
            recommended_recalibration: Vec::new(),
            timestamp_us,
            description: String::from("paradigm-shift-detected"),
        };

        if self.paradigm_alerts.len() < MAX_PARADIGM_ALERTS {
            self.paradigm_alerts.push(alert.clone());
        }
        Some(alert)
    }

    fn classify_regime(&self, avg_metric: f32) -> OperatingRegime {
        if avg_metric < 0.05 {
            OperatingRegime::Idle
        } else if avg_metric < 0.20 {
            OperatingRegime::LightLoad
        } else if avg_metric < 0.50 {
            OperatingRegime::SteadyState
        } else if avg_metric < 0.75 {
            OperatingRegime::HighThroughput
        } else if avg_metric < 0.90 {
            OperatingRegime::Overloaded
        } else {
            OperatingRegime::CriticalState
        }
    }

    fn compute_signal_variance(&self, source_key: u64) -> f32 {
        let window = match self.signal_window.get(&source_key) {
            Some(w) if w.len() > 1 => w,
            _ => return 0.0,
        };
        let n = window.len() as f32;
        let mean = window.iter().map(|s| s.value).sum::<f32>() / n;
        let variance = window.iter().map(|s| (s.value - mean) * (s.value - mean)).sum::<f32>() / n;
        variance
    }

    fn compute_accuracy_trend(&self) -> f32 {
        if self.accuracy_log.len() < 10 {
            return 0.0;
        }
        let half = self.accuracy_log.len() / 2;
        let first_half = self.accuracy_log[..half]
            .iter()
            .filter(|r| r.was_correct)
            .count() as f32
            / half as f32;
        let second_half = self.accuracy_log[half..]
            .iter()
            .filter(|r| r.was_correct)
            .count() as f32
            / (self.accuracy_log.len() - half) as f32;
        second_half - first_half
    }

    fn compute_model_coherence(&self) -> f32 {
        let fast_values: Vec<f32> = self.source_ema_fast.values().copied().collect();
        if fast_values.len() < 2 {
            return 1.0;
        }
        let mean = fast_values.iter().sum::<f32>() / fast_values.len() as f32;
        let variance = fast_values
            .iter()
            .map(|v| (v - mean) * (v - mean))
            .sum::<f32>()
            / fast_values.len() as f32;
        (1.0 - variance.sqrt().min(1.0)).max(0.0)
    }
}
