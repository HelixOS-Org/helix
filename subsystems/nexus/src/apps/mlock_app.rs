// SPDX-License-Identifier: GPL-2.0
//! Apps mlock_app — memory locking management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Mlock flags
#[derive(Debug, Clone, Copy)]
pub struct MlockFlags(pub u32);

impl MlockFlags {
    pub const CURRENT: u32 = 1;
    pub const FUTURE: u32 = 2;
    pub const ONFAULT: u32 = 4;
    pub fn new() -> Self { Self(0) }
    pub fn has(&self, f: u32) -> bool { self.0 & f != 0 }
}

/// Locked region
#[derive(Debug, Clone)]
pub struct LockedRegion {
    pub start: u64,
    pub length: u64,
    pub flags: MlockFlags,
    pub locked_at: u64,
}

impl LockedRegion {
    pub fn pages(&self) -> u64 { (self.length + 4095) / 4096 }
    pub fn overlaps(&self, start: u64, len: u64) -> bool { self.start < start + len && start < self.start + self.length }
}

/// Process mlock state
#[derive(Debug)]
pub struct ProcessMlockState {
    pub pid: u64,
    pub regions: Vec<LockedRegion>,
    pub locked_bytes: u64,
    pub rlimit_memlock: u64,
    pub lock_count: u64,
    pub unlock_count: u64,
    pub lock_failures: u64,
}

impl ProcessMlockState {
    pub fn new(pid: u64, rlimit: u64) -> Self {
        Self { pid, regions: Vec::new(), locked_bytes: 0, rlimit_memlock: rlimit, lock_count: 0, unlock_count: 0, lock_failures: 0 }
    }

    pub fn mlock(&mut self, start: u64, length: u64, flags: MlockFlags, now: u64) -> bool {
        if self.locked_bytes + length > self.rlimit_memlock { self.lock_failures += 1; return false; }
        self.regions.push(LockedRegion { start, length, flags, locked_at: now });
        self.locked_bytes += length;
        self.lock_count += 1;
        true
    }

    pub fn munlock(&mut self, start: u64, length: u64) {
        if let Some(pos) = self.regions.iter().position(|r| r.start == start && r.length == length) {
            self.locked_bytes -= self.regions[pos].length;
            self.regions.remove(pos);
            self.unlock_count += 1;
        }
    }

    pub fn utilization(&self) -> f64 { if self.rlimit_memlock == 0 { 0.0 } else { self.locked_bytes as f64 / self.rlimit_memlock as f64 } }
}

/// Stats
#[derive(Debug, Clone)]
pub struct MlockAppStats {
    pub tracked_processes: u32,
    pub total_locked_bytes: u64,
    pub total_locked_regions: u32,
    pub total_locks: u64,
    pub total_unlocks: u64,
    pub avg_utilization: f64,
}

/// Main mlock app
pub struct AppMlock {
    processes: BTreeMap<u64, ProcessMlockState>,
}

impl AppMlock {
    pub fn new() -> Self { Self { processes: BTreeMap::new() } }
    pub fn register(&mut self, pid: u64, rlimit: u64) { self.processes.insert(pid, ProcessMlockState::new(pid, rlimit)); }

    pub fn mlock(&mut self, pid: u64, start: u64, length: u64, flags: MlockFlags, now: u64) -> bool {
        self.processes.get_mut(&pid).map_or(false, |p| p.mlock(start, length, flags, now))
    }

    pub fn stats(&self) -> MlockAppStats {
        let bytes: u64 = self.processes.values().map(|p| p.locked_bytes).sum();
        let regions: u32 = self.processes.values().map(|p| p.regions.len() as u32).sum();
        let locks: u64 = self.processes.values().map(|p| p.lock_count).sum();
        let unlocks: u64 = self.processes.values().map(|p| p.unlock_count).sum();
        let utils: Vec<f64> = self.processes.values().map(|p| p.utilization()).collect();
        let avg = if utils.is_empty() { 0.0 } else { utils.iter().sum::<f64>() / utils.len() as f64 };
        MlockAppStats { tracked_processes: self.processes.len() as u32, total_locked_bytes: bytes, total_locked_regions: regions, total_locks: locks, total_unlocks: unlocks, avg_utilization: avg }
    }
}
