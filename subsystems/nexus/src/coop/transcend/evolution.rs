// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Evolution — Self-Evolving Cooperation Protocols
//!
//! Applies genetic programming to evolve fairness algorithms, trust models,
//! and negotiation strategies.  Each generation undergoes tournament selection,
//! crossover, and xorshift64-guided mutation.  Fitness is scored via EMA over
//! observed cooperation outcomes.  The champion of each generation is archived
//! with its FNV-1a fingerprint for lineage tracking.

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
const EMA_ALPHA_NUM: u64 = 3;
const EMA_ALPHA_DEN: u64 = 10;
const MAX_POPULATION: usize = 512;
const MAX_GENERATIONS: usize = 4096;
const MAX_ARCHIVE: usize = 256;
const MUTATION_RATE: u64 = 10;
const CROSSOVER_RATE: u64 = 70;
const TOURNAMENT_SIZE: usize = 4;
const FITNESS_FLOOR: u64 = 5;
const ELITE_FRACTION_NUM: u64 = 1;
const ELITE_FRACTION_DEN: u64 = 10;

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

fn clamp(val: u64, lo: u64, hi: u64) -> u64 {
    if val < lo { lo } else if val > hi { hi } else { val }
}

// ---------------------------------------------------------------------------
// Protocol kind
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum ProtocolKind {
    Fairness,
    Trust,
    Negotiation,
    Hybrid,
}

// ---------------------------------------------------------------------------
// Genome — encoded cooperation protocol
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Genome {
    pub genome_id: u64,
    pub kind: ProtocolKind,
    pub genes: Vec<u64>,
    pub fingerprint: u64,
    pub fitness: u64,
    pub generation: u64,
    pub parent_a: u64,
    pub parent_b: u64,
    pub creation_tick: u64,
}

// ---------------------------------------------------------------------------
// Champion record
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Champion {
    pub champion_id: u64,
    pub generation: u64,
    pub fitness: u64,
    pub fingerprint: u64,
    pub gene_count: u64,
    pub crowned_tick: u64,
}

// ---------------------------------------------------------------------------
// Evolution event
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct EvolutionEvent {
    pub event_id: u64,
    pub event_type: EvolutionEventType,
    pub generation: u64,
    pub affected_genome: u64,
    pub tick: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EvolutionEventType {
    Mutation,
    Crossover,
    Selection,
    Extinction,
    Elitism,
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
#[repr(align(64))]
pub struct EvolutionStats {
    pub current_generation: u64,
    pub total_genomes_created: u64,
    pub mutations_applied: u64,
    pub crossovers_performed: u64,
    pub avg_fitness: u64,
    pub best_fitness: u64,
    pub champion_count: u64,
    pub evolutionary_rate: u64,
    pub extinctions: u64,
}

// ---------------------------------------------------------------------------
// Manager
// ---------------------------------------------------------------------------

pub struct CoopEvolution {
    population: BTreeMap<u64, Genome>,
    champions: BTreeMap<u64, Champion>,
    fitness_index: LinearMap<u64, 64>,
    events: VecDeque<EvolutionEvent>,
    generation_best: LinearMap<u64, 64>,
    stats: EvolutionStats,
    rng_state: u64,
    current_tick: u64,
}

impl CoopEvolution {
    pub fn new() -> Self {
        Self {
            population: BTreeMap::new(),
            champions: BTreeMap::new(),
            fitness_index: LinearMap::new(),
            events: VecDeque::new(),
            generation_best: LinearMap::new(),
            stats: EvolutionStats::default(),
            rng_state: 0xBAAD_F00D_CAFE_1337u64,
            current_tick: 0,
        }
    }

    // -----------------------------------------------------------------------
    // evolve_fairness — create and evolve a fairness protocol genome
    // -----------------------------------------------------------------------
    pub fn evolve_fairness(
        &mut self,
        seed_genes: &[u64],
        observed_fairness: u64,
    ) -> u64 {
        self.current_tick += 1;

        let mut genes = seed_genes.to_vec();
        // Inject initial variation
        for g in genes.iter_mut() {
            let r = xorshift64(&mut self.rng_state);
            if r % 100 < MUTATION_RATE {
                let delta = r % 30;
                *g = if r % 2 == 0 {
                    g.wrapping_add(delta)
                } else {
                    g.saturating_sub(delta)
                };
            }
        }
        if genes.is_empty() {
            genes.push(xorshift64(&mut self.rng_state) % 100);
        }

        let fingerprint = genes.iter().fold(FNV_OFFSET, |acc, &g| {
            acc ^ fnv1a(&g.to_le_bytes())
        });
        let gid = fingerprint ^ self.current_tick;

        let fitness = clamp(observed_fairness, FITNESS_FLOOR, 100);

        let genome = Genome {
            genome_id: gid,
            kind: ProtocolKind::Fairness,
            genes,
            fingerprint,
            fitness,
            generation: self.stats.current_generation,
            parent_a: 0,
            parent_b: 0,
            creation_tick: self.current_tick,
        };

        self.insert_genome(genome);
        self.stats.total_genomes_created += 1;

        gid
    }

    // -----------------------------------------------------------------------
    // mutate_protocol — apply mutation to an existing protocol genome
    // -----------------------------------------------------------------------
    pub fn mutate_protocol(&mut self, genome_id: u64) -> u64 {
        self.current_tick += 1;

        let cloned = self.population.get(&genome_id).cloned();
        let parent = match cloned {
            Some(g) => g,
            None => return 0,
        };

        let mut new_genes = parent.genes.clone();
        let num_mutations = (xorshift64(&mut self.rng_state) % 3) + 1;

        for _ in 0..num_mutations {
            if new_genes.is_empty() {
                new_genes.push(xorshift64(&mut self.rng_state) % 100);
                continue;
            }
            let idx = xorshift64(&mut self.rng_state) as usize % new_genes.len();
            let mutation_type = xorshift64(&mut self.rng_state) % 4;

            match mutation_type {
                0 => {
                    // Point mutation
                    let delta = xorshift64(&mut self.rng_state) % 20;
                    new_genes[idx] = new_genes[idx].wrapping_add(delta);
                }
                1 => {
                    // Deletion
                    if new_genes.len() > 1 {
                        new_genes.remove(idx);
                    }
                }
                2 => {
                    // Insertion
                    let val = xorshift64(&mut self.rng_state) % 100;
                    new_genes.insert(idx, val);
                }
                _ => {
                    // Inversion
                    let new_val = 100u64.saturating_sub(new_genes[idx] % 100);
                    new_genes[idx] = new_val;
                }
            }
        }

        let fp = new_genes.iter().fold(FNV_OFFSET, |acc, &g| {
            acc ^ fnv1a(&g.to_le_bytes())
        });
        let child_id = fp ^ self.current_tick;

        let child = Genome {
            genome_id: child_id,
            kind: parent.kind.clone(),
            genes: new_genes,
            fingerprint: fp,
            fitness: parent.fitness,
            generation: self.stats.current_generation,
            parent_a: genome_id,
            parent_b: 0,
            creation_tick: self.current_tick,
        };

        self.insert_genome(child);
        self.stats.mutations_applied += 1;
        self.stats.total_genomes_created += 1;

        self.record_event(EvolutionEventType::Mutation, child_id);

        child_id
    }

    // -----------------------------------------------------------------------
    // crossover_trust — crossover two trust protocol genomes
    // -----------------------------------------------------------------------
    pub fn crossover_trust(&mut self, parent_a_id: u64, parent_b_id: u64) -> u64 {
        self.current_tick += 1;

        let pa = self.population.get(&parent_a_id).cloned();
        let pb = self.population.get(&parent_b_id).cloned();

        let (ga, gb) = match (pa, pb) {
            (Some(a), Some(b)) => (a, b),
            _ => return 0,
        };

        let r = xorshift64(&mut self.rng_state) % 100;
        if r >= CROSSOVER_RATE {
            // No crossover — return clone of fitter parent
            return if ga.fitness >= gb.fitness { parent_a_id } else { parent_b_id };
        }

        let min_len = core::cmp::min(ga.genes.len(), gb.genes.len());
        let crossover_point = if min_len > 1 {
            xorshift64(&mut self.rng_state) as usize % min_len
        } else {
            0
        };

        let mut child_genes = Vec::new();
        for (i, &g) in ga.genes.iter().enumerate() {
            if i < crossover_point {
                child_genes.push(g);
            }
        }
        for (i, &g) in gb.genes.iter().enumerate() {
            if i >= crossover_point {
                child_genes.push(g);
            }
        }

        // Post-crossover mutation
        let mr = xorshift64(&mut self.rng_state) % 100;
        if mr < MUTATION_RATE && !child_genes.is_empty() {
            let idx = xorshift64(&mut self.rng_state) as usize % child_genes.len();
            child_genes[idx] = child_genes[idx].wrapping_add(xorshift64(&mut self.rng_state) % 10);
        }

        let fp = child_genes.iter().fold(FNV_OFFSET, |acc, &g| {
            acc ^ fnv1a(&g.to_le_bytes())
        });
        let child_id = fp ^ self.current_tick;

        let child_fitness = (ga.fitness + gb.fitness) / 2;

        let child = Genome {
            genome_id: child_id,
            kind: ProtocolKind::Trust,
            genes: child_genes,
            fingerprint: fp,
            fitness: child_fitness,
            generation: self.stats.current_generation,
            parent_a: parent_a_id,
            parent_b: parent_b_id,
            creation_tick: self.current_tick,
        };

        self.insert_genome(child);
        self.stats.crossovers_performed += 1;
        self.stats.total_genomes_created += 1;

        self.record_event(EvolutionEventType::Crossover, child_id);

        child_id
    }

    // -----------------------------------------------------------------------
    // protocol_fitness — evaluate fitness of a protocol genome
    // -----------------------------------------------------------------------
    pub fn protocol_fitness(&mut self, genome_id: u64, observed_outcome: u64) -> u64 {
        if let Some(genome) = self.population.get_mut(&genome_id) {
            let new_fitness = ema_update(genome.fitness, observed_outcome);
            genome.fitness = clamp(new_fitness, FITNESS_FLOOR, 200);
            self.fitness_index.insert(genome_id, genome.fitness);
            self.stats.avg_fitness = ema_update(self.stats.avg_fitness, genome.fitness);
            if genome.fitness > self.stats.best_fitness {
                self.stats.best_fitness = genome.fitness;
            }
            genome.fitness
        } else {
            0
        }
    }

    // -----------------------------------------------------------------------
    // generation_champion — find and crown the best genome of a generation
    // -----------------------------------------------------------------------
    pub fn generation_champion(&mut self) -> Option<u64> {
        if self.population.is_empty() {
            return None;
        }

        let best = self
            .fitness_index
            .iter()
            .max_by_key(|(_, &f)| f)
            .map(|(&k, &f)| (k, f));

        if let Some((gid, fitness)) = best {
            let gene_count = self
                .population
                .get(&gid)
                .map(|g| g.genes.len() as u64)
                .unwrap_or(0);

            let champion = Champion {
                champion_id: gid,
                generation: self.stats.current_generation,
                fitness,
                fingerprint: self.population.get(&gid).map(|g| g.fingerprint).unwrap_or(0),
                gene_count,
                crowned_tick: self.current_tick,
            };

            self.generation_best
                .insert(self.stats.current_generation, gid);

            if self.champions.len() >= MAX_ARCHIVE {
                let oldest = self.champions.keys().next().copied();
                if let Some(k) = oldest {
                    self.champions.remove(&k);
                }
            }
            self.champions.insert(gid, champion);
            self.stats.champion_count += 1;

            Some(gid)
        } else {
            None
        }
    }

    // -----------------------------------------------------------------------
    // evolutionary_rate — measure how fast evolution is progressing
    // -----------------------------------------------------------------------
    pub fn evolutionary_rate(&mut self) -> u64 {
        if self.generation_best.len() < 2 {
            self.stats.evolutionary_rate = 0;
            return 0;
        }

        let gens: Vec<(&u64, &u64)> = self.generation_best.iter().collect();
        let mut improvements: u64 = 0;
        let mut comparisons: u64 = 0;

        for w in gens.windows(2) {
            let fit_a = self.fitness_index.get(w[0].1).copied().unwrap_or(0);
            let fit_b = self.fitness_index.get(w[1].1).copied().unwrap_or(0);
            if fit_b > fit_a {
                improvements += fit_b - fit_a;
            }
            comparisons += 1;
        }

        let rate = if comparisons > 0 {
            improvements / comparisons
        } else {
            0
        };

        self.stats.evolutionary_rate = ema_update(self.stats.evolutionary_rate, rate);
        self.stats.evolutionary_rate
    }

    // -----------------------------------------------------------------------
    // advance_generation — move to the next generation
    // -----------------------------------------------------------------------
    pub fn advance_generation(&mut self) {
        self.stats.current_generation += 1;

        // Extinction of unfit genomes
        let elite_count = core::cmp::max(
            (self.population.len() as u64 * ELITE_FRACTION_NUM / ELITE_FRACTION_DEN) as usize,
            1,
        );
        let mut sorted_fitness: Vec<(u64, u64)> = self
            .fitness_index
            .iter()
            .map(|(&k, &f)| (k, f))
            .collect();
        sorted_fitness.sort_by(|a, b| b.1.cmp(&a.1));

        let survivors: Vec<u64> = sorted_fitness.iter().take(elite_count).map(|(k, _)| *k).collect();
        let to_remove: Vec<u64> = self
            .population
            .keys()
            .filter(|k| !survivors.contains(k))
            .copied()
            .collect();

        for k in &to_remove {
            self.population.remove(k);
            self.fitness_index.remove(k);
            self.stats.extinctions += 1;
        }
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn insert_genome(&mut self, genome: Genome) {
        let gid = genome.genome_id;
        let fitness = genome.fitness;
        if self.population.len() >= MAX_POPULATION {
            // Remove least fit
            let worst = self
                .fitness_index
                .iter()
                .min_by_key(|(_, &f)| f)
                .map(|(&k, _)| k);
            if let Some(k) = worst {
                self.population.remove(&k);
                self.fitness_index.remove(k);
            }
        }
        self.population.insert(gid, genome);
        self.fitness_index.insert(gid, fitness);
    }

    fn record_event(&mut self, event_type: EvolutionEventType, genome_id: u64) {
        let eid = fnv1a(&genome_id.to_le_bytes()) ^ self.current_tick;
        let event = EvolutionEvent {
            event_id: eid,
            event_type,
            generation: self.stats.current_generation,
            affected_genome: genome_id,
            tick: self.current_tick,
        };
        if self.events.len() >= MAX_GENERATIONS {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    pub fn tick(&mut self) {
        self.current_tick += 1;

        // Stochastic fitness perturbation
        let keys: Vec<u64> = self.fitness_index.keys().copied().collect();
        for k in keys {
            let r = xorshift64(&mut self.rng_state) % 100;
            if r < 3 {
                if let Some(f) = self.fitness_index.get_mut(&k) {
                    let noise = xorshift64(&mut self.rng_state) % 5;
                    *f = if xorshift64(&mut self.rng_state) % 2 == 0 {
                        f.saturating_add(noise)
                    } else {
                        f.saturating_sub(noise)
                    };
                    *f = clamp(*f, FITNESS_FLOOR, 200);
                }
            }
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &EvolutionStats {
        &self.stats
    }

    #[inline(always)]
    pub fn population_size(&self) -> usize {
        self.population.len()
    }

    #[inline(always)]
    pub fn champion_count(&self) -> usize {
        self.champions.len()
    }
}
