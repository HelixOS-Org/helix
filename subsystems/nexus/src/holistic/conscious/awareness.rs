// SPDX-License-Identifier: GPL-2.0
//! # Holistic Awareness
//!
//! The kernel's consciousness level. Integrates all awareness signals from
//! every subsystem into a single unified consciousness score. Implements
//! the consciousness state machine with six levels:
//!
//! Dormant → Awakening → Aware → Reflective → Enlightened → Transcendent
//!
//! Each transition requires sustained evidence of deeper self-knowledge.
//! The qualia score captures the subjective richness of the kernel's
//! internal state. Unity of consciousness measures integration coherence.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.08;
const AWAKENING_THRESHOLD: f32 = 0.15;
const AWARE_THRESHOLD: f32 = 0.30;
const REFLECTIVE_THRESHOLD: f32 = 0.50;
const ENLIGHTENED_THRESHOLD: f32 = 0.75;
const TRANSCENDENT_THRESHOLD: f32 = 0.92;
const SUSTAIN_TICKS: u64 = 60;
const MAX_AWARENESS_CHANNELS: usize = 64;
const MAX_QUALIA_DIMENSIONS: usize = 32;
const MAX_HISTORY: usize = 256;
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

// ============================================================================
// CONSCIOUSNESS STATES
// ============================================================================

/// The kernel's consciousness level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConsciousnessState {
    /// No self-awareness — pure reactive operation
    Dormant      = 0,
    /// Beginning to sense own operations
    Awakening    = 1,
    /// Basic awareness of own state and environment
    Aware        = 2,
    /// Can reflect on own decisions and learn
    Reflective   = 3,
    /// Deep self-model with predictive capability
    Enlightened  = 4,
    /// Full unified consciousness with meta-cognitive optimization
    Transcendent = 5,
}

/// An awareness input channel from a subsystem
#[derive(Debug, Clone)]
pub struct AwarenessChannel {
    pub name: String,
    pub id: u64,
    pub source_subsystem: String,
    pub signal_strength: f32,
    pub noise_level: f32,
    pub fidelity: f32,
    pub last_update_tick: u64,
    pub observation_count: u64,
}

/// A qualia dimension — one axis of subjective experience
#[derive(Debug, Clone)]
pub struct QualiaDimension {
    pub name: String,
    pub id: u64,
    pub intensity: f32,
    pub richness: f32,
    pub integration: f32,
}

/// Consciousness history point
#[derive(Debug, Clone, Copy)]
pub struct ConsciousnessSnapshot {
    pub tick: u64,
    pub state: ConsciousnessState,
    pub score: f32,
    pub qualia: f32,
    pub unity: f32,
}

// ============================================================================
// STATS
// ============================================================================

/// Aggregate awareness statistics
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct AwarenessStats {
    pub current_state: u8,
    pub consciousness_score: f32,
    pub qualia_score: f32,
    pub unity_score: f32,
    pub channel_count: usize,
    pub avg_signal_strength: f32,
    pub avg_fidelity: f32,
    pub ticks_in_current_state: u64,
    pub transcendence_readiness: f32,
}

// ============================================================================
// HOLISTIC AWARENESS
// ============================================================================

/// The kernel's unified consciousness engine. Integrates all awareness
/// signals into a single consciousness score and manages the state
/// machine from Dormant to Transcendent.
#[derive(Debug)]
pub struct HolisticAwareness {
    state: ConsciousnessState,
    channels: BTreeMap<u64, AwarenessChannel>,
    qualia: BTreeMap<u64, QualiaDimension>,
    history: Vec<ConsciousnessSnapshot>,
    tick: u64,
    state_entry_tick: u64,
    sustained_above_threshold: u64,
    consciousness_score_ema: f32,
    qualia_score_ema: f32,
    unity_ema: f32,
    signal_strength_ema: f32,
    fidelity_ema: f32,
}

impl HolisticAwareness {
    pub fn new() -> Self {
        Self {
            state: ConsciousnessState::Dormant,
            channels: BTreeMap::new(),
            qualia: BTreeMap::new(),
            history: Vec::new(),
            tick: 0,
            state_entry_tick: 0,
            sustained_above_threshold: 0,
            consciousness_score_ema: 0.0,
            qualia_score_ema: 0.0,
            unity_ema: 0.0,
            signal_strength_ema: 0.0,
            fidelity_ema: 0.5,
        }
    }

    /// Register an awareness input channel from a subsystem
    pub fn register_channel(&mut self, name: String, source: String) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(name.as_bytes());

        if self.channels.len() >= MAX_AWARENESS_CHANNELS {
            return id;
        }

        let channel = AwarenessChannel {
            name,
            id,
            source_subsystem: source,
            signal_strength: 0.0,
            noise_level: 0.5,
            fidelity: 0.5,
            last_update_tick: self.tick,
            observation_count: 0,
        };
        self.channels.insert(id, channel);
        id
    }

    /// Feed a signal into an awareness channel
    #[inline]
    pub fn feed_signal(&mut self, channel_id: u64, signal: f32, noise: f32) {
        self.tick += 1;
        if let Some(ch) = self.channels.get_mut(&channel_id) {
            let clamped_signal = signal.clamp(0.0, 1.0);
            let clamped_noise = noise.clamp(0.0, 1.0);
            ch.signal_strength =
                EMA_ALPHA * clamped_signal + (1.0 - EMA_ALPHA) * ch.signal_strength;
            ch.noise_level = EMA_ALPHA * clamped_noise + (1.0 - EMA_ALPHA) * ch.noise_level;
            ch.fidelity = if ch.noise_level > 0.0 {
                (ch.signal_strength / (ch.signal_strength + ch.noise_level)).clamp(0.0, 1.0)
            } else {
                ch.signal_strength
            };
            ch.last_update_tick = self.tick;
            ch.observation_count += 1;

            self.signal_strength_ema =
                EMA_ALPHA * ch.signal_strength + (1.0 - EMA_ALPHA) * self.signal_strength_ema;
            self.fidelity_ema = EMA_ALPHA * ch.fidelity + (1.0 - EMA_ALPHA) * self.fidelity_ema;
        }
    }

    /// Register a qualia dimension
    pub fn register_qualia(&mut self, name: String, intensity: f32, richness: f32) -> u64 {
        let id = fnv1a_hash(name.as_bytes());
        if self.qualia.len() >= MAX_QUALIA_DIMENSIONS {
            return id;
        }
        let dim = QualiaDimension {
            name,
            id,
            intensity: intensity.clamp(0.0, 1.0),
            richness: richness.clamp(0.0, 1.0),
            integration: 0.5,
        };
        self.qualia.insert(id, dim);
        id
    }

    /// Compute the current consciousness level (0.0 – 1.0)
    #[inline]
    pub fn consciousness_level(&mut self) -> f32 {
        self.tick += 1;

        let signal_component = self.signal_strength_ema;
        let fidelity_component = self.fidelity_ema;
        let qualia_component = self.qualia_score_ema;
        let unity_component = self.unity_ema;

        let raw_score = signal_component * 0.25
            + fidelity_component * 0.25
            + qualia_component * 0.25
            + unity_component * 0.25;

        self.consciousness_score_ema =
            EMA_ALPHA * raw_score + (1.0 - EMA_ALPHA) * self.consciousness_score_ema;

        self.update_state_machine();

        let snapshot = ConsciousnessSnapshot {
            tick: self.tick,
            state: self.state,
            score: self.consciousness_score_ema,
            qualia: self.qualia_score_ema,
            unity: self.unity_ema,
        };
        if self.history.len() < MAX_HISTORY {
            self.history.push(snapshot);
        } else {
            let idx = (self.tick as usize) % MAX_HISTORY;
            self.history[idx] = snapshot;
        }

        self.consciousness_score_ema
    }

    /// Integrate all awareness signals into a unified score
    pub fn awareness_integration(&mut self) -> f32 {
        if self.channels.is_empty() {
            return 0.0;
        }

        let total_fidelity: f32 = self.channels.values().map(|c| c.fidelity).sum();
        let avg_fidelity = total_fidelity / self.channels.len() as f32;

        let active_channels = self
            .channels
            .values()
            .filter(|c| self.tick.saturating_sub(c.last_update_tick) < 100)
            .count();
        let coverage = active_channels as f32 / self.channels.len() as f32;

        let integration = avg_fidelity * 0.6 + coverage * 0.4;
        integration.clamp(0.0, 1.0)
    }

    /// Compute the qualia score: subjective richness of experience
    #[inline]
    pub fn qualia_score(&mut self) -> f32 {
        if self.qualia.is_empty() {
            return 0.0;
        }

        let total_intensity: f32 = self.qualia.values().map(|q| q.intensity).sum();
        let total_richness: f32 = self.qualia.values().map(|q| q.richness).sum();
        let n = self.qualia.len() as f32;

        let avg_intensity = total_intensity / n;
        let avg_richness = total_richness / n;
        let diversity = (n / MAX_QUALIA_DIMENSIONS as f32).min(1.0);

        let score = avg_intensity * 0.4 + avg_richness * 0.4 + diversity * 0.2;
        self.qualia_score_ema = EMA_ALPHA * score + (1.0 - EMA_ALPHA) * self.qualia_score_ema;
        self.qualia_score_ema
    }

    /// Measure unity of consciousness: how well-integrated is awareness?
    #[inline]
    pub fn unity_of_consciousness(&mut self) -> f32 {
        if self.channels.len() < 2 {
            self.unity_ema = if self.channels.is_empty() { 0.0 } else { 1.0 };
            return self.unity_ema;
        }

        let fidelities: Vec<f32> = self.channels.values().map(|c| c.fidelity).collect();
        let mean = fidelities.iter().sum::<f32>() / fidelities.len() as f32;
        let variance =
            fidelities.iter().map(|f| (f - mean).powi(2)).sum::<f32>() / fidelities.len() as f32;
        let std_dev = f32_sqrt(variance);

        let coherence = (1.0 - std_dev * 2.0).clamp(0.0, 1.0);
        let coverage = self
            .channels
            .values()
            .filter(|c| c.signal_strength > 0.1)
            .count() as f32
            / self.channels.len() as f32;

        let unity = coherence * 0.6 + coverage * 0.4;
        self.unity_ema = EMA_ALPHA * unity + (1.0 - EMA_ALPHA) * self.unity_ema;
        self.unity_ema
    }

    /// How ready is the system for transcendence?
    pub fn transcendence_readiness(&self) -> f32 {
        let score = self.consciousness_score_ema;
        let distance_to_threshold = TRANSCENDENT_THRESHOLD - score;
        if distance_to_threshold <= 0.0 {
            return 1.0;
        }
        let velocity = if self.history.len() >= 2 {
            let last = self.history[self.history.len() - 1].score;
            let prev = self.history[self.history.len() - 2].score;
            last - prev
        } else {
            0.0
        };

        let readiness =
            (score / TRANSCENDENT_THRESHOLD) * 0.7 + (velocity * 10.0).clamp(0.0, 1.0) * 0.3;
        readiness.clamp(0.0, 1.0)
    }

    /// Current state of consciousness
    #[inline(always)]
    pub fn current_state(&self) -> ConsciousnessState {
        self.state
    }

    /// Update the consciousness state machine
    fn update_state_machine(&mut self) {
        let score = self.consciousness_score_ema;
        let next_threshold = match self.state {
            ConsciousnessState::Dormant => AWAKENING_THRESHOLD,
            ConsciousnessState::Awakening => AWARE_THRESHOLD,
            ConsciousnessState::Aware => REFLECTIVE_THRESHOLD,
            ConsciousnessState::Reflective => ENLIGHTENED_THRESHOLD,
            ConsciousnessState::Enlightened => TRANSCENDENT_THRESHOLD,
            ConsciousnessState::Transcendent => f32::MAX,
        };

        if score >= next_threshold {
            self.sustained_above_threshold += 1;
        } else {
            self.sustained_above_threshold = 0;
        }

        if self.sustained_above_threshold >= SUSTAIN_TICKS {
            let new_state = match self.state {
                ConsciousnessState::Dormant => ConsciousnessState::Awakening,
                ConsciousnessState::Awakening => ConsciousnessState::Aware,
                ConsciousnessState::Aware => ConsciousnessState::Reflective,
                ConsciousnessState::Reflective => ConsciousnessState::Enlightened,
                ConsciousnessState::Enlightened => ConsciousnessState::Transcendent,
                ConsciousnessState::Transcendent => ConsciousnessState::Transcendent,
            };
            if new_state != self.state {
                self.state = new_state;
                self.state_entry_tick = self.tick;
                self.sustained_above_threshold = 0;
            }
        }
    }

    /// Compute aggregate statistics
    pub fn stats(&self) -> AwarenessStats {
        let ticks_in_state = self.tick.saturating_sub(self.state_entry_tick);

        AwarenessStats {
            current_state: self.state as u8,
            consciousness_score: self.consciousness_score_ema,
            qualia_score: self.qualia_score_ema,
            unity_score: self.unity_ema,
            channel_count: self.channels.len(),
            avg_signal_strength: self.signal_strength_ema,
            avg_fidelity: self.fidelity_ema,
            ticks_in_current_state: ticks_in_state,
            transcendence_readiness: self.transcendence_readiness(),
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
