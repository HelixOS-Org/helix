// SPDX-License-Identifier: GPL-2.0
//! # Apps Attention Engine
//!
//! Selective attention for application monitoring. In a system with hundreds
//! or thousands of running applications, deep analysis of every single process
//! every tick is computationally infeasible. The attention engine solves this
//! by allocating a finite "attention budget" across all tracked applications.
//!
//! Hot applications (high resource usage, recent anomalies, SLA violations)
//! receive **spotlight attention** — deep, per-tick analysis with full feature
//! extraction. Cold applications (stable, well-understood, low priority) get
//! **periodic scans** — lightweight probes at exponentially increasing intervals.
//!
//! The engine adapts its allocation using salience scores: a composite of
//! resource impact, novelty, priority class, and recent event density.
//! Attention shifts are logged and their efficiency is tracked so the engine
//! can learn which attention patterns yield the best classification accuracy.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.12;
const MAX_APPS: usize = 1024;
const DEFAULT_BUDGET: u32 = 1000;
const HOT_SALIENCE_THRESHOLD: f32 = 0.7;
const COLD_SCAN_INTERVAL_BASE: u64 = 16;
const COLD_SCAN_INTERVAL_MAX: u64 = 512;
const ATTENTION_DECAY: f32 = 0.99;
const MAX_SHIFT_LOG: usize = 256;
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

/// Xorshift64 PRNG for stochastic attention sampling
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

// ============================================================================
// ATTENTION TYPES
// ============================================================================

/// Attention allocation tier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AttentionTier {
    /// Full per-tick deep analysis
    Spotlight,
    /// Moderate frequency analysis
    Focused,
    /// Periodic lightweight scans
    Peripheral,
    /// Minimal monitoring — exponentially spaced probes
    Cold,
}

impl AttentionTier {
    pub fn label(&self) -> &'static str {
        match self {
            AttentionTier::Spotlight => "spotlight",
            AttentionTier::Focused => "focused",
            AttentionTier::Peripheral => "peripheral",
            AttentionTier::Cold => "cold",
        }
    }

    fn base_cost(&self) -> u32 {
        match self {
            AttentionTier::Spotlight => 20,
            AttentionTier::Focused => 8,
            AttentionTier::Peripheral => 3,
            AttentionTier::Cold => 1,
        }
    }
}

/// A single attention allocation record
#[derive(Debug, Clone)]
pub struct AttentionAllocation {
    pub app_id: u64,
    pub app_name: String,
    pub attention_units: u32,
    pub reason: String,
    pub tier: AttentionTier,
    pub salience: f32,
}

/// Logged attention shift event
#[derive(Debug, Clone)]
pub struct AttentionShift {
    pub tick: u64,
    pub app_id: u64,
    pub from_tier: AttentionTier,
    pub to_tier: AttentionTier,
    pub trigger_salience: f32,
    pub was_beneficial: bool,
}

// ============================================================================
// PER-APP ATTENTION STATE
// ============================================================================

/// Tracked attention state for a single application
#[derive(Debug, Clone)]
pub struct AppAttentionState {
    pub app_id: u64,
    pub app_name: String,
    pub tier: AttentionTier,
    pub salience: f32,
    pub resource_impact: f32,
    pub novelty_signal: f32,
    pub priority_class: f32,
    pub event_density: f32,
    pub attention_units: u32,
    pub scan_interval: u64,
    pub last_scan_tick: u64,
    pub ticks_in_tier: u64,
    pub beneficial_shifts: u64,
    pub total_shifts: u64,
    salience_history: Vec<f32>,
    write_idx: usize,
}

impl AppAttentionState {
    fn new(app_id: u64, app_name: String) -> Self {
        Self {
            app_id,
            app_name,
            tier: AttentionTier::Peripheral,
            salience: 0.3,
            resource_impact: 0.0,
            novelty_signal: 0.0,
            priority_class: 0.5,
            event_density: 0.0,
            attention_units: AttentionTier::Peripheral.base_cost(),
            scan_interval: COLD_SCAN_INTERVAL_BASE,
            last_scan_tick: 0,
            ticks_in_tier: 0,
            beneficial_shifts: 0,
            total_shifts: 0,
            salience_history: Vec::new(),
            write_idx: 0,
        }
    }

    fn update_salience(&mut self, resource_impact: f32, novelty: f32, priority: f32, events: f32) {
        self.resource_impact = resource_impact;
        self.novelty_signal = novelty;
        self.priority_class = priority;
        self.event_density = events;

        // Weighted composite salience
        let raw = 0.30 * resource_impact + 0.25 * novelty + 0.25 * priority + 0.20 * events;

        self.salience = EMA_ALPHA * raw + (1.0 - EMA_ALPHA) * self.salience;

        // History ring
        if self.salience_history.len() < 128 {
            self.salience_history.push(self.salience);
        } else {
            self.salience_history[self.write_idx] = self.salience;
        }
        self.write_idx = (self.write_idx + 1) % 128;
    }

    fn determine_tier(&self) -> AttentionTier {
        if self.salience > HOT_SALIENCE_THRESHOLD {
            AttentionTier::Spotlight
        } else if self.salience > 0.5 {
            AttentionTier::Focused
        } else if self.salience > 0.2 {
            AttentionTier::Peripheral
        } else {
            AttentionTier::Cold
        }
    }

    fn salience_trend(&self) -> f32 {
        if self.salience_history.len() < 4 {
            return 0.0;
        }
        let len = self.salience_history.len();
        let mid = len / 2;
        let first: f32 = self.salience_history[..mid].iter().sum::<f32>() / mid as f32;
        let second: f32 = self.salience_history[mid..].iter().sum::<f32>() / (len - mid) as f32;
        second - first
    }
}

// ============================================================================
// STATS
// ============================================================================

/// Aggregate attention engine statistics
#[derive(Debug, Clone)]
pub struct AttentionStats {
    pub total_apps: usize,
    pub spotlight_count: usize,
    pub focused_count: usize,
    pub peripheral_count: usize,
    pub cold_count: usize,
    pub budget_total: u32,
    pub budget_used: u32,
    pub budget_utilization: f32,
    pub total_shifts: u64,
    pub beneficial_shift_rate: f32,
    pub mean_salience: f32,
}

// ============================================================================
// APPS ATTENTION ENGINE
// ============================================================================

/// Selective attention engine for application monitoring
#[derive(Debug)]
pub struct AppsAttentionEngine {
    apps: BTreeMap<u64, AppAttentionState>,
    budget: u32,
    budget_used: u32,
    shift_log: Vec<AttentionShift>,
    shift_write_idx: usize,
    total_shifts: u64,
    beneficial_shifts: u64,
    tick: u64,
    rng_state: u64,
}

impl AppsAttentionEngine {
    pub fn new(budget: u32, seed: u64) -> Self {
        Self {
            apps: BTreeMap::new(),
            budget: if budget == 0 { DEFAULT_BUDGET } else { budget },
            budget_used: 0,
            shift_log: Vec::new(),
            shift_write_idx: 0,
            total_shifts: 0,
            beneficial_shifts: 0,
            tick: 0,
            rng_state: if seed == 0 {
                0xA77E_CAFE_1234_5678
            } else {
                seed
            },
        }
    }

    /// Allocate attention for a single app based on input signals
    pub fn allocate_attention(
        &mut self,
        app_id: u64,
        app_name: &str,
        resource_impact: f32,
        novelty: f32,
        priority: f32,
        event_density: f32,
    ) -> AttentionAllocation {
        self.tick += 1;

        let state = self
            .apps
            .entry(app_id)
            .or_insert_with(|| AppAttentionState::new(app_id, String::from(app_name)));

        state.update_salience(resource_impact, novelty, priority, event_density);
        let new_tier = state.determine_tier();

        let old_tier = state.tier;
        if new_tier != old_tier {
            let shift = AttentionShift {
                tick: self.tick,
                app_id,
                from_tier: old_tier,
                to_tier: new_tier,
                trigger_salience: state.salience,
                was_beneficial: false, // evaluated retroactively
            };
            self.log_shift(shift);
            state.tier = new_tier;
            state.ticks_in_tier = 0;
            state.total_shifts += 1;
        } else {
            state.ticks_in_tier += 1;
        }

        let units = new_tier.base_cost();
        state.attention_units = units;
        state.last_scan_tick = self.tick;

        // Cold apps get exponentially spaced scans
        if new_tier == AttentionTier::Cold {
            state.scan_interval = (state.scan_interval * 2).min(COLD_SCAN_INTERVAL_MAX);
        } else {
            state.scan_interval = COLD_SCAN_INTERVAL_BASE;
        }

        let reason = match new_tier {
            AttentionTier::Spotlight => String::from("high salience — deep analysis"),
            AttentionTier::Focused => String::from("moderate salience — focused scan"),
            AttentionTier::Peripheral => String::from("low salience — periodic probe"),
            AttentionTier::Cold => String::from("minimal salience — cold monitor"),
        };

        // Evict if over capacity
        if self.apps.len() > MAX_APPS {
            self.evict_coldest(app_id);
        }

        AttentionAllocation {
            app_id,
            app_name: String::from(app_name),
            attention_units: units,
            reason,
            tier: new_tier,
            salience: state.salience,
        }
    }

    /// Perform an attention shift — move an app to a specific tier
    pub fn attention_shift(&mut self, app_id: u64, target_tier: AttentionTier) -> bool {
        if let Some(state) = self.apps.get_mut(&app_id) {
            let old_tier = state.tier;
            if old_tier == target_tier {
                return false;
            }
            let shift = AttentionShift {
                tick: self.tick,
                app_id,
                from_tier: old_tier,
                to_tier: target_tier,
                trigger_salience: state.salience,
                was_beneficial: false,
            };
            self.log_shift(shift);
            state.tier = target_tier;
            state.ticks_in_tier = 0;
            state.attention_units = target_tier.base_cost();
            state.total_shifts += 1;
            true
        } else {
            false
        }
    }

    /// Compute salience score for a specific app
    pub fn app_salience(&self, app_id: u64) -> Option<f32> {
        self.apps.get(&app_id).map(|s| s.salience)
    }

    /// Return the current attention budget state
    pub fn attention_budget(&self) -> (u32, u32, f32) {
        let used: u32 = self.apps.values().map(|a| a.attention_units).sum();
        let utilization = if self.budget > 0 {
            used as f32 / self.budget as f32
        } else {
            0.0
        };
        (self.budget, used, utilization)
    }

    /// Detect all hot apps (spotlight-tier)
    pub fn hot_app_detection(&self) -> Vec<(u64, f32)> {
        let mut hot = Vec::new();
        for (id, state) in &self.apps {
            if state.tier == AttentionTier::Spotlight {
                hot.push((*id, state.salience));
            }
        }
        hot.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        hot
    }

    /// Compute attention efficiency — fraction of shifts that were beneficial
    pub fn attention_efficiency(&self) -> f32 {
        if self.total_shifts == 0 {
            return 1.0;
        }
        self.beneficial_shifts as f32 / self.total_shifts as f32
    }

    /// Mark a recent shift as beneficial (called retroactively when classification improves)
    pub fn mark_shift_beneficial(&mut self, app_id: u64) {
        self.beneficial_shifts += 1;
        if let Some(state) = self.apps.get_mut(&app_id) {
            state.beneficial_shifts += 1;
        }
        // Also mark the most recent shift for this app in the log
        for shift in self.shift_log.iter_mut().rev() {
            if shift.app_id == app_id && !shift.was_beneficial {
                shift.was_beneficial = true;
                break;
            }
        }
    }

    /// Full stats
    pub fn stats(&self) -> AttentionStats {
        let (budget_total, budget_used, budget_utilization) = self.attention_budget();
        let mut spotlight = 0usize;
        let mut focused = 0usize;
        let mut peripheral = 0usize;
        let mut cold = 0usize;
        let mut salience_sum = 0.0_f32;

        for (_, state) in &self.apps {
            match state.tier {
                AttentionTier::Spotlight => spotlight += 1,
                AttentionTier::Focused => focused += 1,
                AttentionTier::Peripheral => peripheral += 1,
                AttentionTier::Cold => cold += 1,
            }
            salience_sum += state.salience;
        }

        let n = self.apps.len().max(1) as f32;

        AttentionStats {
            total_apps: self.apps.len(),
            spotlight_count: spotlight,
            focused_count: focused,
            peripheral_count: peripheral,
            cold_count: cold,
            budget_total,
            budget_used,
            budget_utilization,
            total_shifts: self.total_shifts,
            beneficial_shift_rate: self.attention_efficiency(),
            mean_salience: salience_sum / n,
        }
    }

    /// Salience trend for a specific app
    pub fn salience_trend(&self, app_id: u64) -> Option<f32> {
        self.apps.get(&app_id).map(|s| s.salience_trend())
    }

    /// Decay all salience values (call periodically)
    pub fn decay_all(&mut self) {
        for (_, state) in self.apps.iter_mut() {
            state.salience *= ATTENTION_DECAY;
            let new_tier = state.determine_tier();
            if new_tier != state.tier {
                state.tier = new_tier;
                state.attention_units = new_tier.base_cost();
                state.ticks_in_tier = 0;
            }
        }
    }

    /// Number of tracked apps
    pub fn app_count(&self) -> usize {
        self.apps.len()
    }

    // ========================================================================
    // INTERNAL
    // ========================================================================

    fn log_shift(&mut self, shift: AttentionShift) {
        self.total_shifts += 1;
        if self.shift_log.len() < MAX_SHIFT_LOG {
            self.shift_log.push(shift);
        } else {
            self.shift_log[self.shift_write_idx] = shift;
        }
        self.shift_write_idx = (self.shift_write_idx + 1) % MAX_SHIFT_LOG;
    }

    fn evict_coldest(&mut self, keep_id: u64) {
        let mut coldest_id = 0u64;
        let mut coldest_salience = f32::MAX;
        for (id, state) in &self.apps {
            if *id != keep_id && state.salience < coldest_salience {
                coldest_salience = state.salience;
                coldest_id = *id;
            }
        }
        if coldest_id != 0 {
            self.apps.remove(&coldest_id);
        }
    }

    /// Stochastic exploration: randomly bump a cold app to peripheral for re-evaluation
    pub fn explore_cold_app(&mut self) -> Option<u64> {
        let cold_apps: Vec<u64> = self
            .apps
            .iter()
            .filter(|(_, s)| s.tier == AttentionTier::Cold)
            .map(|(id, _)| *id)
            .collect();

        if cold_apps.is_empty() {
            return None;
        }

        let idx = (xorshift64(&mut self.rng_state) as usize) % cold_apps.len();
        let chosen = cold_apps[idx];
        self.attention_shift(chosen, AttentionTier::Peripheral);
        Some(chosen)
    }
}
