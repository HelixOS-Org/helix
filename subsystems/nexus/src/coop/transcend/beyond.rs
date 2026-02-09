// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Beyond — Transcending Traditional Cooperation Limits
//!
//! Implements anticipatory cooperation (cooperate before the need arises),
//! trust synthesis (create trust from scratch through algorithmic guarantees),
//! and emergent protocol generation.  The module continuously innovates on
//! cooperation strategies and measures its own transcendence level.

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
const MAX_PREDICTIONS: usize = 2048;
const MAX_TRUST_SEEDS: usize = 1024;
const MAX_PROTOCOLS: usize = 512;
const ANTICIPATION_HORIZON: u64 = 16;

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

// ---------------------------------------------------------------------------
// Anticipatory need
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct AnticipatedNeed {
    pub need_id: u64,
    pub agent_id: u64,
    pub predicted_demand: u64,
    pub confidence: u64,
    pub horizon_ticks: u64,
    pub pre_allocated: u64,
    pub accuracy_ema: u64,
}

// ---------------------------------------------------------------------------
// Trust seed
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct TrustSeed {
    pub seed_id: u64,
    pub source_id: u64,
    pub target_id: u64,
    pub synthetic_trust: u64,
    pub verification_count: u64,
    pub maturity: u64,
}

// ---------------------------------------------------------------------------
// Emergent protocol
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct EmergentProtocol {
    pub protocol_id: u64,
    pub genome: Vec<u64>,
    pub fitness: u64,
    pub generation: u64,
    pub adoption_count: u64,
    pub novelty_score: u64,
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
#[repr(align(64))]
pub struct BeyondStats {
    pub anticipated_needs: usize,
    pub trust_seeds: usize,
    pub emergent_protocols: usize,
    pub avg_anticipation_accuracy: u64,
    pub avg_synthetic_trust: u64,
    pub avg_protocol_fitness: u64,
    pub innovation_rate: u64,
    pub transcendence_level: u64,
}

// ---------------------------------------------------------------------------
// CoopBeyond
// ---------------------------------------------------------------------------

pub struct CoopBeyond {
    needs: BTreeMap<u64, AnticipatedNeed>,
    seeds: BTreeMap<u64, TrustSeed>,
    protocols: BTreeMap<u64, EmergentProtocol>,
    rng_state: u64,
    tick: u64,
    innovation_events: u64,
    stats: BeyondStats,
    demand_history: BTreeMap<u64, Vec<u64>>,
}

impl CoopBeyond {
    pub fn new(seed: u64) -> Self {
        Self {
            needs: BTreeMap::new(),
            seeds: BTreeMap::new(),
            protocols: BTreeMap::new(),
            rng_state: seed | 1,
            tick: 0,
            innovation_events: 0,
            stats: BeyondStats {
                anticipated_needs: 0,
                trust_seeds: 0,
                emergent_protocols: 0,
                avg_anticipation_accuracy: 50,
                avg_synthetic_trust: 0,
                avg_protocol_fitness: 0,
                innovation_rate: 0,
                transcendence_level: 0,
            },
            demand_history: BTreeMap::new(),
        }
    }

    // -- demand observation -------------------------------------------------

    #[inline]
    pub fn observe_demand(&mut self, agent_id: u64, demand: u64) {
        let history = self.demand_history.entry(agent_id).or_insert_with(Vec::new);
        history.push(demand);
        if history.len() > 128 {
            history.pop_front();
        }
    }

    // -- anticipatory cooperation -------------------------------------------

    pub fn anticipatory_cooperate(&mut self, agent_id: u64) -> Option<u64> {
        if self.needs.len() >= MAX_PREDICTIONS {
            return None;
        }

        let predicted = self.predict_demand(agent_id);
        if predicted == 0 {
            return None;
        }

        let confidence = self.prediction_confidence(agent_id);
        let pre_alloc = predicted * confidence / 100;

        let need_id = fnv1a(&[agent_id.to_le_bytes(), self.tick.to_le_bytes()].concat());
        let need = AnticipatedNeed {
            need_id,
            agent_id,
            predicted_demand: predicted,
            confidence,
            horizon_ticks: ANTICIPATION_HORIZON,
            pre_allocated: pre_alloc,
            accuracy_ema: 50,
        };

        self.needs.insert(need_id, need);
        self.innovation_events += 1;
        Some(pre_alloc)
    }

    fn predict_demand(&self, agent_id: u64) -> u64 {
        let history = match self.demand_history.get(&agent_id) {
            Some(h) if h.len() >= 2 => h,
            _ => return 0,
        };

        // Weighted moving average with trend extrapolation
        let n = history.len();
        let recent_avg = if n >= 4 {
            history[n - 4..].iter().sum::<u64>() / 4
        } else {
            history.iter().sum::<u64>() / n as u64
        };

        let older_avg = if n >= 8 {
            history[n - 8..n - 4].iter().sum::<u64>() / 4
        } else {
            history[..n / 2].iter().sum::<u64>().max(1) / (n as u64 / 2).max(1)
        };

        // Trend: positive means increasing demand
        let trend = if recent_avg > older_avg {
            recent_avg - older_avg
        } else {
            0
        };

        recent_avg.saturating_add(trend)
    }

    fn prediction_confidence(&self, agent_id: u64) -> u64 {
        let history = match self.demand_history.get(&agent_id) {
            Some(h) => h,
            None => return 0,
        };
        if history.len() < 3 {
            return 20;
        }

        // Coefficient of variation as proxy for predictability
        let n = history.len() as u64;
        let mean = history.iter().sum::<u64>() / n;
        if mean == 0 {
            return 50;
        }
        let variance = history
            .iter()
            .map(|&x| {
                let d = if x > mean { x - mean } else { mean - x };
                d * d
            })
            .sum::<u64>()
            / n;

        // Integer sqrt approximation
        let std_dev = integer_sqrt(variance);
        let cv = std_dev * 100 / mean.max(1);
        clamp(100u64.saturating_sub(cv), 10, 95)
    }

    pub fn validate_anticipation(&mut self, need_id: u64, actual_demand: u64) {
        if let Some(need) = self.needs.get_mut(&need_id) {
            let predicted = need.predicted_demand;
            let error = if actual_demand > predicted {
                actual_demand - predicted
            } else {
                predicted - actual_demand
            };
            let accuracy = if predicted > 0 {
                100u64.saturating_sub(error * 100 / predicted.max(1))
            } else {
                0
            };
            need.accuracy_ema = ema_update(need.accuracy_ema, accuracy);
        }
    }

    // -- trust synthesis ----------------------------------------------------

    pub fn trust_synthesis(&mut self, source: u64, target: u64) -> u64 {
        if self.seeds.len() >= MAX_TRUST_SEEDS {
            return 0;
        }

        let seed_id = fnv1a(&[source.to_le_bytes(), target.to_le_bytes()].concat());

        if let Some(existing) = self.seeds.get_mut(&seed_id) {
            existing.verification_count += 1;
            let growth = clamp(existing.verification_count * 5, 0, 50);
            existing.synthetic_trust = clamp(existing.synthetic_trust + growth, 0, 100);
            existing.maturity = clamp(existing.verification_count * 10, 0, 100);
            self.innovation_events += 1;
            return existing.synthetic_trust;
        }

        // Bootstrap synthetic trust from cooperation history heuristics
        let base_trust = self.bootstrap_trust(source, target);
        let seed = TrustSeed {
            seed_id,
            source_id: source,
            target_id: target,
            synthetic_trust: base_trust,
            verification_count: 1,
            maturity: 5,
        };
        let trust = seed.synthetic_trust;
        self.seeds.insert(seed_id, seed);
        self.innovation_events += 1;
        trust
    }

    fn bootstrap_trust(&self, source: u64, target: u64) -> u64 {
        // Transitive trust: if source trusts X and X trusts target, infer trust
        let mut max_transitive = 0u64;
        for seed in self.seeds.values() {
            if seed.source_id == source && seed.maturity > 30 {
                // Check if this intermediary has a path to target
                let intermediary = seed.target_id;
                for other in self.seeds.values() {
                    if other.source_id == intermediary && other.target_id == target {
                        let transitive = (seed.synthetic_trust * other.synthetic_trust) / 100;
                        if transitive > max_transitive {
                            max_transitive = transitive;
                        }
                    }
                }
            }
        }
        clamp(max_transitive.max(10), 10, 60)
    }

    // -- emergent protocol --------------------------------------------------

    pub fn emergent_protocol(&mut self) -> Option<u64> {
        if self.protocols.len() >= MAX_PROTOCOLS {
            return None;
        }

        // Generate a random protocol genome
        let genome_len = 4 + (xorshift64(&mut self.rng_state) % 8) as usize;
        let mut genome = Vec::with_capacity(genome_len);
        for _ in 0..genome_len {
            genome.push(xorshift64(&mut self.rng_state) % 256);
        }

        let protocol_id = fnv1a(
            &genome
                .iter()
                .flat_map(|g| g.to_le_bytes())
                .collect::<Vec<u8>>(),
        );
        let fitness = self.evaluate_protocol_fitness(&genome);
        let novelty = self.compute_novelty(&genome);

        let generation = self
            .protocols
            .values()
            .map(|p| p.generation)
            .max()
            .unwrap_or(0)
            + 1;

        let proto = EmergentProtocol {
            protocol_id,
            genome,
            fitness,
            generation,
            adoption_count: 0,
            novelty_score: novelty,
        };

        let fit = proto.fitness;
        self.protocols.insert(protocol_id, proto);
        self.innovation_events += 1;
        Some(fit)
    }

    fn evaluate_protocol_fitness(&self, genome: &[u64]) -> u64 {
        if genome.is_empty() {
            return 0;
        }
        let diversity = {
            let mut seen = BTreeMap::new();
            for &g in genome {
                *seen.entry(g).or_insert(0u64) += 1;
            }
            seen.len() as u64 * 100 / genome.len() as u64
        };
        let complexity = clamp(genome.len() as u64 * 10, 10, 100);
        (diversity + complexity) / 2
    }

    fn compute_novelty(&self, genome: &[u64]) -> u64 {
        if self.protocols.is_empty() {
            return 100;
        }
        let mut min_distance = u64::MAX;
        let genome_hash = fnv1a(
            &genome
                .iter()
                .flat_map(|g| g.to_le_bytes())
                .collect::<Vec<u8>>(),
        );
        for proto in self.protocols.values() {
            let other_hash = fnv1a(
                &proto
                    .genome
                    .iter()
                    .flat_map(|g| g.to_le_bytes())
                    .collect::<Vec<u8>>(),
            );
            let dist = genome_hash ^ other_hash;
            let dist_score = (dist % 100).max(1);
            if dist_score < min_distance {
                min_distance = dist_score;
            }
        }
        clamp(min_distance, 0, 100)
    }

    // -- cooperation innovation ---------------------------------------------

    pub fn cooperation_innovation(&mut self) -> u64 {
        // Evolve existing protocols
        let proto_ids: Vec<u64> = self.protocols.keys().copied().collect();
        let mut total_improvement = 0u64;

        for pid in proto_ids.iter().take(16) {
            if let Some(proto) = self.protocols.get_mut(pid) {
                let mutation = xorshift64(&mut self.rng_state) % 256;
                let idx = (xorshift64(&mut self.rng_state) as usize) % proto.genome.len().max(1);
                if idx < proto.genome.len() {
                    proto.genome[idx] = mutation;
                }
                let old_fitness = proto.fitness;
                let new_fitness = {
                    let g = proto.genome.clone();
                    self.evaluate_protocol_fitness(&g)
                };
                proto.fitness = new_fitness;
                proto.generation += 1;
                if new_fitness > old_fitness {
                    total_improvement += new_fitness - old_fitness;
                    proto.adoption_count += 1;
                }
            }
        }

        self.innovation_events += 1;
        total_improvement
    }

    // -- transcendence metric -----------------------------------------------

    pub fn transcendence_metric(&mut self) -> u64 {
        self.refresh_stats();

        let anticipation_score = self.stats.avg_anticipation_accuracy;
        let trust_score = self.stats.avg_synthetic_trust;
        let protocol_score = self.stats.avg_protocol_fitness;
        let innovation_score = clamp(self.innovation_events, 0, 100);

        let transcendence =
            (anticipation_score + trust_score + protocol_score + innovation_score) / 4;
        self.stats.transcendence_level = transcendence;
        transcendence
    }

    // -- tick ---------------------------------------------------------------

    pub fn tick(&mut self) {
        self.tick += 1;
        // Expire old anticipations
        let expired: Vec<u64> = self
            .needs
            .iter()
            .filter(|(_, n)| n.horizon_ticks == 0)
            .map(|(&id, _)| id)
            .collect();
        for id in expired {
            self.needs.remove(&id);
        }
        for need in self.needs.values_mut() {
            need.horizon_ticks = need.horizon_ticks.saturating_sub(1);
        }
        self.refresh_stats();
    }

    // -- stats --------------------------------------------------------------

    fn refresh_stats(&mut self) {
        let n_needs = self.needs.len();
        let n_seeds = self.seeds.len();
        let n_protos = self.protocols.len();

        let avg_acc = if n_needs > 0 {
            self.needs.values().map(|n| n.accuracy_ema).sum::<u64>() / n_needs as u64
        } else {
            50
        };

        let avg_trust = if n_seeds > 0 {
            self.seeds.values().map(|s| s.synthetic_trust).sum::<u64>() / n_seeds as u64
        } else {
            0
        };

        let avg_fit = if n_protos > 0 {
            self.protocols.values().map(|p| p.fitness).sum::<u64>() / n_protos as u64
        } else {
            0
        };

        let inno_rate = if self.tick > 0 {
            self.innovation_events * 100 / self.tick
        } else {
            0
        };

        self.stats = BeyondStats {
            anticipated_needs: n_needs,
            trust_seeds: n_seeds,
            emergent_protocols: n_protos,
            avg_anticipation_accuracy: avg_acc,
            avg_synthetic_trust: avg_trust,
            avg_protocol_fitness: avg_fit,
            innovation_rate: clamp(inno_rate, 0, 100),
            transcendence_level: self.stats.transcendence_level,
        };
    }

    #[inline(always)]
    pub fn stats(&self) -> BeyondStats {
        self.stats.clone()
    }
}

fn integer_sqrt(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anticipatory_cooperate() {
        let mut cb = CoopBeyond::new(42);
        for i in 0..10 {
            cb.observe_demand(1, 50 + i * 5);
        }
        let result = cb.anticipatory_cooperate(1);
        assert!(result.is_some());
        assert!(result.unwrap() > 0);
    }

    #[test]
    fn test_trust_synthesis() {
        let mut cb = CoopBeyond::new(7);
        let t1 = cb.trust_synthesis(1, 2);
        assert!(t1 >= 10);
        let t2 = cb.trust_synthesis(1, 2);
        assert!(t2 >= t1);
    }

    #[test]
    fn test_emergent_protocol() {
        let mut cb = CoopBeyond::new(99);
        let f = cb.emergent_protocol();
        assert!(f.is_some());
    }

    #[test]
    fn test_transcendence() {
        let mut cb = CoopBeyond::new(55);
        cb.trust_synthesis(1, 2);
        cb.emergent_protocol();
        cb.tick();
        let t = cb.transcendence_metric();
        assert!(t > 0);
    }
}
