// SPDX-License-Identifier: GPL-2.0
//! # Apps Monte Carlo Simulator
//!
//! Monte Carlo simulation engine for application workload futures.
//! Samples from learned workload distribution models to generate
//! thousands of possible future trajectories, then aggregates them
//! into percentile forecasts, tail risk assessments, and expected
//! values. Each trial is a stochastic walk through the app's resource
//! distribution, producing a distribution of outcomes rather than
//! a single point estimate.
//!
//! This is the kernel exploring a thousand parallel futures.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_DISTRIBUTIONS: usize = 256;
const MAX_TRIALS: usize = 2048;
const DEFAULT_TRIALS: usize = 500;
const MAX_STEPS_PER_TRIAL: usize = 64;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// Approximate normal sample from two uniform xorshift draws (Box-Muller-like)
fn approx_normal(rng: &mut u64, mean: f32, stddev: f32) -> f32 {
    let u1 = (xorshift64(rng) % 10000) as f32 / 10000.0 + 0.0001;
    let u2 = (xorshift64(rng) % 10000) as f32 / 10000.0;
    let z = (1.0 - u1).ln().abs().sqrt() * 1.4142 * (u2 * 6.2832).cos();
    mean + z * stddev
}

// ============================================================================
// DISTRIBUTION TYPES
// ============================================================================

/// Workload distribution model for a resource dimension
#[derive(Debug, Clone)]
pub struct WorkloadDistribution {
    pub id: u64,
    pub name: String,
    pub mean: f32,
    pub stddev: f32,
    pub skew: f32,
    pub min_val: f32,
    pub max_val: f32,
    pub sample_count: u64,
}

impl WorkloadDistribution {
    /// Update distribution parameters with a new sample (online)
    fn update(&mut self, value: f32) {
        self.sample_count += 1;
        let old_mean = self.mean;
        self.mean = EMA_ALPHA * value + (1.0 - EMA_ALPHA) * self.mean;
        let delta = value - old_mean;
        self.stddev = EMA_ALPHA * delta.abs() + (1.0 - EMA_ALPHA) * self.stddev;
        if value < self.min_val {
            self.min_val = value;
        }
        if value > self.max_val {
            self.max_val = value;
        }
        let norm_delta = if self.stddev > 0.001 {
            delta / self.stddev
        } else {
            0.0
        };
        self.skew = EMA_ALPHA * norm_delta * norm_delta * norm_delta
            + (1.0 - EMA_ALPHA) * self.skew;
    }

    fn sample(&self, rng: &mut u64) -> f32 {
        let val = approx_normal(rng, self.mean, self.stddev);
        val.clamp(self.min_val, self.max_val)
    }
}

/// A single trial outcome
#[derive(Debug, Clone, Copy)]
pub struct TrialOutcome {
    pub trial_id: u32,
    pub final_value: f32,
    pub peak_value: f32,
    pub avg_value: f32,
    pub steps: u32,
}

/// Percentile forecast result
#[derive(Debug, Clone)]
pub struct PercentileForecast {
    pub distribution_id: u64,
    pub p10: f32,
    pub p25: f32,
    pub p50: f32,
    pub p75: f32,
    pub p90: f32,
    pub p99: f32,
    pub num_trials: u32,
}

/// Tail risk assessment
#[derive(Debug, Clone)]
pub struct TailRisk {
    pub distribution_id: u64,
    pub threshold: f32,
    pub probability_above: f32,
    pub expected_excess: f32,
    pub worst_case: f32,
    pub cvar_95: f32,
}

/// Expected value computation
#[derive(Debug, Clone)]
pub struct ExpectedValue {
    pub distribution_id: u64,
    pub mean: f32,
    pub variance: f32,
    pub confidence_interval_low: f32,
    pub confidence_interval_high: f32,
    pub sample_count: u32,
}

/// Workload sample for observation
#[derive(Debug, Clone, Copy)]
pub struct WorkloadSample {
    pub process_id: u64,
    pub resource_key: u64,
    pub value: f32,
    pub tick: u64,
}

// ============================================================================
// MONTE CARLO STATS
// ============================================================================

/// Aggregate Monte Carlo statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct MonteCarloStats {
    pub distributions_tracked: usize,
    pub total_trials_run: u64,
    pub total_samples_ingested: u64,
    pub avg_trial_outcome: f32,
    pub avg_forecast_spread: f32,
    pub tail_risk_assessments: u64,
}

// ============================================================================
// APPS MONTE CARLO SIMULATOR
// ============================================================================

/// Monte Carlo simulation engine for application workload futures.
/// Maintains per-resource distribution models and generates statistical
/// futures via repeated stochastic sampling.
#[derive(Debug)]
pub struct AppsMonteCarlo {
    distributions: BTreeMap<u64, WorkloadDistribution>,
    trial_results: BTreeMap<u64, Vec<TrialOutcome>>,
    total_trials: u64,
    total_samples: u64,
    tail_assessments: u64,
    rng_state: u64,
    avg_outcome_ema: f32,
    avg_spread_ema: f32,
}

impl AppsMonteCarlo {
    pub fn new() -> Self {
        Self {
            distributions: BTreeMap::new(),
            trial_results: BTreeMap::new(),
            total_trials: 0,
            total_samples: 0,
            tail_assessments: 0,
            rng_state: 0x1234_5678_9ABC_DEF0,
            avg_outcome_ema: 0.0,
            avg_spread_ema: 0.0,
        }
    }

    /// Ingest a workload sample to update the distribution model
    pub fn sample_workload(&mut self, sample: WorkloadSample) {
        self.total_samples += 1;
        let key = fnv1a_hash(&[
            &sample.process_id.to_le_bytes()[..],
            &sample.resource_key.to_le_bytes()[..],
        ].concat());

        let dist = self.distributions.entry(key).or_insert_with(|| {
            WorkloadDistribution {
                id: key,
                name: String::new(),
                mean: sample.value,
                stddev: 0.1,
                skew: 0.0,
                min_val: sample.value,
                max_val: sample.value,
                sample_count: 0,
            }
        });
        dist.update(sample.value);
    }

    /// Run Monte Carlo trials for a distribution
    pub fn run_trials(
        &mut self,
        distribution_id: u64,
        num_trials: usize,
        steps_per_trial: usize,
    ) -> Vec<TrialOutcome> {
        let trials = num_trials.min(MAX_TRIALS);
        let steps = steps_per_trial.min(MAX_STEPS_PER_TRIAL);
        let mut outcomes = Vec::new();

        let dist = if let Some(d) = self.distributions.get(&distribution_id) {
            d.clone()
        } else {
            return outcomes;
        };

        for t in 0..trials {
            let mut val = dist.mean;
            let mut peak: f32 = val;
            let mut sum: f32 = 0.0;

            for _ in 0..steps {
                let delta = approx_normal(&mut self.rng_state, 0.0, dist.stddev * 0.3);
                val += delta;
                val = val.clamp(dist.min_val, dist.max_val * 1.5);
                if val > peak {
                    peak = val;
                }
                sum += val;
            }

            let avg = if steps > 0 { sum / steps as f32 } else { val };
            outcomes.push(TrialOutcome {
                trial_id: t as u32,
                final_value: val,
                peak_value: peak,
                avg_value: avg,
                steps: steps as u32,
            });
            self.total_trials += 1;
        }

        if !outcomes.is_empty() {
            let mean_outcome: f32 =
                outcomes.iter().map(|o| o.final_value).sum::<f32>() / outcomes.len() as f32;
            self.avg_outcome_ema =
                EMA_ALPHA * mean_outcome + (1.0 - EMA_ALPHA) * self.avg_outcome_ema;
        }

        if self.trial_results.len() >= MAX_DISTRIBUTIONS {
            if let Some(&oldest_key) = self.trial_results.keys().next() {
                self.trial_results.remove(&oldest_key);
            }
        }
        self.trial_results.insert(distribution_id, outcomes.clone());
        outcomes
    }

    /// Compute percentile forecasts from trial results
    pub fn percentile_forecast(&mut self, distribution_id: u64) -> PercentileForecast {
        let trials = if let Some(existing) = self.trial_results.get(&distribution_id) {
            existing.clone()
        } else {
            self.run_trials(distribution_id, DEFAULT_TRIALS, MAX_STEPS_PER_TRIAL)
        };

        let mut values: Vec<f32> = trials.iter().map(|o| o.final_value).collect();
        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));

        let percentile = |p: f32| -> f32 {
            if values.is_empty() {
                return 0.0;
            }
            let idx = ((p / 100.0) * (values.len() as f32 - 1.0)).round() as usize;
            values[idx.min(values.len() - 1)]
        };

        let p10 = percentile(10.0);
        let p90 = percentile(90.0);
        let spread = p90 - p10;
        self.avg_spread_ema = EMA_ALPHA * spread + (1.0 - EMA_ALPHA) * self.avg_spread_ema;

        PercentileForecast {
            distribution_id,
            p10,
            p25: percentile(25.0),
            p50: percentile(50.0),
            p75: percentile(75.0),
            p90,
            p99: percentile(99.0),
            num_trials: values.len() as u32,
        }
    }

    /// Assess tail risk: probability and magnitude of extreme outcomes
    pub fn tail_risk(&mut self, distribution_id: u64, threshold: f32) -> TailRisk {
        self.tail_assessments += 1;

        let trials = if let Some(existing) = self.trial_results.get(&distribution_id) {
            existing.clone()
        } else {
            self.run_trials(distribution_id, DEFAULT_TRIALS, MAX_STEPS_PER_TRIAL)
        };

        let total = trials.len().max(1) as f32;
        let above: Vec<f32> = trials
            .iter()
            .map(|o| o.final_value)
            .filter(|&v| v > threshold)
            .collect();

        let prob_above = above.len() as f32 / total;
        let expected_excess = if above.is_empty() {
            0.0
        } else {
            above.iter().map(|&v| v - threshold).sum::<f32>() / above.len() as f32
        };

        let worst = trials
            .iter()
            .map(|o| o.final_value)
            .fold(f32::NEG_INFINITY, f32::max);

        let mut sorted: Vec<f32> = trials.iter().map(|o| o.final_value).collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
        let p95_idx = ((0.95 * (sorted.len() as f32 - 1.0)).round() as usize)
            .min(sorted.len().saturating_sub(1));
        let p95_val = if sorted.is_empty() { 0.0 } else { sorted[p95_idx] };
        let tail_vals: Vec<f32> = sorted.iter().copied().filter(|&v| v >= p95_val).collect();
        let cvar_95 = if tail_vals.is_empty() {
            0.0
        } else {
            tail_vals.iter().sum::<f32>() / tail_vals.len() as f32
        };

        TailRisk {
            distribution_id,
            threshold,
            probability_above: prob_above,
            expected_excess,
            worst_case: worst,
            cvar_95,
        }
    }

    /// Compute expected value with confidence intervals
    pub fn expected_value(&self, distribution_id: u64) -> ExpectedValue {
        if let Some(trials) = self.trial_results.get(&distribution_id) {
            let n = trials.len().max(1) as f32;
            let mean = trials.iter().map(|o| o.final_value).sum::<f32>() / n;
            let variance = trials
                .iter()
                .map(|o| {
                    let d = o.final_value - mean;
                    d * d
                })
                .sum::<f32>()
                / n;
            let stddev = if variance > 0.0 { variance.sqrt() } else { 0.01 };
            let se = stddev / n.sqrt();
            ExpectedValue {
                distribution_id,
                mean,
                variance,
                confidence_interval_low: mean - 1.96 * se,
                confidence_interval_high: mean + 1.96 * se,
                sample_count: trials.len() as u32,
            }
        } else if let Some(dist) = self.distributions.get(&distribution_id) {
            ExpectedValue {
                distribution_id,
                mean: dist.mean,
                variance: dist.stddev * dist.stddev,
                confidence_interval_low: dist.mean - 1.96 * dist.stddev,
                confidence_interval_high: dist.mean + 1.96 * dist.stddev,
                sample_count: dist.sample_count as u32,
            }
        } else {
            ExpectedValue {
                distribution_id,
                mean: 0.0,
                variance: 0.0,
                confidence_interval_low: 0.0,
                confidence_interval_high: 0.0,
                sample_count: 0,
            }
        }
    }

    /// Get a reference to a distribution model
    pub fn get_distribution(&self, id: u64) -> Option<&WorkloadDistribution> {
        self.distributions.get(&id)
    }

    /// Remove a distribution
    pub fn remove_distribution(&mut self, id: u64) {
        self.distributions.remove(&id);
        self.trial_results.remove(&id);
    }

    /// Get aggregate statistics
    pub fn stats(&self) -> MonteCarloStats {
        MonteCarloStats {
            distributions_tracked: self.distributions.len(),
            total_trials_run: self.total_trials,
            total_samples_ingested: self.total_samples,
            avg_trial_outcome: self.avg_outcome_ema,
            avg_forecast_spread: self.avg_spread_ema,
            tail_risk_assessments: self.tail_assessments,
        }
    }
}
