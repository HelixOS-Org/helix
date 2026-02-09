// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Emotion Engine
//!
//! Emotional signals tuned for the cooperation protocol perspective. These
//! quantitative emotions drive policy decisions across inter-process resource
//! sharing, trust maintenance, and fairness enforcement. Each emotion is
//! EMA-smoothed, decays over time, and influences downstream cooperation
//! decisions.
//!
//! ## Emotion Categories
//!
//! - **TrustAnxiety** — Trust between processes is degrading
//! - **CooperationJoy** — Successful resource sharing completed
//! - **FairnessAnger** — Unfair allocation detected across participants
//! - **SolidarityPride** — All processes cooperating harmoniously
//! - **IsolationSadness** — A process is isolated from cooperation
//! - **CompetitionExcitement** — Healthy competition driving improvement
//!
//! The dominant emotion for the cooperation landscape shapes mediation
//! policy, trust recalibration, and allocation bias.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.15;
const EMOTION_DECAY: f32 = 0.993;
const MAX_PROCESSES: usize = 512;
const MAX_EMOTION_HISTORY: usize = 128;
const TRUST_ANXIETY_THRESHOLD: f32 = 0.6;
const JOY_THRESHOLD: f32 = 0.7;
const ANGER_THRESHOLD: f32 = 0.65;
const SOLIDARITY_THRESHOLD: f32 = 0.8;
const ISOLATION_THRESHOLD: f32 = 0.55;
const EXCITEMENT_THRESHOLD: f32 = 0.6;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const POLICY_INFLUENCE_SCALE: f32 = 0.35;
const CLIMATE_WINDOW: usize = 32;

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

/// Xorshift64 PRNG for stochastic decay jitter
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

// ============================================================================
// COOPERATION EMOTION KINDS
// ============================================================================

/// Discrete emotional signal categories for cooperation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CoopEmotionKind {
    /// Trust between cooperating processes is degrading
    TrustAnxiety,
    /// Successful resource sharing completed
    CooperationJoy,
    /// Unfair allocation detected across participants
    FairnessAnger,
    /// All processes cooperating harmoniously
    SolidarityPride,
    /// A process is isolated from cooperation
    IsolationSadness,
    /// Healthy competition driving improvement
    CompetitionExcitement,
}

impl CoopEmotionKind {
    /// Return all variants for iteration
    #[inline]
    pub fn all() -> &'static [CoopEmotionKind] {
        &[
            CoopEmotionKind::TrustAnxiety,
            CoopEmotionKind::CooperationJoy,
            CoopEmotionKind::FairnessAnger,
            CoopEmotionKind::SolidarityPride,
            CoopEmotionKind::IsolationSadness,
            CoopEmotionKind::CompetitionExcitement,
        ]
    }

    /// Threshold above which this emotion is considered active
    #[inline]
    pub fn activation_threshold(&self) -> f32 {
        match self {
            CoopEmotionKind::TrustAnxiety => TRUST_ANXIETY_THRESHOLD,
            CoopEmotionKind::CooperationJoy => JOY_THRESHOLD,
            CoopEmotionKind::FairnessAnger => ANGER_THRESHOLD,
            CoopEmotionKind::SolidarityPride => SOLIDARITY_THRESHOLD,
            CoopEmotionKind::IsolationSadness => ISOLATION_THRESHOLD,
            CoopEmotionKind::CompetitionExcitement => EXCITEMENT_THRESHOLD,
        }
    }

    /// Weight this emotion has on policy influence
    #[inline]
    pub fn policy_weight(&self) -> f32 {
        match self {
            CoopEmotionKind::TrustAnxiety => 0.9,
            CoopEmotionKind::CooperationJoy => 0.5,
            CoopEmotionKind::FairnessAnger => 1.0,
            CoopEmotionKind::SolidarityPride => 0.4,
            CoopEmotionKind::IsolationSadness => 0.7,
            CoopEmotionKind::CompetitionExcitement => 0.3,
        }
    }
}

// ============================================================================
// EMOTION SIGNAL
// ============================================================================

/// A single cooperation emotion signal with EMA smoothing
#[derive(Debug, Clone)]
pub struct CoopEmotionSignal {
    pub kind: CoopEmotionKind,
    /// EMA-smoothed intensity (0.0 – 1.0)
    pub intensity: f32,
    /// Raw intensity before smoothing
    pub raw_intensity: f32,
    /// Number of times this emotion was triggered
    pub trigger_count: u64,
    /// Tick of last trigger
    pub last_trigger_tick: u64,
    /// Ring buffer of historical intensities
    history: Vec<f32>,
    write_idx: usize,
    /// Peak intensity ever observed
    pub peak_intensity: f32,
    /// Variance accumulator for confidence intervals
    pub variance_accum: f32,
}

impl CoopEmotionSignal {
    pub fn new(kind: CoopEmotionKind) -> Self {
        let mut history = Vec::with_capacity(MAX_EMOTION_HISTORY);
        for _ in 0..MAX_EMOTION_HISTORY {
            history.push(0.0);
        }
        Self {
            kind,
            intensity: 0.0,
            raw_intensity: 0.0,
            trigger_count: 0,
            last_trigger_tick: 0,
            history,
            write_idx: 0,
            peak_intensity: 0.0,
            variance_accum: 0.0,
        }
    }

    /// Record a new raw intensity observation and update EMA
    #[inline]
    pub fn observe(&mut self, raw: f32, tick: u64) {
        let clamped = if raw < 0.0 { 0.0 } else if raw > 1.0 { 1.0 } else { raw };
        self.raw_intensity = clamped;
        let delta = clamped - self.intensity;
        self.intensity += EMA_ALPHA * delta;
        self.variance_accum += EMA_ALPHA * (delta * delta - self.variance_accum);
        if self.intensity > self.peak_intensity {
            self.peak_intensity = self.intensity;
        }
        self.history[self.write_idx] = clamped;
        self.write_idx = (self.write_idx + 1) % MAX_EMOTION_HISTORY;
        self.trigger_count += 1;
        self.last_trigger_tick = tick;
    }

    /// Apply time-based decay with optional jitter
    #[inline]
    pub fn decay(&mut self, rng: &mut u64) {
        let jitter_raw = xorshift64(rng);
        let jitter = (jitter_raw % 100) as f32 / 100_000.0;
        self.intensity *= EMOTION_DECAY - jitter;
        if self.intensity < 0.001 {
            self.intensity = 0.0;
        }
    }

    /// Whether this emotion is currently above its activation threshold
    #[inline(always)]
    pub fn is_active(&self) -> bool {
        self.intensity >= self.kind.activation_threshold()
    }

    /// Standard deviation estimate from variance accumulator
    pub fn std_dev(&self) -> f32 {
        let v = if self.variance_accum < 0.0 { 0.0 } else { self.variance_accum };
        let mut x = v;
        if x < 0.0001 {
            return 0.0;
        }
        // Newton's method sqrt
        let mut guess = x / 2.0;
        for _ in 0..8 {
            guess = (guess + x / guess) / 2.0;
        }
        guess
    }

    /// Average intensity from history ring buffer
    pub fn history_average(&self) -> f32 {
        let count = if self.trigger_count < MAX_EMOTION_HISTORY as u64 {
            self.trigger_count as usize
        } else {
            MAX_EMOTION_HISTORY
        };
        if count == 0 {
            return 0.0;
        }
        let mut sum = 0.0f32;
        for i in 0..count {
            sum += self.history[i];
        }
        sum / count as f32
    }
}

// ============================================================================
// PROCESS EMOTION PROFILE
// ============================================================================

/// Emotion profile for a single cooperating process
#[derive(Debug, Clone)]
pub struct ProcessEmotionProfile {
    pub process_id: u64,
    pub signals: BTreeMap<u8, CoopEmotionSignal>,
    pub dominant: CoopEmotionKind,
    pub last_evaluation_tick: u64,
    pub evaluation_count: u64,
}

impl ProcessEmotionProfile {
    pub fn new(process_id: u64) -> Self {
        let mut signals = BTreeMap::new();
        for kind in CoopEmotionKind::all() {
            signals.insert(*kind as u8, CoopEmotionSignal::new(*kind));
        }
        Self {
            process_id,
            signals,
            dominant: CoopEmotionKind::CooperationJoy,
            last_evaluation_tick: 0,
            evaluation_count: 0,
        }
    }

    /// Update a specific emotion for this process
    #[inline]
    pub fn update_emotion(&mut self, kind: CoopEmotionKind, raw: f32, tick: u64) {
        if let Some(signal) = self.signals.get_mut(&(kind as u8)) {
            signal.observe(raw, tick);
        }
    }

    /// Find the dominant emotion (highest intensity)
    pub fn compute_dominant(&mut self) -> CoopEmotionKind {
        let mut best_kind = CoopEmotionKind::CooperationJoy;
        let mut best_val = -1.0f32;
        for (_, signal) in self.signals.iter() {
            if signal.intensity > best_val {
                best_val = signal.intensity;
                best_kind = signal.kind;
            }
        }
        self.dominant = best_kind;
        best_kind
    }
}

// ============================================================================
// COOPERATION EMOTION STATS
// ============================================================================

/// Aggregate statistics for the cooperation emotion landscape
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CoopEmotionStats {
    /// Total evaluations performed
    pub total_evaluations: u64,
    /// Number of processes tracked
    pub tracked_processes: usize,
    /// Global dominant emotion across all processes
    pub global_dominant: CoopEmotionKind,
    /// Average trust anxiety across all processes
    pub avg_trust_anxiety: f32,
    /// Average cooperation joy across all processes
    pub avg_cooperation_joy: f32,
    /// Average fairness anger
    pub avg_fairness_anger: f32,
    /// Average solidarity pride
    pub avg_solidarity_pride: f32,
    /// Overall emotional climate score (positive = cooperative, negative = contentious)
    pub climate_score: f32,
    /// Policy influence strength
    pub policy_influence: f32,
    /// Count of emotionally active processes
    pub active_emotion_count: usize,
}

impl CoopEmotionStats {
    pub fn new() -> Self {
        Self {
            total_evaluations: 0,
            tracked_processes: 0,
            global_dominant: CoopEmotionKind::CooperationJoy,
            avg_trust_anxiety: 0.0,
            avg_cooperation_joy: 0.0,
            avg_fairness_anger: 0.0,
            avg_solidarity_pride: 0.0,
            climate_score: 0.0,
            policy_influence: 0.0,
            active_emotion_count: 0,
        }
    }
}

// ============================================================================
// COOPERATION EMOTION ENGINE
// ============================================================================

/// Core engine managing emotional signals across all cooperating processes
pub struct CoopEmotionEngine {
    /// Per-process emotion profiles
    profiles: BTreeMap<u64, ProcessEmotionProfile>,
    /// Global emotion aggregates (EMA-smoothed)
    global_emotions: BTreeMap<u8, f32>,
    /// Climate history ring buffer
    climate_history: Vec<f32>,
    climate_write_idx: usize,
    /// Running statistics
    pub stats: CoopEmotionStats,
    /// PRNG state for decay jitter
    rng_state: u64,
    /// Current tick
    tick: u64,
    /// EMA-smoothed policy influence
    policy_ema: f32,
}

impl CoopEmotionEngine {
    pub fn new(seed: u64) -> Self {
        let mut global_emotions = BTreeMap::new();
        for kind in CoopEmotionKind::all() {
            global_emotions.insert(*kind as u8, 0.0f32);
        }
        let mut climate_history = Vec::with_capacity(CLIMATE_WINDOW);
        for _ in 0..CLIMATE_WINDOW {
            climate_history.push(0.0);
        }
        Self {
            profiles: BTreeMap::new(),
            global_emotions,
            climate_history,
            climate_write_idx: 0,
            stats: CoopEmotionStats::new(),
            rng_state: seed | 1,
            tick: 0,
            policy_ema: 0.0,
        }
    }

    // ========================================================================
    // CORE EVALUATION
    // ========================================================================

    /// Evaluate cooperation emotions for a specific process
    ///
    /// Takes trust_delta (negative = degrading), sharing_success ratio,
    /// fairness_score (1.0 = perfectly fair), isolation flag, and
    /// competition_health score.
    pub fn evaluate_coop_emotion(
        &mut self,
        process_id: u64,
        trust_delta: f32,
        sharing_success: f32,
        fairness_score: f32,
        is_isolated: bool,
        competition_health: f32,
    ) {
        self.tick += 1;

        // Ensure profile exists
        if !self.profiles.contains_key(&process_id) {
            if self.profiles.len() >= MAX_PROCESSES {
                return;
            }
            self.profiles.insert(process_id, ProcessEmotionProfile::new(process_id));
        }

        let tick = self.tick;
        if let Some(profile) = self.profiles.get_mut(&process_id) {
            // TrustAnxiety: rises when trust is degrading
            let anxiety = if trust_delta < 0.0 {
                (-trust_delta).min(1.0)
            } else {
                0.0
            };
            profile.update_emotion(CoopEmotionKind::TrustAnxiety, anxiety, tick);

            // CooperationJoy: rises with successful sharing
            profile.update_emotion(CoopEmotionKind::CooperationJoy, sharing_success, tick);

            // FairnessAnger: rises when allocation is unfair
            let anger = (1.0 - fairness_score).max(0.0).min(1.0);
            profile.update_emotion(CoopEmotionKind::FairnessAnger, anger, tick);

            // SolidarityPride: rises when fairness and sharing are both high
            let solidarity = (fairness_score * sharing_success).min(1.0);
            profile.update_emotion(CoopEmotionKind::SolidarityPride, solidarity, tick);

            // IsolationSadness: binary from isolation flag with decay
            let isolation_val = if is_isolated { 0.9 } else { 0.0 };
            profile.update_emotion(CoopEmotionKind::IsolationSadness, isolation_val, tick);

            // CompetitionExcitement: healthy competition
            profile.update_emotion(CoopEmotionKind::CompetitionExcitement, competition_health, tick);

            profile.compute_dominant();
            profile.last_evaluation_tick = tick;
            profile.evaluation_count += 1;
        }

        self.stats.total_evaluations += 1;
        self.update_global_aggregates();
    }

    /// Update global emotion aggregates from all process profiles
    #[inline]
    fn update_global_aggregates(&mut self) {
        let count = self.profiles.len();
        if count == 0 {
            return;
        }
        let inv = 1.0 / count as f32;

        let mut sums: BTreeMap<u8, f32> = BTreeMap::new();
        for kind in CoopEmotionKind::all() {
            sums.insert(*kind as u8, 0.0);
        }

        let mut active_count = 0usize;
        for (_, profile) in self.profiles.iter() {
            for (key, signal) in profile.signals.iter() {
                if let Some(s) = sums.get_mut(key) {
                    *s += signal.intensity;
                }
                if signal.is_active() {
                    active_count += 1;
                }
            }
        }

        for (key, sum) in sums.iter() {
            let avg = *sum * inv;
            if let Some(g) = self.global_emotions.get_mut(key) {
                *g += EMA_ALPHA * (avg - *g);
            }
        }

        // Update stats
        self.stats.tracked_processes = count;
        self.stats.active_emotion_count = active_count;
        self.stats.avg_trust_anxiety = *self.global_emotions.get(&(CoopEmotionKind::TrustAnxiety as u8)).unwrap_or(&0.0);
        self.stats.avg_cooperation_joy = *self.global_emotions.get(&(CoopEmotionKind::CooperationJoy as u8)).unwrap_or(&0.0);
        self.stats.avg_fairness_anger = *self.global_emotions.get(&(CoopEmotionKind::FairnessAnger as u8)).unwrap_or(&0.0);
        self.stats.avg_solidarity_pride = *self.global_emotions.get(&(CoopEmotionKind::SolidarityPride as u8)).unwrap_or(&0.0);
    }

    // ========================================================================
    // DOMINANT EMOTION
    // ========================================================================

    /// Determine the globally dominant cooperation emotion
    pub fn dominant_emotion(&mut self) -> CoopEmotionKind {
        let mut best = CoopEmotionKind::CooperationJoy;
        let mut best_val = -1.0f32;
        for (key, val) in self.global_emotions.iter() {
            if *val > best_val {
                best_val = *val;
                // Reconstruct kind from key
                best = match *key {
                    0 => CoopEmotionKind::TrustAnxiety,
                    1 => CoopEmotionKind::CooperationJoy,
                    2 => CoopEmotionKind::FairnessAnger,
                    3 => CoopEmotionKind::SolidarityPride,
                    4 => CoopEmotionKind::IsolationSadness,
                    5 => CoopEmotionKind::CompetitionExcitement,
                    _ => CoopEmotionKind::CooperationJoy,
                };
            }
        }
        self.stats.global_dominant = best;
        best
    }

    // ========================================================================
    // TRUST ANXIETY
    // ========================================================================

    /// Get the trust anxiety level for a specific process
    #[inline]
    pub fn trust_anxiety_level(&self, process_id: u64) -> f32 {
        if let Some(profile) = self.profiles.get(&process_id) {
            if let Some(signal) = profile.signals.get(&(CoopEmotionKind::TrustAnxiety as u8)) {
                return signal.intensity;
            }
        }
        0.0
    }

    /// Global average trust anxiety
    #[inline(always)]
    pub fn global_trust_anxiety(&self) -> f32 {
        self.stats.avg_trust_anxiety
    }

    /// Processes with trust anxiety above threshold
    #[inline]
    pub fn anxious_processes(&self) -> Vec<u64> {
        let mut result = Vec::new();
        for (pid, profile) in self.profiles.iter() {
            if let Some(signal) = profile.signals.get(&(CoopEmotionKind::TrustAnxiety as u8)) {
                if signal.intensity >= TRUST_ANXIETY_THRESHOLD {
                    result.push(*pid);
                }
            }
        }
        result
    }

    // ========================================================================
    // COOPERATION JOY
    // ========================================================================

    /// Get cooperation joy for a process — how well sharing is going
    #[inline]
    pub fn cooperation_joy(&self, process_id: u64) -> f32 {
        if let Some(profile) = self.profiles.get(&process_id) {
            if let Some(signal) = profile.signals.get(&(CoopEmotionKind::CooperationJoy as u8)) {
                return signal.intensity;
            }
        }
        0.0
    }

    /// Average joy across all cooperating processes
    #[inline(always)]
    pub fn global_cooperation_joy(&self) -> f32 {
        self.stats.avg_cooperation_joy
    }

    /// Processes experiencing high cooperation joy
    #[inline]
    pub fn joyful_processes(&self) -> Vec<u64> {
        let mut result = Vec::new();
        for (pid, profile) in self.profiles.iter() {
            if let Some(signal) = profile.signals.get(&(CoopEmotionKind::CooperationJoy as u8)) {
                if signal.intensity >= JOY_THRESHOLD {
                    result.push(*pid);
                }
            }
        }
        result
    }

    // ========================================================================
    // EMOTIONAL CLIMATE
    // ========================================================================

    /// Compute the overall emotional climate of cooperation
    ///
    /// Positive score = cooperative, harmonious
    /// Negative score = contentious, anxious
    #[inline]
    pub fn emotional_climate(&mut self) -> f32 {
        let joy = self.stats.avg_cooperation_joy;
        let solidarity = self.stats.avg_solidarity_pride;
        let anxiety = self.stats.avg_trust_anxiety;
        let anger = self.stats.avg_fairness_anger;
        let isolation = *self.global_emotions.get(&(CoopEmotionKind::IsolationSadness as u8)).unwrap_or(&0.0);

        let positive = joy * 0.4 + solidarity * 0.35 + {
            let excitement = *self.global_emotions.get(&(CoopEmotionKind::CompetitionExcitement as u8)).unwrap_or(&0.0);
            excitement * 0.25
        };
        let negative = anxiety * 0.35 + anger * 0.4 + isolation * 0.25;

        let climate = positive - negative;
        let clamped = if climate < -1.0 { -1.0 } else if climate > 1.0 { 1.0 } else { climate };

        self.climate_history[self.climate_write_idx] = clamped;
        self.climate_write_idx = (self.climate_write_idx + 1) % CLIMATE_WINDOW;

        // EMA smooth the climate score
        self.stats.climate_score += EMA_ALPHA * (clamped - self.stats.climate_score);
        self.stats.climate_score
    }

    /// Climate trend: average of recent climate samples
    #[inline]
    pub fn climate_trend(&self) -> f32 {
        let mut sum = 0.0f32;
        let mut count = 0usize;
        for val in self.climate_history.iter() {
            if *val != 0.0 || count < self.climate_write_idx {
                sum += *val;
                count += 1;
            }
        }
        if count == 0 { 0.0 } else { sum / count as f32 }
    }

    // ========================================================================
    // POLICY INFLUENCE
    // ========================================================================

    /// Compute how strongly emotions should influence cooperation policy
    ///
    /// High anxiety + high anger = strong policy override
    /// High joy + high solidarity = relax policy constraints
    #[inline]
    pub fn emotion_influence_on_policy(&mut self) -> f32 {
        let urgency_emotions = self.stats.avg_trust_anxiety * 0.4
            + self.stats.avg_fairness_anger * 0.4
            + {
                let iso = *self.global_emotions.get(&(CoopEmotionKind::IsolationSadness as u8)).unwrap_or(&0.0);
                iso * 0.2
            };

        let calming_emotions = self.stats.avg_cooperation_joy * 0.5
            + self.stats.avg_solidarity_pride * 0.5;

        let raw_influence = (urgency_emotions - calming_emotions * 0.5) * POLICY_INFLUENCE_SCALE;
        let clamped = if raw_influence < 0.0 { 0.0 } else if raw_influence > 1.0 { 1.0 } else { raw_influence };

        self.policy_ema += EMA_ALPHA * (clamped - self.policy_ema);
        self.stats.policy_influence = self.policy_ema;
        self.policy_ema
    }

    /// Recommend policy action based on emotional state
    pub fn policy_recommendation(&self) -> CoopPolicyAction {
        if self.stats.avg_fairness_anger > ANGER_THRESHOLD {
            return CoopPolicyAction::EnforceFairness;
        }
        if self.stats.avg_trust_anxiety > TRUST_ANXIETY_THRESHOLD {
            return CoopPolicyAction::RebuildTrust;
        }
        let iso = *self.global_emotions.get(&(CoopEmotionKind::IsolationSadness as u8)).unwrap_or(&0.0);
        if iso > ISOLATION_THRESHOLD {
            return CoopPolicyAction::ReintegrateIsolated;
        }
        if self.stats.avg_cooperation_joy > JOY_THRESHOLD && self.stats.avg_solidarity_pride > SOLIDARITY_THRESHOLD {
            return CoopPolicyAction::RelaxConstraints;
        }
        CoopPolicyAction::Maintain
    }

    // ========================================================================
    // DECAY & MAINTENANCE
    // ========================================================================

    /// Apply decay to all emotion signals across all processes
    #[inline]
    pub fn decay_all(&mut self) {
        let rng = &mut self.rng_state;
        for (_, profile) in self.profiles.iter_mut() {
            for (_, signal) in profile.signals.iter_mut() {
                signal.decay(rng);
            }
        }
    }

    /// Remove profiles for processes that haven't been evaluated recently
    pub fn prune_stale(&mut self, max_age_ticks: u64) {
        let cutoff = if self.tick > max_age_ticks {
            self.tick - max_age_ticks
        } else {
            0
        };
        let stale_keys: Vec<u64> = self.profiles.iter()
            .filter(|(_, p)| p.last_evaluation_tick < cutoff)
            .map(|(k, _)| *k)
            .collect();
        for key in stale_keys {
            self.profiles.remove(&key);
        }
    }

    // ========================================================================
    // QUERIES
    // ========================================================================

    /// Get the complete emotion profile for a process
    #[inline(always)]
    pub fn process_profile(&self, process_id: u64) -> Option<&ProcessEmotionProfile> {
        self.profiles.get(&process_id)
    }

    /// Number of tracked processes
    #[inline(always)]
    pub fn process_count(&self) -> usize {
        self.profiles.len()
    }

    /// Snapshot of current statistics
    #[inline(always)]
    pub fn snapshot_stats(&self) -> CoopEmotionStats {
        self.stats.clone()
    }

    /// Per-emotion global intensities
    #[inline]
    pub fn global_intensities(&self) -> Vec<(CoopEmotionKind, f32)> {
        let mut result = Vec::new();
        for kind in CoopEmotionKind::all() {
            let val = *self.global_emotions.get(&(*kind as u8)).unwrap_or(&0.0);
            result.push((*kind, val));
        }
        result
    }

    /// Hash the current emotional state for fingerprinting
    pub fn state_fingerprint(&self) -> u64 {
        let mut buf = Vec::new();
        for (key, val) in self.global_emotions.iter() {
            buf.push(*key);
            let bits = val.to_bits();
            buf.push((bits >> 24) as u8);
            buf.push((bits >> 16) as u8);
            buf.push((bits >> 8) as u8);
            buf.push(bits as u8);
        }
        fnv1a_hash(&buf)
    }
}

// ============================================================================
// POLICY ACTION
// ============================================================================

/// Recommended policy action based on emotional landscape
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopPolicyAction {
    /// Enforce fairness constraints more strictly
    EnforceFairness,
    /// Actively rebuild trust between processes
    RebuildTrust,
    /// Reintegrate isolated processes into cooperation
    ReintegrateIsolated,
    /// Relax constraints — cooperation is healthy
    RelaxConstraints,
    /// Maintain current policy
    Maintain,
}
