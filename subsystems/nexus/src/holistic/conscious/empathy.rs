// SPDX-License-Identifier: GPL-2.0
//! # Holistic Empathy Engine
//!
//! **Understanding the ENTIRE system's state holistically.** Builds a unified
//! empathy model of all processes, all subsystems, and all resources. Where
//! per-subsystem empathy sees a single domain, this engine perceives the
//! full cross-subsystem emotional and operational landscape — detecting
//! pain points, happiness zones, and cross-correlations no local model can.
//!
//! ## Empathy Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │              SYSTEM EMPATHY MAP                              │
//! ├──────────────────────────────────────────────────────────────┤
//! │  Per-Subsystem ──▶ Cross-Correlation ──▶ Fusion             │
//! │       │                   │                  │               │
//! │       ▼                   ▼                  ▼               │
//! │  "How does each    "How do they       "Unified              │
//! │   part feel?"       affect each        understanding"       │
//! │                     other?"                                 │
//! │                                                             │
//! │  Pain Point Detection ──▶ System Happiness Index            │
//! │       │                           │                         │
//! │       ▼                           ▼                         │
//! │  "Where does it hurt?"   "Overall system wellbeing"         │
//! └──────────────────────────────────────────────────────────────┘
//! ```

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.12;
const MAX_SUBSYSTEMS: usize = 64;
const MAX_PROCESSES: usize = 1024;
const MAX_PAIN_POINTS: usize = 64;
const MAX_CORRELATIONS: usize = 256;
const MAX_HISTORY: usize = 128;
const PAIN_THRESHOLD: f32 = 0.65;
const HAPPINESS_BASELINE: f32 = 0.50;
const CORRELATION_MIN: f32 = 0.20;
const EMPATHY_DEPTH_MAX: f32 = 1.0;
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
// SUBSYSTEM EMPATHY STATE
// ============================================================================

/// Empathy state for a single subsystem
#[derive(Debug, Clone)]
pub struct SubsystemEmpathyState {
    pub name: String,
    pub id: u64,
    /// Operational wellbeing (0.0 = suffering, 1.0 = thriving)
    pub wellbeing: f32,
    /// Stress level (0.0 = relaxed, 1.0 = critically stressed)
    pub stress: f32,
    /// Resource satisfaction (0.0 = starved, 1.0 = abundant)
    pub resource_satisfaction: f32,
    /// Performance relative to capability (0.0 = terrible, 1.0 = peak)
    pub relative_performance: f32,
    /// How much attention this subsystem needs
    pub attention_need: f32,
    /// Recent trend: positive = improving
    pub trend: f32,
    /// EMA-smoothed composite score
    pub composite: f32,
    /// Observation count
    pub observation_count: u64,
    /// Last update tick
    pub last_tick: u64,
}

impl SubsystemEmpathyState {
    pub fn new(name: String) -> Self {
        let id = fnv1a_hash(name.as_bytes());
        Self {
            name,
            id,
            wellbeing: HAPPINESS_BASELINE,
            stress: 0.0,
            resource_satisfaction: HAPPINESS_BASELINE,
            relative_performance: HAPPINESS_BASELINE,
            attention_need: 0.0,
            trend: 0.0,
            composite: HAPPINESS_BASELINE,
            observation_count: 0,
            last_tick: 0,
        }
    }

    /// Update the empathy state with new observations
    pub fn update(&mut self, wellbeing: f32, stress: f32, resource_sat: f32, perf: f32, tick: u64) {
        let clamp = |v: f32| if v < 0.0 { 0.0 } else if v > 1.0 { 1.0 } else { v };
        let old_composite = self.composite;
        self.wellbeing += EMA_ALPHA * (clamp(wellbeing) - self.wellbeing);
        self.stress += EMA_ALPHA * (clamp(stress) - self.stress);
        self.resource_satisfaction += EMA_ALPHA * (clamp(resource_sat) - self.resource_satisfaction);
        self.relative_performance += EMA_ALPHA * (clamp(perf) - self.relative_performance);
        self.composite = self.wellbeing * 0.3
            + (1.0 - self.stress) * 0.3
            + self.resource_satisfaction * 0.2
            + self.relative_performance * 0.2;
        self.trend = self.composite - old_composite;
        self.attention_need = self.stress * 0.5 + (1.0 - self.wellbeing) * 0.5;
        self.observation_count += 1;
        self.last_tick = tick;
    }

    /// Whether this subsystem is a pain point
    pub fn is_pain_point(&self) -> bool {
        self.stress > PAIN_THRESHOLD || self.wellbeing < (1.0 - PAIN_THRESHOLD)
    }
}

// ============================================================================
// CROSS-SUBSYSTEM CORRELATION
// ============================================================================

/// Correlation between two subsystems' empathy states
#[derive(Debug, Clone)]
pub struct CrossCorrelation {
    pub subsystem_a_id: u64,
    pub subsystem_b_id: u64,
    /// Running correlation coefficient (-1.0 to 1.0)
    pub correlation: f32,
    /// EMA-smoothed co-movement
    pub co_movement: f32,
    /// Number of joint observations
    pub observation_count: u64,
    /// Whether this correlation is significant
    pub significant: bool,
}

impl CrossCorrelation {
    pub fn new(a_id: u64, b_id: u64) -> Self {
        Self {
            subsystem_a_id: a_id,
            subsystem_b_id: b_id,
            correlation: 0.0,
            co_movement: 0.0,
            observation_count: 0,
            significant: false,
        }
    }

    /// Update correlation with new co-observation
    pub fn observe(&mut self, delta_a: f32, delta_b: f32) {
        let product = delta_a * delta_b;
        self.co_movement += EMA_ALPHA * (product - self.co_movement);
        // Approximate correlation from co-movement direction
        self.correlation += EMA_ALPHA * (product.signum() * product.abs().sqrt() - self.correlation);
        self.correlation = self.correlation.max(-1.0).min(1.0);
        self.observation_count += 1;
        self.significant = self.correlation.abs() > CORRELATION_MIN && self.observation_count > 5;
    }
}

// ============================================================================
// PAIN POINT
// ============================================================================

/// A detected system pain point
#[derive(Debug, Clone)]
pub struct PainPoint {
    pub subsystem_id: u64,
    pub subsystem_name: String,
    pub severity: f32,
    pub description: String,
    pub affected_neighbors: Vec<u64>,
    pub detection_tick: u64,
    pub duration_ticks: u64,
    pub resolved: bool,
}

// ============================================================================
// SYSTEM EMPATHY MAP
// ============================================================================

/// Complete empathy map across all subsystems
#[derive(Debug, Clone)]
pub struct SystemEmpathyMap {
    /// Per-subsystem empathy scores
    pub per_subsystem: BTreeMap<u64, f32>,
    /// Overall system happiness (0.0 – 1.0)
    pub system_happiness: f32,
    /// Cross-correlation count of significant pairs
    pub significant_correlations: u32,
    /// Active pain point count
    pub active_pain_points: u32,
    /// Empathy depth reached
    pub empathy_depth: f32,
    pub tick: u64,
}

// ============================================================================
// STATS
// ============================================================================

/// Empathy engine statistics
#[derive(Debug, Clone)]
pub struct HolisticEmpathyStats {
    pub total_observations: u64,
    pub total_pain_points_detected: u64,
    pub pain_points_resolved: u64,
    pub average_system_happiness: f32,
    pub correlation_updates: u64,
    pub empathy_cycles: u64,
    pub deepest_empathy: f32,
    pub most_painful_subsystem: u64,
}

// ============================================================================
// HOLISTIC EMPATHY ENGINE
// ============================================================================

/// The system-wide empathy engine — understands the holistic state of every
/// subsystem, detects pain points, and computes system happiness.
pub struct HolisticEmpathyEngine {
    /// Per-subsystem empathy states
    subsystems: BTreeMap<u64, SubsystemEmpathyState>,
    /// Cross-subsystem correlations
    correlations: BTreeMap<(u64, u64), CrossCorrelation>,
    /// Active pain points
    pain_points: Vec<PainPoint>,
    /// System happiness history
    happiness_history: Vec<f32>,
    happiness_write_idx: usize,
    /// Current empathy map
    current_map: SystemEmpathyMap,
    /// Stats
    stats: HolisticEmpathyStats,
    /// PRNG
    rng: u64,
    /// Tick
    tick: u64,
}

impl HolisticEmpathyEngine {
    /// Create a new holistic empathy engine
    pub fn new(seed: u64) -> Self {
        let mut happiness_history = Vec::with_capacity(MAX_HISTORY);
        for _ in 0..MAX_HISTORY {
            happiness_history.push(HAPPINESS_BASELINE);
        }
        Self {
            subsystems: BTreeMap::new(),
            correlations: BTreeMap::new(),
            pain_points: Vec::with_capacity(MAX_PAIN_POINTS),
            happiness_history,
            happiness_write_idx: 0,
            current_map: SystemEmpathyMap {
                per_subsystem: BTreeMap::new(),
                system_happiness: HAPPINESS_BASELINE,
                significant_correlations: 0,
                active_pain_points: 0,
                empathy_depth: 0.0,
                tick: 0,
            },
            stats: HolisticEmpathyStats {
                total_observations: 0,
                total_pain_points_detected: 0,
                pain_points_resolved: 0,
                average_system_happiness: HAPPINESS_BASELINE,
                correlation_updates: 0,
                empathy_cycles: 0,
                deepest_empathy: 0.0,
                most_painful_subsystem: 0,
            },
            rng: seed ^ 0xE3FA_7741_CAFE_BABE,
            tick: 0,
        }
    }

    /// Run the full system empathy cycle
    pub fn system_empathy(&mut self, tick: u64) -> &SystemEmpathyMap {
        self.tick = tick;
        self.recompute_map();
        self.pain_point_detection();
        self.stats.empathy_cycles += 1;
        &self.current_map
    }

    /// Build holistic understanding from all subsystem states
    pub fn holistic_understanding(&self) -> f32 {
        if self.subsystems.is_empty() {
            return 0.0;
        }
        let total: f32 = self.subsystems.values().map(|s| s.composite).sum();
        total / self.subsystems.len() as f32
    }

    /// Compute cross-subsystem empathy correlations
    pub fn cross_subsystem_empathy(&mut self) {
        let ids: Vec<u64> = self.subsystems.keys().copied().collect();
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                let a_id = ids[i].min(ids[j]);
                let b_id = ids[i].max(ids[j]);
                let delta_a = self.subsystems.get(&ids[i]).map_or(0.0, |s| s.trend);
                let delta_b = self.subsystems.get(&ids[j]).map_or(0.0, |s| s.trend);
                let corr = self
                    .correlations
                    .entry((a_id, b_id))
                    .or_insert_with(|| CrossCorrelation::new(a_id, b_id));
                corr.observe(delta_a, delta_b);
                self.stats.correlation_updates += 1;
            }
        }
    }

    /// Fuse empathy from all sources into unified understanding
    pub fn empathy_fusion(&mut self) -> f32 {
        self.cross_subsystem_empathy();
        let base = self.holistic_understanding();
        let correlation_bonus = self
            .correlations
            .values()
            .filter(|c| c.significant && c.correlation > 0.5)
            .count() as f32
            * 0.01;
        let fused = (base + correlation_bonus).min(1.0);
        fused
    }

    /// Compute system happiness index
    pub fn system_happiness(&self) -> f32 {
        self.current_map.system_happiness
    }

    /// Detect pain points across all subsystems
    pub fn pain_point_detection(&mut self) {
        // Update existing pain points
        for pp in self.pain_points.iter_mut() {
            if !pp.resolved {
                if let Some(sub) = self.subsystems.get(&pp.subsystem_id) {
                    if !sub.is_pain_point() {
                        pp.resolved = true;
                        self.stats.pain_points_resolved += 1;
                    } else {
                        pp.duration_ticks = self.tick.saturating_sub(pp.detection_tick);
                    }
                }
            }
        }
        // Detect new pain points
        for (id, sub) in &self.subsystems {
            if sub.is_pain_point() {
                let already_tracked = self.pain_points.iter().any(|pp| pp.subsystem_id == *id && !pp.resolved);
                if !already_tracked && self.pain_points.len() < MAX_PAIN_POINTS {
                    let affected: Vec<u64> = self
                        .correlations
                        .iter()
                        .filter(|((a, b), c)| (*a == *id || *b == *id) && c.significant)
                        .map(|((a, b), _)| if *a == *id { *b } else { *a })
                        .collect();
                    self.pain_points.push(PainPoint {
                        subsystem_id: *id,
                        subsystem_name: sub.name.clone(),
                        severity: sub.stress,
                        description: String::from("pain detected"),
                        affected_neighbors: affected,
                        detection_tick: self.tick,
                        duration_ticks: 0,
                        resolved: false,
                    });
                    self.stats.total_pain_points_detected += 1;
                    if sub.stress > self.subsystems
                        .get(&self.stats.most_painful_subsystem)
                        .map_or(0.0, |s| s.stress)
                    {
                        self.stats.most_painful_subsystem = *id;
                    }
                }
            }
        }
    }

    /// Compute empathy depth — how deeply we understand the system
    pub fn empathy_depth(&self) -> f32 {
        let subsystem_coverage = (self.subsystems.len() as f32 / MAX_SUBSYSTEMS as f32).min(1.0);
        let correlation_depth = if self.subsystems.len() > 1 {
            let possible_pairs = self.subsystems.len() * (self.subsystems.len() - 1) / 2;
            (self.correlations.len() as f32 / possible_pairs as f32).min(1.0)
        } else {
            0.0
        };
        let obs_depth = self.subsystems.values().map(|s| {
            (s.observation_count as f32 / 100.0).min(1.0)
        }).sum::<f32>() / self.subsystems.len().max(1) as f32;
        let depth = subsystem_coverage * 0.3 + correlation_depth * 0.4 + obs_depth * 0.3;
        depth.min(EMPATHY_DEPTH_MAX)
    }

    /// Register or update a subsystem in the empathy model
    pub fn register_subsystem(&mut self, name: String) -> u64 {
        let id = fnv1a_hash(name.as_bytes());
        self.subsystems
            .entry(id)
            .or_insert_with(|| SubsystemEmpathyState::new(name));
        id
    }

    /// Update a subsystem's empathy state
    pub fn update_subsystem(
        &mut self,
        subsystem_id: u64,
        wellbeing: f32,
        stress: f32,
        resource_sat: f32,
        perf: f32,
        tick: u64,
    ) {
        if let Some(sub) = self.subsystems.get_mut(&subsystem_id) {
            sub.update(wellbeing, stress, resource_sat, perf, tick);
            self.stats.total_observations += 1;
        }
    }

    /// Active pain points
    pub fn active_pain_points(&self) -> Vec<&PainPoint> {
        self.pain_points.iter().filter(|pp| !pp.resolved).collect()
    }

    /// Stats
    pub fn stats(&self) -> &HolisticEmpathyStats {
        &self.stats
    }

    /// Subsystem count
    pub fn subsystem_count(&self) -> usize {
        self.subsystems.len()
    }

    // ========================================================================
    // INTERNAL
    // ========================================================================

    fn recompute_map(&mut self) {
        let mut per_sub = BTreeMap::new();
        let mut total_composite = 0.0f32;
        let count = self.subsystems.len().max(1);
        for (id, sub) in &self.subsystems {
            per_sub.insert(*id, sub.composite);
            total_composite += sub.composite;
        }
        let happiness = total_composite / count as f32;
        let sig_corr = self.correlations.values().filter(|c| c.significant).count() as u32;
        let active_pp = self.pain_points.iter().filter(|pp| !pp.resolved).count() as u32;
        let depth = self.empathy_depth();

        self.current_map = SystemEmpathyMap {
            per_subsystem: per_sub,
            system_happiness: happiness,
            significant_correlations: sig_corr,
            active_pain_points: active_pp,
            empathy_depth: depth,
            tick: self.tick,
        };

        self.happiness_history[self.happiness_write_idx] = happiness;
        self.happiness_write_idx = (self.happiness_write_idx + 1) % MAX_HISTORY;
        self.stats.average_system_happiness +=
            EMA_ALPHA * (happiness - self.stats.average_system_happiness);
        if depth > self.stats.deepest_empathy {
            self.stats.deepest_empathy = depth;
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subsystem_empathy_state() {
        let mut state = SubsystemEmpathyState::new(String::from("memory"));
        state.update(0.8, 0.2, 0.7, 0.9, 1);
        assert!(state.composite > 0.5);
        assert!(!state.is_pain_point());
    }

    #[test]
    fn test_pain_point_detection() {
        let mut state = SubsystemEmpathyState::new(String::from("overloaded"));
        for _ in 0..20 {
            state.update(0.1, 0.95, 0.1, 0.1, 1);
        }
        assert!(state.is_pain_point());
    }

    #[test]
    fn test_engine_creation() {
        let engine = HolisticEmpathyEngine::new(42);
        assert_eq!(engine.subsystem_count(), 0);
        assert_eq!(engine.system_happiness(), HAPPINESS_BASELINE);
    }

    #[test]
    fn test_holistic_understanding_empty() {
        let engine = HolisticEmpathyEngine::new(42);
        assert_eq!(engine.holistic_understanding(), 0.0);
    }

    #[test]
    fn test_fnv1a() {
        assert_eq!(fnv1a_hash(b"empathy"), fnv1a_hash(b"empathy"));
        assert_ne!(fnv1a_hash(b"emp"), fnv1a_hash(b"athy"));
    }
}
