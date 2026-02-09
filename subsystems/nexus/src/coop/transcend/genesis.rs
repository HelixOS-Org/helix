// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Genesis — Creation of New Cooperation Capabilities
//!
//! Creates entirely new cooperation capabilities from scratch.  This module
//! births novel fairness algorithms, trust mechanisms, and negotiation
//! strategies — each scored, evolved, and archived for the kernel to adopt.

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
const MAX_ALGORITHMS: usize = 512;
const MAX_MECHANISMS: usize = 512;
const MAX_STRATEGIES: usize = 512;
const MAX_EVENTS: usize = 2048;
const EVOLUTION_GENERATIONS: usize = 32;

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
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}

fn abs_diff(a: u64, b: u64) -> u64 {
    if a > b { a - b } else { b - a }
}

// ---------------------------------------------------------------------------
// Capability type
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum CapabilityType {
    Fairness,
    Trust,
    Negotiation,
}

// ---------------------------------------------------------------------------
// Fairness algorithm
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct FairnessAlgorithm {
    pub algo_id: u64,
    pub weights: Vec<u64>,
    pub fairness_score: u64,
    pub efficiency_score: u64,
    pub generation: u64,
}

// ---------------------------------------------------------------------------
// Trust mechanism
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct TrustMechanism {
    pub mechanism_id: u64,
    pub trust_params: Vec<u64>,
    pub robustness_score: u64,
    pub convergence_speed: u64,
    pub generation: u64,
}

// ---------------------------------------------------------------------------
// Negotiation strategy
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct NegotiationStrategy {
    pub strategy_id: u64,
    pub tactics: Vec<u64>,
    pub win_rate: u64,
    pub mutual_gain: u64,
    pub generation: u64,
}

// ---------------------------------------------------------------------------
// Genesis event
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct GenesisEvent {
    pub event_id: u64,
    pub capability_type: CapabilityType,
    pub capability_id: u64,
    pub score: u64,
    pub generation: u64,
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
#[repr(align(64))]
pub struct GenesisStats {
    pub total_algorithms: usize,
    pub total_mechanisms: usize,
    pub total_strategies: usize,
    pub total_events: usize,
    pub best_fairness: u64,
    pub best_trust: u64,
    pub best_negotiation: u64,
    pub creation_rate_ema: u64,
    pub generation: u64,
}

// ---------------------------------------------------------------------------
// CoopGenesis
// ---------------------------------------------------------------------------

pub struct CoopGenesis {
    algorithms: BTreeMap<u64, FairnessAlgorithm>,
    mechanisms: BTreeMap<u64, TrustMechanism>,
    strategies: BTreeMap<u64, NegotiationStrategy>,
    events: BTreeMap<u64, GenesisEvent>,
    rng_state: u64,
    generation: u64,
    creation_rate_ema: u64,
    stats: GenesisStats,
}

impl CoopGenesis {
    pub fn new(seed: u64) -> Self {
        Self {
            algorithms: BTreeMap::new(),
            mechanisms: BTreeMap::new(),
            strategies: BTreeMap::new(),
            events: BTreeMap::new(),
            rng_state: seed | 1,
            generation: 0,
            creation_rate_ema: 0,
            stats: GenesisStats {
                total_algorithms: 0,
                total_mechanisms: 0,
                total_strategies: 0,
                total_events: 0,
                best_fairness: 0,
                best_trust: 0,
                best_negotiation: 0,
                creation_rate_ema: 0,
                generation: 0,
            },
        }
    }

    // -- create fairness algo -----------------------------------------------

    #[inline]
    pub fn create_fairness_algo(&mut self, agent_count: u64) -> Option<FairnessAlgorithm> {
        if self.algorithms.len() >= MAX_ALGORITHMS {
            return None;
        }
        self.generation += 1;

        let param_count = clamp(agent_count, 2, 32) as usize;
        let mut best: Option<FairnessAlgorithm> = None;

        for _ in 0..EVOLUTION_GENERATIONS {
            let mut weights = Vec::with_capacity(param_count);
            for _ in 0..param_count {
                weights.push(xorshift64(&mut self.rng_state) % 100);
            }

            let fairness = self.evaluate_fairness(&weights);
            let efficiency = self.evaluate_efficiency(&weights);

            let candidate = FairnessAlgorithm {
                algo_id: self.hash_weights(&weights),
                weights,
                fairness_score: fairness,
                efficiency_score: efficiency,
                generation: self.generation,
            };

            match &best {
                None => best = Some(candidate),
                Some(b) => {
                    let b_combined = b.fairness_score + b.efficiency_score;
                    let c_combined = candidate.fairness_score + candidate.efficiency_score;
                    if c_combined > b_combined {
                        best = Some(candidate);
                    }
                },
            }
        }

        if let Some(ref algo) = best {
            self.algorithms.insert(algo.algo_id, algo.clone());
            self.record_event(CapabilityType::Fairness, algo.algo_id, algo.fairness_score);
            self.creation_rate_ema = ema_update(self.creation_rate_ema, 100);
            self.refresh_stats();
        }
        best
    }

    fn evaluate_fairness(&self, weights: &[u64]) -> u64 {
        if weights.is_empty() {
            return 0;
        }
        let n = weights.len() as u64;
        let sum: u64 = weights.iter().sum();
        let mean = sum / n;
        if mean == 0 {
            return 50;
        }
        let max_dev = weights
            .iter()
            .map(|&w| abs_diff(w, mean))
            .max()
            .unwrap_or(0);
        let uniformity = 100u64.saturating_sub(max_dev);

        let sum_sq: u64 = weights.iter().map(|&w| w * w).sum();
        let jain = if sum_sq > 0 {
            (sum * sum) / (n * sum_sq)
        } else {
            100
        };

        clamp((uniformity + jain) / 2, 0, 100)
    }

    fn evaluate_efficiency(&self, weights: &[u64]) -> u64 {
        let n = weights.len() as u64;
        if n == 0 {
            return 0;
        }
        let sum: u64 = weights.iter().sum();
        let utilization = clamp(sum / n, 0, 100);
        let non_zero = weights.iter().filter(|&&w| w > 0).count() as u64;
        let coverage = non_zero * 100 / n;
        clamp((utilization + coverage) / 2, 0, 100)
    }

    fn hash_weights(&mut self, weights: &[u64]) -> u64 {
        let seed = xorshift64(&mut self.rng_state);
        let bytes: Vec<u8> = weights
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .chain(seed.to_le_bytes().iter().copied())
            .collect();
        fnv1a(&bytes)
    }

    // -- birth trust mechanism ----------------------------------------------

    #[inline]
    pub fn birth_trust_mechanism(&mut self, interaction_types: u64) -> Option<TrustMechanism> {
        if self.mechanisms.len() >= MAX_MECHANISMS {
            return None;
        }
        self.generation += 1;

        let param_count = clamp(interaction_types, 2, 24) as usize;
        let mut best: Option<TrustMechanism> = None;

        for _ in 0..EVOLUTION_GENERATIONS {
            let mut params = Vec::with_capacity(param_count);
            for _ in 0..param_count {
                params.push(xorshift64(&mut self.rng_state) % 100);
            }

            let robustness = self.evaluate_robustness(&params);
            let convergence = self.evaluate_convergence(&params);

            let candidate = TrustMechanism {
                mechanism_id: self.hash_params(&params),
                trust_params: params,
                robustness_score: robustness,
                convergence_speed: convergence,
                generation: self.generation,
            };

            match &best {
                None => best = Some(candidate),
                Some(b) => {
                    let b_combined = b.robustness_score + b.convergence_speed;
                    let c_combined = candidate.robustness_score + candidate.convergence_speed;
                    if c_combined > b_combined {
                        best = Some(candidate);
                    }
                },
            }
        }

        if let Some(ref mech) = best {
            self.mechanisms.insert(mech.mechanism_id, mech.clone());
            self.record_event(
                CapabilityType::Trust,
                mech.mechanism_id,
                mech.robustness_score,
            );
            self.creation_rate_ema = ema_update(self.creation_rate_ema, 100);
            self.refresh_stats();
        }
        best
    }

    fn evaluate_robustness(&self, params: &[u64]) -> u64 {
        if params.is_empty() {
            return 0;
        }
        let n = params.len() as u64;
        let sum: u64 = params.iter().sum();
        let mean = sum / n;
        let variance = params
            .iter()
            .map(|&p| {
                let d = abs_diff(p, mean);
                d * d
            })
            .sum::<u64>()
            / n;

        let stability = 100u64.saturating_sub(variance / 50);
        let min_val = params.iter().copied().min().unwrap_or(0);
        let floor_strength = clamp(min_val, 0, 100);
        clamp((stability + floor_strength) / 2, 0, 100)
    }

    fn evaluate_convergence(&self, params: &[u64]) -> u64 {
        if params.len() < 2 {
            return 50;
        }
        let mut monotonic_count = 0u64;
        for w in params.windows(2) {
            if w[1] >= w[0] {
                monotonic_count += 1;
            }
        }
        let monotonicity = monotonic_count * 100 / (params.len() as u64 - 1);
        let last = params.last().copied().unwrap_or(0);
        let target_proximity = 100u64.saturating_sub(abs_diff(last, 70));
        clamp((monotonicity + target_proximity) / 2, 0, 100)
    }

    fn hash_params(&mut self, params: &[u64]) -> u64 {
        let seed = xorshift64(&mut self.rng_state);
        let bytes: Vec<u8> = params
            .iter()
            .flat_map(|p| p.to_le_bytes())
            .chain(seed.to_le_bytes().iter().copied())
            .collect();
        fnv1a(&bytes)
    }

    // -- novel negotiation --------------------------------------------------

    #[inline]
    pub fn novel_negotiation(&mut self, participant_count: u64) -> Option<NegotiationStrategy> {
        if self.strategies.len() >= MAX_STRATEGIES {
            return None;
        }
        self.generation += 1;

        let tactic_count = clamp(participant_count * 2, 4, 32) as usize;
        let mut best: Option<NegotiationStrategy> = None;

        for _ in 0..EVOLUTION_GENERATIONS {
            let mut tactics = Vec::with_capacity(tactic_count);
            for _ in 0..tactic_count {
                tactics.push(xorshift64(&mut self.rng_state) % 100);
            }

            let win_rate = self.evaluate_win_rate(&tactics);
            let mutual_gain = self.evaluate_mutual_gain(&tactics);

            let candidate = NegotiationStrategy {
                strategy_id: self.hash_tactics(&tactics),
                tactics,
                win_rate,
                mutual_gain,
                generation: self.generation,
            };

            match &best {
                None => best = Some(candidate),
                Some(b) => {
                    let b_combined = b.win_rate + b.mutual_gain * 2;
                    let c_combined = candidate.win_rate + candidate.mutual_gain * 2;
                    if c_combined > b_combined {
                        best = Some(candidate);
                    }
                },
            }
        }

        if let Some(ref strat) = best {
            self.strategies.insert(strat.strategy_id, strat.clone());
            self.record_event(
                CapabilityType::Negotiation,
                strat.strategy_id,
                strat.mutual_gain,
            );
            self.creation_rate_ema = ema_update(self.creation_rate_ema, 100);
            self.refresh_stats();
        }
        best
    }

    fn evaluate_win_rate(&self, tactics: &[u64]) -> u64 {
        if tactics.is_empty() {
            return 0;
        }
        let n = tactics.len() as u64;
        let aggressive = tactics.iter().filter(|&&t| t > 60).count() as u64;
        let cooperative = tactics.iter().filter(|&&t| t <= 60).count() as u64;
        let balance = 100u64.saturating_sub(abs_diff(aggressive, cooperative) * 100 / n);
        let avg = tactics.iter().sum::<u64>() / n;
        let strength = clamp(avg, 0, 100);
        clamp((balance + strength) / 2, 0, 100)
    }

    fn evaluate_mutual_gain(&self, tactics: &[u64]) -> u64 {
        if tactics.is_empty() {
            return 0;
        }
        let n = tactics.len() as u64;
        let cooperative = tactics.iter().filter(|&&t| t >= 30 && t <= 70).count() as u64;
        let cooperation_ratio = cooperative * 100 / n;
        let sum: u64 = tactics.iter().sum();
        let mean = sum / n;
        let variance = tactics
            .iter()
            .map(|&t| {
                let d = abs_diff(t, mean);
                d * d
            })
            .sum::<u64>()
            / n;
        let low_conflict = 100u64.saturating_sub(variance / 30);
        clamp((cooperation_ratio + low_conflict) / 2, 0, 100)
    }

    fn hash_tactics(&mut self, tactics: &[u64]) -> u64 {
        let seed = xorshift64(&mut self.rng_state);
        let bytes: Vec<u8> = tactics
            .iter()
            .flat_map(|t| t.to_le_bytes())
            .chain(seed.to_le_bytes().iter().copied())
            .collect();
        fnv1a(&bytes)
    }

    // -- genesis event (public) ---------------------------------------------

    #[inline(always)]
    pub fn genesis_event(&self, event_id: u64) -> Option<GenesisEvent> {
        self.events.get(&event_id).cloned()
    }

    // -- capability evolution -----------------------------------------------

    pub fn capability_evolution(&self, cap_type: CapabilityType) -> u64 {
        match cap_type {
            CapabilityType::Fairness => self
                .algorithms
                .values()
                .map(|a| a.fairness_score)
                .max()
                .unwrap_or(0),
            CapabilityType::Trust => self
                .mechanisms
                .values()
                .map(|m| m.robustness_score)
                .max()
                .unwrap_or(0),
            CapabilityType::Negotiation => self
                .strategies
                .values()
                .map(|s| s.mutual_gain)
                .max()
                .unwrap_or(0),
        }
    }

    // -- internal -----------------------------------------------------------

    fn record_event(&mut self, cap_type: CapabilityType, cap_id: u64, score: u64) {
        if self.events.len() >= MAX_EVENTS {
            if let Some(&first) = self.events.keys().next() {
                self.events.remove(&first);
            }
        }
        let eid = fnv1a(&[cap_id.to_le_bytes(), self.generation.to_le_bytes()].concat());
        let event = GenesisEvent {
            event_id: eid,
            capability_type: cap_type,
            capability_id: cap_id,
            score,
            generation: self.generation,
        };
        self.events.insert(eid, event);
    }

    fn refresh_stats(&mut self) {
        let bf = self
            .algorithms
            .values()
            .map(|a| a.fairness_score)
            .max()
            .unwrap_or(0);
        let bt = self
            .mechanisms
            .values()
            .map(|m| m.robustness_score)
            .max()
            .unwrap_or(0);
        let bn = self
            .strategies
            .values()
            .map(|s| s.mutual_gain)
            .max()
            .unwrap_or(0);

        self.stats = GenesisStats {
            total_algorithms: self.algorithms.len(),
            total_mechanisms: self.mechanisms.len(),
            total_strategies: self.strategies.len(),
            total_events: self.events.len(),
            best_fairness: bf,
            best_trust: bt,
            best_negotiation: bn,
            creation_rate_ema: self.creation_rate_ema,
            generation: self.generation,
        };
    }

    #[inline(always)]
    pub fn stats(&self) -> GenesisStats {
        self.stats.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_fairness_algo() {
        let mut genesis = CoopGenesis::new(42);
        let algo = genesis.create_fairness_algo(5);
        assert!(algo.is_some());
        let a = algo.unwrap();
        assert!(a.fairness_score > 0);
        assert!(a.efficiency_score > 0);
    }

    #[test]
    fn test_birth_trust_mechanism() {
        let mut genesis = CoopGenesis::new(7);
        let mech = genesis.birth_trust_mechanism(4);
        assert!(mech.is_some());
        let m = mech.unwrap();
        assert!(m.robustness_score > 0);
    }

    #[test]
    fn test_novel_negotiation() {
        let mut genesis = CoopGenesis::new(99);
        let strat = genesis.novel_negotiation(3);
        assert!(strat.is_some());
        let s = strat.unwrap();
        assert!(s.mutual_gain > 0);
    }

    #[test]
    fn test_capability_evolution() {
        let mut genesis = CoopGenesis::new(55);
        genesis.create_fairness_algo(5);
        genesis.birth_trust_mechanism(4);
        genesis.novel_negotiation(3);
        assert!(genesis.capability_evolution(CapabilityType::Fairness) > 0);
        assert!(genesis.capability_evolution(CapabilityType::Trust) > 0);
        assert!(genesis.capability_evolution(CapabilityType::Negotiation) > 0);
    }

    #[test]
    fn test_creation_rate() {
        let mut genesis = CoopGenesis::new(123);
        assert_eq!(genesis.stats().creation_rate_ema, 0);
        genesis.create_fairness_algo(3);
        assert!(genesis.stats().creation_rate_ema > 0);
    }
}
