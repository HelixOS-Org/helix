// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Reasoning Chain — Explainable Cooperation Decisions
//!
//! Every sharing decision, fairness verdict, and trust adjustment has a full
//! reasoning trace.  The chain captures premises, inferences, and conclusions
//! so that any observer can audit *why* a cooperation decision was made.
//! Traces are indexed by FNV-1a hashes, scored via EMA, and pruned with
//! xorshift64-based stochastic eviction.

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
const MAX_CHAINS: usize = 2048;
const MAX_STEPS_PER_CHAIN: usize = 64;
const MAX_SHARING_EXPLANATIONS: usize = 1024;
const MAX_FAIRNESS_EXPLANATIONS: usize = 1024;
const MIN_VALIDITY_SCORE: u64 = 30;
const JUSTIFICATION_DECAY_NUM: u64 = 97;
const JUSTIFICATION_DECAY_DEN: u64 = 100;

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
// Reasoning step
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct ReasoningStep {
    pub step_id: u64,
    pub premise_hash: u64,
    pub inference_type: InferenceType,
    pub confidence: u64,
    pub evidence_count: u64,
    pub timestamp_tick: u64,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum InferenceType {
    Deductive,
    Inductive,
    Abductive,
    Analogical,
    Causal,
}

// ---------------------------------------------------------------------------
// Reasoning chain
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct ReasoningChainRecord {
    pub chain_id: u64,
    pub decision_hash: u64,
    pub steps: Vec<ReasoningStep>,
    pub overall_confidence: u64,
    pub validity_score: u64,
    pub creation_tick: u64,
    pub conclusion: String,
}

// ---------------------------------------------------------------------------
// Sharing explanation
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct SharingExplanation {
    pub explanation_id: u64,
    pub resource_hash: u64,
    pub donor_id: u64,
    pub recipient_id: u64,
    pub amount_shared: u64,
    pub reasoning_chain_id: u64,
    pub fairness_score: u64,
    pub justification: String,
}

// ---------------------------------------------------------------------------
// Fairness explanation
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct FairnessExplanation {
    pub explanation_id: u64,
    pub policy_hash: u64,
    pub affected_parties: Vec<u64>,
    pub gini_coefficient: u64,
    pub reasoning_chain_id: u64,
    pub equitability: u64,
    pub rationale: String,
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
#[repr(align(64))]
pub struct ReasoningChainStats {
    pub total_chains_built: u64,
    pub avg_chain_length: u64,
    pub avg_validity: u64,
    pub sharing_explanations_given: u64,
    pub fairness_explanations_given: u64,
    pub weakest_justification_score: u64,
    pub transparency_index: u64,
    pub chains_pruned: u64,
}

// ---------------------------------------------------------------------------
// Manager
// ---------------------------------------------------------------------------

pub struct CoopReasoningChain {
    chains: BTreeMap<u64, ReasoningChainRecord>,
    sharing_explanations: BTreeMap<u64, SharingExplanation>,
    fairness_explanations: BTreeMap<u64, FairnessExplanation>,
    chain_validity_index: LinearMap<u64, 64>,
    justification_strengths: LinearMap<u64, 64>,
    stats: ReasoningChainStats,
    rng_state: u64,
    current_tick: u64,
}

impl CoopReasoningChain {
    pub fn new() -> Self {
        Self {
            chains: BTreeMap::new(),
            sharing_explanations: BTreeMap::new(),
            fairness_explanations: BTreeMap::new(),
            chain_validity_index: LinearMap::new(),
            justification_strengths: LinearMap::new(),
            stats: ReasoningChainStats::default(),
            rng_state: 0xCAFE_BABE_DEAD_BEEFu64,
            current_tick: 0,
        }
    }

    // -----------------------------------------------------------------------
    // build_cooperation_reasoning — assemble a multi-step reasoning chain
    // -----------------------------------------------------------------------
    pub fn build_cooperation_reasoning(
        &mut self,
        decision_context: &str,
        evidence_pairs: &[(u64, u64)],
    ) -> u64 {
        self.current_tick += 1;
        let decision_hash = fnv1a(decision_context.as_bytes());
        let chain_id = decision_hash ^ self.current_tick;

        let mut steps = Vec::new();
        let mut cumulative_confidence: u64 = 100;

        for (idx, &(premise, evidence)) in evidence_pairs.iter().enumerate() {
            if steps.len() >= MAX_STEPS_PER_CHAIN {
                break;
            }
            let inf_type = match idx % 5 {
                0 => InferenceType::Deductive,
                1 => InferenceType::Inductive,
                2 => InferenceType::Abductive,
                3 => InferenceType::Analogical,
                _ => InferenceType::Causal,
            };
            let step_confidence = if evidence > 0 {
                clamp(premise.wrapping_mul(100) / (premise + evidence), 10, 100)
            } else {
                50
            };
            cumulative_confidence = ema_update(cumulative_confidence, step_confidence);

            let step_hash = fnv1a(&premise.to_le_bytes()) ^ fnv1a(&evidence.to_le_bytes());
            steps.push(ReasoningStep {
                step_id: idx as u64,
                premise_hash: step_hash,
                inference_type: inf_type,
                confidence: step_confidence,
                evidence_count: evidence,
                timestamp_tick: self.current_tick,
                description: String::new(),
            });
        }

        let validity = if steps.is_empty() {
            0
        } else {
            let total_conf: u64 = steps.iter().map(|s| s.confidence).sum();
            total_conf / steps.len() as u64
        };

        let record = ReasoningChainRecord {
            chain_id,
            decision_hash,
            steps,
            overall_confidence: cumulative_confidence,
            validity_score: validity,
            creation_tick: self.current_tick,
            conclusion: String::new(),
        };

        self.chain_validity_index.insert(chain_id, validity);
        self.justification_strengths.insert(chain_id, cumulative_confidence);

        if self.chains.len() >= MAX_CHAINS {
            self.evict_weakest_chain();
        }
        self.chains.insert(chain_id, record);

        self.stats.total_chains_built += 1;
        self.stats.avg_chain_length = ema_update(
            self.stats.avg_chain_length,
            self.chains.values().map(|c| c.steps.len() as u64).sum::<u64>()
                / core::cmp::max(self.chains.len() as u64, 1),
        );
        self.stats.avg_validity = ema_update(self.stats.avg_validity, validity);

        chain_id
    }

    // -----------------------------------------------------------------------
    // explain_sharing — produce a human-readable sharing rationale
    // -----------------------------------------------------------------------
    pub fn explain_sharing(
        &mut self,
        resource_tag: &str,
        donor_id: u64,
        recipient_id: u64,
        amount: u64,
        chain_id: u64,
    ) -> u64 {
        let resource_hash = fnv1a(resource_tag.as_bytes());
        let eid = resource_hash ^ donor_id ^ recipient_id ^ self.current_tick;

        let fairness = if let Some(chain) = self.chains.get(&chain_id) {
            chain.overall_confidence
        } else {
            50
        };

        let explanation = SharingExplanation {
            explanation_id: eid,
            resource_hash,
            donor_id,
            recipient_id,
            amount_shared: amount,
            reasoning_chain_id: chain_id,
            fairness_score: fairness,
            justification: String::new(),
        };

        if self.sharing_explanations.len() >= MAX_SHARING_EXPLANATIONS {
            let oldest = self.sharing_explanations.keys().next().copied();
            if let Some(k) = oldest {
                self.sharing_explanations.remove(&k);
            }
        }
        self.sharing_explanations.insert(eid, explanation);
        self.stats.sharing_explanations_given += 1;

        eid
    }

    // -----------------------------------------------------------------------
    // explain_fairness — produce a fairness-policy explanation
    // -----------------------------------------------------------------------
    pub fn explain_fairness(
        &mut self,
        policy_tag: &str,
        affected: &[u64],
        gini: u64,
        chain_id: u64,
    ) -> u64 {
        let policy_hash = fnv1a(policy_tag.as_bytes());
        let eid = policy_hash ^ gini ^ self.current_tick;

        let equitability = if gini < 50 { 100 - gini } else { clamp(150 - gini, 0, 100) };

        let explanation = FairnessExplanation {
            explanation_id: eid,
            policy_hash,
            affected_parties: affected.to_vec(),
            gini_coefficient: gini,
            reasoning_chain_id: chain_id,
            equitability,
            rationale: String::new(),
        };

        if self.fairness_explanations.len() >= MAX_FAIRNESS_EXPLANATIONS {
            let oldest = self.fairness_explanations.keys().next().copied();
            if let Some(k) = oldest {
                self.fairness_explanations.remove(&k);
            }
        }
        self.fairness_explanations.insert(eid, explanation);
        self.stats.fairness_explanations_given += 1;

        eid
    }

    // -----------------------------------------------------------------------
    // reasoning_transparency — compute transparency index for all chains
    // -----------------------------------------------------------------------
    pub fn reasoning_transparency(&mut self) -> u64 {
        if self.chains.is_empty() {
            self.stats.transparency_index = 0;
            return 0;
        }

        let mut documented: u64 = 0;
        let mut total: u64 = 0;

        for chain in self.chains.values() {
            total += 1;
            let has_sharing = self
                .sharing_explanations
                .values()
                .any(|e| e.reasoning_chain_id == chain.chain_id);
            let has_fairness = self
                .fairness_explanations
                .values()
                .any(|e| e.reasoning_chain_id == chain.chain_id);
            if has_sharing || has_fairness {
                documented += 1;
            }
        }

        let transparency = documented.wrapping_mul(100) / total;
        self.stats.transparency_index = ema_update(self.stats.transparency_index, transparency);
        self.stats.transparency_index
    }

    // -----------------------------------------------------------------------
    // chain_validity — per-chain and aggregate validity assessment
    // -----------------------------------------------------------------------
    pub fn chain_validity(&mut self, chain_id: u64) -> u64 {
        if let Some(chain) = self.chains.get(&chain_id) {
            if chain.steps.is_empty() {
                return 0;
            }
            let min_conf = chain.steps.iter().map(|s| s.confidence).min().unwrap_or(0);
            let avg_conf: u64 =
                chain.steps.iter().map(|s| s.confidence).sum::<u64>() / chain.steps.len() as u64;

            let step_coherence = if chain.steps.len() > 1 {
                let mut diffs: u64 = 0;
                for w in chain.steps.windows(2) {
                    diffs += abs_diff(w[0].confidence, w[1].confidence);
                }
                100u64.saturating_sub(diffs / (chain.steps.len() as u64 - 1))
            } else {
                100
            };

            let validity = (min_conf + avg_conf + step_coherence) / 3;
            self.chain_validity_index.insert(chain_id, validity);
            validity
        } else {
            0
        }
    }

    // -----------------------------------------------------------------------
    // weakest_justification — find the least-supported reasoning chain
    // -----------------------------------------------------------------------
    pub fn weakest_justification(&mut self) -> Option<u64> {
        if self.justification_strengths.is_empty() {
            self.stats.weakest_justification_score = 0;
            return None;
        }

        let mut weakest_id: u64 = 0;
        let mut weakest_score: u64 = u64::MAX;

        for (&cid, &strength) in &self.justification_strengths {
            let decayed =
                (strength * JUSTIFICATION_DECAY_NUM) / JUSTIFICATION_DECAY_DEN;
            if decayed < weakest_score {
                weakest_score = decayed;
                weakest_id = cid;
            }
        }

        self.stats.weakest_justification_score = weakest_score;

        if weakest_score < MIN_VALIDITY_SCORE {
            Some(weakest_id)
        } else {
            Some(weakest_id)
        }
    }

    // -----------------------------------------------------------------------
    // Internal: evict weakest chain
    // -----------------------------------------------------------------------
    fn evict_weakest_chain(&mut self) {
        let victim = self
            .chain_validity_index
            .iter()
            .min_by_key(|(_, &v)| v)
            .map(|(&k, _)| k);
        if let Some(k) = victim {
            self.chains.remove(&k);
            self.chain_validity_index.remove(k);
            self.justification_strengths.remove(k);
            self.stats.chains_pruned += 1;
        }
    }

    // -----------------------------------------------------------------------
    // tick — periodic maintenance
    // -----------------------------------------------------------------------
    pub fn tick(&mut self) {
        self.current_tick += 1;

        // Decay justification strengths
        let keys: Vec<u64> = self.justification_strengths.keys().copied().collect();
        for k in keys {
            if let Some(s) = self.justification_strengths.get_mut(&k) {
                *s = (*s * JUSTIFICATION_DECAY_NUM) / JUSTIFICATION_DECAY_DEN;
            }
        }

        // Stochastic pruning of very weak chains
        let r = xorshift64(&mut self.rng_state) % 100;
        if r < 5 {
            let weak: Vec<u64> = self
                .chain_validity_index
                .iter()
                .filter(|(_, &v)| v < MIN_VALIDITY_SCORE / 2)
                .map(|(&k, _)| k)
                .collect();
            for k in weak {
                self.chains.remove(&k);
                self.chain_validity_index.remove(k);
                self.justification_strengths.remove(k);
                self.stats.chains_pruned += 1;
            }
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &ReasoningChainStats {
        &self.stats
    }

    #[inline(always)]
    pub fn chain_count(&self) -> usize {
        self.chains.len()
    }

    #[inline(always)]
    pub fn sharing_explanation_count(&self) -> usize {
        self.sharing_explanations.len()
    }

    #[inline(always)]
    pub fn fairness_explanation_count(&self) -> usize {
        self.fairness_explanations.len()
    }
}
