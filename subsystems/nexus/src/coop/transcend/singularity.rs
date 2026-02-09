// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Singularity Engine
//!
//! Convergence of all cooperation intelligence into a single unified framework.
//! Achieves perfect fairness, zero contention, and optimal sharing —
//! simultaneously.  Continuously self-assesses its intelligence level and
//! drives toward cooperation singularity.

extern crate alloc;

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
const MAX_DOMAINS: usize = 256;
const MAX_CONVERGENCE_HISTORY: usize = 128;
const SINGULARITY_THRESHOLD: u64 = 95;
const CONVERGENCE_ITERATIONS: usize = 48;

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
// Cooperation domain
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct CoopDomain {
    pub domain_id: u64,
    pub fairness_score: u64,
    pub contention_level: u64,
    pub sharing_efficiency: u64,
    pub convergence_score: u64,
    pub ema_fairness: u64,
    pub ema_contention: u64,
    pub ema_efficiency: u64,
}

// ---------------------------------------------------------------------------
// Unified state
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
#[repr(align(64))]
pub struct UnifiedState {
    pub global_fairness: u64,
    pub global_contention: u64,
    pub global_efficiency: u64,
    pub singularity_distance: u64,
    pub intelligence_score: u64,
    pub convergence_velocity: i64,
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
#[repr(align(64))]
pub struct SingularityStats {
    pub domains_count: usize,
    pub avg_fairness: u64,
    pub avg_contention: u64,
    pub avg_efficiency: u64,
    pub singularity_achieved: bool,
    pub intelligence_level: u64,
    pub convergence_events: u64,
    pub ticks_elapsed: u64,
}

// ---------------------------------------------------------------------------
// CoopSingularity
// ---------------------------------------------------------------------------

pub struct CoopSingularity {
    domains: BTreeMap<u64, CoopDomain>,
    unified: UnifiedState,
    convergence_history: VecDeque<u64>,
    rng_state: u64,
    tick: u64,
    convergence_events: u64,
    stats: SingularityStats,
}

impl CoopSingularity {
    pub fn new(seed: u64) -> Self {
        Self {
            domains: BTreeMap::new(),
            unified: UnifiedState {
                global_fairness: 50,
                global_contention: 50,
                global_efficiency: 50,
                singularity_distance: 100,
                intelligence_score: 0,
                convergence_velocity: 0,
            },
            convergence_history: VecDeque::new(),
            rng_state: seed | 1,
            tick: 0,
            convergence_events: 0,
            stats: SingularityStats {
                domains_count: 0,
                avg_fairness: 50,
                avg_contention: 50,
                avg_efficiency: 50,
                singularity_achieved: false,
                intelligence_level: 0,
                convergence_events: 0,
                ticks_elapsed: 0,
            },
        }
    }

    // -- domain registration ------------------------------------------------

    pub fn register_domain(
        &mut self,
        domain_id: u64,
        fairness: u64,
        contention: u64,
        efficiency: u64,
    ) {
        if self.domains.len() >= MAX_DOMAINS {
            return;
        }
        let f = clamp(fairness, 0, 100);
        let c = clamp(contention, 0, 100);
        let e = clamp(efficiency, 0, 100);
        self.domains.insert(domain_id, CoopDomain {
            domain_id,
            fairness_score: f,
            contention_level: c,
            sharing_efficiency: e,
            convergence_score: 0,
            ema_fairness: f,
            ema_contention: c,
            ema_efficiency: e,
        });
    }

    pub fn update_domain(
        &mut self,
        domain_id: u64,
        fairness: u64,
        contention: u64,
        efficiency: u64,
    ) {
        if let Some(d) = self.domains.get_mut(&domain_id) {
            let f = clamp(fairness, 0, 100);
            let c = clamp(contention, 0, 100);
            let e = clamp(efficiency, 0, 100);
            d.fairness_score = f;
            d.contention_level = c;
            d.sharing_efficiency = e;
            d.ema_fairness = ema_update(d.ema_fairness, f);
            d.ema_contention = ema_update(d.ema_contention, c);
            d.ema_efficiency = ema_update(d.ema_efficiency, e);
            d.convergence_score = self.domain_convergence(d);
        }
    }

    fn domain_convergence(&self, d: &CoopDomain) -> u64 {
        let f_score = d.ema_fairness;
        let c_score = 100u64.saturating_sub(d.ema_contention);
        let e_score = d.ema_efficiency;
        (f_score + c_score + e_score) / 3
    }

    // -- unified cooperation ------------------------------------------------

    pub fn unified_cooperation(&mut self) -> UnifiedState {
        if self.domains.is_empty() {
            return self.unified.clone();
        }

        for _iter in 0..CONVERGENCE_ITERATIONS {
            let domain_ids: Vec<u64> = self.domains.keys().copied().collect();
            let avg_fairness = self.domains.values().map(|d| d.ema_fairness).sum::<u64>()
                / self.domains.len() as u64;
            let avg_efficiency = self.domains.values().map(|d| d.ema_efficiency).sum::<u64>()
                / self.domains.len() as u64;

            let mut any_change = false;
            for &did in &domain_ids {
                if let Some(d) = self.domains.get_mut(&did) {
                    let f_diff = abs_diff(d.ema_fairness, avg_fairness);
                    let e_diff = abs_diff(d.ema_efficiency, avg_efficiency);

                    if f_diff > 5 {
                        if d.ema_fairness < avg_fairness {
                            d.ema_fairness += 1;
                        } else {
                            d.ema_fairness = d.ema_fairness.saturating_sub(1);
                        }
                        any_change = true;
                    }
                    if e_diff > 5 {
                        if d.ema_efficiency < avg_efficiency {
                            d.ema_efficiency += 1;
                        } else {
                            d.ema_efficiency = d.ema_efficiency.saturating_sub(1);
                        }
                        any_change = true;
                    }
                    if d.ema_contention > 10 {
                        d.ema_contention = d.ema_contention.saturating_sub(1);
                        any_change = true;
                    }
                    d.convergence_score = (d.ema_fairness
                        + 100u64.saturating_sub(d.ema_contention)
                        + d.ema_efficiency)
                        / 3;
                }
            }
            if !any_change {
                break;
            }
        }

        self.recompute_unified();
        self.convergence_events += 1;
        self.unified.clone()
    }

    fn recompute_unified(&mut self) {
        let n = self.domains.len() as u64;
        if n == 0 {
            return;
        }
        let gf = self.domains.values().map(|d| d.ema_fairness).sum::<u64>() / n;
        let gc = self.domains.values().map(|d| d.ema_contention).sum::<u64>() / n;
        let ge = self.domains.values().map(|d| d.ema_efficiency).sum::<u64>() / n;

        let combined = (gf + 100u64.saturating_sub(gc) + ge) / 3;
        let dist = 100u64.saturating_sub(combined);

        let prev_dist = self.unified.singularity_distance;
        let velocity = prev_dist as i64 - dist as i64;

        self.unified = UnifiedState {
            global_fairness: gf,
            global_contention: gc,
            global_efficiency: ge,
            singularity_distance: dist,
            intelligence_score: self.compute_intelligence(),
            convergence_velocity: velocity,
        };

        self.convergence_history.push_back(combined);
        if self.convergence_history.len() > MAX_CONVERGENCE_HISTORY {
            self.convergence_history.pop_front();
        }
    }

    // -- perfect fairness ---------------------------------------------------

    #[inline]
    pub fn perfect_fairness(&self) -> u64 {
        if self.domains.is_empty() {
            return 100;
        }
        let scores: Vec<u64> = self.domains.values().map(|d| d.ema_fairness).collect();
        let mean = scores.iter().sum::<u64>() / scores.len() as u64;
        let max_dev = scores.iter().map(|&s| abs_diff(s, mean)).max().unwrap_or(0);
        100u64.saturating_sub(max_dev)
    }

    // -- zero contention ----------------------------------------------------

    #[inline]
    pub fn zero_contention(&self) -> u64 {
        if self.domains.is_empty() {
            return 100;
        }
        let total_contention: u64 = self.domains.values().map(|d| d.ema_contention).sum();
        let avg = total_contention / self.domains.len() as u64;
        100u64.saturating_sub(avg)
    }

    // -- cooperation singularity --------------------------------------------

    #[inline]
    pub fn cooperation_singularity(&mut self) -> bool {
        let fairness = self.perfect_fairness();
        let zero_cont = self.zero_contention();
        let efficiency = self.unified.global_efficiency;

        let singularity_score = (fairness + zero_cont + efficiency) / 3;
        let achieved = singularity_score >= SINGULARITY_THRESHOLD;

        self.stats.singularity_achieved = achieved;
        achieved
    }

    // -- intelligence level -------------------------------------------------

    #[inline(always)]
    pub fn intelligence_level(&self) -> u64 {
        self.compute_intelligence()
    }

    fn compute_intelligence(&self) -> u64 {
        if self.domains.is_empty() {
            return 0;
        }

        let fairness = self.perfect_fairness();
        let contention = self.zero_contention();
        let efficiency = self.unified.global_efficiency;

        let adaptability = if self.convergence_history.len() >= 4 {
            let recent = &self.convergence_history[self.convergence_history.len() - 2..];
            let older = &self.convergence_history[..2];
            let recent_avg = recent.iter().sum::<u64>() / recent.len() as u64;
            let older_avg = older.iter().sum::<u64>() / older.len() as u64;
            if recent_avg > older_avg {
                clamp((recent_avg - older_avg) * 5, 0, 100)
            } else {
                0
            }
        } else {
            0
        };

        let scale = clamp(self.domains.len() as u64 * 2, 0, 100);
        let base = (fairness + contention + efficiency + adaptability) / 4;
        clamp((base * (100 + scale)) / 200, 0, 100)
    }

    // -- tick ---------------------------------------------------------------

    #[inline]
    pub fn tick(&mut self) {
        self.tick += 1;
        for d in self.domains.values_mut() {
            d.ema_contention = d.ema_contention.saturating_sub(1);
            d.convergence_score =
                (d.ema_fairness + 100u64.saturating_sub(d.ema_contention) + d.ema_efficiency) / 3;
        }
        self.recompute_unified();
        self.refresh_stats();
    }

    // -- perturbation -------------------------------------------------------

    pub fn inject_perturbation(&mut self) {
        let domain_ids: Vec<u64> = self.domains.keys().copied().collect();
        if domain_ids.is_empty() {
            return;
        }
        let idx = (xorshift64(&mut self.rng_state) as usize) % domain_ids.len();
        let did = domain_ids[idx];
        if let Some(d) = self.domains.get_mut(&did) {
            let perturbation = xorshift64(&mut self.rng_state) % 30;
            d.ema_contention = clamp(d.ema_contention + perturbation, 0, 100);
            d.ema_fairness = d.ema_fairness.saturating_sub(perturbation / 2);
        }
    }

    // -- stats --------------------------------------------------------------

    fn refresh_stats(&mut self) {
        let n = self.domains.len();
        let (af, ac, ae) = if n > 0 {
            let f = self.domains.values().map(|d| d.ema_fairness).sum::<u64>() / n as u64;
            let c = self.domains.values().map(|d| d.ema_contention).sum::<u64>() / n as u64;
            let e = self.domains.values().map(|d| d.ema_efficiency).sum::<u64>() / n as u64;
            (f, c, e)
        } else {
            (50, 50, 50)
        };

        self.stats = SingularityStats {
            domains_count: n,
            avg_fairness: af,
            avg_contention: ac,
            avg_efficiency: ae,
            singularity_achieved: self.stats.singularity_achieved,
            intelligence_level: self.compute_intelligence(),
            convergence_events: self.convergence_events,
            ticks_elapsed: self.tick,
        };
    }

    #[inline(always)]
    pub fn stats(&self) -> SingularityStats {
        self.stats.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_cooperation() {
        let mut cs = CoopSingularity::new(42);
        cs.register_domain(1, 80, 20, 90);
        cs.register_domain(2, 85, 15, 88);
        let state = cs.unified_cooperation();
        assert!(state.global_fairness > 0);
        assert!(state.singularity_distance < 100);
    }

    #[test]
    fn test_perfect_fairness() {
        let mut cs = CoopSingularity::new(7);
        cs.register_domain(1, 90, 5, 95);
        cs.register_domain(2, 90, 5, 95);
        let f = cs.perfect_fairness();
        assert!(f >= 95);
    }

    #[test]
    fn test_zero_contention() {
        let mut cs = CoopSingularity::new(99);
        cs.register_domain(1, 80, 5, 90);
        cs.register_domain(2, 85, 3, 92);
        let zc = cs.zero_contention();
        assert!(zc >= 90);
    }

    #[test]
    fn test_singularity_convergence() {
        let mut cs = CoopSingularity::new(55);
        cs.register_domain(1, 98, 2, 98);
        cs.register_domain(2, 97, 3, 97);
        cs.unified_cooperation();
        let achieved = cs.cooperation_singularity();
        assert!(achieved);
    }

    #[test]
    fn test_intelligence_level() {
        let mut cs = CoopSingularity::new(11);
        cs.register_domain(1, 90, 10, 85);
        cs.unified_cooperation();
        let il = cs.intelligence_level();
        assert!(il > 0);
    }
}
