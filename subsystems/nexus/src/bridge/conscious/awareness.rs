// SPDX-License-Identifier: GPL-2.0
//! # Bridge Awareness
//!
//! Self-awareness state machine for the bridge. Progresses through levels:
//! Dormant → Aware → Reflective → Transcendent. Each transition requires
//! sustained evidence of deeper self-knowledge. A continuous consciousness
//! score aggregates multiple dimensions of awareness — perception fidelity,
//! attention focus, reflective depth, and meta-cognitive capacity.
//!
//! A kernel that knows what it knows, and knows what it doesn't know.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.08;
const AWARE_THRESHOLD: f32 = 0.25;
const REFLECTIVE_THRESHOLD: f32 = 0.55;
const TRANSCENDENT_THRESHOLD: f32 = 0.85;
const TRANSITION_SUSTAIN_TICKS: u64 = 50;
const MAX_PERCEPTION_CHANNELS: usize = 32;
const MAX_FOCUS_HISTORY: usize = 128;
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
// AWARENESS STATES
// ============================================================================

/// The bridge's awareness level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AwarenessState {
    /// No self-awareness — pure reactive mode
    Dormant = 0,
    /// Basic awareness of own operations
    Aware = 1,
    /// Can reflect on own decisions and learn from them
    Reflective = 2,
    /// Full self-model with meta-cognitive optimization
    Transcendent = 3,
}

/// A perception channel — one dimension of awareness
#[derive(Debug, Clone)]
pub struct PerceptionChannel {
    pub name: String,
    pub id: u64,
    /// Current perception fidelity (0.0 – 1.0)
    pub fidelity: f32,
    /// How noisy this channel is (lower = better)
    pub noise: f32,
    /// EMA-smoothed signal strength
    pub signal_strength: f32,
    /// Signal-to-noise ratio
    pub snr: f32,
    /// Observations count
    pub observations: u64,
}

/// Attention focus target
#[derive(Debug, Clone)]
pub struct AttentionFocus {
    pub target: String,
    pub target_id: u64,
    /// How intensely focused (0.0 – 1.0)
    pub intensity: f32,
    /// How long focus has been sustained (ticks)
    pub duration: u64,
    /// Tick when focus started
    pub start_tick: u64,
}

// ============================================================================
// CONSCIOUSNESS DIMENSIONS
// ============================================================================

/// Multi-dimensional consciousness measurement
#[derive(Debug, Clone, Copy, Default)]
pub struct ConsciousnessDimensions {
    /// Perception: how well the bridge senses its environment
    pub perception: f32,
    /// Attention: how well it allocates focus
    pub attention: f32,
    /// Memory: how well it retains and retrieves experience
    pub memory: f32,
    /// Reflection: how deeply it examines itself
    pub reflection: f32,
    /// Meta-cognition: how well it reasons about reasoning
    pub meta_cognition: f32,
    /// Agency: how effectively it pursues goals
    pub agency: f32,
}

impl ConsciousnessDimensions {
    /// Weighted aggregate score
    pub fn composite(&self) -> f32 {
        self.perception * 0.15
            + self.attention * 0.15
            + self.memory * 0.15
            + self.reflection * 0.20
            + self.meta_cognition * 0.20
            + self.agency * 0.15
    }
}

// ============================================================================
// AWARENESS STATS
// ============================================================================

/// Aggregate awareness statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct AwarenessStats {
    pub current_state: u8,
    pub consciousness_score: f32,
    pub perception_channels: usize,
    pub avg_perception_fidelity: f32,
    pub attention_focus_count: usize,
    pub state_duration_ticks: u64,
    pub transitions_total: u64,
    pub dimension_scores: ConsciousnessDimensions,
}

// ============================================================================
// BRIDGE AWARENESS ENGINE
// ============================================================================

/// Self-awareness state machine: Dormant → Aware → Reflective → Transcendent.
/// Tracks continuous consciousness score across multiple dimensions.
#[derive(Debug)]
pub struct BridgeAwareness {
    /// Current awareness state
    state: AwarenessState,
    /// Tick when current state was entered
    state_entered_tick: u64,
    /// Total state transitions
    transitions: u64,
    /// Perception channels (keyed by FNV hash)
    perceptions: BTreeMap<u64, PerceptionChannel>,
    /// Current attention focuses
    focuses: Vec<AttentionFocus>,
    /// Consciousness dimensions
    dimensions: ConsciousnessDimensions,
    /// EMA-smoothed composite consciousness score
    consciousness_ema: f32,
    /// How many ticks above the next threshold (for transition gating)
    above_threshold_ticks: u64,
    /// Monotonic tick
    tick: u64,
    /// Focus history for pattern analysis
    focus_history: Vec<u64>,
    focus_write_idx: usize,
}

impl BridgeAwareness {
    pub fn new() -> Self {
        Self {
            state: AwarenessState::Dormant,
            state_entered_tick: 0,
            transitions: 0,
            perceptions: BTreeMap::new(),
            focuses: Vec::new(),
            dimensions: ConsciousnessDimensions::default(),
            consciousness_ema: 0.0,
            above_threshold_ticks: 0,
            tick: 0,
            focus_history: Vec::new(),
            focus_write_idx: 0,
        }
    }

    /// Current awareness level
    pub fn awareness_level(&self) -> AwarenessState {
        self.state
    }

    /// Attempt a state transition based on current consciousness score
    pub fn transition_state(&mut self) -> Option<AwarenessState> {
        let score = self.consciousness_ema;
        let target = match self.state {
            AwarenessState::Dormant if score >= AWARE_THRESHOLD => Some(AwarenessState::Aware),
            AwarenessState::Aware if score >= REFLECTIVE_THRESHOLD => Some(AwarenessState::Reflective),
            AwarenessState::Reflective if score >= TRANSCENDENT_THRESHOLD => Some(AwarenessState::Transcendent),
            // Regression: can drop states if score falls too low
            AwarenessState::Transcendent if score < REFLECTIVE_THRESHOLD => Some(AwarenessState::Reflective),
            AwarenessState::Reflective if score < AWARE_THRESHOLD => Some(AwarenessState::Aware),
            AwarenessState::Aware if score < AWARE_THRESHOLD * 0.5 => Some(AwarenessState::Dormant),
            _ => None,
        };

        if let Some(new_state) = target {
            // Require sustained evidence before transitioning up
            if new_state > self.state {
                self.above_threshold_ticks += 1;
                if self.above_threshold_ticks < TRANSITION_SUSTAIN_TICKS {
                    return None;
                }
            }
            self.state = new_state;
            self.state_entered_tick = self.tick;
            self.transitions += 1;
            self.above_threshold_ticks = 0;
            Some(new_state)
        } else {
            self.above_threshold_ticks = 0;
            None
        }
    }

    /// Update a perception channel with new sensory data
    pub fn perception_update(
        &mut self,
        channel_name: &str,
        signal: f32,
        noise: f32,
    ) {
        self.tick += 1;
        let id = fnv1a_hash(channel_name.as_bytes());
        let clamped_signal = signal.max(0.0).min(1.0);
        let clamped_noise = noise.max(0.0).min(1.0);

        let channel = self.perceptions.entry(id).or_insert_with(|| PerceptionChannel {
            name: String::from(channel_name),
            id,
            fidelity: 0.5,
            noise: 0.5,
            signal_strength: 0.5,
            snr: 1.0,
            observations: 0,
        });

        channel.signal_strength = EMA_ALPHA * clamped_signal
            + (1.0 - EMA_ALPHA) * channel.signal_strength;
        channel.noise = EMA_ALPHA * clamped_noise
            + (1.0 - EMA_ALPHA) * channel.noise;
        channel.snr = if channel.noise > 0.001 {
            channel.signal_strength / channel.noise
        } else {
            channel.signal_strength * 100.0
        };
        channel.fidelity = (channel.snr / (channel.snr + 1.0)).min(1.0);
        channel.observations += 1;

        // Update perception dimension
        self.recompute_dimensions();
    }

    /// Set attention focus on a target
    pub fn attention_focus(&mut self, target: &str, intensity: f32) {
        self.tick += 1;
        let id = fnv1a_hash(target.as_bytes());
        let clamped = intensity.max(0.0).min(1.0);

        // Update existing or add new focus
        let mut found = false;
        for focus in self.focuses.iter_mut() {
            if focus.target_id == id {
                focus.intensity = EMA_ALPHA * clamped + (1.0 - EMA_ALPHA) * focus.intensity;
                focus.duration = self.tick - focus.start_tick;
                found = true;
                break;
            }
        }
        if !found {
            self.focuses.push(AttentionFocus {
                target: String::from(target),
                target_id: id,
                intensity: clamped,
                duration: 0,
                start_tick: self.tick,
            });
        }

        // Prune weak focuses (attention has limits)
        self.focuses.retain(|f| f.intensity > 0.05);

        // Normalize intensities to sum to 1.0
        let total: f32 = self.focuses.iter().map(|f| f.intensity).sum();
        if total > 0.0 {
            for f in self.focuses.iter_mut() {
                f.intensity /= total;
            }
        }

        // Record in history
        if self.focus_history.len() < MAX_FOCUS_HISTORY {
            self.focus_history.push(id);
        } else {
            self.focus_history[self.focus_write_idx] = id;
        }
        self.focus_write_idx = (self.focus_write_idx + 1) % MAX_FOCUS_HISTORY;

        self.recompute_dimensions();
    }

    /// Update consciousness dimensions from external module feedback
    pub fn update_dimensions(
        &mut self,
        memory: f32,
        reflection: f32,
        meta_cognition: f32,
        agency: f32,
    ) {
        self.dimensions.memory = EMA_ALPHA * memory.max(0.0).min(1.0)
            + (1.0 - EMA_ALPHA) * self.dimensions.memory;
        self.dimensions.reflection = EMA_ALPHA * reflection.max(0.0).min(1.0)
            + (1.0 - EMA_ALPHA) * self.dimensions.reflection;
        self.dimensions.meta_cognition = EMA_ALPHA * meta_cognition.max(0.0).min(1.0)
            + (1.0 - EMA_ALPHA) * self.dimensions.meta_cognition;
        self.dimensions.agency = EMA_ALPHA * agency.max(0.0).min(1.0)
            + (1.0 - EMA_ALPHA) * self.dimensions.agency;
        self.recompute_dimensions();
    }

    /// Composite consciousness score (0.0 – 1.0)
    pub fn consciousness_score(&self) -> f32 {
        self.consciousness_ema
    }

    /// Recompute perception and attention dimensions, then update EMA
    fn recompute_dimensions(&mut self) {
        // Perception = average channel fidelity
        if !self.perceptions.is_empty() {
            self.dimensions.perception = self.perceptions.values()
                .map(|c| c.fidelity).sum::<f32>() / self.perceptions.len() as f32;
        }
        // Attention = highest focus intensity × focus count diversity
        if !self.focuses.is_empty() {
            let max_intensity = self.focuses.iter()
                .map(|f| f.intensity).fold(0.0f32, |a, b| a.max(b));
            let diversity = (self.focuses.len() as f32).min(5.0) / 5.0;
            self.dimensions.attention = max_intensity * 0.6 + diversity * 0.4;
        }

        let composite = self.dimensions.composite();
        self.consciousness_ema = EMA_ALPHA * composite
            + (1.0 - EMA_ALPHA) * self.consciousness_ema;
    }

    /// Compute aggregate awareness statistics
    pub fn stats(&self) -> AwarenessStats {
        let avg_fidelity = if self.perceptions.is_empty() {
            0.0
        } else {
            self.perceptions.values().map(|c| c.fidelity).sum::<f32>()
                / self.perceptions.len() as f32
        };

        AwarenessStats {
            current_state: self.state as u8,
            consciousness_score: self.consciousness_ema,
            perception_channels: self.perceptions.len(),
            avg_perception_fidelity: avg_fidelity,
            attention_focus_count: self.focuses.len(),
            state_duration_ticks: self.tick.saturating_sub(self.state_entered_tick),
            transitions_total: self.transitions,
            dimension_scores: self.dimensions,
        }
    }
}
