// SPDX-License-Identifier: GPL-2.0
//! NEXUS Holistic — Nice (holistic scheduling priority analysis)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Scheduling fairness level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolisticNiceFairness {
    Fair,
    SlightBias,
    Unfair,
    Starving,
}

/// Nice analysis entry
#[derive(Debug, Clone)]
pub struct HolisticNiceEntry {
    pub pid: u64,
    pub nice: i32,
    pub actual_cpu_share: f64,
    pub expected_share: f64,
    pub fairness: HolisticNiceFairness,
}

/// Nice holistic stats
#[derive(Debug, Clone)]
pub struct HolisticNiceStats {
    pub total_analyzed: u64,
    pub fair_processes: u64,
    pub unfair_processes: u64,
    pub starving_processes: u64,
    pub avg_deviation: f64,
    pub nice_distribution: [u32; 40],
}

/// Manager for holistic nice analysis
pub struct HolisticNiceManager {
    entries: BTreeMap<u64, HolisticNiceEntry>,
    stats: HolisticNiceStats,
}

impl HolisticNiceManager {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            stats: HolisticNiceStats {
                total_analyzed: 0,
                fair_processes: 0,
                unfair_processes: 0,
                starving_processes: 0,
                avg_deviation: 0.0,
                nice_distribution: [0; 40],
            },
        }
    }

    pub fn analyze_process(&mut self, pid: u64, nice: i32, actual_share: f64, expected_share: f64) -> HolisticNiceFairness {
        let deviation = if expected_share > 0.0 {
            libm::fabs(actual_share - expected_share) / expected_share
        } else {
            0.0
        };
        let fairness = if actual_share < 0.001 {
            self.stats.starving_processes += 1;
            HolisticNiceFairness::Starving
        } else if deviation > 0.5 {
            self.stats.unfair_processes += 1;
            HolisticNiceFairness::Unfair
        } else if deviation > 0.2 {
            HolisticNiceFairness::SlightBias
        } else {
            self.stats.fair_processes += 1;
            HolisticNiceFairness::Fair
        };
        let idx = (nice + 20).clamp(0, 39) as usize;
        self.stats.nice_distribution[idx] += 1;
        let entry = HolisticNiceEntry {
            pid,
            nice,
            actual_cpu_share: actual_share,
            expected_share,
            fairness,
        };
        self.entries.insert(pid, entry);
        self.stats.total_analyzed += 1;
        let n = self.stats.total_analyzed as f64;
        self.stats.avg_deviation = (self.stats.avg_deviation * (n - 1.0) + deviation) / n;
        fairness
    }

    pub fn stats(&self) -> &HolisticNiceStats {
        &self.stats
    }
}
