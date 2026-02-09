// SPDX-License-Identifier: GPL-2.0
//! # Bridge Monte Carlo Simulation
//!
//! Monte Carlo simulation engine for syscall futures. Generates thousands of
//! random future scenarios using xorshift64 PRNG, evaluates each against
//! a scoring function, and aggregates results to produce confidence intervals,
//! expected values, and rare-event detection. The bridge making thousands of
//! bets to understand the odds.
//!
//! Statistics is the grammar of science; this is its kernel dialect.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const DEFAULT_SAMPLE_COUNT: usize = 1000;
const MAX_SAMPLES: usize = 10_000;
const MAX_DISTRIBUTIONS: usize = 64;
const EMA_ALPHA: f32 = 0.08;
const RARE_EVENT_THRESHOLD: f32 = 0.01;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const CONFIDENCE_LEVELS: [f32; 3] = [0.90, 0.95, 0.99];

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

/// Generate a pseudo-random f32 in [0.0, 1.0)
fn rand_f32(state: &mut u64) -> f32 {
    (xorshift64(state) % 1_000_000) as f32 / 1_000_000.0
}

// ============================================================================
// DISTRIBUTION TYPES
// ============================================================================

/// A probability distribution for generating random futures
#[derive(Debug, Clone)]
pub enum Distribution {
    /// Uniform distribution [low, high)
    Uniform { low: f32, high: f32 },
    /// Approximate normal via Box-Muller-like transform
    Normal { mean: f32, stddev: f32 },
    /// Exponential distribution with rate parameter
    Exponential { rate: f32 },
    /// Empirical distribution from observed data
    Empirical { values: Vec<f32> },
}

impl Distribution {
    fn sample(&self, rng: &mut u64) -> f32 {
        match self {
            Distribution::Uniform { low, high } => {
                let r = rand_f32(rng);
                low + r * (high - low)
            }
            Distribution::Normal { mean, stddev } => {
                // Approximate normal: average of 12 uniform samples minus 6
                let mut sum = 0.0f32;
                for _ in 0..12 {
                    sum += rand_f32(rng);
                }
                mean + stddev * (sum - 6.0)
            }
            Distribution::Exponential { rate } => {
                let u = rand_f32(rng).max(0.0001);
                // -ln(u) / rate, approximate ln via: ln(x) ≈ (x-1) - (x-1)^2/2 for x near 1
                // Better: use bit manipulation approximation
                let ln_u = ln_approx(u);
                -ln_u / rate.max(0.0001)
            }
            Distribution::Empirical { values } => {
                if values.is_empty() {
                    return 0.0;
                }
                let idx = (xorshift64(rng) as usize) % values.len();
                values[idx]
            }
        }
    }
}

/// Approximate natural log for no_std
fn ln_approx(x: f32) -> f32 {
    if x <= 0.0 {
        return -10.0;
    }
    let bits = x.to_bits();
    let exp = ((bits >> 23) & 0xFF) as f32 - 127.0;
    let mantissa = f32::from_bits((bits & 0x007F_FFFF) | 0x3F80_0000);
    let ln2 = 0.6931471805;
    exp * ln2 + (mantissa - 1.0) * (1.0 - 0.5 * (mantissa - 1.0))
}

// ============================================================================
// SIMULATION TYPES
// ============================================================================

/// A single simulated future scenario
#[derive(Debug, Clone)]
pub struct FutureSample {
    pub sample_id: u64,
    pub outcome_score: f32,
    pub latency_estimate: f32,
    pub resource_usage: f32,
    pub contention_level: f32,
    pub is_rare: bool,
}

/// Confidence interval for an outcome
#[derive(Debug, Clone, Copy)]
pub struct ConfidenceInterval {
    pub level: f32,
    pub lower: f32,
    pub upper: f32,
    pub width: f32,
}

/// A rare event detected during simulation
#[derive(Debug, Clone)]
pub struct RareEvent {
    pub event_id: u64,
    pub probability: f32,
    pub severity: f32,
    pub sample_count: u32,
    pub description_hash: u64,
}

/// Result of a Monte Carlo simulation run
#[derive(Debug, Clone)]
pub struct SimulationResult {
    pub simulation_id: u64,
    pub sample_count: u32,
    pub mean_score: f32,
    pub median_score: f32,
    pub std_dev: f32,
    pub min_score: f32,
    pub max_score: f32,
    pub confidence_intervals: Vec<ConfidenceInterval>,
    pub rare_events: Vec<RareEvent>,
    pub percentiles: BTreeMap<u32, f32>,
}

// ============================================================================
// MONTE CARLO STATS
// ============================================================================

/// Aggregate Monte Carlo statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct MonteCarloStats {
    pub total_simulations: u64,
    pub total_samples: u64,
    pub avg_mean_score: f32,
    pub avg_std_dev: f32,
    pub rare_events_detected: u64,
    pub avg_ci_width_95: f32,
    pub convergence_rate: f32,
}

// ============================================================================
// BRIDGE MONTE CARLO
// ============================================================================

/// Monte Carlo simulation engine for syscall future analysis. Generates
/// random future scenarios, evaluates them, and produces statistical summaries.
#[derive(Debug)]
pub struct BridgeMonteCarlo {
    distributions: BTreeMap<u64, Distribution>,
    rng_state: u64,
    tick: u64,
    total_simulations: u64,
    total_samples: u64,
    total_rare_events: u64,
    avg_mean_ema: f32,
    avg_stddev_ema: f32,
    avg_ci_width_ema: f32,
    convergence_ema: f32,
}

impl BridgeMonteCarlo {
    pub fn new() -> Self {
        Self {
            distributions: BTreeMap::new(),
            rng_state: 0xCAFE_BABE_DEAD_BEEF,
            tick: 0,
            total_simulations: 0,
            total_samples: 0,
            total_rare_events: 0,
            avg_mean_ema: 0.5,
            avg_stddev_ema: 0.1,
            avg_ci_width_ema: 0.2,
            convergence_ema: 0.5,
        }
    }

    /// Register a distribution for sampling
    pub fn register_distribution(&mut self, name_hash: u64, dist: Distribution) {
        if self.distributions.len() < MAX_DISTRIBUTIONS {
            self.distributions.insert(name_hash, dist);
        }
    }

    /// Run a full Monte Carlo simulation
    pub fn run_simulation(
        &mut self,
        sample_count: usize,
        score_weights: (f32, f32, f32),
    ) -> SimulationResult {
        self.tick += 1;
        self.total_simulations += 1;
        let n = sample_count.min(MAX_SAMPLES).max(10);

        let mut samples = Vec::with_capacity(n);
        for i in 0..n {
            let sample = self.sample_future(i as u64, score_weights);
            samples.push(sample);
        }
        self.total_samples += n as u64;

        self.aggregate_results(samples, n)
    }

    /// Generate a single future sample
    pub fn sample_future(&mut self, seed_offset: u64, weights: (f32, f32, f32)) -> FutureSample {
        let base_rng = self.rng_state ^ seed_offset.wrapping_mul(FNV_PRIME);
        let mut local_rng = base_rng;

        let latency = self.sample_from_distributions(&mut local_rng, 0);
        let resource = self.sample_from_distributions(&mut local_rng, 1);
        let contention = self.sample_from_distributions(&mut local_rng, 2);

        let score = weights.0 * (1.0 - latency.max(0.0).min(1.0))
            + weights.1 * (1.0 - resource.max(0.0).min(1.0))
            + weights.2 * (1.0 - contention.max(0.0).min(1.0));
        let normalized = score / (weights.0 + weights.1 + weights.2).max(0.001);

        let is_rare = normalized < RARE_EVENT_THRESHOLD
            || normalized > (1.0 - RARE_EVENT_THRESHOLD);

        self.rng_state = local_rng;

        FutureSample {
            sample_id: fnv1a_hash(&local_rng.to_le_bytes()),
            outcome_score: normalized.max(0.0).min(1.0),
            latency_estimate: latency.max(0.0),
            resource_usage: resource.max(0.0).min(1.0),
            contention_level: contention.max(0.0).min(1.0),
            is_rare,
        }
    }

    fn sample_from_distributions(&self, rng: &mut u64, category: u8) -> f32 {
        // Look for registered distribution matching this category
        let key = category as u64;
        if let Some(dist) = self.distributions.get(&key) {
            return dist.sample(rng);
        }
        // Default: uniform [0, 1)
        rand_f32(rng)
    }

    /// Aggregate samples into a simulation result
    pub fn aggregate_results(
        &mut self,
        mut samples: Vec<FutureSample>,
        n: usize,
    ) -> SimulationResult {
        let sim_id = fnv1a_hash(&self.total_simulations.to_le_bytes());

        // Sort by score for percentile computation
        samples.sort_by(|a, b|
            a.outcome_score.partial_cmp(&b.outcome_score)
                .unwrap_or(core::cmp::Ordering::Equal));

        let scores: Vec<f32> = samples.iter().map(|s| s.outcome_score).collect();
        let n_f = n as f32;

        let mean = scores.iter().sum::<f32>() / n_f;
        let variance = scores.iter()
            .map(|&s| (s - mean) * (s - mean))
            .sum::<f32>() / n_f;
        let std_dev = sqrt_approx(variance);
        let median = scores[n / 2];
        let min_score = scores.first().copied().unwrap_or(0.0);
        let max_score = scores.last().copied().unwrap_or(1.0);

        // Confidence intervals
        let mut confidence_intervals = Vec::new();
        for &level in &CONFIDENCE_LEVELS {
            let ci = self.confidence_interval(&scores, level);
            confidence_intervals.push(ci);
        }

        // Percentiles
        let mut percentiles = BTreeMap::new();
        for &p in &[5u32, 10, 25, 50, 75, 90, 95] {
            let idx = ((p as f32 / 100.0) * n_f) as usize;
            let idx = idx.min(n - 1);
            percentiles.insert(p, scores[idx]);
        }

        // Rare events
        let rare_events = self.detect_rare_events(&samples);
        self.total_rare_events += rare_events.len() as u64;

        // Update EMAs
        self.avg_mean_ema = EMA_ALPHA * mean + (1.0 - EMA_ALPHA) * self.avg_mean_ema;
        self.avg_stddev_ema = EMA_ALPHA * std_dev + (1.0 - EMA_ALPHA) * self.avg_stddev_ema;

        if let Some(ci95) = confidence_intervals.iter().find(|c| (c.level - 0.95).abs() < 0.01) {
            self.avg_ci_width_ema =
                EMA_ALPHA * ci95.width + (1.0 - EMA_ALPHA) * self.avg_ci_width_ema;
        }

        // Convergence: how stable is the mean across halves
        let half = n / 2;
        if half > 0 {
            let mean1 = scores[..half].iter().sum::<f32>() / half as f32;
            let mean2 = scores[half..].iter().sum::<f32>() / (n - half) as f32;
            let conv = 1.0 - (mean1 - mean2).abs().min(1.0);
            self.convergence_ema = EMA_ALPHA * conv + (1.0 - EMA_ALPHA) * self.convergence_ema;
        }

        SimulationResult {
            simulation_id: sim_id,
            sample_count: n as u32,
            mean_score: mean,
            median_score: median,
            std_dev,
            min_score,
            max_score,
            confidence_intervals,
            rare_events,
            percentiles,
        }
    }

    /// Compute confidence interval at a given level
    pub fn confidence_interval(&self, sorted_scores: &[f32], level: f32) -> ConfidenceInterval {
        let n = sorted_scores.len();
        if n == 0 {
            return ConfidenceInterval { level, lower: 0.0, upper: 0.0, width: 0.0 };
        }
        let alpha = 1.0 - level;
        let lower_idx = ((alpha / 2.0) * n as f32) as usize;
        let upper_idx = ((1.0 - alpha / 2.0) * n as f32) as usize;
        let lower = sorted_scores[lower_idx.min(n - 1)];
        let upper = sorted_scores[upper_idx.min(n - 1)];
        ConfidenceInterval {
            level,
            lower,
            upper,
            width: upper - lower,
        }
    }

    /// Detect rare events from samples
    pub fn rare_event_detect(&self, samples: &[FutureSample]) -> Vec<RareEvent> {
        self.detect_rare_events(samples)
    }

    fn detect_rare_events(&self, samples: &[FutureSample]) -> Vec<RareEvent> {
        let mut events = Vec::new();
        let n = samples.len();
        if n == 0 {
            return events;
        }

        let rare_samples: Vec<&FutureSample> = samples.iter()
            .filter(|s| s.is_rare)
            .collect();

        if rare_samples.is_empty() {
            return events;
        }

        let probability = rare_samples.len() as f32 / n as f32;

        // Cluster rare events by score range
        let mut low_severity = 0.0f32;
        let mut high_severity = 0.0f32;
        let mut low_count = 0u32;
        let mut high_count = 0u32;

        for s in &rare_samples {
            if s.outcome_score < RARE_EVENT_THRESHOLD {
                low_severity += 1.0 - s.outcome_score;
                low_count += 1;
            } else {
                high_severity += s.outcome_score;
                high_count += 1;
            }
        }

        if low_count > 0 {
            events.push(RareEvent {
                event_id: fnv1a_hash(b"rare_low"),
                probability: low_count as f32 / n as f32,
                severity: low_severity / low_count as f32,
                sample_count: low_count,
                description_hash: fnv1a_hash(b"catastrophic_low_score"),
            });
        }

        if high_count > 0 {
            events.push(RareEvent {
                event_id: fnv1a_hash(b"rare_high"),
                probability: high_count as f32 / n as f32,
                severity: high_severity / high_count as f32,
                sample_count: high_count,
                description_hash: fnv1a_hash(b"exceptional_high_score"),
            });
        }

        let _ = probability;
        events
    }

    /// Aggregate Monte Carlo statistics
    pub fn stats(&self) -> MonteCarloStats {
        MonteCarloStats {
            total_simulations: self.total_simulations,
            total_samples: self.total_samples,
            avg_mean_score: self.avg_mean_ema,
            avg_std_dev: self.avg_stddev_ema,
            rare_events_detected: self.total_rare_events,
            avg_ci_width_95: self.avg_ci_width_ema,
            convergence_rate: self.convergence_ema,
        }
    }
}

/// Approximate square root for no_std
fn sqrt_approx(x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    // Newton-Raphson: 3 iterations from a bit-hack initial guess
    let bits = x.to_bits();
    let guess_bits = (bits >> 1) + 0x1FC0_0000;
    let mut y = f32::from_bits(guess_bits);
    y = 0.5 * (y + x / y);
    y = 0.5 * (y + x / y);
    y = 0.5 * (y + x / y);
    y
}
