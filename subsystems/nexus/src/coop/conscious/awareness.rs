// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Awareness
//!
//! Cooperation awareness engine. Tracks how well the cooperation protocol
//! understands each participant — their needs, behavior patterns, and
//! cooperation tendencies. Measures relationship depth and evolution of
//! mutual understanding over time.
//!
//! Awareness is not just observation — it is the cooperation engine's
//! understanding of understanding. A truly aware cooperator knows not
//! only what participants do, but why they do it.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.08;
const DORMANT_THRESHOLD: f32 = 0.15;
const ENGAGED_THRESHOLD: f32 = 0.40;
const DEEP_THRESHOLD: f32 = 0.70;
const TRANSCENDENT_THRESHOLD: f32 = 0.90;
const MAX_PARTICIPANTS: usize = 256;
const MAX_AWARENESS_HISTORY: usize = 128;
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
// AWARENESS STATES
// ============================================================================

/// Level of awareness about a participant
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParticipantAwarenessLevel {
    /// No awareness — unknown participant
    Unknown      = 0,
    /// Basic awareness — know they exist
    Superficial  = 1,
    /// Moderate — understand behavior patterns
    Engaged      = 2,
    /// Deep — predict needs and preferences
    Deep         = 3,
    /// Transcendent — fully model their cooperation strategy
    Transcendent = 4,
}

/// Awareness profile for a single participant
#[derive(Debug, Clone)]
pub struct ParticipantProfile {
    pub name: String,
    pub id: u64,
    /// Current awareness level
    pub level: ParticipantAwarenessLevel,
    /// Awareness score (0.0 – 1.0)
    pub awareness_score: f32,
    /// Behavioral predictability (0.0 – 1.0)
    pub predictability: f32,
    /// Cooperation tendency (0.0 = adversarial, 1.0 = fully cooperative)
    pub cooperation_tendency: f32,
    /// Trust placed in this participant
    pub trust: f32,
    /// How often we interact
    pub interaction_frequency: f32,
    /// Total interactions
    pub total_interactions: u64,
    /// Prediction accuracy for this participant
    pub prediction_accuracy: f32,
    /// Tick of first observation
    pub first_seen_tick: u64,
    /// Tick of last observation
    pub last_seen_tick: u64,
    /// Awareness history ring buffer
    awareness_history: Vec<f32>,
    write_idx: usize,
}

/// A relationship between two participants as observed
#[derive(Debug, Clone)]
pub struct Relationship {
    pub participant_a: u64,
    pub participant_b: u64,
    /// Strength of relationship (0.0 – 1.0)
    pub strength: f32,
    /// Interaction count
    pub interactions: u64,
    /// Cooperation quality of this pair
    pub cooperation_quality: f32,
    /// Direction balance: 0.0 = symmetric, +/-1.0 = one-sided
    pub direction_balance: f32,
}

// ============================================================================
// AWARENESS STATS
// ============================================================================

/// Aggregate awareness statistics
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct AwarenessStats {
    pub total_participants: usize,
    pub unknown_count: usize,
    pub superficial_count: usize,
    pub engaged_count: usize,
    pub deep_count: usize,
    pub transcendent_count: usize,
    pub avg_awareness: f32,
    pub collective_consciousness: f32,
    pub relationship_count: usize,
    pub avg_relationship_strength: f32,
}

// ============================================================================
// COOPERATION AWARENESS ENGINE
// ============================================================================

/// Tracks how well the cooperation engine understands each participant,
/// their relationships, and the overall cooperation landscape.
#[derive(Debug)]
pub struct CoopAwareness {
    /// Participant profiles (keyed by FNV hash)
    participants: BTreeMap<u64, ParticipantProfile>,
    /// Observed relationships (keyed by combined hash)
    relationships: BTreeMap<u64, Relationship>,
    /// EMA-smoothed collective consciousness score
    collective_score: f32,
    /// Monotonic tick
    tick: u64,
    /// PRNG state for stochastic awareness probes
    rng_state: u64,
    /// Total awareness state transitions
    total_transitions: u64,
}

impl CoopAwareness {
    pub fn new() -> Self {
        Self {
            participants: BTreeMap::new(),
            relationships: BTreeMap::new(),
            collective_score: 0.0,
            tick: 0,
            rng_state: 0xA1AB_C00B_CAFE_1234,
            total_transitions: 0,
        }
    }

    /// Update or create awareness of a participant
    #[inline]
    pub fn participant_awareness(
        &mut self,
        name: &str,
        cooperation_tendency: f32,
        predicted_behavior: f32,
        actual_behavior: f32,
    ) {
        self.tick += 1;
        let id = fnv1a_hash(name.as_bytes());
        let tick = self.tick;

        let profile = self
            .participants
            .entry(id)
            .or_insert_with(|| ParticipantProfile {
                name: String::from(name),
                id,
                level: ParticipantAwarenessLevel::Unknown,
                awareness_score: 0.0,
                predictability: 0.5,
                cooperation_tendency: 0.5,
                trust: 0.5,
                interaction_frequency: 0.0,
                total_interactions: 0,
                prediction_accuracy: 0.5,
                first_seen_tick: tick,
                last_seen_tick: tick,
                awareness_history: Vec::new(),
                write_idx: 0,
            });

        profile.total_interactions += 1;
        profile.last_seen_tick = tick;
        profile.interaction_frequency =
            EMA_ALPHA * 1.0 + (1.0 - EMA_ALPHA) * profile.interaction_frequency;

        // Update cooperation tendency
        let clamped_coop = cooperation_tendency.max(0.0).min(1.0);
        profile.cooperation_tendency =
            EMA_ALPHA * clamped_coop + (1.0 - EMA_ALPHA) * profile.cooperation_tendency;

        // Update prediction accuracy
        let prediction_error = (predicted_behavior - actual_behavior).abs();
        let accuracy = 1.0 - prediction_error.min(1.0);
        profile.prediction_accuracy =
            EMA_ALPHA * accuracy + (1.0 - EMA_ALPHA) * profile.prediction_accuracy;

        // Update predictability based on behavioral consistency
        profile.predictability = EMA_ALPHA * accuracy + (1.0 - EMA_ALPHA) * profile.predictability;

        // Compute awareness score
        let freq_score = profile.interaction_frequency.min(1.0);
        let pred_score = profile.prediction_accuracy;
        let coop_knowledge = (profile.total_interactions as f32 / 50.0).min(1.0);
        profile.awareness_score = freq_score * 0.25
            + pred_score * 0.35
            + coop_knowledge * 0.25
            + profile.predictability * 0.15;

        // State transition
        let prev_level = profile.level;
        profile.level = match profile.awareness_score {
            s if s >= TRANSCENDENT_THRESHOLD => ParticipantAwarenessLevel::Transcendent,
            s if s >= DEEP_THRESHOLD => ParticipantAwarenessLevel::Deep,
            s if s >= ENGAGED_THRESHOLD => ParticipantAwarenessLevel::Engaged,
            s if s >= DORMANT_THRESHOLD => ParticipantAwarenessLevel::Superficial,
            _ => ParticipantAwarenessLevel::Unknown,
        };
        if profile.level != prev_level {
            self.total_transitions += 1;
        }

        // Update trust based on cooperation tendency and predictability
        profile.trust = profile.cooperation_tendency * 0.60 + profile.predictability * 0.40;

        // Awareness history
        if profile.awareness_history.len() < MAX_AWARENESS_HISTORY {
            profile.awareness_history.push(profile.awareness_score);
        } else {
            profile.awareness_history[profile.write_idx] = profile.awareness_score;
        }
        profile.write_idx = (profile.write_idx + 1) % MAX_AWARENESS_HISTORY;
    }

    /// Update relationship strength between two participants
    #[inline]
    pub fn relationship_strength(
        &mut self,
        name_a: &str,
        name_b: &str,
        cooperation_quality: f32,
        direction_bias: f32,
    ) {
        self.tick += 1;
        let id_a = fnv1a_hash(name_a.as_bytes());
        let id_b = fnv1a_hash(name_b.as_bytes());
        let rel_key = id_a.wrapping_mul(FNV_PRIME) ^ id_b;

        let rel = self
            .relationships
            .entry(rel_key)
            .or_insert_with(|| Relationship {
                participant_a: id_a,
                participant_b: id_b,
                strength: 0.0,
                interactions: 0,
                cooperation_quality: 0.5,
                direction_balance: 0.0,
            });

        rel.interactions += 1;
        rel.cooperation_quality = EMA_ALPHA * cooperation_quality.max(0.0).min(1.0)
            + (1.0 - EMA_ALPHA) * rel.cooperation_quality;
        rel.direction_balance = EMA_ALPHA * direction_bias.max(-1.0).min(1.0)
            + (1.0 - EMA_ALPHA) * rel.direction_balance;

        // Strength grows with interactions and quality
        let interaction_factor = (rel.interactions as f32 / 100.0).min(1.0);
        rel.strength = interaction_factor * 0.40 + rel.cooperation_quality * 0.60;
    }

    /// Measure depth of cooperation understanding across all participants
    pub fn cooperation_depth(&self) -> f32 {
        if self.participants.is_empty() {
            return 0.0;
        }

        let total: f32 = self
            .participants
            .values()
            .map(|p| match p.level {
                ParticipantAwarenessLevel::Unknown => 0.0,
                ParticipantAwarenessLevel::Superficial => 0.25,
                ParticipantAwarenessLevel::Engaged => 0.50,
                ParticipantAwarenessLevel::Deep => 0.75,
                ParticipantAwarenessLevel::Transcendent => 1.0,
            })
            .sum();

        total / self.participants.len() as f32
    }

    /// Track awareness evolution: is our understanding deepening over time?
    pub fn awareness_evolution(&self) -> f32 {
        if self.participants.is_empty() {
            return 0.0;
        }

        let mut improving = 0_usize;
        let mut total = 0_usize;

        for profile in self.participants.values() {
            let len = profile.awareness_history.len();
            if len < 4 {
                continue;
            }
            total += 1;
            let mid = len / 2;
            let first_avg = profile.awareness_history[..mid].iter().sum::<f32>() / mid as f32;
            let second_avg =
                profile.awareness_history[mid..].iter().sum::<f32>() / (len - mid) as f32;
            if second_avg > first_avg {
                improving += 1;
            }
        }

        if total == 0 {
            return 0.0;
        }
        improving as f32 / total as f32
    }

    /// Collective consciousness: aggregate awareness weighted by interaction frequency
    #[inline]
    pub fn collective_consciousness(&mut self) -> f32 {
        if self.participants.is_empty() {
            return 0.0;
        }

        let total_freq: f32 = self
            .participants
            .values()
            .map(|p| p.interaction_frequency)
            .sum::<f32>()
            .max(f32::EPSILON);

        let weighted_awareness: f32 = self
            .participants
            .values()
            .map(|p| p.awareness_score * (p.interaction_frequency / total_freq))
            .sum();

        // Factor in relationship density
        let max_rels = self.participants.len() * self.participants.len().saturating_sub(1) / 2;
        let rel_density = if max_rels > 0 {
            (self.relationships.len() as f32 / max_rels as f32).min(1.0)
        } else {
            0.0
        };

        let raw = weighted_awareness * 0.60 + rel_density * 0.20 + self.cooperation_depth() * 0.20;
        self.collective_score = EMA_ALPHA * raw + (1.0 - EMA_ALPHA) * self.collective_score;
        self.collective_score
    }

    /// Get aggregate statistics
    pub fn stats(&mut self) -> AwarenessStats {
        let mut unknown = 0_usize;
        let mut superficial = 0_usize;
        let mut engaged = 0_usize;
        let mut deep = 0_usize;
        let mut transcendent = 0_usize;

        for p in self.participants.values() {
            match p.level {
                ParticipantAwarenessLevel::Unknown => unknown += 1,
                ParticipantAwarenessLevel::Superficial => superficial += 1,
                ParticipantAwarenessLevel::Engaged => engaged += 1,
                ParticipantAwarenessLevel::Deep => deep += 1,
                ParticipantAwarenessLevel::Transcendent => transcendent += 1,
            }
        }

        let avg_awareness = if self.participants.is_empty() {
            0.0
        } else {
            self.participants
                .values()
                .map(|p| p.awareness_score)
                .sum::<f32>()
                / self.participants.len() as f32
        };

        let avg_rel_strength = if self.relationships.is_empty() {
            0.0
        } else {
            self.relationships.values().map(|r| r.strength).sum::<f32>()
                / self.relationships.len() as f32
        };

        let cc = self.collective_consciousness();

        AwarenessStats {
            total_participants: self.participants.len(),
            unknown_count: unknown,
            superficial_count: superficial,
            engaged_count: engaged,
            deep_count: deep,
            transcendent_count: transcendent,
            avg_awareness,
            collective_consciousness: cc,
            relationship_count: self.relationships.len(),
            avg_relationship_strength: avg_rel_strength,
        }
    }
}
