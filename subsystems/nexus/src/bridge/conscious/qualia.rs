// SPDX-License-Identifier: GPL-2.0
//! # Bridge Qualia Engine
//!
//! Subjective experience representation for the bridge. This module maps
//! objective performance metrics to subjective quality measures — how does
//! syscall processing "feel" from the bridge's perspective?
//!
//! ## Qualia Dimensions
//!
//! - **Smoothness** — are operations flowing without stuttering?
//! - **Flow state** — is the bridge in an optimal processing groove?
//! - **Friction** — how much resistance does the bridge encounter?
//! - **Harmony** — are all subsystems working in concert?
//!
//! These subjective measures provide a higher-level abstraction over raw
//! metrics, enabling holistic assessment of bridge operational quality.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.10;
const FLOW_THRESHOLD_LOW: f32 = 0.60;
const FLOW_THRESHOLD_HIGH: f32 = 0.85;
const FRICTION_WARNING: f32 = 0.50;
const FRICTION_CRITICAL: f32 = 0.80;
const HARMONY_BASELINE: f32 = 0.50;
const SMOOTHNESS_JITTER_PENALTY: f32 = 2.0;
const MAX_EXPERIENCE_HISTORY: usize = 256;
const MAX_METRIC_CHANNELS: usize = 32;
const SNAPSHOT_INTERVAL: u64 = 10;
const MAX_SNAPSHOTS: usize = 128;
const FLOW_STREAK_THRESHOLD: u32 = 10;
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

fn ema_update(current: f32, sample: f32, alpha: f32) -> f32 {
    current * (1.0 - alpha) + sample * alpha
}

// ============================================================================
// FLOW LEVEL
// ============================================================================

/// The bridge's current flow state
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FlowLevel {
    /// Disrupted — frequent interruptions, errors, stalls
    Disrupted,
    /// Normal — functioning but not optimal
    Normal,
    /// Flowing — smooth, efficient, everything clicking
    Flowing,
    /// Peak — absolute optimal state, rare and beautiful
    Peak,
}

impl FlowLevel {
    fn from_score(score: f32) -> Self {
        if score >= FLOW_THRESHOLD_HIGH {
            FlowLevel::Peak
        } else if score >= FLOW_THRESHOLD_LOW {
            FlowLevel::Flowing
        } else if score >= 0.30 {
            FlowLevel::Normal
        } else {
            FlowLevel::Disrupted
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            FlowLevel::Disrupted => "disrupted",
            FlowLevel::Normal => "normal",
            FlowLevel::Flowing => "flowing",
            FlowLevel::Peak => "peak",
        }
    }
}

// ============================================================================
// QUALIA STATE
// ============================================================================

/// The bridge's subjective experience state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct QualiaState {
    pub processing_smoothness: f32,
    pub flow_state: f32,
    pub friction: f32,
    pub harmony: f32,
    pub flow_level: FlowLevel,
    pub overall_quality: f32,
    pub tick: u64,
}

impl QualiaState {
    fn compute_overall(&self) -> f32 {
        let positive = self.processing_smoothness * 0.3 + self.flow_state * 0.3 + self.harmony * 0.2;
        let negative = self.friction * 0.2;
        (positive - negative).clamp(0.0, 1.0)
    }
}

// ============================================================================
// METRIC CHANNEL
// ============================================================================

/// A raw metric channel feeding into qualia computation
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MetricChannel {
    pub name: String,
    pub name_hash: u64,
    pub dimension: QualiaDimension,
    pub current_value: f32,
    pub ema_value: f32,
    pub variance: f32,
    pub sample_count: u64,
    pub last_value: f32,
}

impl MetricChannel {
    fn new(name: &str, dimension: QualiaDimension) -> Self {
        Self {
            name: String::from(name),
            name_hash: fnv1a_hash(name.as_bytes()),
            dimension,
            current_value: 0.0,
            ema_value: 0.0,
            variance: 0.0,
            sample_count: 0,
            last_value: 0.0,
        }
    }

    fn update(&mut self, value: f32) {
        self.last_value = self.current_value;
        self.current_value = value;
        self.ema_value = ema_update(self.ema_value, value, EMA_ALPHA);
        // Running variance using Welford's method (simplified)
        let diff = value - self.ema_value;
        self.variance = ema_update(self.variance, diff * diff, EMA_ALPHA);
        self.sample_count += 1;
    }

    fn jitter(&self) -> f32 {
        (self.current_value - self.last_value).abs()
    }
}

// ============================================================================
// QUALIA DIMENSION
// ============================================================================

/// Which qualia dimension a metric contributes to
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualiaDimension {
    Smoothness,
    Flow,
    Friction,
    Harmony,
}

// ============================================================================
// EXPERIENCE SNAPSHOT
// ============================================================================

/// A snapshot of the qualia state at a point in time
#[derive(Debug, Clone)]
pub struct ExperienceSnapshot {
    pub tick: u64,
    pub smoothness: f32,
    pub flow: f32,
    pub friction: f32,
    pub harmony: f32,
    pub overall: f32,
    pub flow_level: FlowLevel,
}

// ============================================================================
// STATS
// ============================================================================

/// Qualia engine statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct QualiaStats {
    pub total_updates: u64,
    pub avg_smoothness: f32,
    pub avg_flow: f32,
    pub avg_friction: f32,
    pub avg_harmony: f32,
    pub avg_overall: f32,
    pub current_flow_level: FlowLevel,
    pub peak_flow_count: u64,
    pub disrupted_count: u64,
    pub flow_streak: u32,
    pub metric_channels: usize,
}

// ============================================================================
// BRIDGE QUALIA ENGINE
// ============================================================================

/// Subjective experience engine for bridge operational quality
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BridgeQualiaEngine {
    channels: BTreeMap<u64, MetricChannel>,
    snapshots: VecDeque<ExperienceSnapshot>,
    history: VecDeque<QualiaState>,
    smoothness_ema: f32,
    flow_ema: f32,
    friction_ema: f32,
    harmony_ema: f32,
    current_tick: u64,
    total_updates: u64,
    peak_flow_count: u64,
    disrupted_count: u64,
    flow_streak: u32,
    best_flow_streak: u32,
    last_snapshot_tick: u64,
}

impl BridgeQualiaEngine {
    /// Create a new qualia engine
    pub fn new() -> Self {
        Self {
            channels: BTreeMap::new(),
            snapshots: VecDeque::new(),
            history: VecDeque::new(),
            smoothness_ema: 0.5,
            flow_ema: 0.5,
            friction_ema: 0.0,
            harmony_ema: HARMONY_BASELINE,
            current_tick: 0,
            total_updates: 0,
            peak_flow_count: 0,
            disrupted_count: 0,
            flow_streak: 0,
            best_flow_streak: 0,
            last_snapshot_tick: 0,
        }
    }

    /// Register a metric channel
    #[inline]
    pub fn register_channel(&mut self, name: &str, dimension: QualiaDimension) {
        if self.channels.len() < MAX_METRIC_CHANNELS {
            let channel = MetricChannel::new(name, dimension);
            self.channels.insert(channel.name_hash, channel);
        }
    }

    /// Feed a metric value into the qualia computation
    pub fn feed_metric(&mut self, channel_name: &str, value: f32) {
        self.current_tick += 1;
        self.total_updates += 1;

        let hash = fnv1a_hash(channel_name.as_bytes());
        if let Some(channel) = self.channels.get_mut(&hash) {
            channel.update(value.clamp(0.0, 1.0));
        }

        self.recompute_qualia();

        // Periodic snapshots
        if self.current_tick - self.last_snapshot_tick >= SNAPSHOT_INTERVAL {
            self.take_snapshot();
            self.last_snapshot_tick = self.current_tick;
        }
    }

    fn recompute_qualia(&mut self) {
        // Aggregate channels by dimension
        let mut smoothness_sum = 0.0f32;
        let mut smoothness_count = 0u32;
        let mut flow_sum = 0.0f32;
        let mut flow_count = 0u32;
        let mut friction_sum = 0.0f32;
        let mut friction_count = 0u32;
        let mut harmony_sum = 0.0f32;
        let mut harmony_count = 0u32;

        for channel in self.channels.values() {
            match channel.dimension {
                QualiaDimension::Smoothness => {
                    // Smoothness is inversely related to jitter
                    let jitter_penalty = (channel.jitter() * SMOOTHNESS_JITTER_PENALTY).min(1.0);
                    smoothness_sum += (channel.ema_value - jitter_penalty).max(0.0);
                    smoothness_count += 1;
                }
                QualiaDimension::Flow => {
                    flow_sum += channel.ema_value;
                    flow_count += 1;
                }
                QualiaDimension::Friction => {
                    friction_sum += channel.ema_value;
                    friction_count += 1;
                }
                QualiaDimension::Harmony => {
                    // Harmony is inversely related to variance
                    let var_penalty = channel.variance.sqrt().min(1.0);
                    harmony_sum += (channel.ema_value - var_penalty * 0.5).max(0.0);
                    harmony_count += 1;
                }
            }
        }

        let raw_smoothness = if smoothness_count > 0 {
            smoothness_sum / smoothness_count as f32
        } else {
            0.5
        };
        let raw_flow = if flow_count > 0 {
            flow_sum / flow_count as f32
        } else {
            0.5
        };
        let raw_friction = if friction_count > 0 {
            friction_sum / friction_count as f32
        } else {
            0.0
        };
        let raw_harmony = if harmony_count > 0 {
            harmony_sum / harmony_count as f32
        } else {
            HARMONY_BASELINE
        };

        self.smoothness_ema = ema_update(self.smoothness_ema, raw_smoothness, EMA_ALPHA);
        self.flow_ema = ema_update(self.flow_ema, raw_flow, EMA_ALPHA);
        self.friction_ema = ema_update(self.friction_ema, raw_friction, EMA_ALPHA);
        self.harmony_ema = ema_update(self.harmony_ema, raw_harmony, EMA_ALPHA);

        // Track flow state
        let flow_level = FlowLevel::from_score(self.flow_ema);
        match flow_level {
            FlowLevel::Peak => {
                self.peak_flow_count += 1;
                self.flow_streak += 1;
                if self.flow_streak > self.best_flow_streak {
                    self.best_flow_streak = self.flow_streak;
                }
            }
            FlowLevel::Flowing => {
                self.flow_streak += 1;
                if self.flow_streak > self.best_flow_streak {
                    self.best_flow_streak = self.flow_streak;
                }
            }
            FlowLevel::Disrupted => {
                self.disrupted_count += 1;
                self.flow_streak = 0;
            }
            FlowLevel::Normal => {
                self.flow_streak = 0;
            }
        }

        // Record in history
        let state = QualiaState {
            processing_smoothness: self.smoothness_ema,
            flow_state: self.flow_ema,
            friction: self.friction_ema,
            harmony: self.harmony_ema,
            flow_level,
            overall_quality: 0.0,
            tick: self.current_tick,
        };
        let overall = state.compute_overall();
        if self.history.len() >= MAX_EXPERIENCE_HISTORY {
            self.history.pop_front();
        }
        self.history.push_back(QualiaState {
            overall_quality: overall,
            ..state
        });
    }

    fn take_snapshot(&mut self) {
        let flow_level = FlowLevel::from_score(self.flow_ema);
        let snap = ExperienceSnapshot {
            tick: self.current_tick,
            smoothness: self.smoothness_ema,
            flow: self.flow_ema,
            friction: self.friction_ema,
            harmony: self.harmony_ema,
            overall: self.experience_quality(),
            flow_level,
        };
        if self.snapshots.len() >= MAX_SNAPSHOTS {
            self.snapshots.pop_front();
        }
        self.snapshots.push_back(snap);
    }

    /// Overall experience quality score
    #[inline]
    pub fn experience_quality(&self) -> f32 {
        let positive = self.smoothness_ema * 0.3 + self.flow_ema * 0.3 + self.harmony_ema * 0.2;
        let negative = self.friction_ema * 0.2;
        (positive - negative).clamp(0.0, 1.0)
    }

    /// Current flow state
    #[inline(always)]
    pub fn flow_state(&self) -> (FlowLevel, f32) {
        (FlowLevel::from_score(self.flow_ema), self.flow_ema)
    }

    /// Current friction score
    #[inline(always)]
    pub fn friction_score(&self) -> f32 {
        self.friction_ema
    }

    /// Current harmony index
    #[inline(always)]
    pub fn harmony_index(&self) -> f32 {
        self.harmony_ema
    }

    /// Full qualia snapshot
    pub fn qualia_snapshot(&self) -> QualiaState {
        let flow_level = FlowLevel::from_score(self.flow_ema);
        let mut state = QualiaState {
            processing_smoothness: self.smoothness_ema,
            flow_state: self.flow_ema,
            friction: self.friction_ema,
            harmony: self.harmony_ema,
            flow_level,
            overall_quality: 0.0,
            tick: self.current_tick,
        };
        state.overall_quality = state.compute_overall();
        state
    }

    /// Subjective report — human-readable assessment of the bridge's experience
    pub fn subjective_report(&self) -> Vec<(String, String)> {
        let mut report = Vec::new();

        let smoothness_desc = if self.smoothness_ema > 0.8 {
            "silky smooth"
        } else if self.smoothness_ema > 0.5 {
            "reasonably smooth"
        } else {
            "stuttering"
        };
        report.push((String::from("smoothness"), String::from(smoothness_desc)));

        let (flow_level, _) = self.flow_state();
        report.push((String::from("flow"), String::from(flow_level.as_str())));

        let friction_desc = if self.friction_ema > FRICTION_CRITICAL {
            "severe friction"
        } else if self.friction_ema > FRICTION_WARNING {
            "moderate friction"
        } else {
            "minimal friction"
        };
        report.push((String::from("friction"), String::from(friction_desc)));

        let harmony_desc = if self.harmony_ema > 0.8 {
            "deep harmony"
        } else if self.harmony_ema > 0.5 {
            "moderate harmony"
        } else {
            "dissonance"
        };
        report.push((String::from("harmony"), String::from(harmony_desc)));

        let overall = self.experience_quality();
        let overall_desc = if overall > 0.8 {
            "transcendent"
        } else if overall > 0.6 {
            "pleasant"
        } else if overall > 0.4 {
            "neutral"
        } else {
            "distressed"
        };
        report.push((String::from("overall_experience"), String::from(overall_desc)));

        report
    }

    /// Is the bridge in a flow state?
    #[inline(always)]
    pub fn is_flowing(&self) -> bool {
        self.flow_ema >= FLOW_THRESHOLD_LOW
    }

    /// Is friction at a warning level?
    #[inline(always)]
    pub fn friction_warning(&self) -> bool {
        self.friction_ema >= FRICTION_WARNING
    }

    /// Statistics snapshot
    pub fn stats(&self) -> QualiaStats {
        QualiaStats {
            total_updates: self.total_updates,
            avg_smoothness: self.smoothness_ema,
            avg_flow: self.flow_ema,
            avg_friction: self.friction_ema,
            avg_harmony: self.harmony_ema,
            avg_overall: self.experience_quality(),
            current_flow_level: FlowLevel::from_score(self.flow_ema),
            peak_flow_count: self.peak_flow_count,
            disrupted_count: self.disrupted_count,
            flow_streak: self.flow_streak,
            metric_channels: self.channels.len(),
        }
    }

    /// Reset the qualia engine
    #[inline]
    pub fn reset(&mut self) {
        self.channels.clear();
        self.snapshots.clear();
        self.history.clear();
        self.smoothness_ema = 0.5;
        self.flow_ema = 0.5;
        self.friction_ema = 0.0;
        self.harmony_ema = HARMONY_BASELINE;
        self.flow_streak = 0;
    }
}
