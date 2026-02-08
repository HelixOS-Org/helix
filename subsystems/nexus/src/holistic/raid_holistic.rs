// SPDX-License-Identifier: GPL-2.0
//! Holistic RAID — software RAID management with rebuild and scrub

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// RAID level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaidLevel {
    Raid0,
    Raid1,
    Raid5,
    Raid6,
    Raid10,
    Linear,
    Jbod,
}

/// RAID array state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaidState {
    Active,
    Degraded,
    Rebuilding,
    Scrubbing,
    Resyncing,
    Failed,
    Inactive,
}

/// RAID disk state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaidDiskState {
    Active,
    Spare,
    Faulty,
    Rebuilding,
    Removed,
}

/// RAID disk
#[derive(Debug, Clone)]
pub struct RaidDisk {
    pub disk_id: u64,
    pub state: RaidDiskState,
    pub sectors: u64,
    pub read_errors: u64,
    pub write_errors: u64,
    pub corrected_errors: u64,
}

impl RaidDisk {
    pub fn new(disk_id: u64, sectors: u64) -> Self {
        Self { disk_id, state: RaidDiskState::Active, sectors, read_errors: 0, write_errors: 0, corrected_errors: 0 }
    }

    pub fn fail(&mut self) { self.state = RaidDiskState::Faulty; }
    pub fn start_rebuild(&mut self) { self.state = RaidDiskState::Rebuilding; }
    pub fn total_errors(&self) -> u64 { self.read_errors + self.write_errors }
}

/// RAID array
#[derive(Debug, Clone)]
pub struct RaidArray {
    pub array_id: u64,
    pub level: RaidLevel,
    pub state: RaidState,
    pub disks: Vec<RaidDisk>,
    pub chunk_size_kb: u32,
    pub stripe_size_kb: u32,
    pub total_sectors: u64,
    pub usable_sectors: u64,
    pub rebuild_progress_pct: f64,
    pub scrub_progress_pct: f64,
    pub mismatch_count: u64,
}

impl RaidArray {
    pub fn new(array_id: u64, level: RaidLevel, chunk_size_kb: u32) -> Self {
        Self {
            array_id, level, state: RaidState::Active, disks: Vec::new(),
            chunk_size_kb, stripe_size_kb: 0, total_sectors: 0, usable_sectors: 0,
            rebuild_progress_pct: 0.0, scrub_progress_pct: 0.0, mismatch_count: 0,
        }
    }

    pub fn add_disk(&mut self, disk: RaidDisk) {
        self.total_sectors += disk.sectors;
        self.disks.push(disk);
        self.recalculate_usable();
    }

    fn recalculate_usable(&mut self) {
        let n = self.disks.iter().filter(|d| d.state == RaidDiskState::Active).count() as u64;
        if n == 0 { self.usable_sectors = 0; return; }
        let per_disk = self.disks.iter().filter(|d| d.state == RaidDiskState::Active).map(|d| d.sectors).min().unwrap_or(0);
        self.usable_sectors = match self.level {
            RaidLevel::Raid0 | RaidLevel::Linear | RaidLevel::Jbod => self.total_sectors,
            RaidLevel::Raid1 => per_disk,
            RaidLevel::Raid5 => per_disk * (n - 1),
            RaidLevel::Raid6 => per_disk * (n - 2),
            RaidLevel::Raid10 => per_disk * (n / 2),
        };
    }

    pub fn fail_disk(&mut self, idx: usize) {
        if idx < self.disks.len() {
            self.disks[idx].fail();
            self.state = RaidState::Degraded;
            self.recalculate_usable();
        }
    }

    pub fn redundancy(&self) -> u32 {
        match self.level {
            RaidLevel::Raid0 | RaidLevel::Linear | RaidLevel::Jbod => 0,
            RaidLevel::Raid1 => self.disks.len().saturating_sub(1) as u32,
            RaidLevel::Raid5 => 1,
            RaidLevel::Raid6 => 2,
            RaidLevel::Raid10 => 1,
        }
    }
}

/// RAID holistic stats
#[derive(Debug, Clone)]
pub struct HolisticRaidStats {
    pub total_arrays: u64,
    pub degraded_arrays: u64,
    pub total_disks: u64,
    pub faulty_disks: u64,
    pub rebuilds: u64,
}

/// Main holistic RAID manager
#[derive(Debug)]
pub struct HolisticRaid {
    pub arrays: BTreeMap<u64, RaidArray>,
    pub stats: HolisticRaidStats,
}

impl HolisticRaid {
    pub fn new() -> Self {
        Self {
            arrays: BTreeMap::new(),
            stats: HolisticRaidStats { total_arrays: 0, degraded_arrays: 0, total_disks: 0, faulty_disks: 0, rebuilds: 0 },
        }
    }

    pub fn create_array(&mut self, array: RaidArray) {
        self.stats.total_arrays += 1;
        self.stats.total_disks += array.disks.len() as u64;
        self.arrays.insert(array.array_id, array);
    }

    pub fn start_rebuild(&mut self, array_id: u64) -> bool {
        if let Some(array) = self.arrays.get_mut(&array_id) {
            if array.state == RaidState::Degraded {
                array.state = RaidState::Rebuilding;
                self.stats.rebuilds += 1;
                return true;
            }
        }
        false
    }
}
