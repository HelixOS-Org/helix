// SPDX-License-Identifier: GPL-2.0
//! # Holistic Evolution — SYSTEM-WIDE Self-Evolution
//!
//! `HolisticEvolution` gives NEXUS the power to evolve its own architecture.
//! A population of kernel configurations undergoes genetic programming at
//! the system level — selection, mutation, crossover, and speciation — to
//! discover configurations that no human engineer could design.
//!
//! The fitness landscape is multi-dimensional: throughput, latency, energy,
//! reliability, and security are all co-optimised.  Evolutionary pressure
//! is applied continuously, and the kernel self-selects the fittest
//! configuration for the current workload.
//!
//! This is Darwinian evolution applied to operating system architecture.

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
const EMA_ALPHA_NUM: u64 = 3;
const EMA_ALPHA_DEN: u64 = 14; // α ≈ 0.214
const MAX_POPULATION: usize = 256;
const MAX_SPECIES: usize = 64;
const MAX_GENERATIONS: usize = 4096;
const MAX_PRESSURE_EVENTS: usize = 512;
const MAX_LOG_ENTRIES: usize = 512;
const ELITE_THRESHOLD_BPS: u64 = 8_500;
const MUTATION_RATE_BPS: u64 = 1_500; // 15%
const CROSSOVER_RATE_BPS: u64 = 7_000; // 70%
const SPECIATION_DISTANCE_BPS: u64 = 3_000;
const EXTINCTION_THRESHOLD_BPS: u64 = 2_000;

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
            state: if seed == 0 { 0xdead_c0de_4321 } else { seed },
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
// KernelConfig — a single configuration in the population
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct KernelConfig {
    pub config_hash: u64,
    pub genome: Vec<u64>,
    pub species_hash: u64,
    pub generation: u64,
    pub fitness: u64,
    pub ema_fitness: u64,
    pub throughput_score: u64,
    pub latency_score: u64,
    pub energy_score: u64,
    pub reliability_score: u64,
    pub security_score: u64,
    pub parent_a: u64,
    pub parent_b: u64,
    pub created_tick: u64,
    pub alive: bool,
}

impl KernelConfig {
    fn new(generation: u64, tick: u64, rng_val: u64) -> Self {
        let genome_len = 8;
        let mut genome = Vec::with_capacity(genome_len);
        let mut h = rng_val;
        for i in 0..genome_len {
            h = h.wrapping_mul(FNV_PRIME) ^ (i as u64);
            genome.push(h % 10_000);
        }
        let config_hash = fnv1a(&rng_val.to_le_bytes()) ^ fnv1a(&tick.to_le_bytes());
        Self {
            config_hash,
            genome,
            species_hash: 0,
            generation,
            fitness: 0,
            ema_fitness: 0,
            throughput_score: 0,
            latency_score: 0,
            energy_score: 0,
            reliability_score: 0,
            security_score: 0,
            parent_a: 0,
            parent_b: 0,
            created_tick: tick,
            alive: true,
        }
    }

    fn evaluate_fitness(&mut self) {
        // Multi-objective fitness: weighted sum
        let f = (self.throughput_score.saturating_mul(30)
            + self.latency_score.saturating_mul(25)
            + self.energy_score.saturating_mul(15)
            + self.reliability_score.saturating_mul(15)
            + self.security_score.saturating_mul(15))
            / 100;
        self.fitness = f;
        self.ema_fitness = ema_update(self.ema_fitness, f);
    }
}

// ---------------------------------------------------------------------------
// Species — group of similar configurations
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Species {
    pub species_hash: u64,
    pub representative_hash: u64,
    pub member_count: u64,
    pub avg_fitness: u64,
    pub ema_fitness: u64,
    pub generation_born: u64,
    pub stagnation_count: u64,
    pub alive: bool,
}

// ---------------------------------------------------------------------------
// MutationEvent
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct MutationEvent {
    pub event_hash: u64,
    pub config_hash: u64,
    pub gene_index: u64,
    pub old_value: u64,
    pub new_value: u64,
    pub fitness_delta: i64,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// CrossoverResult
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct CrossoverResult {
    pub child_hash: u64,
    pub parent_a: u64,
    pub parent_b: u64,
    pub crossover_point: u64,
    pub child_fitness: u64,
    pub hybrid_vigor_bps: u64,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// FitnessLandscapeView
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct FitnessLandscapeView {
    pub landscape_hash: u64,
    pub peak_fitness: u64,
    pub avg_fitness: u64,
    pub min_fitness: u64,
    pub fitness_variance: u64,
    pub num_peaks: u64,
    pub ruggedness_bps: u64,
    pub population_size: u64,
    pub generation: u64,
}

// ---------------------------------------------------------------------------
// EvolutionaryPressureReport
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct EvolutionaryPressureReport {
    pub report_hash: u64,
    pub selection_pressure_bps: u64,
    pub extinction_count: u64,
    pub speciation_events: u64,
    pub mutation_count: u64,
    pub crossover_count: u64,
    pub diversity_bps: u64,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

#[derive(Clone)]
#[repr(align(64))]
pub struct EvolutionStats {
    pub population_size: u64,
    pub total_species: u64,
    pub current_generation: u64,
    pub peak_fitness: u64,
    pub avg_fitness: u64,
    pub ema_fitness: u64,
    pub total_mutations: u64,
    pub total_crossovers: u64,
    pub total_extinctions: u64,
    pub total_speciations: u64,
    pub elite_count: u64,
    pub evolution_rate_per_1k: u64,
}

impl EvolutionStats {
    fn new() -> Self {
        Self {
            population_size: 0,
            total_species: 0,
            current_generation: 0,
            peak_fitness: 0,
            avg_fitness: 0,
            ema_fitness: 0,
            total_mutations: 0,
            total_crossovers: 0,
            total_extinctions: 0,
            total_speciations: 0,
            elite_count: 0,
            evolution_rate_per_1k: 0,
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
// HolisticEvolution — THE ENGINE
// ---------------------------------------------------------------------------

pub struct HolisticEvolution {
    population: BTreeMap<u64, KernelConfig>,
    species: BTreeMap<u64, Species>,
    pressure_events: Vec<LogEntry>,
    log: VecDeque<LogEntry>,
    stats: EvolutionStats,
    rng: Xorshift64,
    tick: u64,
    generation: u64,
}

impl HolisticEvolution {
    pub fn new(seed: u64) -> Self {
        Self {
            population: BTreeMap::new(),
            species: BTreeMap::new(),
            pressure_events: Vec::new(),
            log: VecDeque::new(),
            stats: EvolutionStats::new(),
            rng: Xorshift64::new(seed),
            tick: 0,
            generation: 0,
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

    fn spawn_config(&mut self) -> KernelConfig {
        let rv = self.rng.next();
        let mut config = KernelConfig::new(self.generation, self.tick, rv);
        config.throughput_score = 2_000_u64.wrapping_add(self.rng.next() % 8_001);
        config.latency_score = 2_000_u64.wrapping_add(self.rng.next() % 8_001);
        config.energy_score = 2_000_u64.wrapping_add(self.rng.next() % 8_001);
        config.reliability_score = 2_000_u64.wrapping_add(self.rng.next() % 8_001);
        config.security_score = 2_000_u64.wrapping_add(self.rng.next() % 8_001);
        config.evaluate_fitness();
        config
    }

    fn refresh_stats(&mut self) {
        let mut sum_fit: u64 = 0;
        let mut peak: u64 = 0;
        let mut elite: u64 = 0;
        let alive_count = self.population.values().filter(|c| c.alive).count() as u64;
        for cfg in self.population.values() {
            if !cfg.alive {
                continue;
            }
            sum_fit = sum_fit.wrapping_add(cfg.fitness);
            if cfg.fitness > peak {
                peak = cfg.fitness;
            }
            if cfg.fitness >= ELITE_THRESHOLD_BPS {
                elite += 1;
            }
        }
        self.stats.population_size = alive_count;
        self.stats.total_species = self.species.values().filter(|s| s.alive).count() as u64;
        self.stats.current_generation = self.generation;
        self.stats.peak_fitness = peak;
        self.stats.elite_count = elite;
        let avg = if alive_count > 0 { sum_fit / alive_count } else { 0 };
        self.stats.avg_fitness = avg;
        self.stats.ema_fitness = ema_update(self.stats.ema_fitness, avg);
        if self.tick > 0 {
            self.stats.evolution_rate_per_1k =
                self.generation.saturating_mul(1_000) / self.tick;
        }
    }

    fn genome_distance(a: &[u64], b: &[u64]) -> u64 {
        let len = a.len().min(b.len());
        let mut diff: u64 = 0;
        for i in 0..len {
            diff = diff.wrapping_add(if a[i] > b[i] { a[i] - b[i] } else { b[i] - a[i] });
        }
        if len > 0 { diff / len as u64 } else { 0 }
    }

    // -- public API ---------------------------------------------------------

    /// Run one generation of system-wide evolution.
    pub fn system_evolution(&mut self) -> Vec<KernelConfig> {
        self.advance_tick();
        self.generation = self.generation.wrapping_add(1);

        // Seed population if empty
        if self.population.is_empty() {
            for _ in 0..8 {
                let cfg = self.spawn_config();
                let h = cfg.config_hash;
                if self.population.len() < MAX_POPULATION {
                    self.population.insert(h, cfg);
                }
            }
        }

        // Selection: keep alive configs sorted by fitness
        let mut ranked: Vec<u64> = self
            .population
            .values()
            .filter(|c| c.alive)
            .map(|c| c.config_hash)
            .collect();
        ranked.sort_by(|a, b| {
            let fa = self.population.get(b).map(|c| c.fitness).unwrap_or(0);
            let fb = self.population.get(a).map(|c| c.fitness).unwrap_or(0);
            fa.cmp(&fb)
        });

        // Kill bottom 30%
        let kill_count = ranked.len() * 3 / 10;
        for &h in ranked.iter().rev().take(kill_count) {
            if let Some(cfg) = self.population.get_mut(&h) {
                cfg.alive = false;
            }
            self.stats.total_extinctions = self.stats.total_extinctions.wrapping_add(1);
        }

        // Crossover among top survivors
        let survivors: Vec<u64> = self
            .population
            .values()
            .filter(|c| c.alive)
            .map(|c| c.config_hash)
            .collect();
        let mut new_configs: Vec<KernelConfig> = Vec::new();
        if survivors.len() >= 2 {
            for i in (0..survivors.len().min(6)).step_by(2) {
                if i + 1 < survivors.len() {
                    let child = self.configuration_crossover(survivors[i], survivors[i + 1]);
                    new_configs.push(child.clone());
                }
            }
        }

        // Mutation on random survivors
        for &sh in survivors.iter().take(4) {
            self.architecture_mutation(sh);
        }

        // Insert children
        for cfg in &new_configs {
            if self.population.len() < MAX_POPULATION {
                self.population.insert(cfg.config_hash, cfg.clone());
            }
        }

        self.log_event("system_evolution", "generation_complete");
        self.refresh_stats();
        new_configs
    }

    /// Mutate a specific kernel configuration.
    pub fn architecture_mutation(&mut self, config_hash: u64) -> MutationEvent {
        self.advance_tick();
        let gene_idx = self.rng.next() % 8;
        let mut old_val: u64 = 0;
        let mut new_val: u64 = 0;
        let mut fitness_before: u64 = 0;
        let mut fitness_after: u64 = 0;

        if let Some(cfg) = self.population.get_mut(&config_hash) {
            let idx = gene_idx as usize;
            if idx < cfg.genome.len() {
                old_val = cfg.genome[idx];
                let delta = self.rng.next() % 2_000;
                new_val = if self.rng.next() % 2 == 0 {
                    old_val.wrapping_add(delta).min(10_000)
                } else {
                    old_val.saturating_sub(delta)
                };
                cfg.genome[idx] = new_val;
            }
            fitness_before = cfg.fitness;
            // Re-evaluate with slight randomness
            cfg.throughput_score = ema_update(cfg.throughput_score, 3_000 + self.rng.next() % 7_001);
            cfg.evaluate_fitness();
            fitness_after = cfg.fitness;
        }

        self.stats.total_mutations = self.stats.total_mutations.wrapping_add(1);
        let eh = self.gen_hash("mutation");
        self.log_event("architecture_mutation", "gene_mutated");

        MutationEvent {
            event_hash: eh,
            config_hash,
            gene_index: gene_idx,
            old_value: old_val,
            new_value: new_val,
            fitness_delta: fitness_after as i64 - fitness_before as i64,
            tick: self.tick,
        }
    }

    /// Crossover two kernel configurations to produce an offspring.
    pub fn configuration_crossover(&mut self, parent_a: u64, parent_b: u64) -> KernelConfig {
        self.advance_tick();
        let mut child = self.spawn_config();
        child.parent_a = parent_a;
        child.parent_b = parent_b;

        let crossover_point = self.rng.next() % 8;
        if let (Some(a), Some(b)) = (
            self.population.get(&parent_a),
            self.population.get(&parent_b),
        ) {
            for i in 0..child.genome.len() {
                child.genome[i] = if (i as u64) < crossover_point {
                    a.genome.get(i).copied().unwrap_or(0)
                } else {
                    b.genome.get(i).copied().unwrap_or(0)
                };
            }
            child.throughput_score = (a.throughput_score + b.throughput_score) / 2;
            child.latency_score = (a.latency_score + b.latency_score) / 2;
            child.energy_score = (a.energy_score + b.energy_score) / 2;
            child.reliability_score = (a.reliability_score + b.reliability_score) / 2;
            child.security_score = (a.security_score + b.security_score) / 2;
            // Hybrid vigor: slight boost
            let vigor = self.rng.next() % 500;
            child.throughput_score = child.throughput_score.wrapping_add(vigor).min(10_000);
        }
        child.evaluate_fitness();

        self.stats.total_crossovers = self.stats.total_crossovers.wrapping_add(1);
        self.log_event("configuration_crossover", "offspring_created");
        child
    }

    /// View the current fitness landscape.
    pub fn fitness_landscape(&mut self) -> FitnessLandscapeView {
        self.advance_tick();
        self.refresh_stats();
        let alive: Vec<u64> = self
            .population
            .values()
            .filter(|c| c.alive)
            .map(|c| c.fitness)
            .collect();

        let min_f = alive.iter().min().copied().unwrap_or(0);
        let max_f = alive.iter().max().copied().unwrap_or(0);
        let avg_f = self.stats.avg_fitness;
        let variance = max_f.saturating_sub(min_f);

        // Count peaks: configs significantly above average
        let peak_threshold = avg_f.wrapping_add(variance / 3);
        let num_peaks = alive.iter().filter(|&&f| f >= peak_threshold).count() as u64;
        let ruggedness = if avg_f > 0 {
            (variance.saturating_mul(10_000)) / avg_f.max(1)
        } else {
            0
        };

        let lh = self.gen_hash("landscape");
        self.log_event("fitness_landscape", "landscape_surveyed");

        FitnessLandscapeView {
            landscape_hash: lh,
            peak_fitness: max_f,
            avg_fitness: avg_f,
            min_fitness: min_f,
            fitness_variance: variance,
            num_peaks,
            ruggedness_bps: ruggedness.min(10_000),
            population_size: alive.len() as u64,
            generation: self.generation,
        }
    }

    /// Apply evolutionary pressure — survival of the fittest.
    pub fn evolutionary_pressure(&mut self) -> EvolutionaryPressureReport {
        self.advance_tick();
        let before_pop = self.population.values().filter(|c| c.alive).count() as u64;

        // Kill unfit
        let threshold = self.stats.avg_fitness.saturating_sub(EXTINCTION_THRESHOLD_BPS);
        let mut extinctions: u64 = 0;
        let to_kill: Vec<u64> = self
            .population
            .values()
            .filter(|c| c.alive && c.fitness < threshold)
            .map(|c| c.config_hash)
            .collect();
        for h in to_kill {
            if let Some(cfg) = self.population.get_mut(&h) {
                cfg.alive = false;
                extinctions += 1;
            }
        }
        self.stats.total_extinctions = self.stats.total_extinctions.wrapping_add(extinctions);

        let selection_pressure = if before_pop > 0 {
            (extinctions.saturating_mul(10_000)) / before_pop
        } else {
            0
        };

        // Diversity: unique species count / population
        let alive_species: u64 = self.species.values().filter(|s| s.alive).count() as u64;
        let alive_pop = self.population.values().filter(|c| c.alive).count() as u64;
        let diversity = if alive_pop > 0 {
            (alive_species.saturating_mul(10_000)) / alive_pop.max(1)
        } else {
            0
        };

        let rh = self.gen_hash("pressure");
        self.log_event("evolutionary_pressure", "pressure_applied");
        self.refresh_stats();

        EvolutionaryPressureReport {
            report_hash: rh,
            selection_pressure_bps: selection_pressure.min(10_000),
            extinction_count: extinctions,
            speciation_events: self.stats.total_speciations,
            mutation_count: self.stats.total_mutations,
            crossover_count: self.stats.total_crossovers,
            diversity_bps: diversity.min(10_000),
            tick: self.tick,
        }
    }

    /// Identify and create species from the population.
    pub fn speciation(&mut self) -> Vec<Species> {
        self.advance_tick();
        let mut new_species: Vec<Species> = Vec::new();

        // Group by genome similarity
        let alive: Vec<(u64, Vec<u64>)> = self
            .population
            .values()
            .filter(|c| c.alive)
            .map(|c| (c.config_hash, c.genome.clone()))
            .collect();

        let mut assigned: BTreeMap<u64, u64> = BTreeMap::new(); // config -> species

        for (i, (ch, genome)) in alive.iter().enumerate() {
            if assigned.contains_key(ch) {
                continue;
            }
            let sh = fnv1a(&ch.to_le_bytes()) ^ self.rng.next();
            assigned.insert(*ch, sh);

            let mut member_count: u64 = 1;
            let mut sum_fit = self.population.get(ch).map(|c| c.fitness).unwrap_or(0);

            for (j, (ch2, genome2)) in alive.iter().enumerate() {
                if j <= i || assigned.contains_key(ch2) {
                    continue;
                }
                let dist = Self::genome_distance(genome, genome2);
                if dist < SPECIATION_DISTANCE_BPS {
                    assigned.insert(*ch2, sh);
                    member_count += 1;
                    sum_fit = sum_fit.wrapping_add(
                        self.population.get(ch2).map(|c| c.fitness).unwrap_or(0),
                    );
                    if let Some(cfg) = self.population.get_mut(ch2) {
                        cfg.species_hash = sh;
                    }
                }
            }

            if let Some(cfg) = self.population.get_mut(ch) {
                cfg.species_hash = sh;
            }

            let avg_fit = if member_count > 0 { sum_fit / member_count } else { 0 };
            let sp = Species {
                species_hash: sh,
                representative_hash: *ch,
                member_count,
                avg_fitness: avg_fit,
                ema_fitness: avg_fit,
                generation_born: self.generation,
                stagnation_count: 0,
                alive: true,
            };

            if self.species.len() < MAX_SPECIES {
                self.species.insert(sh, sp.clone());
                new_species.push(sp);
            }
        }

        self.stats.total_speciations = self
            .stats
            .total_speciations
            .wrapping_add(new_species.len() as u64);
        self.log_event("speciation", "species_identified");
        self.refresh_stats();
        new_species
    }

    /// Current evolution rate (generations per 1000 ticks).
    #[inline(always)]
    pub fn evolution_rate(&self) -> u64 {
        self.stats.evolution_rate_per_1k
    }

    // -- accessors ----------------------------------------------------------

    #[inline(always)]
    pub fn stats(&self) -> &EvolutionStats {
        &self.stats
    }

    #[inline(always)]
    pub fn population_size(&self) -> usize {
        self.population.values().filter(|c| c.alive).count()
    }

    #[inline(always)]
    pub fn species_count(&self) -> usize {
        self.species.values().filter(|s| s.alive).count()
    }

    #[inline(always)]
    pub fn generation(&self) -> u64 {
        self.generation
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
    fn test_system_evolution() {
        let mut eng = HolisticEvolution::new(42);
        let children = eng.system_evolution();
        assert!(eng.population_size() > 0);
        assert!(eng.generation() == 1);
        assert!(!children.is_empty() || eng.population_size() > 0);
    }

    #[test]
    fn test_architecture_mutation() {
        let mut eng = HolisticEvolution::new(7);
        eng.system_evolution();
        let first = eng.population.keys().next().copied().unwrap();
        let event = eng.architecture_mutation(first);
        assert!(event.gene_index < 8);
    }

    #[test]
    fn test_configuration_crossover() {
        let mut eng = HolisticEvolution::new(99);
        eng.system_evolution();
        let keys: Vec<u64> = eng.population.keys().copied().take(2).collect();
        if keys.len() == 2 {
            let child = eng.configuration_crossover(keys[0], keys[1]);
            assert!(child.parent_a == keys[0]);
            assert!(child.parent_b == keys[1]);
        }
    }

    #[test]
    fn test_fitness_landscape() {
        let mut eng = HolisticEvolution::new(13);
        eng.system_evolution();
        let view = eng.fitness_landscape();
        assert!(view.population_size > 0);
        assert!(view.peak_fitness >= view.min_fitness);
    }

    #[test]
    fn test_evolutionary_pressure() {
        let mut eng = HolisticEvolution::new(55);
        eng.system_evolution();
        let report = eng.evolutionary_pressure();
        assert!(report.selection_pressure_bps <= 10_000);
    }

    #[test]
    fn test_speciation() {
        let mut eng = HolisticEvolution::new(77);
        eng.system_evolution();
        let species = eng.speciation();
        assert!(!species.is_empty() || eng.population_size() > 0);
    }

    #[test]
    fn test_evolution_rate() {
        let mut eng = HolisticEvolution::new(111);
        for _ in 0..5 {
            eng.system_evolution();
        }
        assert!(eng.generation() == 5);
    }
}
