// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Abstraction — Dynamic Cooperation Pattern Discovery
//!
//! Discovers and reifies higher-order cooperation patterns: symbiotic groups
//! whose members always benefit together, resource pools that amortise scarcity,
//! and trust clusters that amplify mutual reliability.  Each abstraction is
//! scored for fitness, evolved via xorshift64-guided mutation, and indexed
//! through FNV-1a hashing for O(1) lookup.

extern crate alloc;

use crate::fast::linear_map::LinearMap;
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
const MAX_TRUST_CLUSTERS: usize = 512;
const MAX_RESOURCE_POOLS: usize = 512;
const MAX_SYMBIOTIC_GROUPS: usize = 512;
const MAX_ABSTRACTIONS: usize = 2048;
const MUTATION_RATE_PCT: u64 = 8;
const FITNESS_DECAY_NUM: u64 = 98;
const FITNESS_DECAY_DEN: u64 = 100;
const MIN_FITNESS: u64 = 10;

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
// Abstraction kind
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum AbstractionKind {
    TrustCluster,
    ResourcePool,
    SymbioticGroup,
    Hybrid,
}

// ---------------------------------------------------------------------------
// Trust cluster
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct TrustCluster {
    pub cluster_id: u64,
    pub member_ids: Vec<u64>,
    pub cohesion_score: u64,
    pub mutual_trust: u64,
    pub formation_tick: u64,
    pub interaction_count: u64,
}

// ---------------------------------------------------------------------------
// Resource pool
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
#[repr(align(64))]
pub struct ResourcePool {
    pub pool_id: u64,
    pub contributor_ids: Vec<u64>,
    pub total_capacity: u64,
    pub utilisation: u64,
    pub fairness_index: u64,
    pub creation_tick: u64,
}

// ---------------------------------------------------------------------------
// Symbiotic group
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct SymbioticGroup {
    pub group_id: u64,
    pub member_ids: Vec<u64>,
    pub mutualism_score: u64,
    pub benefit_symmetry: u64,
    pub survival_rate: u64,
    pub formation_tick: u64,
}

// ---------------------------------------------------------------------------
// Abstraction record
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct AbstractionRecord {
    pub abstraction_id: u64,
    pub kind: AbstractionKind,
    pub name_hash: u64,
    pub fitness: u64,
    pub generation: u64,
    pub members: Vec<u64>,
    pub creation_tick: u64,
    pub description: String,
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
#[repr(align(64))]
pub struct AbstractionStats {
    pub total_abstractions: u64,
    pub trust_clusters_discovered: u64,
    pub resource_pools_created: u64,
    pub symbiotic_groups_formed: u64,
    pub avg_fitness: u64,
    pub evolution_cycles: u64,
    pub abstractions_pruned: u64,
    pub best_fitness: u64,
}

// ---------------------------------------------------------------------------
// Manager
// ---------------------------------------------------------------------------

pub struct CoopAbstraction {
    abstractions: BTreeMap<u64, AbstractionRecord>,
    trust_clusters: BTreeMap<u64, TrustCluster>,
    resource_pools: BTreeMap<u64, ResourcePool>,
    symbiotic_groups: BTreeMap<u64, SymbioticGroup>,
    fitness_index: LinearMap<u64, 64>,
    stats: AbstractionStats,
    rng_state: u64,
    current_tick: u64,
}

impl CoopAbstraction {
    pub fn new() -> Self {
        Self {
            abstractions: BTreeMap::new(),
            trust_clusters: BTreeMap::new(),
            resource_pools: BTreeMap::new(),
            symbiotic_groups: BTreeMap::new(),
            fitness_index: LinearMap::new(),
            stats: AbstractionStats::default(),
            rng_state: 0xABCD_1234_5678_EF01u64,
            current_tick: 0,
        }
    }

    // -----------------------------------------------------------------------
    // create_cooperation_abstraction — generic abstraction factory
    // -----------------------------------------------------------------------
    pub fn create_cooperation_abstraction(
        &mut self,
        name: &str,
        kind: AbstractionKind,
        members: &[u64],
    ) -> u64 {
        self.current_tick += 1;
        let name_hash = fnv1a(name.as_bytes());
        let aid = name_hash ^ self.current_tick ^ (members.len() as u64);

        let initial_fitness = if members.is_empty() {
            MIN_FITNESS
        } else {
            let diversity = {
                let mut sorted = members.to_vec();
                sorted.sort_unstable();
                sorted.dedup();
                (sorted.len() as u64).wrapping_mul(100) / members.len() as u64
            };
            clamp(diversity, MIN_FITNESS, 100)
        };

        let record = AbstractionRecord {
            abstraction_id: aid,
            kind: kind.clone(),
            name_hash,
            fitness: initial_fitness,
            generation: 0,
            members: members.to_vec(),
            creation_tick: self.current_tick,
            description: String::new(),
        };

        if self.abstractions.len() >= MAX_ABSTRACTIONS {
            self.evict_weakest();
        }

        self.abstractions.insert(aid, record);
        self.fitness_index.insert(aid, initial_fitness);
        self.stats.total_abstractions += 1;
        self.stats.avg_fitness = ema_update(self.stats.avg_fitness, initial_fitness);
        if initial_fitness > self.stats.best_fitness {
            self.stats.best_fitness = initial_fitness;
        }

        aid
    }

    // -----------------------------------------------------------------------
    // discover_trust_cluster — identify a cluster of mutually trusting nodes
    // -----------------------------------------------------------------------
    pub fn discover_trust_cluster(
        &mut self,
        member_ids: &[u64],
        pairwise_trust: &[(u64, u64, u64)],
    ) -> u64 {
        self.current_tick += 1;
        let cluster_hash: u64 = member_ids.iter().fold(FNV_OFFSET, |acc, &m| {
            acc ^ fnv1a(&m.to_le_bytes())
        });
        let cid = cluster_hash ^ self.current_tick;

        let mutual_trust = if pairwise_trust.is_empty() {
            0
        } else {
            let total: u64 = pairwise_trust.iter().map(|&(_, _, t)| t).sum();
            total / pairwise_trust.len() as u64
        };

        let cohesion = if member_ids.len() < 2 {
            0
        } else {
            let expected_pairs = (member_ids.len() * (member_ids.len() - 1)) / 2;
            let observed = pairwise_trust.len();
            clamp(
                (observed as u64).wrapping_mul(100) / expected_pairs as u64,
                0,
                100,
            )
        };

        let cluster = TrustCluster {
            cluster_id: cid,
            member_ids: member_ids.to_vec(),
            cohesion_score: cohesion,
            mutual_trust,
            formation_tick: self.current_tick,
            interaction_count: pairwise_trust.len() as u64,
        };

        if self.trust_clusters.len() >= MAX_TRUST_CLUSTERS {
            let oldest = self.trust_clusters.keys().next().copied();
            if let Some(k) = oldest {
                self.trust_clusters.remove(&k);
            }
        }
        self.trust_clusters.insert(cid, cluster);
        self.stats.trust_clusters_discovered += 1;

        cid
    }

    // -----------------------------------------------------------------------
    // resource_pool_abstraction — create a pooled resource abstraction
    // -----------------------------------------------------------------------
    pub fn resource_pool_abstraction(
        &mut self,
        contributors: &[u64],
        capacities: &[u64],
    ) -> u64 {
        self.current_tick += 1;
        let pool_hash: u64 = contributors.iter().fold(FNV_OFFSET, |acc, &c| {
            acc ^ fnv1a(&c.to_le_bytes())
        });
        let pid = pool_hash ^ self.current_tick;

        let total_capacity: u64 = capacities.iter().sum();
        let fairness_index = if capacities.is_empty() || total_capacity == 0 {
            0
        } else {
            let mean = total_capacity / capacities.len() as u64;
            let variance: u64 = capacities
                .iter()
                .map(|&c| {
                    let d = if c > mean { c - mean } else { mean - c };
                    d.wrapping_mul(d)
                })
                .sum::<u64>()
                / capacities.len() as u64;
            let max_var = mean.wrapping_mul(mean);
            if max_var == 0 {
                100
            } else {
                100u64.saturating_sub(variance.wrapping_mul(100) / max_var)
            }
        };

        let pool = ResourcePool {
            pool_id: pid,
            contributor_ids: contributors.to_vec(),
            total_capacity,
            utilisation: 0,
            fairness_index,
            creation_tick: self.current_tick,
        };

        if self.resource_pools.len() >= MAX_RESOURCE_POOLS {
            let oldest = self.resource_pools.keys().next().copied();
            if let Some(k) = oldest {
                self.resource_pools.remove(&k);
            }
        }
        self.resource_pools.insert(pid, pool);
        self.stats.resource_pools_created += 1;

        pid
    }

    // -----------------------------------------------------------------------
    // symbiotic_group — form a symbiotic cooperation group
    // -----------------------------------------------------------------------
    pub fn symbiotic_group(
        &mut self,
        member_ids: &[u64],
        benefit_scores: &[u64],
    ) -> u64 {
        self.current_tick += 1;
        let group_hash: u64 = member_ids.iter().fold(FNV_OFFSET, |acc, &m| {
            acc ^ fnv1a(&m.to_le_bytes())
        });
        let gid = group_hash ^ self.current_tick;

        let mutualism = if benefit_scores.is_empty() {
            0
        } else {
            benefit_scores.iter().sum::<u64>() / benefit_scores.len() as u64
        };

        let symmetry = if benefit_scores.len() < 2 {
            100
        } else {
            let max_b = benefit_scores.iter().copied().max().unwrap_or(1);
            let min_b = benefit_scores.iter().copied().min().unwrap_or(0);
            if max_b == 0 { 100 } else { min_b.wrapping_mul(100) / max_b }
        };

        let group = SymbioticGroup {
            group_id: gid,
            member_ids: member_ids.to_vec(),
            mutualism_score: mutualism,
            benefit_symmetry: symmetry,
            survival_rate: 100,
            formation_tick: self.current_tick,
        };

        if self.symbiotic_groups.len() >= MAX_SYMBIOTIC_GROUPS {
            let oldest = self.symbiotic_groups.keys().next().copied();
            if let Some(k) = oldest {
                self.symbiotic_groups.remove(&k);
            }
        }
        self.symbiotic_groups.insert(gid, group);
        self.stats.symbiotic_groups_formed += 1;

        gid
    }

    // -----------------------------------------------------------------------
    // abstraction_fitness — evaluate and update fitness of an abstraction
    // -----------------------------------------------------------------------
    pub fn abstraction_fitness(&mut self, abstraction_id: u64) -> u64 {
        if let Some(record) = self.abstractions.get(&abstraction_id) {
            let member_count = record.members.len() as u64;
            let age = self.current_tick.saturating_sub(record.creation_tick);
            let age_bonus = clamp(age / 10, 0, 20);
            let size_bonus = clamp(member_count * 5, 0, 30);

            let kind_bonus = match record.kind {
                AbstractionKind::TrustCluster => 10,
                AbstractionKind::ResourcePool => 15,
                AbstractionKind::SymbioticGroup => 20,
                AbstractionKind::Hybrid => 25,
            };

            let raw_fitness = record.fitness + age_bonus + size_bonus + kind_bonus;
            let fitness = clamp(raw_fitness, MIN_FITNESS, 200);

            self.fitness_index.insert(abstraction_id, fitness);
            if fitness > self.stats.best_fitness {
                self.stats.best_fitness = fitness;
            }
            fitness
        } else {
            0
        }
    }

    // -----------------------------------------------------------------------
    // evolve_cooperation_model — genetic evolution of abstractions
    // -----------------------------------------------------------------------
    pub fn evolve_cooperation_model(&mut self) -> u64 {
        self.stats.evolution_cycles += 1;

        let ids: Vec<u64> = self.abstractions.keys().copied().collect();
        if ids.len() < 2 {
            return 0;
        }

        // Tournament selection
        let idx_a = xorshift64(&mut self.rng_state) as usize % ids.len();
        let idx_b = xorshift64(&mut self.rng_state) as usize % ids.len();
        let parent_a_id = ids[idx_a];
        let parent_b_id = ids[idx_b];

        let (child_members, child_kind) = {
            let pa = self.abstractions.get(&parent_a_id).cloned();
            let pb = self.abstractions.get(&parent_b_id).cloned();
            match (pa, pb) {
                (Some(a), Some(b)) => {
                    // Crossover: merge members from both parents
                    let mut merged = a.members.clone();
                    for m in &b.members {
                        if !merged.contains(m) {
                            merged.push(*m);
                        }
                    }
                    // Mutation: randomly drop a member
                    let r = xorshift64(&mut self.rng_state) % 100;
                    if r < MUTATION_RATE_PCT && !merged.is_empty() {
                        let drop_idx = xorshift64(&mut self.rng_state) as usize % merged.len();
                        merged.remove(drop_idx);
                    }
                    let kind = if a.kind == b.kind { a.kind.clone() } else { AbstractionKind::Hybrid };
                    (merged, kind)
                }
                _ => return 0,
            }
        };

        let child_id = self.create_cooperation_abstraction("evolved", child_kind, &child_members);

        if let Some(child) = self.abstractions.get_mut(&child_id) {
            let pa_gen = self.abstractions.get(&parent_a_id).map(|r| r.generation).unwrap_or(0);
            let pb_gen = self.abstractions.get(&parent_b_id).map(|r| r.generation).unwrap_or(0);
            let max_gen = core::cmp::max(pa_gen, pb_gen);
            child.generation = max_gen + 1;
        }

        child_id
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn evict_weakest(&mut self) {
        let victim = self
            .fitness_index
            .iter()
            .min_by_key(|(_, &f)| f)
            .map(|(&k, _)| k);
        if let Some(k) = victim {
            self.abstractions.remove(&k);
            self.fitness_index.remove(k);
            self.stats.abstractions_pruned += 1;
        }
    }

    pub fn tick(&mut self) {
        self.current_tick += 1;

        // Decay all fitness values
        let keys: Vec<u64> = self.fitness_index.keys().copied().collect();
        for k in keys {
            if let Some(f) = self.fitness_index.get_mut(&k) {
                *f = (*f * FITNESS_DECAY_NUM) / FITNESS_DECAY_DEN;
                if *f < MIN_FITNESS {
                    *f = MIN_FITNESS;
                }
            }
            if let Some(rec) = self.abstractions.get_mut(&k) {
                rec.fitness = (rec.fitness * FITNESS_DECAY_NUM) / FITNESS_DECAY_DEN;
                if rec.fitness < MIN_FITNESS {
                    rec.fitness = MIN_FITNESS;
                }
            }
        }

        // Decay symbiotic survival rates
        for group in self.symbiotic_groups.values_mut() {
            group.survival_rate = (group.survival_rate * FITNESS_DECAY_NUM) / FITNESS_DECAY_DEN;
        }

        // Stochastic pruning
        let r = xorshift64(&mut self.rng_state) % 100;
        if r < 3 {
            self.evict_weakest();
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &AbstractionStats {
        &self.stats
    }

    #[inline(always)]
    pub fn abstraction_count(&self) -> usize {
        self.abstractions.len()
    }

    #[inline(always)]
    pub fn trust_cluster_count(&self) -> usize {
        self.trust_clusters.len()
    }

    #[inline(always)]
    pub fn resource_pool_count(&self) -> usize {
        self.resource_pools.len()
    }

    #[inline(always)]
    pub fn symbiotic_group_count(&self) -> usize {
        self.symbiotic_groups.len()
    }
}
