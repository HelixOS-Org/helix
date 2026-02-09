// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Creativity — Novel Cooperation Inventions
//!
//! Generates genuinely novel fairness algorithms, negotiation protocols, and
//! trust mechanisms that have never been seen before.  Each invention is
//! scored for novelty and impact, evolved through xorshift64-guided mutation,
//! and archived via FNV-1a indexed registries.  Creativity is measured as the
//! distance from all known solutions in the invention space.

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
const MAX_FAIRNESS_INVENTIONS: usize = 512;
const MAX_NEGOTIATION_PROTOCOLS: usize = 512;
const MAX_TRUST_INNOVATIONS: usize = 512;
const MAX_INVENTIONS: usize = 2048;
const NOVELTY_THRESHOLD: u64 = 40;
const IMPACT_DECAY_NUM: u64 = 97;
const IMPACT_DECAY_DEN: u64 = 100;
const CREATIVITY_BASELINE: u64 = 50;

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

fn abs_diff(a: u64, b: u64) -> u64 {
    if a > b { a - b } else { b - a }
}

// ---------------------------------------------------------------------------
// Invention category
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum InventionCategory {
    FairnessAlgorithm,
    NegotiationProtocol,
    TrustMechanism,
    HybridInvention,
}

// ---------------------------------------------------------------------------
// Fairness invention
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct FairnessInvention {
    pub invention_id: u64,
    pub algorithm_hash: u64,
    pub equity_score: u64,
    pub novelty_score: u64,
    pub parameters: Vec<u64>,
    pub creation_tick: u64,
    pub description: String,
}

// ---------------------------------------------------------------------------
// Negotiation protocol
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct NegotiationProtocol {
    pub protocol_id: u64,
    pub protocol_hash: u64,
    pub rounds: u64,
    pub convergence_rate: u64,
    pub novelty_score: u64,
    pub strategy_params: Vec<u64>,
    pub creation_tick: u64,
}

// ---------------------------------------------------------------------------
// Trust innovation
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct TrustInnovation {
    pub innovation_id: u64,
    pub mechanism_hash: u64,
    pub robustness: u64,
    pub adaptability: u64,
    pub novelty_score: u64,
    pub trust_params: Vec<u64>,
    pub creation_tick: u64,
}

// ---------------------------------------------------------------------------
// Invention record (general)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct InventionRecord {
    pub id: u64,
    pub category: InventionCategory,
    pub novelty: u64,
    pub impact: u64,
    pub generation: u64,
    pub parent_ids: Vec<u64>,
    pub fingerprint: u64,
    pub creation_tick: u64,
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
#[repr(align(64))]
pub struct CreativityStats {
    pub total_inventions: u64,
    pub fairness_inventions: u64,
    pub negotiation_protocols: u64,
    pub trust_innovations: u64,
    pub avg_novelty: u64,
    pub avg_impact: u64,
    pub creativity_index: u64,
    pub inventions_pruned: u64,
}

// ---------------------------------------------------------------------------
// Manager
// ---------------------------------------------------------------------------

pub struct CoopCreativity {
    inventions: BTreeMap<u64, InventionRecord>,
    fairness_inventions: BTreeMap<u64, FairnessInvention>,
    negotiation_protocols: BTreeMap<u64, NegotiationProtocol>,
    trust_innovations: BTreeMap<u64, TrustInnovation>,
    novelty_archive: LinearMap<u64, 64>,
    impact_scores: LinearMap<u64, 64>,
    stats: CreativityStats,
    rng_state: u64,
    current_tick: u64,
}

impl CoopCreativity {
    pub fn new() -> Self {
        Self {
            inventions: BTreeMap::new(),
            fairness_inventions: BTreeMap::new(),
            negotiation_protocols: BTreeMap::new(),
            trust_innovations: BTreeMap::new(),
            novelty_archive: LinearMap::new(),
            impact_scores: LinearMap::new(),
            stats: CreativityStats::default(),
            rng_state: 0xFACE_CAFE_BEAD_DAD0u64,
            current_tick: 0,
        }
    }

    // -----------------------------------------------------------------------
    // creative_fairness — invent a new fairness algorithm
    // -----------------------------------------------------------------------
    pub fn creative_fairness(
        &mut self,
        seed_params: &[u64],
        equity_target: u64,
    ) -> u64 {
        self.current_tick += 1;

        // Generate novel parameters via mutation of seed
        let mut params = seed_params.to_vec();
        for p in params.iter_mut() {
            let r = xorshift64(&mut self.rng_state);
            let delta = r % 20;
            if r % 2 == 0 {
                *p = p.wrapping_add(delta);
            } else {
                *p = p.saturating_sub(delta);
            }
        }
        // Add a random new parameter for novelty
        params.push(xorshift64(&mut self.rng_state) % 100);

        let alg_hash = params.iter().fold(FNV_OFFSET, |acc, &p| {
            acc ^ fnv1a(&p.to_le_bytes())
        });
        let iid = alg_hash ^ self.current_tick;

        let novelty = self.compute_novelty(alg_hash);
        let equity = clamp(
            equity_target.wrapping_add(xorshift64(&mut self.rng_state) % 10).saturating_sub(5),
            0,
            100,
        );

        let invention = FairnessInvention {
            invention_id: iid,
            algorithm_hash: alg_hash,
            equity_score: equity,
            novelty_score: novelty,
            parameters: params,
            creation_tick: self.current_tick,
            description: String::new(),
        };

        if self.fairness_inventions.len() >= MAX_FAIRNESS_INVENTIONS {
            let oldest = self.fairness_inventions.keys().next().copied();
            if let Some(k) = oldest { self.fairness_inventions.remove(&k); }
        }
        self.fairness_inventions.insert(iid, invention);
        self.register_invention(iid, InventionCategory::FairnessAlgorithm, novelty);
        self.stats.fairness_inventions += 1;

        iid
    }

    // -----------------------------------------------------------------------
    // novel_negotiation — invent a new negotiation protocol
    // -----------------------------------------------------------------------
    pub fn novel_negotiation(
        &mut self,
        max_rounds: u64,
        strategy_seeds: &[u64],
    ) -> u64 {
        self.current_tick += 1;

        let mut strategy = strategy_seeds.to_vec();
        for s in strategy.iter_mut() {
            let r = xorshift64(&mut self.rng_state);
            *s = s.wrapping_add(r % 15).wrapping_mul(r % 3 + 1);
        }
        strategy.push(xorshift64(&mut self.rng_state) % 50);

        let proto_hash = strategy.iter().fold(FNV_OFFSET, |acc, &s| {
            acc ^ fnv1a(&s.to_le_bytes())
        });
        let pid = proto_hash ^ self.current_tick;

        let novelty = self.compute_novelty(proto_hash);
        let convergence = clamp(
            100u64.saturating_sub(max_rounds.wrapping_mul(3)),
            10,
            100,
        );

        let protocol = NegotiationProtocol {
            protocol_id: pid,
            protocol_hash: proto_hash,
            rounds: max_rounds,
            convergence_rate: convergence,
            novelty_score: novelty,
            strategy_params: strategy,
            creation_tick: self.current_tick,
        };

        if self.negotiation_protocols.len() >= MAX_NEGOTIATION_PROTOCOLS {
            let oldest = self.negotiation_protocols.keys().next().copied();
            if let Some(k) = oldest { self.negotiation_protocols.remove(&k); }
        }
        self.negotiation_protocols.insert(pid, protocol);
        self.register_invention(pid, InventionCategory::NegotiationProtocol, novelty);
        self.stats.negotiation_protocols += 1;

        pid
    }

    // -----------------------------------------------------------------------
    // trust_innovation — invent a new trust mechanism
    // -----------------------------------------------------------------------
    pub fn trust_innovation(
        &mut self,
        trust_seeds: &[u64],
        robustness_target: u64,
    ) -> u64 {
        self.current_tick += 1;

        let mut params = trust_seeds.to_vec();
        for p in params.iter_mut() {
            let r = xorshift64(&mut self.rng_state);
            let shift = r % 25;
            *p = if r % 3 == 0 {
                p.wrapping_add(shift)
            } else {
                p.saturating_sub(shift / 2)
            };
        }

        let mech_hash = params.iter().fold(FNV_OFFSET, |acc, &p| {
            acc ^ fnv1a(&p.to_le_bytes())
        });
        let tid = mech_hash ^ self.current_tick;

        let novelty = self.compute_novelty(mech_hash);
        let robustness = clamp(robustness_target, 10, 100);
        let adaptability = clamp(
            novelty.wrapping_mul(robustness) / 100,
            10,
            100,
        );

        let innovation = TrustInnovation {
            innovation_id: tid,
            mechanism_hash: mech_hash,
            robustness,
            adaptability,
            novelty_score: novelty,
            trust_params: params,
            creation_tick: self.current_tick,
        };

        if self.trust_innovations.len() >= MAX_TRUST_INNOVATIONS {
            let oldest = self.trust_innovations.keys().next().copied();
            if let Some(k) = oldest { self.trust_innovations.remove(&k); }
        }
        self.trust_innovations.insert(tid, innovation);
        self.register_invention(tid, InventionCategory::TrustMechanism, novelty);
        self.stats.trust_innovations += 1;

        tid
    }

    // -----------------------------------------------------------------------
    // cooperation_invention — generic invention combining multiple categories
    // -----------------------------------------------------------------------
    pub fn cooperation_invention(
        &mut self,
        parent_ids: &[u64],
    ) -> u64 {
        self.current_tick += 1;

        // Combine fingerprints of parents
        let mut combined_hash = FNV_OFFSET;
        for &pid in parent_ids {
            if let Some(rec) = self.inventions.get(&pid) {
                combined_hash ^= rec.fingerprint;
            }
        }
        combined_hash ^= xorshift64(&mut self.rng_state);
        let iid = combined_hash ^ self.current_tick;
        let novelty = self.compute_novelty(combined_hash);

        let record = InventionRecord {
            id: iid,
            category: InventionCategory::HybridInvention,
            novelty,
            impact: 0,
            generation: parent_ids.len() as u64,
            parent_ids: parent_ids.to_vec(),
            fingerprint: combined_hash,
            creation_tick: self.current_tick,
        };

        if self.inventions.len() >= MAX_INVENTIONS {
            self.evict_lowest_impact();
        }
        self.inventions.insert(iid, record);
        self.novelty_archive.insert(iid, novelty);
        self.impact_scores.insert(iid, 0);
        self.stats.total_inventions += 1;
        self.stats.avg_novelty = ema_update(self.stats.avg_novelty, novelty);

        iid
    }

    // -----------------------------------------------------------------------
    // creativity_assessment — overall creativity score
    // -----------------------------------------------------------------------
    pub fn creativity_assessment(&mut self) -> u64 {
        if self.inventions.is_empty() {
            self.stats.creativity_index = CREATIVITY_BASELINE;
            return CREATIVITY_BASELINE;
        }

        let avg_nov: u64 = self.novelty_archive.values().sum::<u64>()
            / core::cmp::max(self.novelty_archive.len() as u64, 1);

        let category_diversity = {
            let mut has_fairness = false;
            let mut has_negotiation = false;
            let mut has_trust = false;
            let mut has_hybrid = false;
            for rec in self.inventions.values() {
                match rec.category {
                    InventionCategory::FairnessAlgorithm => has_fairness = true,
                    InventionCategory::NegotiationProtocol => has_negotiation = true,
                    InventionCategory::TrustMechanism => has_trust = true,
                    InventionCategory::HybridInvention => has_hybrid = true,
                }
            }
            let count = has_fairness as u64 + has_negotiation as u64
                + has_trust as u64 + has_hybrid as u64;
            count * 25
        };

        let volume_bonus = clamp(self.inventions.len() as u64 / 10, 0, 20);
        let creativity = clamp(avg_nov + category_diversity + volume_bonus, 0, 200);

        self.stats.creativity_index = ema_update(self.stats.creativity_index, creativity);
        self.stats.creativity_index
    }

    // -----------------------------------------------------------------------
    // innovation_impact — measure the impact of a specific invention
    // -----------------------------------------------------------------------
    pub fn innovation_impact(&mut self, invention_id: u64, observed_benefit: u64) -> u64 {
        let current_impact = self.impact_scores.get(invention_id).copied().unwrap_or(0);
        let updated = ema_update(current_impact, observed_benefit);
        self.impact_scores.insert(invention_id, updated);

        if let Some(rec) = self.inventions.get_mut(&invention_id) {
            rec.impact = updated;
        }

        self.stats.avg_impact = if self.impact_scores.is_empty() {
            0
        } else {
            self.impact_scores.values().sum::<u64>()
                / self.impact_scores.len() as u64
        };

        updated
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn compute_novelty(&self, fingerprint: u64) -> u64 {
        if self.novelty_archive.is_empty() {
            return 100;
        }
        let mut min_dist: u64 = u64::MAX;
        for &existing_fp in self.novelty_archive.values() {
            let dist = abs_diff(fingerprint, existing_fp);
            if dist < min_dist {
                min_dist = dist;
            }
        }
        // Normalise distance to 0-100 range
        clamp(min_dist % 101, 0, 100)
    }

    fn register_invention(&mut self, id: u64, category: InventionCategory, novelty: u64) {
        let fingerprint = fnv1a(&id.to_le_bytes());
        let record = InventionRecord {
            id,
            category,
            novelty,
            impact: 0,
            generation: 0,
            parent_ids: Vec::new(),
            fingerprint,
            creation_tick: self.current_tick,
        };

        if self.inventions.len() >= MAX_INVENTIONS {
            self.evict_lowest_impact();
        }
        self.inventions.insert(id, record);
        self.novelty_archive.insert(id, novelty);
        self.impact_scores.insert(id, 0);
        self.stats.total_inventions += 1;
        self.stats.avg_novelty = ema_update(self.stats.avg_novelty, novelty);
    }

    fn evict_lowest_impact(&mut self) {
        let victim = self
            .impact_scores
            .iter()
            .min_by_key(|(_, &v)| v)
            .map(|(&k, _)| k);
        if let Some(k) = victim {
            self.inventions.remove(&k);
            self.novelty_archive.remove(k);
            self.impact_scores.remove(k);
            self.stats.inventions_pruned += 1;
        }
    }

    pub fn tick(&mut self) {
        self.current_tick += 1;

        // Decay impact scores
        let keys: Vec<u64> = self.impact_scores.keys().copied().collect();
        for k in keys {
            if let Some(v) = self.impact_scores.get_mut(&k) {
                *v = (*v * IMPACT_DECAY_NUM) / IMPACT_DECAY_DEN;
            }
        }

        // Stochastic pruning of low-novelty inventions
        let r = xorshift64(&mut self.rng_state) % 100;
        if r < 4 {
            let low: Vec<u64> = self
                .novelty_archive
                .iter()
                .filter(|(_, &n)| n < NOVELTY_THRESHOLD / 2)
                .map(|(&k, _)| k)
                .collect();
            for k in low {
                self.inventions.remove(&k);
                self.novelty_archive.remove(k);
                self.impact_scores.remove(k);
                self.stats.inventions_pruned += 1;
            }
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &CreativityStats {
        &self.stats
    }

    #[inline(always)]
    pub fn invention_count(&self) -> usize {
        self.inventions.len()
    }

    #[inline(always)]
    pub fn fairness_invention_count(&self) -> usize {
        self.fairness_inventions.len()
    }

    #[inline(always)]
    pub fn negotiation_protocol_count(&self) -> usize {
        self.negotiation_protocols.len()
    }

    #[inline(always)]
    pub fn trust_innovation_count(&self) -> usize {
        self.trust_innovations.len()
    }
}
