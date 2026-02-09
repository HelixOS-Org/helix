// SPDX-License-Identifier: GPL-2.0
//! # Holistic Paradigm — System-Wide Paradigm Shift Detection
//!
//! Detects when the ENTIRE kernel's optimisation philosophy needs to
//! change. Individual improvements are evolutionary — they refine what
//! exists. But sometimes the accumulated weight of evidence shows that
//! the fundamental assumptions are wrong, and a revolutionary paradigm
//! shift is required. This engine monitors paradigm health, detects
//! revolutionary pressure, and plans managed transitions.
//!
//! ## Capabilities
//!
//! - **System paradigm analysis** — evaluate current paradigm fitness
//! - **Paradigm health monitoring** — detect paradigm degradation
//! - **Revolution detection** — identify when a paradigm shift is imminent
//! - **Evolutionary pressure** — measure forces pushing for change
//! - **Transition planning** — create managed paradigm transition plans
//! - **Paradigm history** — track the kernel's philosophical evolution
//!
//! The engine that knows when the kernel must reinvent itself.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_PARADIGMS: usize = 64;
const MAX_PRESSURES: usize = 512;
const MAX_TRANSITIONS: usize = 32;
const MAX_ANOMALIES: usize = 256;
const MAX_HISTORY_LOG: usize = 1024;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const HEALTH_WARNING_THRESHOLD: f32 = 0.60;
const HEALTH_CRITICAL_THRESHOLD: f32 = 0.35;
const REVOLUTION_THRESHOLD: f32 = 0.80;
const PRESSURE_ACCUMULATION_RATE: f32 = 0.05;
const ANOMALY_WEIGHT: f32 = 0.15;
const TRANSITION_RISK_FLOOR: f32 = 0.10;
const PARADIGM_INERTIA: f32 = 0.90;

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

fn xorshift_f32(state: &mut u64) -> f32 {
    (xorshift64(state) % 10000) as f32 / 10000.0
}

// ============================================================================
// TYPES
// ============================================================================

/// Aspect of kernel philosophy being examined
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParadigmDomain {
    SchedulingPhilosophy,
    MemoryModel,
    IpcArchitecture,
    TrustFramework,
    EnergyStrategy,
    FairnessModel,
    ScalabilityApproach,
    SecurityPosture,
    ResourceAllocation,
    SystemEvolution,
}

/// Health status of a paradigm
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParadigmHealth {
    Thriving,
    Stable,
    Strained,
    Degrading,
    Critical,
    Obsolete,
}

/// Phase of a paradigm transition
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TransitionPhase {
    Assessment,
    Planning,
    PreparingRollback,
    Migrating,
    Validating,
    Completed,
    Aborted,
}

/// A paradigm — a set of fundamental assumptions about how to optimise
#[derive(Debug, Clone)]
pub struct Paradigm {
    pub id: u64,
    pub domain: ParadigmDomain,
    pub name: String,
    pub core_assumptions: Vec<u64>,
    pub health: ParadigmHealth,
    pub health_score: f32,
    pub fitness: f32,
    pub anomaly_count: u64,
    pub adopted_tick: u64,
    pub last_validated_tick: u64,
    pub hash: u64,
}

/// Evolutionary pressure — a force pushing for paradigm change
#[derive(Debug, Clone)]
pub struct EvolutionaryPressure {
    pub id: u64,
    pub domain: ParadigmDomain,
    pub source_description: String,
    pub magnitude: f32,
    pub accumulated: f32,
    pub direction: PressureDirection,
    pub first_detected_tick: u64,
    pub last_observed_tick: u64,
}

/// Direction of evolutionary pressure
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PressureDirection {
    TowardsSimplicity,
    TowardsComplexity,
    TowardsDistributed,
    TowardsCentralised,
    TowardsAdaptive,
    TowardsStatic,
    TowardsHybrid,
    Undetermined,
}

/// An anomaly that the current paradigm cannot explain
#[derive(Debug, Clone)]
pub struct ParadigmAnomaly {
    pub id: u64,
    pub domain: ParadigmDomain,
    pub description_hash: u64,
    pub severity: f32,
    pub frequency: u64,
    pub first_seen_tick: u64,
    pub last_seen_tick: u64,
}

/// A paradigm transition plan
#[derive(Debug, Clone)]
pub struct TransitionPlan {
    pub id: u64,
    pub from_paradigm: u64,
    pub to_paradigm: u64,
    pub domain: ParadigmDomain,
    pub phase: TransitionPhase,
    pub risk_score: f32,
    pub progress: f32,
    pub rollback_ready: bool,
    pub created_tick: u64,
    pub estimated_completion_tick: u64,
}

/// Historical paradigm record
#[derive(Debug, Clone)]
pub struct ParadigmHistoryEntry {
    pub paradigm_id: u64,
    pub domain: ParadigmDomain,
    pub name: String,
    pub peak_fitness: f32,
    pub duration_ticks: u64,
    pub adopted_tick: u64,
    pub retired_tick: u64,
    pub successor_id: u64,
    pub retirement_reason_hash: u64,
}

/// Paradigm engine statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ParadigmStats {
    pub active_paradigms: u64,
    pub total_paradigms_ever: u64,
    pub avg_health_ema: f32,
    pub avg_fitness_ema: f32,
    pub total_pressures: u64,
    pub total_anomalies: u64,
    pub revolution_pressure_ema: f32,
    pub transitions_completed: u64,
    pub transitions_aborted: u64,
    pub active_transitions: u64,
    pub paradigm_age_avg_ema: f32,
    pub last_tick: u64,
}

// ============================================================================
// HOLISTIC PARADIGM
// ============================================================================

/// System-wide paradigm shift detection and management engine
pub struct HolisticParadigm {
    paradigms: BTreeMap<u64, Paradigm>,
    pressures: VecDeque<EvolutionaryPressure>,
    anomalies: VecDeque<ParadigmAnomaly>,
    transitions: BTreeMap<u64, TransitionPlan>,
    history: Vec<ParadigmHistoryEntry>,
    rng_state: u64,
    tick: u64,
    stats: ParadigmStats,
}

impl HolisticParadigm {
    /// Create a new holistic paradigm engine
    pub fn new(seed: u64) -> Self {
        Self {
            paradigms: BTreeMap::new(),
            pressures: VecDeque::new(),
            anomalies: VecDeque::new(),
            transitions: BTreeMap::new(),
            history: Vec::new(),
            rng_state: seed | 1,
            tick: 0,
            stats: ParadigmStats {
                active_paradigms: 0,
                total_paradigms_ever: 0,
                avg_health_ema: 0.5,
                avg_fitness_ema: 0.5,
                total_pressures: 0,
                total_anomalies: 0,
                revolution_pressure_ema: 0.0,
                transitions_completed: 0,
                transitions_aborted: 0,
                active_transitions: 0,
                paradigm_age_avg_ema: 0.0,
                last_tick: 0,
            },
        }
    }

    /// Register a paradigm
    pub fn register_paradigm(&mut self, domain: ParadigmDomain, name: String,
                              assumptions: Vec<u64>) -> u64 {
        let hash = fnv1a_hash(name.as_bytes());
        let id = self.stats.total_paradigms_ever;
        let paradigm = Paradigm {
            id, domain, name, core_assumptions: assumptions,
            health: ParadigmHealth::Stable, health_score: 0.8,
            fitness: 0.7, anomaly_count: 0,
            adopted_tick: self.tick, last_validated_tick: self.tick, hash,
        };
        if self.paradigms.len() < MAX_PARADIGMS {
            self.paradigms.insert(id, paradigm);
            self.stats.total_paradigms_ever += 1;
            self.stats.active_paradigms = self.paradigms.len() as u64;
        }
        id
    }

    /// Analyse the current state of all system paradigms
    pub fn system_paradigm_analysis(&mut self) -> Vec<(u64, ParadigmHealth, f32)> {
        let mut results = Vec::new();
        let ids: Vec<u64> = self.paradigms.keys().copied().collect();
        for id in ids {
            let (health_score, fitness, anomaly_count) = {
                let p = match self.paradigms.get(&id) { Some(p) => p, None => continue };
                (p.health_score, p.fitness, p.anomaly_count)
            };
            let age = self.tick.saturating_sub(
                self.paradigms.get(&id).map(|p| p.adopted_tick).unwrap_or(0)
            ) as f32;
            let age_decay = PARADIGM_INERTIA.powf(age / 1000.0);
            let anomaly_penalty = anomaly_count as f32 * ANOMALY_WEIGHT;
            let pressure_sum: f32 = self.pressures.iter()
                .filter(|pr| {
                    self.paradigms.get(&id).map(|p| p.domain == pr.domain).unwrap_or(false)
                })
                .map(|pr| pr.accumulated)
                .sum();
            let adjusted_health = (health_score * age_decay - anomaly_penalty
                - pressure_sum * 0.1).max(0.0).min(1.0);
            let health = if adjusted_health >= 0.80 { ParadigmHealth::Thriving }
                else if adjusted_health >= HEALTH_WARNING_THRESHOLD { ParadigmHealth::Stable }
                else if adjusted_health >= 0.50 { ParadigmHealth::Strained }
                else if adjusted_health >= HEALTH_CRITICAL_THRESHOLD { ParadigmHealth::Degrading }
                else if adjusted_health >= 0.15 { ParadigmHealth::Critical }
                else { ParadigmHealth::Obsolete };
            if let Some(p) = self.paradigms.get_mut(&id) {
                p.health = health;
                p.health_score = adjusted_health;
                p.last_validated_tick = self.tick;
            }
            results.push((id, health, adjusted_health));
        }
        let avg_health: f32 = if results.is_empty() { 0.5 } else {
            results.iter().map(|(_, _, h)| *h).sum::<f32>() / results.len() as f32
        };
        self.stats.avg_health_ema = self.stats.avg_health_ema
            * (1.0 - EMA_ALPHA) + avg_health * EMA_ALPHA;
        self.stats.last_tick = self.tick;
        results
    }

    /// Monitor paradigm health across the system
    #[inline]
    pub fn paradigm_health(&self) -> f32 {
        if self.paradigms.is_empty() { return 0.5; }
        let total: f32 = self.paradigms.values()
            .map(|p| p.health_score).sum();
        total / self.paradigms.len() as f32
    }

    /// Detect if a revolution (paradigm shift) is imminent
    pub fn revolution_detection(&mut self) -> Vec<(ParadigmDomain, f32)> {
        let mut revolutions = Vec::new();
        let domains = [
            ParadigmDomain::SchedulingPhilosophy, ParadigmDomain::MemoryModel,
            ParadigmDomain::IpcArchitecture, ParadigmDomain::TrustFramework,
            ParadigmDomain::EnergyStrategy, ParadigmDomain::FairnessModel,
            ParadigmDomain::ScalabilityApproach, ParadigmDomain::SecurityPosture,
        ];
        for &domain in &domains {
            let pressure_total: f32 = self.pressures.iter()
                .filter(|p| p.domain == domain)
                .map(|p| p.accumulated)
                .sum();
            let anomaly_count: u64 = self.anomalies.iter()
                .filter(|a| a.domain == domain)
                .map(|a| a.frequency)
                .sum();
            let paradigm_weakness: f32 = self.paradigms.values()
                .filter(|p| p.domain == domain)
                .map(|p| 1.0 - p.health_score)
                .sum();
            let revolution_score = (pressure_total * 0.4
                + anomaly_count as f32 * 0.1
                + paradigm_weakness * 0.5).min(1.0);
            if revolution_score >= REVOLUTION_THRESHOLD {
                revolutions.push((domain, revolution_score));
            }
        }
        let max_revolution = revolutions.iter()
            .map(|(_, s)| *s).fold(0.0f32, f32::max);
        self.stats.revolution_pressure_ema = self.stats.revolution_pressure_ema
            * (1.0 - EMA_ALPHA) + max_revolution * EMA_ALPHA;
        revolutions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        revolutions
    }

    /// Record and measure evolutionary pressure
    pub fn evolutionary_pressure(&mut self, domain: ParadigmDomain,
                                  description: String, magnitude: f32,
                                  direction: PressureDirection) {
        let existing = self.pressures.iter_mut()
            .find(|p| p.domain == domain && p.direction == direction);
        if let Some(pressure) = existing {
            pressure.accumulated += magnitude * PRESSURE_ACCUMULATION_RATE;
            pressure.magnitude = pressure.magnitude
                * (1.0 - EMA_ALPHA) + magnitude * EMA_ALPHA;
            pressure.last_observed_tick = self.tick;
        } else {
            if self.pressures.len() >= MAX_PRESSURES {
                self.pressures.pop_front();
            }
            let id = self.stats.total_pressures;
            self.pressures.push_back(EvolutionaryPressure {
                id, domain, source_description: description,
                magnitude, accumulated: magnitude * PRESSURE_ACCUMULATION_RATE,
                direction, first_detected_tick: self.tick,
                last_observed_tick: self.tick,
            });
            self.stats.total_pressures += 1;
        }
    }

    /// Record an anomaly that the current paradigm cannot explain
    pub fn record_anomaly(&mut self, domain: ParadigmDomain, severity: f32) {
        let existing = self.anomalies.iter_mut()
            .find(|a| a.domain == domain && a.severity == severity);
        if let Some(anomaly) = existing {
            anomaly.frequency += 1;
            anomaly.last_seen_tick = self.tick;
        } else {
            if self.anomalies.len() >= MAX_ANOMALIES {
                self.anomalies.pop_front();
            }
            let id = self.stats.total_anomalies;
            let desc_hash = fnv1a_hash(&[domain as u8, (self.tick & 0xFF) as u8]);
            self.anomalies.push_back(ParadigmAnomaly {
                id, domain, description_hash: desc_hash,
                severity, frequency: 1,
                first_seen_tick: self.tick, last_seen_tick: self.tick,
            });
            self.stats.total_anomalies += 1;
        }
        for p in self.paradigms.values_mut() {
            if p.domain == domain { p.anomaly_count += 1; }
        }
    }

    /// Create a paradigm transition plan
    pub fn paradigm_transition_plan(&mut self, from_id: u64, to_id: u64,
                                     domain: ParadigmDomain) -> TransitionPlan {
        let risk = {
            let from_health = self.paradigms.get(&from_id)
                .map(|p| p.health_score).unwrap_or(0.5);
            let to_fitness = self.paradigms.get(&to_id)
                .map(|p| p.fitness).unwrap_or(0.5);
            let noise = xorshift_f32(&mut self.rng_state) * 0.1;
            ((1.0 - to_fitness) * 0.5 + from_health * 0.2 + noise)
                .max(TRANSITION_RISK_FLOOR).min(1.0)
        };
        let plan_id = self.transitions.len() as u64;
        let estimated_duration = 100 + (xorshift64(&mut self.rng_state) % 200) as u64;
        let plan = TransitionPlan {
            id: plan_id, from_paradigm: from_id, to_paradigm: to_id,
            domain, phase: TransitionPhase::Assessment,
            risk_score: risk, progress: 0.0,
            rollback_ready: false,
            created_tick: self.tick,
            estimated_completion_tick: self.tick + estimated_duration,
        };
        if self.transitions.len() < MAX_TRANSITIONS {
            self.transitions.insert(plan_id, plan.clone());
            self.stats.active_transitions = self.transitions.values()
                .filter(|t| t.phase != TransitionPhase::Completed
                    && t.phase != TransitionPhase::Aborted).count() as u64;
        }
        plan
    }

    /// Get the full paradigm history
    #[inline(always)]
    pub fn paradigm_history(&self) -> &[ParadigmHistoryEntry] {
        &self.history
    }

    /// Retire a paradigm and record it in history
    pub fn retire_paradigm(&mut self, paradigm_id: u64, successor_id: u64) {
        let entry = {
            let p = match self.paradigms.get(&paradigm_id) { Some(p) => p, None => return };
            ParadigmHistoryEntry {
                paradigm_id: p.id, domain: p.domain, name: p.name.clone(),
                peak_fitness: p.fitness,
                duration_ticks: self.tick.saturating_sub(p.adopted_tick),
                adopted_tick: p.adopted_tick, retired_tick: self.tick,
                successor_id,
                retirement_reason_hash: fnv1a_hash(&[paradigm_id as u8, 0xDD]),
            }
        };
        if self.history.len() < MAX_HISTORY_LOG {
            self.history.push(entry);
        }
        self.paradigms.remove(&paradigm_id);
        self.stats.active_paradigms = self.paradigms.len() as u64;
    }

    /// Advance the engine tick
    #[inline(always)]
    pub fn tick(&mut self) { self.tick += 1; }

    /// Get current statistics
    #[inline(always)]
    pub fn stats(&self) -> &ParadigmStats { &self.stats }

    /// Get all active paradigms
    #[inline(always)]
    pub fn paradigms(&self) -> &BTreeMap<u64, Paradigm> { &self.paradigms }

    /// Get all evolutionary pressures
    #[inline(always)]
    pub fn pressures(&self) -> &[EvolutionaryPressure] { &self.pressures }

    /// Get all anomalies
    #[inline(always)]
    pub fn anomalies(&self) -> &[ParadigmAnomaly] { &self.anomalies }

    /// Get all transition plans
    #[inline(always)]
    pub fn transition_plans(&self) -> &BTreeMap<u64, TransitionPlan> { &self.transitions }
}
