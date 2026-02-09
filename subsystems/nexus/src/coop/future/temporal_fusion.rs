// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Temporal Fusion
//!
//! Multi-horizon cooperation prediction via temporal fusion. Combines
//! short-term (next contention event), medium-term (trust trajectory),
//! and long-term (cooperation equilibrium) forecasts into a coherent
//! unified outlook with horizon alignment guarantees.

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

/// Time horizon category.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Horizon {
    ShortTerm,
    MediumTerm,
    LongTerm,
}

/// Short-term contention prediction.
#[derive(Clone, Debug)]
pub struct ImmediateContention {
    pub resource_id: u64,
    pub ticks_until_event: u64,
    pub predicted_severity: u64,
    pub involved_processes: Vec<u64>,
    pub confidence: u64,
}

/// Medium-term trust trajectory forecast.
#[derive(Clone, Debug)]
pub struct TrustTrajectory {
    pub partner_id: u64,
    pub current_trust: u64,
    pub trajectory_points: Vec<u64>,
    pub trend: i64,
    pub inflection_tick: u64,
    pub confidence: u64,
}

/// Long-term cooperation equilibrium forecast.
#[derive(Clone, Debug)]
pub struct EquilibriumForecast {
    pub equilibrium_level: u64,
    pub convergence_ticks: u64,
    pub stability_score: u64,
    pub equilibrium_type: String,
    pub confidence: u64,
}

/// Fused multi-horizon prediction result.
#[derive(Clone, Debug)]
pub struct FusedPrediction {
    pub target_id: u64,
    pub short_term_value: u64,
    pub medium_term_value: u64,
    pub long_term_value: u64,
    pub fused_value: u64,
    pub horizon_weights: (u64, u64, u64),
    pub coherence_score: u64,
    pub confidence: u64,
}

/// Horizon alignment measurement.
#[derive(Clone, Debug)]
pub struct HorizonAlignment {
    pub short_medium_align: u64,
    pub medium_long_align: u64,
    pub short_long_align: u64,
    pub overall_alignment: u64,
    pub contradiction_count: u32,
}

/// Fusion coherence metrics.
#[derive(Clone, Debug)]
pub struct FusionCoherence {
    pub temporal_consistency: u64,
    pub monotonicity_score: u64,
    pub smoothness_score: u64,
    pub anomaly_count: u32,
    pub overall_coherence: u64,
}

/// Rolling statistics for the temporal fusion engine.
#[derive(Clone, Debug)]
#[repr(align(64))]
pub struct TemporalFusionStats {
    pub fusions_performed: u64,
    pub contention_predictions: u64,
    pub trust_trajectories: u64,
    pub equilibrium_forecasts: u64,
    pub alignment_checks: u64,
    pub coherence_checks: u64,
    pub avg_coherence: u64,
    pub avg_alignment: u64,
}

impl TemporalFusionStats {
    pub fn new() -> Self {
        Self {
            fusions_performed: 0,
            contention_predictions: 0,
            trust_trajectories: 0,
            equilibrium_forecasts: 0,
            alignment_checks: 0,
            coherence_checks: 0,
            avg_coherence: 500,
            avg_alignment: 500,
        }
    }
}

/// Internal short-term signal tracking.
#[derive(Clone, Debug)]
struct ShortTermSignal {
    resource_id: u64,
    pressure_history: VecDeque<u64>,
    ema_pressure: u64,
    event_intervals: VecDeque<u64>,
    last_event_tick: u64,
}

/// Internal medium-term trajectory state.
#[derive(Clone, Debug)]
struct MediumTermState {
    partner_id: u64,
    trust_history: VecDeque<u64>,
    ema_trust: u64,
    ema_trend: i64,
    inflection_candidates: VecDeque<u64>,
}

/// Internal long-term equilibrium tracker.
#[derive(Clone, Debug)]
struct LongTermTracker {
    coop_history: VecDeque<u64>,
    ema_cooperation: u64,
    convergence_ema: u64,
    stability_window: VecDeque<u64>,
}

/// Internal horizon prediction record.
#[derive(Clone, Debug)]
struct HorizonRecord {
    target_id: u64,
    short_pred: u64,
    medium_pred: u64,
    long_pred: u64,
    fused_pred: u64,
    tick: u64,
}

/// Multi-horizon cooperation temporal fusion engine.
pub struct CoopTemporalFusion {
    short_signals: BTreeMap<u64, ShortTermSignal>,
    medium_states: BTreeMap<u64, MediumTermState>,
    long_tracker: LongTermTracker,
    history: VecDeque<HorizonRecord>,
    horizon_weights: (u64, u64, u64),
    stats: TemporalFusionStats,
    rng_state: u64,
    current_tick: u64,
    max_history: usize,
}

impl CoopTemporalFusion {
    /// Create a new temporal fusion engine.
    pub fn new(seed: u64) -> Self {
        Self {
            short_signals: BTreeMap::new(),
            medium_states: BTreeMap::new(),
            long_tracker: LongTermTracker {
                coop_history: VecDeque::new(),
                ema_cooperation: 500,
                convergence_ema: 500,
                stability_window: VecDeque::new(),
            },
            history: VecDeque::new(),
            horizon_weights: (400, 350, 250),
            stats: TemporalFusionStats::new(),
            rng_state: seed ^ 0x7E7F_FU51_0000_C00P,
            current_tick: 0,
            max_history: 256,
        }
    }

    /// Record a resource contention signal (short-term).
    pub fn record_contention_signal(&mut self, resource_id: u64, pressure: u64, processes: &[u64]) {
        let signal = self.short_signals.entry(resource_id).or_insert_with(|| ShortTermSignal {
            resource_id,
            pressure_history: VecDeque::new(),
            ema_pressure: pressure,
            event_intervals: VecDeque::new(),
            last_event_tick: self.current_tick,
        });

        signal.pressure_history.push(pressure);
        signal.ema_pressure = ema_update(signal.ema_pressure, pressure, 300, 1000);

        if pressure > 700 {
            let interval = self.current_tick.saturating_sub(signal.last_event_tick);
            signal.event_intervals.push(interval);
            signal.last_event_tick = self.current_tick;
            if signal.event_intervals.len() > 64 {
                signal.event_intervals.pop_front().unwrap();
            }
        }

        if signal.pressure_history.len() > self.max_history {
            signal.pressure_history.pop_front().unwrap();
        }
    }

    /// Record a trust observation (medium-term).
    pub fn record_trust_signal(&mut self, partner_id: u64, trust_value: u64) {
        let state = self.medium_states.entry(partner_id).or_insert_with(|| MediumTermState {
            partner_id,
            trust_history: VecDeque::new(),
            ema_trust: trust_value,
            ema_trend: 0,
            inflection_candidates: VecDeque::new(),
        });

        let prev = state.ema_trust;
        state.trust_history.push(trust_value);
        state.ema_trust = ema_update(state.ema_trust, trust_value, 150, 1000);

        let new_trend = state.ema_trust as i64 - prev as i64;
        let old_trend = state.ema_trend;
        state.ema_trend = (old_trend.saturating_mul(800) + new_trend.saturating_mul(200)) / 1000;

        if (old_trend > 0 && state.ema_trend <= 0) || (old_trend < 0 && state.ema_trend >= 0) {
            state.inflection_candidates.push(self.current_tick);
            if state.inflection_candidates.len() > 16 {
                state.inflection_candidates.pop_front().unwrap();
            }
        }

        if state.trust_history.len() > self.max_history {
            state.trust_history.pop_front().unwrap();
        }
    }

    /// Record a global cooperation level (long-term).
    pub fn record_cooperation_level(&mut self, level: u64) {
        self.long_tracker.coop_history.push(level);
        self.long_tracker.ema_cooperation = ema_update(
            self.long_tracker.ema_cooperation,
            level,
            100,
            1000,
        );

        self.long_tracker.stability_window.push(level);
        if self.long_tracker.stability_window.len() > 32 {
            self.long_tracker.stability_window.pop_front().unwrap();
        }

        let variance = self.compute_window_variance(&self.long_tracker.stability_window);
        self.long_tracker.convergence_ema = ema_update(
            self.long_tracker.convergence_ema,
            1000u64.saturating_sub(variance.min(1000)),
            100,
            1000,
        );

        if self.long_tracker.coop_history.len() > self.max_history {
            self.long_tracker.coop_history.pop_front().unwrap();
        }
    }

    /// Fuse all horizons into a single prediction for a target.
    pub fn fuse_cooperation_horizons(&mut self, target_id: u64) -> FusedPrediction {
        let short = self.predict_short_term(target_id);
        let medium = self.predict_medium_term(target_id);
        let long = self.predict_long_term();

        let (ws, wm, wl) = self.horizon_weights;
        let total_w = ws.saturating_add(wm).saturating_add(wl).max(1);
        let fused = short.saturating_mul(ws)
            .saturating_add(medium.saturating_mul(wm))
            .saturating_add(long.saturating_mul(wl))
            / total_w;

        let coherence = self.compute_horizon_coherence(short, medium, long);

        let confidence = coherence.saturating_mul(800) / 1000;

        self.history.push_back(HorizonRecord {
            target_id,
            short_pred: short,
            medium_pred: medium,
            long_pred: long,
            fused_pred: fused,
            tick: self.current_tick,
        });
        if self.history.len() > self.max_history {
            self.history.pop_front();
        }

        self.stats.fusions_performed = self.stats.fusions_performed.saturating_add(1);
        self.stats.avg_coherence = ema_update(self.stats.avg_coherence, coherence, 200, 1000);

        FusedPrediction {
            target_id,
            short_term_value: short,
            medium_term_value: medium,
            long_term_value: long,
            fused_value: fused,
            horizon_weights: self.horizon_weights,
            coherence_score: coherence,
            confidence,
        }
    }

    /// Predict immediate contention for a resource.
    pub fn immediate_contention(&mut self, resource_id: u64) -> ImmediateContention {
        let signal = self.short_signals.get(&resource_id);

        let (ticks_until, severity, confidence) = match signal {
            Some(s) => {
                let avg_interval = if s.event_intervals.is_empty() {
                    100
                } else {
                    s.event_intervals.iter().sum::<u64>()
                        / s.event_intervals.len() as u64
                };

                let since_last = self.current_tick.saturating_sub(s.last_event_tick);
                let ticks_until = avg_interval.saturating_sub(since_last);
                let severity = s.ema_pressure;
                let conf = if s.event_intervals.len() > 3 { 700 } else { 400 };

                (ticks_until, severity, conf)
            }
            None => (100, 300, 200),
        };

        self.stats.contention_predictions = self.stats.contention_predictions.saturating_add(1);

        ImmediateContention {
            resource_id,
            ticks_until_event: ticks_until,
            predicted_severity: severity,
            involved_processes: Vec::new(),
            confidence,
        }
    }

    /// Compute a trust trajectory for a partner.
    pub fn trust_trajectory(&mut self, partner_id: u64) -> TrustTrajectory {
        let state = self.medium_states.get(&partner_id);

        let (current, points, trend, inflection, confidence) = match state {
            Some(s) => {
                let current = s.ema_trust;
                let mut points = Vec::new();
                for step in 1..=10u64 {
                    let projected = current as i64 + s.ema_trend * step as i64;
                    points.push((projected.max(0) as u64).min(1000));
                }

                let inflection = s.inflection_candidates.last().copied().unwrap_or(0);
                let conf = if s.trust_history.len() > 10 { 700 } else { 400 };

                (current, points, s.ema_trend, inflection, conf)
            }
            None => (500, alloc::vec![500; 10], 0, 0, 200),
        };

        self.stats.trust_trajectories = self.stats.trust_trajectories.saturating_add(1);

        TrustTrajectory {
            partner_id,
            current_trust: current,
            trajectory_points: points,
            trend,
            inflection_tick: inflection,
            confidence,
        }
    }

    /// Forecast cooperation equilibrium.
    pub fn equilibrium_forecast(&mut self) -> EquilibriumForecast {
        let eq_level = self.long_tracker.ema_cooperation;
        let convergence = self.long_tracker.convergence_ema;

        let stability = if self.long_tracker.stability_window.len() >= 8 {
            let var = self.compute_window_variance(&self.long_tracker.stability_window);
            1000u64.saturating_sub(var.min(1000))
        } else {
            300
        };

        let convergence_ticks = if convergence > 800 {
            10
        } else if convergence > 500 {
            50
        } else {
            200
        };

        let eq_type = if stability > 800 {
            String::from("stable")
        } else if stability > 500 {
            String::from("oscillating")
        } else if convergence < 300 {
            String::from("divergent")
        } else {
            String::from("transitional")
        };

        let confidence = stability.saturating_mul(convergence) / 1000;

        self.stats.equilibrium_forecasts = self.stats.equilibrium_forecasts.saturating_add(1);

        EquilibriumForecast {
            equilibrium_level: eq_level,
            convergence_ticks,
            stability_score: stability,
            equilibrium_type: eq_type,
            confidence,
        }
    }

    /// Measure alignment between horizons.
    pub fn horizon_alignment(&mut self) -> HorizonAlignment {
        let recent: Vec<&HorizonRecord> = self.history.iter().rev().take(20).collect();

        if recent.is_empty() {
            self.stats.alignment_checks = self.stats.alignment_checks.saturating_add(1);
            return HorizonAlignment {
                short_medium_align: 500,
                medium_long_align: 500,
                short_long_align: 500,
                overall_alignment: 500,
                contradiction_count: 0,
            };
        }

        let mut sm_align_sum: u64 = 0;
        let mut ml_align_sum: u64 = 0;
        let mut sl_align_sum: u64 = 0;
        let mut contradictions: u32 = 0;

        for rec in &recent {
            let sm = self.alignment_pair(rec.short_pred, rec.medium_pred);
            let ml = self.alignment_pair(rec.medium_pred, rec.long_pred);
            let sl = self.alignment_pair(rec.short_pred, rec.long_pred);
            sm_align_sum = sm_align_sum.saturating_add(sm);
            ml_align_sum = ml_align_sum.saturating_add(ml);
            sl_align_sum = sl_align_sum.saturating_add(sl);

            if sm < 300 || ml < 300 || sl < 300 {
                contradictions += 1;
            }
        }

        let n = recent.len() as u64;
        let sm = sm_align_sum / n;
        let ml = ml_align_sum / n;
        let sl = sl_align_sum / n;
        let overall = (sm + ml + sl) / 3;

        self.stats.alignment_checks = self.stats.alignment_checks.saturating_add(1);
        self.stats.avg_alignment = ema_update(self.stats.avg_alignment, overall, 200, 1000);

        HorizonAlignment {
            short_medium_align: sm,
            medium_long_align: ml,
            short_long_align: sl,
            overall_alignment: overall,
            contradiction_count: contradictions,
        }
    }

    /// Assess fusion coherence.
    pub fn fusion_coherence(&mut self) -> FusionCoherence {
        let recent: Vec<&HorizonRecord> = self.history.iter().rev().take(30).collect();

        let temporal_consistency = self.compute_temporal_consistency(&recent);
        let monotonicity = self.compute_monotonicity(&recent);
        let smoothness = self.compute_smoothness(&recent);
        let anomaly_count = self.count_anomalies(&recent);

        let overall = (temporal_consistency + monotonicity + smoothness) / 3;

        self.stats.coherence_checks = self.stats.coherence_checks.saturating_add(1);
        self.stats.avg_coherence = ema_update(self.stats.avg_coherence, overall, 200, 1000);

        FusionCoherence {
            temporal_consistency,
            monotonicity_score: monotonicity,
            smoothness_score: smoothness,
            anomaly_count,
            overall_coherence: overall,
        }
    }

    /// Advance the internal tick.
    #[inline(always)]
    pub fn tick(&mut self) {
        self.current_tick = self.current_tick.wrapping_add(1);
    }

    /// Retrieve current statistics.
    #[inline(always)]
    pub fn stats(&self) -> &TemporalFusionStats {
        &self.stats
    }

    // ── Private helpers ──────────────────────────────────────────────

    fn predict_short_term(&self, target_id: u64) -> u64 {
        self.short_signals.get(&target_id)
            .map(|s| s.ema_pressure)
            .unwrap_or(500)
    }

    fn predict_medium_term(&self, target_id: u64) -> u64 {
        self.medium_states.get(&target_id)
            .map(|s| {
                let projected = s.ema_trust as i64 + s.ema_trend * 5;
                (projected.max(0) as u64).min(1000)
            })
            .unwrap_or(500)
    }

    fn predict_long_term(&self) -> u64 {
        self.long_tracker.ema_cooperation
    }

    fn compute_horizon_coherence(&self, short: u64, medium: u64, long: u64) -> u64 {
        let sm = self.alignment_pair(short, medium);
        let ml = self.alignment_pair(medium, long);
        let sl = self.alignment_pair(short, long);
        (sm + ml + sl) / 3
    }

    fn alignment_pair(&self, a: u64, b: u64) -> u64 {
        let diff = if a > b { a - b } else { b - a };
        1000u64.saturating_sub(diff.min(1000))
    }

    fn compute_window_variance(&self, window: &[u64]) -> u64 {
        if window.is_empty() {
            return 0;
        }
        let mean = window.iter().sum::<u64>() / window.len() as u64;
        let variance: u64 = window.iter()
            .map(|&v| {
                let d = if v > mean { v - mean } else { mean - v };
                d.saturating_mul(d)
            })
            .sum::<u64>()
            / window.len() as u64;
        integer_sqrt(variance)
    }

    fn compute_temporal_consistency(&self, records: &[&HorizonRecord]) -> u64 {
        if records.len() < 2 {
            return 500;
        }
        let mut consistency_sum: u64 = 0;
        for i in 1..records.len() {
            let diff = if records[i].fused_pred > records[i - 1].fused_pred {
                records[i].fused_pred - records[i - 1].fused_pred
            } else {
                records[i - 1].fused_pred - records[i].fused_pred
            };
            consistency_sum = consistency_sum
                .saturating_add(1000u64.saturating_sub(diff.min(1000)));
        }
        consistency_sum / (records.len() as u64 - 1).max(1)
    }

    fn compute_monotonicity(&self, records: &[&HorizonRecord]) -> u64 {
        if records.len() < 3 {
            return 500;
        }
        let mut mono_count: u64 = 0;
        for i in 2..records.len() {
            let d1 = records[i].fused_pred as i64 - records[i - 1].fused_pred as i64;
            let d2 = records[i - 1].fused_pred as i64 - records[i - 2].fused_pred as i64;
            if (d1 >= 0 && d2 >= 0) || (d1 <= 0 && d2 <= 0) {
                mono_count += 1;
            }
        }
        mono_count.saturating_mul(1000) / (records.len() as u64 - 2).max(1)
    }

    fn compute_smoothness(&self, records: &[&HorizonRecord]) -> u64 {
        if records.len() < 2 {
            return 500;
        }
        let mut total_change: u64 = 0;
        for i in 1..records.len() {
            let diff = if records[i].fused_pred > records[i - 1].fused_pred {
                records[i].fused_pred - records[i - 1].fused_pred
            } else {
                records[i - 1].fused_pred - records[i].fused_pred
            };
            total_change = total_change.saturating_add(diff);
        }
        let avg_change = total_change / (records.len() as u64 - 1).max(1);
        1000u64.saturating_sub(avg_change.min(1000))
    }

    fn count_anomalies(&self, records: &[&HorizonRecord]) -> u32 {
        let mut count: u32 = 0;
        for i in 1..records.len() {
            let diff = if records[i].fused_pred > records[i - 1].fused_pred {
                records[i].fused_pred - records[i - 1].fused_pred
            } else {
                records[i - 1].fused_pred - records[i].fused_pred
            };
            if diff > 300 {
                count += 1;
            }
        }
        count
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
