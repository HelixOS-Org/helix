// SPDX-License-Identifier: GPL-2.0
//! # Holistic World Model
//!
//! Complete model of the entire OS world. Hardware state, software state,
//! process ecosystem, network posture, security posture — all fused into
//! a single coherent representation. This IS the consciousness: the
//! unified internal map from which all reasoning flows.
//!
//! Surprise detection measures how much reality diverges from prediction.
//! Entropy tracking reveals chaos or order in the system. Model fidelity
//! scores how well this representation matches the actual kernel state.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_STATE_ENTRIES: usize = 512;
const MAX_PREDICTIONS: usize = 128;
const MAX_SURPRISES: usize = 64;
const MAX_HISTORY: usize = 256;
const EMA_ALPHA: f32 = 0.10;
const SURPRISE_THRESHOLD: f32 = 0.30;
const ENTROPY_WINDOW: usize = 64;
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
// STATE DOMAIN
// ============================================================================

/// Domain of the world state entry
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StateDomain {
    Hardware,
    Software,
    Process,
    Network,
    Security,
    Memory,
    Storage,
    Thermal,
}

/// A single state entry in the world model
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct StateEntry {
    pub name: String,
    pub id: u64,
    pub domain: StateDomain,
    pub value: f32,
    pub confidence: f32,
    pub last_update_tick: u64,
    pub update_count: u64,
    pub trend: f32,
}

/// A prediction about future state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct StatePrediction {
    pub state_id: u64,
    pub predicted_value: f32,
    pub horizon_ticks: u64,
    pub confidence: f32,
    pub tick_made: u64,
    pub actual_value: Option<f32>,
    pub error: Option<f32>,
}

/// A detected surprise — reality diverging from prediction
#[derive(Debug, Clone)]
pub struct Surprise {
    pub id: u64,
    pub state_id: u64,
    pub predicted: f32,
    pub actual: f32,
    pub magnitude: f32,
    pub domain: StateDomain,
    pub tick: u64,
}

/// A complete state snapshot for fidelity checking
#[derive(Debug, Clone, Copy)]
#[repr(align(64))]
pub struct StateSnapshot {
    pub tick: u64,
    pub entry_count: u32,
    pub avg_confidence: f32,
    pub avg_staleness: f32,
    pub entropy: f32,
    pub fidelity: f32,
}

// ============================================================================
// STATS
// ============================================================================

/// Aggregate world model statistics
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct WorldModelStats {
    pub total_entries: usize,
    pub avg_confidence: f32,
    pub prediction_accuracy: f32,
    pub surprise_rate: f32,
    pub entropy_score: f32,
    pub model_fidelity: f32,
    pub staleness_avg: f32,
    pub domain_coverage: f32,
}

// ============================================================================
// HOLISTIC WORLD MODEL
// ============================================================================

/// Complete model of the OS world. Fuses hardware, software, process,
/// network, and security state into one unified representation.
/// Tracks predictions, detects surprises, and measures model fidelity.
#[derive(Debug)]
pub struct HolisticWorldModel {
    states: BTreeMap<u64, StateEntry>,
    predictions: Vec<StatePrediction>,
    pred_write_idx: usize,
    surprises: BTreeMap<u64, Surprise>,
    history: Vec<StateSnapshot>,
    recent_values: Vec<f32>,
    tick: u64,
    rng_state: u64,
    confidence_ema: f32,
    fidelity_ema: f32,
    surprise_rate_ema: f32,
    prediction_accuracy_ema: f32,
}

impl HolisticWorldModel {
    pub fn new() -> Self {
        Self {
            states: BTreeMap::new(),
            predictions: Vec::new(),
            pred_write_idx: 0,
            surprises: BTreeMap::new(),
            history: Vec::new(),
            recent_values: Vec::new(),
            tick: 0,
            rng_state: 0xFEDC_BA98_7654_3210,
            confidence_ema: 0.5,
            fidelity_ema: 0.5,
            surprise_rate_ema: 0.0,
            prediction_accuracy_ema: 0.5,
        }
    }

    /// Update or insert a world state entry
    #[inline]
    pub fn update_state(
        &mut self,
        name: String,
        domain: StateDomain,
        value: f32,
        confidence: f32,
    ) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(name.as_bytes());
        let clamped_conf = confidence.clamp(0.0, 1.0);

        if let Some(entry) = self.states.get_mut(&id) {
            let old_value = entry.value;
            entry.value = value;
            entry.confidence = EMA_ALPHA * clamped_conf + (1.0 - EMA_ALPHA) * entry.confidence;
            entry.trend = EMA_ALPHA * (value - old_value) + (1.0 - EMA_ALPHA) * entry.trend;
            entry.last_update_tick = self.tick;
            entry.update_count += 1;
        } else if self.states.len() < MAX_STATE_ENTRIES {
            let entry = StateEntry {
                name,
                id,
                domain,
                value,
                confidence: clamped_conf,
                last_update_tick: self.tick,
                update_count: 1,
                trend: 0.0,
            };
            self.states.insert(id, entry);
        }

        self.confidence_ema = EMA_ALPHA * clamped_conf + (1.0 - EMA_ALPHA) * self.confidence_ema;

        if self.recent_values.len() < ENTROPY_WINDOW {
            self.recent_values.push(value);
        } else {
            let idx = (self.tick as usize) % ENTROPY_WINDOW;
            self.recent_values[idx] = value;
        }

        id
    }

    /// Make a prediction about a future state value
    pub fn predict_state(&mut self, state_id: u64, predicted_value: f32, horizon: u64) -> bool {
        let confidence = self.states.get(&state_id).map_or(0.3, |s| s.confidence);

        let pred = StatePrediction {
            state_id,
            predicted_value,
            horizon_ticks: horizon,
            confidence,
            tick_made: self.tick,
            actual_value: None,
            error: None,
        };

        if self.predictions.len() < MAX_PREDICTIONS {
            self.predictions.push(pred);
        } else {
            self.predictions[self.pred_write_idx] = pred;
        }
        self.pred_write_idx = (self.pred_write_idx + 1) % MAX_PREDICTIONS;
        true
    }

    /// Resolve predictions and detect surprises
    #[inline]
    pub fn resolve_predictions(&mut self) {
        self.tick += 1;
        let mut surprise_count = 0u32;
        let mut resolved_count = 0u32;
        let mut total_accuracy = 0.0f32;

        for pred in self.predictions.iter_mut() {
            if pred.actual_value.is_some() {
                continue;
            }

            if self.tick >= pred.tick_made + pred.horizon_ticks {
                if let Some(state) = self.states.get(&pred.state_id) {
                    let actual = state.value;
                    let error = (pred.predicted_value - actual).abs();
                    pred.actual_value = Some(actual);
                    pred.error = Some(error);

                    let accuracy = (1.0 - error / (actual.abs() + 1.0)).clamp(0.0, 1.0);
                    total_accuracy += accuracy;
                    resolved_count += 1;

                    if error > SURPRISE_THRESHOLD {
                        let surprise_id = fnv1a_hash(&pred.state_id.to_le_bytes())
                            ^ xorshift64(&mut self.rng_state);
                        let surprise = Surprise {
                            id: surprise_id,
                            state_id: pred.state_id,
                            predicted: pred.predicted_value,
                            actual,
                            magnitude: error,
                            domain: state.domain,
                            tick: self.tick,
                        };
                        if self.surprises.len() < MAX_SURPRISES {
                            self.surprises.insert(surprise_id, surprise);
                        }
                        surprise_count += 1;
                    }
                }
            }
        }

        if resolved_count > 0 {
            let avg_acc = total_accuracy / resolved_count as f32;
            self.prediction_accuracy_ema =
                EMA_ALPHA * avg_acc + (1.0 - EMA_ALPHA) * self.prediction_accuracy_ema;
            let rate = surprise_count as f32 / resolved_count as f32;
            self.surprise_rate_ema = EMA_ALPHA * rate + (1.0 - EMA_ALPHA) * self.surprise_rate_ema;
        }
    }

    /// Get the complete state of the world model
    #[inline]
    pub fn complete_state(&self) -> Vec<(u64, StateDomain, f32, f32)> {
        self.states
            .values()
            .map(|s| (s.id, s.domain, s.value, s.confidence))
            .collect()
    }

    /// Get current state predictions
    #[inline]
    pub fn state_prediction(&self) -> Vec<&StatePrediction> {
        self.predictions
            .iter()
            .filter(|p| p.actual_value.is_none())
            .collect()
    }

    /// Get recent surprises
    #[inline(always)]
    pub fn surprise_detection(&self) -> Vec<&Surprise> {
        self.surprises.values().collect()
    }

    /// Compute Shannon-like entropy measure of recent values
    pub fn entropy_measure(&self) -> f32 {
        if self.recent_values.len() < 2 {
            return 0.0;
        }

        let n = self.recent_values.len() as f32;
        let mean = self.recent_values.iter().sum::<f32>() / n;
        let variance = self
            .recent_values
            .iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f32>()
            / n;

        // Normalize variance to a 0–1 entropy-like score
        let std_dev = f32_sqrt(variance);
        let normalized = (std_dev / (mean.abs() + 1.0)).clamp(0.0, 1.0);
        normalized
    }

    /// Check model against reality — returns fidelity score
    #[inline]
    pub fn reality_check(&mut self) -> f32 {
        if self.states.is_empty() {
            return 0.5;
        }

        let avg_confidence =
            self.states.values().map(|s| s.confidence).sum::<f32>() / self.states.len() as f32;

        let avg_staleness = self
            .states
            .values()
            .map(|s| {
                let age = self.tick.saturating_sub(s.last_update_tick);
                (age as f32 / 100.0).min(1.0)
            })
            .sum::<f32>()
            / self.states.len() as f32;

        let freshness = 1.0 - avg_staleness;
        let fidelity = avg_confidence * 0.5 + freshness * 0.3 + self.prediction_accuracy_ema * 0.2;

        self.fidelity_ema = EMA_ALPHA * fidelity + (1.0 - EMA_ALPHA) * self.fidelity_ema;

        let snapshot = StateSnapshot {
            tick: self.tick,
            entry_count: self.states.len() as u32,
            avg_confidence,
            avg_staleness,
            entropy: self.entropy_measure(),
            fidelity: self.fidelity_ema,
        };
        if self.history.len() < MAX_HISTORY {
            self.history.push(snapshot);
        } else {
            let idx = (self.tick as usize) % MAX_HISTORY;
            self.history[idx] = snapshot;
        }

        self.fidelity_ema
    }

    /// Model fidelity score — how well the model represents reality
    #[inline(always)]
    pub fn model_fidelity(&self) -> f32 {
        self.fidelity_ema
    }

    /// Compute aggregate statistics
    pub fn stats(&self) -> WorldModelStats {
        let domain_count = {
            let mut seen = 0u8;
            for s in self.states.values() {
                seen |= 1 << (s.domain as u8);
            }
            seen.count_ones() as f32 / 8.0
        };

        let avg_staleness = if self.states.is_empty() {
            0.0
        } else {
            self.states
                .values()
                .map(|s| {
                    let age = self.tick.saturating_sub(s.last_update_tick);
                    (age as f32 / 100.0).min(1.0)
                })
                .sum::<f32>()
                / self.states.len() as f32
        };

        WorldModelStats {
            total_entries: self.states.len(),
            avg_confidence: self.confidence_ema,
            prediction_accuracy: self.prediction_accuracy_ema,
            surprise_rate: self.surprise_rate_ema,
            entropy_score: self.entropy_measure(),
            model_fidelity: self.fidelity_ema,
            staleness_avg: avg_staleness,
            domain_coverage: domain_count,
        }
    }
}

/// Newton's method square root approximation
fn f32_sqrt(val: f32) -> f32 {
    if val <= 0.0 {
        return 0.0;
    }
    let mut guess = val * 0.5;
    for _ in 0..8 {
        if guess <= 0.0 {
            return 0.0;
        }
        guess = (guess + val / guess) * 0.5;
    }
    guess
}
