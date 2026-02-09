// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Ascension — Final Ascension of Cooperation Intelligence
//!
//! The ultimate cooperation state: perfect, self-sustaining cooperation that
//! transcends all conflict.  Tracks ascension levels, conflict-free zones,
//! autonomous cooperation regions, and divine fairness where every allocation
//! is Pareto-optimal and universally perceived as just.  Progress is measured
//! via EMA, indexed by FNV-1a, and stochastically refined via xorshift64.

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
const MAX_ASCENSION_RECORDS: usize = 512;
const MAX_CONFLICT_FREE_ZONES: usize = 1024;
const MAX_HARMONY_REGIONS: usize = 512;
const MAX_FAIRNESS_RECORDS: usize = 512;
const ASCENSION_THRESHOLD: u64 = 95;
const CONFLICT_FREE_THRESHOLD: u64 = 90;
const DIVINE_FAIRNESS_THRESHOLD: u64 = 95;
const PROGRESS_DECAY_NUM: u64 = 99;
const PROGRESS_DECAY_DEN: u64 = 100;

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
// Ascension tier
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum AscensionTier {
    Mortal,
    Elevated,
    Exalted,
    Transcendent,
    Divine,
}

// ---------------------------------------------------------------------------
// Ascension record
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct AscensionRecord {
    pub record_id: u64,
    pub subsystem_ids: Vec<u64>,
    pub tier: AscensionTier,
    pub ascension_score: u64,
    pub cooperation_depth: u64,
    pub conflict_residue: u64,
    pub creation_tick: u64,
}

// ---------------------------------------------------------------------------
// Conflict-free zone
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct ConflictFreeZone {
    pub zone_id: u64,
    pub member_ids: Vec<u64>,
    pub peace_duration_ticks: u64,
    pub stability_score: u64,
    pub cooperation_intensity: u64,
    pub creation_tick: u64,
}

// ---------------------------------------------------------------------------
// Harmony region — autonomous cooperation
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct HarmonyRegion {
    pub region_id: u64,
    pub participant_ids: Vec<u64>,
    pub autonomy_score: u64,
    pub self_regulation: u64,
    pub external_dependency: u64,
    pub harmony_level: u64,
    pub creation_tick: u64,
}

// ---------------------------------------------------------------------------
// Divine fairness record
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct DivineFairnessRecord {
    pub record_id: u64,
    pub allocation_hash: u64,
    pub party_ids: Vec<u64>,
    pub allocations: Vec<u64>,
    pub pareto_optimal: bool,
    pub perception_unanimity: u64,
    pub gini_coefficient: u64,
    pub creation_tick: u64,
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
#[repr(align(64))]
pub struct AscensionStats {
    pub total_ascension_assessments: u64,
    pub conflict_free_zones: u64,
    pub harmony_regions: u64,
    pub divine_fairness_achieved: u64,
    pub avg_ascension_score: u64,
    pub peak_ascension: u64,
    pub avg_peace_duration: u64,
    pub overall_progress: u64,
}

// ---------------------------------------------------------------------------
// Manager
// ---------------------------------------------------------------------------

pub struct CoopAscension {
    ascension_records: BTreeMap<u64, AscensionRecord>,
    conflict_free_zones: BTreeMap<u64, ConflictFreeZone>,
    harmony_regions: BTreeMap<u64, HarmonyRegion>,
    fairness_records: BTreeMap<u64, DivineFairnessRecord>,
    progress_index: LinearMap<u64, 64>,
    stats: AscensionStats,
    rng_state: u64,
    current_tick: u64,
}

impl CoopAscension {
    pub fn new() -> Self {
        Self {
            ascension_records: BTreeMap::new(),
            conflict_free_zones: BTreeMap::new(),
            harmony_regions: BTreeMap::new(),
            fairness_records: BTreeMap::new(),
            progress_index: LinearMap::new(),
            stats: AscensionStats::default(),
            rng_state: 0xA5CE_ND00_CAFE_BABEu64,
            current_tick: 0,
        }
    }

    // -----------------------------------------------------------------------
    // ascension_level — assess overall cooperation ascension level
    // -----------------------------------------------------------------------
    pub fn ascension_level(
        &mut self,
        subsystem_ids: &[u64],
        trust_scores: &[u64],
        fairness_scores: &[u64],
        conflict_history: &[u64],
    ) -> AscensionTier {
        self.current_tick += 1;

        let avg_trust = if trust_scores.is_empty() {
            0
        } else {
            trust_scores.iter().sum::<u64>() / trust_scores.len() as u64
        };

        let avg_fairness = if fairness_scores.is_empty() {
            0
        } else {
            fairness_scores.iter().sum::<u64>() / fairness_scores.len() as u64
        };

        let conflict_penalty = if conflict_history.is_empty() {
            0
        } else {
            let recent_conflicts: u64 = conflict_history.iter().sum();
            clamp(recent_conflicts / conflict_history.len() as u64, 0, 50)
        };

        let raw_score = avg_trust
            .wrapping_add(avg_fairness)
            .wrapping_mul(2)
            .saturating_sub(conflict_penalty.wrapping_mul(3));
        let score = clamp(raw_score / 4, 0, 100);

        let tier = if score >= ASCENSION_THRESHOLD {
            AscensionTier::Divine
        } else if score >= 85 {
            AscensionTier::Transcendent
        } else if score >= 70 {
            AscensionTier::Exalted
        } else if score >= 50 {
            AscensionTier::Elevated
        } else {
            AscensionTier::Mortal
        };

        let rid = subsystem_ids.iter().fold(FNV_OFFSET, |acc, &id| {
            acc ^ fnv1a(&id.to_le_bytes())
        }) ^ self.current_tick;

        let record = AscensionRecord {
            record_id: rid,
            subsystem_ids: subsystem_ids.to_vec(),
            tier: tier.clone(),
            ascension_score: score,
            cooperation_depth: avg_trust,
            conflict_residue: conflict_penalty,
            creation_tick: self.current_tick,
        };

        if self.ascension_records.len() >= MAX_ASCENSION_RECORDS {
            let oldest = self.ascension_records.keys().next().copied();
            if let Some(k) = oldest { self.ascension_records.remove(&k); }
        }
        self.ascension_records.insert(rid, record);
        self.progress_index.insert(rid, score);
        self.stats.total_ascension_assessments += 1;
        self.stats.avg_ascension_score = ema_update(self.stats.avg_ascension_score, score);
        if score > self.stats.peak_ascension {
            self.stats.peak_ascension = score;
        }

        tier
    }

    // -----------------------------------------------------------------------
    // conflict_free_state — establish a conflict-free cooperation zone
    // -----------------------------------------------------------------------
    pub fn conflict_free_state(
        &mut self,
        member_ids: &[u64],
        current_peace_ticks: u64,
        cooperation_intensity: u64,
    ) -> u64 {
        self.current_tick += 1;
        let zid = member_ids.iter().fold(FNV_OFFSET, |acc, &id| {
            acc ^ fnv1a(&id.to_le_bytes())
        }) ^ self.current_tick;

        let stability = if current_peace_ticks > 100 {
            clamp(
                80 + (current_peace_ticks - 100) / 10,
                80,
                100,
            )
        } else {
            clamp(current_peace_ticks * 80 / 100, 0, 80)
        };

        let zone = ConflictFreeZone {
            zone_id: zid,
            member_ids: member_ids.to_vec(),
            peace_duration_ticks: current_peace_ticks,
            stability_score: stability,
            cooperation_intensity,
            creation_tick: self.current_tick,
        };

        if self.conflict_free_zones.len() >= MAX_CONFLICT_FREE_ZONES {
            let oldest = self.conflict_free_zones.keys().next().copied();
            if let Some(k) = oldest { self.conflict_free_zones.remove(&k); }
        }
        self.conflict_free_zones.insert(zid, zone);
        self.stats.conflict_free_zones += 1;
        self.stats.avg_peace_duration =
            ema_update(self.stats.avg_peace_duration, current_peace_ticks);

        stability
    }

    // -----------------------------------------------------------------------
    // perfect_harmony — assess and create a self-sustaining harmony region
    // -----------------------------------------------------------------------
    pub fn perfect_harmony(
        &mut self,
        participant_ids: &[u64],
        cooperation_metrics: &[u64],
    ) -> u64 {
        self.current_tick += 1;
        let rid = participant_ids.iter().fold(FNV_OFFSET, |acc, &id| {
            acc ^ fnv1a(&id.to_le_bytes())
        }) ^ self.current_tick;

        let avg_metric = if cooperation_metrics.is_empty() {
            0
        } else {
            cooperation_metrics.iter().sum::<u64>() / cooperation_metrics.len() as u64
        };

        // Variance analysis: low variance = high harmony
        let variance = if cooperation_metrics.len() < 2 {
            0
        } else {
            let mean = avg_metric;
            cooperation_metrics
                .iter()
                .map(|&m| {
                    let d = abs_diff(m, mean);
                    d.wrapping_mul(d)
                })
                .sum::<u64>()
                / cooperation_metrics.len() as u64
        };

        let harmony = clamp(
            avg_metric.saturating_sub(variance.min(avg_metric)),
            0,
            100,
        );

        let autonomy = if harmony >= CONFLICT_FREE_THRESHOLD {
            clamp(harmony - 50, 30, 100)
        } else {
            clamp(harmony / 3, 0, 30)
        };

        let self_regulation = clamp(
            harmony.wrapping_mul(autonomy) / 100,
            0,
            100,
        );

        let external_dep = 100u64.saturating_sub(autonomy);

        let region = HarmonyRegion {
            region_id: rid,
            participant_ids: participant_ids.to_vec(),
            autonomy_score: autonomy,
            self_regulation,
            external_dependency: external_dep,
            harmony_level: harmony,
            creation_tick: self.current_tick,
        };

        if self.harmony_regions.len() >= MAX_HARMONY_REGIONS {
            let oldest = self.harmony_regions.keys().next().copied();
            if let Some(k) = oldest { self.harmony_regions.remove(&k); }
        }
        self.harmony_regions.insert(rid, region);
        self.stats.harmony_regions += 1;

        harmony
    }

    // -----------------------------------------------------------------------
    // autonomous_cooperation — measure cooperation that sustains itself
    // -----------------------------------------------------------------------
    pub fn autonomous_cooperation(&self) -> u64 {
        if self.harmony_regions.is_empty() {
            return 0;
        }

        let total_autonomy: u64 = self
            .harmony_regions
            .values()
            .map(|r| r.autonomy_score)
            .sum();
        let total_self_reg: u64 = self
            .harmony_regions
            .values()
            .map(|r| r.self_regulation)
            .sum();
        let count = self.harmony_regions.len() as u64;

        let avg_autonomy = total_autonomy / count;
        let avg_self_reg = total_self_reg / count;

        (avg_autonomy + avg_self_reg) / 2
    }

    // -----------------------------------------------------------------------
    // ascension_progress — track overall progress toward divine cooperation
    // -----------------------------------------------------------------------
    pub fn ascension_progress(&mut self) -> u64 {
        if self.progress_index.is_empty() {
            self.stats.overall_progress = 0;
            return 0;
        }

        let avg_score: u64 = self.progress_index.values().sum::<u64>()
            / self.progress_index.len() as u64;

        let conflict_free_bonus = clamp(self.stats.conflict_free_zones * 2, 0, 20);
        let harmony_bonus = clamp(self.stats.harmony_regions * 3, 0, 20);
        let fairness_bonus = clamp(self.stats.divine_fairness_achieved * 5, 0, 20);

        let progress = clamp(
            avg_score + conflict_free_bonus + harmony_bonus + fairness_bonus,
            0,
            200,
        );

        self.stats.overall_progress = ema_update(self.stats.overall_progress, progress);
        self.stats.overall_progress
    }

    // -----------------------------------------------------------------------
    // divine_fairness — allocate resources with Pareto-optimal fairness
    // -----------------------------------------------------------------------
    pub fn divine_fairness(
        &mut self,
        allocation_tag: &str,
        party_ids: &[u64],
        demands: &[u64],
        total_supply: u64,
    ) -> u64 {
        self.current_tick += 1;
        let alloc_hash = fnv1a(allocation_tag.as_bytes());
        let rid = alloc_hash ^ self.current_tick;

        let total_demand: u64 = demands.iter().sum();
        let mut allocations = Vec::new();

        if total_demand == 0 || party_ids.is_empty() {
            let equal_share = if party_ids.is_empty() {
                0
            } else {
                total_supply / party_ids.len() as u64
            };
            for _ in party_ids {
                allocations.push(equal_share);
            }
        } else {
            // Proportional allocation with fairness correction
            for &demand in demands {
                let proportion = demand.wrapping_mul(total_supply) / total_demand;
                allocations.push(proportion);
            }

            // Fairness correction: redistribute from over-served to under-served
            let mean_alloc = if allocations.is_empty() {
                0
            } else {
                allocations.iter().sum::<u64>() / allocations.len() as u64
            };

            for alloc in allocations.iter_mut() {
                if *alloc > mean_alloc.wrapping_mul(2) {
                    let excess = *alloc - mean_alloc;
                    *alloc -= excess / 3; // Redistribute a third of excess
                }
            }
        }

        // Check Pareto optimality (simplified: total allocation ≈ supply)
        let total_allocated: u64 = allocations.iter().sum();
        let pareto = abs_diff(total_allocated, total_supply) < total_supply / 20 + 1;

        // Gini coefficient
        let gini = if allocations.len() < 2 {
            0
        } else {
            let n = allocations.len() as u64;
            let mut sum_abs_diff: u64 = 0;
            for i in 0..allocations.len() {
                for j in 0..allocations.len() {
                    sum_abs_diff += abs_diff(allocations[i], allocations[j]);
                }
            }
            let mean = total_allocated / core::cmp::max(n, 1);
            if mean == 0 || n == 0 {
                0
            } else {
                sum_abs_diff / (2 * n * n * mean / 100)
            }
        };

        // Perception unanimity: how similar the allocations are to demands
        let unanimity = if demands.is_empty() || allocations.is_empty() {
            100
        } else {
            let mut satisfaction_sum: u64 = 0;
            for (alloc, &demand) in allocations.iter().zip(demands.iter()) {
                if demand == 0 {
                    satisfaction_sum += 100;
                } else {
                    satisfaction_sum += clamp(
                        alloc.wrapping_mul(100) / demand,
                        0,
                        100,
                    );
                }
            }
            satisfaction_sum / allocations.len() as u64
        };

        let record = DivineFairnessRecord {
            record_id: rid,
            allocation_hash: alloc_hash,
            party_ids: party_ids.to_vec(),
            allocations,
            pareto_optimal: pareto,
            perception_unanimity: unanimity,
            gini_coefficient: gini,
            creation_tick: self.current_tick,
        };

        if self.fairness_records.len() >= MAX_FAIRNESS_RECORDS {
            let oldest = self.fairness_records.keys().next().copied();
            if let Some(k) = oldest { self.fairness_records.remove(&k); }
        }
        self.fairness_records.insert(rid, record);
        if unanimity >= DIVINE_FAIRNESS_THRESHOLD && pareto {
            self.stats.divine_fairness_achieved += 1;
        }

        unanimity
    }

    // -----------------------------------------------------------------------
    // Maintenance
    // -----------------------------------------------------------------------

    pub fn tick(&mut self) {
        self.current_tick += 1;

        // Decay progress scores
        let keys: Vec<u64> = self.progress_index.keys().copied().collect();
        for k in keys {
            if let Some(v) = self.progress_index.get_mut(&k) {
                *v = (*v * PROGRESS_DECAY_NUM) / PROGRESS_DECAY_DEN;
            }
        }

        // Extend peace duration for active zones
        for zone in self.conflict_free_zones.values_mut() {
            zone.peace_duration_ticks += 1;
            zone.stability_score = clamp(
                zone.stability_score + 1,
                0,
                100,
            );
        }

        // Stochastic harmony boost
        let r = xorshift64(&mut self.rng_state) % 100;
        if r < 3 {
            if let Some((_, region)) = self.harmony_regions.iter_mut().next() {
                let boost = xorshift64(&mut self.rng_state) % 2;
                region.harmony_level = clamp(region.harmony_level + boost, 0, 100);
            }
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &AscensionStats {
        &self.stats
    }

    #[inline(always)]
    pub fn ascension_record_count(&self) -> usize {
        self.ascension_records.len()
    }

    #[inline(always)]
    pub fn conflict_free_zone_count(&self) -> usize {
        self.conflict_free_zones.len()
    }

    #[inline(always)]
    pub fn harmony_region_count(&self) -> usize {
        self.harmony_regions.len()
    }

    #[inline(always)]
    pub fn fairness_record_count(&self) -> usize {
        self.fairness_records.len()
    }
}
