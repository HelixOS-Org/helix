// SPDX-License-Identifier: MIT
//! # Holistic OOM Strategy Engine
//!
//! System-wide OOM prevention and management:
//! - Global memory pressure forecasting
//! - Early warning system before OOM conditions
//! - System-wide kill strategy optimization
//! - Memory reclaim waterfall (caches → buffers → swap → kill)
//! - Post-OOM recovery scoring

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemPressureLevel { Low, Medium, High, Critical, OOM }

impl MemPressureLevel {
    pub fn from_free_ratio(ratio: f64) -> Self {
        if ratio > 0.25 { Self::Low }
        else if ratio > 0.15 { Self::Medium }
        else if ratio > 0.05 { Self::High }
        else if ratio > 0.01 { Self::Critical }
        else { Self::OOM }
    }
    pub fn severity(&self) -> u32 {
        match self {
            Self::Low => 0, Self::Medium => 1,
            Self::High => 2, Self::Critical => 3, Self::OOM => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReclaimAction { DropDentryCache, DropPageCache, CompressAnon, SwapOut, Kill }

#[derive(Debug, Clone)]
pub struct ReclaimWaterfall {
    pub actions: Vec<(ReclaimAction, u64)>, // (action, pages_reclaimable)
    pub total_reclaimable: u64,
    pub estimated_time_ns: u64,
}

#[derive(Debug, Clone)]
pub struct PressureForecast {
    pub current_level: MemPressureLevel,
    pub predicted_level: MemPressureLevel,
    pub time_to_oom_ns: Option<u64>,
    pub consumption_rate_bps: u64,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub struct RecoveryScore {
    pub kill_count: u32,
    pub pages_recovered: u64,
    pub time_to_stable_ns: u64,
    pub collateral_damage: f64, // 0-1: how much useful work was lost
}

#[derive(Debug, Clone, Default)]
pub struct OomHolisticStats {
    pub oom_events: u64,
    pub early_warnings: u64,
    pub reclaim_cascades: u64,
    pub total_killed: u64,
    pub total_recovered_pages: u64,
    pub avg_recovery_time_ns: u64,
    pub false_alarm_rate: f64,
}

pub struct OomHolisticManager {
    /// Free page history for trend analysis
    free_history: Vec<(u64, u64)>, // (timestamp, free_pages)
    total_ram_pages: u64,
    /// Per-process kill history: pid → (timestamp, pages_recovered)
    kill_history: BTreeMap<u64, Vec<(u64, u64)>>,
    recovery_scores: Vec<RecoveryScore>,
    stats: OomHolisticStats,
}

impl OomHolisticManager {
    pub fn new(total_ram_pages: u64) -> Self {
        Self {
            free_history: Vec::new(),
            total_ram_pages,
            kill_history: BTreeMap::new(),
            recovery_scores: Vec::new(),
            stats: OomHolisticStats::default(),
        }
    }

    /// Record current free page count
    pub fn sample_free_pages(&mut self, free_pages: u64, now: u64) {
        self.free_history.push((now, free_pages));
        if self.free_history.len() > 512 { self.free_history.drain(..256); }
    }

    /// Forecast memory pressure
    pub fn forecast(&self, now: u64) -> PressureForecast {
        let current_free = self.free_history.last()
            .map(|&(_, f)| f).unwrap_or(self.total_ram_pages);
        let current_ratio = current_free as f64 / self.total_ram_pages.max(1) as f64;
        let current_level = MemPressureLevel::from_free_ratio(current_ratio);

        // Compute consumption rate from recent trend
        let (rate, confidence) = if self.free_history.len() >= 10 {
            let recent = &self.free_history[self.free_history.len() - 10..];
            let dt = recent.last().unwrap().0.saturating_sub(recent.first().unwrap().0);
            let df = recent.first().unwrap().1 as i64 - recent.last().unwrap().1 as i64;
            if dt > 0 && df > 0 {
                let rate = (df as u64 * 4096 * 1_000_000_000) / dt;
                (rate, 0.7)
            } else {
                (0, 0.3)
            }
        } else {
            (0, 0.1)
        };

        let time_to_oom = if rate > 0 {
            let pages_left = current_free.saturating_sub(self.total_ram_pages / 100);
            Some((pages_left * 4096 * 1_000_000_000) / rate.max(1))
        } else {
            None
        };

        let predicted = if let Some(ttoom) = time_to_oom {
            if ttoom < 5_000_000_000 { MemPressureLevel::Critical }
            else if ttoom < 30_000_000_000 { MemPressureLevel::High }
            else { current_level }
        } else {
            current_level
        };

        PressureForecast {
            current_level, predicted_level: predicted,
            time_to_oom_ns: time_to_oom,
            consumption_rate_bps: rate, confidence,
        }
    }

    /// Build reclaim waterfall: ordered actions to try before killing
    pub fn build_waterfall(
        &self,
        cache_pages: u64, buffer_pages: u64,
        compressible: u64, swappable: u64,
    ) -> ReclaimWaterfall {
        let mut actions = Vec::new();
        let mut total = 0u64;

        if cache_pages > 0 {
            actions.push((ReclaimAction::DropDentryCache, cache_pages / 2));
            actions.push((ReclaimAction::DropPageCache, cache_pages / 2));
            total += cache_pages;
        }
        if compressible > 0 {
            actions.push((ReclaimAction::CompressAnon, compressible));
            total += compressible;
        }
        if swappable > 0 {
            actions.push((ReclaimAction::SwapOut, swappable));
            total += swappable;
        }
        actions.push((ReclaimAction::Kill, 0));

        let est_time = cache_pages * 50 + compressible * 200 + swappable * 500;
        ReclaimWaterfall { actions, total_reclaimable: total, estimated_time_ns: est_time }
    }

    /// Record a kill event
    pub fn record_kill(&mut self, pid: u64, pages: u64, now: u64) {
        self.kill_history.entry(pid).or_insert_with(Vec::new).push((now, pages));
        self.stats.total_killed += 1;
        self.stats.total_recovered_pages += pages;
    }

    /// Record an OOM event's recovery
    pub fn record_recovery(&mut self, score: RecoveryScore) {
        self.stats.avg_recovery_time_ns = (self.stats.avg_recovery_time_ns * 7
            + score.time_to_stable_ns) / 8;
        self.recovery_scores.push(score);
        self.stats.oom_events += 1;
    }

    /// Should we issue an early warning?
    pub fn should_warn(&self, now: u64) -> bool {
        let forecast = self.forecast(now);
        forecast.predicted_level.severity() >= MemPressureLevel::High.severity()
            && forecast.confidence > 0.5
    }

    pub fn stats(&self) -> &OomHolisticStats { &self.stats }
}
