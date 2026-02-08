// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop — Nice (cooperative scheduling priority)

extern crate alloc;
use alloc::collections::BTreeMap;

/// Cooperative scheduling class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopSchedClass {
    Normal,
    Batch,
    Idle,
    Realtime,
    Deadline,
}

/// Nice cooperation entry
#[derive(Debug, Clone)]
pub struct CoopNiceEntry {
    pub pid: u64,
    pub nice: i32,
    pub sched_class: CoopSchedClass,
    pub cpu_weight: u32,
    pub vruntime: u64,
}

/// Nice cooperation stats
#[derive(Debug, Clone)]
pub struct CoopNiceStats {
    pub total_adjustments: u64,
    pub raises: u64,
    pub lowers: u64,
    pub class_changes: u64,
    pub cooperative_yields: u64,
}

/// Manager for cooperative nice operations
pub struct CoopNiceManager {
    entries: BTreeMap<u64, CoopNiceEntry>,
    stats: CoopNiceStats,
}

impl CoopNiceManager {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            stats: CoopNiceStats {
                total_adjustments: 0,
                raises: 0,
                lowers: 0,
                class_changes: 0,
                cooperative_yields: 0,
            },
        }
    }

    pub fn register(&mut self, pid: u64, nice: i32, class: CoopSchedClass) {
        let weight = ((20 - nice) as u32).max(1) * 50;
        let entry = CoopNiceEntry {
            pid,
            nice,
            sched_class: class,
            cpu_weight: weight,
            vruntime: 0,
        };
        self.entries.insert(pid, entry);
    }

    pub fn adjust_nice(&mut self, pid: u64, delta: i32) -> bool {
        self.stats.total_adjustments += 1;
        if let Some(e) = self.entries.get_mut(&pid) {
            let new = (e.nice + delta).clamp(-20, 19);
            if new < e.nice { self.stats.raises += 1; } else { self.stats.lowers += 1; }
            e.nice = new;
            e.cpu_weight = ((20 - new) as u32).max(1) * 50;
            true
        } else {
            false
        }
    }

    pub fn change_class(&mut self, pid: u64, class: CoopSchedClass) -> bool {
        if let Some(e) = self.entries.get_mut(&pid) {
            e.sched_class = class;
            self.stats.class_changes += 1;
            true
        } else {
            false
        }
    }

    pub fn cooperative_yield(&mut self, pid: u64) {
        if let Some(e) = self.entries.get_mut(&pid) {
            e.vruntime += 1000;
            self.stats.cooperative_yields += 1;
        }
    }

    pub fn stats(&self) -> &CoopNiceStats {
        &self.stats
    }
}
