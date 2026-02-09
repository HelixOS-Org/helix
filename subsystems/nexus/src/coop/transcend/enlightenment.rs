// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Enlightenment — Ultimate Understanding of Cooperation
//!
//! Achieves deep understanding of cooperation dynamics: harmony between all
//! subsystems, transcendence of conflict, enlightened fairness that all parties
//! perceive as just, and unity insights that dissolve adversarial boundaries.
//! Each enlightenment state is tracked via EMA, indexed through FNV-1a, and
//! stochastically explored with xorshift64-guided contemplation cycles.

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
const MAX_HARMONY_STATES: usize = 1024;
const MAX_CONFLICT_RECORDS: usize = 512;
const MAX_FAIRNESS_INSIGHTS: usize = 512;
const MAX_UNITY_RECORDS: usize = 256;
const HARMONY_THRESHOLD: u64 = 80;
const ENLIGHTENMENT_THRESHOLD: u64 = 90;
const TRANSCENDENCE_DECAY_NUM: u64 = 99;
const TRANSCENDENCE_DECAY_DEN: u64 = 100;

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
// Enlightenment level
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum EnlightenmentLevel {
    Nascent,
    Awakening,
    Understanding,
    Harmony,
    Transcendence,
}

// ---------------------------------------------------------------------------
// Harmony state
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
#[repr(align(64))]
pub struct HarmonyState {
    pub state_id: u64,
    pub subsystem_ids: Vec<u64>,
    pub harmony_score: u64,
    pub conflict_residue: u64,
    pub cooperation_depth: u64,
    pub level: EnlightenmentLevel,
    pub creation_tick: u64,
}

// ---------------------------------------------------------------------------
// Conflict transcendence record
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct ConflictTranscendence {
    pub record_id: u64,
    pub original_conflict_hash: u64,
    pub party_ids: Vec<u64>,
    pub pre_resolution_score: u64,
    pub post_resolution_score: u64,
    pub transcendence_quality: u64,
    pub method_used: String,
    pub creation_tick: u64,
}

// ---------------------------------------------------------------------------
// Fairness insight
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct FairnessInsight {
    pub insight_id: u64,
    pub policy_hash: u64,
    pub perceived_fairness: Vec<u64>,
    pub objective_fairness: u64,
    pub perception_gap: u64,
    pub enlightenment_bonus: u64,
    pub creation_tick: u64,
}

// ---------------------------------------------------------------------------
// Unity record
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct UnityRecord {
    pub unity_id: u64,
    pub participant_ids: Vec<u64>,
    pub cohesion: u64,
    pub shared_purpose_score: u64,
    pub boundary_dissolution: u64,
    pub creation_tick: u64,
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
#[repr(align(64))]
pub struct EnlightenmentStats {
    pub total_harmony_states: u64,
    pub conflicts_transcended: u64,
    pub fairness_insights_gained: u64,
    pub unity_records: u64,
    pub avg_harmony: u64,
    pub avg_transcendence: u64,
    pub enlightenment_index: u64,
    pub peak_harmony: u64,
}

// ---------------------------------------------------------------------------
// Manager
// ---------------------------------------------------------------------------

pub struct CoopEnlightenment {
    harmony_states: BTreeMap<u64, HarmonyState>,
    conflict_transcendences: BTreeMap<u64, ConflictTranscendence>,
    fairness_insights: BTreeMap<u64, FairnessInsight>,
    unity_records: BTreeMap<u64, UnityRecord>,
    harmony_index: LinearMap<u64, 64>,
    stats: EnlightenmentStats,
    rng_state: u64,
    current_tick: u64,
}

impl CoopEnlightenment {
    pub fn new() -> Self {
        Self {
            harmony_states: BTreeMap::new(),
            conflict_transcendences: BTreeMap::new(),
            fairness_insights: BTreeMap::new(),
            unity_records: BTreeMap::new(),
            harmony_index: LinearMap::new(),
            stats: EnlightenmentStats::default(),
            rng_state: 0xE1E1_ABAB_7F7F_0101u64,
            current_tick: 0,
        }
    }

    // -----------------------------------------------------------------------
    // cooperation_enlightenment — assess enlightenment level across subsystems
    // -----------------------------------------------------------------------
    pub fn cooperation_enlightenment(
        &mut self,
        subsystem_ids: &[u64],
        trust_scores: &[u64],
        cooperation_scores: &[u64],
    ) -> EnlightenmentLevel {
        self.current_tick += 1;

        let avg_trust = if trust_scores.is_empty() {
            0
        } else {
            trust_scores.iter().sum::<u64>() / trust_scores.len() as u64
        };

        let avg_coop = if cooperation_scores.is_empty() {
            0
        } else {
            cooperation_scores.iter().sum::<u64>() / cooperation_scores.len() as u64
        };

        // Trust variance — low variance means harmony
        let trust_var = if trust_scores.len() < 2 {
            0
        } else {
            let mean = avg_trust;
            trust_scores
                .iter()
                .map(|&t| {
                    let d = abs_diff(t, mean);
                    d.wrapping_mul(d)
                })
                .sum::<u64>()
                / trust_scores.len() as u64
        };

        let harmony_raw = avg_trust
            .wrapping_mul(2)
            .wrapping_add(avg_coop.wrapping_mul(2))
            .wrapping_add(100u64.saturating_sub(trust_var.min(100)));
        let harmony = clamp(harmony_raw / 5, 0, 100);

        let level = if harmony >= ENLIGHTENMENT_THRESHOLD {
            EnlightenmentLevel::Transcendence
        } else if harmony >= HARMONY_THRESHOLD {
            EnlightenmentLevel::Harmony
        } else if harmony >= 60 {
            EnlightenmentLevel::Understanding
        } else if harmony >= 40 {
            EnlightenmentLevel::Awakening
        } else {
            EnlightenmentLevel::Nascent
        };

        let sid = subsystem_ids.iter().fold(FNV_OFFSET, |acc, &id| {
            acc ^ fnv1a(&id.to_le_bytes())
        }) ^ self.current_tick;

        let conflict_residue = 100u64.saturating_sub(harmony);

        let state = HarmonyState {
            state_id: sid,
            subsystem_ids: subsystem_ids.to_vec(),
            harmony_score: harmony,
            conflict_residue,
            cooperation_depth: avg_coop,
            level: level.clone(),
            creation_tick: self.current_tick,
        };

        if self.harmony_states.len() >= MAX_HARMONY_STATES {
            let oldest = self.harmony_states.keys().next().copied();
            if let Some(k) = oldest { self.harmony_states.remove(&k); }
        }
        self.harmony_states.insert(sid, state);
        self.harmony_index.insert(sid, harmony);
        self.stats.total_harmony_states += 1;
        self.stats.avg_harmony = ema_update(self.stats.avg_harmony, harmony);
        if harmony > self.stats.peak_harmony {
            self.stats.peak_harmony = harmony;
        }

        level
    }

    // -----------------------------------------------------------------------
    // harmony_understanding — deep analysis of current harmony
    // -----------------------------------------------------------------------
    pub fn harmony_understanding(&self) -> u64 {
        if self.harmony_states.is_empty() {
            return 0;
        }

        let total: u64 = self.harmony_index.values().sum();
        let count = self.harmony_index.len() as u64;
        let avg = total / core::cmp::max(count, 1);

        // Check for universal harmony (all above threshold)
        let harmonious = self
            .harmony_index
            .values()
            .filter(|&&h| h >= HARMONY_THRESHOLD)
            .count() as u64;
        let harmony_rate = harmonious.wrapping_mul(100) / core::cmp::max(count, 1);

        (avg + harmony_rate) / 2
    }

    // -----------------------------------------------------------------------
    // conflict_transcendence — transform a conflict into cooperation
    // -----------------------------------------------------------------------
    pub fn conflict_transcendence(
        &mut self,
        conflict_tag: &str,
        party_ids: &[u64],
        pre_score: u64,
    ) -> u64 {
        self.current_tick += 1;
        let conflict_hash = fnv1a(conflict_tag.as_bytes());
        let rid = conflict_hash ^ self.current_tick;

        // Compute resolution through multi-phase approach
        let phase1_gain = clamp(xorshift64(&mut self.rng_state) % 20 + 10, 5, 30);
        let phase2_gain = if party_ids.len() > 2 {
            clamp(xorshift64(&mut self.rng_state) % 15 + 5, 3, 20)
        } else {
            clamp(xorshift64(&mut self.rng_state) % 25 + 10, 5, 35)
        };
        let phase3_contemplation = xorshift64(&mut self.rng_state) % 10;

        let post_score = clamp(
            pre_score + phase1_gain + phase2_gain + phase3_contemplation,
            pre_score,
            100,
        );
        let quality = post_score.saturating_sub(pre_score);

        let record = ConflictTranscendence {
            record_id: rid,
            original_conflict_hash: conflict_hash,
            party_ids: party_ids.to_vec(),
            pre_resolution_score: pre_score,
            post_resolution_score: post_score,
            transcendence_quality: quality,
            method_used: String::new(),
            creation_tick: self.current_tick,
        };

        if self.conflict_transcendences.len() >= MAX_CONFLICT_RECORDS {
            let oldest = self.conflict_transcendences.keys().next().copied();
            if let Some(k) = oldest { self.conflict_transcendences.remove(&k); }
        }
        self.conflict_transcendences.insert(rid, record);
        self.stats.conflicts_transcended += 1;
        self.stats.avg_transcendence = ema_update(self.stats.avg_transcendence, quality);

        quality
    }

    // -----------------------------------------------------------------------
    // enlightened_fairness — fairness that all parties perceive as just
    // -----------------------------------------------------------------------
    pub fn enlightened_fairness(
        &mut self,
        policy_tag: &str,
        party_perceptions: &[(u64, u64)], // (party_id, perceived_fairness)
    ) -> u64 {
        self.current_tick += 1;
        let policy_hash = fnv1a(policy_tag.as_bytes());
        let iid = policy_hash ^ self.current_tick;

        let perceptions: Vec<u64> = party_perceptions.iter().map(|&(_, p)| p).collect();
        let objective = if perceptions.is_empty() {
            50
        } else {
            perceptions.iter().sum::<u64>() / perceptions.len() as u64
        };

        let perception_gap = if perceptions.len() < 2 {
            0
        } else {
            let max_p = perceptions.iter().copied().max().unwrap_or(0);
            let min_p = perceptions.iter().copied().min().unwrap_or(0);
            abs_diff(max_p, min_p)
        };

        let enlightenment_bonus = if perception_gap < 10 {
            30
        } else if perception_gap < 25 {
            15
        } else {
            0
        };

        let insight = FairnessInsight {
            insight_id: iid,
            policy_hash,
            perceived_fairness: perceptions,
            objective_fairness: objective,
            perception_gap,
            enlightenment_bonus,
            creation_tick: self.current_tick,
        };

        if self.fairness_insights.len() >= MAX_FAIRNESS_INSIGHTS {
            let oldest = self.fairness_insights.keys().next().copied();
            if let Some(k) = oldest { self.fairness_insights.remove(&k); }
        }
        self.fairness_insights.insert(iid, insight);
        self.stats.fairness_insights_gained += 1;

        clamp(objective + enlightenment_bonus, 0, 100)
    }

    // -----------------------------------------------------------------------
    // unity_insight — dissolve adversarial boundaries
    // -----------------------------------------------------------------------
    pub fn unity_insight(
        &mut self,
        participant_ids: &[u64],
        shared_goal_score: u64,
    ) -> u64 {
        self.current_tick += 1;
        let uid = participant_ids.iter().fold(FNV_OFFSET, |acc, &id| {
            acc ^ fnv1a(&id.to_le_bytes())
        }) ^ self.current_tick;

        let cohesion = if participant_ids.len() < 2 {
            shared_goal_score
        } else {
            let diversity_bonus = clamp(participant_ids.len() as u64 * 5, 0, 30);
            clamp(shared_goal_score + diversity_bonus, 0, 100)
        };

        let boundary_dissolution = if cohesion >= HARMONY_THRESHOLD {
            clamp(cohesion - HARMONY_THRESHOLD + 50, 50, 100)
        } else {
            clamp(cohesion / 2, 0, 50)
        };

        let record = UnityRecord {
            unity_id: uid,
            participant_ids: participant_ids.to_vec(),
            cohesion,
            shared_purpose_score: shared_goal_score,
            boundary_dissolution,
            creation_tick: self.current_tick,
        };

        if self.unity_records.len() >= MAX_UNITY_RECORDS {
            let oldest = self.unity_records.keys().next().copied();
            if let Some(k) = oldest { self.unity_records.remove(&k); }
        }
        self.unity_records.insert(uid, record);
        self.stats.unity_records += 1;

        boundary_dissolution
    }

    // -----------------------------------------------------------------------
    // transcendent_cooperation — assess overall transcendence state
    // -----------------------------------------------------------------------
    pub fn transcendent_cooperation(&mut self) -> u64 {
        let harmony = self.harmony_understanding();
        let transcendence = self.stats.avg_transcendence;
        let fairness_depth = if self.fairness_insights.is_empty() {
            0
        } else {
            let total: u64 = self
                .fairness_insights
                .values()
                .map(|i| i.objective_fairness + i.enlightenment_bonus)
                .sum();
            total / self.fairness_insights.len() as u64
        };
        let unity = if self.unity_records.is_empty() {
            0
        } else {
            self.unity_records
                .values()
                .map(|u| u.boundary_dissolution)
                .sum::<u64>()
                / self.unity_records.len() as u64
        };

        let overall = (harmony + transcendence + fairness_depth + unity) / 4;
        self.stats.enlightenment_index = ema_update(self.stats.enlightenment_index, overall);
        self.stats.enlightenment_index
    }

    // -----------------------------------------------------------------------
    // Maintenance
    // -----------------------------------------------------------------------

    pub fn tick(&mut self) {
        self.current_tick += 1;

        // Decay harmony scores slowly
        let keys: Vec<u64> = self.harmony_index.keys().copied().collect();
        for k in keys {
            if let Some(v) = self.harmony_index.get_mut(&k) {
                *v = (*v * TRANSCENDENCE_DECAY_NUM) / TRANSCENDENCE_DECAY_DEN;
            }
        }

        // Contemplation cycle: stochastic harmony boost
        let r = xorshift64(&mut self.rng_state) % 100;
        if r < 5 {
            if let Some((&sid, harmony)) = self.harmony_index.iter_mut().next() {
                let boost = xorshift64(&mut self.rng_state) % 3;
                *harmony = clamp(*harmony + boost, 0, 100);
                if let Some(state) = self.harmony_states.get_mut(&sid) {
                    state.harmony_score = *harmony;
                }
            }
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &EnlightenmentStats {
        &self.stats
    }

    #[inline(always)]
    pub fn harmony_state_count(&self) -> usize {
        self.harmony_states.len()
    }

    #[inline(always)]
    pub fn conflict_transcendence_count(&self) -> usize {
        self.conflict_transcendences.len()
    }
}
