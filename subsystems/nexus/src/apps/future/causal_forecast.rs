// SPDX-License-Identifier: GPL-2.0
//! # Apps Causal Forecast
//!
//! Causal prediction of application behavior. Rather than mere correlation this
//! module discovers *why* apps behave as they do — "large allocation caused
//! page-fault storm", "thread spawn preceded I/O stall". Causal relations are
//! tracked per application and scored for strength using an EMA-smoothed
//! co-occurrence metric.
//!
//! With a causal graph in hand the engine can answer interventional queries:
//! "If we throttle this app's allocation rate, what happens to its fault
//! rate?" and produce human-readable causal explanations for observed behavior.
//!
//! This is the apps engine understanding cause and effect.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_RELATIONS: usize = 2048;
const MAX_APPS: usize = 256;
const MAX_GRAPH_EDGES: usize = 4096;
const STRENGTH_DECAY: f64 = 0.05;
const EMA_ALPHA: f64 = 0.15;
const MIN_STRENGTH: f64 = 0.02;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const XORSHIFT_SEED: u64 = 0xa5a5a5a5_5a5a5a5a;

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

// ============================================================================
// ACTION / EFFECT TYPES
// ============================================================================

/// An action that an application may perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AppActionType {
    LargeAllocation,
    ThreadSpawn,
    FileOpen,
    SocketConnect,
    CpuBurst,
    MemoryFree,
    SignalSend,
    SyscallFlood,
}

impl AppActionType {
    fn as_str(&self) -> &'static str {
        match self {
            Self::LargeAllocation => "large_alloc",
            Self::ThreadSpawn => "thread_spawn",
            Self::FileOpen => "file_open",
            Self::SocketConnect => "socket_connect",
            Self::CpuBurst => "cpu_burst",
            Self::MemoryFree => "mem_free",
            Self::SignalSend => "signal_send",
            Self::SyscallFlood => "syscall_flood",
        }
    }

    fn from_index(i: usize) -> Self {
        match i % 8 {
            0 => Self::LargeAllocation,
            1 => Self::ThreadSpawn,
            2 => Self::FileOpen,
            3 => Self::SocketConnect,
            4 => Self::CpuBurst,
            5 => Self::MemoryFree,
            6 => Self::SignalSend,
            _ => Self::SyscallFlood,
        }
    }
}

/// A system-level effect observed after an app action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SystemEffect {
    PageFaultStorm,
    SchedulerContention,
    IoLatencySpike,
    MemoryPressure,
    CacheFlush,
    TlbShootdown,
    InterruptStorm,
    Thrashing,
}

impl SystemEffect {
    fn as_str(&self) -> &'static str {
        match self {
            Self::PageFaultStorm => "page_fault_storm",
            Self::SchedulerContention => "sched_contention",
            Self::IoLatencySpike => "io_latency",
            Self::MemoryPressure => "mem_pressure",
            Self::CacheFlush => "cache_flush",
            Self::TlbShootdown => "tlb_shootdown",
            Self::InterruptStorm => "irq_storm",
            Self::Thrashing => "thrashing",
        }
    }

    fn severity(&self) -> f64 {
        match self {
            Self::PageFaultStorm => 0.8,
            Self::SchedulerContention => 0.6,
            Self::IoLatencySpike => 0.7,
            Self::MemoryPressure => 0.9,
            Self::CacheFlush => 0.4,
            Self::TlbShootdown => 0.5,
            Self::InterruptStorm => 0.75,
            Self::Thrashing => 1.0,
        }
    }

    fn from_index(i: usize) -> Self {
        match i % 8 {
            0 => Self::PageFaultStorm,
            1 => Self::SchedulerContention,
            2 => Self::IoLatencySpike,
            3 => Self::MemoryPressure,
            4 => Self::CacheFlush,
            5 => Self::TlbShootdown,
            6 => Self::InterruptStorm,
            _ => Self::Thrashing,
        }
    }
}

// ============================================================================
// CAUSAL RELATION
// ============================================================================

/// A causal link between an application action and a system effect.
#[derive(Debug, Clone)]
pub struct CausalRelation {
    pub app_action: AppActionType,
    pub system_effect: SystemEffect,
    pub strength: f64,
    pub observation_count: u64,
    pub last_seen_tick: u64,
    pub confidence: f64,
    pub hash_key: u64,
}

impl CausalRelation {
    fn new(action: AppActionType, effect: SystemEffect, tick: u64) -> Self {
        let key_bytes = [action.as_str().as_bytes(), b"->", effect.as_str().as_bytes()].concat();
        let hash_key = fnv1a_hash(&key_bytes);
        Self {
            app_action: action,
            system_effect: effect,
            strength: 0.1,
            observation_count: 1,
            last_seen_tick: tick,
            confidence: 0.1,
            hash_key,
        }
    }

    fn reinforce(&mut self, tick: u64) {
        self.observation_count += 1;
        self.last_seen_tick = tick;
        self.strength = ema_update(self.strength, 1.0, EMA_ALPHA);
        self.confidence = 1.0 - (1.0 / (1.0 + self.observation_count as f64));
    }

    fn decay(&mut self) {
        self.strength *= 1.0 - STRENGTH_DECAY;
        if self.strength < MIN_STRENGTH {
            self.strength = MIN_STRENGTH;
        }
    }
}

// ============================================================================
// CAUSAL GRAPH EDGE
// ============================================================================

/// An edge in the causal graph linking two node identifiers with a weight.
#[derive(Debug, Clone)]
pub struct CausalEdge {
    pub from_node: u64,
    pub to_node: u64,
    pub weight: f64,
    pub relation_index: usize,
}

// ============================================================================
// CAUSAL EXPLANATION
// ============================================================================

/// A human-readable causal explanation of observed behavior.
#[derive(Debug, Clone)]
pub struct CausalExplanation {
    pub app_id: u64,
    pub chain: Vec<(AppActionType, SystemEffect, f64)>,
    pub root_action: AppActionType,
    pub terminal_effect: SystemEffect,
    pub total_strength: f64,
}

// ============================================================================
// PER-APP CAUSAL STATE
// ============================================================================

#[derive(Debug, Clone)]
struct AppCausalState {
    app_id: u64,
    relations: Vec<CausalRelation>,
    recent_actions: VecDeque<(AppActionType, u64)>,
    recent_effects: VecDeque<(SystemEffect, u64)>,
    prediction_count: u64,
    correct_predictions: u64,
}

impl AppCausalState {
    fn new(app_id: u64) -> Self {
        Self {
            app_id,
            relations: Vec::new(),
            recent_actions: VecDeque::new(),
            recent_effects: VecDeque::new(),
            prediction_count: 0,
            correct_predictions: 0,
        }
    }

    fn record_action(&mut self, action: AppActionType, tick: u64) {
        self.recent_actions.push_back((action, tick));
        if self.recent_actions.len() > 64 {
            self.recent_actions.pop_front();
        }
    }

    fn record_effect(&mut self, effect: SystemEffect, tick: u64) {
        self.recent_effects.push_back((effect, tick));
        if self.recent_effects.len() > 64 {
            self.recent_effects.pop_front();
        }
        // Check for causal linkage: any recent action within 10 ticks
        for &(action, a_tick) in self.recent_actions.iter().rev() {
            if tick.saturating_sub(a_tick) > 10 {
                break;
            }
            self.link_or_reinforce(action, effect, tick);
        }
    }

    fn link_or_reinforce(&mut self, action: AppActionType, effect: SystemEffect, tick: u64) {
        let key_bytes = [action.as_str().as_bytes(), b"->", effect.as_str().as_bytes()].concat();
        let hash_key = fnv1a_hash(&key_bytes);

        for rel in &mut self.relations {
            if rel.hash_key == hash_key {
                rel.reinforce(tick);
                return;
            }
        }

        if self.relations.len() < MAX_RELATIONS {
            self.relations.push(CausalRelation::new(action, effect, tick));
        }
    }

    fn strongest_effect_for(&self, action: AppActionType) -> Option<&CausalRelation> {
        self.relations
            .iter()
            .filter(|r| r.app_action as u8 == action as u8)
            .max_by(|a, b| a.strength.partial_cmp(&b.strength).unwrap_or(core::cmp::Ordering::Equal))
    }

    fn decay_all(&mut self) {
        for rel in &mut self.relations {
            rel.decay();
        }
        self.relations.retain(|r| r.strength > MIN_STRENGTH * 0.5);
    }
}

// ============================================================================
// CAUSAL FORECAST STATS
// ============================================================================

/// Engine-level statistics for the causal forecast module.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CausalForecastStats {
    pub total_predictions: u64,
    pub total_relations_discovered: u64,
    pub total_interventions_analyzed: u64,
    pub average_relation_strength: f64,
    pub causal_accuracy: f64,
    pub graph_edge_count: u64,
    pub root_cause_queries: u64,
}

impl CausalForecastStats {
    fn new() -> Self {
        Self {
            total_predictions: 0,
            total_relations_discovered: 0,
            total_interventions_analyzed: 0,
            average_relation_strength: 0.0,
            causal_accuracy: 0.0,
            graph_edge_count: 0,
            root_cause_queries: 0,
        }
    }
}

// ============================================================================
// APPS CAUSAL FORECAST ENGINE
// ============================================================================

/// Causal prediction engine for application behavior.
///
/// Maintains per-application causal graphs linking observed actions to
/// system effects. Supports root-cause analysis, intervention queries,
/// and causal explanation generation.
pub struct AppsCausalForecast {
    app_states: BTreeMap<u64, AppCausalState>,
    stats: CausalForecastStats,
    rng_state: u64,
    tick: u64,
    ema_strength: f64,
    ema_accuracy: f64,
}

impl AppsCausalForecast {
    /// Create a new causal forecast engine.
    pub fn new() -> Self {
        Self {
            app_states: BTreeMap::new(),
            stats: CausalForecastStats::new(),
            rng_state: XORSHIFT_SEED,
            tick: 0,
            ema_strength: 0.0,
            ema_accuracy: 0.5,
        }
    }

    /// Record that an application performed an action.
    #[inline]
    pub fn observe_action(&mut self, app_id: u64, action: AppActionType) {
        self.tick += 1;
        if self.app_states.len() >= MAX_APPS && !self.app_states.contains_key(&app_id) {
            return;
        }
        let state = self.app_states.entry(app_id).or_insert_with(|| AppCausalState::new(app_id));
        state.record_action(action, self.tick);
    }

    /// Record that a system effect was observed for an application.
    pub fn observe_effect(&mut self, app_id: u64, effect: SystemEffect) {
        self.tick += 1;
        let old_rel_count: usize = self.app_states.values().map(|s| s.relations.len()).sum();

        if self.app_states.len() >= MAX_APPS && !self.app_states.contains_key(&app_id) {
            return;
        }
        let state = self.app_states.entry(app_id).or_insert_with(|| AppCausalState::new(app_id));
        state.record_effect(effect, self.tick);

        let new_rel_count: usize = self.app_states.values().map(|s| s.relations.len()).sum();
        if new_rel_count > old_rel_count {
            self.stats.total_relations_discovered += (new_rel_count - old_rel_count) as u64;
        }
    }

    /// Predict the most likely system effect given an application action.
    #[inline]
    pub fn causal_app_predict(&mut self, app_id: u64, action: AppActionType) -> Option<(SystemEffect, f64)> {
        self.stats.total_predictions += 1;
        let state = self.app_states.get(&app_id)?;
        let rel = state.strongest_effect_for(action)?;
        Some((rel.system_effect, rel.strength))
    }

    /// Trace the root cause of an observed system effect for an application.
    ///
    /// Returns the most likely originating action and its causal strength.
    pub fn root_cause(&mut self, app_id: u64, effect: SystemEffect) -> Option<(AppActionType, f64)> {
        self.stats.root_cause_queries += 1;
        let state = self.app_states.get(&app_id)?;

        let best = state
            .relations
            .iter()
            .filter(|r| r.system_effect as u8 == effect as u8)
            .max_by(|a, b| a.strength.partial_cmp(&b.strength).unwrap_or(core::cmp::Ordering::Equal))?;

        Some((best.app_action, best.strength))
    }

    /// Build a full causal graph for the given application.
    ///
    /// Nodes are hashed action and effect identifiers; edges carry strength.
    pub fn causal_graph(&mut self, app_id: u64) -> Vec<CausalEdge> {
        let state = match self.app_states.get(&app_id) {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut edges = Vec::new();
        for (i, rel) in state.relations.iter().enumerate() {
            if edges.len() >= MAX_GRAPH_EDGES {
                break;
            }
            let from = fnv1a_hash(rel.app_action.as_str().as_bytes());
            let to = fnv1a_hash(rel.system_effect.as_str().as_bytes());
            edges.push(CausalEdge {
                from_node: from,
                to_node: to,
                weight: rel.strength,
                relation_index: i,
            });
        }

        self.stats.graph_edge_count = edges.len() as u64;
        edges
    }

    /// Analyze the impact of an intervention: if we suppress a given action,
    /// how much does the total expected severity decrease?
    pub fn intervention_analysis(&mut self, app_id: u64, suppress: AppActionType) -> f64 {
        self.stats.total_interventions_analyzed += 1;
        let state = match self.app_states.get(&app_id) {
            Some(s) => s,
            None => return 0.0,
        };

        let mut total_severity = 0.0;
        let mut suppressed_severity = 0.0;

        for rel in &state.relations {
            let sev = rel.strength * rel.system_effect.severity();
            total_severity += sev;
            if rel.app_action as u8 == suppress as u8 {
                suppressed_severity += sev;
            }
        }

        if total_severity > 0.0 {
            suppressed_severity / total_severity
        } else {
            0.0
        }
    }

    /// Generate a causal explanation chain for an app's behavior.
    pub fn causal_explanation(&self, app_id: u64) -> Option<CausalExplanation> {
        let state = self.app_states.get(&app_id)?;
        if state.relations.is_empty() {
            return None;
        }

        let mut chain = Vec::new();
        let mut total_strength = 0.0;

        // Sort relations by strength descending (manual insertion sort for no_std)
        let mut sorted_indices: Vec<usize> = (0..state.relations.len()).collect();
        for i in 1..sorted_indices.len() {
            let mut j = i;
            while j > 0 && state.relations[sorted_indices[j]].strength > state.relations[sorted_indices[j - 1]].strength {
                sorted_indices.swap(j, j - 1);
                j -= 1;
            }
        }

        let top_n = if sorted_indices.len() > 5 { 5 } else { sorted_indices.len() };
        for &idx in &sorted_indices[..top_n] {
            let rel = &state.relations[idx];
            chain.push((rel.app_action, rel.system_effect, rel.strength));
            total_strength += rel.strength;
        }

        let root_action = chain.first().map(|c| c.0).unwrap_or(AppActionType::CpuBurst);
        let terminal = chain.last().map(|c| c.1).unwrap_or(SystemEffect::MemoryPressure);

        Some(CausalExplanation {
            app_id,
            chain,
            root_action,
            terminal_effect: terminal,
            total_strength,
        })
    }

    /// Forecast an effect from an observed cause with confidence-weighted strength.
    pub fn forecast_from_cause(&mut self, app_id: u64, cause: AppActionType) -> Vec<(SystemEffect, f64)> {
        let state = match self.app_states.get(&app_id) {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut results: Vec<(SystemEffect, f64)> = state
            .relations
            .iter()
            .filter(|r| r.app_action as u8 == cause as u8)
            .map(|r| (r.system_effect, r.strength * r.confidence))
            .collect();

        // Sort descending by strength
        for i in 1..results.len() {
            let mut j = i;
            while j > 0 && results[j].1 > results[j - 1].1 {
                results.swap(j, j - 1);
                j -= 1;
            }
        }

        results
    }

    /// Decay all causal relations to forget stale links.
    pub fn decay_all(&mut self) {
        for state in self.app_states.values_mut() {
            state.decay_all();
        }

        // Recompute EMA strength
        let mut total = 0.0;
        let mut count = 0u64;
        for state in self.app_states.values() {
            for rel in &state.relations {
                total += rel.strength;
                count += 1;
            }
        }
        let avg = if count > 0 { total / count as f64 } else { 0.0 };
        self.ema_strength = ema_update(self.ema_strength, avg, EMA_ALPHA);
        self.stats.average_relation_strength = self.ema_strength;
    }

    /// Return a snapshot of engine statistics.
    #[inline(always)]
    pub fn stats(&self) -> &CausalForecastStats {
        &self.stats
    }

    /// Total number of tracked causal relations across all apps.
    #[inline(always)]
    pub fn total_relations(&self) -> usize {
        self.app_states.values().map(|s| s.relations.len()).sum()
    }

    /// Number of tracked applications.
    #[inline(always)]
    pub fn tracked_apps(&self) -> usize {
        self.app_states.len()
    }
}
