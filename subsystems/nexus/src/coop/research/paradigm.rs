// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Paradigm — Paradigm Shift Detection in Cooperation
//!
//! Detects when fundamental assumptions about cooperation no longer hold
//! and entirely new paradigms are needed. Classical game theory assumes
//! rational actors and stable payoff matrices — but when subsystem
//! populations change, workload characteristics shift, or new resource
//! types emerge, old equilibria break down. This engine monitors for
//! assumption violations, equilibrium instability, and the emergence of
//! new cooperative game models. It chronicles paradigm transitions and
//! helps the cooperation system adapt to genuinely new realities.
//!
//! The engine that detects when the rules of the game have changed.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_PARADIGMS: usize = 64;
const MAX_ASSUMPTIONS: usize = 128;
const MAX_EQUILIBRIA: usize = 64;
const MAX_GAME_MODELS: usize = 32;
const ASSUMPTION_VIOLATION_THRESHOLD: f32 = 0.30;
const EQUILIBRIUM_INSTABILITY_THRESHOLD: f32 = 0.25;
const PARADIGM_SHIFT_EVIDENCE_MIN: usize = 5;
const EMA_ALPHA: f32 = 0.12;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const ASSUMPTION_DECAY_RATE: f32 = 0.002;
const EQUILIBRIUM_CHECK_WINDOW: usize = 50;
const CHRONICLE_MAX: usize = 128;
const TRANSITION_GRACE_PERIOD: u64 = 1000;
const NEW_MODEL_BONUS: f32 = 0.15;

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
// PARADIGM TYPES
// ============================================================================

/// Type of cooperation paradigm
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParadigmType {
    ClassicalGameTheory,
    EvolutionaryCooperation,
    MechanismDesign,
    BayesianNegotiation,
    MultiAgentLearning,
    EmergentCooperation,
    HybridParadigm,
}

/// Status of an assumption
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AssumptionStatus {
    Valid,
    Weakening,
    Violated,
    Retired,
}

/// Phase of a paradigm shift
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShiftPhase {
    Anomalies,
    Crisis,
    Transition,
    NewNormal,
    Consolidated,
}

/// A fundamental assumption about cooperation
#[derive(Debug, Clone)]
pub struct CoopAssumption {
    pub id: u64,
    pub paradigm: ParadigmType,
    pub statement: String,
    pub validity_score: f32,
    pub violation_count: u32,
    pub test_count: u32,
    pub status: AssumptionStatus,
    pub created_tick: u64,
    pub last_tested_tick: u64,
}

/// A cooperation equilibrium being monitored
#[derive(Debug, Clone)]
pub struct CoopEquilibrium {
    pub id: u64,
    pub paradigm: ParadigmType,
    pub description: String,
    pub stability_score: f32,
    pub stability_history: VecDeque<f32>,
    pub payoff_matrix_hash: u64,
    pub player_count: u32,
    pub stable: bool,
    pub created_tick: u64,
}

/// A new game model that has emerged
#[derive(Debug, Clone)]
pub struct NewGameModel {
    pub id: u64,
    pub name: String,
    pub paradigm: ParadigmType,
    pub assumptions: Vec<u64>,
    pub performance_score: f32,
    pub novelty_score: f32,
    pub created_tick: u64,
    pub validated: bool,
}

/// A paradigm shift event
#[derive(Debug, Clone)]
pub struct ParadigmShift {
    pub id: u64,
    pub from_paradigm: ParadigmType,
    pub to_paradigm: ParadigmType,
    pub phase: ShiftPhase,
    pub evidence_count: usize,
    pub violation_severity: f32,
    pub trigger_description: String,
    pub detected_tick: u64,
    pub transition_complete_tick: u64,
}

/// Chronicle entry for paradigm shifts
#[derive(Debug, Clone)]
pub struct ShiftChronicle {
    pub shift_id: u64,
    pub from: ParadigmType,
    pub to: ParadigmType,
    pub tick: u64,
    pub impact: f32,
    pub duration: u64,
}

// ============================================================================
// PARADIGM STATS
// ============================================================================

/// Aggregate statistics for paradigm detection
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct ParadigmStats {
    pub total_assumptions_tested: u64,
    pub violations_detected: u64,
    pub paradigm_shifts_detected: u64,
    pub active_paradigm_count: u64,
    pub equilibria_monitored: u64,
    pub unstable_equilibria: u64,
    pub new_models_created: u64,
    pub avg_assumption_validity_ema: f32,
    pub avg_equilibrium_stability_ema: f32,
    pub chronicle_size: u64,
}

// ============================================================================
// COOPERATION PARADIGM
// ============================================================================

/// Engine for detecting paradigm shifts in cooperation
#[derive(Debug)]
pub struct CoopParadigm {
    assumptions: BTreeMap<u64, CoopAssumption>,
    equilibria: BTreeMap<u64, CoopEquilibrium>,
    game_models: Vec<NewGameModel>,
    shifts: VecDeque<ParadigmShift>,
    chronicle: VecDeque<ShiftChronicle>,
    current_paradigm: ParadigmType,
    rng_state: u64,
    tick: u64,
    stats: ParadigmStats,
}

impl CoopParadigm {
    /// Create a new paradigm detector with the given PRNG seed
    pub fn new(seed: u64) -> Self {
        Self {
            assumptions: BTreeMap::new(),
            equilibria: BTreeMap::new(),
            game_models: Vec::new(),
            shifts: VecDeque::new(),
            chronicle: VecDeque::new(),
            current_paradigm: ParadigmType::ClassicalGameTheory,
            rng_state: seed | 1,
            tick: 0,
            stats: ParadigmStats::default(),
        }
    }

    /// Register a fundamental assumption about the current cooperation paradigm
    pub fn register_assumption(&mut self, paradigm: ParadigmType, statement: String) -> u64 {
        let id = fnv1a_hash(statement.as_bytes()) ^ xorshift64(&mut self.rng_state);
        let assumption = CoopAssumption {
            id,
            paradigm,
            statement,
            validity_score: 1.0,
            violation_count: 0,
            test_count: 0,
            status: AssumptionStatus::Valid,
            created_tick: self.tick,
            last_tested_tick: self.tick,
        };
        if self.assumptions.len() < MAX_ASSUMPTIONS {
            self.assumptions.insert(id, assumption);
        }
        id
    }

    /// Register an equilibrium to monitor
    pub fn register_equilibrium(
        &mut self,
        paradigm: ParadigmType,
        description: String,
        player_count: u32,
    ) -> u64 {
        let id = fnv1a_hash(description.as_bytes()) ^ xorshift64(&mut self.rng_state);
        let payoff_hash = fnv1a_hash(&player_count.to_le_bytes()) ^ id;
        let equilibrium = CoopEquilibrium {
            id,
            paradigm,
            description,
            stability_score: 1.0,
            stability_history: VecDeque::new(),
            payoff_matrix_hash: payoff_hash,
            player_count,
            stable: true,
            created_tick: self.tick,
        };
        if self.equilibria.len() < MAX_EQUILIBRIA {
            self.equilibria.insert(id, equilibrium);
            self.stats.equilibria_monitored = self.equilibria.len() as u64;
        }
        id
    }

    /// Detect a paradigm shift based on accumulated evidence
    pub fn detect_coop_paradigm_shift(&mut self) -> Option<ParadigmShift> {
        self.tick += 1;
        // Count violated assumptions in current paradigm
        let violated_count = self
            .assumptions
            .values()
            .filter(|a| a.paradigm == self.current_paradigm && a.status == AssumptionStatus::Violated)
            .count();
        let total_current = self
            .assumptions
            .values()
            .filter(|a| a.paradigm == self.current_paradigm)
            .count();
        if total_current == 0 {
            return None;
        }
        let violation_ratio = violated_count as f32 / total_current as f32;
        // Count unstable equilibria
        let unstable_count = self
            .equilibria
            .values()
            .filter(|e| e.paradigm == self.current_paradigm && !e.stable)
            .count();
        let total_eq = self
            .equilibria
            .values()
            .filter(|e| e.paradigm == self.current_paradigm)
            .count();
        let instability_ratio = if total_eq > 0 {
            unstable_count as f32 / total_eq as f32
        } else {
            0.0
        };
        let severity = violation_ratio * 0.6 + instability_ratio * 0.4;
        if severity < ASSUMPTION_VIOLATION_THRESHOLD || violated_count < PARADIGM_SHIFT_EVIDENCE_MIN {
            return None;
        }
        // Determine which paradigm to shift to
        let new_paradigm = self.suggest_new_paradigm(severity);
        let shift_id = fnv1a_hash(&self.tick.to_le_bytes()) ^ xorshift64(&mut self.rng_state);
        let shift = ParadigmShift {
            id: shift_id,
            from_paradigm: self.current_paradigm,
            to_paradigm: new_paradigm,
            phase: if severity > 0.6 {
                ShiftPhase::Crisis
            } else {
                ShiftPhase::Anomalies
            },
            evidence_count: violated_count,
            violation_severity: severity,
            trigger_description: String::from("Accumulated assumption violations exceed threshold"),
            detected_tick: self.tick,
            transition_complete_tick: 0,
        };
        self.stats.paradigm_shifts_detected += 1;
        self.add_shift_chronicle(&shift);
        if self.shifts.len() >= MAX_PARADIGMS {
            self.shifts.pop_front();
        }
        self.shifts.push_back(shift.clone());
        // Begin transition
        self.current_paradigm = new_paradigm;
        self.stats.active_paradigm_count = self.count_active_paradigms();
        Some(shift)
    }

    /// Test a specific assumption against observed data
    #[inline]
    pub fn assumption_testing(
        &mut self,
        assumption_id: u64,
        observed_value: f32,
        expected_value: f32,
    ) -> bool {
        self.tick += 1;
        self.stats.total_assumptions_tested += 1;
        let assumption = match self.assumptions.get_mut(&assumption_id) {
            Some(a) => a,
            None => return false,
        };
        assumption.test_count += 1;
        assumption.last_tested_tick = self.tick;
        let deviation = (observed_value - expected_value).abs();
        let relative_deviation = if expected_value.abs() > 0.001 {
            deviation / expected_value.abs()
        } else {
            deviation
        };
        let holds = relative_deviation < ASSUMPTION_VIOLATION_THRESHOLD;
        if holds {
            assumption.validity_score =
                EMA_ALPHA * 1.0 + (1.0 - EMA_ALPHA) * assumption.validity_score;
        } else {
            assumption.violation_count += 1;
            assumption.validity_score =
                EMA_ALPHA * 0.0 + (1.0 - EMA_ALPHA) * assumption.validity_score;
            self.stats.violations_detected += 1;
        }
        // Update status
        assumption.status = if assumption.validity_score >= 0.7 {
            AssumptionStatus::Valid
        } else if assumption.validity_score >= 0.4 {
            AssumptionStatus::Weakening
        } else {
            AssumptionStatus::Violated
        };
        // Update global EMA
        self.stats.avg_assumption_validity_ema =
            EMA_ALPHA * assumption.validity_score
                + (1.0 - EMA_ALPHA) * self.stats.avg_assumption_validity_ema;
        holds
    }

    /// Monitor an equilibrium for stability changes
    #[inline]
    pub fn equilibrium_change(
        &mut self,
        equilibrium_id: u64,
        current_stability: f32,
    ) -> bool {
        self.tick += 1;
        let eq = match self.equilibria.get_mut(&equilibrium_id) {
            Some(e) => e,
            None => return false,
        };
        eq.stability_history.push(current_stability);
        if eq.stability_history.len() > EQUILIBRIUM_CHECK_WINDOW {
            eq.stability_history.pop_front().unwrap();
        }
        eq.stability_score = EMA_ALPHA * current_stability + (1.0 - EMA_ALPHA) * eq.stability_score;
        let prev_stable = eq.stable;
        eq.stable = eq.stability_score >= (1.0 - EQUILIBRIUM_INSTABILITY_THRESHOLD);
        if prev_stable && !eq.stable {
            self.stats.unstable_equilibria += 1;
        } else if !prev_stable && eq.stable && self.stats.unstable_equilibria > 0 {
            self.stats.unstable_equilibria -= 1;
        }
        self.stats.avg_equilibrium_stability_ema =
            EMA_ALPHA * current_stability
                + (1.0 - EMA_ALPHA) * self.stats.avg_equilibrium_stability_ema;
        eq.stable
    }

    /// Create a new game model for an emerging cooperation paradigm
    pub fn new_game_model(
        &mut self,
        name: String,
        paradigm: ParadigmType,
        assumption_ids: Vec<u64>,
        initial_performance: f32,
    ) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(name.as_bytes()) ^ xorshift64(&mut self.rng_state);
        // Compute novelty relative to existing models
        let novelty = self.compute_model_novelty(&name, paradigm);
        let model = NewGameModel {
            id,
            name,
            paradigm,
            assumptions: assumption_ids,
            performance_score: initial_performance + novelty * NEW_MODEL_BONUS,
            novelty_score: novelty,
            created_tick: self.tick,
            validated: false,
        };
        if self.game_models.len() >= MAX_GAME_MODELS {
            // Remove worst performing model
            let min_idx = self
                .game_models
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    a.performance_score
                        .partial_cmp(&b.performance_score)
                        .unwrap_or(core::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i);
            if let Some(idx) = min_idx {
                self.game_models.remove(idx);
            }
        }
        self.game_models.push(model);
        self.stats.new_models_created += 1;
        id
    }

    /// Manage the transition between paradigms
    pub fn paradigm_transition(&mut self) -> Option<ShiftPhase> {
        self.tick += 1;
        let active_shift = self.shifts.iter_mut().find(|s| {
            s.transition_complete_tick == 0
                && !matches!(s.phase, ShiftPhase::Consolidated | ShiftPhase::NewNormal)
        });
        let shift = match active_shift {
            Some(s) => s,
            None => return None,
        };
        let elapsed = self.tick.saturating_sub(shift.detected_tick);
        // Progress through phases
        let new_phase = match shift.phase {
            ShiftPhase::Anomalies => {
                if shift.violation_severity > 0.5 {
                    ShiftPhase::Crisis
                } else {
                    ShiftPhase::Anomalies
                }
            }
            ShiftPhase::Crisis => {
                if elapsed > TRANSITION_GRACE_PERIOD / 2 {
                    ShiftPhase::Transition
                } else {
                    ShiftPhase::Crisis
                }
            }
            ShiftPhase::Transition => {
                if elapsed > TRANSITION_GRACE_PERIOD {
                    ShiftPhase::NewNormal
                } else {
                    ShiftPhase::Transition
                }
            }
            ShiftPhase::NewNormal => {
                shift.transition_complete_tick = self.tick;
                ShiftPhase::Consolidated
            }
            ShiftPhase::Consolidated => ShiftPhase::Consolidated,
        };
        shift.phase = new_phase;
        Some(new_phase)
    }

    /// Get the chronicle of all paradigm shifts
    #[inline(always)]
    pub fn shift_chronicle(&self) -> &[ShiftChronicle] {
        &self.chronicle
    }

    /// Get the current cooperation paradigm
    #[inline(always)]
    pub fn current_paradigm(&self) -> ParadigmType {
        self.current_paradigm
    }

    /// Get current paradigm detection statistics
    #[inline(always)]
    pub fn stats(&self) -> &ParadigmStats {
        &self.stats
    }

    /// Number of registered assumptions
    #[inline(always)]
    pub fn assumption_count(&self) -> usize {
        self.assumptions.len()
    }

    /// Number of monitored equilibria
    #[inline(always)]
    pub fn equilibrium_count(&self) -> usize {
        self.equilibria.len()
    }

    /// Number of paradigm shifts detected
    #[inline(always)]
    pub fn shift_count(&self) -> usize {
        self.shifts.len()
    }

    // ========================================================================
    // INTERNAL HELPERS
    // ========================================================================

    fn suggest_new_paradigm(&self, severity: f32) -> ParadigmType {
        match self.current_paradigm {
            ParadigmType::ClassicalGameTheory => {
                if severity > 0.7 {
                    ParadigmType::MultiAgentLearning
                } else {
                    ParadigmType::EvolutionaryCooperation
                }
            }
            ParadigmType::EvolutionaryCooperation => {
                if severity > 0.7 {
                    ParadigmType::EmergentCooperation
                } else {
                    ParadigmType::MechanismDesign
                }
            }
            ParadigmType::MechanismDesign => {
                ParadigmType::BayesianNegotiation
            }
            ParadigmType::BayesianNegotiation => {
                ParadigmType::MultiAgentLearning
            }
            ParadigmType::MultiAgentLearning => {
                ParadigmType::EmergentCooperation
            }
            ParadigmType::EmergentCooperation => {
                ParadigmType::HybridParadigm
            }
            ParadigmType::HybridParadigm => {
                ParadigmType::ClassicalGameTheory // full cycle
            }
        }
    }

    fn compute_model_novelty(&self, name: &str, paradigm: ParadigmType) -> f32 {
        let name_hash = fnv1a_hash(name.as_bytes());
        let existing_count = self
            .game_models
            .iter()
            .filter(|m| m.paradigm == paradigm)
            .count();
        if existing_count == 0 {
            return 1.0;
        }
        let mut min_hash_dist = u64::MAX;
        for model in &self.game_models {
            let model_hash = fnv1a_hash(model.name.as_bytes());
            let dist = (name_hash ^ model_hash).count_ones() as u64;
            if dist < min_hash_dist {
                min_hash_dist = dist;
            }
        }
        let novelty = min_hash_dist as f32 / 64.0;
        novelty.min(1.0)
    }

    fn add_shift_chronicle(&mut self, shift: &ParadigmShift) {
        let entry = ShiftChronicle {
            shift_id: shift.id,
            from: shift.from_paradigm,
            to: shift.to_paradigm,
            tick: self.tick,
            impact: shift.violation_severity,
            duration: 0,
        };
        if self.chronicle.len() >= CHRONICLE_MAX {
            self.chronicle.pop_front();
        }
        self.chronicle.push_back(entry);
        self.stats.chronicle_size = self.chronicle.len() as u64;
    }

    fn count_active_paradigms(&self) -> u64 {
        let mut paradigms: Vec<ParadigmType> = Vec::new();
        paradigms.push(self.current_paradigm);
        for model in &self.game_models {
            if !paradigms.contains(&model.paradigm) {
                paradigms.push(model.paradigm);
            }
        }
        paradigms.len() as u64
    }
}
