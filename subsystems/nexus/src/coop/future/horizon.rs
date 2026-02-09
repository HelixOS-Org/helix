// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Horizon Predictor
//!
//! Long-horizon cooperation prediction engine that forecasts future resource
//! contention, cooperation demand, trust evolution, and negotiation workload
//! across cooperative subsystem boundaries.

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

/// Contention forecast for a single resource domain.
#[derive(Clone, Debug)]
pub struct ContentionForecast {
    pub resource_id: u64,
    pub predicted_pressure: u64,
    pub confidence: u64,
    pub horizon_ticks: u64,
    pub trend_direction: i64,
}

/// Trust evolution forecast for a cooperation partner.
#[derive(Clone, Debug)]
pub struct TrustForecast {
    pub partner_id: u64,
    pub current_trust: u64,
    pub predicted_trust: u64,
    pub decay_rate: u64,
    pub volatility: u64,
}

/// Demand projection for cooperation services.
#[derive(Clone, Debug)]
pub struct DemandProjection {
    pub service_hash: u64,
    pub current_demand: u64,
    pub projected_demand: u64,
    pub growth_rate: u64,
    pub saturation_point: u64,
}

/// Rolling statistics for the horizon predictor.
#[derive(Clone, Debug)]
#[repr(align(64))]
pub struct HorizonStats {
    pub total_predictions: u64,
    pub contention_forecasts: u64,
    pub trust_forecasts: u64,
    pub demand_projections: u64,
    pub avg_confidence: u64,
    pub avg_reliability: u64,
    pub horizon_errors: u64,
}

impl HorizonStats {
    pub fn new() -> Self {
        Self {
            total_predictions: 0,
            contention_forecasts: 0,
            trust_forecasts: 0,
            demand_projections: 0,
            avg_confidence: 500,
            avg_reliability: 500,
            horizon_errors: 0,
        }
    }
}

/// Internal record for tracking resource contention history.
#[derive(Clone, Debug)]
struct ContentionRecord {
    resource_id: u64,
    pressure_history: VecDeque<u64>,
    ema_pressure: u64,
    peak_pressure: u64,
    last_tick: u64,
}

/// Internal record for partner trust tracking.
#[derive(Clone, Debug)]
struct TrustRecord {
    partner_id: u64,
    trust_history: VecDeque<u64>,
    ema_trust: u64,
    decay_factor: u64,
    interaction_count: u64,
}

/// Internal record for service demand history.
#[derive(Clone, Debug)]
struct DemandRecord {
    service_hash: u64,
    demand_history: VecDeque<u64>,
    ema_demand: u64,
    peak_demand: u64,
    growth_ema: u64,
}

/// Long-horizon cooperation prediction engine.
pub struct CoopHorizonPredictor {
    contention_map: BTreeMap<u64, ContentionRecord>,
    trust_map: BTreeMap<u64, TrustRecord>,
    demand_map: BTreeMap<u64, DemandRecord>,
    reliability_history: VecDeque<u64>,
    stats: HorizonStats,
    rng_state: u64,
    current_tick: u64,
    max_history: usize,
}

impl CoopHorizonPredictor {
    /// Create a new horizon predictor with the given seed.
    pub fn new(seed: u64) -> Self {
        Self {
            contention_map: BTreeMap::new(),
            trust_map: BTreeMap::new(),
            demand_map: BTreeMap::new(),
            reliability_history: VecDeque::new(),
            stats: HorizonStats::new(),
            rng_state: seed | 1,
            current_tick: 0,
            max_history: 64,
        }
    }

    /// Advance the internal tick counter.
    #[inline(always)]
    pub fn tick(&mut self, now: u64) {
        self.current_tick = now;
    }

    /// Record observed contention pressure for a resource.
    pub fn record_contention(&mut self, resource_id: u64, pressure: u64) {
        let key = fnv1a_hash(&resource_id.to_le_bytes());
        let record = self
            .contention_map
            .entry(key)
            .or_insert_with(|| ContentionRecord {
                resource_id,
                pressure_history: VecDeque::new(),
                ema_pressure: pressure,
                peak_pressure: pressure,
                last_tick: self.current_tick,
            });
        record.ema_pressure = ema_update(record.ema_pressure, pressure, 3, 10);
        if pressure > record.peak_pressure {
            record.peak_pressure = pressure;
        }
        if record.pressure_history.len() >= self.max_history {
            record.pressure_history.pop_front().unwrap();
        }
        record.pressure_history.push(pressure);
        record.last_tick = self.current_tick;
    }

    /// Record observed trust level for a cooperation partner.
    pub fn record_trust(&mut self, partner_id: u64, trust_level: u64) {
        let key = fnv1a_hash(&partner_id.to_le_bytes());
        let record = self.trust_map.entry(key).or_insert_with(|| TrustRecord {
            partner_id,
            trust_history: VecDeque::new(),
            ema_trust: trust_level,
            decay_factor: 50,
            interaction_count: 0,
        });
        record.ema_trust = ema_update(record.ema_trust, trust_level, 2, 10);
        record.interaction_count = record.interaction_count.saturating_add(1);
        if record.trust_history.len() >= self.max_history {
            record.trust_history.pop_front().unwrap();
        }
        record.trust_history.push(trust_level);
    }

    /// Record observed demand for a cooperation service.
    pub fn record_demand(&mut self, service_name: &str, demand: u64) {
        let key = fnv1a_hash(service_name.as_bytes());
        let record = self.demand_map.entry(key).or_insert_with(|| DemandRecord {
            service_hash: key,
            demand_history: VecDeque::new(),
            ema_demand: demand,
            peak_demand: demand,
            growth_ema: 0,
        });
        let growth = if demand > record.ema_demand {
            demand.saturating_sub(record.ema_demand)
        } else {
            0
        };
        record.growth_ema = ema_update(record.growth_ema, growth, 3, 10);
        record.ema_demand = ema_update(record.ema_demand, demand, 3, 10);
        if demand > record.peak_demand {
            record.peak_demand = demand;
        }
        if record.demand_history.len() >= self.max_history {
            record.demand_history.pop_front().unwrap();
        }
        record.demand_history.push(demand);
    }

    /// Predict future resource contention for a given horizon.
    pub fn predict_contention(
        &mut self,
        resource_id: u64,
        horizon_ticks: u64,
    ) -> ContentionForecast {
        self.stats.total_predictions = self.stats.total_predictions.saturating_add(1);
        self.stats.contention_forecasts = self.stats.contention_forecasts.saturating_add(1);

        let key = fnv1a_hash(&resource_id.to_le_bytes());
        let (predicted, confidence, trend) = if let Some(record) = self.contention_map.get(&key) {
            let trend = self.compute_trend(&record.pressure_history);
            let extrapolated = if trend >= 0 {
                record
                    .ema_pressure
                    .saturating_add((trend as u64).saturating_mul(horizon_ticks) / 100)
            } else {
                let decrease = ((-trend) as u64).saturating_mul(horizon_ticks) / 100;
                record.ema_pressure.saturating_sub(decrease)
            };
            let noise = xorshift64(&mut self.rng_state) % 50;
            let predicted = extrapolated
                .saturating_add(noise)
                .min(record.peak_pressure.saturating_mul(2));
            let conf = self.compute_confidence(record.pressure_history.len(), horizon_ticks);
            (predicted, conf, trend)
        } else {
            let noise = xorshift64(&mut self.rng_state) % 100;
            (noise, 100, 0)
        };

        self.stats.avg_confidence = ema_update(self.stats.avg_confidence, confidence, 2, 10);

        ContentionForecast {
            resource_id,
            predicted_pressure: predicted,
            confidence,
            horizon_ticks,
            trend_direction: trend,
        }
    }

    /// Forecast trust evolution for a cooperation partner.
    pub fn trust_forecast(&mut self, partner_id: u64, horizon_ticks: u64) -> TrustForecast {
        self.stats.total_predictions = self.stats.total_predictions.saturating_add(1);
        self.stats.trust_forecasts = self.stats.trust_forecasts.saturating_add(1);

        let key = fnv1a_hash(&partner_id.to_le_bytes());
        if let Some(record) = self.trust_map.get(&key) {
            let decay_amount = record.decay_factor.saturating_mul(horizon_ticks) / 1000;
            let trend = self.compute_trend(&record.trust_history);
            let base_prediction = if trend >= 0 {
                record
                    .ema_trust
                    .saturating_add((trend as u64).saturating_mul(horizon_ticks) / 100)
            } else {
                let dec = ((-trend) as u64).saturating_mul(horizon_ticks) / 100;
                record.ema_trust.saturating_sub(dec)
            };
            let predicted = base_prediction.saturating_sub(decay_amount).min(1000);
            let volatility = self.compute_volatility(&record.trust_history);
            TrustForecast {
                partner_id,
                current_trust: record.ema_trust,
                predicted_trust: predicted,
                decay_rate: record.decay_factor,
                volatility,
            }
        } else {
            TrustForecast {
                partner_id,
                current_trust: 500,
                predicted_trust: 450,
                decay_rate: 50,
                volatility: 100,
            }
        }
    }

    /// Project future cooperation demand for a named service.
    pub fn demand_projection(
        &mut self,
        service_name: &str,
        horizon_ticks: u64,
    ) -> DemandProjection {
        self.stats.total_predictions = self.stats.total_predictions.saturating_add(1);
        self.stats.demand_projections = self.stats.demand_projections.saturating_add(1);

        let key = fnv1a_hash(service_name.as_bytes());
        if let Some(record) = self.demand_map.get(&key) {
            let projected = record
                .ema_demand
                .saturating_add(record.growth_ema.saturating_mul(horizon_ticks) / 100);
            let saturation = record.peak_demand.saturating_mul(3);
            let clamped = projected.min(saturation);
            DemandProjection {
                service_hash: key,
                current_demand: record.ema_demand,
                projected_demand: clamped,
                growth_rate: record.growth_ema,
                saturation_point: saturation,
            }
        } else {
            DemandProjection {
                service_hash: key,
                current_demand: 0,
                projected_demand: 0,
                growth_rate: 0,
                saturation_point: 0,
            }
        }
    }

    /// Estimate total cooperation negotiation load at the given horizon.
    pub fn cooperation_load(&self, horizon_ticks: u64) -> u64 {
        let mut total_load: u64 = 0;
        for record in self.contention_map.values() {
            let trend = self.compute_trend(&record.pressure_history);
            let projected = if trend >= 0 {
                record
                    .ema_pressure
                    .saturating_add((trend as u64).saturating_mul(horizon_ticks) / 200)
            } else {
                let dec = ((-trend) as u64).saturating_mul(horizon_ticks) / 200;
                record.ema_pressure.saturating_sub(dec)
            };
            total_load = total_load.saturating_add(projected);
        }
        for record in self.demand_map.values() {
            let projected_demand = record
                .ema_demand
                .saturating_add(record.growth_ema.saturating_mul(horizon_ticks) / 200);
            total_load = total_load.saturating_add(projected_demand / 10);
        }
        total_load
    }

    /// Compute the overall reliability of horizon predictions.
    #[inline]
    pub fn horizon_reliability(&mut self) -> u64 {
        let history_count = self.reliability_history.len() as u64;
        if history_count == 0 {
            return self.stats.avg_reliability;
        }
        let sum: u64 = self.reliability_history.iter().sum();
        let avg = sum / history_count.max(1);
        self.stats.avg_reliability = ema_update(self.stats.avg_reliability, avg, 3, 10);
        self.stats.avg_reliability
    }

    /// Submit an actual observation to refine reliability tracking.
    pub fn submit_observation(&mut self, predicted: u64, actual: u64) {
        let error = if predicted > actual {
            predicted.saturating_sub(actual)
        } else {
            actual.saturating_sub(predicted)
        };
        let max_val = predicted.max(actual).max(1);
        let accuracy = 1000u64.saturating_sub(error.saturating_mul(1000) / max_val);
        if self.reliability_history.len() >= self.max_history {
            self.reliability_history.pop_front();
        }
        self.reliability_history.push_back(accuracy);
    }

    /// Get a snapshot of current statistics.
    #[inline(always)]
    pub fn stats(&self) -> &HorizonStats {
        &self.stats
    }

    /// Compute linear trend from a history buffer. Returns per-100-tick slope.
    fn compute_trend(&self, history: &[u64]) -> i64 {
        if history.len() < 2 {
            return 0;
        }
        let n = history.len();
        let last = history[n - 1] as i64;
        let first = history[0] as i64;
        let diff = last - first;
        diff * 100 / n as i64
    }

    /// Compute confidence based on data points and horizon distance.
    fn compute_confidence(&self, data_points: usize, horizon: u64) -> u64 {
        let data_factor = (data_points as u64).min(64).saturating_mul(15);
        let horizon_penalty = horizon.min(1000).saturating_mul(1);
        data_factor.saturating_sub(horizon_penalty).min(1000)
    }

    /// Compute volatility as mean absolute deviation from EMA.
    fn compute_volatility(&self, history: &[u64]) -> u64 {
        if history.is_empty() {
            return 0;
        }
        let sum: u64 = history.iter().sum();
        let mean = sum / history.len() as u64;
        let dev_sum: u64 = history
            .iter()
            .map(|&v| if v > mean { v - mean } else { mean - v })
            .sum();
        dev_sum / history.len() as u64
    }
}
