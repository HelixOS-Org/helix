// SPDX-License-Identifier: GPL-2.0
//! # Bridge Timeline
//!
//! Projects the sequence of future syscalls for each process using a Markov
//! chain with configurable memory depth. Beyond simple next-state prediction,
//! the timeline engine can project N-step sequences, identify branching points
//! where multiple futures are equally likely, and compute the convergence point
//! where divergent paths rejoin. Timeline entropy measures how unpredictable
//! a process's future is at each horizon.
//!
//! The bridge reading the script before the actors know their lines.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_PROCESSES: usize = 128;
const MAX_STATES: usize = 256;
const MARKOV_MEMORY: usize = 3;
const MAX_PROJECTION_LENGTH: usize = 64;
const EMA_ALPHA: f32 = 0.12;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const BRANCH_THRESHOLD: f32 = 0.30;
const CONVERGENCE_SCAN_DEPTH: usize = 32;

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
// MARKOV CHAIN TYPES
// ============================================================================

/// A state context: the last N syscalls (Markov memory)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct StateContext {
    syscalls: Vec<u32>,
}

impl StateContext {
    fn hash(&self) -> u64 {
        let mut h = FNV_OFFSET;
        for &s in &self.syscalls {
            for &b in &s.to_le_bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(FNV_PRIME);
            }
        }
        h
    }
}

/// Transition counts from a state context
#[derive(Debug, Clone)]
struct TransitionRow {
    counts: BTreeMap<u32, u64>,
    total: u64,
}

impl TransitionRow {
    fn new() -> Self {
        Self {
            counts: BTreeMap::new(),
            total: 0,
        }
    }

    fn record(&mut self, next_syscall: u32) {
        *self.counts.entry(next_syscall).or_insert(0) += 1;
        self.total += 1;
    }

    fn probability(&self, syscall: u32) -> f32 {
        if self.total == 0 {
            return 0.0;
        }
        self.counts.get(&syscall).copied().unwrap_or(0) as f32 / self.total as f32
    }

    fn most_likely(&self) -> Option<(u32, f32)> {
        self.counts
            .iter()
            .max_by_key(|(_, &count)| count)
            .map(|(&syscall, &count)| (syscall, count as f32 / self.total.max(1) as f32))
    }

    fn top_n(&self, n: usize) -> Vec<(u32, f32)> {
        let mut entries: Vec<(u32, f32)> = self
            .counts
            .iter()
            .map(|(&s, &c)| (s, c as f32 / self.total.max(1) as f32))
            .collect();
        entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        entries.truncate(n);
        entries
    }

    fn entropy(&self) -> f32 {
        if self.total == 0 {
            return 0.0;
        }
        let mut h = 0.0f32;
        for &count in self.counts.values() {
            let p = count as f32 / self.total as f32;
            if p > 0.0 {
                h -= p * log2_approx(p);
            }
        }
        h
    }
}

/// Approximate log2 for no_std entropy calculation
fn log2_approx(x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    // Use the identity: log2(x) = ln(x) / ln(2)
    // Approximate ln(x) using the series around 1
    let bits = x.to_bits();
    let exp = ((bits >> 23) & 0xFF) as f32 - 127.0;
    let mantissa = f32::from_bits((bits & 0x007F_FFFF) | 0x3F80_0000);
    // Minimax polynomial for log2(mantissa) in [1,2)
    exp + (mantissa - 1.0) * (1.4426950408 - 0.7213 * (mantissa - 1.0))
}

// ============================================================================
// PER-PROCESS TIMELINE
// ============================================================================

/// Timeline state for a single process
#[derive(Debug, Clone)]
struct ProcessTimeline {
    process_id: u64,
    recent_syscalls: Vec<u32>,
    transitions: BTreeMap<u64, TransitionRow>,
    total_observations: u64,
    entropy_ema: f32,
}

impl ProcessTimeline {
    fn new(process_id: u64) -> Self {
        Self {
            process_id,
            recent_syscalls: Vec::new(),
            transitions: BTreeMap::new(),
            total_observations: 0,
            entropy_ema: 1.0,
        }
    }

    fn observe(&mut self, syscall_nr: u32) {
        self.total_observations += 1;

        if self.recent_syscalls.len() >= MARKOV_MEMORY {
            let ctx = StateContext {
                syscalls: self.recent_syscalls.clone(),
            };
            let ctx_hash = ctx.hash();
            let row = self
                .transitions
                .entry(ctx_hash)
                .or_insert_with(TransitionRow::new);
            row.record(syscall_nr);

            let ent = row.entropy();
            self.entropy_ema = EMA_ALPHA * ent + (1.0 - EMA_ALPHA) * self.entropy_ema;
        }

        self.recent_syscalls.push(syscall_nr);
        if self.recent_syscalls.len() > MARKOV_MEMORY {
            self.recent_syscalls.remove(0);
        }

        // Limit transition table size
        if self.transitions.len() > MAX_STATES {
            let smallest = self
                .transitions
                .iter()
                .min_by_key(|(_, row)| row.total)
                .map(|(&k, _)| k);
            if let Some(k) = smallest {
                self.transitions.remove(&k);
            }
        }
    }

    fn predict_next(&self) -> Option<(u32, f32)> {
        if self.recent_syscalls.len() < MARKOV_MEMORY {
            return None;
        }
        let ctx = StateContext {
            syscalls: self.recent_syscalls.clone(),
        };
        let ctx_hash = ctx.hash();
        self.transitions
            .get(&ctx_hash)
            .and_then(|row| row.most_likely())
    }

    fn predict_sequence(&self, n: usize, rng: &mut u64) -> Vec<(u32, f32)> {
        let mut result = Vec::new();
        let mut current_ctx = self.recent_syscalls.clone();
        let steps = n.min(MAX_PROJECTION_LENGTH);

        for _ in 0..steps {
            if current_ctx.len() < MARKOV_MEMORY {
                break;
            }
            let ctx = StateContext {
                syscalls: current_ctx.clone(),
            };
            let ctx_hash = ctx.hash();

            match self.transitions.get(&ctx_hash) {
                Some(row) => {
                    let top = row.top_n(3);
                    if top.is_empty() {
                        break;
                    }
                    // Weighted random selection from top candidates
                    let total_w: f32 = top.iter().map(|(_, p)| *p).sum();
                    let threshold = (xorshift64(rng) % 1000) as f32 / 1000.0 * total_w;
                    let mut cumulative = 0.0f32;
                    let mut chosen = top[0];
                    for &(s, p) in &top {
                        cumulative += p;
                        if cumulative >= threshold {
                            chosen = (s, p);
                            break;
                        }
                    }
                    result.push(chosen);
                    current_ctx.push(chosen.0);
                    if current_ctx.len() > MARKOV_MEMORY {
                        current_ctx.remove(0);
                    }
                },
                None => break,
            }
        }
        result
    }

    fn branch_points(&self) -> Vec<(u64, f32)> {
        let mut points = Vec::new();
        for (&ctx_hash, row) in &self.transitions {
            let top = row.top_n(2);
            if top.len() >= 2 && top[1].1 >= BRANCH_THRESHOLD {
                let branch_prob = top[1].1;
                points.push((ctx_hash, branch_prob));
            }
        }
        points.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        points
    }
}

// ============================================================================
// TIMELINE STATS
// ============================================================================

/// Aggregate timeline projection statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct TimelineStats {
    pub tracked_processes: u32,
    pub total_observations: u64,
    pub total_projections: u64,
    pub avg_entropy: f32,
    pub avg_prediction_confidence: f32,
    pub branch_point_count: u32,
    pub total_transitions: u64,
}

// ============================================================================
// BRIDGE TIMELINE
// ============================================================================

/// Syscall timeline projection engine using higher-order Markov chains.
/// Projects per-process syscall sequences into the future, identifies
/// branch points, and measures timeline entropy.
#[derive(Debug)]
pub struct BridgeTimeline {
    timelines: BTreeMap<u64, ProcessTimeline>,
    tick: u64,
    total_observations: u64,
    total_projections: u64,
    avg_confidence_ema: f32,
    rng_state: u64,
}

impl BridgeTimeline {
    pub fn new() -> Self {
        Self {
            timelines: BTreeMap::new(),
            tick: 0,
            total_observations: 0,
            total_projections: 0,
            avg_confidence_ema: 0.5,
            rng_state: 0xA5A5_5A5A_1234_CDEF,
        }
    }

    /// Record a syscall observation for a process
    pub fn observe(&mut self, process_id: u64, syscall_nr: u32) {
        self.tick += 1;
        self.total_observations += 1;

        let timeline = self
            .timelines
            .entry(process_id)
            .or_insert_with(|| ProcessTimeline::new(process_id));
        timeline.observe(syscall_nr);

        if self.timelines.len() > MAX_PROCESSES {
            let least = self
                .timelines
                .iter()
                .min_by_key(|(_, t)| t.total_observations)
                .map(|(&k, _)| k);
            if let Some(k) = least {
                self.timelines.remove(&k);
            }
        }
    }

    /// Project the next N syscalls for a process
    pub fn project_sequence(&mut self, process_id: u64, n: usize) -> Vec<(u32, f32)> {
        self.total_projections += 1;
        let rng = &mut self.rng_state;
        match self.timelines.get(&process_id) {
            Some(timeline) => {
                let seq = timeline.predict_sequence(n, rng);
                if let Some(last) = seq.last() {
                    self.avg_confidence_ema =
                        EMA_ALPHA * last.1 + (1.0 - EMA_ALPHA) * self.avg_confidence_ema;
                }
                seq
            },
            None => Vec::new(),
        }
    }

    /// Get the N most likely next syscalls for a process
    pub fn likely_next_n(&self, process_id: u64, n: usize) -> Vec<(u32, f32)> {
        match self.timelines.get(&process_id) {
            Some(timeline) => {
                if timeline.recent_syscalls.len() < MARKOV_MEMORY {
                    return Vec::new();
                }
                let ctx = StateContext {
                    syscalls: timeline.recent_syscalls.clone(),
                };
                let ctx_hash = ctx.hash();
                timeline
                    .transitions
                    .get(&ctx_hash)
                    .map(|row| row.top_n(n))
                    .unwrap_or_default()
            },
            None => Vec::new(),
        }
    }

    /// Get branching points for a process timeline
    pub fn timeline_branch(&self, process_id: u64) -> Vec<(u64, f32)> {
        self.timelines
            .get(&process_id)
            .map(|t| t.branch_points())
            .unwrap_or_default()
    }

    /// Find convergence point: where two divergent sequences rejoin
    pub fn convergence_point(
        &mut self,
        process_id: u64,
        alt_context: &[u32],
    ) -> Option<(usize, u32)> {
        let timeline = self.timelines.get(&process_id)?;
        let rng = &mut self.rng_state;

        let main_seq = timeline.predict_sequence(CONVERGENCE_SCAN_DEPTH, rng);
        let mut alt_ctx_full = alt_context.to_vec();
        while alt_ctx_full.len() < MARKOV_MEMORY {
            alt_ctx_full.insert(0, 0);
        }
        if alt_ctx_full.len() > MARKOV_MEMORY {
            let skip = alt_ctx_full.len() - MARKOV_MEMORY;
            alt_ctx_full = alt_ctx_full[skip..].to_vec();
        }

        // Simulate alt branch
        let mut alt_seq = Vec::new();
        let mut current = alt_ctx_full;
        for _ in 0..CONVERGENCE_SCAN_DEPTH {
            let ctx = StateContext {
                syscalls: current.clone(),
            };
            let ctx_hash = ctx.hash();
            match timeline.transitions.get(&ctx_hash) {
                Some(row) => {
                    if let Some((s, _)) = row.most_likely() {
                        alt_seq.push(s);
                        current.push(s);
                        if current.len() > MARKOV_MEMORY {
                            current.remove(0);
                        }
                    } else {
                        break;
                    }
                },
                None => break,
            }
        }

        // Find first common syscall at the same position
        for (i, &(main_s, _)) in main_seq.iter().enumerate() {
            if i < alt_seq.len() && alt_seq[i] == main_s {
                return Some((i, main_s));
            }
        }
        None
    }

    /// Compute timeline entropy for a process
    pub fn timeline_entropy(&self, process_id: u64) -> f32 {
        self.timelines
            .get(&process_id)
            .map(|t| t.entropy_ema)
            .unwrap_or(0.0)
    }

    /// Aggregate timeline statistics
    pub fn stats(&self) -> TimelineStats {
        let avg_ent = if self.timelines.is_empty() {
            0.0
        } else {
            self.timelines.values().map(|t| t.entropy_ema).sum::<f32>()
                / self.timelines.len() as f32
        };
        let branch_count: u32 = self
            .timelines
            .values()
            .map(|t| t.branch_points().len() as u32)
            .sum();
        let total_trans: u64 = self
            .timelines
            .values()
            .map(|t| t.transitions.values().map(|r| r.total).sum::<u64>())
            .sum();

        TimelineStats {
            tracked_processes: self.timelines.len() as u32,
            total_observations: self.total_observations,
            total_projections: self.total_projections,
            avg_entropy: avg_ent,
            avg_prediction_confidence: self.avg_confidence_ema,
            branch_point_count: branch_count,
            total_transitions: total_trans,
        }
    }
}
