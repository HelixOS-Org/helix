// SPDX-License-Identifier: GPL-2.0
//! Coop RAID — cooperative RAID rebuild with distributed parity

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Coop RAID level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopRaidLevel {
    Raid0,
    Raid1,
    Raid5,
    Raid6,
    Raid10,
}

/// Coop RAID state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopRaidState {
    Optimal,
    Degraded,
    Rebuilding,
    Failed,
    Reshaping,
}

/// Cooperative rebuild task
#[derive(Debug, Clone)]
pub struct CoopRebuildTask {
    pub array_id: u64,
    pub source_disk: u64,
    pub target_disk: u64,
    pub progress_pct: u64,
    pub bandwidth_bps: u64,
    pub donated_bandwidth: u64,
    pub sectors_rebuilt: u64,
    pub total_sectors: u64,
}

impl CoopRebuildTask {
    pub fn new(array_id: u64, src: u64, tgt: u64, total: u64) -> Self {
        Self {
            array_id,
            source_disk: src,
            target_disk: tgt,
            progress_pct: 0,
            bandwidth_bps: 0,
            donated_bandwidth: 0,
            sectors_rebuilt: 0,
            total_sectors: total,
        }
    }

    pub fn advance(&mut self, sectors: u64) {
        self.sectors_rebuilt += sectors;
        if self.total_sectors > 0 {
            self.progress_pct = (self.sectors_rebuilt * 100) / self.total_sectors;
        }
    }

    pub fn donate_bandwidth(&mut self, bps: u64) {
        self.donated_bandwidth += bps;
    }
    pub fn is_complete(&self) -> bool {
        self.sectors_rebuilt >= self.total_sectors
    }
}

/// Coop RAID stats
#[derive(Debug, Clone)]
pub struct CoopRaidStats {
    pub total_arrays: u64,
    pub degraded: u64,
    pub active_rebuilds: u64,
    pub completed_rebuilds: u64,
    pub bandwidth_donated: u64,
}

/// Main coop RAID
#[derive(Debug)]
pub struct CoopRaid {
    pub rebuilds: BTreeMap<u64, CoopRebuildTask>,
    pub stats: CoopRaidStats,
}

impl CoopRaid {
    pub fn new() -> Self {
        Self {
            rebuilds: BTreeMap::new(),
            stats: CoopRaidStats {
                total_arrays: 0,
                degraded: 0,
                active_rebuilds: 0,
                completed_rebuilds: 0,
                bandwidth_donated: 0,
            },
        }
    }

    pub fn start_rebuild(&mut self, array_id: u64, src: u64, tgt: u64, total_sectors: u64) {
        self.stats.active_rebuilds += 1;
        self.rebuilds.insert(
            array_id,
            CoopRebuildTask::new(array_id, src, tgt, total_sectors),
        );
    }

    pub fn advance_rebuild(&mut self, array_id: u64, sectors: u64) {
        if let Some(task) = self.rebuilds.get_mut(&array_id) {
            task.advance(sectors);
            if task.is_complete() {
                self.stats.active_rebuilds -= 1;
                self.stats.completed_rebuilds += 1;
            }
        }
    }
}
