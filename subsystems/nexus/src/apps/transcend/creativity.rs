// SPDX-License-Identifier: GPL-2.0
//! # Apps Creativity — Creative Problem Solving for App Management
//!
//! Invents novel scheduling strategies, unprecedented allocation schemes, and
//! creative optimization paths that no predefined algorithm would produce.
//! The engine maintains a population of strategy genomes that are mutated,
//! combined, and evaluated, retaining only those that demonstrably improve
//! application performance.
//!
//! Creativity is measured as the novelty distance of a strategy from all
//! previously evaluated strategies, rewarding genuine innovation over
//! incremental refinement.

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
const MAX_STRATEGIES: usize = 512;
const MAX_INVENTIONS: usize = 256;
const MUTATION_RANGE: u64 = 15;
const NOVELTY_FLOOR: u64 = 25;
const CREATIVITY_ELITE_FRAC: usize = 4; // top 1/4

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

/// Genome encoding of a scheduling/allocation strategy.
#[derive(Clone, Debug)]
pub struct StrategyGenome {
    pub genome_id: u64,
    pub genes: Vec<u64>,
    pub fitness: u64,
    pub novelty: u64,
    pub generation: u64,
    pub parent_ids: Vec<u64>,
}

/// A creative allocation scheme produced by the engine.
#[derive(Clone, Debug)]
pub struct CreativeAllocation {
    pub allocation_id: u64,
    pub label: String,
    pub genome_id: u64,
    pub cpu_weight: u64,
    pub mem_weight: u64,
    pub io_weight: u64,
    pub priority_boost: u64,
    pub effectiveness_ema: u64,
    pub times_applied: u64,
}

/// A novel scheduling strategy.
#[derive(Clone, Debug)]
pub struct NovelSchedule {
    pub schedule_id: u64,
    pub label: String,
    pub genome_id: u64,
    pub quantum_base: u64,
    pub preempt_threshold: u64,
    pub affinity_weight: u64,
    pub effectiveness_ema: u64,
    pub times_applied: u64,
}

/// Record of a combination invention — two strategies merged into one.
#[derive(Clone, Debug)]
pub struct CombinationInvention {
    pub invention_id: u64,
    pub parent_a: u64,
    pub parent_b: u64,
    pub offspring_genome_id: u64,
    pub predicted_fitness: u64,
    pub actual_fitness: u64,
    pub tick: u64,
}

/// Aggregated creativity statistics.
#[derive(Clone, Debug, Default)]
#[repr(align(64))]
pub struct CreativityStats {
    pub total_genomes: u64,
    pub total_allocations: u64,
    pub total_schedules: u64,
    pub total_inventions: u64,
    pub avg_fitness_ema: u64,
    pub avg_novelty_ema: u64,
    pub creativity_score_ema: u64,
    pub innovations_per_tick_ema: u64,
    pub generation: u64,
}

// ---------------------------------------------------------------------------
// AppsCreativity
// ---------------------------------------------------------------------------

/// Creative problem-solving engine for app management. Invents novel
/// scheduling and allocation strategies by evolutionary search.
pub struct AppsCreativity {
    genomes: BTreeMap<u64, StrategyGenome>,
    allocations: BTreeMap<u64, CreativeAllocation>,
    schedules: BTreeMap<u64, NovelSchedule>,
    inventions: Vec<CombinationInvention>,
    stats: CreativityStats,
    generation: u64,
    rng: u64,
    tick: u64,
    innovations_this_tick: u64,
}

impl AppsCreativity {
    /// Create a new creativity engine.
    pub fn new(seed: u64) -> Self {
        Self {
            genomes: BTreeMap::new(),
            allocations: BTreeMap::new(),
            schedules: BTreeMap::new(),
            inventions: Vec::new(),
            stats: CreativityStats::default(),
            generation: 0,
            rng: seed | 1,
            tick: 0,
            innovations_this_tick: 0,
        }
    }

    // -- genome management --------------------------------------------------

    /// Seed the population with a random genome.
    pub fn seed_genome(&mut self, gene_count: usize) -> u64 {
        let mut genes = Vec::with_capacity(gene_count);
        for _ in 0..gene_count {
            genes.push(xorshift64(&mut self.rng) % 101);
        }
        let gid = self.genome_hash(&genes);
        self.genomes.insert(gid, StrategyGenome {
            genome_id: gid,
            genes,
            fitness: 0,
            novelty: 0,
            generation: self.generation,
            parent_ids: Vec::new(),
        });
        self.stats.total_genomes = self.genomes.len() as u64;
        gid
    }

    /// Record the fitness of a genome after evaluation.
    #[inline]
    pub fn record_fitness(&mut self, genome_id: u64, fitness: u64) {
        if let Some(g) = self.genomes.get_mut(&genome_id) {
            g.fitness = ema_update(g.fitness, fitness.min(100));
        }
        self.refresh_avg_fitness();
    }

    // -- creative operations ------------------------------------------------

    /// Produce a creative allocation scheme from the best genome.
    pub fn creative_allocation(&mut self, label: &str) -> Option<CreativeAllocation> {
        let best = self.select_elite_genome()?;
        let gid = best.genome_id;
        let genes = best.genes.clone();

        let cpu_w = *genes.first().unwrap_or(&50);
        let mem_w = *genes.get(1).unwrap_or(&50);
        let io_w = *genes.get(2).unwrap_or(&50);
        let prio = *genes.get(3).unwrap_or(&10);

        let alloc_id = fnv1a(label.as_bytes()) ^ xorshift64(&mut self.rng);
        let alloc = CreativeAllocation {
            allocation_id: alloc_id,
            label: String::from(label),
            genome_id: gid,
            cpu_weight: cpu_w,
            mem_weight: mem_w,
            io_weight: io_w,
            priority_boost: prio,
            effectiveness_ema: 50,
            times_applied: 0,
        };
        self.allocations.insert(alloc_id, alloc.clone());
        self.stats.total_allocations = self.allocations.len() as u64;
        self.count_innovation();
        Some(alloc)
    }

    /// Produce a novel scheduling strategy from the best genome.
    pub fn novel_scheduling(&mut self, label: &str) -> Option<NovelSchedule> {
        let best = self.select_elite_genome()?;
        let gid = best.genome_id;
        let genes = best.genes.clone();

        let quantum = genes.first().copied().unwrap_or(20) + 5;
        let preempt = genes.get(1).copied().unwrap_or(50);
        let affinity = genes.get(2).copied().unwrap_or(30);

        let sid = fnv1a(label.as_bytes()) ^ xorshift64(&mut self.rng);
        let sched = NovelSchedule {
            schedule_id: sid,
            label: String::from(label),
            genome_id: gid,
            quantum_base: quantum,
            preempt_threshold: preempt,
            affinity_weight: affinity,
            effectiveness_ema: 50,
            times_applied: 0,
        };
        self.schedules.insert(sid, sched.clone());
        self.stats.total_schedules = self.schedules.len() as u64;
        self.count_innovation();
        Some(sched)
    }

    /// Mutate a genome to produce a novel variant.
    pub fn strategy_mutation(&mut self, genome_id: u64) -> Option<u64> {
        let parent = self.genomes.get(&genome_id)?.clone();
        self.generation += 1;

        let mut new_genes = parent.genes.clone();
        let mutations = (xorshift64(&mut self.rng) % 3) + 1;
        for _ in 0..mutations {
            if new_genes.is_empty() {
                break;
            }
            let idx = (xorshift64(&mut self.rng) as usize) % new_genes.len();
            let delta = xorshift64(&mut self.rng) % (MUTATION_RANGE * 2 + 1);
            let val = new_genes[idx];
            new_genes[idx] = val
                .wrapping_add(delta)
                .wrapping_sub(MUTATION_RANGE)
                .min(100);
        }

        let new_id = self.genome_hash(&new_genes);
        let novelty = self.compute_novelty(&new_genes);
        self.genomes.insert(new_id, StrategyGenome {
            genome_id: new_id,
            genes: new_genes,
            fitness: 0,
            novelty,
            generation: self.generation,
            parent_ids: alloc::vec![genome_id],
        });
        self.enforce_population_cap();
        self.stats.total_genomes = self.genomes.len() as u64;
        self.refresh_avg_novelty();
        Some(new_id)
    }

    /// Combine two genomes to produce an offspring.
    pub fn combination_invention(&mut self, genome_a: u64, genome_b: u64) -> Option<u64> {
        let ga = self.genomes.get(&genome_a)?.clone();
        let gb = self.genomes.get(&genome_b)?.clone();
        self.generation += 1;

        let max_len = ga.genes.len().max(gb.genes.len());
        let mut offspring_genes = Vec::with_capacity(max_len);
        for i in 0..max_len {
            let va = ga.genes.get(i).copied().unwrap_or(50);
            let vb = gb.genes.get(i).copied().unwrap_or(50);
            let pick = if xorshift64(&mut self.rng) % 2 == 0 { va } else { vb };
            offspring_genes.push(pick);
        }

        let oid = self.genome_hash(&offspring_genes);
        let novelty = self.compute_novelty(&offspring_genes);
        let predicted_fitness = (ga.fitness + gb.fitness) / 2;

        self.genomes.insert(oid, StrategyGenome {
            genome_id: oid,
            genes: offspring_genes,
            fitness: 0,
            novelty,
            generation: self.generation,
            parent_ids: alloc::vec![genome_a, genome_b],
        });

        if self.inventions.len() < MAX_INVENTIONS {
            self.inventions.push(CombinationInvention {
                invention_id: oid,
                parent_a: genome_a,
                parent_b: genome_b,
                offspring_genome_id: oid,
                predicted_fitness,
                actual_fitness: 0,
                tick: self.tick,
            });
            self.stats.total_inventions = self.inventions.len() as u64;
        }

        self.enforce_population_cap();
        self.stats.total_genomes = self.genomes.len() as u64;
        self.count_innovation();
        Some(oid)
    }

    /// Return the overall creativity metric (0–100).
    #[inline(always)]
    pub fn creativity_metric(&self) -> u64 {
        self.stats.creativity_score_ema
    }

    /// Return the innovation rate (innovations per tick, EMA-smoothed).
    #[inline(always)]
    pub fn innovation_rate(&self) -> u64 {
        self.stats.innovations_per_tick_ema
    }

    /// Record effectiveness of a creative allocation.
    #[inline]
    pub fn record_allocation_result(&mut self, allocation_id: u64, success: bool) {
        if let Some(a) = self.allocations.get_mut(&allocation_id) {
            a.times_applied += 1;
            let sample = if success { 100 } else { 0 };
            a.effectiveness_ema = ema_update(a.effectiveness_ema, sample);
        }
    }

    /// Record effectiveness of a novel schedule.
    #[inline]
    pub fn record_schedule_result(&mut self, schedule_id: u64, success: bool) {
        if let Some(s) = self.schedules.get_mut(&schedule_id) {
            s.times_applied += 1;
            let sample = if success { 100 } else { 0 };
            s.effectiveness_ema = ema_update(s.effectiveness_ema, sample);
        }
    }

    /// Advance tick and refresh innovation rate.
    #[inline]
    pub fn tick(&mut self) {
        self.tick += 1;
        self.stats.innovations_per_tick_ema =
            ema_update(self.stats.innovations_per_tick_ema, self.innovations_this_tick);
        self.innovations_this_tick = 0;
        self.refresh_creativity_score();
        self.stats.generation = self.generation;
    }

    /// Return current statistics.
    #[inline(always)]
    pub fn stats(&self) -> &CreativityStats {
        &self.stats
    }

    // -- internal -----------------------------------------------------------

    fn genome_hash(&mut self, genes: &[u64]) -> u64 {
        let mut buf = Vec::with_capacity(genes.len() * 8);
        for g in genes {
            buf.extend_from_slice(&g.to_le_bytes());
        }
        fnv1a(&buf) ^ xorshift64(&mut self.rng)
    }

    fn compute_novelty(&self, genes: &[u64]) -> u64 {
        if self.genomes.is_empty() {
            return 100;
        }
        let mut min_dist = u64::MAX;
        for existing in self.genomes.values() {
            let d = self.gene_distance(genes, &existing.genes);
            if d < min_dist {
                min_dist = d;
            }
        }
        min_dist.min(100)
    }

    fn gene_distance(&self, a: &[u64], b: &[u64]) -> u64 {
        let max_len = a.len().max(b.len());
        if max_len == 0 {
            return 0;
        }
        let mut total_diff: u64 = 0;
        for i in 0..max_len {
            let va = a.get(i).copied().unwrap_or(50);
            let vb = b.get(i).copied().unwrap_or(50);
            total_diff += if va > vb { va - vb } else { vb - va };
        }
        total_diff / max_len as u64
    }

    fn select_elite_genome(&self) -> Option<&StrategyGenome> {
        if self.genomes.is_empty() {
            return None;
        }
        // Return the genome with the best combined fitness + novelty score.
        self.genomes.values().max_by_key(|g| g.fitness + g.novelty / 2)
    }

    fn enforce_population_cap(&mut self) {
        while self.genomes.len() > MAX_STRATEGIES {
            // Remove worst genome.
            let worst_key = self
                .genomes
                .iter()
                .min_by_key(|(_, g)| g.fitness + g.novelty / 3)
                .map(|(k, _)| *k);
            if let Some(key) = worst_key {
                self.genomes.remove(&key);
            } else {
                break;
            }
        }
    }

    fn count_innovation(&mut self) {
        self.innovations_this_tick += 1;
    }

    fn refresh_avg_fitness(&mut self) {
        if self.genomes.is_empty() {
            return;
        }
        let sum: u64 = self.genomes.values().map(|g| g.fitness).sum();
        let avg = sum / self.genomes.len() as u64;
        self.stats.avg_fitness_ema = ema_update(self.stats.avg_fitness_ema, avg);
    }

    fn refresh_avg_novelty(&mut self) {
        if self.genomes.is_empty() {
            return;
        }
        let sum: u64 = self.genomes.values().map(|g| g.novelty).sum();
        let avg = sum / self.genomes.len() as u64;
        self.stats.avg_novelty_ema = ema_update(self.stats.avg_novelty_ema, avg);
    }

    fn refresh_creativity_score(&mut self) {
        let fitness_part = self.stats.avg_fitness_ema / 3;
        let novelty_part = self.stats.avg_novelty_ema / 3;
        let innovation_part = self.stats.innovations_per_tick_ema.min(30);
        let score = (fitness_part + novelty_part + innovation_part).min(100);
        self.stats.creativity_score_ema = ema_update(self.stats.creativity_score_ema, score);
    }
}
