// SPDX-License-Identifier: GPL-2.0
//! # Bridge Evolution — Self-Modification of Bridge Strategies
//!
//! The bridge evolves its own optimisation algorithms through
//! genetic-programming-like mutation and selection. Each `GeneticStrategy`
//! carries a genome (parameter vector), a fitness score, and a generation
//! counter. Populations are managed with tournament selection, single-point
//! crossover, and Gaussian-ish mutation driven by xorshift64.
//!
//! FNV-1a hashing fingerprints genomes for duplicate detection; EMA
//! tracks average population fitness across generations.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_POPULATION: usize = 256;
const MAX_GENOME_LEN: usize = 64;
const MAX_GENERATIONS: u64 = 10_000;
const TOURNAMENT_SIZE: usize = 4;
const MUTATION_RATE: f32 = 0.10;
const MUTATION_STRENGTH: f32 = 0.25;
const CROSSOVER_RATE: f32 = 0.70;
const ELITISM_FRACTION: f32 = 0.10;
const EMA_ALPHA: f32 = 0.10;
const FITNESS_STAGNATION_WINDOW: usize = 20;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

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
// EVOLUTION TYPES
// ============================================================================

/// Strategy status in the population.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StrategyStatus {
    Active,
    Elite,
    Offspring,
    Retired,
    Mutant,
}

/// A single genetic strategy with a parameter genome.
#[derive(Debug, Clone)]
pub struct GeneticStrategy {
    pub strategy_id: u64,
    pub name: String,
    pub genome: Vec<f32>,
    pub fitness: f32,
    pub generation: u64,
    pub status: StrategyStatus,
    pub parent_ids: Vec<u64>,
    pub genome_hash: u64,
    pub created_tick: u64,
}

/// Report for a single generation.
#[derive(Debug, Clone)]
pub struct GenerationReport {
    pub generation: u64,
    pub population_size: usize,
    pub best_fitness: f32,
    pub avg_fitness: f32,
    pub worst_fitness: f32,
    pub diversity: f32,
    pub elites_kept: usize,
    pub mutations_applied: u64,
    pub crossovers_applied: u64,
}

/// Long-term evolution trend.
#[derive(Debug, Clone)]
pub struct EvolutionTrend {
    pub generations_elapsed: u64,
    pub fitness_improvement: f32,
    pub stagnation_count: usize,
    pub is_stagnating: bool,
    pub best_ever_fitness: f32,
}

// ============================================================================
// EVOLUTION STATS
// ============================================================================

/// Aggregate statistics for the evolution engine.
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct EvolutionStats {
    pub current_generation: u64,
    pub population_size: u64,
    pub best_fitness: f32,
    pub avg_fitness: f32,
    pub total_mutations: u64,
    pub total_crossovers: u64,
    pub total_selections: u64,
    pub diversity: f32,
    pub fitness_ema: f32,
}

// ============================================================================
// BRIDGE EVOLUTION ENGINE
// ============================================================================

/// Genetic-programming-inspired evolution of bridge strategies.
/// Maintains a population, performs selection, crossover, mutation,
/// and tracks generational fitness.
#[derive(Debug)]
#[repr(align(64))]
pub struct BridgeEvolution {
    population: BTreeMap<u64, GeneticStrategy>,
    generation: u64,
    total_mutations: u64,
    total_crossovers: u64,
    total_selections: u64,
    fitness_history: VecDeque<f32>,
    best_ever_fitness: f32,
    best_ever_id: u64,
    tick: u64,
    rng_state: u64,
    fitness_ema: f32,
}

impl BridgeEvolution {
    /// Create a new evolution engine.
    pub fn new(seed: u64) -> Self {
        Self {
            population: BTreeMap::new(),
            generation: 0,
            total_mutations: 0,
            total_crossovers: 0,
            total_selections: 0,
            fitness_history: VecDeque::new(),
            best_ever_fitness: 0.0,
            best_ever_id: 0,
            tick: 0,
            rng_state: seed ^ 0xEV01_0000_CAFE,
            fitness_ema: 0.0,
        }
    }

    /// Seed the population with a random strategy.
    pub fn seed_strategy(&mut self, name: &str, genome_len: usize) -> u64 {
        self.tick += 1;
        let mut genome = Vec::with_capacity(genome_len.min(MAX_GENOME_LEN));
        for _ in 0..genome_len.min(MAX_GENOME_LEN) {
            let val = ((xorshift64(&mut self.rng_state) % 2000) as f32 - 1000.0) / 1000.0;
            genome.push(val);
        }

        let genome_hash = self.hash_genome(&genome);
        let sid = fnv1a_hash(name.as_bytes()) ^ self.tick;

        let strat = GeneticStrategy {
            strategy_id: sid,
            name: String::from(name),
            genome,
            fitness: 0.0,
            generation: self.generation,
            status: StrategyStatus::Active,
            parent_ids: Vec::new(),
            genome_hash,
            created_tick: self.tick,
        };

        if self.population.len() < MAX_POPULATION {
            self.population.insert(sid, strat);
        }
        sid
    }

    /// Evaluate fitness for a strategy using a provided scoring closure
    /// approximated by a deterministic scoring heuristic.
    #[inline]
    pub fn fitness_evaluation(&mut self, strategy_id: u64) -> f32 {
        if let Some(strat) = self.population.get_mut(&strategy_id) {
            // Deterministic fitness: sum of genome values normalised
            let sum: f32 = strat.genome.iter().map(|g| (*g).abs()).sum();
            let len = strat.genome.len().max(1) as f32;
            let raw = 1.0 - (sum / len).min(1.0);
            // Add slight noise to differentiate
            let noise = ((xorshift64(&mut self.rng_state) % 100) as f32) / 10000.0;
            let fitness = (raw + noise).max(0.0).min(1.0);
            strat.fitness = fitness;

            if fitness > self.best_ever_fitness {
                self.best_ever_fitness = fitness;
                self.best_ever_id = strategy_id;
            }

            self.fitness_ema = EMA_ALPHA * fitness + (1.0 - EMA_ALPHA) * self.fitness_ema;
            fitness
        } else {
            0.0
        }
    }

    /// Tournament selection: pick the best of TOURNAMENT_SIZE random individuals.
    pub fn selection(&mut self) -> Option<u64> {
        let keys: Vec<u64> = self.population.keys().copied().collect();
        if keys.is_empty() {
            return None;
        }

        self.total_selections += 1;
        let mut best_id = keys[0];
        let mut best_fit = f32::MIN;

        for _ in 0..TOURNAMENT_SIZE.min(keys.len()) {
            let idx = (xorshift64(&mut self.rng_state) as usize) % keys.len();
            let cid = keys[idx];
            let fit = self.population.get(&cid).map_or(0.0, |s| s.fitness);
            if fit > best_fit {
                best_fit = fit;
                best_id = cid;
            }
        }

        Some(best_id)
    }

    /// Single-point crossover between two parent genomes.
    pub fn crossover(&mut self, parent_a: u64, parent_b: u64) -> Option<u64> {
        let roll = (xorshift64(&mut self.rng_state) % 100) as f32 / 100.0;
        if roll > CROSSOVER_RATE {
            return None;
        }

        let ga = self.population.get(&parent_a)?.genome.clone();
        let gb = self.population.get(&parent_b)?.genome.clone();

        let min_len = ga.len().min(gb.len());
        if min_len < 2 {
            return None;
        }

        let crossover_point = (xorshift64(&mut self.rng_state) as usize) % (min_len - 1) + 1;

        let mut child_genome = Vec::with_capacity(min_len);
        for i in 0..min_len {
            if i < crossover_point {
                child_genome.push(ga[i]);
            } else {
                child_genome.push(gb[i]);
            }
        }

        self.tick += 1;
        self.total_crossovers += 1;
        let genome_hash = self.hash_genome(&child_genome);
        let cid = genome_hash ^ self.tick;

        let child = GeneticStrategy {
            strategy_id: cid,
            name: String::from("offspring"),
            genome: child_genome,
            fitness: 0.0,
            generation: self.generation + 1,
            status: StrategyStatus::Offspring,
            parent_ids: alloc::vec![parent_a, parent_b],
            genome_hash,
            created_tick: self.tick,
        };

        if self.population.len() < MAX_POPULATION {
            self.population.insert(cid, child);
        }

        Some(cid)
    }

    /// Mutate a strategy's genome in place.
    pub fn mutate(&mut self, strategy_id: u64) -> bool {
        if let Some(strat) = self.population.get_mut(&strategy_id) {
            let mut mutated = false;
            for gene in strat.genome.iter_mut() {
                let roll = (xorshift64(&mut self.rng_state) % 100) as f32 / 100.0;
                if roll < MUTATION_RATE {
                    let delta = ((xorshift64(&mut self.rng_state) % 2000) as f32 - 1000.0)
                        / 1000.0
                        * MUTATION_STRENGTH;
                    *gene += delta;
                    *gene = gene.max(-5.0).min(5.0);
                    mutated = true;
                }
            }
            if mutated {
                strat.genome_hash = self.hash_genome_inline(&strat.genome);
                strat.status = StrategyStatus::Mutant;
                self.total_mutations += 1;
            }
            mutated
        } else {
            false
        }
    }

    /// Advance to the next generation: evaluate all, select, breed, mutate.
    pub fn generation_advance(&mut self) -> GenerationReport {
        self.tick += 1;
        self.generation += 1;

        // Evaluate all
        let ids: Vec<u64> = self.population.keys().copied().collect();
        for id in &ids {
            self.fitness_evaluation(*id);
        }

        // Sort by fitness descending
        let mut ranked: Vec<(u64, f32)> = self
            .population
            .iter()
            .map(|(&id, s)| (id, s.fitness))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));

        let pop_size = ranked.len();
        let elite_count = ((pop_size as f32) * ELITISM_FRACTION).ceil() as usize;
        let elite_count = elite_count.max(1).min(pop_size);

        // Mark elites
        for i in 0..elite_count {
            if let Some(strat) = self.population.get_mut(&ranked[i].0) {
                strat.status = StrategyStatus::Elite;
            }
        }

        let mut mutations_applied = 0u64;
        let mut crossovers_applied = 0u64;

        // Breed new offspring to fill population
        let offspring_needed = (MAX_POPULATION - elite_count).min(pop_size);
        for _ in 0..offspring_needed {
            let pa = self.selection();
            let pb = self.selection();
            if let (Some(a), Some(b)) = (pa, pb) {
                if let Some(child_id) = self.crossover(a, b) {
                    crossovers_applied += 1;
                    if self.mutate(child_id) {
                        mutations_applied += 1;
                    }
                }
            }
        }

        // Retire non-elite old generation
        let retirement_gen = self.generation.saturating_sub(2);
        for (_, strat) in self.population.iter_mut() {
            if strat.generation < retirement_gen && strat.status != StrategyStatus::Elite {
                strat.status = StrategyStatus::Retired;
            }
        }

        // Remove retired
        self.population.retain(|_, s| s.status != StrategyStatus::Retired);

        // Compute generation stats
        let best_fit = ranked.first().map_or(0.0, |r| r.1);
        let worst_fit = ranked.last().map_or(0.0, |r| r.1);
        let avg_fit = if ranked.is_empty() {
            0.0
        } else {
            ranked.iter().map(|r| r.1).sum::<f32>() / ranked.len() as f32
        };

        self.fitness_history.push_back(avg_fit);
        if self.fitness_history.len() > FITNESS_STAGNATION_WINDOW * 2 {
            self.fitness_history.pop_front();
        }

        let diversity = self.compute_diversity();

        GenerationReport {
            generation: self.generation,
            population_size: self.population.len(),
            best_fitness: best_fit,
            avg_fitness: avg_fit,
            worst_fitness: worst_fit,
            diversity,
            elites_kept: elite_count,
            mutations_applied,
            crossovers_applied,
        }
    }

    /// Evolve a strategy: shorthand for selection → crossover → mutation.
    #[inline]
    pub fn evolve_strategy(&mut self) -> Option<u64> {
        let pa = self.selection()?;
        let pb = self.selection()?;
        let child = self.crossover(pa, pb)?;
        self.mutate(child);
        self.fitness_evaluation(child);
        Some(child)
    }

    /// Evolution trend analysis.
    pub fn evolution_trend(&self) -> EvolutionTrend {
        let mut stagnation_count = 0usize;
        if self.fitness_history.len() >= FITNESS_STAGNATION_WINDOW {
            let recent = &self.fitness_history[self.fitness_history.len() - FITNESS_STAGNATION_WINDOW..];
            let first = recent[0];
            let last = recent[recent.len() - 1];
            let improvement = last - first;
            if improvement.abs() < 0.01 {
                stagnation_count = FITNESS_STAGNATION_WINDOW;
            }
        }

        let fitness_improvement = if self.fitness_history.len() >= 2 {
            let first = self.fitness_history[0];
            let last = self.fitness_history[self.fitness_history.len() - 1];
            last - first
        } else {
            0.0
        };

        EvolutionTrend {
            generations_elapsed: self.generation,
            fitness_improvement,
            stagnation_count,
            is_stagnating: stagnation_count >= FITNESS_STAGNATION_WINDOW,
            best_ever_fitness: self.best_ever_fitness,
        }
    }

    /// Genome diversity: average pairwise distance in the population.
    fn compute_diversity(&self) -> f32 {
        let strats: Vec<&GeneticStrategy> = self.population.values().collect();
        if strats.len() < 2 {
            return 0.0;
        }
        let mut total_dist = 0.0_f32;
        let mut pairs = 0u64;

        // Sample to avoid O(n^2) for large populations
        let sample_limit = 50usize;
        let n = strats.len().min(sample_limit);
        for i in 0..n {
            for j in (i + 1)..n {
                let d = genome_distance(&strats[i].genome, &strats[j].genome);
                total_dist += d;
                pairs += 1;
            }
        }

        if pairs > 0 { total_dist / pairs as f32 } else { 0.0 }
    }

    /// Get a strategy by ID.
    #[inline(always)]
    pub fn get_strategy(&self, id: u64) -> Option<&GeneticStrategy> {
        self.population.get(&id)
    }

    /// Population size.
    #[inline(always)]
    pub fn population_size(&self) -> usize {
        self.population.len()
    }

    /// Current generation.
    #[inline(always)]
    pub fn current_generation(&self) -> u64 {
        self.generation
    }

    /// Aggregate statistics.
    pub fn stats(&self) -> EvolutionStats {
        let avg = if self.population.is_empty() {
            0.0
        } else {
            let sum: f32 = self.population.values().map(|s| s.fitness).sum();
            sum / self.population.len() as f32
        };

        EvolutionStats {
            current_generation: self.generation,
            population_size: self.population.len() as u64,
            best_fitness: self.best_ever_fitness,
            avg_fitness: avg,
            total_mutations: self.total_mutations,
            total_crossovers: self.total_crossovers,
            total_selections: self.total_selections,
            diversity: self.compute_diversity(),
            fitness_ema: self.fitness_ema,
        }
    }

    // --- private helpers ---

    fn hash_genome(&self, genome: &[f32]) -> u64 {
        let mut data = Vec::with_capacity(genome.len() * 4);
        for g in genome {
            data.extend_from_slice(&g.to_le_bytes());
        }
        fnv1a_hash(&data)
    }

    fn hash_genome_inline(&self, genome: &[f32]) -> u64 {
        self.hash_genome(genome)
    }
}

// ============================================================================
// FREE FUNCTIONS
// ============================================================================

fn genome_distance(a: &[f32], b: &[f32]) -> f32 {
    let max_len = a.len().max(b.len());
    let mut sum_sq = 0.0_f32;
    for i in 0..max_len {
        let va = a.get(i).copied().unwrap_or(0.0);
        let vb = b.get(i).copied().unwrap_or(0.0);
        let diff = va - vb;
        sum_sq += diff * diff;
    }
    sqrt_approx(sum_sq)
}

fn sqrt_approx(x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    let mut guess = x;
    for _ in 0..8 {
        guess = 0.5 * (guess + x / guess);
    }
    guess
}
