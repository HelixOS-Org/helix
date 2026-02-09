// SPDX-License-Identifier: MIT
//! # Holistic Memory Lock Analysis
//!
//! System-wide mlock optimization:
//! - Global locked page budget enforcement
//! - Cross-process lock fairness scoring
//! - System-wide lock pressure monitoring
//! - Lock efficiency analysis (are locked pages actually accessed?)
//! - Auto-unlock policy for system-wide memory pressure

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockEfficiency { Excellent, Good, Marginal, Wasteful }

impl LockEfficiency {
    pub fn from_access_rate(rate: f64) -> Self {
        if rate > 0.8 { Self::Excellent }
        else if rate > 0.5 { Self::Good }
        else if rate > 0.2 { Self::Marginal }
        else { Self::Wasteful }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessLockProfile {
    pub pid: u64,
    pub locked_pages: u64,
    pub access_rate: f64,       // fraction of locked pages accessed recently
    pub lock_hold_time: u64,    // average ns held
    pub efficiency: LockEfficiency,
    pub denied_requests: u64,
}

impl ProcessLockProfile {
    pub fn wasted_pages(&self) -> u64 {
        ((1.0 - self.access_rate) * self.locked_pages as f64) as u64
    }
}

#[derive(Debug, Clone)]
pub struct LockFairness {
    pub gini_coefficient: f64,
    pub max_hog_pid: u64,
    pub max_hog_pages: u64,
    pub median_locked: u64,
}

#[derive(Debug, Clone, Default)]
pub struct MlockHolisticStats {
    pub total_locked_pages: u64,
    pub total_budget_pages: u64,
    pub lock_pressure: f64,
    pub wasted_locked_pages: u64,
    pub auto_unlocks_triggered: u64,
    pub fairness_score: f64,
}

pub struct MlockHolisticManager {
    profiles: BTreeMap<u64, ProcessLockProfile>,
    system_budget: u64,
    pressure_threshold: f64,
    stats: MlockHolisticStats,
}

impl MlockHolisticManager {
    pub fn new(system_budget: u64, pressure_threshold: f64) -> Self {
        Self {
            profiles: BTreeMap::new(),
            system_budget,
            pressure_threshold,
            stats: MlockHolisticStats {
                total_budget_pages: system_budget,
                ..Default::default()
            },
        }
    }

    pub fn update_profile(&mut self, profile: ProcessLockProfile) {
        self.profiles.insert(profile.pid, profile);
        self.recompute();
    }

    fn recompute(&mut self) {
        self.stats.total_locked_pages = self.profiles.values()
            .map(|p| p.locked_pages).sum();
        self.stats.wasted_locked_pages = self.profiles.values()
            .map(|p| p.wasted_pages()).sum();
        self.stats.lock_pressure = self.stats.total_locked_pages as f64
            / self.system_budget.max(1) as f64;
        self.stats.fairness_score = self.compute_fairness().gini_coefficient;
    }

    /// Compute Gini coefficient for lock distribution fairness
    fn compute_fairness(&self) -> LockFairness {
        let mut values: Vec<u64> = self.profiles.values()
            .map(|p| p.locked_pages).collect();
        values.sort();

        let n = values.len();
        if n == 0 {
            return LockFairness {
                gini_coefficient: 0.0, max_hog_pid: 0,
                max_hog_pages: 0, median_locked: 0,
            };
        }

        let sum: f64 = values.iter().map(|&v| v as f64).sum();
        if sum == 0.0 {
            return LockFairness {
                gini_coefficient: 0.0, max_hog_pid: 0,
                max_hog_pages: 0, median_locked: 0,
            };
        }

        let mut numerator = 0.0f64;
        for (i, &v) in values.iter().enumerate() {
            numerator += (2.0 * (i + 1) as f64 - n as f64 - 1.0) * v as f64;
        }
        let gini = numerator / (n as f64 * sum);

        let (max_pid, max_pages) = self.profiles.iter()
            .max_by_key(|(_, p)| p.locked_pages)
            .map(|(&pid, p)| (pid, p.locked_pages))
            .unwrap_or((0, 0));

        let median = if n > 0 { values[n / 2] } else { 0 };

        LockFairness {
            gini_coefficient: gini.abs(),
            max_hog_pid: max_pid,
            max_hog_pages: max_pages,
            median_locked: median,
        }
    }

    /// Find processes with wasteful locks (should auto-unlock)
    pub fn auto_unlock_candidates(&self) -> Vec<(u64, u64)> {
        if self.stats.lock_pressure < self.pressure_threshold {
            return Vec::new();
        }
        let mut candidates: Vec<_> = self.profiles.iter()
            .filter(|(_, p)| matches!(p.efficiency, LockEfficiency::Wasteful | LockEfficiency::Marginal))
            .map(|(&pid, p)| (pid, p.wasted_pages()))
            .collect();
        candidates.sort_by(|a, b| b.1.cmp(&a.1));
        candidates
    }

    /// Is system under lock pressure?
    pub fn under_pressure(&self) -> bool {
        self.stats.lock_pressure > self.pressure_threshold
    }

    /// Recommend lock budget adjustment for a process
    pub fn recommended_budget(&self, pid: u64) -> Option<u64> {
        let profile = self.profiles.get(&pid)?;
        // Recommend based on access rate: only lock what you use
        Some(((profile.locked_pages as f64 * profile.access_rate) * 1.2) as u64)
    }

    pub fn profile(&self, pid: u64) -> Option<&ProcessLockProfile> { self.profiles.get(&pid) }
    pub fn stats(&self) -> &MlockHolisticStats { &self.stats }
}
