// SPDX-License-Identifier: GPL-2.0
//! NEXUS Holistic — Flock (holistic file locking analysis)

extern crate alloc;
use alloc::vec::Vec;

/// Flock holistic metric
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolisticFlockMetric {
    ContentionRate,
    WaitTime,
    DeadlockFrequency,
    LockHoldDuration,
    UpgradeDowngradeRate,
    FairnesssIndex,
}

/// Flock analysis sample
#[derive(Debug, Clone)]
pub struct HolisticFlockSample {
    pub metric: HolisticFlockMetric,
    pub value: u64,
    pub inode: u64,
    pub timestamp: u64,
}

/// Flock health assessment
#[derive(Debug, Clone)]
pub struct HolisticFlockHealth {
    pub contention_score: u64,
    pub deadlock_risk: u64,
    pub fairness_score: u64,
    pub overall: u64,
}

/// Stats for flock analysis
#[derive(Debug, Clone)]
pub struct HolisticFlockStats {
    pub samples: u64,
    pub analyses: u64,
    pub deadlock_warnings: u64,
    pub contention_alerts: u64,
}

/// Manager for flock holistic analysis
pub struct HolisticFlockManager {
    samples: Vec<HolisticFlockSample>,
    health: HolisticFlockHealth,
    stats: HolisticFlockStats,
}

impl HolisticFlockManager {
    pub fn new() -> Self {
        Self {
            samples: Vec::new(),
            health: HolisticFlockHealth {
                contention_score: 0,
                deadlock_risk: 0,
                fairness_score: 100,
                overall: 100,
            },
            stats: HolisticFlockStats {
                samples: 0,
                analyses: 0,
                deadlock_warnings: 0,
                contention_alerts: 0,
            },
        }
    }

    pub fn record(&mut self, metric: HolisticFlockMetric, value: u64, inode: u64) {
        let sample = HolisticFlockSample {
            metric,
            value,
            inode,
            timestamp: self.samples.len() as u64,
        };
        self.samples.push(sample);
        self.stats.samples += 1;
    }

    pub fn analyze(&mut self) -> &HolisticFlockHealth {
        self.stats.analyses += 1;
        let contention: Vec<&HolisticFlockSample> = self.samples.iter()
            .filter(|s| matches!(s.metric, HolisticFlockMetric::ContentionRate))
            .collect();
        if !contention.is_empty() {
            let avg: u64 = contention.iter().map(|s| s.value).sum::<u64>() / contention.len() as u64;
            self.health.contention_score = avg.min(100);
            if avg > 60 {
                self.stats.contention_alerts += 1;
            }
        }
        let deadlock: Vec<&HolisticFlockSample> = self.samples.iter()
            .filter(|s| matches!(s.metric, HolisticFlockMetric::DeadlockFrequency))
            .collect();
        if !deadlock.is_empty() {
            let avg: u64 = deadlock.iter().map(|s| s.value).sum::<u64>() / deadlock.len() as u64;
            self.health.deadlock_risk = avg.min(100);
            if avg > 0 {
                self.stats.deadlock_warnings += 1;
            }
        }
        self.health.overall = (self.health.fairness_score + (100 - self.health.contention_score) + (100 - self.health.deadlock_risk)) / 3;
        &self.health
    }

    pub fn stats(&self) -> &HolisticFlockStats {
        &self.stats
    }
}
