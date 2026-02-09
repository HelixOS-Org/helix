// SPDX-License-Identifier: GPL-2.0
//! # Apps Precognition Engine
//!
//! Pre-cognitive application behavior sensing. Detects subtle shifts in
//! application behavior — micro-trends, phase transitions, workload regime
//! changes — before they become statistically significant to conventional
//! detectors. Uses drift scores, CUSUM-like changepoint detection, and
//! multi-metric phase correlation.
//!
//! Where the anomaly forecaster waits for sigma-level deviations, the
//! precognition engine picks up on 0.3-sigma whispers and accumulates
//! evidence until a behavioral shift crystallizes.
//!
//! This is the apps engine feeling the future before it arrives.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_APPS: usize = 256;
const MAX_DRIFT_HISTORY: usize = 256;
const MAX_PHASE_HISTORY: usize = 64;
const DRIFT_THRESHOLD: f64 = 0.15;
const CUSUM_SLACK: f64 = 0.5;
const CUSUM_TRIGGER: f64 = 4.0;
const EMA_ALPHA: f64 = 0.10;
const EMA_FAST: f64 = 0.25;
const EMA_SLOW: f64 = 0.05;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const XORSHIFT_SEED: u64 = 0xaced_face_cafe_bead;

// ============================================================================
// UTILITY FUNCTIONS
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

#[inline]
fn ema_update(current: f64, sample: f64, alpha: f64) -> f64 {
    alpha * sample + (1.0 - alpha) * current
}

fn abs_f64(v: f64) -> f64 {
    if v < 0.0 { -v } else { v }
}

fn sqrt_f64(v: f64) -> f64 {
    if v <= 0.0 {
        return 0.0;
    }
    let mut g = v;
    for _ in 0..25 {
        g = 0.5 * (g + v / g);
    }
    g
}

// ============================================================================
// BEHAVIORAL METRIC
// ============================================================================

/// Which behavioral metric is being tracked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BehaviorMetric {
    CpuUsage,
    MemoryFootprint,
    IoRate,
    SyscallRate,
    FaultRate,
    ThreadActivity,
}

impl BehaviorMetric {
    fn as_str(&self) -> &'static str {
        match self {
            Self::CpuUsage => "cpu",
            Self::MemoryFootprint => "memory",
            Self::IoRate => "io",
            Self::SyscallRate => "syscalls",
            Self::FaultRate => "faults",
            Self::ThreadActivity => "threads",
        }
    }

    fn from_index(i: usize) -> Self {
        match i % 6 {
            0 => Self::CpuUsage,
            1 => Self::MemoryFootprint,
            2 => Self::IoRate,
            3 => Self::SyscallRate,
            4 => Self::FaultRate,
            _ => Self::ThreadActivity,
        }
    }
}

// ============================================================================
// DRIFT DETECTION RESULT
// ============================================================================

/// Result of behavioral drift analysis.
#[derive(Debug, Clone)]
pub struct DriftResult {
    pub metric: BehaviorMetric,
    pub drift_score: f64,
    pub direction: f64,
    pub duration_ticks: u64,
    pub is_significant: bool,
}

// ============================================================================
// PHASE TRANSITION
// ============================================================================

/// A detected phase transition in app behavior.
#[derive(Debug, Clone)]
pub struct PhaseTransitionEvent {
    pub app_id: u64,
    pub from_phase_hash: u64,
    pub to_phase_hash: u64,
    pub transition_tick: u64,
    pub confidence: f64,
    pub metrics_changed: Vec<BehaviorMetric>,
}

// ============================================================================
// WORKLOAD CHANGE
// ============================================================================

/// A detected workload regime change.
#[derive(Debug, Clone)]
pub struct WorkloadChange {
    pub app_id: u64,
    pub tick: u64,
    pub magnitude: f64,
    pub primary_metric: BehaviorMetric,
    pub direction: f64,
}

// ============================================================================
// METRIC TRACKER (CUSUM + DUAL EMA)
// ============================================================================

#[derive(Debug, Clone)]
struct MetricTracker {
    metric: BehaviorMetric,
    ema_fast: f64,
    ema_slow: f64,
    cusum_pos: f64,
    cusum_neg: f64,
    ema_variance: f64,
    drift_score: f64,
    drift_direction: f64,
    drift_duration: u64,
    sample_count: u64,
    last_value: f64,
    changepoint_detected: bool,
}

impl MetricTracker {
    fn new(metric: BehaviorMetric) -> Self {
        Self {
            metric,
            ema_fast: 0.0,
            ema_slow: 0.0,
            cusum_pos: 0.0,
            cusum_neg: 0.0,
            ema_variance: 1.0,
            drift_score: 0.0,
            drift_direction: 0.0,
            drift_duration: 0,
            sample_count: 0,
            last_value: 0.0,
            changepoint_detected: false,
        }
    }

    fn update(&mut self, value: f64) {
        self.sample_count += 1;

        let old_fast = self.ema_fast;
        self.ema_fast = ema_update(self.ema_fast, value, EMA_FAST);
        self.ema_slow = ema_update(self.ema_slow, value, EMA_SLOW);

        let diff = value - self.ema_slow;
        self.ema_variance = ema_update(self.ema_variance, diff * diff, EMA_ALPHA);

        // CUSUM for changepoint detection
        let std = sqrt_f64(self.ema_variance).max(0.001);
        let normalized = (value - self.ema_slow) / std;
        self.cusum_pos = (self.cusum_pos + normalized - CUSUM_SLACK).max(0.0);
        self.cusum_neg = (self.cusum_neg - normalized - CUSUM_SLACK).max(0.0);
        self.changepoint_detected = self.cusum_pos > CUSUM_TRIGGER || self.cusum_neg > CUSUM_TRIGGER;

        if self.changepoint_detected {
            self.cusum_pos = 0.0;
            self.cusum_neg = 0.0;
        }

        // Drift: divergence of fast EMA from slow EMA
        let divergence = self.ema_fast - self.ema_slow;
        let rel_drift = abs_f64(divergence) / (abs_f64(self.ema_slow) + 0.001);
        self.drift_score = ema_update(self.drift_score, rel_drift, EMA_ALPHA);
        self.drift_direction = if divergence > 0.0 { 1.0 } else { -1.0 };

        if self.drift_score > DRIFT_THRESHOLD {
            self.drift_duration += 1;
        } else {
            self.drift_duration = 0;
        }

        self.last_value = value;
    }

    fn is_drifting(&self) -> bool {
        self.drift_score > DRIFT_THRESHOLD && self.drift_duration >= 3
    }
}

// ============================================================================
// PER-APP PRECOGNITION STATE
// ============================================================================

const NUM_METRICS: usize = 6;

#[derive(Debug, Clone)]
struct AppPrecogState {
    app_id: u64,
    trackers: [MetricTracker; NUM_METRICS],
    phase_hash: u64,
    phase_transitions: VecDeque<PhaseTransitionEvent>,
    workload_changes: VecDeque<WorkloadChange>,
    precognition_score: f64,
    total_shifts_detected: u64,
    total_adaptations: u64,
}

impl AppPrecogState {
    fn new(app_id: u64) -> Self {
        Self {
            app_id,
            trackers: [
                MetricTracker::new(BehaviorMetric::CpuUsage),
                MetricTracker::new(BehaviorMetric::MemoryFootprint),
                MetricTracker::new(BehaviorMetric::IoRate),
                MetricTracker::new(BehaviorMetric::SyscallRate),
                MetricTracker::new(BehaviorMetric::FaultRate),
                MetricTracker::new(BehaviorMetric::ThreadActivity),
            ],
            phase_hash: 0,
            phase_transitions: VecDeque::new(),
            workload_changes: VecDeque::new(),
            precognition_score: 0.0,
            total_shifts_detected: 0,
            total_adaptations: 0,
        }
    }

    fn compute_phase_hash(&self) -> u64 {
        let mut data = Vec::new();
        for t in &self.trackers {
            // Quantize the slow EMA to reduce noise
            let quantized = (t.ema_slow * 100.0) as i64;
            data.extend_from_slice(&quantized.to_le_bytes());
        }
        fnv1a_hash(&data)
    }

    fn update_metrics(&mut self, values: &[f64; NUM_METRICS]) {
        for (i, &v) in values.iter().enumerate() {
            self.trackers[i].update(v);
        }
    }

    fn detect_phase_transition(&mut self, tick: u64) -> Option<PhaseTransitionEvent> {
        let new_hash = self.compute_phase_hash();
        if new_hash == self.phase_hash || self.phase_hash == 0 {
            self.phase_hash = new_hash;
            return None;
        }

        // Check if enough metrics have changepoints
        let changepoint_count = self.trackers.iter().filter(|t| t.changepoint_detected).count();
        if changepoint_count < 2 {
            self.phase_hash = new_hash;
            return None;
        }

        let metrics_changed: Vec<BehaviorMetric> = self
            .trackers
            .iter()
            .filter(|t| t.changepoint_detected)
            .map(|t| t.metric)
            .collect();

        let confidence = changepoint_count as f64 / NUM_METRICS as f64;

        let event = PhaseTransitionEvent {
            app_id: self.app_id,
            from_phase_hash: self.phase_hash,
            to_phase_hash: new_hash,
            transition_tick: tick,
            confidence,
            metrics_changed,
        };

        self.phase_hash = new_hash;
        if self.phase_transitions.len() >= MAX_PHASE_HISTORY {
            self.phase_transitions.pop_front();
        }
        self.phase_transitions.push_back(event.clone());
        self.total_shifts_detected += 1;

        Some(event)
    }

    fn detect_workload_change(&mut self, tick: u64) -> Option<WorkloadChange> {
        // Find the metric with the strongest drift
        let mut max_drift = 0.0;
        let mut max_idx = 0;
        for (i, t) in self.trackers.iter().enumerate() {
            if t.drift_score > max_drift {
                max_drift = t.drift_score;
                max_idx = i;
            }
        }

        if max_drift < DRIFT_THRESHOLD * 2.0 {
            return None;
        }

        let tracker = &self.trackers[max_idx];
        let change = WorkloadChange {
            app_id: self.app_id,
            tick,
            magnitude: max_drift,
            primary_metric: tracker.metric,
            direction: tracker.drift_direction,
        };

        if self.workload_changes.len() >= MAX_DRIFT_HISTORY {
            self.workload_changes.pop_front();
        }
        self.workload_changes.push_back(change.clone());

        Some(change)
    }
}

// ============================================================================
// PRECOGNITION STATS
// ============================================================================

/// Engine-level statistics for the precognition module.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PrecognitionStats {
    pub total_shifts_detected: u64,
    pub total_phase_transitions: u64,
    pub total_workload_changes: u64,
    pub average_precognition_score: f64,
    pub total_adaptations: u64,
    pub drift_detections: u64,
    pub average_drift_magnitude: f64,
}

impl PrecognitionStats {
    fn new() -> Self {
        Self {
            total_shifts_detected: 0,
            total_phase_transitions: 0,
            total_workload_changes: 0,
            average_precognition_score: 0.0,
            total_adaptations: 0,
            drift_detections: 0,
            average_drift_magnitude: 0.0,
        }
    }
}

// ============================================================================
// APPS PRECOGNITION ENGINE
// ============================================================================

/// Pre-cognitive behavior sensing engine for applications.
///
/// Detects subtle behavioral shifts using dual-EMA drift analysis,
/// CUSUM changepoint detection, and phase hashing. Operates below the
/// threshold of conventional anomaly detectors to provide early signals.
pub struct AppsPrecognition {
    app_states: BTreeMap<u64, AppPrecogState>,
    stats: PrecognitionStats,
    rng_state: u64,
    tick: u64,
    ema_drift_global: f64,
    ema_precog_score: f64,
}

impl AppsPrecognition {
    /// Create a new precognition engine.
    pub fn new() -> Self {
        Self {
            app_states: BTreeMap::new(),
            stats: PrecognitionStats::new(),
            rng_state: XORSHIFT_SEED,
            tick: 0,
            ema_drift_global: 0.0,
            ema_precog_score: 0.0,
        }
    }

    /// Sense behavioral shifts for an application given current metrics.
    ///
    /// Returns a list of drift results for each metric that is currently drifting.
    pub fn sense_app_shift(
        &mut self,
        app_id: u64,
        cpu: f64,
        memory: f64,
        io: f64,
        syscalls: f64,
        faults: f64,
        threads: f64,
    ) -> Vec<DriftResult> {
        self.tick += 1;

        if self.app_states.len() >= MAX_APPS && !self.app_states.contains_key(&app_id) {
            self.app_states.insert(app_id, AppPrecogState::new(app_id));
        }
        let state = self.app_states.get_mut(&app_id).unwrap();
        let values = [cpu, memory, io, syscalls, faults, threads];
        state.update_metrics(&values);

        let mut drifts = Vec::new();
        for tracker in &state.trackers {
            if tracker.is_drifting() {
                drifts.push(DriftResult {
                    metric: tracker.metric,
                    drift_score: tracker.drift_score,
                    direction: tracker.drift_direction,
                    duration_ticks: tracker.drift_duration,
                    is_significant: tracker.drift_score > DRIFT_THRESHOLD * 2.0,
                });
            }
        }

        if !drifts.is_empty() {
            self.stats.drift_detections += 1;
            let max_drift = drifts.iter().map(|d| d.drift_score).fold(0.0_f64, |a, b| if b > a { b } else { a });
            self.ema_drift_global = ema_update(self.ema_drift_global, max_drift, EMA_ALPHA);
            self.stats.average_drift_magnitude = self.ema_drift_global;
        }

        drifts
    }

    /// Check for behavioral drift across all metrics.
    pub fn behavioral_drift(&self, app_id: u64) -> Vec<DriftResult> {
        let state = match self.app_states.get(&app_id) {
            Some(s) => s,
            None => return Vec::new(),
        };

        state
            .trackers
            .iter()
            .filter(|t| t.is_drifting())
            .map(|t| DriftResult {
                metric: t.metric,
                drift_score: t.drift_score,
                direction: t.drift_direction,
                duration_ticks: t.drift_duration,
                is_significant: t.drift_score > DRIFT_THRESHOLD * 2.0,
            })
            .collect()
    }

    /// Detect a phase transition in the app's behavior.
    #[inline]
    pub fn phase_transition(&mut self, app_id: u64) -> Option<PhaseTransitionEvent> {
        let state = self.app_states.get_mut(&app_id)?;
        let event = state.detect_phase_transition(self.tick);
        if event.is_some() {
            self.stats.total_phase_transitions += 1;
            self.stats.total_shifts_detected += 1;
        }
        event
    }

    /// Detect a workload regime change.
    #[inline]
    pub fn workload_change_detection(&mut self, app_id: u64) -> Option<WorkloadChange> {
        let state = self.app_states.get_mut(&app_id)?;
        let change = state.detect_workload_change(self.tick);
        if change.is_some() {
            self.stats.total_workload_changes += 1;
        }
        change
    }

    /// Compute a precognition score for the app: how much early signal is present.
    ///
    /// Higher scores mean more metrics are drifting and changepoints are forming.
    pub fn precognition_score(&mut self, app_id: u64) -> f64 {
        let state = match self.app_states.get_mut(&app_id) {
            Some(s) => s,
            None => return 0.0,
        };

        let drift_count = state.trackers.iter().filter(|t| t.is_drifting()).count();
        let cusum_max = state
            .trackers
            .iter()
            .map(|t| t.cusum_pos.max(t.cusum_neg))
            .fold(0.0_f64, |a, b| if b > a { b } else { a });

        let drift_component = drift_count as f64 / NUM_METRICS as f64;
        let cusum_component = (cusum_max / CUSUM_TRIGGER).min(1.0);
        let score = 0.6 * drift_component + 0.4 * cusum_component;

        state.precognition_score = ema_update(state.precognition_score, score, EMA_ALPHA);
        self.ema_precog_score = ema_update(self.ema_precog_score, state.precognition_score, EMA_ALPHA);
        self.stats.average_precognition_score = self.ema_precog_score;

        state.precognition_score
    }

    /// Suggest early adaptation actions based on detected shifts.
    pub fn early_adaptation(&mut self, app_id: u64) -> Vec<(BehaviorMetric, f64, String)> {
        let state = match self.app_states.get_mut(&app_id) {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut actions = Vec::new();
        for tracker in &state.trackers {
            if tracker.is_drifting() && tracker.drift_score > DRIFT_THRESHOLD * 1.5 {
                let suggestion = match tracker.metric {
                    BehaviorMetric::CpuUsage => {
                        if tracker.drift_direction > 0.0 {
                            String::from("pre_scale_cpu_quota_up")
                        } else {
                            String::from("reclaim_idle_cpu_slices")
                        }
                    }
                    BehaviorMetric::MemoryFootprint => {
                        if tracker.drift_direction > 0.0 {
                            String::from("pre_allocate_memory_pool")
                        } else {
                            String::from("shrink_memory_reservation")
                        }
                    }
                    BehaviorMetric::IoRate => {
                        if tracker.drift_direction > 0.0 {
                            String::from("warm_io_cache_prefetch")
                        } else {
                            String::from("release_io_buffers")
                        }
                    }
                    BehaviorMetric::SyscallRate => {
                        if tracker.drift_direction > 0.0 {
                            String::from("prepare_syscall_fast_path")
                        } else {
                            String::from("no_action_needed")
                        }
                    }
                    BehaviorMetric::FaultRate => {
                        if tracker.drift_direction > 0.0 {
                            String::from("pre_map_likely_pages")
                        } else {
                            String::from("no_action_needed")
                        }
                    }
                    BehaviorMetric::ThreadActivity => {
                        if tracker.drift_direction > 0.0 {
                            String::from("reserve_scheduler_slots")
                        } else {
                            String::from("compact_thread_pool")
                        }
                    }
                };
                actions.push((tracker.metric, tracker.drift_score, suggestion));
                state.total_adaptations += 1;
            }
        }

        self.stats.total_adaptations += actions.len() as u64;
        actions
    }

    /// Return a snapshot of engine statistics.
    #[inline(always)]
    pub fn stats(&self) -> &PrecognitionStats {
        &self.stats
    }

    /// Number of tracked apps.
    #[inline(always)]
    pub fn tracked_apps(&self) -> usize {
        self.app_states.len()
    }

    /// Global EMA-smoothed precognition score.
    #[inline(always)]
    pub fn avg_precognition_score(&self) -> f64 {
        self.ema_precog_score
    }
}
