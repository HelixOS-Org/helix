// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Confidence Interval Engine
//!
//! Uncertainty quantification for cooperation predictions. Every trust
//! prediction, contention forecast, and sharing estimate comes with
//! calibrated confidence bounds. Tracks calibration quality and manages
//! an uncertainty budget across prediction domains.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

/// FNV-1a hash for deterministic key hashing in no_std.
fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Xorshift64 PRNG for lightweight stochastic perturbation.
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// Exponential moving average update.
fn ema_update(current: u64, new_sample: u64, alpha_num: u64, alpha_den: u64) -> u64 {
    let weighted_old = current.saturating_mul(alpha_den.saturating_sub(alpha_num));
    let weighted_new = new_sample.saturating_mul(alpha_num);
    weighted_old.saturating_add(weighted_new) / alpha_den.max(1)
}

/// A confidence interval with lower, point estimate, and upper bounds.
#[derive(Clone, Debug)]
pub struct ConfidenceInterval {
    pub lower: u64,
    pub point_estimate: u64,
    pub upper: u64,
    pub confidence_level: u64,
    pub width: u64,
}

impl ConfidenceInterval {
    fn new(lower: u64, point: u64, upper: u64, level: u64) -> Self {
        Self {
            lower,
            point_estimate: point,
            upper,
            confidence_level: level,
            width: upper.saturating_sub(lower),
        }
    }
}

/// Trust prediction with confidence interval.
#[derive(Clone, Debug)]
pub struct TrustPredictionCI {
    pub partner_id: u64,
    pub interval: ConfidenceInterval,
    pub volatility: u64,
    pub sample_count: u64,
    pub calibration_score: u64,
}

/// Contention forecast interval.
#[derive(Clone, Debug)]
pub struct ContentionInterval {
    pub resource_id: u64,
    pub interval: ConfidenceInterval,
    pub trend_uncertainty: u64,
    pub historical_variance: u64,
    pub reliability: u64,
}

/// Sharing uncertainty quantification.
#[derive(Clone, Debug)]
pub struct SharingUncertainty {
    pub sharing_pair: (u64, u64),
    pub amount_interval: ConfidenceInterval,
    pub fairness_interval: ConfidenceInterval,
    pub cooperation_stability: u64,
}

/// Calibration quality metrics.
#[derive(Clone, Debug)]
pub struct CalibrationQuality {
    pub domain_hash: u64,
    pub expected_coverage: u64,
    pub actual_coverage: u64,
    pub calibration_error: u64,
    pub sharpness: u64,
    pub total_predictions: u64,
    pub well_calibrated: bool,
}

/// Result of interval narrowing analysis.
#[derive(Clone, Debug)]
pub struct IntervalNarrowing {
    pub domain_hash: u64,
    pub previous_width: u64,
    pub current_width: u64,
    pub narrowing_rate: u64,
    pub information_gain: u64,
    pub samples_needed_for_target: u64,
}

/// Uncertainty budget allocation across domains.
#[derive(Clone, Debug)]
pub struct UncertaintyBudget {
    pub total_uncertainty: u64,
    pub trust_uncertainty: u64,
    pub contention_uncertainty: u64,
    pub sharing_uncertainty: u64,
    pub dominant_source: String,
    pub reduction_priority: String,
}

/// Rolling statistics for the confidence interval engine.
#[derive(Clone, Debug)]
#[repr(align(64))]
pub struct ConfidenceIntervalStats {
    pub intervals_computed: u64,
    pub trust_intervals: u64,
    pub contention_intervals: u64,
    pub sharing_intervals: u64,
    pub calibration_checks: u64,
    pub avg_width: u64,
    pub avg_calibration_error: u64,
    pub narrowing_events: u64,
}

impl ConfidenceIntervalStats {
    pub fn new() -> Self {
        Self {
            intervals_computed: 0,
            trust_intervals: 0,
            contention_intervals: 0,
            sharing_intervals: 0,
            calibration_checks: 0,
            avg_width: 500,
            avg_calibration_error: 100,
            narrowing_events: 0,
        }
    }
}

/// Internal tracking for prediction-outcome pairs.
#[derive(Clone, Debug)]
struct PredictionOutcome {
    predicted_lower: u64,
    predicted_upper: u64,
    actual_value: u64,
    tick: u64,
    was_covered: bool,
}

/// Internal sample record for a domain.
#[derive(Clone, Debug)]
struct DomainSamples {
    domain_hash: u64,
    samples: VecDeque<u64>,
    ema_mean: u64,
    ema_variance: u64,
    prediction_outcomes: VecDeque<PredictionOutcome>,
}

/// Internal width tracking for narrowing analysis.
#[derive(Clone, Debug)]
struct WidthHistory {
    domain_hash: u64,
    widths: VecDeque<u64>,
    ema_width: u64,
}

/// Confidence interval engine for cooperation predictions.
pub struct CoopConfidenceInterval {
    trust_samples: BTreeMap<u64, DomainSamples>,
    contention_samples: BTreeMap<u64, DomainSamples>,
    sharing_samples: BTreeMap<u64, DomainSamples>,
    width_history: BTreeMap<u64, WidthHistory>,
    stats: ConfidenceIntervalStats,
    rng_state: u64,
    current_tick: u64,
    max_samples: usize,
    default_confidence: u64,
}

impl CoopConfidenceInterval {
    /// Create a new confidence interval engine.
    pub fn new(seed: u64) -> Self {
        Self {
            trust_samples: BTreeMap::new(),
            contention_samples: BTreeMap::new(),
            sharing_samples: BTreeMap::new(),
            width_history: BTreeMap::new(),
            stats: ConfidenceIntervalStats::new(),
            rng_state: seed ^ 0xC1C1_C1C1_0000_BEEF,
            current_tick: 0,
            max_samples: 256,
            default_confidence: 950,
        }
    }

    /// Record a trust observation for a partner.
    pub fn record_trust_sample(&mut self, partner_id: u64, trust_value: u64) {
        self.add_sample(&mut self.trust_samples.clone(), partner_id, trust_value);
        let domain = self.trust_samples.entry(partner_id).or_insert_with(|| DomainSamples {
            domain_hash: partner_id,
            samples: VecDeque::new(),
            ema_mean: trust_value,
            ema_variance: 100,
            prediction_outcomes: VecDeque::new(),
        });
        domain.samples.push(trust_value);
        domain.ema_mean = ema_update(domain.ema_mean, trust_value, 200, 1000);
        let diff = if trust_value > domain.ema_mean {
            trust_value - domain.ema_mean
        } else {
            domain.ema_mean - trust_value
        };
        domain.ema_variance = ema_update(domain.ema_variance, diff.saturating_mul(diff), 150, 1000);
        if domain.samples.len() > self.max_samples {
            domain.samples.pop_front().unwrap();
        }
    }

    /// Record a contention observation for a resource.
    pub fn record_contention_sample(&mut self, resource_id: u64, contention_value: u64) {
        let domain = self.contention_samples.entry(resource_id).or_insert_with(|| DomainSamples {
            domain_hash: resource_id,
            samples: VecDeque::new(),
            ema_mean: contention_value,
            ema_variance: 100,
            prediction_outcomes: VecDeque::new(),
        });
        domain.samples.push(contention_value);
        domain.ema_mean = ema_update(domain.ema_mean, contention_value, 200, 1000);
        let diff = if contention_value > domain.ema_mean {
            contention_value - domain.ema_mean
        } else {
            domain.ema_mean - contention_value
        };
        domain.ema_variance = ema_update(domain.ema_variance, diff.saturating_mul(diff), 150, 1000);
        if domain.samples.len() > self.max_samples {
            domain.samples.pop_front().unwrap();
        }
    }

    /// Record a sharing observation for a pair of processes.
    pub fn record_sharing_sample(&mut self, pair_hash: u64, sharing_value: u64) {
        let domain = self.sharing_samples.entry(pair_hash).or_insert_with(|| DomainSamples {
            domain_hash: pair_hash,
            samples: VecDeque::new(),
            ema_mean: sharing_value,
            ema_variance: 100,
            prediction_outcomes: VecDeque::new(),
        });
        domain.samples.push(sharing_value);
        domain.ema_mean = ema_update(domain.ema_mean, sharing_value, 200, 1000);
        let diff = if sharing_value > domain.ema_mean {
            sharing_value - domain.ema_mean
        } else {
            domain.ema_mean - sharing_value
        };
        domain.ema_variance = ema_update(domain.ema_variance, diff.saturating_mul(diff), 150, 1000);
        if domain.samples.len() > self.max_samples {
            domain.samples.pop_front().unwrap();
        }
    }

    /// Compute a trust prediction confidence interval for a partner.
    pub fn trust_prediction_ci(&mut self, partner_id: u64) -> TrustPredictionCI {
        let domain = self.trust_samples.get(&partner_id);
        let (interval, sample_count) = self.compute_interval(domain);

        let volatility = domain.map(|d| {
            let isqrt = integer_sqrt(d.ema_variance);
            isqrt.min(1000)
        }).unwrap_or(500);

        let calibration = self.domain_calibration_score(
            domain.map(|d| &d.prediction_outcomes[..]).unwrap_or(&[]),
        );

        self.track_width(partner_id, interval.width);
        self.stats.trust_intervals = self.stats.trust_intervals.saturating_add(1);
        self.stats.intervals_computed = self.stats.intervals_computed.saturating_add(1);
        self.stats.avg_width = ema_update(self.stats.avg_width, interval.width, 150, 1000);

        TrustPredictionCI {
            partner_id,
            interval,
            volatility,
            sample_count,
            calibration_score: calibration,
        }
    }

    /// Compute a contention forecast interval for a resource.
    pub fn contention_interval(&mut self, resource_id: u64) -> ContentionInterval {
        let domain = self.contention_samples.get(&resource_id);
        let (interval, _sample_count) = self.compute_interval(domain);

        let trend_uncertainty = domain.map(|d| {
            if d.samples.len() >= 3 {
                let n = d.samples.len();
                let recent = d.samples[n - 1];
                let mid = d.samples[n - 2];
                let older = d.samples[n - 3];
                let d1 = if recent > mid { recent - mid } else { mid - recent };
                let d2 = if mid > older { mid - older } else { older - mid };
                if d1 > d2 { d1 - d2 } else { d2 - d1 }
            } else {
                200
            }
        }).unwrap_or(300);

        let hist_var = domain.map(|d| integer_sqrt(d.ema_variance).min(1000)).unwrap_or(500);
        let reliability = 1000u64.saturating_sub(trend_uncertainty.min(1000));

        self.track_width(resource_id, interval.width);
        self.stats.contention_intervals = self.stats.contention_intervals.saturating_add(1);
        self.stats.intervals_computed = self.stats.intervals_computed.saturating_add(1);

        ContentionInterval {
            resource_id,
            interval,
            trend_uncertainty,
            historical_variance: hist_var,
            reliability,
        }
    }

    /// Quantify sharing uncertainty for a process pair.
    pub fn sharing_uncertainty(&mut self, sharer: u64, receiver: u64) -> SharingUncertainty {
        let pair_hash = fnv1a_hash(&[
            sharer.to_le_bytes().as_slice(),
            receiver.to_le_bytes().as_slice(),
        ].concat());

        let domain = self.sharing_samples.get(&pair_hash);
        let (amount_interval, _) = self.compute_interval(domain);

        let fairness_domain = self.sharing_samples.get(&fnv1a_hash(&[
            b"fair_",
            pair_hash.to_le_bytes().as_slice(),
        ].concat()));
        let (fairness_interval, _) = self.compute_interval(fairness_domain);

        let stability = domain.map(|d| {
            if d.samples.len() < 3 {
                return 300;
            }
            let n = d.samples.len();
            let mut changes: u64 = 0;
            for i in 1..n {
                let delta = if d.samples[i] > d.samples[i - 1] {
                    d.samples[i] - d.samples[i - 1]
                } else {
                    d.samples[i - 1] - d.samples[i]
                };
                changes = changes.saturating_add(delta);
            }
            let avg_change = changes / (n as u64 - 1).max(1);
            1000u64.saturating_sub(avg_change.min(1000))
        }).unwrap_or(500);

        self.stats.sharing_intervals = self.stats.sharing_intervals.saturating_add(1);
        self.stats.intervals_computed = self.stats.intervals_computed.saturating_add(1);

        SharingUncertainty {
            sharing_pair: (sharer, receiver),
            amount_interval,
            fairness_interval,
            cooperation_stability: stability,
        }
    }

    /// Assess calibration quality for a prediction domain.
    pub fn calibration_quality(&mut self, domain_hash: u64) -> CalibrationQuality {
        let outcomes = self.get_all_outcomes(domain_hash);

        let total = outcomes.len() as u64;
        let covered = outcomes.iter().filter(|o| o.was_covered).count() as u64;

        let expected_coverage = self.default_confidence;
        let actual_coverage = if total > 0 {
            covered.saturating_mul(1000) / total
        } else {
            0
        };

        let calibration_error = if actual_coverage > expected_coverage {
            actual_coverage - expected_coverage
        } else {
            expected_coverage - actual_coverage
        };

        let sharpness = self.width_history.get(&domain_hash)
            .map(|wh| 1000u64.saturating_sub(wh.ema_width.min(1000)))
            .unwrap_or(0);

        let well_calibrated = calibration_error < 100 && total >= 10;

        self.stats.calibration_checks = self.stats.calibration_checks.saturating_add(1);
        self.stats.avg_calibration_error = ema_update(
            self.stats.avg_calibration_error,
            calibration_error,
            200,
            1000,
        );

        CalibrationQuality {
            domain_hash,
            expected_coverage,
            actual_coverage,
            calibration_error,
            sharpness,
            total_predictions: total,
            well_calibrated,
        }
    }

    /// Analyze interval narrowing trend for a domain.
    pub fn interval_narrowing(&mut self, domain_hash: u64) -> IntervalNarrowing {
        let wh = self.width_history.get(&domain_hash);

        let (prev_width, curr_width) = match wh {
            Some(h) if h.widths.len() >= 2 => {
                let n = h.widths.len();
                (h.widths[n - 2], h.widths[n - 1])
            }
            Some(h) if !h.widths.is_empty() => (h.widths[0], h.widths[0]),
            _ => (500, 500),
        };

        let narrowing_rate = if prev_width > curr_width {
            (prev_width - curr_width).saturating_mul(1000) / prev_width.max(1)
        } else {
            0
        };

        let info_gain = narrowing_rate.saturating_mul(100) / 1000;

        let samples_count = self.trust_samples.get(&domain_hash)
            .map(|d| d.samples.len() as u64)
            .or_else(|| self.contention_samples.get(&domain_hash).map(|d| d.samples.len() as u64))
            .unwrap_or(0);

        let target_width: u64 = 100;
        let samples_needed = if curr_width > target_width && narrowing_rate > 0 {
            let remaining = curr_width.saturating_sub(target_width);
            let rate_per_sample = narrowing_rate.max(1);
            remaining.saturating_mul(1000) / rate_per_sample
        } else {
            0
        };

        self.stats.narrowing_events = self.stats.narrowing_events.saturating_add(1);

        IntervalNarrowing {
            domain_hash,
            previous_width: prev_width,
            current_width: curr_width,
            narrowing_rate,
            information_gain: info_gain,
            samples_needed_for_target: samples_needed.saturating_add(samples_count),
        }
    }

    /// Compute uncertainty budget across all prediction domains.
    pub fn uncertainty_budget(&self) -> UncertaintyBudget {
        let trust_unc = self.aggregate_uncertainty(&self.trust_samples);
        let contention_unc = self.aggregate_uncertainty(&self.contention_samples);
        let sharing_unc = self.aggregate_uncertainty(&self.sharing_samples);

        let total = trust_unc.saturating_add(contention_unc).saturating_add(sharing_unc);

        let (dominant, priority) = if trust_unc >= contention_unc && trust_unc >= sharing_unc {
            (String::from("trust"), String::from("trust"))
        } else if contention_unc >= sharing_unc {
            (String::from("contention"), String::from("contention"))
        } else {
            (String::from("sharing"), String::from("sharing"))
        };

        UncertaintyBudget {
            total_uncertainty: total,
            trust_uncertainty: trust_unc,
            contention_uncertainty: contention_unc,
            sharing_uncertainty: sharing_unc,
            dominant_source: dominant,
            reduction_priority: priority,
        }
    }

    /// Record a prediction outcome for calibration tracking.
    pub fn record_outcome(&mut self, domain_hash: u64, predicted_lower: u64, predicted_upper: u64, actual: u64) {
        let was_covered = actual >= predicted_lower && actual <= predicted_upper;

        let outcome = PredictionOutcome {
            predicted_lower,
            predicted_upper,
            actual_value: actual,
            tick: self.current_tick,
            was_covered,
        };

        if let Some(domain) = self.trust_samples.get_mut(&domain_hash) {
            domain.prediction_outcomes.push(outcome.clone());
            if domain.prediction_outcomes.len() > self.max_samples {
                domain.prediction_outcomes.pop_front().unwrap();
            }
        }
        if let Some(domain) = self.contention_samples.get_mut(&domain_hash) {
            domain.prediction_outcomes.push(outcome.clone());
            if domain.prediction_outcomes.len() > self.max_samples {
                domain.prediction_outcomes.pop_front().unwrap();
            }
        }
    }

    /// Advance the internal tick.
    #[inline(always)]
    pub fn tick(&mut self) {
        self.current_tick = self.current_tick.wrapping_add(1);
    }

    /// Retrieve current statistics.
    #[inline(always)]
    pub fn stats(&self) -> &ConfidenceIntervalStats {
        &self.stats
    }

    // ── Private helpers ──────────────────────────────────────────────

    fn compute_interval(&self, domain: Option<&DomainSamples>) -> (ConfidenceInterval, u64) {
        match domain {
            Some(d) if !d.samples.is_empty() => {
                let n = d.samples.len() as u64;
                let mean = d.ema_mean;
                let std_est = integer_sqrt(d.ema_variance).max(1);

                let z_factor: u64 = 196;
                let margin = std_est.saturating_mul(z_factor) / 100;
                let sqrt_n = integer_sqrt(n).max(1);
                let adjusted_margin = margin / sqrt_n;

                let lower = mean.saturating_sub(adjusted_margin);
                let upper = mean.saturating_add(adjusted_margin).min(1000);

                (ConfidenceInterval::new(lower, mean, upper, self.default_confidence), n)
            }
            _ => {
                (ConfidenceInterval::new(100, 500, 900, 500), 0)
            }
        }
    }

    fn add_sample(&self, _map: &BTreeMap<u64, DomainSamples>, _id: u64, _val: u64) {
        // Intentionally a no-op; actual insertion is done inline
    }

    fn track_width(&mut self, domain_hash: u64, width: u64) {
        let wh = self.width_history.entry(domain_hash).or_insert_with(|| WidthHistory {
            domain_hash,
            widths: VecDeque::new(),
            ema_width: width,
        });
        wh.widths.push(width);
        wh.ema_width = ema_update(wh.ema_width, width, 200, 1000);
        if wh.widths.len() > 128 {
            wh.widths.pop_front().unwrap();
        }
    }

    fn domain_calibration_score(&self, outcomes: &[PredictionOutcome]) -> u64 {
        if outcomes.is_empty() {
            return 500;
        }
        let covered = outcomes.iter().filter(|o| o.was_covered).count() as u64;
        let total = outcomes.len() as u64;
        covered.saturating_mul(1000) / total
    }

    fn get_all_outcomes(&self, domain_hash: u64) -> Vec<PredictionOutcome> {
        self.trust_samples.get(&domain_hash)
            .map(|d| d.prediction_outcomes.clone())
            .or_else(|| self.contention_samples.get(&domain_hash).map(|d| d.prediction_outcomes.clone()))
            .or_else(|| self.sharing_samples.get(&domain_hash).map(|d| d.prediction_outcomes.clone()))
            .unwrap_or_default()
    }

    fn aggregate_uncertainty(&self, map: &BTreeMap<u64, DomainSamples>) -> u64 {
        if map.is_empty() {
            return 500;
        }
        let total_var: u64 = map.values().map(|d| integer_sqrt(d.ema_variance)).sum();
        total_var / map.len().max(1) as u64
    }
}

/// Integer square root (Babylonian method) for no_std.
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
