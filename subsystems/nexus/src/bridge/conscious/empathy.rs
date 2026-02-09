// SPDX-License-Identifier: GPL-2.0
//! # Bridge Empathy Engine
//!
//! Understanding other subsystems' states through the bridge lens. The bridge
//! "empathizes" with each major kernel subsystem — inferring their internal
//! state from observable signals:
//!
//! - **Scheduler** — is it overloaded? Are run queues growing?
//! - **Memory** — is it under pressure? Reclaim activity spiking?
//! - **I/O** — is it bottlenecked? Queue depths growing?
//!
//! Empathy models allow the bridge to anticipate subsystem needs and adapt
//! its own behaviour proactively. Empathy accuracy is tracked by comparing
//! inferred states against actual feedback.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_SUBSYSTEMS: usize = 32;
const MAX_EMPATHY_HISTORY: usize = 256;
const MAX_SIGNALS_PER_SUBSYSTEM: usize = 64;
const EMA_ALPHA: f32 = 0.10;
const STRESS_HIGH_THRESHOLD: f32 = 0.70;
const STRESS_CRITICAL_THRESHOLD: f32 = 0.90;
const EMPATHY_ACCURACY_DECAY: f32 = 0.02;
const CROSS_INSIGHT_THRESHOLD: f32 = 0.50;
const CONFIDENCE_INITIAL: f32 = 0.30;
const CONFIDENCE_CORRECT_BOOST: f32 = 0.08;
const CONFIDENCE_WRONG_PENALTY: f32 = 0.12;
const STALE_THRESHOLD_TICKS: u64 = 100;
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

fn ema_update(current: f32, sample: f32, alpha: f32) -> f32 {
    current * (1.0 - alpha) + sample * alpha
}

// ============================================================================
// SUBSYSTEM STATE
// ============================================================================

/// Inferred state of a subsystem
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SubsystemState {
    /// Operating normally, low pressure
    Healthy,
    /// Moderate load, coping but strained
    Strained,
    /// High load, performance degradation likely
    Stressed,
    /// Near failure, immediate intervention needed
    Critical,
    /// Unknown — insufficient data to infer
    Unknown,
}

impl SubsystemState {
    fn from_stress(stress: f32) -> Self {
        if stress >= STRESS_CRITICAL_THRESHOLD {
            SubsystemState::Critical
        } else if stress >= STRESS_HIGH_THRESHOLD {
            SubsystemState::Stressed
        } else if stress >= 0.40 {
            SubsystemState::Strained
        } else if stress >= 0.0 {
            SubsystemState::Healthy
        } else {
            SubsystemState::Unknown
        }
    }

    fn severity(&self) -> f32 {
        match self {
            SubsystemState::Healthy => 0.0,
            SubsystemState::Strained => 0.3,
            SubsystemState::Stressed => 0.7,
            SubsystemState::Critical => 1.0,
            SubsystemState::Unknown => 0.5,
        }
    }
}

// ============================================================================
// SUBSYSTEM SIGNAL
// ============================================================================

/// An observable signal from a subsystem
#[derive(Debug, Clone)]
pub struct SubsystemSignal {
    pub signal_name: String,
    pub value: f32,
    pub tick: u64,
}

// ============================================================================
// EMPATHY MODEL
// ============================================================================

/// Empathy model for a single subsystem
#[derive(Debug, Clone)]
pub struct EmpathyModel {
    pub target_subsystem: String,
    pub subsystem_hash: u64,
    pub inferred_state: SubsystemState,
    pub confidence: f32,
    pub last_update: u64,
    pub stress_ema: f32,
    pub signal_history: Vec<SubsystemSignal>,
    pub predictions_made: u64,
    pub predictions_correct: u64,
    pub consecutive_correct: u32,
    pub total_signals_received: u64,
}

impl EmpathyModel {
    fn new(subsystem: &str, tick: u64) -> Self {
        Self {
            target_subsystem: String::from(subsystem),
            subsystem_hash: fnv1a_hash(subsystem.as_bytes()),
            inferred_state: SubsystemState::Unknown,
            confidence: CONFIDENCE_INITIAL,
            last_update: tick,
            stress_ema: 0.0,
            signal_history: Vec::new(),
            predictions_made: 0,
            predictions_correct: 0,
            consecutive_correct: 0,
            total_signals_received: 0,
        }
    }

    fn record_signal(&mut self, signal: SubsystemSignal) {
        self.total_signals_received += 1;
        self.stress_ema = ema_update(self.stress_ema, signal.value, EMA_ALPHA);
        self.last_update = signal.tick;

        if self.signal_history.len() >= MAX_SIGNALS_PER_SUBSYSTEM {
            self.signal_history.remove(0);
        }
        self.signal_history.push(signal);

        self.inferred_state = SubsystemState::from_stress(self.stress_ema);
    }

    fn verify_prediction(&mut self, actual_state: SubsystemState) {
        self.predictions_made += 1;
        if self.inferred_state == actual_state {
            self.predictions_correct += 1;
            self.consecutive_correct += 1;
            self.confidence =
                (self.confidence + CONFIDENCE_CORRECT_BOOST).clamp(0.0, 1.0);
        } else {
            self.consecutive_correct = 0;
            self.confidence =
                (self.confidence - CONFIDENCE_WRONG_PENALTY).clamp(0.0, 1.0);
        }
    }

    fn accuracy(&self) -> f32 {
        if self.predictions_made == 0 {
            return 0.0;
        }
        self.predictions_correct as f32 / self.predictions_made as f32
    }

    fn is_stale(&self, current_tick: u64) -> bool {
        current_tick.saturating_sub(self.last_update) > STALE_THRESHOLD_TICKS
    }

    fn stress_trend(&self) -> f32 {
        let len = self.signal_history.len();
        if len < 4 {
            return 0.0;
        }
        let half = len / 2;
        let first_half_avg: f32 =
            self.signal_history[..half].iter().map(|s| s.value).sum::<f32>() / half as f32;
        let second_half_avg: f32 =
            self.signal_history[half..].iter().map(|s| s.value).sum::<f32>()
                / (len - half) as f32;
        second_half_avg - first_half_avg
    }
}

// ============================================================================
// EMPATHY HISTORY ENTRY
// ============================================================================

#[derive(Debug, Clone)]
struct EmpathyHistoryEntry {
    subsystem_hash: u64,
    inferred_state: SubsystemState,
    confidence: f32,
    tick: u64,
}

// ============================================================================
// CROSS-SUBSYSTEM INSIGHT
// ============================================================================

/// An insight from observing correlations between subsystem states
#[derive(Debug, Clone)]
pub struct CrossSubsystemInsight {
    pub subsystem_a: String,
    pub subsystem_b: String,
    pub correlation: f32,
    pub description: String,
    pub discovered_tick: u64,
}

// ============================================================================
// STATS
// ============================================================================

/// Empathy engine statistics
#[derive(Debug, Clone)]
pub struct EmpathyStats {
    pub tracked_subsystems: usize,
    pub total_signals: u64,
    pub total_predictions: u64,
    pub overall_accuracy: f32,
    pub avg_confidence: f32,
    pub cross_insights_found: usize,
    pub stale_models: usize,
}

// ============================================================================
// BRIDGE EMPATHY ENGINE
// ============================================================================

/// Engine for inferring and tracking other subsystems' internal states
#[derive(Debug, Clone)]
pub struct BridgeEmpathyEngine {
    models: BTreeMap<u64, EmpathyModel>,
    history: Vec<EmpathyHistoryEntry>,
    cross_insights: Vec<CrossSubsystemInsight>,
    current_tick: u64,
    total_signals: u64,
    total_predictions: u64,
    total_correct: u64,
    overall_accuracy_ema: f32,
}

impl BridgeEmpathyEngine {
    /// Create a new empathy engine
    pub fn new() -> Self {
        Self {
            models: BTreeMap::new(),
            history: Vec::new(),
            cross_insights: Vec::new(),
            current_tick: 0,
            total_signals: 0,
            total_predictions: 0,
            total_correct: 0,
            overall_accuracy_ema: 0.0,
        }
    }

    /// Empathize with a subsystem — feed an observable signal
    pub fn empathize_with(&mut self, subsystem: &str, signal_name: &str, value: f32) {
        self.current_tick += 1;
        self.total_signals += 1;

        let hash = fnv1a_hash(subsystem.as_bytes());

        if !self.models.contains_key(&hash) && self.models.len() < MAX_SUBSYSTEMS {
            let model = EmpathyModel::new(subsystem, self.current_tick);
            self.models.insert(hash, model);
        }

        if let Some(model) = self.models.get_mut(&hash) {
            let signal = SubsystemSignal {
                signal_name: String::from(signal_name),
                value: value.clamp(0.0, 1.0),
                tick: self.current_tick,
            };
            model.record_signal(signal);

            // Record history
            if self.history.len() >= MAX_EMPATHY_HISTORY {
                self.history.remove(0);
            }
            self.history.push(EmpathyHistoryEntry {
                subsystem_hash: hash,
                inferred_state: model.inferred_state,
                confidence: model.confidence,
                tick: self.current_tick,
            });
        }
    }

    /// Infer a subsystem's current state
    pub fn infer_subsystem_state(&self, subsystem: &str) -> (SubsystemState, f32) {
        let hash = fnv1a_hash(subsystem.as_bytes());
        match self.models.get(&hash) {
            Some(model) => (model.inferred_state, model.confidence),
            None => (SubsystemState::Unknown, 0.0),
        }
    }

    /// Build a stress map of all tracked subsystems
    pub fn subsystem_stress_map(&self) -> BTreeMap<u64, (String, f32, SubsystemState)> {
        let mut map = BTreeMap::new();
        for (&hash, model) in &self.models {
            map.insert(
                hash,
                (
                    model.target_subsystem.clone(),
                    model.stress_ema,
                    model.inferred_state,
                ),
            );
        }
        map
    }

    /// Verify empathy accuracy by providing actual subsystem state
    pub fn empathy_accuracy(&mut self, subsystem: &str, actual_state: SubsystemState) -> f32 {
        self.current_tick += 1;
        self.total_predictions += 1;

        let hash = fnv1a_hash(subsystem.as_bytes());
        if let Some(model) = self.models.get_mut(&hash) {
            let was_correct = model.inferred_state == actual_state;
            model.verify_prediction(actual_state);
            if was_correct {
                self.total_correct += 1;
            }
            let accuracy = model.accuracy();
            self.overall_accuracy_ema = ema_update(self.overall_accuracy_ema, accuracy, EMA_ALPHA);
            accuracy
        } else {
            0.0
        }
    }

    /// Look for cross-subsystem correlations
    pub fn cross_subsystem_insight(&mut self) -> Vec<CrossSubsystemInsight> {
        let mut insights = Vec::new();
        let hashes: Vec<u64> = self.models.keys().copied().collect();

        for i in 0..hashes.len() {
            for j in (i + 1)..hashes.len() {
                let model_a = &self.models[&hashes[i]];
                let model_b = &self.models[&hashes[j]];

                // Compute correlation via stress trend similarity
                let trend_a = model_a.stress_trend();
                let trend_b = model_b.stress_trend();

                let correlation = if (trend_a.abs() > 0.01) && (trend_b.abs() > 0.01) {
                    // Same direction and similar magnitude → correlated
                    let direction_match = if trend_a.signum() == trend_b.signum() {
                        1.0
                    } else {
                        -1.0
                    };
                    let magnitude_sim =
                        1.0 - (trend_a.abs() - trend_b.abs()).abs() / (trend_a.abs() + trend_b.abs());
                    direction_match * magnitude_sim
                } else {
                    0.0
                };

                if correlation.abs() > CROSS_INSIGHT_THRESHOLD {
                    let mut desc = String::from("correlation_between_");
                    desc.push_str(&model_a.target_subsystem);
                    desc.push('_');
                    desc.push_str(&model_b.target_subsystem);

                    let insight = CrossSubsystemInsight {
                        subsystem_a: model_a.target_subsystem.clone(),
                        subsystem_b: model_b.target_subsystem.clone(),
                        correlation,
                        description: desc,
                        discovered_tick: self.current_tick,
                    };
                    insights.push(insight.clone());
                    self.cross_insights.push(insight);
                }
            }
        }

        insights
    }

    /// Overall empathy score across all subsystems
    pub fn empathy_score(&self) -> f32 {
        if self.models.is_empty() {
            return 0.0;
        }
        let total_conf: f32 = self.models.values().map(|m| m.confidence).sum();
        let total_accuracy: f32 = self.models.values().map(|m| m.accuracy()).sum();
        let count = self.models.len() as f32;
        (total_conf / count * 0.5 + total_accuracy / count * 0.5).clamp(0.0, 1.0)
    }

    /// Get the most stressed subsystem
    pub fn most_stressed_subsystem(&self) -> Option<(String, f32)> {
        self.models
            .values()
            .max_by(|a, b| {
                a.stress_ema
                    .partial_cmp(&b.stress_ema)
                    .unwrap_or(core::cmp::Ordering::Equal)
            })
            .map(|m| (m.target_subsystem.clone(), m.stress_ema))
    }

    /// Count stale models that need refreshing
    pub fn stale_model_count(&self) -> usize {
        self.models
            .values()
            .filter(|m| m.is_stale(self.current_tick))
            .count()
    }

    /// Statistics snapshot
    pub fn stats(&self) -> EmpathyStats {
        let avg_conf = if self.models.is_empty() {
            0.0
        } else {
            let total: f32 = self.models.values().map(|m| m.confidence).sum();
            total / self.models.len() as f32
        };

        EmpathyStats {
            tracked_subsystems: self.models.len(),
            total_signals: self.total_signals,
            total_predictions: self.total_predictions,
            overall_accuracy: self.overall_accuracy_ema,
            avg_confidence: avg_conf,
            cross_insights_found: self.cross_insights.len(),
            stale_models: self.stale_model_count(),
        }
    }

    /// Reset all empathy models
    pub fn reset(&mut self) {
        self.models.clear();
        self.history.clear();
        self.cross_insights.clear();
        self.overall_accuracy_ema = 0.0;
    }
}
