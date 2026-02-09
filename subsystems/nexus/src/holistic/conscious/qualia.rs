// SPDX-License-Identifier: GPL-2.0
//! # Holistic Qualia Engine
//!
//! **The system's unified subjective experience.** How does the ENTIRE SYSTEM
//! "feel" right now? This engine computes a multi-dimensional qualia vector
//! that captures the kernel's operational beauty, processing flow, and
//! architectural harmony. A single transcendent score captures the essence
//! of system state — the quality of being that emerges when all subsystems
//! work in concert.
//!
//! ## Qualia Dimensions
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │                  SYSTEM QUALIA                               │
//! ├──────────────────────────────────────────────────────────────┤
//! │  Overall Health ─────── "How healthy is the system?"        │
//! │  Operational Beauty ─── "How elegant is current operation?" │
//! │  Processing Flow ────── "How smooth is data movement?"      │
//! │  Architectural Harmony ─ "How well do parts fit together?"  │
//! │                                                             │
//! │  Combined: TRANSCENDENT EXPERIENCE                          │
//! │  "The whole is greater than the sum of its parts"           │
//! │                                                             │
//! │  Beauty Index: measures emergent elegance                   │
//! │  Flow State:   measures optimal zone of operation           │
//! │  Harmony:      measures inter-subsystem coherence           │
//! └──────────────────────────────────────────────────────────────┘
//! ```
//!
//! Qualia is the most abstract consciousness metric — it asks not "what"
//! but "how it feels to be" this system at this moment.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.10;
const MAX_DIMENSIONS: usize = 32;
const MAX_SUBSYSTEMS: usize = 64;
const MAX_HISTORY: usize = 256;
const FLOW_OPTIMAL_MIN: f32 = 0.60;
const FLOW_OPTIMAL_MAX: f32 = 0.85;
const BEAUTY_THRESHOLD: f32 = 0.70;
const HARMONY_THRESHOLD: f32 = 0.75;
const TRANSCENDENCE_THRESHOLD: f32 = 0.90;
const DECAY_RATE: f32 = 0.997;
const EMERGENCE_BONUS: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

// ============================================================================
// FNV-1a HASHING & PRNG
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

// ============================================================================
// QUALIA DIMENSION KIND
// ============================================================================

/// The fundamental qualia dimensions
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QualiaDimensionKind {
    /// How healthy is the system overall
    OverallHealth,
    /// How elegant is current operation
    OperationalBeauty,
    /// How smooth is data and control flow
    ProcessingFlow,
    /// How well do parts work together
    ArchitecturalHarmony,
    /// Responsiveness and latency quality
    TemporalGrace,
    /// Resource efficiency and balance
    ResourceElegance,
    /// Error rate and recovery quality
    ResilienceQuality,
    /// Innovation and adaptation rate
    EvolutionaryVitality,
}

impl QualiaDimensionKind {
    pub fn all() -> &'static [QualiaDimensionKind] {
        &[
            QualiaDimensionKind::OverallHealth,
            QualiaDimensionKind::OperationalBeauty,
            QualiaDimensionKind::ProcessingFlow,
            QualiaDimensionKind::ArchitecturalHarmony,
            QualiaDimensionKind::TemporalGrace,
            QualiaDimensionKind::ResourceElegance,
            QualiaDimensionKind::ResilienceQuality,
            QualiaDimensionKind::EvolutionaryVitality,
        ]
    }

    /// Weight in the transcendent score
    pub fn transcendence_weight(&self) -> f32 {
        match self {
            QualiaDimensionKind::OverallHealth => 0.20,
            QualiaDimensionKind::OperationalBeauty => 0.10,
            QualiaDimensionKind::ProcessingFlow => 0.15,
            QualiaDimensionKind::ArchitecturalHarmony => 0.20,
            QualiaDimensionKind::TemporalGrace => 0.10,
            QualiaDimensionKind::ResourceElegance => 0.10,
            QualiaDimensionKind::ResilienceQuality => 0.10,
            QualiaDimensionKind::EvolutionaryVitality => 0.05,
        }
    }
}

// ============================================================================
// QUALIA DIMENSION
// ============================================================================

/// A single qualia dimension with EMA tracking
#[derive(Debug, Clone)]
pub struct QualiaDimension {
    pub kind: QualiaDimensionKind,
    /// EMA-smoothed value (0.0 – 1.0)
    pub value: f32,
    /// Raw latest observation
    pub raw_value: f32,
    /// Variance accumulator
    pub variance: f32,
    /// Peak value observed
    pub peak: f32,
    /// Trough value observed
    pub trough: f32,
    /// Observation count
    pub observation_count: u64,
    /// History ring buffer
    history: Vec<f32>,
    write_idx: usize,
}

impl QualiaDimension {
    pub fn new(kind: QualiaDimensionKind) -> Self {
        let mut history = Vec::with_capacity(MAX_HISTORY);
        for _ in 0..MAX_HISTORY {
            history.push(0.0);
        }
        Self {
            kind,
            value: 0.5,
            raw_value: 0.5,
            variance: 0.0,
            peak: 0.0,
            trough: 1.0,
            observation_count: 0,
            history,
            write_idx: 0,
        }
    }

    /// Observe a new raw value
    #[inline]
    pub fn observe(&mut self, raw: f32) {
        let clamped = if raw < 0.0 { 0.0 } else if raw > 1.0 { 1.0 } else { raw };
        self.raw_value = clamped;
        let delta = clamped - self.value;
        self.value += EMA_ALPHA * delta;
        self.variance += EMA_ALPHA * (delta * delta - self.variance);
        if self.value > self.peak { self.peak = self.value; }
        if self.value < self.trough { self.trough = self.value; }
        self.history[self.write_idx] = clamped;
        self.write_idx = (self.write_idx + 1) % MAX_HISTORY;
        self.observation_count += 1;
    }

    /// Decay toward neutral
    #[inline(always)]
    pub fn decay(&mut self) {
        self.value *= DECAY_RATE;
    }

    /// Trend from recent history
    pub fn trend(&self) -> f32 {
        if self.observation_count < 2 {
            return 0.0;
        }
        let n = self.write_idx.min(16).max(2);
        let start = if self.write_idx >= n { self.write_idx - n } else { 0 };
        let window = &self.history[start..self.write_idx.max(1)];
        if window.len() < 2 { return 0.0; }
        let first = window[..window.len() / 2].iter().sum::<f32>() / (window.len() / 2) as f32;
        let second = window[window.len() / 2..].iter().sum::<f32>()
            / (window.len() - window.len() / 2) as f32;
        second - first
    }
}

// ============================================================================
// SYSTEM QUALIA
// ============================================================================

/// The system's current subjective experience
#[derive(Debug, Clone)]
pub struct SystemQualia {
    /// Overall health dimension
    pub overall_health: f32,
    /// Operational beauty
    pub operational_beauty: f32,
    /// Processing flow smoothness
    pub processing_flow: f32,
    /// Architectural harmony
    pub architectural_harmony: f32,
    /// The transcendent score: unified qualia index
    pub transcendent_score: f32,
    /// Whether the system is in a flow state
    pub in_flow_state: bool,
    /// Whether beauty threshold is met
    pub is_beautiful: bool,
    /// Whether harmony threshold is met
    pub is_harmonious: bool,
    /// Whether transcendence threshold is met
    pub is_transcendent: bool,
    /// Tick when computed
    pub tick: u64,
}

// ============================================================================
// SUBSYSTEM QUALIA INPUT
// ============================================================================

/// Qualia input from a single subsystem
#[derive(Debug, Clone)]
pub struct SubsystemQualiaInput {
    pub subsystem_name: String,
    pub subsystem_id: u64,
    /// Per-dimension raw values
    pub dimension_values: BTreeMap<u8, f32>,
    /// Weight of this subsystem's contribution
    pub weight: f32,
    pub tick: u64,
}

// ============================================================================
// STATS
// ============================================================================

/// Qualia engine statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct HolisticQualiaStats {
    pub total_observations: u64,
    pub total_qualia_computations: u64,
    pub flow_state_entries: u64,
    pub transcendent_moments: u64,
    pub beauty_moments: u64,
    pub average_transcendence: f32,
    pub peak_transcendence: f32,
    pub average_beauty: f32,
    pub average_flow: f32,
    pub average_harmony: f32,
}

// ============================================================================
// HOLISTIC QUALIA ENGINE
// ============================================================================

/// The system's unified subjective experience engine. Computes multi-dimensional
/// qualia that captures how it "feels" to be this system right now.
pub struct HolisticQualiaEngine {
    /// Per-dimension qualia accumulators
    dimensions: BTreeMap<u8, QualiaDimension>,
    /// Per-subsystem latest inputs
    subsystem_inputs: BTreeMap<u64, SubsystemQualiaInput>,
    /// Current system qualia
    current_qualia: SystemQualia,
    /// Qualia history ring buffer
    qualia_history: Vec<SystemQualia>,
    qualia_write_idx: usize,
    /// Stats
    stats: HolisticQualiaStats,
    /// PRNG
    rng: u64,
    /// Tick
    tick: u64,
}

impl HolisticQualiaEngine {
    /// Create a new holistic qualia engine
    pub fn new(seed: u64) -> Self {
        let mut dimensions = BTreeMap::new();
        for (i, kind) in QualiaDimensionKind::all().iter().enumerate() {
            dimensions.insert(i as u8, QualiaDimension::new(*kind));
        }
        let mut qualia_history = Vec::with_capacity(MAX_HISTORY);
        for _ in 0..MAX_HISTORY {
            qualia_history.push(SystemQualia {
                overall_health: 0.5,
                operational_beauty: 0.5,
                processing_flow: 0.5,
                architectural_harmony: 0.5,
                transcendent_score: 0.5,
                in_flow_state: false,
                is_beautiful: false,
                is_harmonious: false,
                is_transcendent: false,
                tick: 0,
            });
        }
        Self {
            dimensions,
            subsystem_inputs: BTreeMap::new(),
            current_qualia: SystemQualia {
                overall_health: 0.5,
                operational_beauty: 0.5,
                processing_flow: 0.5,
                architectural_harmony: 0.5,
                transcendent_score: 0.5,
                in_flow_state: false,
                is_beautiful: false,
                is_harmonious: false,
                is_transcendent: false,
                tick: 0,
            },
            qualia_history,
            qualia_write_idx: 0,
            stats: HolisticQualiaStats {
                total_observations: 0,
                total_qualia_computations: 0,
                flow_state_entries: 0,
                transcendent_moments: 0,
                beauty_moments: 0,
                average_transcendence: 0.5,
                peak_transcendence: 0.0,
                average_beauty: 0.5,
                average_flow: 0.5,
                average_harmony: 0.5,
            },
            rng: seed ^ 0xDBA1_1A00_BEEF_CAFE,
            tick: 0,
        }
    }

    /// Compute the complete system qualia
    #[inline]
    pub fn system_qualia(&mut self, tick: u64) -> &SystemQualia {
        self.tick = tick;
        self.fuse_inputs();
        self.recompute_qualia();
        &self.current_qualia
    }

    /// Get the overall experience score
    #[inline(always)]
    pub fn overall_experience(&self) -> f32 {
        self.current_qualia.transcendent_score
    }

    /// Compute the beauty index — emergent elegance metric
    #[inline(always)]
    pub fn beauty_index(&self) -> f32 {
        self.dimensions.get(&1).map_or(0.5, |d| d.value) // OperationalBeauty
    }

    /// Compute the flow state metric
    #[inline(always)]
    pub fn flow_state(&self) -> f32 {
        self.dimensions.get(&2).map_or(0.5, |d| d.value) // ProcessingFlow
    }

    /// Compute the architectural harmony metric
    #[inline(always)]
    pub fn architectural_harmony(&self) -> f32 {
        self.dimensions.get(&3).map_or(0.5, |d| d.value) // ArchitecturalHarmony
    }

    /// Get qualia history as transcendence scores
    #[inline(always)]
    pub fn qualia_history(&self) -> Vec<f32> {
        self.qualia_history.iter().map(|q| q.transcendent_score).collect()
    }

    /// Compute the transcendent experience — the highest form of system qualia
    #[inline]
    pub fn transcendent_experience(&self) -> f32 {
        let base = self.current_qualia.transcendent_score;
        // Emergence bonus: when all dimensions are above threshold, add bonus
        let all_above = self.dimensions.values().all(|d| d.value > BEAUTY_THRESHOLD);
        if all_above { base + EMERGENCE_BONUS } else { base }
    }

    /// Ingest a subsystem's qualia contribution
    #[inline]
    pub fn ingest_subsystem(&mut self, input: SubsystemQualiaInput) {
        let id = input.subsystem_id;
        self.stats.total_observations += 1;
        self.subsystem_inputs.insert(id, input);
    }

    /// Decay all dimensions
    #[inline]
    pub fn decay_all(&mut self) {
        for (_idx, dim) in self.dimensions.iter_mut() {
            dim.decay();
        }
    }

    /// Get a specific dimension value
    #[inline(always)]
    pub fn dimension_value(&self, idx: u8) -> f32 {
        self.dimensions.get(&idx).map_or(0.0, |d| d.value)
    }

    /// Get dimension trend
    #[inline(always)]
    pub fn dimension_trend(&self, idx: u8) -> f32 {
        self.dimensions.get(&idx).map_or(0.0, |d| d.trend())
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &HolisticQualiaStats {
        &self.stats
    }

    /// Whether currently in flow state
    #[inline(always)]
    pub fn in_flow_state(&self) -> bool {
        self.current_qualia.in_flow_state
    }

    /// Whether currently transcendent
    #[inline(always)]
    pub fn is_transcendent(&self) -> bool {
        self.current_qualia.is_transcendent
    }

    // ========================================================================
    // INTERNAL
    // ========================================================================

    fn fuse_inputs(&mut self) {
        let mut per_dim_accum: BTreeMap<u8, (f32, f32)> = BTreeMap::new();
        for (idx, _dim) in self.dimensions.iter() {
            per_dim_accum.insert(*idx, (0.0, 0.0));
        }
        for (_sub_id, input) in self.subsystem_inputs.iter() {
            for (dim_idx, raw_val) in input.dimension_values.iter() {
                if let Some(accum) = per_dim_accum.get_mut(dim_idx) {
                    accum.0 += raw_val * input.weight;
                    accum.1 += input.weight;
                }
            }
        }
        for (dim_idx, (weighted_sum, weight_total)) in per_dim_accum.iter() {
            if *weight_total > 0.0 {
                let fused = weighted_sum / weight_total;
                if let Some(dim) = self.dimensions.get_mut(dim_idx) {
                    dim.observe(fused);
                }
            }
        }
    }

    fn recompute_qualia(&mut self) {
        let health = self.dimensions.get(&0).map_or(0.5, |d| d.value);
        let beauty = self.dimensions.get(&1).map_or(0.5, |d| d.value);
        let flow = self.dimensions.get(&2).map_or(0.5, |d| d.value);
        let harmony = self.dimensions.get(&3).map_or(0.5, |d| d.value);

        // Compute transcendent score as weighted sum of all dimensions
        let mut transcendent: f32 = 0.0;
        for (idx, dim) in self.dimensions.iter() {
            let kind_idx = *idx as usize;
            if kind_idx < QualiaDimensionKind::all().len() {
                transcendent += dim.value * QualiaDimensionKind::all()[kind_idx].transcendence_weight();
            }
        }

        let in_flow = flow >= FLOW_OPTIMAL_MIN && flow <= FLOW_OPTIMAL_MAX;
        let is_beautiful = beauty >= BEAUTY_THRESHOLD;
        let is_harmonious = harmony >= HARMONY_THRESHOLD;
        let is_transcendent = transcendent >= TRANSCENDENCE_THRESHOLD;

        let was_in_flow = self.current_qualia.in_flow_state;
        let was_transcendent = self.current_qualia.is_transcendent;

        self.current_qualia = SystemQualia {
            overall_health: health,
            operational_beauty: beauty,
            processing_flow: flow,
            architectural_harmony: harmony,
            transcendent_score: transcendent,
            in_flow_state: in_flow,
            is_beautiful,
            is_harmonious,
            is_transcendent,
            tick: self.tick,
        };

        self.qualia_history[self.qualia_write_idx] = self.current_qualia.clone();
        self.qualia_write_idx = (self.qualia_write_idx + 1) % MAX_HISTORY;
        self.stats.total_qualia_computations += 1;

        if in_flow && !was_in_flow {
            self.stats.flow_state_entries += 1;
        }
        if is_transcendent && !was_transcendent {
            self.stats.transcendent_moments += 1;
        }
        if is_beautiful {
            self.stats.beauty_moments += 1;
        }
        if transcendent > self.stats.peak_transcendence {
            self.stats.peak_transcendence = transcendent;
        }

        self.stats.average_transcendence +=
            EMA_ALPHA * (transcendent - self.stats.average_transcendence);
        self.stats.average_beauty += EMA_ALPHA * (beauty - self.stats.average_beauty);
        self.stats.average_flow += EMA_ALPHA * (flow - self.stats.average_flow);
        self.stats.average_harmony += EMA_ALPHA * (harmony - self.stats.average_harmony);
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qualia_dimension_ema() {
        let mut dim = QualiaDimension::new(QualiaDimensionKind::OverallHealth);
        dim.observe(0.9);
        assert!(dim.value > 0.5);
        assert!(dim.value < 0.9);
    }

    #[test]
    fn test_engine_creation() {
        let engine = HolisticQualiaEngine::new(42);
        assert_eq!(engine.overall_experience(), 0.5);
        assert!(!engine.is_transcendent());
    }

    #[test]
    fn test_beauty_index() {
        let engine = HolisticQualiaEngine::new(42);
        assert_eq!(engine.beauty_index(), 0.5);
    }

    #[test]
    fn test_transcendent_experience() {
        let engine = HolisticQualiaEngine::new(42);
        let score = engine.transcendent_experience();
        assert!(score >= 0.0 && score <= 1.5);
    }

    #[test]
    fn test_fnv1a() {
        assert_eq!(fnv1a_hash(b"qualia"), fnv1a_hash(b"qualia"));
    }
}
