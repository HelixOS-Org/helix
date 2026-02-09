// SPDX-License-Identifier: GPL-2.0
//! # Holistic Attention Engine
//!
//! **SYSTEM-WIDE attention allocation.** The kernel possesses limited cognitive
//! resources — introspection cycles, analysis bandwidth, optimization passes.
//! This engine allocates those resources across ALL subsystems using a global
//! attention budget, priority queue, and salience computation.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │              GLOBAL ATTENTION BUDGET                         │
//! ├──────────────────────────────────────────────────────────────┤
//! │  Salience Ranking ──▶ Priority Queue ──▶ Allocation         │
//! │       │                    │                  │              │
//! │       ▼                    ▼                  ▼              │
//! │  "What matters    "What order?"      "How much focus?"      │
//! │   right now?"                                               │
//! │                                                             │
//! │  Cognitive Load ──▶ Attention Crisis ──▶ Focus Shift        │
//! │       │                    │                  │              │
//! │       ▼                    ▼                  ▼              │
//! │  "How overloaded?"  "Emergency!"      "Redirect NOW"        │
//! └──────────────────────────────────────────────────────────────┘
//! ```
//!
//! Attention is the scarcest kernel resource. This engine ensures we spend
//! it wisely.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.10;
const MAX_SUBSYSTEMS: usize = 64;
const MAX_ATTENTION_SLOTS: usize = 128;
const MAX_HISTORY: usize = 256;
const GLOBAL_BUDGET_DEFAULT: f32 = 100.0;
const CRISIS_THRESHOLD: f32 = 0.90;
const OVERLOAD_THRESHOLD: f32 = 0.85;
const SALIENCE_DECAY: f32 = 0.97;
const FOCUS_SHIFT_COST: f32 = 5.0;
const MIN_ALLOCATION: f32 = 1.0;
const PRIORITY_BOOST_CRISIS: f32 = 3.0;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

// ============================================================================
// FNV-1a HASHING & PRNG
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

// ============================================================================
// ATTENTION PRIORITY
// ============================================================================

/// Priority level for attention allocation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AttentionPriority {
    /// Background — minimal attention required
    Background  = 0,
    /// Normal operating level
    Normal      = 1,
    /// Elevated — requires more focus
    Elevated    = 2,
    /// High — significant issue or opportunity
    High        = 3,
    /// Critical — immediate attention required
    Critical    = 4,
    /// Emergency — ALL resources redirect here
    Emergency   = 5,
}

// ============================================================================
// ATTENTION TARGET
// ============================================================================

/// A target that competes for kernel cognitive resources
#[derive(Debug, Clone)]
pub struct AttentionTarget {
    pub name: String,
    pub id: u64,
    pub subsystem: String,
    pub priority: AttentionPriority,
    /// Computed salience (0.0 – 1.0)
    pub salience: f32,
    /// How much budget is currently allocated
    pub allocation: f32,
    /// Desired budget (request)
    pub requested: f32,
    /// How urgent — time-pressure factor
    pub urgency: f32,
    /// How important — long-term value factor
    pub importance: f32,
    /// Novelty — newly appeared targets get a boost
    pub novelty: f32,
    /// Tick when first seen
    pub first_seen_tick: u64,
    /// Tick when last updated
    pub last_update_tick: u64,
    /// How many times this target has been focused on
    pub focus_count: u64,
}

impl AttentionTarget {
    pub fn new(name: String, subsystem: String, tick: u64) -> Self {
        let id = fnv1a_hash(name.as_bytes());
        Self {
            name,
            id,
            subsystem,
            priority: AttentionPriority::Normal,
            salience: 0.0,
            allocation: 0.0,
            requested: MIN_ALLOCATION,
            urgency: 0.5,
            importance: 0.5,
            novelty: 1.0,
            first_seen_tick: tick,
            last_update_tick: tick,
            focus_count: 0,
        }
    }

    /// Recompute salience from urgency, importance, and novelty
    pub fn recompute_salience(&mut self) {
        let base = self.urgency * 0.4 + self.importance * 0.4 + self.novelty * 0.2;
        let priority_mult = match self.priority {
            AttentionPriority::Background => 0.3,
            AttentionPriority::Normal => 1.0,
            AttentionPriority::Elevated => 1.5,
            AttentionPriority::High => 2.0,
            AttentionPriority::Critical => 3.0,
            AttentionPriority::Emergency => 5.0,
        };
        self.salience = (base * priority_mult).min(1.0);
    }

    /// Decay novelty over time
    pub fn decay_novelty(&mut self) {
        self.novelty *= SALIENCE_DECAY;
        if self.novelty < 0.01 {
            self.novelty = 0.01;
        }
    }
}

// ============================================================================
// COGNITIVE LOAD REPORT
// ============================================================================

/// Current cognitive load assessment
#[derive(Debug, Clone, Copy)]
pub struct CognitiveLoadReport {
    /// Total attention budget consumed (0.0 – 1.0 fraction)
    pub utilization: f32,
    /// Number of active targets
    pub active_target_count: u32,
    /// Number of starved targets (requested > allocated significantly)
    pub starved_count: u32,
    /// Whether we are overloaded
    pub overloaded: bool,
    /// Remaining budget
    pub remaining_budget: f32,
    pub tick: u64,
}

// ============================================================================
// FOCUS SHIFT EVENT
// ============================================================================

/// Record of an attention focus shift
#[derive(Debug, Clone)]
pub struct FocusShiftEvent {
    pub from_target_id: u64,
    pub to_target_id: u64,
    pub reason: String,
    pub cost: f32,
    pub tick: u64,
}

// ============================================================================
// STATS
// ============================================================================

/// Statistics for the holistic attention engine
#[derive(Debug, Clone)]
pub struct HolisticAttentionStats {
    pub total_allocations: u64,
    pub total_focus_shifts: u64,
    pub total_crises: u64,
    pub average_utilization: f32,
    pub peak_utilization: f32,
    pub targets_registered: u64,
    pub targets_retired: u64,
    pub total_budget_distributed: f32,
}

// ============================================================================
// HOLISTIC ATTENTION ENGINE
// ============================================================================

/// System-wide attention allocation engine. Distributes limited cognitive
/// resources across all subsystems based on salience, urgency, and importance.
pub struct HolisticAttentionEngine {
    /// All registered attention targets
    targets: BTreeMap<u64, AttentionTarget>,
    /// Global attention budget (replenished each cycle)
    global_budget: f32,
    /// Current budget remaining in this cycle
    remaining_budget: f32,
    /// Focus shift log
    focus_history: Vec<FocusShiftEvent>,
    focus_write_idx: usize,
    /// Cognitive load history
    load_history: Vec<f32>,
    load_write_idx: usize,
    /// Currently focused target id
    current_focus: u64,
    /// Stats
    stats: HolisticAttentionStats,
    /// PRNG
    rng: u64,
    /// Current tick
    tick: u64,
}

impl HolisticAttentionEngine {
    /// Create a new holistic attention engine
    pub fn new(seed: u64) -> Self {
        let mut focus_history = Vec::with_capacity(MAX_HISTORY);
        for _ in 0..MAX_HISTORY {
            focus_history.push(FocusShiftEvent {
                from_target_id: 0,
                to_target_id: 0,
                reason: String::new(),
                cost: 0.0,
                tick: 0,
            });
        }
        let mut load_history = Vec::with_capacity(MAX_HISTORY);
        for _ in 0..MAX_HISTORY {
            load_history.push(0.0);
        }
        Self {
            targets: BTreeMap::new(),
            global_budget: GLOBAL_BUDGET_DEFAULT,
            remaining_budget: GLOBAL_BUDGET_DEFAULT,
            focus_history,
            focus_write_idx: 0,
            load_history,
            load_write_idx: 0,
            current_focus: 0,
            stats: HolisticAttentionStats {
                total_allocations: 0,
                total_focus_shifts: 0,
                total_crises: 0,
                average_utilization: 0.0,
                peak_utilization: 0.0,
                targets_registered: 0,
                targets_retired: 0,
                total_budget_distributed: 0.0,
            },
            rng: seed ^ 0xA77E_471_0000_CAFE,
            tick: 0,
        }
    }

    /// Run the full global attention cycle: salience → rank → allocate
    pub fn global_attention(&mut self, tick: u64) {
        self.tick = tick;
        self.remaining_budget = self.global_budget;
        for (_id, target) in self.targets.iter_mut() {
            target.recompute_salience();
            target.decay_novelty();
            target.allocation = 0.0;
        }
        self.allocate_across_subsystems();
    }

    /// Distribute budget across all targets proportional to salience
    pub fn allocate_across_subsystems(&mut self) {
        let total_salience: f32 = self.targets.values().map(|t| t.salience).sum();
        if total_salience <= 0.0 {
            return;
        }
        let budget = self.remaining_budget;
        let mut distributed: f32 = 0.0;
        let ids: Vec<u64> = self.targets.keys().copied().collect();
        for id in ids {
            if let Some(target) = self.targets.get_mut(&id) {
                let share = (target.salience / total_salience) * budget;
                let clamped = share.max(MIN_ALLOCATION).min(budget - distributed);
                target.allocation = clamped;
                target.focus_count += 1;
                distributed += clamped;
            }
        }
        self.remaining_budget = budget - distributed;
        self.stats.total_allocations += 1;
        self.stats.total_budget_distributed += distributed;
    }

    /// Handle an attention crisis — emergency reallocation
    pub fn attention_crisis(&mut self, crisis_target_id: u64, tick: u64) {
        self.tick = tick;
        self.stats.total_crises += 1;
        if let Some(target) = self.targets.get_mut(&crisis_target_id) {
            target.priority = AttentionPriority::Emergency;
            target.urgency = 1.0;
            target.recompute_salience();
        }
        // Steal from lowest-priority targets
        let mut steal_pool: f32 = 0.0;
        let ids: Vec<u64> = self.targets.keys().copied().collect();
        for id in &ids {
            if *id != crisis_target_id {
                if let Some(target) = self.targets.get_mut(id) {
                    if target.priority <= AttentionPriority::Normal {
                        let steal = target.allocation * 0.5;
                        target.allocation -= steal;
                        steal_pool += steal;
                    }
                }
            }
        }
        if let Some(target) = self.targets.get_mut(&crisis_target_id) {
            target.allocation += steal_pool + PRIORITY_BOOST_CRISIS;
        }
    }

    /// Get salience ranking — sorted list of (target_id, salience)
    pub fn salience_ranking(&self) -> Vec<(u64, f32)> {
        let mut ranking: Vec<(u64, f32)> = self
            .targets
            .iter()
            .map(|(id, t)| (*id, t.salience))
            .collect();
        ranking.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        ranking
    }

    /// Compute current cognitive load
    pub fn cognitive_load(&self) -> CognitiveLoadReport {
        let used = self.global_budget - self.remaining_budget;
        let utilization = if self.global_budget > 0.0 {
            used / self.global_budget
        } else {
            0.0
        };
        let starved = self
            .targets
            .values()
            .filter(|t| t.requested > t.allocation * 1.5 && t.allocation < MIN_ALLOCATION * 2.0)
            .count() as u32;
        CognitiveLoadReport {
            utilization,
            active_target_count: self.targets.len() as u32,
            starved_count: starved,
            overloaded: utilization > OVERLOAD_THRESHOLD,
            remaining_budget: self.remaining_budget,
            tick: self.tick,
        }
    }

    /// Optimize attention distribution using load feedback
    pub fn attention_optimization(&mut self) {
        let load = self.cognitive_load();
        if load.overloaded {
            // Increase budget slightly under sustained overload
            self.global_budget *= 1.05;
        } else if load.utilization < 0.5 && self.global_budget > GLOBAL_BUDGET_DEFAULT {
            self.global_budget *= 0.98;
        }
        let utilization = load.utilization;
        self.load_history[self.load_write_idx] = utilization;
        self.load_write_idx = (self.load_write_idx + 1) % MAX_HISTORY;
        self.stats.average_utilization +=
            EMA_ALPHA * (utilization - self.stats.average_utilization);
        if utilization > self.stats.peak_utilization {
            self.stats.peak_utilization = utilization;
        }
    }

    /// Shift focus to a new target
    pub fn focus_shift(&mut self, new_target_id: u64, reason: String) {
        let old_focus = self.current_focus;
        self.current_focus = new_target_id;
        let event = FocusShiftEvent {
            from_target_id: old_focus,
            to_target_id: new_target_id,
            reason,
            cost: FOCUS_SHIFT_COST,
            tick: self.tick,
        };
        self.focus_history[self.focus_write_idx] = event;
        self.focus_write_idx = (self.focus_write_idx + 1) % MAX_HISTORY;
        self.stats.total_focus_shifts += 1;
        self.remaining_budget -= FOCUS_SHIFT_COST;
        if self.remaining_budget < 0.0 {
            self.remaining_budget = 0.0;
        }
    }

    /// Register a new attention target
    pub fn register_target(&mut self, target: AttentionTarget) {
        self.stats.targets_registered += 1;
        self.targets.insert(target.id, target);
    }

    /// Retire a target
    pub fn retire_target(&mut self, target_id: u64) {
        if self.targets.remove(&target_id).is_some() {
            self.stats.targets_retired += 1;
        }
    }

    /// Get a specific target
    pub fn get_target(&self, target_id: u64) -> Option<&AttentionTarget> {
        self.targets.get(&target_id)
    }

    /// Current focus target ID
    pub fn current_focus_id(&self) -> u64 {
        self.current_focus
    }

    /// Engine statistics
    pub fn stats(&self) -> &HolisticAttentionStats {
        &self.stats
    }

    /// Adjust global budget
    pub fn set_budget(&mut self, budget: f32) {
        self.global_budget = budget.max(MIN_ALLOCATION);
    }

    /// Number of registered targets
    pub fn target_count(&self) -> usize {
        self.targets.len()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_salience() {
        let mut target = AttentionTarget::new(
            String::from("memory_gc"),
            String::from("memory"),
            1,
        );
        target.urgency = 0.9;
        target.importance = 0.8;
        target.recompute_salience();
        assert!(target.salience > 0.0);
    }

    #[test]
    fn test_engine_allocation() {
        let mut engine = HolisticAttentionEngine::new(42);
        let t1 = AttentionTarget::new(String::from("sched"), String::from("scheduler"), 1);
        let t2 = AttentionTarget::new(String::from("mem"), String::from("memory"), 1);
        engine.register_target(t1);
        engine.register_target(t2);
        engine.global_attention(1);
        assert_eq!(engine.stats().total_allocations, 1);
    }

    #[test]
    fn test_cognitive_load() {
        let engine = HolisticAttentionEngine::new(99);
        let load = engine.cognitive_load();
        assert_eq!(load.active_target_count, 0);
        assert!(!load.overloaded);
    }

    #[test]
    fn test_focus_shift() {
        let mut engine = HolisticAttentionEngine::new(77);
        engine.focus_shift(123, String::from("test"));
        assert_eq!(engine.current_focus_id(), 123);
        assert_eq!(engine.stats().total_focus_shifts, 1);
    }

    #[test]
    fn test_fnv1a() {
        assert_eq!(fnv1a_hash(b"attention"), fnv1a_hash(b"attention"));
        assert_ne!(fnv1a_hash(b"a"), fnv1a_hash(b"b"));
    }
}
