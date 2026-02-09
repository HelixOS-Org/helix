// SPDX-License-Identifier: GPL-2.0
//! # Apps Awareness
//!
//! Application awareness consciousness. Tracks how well the apps engine
//! understands each application via a per-process awareness score. Detects
//! unknown behavior patterns that fall outside established models, measures
//! the learning rate for new applications, computes a familiarity index,
//! and triggers novelty responses when truly unprecedented behavior appears.
//!
//! A kernel that knows what it understands — and what it doesn't — can
//! allocate learning effort where it matters most.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.12;
const NOVELTY_THRESHOLD: f32 = 0.7;
const FAMILIAR_THRESHOLD: f32 = 0.6;
const MAX_PROCESSES: usize = 1024;
const MAX_BEHAVIOR_HISTORY: usize = 128;
const AWARENESS_DECAY: f32 = 0.998;
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

/// Xorshift64 PRNG for generating exploration noise
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

// ============================================================================
// PER-PROCESS AWARENESS
// ============================================================================

/// Awareness state for a single process
#[derive(Debug, Clone)]
pub struct ProcessAwareness {
    pub process_id: u64,
    pub process_name: String,
    /// Overall awareness score (0.0 – 1.0)
    pub awareness: f32,
    /// How familiar the engine is with this process's behavior
    pub familiarity: f32,
    /// Current novelty level (how unusual recent behavior is)
    pub novelty: f32,
    /// EMA-smoothed learning rate (how fast awareness is growing)
    pub learning_rate: f32,
    /// Number of behavioral observations
    pub observations: u64,
    /// Number of unknown behavior events
    pub unknown_events: u64,
    /// Tick of last observation
    pub last_tick: u64,
    /// Behavioral feature vector (FNV hashes of observed features)
    known_features: Vec<u64>,
    /// Behavior signature history (ring buffer)
    behavior_history: Vec<u64>,
    write_idx: usize,
    /// Previous awareness for learning rate computation
    prev_awareness: f32,
}

impl ProcessAwareness {
    fn new(process_id: u64, process_name: String) -> Self {
        Self {
            process_id,
            process_name,
            awareness: 0.1,
            familiarity: 0.0,
            novelty: 0.5,
            learning_rate: 0.0,
            observations: 0,
            unknown_events: 0,
            last_tick: 0,
            known_features: Vec::new(),
            behavior_history: Vec::new(),
            write_idx: 0,
            prev_awareness: 0.1,
        }
    }

    /// Record a behavioral observation
    #[inline]
    fn observe(&mut self, behavior_hash: u64, matched_known: bool, tick: u64) {
        self.observations += 1;
        self.last_tick = tick;
        self.prev_awareness = self.awareness;

        // Update awareness based on whether behavior matches known patterns
        let signal = if matched_known { 0.8 } else { 0.2 };
        self.awareness = EMA_ALPHA * signal + (1.0 - EMA_ALPHA) * self.awareness;

        // Learning rate = change in awareness
        self.learning_rate = EMA_ALPHA * (self.awareness - self.prev_awareness).abs()
            + (1.0 - EMA_ALPHA) * self.learning_rate;

        // Track novelty: inverse of match rate
        let novelty_signal = if matched_known { 0.1 } else { 0.9 };
        self.novelty = EMA_ALPHA * novelty_signal + (1.0 - EMA_ALPHA) * self.novelty;

        if !matched_known {
            self.unknown_events += 1;
        }

        // Update known features
        if matched_known && !self.known_features.contains(&behavior_hash) {
            self.known_features.push(behavior_hash);
        }

        // Familiarity grows with known feature coverage
        self.familiarity = if self.observations > 0 {
            let matched = self.observations.saturating_sub(self.unknown_events);
            matched as f32 / self.observations as f32
        } else {
            0.0
        };

        // Behavior history ring buffer
        if self.behavior_history.len() < MAX_BEHAVIOR_HISTORY {
            self.behavior_history.push(behavior_hash);
        } else {
            self.behavior_history[self.write_idx] = behavior_hash;
        }
        self.write_idx = (self.write_idx + 1) % MAX_BEHAVIOR_HISTORY;
    }

    /// Apply temporal decay to awareness (processes not recently seen fade)
    fn decay(&mut self) {
        self.awareness *= AWARENESS_DECAY;
        self.familiarity *= AWARENESS_DECAY;
    }
}

// ============================================================================
// NOVELTY RESPONSE
// ============================================================================

/// A triggered novelty response — something truly unprecedented happened
#[derive(Debug, Clone)]
pub struct NoveltyEvent {
    pub process_id: u64,
    pub process_name: String,
    pub novelty_score: f32,
    pub behavior_hash: u64,
    pub tick: u64,
}

// ============================================================================
// AWARENESS STATS
// ============================================================================

/// Aggregate awareness statistics
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct AwarenessStats {
    pub processes_tracked: usize,
    pub avg_awareness: f32,
    pub avg_familiarity: f32,
    pub avg_novelty: f32,
    pub avg_learning_rate: f32,
    pub total_unknown_events: u64,
    pub novelty_events_triggered: u64,
    pub highly_familiar_count: usize,
}

// ============================================================================
// APPS AWARENESS ENGINE
// ============================================================================

/// Application awareness consciousness — per-process awareness scoring
/// with novelty detection and familiarity tracking.
#[derive(Debug)]
pub struct AppsAwareness {
    /// Per-process awareness state keyed by process ID
    processes: BTreeMap<u64, ProcessAwareness>,
    /// Recent novelty events
    novelty_events: Vec<NoveltyEvent>,
    /// Monotonic tick
    tick: u64,
    /// Total novelty events triggered
    total_novelty_events: u64,
    /// PRNG state for exploration noise
    rng_state: u64,
    /// Global EMA of awareness across all processes
    global_awareness_ema: f32,
}

impl AppsAwareness {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            novelty_events: Vec::new(),
            tick: 0,
            total_novelty_events: 0,
            rng_state: 0xABCD_1234_5678_EF00,
            global_awareness_ema: 0.0,
        }
    }

    /// Compute the awareness score for a specific application
    #[inline]
    pub fn app_awareness_score(&self, process_id: u64) -> f32 {
        self.processes
            .get(&process_id)
            .map(|p| p.awareness)
            .unwrap_or(0.0)
    }

    /// Detect unknown behavior for a process
    #[inline]
    pub fn unknown_behavior_detect(
        &mut self,
        process_id: u64,
        process_name: &str,
        behavior_signature: &str,
        matched_known: bool,
    ) -> Option<NoveltyEvent> {
        self.tick += 1;
        let behavior_hash = fnv1a_hash(behavior_signature.as_bytes());

        let proc_entry = self
            .processes
            .entry(process_id)
            .or_insert_with(|| ProcessAwareness::new(process_id, String::from(process_name)));
        proc_entry.observe(behavior_hash, matched_known, self.tick);

        // Update global awareness EMA
        let avg: f32 = self.processes.values().map(|p| p.awareness).sum::<f32>()
            / self.processes.len().max(1) as f32;
        self.global_awareness_ema = EMA_ALPHA * avg + (1.0 - EMA_ALPHA) * self.global_awareness_ema;

        // Trigger novelty response if novelty exceeds threshold
        if proc_entry.novelty > NOVELTY_THRESHOLD && !matched_known {
            self.total_novelty_events += 1;
            let event = NoveltyEvent {
                process_id,
                process_name: String::from(process_name),
                novelty_score: proc_entry.novelty,
                behavior_hash,
                tick: self.tick,
            };
            self.novelty_events.push(event.clone());
            return Some(event);
        }
        None
    }

    /// Get the learning rate for a specific process
    #[inline]
    pub fn learning_rate(&self, process_id: u64) -> f32 {
        self.processes
            .get(&process_id)
            .map(|p| p.learning_rate)
            .unwrap_or(0.0)
    }

    /// Compute the familiarity index for a process (0.0 – 1.0)
    #[inline]
    pub fn familiarity_index(&self, process_id: u64) -> f32 {
        self.processes
            .get(&process_id)
            .map(|p| p.familiarity)
            .unwrap_or(0.0)
    }

    /// Generate a novelty response: prioritize exploration for novel processes
    pub fn novelty_response(&mut self) -> Vec<(u64, String, f32)> {
        let mut novel: Vec<(u64, String, f32)> = self
            .processes
            .values()
            .filter(|p| p.novelty > NOVELTY_THRESHOLD)
            .map(|p| (p.process_id, p.process_name.clone(), p.novelty))
            .collect();

        // Add exploration noise to break ties
        for item in novel.iter_mut() {
            let noise = (xorshift64(&mut self.rng_state) % 100) as f32 / 10000.0;
            item.2 += noise;
        }

        novel.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(core::cmp::Ordering::Equal));
        novel
    }

    /// Apply temporal decay to all processes (staleness reduction)
    #[inline]
    pub fn apply_decay(&mut self) {
        for proc_state in self.processes.values_mut() {
            proc_state.decay();
        }
    }

    /// Evict processes below minimum awareness threshold
    #[inline(always)]
    pub fn evict_stale(&mut self, min_awareness: f32) {
        self.processes.retain(|_, p| p.awareness > min_awareness);
    }

    /// Compute aggregate awareness statistics
    pub fn stats(&self) -> AwarenessStats {
        let n = self.processes.len();
        let (avg_aw, avg_fam, avg_nov, avg_lr, total_unk, familiar_count) = if n > 0 {
            let aw: f32 = self.processes.values().map(|p| p.awareness).sum::<f32>() / n as f32;
            let fam: f32 = self.processes.values().map(|p| p.familiarity).sum::<f32>() / n as f32;
            let nov: f32 = self.processes.values().map(|p| p.novelty).sum::<f32>() / n as f32;
            let lr: f32 = self
                .processes
                .values()
                .map(|p| p.learning_rate)
                .sum::<f32>()
                / n as f32;
            let unk: u64 = self.processes.values().map(|p| p.unknown_events).sum();
            let fam_c = self
                .processes
                .values()
                .filter(|p| p.familiarity > FAMILIAR_THRESHOLD)
                .count();
            (aw, fam, nov, lr, unk, fam_c)
        } else {
            (0.0, 0.0, 0.0, 0.0, 0, 0)
        };

        AwarenessStats {
            processes_tracked: n,
            avg_awareness: avg_aw,
            avg_familiarity: avg_fam,
            avg_novelty: avg_nov,
            avg_learning_rate: avg_lr,
            total_unknown_events: total_unk,
            novelty_events_triggered: self.total_novelty_events,
            highly_familiar_count: familiar_count,
        }
    }
}
