// SPDX-License-Identifier: GPL-2.0
//! # Apps Timeline Projector
//!
//! Projects application lifecycle timelines by modeling phase transitions
//! as a stochastic process. Each app is mapped through the canonical
//! sequence: startup → warming → steady-state → peak → wind-down → exit.
//! Transition probabilities are learned from historical observations, and
//! remaining lifetime is estimated via survival analysis on phase durations.
//!
//! This is the kernel knowing how long an application will live.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_TRACKED_APPS: usize = 512;
const MAX_PHASE_HISTORY: usize = 64;
const NUM_PHASES: usize = 6;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const DIVERGENCE_THRESHOLD: f32 = 0.30;

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
// LIFECYCLE PHASES
// ============================================================================

/// Canonical application lifecycle phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LifecyclePhase {
    Startup = 0,
    Warming = 1,
    SteadyState = 2,
    Peak = 3,
    WindDown = 4,
    Exit = 5,
}

impl LifecyclePhase {
    fn from_index(i: usize) -> Self {
        match i {
            0 => LifecyclePhase::Startup,
            1 => LifecyclePhase::Warming,
            2 => LifecyclePhase::SteadyState,
            3 => LifecyclePhase::Peak,
            4 => LifecyclePhase::WindDown,
            _ => LifecyclePhase::Exit,
        }
    }

    fn next(self) -> Self {
        match self {
            LifecyclePhase::Startup => LifecyclePhase::Warming,
            LifecyclePhase::Warming => LifecyclePhase::SteadyState,
            LifecyclePhase::SteadyState => LifecyclePhase::Peak,
            LifecyclePhase::Peak => LifecyclePhase::WindDown,
            LifecyclePhase::WindDown => LifecyclePhase::Exit,
            LifecyclePhase::Exit => LifecyclePhase::Exit,
        }
    }
}

// ============================================================================
// TIMELINE TYPES
// ============================================================================

/// A projected lifecycle segment
#[derive(Debug, Clone)]
pub struct TimelineSegment {
    pub phase: LifecyclePhase,
    pub start_tick: u64,
    pub estimated_duration: u64,
    pub confidence: f32,
}

/// Full projected timeline for a process
#[derive(Debug, Clone)]
pub struct ProjectedTimeline {
    pub process_id: u64,
    pub segments: Vec<TimelineSegment>,
    pub total_estimated_ticks: u64,
    pub current_phase: LifecyclePhase,
    pub projection_confidence: f32,
}

/// Phase duration statistics
#[derive(Debug, Clone, Copy)]
pub struct PhaseDurationStats {
    pub phase: LifecyclePhase,
    pub avg_duration: f32,
    pub min_duration: u64,
    pub max_duration: u64,
    pub variance: f32,
    pub sample_count: u64,
}

/// Transition probability between two phases
#[derive(Debug, Clone, Copy)]
pub struct TransitionProb {
    pub from: LifecyclePhase,
    pub to: LifecyclePhase,
    pub probability: f32,
    pub avg_transition_time: f32,
    pub observations: u64,
}

/// Remaining lifetime estimate
#[derive(Debug, Clone)]
pub struct RemainingLifetime {
    pub process_id: u64,
    pub current_phase: LifecyclePhase,
    pub ticks_in_current_phase: u64,
    pub estimated_remaining: u64,
    pub confidence: f32,
    pub survival_probability: f32,
}

/// Timeline divergence report
#[derive(Debug, Clone)]
pub struct TimelineDivergence {
    pub process_id: u64,
    pub expected_phase: LifecyclePhase,
    pub actual_phase: LifecyclePhase,
    pub divergence_score: f32,
    pub description: String,
}

// ============================================================================
// PER-APP TIMELINE TRACKER
// ============================================================================

/// Per-process phase transition tracker
#[derive(Debug, Clone)]
struct AppTimeline {
    current_phase: LifecyclePhase,
    phase_entry_tick: u64,
    transitions: Vec<(LifecyclePhase, LifecyclePhase, u64)>,
    phase_durations: BTreeMap<u8, Vec<u64>>,
    total_ticks_alive: u64,
}

impl AppTimeline {
    fn new(start_tick: u64) -> Self {
        let mut durations = BTreeMap::new();
        for i in 0..NUM_PHASES {
            durations.insert(i as u8, Vec::new());
        }
        Self {
            current_phase: LifecyclePhase::Startup,
            phase_entry_tick: start_tick,
            transitions: Vec::new(),
            phase_durations: durations,
            total_ticks_alive: 0,
        }
    }

    fn transition(&mut self, new_phase: LifecyclePhase, tick: u64) {
        let duration = tick.saturating_sub(self.phase_entry_tick);
        let key = self.current_phase as u8;
        if let Some(durs) = self.phase_durations.get_mut(&key) {
            if durs.len() < MAX_PHASE_HISTORY {
                durs.push(duration);
            }
        }
        self.transitions.push((self.current_phase, new_phase, tick));
        self.current_phase = new_phase;
        self.phase_entry_tick = tick;
    }

    fn avg_phase_duration(&self, phase: LifecyclePhase) -> f32 {
        let key = phase as u8;
        if let Some(durs) = self.phase_durations.get(&key) {
            if durs.is_empty() {
                return 5000.0;
            }
            let sum: u64 = durs.iter().sum();
            sum as f32 / durs.len() as f32
        } else {
            5000.0
        }
    }

    fn phase_variance(&self, phase: LifecyclePhase) -> f32 {
        let key = phase as u8;
        if let Some(durs) = self.phase_durations.get(&key) {
            if durs.len() < 2 {
                return 1000.0;
            }
            let avg = self.avg_phase_duration(phase);
            let sum_sq: f32 = durs.iter().map(|&d| {
                let diff = d as f32 - avg;
                diff * diff
            }).sum();
            sum_sq / (durs.len() - 1) as f32
        } else {
            1000.0
        }
    }

    fn transition_count(&self, from: LifecyclePhase, to: LifecyclePhase) -> u64 {
        self.transitions.iter()
            .filter(|(f, t, _)| *f == from && *t == to)
            .count() as u64
    }

    fn total_transitions_from(&self, from: LifecyclePhase) -> u64 {
        self.transitions.iter()
            .filter(|(f, _, _)| *f == from)
            .count() as u64
    }
}

// ============================================================================
// TIMELINE STATS
// ============================================================================

/// Aggregate timeline statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct TimelineStats {
    pub tracked_apps: usize,
    pub total_transitions: u64,
    pub avg_lifetime_estimate: f32,
    pub avg_projection_confidence: f32,
    pub divergence_count: u64,
    pub phase_accuracy: f32,
}

// ============================================================================
// APPS TIMELINE PROJECTOR
// ============================================================================

/// Projects application lifecycle timelines using learned phase transition
/// statistics and survival analysis for remaining lifetime estimation.
#[derive(Debug)]
pub struct AppsTimeline {
    apps: BTreeMap<u64, AppTimeline>,
    global_phase_avg: [f32; NUM_PHASES],
    total_transitions: u64,
    divergence_count: u64,
    tick: u64,
    rng_state: u64,
    projection_confidence_ema: f32,
    phase_accuracy_ema: f32,
}

impl AppsTimeline {
    pub fn new() -> Self {
        Self {
            apps: BTreeMap::new(),
            global_phase_avg: [5000.0; NUM_PHASES],
            total_transitions: 0,
            divergence_count: 0,
            tick: 0,
            rng_state: 0xCAFE_BABE_1234_5678,
            projection_confidence_ema: 0.5,
            phase_accuracy_ema: 0.5,
        }
    }

    /// Register a new app for timeline tracking
    pub fn register_app(&mut self, process_id: u64, start_tick: u64) {
        self.tick = start_tick;
        if self.apps.len() < MAX_TRACKED_APPS {
            self.apps.insert(process_id, AppTimeline::new(start_tick));
        }
    }

    /// Record a phase transition for an app
    pub fn record_transition(
        &mut self,
        process_id: u64,
        new_phase: LifecyclePhase,
        tick: u64,
    ) {
        self.tick = tick;
        self.total_transitions += 1;
        if let Some(app) = self.apps.get_mut(&process_id) {
            let old_phase = app.current_phase;
            app.transition(new_phase, tick);
            let dur = app.avg_phase_duration(old_phase);
            let idx = old_phase as usize;
            if idx < NUM_PHASES {
                self.global_phase_avg[idx] =
                    EMA_ALPHA * dur + (1.0 - EMA_ALPHA) * self.global_phase_avg[idx];
            }
        }
    }

    /// Project the full lifecycle timeline for a process
    pub fn project_lifecycle(&self, process_id: u64) -> ProjectedTimeline {
        let mut segments = Vec::new();
        let mut total_ticks: u64 = 0;
        let mut current_phase = LifecyclePhase::Startup;
        let mut confidence: f32 = 0.8;

        if let Some(app) = self.apps.get(&process_id) {
            current_phase = app.current_phase;
            let mut phase = current_phase;
            let mut start_tick = self.tick;

            for _ in 0..NUM_PHASES {
                if phase == LifecyclePhase::Exit {
                    break;
                }
                let est_dur = app.avg_phase_duration(phase) as u64;
                segments.push(TimelineSegment {
                    phase,
                    start_tick,
                    estimated_duration: est_dur,
                    confidence,
                });
                start_tick += est_dur;
                total_ticks += est_dur;
                confidence *= 0.85;
                phase = phase.next();
            }
        } else {
            for i in 0..NUM_PHASES {
                let phase = LifecyclePhase::from_index(i);
                if phase == LifecyclePhase::Exit {
                    break;
                }
                let est_dur = self.global_phase_avg[i] as u64;
                segments.push(TimelineSegment {
                    phase,
                    start_tick: total_ticks,
                    estimated_duration: est_dur,
                    confidence,
                });
                total_ticks += est_dur;
                confidence *= 0.8;
            }
        }

        ProjectedTimeline {
            process_id,
            segments,
            total_estimated_ticks: total_ticks,
            current_phase,
            projection_confidence: confidence,
        }
    }

    /// Get duration statistics for a specific phase
    pub fn phase_duration(&self, process_id: u64, phase: LifecyclePhase) -> PhaseDurationStats {
        if let Some(app) = self.apps.get(&process_id) {
            let key = phase as u8;
            let durs = app.phase_durations.get(&key);
            let sample_count = durs.map(|d| d.len() as u64).unwrap_or(0);
            let min_d = durs.and_then(|d| d.iter().copied().min()).unwrap_or(0);
            let max_d = durs.and_then(|d| d.iter().copied().max()).unwrap_or(0);
            PhaseDurationStats {
                phase,
                avg_duration: app.avg_phase_duration(phase),
                min_duration: min_d,
                max_duration: max_d,
                variance: app.phase_variance(phase),
                sample_count,
            }
        } else {
            let idx = phase as usize;
            PhaseDurationStats {
                phase,
                avg_duration: self.global_phase_avg[idx.min(NUM_PHASES - 1)],
                min_duration: 0,
                max_duration: 0,
                variance: 1000.0,
                sample_count: 0,
            }
        }
    }

    /// Compute the transition probability from one phase to another
    pub fn transition_probability(
        &self,
        process_id: u64,
        from: LifecyclePhase,
        to: LifecyclePhase,
    ) -> TransitionProb {
        if let Some(app) = self.apps.get(&process_id) {
            let count = app.transition_count(from, to);
            let total = app.total_transitions_from(from);
            let prob = if total > 0 {
                count as f32 / total as f32
            } else if to == from.next() {
                0.8
            } else {
                0.05
            };
            TransitionProb {
                from,
                to,
                probability: prob,
                avg_transition_time: app.avg_phase_duration(from),
                observations: count,
            }
        } else {
            let prob = if to == from.next() { 0.7 } else { 0.1 };
            TransitionProb {
                from,
                to,
                probability: prob,
                avg_transition_time: 5000.0,
                observations: 0,
            }
        }
    }

    /// Estimate the remaining lifetime of a process
    pub fn remaining_lifetime(&self, process_id: u64) -> RemainingLifetime {
        if let Some(app) = self.apps.get(&process_id) {
            let ticks_in_phase = self.tick.saturating_sub(app.phase_entry_tick);
            let mut remaining: u64 = 0;
            let mut phase = app.current_phase;
            let avg_cur = app.avg_phase_duration(phase) as u64;
            remaining += avg_cur.saturating_sub(ticks_in_phase);
            phase = phase.next();
            while phase != LifecyclePhase::Exit {
                remaining += app.avg_phase_duration(phase) as u64;
                phase = phase.next();
            }
            let survival = if avg_cur > 0 {
                1.0 - (ticks_in_phase as f32 / avg_cur as f32).min(1.0)
            } else {
                0.5
            };
            RemainingLifetime {
                process_id,
                current_phase: app.current_phase,
                ticks_in_current_phase: ticks_in_phase,
                estimated_remaining: remaining,
                confidence: 0.6,
                survival_probability: survival.max(0.01),
            }
        } else {
            RemainingLifetime {
                process_id,
                current_phase: LifecyclePhase::Startup,
                ticks_in_current_phase: 0,
                estimated_remaining: 30_000,
                confidence: 0.1,
                survival_probability: 0.9,
            }
        }
    }

    /// Compute timeline divergence: how far actual behavior deviates from projection
    pub fn timeline_divergence(
        &mut self,
        process_id: u64,
        actual_phase: LifecyclePhase,
    ) -> TimelineDivergence {
        let expected = if let Some(app) = self.apps.get(&process_id) {
            let elapsed = self.tick.saturating_sub(app.phase_entry_tick);
            let avg_dur = app.avg_phase_duration(app.current_phase);
            if elapsed as f32 > avg_dur {
                app.current_phase.next()
            } else {
                app.current_phase
            }
        } else {
            LifecyclePhase::Startup
        };

        let phase_distance = ((expected as i32) - (actual_phase as i32)).unsigned_abs() as f32;
        let divergence = (phase_distance / NUM_PHASES as f32).min(1.0);

        if divergence > DIVERGENCE_THRESHOLD {
            self.divergence_count += 1;
        }

        let correct = if expected == actual_phase { 1.0 } else { 0.0 };
        self.phase_accuracy_ema =
            EMA_ALPHA * correct + (1.0 - EMA_ALPHA) * self.phase_accuracy_ema;

        let mut desc = String::new();
        if divergence > DIVERGENCE_THRESHOLD {
            desc.push_str("significant_divergence");
        } else {
            desc.push_str("within_tolerance");
        }

        TimelineDivergence {
            process_id,
            expected_phase: expected,
            actual_phase,
            divergence_score: divergence,
            description: desc,
        }
    }

    /// Deregister an app
    pub fn deregister_app(&mut self, process_id: u64) {
        self.apps.remove(&process_id);
    }

    /// Get aggregate statistics
    pub fn stats(&self) -> TimelineStats {
        let avg_life: f32 = self.global_phase_avg.iter().sum();
        TimelineStats {
            tracked_apps: self.apps.len(),
            total_transitions: self.total_transitions,
            avg_lifetime_estimate: avg_life,
            avg_projection_confidence: self.projection_confidence_ema,
            divergence_count: self.divergence_count,
            phase_accuracy: self.phase_accuracy_ema,
        }
    }
}
