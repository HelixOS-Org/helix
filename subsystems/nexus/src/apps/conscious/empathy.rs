// SPDX-License-Identifier: GPL-2.0
//! # Apps Empathy Engine
//!
//! Understanding applications from the application's "perspective". Rather than
//! treating every process as an opaque resource consumer, the empathy engine
//! models what each application *wants*: low latency, high throughput, large
//! memory footprint, sustained I/O bandwidth, or some weighted combination.
//!
//! By building **empathy profiles** — inferred need vectors with confidence
//! scores — the engine can predict satisfaction and proactively allocate
//! resources *before* an SLA violation occurs. The engine also tracks how
//! well it has predicted each app's needs historically, providing a feedback
//! loop that improves empathy accuracy over time.
//!
//! Key concepts:
//! - **Inferred needs** — weighted vector of (latency, throughput, memory, io)
//! - **Satisfaction model** — how happy is the app given its current allocation?
//! - **Need prediction** — forecast what the app will want next tick
//! - **Happiness score** — composite well-being metric per app

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.13;
const MAX_APPS: usize = 1024;
const MAX_NEED_HISTORY: usize = 128;
const SATISFACTION_HIGH: f32 = 0.8;
const SATISFACTION_LOW: f32 = 0.3;
const PREDICTION_DECAY: f32 = 0.997;
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

/// Xorshift64 PRNG for need prediction perturbation
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

// ============================================================================
// NEED VECTOR
// ============================================================================

/// Weighted need vector representing what an application desires
#[derive(Debug, Clone)]
pub struct NeedVector {
    /// Desire for low latency (0.0–1.0)
    pub latency_need: f32,
    /// Desire for high throughput (0.0–1.0)
    pub throughput_need: f32,
    /// Desire for large memory (0.0–1.0)
    pub memory_need: f32,
    /// Desire for I/O bandwidth (0.0–1.0)
    pub io_need: f32,
    /// Desire for network bandwidth (0.0–1.0)
    pub network_need: f32,
}

impl NeedVector {
    fn zero() -> Self {
        Self {
            latency_need: 0.0,
            throughput_need: 0.0,
            memory_need: 0.0,
            io_need: 0.0,
            network_need: 0.0,
        }
    }

    fn dominant_need(&self) -> &'static str {
        let vals = [
            (self.latency_need, "latency"),
            (self.throughput_need, "throughput"),
            (self.memory_need, "memory"),
            (self.io_need, "io"),
            (self.network_need, "network"),
        ];
        let mut best = vals[0];
        for v in &vals[1..] {
            if v.0 > best.0 {
                best = *v;
            }
        }
        best.1
    }

    fn magnitude(&self) -> f32 {
        let sum = self.latency_need * self.latency_need
            + self.throughput_need * self.throughput_need
            + self.memory_need * self.memory_need
            + self.io_need * self.io_need
            + self.network_need * self.network_need;
        sum.sqrt()
    }

    fn cosine_similarity(&self, other: &NeedVector) -> f32 {
        let dot = self.latency_need * other.latency_need
            + self.throughput_need * other.throughput_need
            + self.memory_need * other.memory_need
            + self.io_need * other.io_need
            + self.network_need * other.network_need;
        let mag_a = self.magnitude();
        let mag_b = other.magnitude();
        if mag_a < 0.001 || mag_b < 0.001 {
            return 0.0;
        }
        dot / (mag_a * mag_b)
    }
}

// ============================================================================
// EMPATHY PROFILE
// ============================================================================

/// Complete empathy profile for a single application
#[derive(Debug, Clone)]
pub struct EmpathyProfile {
    pub app_id: u64,
    pub app_name: String,
    pub inferred_needs: NeedVector,
    pub confidence: f32,
    pub satisfaction: f32,
    pub happiness_score: f32,
    /// Historical satisfaction samples
    satisfaction_history: Vec<f32>,
    sat_write_idx: usize,
    /// Previous predicted needs for accuracy tracking
    prev_prediction: NeedVector,
    pub prediction_accuracy: f32,
    pub observations: u64,
    pub need_changes: u64,
    pub last_tick: u64,
    /// Smoothed need deltas for prediction
    need_delta: NeedVector,
}

impl EmpathyProfile {
    fn new(app_id: u64, app_name: String) -> Self {
        Self {
            app_id,
            app_name,
            inferred_needs: NeedVector::zero(),
            confidence: 0.1,
            satisfaction: 0.5,
            happiness_score: 0.5,
            satisfaction_history: Vec::new(),
            sat_write_idx: 0,
            prev_prediction: NeedVector::zero(),
            prediction_accuracy: 0.5,
            observations: 0,
            need_changes: 0,
            last_tick: 0,
            need_delta: NeedVector::zero(),
        }
    }

    fn update_needs(
        &mut self,
        latency: f32,
        throughput: f32,
        memory: f32,
        io: f32,
        network: f32,
    ) {
        let old = self.inferred_needs.clone();

        self.inferred_needs.latency_need =
            EMA_ALPHA * latency + (1.0 - EMA_ALPHA) * self.inferred_needs.latency_need;
        self.inferred_needs.throughput_need =
            EMA_ALPHA * throughput + (1.0 - EMA_ALPHA) * self.inferred_needs.throughput_need;
        self.inferred_needs.memory_need =
            EMA_ALPHA * memory + (1.0 - EMA_ALPHA) * self.inferred_needs.memory_need;
        self.inferred_needs.io_need =
            EMA_ALPHA * io + (1.0 - EMA_ALPHA) * self.inferred_needs.io_need;
        self.inferred_needs.network_need =
            EMA_ALPHA * network + (1.0 - EMA_ALPHA) * self.inferred_needs.network_need;

        // Track deltas for prediction
        self.need_delta.latency_need = EMA_ALPHA
            * (self.inferred_needs.latency_need - old.latency_need)
            + (1.0 - EMA_ALPHA) * self.need_delta.latency_need;
        self.need_delta.throughput_need = EMA_ALPHA
            * (self.inferred_needs.throughput_need - old.throughput_need)
            + (1.0 - EMA_ALPHA) * self.need_delta.throughput_need;
        self.need_delta.memory_need = EMA_ALPHA
            * (self.inferred_needs.memory_need - old.memory_need)
            + (1.0 - EMA_ALPHA) * self.need_delta.memory_need;
        self.need_delta.io_need = EMA_ALPHA
            * (self.inferred_needs.io_need - old.io_need)
            + (1.0 - EMA_ALPHA) * self.need_delta.io_need;
        self.need_delta.network_need = EMA_ALPHA
            * (self.inferred_needs.network_need - old.network_need)
            + (1.0 - EMA_ALPHA) * self.need_delta.network_need;

        // Detect significant need change
        let sim = old.cosine_similarity(&self.inferred_needs);
        if sim < 0.95 {
            self.need_changes += 1;
        }
    }

    fn update_satisfaction(&mut self, actual_satisfaction: f32) {
        self.satisfaction =
            EMA_ALPHA * actual_satisfaction + (1.0 - EMA_ALPHA) * self.satisfaction;

        if self.satisfaction_history.len() < MAX_NEED_HISTORY {
            self.satisfaction_history.push(actual_satisfaction);
        } else {
            self.satisfaction_history[self.sat_write_idx] = actual_satisfaction;
        }
        self.sat_write_idx = (self.sat_write_idx + 1) % MAX_NEED_HISTORY;

        // Happiness is a composite of satisfaction and confidence
        self.happiness_score = 0.6 * self.satisfaction + 0.4 * self.confidence;
    }

    fn update_confidence(&mut self) {
        // Confidence grows with observations and prediction accuracy
        let obs_factor = 1.0 - 1.0 / (1.0 + self.observations as f32 * 0.05);
        self.confidence =
            EMA_ALPHA * (obs_factor * self.prediction_accuracy)
                + (1.0 - EMA_ALPHA) * self.confidence;
        self.confidence = self.confidence.clamp(0.0, 1.0);
    }

    fn evaluate_prediction_accuracy(&mut self) {
        let sim = self.prev_prediction.cosine_similarity(&self.inferred_needs);
        self.prediction_accuracy =
            EMA_ALPHA * sim + (1.0 - EMA_ALPHA) * self.prediction_accuracy;
    }

    fn predict_next_needs(&mut self) -> NeedVector {
        let predicted = NeedVector {
            latency_need: (self.inferred_needs.latency_need
                + self.need_delta.latency_need)
                .clamp(0.0, 1.0),
            throughput_need: (self.inferred_needs.throughput_need
                + self.need_delta.throughput_need)
                .clamp(0.0, 1.0),
            memory_need: (self.inferred_needs.memory_need
                + self.need_delta.memory_need)
                .clamp(0.0, 1.0),
            io_need: (self.inferred_needs.io_need + self.need_delta.io_need)
                .clamp(0.0, 1.0),
            network_need: (self.inferred_needs.network_need
                + self.need_delta.network_need)
                .clamp(0.0, 1.0),
        };
        self.prev_prediction = predicted.clone();
        predicted
    }

    fn satisfaction_trend(&self) -> f32 {
        if self.satisfaction_history.len() < 4 {
            return 0.0;
        }
        let len = self.satisfaction_history.len();
        let mid = len / 2;
        let first: f32 =
            self.satisfaction_history[..mid].iter().sum::<f32>() / mid as f32;
        let second: f32 =
            self.satisfaction_history[mid..].iter().sum::<f32>() / (len - mid) as f32;
        second - first
    }
}

// ============================================================================
// STATS
// ============================================================================

/// Aggregate empathy engine statistics
#[derive(Debug, Clone)]
pub struct EmpathyStats {
    pub total_apps: usize,
    pub mean_satisfaction: f32,
    pub mean_confidence: f32,
    pub mean_happiness: f32,
    pub happy_app_count: usize,
    pub unhappy_app_count: usize,
    pub mean_prediction_accuracy: f32,
    pub total_observations: u64,
}

// ============================================================================
// APPS EMPATHY ENGINE
// ============================================================================

/// Engine that models application needs from the app's perspective
#[derive(Debug)]
pub struct AppsEmpathyEngine {
    profiles: BTreeMap<u64, EmpathyProfile>,
    tick: u64,
    total_observations: u64,
    rng_state: u64,
}

impl AppsEmpathyEngine {
    pub fn new(seed: u64) -> Self {
        Self {
            profiles: BTreeMap::new(),
            tick: 0,
            total_observations: 0,
            rng_state: if seed == 0 { 0xE3AA_CAFE_1234_0000 } else { seed },
        }
    }

    /// Empathize with an application — update its need vector and satisfaction
    pub fn empathize_with_app(
        &mut self,
        app_id: u64,
        app_name: &str,
        latency_signal: f32,
        throughput_signal: f32,
        memory_signal: f32,
        io_signal: f32,
        network_signal: f32,
        actual_satisfaction: f32,
    ) {
        self.tick += 1;
        self.total_observations += 1;

        let profile = self
            .profiles
            .entry(app_id)
            .or_insert_with(|| EmpathyProfile::new(app_id, String::from(app_name)));

        // Evaluate prediction accuracy before updating needs
        profile.evaluate_prediction_accuracy();

        profile.update_needs(
            latency_signal,
            throughput_signal,
            memory_signal,
            io_signal,
            network_signal,
        );
        profile.update_satisfaction(actual_satisfaction);
        profile.observations += 1;
        profile.last_tick = self.tick;
        profile.update_confidence();

        // Evict if over capacity
        if self.profiles.len() > MAX_APPS {
            self.evict_least_observed(app_id);
        }
    }

    /// Infer what an app needs right now
    pub fn infer_app_needs(&self, app_id: u64) -> Option<&NeedVector> {
        self.profiles.get(&app_id).map(|p| &p.inferred_needs)
    }

    /// Get the satisfaction model for an app
    pub fn satisfaction_model(&self, app_id: u64) -> Option<(f32, f32)> {
        let profile = self.profiles.get(&app_id)?;
        Some((profile.satisfaction, profile.satisfaction_trend()))
    }

    /// Predict what an app will need next tick
    pub fn need_prediction(&mut self, app_id: u64) -> Option<NeedVector> {
        let profile = self.profiles.get_mut(&app_id)?;
        Some(profile.predict_next_needs())
    }

    /// How accurate have our empathy predictions been for this app?
    pub fn empathy_accuracy(&self, app_id: u64) -> Option<f32> {
        self.profiles.get(&app_id).map(|p| p.prediction_accuracy)
    }

    /// Compute a happiness score for an app
    pub fn app_happiness_score(&self, app_id: u64) -> Option<f32> {
        self.profiles.get(&app_id).map(|p| p.happiness_score)
    }

    /// Get the full empathy profile
    pub fn profile(&self, app_id: u64) -> Option<&EmpathyProfile> {
        self.profiles.get(&app_id)
    }

    /// List all unhappy apps (satisfaction below threshold)
    pub fn unhappy_apps(&self) -> Vec<(u64, f32)> {
        let mut result = Vec::new();
        for (id, profile) in &self.profiles {
            if profile.satisfaction < SATISFACTION_LOW {
                result.push((*id, profile.satisfaction));
            }
        }
        result.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal));
        result
    }

    /// List all happy apps
    pub fn happy_apps(&self) -> Vec<(u64, f32)> {
        let mut result = Vec::new();
        for (id, profile) in &self.profiles {
            if profile.satisfaction > SATISFACTION_HIGH {
                result.push((*id, profile.satisfaction));
            }
        }
        result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        result
    }

    /// Dominant need across all apps
    pub fn dominant_system_need(&self) -> &'static str {
        let mut lat = 0.0_f32;
        let mut thr = 0.0_f32;
        let mut mem = 0.0_f32;
        let mut io = 0.0_f32;
        let mut net = 0.0_f32;
        let n = self.profiles.len().max(1) as f32;

        for (_, p) in &self.profiles {
            lat += p.inferred_needs.latency_need;
            thr += p.inferred_needs.throughput_need;
            mem += p.inferred_needs.memory_need;
            io += p.inferred_needs.io_need;
            net += p.inferred_needs.network_need;
        }

        let agg = NeedVector {
            latency_need: lat / n,
            throughput_need: thr / n,
            memory_need: mem / n,
            io_need: io / n,
            network_need: net / n,
        };
        agg.dominant_need()
    }

    /// Full stats
    pub fn stats(&self) -> EmpathyStats {
        let n = self.profiles.len().max(1) as f32;
        let mut sat_sum = 0.0_f32;
        let mut conf_sum = 0.0_f32;
        let mut hap_sum = 0.0_f32;
        let mut pred_sum = 0.0_f32;
        let mut happy = 0usize;
        let mut unhappy = 0usize;

        for (_, p) in &self.profiles {
            sat_sum += p.satisfaction;
            conf_sum += p.confidence;
            hap_sum += p.happiness_score;
            pred_sum += p.prediction_accuracy;
            if p.satisfaction > SATISFACTION_HIGH {
                happy += 1;
            }
            if p.satisfaction < SATISFACTION_LOW {
                unhappy += 1;
            }
        }

        EmpathyStats {
            total_apps: self.profiles.len(),
            mean_satisfaction: sat_sum / n,
            mean_confidence: conf_sum / n,
            mean_happiness: hap_sum / n,
            happy_app_count: happy,
            unhappy_app_count: unhappy,
            mean_prediction_accuracy: pred_sum / n,
            total_observations: self.total_observations,
        }
    }

    /// Decay satisfaction over time
    pub fn decay(&mut self) {
        for (_, p) in self.profiles.iter_mut() {
            p.satisfaction *= PREDICTION_DECAY;
            p.confidence *= PREDICTION_DECAY;
            p.happiness_score = 0.6 * p.satisfaction + 0.4 * p.confidence;
        }
    }

    // ========================================================================
    // INTERNAL
    // ========================================================================

    fn evict_least_observed(&mut self, keep_id: u64) {
        let mut min_obs = u64::MAX;
        let mut min_id = 0u64;
        for (id, p) in &self.profiles {
            if *id != keep_id && p.observations < min_obs {
                min_obs = p.observations;
                min_id = *id;
            }
        }
        if min_id != 0 {
            self.profiles.remove(&min_id);
        }
    }
}
