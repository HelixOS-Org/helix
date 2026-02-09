// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Synthesis Engine — Protocol Evolution
//!
//! Evolves cooperation protocols through a genetic algorithm.  Each protocol
//! is encoded as a `ProtocolGenome` — a fixed-length vector of integer genes.
//! The engine applies tournament selection, crossover, and mutation to
//! produce fitter protocols every generation, tracking novelty to prevent
//! premature convergence.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const EMA_ALPHA_NUM: u64 = 3;
const EMA_ALPHA_DEN: u64 = 10;
const GENOME_LENGTH: usize = 16;
const MAX_POPULATION: usize = 256;
const MAX_ARCHIVE: usize = 256;
const TOURNAMENT_SIZE: usize = 4;
const CROSSOVER_RATE: u64 = 70;
const MUTATION_RATE: u64 = 15;
const MAX_EVENTS: usize = 2048;
const GENE_MAX: u64 = 1000;

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

fn clamp(v: u64, lo: u64, hi: u64) -> u64 {
    if v < lo { lo } else if v > hi { hi } else { v }
}

fn abs_diff(a: u64, b: u64) -> u64 {
    if a > b { a - b } else { b - a }
}

// ---------------------------------------------------------------------------
// Synthesis method
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum SynthesisMethod {
    Mutation,
    Crossover,
    RandomGenesis,
    NoveltyDriven,
}

// ---------------------------------------------------------------------------
// Protocol genome
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct ProtocolGenome {
    pub genome_id: u64,
    pub genes: Vec<u64>,
    pub fitness: u64,
    pub novelty: u64,
    pub generation: u64,
    pub method: SynthesisMethod,
}

// ---------------------------------------------------------------------------
// Synthesis event
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct SynthesisEvent {
    pub event_id: u64,
    pub parent_ids: Vec<u64>,
    pub child_id: u64,
    pub method: SynthesisMethod,
    pub fitness_delta: i64,
    pub generation: u64,
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct SynthesisStats {
    pub population_size: usize,
    pub archive_size: usize,
    pub total_events: usize,
    pub best_fitness: u64,
    pub avg_fitness: u64,
    pub avg_novelty: u64,
    pub generation: u64,
    pub synthesis_rate_ema: u64,
}

// ---------------------------------------------------------------------------
// CoopSynthesisEngine
// ---------------------------------------------------------------------------

pub struct CoopSynthesisEngine {
    population: BTreeMap<u64, ProtocolGenome>,
    novelty_archive: Vec<Vec<u64>>,
    events: BTreeMap<u64, SynthesisEvent>,
    rng_state: u64,
    generation: u64,
    synthesis_rate_ema: u64,
    best_fitness_ever: u64,
    stats: SynthesisStats,
}

impl CoopSynthesisEngine {
    pub fn new(seed: u64) -> Self {
        Self {
            population: BTreeMap::new(),
            novelty_archive: Vec::new(),
            events: BTreeMap::new(),
            rng_state: seed | 1,
            generation: 0,
            synthesis_rate_ema: 0,
            best_fitness_ever: 0,
            stats: SynthesisStats {
                population_size: 0,
                archive_size: 0,
                total_events: 0,
                best_fitness: 0,
                avg_fitness: 0,
                avg_novelty: 0,
                generation: 0,
                synthesis_rate_ema: 0,
            },
        }
    }

    // -- seed population ----------------------------------------------------

    pub fn seed_population(&mut self, count: usize) {
        let count = count.min(MAX_POPULATION);
        for _ in 0..count {
            if self.population.len() >= MAX_POPULATION {
                break;
            }
            let genome = self.random_genome(SynthesisMethod::RandomGenesis);
            self.population.insert(genome.genome_id, genome);
        }
    }

    fn random_genome(&mut self, method: SynthesisMethod) -> ProtocolGenome {
        let mut genes = Vec::with_capacity(GENOME_LENGTH);
        for _ in 0..GENOME_LENGTH {
            let g = xorshift64(&mut self.rng_state) % GENE_MAX;
            genes.push(g);
        }
        let gid = self.genome_hash(&genes);
        let fitness = self.evaluate_fitness(&genes);
        let novelty = self.compute_novelty(&genes);
        ProtocolGenome {
            genome_id: gid,
            genes,
            fitness,
            novelty,
            generation: self.generation,
            method,
        }
    }

    fn genome_hash(&self, genes: &[u64]) -> u64 {
        let bytes: Vec<u8> = genes.iter().flat_map(|g| g.to_le_bytes()).collect();
        fnv1a(&bytes)
    }

    // -- fitness evaluation -------------------------------------------------

    fn evaluate_fitness(&self, genes: &[u64]) -> u64 {
        let sum: u64 = genes.iter().sum();
        let mean = sum / GENOME_LENGTH as u64;
        let variance = genes.iter().map(|&g| {
            let d = abs_diff(g, mean);
            d * d
        }).sum::<u64>() / GENOME_LENGTH as u64;

        let balance_score = 100u64.saturating_sub(variance / 100);
        let magnitude_score = clamp(mean / 10, 0, 100);

        let pair_harmony: u64 = genes.windows(2).map(|w| {
            100u64.saturating_sub(abs_diff(w[0], w[1]) / 10)
        }).sum::<u64>() / (GENOME_LENGTH as u64 - 1).max(1);

        clamp((balance_score + magnitude_score + pair_harmony) / 3, 0, 100)
    }

    // -- novelty computation ------------------------------------------------

    fn compute_novelty(&self, genes: &[u64]) -> u64 {
        if self.novelty_archive.is_empty() {
            return 100;
        }
        let mut min_dist = u64::MAX;
        for archived in &self.novelty_archive {
            let dist = self.genome_distance(genes, archived);
            if dist < min_dist {
                min_dist = dist;
            }
        }
        clamp(min_dist / GENOME_LENGTH as u64, 0, 100)
    }

    fn genome_distance(&self, a: &[u64], b: &[u64]) -> u64 {
        let len = a.len().min(b.len());
        let mut total = 0u64;
        for i in 0..len {
            total += abs_diff(a[i], b[i]);
        }
        total
    }

    // -- evolve protocol ----------------------------------------------------

    pub fn evolve_protocol(&mut self) -> Option<ProtocolGenome> {
        if self.population.len() < TOURNAMENT_SIZE {
            return None;
        }
        self.generation += 1;

        let roll = xorshift64(&mut self.rng_state) % 100;
        let child = if roll < CROSSOVER_RATE {
            self.crossover_child()
        } else if roll < CROSSOVER_RATE + MUTATION_RATE {
            self.mutation_child()
        } else {
            self.random_genome(SynthesisMethod::NoveltyDriven)
        };

        if child.fitness > self.best_fitness_ever {
            self.best_fitness_ever = child.fitness;
        }

        if self.novelty_archive.len() < MAX_ARCHIVE {
            self.novelty_archive.push(child.genes.clone());
        } else {
            let idx = (xorshift64(&mut self.rng_state) as usize) % MAX_ARCHIVE;
            self.novelty_archive[idx] = child.genes.clone();
        }

        self.record_event(&child, &[]);

        if self.population.len() >= MAX_POPULATION {
            let worst_id = self.worst_genome_id();
            if let Some(wid) = worst_id {
                self.population.remove(&wid);
            }
        }

        self.population.insert(child.genome_id, child.clone());
        self.synthesis_rate_ema = ema_update(self.synthesis_rate_ema, 100);
        self.refresh_stats();
        Some(child)
    }

    fn crossover_child(&mut self) -> ProtocolGenome {
        let p1 = self.tournament_select();
        let p2 = self.tournament_select();

        let crosspoint = (xorshift64(&mut self.rng_state) as usize) % GENOME_LENGTH;
        let mut genes = Vec::with_capacity(GENOME_LENGTH);
        for i in 0..GENOME_LENGTH {
            if i < crosspoint {
                genes.push(p1.genes.get(i).copied().unwrap_or(0));
            } else {
                genes.push(p2.genes.get(i).copied().unwrap_or(0));
            }
        }

        let gid = self.genome_hash(&genes);
        let fitness = self.evaluate_fitness(&genes);
        let novelty = self.compute_novelty(&genes);
        ProtocolGenome {
            genome_id: gid,
            genes,
            fitness,
            novelty,
            generation: self.generation,
            method: SynthesisMethod::Crossover,
        }
    }

    fn mutation_child(&mut self) -> ProtocolGenome {
        let parent = self.tournament_select();
        let mut genes = parent.genes.clone();
        let idx = (xorshift64(&mut self.rng_state) as usize) % GENOME_LENGTH;
        let delta = xorshift64(&mut self.rng_state) % 200;
        let direction = xorshift64(&mut self.rng_state) % 2;
        if direction == 0 {
            genes[idx] = genes[idx].saturating_add(delta).min(GENE_MAX);
        } else {
            genes[idx] = genes[idx].saturating_sub(delta);
        }

        let gid = self.genome_hash(&genes);
        let fitness = self.evaluate_fitness(&genes);
        let novelty = self.compute_novelty(&genes);
        ProtocolGenome {
            genome_id: gid,
            genes,
            fitness,
            novelty,
            generation: self.generation,
            method: SynthesisMethod::Mutation,
        }
    }

    fn tournament_select(&mut self) -> ProtocolGenome {
        let ids: Vec<u64> = self.population.keys().copied().collect();
        let mut best: Option<&ProtocolGenome> = None;
        for _ in 0..TOURNAMENT_SIZE {
            let idx = (xorshift64(&mut self.rng_state) as usize) % ids.len();
            if let Some(candidate) = self.population.get(&ids[idx]) {
                match best {
                    None => best = Some(candidate),
                    Some(b) => {
                        if candidate.fitness > b.fitness {
                            best = Some(candidate);
                        }
                    }
                }
            }
        }
        best.cloned().unwrap_or_else(|| self.random_genome(SynthesisMethod::RandomGenesis))
    }

    fn worst_genome_id(&self) -> Option<u64> {
        self.population.values()
            .min_by_key(|g| g.fitness)
            .map(|g| g.genome_id)
    }

    fn record_event(&mut self, child: &ProtocolGenome, parents: &[u64]) {
        if self.events.len() >= MAX_EVENTS {
            if let Some(&first_key) = self.events.keys().next() {
                self.events.remove(&first_key);
            }
        }
        let eid = fnv1a(&child.genome_id.to_le_bytes());
        let event = SynthesisEvent {
            event_id: eid,
            parent_ids: parents.to_vec(),
            child_id: child.genome_id,
            method: child.method.clone(),
            fitness_delta: 0,
            generation: self.generation,
        };
        self.events.insert(eid, event);
    }

    // -- synthesize mechanism -----------------------------------------------

    pub fn synthesize_mechanism(&mut self) -> Option<ProtocolGenome> {
        let mut best: Option<ProtocolGenome> = None;
        for _ in 0..5 {
            if let Some(child) = self.evolve_protocol() {
                match &best {
                    None => best = Some(child),
                    Some(b) => {
                        if child.fitness > b.fitness {
                            best = Some(child);
                        }
                    }
                }
            }
        }
        best
    }

    // -- novelty score (public) ---------------------------------------------

    pub fn novelty_score(&self, genome_id: u64) -> u64 {
        match self.population.get(&genome_id) {
            Some(g) => g.novelty,
            None => 0,
        }
    }

    // -- protocol fitness (public) ------------------------------------------

    pub fn protocol_fitness(&self, genome_id: u64) -> u64 {
        match self.population.get(&genome_id) {
            Some(g) => g.fitness,
            None => 0,
        }
    }

    // -- synthesis rate (public) --------------------------------------------

    pub fn synthesis_rate(&self) -> u64 {
        self.synthesis_rate_ema
    }

    // -- stats --------------------------------------------------------------

    fn refresh_stats(&mut self) {
        let pop = self.population.len();
        let (best, avg_f, avg_n) = if pop > 0 {
            let bf = self.population.values().map(|g| g.fitness).max().unwrap_or(0);
            let af = self.population.values().map(|g| g.fitness).sum::<u64>() / pop as u64;
            let an = self.population.values().map(|g| g.novelty).sum::<u64>() / pop as u64;
            (bf, af, an)
        } else {
            (0, 0, 0)
        };

        self.stats = SynthesisStats {
            population_size: pop,
            archive_size: self.novelty_archive.len(),
            total_events: self.events.len(),
            best_fitness: best,
            avg_fitness: avg_f,
            avg_novelty: avg_n,
            generation: self.generation,
            synthesis_rate_ema: self.synthesis_rate_ema,
        };
    }

    pub fn stats(&self) -> SynthesisStats {
        self.stats.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_and_evolve() {
        let mut engine = CoopSynthesisEngine::new(42);
        engine.seed_population(20);
        assert!(engine.stats().population_size >= 10);
        let child = engine.evolve_protocol();
        assert!(child.is_some());
        assert!(child.unwrap().fitness > 0);
    }

    #[test]
    fn test_synthesize_mechanism() {
        let mut engine = CoopSynthesisEngine::new(7);
        engine.seed_population(30);
        let best = engine.synthesize_mechanism();
        assert!(best.is_some());
    }

    #[test]
    fn test_novelty_tracking() {
        let mut engine = CoopSynthesisEngine::new(99);
        engine.seed_population(10);
        for _ in 0..10 {
            engine.evolve_protocol();
        }
        assert!(engine.stats().archive_size > 0);
    }

    #[test]
    fn test_synthesis_rate() {
        let mut engine = CoopSynthesisEngine::new(55);
        engine.seed_population(20);
        assert_eq!(engine.synthesis_rate(), 0);
        engine.evolve_protocol();
        assert!(engine.synthesis_rate() > 0);
    }
}
