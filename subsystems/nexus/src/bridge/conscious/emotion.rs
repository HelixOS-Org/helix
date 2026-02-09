// SPDX-License-Identifier: GPL-2.0
//! # Bridge Emotion Engine
//!
//! Computational emotional signals for bridge operations. These are NOT human
//! emotions — they are control signals that modulate bridge behaviour:
//!
//! - **Stress** — syscall queue overload, excessive contention
//! - **Confidence** — prediction accuracy high, optimizations landing
//! - **Curiosity** — unknown syscall patterns detected, exploration warranted
//! - **Satisfaction** — optimization target met, efficiency goals reached
//! - **Urgency** — deadline approaching, latency budgets shrinking
//! - **Calm** — system stable, low variance, steady-state
//!
//! Emotions influence bridge routing: high stress → conservative paths,
//! high confidence → aggressive optimization, high curiosity → exploration.
//! A full state machine governs transitions between emotional states.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.10;
const MAX_EMOTION_HISTORY: usize = 256;
const MAX_ACTIVE_EMOTIONS: usize = 16;
const MAX_TRANSITION_RULES: usize = 64;
const INTENSITY_DECAY_RATE: f32 = 0.02;
const STRESS_THRESHOLD_HIGH: f32 = 0.75;
const STRESS_THRESHOLD_CRITICAL: f32 = 0.90;
const CONFIDENCE_BOOST: f32 = 0.05;
const CURIOSITY_TRIGGER_NOVELTY: f32 = 0.60;
const SATISFACTION_GOAL_MET: f32 = 0.80;
const URGENCY_DEADLINE_FACTOR: f32 = 0.15;
const CALM_VARIANCE_CEILING: f32 = 0.10;
const TRAJECTORY_WINDOW: usize = 32;
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

fn ema_update(current: f32, new_sample: f32, alpha: f32) -> f32 {
    current * (1.0 - alpha) + new_sample * alpha
}

// ============================================================================
// EMOTION ENUM
// ============================================================================

/// The six computational emotions of the bridge
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BridgeEmotion {
    /// Syscall queue overload, excessive contention
    Stress,
    /// Prediction accuracy high, optimizations landing
    Confidence,
    /// Unknown syscall patterns detected
    Curiosity,
    /// Optimization target met
    Satisfaction,
    /// Deadline approaching, latency budget shrinking
    Urgency,
    /// System stable, low variance, steady-state
    Calm,
}

impl BridgeEmotion {
    fn all() -> &'static [BridgeEmotion] {
        &[
            BridgeEmotion::Stress,
            BridgeEmotion::Confidence,
            BridgeEmotion::Curiosity,
            BridgeEmotion::Satisfaction,
            BridgeEmotion::Urgency,
            BridgeEmotion::Calm,
        ]
    }

    fn as_str(&self) -> &'static str {
        match self {
            BridgeEmotion::Stress => "stress",
            BridgeEmotion::Confidence => "confidence",
            BridgeEmotion::Curiosity => "curiosity",
            BridgeEmotion::Satisfaction => "satisfaction",
            BridgeEmotion::Urgency => "urgency",
            BridgeEmotion::Calm => "calm",
        }
    }

    fn opposes(&self, other: &BridgeEmotion) -> bool {
        matches!(
            (self, other),
            (BridgeEmotion::Stress, BridgeEmotion::Calm)
                | (BridgeEmotion::Calm, BridgeEmotion::Stress)
                | (BridgeEmotion::Confidence, BridgeEmotion::Urgency)
                | (BridgeEmotion::Urgency, BridgeEmotion::Confidence)
        )
    }
}

// ============================================================================
// EMOTION STATE
// ============================================================================

/// A single active emotional state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EmotionState {
    pub emotion: BridgeEmotion,
    pub intensity: f32,
    pub duration_ticks: u64,
    pub trigger: String,
    pub onset_tick: u64,
    pub peak_intensity: f32,
    pub decay_rate: f32,
}

impl EmotionState {
    fn new(emotion: BridgeEmotion, intensity: f32, trigger: String, tick: u64) -> Self {
        let clamped = intensity.clamp(0.0, 1.0);
        Self {
            emotion,
            intensity: clamped,
            duration_ticks: 0,
            trigger,
            onset_tick: tick,
            peak_intensity: clamped,
            decay_rate: INTENSITY_DECAY_RATE,
        }
    }

    fn tick(&mut self) {
        self.duration_ticks += 1;
        self.intensity = (self.intensity - self.decay_rate).max(0.0);
    }

    fn reinforce(&mut self, amount: f32) {
        self.intensity = (self.intensity + amount).clamp(0.0, 1.0);
        if self.intensity > self.peak_intensity {
            self.peak_intensity = self.intensity;
        }
    }

    fn is_active(&self) -> bool {
        self.intensity > 0.01
    }
}

// ============================================================================
// TRANSITION RULE
// ============================================================================

/// A rule governing transitions between emotional states
#[derive(Debug, Clone)]
pub struct TransitionRule {
    pub from: BridgeEmotion,
    pub to: BridgeEmotion,
    pub condition_hash: u64,
    pub probability: f32,
    pub times_fired: u64,
}

// ============================================================================
// EMOTION HISTORY ENTRY
// ============================================================================

#[derive(Debug, Clone)]
struct EmotionHistoryEntry {
    emotion: BridgeEmotion,
    intensity: f32,
    tick: u64,
}

// ============================================================================
// STATS
// ============================================================================

/// Statistics for the emotion engine
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EmotionStats {
    pub total_evaluations: u64,
    pub total_transitions: u64,
    pub dominant_emotion_changes: u64,
    pub avg_stress_level: f32,
    pub avg_confidence: f32,
    pub current_dominant: Option<BridgeEmotion>,
    pub active_emotion_count: usize,
}

// ============================================================================
// BRIDGE EMOTION ENGINE
// ============================================================================

/// Core engine managing computational emotional signals for bridge routing
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BridgeEmotionEngine {
    active_emotions: BTreeMap<u64, EmotionState>,
    transition_rules: Vec<TransitionRule>,
    history: VecDeque<EmotionHistoryEntry>,
    trajectory_buffer: VecDeque<(BridgeEmotion, f32)>,
    stress_ema: f32,
    confidence_ema: f32,
    curiosity_ema: f32,
    satisfaction_ema: f32,
    urgency_ema: f32,
    calm_ema: f32,
    current_tick: u64,
    total_evaluations: u64,
    total_transitions: u64,
    dominant_changes: u64,
    last_dominant: Option<BridgeEmotion>,
    rng_state: u64,
}

impl BridgeEmotionEngine {
    /// Create a new emotion engine
    pub fn new(seed: u64) -> Self {
        let mut engine = Self {
            active_emotions: BTreeMap::new(),
            transition_rules: Vec::new(),
            history: VecDeque::new(),
            trajectory_buffer: VecDeque::new(),
            stress_ema: 0.0,
            confidence_ema: 0.5,
            curiosity_ema: 0.0,
            satisfaction_ema: 0.0,
            urgency_ema: 0.0,
            calm_ema: 0.5,
            current_tick: 0,
            total_evaluations: 0,
            total_transitions: 0,
            dominant_changes: 0,
            last_dominant: None,
            rng_state: seed | 1,
        };
        engine.install_default_transitions();
        engine
    }

    fn install_default_transitions(&mut self) {
        let pairs = [
            (BridgeEmotion::Stress, BridgeEmotion::Urgency, 0.7),
            (BridgeEmotion::Urgency, BridgeEmotion::Stress, 0.5),
            (BridgeEmotion::Confidence, BridgeEmotion::Satisfaction, 0.6),
            (BridgeEmotion::Satisfaction, BridgeEmotion::Calm, 0.8),
            (BridgeEmotion::Curiosity, BridgeEmotion::Confidence, 0.4),
            (BridgeEmotion::Calm, BridgeEmotion::Curiosity, 0.3),
            (BridgeEmotion::Stress, BridgeEmotion::Calm, 0.1),
            (BridgeEmotion::Calm, BridgeEmotion::Stress, 0.2),
        ];
        for (from, to, prob) in pairs {
            let key = {
                let mut buf = Vec::new();
                buf.extend_from_slice(from.as_str().as_bytes());
                buf.push(b'-');
                buf.push(b'>');
                buf.extend_from_slice(to.as_str().as_bytes());
                fnv1a_hash(&buf)
            };
            self.transition_rules.push(TransitionRule {
                from,
                to,
                condition_hash: key,
                probability: prob,
                times_fired: 0,
            });
        }
    }

    /// Evaluate an emotional signal from a bridge observation
    #[inline]
    pub fn evaluate_emotion(
        &mut self,
        emotion: BridgeEmotion,
        raw_intensity: f32,
        trigger: &str,
    ) {
        self.current_tick += 1;
        self.total_evaluations += 1;
        let intensity = raw_intensity.clamp(0.0, 1.0);

        // Update EMA for this emotion
        match emotion {
            BridgeEmotion::Stress => self.stress_ema = ema_update(self.stress_ema, intensity, EMA_ALPHA),
            BridgeEmotion::Confidence => self.confidence_ema = ema_update(self.confidence_ema, intensity, EMA_ALPHA),
            BridgeEmotion::Curiosity => self.curiosity_ema = ema_update(self.curiosity_ema, intensity, EMA_ALPHA),
            BridgeEmotion::Satisfaction => self.satisfaction_ema = ema_update(self.satisfaction_ema, intensity, EMA_ALPHA),
            BridgeEmotion::Urgency => self.urgency_ema = ema_update(self.urgency_ema, intensity, EMA_ALPHA),
            BridgeEmotion::Calm => self.calm_ema = ema_update(self.calm_ema, intensity, EMA_ALPHA),
        }

        let key = fnv1a_hash(emotion.as_str().as_bytes());

        if let Some(existing) = self.active_emotions.get_mut(&key) {
            existing.reinforce(intensity * 0.5);
        } else if self.active_emotions.len() < MAX_ACTIVE_EMOTIONS {
            let state = EmotionState::new(
                emotion,
                intensity,
                String::from(trigger),
                self.current_tick,
            );
            self.active_emotions.insert(key, state);
        }

        // Suppress opposing emotions
        let mut to_suppress = Vec::new();
        for (&k, es) in self.active_emotions.iter() {
            if emotion.opposes(&es.emotion) && intensity > es.intensity {
                to_suppress.push(k);
            }
        }
        for k in to_suppress {
            if let Some(es) = self.active_emotions.get_mut(&k) {
                es.intensity = (es.intensity - intensity * 0.3).max(0.0);
            }
        }

        // Record in history
        if self.history.len() >= MAX_EMOTION_HISTORY {
            self.history.pop_front();
        }
        self.history.push_back(EmotionHistoryEntry {
            emotion,
            intensity,
            tick: self.current_tick,
        });

        // Update trajectory buffer
        if self.trajectory_buffer.len() >= TRAJECTORY_WINDOW {
            self.trajectory_buffer.pop_front();
        }
        self.trajectory_buffer.push_back((emotion, intensity));

        // Check for emotional transitions
        self.process_transitions(emotion, intensity);

        // Decay all active emotions
        let mut to_remove = Vec::new();
        for (&k, es) in self.active_emotions.iter_mut() {
            es.tick();
            if !es.is_active() {
                to_remove.push(k);
            }
        }
        for k in to_remove {
            self.active_emotions.remove(&k);
        }
    }

    fn process_transitions(&mut self, current: BridgeEmotion, intensity: f32) {
        let mut triggered = Vec::new();
        for (i, rule) in self.transition_rules.iter().enumerate() {
            if rule.from == current && intensity > 0.5 {
                let rand_val = xorshift64(&mut self.rng_state);
                let roll = (rand_val % 1000) as f32 / 1000.0;
                if roll < rule.probability {
                    triggered.push((i, rule.to, intensity * rule.probability));
                }
            }
        }
        for (idx, target_emotion, derived_intensity) in triggered {
            self.transition_rules[idx].times_fired += 1;
            self.total_transitions += 1;

            let key = fnv1a_hash(target_emotion.as_str().as_bytes());
            if let Some(existing) = self.active_emotions.get_mut(&key) {
                existing.reinforce(derived_intensity * 0.3);
            } else if self.active_emotions.len() < MAX_ACTIVE_EMOTIONS {
                let state = EmotionState::new(
                    target_emotion,
                    derived_intensity,
                    String::from("transition"),
                    self.current_tick,
                );
                self.active_emotions.insert(key, state);
            }
        }
    }

    /// Return the dominant emotion — the one with highest current intensity
    pub fn dominant_emotion(&mut self) -> Option<(BridgeEmotion, f32)> {
        let mut best: Option<(BridgeEmotion, f32)> = None;
        for es in self.active_emotions.values() {
            match best {
                None => best = Some((es.emotion, es.intensity)),
                Some((_, bi)) if es.intensity > bi => {
                    best = Some((es.emotion, es.intensity));
                }
                _ => {}
            }
        }
        if let Some((emo, _)) = best {
            if self.last_dominant != Some(emo) {
                self.dominant_changes += 1;
                self.last_dominant = Some(emo);
            }
        }
        best
    }

    /// Compute emotion transition: what is the bridge likely to feel next?
    pub fn emotion_transition(&self) -> Vec<(BridgeEmotion, f32)> {
        let mut predictions: BTreeMap<u64, (BridgeEmotion, f32)> = BTreeMap::new();
        for es in self.active_emotions.values() {
            for rule in &self.transition_rules {
                if rule.from == es.emotion && es.intensity > 0.3 {
                    let score = es.intensity * rule.probability;
                    let key = fnv1a_hash(rule.to.as_str().as_bytes());
                    let entry = predictions.entry(key).or_insert((rule.to, 0.0));
                    entry.1 += score;
                }
            }
        }
        let mut result: Vec<(BridgeEmotion, f32)> = predictions.values().cloned().collect();
        result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        result
    }

    /// Current stress level (EMA-smoothed)
    #[inline(always)]
    pub fn stress_level(&self) -> f32 {
        self.stress_ema
    }

    /// Current confidence score (EMA-smoothed)
    #[inline(always)]
    pub fn confidence_score(&self) -> f32 {
        self.confidence_ema
    }

    /// Compute emotional trajectory — trend direction over the trajectory window
    pub fn emotional_trajectory(&self) -> BTreeMap<u64, f32> {
        let mut trends: BTreeMap<u64, (f32, f32, u32)> = BTreeMap::new();
        let len = self.trajectory_buffer.len();
        if len < 2 {
            return BTreeMap::new();
        }
        for (i, (emo, intensity)) in self.trajectory_buffer.iter().enumerate() {
            let key = fnv1a_hash(emo.as_str().as_bytes());
            let position = i as f32 / len as f32;
            let entry = trends.entry(key).or_insert((0.0, 0.0, 0));
            entry.0 += position * intensity;
            entry.1 += intensity;
            entry.2 += 1;
        }
        let mut result = BTreeMap::new();
        for (key, (weighted_sum, total, count)) in &trends {
            if *count > 1 {
                let avg_position = *weighted_sum / *total;
                let trend = (avg_position - 0.5) * 2.0;
                result.insert(*key, trend);
            }
        }
        result
    }

    /// Is the bridge in a stressed state?
    #[inline(always)]
    pub fn is_stressed(&self) -> bool {
        self.stress_ema > STRESS_THRESHOLD_HIGH
    }

    /// Is the bridge in a critically stressed state?
    #[inline(always)]
    pub fn is_critical_stress(&self) -> bool {
        self.stress_ema > STRESS_THRESHOLD_CRITICAL
    }

    /// Routing recommendation based on emotional state
    #[inline]
    pub fn routing_recommendation(&self) -> RoutingBias {
        if self.stress_ema > STRESS_THRESHOLD_HIGH {
            RoutingBias::Conservative
        } else if self.confidence_ema > 0.7 && self.stress_ema < 0.3 {
            RoutingBias::Aggressive
        } else if self.curiosity_ema > CURIOSITY_TRIGGER_NOVELTY {
            RoutingBias::Exploratory
        } else {
            RoutingBias::Balanced
        }
    }

    /// Get statistics snapshot
    #[inline]
    pub fn stats(&self) -> EmotionStats {
        EmotionStats {
            total_evaluations: self.total_evaluations,
            total_transitions: self.total_transitions,
            dominant_emotion_changes: self.dominant_changes,
            avg_stress_level: self.stress_ema,
            avg_confidence: self.confidence_ema,
            current_dominant: self.last_dominant,
            active_emotion_count: self.active_emotions.len(),
        }
    }

    /// Get the current emotional palette — all active emotions and intensities
    #[inline]
    pub fn emotional_palette(&self) -> Vec<(BridgeEmotion, f32)> {
        let mut palette: Vec<(BridgeEmotion, f32)> = self
            .active_emotions
            .values()
            .map(|es| (es.emotion, es.intensity))
            .collect();
        palette.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        palette
    }

    /// Emotional valence: positive emotions minus negative emotions
    #[inline]
    pub fn valence(&self) -> f32 {
        let positive = self.confidence_ema + self.satisfaction_ema + self.calm_ema;
        let negative = self.stress_ema + self.urgency_ema;
        let neutral = self.curiosity_ema;
        (positive - negative + neutral * 0.1) / 3.0
    }

    /// Emotional arousal: total intensity across all active emotions
    #[inline]
    pub fn arousal(&self) -> f32 {
        if self.active_emotions.is_empty() {
            return 0.0;
        }
        let total: f32 = self.active_emotions.values().map(|es| es.intensity).sum();
        (total / self.active_emotions.len() as f32).clamp(0.0, 1.0)
    }

    /// Reset all emotional states
    #[inline]
    pub fn reset(&mut self) {
        self.active_emotions.clear();
        self.stress_ema = 0.0;
        self.confidence_ema = 0.5;
        self.curiosity_ema = 0.0;
        self.satisfaction_ema = 0.0;
        self.urgency_ema = 0.0;
        self.calm_ema = 0.5;
    }
}

// ============================================================================
// ROUTING BIAS
// ============================================================================

/// Bridge routing recommendation derived from emotional state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingBias {
    /// High stress → safe, well-tested paths only
    Conservative,
    /// High confidence → aggressive optimizations enabled
    Aggressive,
    /// High curiosity → try new paths, gather data
    Exploratory,
    /// Default — balanced routing
    Balanced,
}
