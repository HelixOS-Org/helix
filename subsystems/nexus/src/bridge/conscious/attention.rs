// SPDX-License-Identifier: GPL-2.0
//! # Bridge Attention Engine
//!
//! Selective attention for bridge monitoring. The bridge cannot monitor every
//! syscall with equal depth — it must choose where to focus. This module
//! implements an attention scheduling system with two modes:
//!
//! - **Spotlight attention**: focus deeply on one target (expensive analysis)
//! - **Distributed attention**: scan many targets lightly (quick pass)
//!
//! Attention is a limited resource (budget = 100 units). Allocation decisions
//! are driven by salience scoring: novel patterns, high-impact syscalls, and
//! anomalies attract attention. Unused attention decays over time, ensuring
//! the system does not fixate on stale targets.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const ATTENTION_BUDGET: u32 = 100;
const MAX_FOCUS_TARGETS: usize = 64;
const MAX_ATTENTION_HISTORY: usize = 256;
const SPOTLIGHT_COST: u32 = 25;
const DISTRIBUTED_COST_PER_TARGET: u32 = 3;
const DEFAULT_DECAY_RATE: f32 = 0.05;
const SALIENCE_NOVELTY_WEIGHT: f32 = 0.4;
const SALIENCE_IMPACT_WEIGHT: f32 = 0.35;
const SALIENCE_ANOMALY_WEIGHT: f32 = 0.25;
const EMA_ALPHA: f32 = 0.12;
const MIN_SALIENCE_THRESHOLD: f32 = 0.10;
const SPOTLIGHT_DEPTH_MULTIPLIER: f32 = 4.0;
const SHIFT_COOLDOWN_TICKS: u64 = 5;
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

fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

fn ema_update(current: f32, sample: f32, alpha: f32) -> f32 {
    current * (1.0 - alpha) + sample * alpha
}

// ============================================================================
// ATTENTION MODE
// ============================================================================

/// How the bridge allocates its attention
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttentionMode {
    /// Deep focus on a single target
    Spotlight,
    /// Light monitoring of many targets
    Distributed,
    /// Transitioning between modes
    Shifting,
    /// No active attention (idle)
    Idle,
}

// ============================================================================
// ATTENTION FOCUS
// ============================================================================

/// A single focus of attention
#[derive(Debug, Clone)]
pub struct AttentionFocus {
    pub target: String,
    pub target_hash: u64,
    pub priority: f32,
    pub duration_ticks: u64,
    pub decay_rate: f32,
    pub allocated_budget: u32,
    pub salience: f32,
    pub depth: f32,
    pub created_tick: u64,
    pub last_refresh_tick: u64,
}

impl AttentionFocus {
    fn new(target: &str, priority: f32, budget: u32, tick: u64) -> Self {
        Self {
            target: String::from(target),
            target_hash: fnv1a_hash(target.as_bytes()),
            priority: priority.clamp(0.0, 1.0),
            duration_ticks: 0,
            decay_rate: DEFAULT_DECAY_RATE,
            allocated_budget: budget,
            salience: priority,
            depth: 1.0,
            created_tick: tick,
            last_refresh_tick: tick,
        }
    }

    fn tick(&mut self) {
        self.duration_ticks += 1;
        self.salience = (self.salience - self.decay_rate).max(0.0);
    }

    fn refresh(&mut self, new_salience: f32, tick: u64) {
        self.salience = (self.salience + new_salience * 0.5).clamp(0.0, 1.0);
        self.last_refresh_tick = tick;
    }

    fn is_active(&self) -> bool {
        self.salience > MIN_SALIENCE_THRESHOLD
    }
}

// ============================================================================
// ATTENTION HISTORY ENTRY
// ============================================================================

#[derive(Debug, Clone)]
struct AttentionHistoryEntry {
    target_hash: u64,
    mode: AttentionMode,
    salience: f32,
    tick: u64,
    budget_used: u32,
}

// ============================================================================
// SALIENCE SIGNAL
// ============================================================================

/// An input signal contributing to salience computation
#[derive(Debug, Clone)]
pub struct SalienceSignal {
    pub source: String,
    pub novelty: f32,
    pub impact: f32,
    pub anomaly: f32,
}

impl SalienceSignal {
    fn composite_score(&self) -> f32 {
        self.novelty * SALIENCE_NOVELTY_WEIGHT
            + self.impact * SALIENCE_IMPACT_WEIGHT
            + self.anomaly * SALIENCE_ANOMALY_WEIGHT
    }
}

// ============================================================================
// STATS
// ============================================================================

/// Attention engine statistics
#[derive(Debug, Clone)]
pub struct AttentionStats {
    pub total_focus_events: u64,
    pub total_shifts: u64,
    pub budget_remaining: u32,
    pub active_targets: usize,
    pub current_mode: AttentionMode,
    pub spotlight_time_ratio: f32,
    pub avg_salience: f32,
}

// ============================================================================
// BRIDGE ATTENTION ENGINE
// ============================================================================

/// Manages selective attention for bridge monitoring and analysis
#[derive(Debug, Clone)]
pub struct BridgeAttentionEngine {
    focus_targets: BTreeMap<u64, AttentionFocus>,
    history: Vec<AttentionHistoryEntry>,
    current_mode: AttentionMode,
    spotlight_target: Option<u64>,
    budget_remaining: u32,
    current_tick: u64,
    total_focus_events: u64,
    total_shifts: u64,
    last_shift_tick: u64,
    spotlight_ticks: u64,
    distributed_ticks: u64,
    avg_salience_ema: f32,
    salience_signals: BTreeMap<u64, f32>,
    rng_state: u64,
}

impl BridgeAttentionEngine {
    /// Create a new attention engine
    pub fn new(seed: u64) -> Self {
        Self {
            focus_targets: BTreeMap::new(),
            history: Vec::new(),
            current_mode: AttentionMode::Idle,
            spotlight_target: None,
            budget_remaining: ATTENTION_BUDGET,
            current_tick: 0,
            total_focus_events: 0,
            total_shifts: 0,
            last_shift_tick: 0,
            spotlight_ticks: 0,
            distributed_ticks: 0,
            avg_salience_ema: 0.0,
            salience_signals: BTreeMap::new(),
            rng_state: seed | 1,
        }
    }

    /// Direct attention to a specific target
    pub fn focus_attention(&mut self, target: &str, signal: &SalienceSignal) {
        self.current_tick += 1;
        self.total_focus_events += 1;

        let salience = signal.composite_score();
        self.avg_salience_ema = ema_update(self.avg_salience_ema, salience, EMA_ALPHA);

        let target_hash = fnv1a_hash(target.as_bytes());
        self.salience_signals.insert(target_hash, salience);

        if let Some(existing) = self.focus_targets.get_mut(&target_hash) {
            existing.refresh(salience, self.current_tick);
        } else if self.focus_targets.len() < MAX_FOCUS_TARGETS {
            let budget_cost = if self.current_mode == AttentionMode::Spotlight {
                SPOTLIGHT_COST
            } else {
                DISTRIBUTED_COST_PER_TARGET
            };

            if self.budget_remaining >= budget_cost {
                let focus = AttentionFocus::new(target, salience, budget_cost, self.current_tick);
                self.focus_targets.insert(target_hash, focus);
                self.budget_remaining -= budget_cost;
            }
        }

        // Record history
        if self.history.len() >= MAX_ATTENTION_HISTORY {
            self.history.remove(0);
        }
        self.history.push(AttentionHistoryEntry {
            target_hash,
            mode: self.current_mode,
            salience,
            tick: self.current_tick,
            budget_used: ATTENTION_BUDGET - self.budget_remaining,
        });

        self.decay_and_prune();
        self.update_mode_counters();
    }

    /// Shift attention — evaluate whether to change mode or spotlight target
    pub fn attention_shift(&mut self) -> AttentionMode {
        self.current_tick += 1;
        self.total_shifts += 1;
        self.last_shift_tick = self.current_tick;

        // Find highest-salience target
        let mut best_target: Option<(u64, f32)> = None;
        for (&hash, focus) in &self.focus_targets {
            match best_target {
                None => best_target = Some((hash, focus.salience)),
                Some((_, bs)) if focus.salience > bs => {
                    best_target = Some((hash, focus.salience));
                }
                _ => {}
            }
        }

        // Decide mode based on salience distribution
        if let Some((top_hash, top_salience)) = best_target {
            let second_best = self
                .focus_targets
                .iter()
                .filter(|(&h, _)| h != top_hash)
                .map(|(_, f)| f.salience)
                .fold(0.0f32, |a, b| a.max(b));

            let salience_gap = top_salience - second_best;

            if salience_gap > 0.3 && top_salience > 0.5 {
                // One clear target → spotlight mode
                self.current_mode = AttentionMode::Spotlight;
                self.spotlight_target = Some(top_hash);
                if let Some(focus) = self.focus_targets.get_mut(&top_hash) {
                    focus.depth = SPOTLIGHT_DEPTH_MULTIPLIER;
                }
            } else if self.focus_targets.len() > 3 {
                self.current_mode = AttentionMode::Distributed;
                self.spotlight_target = None;
                for focus in self.focus_targets.values_mut() {
                    focus.depth = 1.0;
                }
            } else {
                self.current_mode = AttentionMode::Idle;
                self.spotlight_target = None;
            }
        } else {
            self.current_mode = AttentionMode::Idle;
            self.spotlight_target = None;
        }

        self.current_mode
    }

    /// Compute salience score for a target
    pub fn salience_score(&self, target: &str) -> f32 {
        let hash = fnv1a_hash(target.as_bytes());
        self.salience_signals.get(&hash).copied().unwrap_or(0.0)
    }

    /// How much attention budget remains?
    pub fn attention_budget(&self) -> (u32, u32) {
        (self.budget_remaining, ATTENTION_BUDGET)
    }

    /// Get the current spotlight target, if any
    pub fn spotlight_target(&self) -> Option<&AttentionFocus> {
        self.spotlight_target
            .and_then(|hash| self.focus_targets.get(&hash))
    }

    /// Apply attention decay to all targets, prune dead ones
    pub fn attention_decay(&mut self) {
        self.current_tick += 1;
        self.decay_and_prune();
    }

    fn decay_and_prune(&mut self) {
        let mut to_remove = Vec::new();
        let mut reclaimed_budget: u32 = 0;

        for (&hash, focus) in self.focus_targets.iter_mut() {
            focus.tick();
            if !focus.is_active() {
                to_remove.push(hash);
                reclaimed_budget += focus.allocated_budget;
            }
        }

        for hash in to_remove {
            self.focus_targets.remove(&hash);
            if self.spotlight_target == Some(hash) {
                self.spotlight_target = None;
                self.current_mode = AttentionMode::Idle;
            }
        }

        self.budget_remaining = (self.budget_remaining + reclaimed_budget).min(ATTENTION_BUDGET);
    }

    fn update_mode_counters(&mut self) {
        match self.current_mode {
            AttentionMode::Spotlight => self.spotlight_ticks += 1,
            AttentionMode::Distributed => self.distributed_ticks += 1,
            _ => {}
        }
    }

    /// Refill the attention budget (called periodically)
    pub fn refill_budget(&mut self, amount: u32) {
        self.budget_remaining = (self.budget_remaining + amount).min(ATTENTION_BUDGET);
    }

    /// Compute the ratio of spotlight time to total active time
    pub fn spotlight_time_ratio(&self) -> f32 {
        let total = self.spotlight_ticks + self.distributed_ticks;
        if total == 0 {
            return 0.0;
        }
        self.spotlight_ticks as f32 / total as f32
    }

    /// Force focus on a target with maximum priority
    pub fn force_focus(&mut self, target: &str) {
        self.current_tick += 1;
        let target_hash = fnv1a_hash(target.as_bytes());

        let focus = AttentionFocus::new(target, 1.0, SPOTLIGHT_COST, self.current_tick);
        self.focus_targets.insert(target_hash, focus);
        self.spotlight_target = Some(target_hash);
        self.current_mode = AttentionMode::Spotlight;
        if let Some(f) = self.focus_targets.get_mut(&target_hash) {
            f.depth = SPOTLIGHT_DEPTH_MULTIPLIER;
        }
    }

    /// Get the number of currently active focus targets
    pub fn active_target_count(&self) -> usize {
        self.focus_targets.len()
    }

    /// Get all active focus targets sorted by salience
    pub fn ranked_targets(&self) -> Vec<(String, f32)> {
        let mut targets: Vec<(String, f32)> = self
            .focus_targets
            .values()
            .map(|f| (f.target.clone(), f.salience))
            .collect();
        targets.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        targets
    }

    /// Statistics snapshot
    pub fn stats(&self) -> AttentionStats {
        AttentionStats {
            total_focus_events: self.total_focus_events,
            total_shifts: self.total_shifts,
            budget_remaining: self.budget_remaining,
            active_targets: self.focus_targets.len(),
            current_mode: self.current_mode,
            spotlight_time_ratio: self.spotlight_time_ratio(),
            avg_salience: self.avg_salience_ema,
        }
    }

    /// Reset the attention engine completely
    pub fn reset(&mut self) {
        self.focus_targets.clear();
        self.history.clear();
        self.current_mode = AttentionMode::Idle;
        self.spotlight_target = None;
        self.budget_remaining = ATTENTION_BUDGET;
        self.avg_salience_ema = 0.0;
        self.salience_signals.clear();
    }
}
