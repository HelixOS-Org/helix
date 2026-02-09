// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Attention Engine
//!
//! Selective attention for cooperation monitoring. Not every resource contention
//! or trust fluctuation deserves equal analysis time. This engine models
//! cooperation salience, allowing the system to focus deep mediation effort on
//! the most critical contentions while passively monitoring routine sharing.
//!
//! ## Attention Targets
//!
//! Each target carries a focus level, priority class, and urgency rating.
//! The engine dynamically reallocates attention budget as cooperation
//! conditions change, ensuring critical fairness violations receive
//! immediate and intense scrutiny.
//!
//! ## Key Methods
//!
//! - `focus_on_contention()` — Direct attention to a resource contention
//! - `monitor_trust()` — Add a trust relationship to the watch list
//! - `attention_allocation()` — Redistribute attention budget optimally
//! - `critical_cooperation()` — Identify cooperation events needing immediate focus
//! - `attention_shift()` — Shift attention smoothly between targets
//! - `cooperation_salience()` — Compute salience score for a cooperation event

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.12;
const ATTENTION_DECAY: f32 = 0.985;
const MAX_TARGETS: usize = 256;
const MAX_ATTENTION_HISTORY: usize = 64;
const URGENCY_HIGH: f32 = 0.8;
const URGENCY_CRITICAL: f32 = 0.95;
const FOCUS_BUDGET: f32 = 1.0;
const SHIFT_RATE: f32 = 0.2;
const SALIENCE_BASELINE: f32 = 0.1;
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

/// Xorshift64 PRNG for attention jitter
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

// ============================================================================
// ATTENTION TARGET KIND
// ============================================================================

/// Classification of cooperation attention targets
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AttentionTargetKind {
    /// Resource contention between processes
    ResourceContention,
    /// Trust relationship requiring monitoring
    TrustRelationship,
    /// Fairness violation in progress
    FairnessViolation,
    /// Active negotiation between processes
    ActiveNegotiation,
    /// Process isolation event
    IsolationEvent,
    /// Cooperation protocol anomaly
    ProtocolAnomaly,
}

// ============================================================================
// ATTENTION TARGET
// ============================================================================

/// A single target of cooperation attention
#[derive(Debug, Clone)]
pub struct AttentionTarget {
    pub id: u64,
    pub name: String,
    pub kind: AttentionTargetKind,
    /// Current focus intensity (0.0 – 1.0)
    pub focus: f32,
    /// Priority class (higher = more important)
    pub priority: u32,
    /// Urgency rating (0.0 – 1.0)
    pub urgency: f32,
    /// Salience score (computed from context)
    pub salience: f32,
    /// Processes involved in this attention target
    pub involved_processes: Vec<u64>,
    /// Tick when first noticed
    pub noticed_tick: u64,
    /// Tick of last update
    pub last_update_tick: u64,
    /// Number of times attended
    pub attend_count: u64,
    /// EMA-smoothed importance
    pub importance_ema: f32,
    /// History of focus values
    focus_history: Vec<f32>,
    focus_write_idx: usize,
}

impl AttentionTarget {
    pub fn new(name: String, kind: AttentionTargetKind, priority: u32, tick: u64) -> Self {
        let id = fnv1a_hash(name.as_bytes());
        let mut focus_history = Vec::with_capacity(MAX_ATTENTION_HISTORY);
        for _ in 0..MAX_ATTENTION_HISTORY {
            focus_history.push(0.0);
        }
        Self {
            id,
            name,
            kind,
            focus: 0.0,
            priority,
            urgency: 0.0,
            salience: SALIENCE_BASELINE,
            involved_processes: Vec::new(),
            noticed_tick: tick,
            last_update_tick: tick,
            attend_count: 0,
            importance_ema: 0.0,
            focus_history,
            focus_write_idx: 0,
        }
    }

    /// Update urgency and recompute salience
    pub fn update_urgency(&mut self, urgency: f32, tick: u64) {
        let clamped = if urgency < 0.0 {
            0.0
        } else if urgency > 1.0 {
            1.0
        } else {
            urgency
        };
        self.urgency = clamped;
        self.salience = clamped * 0.6 + (self.priority as f32 / 10.0).min(1.0) * 0.4;
        self.last_update_tick = tick;
    }

    /// Apply focus and record history
    pub fn apply_focus(&mut self, new_focus: f32) {
        let clamped = if new_focus < 0.0 {
            0.0
        } else if new_focus > 1.0 {
            1.0
        } else {
            new_focus
        };
        self.focus = clamped;
        self.focus_history[self.focus_write_idx] = clamped;
        self.focus_write_idx = (self.focus_write_idx + 1) % MAX_ATTENTION_HISTORY;
        self.attend_count += 1;
        self.importance_ema += EMA_ALPHA * (clamped - self.importance_ema);
    }

    /// Decay focus over time
    pub fn decay_focus(&mut self, rng: &mut u64) {
        let jitter = (xorshift64(rng) % 50) as f32 / 100_000.0;
        self.focus *= ATTENTION_DECAY - jitter;
        if self.focus < 0.001 {
            self.focus = 0.0;
        }
    }

    /// Average focus from history
    pub fn average_focus(&self) -> f32 {
        let count = if self.attend_count < MAX_ATTENTION_HISTORY as u64 {
            self.attend_count as usize
        } else {
            MAX_ATTENTION_HISTORY
        };
        if count == 0 {
            return 0.0;
        }
        let mut sum = 0.0f32;
        for i in 0..count {
            sum += self.focus_history[i];
        }
        sum / count as f32
    }
}

// ============================================================================
// ATTENTION STATS
// ============================================================================

/// Statistics for the cooperation attention system
#[derive(Debug, Clone)]
pub struct CoopAttentionStats {
    pub total_targets: usize,
    pub active_targets: usize,
    pub total_focus_shifts: u64,
    pub budget_utilization: f32,
    pub critical_count: usize,
    pub avg_urgency: f32,
    pub avg_salience: f32,
    pub top_target_id: u64,
    pub attention_entropy: f32,
}

impl CoopAttentionStats {
    pub fn new() -> Self {
        Self {
            total_targets: 0,
            active_targets: 0,
            total_focus_shifts: 0,
            budget_utilization: 0.0,
            critical_count: 0,
            avg_urgency: 0.0,
            avg_salience: 0.0,
            top_target_id: 0,
            attention_entropy: 0.0,
        }
    }
}

// ============================================================================
// COOPERATION ATTENTION ENGINE
// ============================================================================

/// Engine managing selective attention for cooperation monitoring
pub struct CoopAttentionEngine {
    targets: BTreeMap<u64, AttentionTarget>,
    /// Focus budget remaining this cycle
    budget_remaining: f32,
    /// PRNG state
    rng_state: u64,
    /// Current tick
    tick: u64,
    /// Running statistics
    pub stats: CoopAttentionStats,
    /// Focus shift EMA
    shift_ema: f32,
    /// Salience threshold for automatic attention
    salience_threshold: f32,
}

impl CoopAttentionEngine {
    pub fn new(seed: u64) -> Self {
        Self {
            targets: BTreeMap::new(),
            budget_remaining: FOCUS_BUDGET,
            rng_state: seed | 1,
            tick: 0,
            stats: CoopAttentionStats::new(),
            shift_ema: 0.0,
            salience_threshold: 0.3,
        }
    }

    // ========================================================================
    // FOCUS ON CONTENTION
    // ========================================================================

    /// Direct attention to a resource contention between processes
    pub fn focus_on_contention(
        &mut self,
        contention_name: String,
        processes: Vec<u64>,
        urgency: f32,
        priority: u32,
    ) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(contention_name.as_bytes());

        if let Some(target) = self.targets.get_mut(&id) {
            target.update_urgency(urgency, self.tick);
            target.involved_processes = processes;
            return id;
        }

        if self.targets.len() >= MAX_TARGETS {
            self.evict_lowest_salience();
        }

        let mut target = AttentionTarget::new(
            contention_name,
            AttentionTargetKind::ResourceContention,
            priority,
            self.tick,
        );
        target.update_urgency(urgency, self.tick);
        target.involved_processes = processes;
        self.targets.insert(id, target);
        id
    }

    // ========================================================================
    // MONITOR TRUST
    // ========================================================================

    /// Add a trust relationship to the attention watch list
    pub fn monitor_trust(
        &mut self,
        relationship_name: String,
        process_a: u64,
        process_b: u64,
        trust_volatility: f32,
    ) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(relationship_name.as_bytes());

        if let Some(target) = self.targets.get_mut(&id) {
            target.update_urgency(trust_volatility, self.tick);
            return id;
        }

        if self.targets.len() >= MAX_TARGETS {
            self.evict_lowest_salience();
        }

        let priority = if trust_volatility > 0.7 { 8 } else { 4 };
        let mut target = AttentionTarget::new(
            relationship_name,
            AttentionTargetKind::TrustRelationship,
            priority,
            self.tick,
        );
        target.update_urgency(trust_volatility, self.tick);
        target.involved_processes.push(process_a);
        target.involved_processes.push(process_b);
        self.targets.insert(id, target);
        id
    }

    // ========================================================================
    // ATTENTION ALLOCATION
    // ========================================================================

    /// Redistribute attention budget across all targets optimally
    ///
    /// Targets with higher salience receive proportionally more focus.
    /// Budget is normalized so total focus across all targets equals FOCUS_BUDGET.
    pub fn attention_allocation(&mut self) {
        self.tick += 1;
        let total_salience: f32 = self.targets.values().map(|t| t.salience).sum();
        if total_salience < 0.001 {
            return;
        }

        let inv_salience = FOCUS_BUDGET / total_salience;
        let target_ids: Vec<u64> = self.targets.keys().copied().collect();

        let mut allocated = 0.0f32;
        for tid in target_ids.iter() {
            if let Some(target) = self.targets.get_mut(tid) {
                let focus = target.salience * inv_salience;
                let shift = (focus - target.focus) * SHIFT_RATE;
                let new_focus = target.focus + shift;
                target.apply_focus(new_focus);
                allocated += new_focus;
            }
        }

        self.budget_remaining = (FOCUS_BUDGET - allocated).max(0.0);
        self.update_stats();
    }

    // ========================================================================
    // CRITICAL COOPERATION
    // ========================================================================

    /// Identify cooperation events that need immediate attention
    pub fn critical_cooperation(&self) -> Vec<u64> {
        let mut critical = Vec::new();
        for (id, target) in self.targets.iter() {
            if target.urgency >= URGENCY_CRITICAL {
                critical.push(*id);
            }
        }
        // Sort by urgency descending (stable since IDs are unique)
        critical.sort_by(|a, b| {
            let ua = self.targets.get(a).map(|t| t.urgency).unwrap_or(0.0);
            let ub = self.targets.get(b).map(|t| t.urgency).unwrap_or(0.0);
            ub.partial_cmp(&ua).unwrap_or(core::cmp::Ordering::Equal)
        });
        critical
    }

    /// Count of urgent targets (above URGENCY_HIGH)
    pub fn urgent_count(&self) -> usize {
        self.targets
            .values()
            .filter(|t| t.urgency >= URGENCY_HIGH)
            .count()
    }

    // ========================================================================
    // ATTENTION SHIFT
    // ========================================================================

    /// Shift attention smoothly from one target to another
    pub fn attention_shift(&mut self, from_id: u64, to_id: u64, amount: f32) -> bool {
        let clamped = if amount < 0.0 {
            0.0
        } else if amount > 1.0 {
            1.0
        } else {
            amount
        };

        let from_focus = if let Some(t) = self.targets.get(&from_id) {
            t.focus
        } else {
            return false;
        };

        let transfer = from_focus.min(clamped);

        if let Some(from) = self.targets.get_mut(&from_id) {
            from.apply_focus(from.focus - transfer);
        }
        if let Some(to) = self.targets.get_mut(&to_id) {
            to.apply_focus((to.focus + transfer).min(1.0));
        }

        self.shift_ema += EMA_ALPHA * (transfer - self.shift_ema);
        self.stats.total_focus_shifts += 1;
        true
    }

    // ========================================================================
    // COOPERATION SALIENCE
    // ========================================================================

    /// Compute salience score for a cooperation event
    ///
    /// Salience considers urgency, number of processes involved,
    /// recency, and target kind weight.
    pub fn cooperation_salience(
        &self,
        urgency: f32,
        process_count: usize,
        recency_ticks: u64,
        kind: AttentionTargetKind,
    ) -> f32 {
        let kind_weight = match kind {
            AttentionTargetKind::FairnessViolation => 1.0,
            AttentionTargetKind::ResourceContention => 0.85,
            AttentionTargetKind::ProtocolAnomaly => 0.9,
            AttentionTargetKind::TrustRelationship => 0.7,
            AttentionTargetKind::ActiveNegotiation => 0.6,
            AttentionTargetKind::IsolationEvent => 0.75,
        };

        let process_factor = (process_count as f32 / 10.0).min(1.0);
        let recency_factor = if recency_ticks == 0 {
            1.0
        } else {
            1.0 / (1.0 + recency_ticks as f32 / 100.0)
        };

        let raw = urgency * 0.4 + kind_weight * 0.25 + process_factor * 0.2 + recency_factor * 0.15;
        if raw < 0.0 {
            0.0
        } else if raw > 1.0 {
            1.0
        } else {
            raw
        }
    }

    // ========================================================================
    // DECAY & MAINTENANCE
    // ========================================================================

    /// Decay all target focus levels
    pub fn decay_all(&mut self) {
        let rng = &mut self.rng_state;
        for (_, target) in self.targets.iter_mut() {
            target.decay_focus(rng);
        }
    }

    /// Evict the target with the lowest salience
    fn evict_lowest_salience(&mut self) {
        let mut lowest_id: Option<u64> = None;
        let mut lowest_sal = f32::MAX;
        for (id, target) in self.targets.iter() {
            if target.salience < lowest_sal {
                lowest_sal = target.salience;
                lowest_id = Some(*id);
            }
        }
        if let Some(id) = lowest_id {
            self.targets.remove(&id);
        }
    }

    /// Prune targets not updated within max_age ticks
    pub fn prune_stale(&mut self, max_age: u64) {
        let cutoff = if self.tick > max_age {
            self.tick - max_age
        } else {
            0
        };
        let stale: Vec<u64> = self
            .targets
            .iter()
            .filter(|(_, t)| t.last_update_tick < cutoff)
            .map(|(k, _)| *k)
            .collect();
        for key in stale {
            self.targets.remove(&key);
        }
    }

    // ========================================================================
    // QUERIES
    // ========================================================================

    /// Get a target by ID
    pub fn target(&self, id: u64) -> Option<&AttentionTarget> {
        self.targets.get(&id)
    }

    /// Total number of tracked targets
    pub fn target_count(&self) -> usize {
        self.targets.len()
    }

    /// Snapshot of attention statistics
    pub fn snapshot_stats(&self) -> CoopAttentionStats {
        self.stats.clone()
    }

    // ========================================================================
    // STATS
    // ========================================================================

    fn update_stats(&mut self) {
        let count = self.targets.len();
        self.stats.total_targets = count;
        if count == 0 {
            return;
        }

        let mut active = 0usize;
        let mut critical = 0usize;
        let mut sum_urgency = 0.0f32;
        let mut sum_salience = 0.0f32;
        let mut top_id = 0u64;
        let mut top_focus = -1.0f32;
        let mut focus_sum = 0.0f32;

        for (id, target) in self.targets.iter() {
            if target.focus > 0.01 {
                active += 1;
            }
            if target.urgency >= URGENCY_CRITICAL {
                critical += 1;
            }
            sum_urgency += target.urgency;
            sum_salience += target.salience;
            if target.focus > top_focus {
                top_focus = target.focus;
                top_id = *id;
            }
            focus_sum += target.focus;
        }

        self.stats.active_targets = active;
        self.stats.critical_count = critical;
        self.stats.avg_urgency = sum_urgency / count as f32;
        self.stats.avg_salience = sum_salience / count as f32;
        self.stats.top_target_id = top_id;
        self.stats.budget_utilization = focus_sum / FOCUS_BUDGET;

        // Compute attention entropy: -sum(p * ln(p))
        let mut entropy = 0.0f32;
        if focus_sum > 0.001 {
            for (_, target) in self.targets.iter() {
                let p = target.focus / focus_sum;
                if p > 0.001 {
                    // Approximate ln using series: ln(x) ≈ 2*((x-1)/(x+1)) for x near 1
                    let ratio = (p - 1.0) / (p + 1.0);
                    let ln_approx = 2.0 * (ratio + ratio * ratio * ratio / 3.0);
                    entropy -= p * ln_approx;
                }
            }
        }
        self.stats.attention_entropy = if entropy < 0.0 { 0.0 } else { entropy };
    }
}
