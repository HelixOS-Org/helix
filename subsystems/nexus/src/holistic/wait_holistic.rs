// SPDX-License-Identifier: GPL-2.0
//! NEXUS Holistic — Wait (holistic wait/reap analysis)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Wait pattern classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolisticWaitPattern {
    SingleChild,
    MultiChild,
    BusyWait,
    EventDriven,
    ZombieAccumulation,
}

/// Wait analysis entry
#[derive(Debug, Clone)]
pub struct HolisticWaitEntry {
    pub waiter: u64,
    pub pattern: HolisticWaitPattern,
    pub pending_zombies: u32,
    pub avg_wait_us: u64,
    pub reap_rate: f64,
}

/// Wait holistic stats
#[derive(Debug, Clone)]
pub struct HolisticWaitStats {
    pub total_analyzed: u64,
    pub busy_waits_detected: u64,
    pub zombie_accumulations: u64,
    pub avg_reap_rate: f64,
    pub peak_zombies: u32,
    pub orphan_reaps: u64,
}

/// Manager for holistic wait analysis
pub struct HolisticWaitManager {
    entries: Vec<HolisticWaitEntry>,
    zombie_counts: BTreeMap<u64, u32>,
    wait_counts: BTreeMap<u64, u64>,
    stats: HolisticWaitStats,
}

impl HolisticWaitManager {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            zombie_counts: BTreeMap::new(),
            wait_counts: BTreeMap::new(),
            stats: HolisticWaitStats {
                total_analyzed: 0,
                busy_waits_detected: 0,
                zombie_accumulations: 0,
                avg_reap_rate: 0.0,
                peak_zombies: 0,
                orphan_reaps: 0,
            },
        }
    }

    pub fn analyze_wait(&mut self, waiter: u64, zombies: u32, wait_us: u64) -> HolisticWaitPattern {
        let count = self.wait_counts.entry(waiter).or_insert(0);
        *count += 1;
        self.zombie_counts.insert(waiter, zombies);
        if zombies > self.stats.peak_zombies {
            self.stats.peak_zombies = zombies;
        }
        let pattern = if zombies > 50 {
            self.stats.zombie_accumulations += 1;
            HolisticWaitPattern::ZombieAccumulation
        } else if wait_us < 10 && *count > 100 {
            self.stats.busy_waits_detected += 1;
            HolisticWaitPattern::BusyWait
        } else if zombies > 1 {
            HolisticWaitPattern::MultiChild
        } else {
            HolisticWaitPattern::SingleChild
        };
        let entry = HolisticWaitEntry {
            waiter,
            pattern,
            pending_zombies: zombies,
            avg_wait_us: wait_us,
            reap_rate: if wait_us > 0 { 1_000_000.0 / wait_us as f64 } else { 0.0 },
        };
        self.entries.push(entry);
        self.stats.total_analyzed += 1;
        pattern
    }

    pub fn stats(&self) -> &HolisticWaitStats {
        &self.stats
    }
}
