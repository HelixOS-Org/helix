// SPDX-License-Identifier: GPL-2.0
//! # Holistic Experiment — System-Wide Experimentation Engine
//!
//! Runs controlled experiments that span multiple NEXUS subsystems
//! simultaneously. Where sub-module experiment engines test a single
//! protocol dimension in isolation, the holistic experiment engine tests
//! *interactions*: "Does changing the scheduler quantum AND the memory
//! prefetch distance together produce a super-additive throughput gain?"
//!
//! The engine implements factorial experimental design (full and fractional),
//! computing main effects and interaction effects from the resulting data
//! matrix. A significance matrix identifies which factor pairs produce
//! statistically meaningful interactions, and experiment synthesis merges
//! results across independent experiments into a unified conclusion.
//!
//! The engine that tests system-wide "what if?" simultaneously.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_FACTORS: usize = 12;
const MAX_LEVELS: usize = 4;
const MAX_EXPERIMENTS: usize = 256;
const MAX_TRIALS_PER_EXP: usize = 512;
const SIGNIFICANCE_ALPHA: f32 = 0.05;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const MIN_SAMPLES_FOR_SIG: usize = 8;
const EFFECT_SIZE_THRESHOLD: f32 = 0.15;
const INTERACTION_THRESHOLD: f32 = 0.10;
const SYNTHESIS_CONFIDENCE_MIN: f32 = 0.60;

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

/// An experimental factor (e.g., scheduler quantum, prefetch depth)
#[derive(Debug, Clone)]
pub struct Factor {
    pub name: String,
    pub subsystem: String,
    pub levels: Vec<f32>,
    pub current_level: usize,
}

/// A single trial: one combination of factor levels and the measured response
#[derive(Debug, Clone)]
pub struct Trial {
    pub id: u64,
    pub level_assignments: Vec<usize>,
    pub response: f32,
    pub latency: f32,
    pub throughput: f32,
    pub tick: u64,
    pub valid: bool,
}

/// Factorial design specification
#[derive(Debug, Clone)]
pub struct FactorialDesign {
    pub experiment_id: u64,
    pub factors: Vec<Factor>,
    pub is_fractional: bool,
    pub fraction_power: u32,
    pub total_combinations: usize,
    pub completed_trials: usize,
}

/// A main effect: the average impact of one factor across all levels
#[derive(Debug, Clone)]
pub struct MainEffect {
    pub factor_name: String,
    pub effect_size: f32,
    pub direction: f32,
    pub samples: usize,
    pub significant: bool,
}

/// An interaction effect between two factors
#[derive(Debug, Clone)]
pub struct InteractionEffect {
    pub factor_a: String,
    pub factor_b: String,
    pub interaction_size: f32,
    pub synergistic: bool,
    pub significant: bool,
    pub samples: usize,
}

/// Status of an experiment
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExperimentStatus {
    Designed,
    Running,
    Completed,
    Analysed,
    Synthesised,
    Failed,
}

/// A complete experiment record
#[derive(Debug, Clone)]
pub struct ExperimentRecord {
    pub id: u64,
    pub name: String,
    pub design: FactorialDesign,
    pub trials: Vec<Trial>,
    pub main_effects: Vec<MainEffect>,
    pub interactions: Vec<InteractionEffect>,
    pub status: ExperimentStatus,
    pub conclusion: String,
    pub created_tick: u64,
    pub completed_tick: u64,
}

/// Significance matrix entry
#[derive(Debug, Clone)]
pub struct SignificanceEntry {
    pub row_factor: String,
    pub col_factor: String,
    pub p_approx: f32,
    pub significant: bool,
    pub effect: f32,
}

/// Experiment engine statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ExperimentStats {
    pub total_experiments: u64,
    pub running_count: u64,
    pub completed_count: u64,
    pub total_trials: u64,
    pub significant_main_effects: u64,
    pub significant_interactions: u64,
    pub avg_effect_size_ema: f32,
    pub synthesis_count: u64,
}

// ============================================================================
// HOLISTIC EXPERIMENT ENGINE
// ============================================================================

/// System-wide experimentation engine with factorial design
pub struct HolisticExperiment {
    experiments: BTreeMap<u64, ExperimentRecord>,
    significance_matrix: Vec<SignificanceEntry>,
    synthesis_log: Vec<(u64, String)>,
    rng_state: u64,
    stats: ExperimentStats,
}

impl HolisticExperiment {
    /// Create a new experiment engine
    pub fn new(seed: u64) -> Self {
        Self {
            experiments: BTreeMap::new(),
            significance_matrix: Vec::new(),
            synthesis_log: Vec::new(),
            rng_state: seed | 1,
            stats: ExperimentStats {
                total_experiments: 0,
                running_count: 0,
                completed_count: 0,
                total_trials: 0,
                significant_main_effects: 0,
                significant_interactions: 0,
                avg_effect_size_ema: 0.0,
                synthesis_count: 0,
            },
        }
    }

    /// Create a factorial design for a set of factors
    pub fn factorial_design(
        &mut self,
        name: String,
        factors: Vec<Factor>,
        fractional: bool,
        tick: u64,
    ) -> u64 {
        let id = fnv1a_hash(name.as_bytes()) ^ fnv1a_hash(&tick.to_le_bytes());
        let total = if fractional {
            let base: usize = factors.iter().map(|f| f.levels.len()).product();
            base / 2_usize.max(1)
        } else {
            factors.iter().map(|f| f.levels.len()).product()
        };
        let design = FactorialDesign {
            experiment_id: id,
            factors,
            is_fractional: fractional,
            fraction_power: if fractional { 1 } else { 0 },
            total_combinations: total,
            completed_trials: 0,
        };
        let record = ExperimentRecord {
            id,
            name,
            design,
            trials: Vec::new(),
            main_effects: Vec::new(),
            interactions: Vec::new(),
            status: ExperimentStatus::Designed,
            conclusion: String::new(),
            created_tick: tick,
            completed_tick: 0,
        };
        self.experiments.insert(id, record);
        self.stats.total_experiments += 1;
        id
    }

    /// Execute the next batch of trials for an experiment
    pub fn run_global_experiment(&mut self, exp_id: u64, tick: u64) -> usize {
        let exp = match self.experiments.get_mut(&exp_id) {
            Some(e) => e,
            None => return 0,
        };
        exp.status = ExperimentStatus::Running;
        let n_factors = exp.design.factors.len();
        if n_factors == 0 {
            return 0;
        }
        let remaining = exp
            .design
            .total_combinations
            .saturating_sub(exp.design.completed_trials);
        let batch = remaining.min(32);
        let mut trials_run = 0;
        for _ in 0..batch {
            if exp.trials.len() >= MAX_TRIALS_PER_EXP {
                break;
            }
            let mut assignments = Vec::new();
            for f in &exp.design.factors {
                let lvl = (xorshift64(&mut self.rng_state) % f.levels.len() as u64) as usize;
                assignments.push(lvl);
            }
            let response = self.simulate_response(&exp.design.factors, &assignments);
            let trial_id = fnv1a_hash(&self.stats.total_trials.to_le_bytes());
            exp.trials.push(Trial {
                id: trial_id,
                level_assignments: assignments,
                response,
                latency: xorshift_f32(&mut self.rng_state) * 10.0,
                throughput: response * 100.0,
                tick,
                valid: true,
            });
            exp.design.completed_trials += 1;
            self.stats.total_trials += 1;
            trials_run += 1;
        }
        if exp.design.completed_trials >= exp.design.total_combinations {
            exp.status = ExperimentStatus::Completed;
            exp.completed_tick = tick;
        }
        self.refresh_counts();
        trials_run
    }

    /// Compute interaction effects between factor pairs
    pub fn interaction_effects(&mut self, exp_id: u64) -> Vec<InteractionEffect> {
        let exp = match self.experiments.get_mut(&exp_id) {
            Some(e) => e,
            None => return Vec::new(),
        };
        let mut interactions = Vec::new();
        let n = exp.design.factors.len();
        let valid_trials: Vec<&Trial> = exp.trials.iter().filter(|t| t.valid).collect();
        let grand_mean =
            valid_trials.iter().map(|t| t.response).sum::<f32>() / valid_trials.len().max(1) as f32;
        for i in 0..n {
            for j in (i + 1)..n {
                let mut groups: BTreeMap<(usize, usize), Vec<f32>> = BTreeMap::new();
                for trial in &valid_trials {
                    if trial.level_assignments.len() <= j {
                        continue;
                    }
                    let key = (trial.level_assignments[i], trial.level_assignments[j]);
                    groups
                        .entry(key)
                        .or_insert_with(Vec::new)
                        .push(trial.response);
                }
                let (mut isum, mut count) = (0.0f32, 0usize);
                for (_, vals) in &groups {
                    isum +=
                        (vals.iter().sum::<f32>() / vals.len().max(1) as f32 - grand_mean).abs();
                    count += vals.len();
                }
                let effect = if count > 0 {
                    isum / groups.len().max(1) as f32
                } else {
                    0.0
                };
                let sig = effect > INTERACTION_THRESHOLD && count >= MIN_SAMPLES_FOR_SIG;
                if sig {
                    self.stats.significant_interactions += 1;
                }
                interactions.push(InteractionEffect {
                    factor_a: exp.design.factors[i].name.clone(),
                    factor_b: exp.design.factors[j].name.clone(),
                    interaction_size: effect,
                    synergistic: effect > 0.0,
                    significant: sig,
                    samples: count,
                });
            }
        }
        exp.interactions = interactions.clone();
        interactions
    }

    /// Compute main effects for each factor
    #[inline]
    pub fn main_effects(&mut self, exp_id: u64) -> Vec<MainEffect> {
        let exp = match self.experiments.get_mut(&exp_id) {
            Some(e) => e,
            None => return Vec::new(),
        };
        let mut effects = Vec::new();
        let grand_mean = exp
            .trials
            .iter()
            .filter(|t| t.valid)
            .map(|t| t.response)
            .sum::<f32>()
            / exp.trials.iter().filter(|t| t.valid).count().max(1) as f32;
        for (fi, factor) in exp.design.factors.iter().enumerate() {
            let mut level_means: Vec<(usize, f32, usize)> = Vec::new();
            for lvl in 0..factor.levels.len() {
                let vals: Vec<f32> = exp
                    .trials
                    .iter()
                    .filter(|t| {
                        t.valid && t.level_assignments.len() > fi && t.level_assignments[fi] == lvl
                    })
                    .map(|t| t.response)
                    .collect();
                if !vals.is_empty() {
                    let mean = vals.iter().sum::<f32>() / vals.len() as f32;
                    level_means.push((lvl, mean, vals.len()));
                }
            }
            let effect_size: f32 = level_means
                .iter()
                .map(|(_, m, _)| (*m - grand_mean).abs())
                .sum::<f32>()
                / level_means.len().max(1) as f32;
            let total_samples: usize = level_means.iter().map(|(_, _, n)| n).sum();
            let direction = level_means
                .last()
                .map(|(_, m, _)| *m - grand_mean)
                .unwrap_or(0.0);
            let sig = effect_size > EFFECT_SIZE_THRESHOLD && total_samples >= MIN_SAMPLES_FOR_SIG;
            if sig {
                self.stats.significant_main_effects += 1;
            }
            self.stats.avg_effect_size_ema =
                EMA_ALPHA * effect_size + (1.0 - EMA_ALPHA) * self.stats.avg_effect_size_ema;
            effects.push(MainEffect {
                factor_name: factor.name.clone(),
                effect_size,
                direction,
                samples: total_samples,
                significant: sig,
            });
        }
        exp.main_effects = effects.clone();
        effects
    }

    /// Build a significance matrix of all factor×factor interactions
    pub fn significance_matrix(&mut self, exp_id: u64) -> &[SignificanceEntry] {
        self.significance_matrix.clear();
        let exp = match self.experiments.get(&exp_id) {
            Some(e) => e,
            None => return &self.significance_matrix,
        };
        for ie in &exp.interactions {
            let p_approx = if ie.samples >= MIN_SAMPLES_FOR_SIG && ie.interaction_size > 0.0 {
                (1.0 / (1.0 + ie.interaction_size * ie.samples as f32)).min(1.0)
            } else {
                1.0
            };
            self.significance_matrix.push(SignificanceEntry {
                row_factor: ie.factor_a.clone(),
                col_factor: ie.factor_b.clone(),
                p_approx,
                significant: p_approx < SIGNIFICANCE_ALPHA,
                effect: ie.interaction_size,
            });
        }
        &self.significance_matrix
    }

    /// Synthesise conclusions across multiple completed experiments
    pub fn experiment_synthesis(&mut self) -> Vec<(u64, String)> {
        let mut conclusions = Vec::new();
        for (&id, exp) in &self.experiments {
            if exp.status != ExperimentStatus::Completed && exp.status != ExperimentStatus::Analysed
            {
                continue;
            }
            let sig_main: Vec<&MainEffect> =
                exp.main_effects.iter().filter(|m| m.significant).collect();
            let sig_int: Vec<&InteractionEffect> =
                exp.interactions.iter().filter(|i| i.significant).collect();
            if sig_main.is_empty() && sig_int.is_empty() {
                continue;
            }
            let mut summary = String::from("EXP:");
            for m in &sig_main {
                summary.push_str(&m.factor_name);
                summary.push(':');
            }
            for i in &sig_int {
                summary.push_str(&i.factor_a);
                summary.push('*');
                summary.push_str(&i.factor_b);
                summary.push(':');
            }
            conclusions.push((id, summary.clone()));
            self.synthesis_log.push((id, summary));
        }
        self.stats.synthesis_count = self.synthesis_log.len() as u64;
        conclusions
    }

    /// Current statistics snapshot
    #[inline(always)]
    pub fn stats(&self) -> &ExperimentStats {
        &self.stats
    }

    // ── private helpers ─────────────────────────────────────────────────

    fn simulate_response(&mut self, factors: &[Factor], assignments: &[usize]) -> f32 {
        let mut response = 0.0f32;
        for (i, factor) in factors.iter().enumerate() {
            if i < assignments.len() && assignments[i] < factor.levels.len() {
                response += factor.levels[assignments[i]];
            }
        }
        let noise = (xorshift_f32(&mut self.rng_state) - 0.5) * 0.1;
        (response / factors.len().max(1) as f32 + noise).max(0.0)
    }

    fn refresh_counts(&mut self) {
        let mut running = 0u64;
        let mut completed = 0u64;
        for (_, exp) in &self.experiments {
            match exp.status {
                ExperimentStatus::Running => running += 1,
                ExperimentStatus::Completed
                | ExperimentStatus::Analysed
                | ExperimentStatus::Synthesised => completed += 1,
                _ => {},
            }
        }
        self.stats.running_count = running;
        self.stats.completed_count = completed;
    }
}
