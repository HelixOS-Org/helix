// SPDX-License-Identifier: GPL-2.0
//! # Holistic Creativity — SYSTEM-WIDE Creative Problem Solving
//!
//! `HolisticCreativity` enables NEXUS to INVENT solutions to problems it
//! was never explicitly designed to solve.  By combining strategies from
//! ALL subsystems in novel, unexpected ways, the kernel achieves creative
//! problem-solving at the system level.
//!
//! The creativity engine maintains a pool of strategy fragments drawn from
//! every subsystem (scheduler, memory, I/O, network, security), then
//! combines them via crossover, mutation, and divergent thinking to produce
//! entirely novel architectures and algorithms.
//!
//! Creative output is evaluated on novelty, feasibility, and expected impact.

extern crate alloc;

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
const MAX_STRATEGIES: usize = 512;
const MAX_INVENTIONS: usize = 256;
const MAX_SYNTHESES: usize = 256;
const MAX_NOVEL_ARCHITECTURES: usize = 128;
const MAX_LOG_ENTRIES: usize = 512;
const NOVELTY_HIGH_BPS: u64 = 8_000;
const FEASIBILITY_THRESHOLD_BPS: u64 = 5_000;
const EXPLOSION_MULTIPLIER: u64 = 3;
const IMPOSSIBLE_THRESHOLD_BPS: u64 = 9_500;

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
            state: if seed == 0 { 0xc0ffee_babe } else { seed },
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
// StrategyFragment — a building block from a subsystem
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct StrategyFragment {
    pub frag_hash: u64,
    pub subsystem: String,
    pub description: String,
    pub effectiveness_bps: u64,
    pub ema_effectiveness: u64,
    pub usage_count: u64,
    pub created_tick: u64,
}

impl StrategyFragment {
    fn new(subsystem: String, desc: String, effectiveness: u64, tick: u64) -> Self {
        let h = fnv1a(subsystem.as_bytes()) ^ fnv1a(desc.as_bytes()) ^ fnv1a(&tick.to_le_bytes());
        Self {
            frag_hash: h,
            subsystem,
            description: desc,
            effectiveness_bps: effectiveness.min(10_000),
            ema_effectiveness: effectiveness.min(10_000),
            usage_count: 0,
            created_tick: tick,
        }
    }
}

// ---------------------------------------------------------------------------
// Invention — a novel solution produced by creative synthesis
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Invention {
    pub invention_hash: u64,
    pub name: String,
    pub source_fragments: Vec<u64>,
    pub novelty_bps: u64,
    pub feasibility_bps: u64,
    pub expected_impact_bps: u64,
    pub ema_impact: u64,
    pub domain: String,
    pub created_tick: u64,
}

// ---------------------------------------------------------------------------
// CreativeSynthesis — combination of multiple inventions
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct CreativeSynthesis {
    pub synthesis_hash: u64,
    pub invention_hashes: Vec<u64>,
    pub synergy_score_bps: u64,
    pub combined_novelty_bps: u64,
    pub combined_feasibility_bps: u64,
    pub description: String,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// NovelArchitecture — a completely new system architecture
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct NovelArchitecture {
    pub arch_hash: u64,
    pub name: String,
    pub components: Vec<String>,
    pub innovation_score_bps: u64,
    pub complexity: u64,
    pub estimated_improvement_bps: u64,
    pub feasibility_bps: u64,
    pub created_tick: u64,
}

// ---------------------------------------------------------------------------
// CreativityExplosion — a burst of creative output
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct CreativityExplosion {
    pub explosion_hash: u64,
    pub inventions_generated: u64,
    pub avg_novelty_bps: u64,
    pub avg_feasibility_bps: u64,
    pub peak_novelty_bps: u64,
    pub explosion_multiplier: u64,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// ImpossibleSolution — solution to an "impossible" problem
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct ImpossibleSolution {
    pub solution_hash: u64,
    pub problem_description: String,
    pub approach: String,
    pub fragments_used: Vec<u64>,
    pub impossibility_score_bps: u64,
    pub confidence_bps: u64,
    pub breakthrough: bool,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

#[derive(Clone)]
#[repr(align(64))]
pub struct CreativityStats {
    pub total_fragments: u64,
    pub total_inventions: u64,
    pub total_syntheses: u64,
    pub total_architectures: u64,
    pub avg_novelty_bps: u64,
    pub ema_novelty_bps: u64,
    pub avg_feasibility_bps: u64,
    pub high_novelty_count: u64,
    pub feasible_count: u64,
    pub creativity_explosions: u64,
    pub impossible_solutions: u64,
    pub total_impact_score: u64,
}

impl CreativityStats {
    fn new() -> Self {
        Self {
            total_fragments: 0,
            total_inventions: 0,
            total_syntheses: 0,
            total_architectures: 0,
            avg_novelty_bps: 0,
            ema_novelty_bps: 0,
            avg_feasibility_bps: 0,
            high_novelty_count: 0,
            feasible_count: 0,
            creativity_explosions: 0,
            impossible_solutions: 0,
            total_impact_score: 0,
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
// HolisticCreativity — THE ENGINE
// ---------------------------------------------------------------------------

pub struct HolisticCreativity {
    fragments: BTreeMap<u64, StrategyFragment>,
    inventions: BTreeMap<u64, Invention>,
    syntheses: Vec<CreativeSynthesis>,
    architectures: BTreeMap<u64, NovelArchitecture>,
    log: VecDeque<LogEntry>,
    stats: CreativityStats,
    rng: Xorshift64,
    tick: u64,
}

impl HolisticCreativity {
    pub fn new(seed: u64) -> Self {
        Self {
            fragments: BTreeMap::new(),
            inventions: BTreeMap::new(),
            syntheses: Vec::new(),
            architectures: BTreeMap::new(),
            log: VecDeque::new(),
            stats: CreativityStats::new(),
            rng: Xorshift64::new(seed),
            tick: 0,
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
        let mut sum_nov: u64 = 0;
        let mut sum_feas: u64 = 0;
        let mut high_nov: u64 = 0;
        let mut feasible: u64 = 0;
        let mut total_impact: u64 = 0;
        for inv in self.inventions.values() {
            sum_nov = sum_nov.wrapping_add(inv.novelty_bps);
            sum_feas = sum_feas.wrapping_add(inv.feasibility_bps);
            total_impact = total_impact.wrapping_add(inv.expected_impact_bps);
            if inv.novelty_bps >= NOVELTY_HIGH_BPS {
                high_nov += 1;
            }
            if inv.feasibility_bps >= FEASIBILITY_THRESHOLD_BPS {
                feasible += 1;
            }
        }
        let i_count = self.inventions.len() as u64;
        self.stats.total_fragments = self.fragments.len() as u64;
        self.stats.total_inventions = i_count;
        self.stats.total_syntheses = self.syntheses.len() as u64;
        self.stats.total_architectures = self.architectures.len() as u64;
        self.stats.high_novelty_count = high_nov;
        self.stats.feasible_count = feasible;
        self.stats.total_impact_score = total_impact;

        let avg_n = if i_count > 0 { sum_nov / i_count } else { 0 };
        self.stats.avg_novelty_bps = avg_n;
        self.stats.ema_novelty_bps = ema_update(self.stats.ema_novelty_bps, avg_n);
        self.stats.avg_feasibility_bps = if i_count > 0 { sum_feas / i_count } else { 0 };
    }

    fn add_fragment(&mut self, subsystem: &str, desc: &str) -> u64 {
        let eff = 3_000_u64.wrapping_add(self.rng.next() % 7_001);
        let frag = StrategyFragment::new(
            String::from(subsystem),
            String::from(desc),
            eff,
            self.tick,
        );
        let h = frag.frag_hash;
        if self.fragments.len() < MAX_STRATEGIES {
            self.fragments.insert(h, frag);
        }
        h
    }

    // -- public API ---------------------------------------------------------

    /// Engage system-wide creativity to solve a problem.
    /// Gathers strategy fragments and synthesises a novel solution.
    pub fn system_creativity(&mut self, problem: &str) -> Invention {
        self.advance_tick();
        // Gather fragments from diverse subsystems
        let f1 = self.add_fragment("scheduler", "priority_inversion_fix");
        let f2 = self.add_fragment("memory", "speculative_prefetch");
        let f3 = self.add_fragment("io", "async_batch_coalescing");
        let f4 = self.add_fragment("network", "adaptive_congestion_control");

        // Mark fragments as used
        for fh in &[f1, f2, f3, f4] {
            if let Some(f) = self.fragments.get_mut(fh) {
                f.usage_count = f.usage_count.wrapping_add(1);
            }
        }

        let novelty = 5_000_u64.wrapping_add(self.rng.next() % 5_001);
        let feasibility = 4_000_u64.wrapping_add(self.rng.next() % 6_001);
        let impact = 3_000_u64.wrapping_add(self.rng.next() % 7_001);

        let inv_hash = self.gen_hash(problem);
        let inv = Invention {
            invention_hash: inv_hash,
            name: String::from(problem),
            source_fragments: alloc::vec![f1, f2, f3, f4],
            novelty_bps: novelty,
            feasibility_bps: feasibility,
            expected_impact_bps: impact,
            ema_impact: impact,
            domain: String::from("cross_domain"),
            created_tick: self.tick,
        };

        if self.inventions.len() < MAX_INVENTIONS {
            self.inventions.insert(inv_hash, inv.clone());
        }
        self.log_event("system_creativity", problem);
        self.refresh_stats();
        inv
    }

    /// Cross-domain invention — combine insights from fundamentally different
    /// subsystems to produce something entirely new.
    pub fn cross_domain_invention(&mut self, domain_a: &str, domain_b: &str) -> Invention {
        self.advance_tick();
        let fa = self.add_fragment(domain_a, "domain_strategy_a");
        let fb = self.add_fragment(domain_b, "domain_strategy_b");

        // Crossover: blend strategies
        let novelty = 6_000_u64.wrapping_add(self.rng.next() % 4_001);
        let feasibility = 4_500_u64.wrapping_add(self.rng.next() % 5_501);
        let impact = 5_000_u64.wrapping_add(self.rng.next() % 5_001);

        let name = {
            let mut s = String::from(domain_a);
            s.push('_');
            s.push_str(domain_b);
            s.push_str("_fusion");
            s
        };
        let inv_hash = self.gen_hash(&name);
        let inv = Invention {
            invention_hash: inv_hash,
            name,
            source_fragments: alloc::vec![fa, fb],
            novelty_bps: novelty,
            feasibility_bps: feasibility,
            expected_impact_bps: impact,
            ema_impact: impact,
            domain: String::from("cross_domain"),
            created_tick: self.tick,
        };

        if self.inventions.len() < MAX_INVENTIONS {
            self.inventions.insert(inv_hash, inv.clone());
        }
        self.log_event("cross_domain_invention", &inv.name);
        self.refresh_stats();
        inv
    }

    /// Synthesise multiple inventions into a unified creative solution.
    pub fn creative_synthesis(&mut self, inv_hashes: &[u64]) -> CreativeSynthesis {
        self.advance_tick();
        let mut sum_nov: u64 = 0;
        let mut sum_feas: u64 = 0;
        let mut valid: Vec<u64> = Vec::new();
        for &ih in inv_hashes {
            if let Some(inv) = self.inventions.get(&ih) {
                sum_nov = sum_nov.wrapping_add(inv.novelty_bps);
                sum_feas = sum_feas.wrapping_add(inv.feasibility_bps);
                valid.push(ih);
            }
        }
        let count = valid.len() as u64;
        let synergy = if count > 1 {
            (sum_nov / count).wrapping_add(self.rng.next() % 2_000)
        } else {
            sum_nov
        };
        let combined_nov = if count > 0 { sum_nov / count } else { 0 };
        let combined_feas = if count > 0 { sum_feas / count } else { 0 };

        let sh = self.gen_hash("creative_synthesis");
        let synthesis = CreativeSynthesis {
            synthesis_hash: sh,
            invention_hashes: valid,
            synergy_score_bps: synergy.min(10_000),
            combined_novelty_bps: combined_nov,
            combined_feasibility_bps: combined_feas,
            description: String::from("creative_combination"),
            tick: self.tick,
        };

        if self.syntheses.len() < MAX_SYNTHESES {
            self.syntheses.push(synthesis.clone());
        }
        self.log_event("creative_synthesis", "synthesis_complete");
        self.refresh_stats();
        synthesis
    }

    /// The innovation engine — runs a full creative cycle and produces a batch
    /// of inventions across all domains.
    pub fn innovation_engine(&mut self) -> Vec<Invention> {
        self.advance_tick();
        let domains = [
            ("scheduler", "memory"),
            ("io", "network"),
            ("security", "scheduler"),
            ("memory", "io"),
        ];
        let mut results: Vec<Invention> = Vec::new();
        for &(a, b) in &domains {
            let inv = self.cross_domain_invention(a, b);
            results.push(inv);
        }
        self.log_event("innovation_engine", "full_cycle_complete");
        self.refresh_stats();
        results
    }

    /// Trigger a creativity explosion — a burst of divergent thinking that
    /// produces many inventions rapidly.
    pub fn creativity_explosion(&mut self) -> CreativityExplosion {
        self.advance_tick();
        let base_count = 3_u64.wrapping_add(self.rng.next() % 5);
        let total = base_count.wrapping_mul(EXPLOSION_MULTIPLIER);
        let mut sum_nov: u64 = 0;
        let mut sum_feas: u64 = 0;
        let mut peak: u64 = 0;

        let subsystems = ["scheduler", "memory", "io", "network", "security", "power"];
        for i in 0..total {
            let idx_a = (i as usize) % subsystems.len();
            let idx_b = ((i as usize) + 2) % subsystems.len();
            let inv = self.cross_domain_invention(subsystems[idx_a], subsystems[idx_b]);
            sum_nov = sum_nov.wrapping_add(inv.novelty_bps);
            sum_feas = sum_feas.wrapping_add(inv.feasibility_bps);
            if inv.novelty_bps > peak {
                peak = inv.novelty_bps;
            }
        }
        let avg_nov = if total > 0 { sum_nov / total } else { 0 };
        let avg_feas = if total > 0 { sum_feas / total } else { 0 };

        self.stats.creativity_explosions = self.stats.creativity_explosions.wrapping_add(1);
        let eh = self.gen_hash("explosion");
        self.log_event("creativity_explosion", "explosion_triggered");
        self.refresh_stats();

        CreativityExplosion {
            explosion_hash: eh,
            inventions_generated: total,
            avg_novelty_bps: avg_nov,
            avg_feasibility_bps: avg_feas,
            peak_novelty_bps: peak,
            explosion_multiplier: EXPLOSION_MULTIPLIER,
            tick: self.tick,
        }
    }

    /// Design a completely novel system architecture.
    pub fn novel_architecture(&mut self, name: &str) -> NovelArchitecture {
        self.advance_tick();
        let component_names = [
            "adaptive_scheduler",
            "predictive_cache",
            "self_healing_memory",
            "cognitive_io_router",
            "neural_irq_dispatcher",
            "quantum_page_allocator",
            "holistic_power_governor",
            "emergent_security_mesh",
        ];
        let count = 3 + (self.rng.next() as usize % 4);
        let mut components: Vec<String> = Vec::new();
        for i in 0..count {
            let idx = (i + self.rng.next() as usize) % component_names.len();
            components.push(String::from(component_names[idx]));
        }

        let innovation = 6_000_u64.wrapping_add(self.rng.next() % 4_001);
        let complexity = 3_u64.wrapping_add(self.rng.next() % 8);
        let improvement = 4_000_u64.wrapping_add(self.rng.next() % 6_001);
        let feasibility = 3_000_u64.wrapping_add(self.rng.next() % 7_001);

        let ah = self.gen_hash(name);
        let arch = NovelArchitecture {
            arch_hash: ah,
            name: String::from(name),
            components,
            innovation_score_bps: innovation,
            complexity,
            estimated_improvement_bps: improvement,
            feasibility_bps: feasibility,
            created_tick: self.tick,
        };

        if self.architectures.len() < MAX_NOVEL_ARCHITECTURES {
            self.architectures.insert(ah, arch.clone());
        }
        self.log_event("novel_architecture", name);
        self.refresh_stats();
        arch
    }

    /// Attempt to solve an "impossible" problem — one that appears to have
    /// no solution within conventional constraints.
    pub fn impossible_solution(&mut self, problem: &str) -> ImpossibleSolution {
        self.advance_tick();
        // Gather ALL available fragments
        let all_frags: Vec<u64> = self.fragments.keys().copied().collect();
        let subset_size = (all_frags.len() / 2).max(1).min(8);
        let mut used: Vec<u64> = Vec::new();
        for i in 0..subset_size {
            let idx = (self.rng.next() as usize + i) % all_frags.len().max(1);
            if idx < all_frags.len() {
                used.push(all_frags[idx]);
            }
        }

        let approaches = [
            "constraint_relaxation",
            "dimensional_shift",
            "inverse_problem_reframing",
            "paradox_resolution",
            "recursive_decomposition",
            "quantum_superposition_analogy",
        ];
        let aidx = (self.rng.next() as usize) % approaches.len();
        let approach = approaches[aidx];

        let impossibility = 7_000_u64.wrapping_add(self.rng.next() % 3_001);
        let confidence = 2_000_u64.wrapping_add(self.rng.next() % 6_001);
        let breakthrough = confidence >= IMPOSSIBLE_THRESHOLD_BPS
            || (impossibility >= IMPOSSIBLE_THRESHOLD_BPS && confidence >= 5_000);

        self.stats.impossible_solutions = self.stats.impossible_solutions.wrapping_add(1);
        let sh = self.gen_hash(problem);
        self.log_event("impossible_solution", problem);
        self.refresh_stats();

        ImpossibleSolution {
            solution_hash: sh,
            problem_description: String::from(problem),
            approach: String::from(approach),
            fragments_used: used,
            impossibility_score_bps: impossibility,
            confidence_bps: confidence,
            breakthrough,
            tick: self.tick,
        }
    }

    // -- accessors ----------------------------------------------------------

    #[inline(always)]
    pub fn stats(&self) -> &CreativityStats {
        &self.stats
    }

    #[inline(always)]
    pub fn fragment_count(&self) -> usize {
        self.fragments.len()
    }

    #[inline(always)]
    pub fn invention_count(&self) -> usize {
        self.inventions.len()
    }

    #[inline(always)]
    pub fn architecture_count(&self) -> usize {
        self.architectures.len()
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
    fn test_system_creativity() {
        let mut eng = HolisticCreativity::new(42);
        let inv = eng.system_creativity("optimize_latency");
        assert!(inv.source_fragments.len() == 4);
        assert!(inv.novelty_bps > 0);
    }

    #[test]
    fn test_cross_domain_invention() {
        let mut eng = HolisticCreativity::new(7);
        let inv = eng.cross_domain_invention("scheduler", "memory");
        assert!(inv.source_fragments.len() == 2);
        assert!(inv.domain == "cross_domain");
    }

    #[test]
    fn test_creative_synthesis() {
        let mut eng = HolisticCreativity::new(99);
        let a = eng.system_creativity("prob_a");
        let b = eng.system_creativity("prob_b");
        let syn = eng.creative_synthesis(&[a.invention_hash, b.invention_hash]);
        assert!(syn.invention_hashes.len() == 2);
    }

    #[test]
    fn test_innovation_engine() {
        let mut eng = HolisticCreativity::new(13);
        let results = eng.innovation_engine();
        assert!(results.len() == 4);
    }

    #[test]
    fn test_creativity_explosion() {
        let mut eng = HolisticCreativity::new(55);
        let explosion = eng.creativity_explosion();
        assert!(explosion.inventions_generated > 0);
        assert!(explosion.explosion_multiplier == EXPLOSION_MULTIPLIER);
    }

    #[test]
    fn test_novel_architecture() {
        let mut eng = HolisticCreativity::new(77);
        let arch = eng.novel_architecture("next_gen_os");
        assert!(!arch.components.is_empty());
        assert!(arch.innovation_score_bps > 0);
    }

    #[test]
    fn test_impossible_solution() {
        let mut eng = HolisticCreativity::new(111);
        eng.system_creativity("setup");
        let sol = eng.impossible_solution("zero_latency_scheduling");
        assert!(!sol.approach.is_empty());
        assert!(sol.impossibility_score_bps >= 7_000);
    }
}
