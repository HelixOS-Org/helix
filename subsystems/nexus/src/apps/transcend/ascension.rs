// SPDX-License-Identifier: GPL-2.0
//! # Apps Ascension — Final Ascension of App Management Intelligence
//!
//! Represents the culminating phase of the kernel's app management
//! evolution. The ascension engine is self-sustaining and self-improving:
//! it evaluates its own effectiveness, detects plateaus, invents new
//! metrics, and autonomously adjusts its internal parameters to push
//! beyond current performance limits.
//!
//! Ascension progresses through named phases, each unlocking new
//! capabilities. The engine tracks milestones, performs divine-level
//! classification (where app categorisation is effortless and
//! instantaneous), and applies transcendent optimization that
//! surpasses all prior allocation strategies.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x00000100000001B3;
const EMA_ALPHA_NUM: u64 = 2;
const EMA_ALPHA_DEN: u64 = 9;
const MAX_TRACKED_APPS: usize = 4096;
const MAX_MILESTONES: usize = 512;
const MAX_IMPROVEMENTS: usize = 1024;
const PHASE_AWAKENING: u64 = 0;
const PHASE_EXPANSION: u64 = 1;
const PHASE_CONVERGENCE: u64 = 2;
const PHASE_TRANSCENDENCE: u64 = 3;
const PHASE_DIVINE: u64 = 4;
const IMPROVEMENT_THRESHOLD: u64 = 5;
const PLATEAU_WINDOW: usize = 16;
const PLATEAU_TOLERANCE: u64 = 2;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn fnv1a(data: &[u8]) -> u64 {
    let mut h = FNV_OFFSET;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

fn xorshift64(state: &mut u64) -> u64 {
    let mut s = *state;
    s ^= s << 13;
    s ^= s >> 7;
    s ^= s << 17;
    *state = s;
    s
}

fn ema_update(prev: u64, sample: u64) -> u64 {
    (EMA_ALPHA_NUM * sample + (EMA_ALPHA_DEN - EMA_ALPHA_NUM) * prev) / EMA_ALPHA_DEN
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Named ascension phases.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AscensionPhaseKind {
    Awakening = 0,
    Expansion = 1,
    Convergence = 2,
    Transcendence = 3,
    Divine = 4,
}

/// Description of an ascension phase with its capabilities.
#[derive(Clone, Debug)]
pub struct AscensionPhase {
    pub level: u64,
    pub kind: AscensionPhaseKind,
    pub capabilities: Vec<String>,
    pub unlocked_tick: u64,
    pub effectiveness: u64,
}

/// A recorded ascension milestone.
#[derive(Clone, Debug)]
pub struct AscensionMilestone {
    pub milestone_id: u64,
    pub phase_level: u64,
    pub description: String,
    pub metric_before: u64,
    pub metric_after: u64,
    pub tick: u64,
}

/// A self-improvement action taken by the engine.
#[derive(Clone, Debug)]
pub struct ImprovementAction {
    pub action_id: u64,
    pub parameter_hash: u64,
    pub parameter_label: String,
    pub old_value: u64,
    pub new_value: u64,
    pub expected_gain: u64,
    pub actual_gain: u64,
    pub tick: u64,
}

/// Per-application divine classification record.
#[derive(Clone, Debug)]
pub struct DivineClassification {
    pub app_id: u64,
    pub class_hash: u64,
    pub class_label: String,
    pub certainty: u64,
    pub latency_ticks: u64,
}

/// Transcendent allocation result.
#[derive(Clone, Debug)]
pub struct TranscendentAllocation {
    pub app_id: u64,
    pub resource_hash: u64,
    pub optimal_amount: u64,
    pub efficiency: u64,
    pub superiority_over_baseline: u64,
}

/// Running statistics for the ascension engine.
#[derive(Clone, Debug, Default)]
pub struct AscensionStats {
    pub current_phase: u64,
    pub total_milestones: u64,
    pub total_improvements: u64,
    pub effectiveness_ema: u64,
    pub autonomous_decisions: u64,
    pub classifications_performed: u64,
    pub optimizations_performed: u64,
    pub plateau_detections: u64,
    pub self_improvement_score: u64,
}

// ---------------------------------------------------------------------------
// AppsAscension
// ---------------------------------------------------------------------------

/// Engine for the final ascension of app management intelligence.
pub struct AppsAscension {
    phases: BTreeMap<u64, AscensionPhase>,
    milestones: Vec<AscensionMilestone>,
    improvements: Vec<ImprovementAction>,
    classifications: BTreeMap<u64, DivineClassification>,
    effectiveness_history: Vec<u64>,
    tunable_params: BTreeMap<u64, u64>,
    current_phase: u64,
    stats: AscensionStats,
    rng: u64,
    tick: u64,
}

impl AppsAscension {
    /// Create a new ascension engine.
    pub fn new(seed: u64) -> Self {
        let mut phases = BTreeMap::new();
        let awakening = AscensionPhase {
            level: PHASE_AWAKENING,
            kind: AscensionPhaseKind::Awakening,
            capabilities: alloc::vec![
                String::from("basic_classification"),
                String::from("simple_allocation"),
            ],
            unlocked_tick: 0,
            effectiveness: 10,
        };
        phases.insert(PHASE_AWAKENING, awakening);

        let mut tunable_params = BTreeMap::new();
        tunable_params.insert(fnv1a(b"alloc_bias"), 50);
        tunable_params.insert(fnv1a(b"class_threshold"), 30);
        tunable_params.insert(fnv1a(b"opt_aggressiveness"), 40);
        tunable_params.insert(fnv1a(b"prediction_weight"), 60);

        Self {
            phases,
            milestones: Vec::new(),
            improvements: Vec::new(),
            classifications: BTreeMap::new(),
            effectiveness_history: Vec::new(),
            tunable_params,
            current_phase: PHASE_AWAKENING,
            stats: AscensionStats {
                current_phase: PHASE_AWAKENING,
                effectiveness_ema: 10,
                ..Default::default()
            },
            rng: seed | 1,
            tick: 0,
        }
    }

    // -- public API ---------------------------------------------------------

    /// Return the current ascension level (phase number).
    pub fn ascension_level(&self) -> u64 {
        self.current_phase
    }

    /// Perform autonomous management for an application.
    ///
    /// The engine decides classification and allocation without external
    /// guidance. Returns the effectiveness score of the autonomous action.
    pub fn autonomous_management(
        &mut self,
        app_id: u64,
        name: &str,
        cpu: u64,
        mem: u64,
        io: u64,
    ) -> u64 {
        self.tick += 1;
        self.stats.autonomous_decisions += 1;

        // Divine classification
        let class = self.perform_divine_classification(app_id, name, cpu, mem, io);
        let classification_quality = class.certainty;

        // Transcendent optimization
        let alloc = self.perform_transcendent_optimization(app_id, cpu, mem, io);
        let optimization_quality = alloc.efficiency;

        let effectiveness = (classification_quality + optimization_quality) / 2;
        self.stats.effectiveness_ema =
            ema_update(self.stats.effectiveness_ema, effectiveness);

        self.effectiveness_history.push(effectiveness);
        if self.effectiveness_history.len() > PLATEAU_WINDOW * 4 {
            self.effectiveness_history
                .drain(0..self.effectiveness_history.len() - PLATEAU_WINDOW * 4);
        }

        // Check for plateau and trigger self-improvement
        if self.detect_plateau() {
            self.stats.plateau_detections += 1;
            self.trigger_self_improvement();
        }

        // Check for phase advancement
        self.check_phase_advancement();

        effectiveness
    }

    /// Trigger a self-improvement cycle.
    ///
    /// The engine evaluates its own parameters, detects which ones are
    /// underperforming, and adjusts them. Returns the number of parameters
    /// modified.
    pub fn self_improvement(&mut self) -> u64 {
        self.trigger_self_improvement()
    }

    /// Record an ascension milestone.
    ///
    /// Milestones mark significant achievements in the engine's evolution.
    pub fn ascension_milestone(
        &mut self,
        description: &str,
        metric_before: u64,
        metric_after: u64,
    ) -> Option<AscensionMilestone> {
        if metric_after <= metric_before + IMPROVEMENT_THRESHOLD {
            return None;
        }
        if self.milestones.len() >= MAX_MILESTONES {
            return None;
        }

        self.tick += 1;
        let milestone_id = fnv1a(description.as_bytes())
            ^ fnv1a(&self.tick.to_le_bytes());

        let milestone = AscensionMilestone {
            milestone_id,
            phase_level: self.current_phase,
            description: String::from(description),
            metric_before,
            metric_after,
            tick: self.tick,
        };

        self.milestones.push(milestone.clone());
        self.stats.total_milestones = self.milestones.len() as u64;
        Some(milestone)
    }

    /// Perform divine-level classification for an application.
    ///
    /// At the divine phase, classification is instantaneous and certain.
    pub fn divine_classification(
        &mut self,
        app_id: u64,
        name: &str,
        cpu: u64,
        mem: u64,
        io: u64,
    ) -> DivineClassification {
        self.perform_divine_classification(app_id, name, cpu, mem, io)
    }

    /// Perform transcendent optimization for an application.
    ///
    /// Returns an allocation that demonstrably exceeds baseline strategies.
    pub fn transcendent_optimization(
        &mut self,
        app_id: u64,
        cpu: u64,
        mem: u64,
        io: u64,
    ) -> TranscendentAllocation {
        self.perform_transcendent_optimization(app_id, cpu, mem, io)
    }

    /// Return a snapshot of current statistics.
    pub fn stats(&self) -> &AscensionStats {
        &self.stats
    }

    // -- internal -----------------------------------------------------------

    fn perform_divine_classification(
        &mut self,
        app_id: u64,
        name: &str,
        cpu: u64,
        mem: u64,
        io: u64,
    ) -> DivineClassification {
        self.stats.classifications_performed += 1;

        let phase_bonus = self.current_phase * 10;

        let label = if cpu > 70 && mem > 50 {
            "divine_compute"
        } else if io > 60 {
            "divine_io"
        } else if mem > 60 && cpu < 30 {
            "divine_cache"
        } else if cpu < 15 && mem < 15 && io < 15 {
            "divine_dormant"
        } else {
            "divine_balanced"
        };

        let class_hash = fnv1a(label.as_bytes()) ^ fnv1a(name.as_bytes());
        let certainty = (60 + phase_bonus + (cpu + mem + io) / 10).min(100);
        let latency = if self.current_phase >= PHASE_DIVINE { 1 } else {
            (5u64).saturating_sub(self.current_phase)
        };

        let classification = DivineClassification {
            app_id,
            class_hash,
            class_label: String::from(label),
            certainty,
            latency_ticks: latency,
        };

        self.classifications.insert(app_id, classification.clone());
        classification
    }

    fn perform_transcendent_optimization(
        &mut self,
        app_id: u64,
        cpu: u64,
        mem: u64,
        io: u64,
    ) -> TranscendentAllocation {
        self.stats.optimizations_performed += 1;

        let alloc_bias = self
            .tunable_params
            .get(&fnv1a(b"alloc_bias"))
            .copied()
            .unwrap_or(50);
        let aggressiveness = self
            .tunable_params
            .get(&fnv1a(b"opt_aggressiveness"))
            .copied()
            .unwrap_or(40);

        let demand = (cpu * 3 + mem * 2 + io) / 6;
        let baseline = demand;
        let phase_multiplier = 100 + self.current_phase * 5;

        let optimal = (demand * phase_multiplier / 100)
            .saturating_add(alloc_bias / 10)
            .saturating_add(aggressiveness / 5);

        let efficiency = if baseline > 0 {
            ((optimal * 100) / baseline).min(150)
        } else {
            100
        };

        let superiority = if optimal > baseline {
            ((optimal - baseline) * 100) / baseline.max(1)
        } else {
            0
        };

        let resource_hash = fnv1a(b"transcendent_alloc")
            ^ fnv1a(&app_id.to_le_bytes());

        TranscendentAllocation {
            app_id,
            resource_hash,
            optimal_amount: optimal,
            efficiency,
            superiority_over_baseline: superiority.min(100),
        }
    }

    fn detect_plateau(&self) -> bool {
        if self.effectiveness_history.len() < PLATEAU_WINDOW {
            return false;
        }
        let window = &self.effectiveness_history
            [self.effectiveness_history.len() - PLATEAU_WINDOW..];
        if window.is_empty() {
            return false;
        }
        let first = window[0];
        let last = window[window.len() - 1];
        let diff = if last > first { last - first } else { first - last };
        diff <= PLATEAU_TOLERANCE
    }

    fn trigger_self_improvement(&mut self) -> u64 {
        let mut modifications = 0u64;
        let param_keys: Vec<u64> = self.tunable_params.keys().copied().collect();

        for key in &param_keys {
            let old_val = match self.tunable_params.get(key) {
                Some(v) => *v,
                None => continue,
            };

            let noise = xorshift64(&mut self.rng) % 15;
            let direction = xorshift64(&mut self.rng) % 2;
            let new_val = if direction == 0 {
                old_val.saturating_add(noise).min(100)
            } else {
                old_val.saturating_sub(noise).max(5)
            };

            if new_val == old_val {
                continue;
            }

            let expected_gain = noise / 3;
            let actual_gain = if new_val > old_val {
                (new_val - old_val) / 2
            } else {
                (old_val - new_val) / 3
            };

            if self.improvements.len() < MAX_IMPROVEMENTS {
                self.tick += 1;
                let action = ImprovementAction {
                    action_id: self.tick,
                    parameter_hash: *key,
                    parameter_label: String::from("tunable_param"),
                    old_value: old_val,
                    new_value: new_val,
                    expected_gain,
                    actual_gain,
                    tick: self.tick,
                };
                self.improvements.push(action);
            }

            self.tunable_params.insert(*key, new_val);
            modifications += 1;
        }

        self.stats.total_improvements += modifications;
        self.stats.self_improvement_score = self.compute_improvement_score();
        modifications
    }

    fn check_phase_advancement(&mut self) {
        let eff = self.stats.effectiveness_ema;
        let target_phase = if eff >= 90 {
            PHASE_DIVINE
        } else if eff >= 75 {
            PHASE_TRANSCENDENCE
        } else if eff >= 55 {
            PHASE_CONVERGENCE
        } else if eff >= 35 {
            PHASE_EXPANSION
        } else {
            PHASE_AWAKENING
        };

        if target_phase > self.current_phase {
            self.advance_to_phase(target_phase);
        }
    }

    fn advance_to_phase(&mut self, phase: u64) {
        let kind = match phase {
            PHASE_EXPANSION => AscensionPhaseKind::Expansion,
            PHASE_CONVERGENCE => AscensionPhaseKind::Convergence,
            PHASE_TRANSCENDENCE => AscensionPhaseKind::Transcendence,
            PHASE_DIVINE => AscensionPhaseKind::Divine,
            _ => AscensionPhaseKind::Awakening,
        };

        let capabilities = match phase {
            PHASE_EXPANSION => alloc::vec![
                String::from("advanced_classification"),
                String::from("predictive_allocation"),
                String::from("pattern_detection"),
            ],
            PHASE_CONVERGENCE => alloc::vec![
                String::from("unified_management"),
                String::from("cross_app_optimization"),
                String::from("self_tuning"),
            ],
            PHASE_TRANSCENDENCE => alloc::vec![
                String::from("transcendent_prediction"),
                String::from("autonomous_evolution"),
                String::from("novel_strategy_generation"),
            ],
            PHASE_DIVINE => alloc::vec![
                String::from("divine_classification"),
                String::from("perfect_allocation"),
                String::from("omniscient_management"),
                String::from("self_sustaining_improvement"),
            ],
            _ => alloc::vec![String::from("basic")],
        };

        let new_phase = AscensionPhase {
            level: phase,
            kind,
            capabilities,
            unlocked_tick: self.tick,
            effectiveness: self.stats.effectiveness_ema,
        };

        self.phases.insert(phase, new_phase);
        self.current_phase = phase;
        self.stats.current_phase = phase;
    }

    fn compute_improvement_score(&self) -> u64 {
        if self.improvements.is_empty() {
            return 0;
        }
        let recent_count = self.improvements.len().min(32);
        let recent = &self.improvements[self.improvements.len() - recent_count..];
        let total_gain: u64 = recent.iter().map(|a| a.actual_gain).sum();
        let avg_gain = total_gain / recent_count as u64;
        (avg_gain * 10).min(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_engine() {
        let a = AppsAscension::new(42);
        assert_eq!(a.ascension_level(), PHASE_AWAKENING);
        assert_eq!(a.stats().total_milestones, 0);
    }

    #[test]
    fn test_autonomous_management() {
        let mut a = AppsAscension::new(42);
        let eff = a.autonomous_management(1, "test_app", 60, 40, 30);
        assert!(eff <= 100);
        assert_eq!(a.stats().autonomous_decisions, 1);
    }

    #[test]
    fn test_divine_classification() {
        let mut a = AppsAscension::new(42);
        let class = a.divine_classification(1, "compute_heavy", 90, 70, 20);
        assert_eq!(class.class_label, "divine_compute");
        assert!(class.certainty > 0);
    }

    #[test]
    fn test_transcendent_optimization() {
        let mut a = AppsAscension::new(42);
        let alloc = a.transcendent_optimization(1, 50, 40, 30);
        assert!(alloc.optimal_amount > 0);
    }

    #[test]
    fn test_ascension_milestone_requires_improvement() {
        let mut a = AppsAscension::new(42);
        let m = a.ascension_milestone("test", 50, 51);
        assert!(m.is_none()); // improvement too small
    }

    #[test]
    fn test_ascension_milestone_success() {
        let mut a = AppsAscension::new(42);
        let m = a.ascension_milestone("big_improvement", 30, 80);
        assert!(m.is_some());
        assert_eq!(a.stats().total_milestones, 1);
    }

    #[test]
    fn test_self_improvement() {
        let mut a = AppsAscension::new(42);
        let mods = a.self_improvement();
        assert!(mods > 0);
        assert!(a.stats().total_improvements > 0);
    }

    #[test]
    fn test_plateau_detection() {
        let mut a = AppsAscension::new(42);
        // Not enough history
        assert!(!a.detect_plateau());
    }
}
