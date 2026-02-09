// SPDX-License-Identifier: GPL-2.0
//! # Apps Evolution — Self-Evolving App Management Strategies
//!
//! Applies genetic programming to the classification and allocation
//! algorithms themselves. Populations of algorithm genomes are maintained,
//! subjected to fitness evaluation, selection pressure, crossover, and
//! mutation. Each generation produces increasingly effective classifiers
//! and allocators that adapt to the real workload mix in the system.
//!
//! The evolutionary process tracks lineage, fitness trajectories, and
//! environmental pressure so that the kernel can introspect on how and
//! why its own strategies evolved.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x00000100000001B3;
const EMA_ALPHA_NUM: u64 = 2;
const EMA_ALPHA_DEN: u64 = 9;
const MAX_POPULATION: usize = 256;
const GENE_COUNT: usize = 8;
const MUTATION_SIGMA: u64 = 12;
const CROSSOVER_RATE: u64 = 70; // percent
const ELITE_FRACTION: usize = 4; // top 1/4 survive
const MIN_FITNESS_IMPROVEMENT: u64 = 2;
const PRESSURE_WINDOW: usize = 16;

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

/// A genome encoding a classifier or allocator algorithm.
#[derive(Clone, Debug)]
pub struct AlgorithmGenome {
    pub genome_id: u64,
    pub genes: Vec<u64>,
    pub fitness: u64,
    pub generation: u64,
    pub lineage: VecDeque<u64>,
    pub evaluations: u64,
    pub fitness_history: Vec<u64>,
}

/// Represents a single fitness test case.
#[derive(Clone, Debug)]
pub struct FitnessCase {
    pub case_id: u64,
    pub input_features: Vec<u64>,
    pub expected_output: u64,
    pub weight: u64,
}

/// Environmental pressure record — snapshots of selection difficulty.
#[derive(Clone, Debug)]
pub struct PressureRecord {
    pub generation: u64,
    pub avg_fitness: u64,
    pub best_fitness: u64,
    pub worst_fitness: u64,
    pub diversity: u64,
    pub survival_rate: u64,
}

/// Result of a crossover operation.
#[derive(Clone, Debug)]
pub struct CrossoverResult {
    pub parent_a: u64,
    pub parent_b: u64,
    pub offspring_id: u64,
    pub crossover_point: usize,
    pub predicted_fitness: u64,
}

/// Aggregated statistics for the evolution engine.
#[derive(Clone, Debug, Default)]
#[repr(align(64))]
pub struct EvolutionStats {
    pub generation: u64,
    pub population_size: u64,
    pub best_fitness: u64,
    pub avg_fitness_ema: u64,
    pub diversity_ema: u64,
    pub mutations_performed: u64,
    pub crossovers_performed: u64,
    pub fitness_tests_run: u64,
    pub pressure_ema: u64,
}

// ---------------------------------------------------------------------------
// AppsEvolution
// ---------------------------------------------------------------------------

/// Genetic-programming engine that evolves classifier and allocator
/// algorithms over generations.
pub struct AppsEvolution {
    population: BTreeMap<u64, AlgorithmGenome>,
    fitness_cases: BTreeMap<u64, FitnessCase>,
    pressure_history: VecDeque<PressureRecord>,
    crossover_log: Vec<CrossoverResult>,
    stats: EvolutionStats,
    generation: u64,
    rng: u64,
}

impl AppsEvolution {
    /// Create a new evolution engine.
    pub fn new(seed: u64) -> Self {
        Self {
            population: BTreeMap::new(),
            fitness_cases: BTreeMap::new(),
            pressure_history: VecDeque::new(),
            crossover_log: Vec::new(),
            stats: EvolutionStats::default(),
            generation: 0,
            rng: seed | 1,
        }
    }

    // -- population initialization ------------------------------------------

    /// Seed the population with random genomes.
    pub fn seed_population(&mut self, count: usize) {
        for _ in 0..count.min(MAX_POPULATION) {
            let mut genes = Vec::with_capacity(GENE_COUNT);
            for _ in 0..GENE_COUNT {
                genes.push(xorshift64(&mut self.rng) % 101);
            }
            let gid = self.hash_genes(&genes);
            self.population.insert(gid, AlgorithmGenome {
                genome_id: gid,
                genes,
                fitness: 0,
                generation: self.generation,
                lineage: VecDeque::new(),
                evaluations: 0,
                fitness_history: Vec::new(),
            });
        }
        self.stats.population_size = self.population.len() as u64;
    }

    /// Register a fitness test case.
    #[inline]
    pub fn register_fitness_case(
        &mut self,
        features: &[u64],
        expected: u64,
        weight: u64,
    ) -> u64 {
        let cid = self.hash_features(features) ^ xorshift64(&mut self.rng);
        self.fitness_cases.insert(cid, FitnessCase {
            case_id: cid,
            input_features: features.to_vec(),
            expected_output: expected,
            weight: weight.max(1),
        });
        cid
    }

    // -- public API ---------------------------------------------------------

    /// Evolve the population of classifiers through one generation.
    /// Returns the best genome ID after selection.
    pub fn evolve_classifier(&mut self) -> Option<u64> {
        if self.population.is_empty() {
            return None;
        }

        self.generation += 1;

        // 1. Evaluate fitness for all genomes.
        let genome_ids: Vec<u64> = self.population.keys().copied().collect();
        for gid in &genome_ids {
            self.evaluate_genome(*gid);
        }

        // 2. Record pressure before selection.
        self.record_pressure();

        // 3. Select elites.
        let elite_count = (self.population.len() / ELITE_FRACTION).max(1);
        let mut sorted: Vec<(u64, u64)> = self
            .population
            .values()
            .map(|g| (g.genome_id, g.fitness))
            .collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));

        let elites: Vec<u64> = sorted.iter().take(elite_count).map(|(id, _)| *id).collect();
        let best_id = elites.first().copied()?;

        // 4. Breed next generation from elites.
        let mut new_genomes: Vec<AlgorithmGenome> = Vec::new();
        for &eid in &elites {
            if let Some(g) = self.population.get(&eid) {
                new_genomes.push(g.clone());
            }
        }

        // Fill remaining slots with crossover and mutation.
        while new_genomes.len() < MAX_POPULATION && elites.len() >= 2 {
            let idx_a = (xorshift64(&mut self.rng) as usize) % elites.len();
            let idx_b = (xorshift64(&mut self.rng) as usize) % elites.len();
            if idx_a == idx_b {
                continue;
            }
            let child = self.crossover_internal(elites[idx_a], elites[idx_b]);
            if let Some(c) = child {
                new_genomes.push(c);
            }
        }

        // Replace population.
        self.population.clear();
        for g in new_genomes {
            self.population.insert(g.genome_id, g);
        }

        self.stats.population_size = self.population.len() as u64;
        self.stats.generation = self.generation;
        self.refresh_stats();
        Some(best_id)
    }

    /// Mutate a specific genome, producing a new variant.
    pub fn mutate_strategy(&mut self, genome_id: u64) -> Option<u64> {
        let parent = self.population.get(&genome_id)?.clone();

        let mut new_genes = parent.genes.clone();
        let num_mutations = (xorshift64(&mut self.rng) % 3) + 1;
        for _ in 0..num_mutations {
            if new_genes.is_empty() {
                break;
            }
            let idx = (xorshift64(&mut self.rng) as usize) % new_genes.len();
            let delta = xorshift64(&mut self.rng) % (MUTATION_SIGMA * 2 + 1);
            new_genes[idx] = new_genes[idx]
                .wrapping_add(delta)
                .wrapping_sub(MUTATION_SIGMA)
                .min(100);
        }

        let new_id = self.hash_genes(&new_genes);
        let mut lineage = parent.lineage.clone();
        lineage.push(genome_id);
        if lineage.len() > 16 {
            lineage.pop_front();
        }

        self.population.insert(new_id, AlgorithmGenome {
            genome_id: new_id,
            genes: new_genes,
            fitness: 0,
            generation: self.generation,
            lineage,
            evaluations: 0,
            fitness_history: Vec::new(),
        });

        self.stats.mutations_performed += 1;
        self.enforce_population_cap();
        self.stats.population_size = self.population.len() as u64;
        Some(new_id)
    }

    /// Perform crossover between two genomes.
    #[inline]
    pub fn crossover_algorithms(&mut self, genome_a: u64, genome_b: u64) -> Option<u64> {
        let child = self.crossover_internal(genome_a, genome_b)?;
        let child_id = child.genome_id;
        self.population.insert(child_id, child);
        self.enforce_population_cap();
        self.stats.population_size = self.population.len() as u64;
        Some(child_id)
    }

    /// Run a fitness test on a specific genome and return its score.
    #[inline(always)]
    pub fn fitness_test(&mut self, genome_id: u64) -> Option<u64> {
        self.evaluate_genome(genome_id);
        self.population.get(&genome_id).map(|g| g.fitness)
    }

    /// Return the best genome in the current generation.
    #[inline(always)]
    pub fn generation_best(&self) -> Option<&AlgorithmGenome> {
        self.population.values().max_by_key(|g| g.fitness)
    }

    /// Return a summary of evolutionary pressure over recent generations.
    #[inline(always)]
    pub fn evolutionary_pressure(&self) -> u64 {
        self.stats.pressure_ema
    }

    /// Return current statistics.
    #[inline(always)]
    pub fn stats(&self) -> &EvolutionStats {
        &self.stats
    }

    /// Return pressure history.
    #[inline(always)]
    pub fn pressure_history(&self) -> &[PressureRecord] {
        &self.pressure_history
    }

    /// Return crossover log.
    #[inline(always)]
    pub fn crossover_log(&self) -> &[CrossoverResult] {
        &self.crossover_log
    }

    // -- internal -----------------------------------------------------------

    fn evaluate_genome(&mut self, genome_id: u64) {
        let genome = match self.population.get(&genome_id) {
            Some(g) => g.clone(),
            None => return,
        };

        if self.fitness_cases.is_empty() {
            // No test cases — assign random baseline fitness.
            let f = xorshift64(&mut self.rng) % 50 + 25;
            if let Some(g) = self.population.get_mut(&genome_id) {
                g.fitness = f;
                g.evaluations += 1;
                g.fitness_history.push(f);
            }
            self.stats.fitness_tests_run += 1;
            return;
        }

        let mut total_score: u64 = 0;
        let mut total_weight: u64 = 0;
        for case in self.fitness_cases.values() {
            let output = self.run_genome(&genome.genes, &case.input_features);
            let error = if output > case.expected_output {
                output - case.expected_output
            } else {
                case.expected_output - output
            };
            let case_score = 100u64.saturating_sub(error.min(100));
            total_score += case_score * case.weight;
            total_weight += case.weight;
        }

        let fitness = if total_weight > 0 {
            total_score / total_weight
        } else {
            0
        };

        if let Some(g) = self.population.get_mut(&genome_id) {
            g.fitness = fitness;
            g.evaluations += 1;
            g.fitness_history.push(fitness);
        }
        self.stats.fitness_tests_run += 1;
    }

    fn run_genome(&self, genes: &[u64], inputs: &[u64]) -> u64 {
        // Simple weighted-sum model: genome genes act as weights.
        let mut result: u64 = 0;
        for (i, &inp) in inputs.iter().enumerate() {
            let weight = genes.get(i).copied().unwrap_or(50);
            result = result.wrapping_add(inp.wrapping_mul(weight) / 100);
        }
        result % 101
    }

    fn crossover_internal(
        &mut self,
        genome_a: u64,
        genome_b: u64,
    ) -> Option<AlgorithmGenome> {
        let ga = self.population.get(&genome_a)?;
        let gb = self.population.get(&genome_b)?;

        let max_len = ga.genes.len().max(gb.genes.len());
        if max_len == 0 {
            return None;
        }

        let crossover_point = (xorshift64(&mut self.rng) as usize) % max_len;
        let mut child_genes = Vec::with_capacity(max_len);

        for i in 0..max_len {
            let va = ga.genes.get(i).copied().unwrap_or(50);
            let vb = gb.genes.get(i).copied().unwrap_or(50);
            if i < crossover_point {
                child_genes.push(va);
            } else {
                child_genes.push(vb);
            }
        }

        // Apply light mutation to offspring.
        if xorshift64(&mut self.rng) % 100 < 30 {
            if !child_genes.is_empty() {
                let idx = (xorshift64(&mut self.rng) as usize) % child_genes.len();
                let delta = xorshift64(&mut self.rng) % (MUTATION_SIGMA + 1);
                child_genes[idx] = child_genes[idx].wrapping_add(delta).min(100);
            }
        }

        let child_id = self.hash_genes(&child_genes);
        let predicted_fitness = (ga.fitness + gb.fitness) / 2;
        let mut lineage = ga.lineage.clone();
        lineage.push(genome_a);
        lineage.push(genome_b);
        if lineage.len() > 16 {
            lineage.drain(0..lineage.len() - 16);
        }

        self.stats.crossovers_performed += 1;
        self.crossover_log.push(CrossoverResult {
            parent_a: genome_a,
            parent_b: genome_b,
            offspring_id: child_id,
            crossover_point,
            predicted_fitness,
        });

        Some(AlgorithmGenome {
            genome_id: child_id,
            genes: child_genes,
            fitness: 0,
            generation: self.generation,
            lineage,
            evaluations: 0,
            fitness_history: Vec::new(),
        })
    }

    fn hash_genes(&mut self, genes: &[u64]) -> u64 {
        let mut buf = Vec::with_capacity(genes.len() * 8);
        for g in genes {
            buf.extend_from_slice(&g.to_le_bytes());
        }
        fnv1a(&buf) ^ xorshift64(&mut self.rng)
    }

    fn hash_features(&mut self, features: &[u64]) -> u64 {
        let mut buf = Vec::with_capacity(features.len() * 8);
        for f in features {
            buf.extend_from_slice(&f.to_le_bytes());
        }
        fnv1a(&buf)
    }

    fn record_pressure(&mut self) {
        if self.population.is_empty() {
            return;
        }
        let fitnesses: Vec<u64> = self.population.values().map(|g| g.fitness).collect();
        let avg = fitnesses.iter().sum::<u64>() / fitnesses.len() as u64;
        let best = fitnesses.iter().copied().max().unwrap_or(0);
        let worst = fitnesses.iter().copied().min().unwrap_or(0);
        let diversity = self.compute_diversity();
        let survival = (self.population.len() as u64 * 100) / MAX_POPULATION as u64;

        let pressure = PressureRecord {
            generation: self.generation,
            avg_fitness: avg,
            best_fitness: best,
            worst_fitness: worst,
            diversity,
            survival_rate: survival,
        };

        self.pressure_history.push_back(pressure);
        if self.pressure_history.len() > PRESSURE_WINDOW {
            self.pressure_history.pop_front();
        }

        // Pressure = how hard it is to improve: inverse of recent fitness gains.
        let pressure_val = if self.pressure_history.len() >= 2 {
            let recent = self.pressure_history.back().unwrap().best_fitness;
            let older = self.pressure_history.first().unwrap().best_fitness;
            let improvement = recent.saturating_sub(older);
            100u64.saturating_sub(improvement.min(100))
        } else {
            50
        };
        self.stats.pressure_ema = ema_update(self.stats.pressure_ema, pressure_val);
    }

    fn compute_diversity(&self) -> u64 {
        if self.population.len() < 2 {
            return 0;
        }
        let genomes: Vec<&Vec<u64>> = self.population.values().map(|g| &g.genes).collect();
        let mut total_dist: u64 = 0;
        let mut pairs: u64 = 0;
        for i in 0..genomes.len().min(20) {
            for j in (i + 1)..genomes.len().min(20) {
                total_dist += self.gene_distance(genomes[i], genomes[j]);
                pairs += 1;
            }
        }
        if pairs == 0 {
            return 0;
        }
        total_dist / pairs
    }

    fn gene_distance(&self, a: &[u64], b: &[u64]) -> u64 {
        let max_len = a.len().max(b.len());
        if max_len == 0 {
            return 0;
        }
        let mut total: u64 = 0;
        for i in 0..max_len {
            let va = a.get(i).copied().unwrap_or(50);
            let vb = b.get(i).copied().unwrap_or(50);
            total += if va > vb { va - vb } else { vb - va };
        }
        total / max_len as u64
    }

    fn enforce_population_cap(&mut self) {
        while self.population.len() > MAX_POPULATION {
            let worst = self
                .population
                .iter()
                .min_by_key(|(_, g)| g.fitness)
                .map(|(k, _)| *k);
            if let Some(key) = worst {
                self.population.remove(&key);
            } else {
                break;
            }
        }
    }

    fn refresh_stats(&mut self) {
        if self.population.is_empty() {
            return;
        }
        let sum: u64 = self.population.values().map(|g| g.fitness).sum();
        let avg = sum / self.population.len() as u64;
        self.stats.avg_fitness_ema = ema_update(self.stats.avg_fitness_ema, avg);
        self.stats.best_fitness = self
            .population
            .values()
            .map(|g| g.fitness)
            .max()
            .unwrap_or(0);
        self.stats.diversity_ema = ema_update(self.stats.diversity_ema, self.compute_diversity());
    }
}
