// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Qualia Engine
//!
//! Subjective experience of cooperation. What does cooperation "feel" like to
//! the system? When all processes share fairly the qualia state is harmonious;
//! when contention rises friction intensifies; when processes band together
//! solidarity emerges; and when trust erodes tension builds.
//!
//! Qualia are not mere metrics — they are higher-order integrations of multiple
//! cooperation signals into a unified experiential snapshot. The engine
//! maintains a continuous qualia state that reflects the overall cooperation
//! atmosphere and can be queried to make holistic policy decisions.
//!
//! ## Qualia Dimensions
//!
//! - **Harmony** — Degree of cooperative alignment across processes
//! - **Friction** — Level of resistance and contention in resource sharing
//! - **Solidarity** — Strength of collective cooperation bonds
//! - **Tension** — Latent stress from unresolved fairness issues
//!
//! ## Key Methods
//!
//! - `cooperation_experience()` — Update qualia from cooperation signals
//! - `harmony_level()` — Current harmony measure
//! - `friction_score()` — Current friction measure
//! - `solidarity_index()` — Current solidarity strength
//! - `qualia_snapshot()` — Full qualia state snapshot
//! - `experience_quality()` — Overall quality of the cooperation experience

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.10;
const QUALIA_DECAY: f32 = 0.994;
const MAX_EXPERIENCE_HISTORY: usize = 128;
const MAX_PROCESS_QUALIA: usize = 512;
const HARMONY_WEIGHT: f32 = 0.35;
const FRICTION_WEIGHT: f32 = 0.25;
const SOLIDARITY_WEIGHT: f32 = 0.25;
const TENSION_WEIGHT: f32 = 0.15;
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

/// Xorshift64 PRNG for qualia noise
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

/// The subjective cooperation experience state
#[derive(Debug, Clone)]
pub struct QualiaState {
    /// Degree of cooperative alignment (0.0–1.0)
    pub harmony: f32,
    /// Level of resource contention friction (0.0–1.0)
    pub friction: f32,
    /// Strength of collective cooperation bonds (0.0–1.0)
    pub solidarity: f32,
    /// Latent stress from unresolved fairness issues (0.0–1.0)
    pub tension: f32,
    /// Composite experience quality (-1.0 to 1.0, positive = good)
    pub quality: f32,
    /// Tick when this state was computed
    pub tick: u64,
}

impl QualiaState {
    pub fn new() -> Self {
        Self {
            harmony: 0.5,
            friction: 0.0,
            solidarity: 0.5,
            tension: 0.0,
            quality: 0.5,
            tick: 0,
        }
    }

    /// Compute composite quality from dimensions
    pub fn compute_quality(&mut self) {
        let positive = self.harmony * HARMONY_WEIGHT + self.solidarity * SOLIDARITY_WEIGHT;
        let negative = self.friction * FRICTION_WEIGHT + self.tension * TENSION_WEIGHT;
        let raw = positive - negative;
        self.quality = if raw < -1.0 {
            -1.0
        } else if raw > 1.0 {
            1.0
        } else {
            raw
        };
    }
}

// ============================================================================
// PROCESS QUALIA
// ============================================================================

/// Per-process qualia contribution
#[derive(Debug, Clone)]
pub struct ProcessQualia {
    pub process_id: u64,
    /// This process's harmony contribution
    pub harmony_contribution: f32,
    /// This process's friction contribution
    pub friction_contribution: f32,
    /// This process's solidarity score
    pub solidarity_score: f32,
    /// This process's tension level
    pub tension_level: f32,
    /// Last update tick
    pub last_update_tick: u64,
    /// Number of experience updates
    pub update_count: u64,
}

impl ProcessQualia {
    pub fn new(process_id: u64) -> Self {
        Self {
            process_id,
            harmony_contribution: 0.5,
            friction_contribution: 0.0,
            solidarity_score: 0.5,
            tension_level: 0.0,
            last_update_tick: 0,
            update_count: 0,
        }
    }

    /// Update this process's qualia contributions
    pub fn update(
        &mut self,
        harmony: f32,
        friction: f32,
        solidarity: f32,
        tension: f32,
        tick: u64,
    ) {
        let clamp = |v: f32| {
            if v < 0.0 {
                0.0
            } else if v > 1.0 {
                1.0
            } else {
                v
            }
        };
        self.harmony_contribution += EMA_ALPHA * (clamp(harmony) - self.harmony_contribution);
        self.friction_contribution += EMA_ALPHA * (clamp(friction) - self.friction_contribution);
        self.solidarity_score += EMA_ALPHA * (clamp(solidarity) - self.solidarity_score);
        self.tension_level += EMA_ALPHA * (clamp(tension) - self.tension_level);
        self.last_update_tick = tick;
        self.update_count += 1;
    }
}

// ============================================================================
// EXPERIENCE RECORD
// ============================================================================

/// Historical record of a qualia snapshot
#[derive(Debug, Clone)]
pub struct ExperienceRecord {
    pub tick: u64,
    pub quality: f32,
    pub harmony: f32,
    pub friction: f32,
    pub solidarity: f32,
    pub tension: f32,
}

// ============================================================================
// QUALIA STATS
// ============================================================================

#[derive(Debug, Clone)]
pub struct CoopQualiaStats {
    pub total_experiences: u64,
    pub tracked_processes: usize,
    pub avg_harmony: f32,
    pub avg_friction: f32,
    pub avg_solidarity: f32,
    pub avg_tension: f32,
    pub overall_quality: f32,
    pub quality_trend: f32,
    pub peak_harmony: f32,
    pub peak_friction: f32,
}

impl CoopQualiaStats {
    pub fn new() -> Self {
        Self {
            total_experiences: 0,
            tracked_processes: 0,
            avg_harmony: 0.5,
            avg_friction: 0.0,
            avg_solidarity: 0.5,
            avg_tension: 0.0,
            overall_quality: 0.5,
            quality_trend: 0.0,
            peak_harmony: 0.0,
            peak_friction: 0.0,
        }
    }
}

// ============================================================================
// COOPERATION QUALIA ENGINE
// ============================================================================

/// Engine modeling the subjective experience of cooperation
pub struct CoopQualiaEngine {
    /// Current global qualia state
    current_state: QualiaState,
    /// Per-process qualia contributions
    process_qualia: BTreeMap<u64, ProcessQualia>,
    /// History of experience snapshots
    experience_history: Vec<ExperienceRecord>,
    history_write_idx: usize,
    /// Running statistics
    pub stats: CoopQualiaStats,
    /// PRNG state
    rng_state: u64,
    /// Current tick
    tick: u64,
    /// EMA-smoothed quality trend
    quality_trend_ema: f32,
    /// Previous quality for trend computation
    prev_quality: f32,
}

impl CoopQualiaEngine {
    pub fn new(seed: u64) -> Self {
        let mut experience_history = Vec::with_capacity(MAX_EXPERIENCE_HISTORY);
        for _ in 0..MAX_EXPERIENCE_HISTORY {
            experience_history.push(ExperienceRecord {
                tick: 0,
                quality: 0.0,
                harmony: 0.0,
                friction: 0.0,
                solidarity: 0.0,
                tension: 0.0,
            });
        }
        Self {
            current_state: QualiaState::new(),
            process_qualia: BTreeMap::new(),
            experience_history,
            history_write_idx: 0,
            stats: CoopQualiaStats::new(),
            rng_state: seed | 1,
            tick: 0,
            quality_trend_ema: 0.0,
            prev_quality: 0.5,
        }
    }

    // ========================================================================
    // COOPERATION EXPERIENCE
    // ========================================================================

    /// Update the qualia state from cooperation signals for a process
    ///
    /// Takes sharing_success (→ harmony), contention_level (→ friction),
    /// collective_cooperation (→ solidarity), and unresolved_fairness (→ tension).
    pub fn cooperation_experience(
        &mut self,
        process_id: u64,
        sharing_success: f32,
        contention_level: f32,
        collective_cooperation: f32,
        unresolved_fairness: f32,
    ) {
        self.tick += 1;

        // Update per-process qualia
        if !self.process_qualia.contains_key(&process_id) {
            if self.process_qualia.len() >= MAX_PROCESS_QUALIA {
                self.evict_stale_process();
            }
            self.process_qualia
                .insert(process_id, ProcessQualia::new(process_id));
        }

        let tick = self.tick;
        if let Some(pq) = self.process_qualia.get_mut(&process_id) {
            pq.update(
                sharing_success,
                contention_level,
                collective_cooperation,
                unresolved_fairness,
                tick,
            );
        }

        // Recompute global qualia from all process contributions
        self.recompute_global_qualia();
        self.stats.total_experiences += 1;
    }

    /// Recompute the global qualia state from all process contributions
    fn recompute_global_qualia(&mut self) {
        let count = self.process_qualia.len();
        if count == 0 {
            return;
        }
        let inv = 1.0 / count as f32;

        let mut sum_harmony = 0.0f32;
        let mut sum_friction = 0.0f32;
        let mut sum_solidarity = 0.0f32;
        let mut sum_tension = 0.0f32;

        for (_, pq) in self.process_qualia.iter() {
            sum_harmony += pq.harmony_contribution;
            sum_friction += pq.friction_contribution;
            sum_solidarity += pq.solidarity_score;
            sum_tension += pq.tension_level;
        }

        let avg_h = sum_harmony * inv;
        let avg_f = sum_friction * inv;
        let avg_s = sum_solidarity * inv;
        let avg_t = sum_tension * inv;

        self.current_state.harmony += EMA_ALPHA * (avg_h - self.current_state.harmony);
        self.current_state.friction += EMA_ALPHA * (avg_f - self.current_state.friction);
        self.current_state.solidarity += EMA_ALPHA * (avg_s - self.current_state.solidarity);
        self.current_state.tension += EMA_ALPHA * (avg_t - self.current_state.tension);
        self.current_state.tick = self.tick;
        self.current_state.compute_quality();

        // Update trend
        let quality_delta = self.current_state.quality - self.prev_quality;
        self.quality_trend_ema += EMA_ALPHA * (quality_delta - self.quality_trend_ema);
        self.prev_quality = self.current_state.quality;

        // Record history
        self.experience_history[self.history_write_idx] = ExperienceRecord {
            tick: self.tick,
            quality: self.current_state.quality,
            harmony: self.current_state.harmony,
            friction: self.current_state.friction,
            solidarity: self.current_state.solidarity,
            tension: self.current_state.tension,
        };
        self.history_write_idx = (self.history_write_idx + 1) % MAX_EXPERIENCE_HISTORY;

        // Update stats
        self.stats.tracked_processes = count;
        self.stats.avg_harmony = self.current_state.harmony;
        self.stats.avg_friction = self.current_state.friction;
        self.stats.avg_solidarity = self.current_state.solidarity;
        self.stats.avg_tension = self.current_state.tension;
        self.stats.overall_quality = self.current_state.quality;
        self.stats.quality_trend = self.quality_trend_ema;
        if self.current_state.harmony > self.stats.peak_harmony {
            self.stats.peak_harmony = self.current_state.harmony;
        }
        if self.current_state.friction > self.stats.peak_friction {
            self.stats.peak_friction = self.current_state.friction;
        }
    }

    // ========================================================================
    // HARMONY LEVEL
    // ========================================================================

    /// Current cooperation harmony measure
    pub fn harmony_level(&self) -> f32 {
        self.current_state.harmony
    }

    /// Per-process harmony contributions
    pub fn harmony_by_process(&self) -> Vec<(u64, f32)> {
        let mut result = Vec::new();
        for (pid, pq) in self.process_qualia.iter() {
            result.push((*pid, pq.harmony_contribution));
        }
        result
    }

    /// Average harmony from history
    pub fn harmony_trend(&self) -> f32 {
        let count = if self.stats.total_experiences < MAX_EXPERIENCE_HISTORY as u64 {
            self.stats.total_experiences as usize
        } else {
            MAX_EXPERIENCE_HISTORY
        };
        if count == 0 {
            return 0.0;
        }
        let mut sum = 0.0f32;
        for i in 0..count {
            sum += self.experience_history[i].harmony;
        }
        sum / count as f32
    }

    // ========================================================================
    // FRICTION SCORE
    // ========================================================================

    /// Current cooperation friction measure
    pub fn friction_score(&self) -> f32 {
        self.current_state.friction
    }

    /// Processes contributing most to friction
    pub fn friction_sources(&self) -> Vec<(u64, f32)> {
        let mut sources: Vec<(u64, f32)> = self
            .process_qualia
            .iter()
            .map(|(pid, pq)| (*pid, pq.friction_contribution))
            .collect();
        sources.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        sources
    }

    // ========================================================================
    // SOLIDARITY INDEX
    // ========================================================================

    /// Current solidarity strength across all cooperating processes
    pub fn solidarity_index(&self) -> f32 {
        self.current_state.solidarity
    }

    /// Solidarity variance — how uniform is solidarity across processes?
    pub fn solidarity_variance(&self) -> f32 {
        let count = self.process_qualia.len();
        if count < 2 {
            return 0.0;
        }
        let mean = self.current_state.solidarity;
        let mut var_sum = 0.0f32;
        for (_, pq) in self.process_qualia.iter() {
            let dev = pq.solidarity_score - mean;
            var_sum += dev * dev;
        }
        var_sum / count as f32
    }

    // ========================================================================
    // QUALIA SNAPSHOT
    // ========================================================================

    /// Full qualia state snapshot
    pub fn qualia_snapshot(&self) -> QualiaState {
        self.current_state.clone()
    }

    /// Historical quality values
    pub fn quality_history(&self) -> Vec<f32> {
        let count = if self.stats.total_experiences < MAX_EXPERIENCE_HISTORY as u64 {
            self.stats.total_experiences as usize
        } else {
            MAX_EXPERIENCE_HISTORY
        };
        let mut history = Vec::with_capacity(count);
        for i in 0..count {
            history.push(self.experience_history[i].quality);
        }
        history
    }

    // ========================================================================
    // EXPERIENCE QUALITY
    // ========================================================================

    /// Overall quality of the cooperation experience
    ///
    /// Integrates all four qualia dimensions into a single score.
    /// Positive = good cooperation experience, negative = poor.
    pub fn experience_quality(&self) -> f32 {
        self.current_state.quality
    }

    /// Quality trend: positive = improving, negative = degrading
    pub fn quality_trend(&self) -> f32 {
        self.quality_trend_ema
    }

    /// Fingerprint the current qualia state for change detection
    pub fn state_fingerprint(&self) -> u64 {
        let mut buf = Vec::new();
        let h_bits = self.current_state.harmony.to_bits().to_le_bytes();
        let f_bits = self.current_state.friction.to_bits().to_le_bytes();
        let s_bits = self.current_state.solidarity.to_bits().to_le_bytes();
        let t_bits = self.current_state.tension.to_bits().to_le_bytes();
        buf.extend_from_slice(&h_bits);
        buf.extend_from_slice(&f_bits);
        buf.extend_from_slice(&s_bits);
        buf.extend_from_slice(&t_bits);
        fnv1a_hash(&buf)
    }

    // ========================================================================
    // DECAY & MAINTENANCE
    // ========================================================================

    /// Apply decay to all qualia dimensions
    pub fn decay_all(&mut self) {
        let rng = &mut self.rng_state;
        let jitter = (xorshift64(rng) % 40) as f32 / 100_000.0;
        let decay = QUALIA_DECAY - jitter;

        // Decay negative dimensions toward zero
        self.current_state.friction *= decay;
        self.current_state.tension *= decay;
        if self.current_state.friction < 0.001 {
            self.current_state.friction = 0.0;
        }
        if self.current_state.tension < 0.001 {
            self.current_state.tension = 0.0;
        }

        // Positive dimensions decay toward baseline (0.5)
        self.current_state.harmony += (0.5 - self.current_state.harmony) * (1.0 - decay);
        self.current_state.solidarity += (0.5 - self.current_state.solidarity) * (1.0 - decay);

        self.current_state.compute_quality();
    }

    /// Prune stale process qualia
    pub fn prune_stale(&mut self, max_age: u64) {
        let cutoff = if self.tick > max_age {
            self.tick - max_age
        } else {
            0
        };
        let stale: Vec<u64> = self
            .process_qualia
            .iter()
            .filter(|(_, pq)| pq.last_update_tick < cutoff)
            .map(|(k, _)| *k)
            .collect();
        for key in stale {
            self.process_qualia.remove(&key);
        }
    }

    fn evict_stale_process(&mut self) {
        let mut oldest_tick = u64::MAX;
        let mut oldest_pid: Option<u64> = None;
        for (pid, pq) in self.process_qualia.iter() {
            if pq.last_update_tick < oldest_tick {
                oldest_tick = pq.last_update_tick;
                oldest_pid = Some(*pid);
            }
        }
        if let Some(pid) = oldest_pid {
            self.process_qualia.remove(&pid);
        }
    }

    // ========================================================================
    // QUERIES
    // ========================================================================

    pub fn process_qualia(&self, process_id: u64) -> Option<&ProcessQualia> {
        self.process_qualia.get(&process_id)
    }

    pub fn process_count(&self) -> usize {
        self.process_qualia.len()
    }

    pub fn snapshot_stats(&self) -> CoopQualiaStats {
        self.stats.clone()
    }

    /// Tension level
    pub fn tension_level(&self) -> f32 {
        self.current_state.tension
    }
}
