// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Oracle — Perfect Cooperation Prediction
//!
//! Predicts every cooperation outcome before it happens.  Maintains a
//! prediction model built on demand history, trust trajectories, and
//! contention patterns.  Continuously calibrates itself and exposes
//! reliability metrics so the rest of the kernel can weight its forecasts.

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
const MAX_PREDICTIONS: usize = 4096;
const MAX_TRUST_TRAJECTORIES: usize = 1024;
const MAX_CONFLICT_FORECASTS: usize = 512;
const HISTORY_WINDOW: usize = 128;

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

// ---------------------------------------------------------------------------
// Prediction record
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct PredictionRecord {
    pub prediction_id: u64,
    pub agent_id: u64,
    pub predicted_value: u64,
    pub actual_value: Option<u64>,
    pub certainty: u64,
    pub horizon_ticks: u64,
    pub error_ema: u64,
    pub tick_created: u64,
}

// ---------------------------------------------------------------------------
// Trust trajectory
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct TrustTrajectory {
    pub pair_id: u64,
    pub source_id: u64,
    pub target_id: u64,
    pub current_trust: u64,
    pub velocity: i64,
    pub predicted_trust: u64,
    pub history: VecDeque<u64>,
}

// ---------------------------------------------------------------------------
// Conflict forecast
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct ConflictForecast {
    pub forecast_id: u64,
    pub participants: Vec<u64>,
    pub predicted_severity: u64,
    pub probability: u64,
    pub lead_time_ticks: u64,
    pub ema_accuracy: u64,
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
#[repr(align(64))]
pub struct OracleStats {
    pub total_predictions: usize,
    pub total_trajectories: usize,
    pub total_forecasts: usize,
    pub avg_certainty: u64,
    pub avg_error: u64,
    pub reliability_score: u64,
    pub calibration_events: u64,
    pub ticks_elapsed: u64,
}

// ---------------------------------------------------------------------------
// CoopOracle
// ---------------------------------------------------------------------------

pub struct CoopOracle {
    predictions: BTreeMap<u64, PredictionRecord>,
    trajectories: BTreeMap<u64, TrustTrajectory>,
    forecasts: BTreeMap<u64, ConflictForecast>,
    demand_history: BTreeMap<u64, Vec<u64>>,
    rng_state: u64,
    tick: u64,
    calibration_count: u64,
    global_error_ema: u64,
    global_certainty_ema: u64,
    stats: OracleStats,
}

impl CoopOracle {
    pub fn new(seed: u64) -> Self {
        Self {
            predictions: BTreeMap::new(),
            trajectories: BTreeMap::new(),
            forecasts: BTreeMap::new(),
            demand_history: BTreeMap::new(),
            rng_state: seed | 1,
            tick: 0,
            calibration_count: 0,
            global_error_ema: 50,
            global_certainty_ema: 50,
            stats: OracleStats {
                total_predictions: 0,
                total_trajectories: 0,
                total_forecasts: 0,
                avg_certainty: 50,
                avg_error: 50,
                reliability_score: 50,
                calibration_events: 0,
                ticks_elapsed: 0,
            },
        }
    }

    // -- demand observation -------------------------------------------------

    #[inline]
    pub fn observe(&mut self, agent_id: u64, value: u64) {
        let history = self.demand_history.entry(agent_id).or_insert_with(Vec::new);
        history.push(value);
        if history.len() > HISTORY_WINDOW {
            history.pop_front();
        }
    }

    // -- predict cooperation ------------------------------------------------

    pub fn predict_cooperation(&mut self, agent_id: u64, horizon: u64) -> Option<PredictionRecord> {
        if self.predictions.len() >= MAX_PREDICTIONS {
            return None;
        }
        let history = self.demand_history.get(&agent_id)?;
        if history.len() < 3 {
            return None;
        }

        let predicted = self.extrapolate(history, horizon);
        let certainty = self.compute_certainty(history);

        let pid = fnv1a(
            &[
                agent_id.to_le_bytes(),
                self.tick.to_le_bytes(),
                horizon.to_le_bytes(),
            ]
            .concat(),
        );

        let record = PredictionRecord {
            prediction_id: pid,
            agent_id,
            predicted_value: predicted,
            actual_value: None,
            certainty,
            horizon_ticks: horizon,
            error_ema: self.global_error_ema,
            tick_created: self.tick,
        };

        self.predictions.insert(pid, record.clone());
        Some(record)
    }

    fn extrapolate(&self, history: &[u64], horizon: u64) -> u64 {
        let n = history.len();
        let mut sum_x = 0u64;
        let mut sum_y = 0u64;
        let mut sum_xy = 0u64;
        let mut sum_xx = 0u64;

        for (i, &y) in history.iter().enumerate() {
            let x = i as u64;
            sum_x += x;
            sum_y += y;
            sum_xy += x * y;
            sum_xx += x * x;
        }

        let n_u64 = n as u64;
        let denom = n_u64 * sum_xx - sum_x * sum_x;
        if denom == 0 {
            return sum_y / n_u64;
        }

        let slope_num = if n_u64 * sum_xy >= sum_x * sum_y {
            n_u64 * sum_xy - sum_x * sum_y
        } else {
            return sum_y / n_u64;
        };

        let intercept_num = sum_y * sum_xx - sum_x * sum_xy;

        let future_x = n_u64 + horizon;
        let predicted = (slope_num * future_x + intercept_num) / denom;
        predicted
    }

    fn compute_certainty(&self, history: &[u64]) -> u64 {
        let n = history.len() as u64;
        if n < 3 {
            return 20;
        }
        let mean = history.iter().sum::<u64>() / n;
        if mean == 0 {
            return 50;
        }
        let variance = history
            .iter()
            .map(|&v| {
                let d = abs_diff(v, mean);
                d * d
            })
            .sum::<u64>()
            / n;

        let std_dev = integer_sqrt(variance);
        let cv = std_dev * 100 / mean.max(1);
        let base_certainty = 100u64.saturating_sub(cv);

        let data_bonus = clamp(n * 2, 0, 20);
        clamp(base_certainty + data_bonus, 10, 98)
    }

    // -- outcome certainty --------------------------------------------------

    #[inline]
    pub fn outcome_certainty(&self, prediction_id: u64) -> u64 {
        match self.predictions.get(&prediction_id) {
            Some(p) => p.certainty,
            None => 0,
        }
    }

    // -- validate prediction ------------------------------------------------

    #[inline]
    pub fn validate_prediction(&mut self, prediction_id: u64, actual: u64) {
        if let Some(pred) = self.predictions.get_mut(&prediction_id) {
            pred.actual_value = Some(actual);
            let error = abs_diff(pred.predicted_value, actual);
            let error_pct = if pred.predicted_value > 0 {
                error * 100 / pred.predicted_value.max(1)
            } else if actual > 0 {
                100
            } else {
                0
            };
            pred.error_ema = ema_update(pred.error_ema, error_pct);
            self.global_error_ema = ema_update(self.global_error_ema, error_pct);
            self.calibration_count += 1;
        }
    }

    // -- conflict foresight -------------------------------------------------

    pub fn conflict_foresight(&mut self, participants: &[u64]) -> Option<ConflictForecast> {
        if self.forecasts.len() >= MAX_CONFLICT_FORECASTS || participants.len() < 2 {
            return None;
        }

        let buf: Vec<u8> = participants.iter().flat_map(|p| p.to_le_bytes()).collect();
        let fid = fnv1a(&buf);

        let mut total_demand = 0u64;
        let mut demand_count = 0u64;
        for &pid in participants {
            if let Some(hist) = self.demand_history.get(&pid) {
                if let Some(&last) = hist.last() {
                    total_demand += last;
                    demand_count += 1;
                }
            }
        }

        let avg_demand = if demand_count > 0 {
            total_demand / demand_count
        } else {
            0
        };
        let overlap_factor = clamp(participants.len() as u64 * 15, 0, 100);
        let severity = clamp((avg_demand / 10).saturating_add(overlap_factor), 0, 100);
        let probability = clamp(severity * 80 / 100, 5, 95);
        let lead_time = clamp(20u64.saturating_sub(severity / 5), 1, 20);

        let existing_acc = self
            .forecasts
            .get(&fid)
            .map(|f| f.ema_accuracy)
            .unwrap_or(50);

        let forecast = ConflictForecast {
            forecast_id: fid,
            participants: participants.to_vec(),
            predicted_severity: severity,
            probability,
            lead_time_ticks: lead_time,
            ema_accuracy: existing_acc,
        };

        self.forecasts.insert(fid, forecast.clone());
        Some(forecast)
    }

    #[inline]
    pub fn validate_forecast(&mut self, forecast_id: u64, actual_severity: u64) {
        if let Some(fc) = self.forecasts.get_mut(&forecast_id) {
            let error = abs_diff(fc.predicted_severity, actual_severity);
            let accuracy = 100u64.saturating_sub(error);
            fc.ema_accuracy = ema_update(fc.ema_accuracy, accuracy);
            self.calibration_count += 1;
        }
    }

    // -- trust trajectory ---------------------------------------------------

    pub fn trust_trajectory(
        &mut self,
        source: u64,
        target: u64,
        current_trust: u64,
    ) -> TrustTrajectory {
        let pair_id = fnv1a(&[source.to_le_bytes(), target.to_le_bytes()].concat());

        if let Some(traj) = self.trajectories.get_mut(&pair_id) {
            let prev = traj.current_trust;
            let velocity = current_trust as i64 - prev as i64;
            traj.history.push(current_trust);
            if traj.history.len() > HISTORY_WINDOW {
                traj.history.pop_front().unwrap();
            }
            traj.current_trust = current_trust;
            traj.velocity = velocity;
            let projected = (current_trust as i64 + velocity * 10).max(0).min(100) as u64;
            traj.predicted_trust = projected;
            return traj.clone();
        }

        if self.trajectories.len() >= MAX_TRUST_TRAJECTORIES {
            return TrustTrajectory {
                pair_id,
                source_id: source,
                target_id: target,
                current_trust,
                velocity: 0,
                predicted_trust: current_trust,
                history: VecDeque::new(),
            };
        }

        let traj = TrustTrajectory {
            pair_id,
            source_id: source,
            target_id: target,
            current_trust,
            velocity: 0,
            predicted_trust: current_trust,
            history: {
                let mut v = Vec::new();
                v.push(current_trust);
                v
            },
        };
        self.trajectories.insert(pair_id, traj.clone());
        traj
    }

    // -- oracle reliability -------------------------------------------------

    #[inline]
    pub fn oracle_reliability(&self) -> u64 {
        if self.calibration_count == 0 {
            return 50;
        }
        let error_inverse = 100u64.saturating_sub(self.global_error_ema);
        let data_factor = clamp(self.calibration_count, 0, 50);
        let pred_count_factor = clamp(self.predictions.len() as u64 / 2, 0, 25);
        clamp(error_inverse / 2 + data_factor + pred_count_factor, 0, 100)
    }

    // -- tick ---------------------------------------------------------------

    pub fn tick(&mut self) {
        self.tick += 1;
        let expired: Vec<u64> = self
            .predictions
            .iter()
            .filter(|(_, p)| {
                p.actual_value.is_none() && self.tick > p.tick_created + p.horizon_ticks + 5
            })
            .map(|(&id, _)| id)
            .collect();
        for id in expired {
            self.predictions.remove(&id);
        }
        self.refresh_stats();
    }

    // -- stats --------------------------------------------------------------

    fn refresh_stats(&mut self) {
        let np = self.predictions.len();
        let nt = self.trajectories.len();
        let nf = self.forecasts.len();

        let avg_cert = if np > 0 {
            self.predictions.values().map(|p| p.certainty).sum::<u64>() / np as u64
        } else {
            50
        };

        let avg_err = if np > 0 {
            self.predictions.values().map(|p| p.error_ema).sum::<u64>() / np as u64
        } else {
            50
        };

        self.stats = OracleStats {
            total_predictions: np,
            total_trajectories: nt,
            total_forecasts: nf,
            avg_certainty: avg_cert,
            avg_error: avg_err,
            reliability_score: self.oracle_reliability(),
            calibration_events: self.calibration_count,
            ticks_elapsed: self.tick,
        };
    }

    #[inline(always)]
    pub fn stats(&self) -> OracleStats {
        self.stats.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_predict_cooperation() {
        let mut oracle = CoopOracle::new(42);
        for i in 0..10 {
            oracle.observe(1, 50 + i * 3);
        }
        let pred = oracle.predict_cooperation(1, 5);
        assert!(pred.is_some());
        let p = pred.unwrap();
        assert!(p.predicted_value > 0);
        assert!(p.certainty > 0);
    }

    #[test]
    fn test_conflict_foresight() {
        let mut oracle = CoopOracle::new(7);
        oracle.observe(1, 100);
        oracle.observe(2, 120);
        let fc = oracle.conflict_foresight(&[1, 2]);
        assert!(fc.is_some());
        assert!(fc.unwrap().probability > 0);
    }

    #[test]
    fn test_trust_trajectory() {
        let mut oracle = CoopOracle::new(99);
        let t1 = oracle.trust_trajectory(1, 2, 50);
        assert_eq!(t1.velocity, 0);
        let t2 = oracle.trust_trajectory(1, 2, 60);
        assert_eq!(t2.velocity, 10);
        assert!(t2.predicted_trust > 60);
    }

    #[test]
    fn test_oracle_reliability() {
        let mut oracle = CoopOracle::new(55);
        assert_eq!(oracle.oracle_reliability(), 50);
        for i in 0..10 {
            oracle.observe(1, 50 + i);
        }
        let pred = oracle.predict_cooperation(1, 1);
        if let Some(p) = pred {
            oracle.validate_prediction(p.prediction_id, p.predicted_value);
        }
        assert!(oracle.oracle_reliability() > 50);
    }
}
