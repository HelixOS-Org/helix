// SPDX-License-Identifier: GPL-2.0
//! # Apps Emotion Engine
//!
//! Computational emotional signals for application understanding. Emotions here
//! are not anthropomorphic whimsy — they are structured, quantitative signals
//! that guide resource allocation and attention priority. When an application
//! is "stressed" (thrashing pages, starving for CPU), the engine raises an
//! `AppStress` signal that biases the scheduler toward intervention. When a
//! classification is certain, `AppConfidence` allows the engine to allocate
//! monitoring resources elsewhere.
//!
//! Supported emotional signals:
//! - **AppStress** — Application is thrashing or resource-starved
//! - **AppConfidence** — Classification certainty is high
//! - **AppCuriosity** — Unknown pattern discovered, warrants investigation
//! - **AppSatisfaction** — Application is meeting its SLA comfortably
//! - **AppFrustration** — Repeated failures or misclassifications
//! - **AppExcitement** — Novel positive behavior detected
//!
//! Emotions are EMA-smoothed and decay over time. The dominant emotion for
//! any app or the entire landscape drives downstream allocation decisions.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.15;
const EMOTION_DECAY: f32 = 0.995;
const MAX_APPS: usize = 1024;
const MAX_EMOTION_HISTORY: usize = 128;
const STRESS_THRESHOLD: f32 = 0.7;
const CONFIDENCE_HIGH: f32 = 0.85;
const CURIOSITY_THRESHOLD: f32 = 0.6;
const SATISFACTION_THRESHOLD: f32 = 0.75;
const FRUSTRATION_THRESHOLD: f32 = 0.65;
const EXCITEMENT_THRESHOLD: f32 = 0.7;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

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

/// Xorshift64 PRNG for noise injection in emotion decay
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

// ============================================================================
// EMOTION TYPES
// ============================================================================

/// Discrete emotional signal categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EmotionKind {
    /// Application is thrashing or resource-starved
    Stress,
    /// Classification certainty is high
    Confidence,
    /// Unknown pattern warrants investigation
    Curiosity,
    /// Application meeting its SLA comfortably
    Satisfaction,
    /// Repeated failures or misclassifications
    Frustration,
    /// Novel positive behavior detected
    Excitement,
}

impl EmotionKind {
    /// All variants for iteration
    fn all() -> &'static [EmotionKind] {
        &[
            EmotionKind::Stress,
            EmotionKind::Confidence,
            EmotionKind::Curiosity,
            EmotionKind::Satisfaction,
            EmotionKind::Frustration,
            EmotionKind::Excitement,
        ]
    }

    /// Human-readable label
    #[inline]
    pub fn label(&self) -> &'static str {
        match self {
            EmotionKind::Stress => "stress",
            EmotionKind::Confidence => "confidence",
            EmotionKind::Curiosity => "curiosity",
            EmotionKind::Satisfaction => "satisfaction",
            EmotionKind::Frustration => "frustration",
            EmotionKind::Excitement => "excitement",
        }
    }
}

// ============================================================================
// PER-APP EMOTIONAL STATE
// ============================================================================

/// Emotional signal with EMA smoothing
#[derive(Debug, Clone)]
pub struct EmotionSignal {
    pub kind: EmotionKind,
    pub intensity: f32,
    pub raw_latest: f32,
    pub variance: f32,
    pub trigger_count: u64,
    pub last_trigger_tick: u64,
    /// History ring buffer
    history: Vec<f32>,
    write_idx: usize,
}

impl EmotionSignal {
    fn new(kind: EmotionKind) -> Self {
        Self {
            kind,
            intensity: 0.0,
            raw_latest: 0.0,
            variance: 0.0,
            trigger_count: 0,
            last_trigger_tick: 0,
            history: Vec::new(),
            write_idx: 0,
        }
    }

    #[inline]
    fn update(&mut self, raw: f32, tick: u64) {
        self.raw_latest = raw;
        self.trigger_count += 1;
        self.last_trigger_tick = tick;

        // EMA update
        self.intensity = EMA_ALPHA * raw + (1.0 - EMA_ALPHA) * self.intensity;

        // Online variance
        let diff = raw - self.intensity;
        self.variance = EMA_ALPHA * diff * diff + (1.0 - EMA_ALPHA) * self.variance;

        // Ring buffer
        if self.history.len() < MAX_EMOTION_HISTORY {
            self.history.push(raw);
        } else {
            self.history[self.write_idx] = raw;
        }
        self.write_idx = (self.write_idx + 1) % MAX_EMOTION_HISTORY;
    }

    fn decay(&mut self) {
        self.intensity *= EMOTION_DECAY;
        if self.intensity < 0.001 {
            self.intensity = 0.0;
        }
    }

    fn trend(&self) -> f32 {
        if self.history.len() < 4 {
            return 0.0;
        }
        let len = self.history.len();
        let mid = len / 2;
        let first_half: f32 = self.history[..mid].iter().sum::<f32>() / mid as f32;
        let second_half: f32 = self.history[mid..].iter().sum::<f32>() / (len - mid) as f32;
        second_half - first_half
    }
}

/// Complete emotional profile for a single application
#[derive(Debug, Clone)]
pub struct AppEmotionalProfile {
    pub app_id: u64,
    pub app_name: String,
    pub emotions: BTreeMap<u8, EmotionSignal>,
    pub dominant: EmotionKind,
    pub emotional_volatility: f32,
    pub last_evaluation_tick: u64,
    pub total_evaluations: u64,
}

impl AppEmotionalProfile {
    fn new(app_id: u64, app_name: String) -> Self {
        let mut emotions = BTreeMap::new();
        for (idx, kind) in EmotionKind::all().iter().enumerate() {
            emotions.insert(idx as u8, EmotionSignal::new(*kind));
        }
        Self {
            app_id,
            app_name,
            emotions,
            dominant: EmotionKind::Confidence,
            emotional_volatility: 0.0,
            last_evaluation_tick: 0,
            total_evaluations: 0,
        }
    }

    fn signal_mut(&mut self, kind: EmotionKind) -> Option<&mut EmotionSignal> {
        let idx = EmotionKind::all().iter().position(|k| *k == kind)? as u8;
        self.emotions.get_mut(&idx)
    }

    fn signal(&self, kind: EmotionKind) -> Option<&EmotionSignal> {
        let idx = EmotionKind::all().iter().position(|k| *k == kind)? as u8;
        self.emotions.get(&idx)
    }

    fn recompute_dominant(&mut self) {
        let mut best_kind = EmotionKind::Confidence;
        let mut best_intensity = -1.0_f32;
        for (_, sig) in &self.emotions {
            if sig.intensity > best_intensity {
                best_intensity = sig.intensity;
                best_kind = sig.kind;
            }
        }
        self.dominant = best_kind;
    }

    fn recompute_volatility(&mut self) {
        let mut sum_var = 0.0_f32;
        let mut count = 0u32;
        for (_, sig) in &self.emotions {
            sum_var += sig.variance;
            count += 1;
        }
        self.emotional_volatility = if count > 0 { sum_var / count as f32 } else { 0.0 };
    }

    fn decay_all(&mut self) {
        for (_, sig) in self.emotions.iter_mut() {
            sig.decay();
        }
    }
}

// ============================================================================
// STATS
// ============================================================================

/// Aggregate stats for the emotion engine
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EmotionStats {
    pub total_apps_tracked: usize,
    pub total_evaluations: u64,
    pub dominant_landscape_emotion: EmotionKind,
    pub average_volatility: f32,
    pub stress_app_count: usize,
    pub curious_app_count: usize,
    pub frustrated_app_count: usize,
    pub satisfied_app_count: usize,
}

// ============================================================================
// APPS EMOTION ENGINE
// ============================================================================

/// Main engine for computing and tracking emotional signals per application
#[derive(Debug)]
pub struct AppsEmotionEngine {
    profiles: BTreeMap<u64, AppEmotionalProfile>,
    rng_state: u64,
    tick: u64,
    total_evaluations: u64,
    landscape_emotion: EmotionKind,
    landscape_intensities: BTreeMap<u8, f32>,
}

impl AppsEmotionEngine {
    /// Create a new emotion engine
    pub fn new(seed: u64) -> Self {
        let mut landscape_intensities = BTreeMap::new();
        for (idx, _kind) in EmotionKind::all().iter().enumerate() {
            landscape_intensities.insert(idx as u8, 0.0);
        }
        Self {
            profiles: BTreeMap::new(),
            rng_state: if seed == 0 { 0xDEAD_BEEF_CAFE_1234 } else { seed },
            tick: 0,
            total_evaluations: 0,
            landscape_emotion: EmotionKind::Confidence,
            landscape_intensities,
        }
    }

    /// Evaluate the emotional state of a specific application based on raw signals.
    ///
    /// `stress_raw` — thrashing/starvation indicator (0..1)
    /// `confidence_raw` — classification certainty (0..1)
    /// `curiosity_raw` — pattern novelty (0..1)
    /// `satisfaction_raw` — SLA compliance fraction (0..1)
    /// `frustration_raw` — failure recurrence (0..1)
    /// `excitement_raw` — novel positive behavior (0..1)
    pub fn evaluate_app_emotion(
        &mut self,
        app_id: u64,
        app_name: &str,
        stress_raw: f32,
        confidence_raw: f32,
        curiosity_raw: f32,
        satisfaction_raw: f32,
        frustration_raw: f32,
        excitement_raw: f32,
    ) {
        self.tick += 1;
        self.total_evaluations += 1;

        let profile = self
            .profiles
            .entry(app_id)
            .or_insert_with(|| AppEmotionalProfile::new(app_id, String::from(app_name)));

        if self.profiles.len() > MAX_APPS {
            // Evict oldest profile
            let mut oldest_tick = u64::MAX;
            let mut oldest_id = 0u64;
            for (id, p) in &self.profiles {
                if p.last_evaluation_tick < oldest_tick {
                    oldest_tick = p.last_evaluation_tick;
                    oldest_id = *id;
                }
            }
            if oldest_id != app_id {
                self.profiles.remove(&oldest_id);
            }
        }

        // Re-fetch after potential eviction
        if let Some(profile) = self.profiles.get_mut(&app_id) {
            let raws = [
                (EmotionKind::Stress, stress_raw),
                (EmotionKind::Confidence, confidence_raw),
                (EmotionKind::Curiosity, curiosity_raw),
                (EmotionKind::Satisfaction, satisfaction_raw),
                (EmotionKind::Frustration, frustration_raw),
                (EmotionKind::Excitement, excitement_raw),
            ];

            for (kind, raw) in &raws {
                // Add slight noise for exploration
                let noise_val = xorshift64(&mut self.rng_state);
                let noise = ((noise_val % 100) as f32 / 10000.0) - 0.005;
                let clamped = (raw + noise).clamp(0.0, 1.0);

                if let Some(sig) = profile.signal_mut(*kind) {
                    sig.update(clamped, self.tick);
                }
            }

            profile.last_evaluation_tick = self.tick;
            profile.total_evaluations += 1;
            profile.recompute_dominant();
            profile.recompute_volatility();
        }

        self.update_landscape();
    }

    /// Return the dominant emotion for a specific application
    #[inline]
    pub fn dominant_emotion(&self, app_id: u64) -> Option<(EmotionKind, f32)> {
        let profile = self.profiles.get(&app_id)?;
        let sig = profile.signal(profile.dominant)?;
        Some((profile.dominant, sig.intensity))
    }

    /// Return the full emotional profile for an app
    #[inline(always)]
    pub fn emotion_for_app(&self, app_id: u64) -> Option<&AppEmotionalProfile> {
        self.profiles.get(&app_id)
    }

    /// Detect all apps currently experiencing stress above threshold
    pub fn stress_detection(&self) -> Vec<(u64, f32)> {
        let mut stressed = Vec::new();
        for (id, profile) in &self.profiles {
            if let Some(sig) = profile.signal(EmotionKind::Stress) {
                if sig.intensity > STRESS_THRESHOLD {
                    stressed.push((*id, sig.intensity));
                }
            }
        }
        stressed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        stressed
    }

    /// Assess classification confidence across all apps
    #[inline]
    pub fn confidence_assessment(&self) -> Vec<(u64, f32)> {
        let mut result = Vec::new();
        for (id, profile) in &self.profiles {
            if let Some(sig) = profile.signal(EmotionKind::Confidence) {
                result.push((*id, sig.intensity));
            }
        }
        result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        result
    }

    /// Compute the full emotional landscape — aggregated emotion state
    pub fn emotional_landscape(&self) -> EmotionStats {
        let mut total_vol = 0.0_f32;
        let mut stress_count = 0usize;
        let mut curious_count = 0usize;
        let mut frustrated_count = 0usize;
        let mut satisfied_count = 0usize;

        for (_, profile) in &self.profiles {
            total_vol += profile.emotional_volatility;

            if let Some(s) = profile.signal(EmotionKind::Stress) {
                if s.intensity > STRESS_THRESHOLD {
                    stress_count += 1;
                }
            }
            if let Some(s) = profile.signal(EmotionKind::Curiosity) {
                if s.intensity > CURIOSITY_THRESHOLD {
                    curious_count += 1;
                }
            }
            if let Some(s) = profile.signal(EmotionKind::Frustration) {
                if s.intensity > FRUSTRATION_THRESHOLD {
                    frustrated_count += 1;
                }
            }
            if let Some(s) = profile.signal(EmotionKind::Satisfaction) {
                if s.intensity > SATISFACTION_THRESHOLD {
                    satisfied_count += 1;
                }
            }
        }

        let n = self.profiles.len().max(1) as f32;

        EmotionStats {
            total_apps_tracked: self.profiles.len(),
            total_evaluations: self.total_evaluations,
            dominant_landscape_emotion: self.landscape_emotion,
            average_volatility: total_vol / n,
            stress_app_count: stress_count,
            curious_app_count: curious_count,
            frustrated_app_count: frustrated_count,
            satisfied_app_count: satisfied_count,
        }
    }

    // ========================================================================
    // INTERNAL HELPERS
    // ========================================================================

    fn update_landscape(&mut self) {
        // Aggregate intensities across all profiles
        for (idx, _kind) in EmotionKind::all().iter().enumerate() {
            let key = idx as u8;
            let mut sum = 0.0_f32;
            let mut count = 0u32;
            for (_, profile) in &self.profiles {
                if let Some(sig) = profile.emotions.get(&key) {
                    sum += sig.intensity;
                    count += 1;
                }
            }
            let avg = if count > 0 { sum / count as f32 } else { 0.0 };
            self.landscape_intensities.insert(key, avg);
        }

        // Determine dominant landscape emotion
        let mut best_idx = 0u8;
        let mut best_val = -1.0_f32;
        for (idx, val) in &self.landscape_intensities {
            if *val > best_val {
                best_val = *val;
                best_idx = *idx;
            }
        }

        let kinds = EmotionKind::all();
        if (best_idx as usize) < kinds.len() {
            self.landscape_emotion = kinds[best_idx as usize];
        }
    }

    /// Decay all emotion signals (call periodically)
    #[inline]
    pub fn decay_all(&mut self) {
        for (_, profile) in self.profiles.iter_mut() {
            profile.decay_all();
            profile.recompute_dominant();
        }
        self.update_landscape();
    }

    /// Return trend for a specific emotion across all apps
    #[inline]
    pub fn emotion_trend(&self, kind: EmotionKind) -> f32 {
        let mut sum = 0.0_f32;
        let mut count = 0u32;
        for (_, profile) in &self.profiles {
            if let Some(sig) = profile.signal(kind) {
                sum += sig.trend();
                count += 1;
            }
        }
        if count > 0 { sum / count as f32 } else { 0.0 }
    }

    /// Number of apps tracked
    #[inline(always)]
    pub fn app_count(&self) -> usize {
        self.profiles.len()
    }

    /// Current tick
    #[inline(always)]
    pub fn current_tick(&self) -> u64 {
        self.tick
    }
}
