// SPDX-License-Identifier: GPL-2.0
//! # Holistic Monte Carlo Engine
//!
//! Full system Monte Carlo simulation for the NEXUS prediction framework.
//! Samples entire system futures by running thousands of stochastic trials,
//! then computes failure probability distributions, performance percentiles,
//! resource exhaustion timing, and rare event analysis.
//!
//! Where deterministic simulation gives one answer, Monte Carlo gives the
//! full probability landscape — the kernel knows not just *what* will happen,
//! but *how likely* each outcome is.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_TRIALS: usize = 2048;
const MAX_DISTRIBUTION_BINS: usize = 64;
const MAX_RARE_EVENTS: usize = 128;
const MAX_CONFIDENCE_RECORDS: usize = 256;
const DEFAULT_TRIAL_STEPS: u64 = 100;
const EMA_ALPHA: f32 = 0.10;
const RARE_EVENT_THRESHOLD: f32 = 0.05;
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

// ============================================================================
// FAILURE MODE
// ============================================================================

/// System failure mode being evaluated
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FailureMode {
    OomCrash,
    CpuStarvation,
    IoDeadlock,
    NetworkPartition,
    ThermalShutdown,
    CascadeCollapse,
    ResourceExhaustion,
}

/// Resource being tracked for exhaustion
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResourceKind {
    PhysicalMemory,
    SwapSpace,
    FileDescriptors,
    CpuBudget,
    IoBandwidth,
    NetworkBandwidth,
    DiskSpace,
}

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// Result of a single Monte Carlo trial
#[derive(Debug, Clone)]
pub struct TrialResult {
    pub trial_id: u64,
    pub steps: u64,
    pub final_cpu: f32,
    pub final_mem: f32,
    pub final_io: f32,
    pub peak_cpu: f32,
    pub peak_mem: f32,
    pub failed: bool,
    pub failure_mode: Option<FailureMode>,
    pub failure_step: Option<u64>,
}

/// Distribution bin for performance histograms
#[derive(Debug, Clone)]
pub struct DistributionBin {
    pub lower: f32,
    pub upper: f32,
    pub count: u64,
    pub frequency: f32,
}

/// Performance distribution across all trials
#[derive(Debug, Clone)]
pub struct PerformanceDistribution {
    pub dimension: String,
    pub bins: Vec<DistributionBin>,
    pub mean: f32,
    pub median: f32,
    pub p5: f32,
    pub p95: f32,
    pub std_dev: f32,
    pub sample_count: u64,
}

/// Resource exhaustion ETA estimate
#[derive(Debug, Clone)]
pub struct ExhaustionEstimate {
    pub resource: ResourceKind,
    pub eta_ticks_mean: u64,
    pub eta_ticks_p5: u64,
    pub eta_ticks_p95: u64,
    pub probability_within_horizon: f32,
    pub current_usage_pct: f32,
    pub depletion_rate: f32,
}

/// A rare event detected across Monte Carlo trials
#[derive(Debug, Clone)]
pub struct RareEvent {
    pub id: u64,
    pub description: String,
    pub probability: f32,
    pub impact: f32,
    pub expected_loss: f32,
    pub trial_count: u64,
    pub occurrence_count: u64,
}

/// Confidence bounds for a Monte Carlo estimate
#[derive(Debug, Clone)]
pub struct ConfidenceBounds {
    pub dimension: String,
    pub point_estimate: f32,
    pub lower_95: f32,
    pub upper_95: f32,
    pub lower_99: f32,
    pub upper_99: f32,
    pub sample_count: u64,
}

// ============================================================================
// STATS
// ============================================================================

/// Aggregate Monte Carlo statistics
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct MonteCarloStats {
    pub total_trials: u64,
    pub total_simulations: u64,
    pub failure_rate: f32,
    pub avg_performance: f32,
    pub rare_event_count: u64,
    pub convergence_score: f32,
    pub mean_exhaustion_eta: u64,
    pub variance_reduction: f32,
}

// ============================================================================
// HOLISTIC MONTE CARLO ENGINE
// ============================================================================

/// Full system Monte Carlo engine. Samples entire system futures and
/// computes probability distributions for all key metrics.
#[derive(Debug)]
pub struct HolisticMonteCarlo {
    trial_results: VecDeque<TrialResult>,
    distributions: BTreeMap<u64, PerformanceDistribution>,
    exhaustion_estimates: BTreeMap<u8, ExhaustionEstimate>,
    rare_events: BTreeMap<u64, RareEvent>,
    confidence_records: BTreeMap<u64, ConfidenceBounds>,
    total_trials: u64,
    total_simulations: u64,
    failure_count: u64,
    tick: u64,
    rng_state: u64,
    performance_ema: f32,
    convergence_ema: f32,
}

impl HolisticMonteCarlo {
    pub fn new() -> Self {
        Self {
            trial_results: VecDeque::new(),
            distributions: BTreeMap::new(),
            exhaustion_estimates: BTreeMap::new(),
            rare_events: BTreeMap::new(),
            confidence_records: BTreeMap::new(),
            total_trials: 0,
            total_simulations: 0,
            failure_count: 0,
            tick: 0,
            rng_state: 0xB0B7_EC41_050A_73B0,
            performance_ema: 0.5,
            convergence_ema: 0.5,
        }
    }

    /// Run a full system Monte Carlo simulation with N trials
    pub fn system_simulation(
        &mut self,
        num_trials: u32,
        steps_per_trial: u64,
        initial_cpu: f32,
        initial_mem: f32,
    ) -> Vec<TrialResult> {
        self.tick += 1;
        self.total_simulations += 1;

        let steps = steps_per_trial.min(DEFAULT_TRIAL_STEPS * 10);
        let trials = (num_trials as usize).min(MAX_TRIALS);
        let mut results = Vec::new();

        for t in 0..trials {
            let mut cpu = initial_cpu;
            let mut mem = initial_mem;
            let mut io = 50.0_f32;
            let mut peak_cpu = cpu;
            let mut peak_mem = mem;
            let mut failed = false;
            let mut failure_mode = None;
            let mut failure_step = None;

            for s in 0..steps {
                let r = xorshift64(&mut self.rng_state);
                let noise = (r % 200) as f32 / 1000.0 - 0.1;

                cpu = (cpu + noise * 15.0).clamp(0.0, 100.0);
                mem = (mem + noise * 0.03).clamp(0.0, 1.0);
                io = (io + noise * 8.0).clamp(0.0, 100.0);

                if cpu > peak_cpu {
                    peak_cpu = cpu;
                }
                if mem > peak_mem {
                    peak_mem = mem;
                }

                if mem > 0.95 && !failed {
                    failed = true;
                    failure_mode = Some(FailureMode::OomCrash);
                    failure_step = Some(s);
                    self.failure_count += 1;
                } else if cpu > 99.0 && !failed {
                    failed = true;
                    failure_mode = Some(FailureMode::CpuStarvation);
                    failure_step = Some(s);
                    self.failure_count += 1;
                }
            }

            self.total_trials += 1;

            let trial = TrialResult {
                trial_id: self.total_trials,
                steps,
                final_cpu: cpu,
                final_mem: mem,
                final_io: io,
                peak_cpu,
                peak_mem,
                failed,
                failure_mode,
                failure_step,
            };

            results.push(trial);
        }

        // Update performance EMA based on non-failed trials
        let non_failed: Vec<&TrialResult> = results.iter().filter(|r| !r.failed).collect();
        if !non_failed.is_empty() {
            let avg_perf = non_failed
                .iter()
                .map(|r| 1.0 - r.final_cpu / 100.0)
                .sum::<f32>()
                / non_failed.len() as f32;
            self.performance_ema = EMA_ALPHA * avg_perf + (1.0 - EMA_ALPHA) * self.performance_ema;
        }

        // Store results (keeping bounded)
        for r in &results {
            self.trial_results.push_back(r.clone());
        }
        while self.trial_results.len() > MAX_TRIALS {
            self.trial_results.pop_front();
        }

        results
    }

    /// Compute failure probability from recent trials
    pub fn failure_probability(&self) -> BTreeMap<u8, f32> {
        let mut mode_counts: BTreeMap<u8, u64> = BTreeMap::new();
        let total = self.trial_results.len().max(1) as f32;

        for trial in &self.trial_results {
            if let Some(mode) = trial.failure_mode {
                let key = mode as u8;
                *mode_counts.entry(key).or_insert(0) += 1;
            }
        }

        let mut probabilities = BTreeMap::new();
        for (&key, &count) in &mode_counts {
            probabilities.insert(key, count as f32 / total);
        }

        // Total failure probability
        let total_failures = self.trial_results.iter().filter(|t| t.failed).count() as f32;
        probabilities.insert(255, total_failures / total);

        probabilities
    }

    /// Build performance distribution for a given dimension
    pub fn performance_distribution(&mut self, dimension: &str) -> PerformanceDistribution {
        let values: Vec<f32> = match dimension {
            "cpu" => self.trial_results.iter().map(|t| t.final_cpu).collect(),
            "memory" => self
                .trial_results
                .iter()
                .map(|t| t.final_mem * 100.0)
                .collect(),
            "io" => self.trial_results.iter().map(|t| t.final_io).collect(),
            _ => self.trial_results.iter().map(|t| t.final_cpu).collect(),
        };

        let n = values.len().max(1);
        let mean = values.iter().sum::<f32>() / n as f32;

        let mut sorted = values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));

        let median = if sorted.is_empty() {
            0.0
        } else {
            sorted[n / 2]
        };
        let p5 = if sorted.is_empty() {
            0.0
        } else {
            sorted[n * 5 / 100]
        };
        let p95 = if sorted.is_empty() {
            0.0
        } else {
            sorted[n * 95 / 100]
        };

        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / n as f32;
        let std_dev = variance.sqrt();

        // Build histogram bins
        let min_val = sorted.first().copied().unwrap_or(0.0);
        let max_val = sorted.last().copied().unwrap_or(100.0);
        let range = (max_val - min_val).max(1.0);
        let bin_count = MAX_DISTRIBUTION_BINS.min(20);
        let bin_width = range / bin_count as f32;

        let mut bins = Vec::new();
        for b in 0..bin_count {
            let lower = min_val + b as f32 * bin_width;
            let upper = lower + bin_width;
            let count = values
                .iter()
                .filter(|&&v| v >= lower && (v < upper || (b == bin_count - 1 && v <= upper)))
                .count() as u64;

            bins.push(DistributionBin {
                lower,
                upper,
                count,
                frequency: count as f32 / n as f32,
            });
        }

        let dist = PerformanceDistribution {
            dimension: String::from(dimension),
            bins,
            mean,
            median,
            p5,
            p95,
            std_dev,
            sample_count: n as u64,
        };

        let key = fnv1a_hash(dimension.as_bytes());
        self.distributions.insert(key, dist.clone());
        dist
    }

    /// Estimate resource exhaustion timing
    pub fn resource_exhaustion_eta(
        &mut self,
        resource: ResourceKind,
        current_usage: f32,
        capacity: f32,
    ) -> ExhaustionEstimate {
        let remaining = (capacity - current_usage).max(0.0);
        let usage_pct = if capacity > 0.0 {
            current_usage / capacity
        } else {
            1.0
        };

        // Estimate depletion rate from trial data
        let depletion_samples: Vec<f32> = self
            .trial_results
            .iter()
            .map(|t| match resource {
                ResourceKind::PhysicalMemory | ResourceKind::SwapSpace => {
                    (t.final_mem - t.peak_mem * 0.8).abs()
                },
                ResourceKind::CpuBudget => (t.final_cpu - t.peak_cpu * 0.8).abs() / 100.0,
                _ => 0.01,
            })
            .collect();

        let avg_depletion = if depletion_samples.is_empty() {
            0.01
        } else {
            depletion_samples.iter().sum::<f32>() / depletion_samples.len() as f32
        };
        let rate = avg_depletion.max(0.001);

        let eta_mean = (remaining / rate) as u64;
        let eta_p5 = (remaining / (rate * 2.0)) as u64;
        let eta_p95 = (remaining / (rate * 0.5)).min(u64::MAX as f32) as u64;

        let horizon_ticks = DEFAULT_TRIAL_STEPS;
        let prob_within = if eta_mean < horizon_ticks {
            1.0
        } else {
            (horizon_ticks as f32 / eta_mean as f32).clamp(0.0, 1.0)
        };

        let estimate = ExhaustionEstimate {
            resource,
            eta_ticks_mean: eta_mean,
            eta_ticks_p5: eta_p5.min(eta_mean),
            eta_ticks_p95: eta_p95.max(eta_mean),
            probability_within_horizon: prob_within,
            current_usage_pct: usage_pct.clamp(0.0, 1.0),
            depletion_rate: rate,
        };

        self.exhaustion_estimates
            .insert(resource as u8, estimate.clone());
        estimate
    }

    /// Analyze rare events across all trials
    pub fn rare_event_analysis(&mut self) -> Vec<RareEvent> {
        let total = self.trial_results.len().max(1) as f32;
        let mut events = Vec::new();

        // High memory + high CPU simultaneous
        let extreme_load = self
            .trial_results
            .iter()
            .filter(|t| t.peak_cpu > 95.0 && t.peak_mem > 0.9)
            .count() as u64;
        let prob = extreme_load as f32 / total;
        if prob < RARE_EVENT_THRESHOLD && extreme_load > 0 {
            let id = fnv1a_hash(b"rare-extreme-load");
            events.push(RareEvent {
                id,
                description: String::from("simultaneous extreme CPU and memory load"),
                probability: prob,
                impact: 0.9,
                expected_loss: prob * 0.9,
                trial_count: self.trial_results.len() as u64,
                occurrence_count: extreme_load,
            });
        }

        // Cascade failure (both fail in same trial)
        let cascade = self
            .trial_results
            .iter()
            .filter(|t| t.failed && t.peak_cpu > 90.0 && t.peak_mem > 0.85)
            .count() as u64;
        let cascade_prob = cascade as f32 / total;
        if cascade_prob < RARE_EVENT_THRESHOLD && cascade > 0 {
            let id = fnv1a_hash(b"rare-cascade-failure");
            events.push(RareEvent {
                id,
                description: String::from("cascade failure across CPU and memory"),
                probability: cascade_prob,
                impact: 1.0,
                expected_loss: cascade_prob * 1.0,
                trial_count: self.trial_results.len() as u64,
                occurrence_count: cascade,
            });
        }

        // Early failure (within first 10% of steps)
        let early = self
            .trial_results
            .iter()
            .filter(|t| t.failure_step.map(|s| s < t.steps / 10).unwrap_or(false))
            .count() as u64;
        let early_prob = early as f32 / total;
        if early_prob < RARE_EVENT_THRESHOLD && early > 0 {
            let id = fnv1a_hash(b"rare-early-failure");
            events.push(RareEvent {
                id,
                description: String::from("early system failure within first 10% of horizon"),
                probability: early_prob,
                impact: 0.8,
                expected_loss: early_prob * 0.8,
                trial_count: self.trial_results.len() as u64,
                occurrence_count: early,
            });
        }

        for ev in &events {
            self.rare_events.insert(ev.id, ev.clone());
        }
        while self.rare_events.len() > MAX_RARE_EVENTS {
            if let Some((&oldest, _)) = self.rare_events.iter().next() {
                self.rare_events.remove(&oldest);
            }
        }

        events
    }

    /// Compute confidence bounds for key estimates
    #[inline]
    pub fn confidence_bounds(&mut self, dimension: &str) -> ConfidenceBounds {
        let values: Vec<f32> = match dimension {
            "cpu" => self.trial_results.iter().map(|t| t.final_cpu).collect(),
            "memory" => self.trial_results.iter().map(|t| t.final_mem).collect(),
            "failure_rate" => {
                let rate = self.trial_results.iter().filter(|t| t.failed).count() as f32
                    / self.trial_results.len().max(1) as f32;
                alloc::vec![rate]
            },
            _ => self.trial_results.iter().map(|t| t.final_cpu).collect(),
        };

        let n = values.len().max(1) as f32;
        let mean = values.iter().sum::<f32>() / n;
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / n;
        let std_err = (variance / n).sqrt();

        let z95 = 1.96;
        let z99 = 2.576;

        let bounds = ConfidenceBounds {
            dimension: String::from(dimension),
            point_estimate: mean,
            lower_95: (mean - z95 * std_err).max(0.0),
            upper_95: mean + z95 * std_err,
            lower_99: (mean - z99 * std_err).max(0.0),
            upper_99: mean + z99 * std_err,
            sample_count: values.len() as u64,
        };

        let key = fnv1a_hash(dimension.as_bytes());
        self.confidence_records.insert(key, bounds.clone());
        while self.confidence_records.len() > MAX_CONFIDENCE_RECORDS {
            if let Some((&oldest, _)) = self.confidence_records.iter().next() {
                self.confidence_records.remove(&oldest);
            }
        }

        // Update convergence EMA: convergence improves as std_err decreases
        let convergence = (1.0 / (1.0 + std_err * 10.0)).clamp(0.0, 1.0);
        self.convergence_ema = EMA_ALPHA * convergence + (1.0 - EMA_ALPHA) * self.convergence_ema;

        bounds
    }

    /// Gather aggregate statistics
    pub fn stats(&self) -> MonteCarloStats {
        let failure_rate = if self.total_trials > 0 {
            self.failure_count as f32 / self.total_trials as f32
        } else {
            0.0
        };

        let mean_eta = self
            .exhaustion_estimates
            .values()
            .map(|e| e.eta_ticks_mean)
            .sum::<u64>()
            / self.exhaustion_estimates.len().max(1) as u64;

        MonteCarloStats {
            total_trials: self.total_trials,
            total_simulations: self.total_simulations,
            failure_rate,
            avg_performance: self.performance_ema,
            rare_event_count: self.rare_events.len() as u64,
            convergence_score: self.convergence_ema,
            mean_exhaustion_eta: mean_eta,
            variance_reduction: self.convergence_ema * 0.8,
        }
    }
}
