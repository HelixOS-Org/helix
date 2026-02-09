// SPDX-License-Identifier: GPL-2.0
//! # Apps Qualia Engine
//!
//! Subjective experience of application management. Qualia — in philosophy of
//! mind — refers to the subjective, conscious experience of perception. Here,
//! we adapt the concept computationally: the qualia engine synthesizes a
//! holistic "experiential state" for the apps management subsystem.
//!
//! Rather than exposing raw metrics, the engine answers higher-order questions:
//! - How harmonious is the current workload mix?
//! - How clear are our classifications?
//! - How confident are our predictions?
//! - Is the apps engine in a state of "flow" — smooth, balanced management?
//!
//! The `QualiaState` struct captures these experiential dimensions as
//! EMA-smoothed values. The engine can report a subjective quality score
//! and generate narrative-style reports about the system's "feeling".
//!
//! This is the introspective capstone — the engine observing its own
//! operational quality from a phenomenological perspective.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.10;
const MAX_HISTORY: usize = 256;
const FLOW_THRESHOLD: f32 = 0.7;
const DISSONANCE_THRESHOLD: f32 = 0.3;
const CLARITY_HIGH: f32 = 0.8;
const CLARITY_LOW: f32 = 0.4;
const MAX_DIMENSION_ENTRIES: usize = 64;
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

/// Xorshift64 PRNG for experiential noise
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

// ============================================================================
// QUALIA STATE
// ============================================================================

/// The subjective experiential state of the apps engine
#[derive(Debug, Clone)]
pub struct QualiaState {
    /// How harmoniously apps coexist (0.0 = chaotic, 1.0 = perfect harmony)
    pub workload_harmony: f32,
    /// How clear our classifications are (0.0 = confused, 1.0 = crystal clear)
    pub classification_clarity: f32,
    /// How confident our predictions are (0.0 = guessing, 1.0 = certain)
    pub prediction_confidence: f32,
    /// Overall flow state (0.0 = struggling, 1.0 = effortless management)
    pub flow_state: f32,
    /// Cognitive load — how hard is the engine working? (0.0 = idle, 1.0 = overloaded)
    pub cognitive_load: f32,
    /// Dissonance — contradiction between expectations and reality
    pub dissonance: f32,
    /// Overall experiential quality (composite)
    pub experience_quality: f32,
}

impl QualiaState {
    fn new() -> Self {
        Self {
            workload_harmony: 0.5,
            classification_clarity: 0.5,
            prediction_confidence: 0.5,
            flow_state: 0.5,
            cognitive_load: 0.3,
            dissonance: 0.1,
            experience_quality: 0.5,
        }
    }

    fn recompute_quality(&mut self) {
        // Quality is a weighted composite with dissonance and cognitive load as penalties
        let positive = 0.25 * self.workload_harmony
            + 0.25 * self.classification_clarity
            + 0.25 * self.prediction_confidence
            + 0.25 * self.flow_state;
        let penalty = 0.3 * self.dissonance + 0.2 * self.cognitive_load;
        self.experience_quality = (positive - penalty).clamp(0.0, 1.0);
    }

    fn is_in_flow(&self) -> bool {
        self.flow_state > FLOW_THRESHOLD && self.dissonance < DISSONANCE_THRESHOLD
    }

    fn experiential_label(&self) -> &'static str {
        if self.experience_quality > 0.8 {
            "transcendent"
        } else if self.experience_quality > 0.6 {
            "flowing"
        } else if self.experience_quality > 0.4 {
            "adequate"
        } else if self.experience_quality > 0.2 {
            "struggling"
        } else {
            "overwhelmed"
        }
    }
}

/// A single experiential dimension tracked over time
#[derive(Debug, Clone)]
pub struct ExperientialDimension {
    pub name: String,
    pub id: u64,
    pub value: f32,
    pub variance: f32,
    pub trend: f32,
    history: Vec<f32>,
    write_idx: usize,
}

impl ExperientialDimension {
    fn new(name: String) -> Self {
        let id = fnv1a_hash(name.as_bytes());
        Self {
            name,
            id,
            value: 0.5,
            variance: 0.0,
            trend: 0.0,
            history: Vec::new(),
            write_idx: 0,
        }
    }

    fn update(&mut self, raw: f32) {
        self.value = EMA_ALPHA * raw + (1.0 - EMA_ALPHA) * self.value;

        let diff = raw - self.value;
        self.variance = EMA_ALPHA * diff * diff + (1.0 - EMA_ALPHA) * self.variance;

        if self.history.len() < MAX_HISTORY {
            self.history.push(raw);
        } else {
            self.history[self.write_idx] = raw;
        }
        self.write_idx = (self.write_idx + 1) % MAX_HISTORY;

        self.recompute_trend();
    }

    fn recompute_trend(&mut self) {
        if self.history.len() < 4 {
            self.trend = 0.0;
            return;
        }
        let len = self.history.len();
        let mid = len / 2;
        let first: f32 = self.history[..mid].iter().sum::<f32>() / mid as f32;
        let second: f32 =
            self.history[mid..].iter().sum::<f32>() / (len - mid) as f32;
        self.trend = second - first;
    }
}

/// Qualia report — narrative summary of experiential state
#[derive(Debug, Clone)]
pub struct QualiaReport {
    pub tick: u64,
    pub state: QualiaState,
    pub label: String,
    pub in_flow: bool,
    pub dimension_summaries: Vec<(String, f32, f32)>,
    pub recommendations: Vec<String>,
}

// ============================================================================
// STATS
// ============================================================================

/// Aggregate qualia engine statistics
#[derive(Debug, Clone)]
pub struct QualiaStats {
    pub total_evaluations: u64,
    pub current_quality: f32,
    pub mean_quality: f32,
    pub quality_variance: f32,
    pub flow_ticks: u64,
    pub flow_fraction: f32,
    pub dimension_count: usize,
    pub current_label: String,
}

// ============================================================================
// APPS QUALIA ENGINE
// ============================================================================

/// Engine that synthesizes the subjective experiential state of app management
#[derive(Debug)]
pub struct AppsQualiaEngine {
    state: QualiaState,
    dimensions: BTreeMap<u64, ExperientialDimension>,
    quality_history: Vec<f32>,
    quality_write_idx: usize,
    tick: u64,
    total_evaluations: u64,
    flow_ticks: u64,
    quality_variance: f32,
    mean_quality: f32,
    rng_state: u64,
}

impl AppsQualiaEngine {
    pub fn new(seed: u64) -> Self {
        let mut dimensions = BTreeMap::new();

        let default_dims = [
            "workload_harmony",
            "classification_clarity",
            "prediction_confidence",
            "flow_state",
            "cognitive_load",
            "dissonance",
        ];
        for name in &default_dims {
            let dim = ExperientialDimension::new(String::from(*name));
            dimensions.insert(dim.id, dim);
        }

        Self {
            state: QualiaState::new(),
            dimensions,
            quality_history: Vec::new(),
            quality_write_idx: 0,
            tick: 0,
            total_evaluations: 0,
            flow_ticks: 0,
            quality_variance: 0.0,
            mean_quality: 0.5,
            rng_state: if seed == 0 { 0xA4A1_CAFE_1234_5678 } else { seed },
        }
    }

    /// Evaluate the overall experience quality given raw signals
    pub fn experience_quality(
        &mut self,
        harmony: f32,
        clarity: f32,
        confidence: f32,
        load: f32,
        dissonance: f32,
    ) -> f32 {
        self.tick += 1;
        self.total_evaluations += 1;

        // Update core state with EMA
        self.state.workload_harmony =
            EMA_ALPHA * harmony + (1.0 - EMA_ALPHA) * self.state.workload_harmony;
        self.state.classification_clarity =
            EMA_ALPHA * clarity + (1.0 - EMA_ALPHA) * self.state.classification_clarity;
        self.state.prediction_confidence =
            EMA_ALPHA * confidence + (1.0 - EMA_ALPHA) * self.state.prediction_confidence;
        self.state.cognitive_load =
            EMA_ALPHA * load + (1.0 - EMA_ALPHA) * self.state.cognitive_load;
        self.state.dissonance =
            EMA_ALPHA * dissonance + (1.0 - EMA_ALPHA) * self.state.dissonance;

        // Compute flow state
        let flow_raw = if self.state.workload_harmony > 0.6
            && self.state.classification_clarity > 0.6
            && self.state.cognitive_load < 0.5
        {
            (self.state.workload_harmony + self.state.classification_clarity) / 2.0
        } else {
            self.state.flow_state * 0.9
        };
        self.state.flow_state =
            EMA_ALPHA * flow_raw + (1.0 - EMA_ALPHA) * self.state.flow_state;

        self.state.recompute_quality();

        if self.state.is_in_flow() {
            self.flow_ticks += 1;
        }

        // Update dimension trackers
        self.update_dimension("workload_harmony", harmony);
        self.update_dimension("classification_clarity", clarity);
        self.update_dimension("prediction_confidence", confidence);
        self.update_dimension("cognitive_load", load);
        self.update_dimension("dissonance", dissonance);
        self.update_dimension("flow_state", flow_raw);

        // Track quality history
        let q = self.state.experience_quality;
        if self.quality_history.len() < MAX_HISTORY {
            self.quality_history.push(q);
        } else {
            self.quality_history[self.quality_write_idx] = q;
        }
        self.quality_write_idx = (self.quality_write_idx + 1) % MAX_HISTORY;

        // Running quality mean and variance
        let diff = q - self.mean_quality;
        self.mean_quality = EMA_ALPHA * q + (1.0 - EMA_ALPHA) * self.mean_quality;
        self.quality_variance =
            EMA_ALPHA * diff * diff + (1.0 - EMA_ALPHA) * self.quality_variance;

        q
    }

    /// Get the current workload harmony
    pub fn workload_harmony(&self) -> f32 {
        self.state.workload_harmony
    }

    /// Get the classification clarity index
    pub fn clarity_index(&self) -> f32 {
        self.state.classification_clarity
    }

    /// Is the engine currently in a flow state?
    pub fn app_management_flow(&self) -> bool {
        self.state.is_in_flow()
    }

    /// Generate a comprehensive qualia report
    pub fn qualia_report(&self) -> QualiaReport {
        let mut dim_summaries = Vec::new();
        for (_, dim) in &self.dimensions {
            dim_summaries.push((dim.name.clone(), dim.value, dim.trend));
        }

        let mut recommendations = Vec::new();

        if self.state.workload_harmony < 0.4 {
            recommendations.push(String::from("workload_rebalancing_needed"));
        }
        if self.state.classification_clarity < CLARITY_LOW {
            recommendations.push(String::from("classification_refinement_needed"));
        }
        if self.state.cognitive_load > 0.7 {
            recommendations.push(String::from("reduce_analysis_depth"));
        }
        if self.state.dissonance > 0.5 {
            recommendations.push(String::from("model_recalibration_needed"));
        }
        if self.state.prediction_confidence < 0.4 {
            recommendations.push(String::from("more_training_data_needed"));
        }

        QualiaReport {
            tick: self.tick,
            state: self.state.clone(),
            label: String::from(self.state.experiential_label()),
            in_flow: self.state.is_in_flow(),
            dimension_summaries: dim_summaries,
            recommendations,
        }
    }

    /// Get the current subjective state
    pub fn subjective_state(&self) -> &QualiaState {
        &self.state
    }

    /// Add a custom experiential dimension
    pub fn add_dimension(&mut self, name: &str) {
        if self.dimensions.len() >= MAX_DIMENSION_ENTRIES {
            return;
        }
        let dim = ExperientialDimension::new(String::from(name));
        self.dimensions.insert(dim.id, dim);
    }

    /// Update a custom dimension
    pub fn update_custom_dimension(&mut self, name: &str, value: f32) {
        let id = fnv1a_hash(name.as_bytes());
        if let Some(dim) = self.dimensions.get_mut(&id) {
            dim.update(value);
        }
    }

    /// Quality trend over time
    pub fn quality_trend(&self) -> f32 {
        if self.quality_history.len() < 4 {
            return 0.0;
        }
        let len = self.quality_history.len();
        let mid = len / 2;
        let first: f32 = self.quality_history[..mid].iter().sum::<f32>() / mid as f32;
        let second: f32 =
            self.quality_history[mid..].iter().sum::<f32>() / (len - mid) as f32;
        second - first
    }

    /// Full stats
    pub fn stats(&self) -> QualiaStats {
        let flow_fraction = if self.total_evaluations > 0 {
            self.flow_ticks as f32 / self.total_evaluations as f32
        } else {
            0.0
        };

        QualiaStats {
            total_evaluations: self.total_evaluations,
            current_quality: self.state.experience_quality,
            mean_quality: self.mean_quality,
            quality_variance: self.quality_variance,
            flow_ticks: self.flow_ticks,
            flow_fraction,
            dimension_count: self.dimensions.len(),
            current_label: String::from(self.state.experiential_label()),
        }
    }

    /// Get a specific dimension's current value and trend
    pub fn dimension_state(&self, name: &str) -> Option<(f32, f32)> {
        let id = fnv1a_hash(name.as_bytes());
        self.dimensions.get(&id).map(|d| (d.value, d.trend))
    }

    /// Current tick
    pub fn current_tick(&self) -> u64 {
        self.tick
    }

    // ========================================================================
    // INTERNAL
    // ========================================================================

    fn update_dimension(&mut self, name: &str, value: f32) {
        let id = fnv1a_hash(name.as_bytes());
        if let Some(dim) = self.dimensions.get_mut(&id) {
            dim.update(value);
        }
    }
}
