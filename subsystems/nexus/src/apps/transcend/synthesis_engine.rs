// SPDX-License-Identifier: GPL-2.0
//! # Apps Synthesis Engine — Self-Evolving Application Understanding
//!
//! Creates new classification dimensions, prediction features, and
//! optimization strategies autonomously. The engine evolves its own
//! understanding framework, synthesising novel analytical dimensions
//! and continuously self-improving its accuracy and coverage.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x00000100000001B3;
const EMA_ALPHA_NUM: u64 = 3;
const EMA_ALPHA_DEN: u64 = 10;
const MAX_DIMENSIONS: usize = 128;
const MAX_FEATURES: usize = 256;
const EVOLUTION_THRESHOLD: u64 = 50;

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

/// A synthesised classification dimension.
#[derive(Clone, Debug)]
pub struct SynthDimension {
    pub dim_id: u64,
    pub label: String,
    pub origin_hash: u64,
    pub discriminative_power: u64,
    pub usage_count: u64,
    pub generation: u64,
}

/// A synthesised prediction feature.
#[derive(Clone, Debug)]
pub struct SynthFeature {
    pub feature_id: u64,
    pub label: String,
    pub source_dims: Vec<u64>,
    pub predictive_value: u64,
    pub generation: u64,
}

/// An autonomously created optimization strategy.
#[derive(Clone, Debug)]
pub struct SynthStrategy {
    pub strategy_id: u64,
    pub label: String,
    pub trigger_condition_hash: u64,
    pub estimated_gain: u64,
    pub apps_applied: u64,
    pub generation: u64,
}

/// Per-app observation buffer for synthesis.
#[derive(Clone, Debug)]
#[repr(align(64))]
pub struct AppObservationBuffer {
    pub app_id: u64,
    pub samples: VecDeque<[u64; 4]>,
    pub sample_count: u64,
}

/// Evolution record tracking a self-improvement cycle.
#[derive(Clone, Debug)]
pub struct EvolutionRecord {
    pub cycle: u64,
    pub dimensions_added: u64,
    pub features_added: u64,
    pub strategies_added: u64,
    pub accuracy_before: u64,
    pub accuracy_after: u64,
}

/// Statistics for the synthesis engine.
#[derive(Clone, Debug, Default)]
#[repr(align(64))]
pub struct SynthesisStats {
    pub total_dimensions: u64,
    pub total_features: u64,
    pub total_strategies: u64,
    pub evolution_cycles: u64,
    pub avg_discriminative_power_ema: u64,
    pub avg_predictive_value_ema: u64,
    pub synthesis_impact: u64,
}

// ---------------------------------------------------------------------------
// AppsSynthesisEngine
// ---------------------------------------------------------------------------

/// Engine that autonomously evolves its application understanding framework.
pub struct AppsSynthesisEngine {
    dimensions: BTreeMap<u64, SynthDimension>,
    features: BTreeMap<u64, SynthFeature>,
    strategies: Vec<SynthStrategy>,
    observations: BTreeMap<u64, AppObservationBuffer>,
    evolution_log: Vec<EvolutionRecord>,
    stats: SynthesisStats,
    generation: u64,
    rng: u64,
}

impl AppsSynthesisEngine {
    /// Create a new synthesis engine.
    pub fn new(seed: u64) -> Self {
        Self {
            dimensions: BTreeMap::new(),
            features: BTreeMap::new(),
            strategies: Vec::new(),
            observations: BTreeMap::new(),
            evolution_log: Vec::new(),
            stats: SynthesisStats::default(),
            generation: 0,
            rng: seed | 1,
        }
    }

    // -- ingestion ----------------------------------------------------------

    /// Feed a raw observation sample for an app (cpu, mem, io, ipc).
    pub fn feed_sample(&mut self, app_id: u64, cpu: u64, mem: u64, io: u64, ipc: u64) {
        let buf = self.observations.entry(app_id).or_insert(AppObservationBuffer {
            app_id,
            samples: VecDeque::new(),
            sample_count: 0,
        });
        if buf.samples.len() >= 256 {
            buf.samples.pop_front().unwrap();
        }
        buf.samples.push([cpu, mem, io, ipc]);
        buf.sample_count += 1;
    }

    // -- public API ---------------------------------------------------------

    /// Trigger an evolution cycle — the engine attempts to improve itself.
    pub fn evolve_understanding(&mut self) -> EvolutionRecord {
        self.generation += 1;
        let accuracy_before = self.current_accuracy();

        let dims_added = self.auto_synthesize_dimensions();
        let feats_added = self.auto_synthesize_features();
        let strats_added = self.auto_synthesize_strategies();

        let accuracy_after = self.current_accuracy();

        let record = EvolutionRecord {
            cycle: self.generation,
            dimensions_added: dims_added,
            features_added: feats_added,
            strategies_added: strats_added,
            accuracy_before,
            accuracy_after,
        };
        self.evolution_log.push(record.clone());
        self.stats.evolution_cycles = self.generation;
        self.refresh_stats();
        record
    }

    /// Synthesise a single new classification dimension from data.
    pub fn synthesize_dimension(&mut self, label: &str) -> Option<SynthDimension> {
        if self.dimensions.len() >= MAX_DIMENSIONS {
            return None;
        }

        let dim_id = fnv1a(label.as_bytes()) ^ xorshift64(&mut self.rng);
        let disc_power = self.compute_discriminative_power(dim_id);

        let dim = SynthDimension {
            dim_id,
            label: String::from(label),
            origin_hash: fnv1a(&dim_id.to_le_bytes()),
            discriminative_power: disc_power,
            usage_count: 0,
            generation: self.generation,
        };
        self.dimensions.insert(dim_id, dim.clone());
        self.stats.total_dimensions = self.dimensions.len() as u64;
        Some(dim)
    }

    /// Create a new prediction feature from existing dimensions.
    pub fn create_feature(&mut self, label: &str, source_dims: &[u64]) -> Option<SynthFeature> {
        if self.features.len() >= MAX_FEATURES {
            return None;
        }

        let feature_id = fnv1a(label.as_bytes()) ^ xorshift64(&mut self.rng);
        let pred_value = self.compute_predictive_value(source_dims);

        let feature = SynthFeature {
            feature_id,
            label: String::from(label),
            source_dims: source_dims.to_vec(),
            predictive_value: pred_value,
            generation: self.generation,
        };
        self.features.insert(feature_id, feature.clone());
        self.stats.total_features = self.features.len() as u64;
        Some(feature)
    }

    /// Trigger self-improvement: prune weak dimensions/features, amplify strong ones.
    pub fn self_improve(&mut self) -> u64 {
        let mut pruned: u64 = 0;

        // Prune dimensions with low discriminative power.
        let weak_dims: Vec<u64> = self.dimensions.iter()
            .filter(|(_, d)| d.discriminative_power < 10 && d.usage_count < 3)
            .map(|(k, _)| *k)
            .collect();
        for dim_id in &weak_dims {
            self.dimensions.remove(dim_id);
            pruned += 1;
        }

        // Prune features with low predictive value.
        let weak_feats: Vec<u64> = self.features.iter()
            .filter(|(_, f)| f.predictive_value < 10)
            .map(|(k, _)| *k)
            .collect();
        for feat_id in &weak_feats {
            self.features.remove(feat_id);
            pruned += 1;
        }

        // Amplify strong dimensions.
        let strong_dims: Vec<u64> = self.dimensions.iter()
            .filter(|(_, d)| d.discriminative_power > EVOLUTION_THRESHOLD)
            .map(|(k, _)| *k)
            .collect();
        for dim_id in &strong_dims {
            if let Some(d) = self.dimensions.get_mut(&dim_id) {
                d.usage_count += 1;
                d.discriminative_power = ema_update(
                    d.discriminative_power,
                    d.discriminative_power + 5,
                );
            }
        }

        self.stats.total_dimensions = self.dimensions.len() as u64;
        self.stats.total_features = self.features.len() as u64;
        pruned
    }

    /// Compute the overall impact of synthesis on understanding quality (0–100).
    #[inline]
    pub fn synthesis_impact(&self) -> u64 {
        let dim_factor = (self.stats.total_dimensions * 3).min(100);
        let feat_factor = (self.stats.total_features * 2).min(100);
        let strat_factor = (self.stats.total_strategies * 5).min(100);
        let evo_factor = (self.stats.evolution_cycles * 8).min(100);
        (dim_factor + feat_factor + strat_factor + evo_factor) / 4
    }

    /// Return current statistics.
    #[inline(always)]
    pub fn stats(&self) -> &SynthesisStats {
        &self.stats
    }

    // -- internal -----------------------------------------------------------

    fn auto_synthesize_dimensions(&mut self) -> u64 {
        let mut added: u64 = 0;
        let app_ids: Vec<u64> = self.observations.keys().copied().collect();

        for app_id in &app_ids {
            let buf = match self.observations.get(app_id) {
                Some(b) => b,
                None => continue,
            };
            if buf.sample_count < 10 {
                continue;
            }

            // Compute variance for each metric dimension.
            for metric_idx in 0..4u64 {
                let values: Vec<u64> = buf.samples.iter()
                    .map(|s| s[metric_idx as usize])
                    .collect();
                let variance = self.compute_variance(&values);

                if variance > 20 && self.dimensions.len() < MAX_DIMENSIONS {
                    let label_seed = app_id.wrapping_mul(metric_idx + 1);
                    let dim_id = fnv1a(&label_seed.to_le_bytes()) ^ xorshift64(&mut self.rng);

                    if !self.dimensions.contains_key(&dim_id) {
                        let label = alloc::format!("auto_dim_{}_{}", app_id & 0xFFFF, metric_idx);
                        self.dimensions.insert(dim_id, SynthDimension {
                            dim_id,
                            label,
                            origin_hash: fnv1a(&dim_id.to_le_bytes()),
                            discriminative_power: variance.min(100),
                            usage_count: 0,
                            generation: self.generation,
                        });
                        added += 1;
                    }
                }
            }
        }

        self.stats.total_dimensions = self.dimensions.len() as u64;
        added
    }

    fn auto_synthesize_features(&mut self) -> u64 {
        let mut added: u64 = 0;
        let dim_ids: Vec<u64> = self.dimensions.keys().copied().collect();

        let len = dim_ids.len();
        let mut i = 0;
        while i < len && self.features.len() < MAX_FEATURES {
            let j = (i + 1) % len.max(1);
            if i == j {
                i += 1;
                continue;
            }

            let combo_hash = fnv1a(&dim_ids[i].to_le_bytes())
                ^ fnv1a(&dim_ids[j].to_le_bytes())
                ^ xorshift64(&mut self.rng);

            if !self.features.contains_key(&combo_hash) {
                let label = alloc::format!("auto_feat_{:x}", combo_hash & 0xFFFF);
                let pred_value = self.compute_predictive_value(&[dim_ids[i], dim_ids[j]]);
                if pred_value > 15 {
                    self.features.insert(combo_hash, SynthFeature {
                        feature_id: combo_hash,
                        label,
                        source_dims: alloc::vec![dim_ids[i], dim_ids[j]],
                        predictive_value: pred_value,
                        generation: self.generation,
                    });
                    added += 1;
                }
            }
            i += 2;
        }

        self.stats.total_features = self.features.len() as u64;
        added
    }

    fn auto_synthesize_strategies(&mut self) -> u64 {
        let mut added: u64 = 0;

        let strong_features: Vec<&SynthFeature> = self.features.values()
            .filter(|f| f.predictive_value > 40)
            .collect();

        for feat in &strong_features {
            let strat_id = feat.feature_id ^ xorshift64(&mut self.rng);
            let already_exists = self.strategies.iter().any(|s| {
                s.trigger_condition_hash == feat.feature_id
            });
            if already_exists {
                continue;
            }
            let label = alloc::format!("strategy_from_{}", feat.label);
            let gain = feat.predictive_value / 2 + xorshift64(&mut self.rng) % 10;
            self.strategies.push(SynthStrategy {
                strategy_id: strat_id,
                label,
                trigger_condition_hash: feat.feature_id,
                estimated_gain: gain.min(100),
                apps_applied: 0,
                generation: self.generation,
            });
            added += 1;
        }

        self.stats.total_strategies = self.strategies.len() as u64;
        added
    }

    fn compute_discriminative_power(&self, dim_id: u64) -> u64 {
        let mut total_variance: u64 = 0;
        let mut count: u64 = 0;

        for buf in self.observations.values() {
            if buf.sample_count < 5 {
                continue;
            }
            let idx = (dim_id % 4) as usize;
            let values: Vec<u64> = buf.samples.iter().map(|s| s[idx]).collect();
            total_variance += self.compute_variance(&values);
            count += 1;
        }

        if count == 0 { 20 } else { (total_variance / count).min(100) }
    }

    fn compute_predictive_value(&self, source_dims: &[u64]) -> u64 {
        let mut value: u64 = 0;
        for &dim_id in source_dims {
            if let Some(d) = self.dimensions.get(&dim_id) {
                value += d.discriminative_power;
            }
        }
        if !source_dims.is_empty() {
            value /= source_dims.len() as u64;
        }
        value.min(100)
    }

    fn compute_variance(&self, values: &[u64]) -> u64 {
        if values.is_empty() {
            return 0;
        }
        let mean = values.iter().sum::<u64>() / values.len() as u64;
        let var: u64 = values.iter().map(|&v| {
            let d = if v > mean { v - mean } else { mean - v };
            d * d
        }).sum::<u64>() / values.len() as u64;
        // Return sqrt approximation (integer).
        let mut r: u64 = var;
        if r > 1 {
            let mut x = r;
            let mut y = (x + 1) / 2;
            while y < x {
                x = y;
                y = (x + r / x.max(1)) / 2;
            }
            r = x;
        }
        r.min(100)
    }

    fn current_accuracy(&self) -> u64 {
        let dim_avg = if self.dimensions.is_empty() {
            0
        } else {
            self.dimensions.values()
                .map(|d| d.discriminative_power)
                .sum::<u64>() / self.dimensions.len() as u64
        };
        let feat_avg = if self.features.is_empty() {
            0
        } else {
            self.features.values()
                .map(|f| f.predictive_value)
                .sum::<u64>() / self.features.len() as u64
        };
        (dim_avg + feat_avg) / 2
    }

    fn refresh_stats(&mut self) {
        let dim_avg = if self.dimensions.is_empty() {
            0
        } else {
            self.dimensions.values()
                .map(|d| d.discriminative_power)
                .sum::<u64>() / self.dimensions.len() as u64
        };
        let feat_avg = if self.features.is_empty() {
            0
        } else {
            self.features.values()
                .map(|f| f.predictive_value)
                .sum::<u64>() / self.features.len() as u64
        };
        self.stats.avg_discriminative_power_ema = ema_update(
            self.stats.avg_discriminative_power_ema,
            dim_avg,
        );
        self.stats.avg_predictive_value_ema = ema_update(
            self.stats.avg_predictive_value_ema,
            feat_avg,
        );
        self.stats.synthesis_impact = self.synthesis_impact();
    }
}
