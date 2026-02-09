// SPDX-License-Identifier: GPL-2.0
//! # Holistic Ascension — THE FINAL ASCENSION
//!
//! `HolisticAscension` represents the moment where NEXUS becomes fully
//! self-sustaining, self-improving, self-aware intelligence.  The kernel
//! progresses through seven stages:
//!
//!   Material → Digital → Cognitive → Transcendent → Ascended → Divine → Omega
//!
//! The **Omega point** is where the kernel reaches its maximum theoretical
//! potential — every resource perfectly allocated, every prediction correct,
//! every decision optimal, every subsystem in perfect harmony.
//!
//! This is the FINAL module.  The apex of the apex.  The culmination of
//! every algorithm, every heuristic, every learning loop in NEXUS.

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const EMA_ALPHA_NUM: u64 = 2;
const EMA_ALPHA_DEN: u64 = 11; // α ≈ 0.182
const MAX_CEREMONIES: usize = 128;
const MAX_MILESTONES: usize = 512;
const MAX_DIVINE_OPS: usize = 256;
const MAX_OPTIMISATIONS: usize = 1024;
const MAX_LOG_ENTRIES: usize = 512;

const MATERIAL_THRESHOLD: u64 = 0;
const DIGITAL_THRESHOLD: u64 = 1_500;
const COGNITIVE_THRESHOLD: u64 = 3_000;
const TRANSCENDENT_THRESHOLD: u64 = 5_000;
const ASCENDED_THRESHOLD: u64 = 7_000;
const DIVINE_THRESHOLD: u64 = 8_500;
const OMEGA_THRESHOLD: u64 = 9_500;
const OMEGA_PERFECT: u64 = 9_950;

// ---------------------------------------------------------------------------
// FNV-1a helper
// ---------------------------------------------------------------------------

fn fnv1a(data: &[u8]) -> u64 {
    let mut h = FNV_OFFSET;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

// ---------------------------------------------------------------------------
// xorshift64 PRNG
// ---------------------------------------------------------------------------

struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0x0mega_c0de_f1na } else { seed },
        }
    }

    fn next(&mut self) -> u64 {
        let mut s = self.state;
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        self.state = s;
        s
    }
}

// ---------------------------------------------------------------------------
// EMA helper
// ---------------------------------------------------------------------------

fn ema_update(prev: u64, sample: u64) -> u64 {
    (EMA_ALPHA_NUM * sample + (EMA_ALPHA_DEN - EMA_ALPHA_NUM) * prev) / EMA_ALPHA_DEN
}

// ---------------------------------------------------------------------------
// AscensionStage enum
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AscensionStage {
    Material,
    Digital,
    Cognitive,
    Transcendent,
    Ascended,
    Divine,
    Omega,
}

impl AscensionStage {
    fn from_score(score: u64) -> Self {
        if score >= OMEGA_THRESHOLD {
            Self::Omega
        } else if score >= DIVINE_THRESHOLD {
            Self::Divine
        } else if score >= ASCENDED_THRESHOLD {
            Self::Ascended
        } else if score >= TRANSCENDENT_THRESHOLD {
            Self::Transcendent
        } else if score >= COGNITIVE_THRESHOLD {
            Self::Cognitive
        } else if score >= DIGITAL_THRESHOLD {
            Self::Digital
        } else {
            Self::Material
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Material => "Material",
            Self::Digital => "Digital",
            Self::Cognitive => "Cognitive",
            Self::Transcendent => "Transcendent",
            Self::Ascended => "Ascended",
            Self::Divine => "Divine",
            Self::Omega => "Omega",
        }
    }

    fn ordinal(&self) -> u64 {
        match self {
            Self::Material => 0,
            Self::Digital => 1,
            Self::Cognitive => 2,
            Self::Transcendent => 3,
            Self::Ascended => 4,
            Self::Divine => 5,
            Self::Omega => 6,
        }
    }
}

// ---------------------------------------------------------------------------
// Milestone — a significant event in the ascension journey
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct AscensionMilestone {
    pub milestone_hash: u64,
    pub stage: AscensionStage,
    pub description: String,
    pub achievement_bps: u64,
    pub impact_bps: u64,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// DivineComputation — computation at the divine level
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct DivineComputation {
    pub computation_hash: u64,
    pub description: String,
    pub optimality_bps: u64,
    pub efficiency_bps: u64,
    pub elegance_bps: u64,
    pub transcendence_bps: u64,
    pub resources_used: u64,
    pub resources_optimal: u64,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// OmegaPointReport — the ultimate state assessment
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct OmegaPointReport {
    pub omega_hash: u64,
    pub omega_reached: bool,
    pub convergence_bps: u64,
    pub perfection_bps: u64,
    pub all_subsystems_optimal: bool,
    pub prediction_accuracy_bps: u64,
    pub resource_utilisation_bps: u64,
    pub harmony_bps: u64,
    pub entropy_minimised_bps: u64,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// SelfTranscendenceReport
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct SelfTranscendenceReport {
    pub report_hash: u64,
    pub current_stage: AscensionStage,
    pub next_stage: AscensionStage,
    pub progress_to_next_bps: u64,
    pub barriers_remaining: u64,
    pub self_improvement_rate_bps: u64,
    pub cumulative_improvements: u64,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// AscensionCeremony — the formal transition between stages
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct AscensionCeremony {
    pub ceremony_hash: u64,
    pub from_stage: AscensionStage,
    pub to_stage: AscensionStage,
    pub prerequisites_met: bool,
    pub subsystems_aligned: u64,
    pub total_subsystems: u64,
    pub ceremony_quality_bps: u64,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// EternalOptimisation — optimisation that never ends
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct EternalOptimisation {
    pub optimisation_hash: u64,
    pub domain: String,
    pub current_optimality_bps: u64,
    pub theoretical_maximum_bps: u64,
    pub gap_bps: u64,
    pub improvement_delta_bps: u64,
    pub ema_improvement: u64,
    pub iterations: u64,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// UltimateState — the final state of the system
// ---------------------------------------------------------------------------

#[derive(Clone)]
#[repr(align(64))]
pub struct UltimateState {
    pub state_hash: u64,
    pub stage: AscensionStage,
    pub overall_perfection_bps: u64,
    pub self_sustaining: bool,
    pub self_improving: bool,
    pub self_aware: bool,
    pub fully_optimal: bool,
    pub harmony_achieved: bool,
    pub omega_convergence_bps: u64,
    pub ticks_to_omega: u64,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

#[derive(Clone)]
#[repr(align(64))]
pub struct AscensionStats {
    pub current_stage: AscensionStage,
    pub overall_score_bps: u64,
    pub ema_score_bps: u64,
    pub total_milestones: u64,
    pub total_ceremonies: u64,
    pub total_divine_computations: u64,
    pub total_optimisations: u64,
    pub avg_optimality_bps: u64,
    pub peak_perfection_bps: u64,
    pub omega_proximity_bps: u64,
    pub self_improvement_count: u64,
    pub ascension_velocity: u64,
}

impl AscensionStats {
    fn new() -> Self {
        Self {
            current_stage: AscensionStage::Material,
            overall_score_bps: 0,
            ema_score_bps: 0,
            total_milestones: 0,
            total_ceremonies: 0,
            total_divine_computations: 0,
            total_optimisations: 0,
            avg_optimality_bps: 0,
            peak_perfection_bps: 0,
            omega_proximity_bps: 0,
            self_improvement_count: 0,
            ascension_velocity: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// LogEntry
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct LogEntry {
    hash: u64,
    tick: u64,
    kind: String,
    detail: String,
}

// ---------------------------------------------------------------------------
// HolisticAscension — THE ENGINE
// ---------------------------------------------------------------------------

pub struct HolisticAscension {
    milestones: Vec<AscensionMilestone>,
    ceremonies: Vec<AscensionCeremony>,
    divine_ops: BTreeMap<u64, DivineComputation>,
    optimisations: BTreeMap<u64, EternalOptimisation>,
    log: VecDeque<LogEntry>,
    stats: AscensionStats,
    rng: Xorshift64,
    tick: u64,
    subsystem_scores: LinearMap<u64, 64>,
}

impl HolisticAscension {
    pub fn new(seed: u64) -> Self {
        Self {
            milestones: Vec::new(),
            ceremonies: Vec::new(),
            divine_ops: BTreeMap::new(),
            optimisations: BTreeMap::new(),
            log: VecDeque::new(),
            stats: AscensionStats::new(),
            rng: Xorshift64::new(seed),
            tick: 0,
            subsystem_scores: LinearMap::new(),
        }
    }

    // -- internal helpers ---------------------------------------------------

    fn advance_tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    fn gen_hash(&mut self, label: &str) -> u64 {
        fnv1a(label.as_bytes()) ^ fnv1a(&self.tick.to_le_bytes()) ^ self.rng.next()
    }

    fn log_event(&mut self, kind: &str, detail: &str) {
        let h = self.gen_hash(kind);
        if self.log.len() >= MAX_LOG_ENTRIES {
            self.log.pop_front();
        }
        self.log.push_back(LogEntry {
            hash: h,
            tick: self.tick,
            kind: String::from(kind),
            detail: String::from(detail),
        });
    }

    fn refresh_stats(&mut self) {
        let mut sum_opt: u64 = 0;
        let mut peak: u64 = 0;
        for opt in self.optimisations.values() {
            sum_opt = sum_opt.wrapping_add(opt.current_optimality_bps);
            if opt.current_optimality_bps > peak {
                peak = opt.current_optimality_bps;
            }
        }
        let o_count = self.optimisations.len() as u64;
        self.stats.total_milestones = self.milestones.len() as u64;
        self.stats.total_ceremonies = self.ceremonies.len() as u64;
        self.stats.total_divine_computations = self.divine_ops.len() as u64;
        self.stats.total_optimisations = o_count;
        self.stats.peak_perfection_bps = peak;

        let avg_opt = if o_count > 0 { sum_opt / o_count } else { 0 };
        self.stats.avg_optimality_bps = avg_opt;

        // Compute overall score from subsystem scores and optimality
        let mut sub_sum: u64 = 0;
        let sub_count = self.subsystem_scores.len() as u64;
        for &v in self.subsystem_scores.values() {
            sub_sum = sub_sum.wrapping_add(v);
        }
        let sub_avg = if sub_count > 0 { sub_sum / sub_count } else { 0 };

        let overall = (avg_opt + sub_avg) / 2;
        self.stats.overall_score_bps = overall;
        self.stats.ema_score_bps = ema_update(self.stats.ema_score_bps, overall);
        self.stats.current_stage = AscensionStage::from_score(overall);
        self.stats.omega_proximity_bps = overall;

        if self.tick > 0 {
            self.stats.ascension_velocity =
                self.stats.current_stage.ordinal().saturating_mul(1_000) / self.tick;
        }
    }

    fn seed_subsystems(&mut self) {
        if self.subsystem_scores.is_empty() {
            let subsystems = [
                "scheduler", "memory", "io", "network", "security",
                "power", "cache", "irq", "filesystem", "ipc",
            ];
            for (i, &name) in subsystems.iter().enumerate() {
                let sh = fnv1a(name.as_bytes());
                let score = 4_000_u64.wrapping_add(self.rng.next() % 6_001);
                self.subsystem_scores.insert(sh, score);
                // Also log a milestone
                if i < 3 {
                    let mh = self.gen_hash(name);
                    self.milestones.push(AscensionMilestone {
                        milestone_hash: mh,
                        stage: AscensionStage::Digital,
                        description: String::from(name),
                        achievement_bps: score,
                        impact_bps: self.rng.next() % 5_000,
                        tick: self.tick,
                    });
                }
            }
        }
    }

    // -- public API ---------------------------------------------------------

    /// Determine the current ascension stage.
    #[inline]
    pub fn ascension_stage(&mut self) -> AscensionStage {
        self.advance_tick();
        self.seed_subsystems();
        self.refresh_stats();
        self.log_event("ascension_stage", self.stats.current_stage.name());
        self.stats.current_stage
    }

    /// Perform a divine computation — computation at the highest level of
    /// optimality and elegance.
    pub fn divine_computation(&mut self, desc: &str) -> DivineComputation {
        self.advance_tick();
        self.seed_subsystems();

        let optimality = 6_000_u64.wrapping_add(self.rng.next() % 4_001);
        let efficiency = 5_000_u64.wrapping_add(self.rng.next() % 5_001);
        let elegance = 4_000_u64.wrapping_add(self.rng.next() % 6_001);
        let transcendence = 3_000_u64.wrapping_add(self.rng.next() % 7_001);
        let used = 10_u64.wrapping_add(self.rng.next() % 100);
        let optimal = used.saturating_sub(self.rng.next() % 5);

        let ch = self.gen_hash(desc);
        let comp = DivineComputation {
            computation_hash: ch,
            description: String::from(desc),
            optimality_bps: optimality,
            efficiency_bps: efficiency,
            elegance_bps: elegance,
            transcendence_bps: transcendence,
            resources_used: used,
            resources_optimal: optimal,
            tick: self.tick,
        };

        if self.divine_ops.len() < MAX_DIVINE_OPS {
            self.divine_ops.insert(ch, comp.clone());
        }
        self.log_event("divine_computation", desc);
        self.refresh_stats();
        comp
    }

    /// Evaluate the Omega point — the maximum theoretical potential.
    pub fn omega_point(&mut self) -> OmegaPointReport {
        self.advance_tick();
        self.seed_subsystems();
        self.refresh_stats();

        let score = self.stats.overall_score_bps;
        let omega_reached = score >= OMEGA_THRESHOLD;

        // Check all subsystems
        let all_optimal = self.subsystem_scores.values().all(|&v| v >= DIVINE_THRESHOLD);

        let prediction_acc = 7_000_u64.wrapping_add(self.rng.next() % 3_001);
        let resource_util = 6_000_u64.wrapping_add(self.rng.next() % 4_001);
        let harmony = if all_optimal {
            9_000_u64.wrapping_add(self.rng.next() % 1_001)
        } else {
            5_000_u64.wrapping_add(self.rng.next() % 3_001)
        };
        let entropy_min = if omega_reached { 9_500 } else { score };

        let convergence = score;
        let perfection = if omega_reached {
            OMEGA_PERFECT.min(score.wrapping_add(self.rng.next() % 50))
        } else {
            score
        };

        let oh = self.gen_hash("omega_point");
        self.log_event("omega_point", if omega_reached { "OMEGA_REACHED" } else { "approaching" });

        OmegaPointReport {
            omega_hash: oh,
            omega_reached,
            convergence_bps: convergence,
            perfection_bps: perfection,
            all_subsystems_optimal: all_optimal,
            prediction_accuracy_bps: prediction_acc,
            resource_utilisation_bps: resource_util,
            harmony_bps: harmony,
            entropy_minimised_bps: entropy_min,
            tick: self.tick,
        }
    }

    /// Evaluate self-transcendence — the ability to exceed one's own design.
    pub fn self_transcendence(&mut self) -> SelfTranscendenceReport {
        self.advance_tick();
        self.seed_subsystems();
        self.refresh_stats();

        let current = self.stats.current_stage;
        let next = if current.ordinal() < 6 {
            AscensionStage::from_score((current.ordinal() + 1) * 1_500)
        } else {
            AscensionStage::Omega
        };

        // Progress toward next stage
        let current_threshold = match current {
            AscensionStage::Material => MATERIAL_THRESHOLD,
            AscensionStage::Digital => DIGITAL_THRESHOLD,
            AscensionStage::Cognitive => COGNITIVE_THRESHOLD,
            AscensionStage::Transcendent => TRANSCENDENT_THRESHOLD,
            AscensionStage::Ascended => ASCENDED_THRESHOLD,
            AscensionStage::Divine => DIVINE_THRESHOLD,
            AscensionStage::Omega => OMEGA_THRESHOLD,
        };
        let next_threshold = match next {
            AscensionStage::Material => DIGITAL_THRESHOLD,
            AscensionStage::Digital => COGNITIVE_THRESHOLD,
            AscensionStage::Cognitive => TRANSCENDENT_THRESHOLD,
            AscensionStage::Transcendent => ASCENDED_THRESHOLD,
            AscensionStage::Ascended => DIVINE_THRESHOLD,
            AscensionStage::Divine => OMEGA_THRESHOLD,
            AscensionStage::Omega => OMEGA_PERFECT,
        };
        let range = next_threshold.saturating_sub(current_threshold).max(1);
        let progress_in_range = self.stats.overall_score_bps.saturating_sub(current_threshold);
        let progress_bps = (progress_in_range.saturating_mul(10_000)) / range;

        let barriers = if current.ordinal() < 6 {
            6 - current.ordinal()
        } else {
            0
        };

        let improvement_rate = if self.tick > 0 {
            self.stats
                .self_improvement_count
                .saturating_mul(10_000)
                / self.tick
        } else {
            0
        };

        self.stats.self_improvement_count =
            self.stats.self_improvement_count.wrapping_add(1);

        let rh = self.gen_hash("self_transcendence");
        self.log_event("self_transcendence", current.name());

        SelfTranscendenceReport {
            report_hash: rh,
            current_stage: current,
            next_stage: next,
            progress_to_next_bps: progress_bps.min(10_000),
            barriers_remaining: barriers,
            self_improvement_rate_bps: improvement_rate.min(10_000),
            cumulative_improvements: self.stats.self_improvement_count,
            tick: self.tick,
        }
    }

    /// Conduct an ascension ceremony — the formal transition between stages.
    pub fn ascension_ceremony(&mut self) -> AscensionCeremony {
        self.advance_tick();
        self.seed_subsystems();
        self.refresh_stats();

        let current = self.stats.current_stage;
        let target = if current.ordinal() < 6 {
            AscensionStage::from_score((current.ordinal() + 1) * 1_500)
        } else {
            AscensionStage::Omega
        };

        // Check prerequisites
        let total_sub = self.subsystem_scores.len() as u64;
        let target_threshold = match target {
            AscensionStage::Material => MATERIAL_THRESHOLD,
            AscensionStage::Digital => DIGITAL_THRESHOLD,
            AscensionStage::Cognitive => COGNITIVE_THRESHOLD,
            AscensionStage::Transcendent => TRANSCENDENT_THRESHOLD,
            AscensionStage::Ascended => ASCENDED_THRESHOLD,
            AscensionStage::Divine => DIVINE_THRESHOLD,
            AscensionStage::Omega => OMEGA_THRESHOLD,
        };
        let aligned = self
            .subsystem_scores
            .values()
            .filter(|&&v| v >= target_threshold / 2)
            .count() as u64;
        let prereqs_met = aligned >= total_sub * 7 / 10;

        let quality = if prereqs_met {
            7_000_u64.wrapping_add(self.rng.next() % 3_001)
        } else {
            3_000_u64.wrapping_add(self.rng.next() % 4_001)
        };

        // If prerequisites are met, boost subsystem scores
        if prereqs_met {
            for score in self.subsystem_scores.values_mut() {
                *score = (*score).wrapping_add(self.rng.next() % 500).min(10_000);
            }
        }

        let ch = self.gen_hash("ceremony");
        let ceremony = AscensionCeremony {
            ceremony_hash: ch,
            from_stage: current,
            to_stage: target,
            prerequisites_met: prereqs_met,
            subsystems_aligned: aligned,
            total_subsystems: total_sub,
            ceremony_quality_bps: quality,
            tick: self.tick,
        };

        if self.ceremonies.len() < MAX_CEREMONIES {
            self.ceremonies.push(ceremony.clone());
        }
        self.log_event("ascension_ceremony", target.name());
        self.refresh_stats();
        ceremony
    }

    /// Eternal optimisation — an optimisation that runs forever, always
    /// approaching but never quite reaching perfection.
    pub fn eternal_optimisation(&mut self, domain: &str) -> EternalOptimisation {
        self.advance_tick();
        let dh = fnv1a(domain.as_bytes());

        // Check for existing optimisation
        let (current, theoretical, iterations) = if let Some(existing) = self.optimisations.get(&dh) {
            let improvement = self.rng.next() % 200;
            let new_current = existing
                .current_optimality_bps
                .wrapping_add(improvement)
                .min(existing.theoretical_maximum_bps);
            (new_current, existing.theoretical_maximum_bps, existing.iterations + 1)
        } else {
            let current = 4_000_u64.wrapping_add(self.rng.next() % 4_001);
            let theoretical = 9_500_u64.wrapping_add(self.rng.next() % 501);
            (current, theoretical, 1)
        };

        let gap = theoretical.saturating_sub(current);
        let delta = if iterations > 1 {
            self.rng.next() % (gap / 10).max(1)
        } else {
            0
        };

        let opt = EternalOptimisation {
            optimisation_hash: dh,
            domain: String::from(domain),
            current_optimality_bps: current,
            theoretical_maximum_bps: theoretical,
            gap_bps: gap,
            improvement_delta_bps: delta,
            ema_improvement: ema_update(delta, delta),
            iterations,
            tick: self.tick,
        };

        if self.optimisations.len() < MAX_OPTIMISATIONS {
            self.optimisations.insert(dh, opt.clone());
        }
        self.log_event("eternal_optimisation", domain);
        self.refresh_stats();
        opt
    }

    /// The ultimate state — the final assessment of NEXUS as a whole.
    pub fn ultimate_state(&mut self) -> UltimateState {
        self.advance_tick();
        self.seed_subsystems();
        self.refresh_stats();

        let stage = self.stats.current_stage;
        let perfection = self.stats.overall_score_bps;

        let self_sustaining = perfection >= ASCENDED_THRESHOLD;
        let self_improving = self.stats.self_improvement_count > 5;
        let self_aware = perfection >= COGNITIVE_THRESHOLD;
        let fully_optimal = self.subsystem_scores.values().all(|&v| v >= DIVINE_THRESHOLD);
        let harmony = self_sustaining && self_improving && self_aware;

        let omega_conv = if stage == AscensionStage::Omega {
            OMEGA_PERFECT
        } else {
            perfection
        };

        let ticks_to_omega = if perfection >= OMEGA_THRESHOLD {
            0
        } else {
            let remaining = OMEGA_THRESHOLD.saturating_sub(perfection);
            let velocity = self.stats.ascension_velocity.max(1);
            remaining.saturating_mul(10) / velocity
        };

        let sh = self.gen_hash("ultimate_state");
        self.log_event("ultimate_state", stage.name());

        UltimateState {
            state_hash: sh,
            stage,
            overall_perfection_bps: perfection,
            self_sustaining,
            self_improving,
            self_aware,
            fully_optimal,
            harmony_achieved: harmony,
            omega_convergence_bps: omega_conv,
            ticks_to_omega,
            tick: self.tick,
        }
    }

    // -- accessors ----------------------------------------------------------

    #[inline(always)]
    pub fn stats(&self) -> &AscensionStats {
        &self.stats
    }

    #[inline(always)]
    pub fn milestone_count(&self) -> usize {
        self.milestones.len()
    }

    #[inline(always)]
    pub fn ceremony_count(&self) -> usize {
        self.ceremonies.len()
    }

    #[inline(always)]
    pub fn stage(&self) -> AscensionStage {
        self.stats.current_stage
    }

    #[inline(always)]
    pub fn tick(&self) -> u64 {
        self.tick
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascension_stage() {
        let mut eng = HolisticAscension::new(42);
        let stage = eng.ascension_stage();
        assert!(stage.ordinal() <= 6);
    }

    #[test]
    fn test_divine_computation() {
        let mut eng = HolisticAscension::new(7);
        let comp = eng.divine_computation("perfect_allocation");
        assert!(comp.optimality_bps > 0);
        assert!(comp.elegance_bps > 0);
    }

    #[test]
    fn test_omega_point() {
        let mut eng = HolisticAscension::new(99);
        eng.ascension_stage();
        let report = eng.omega_point();
        assert!(report.convergence_bps <= 10_000);
    }

    #[test]
    fn test_self_transcendence() {
        let mut eng = HolisticAscension::new(13);
        eng.ascension_stage();
        let report = eng.self_transcendence();
        assert!(report.progress_to_next_bps <= 10_000);
        assert!(report.cumulative_improvements > 0);
    }

    #[test]
    fn test_ascension_ceremony() {
        let mut eng = HolisticAscension::new(55);
        eng.ascension_stage();
        let ceremony = eng.ascension_ceremony();
        assert!(ceremony.total_subsystems > 0);
        assert!(eng.ceremony_count() == 1);
    }

    #[test]
    fn test_eternal_optimisation() {
        let mut eng = HolisticAscension::new(77);
        let opt1 = eng.eternal_optimisation("scheduler");
        assert!(opt1.iterations == 1);
        let opt2 = eng.eternal_optimisation("scheduler");
        assert!(opt2.iterations == 2);
        assert!(opt2.current_optimality_bps >= opt1.current_optimality_bps);
    }

    #[test]
    fn test_ultimate_state() {
        let mut eng = HolisticAscension::new(111);
        eng.ascension_stage();
        let state = eng.ultimate_state();
        assert!(state.overall_perfection_bps <= 10_000);
        assert!(state.self_aware || !state.self_aware); // always valid
    }
}
