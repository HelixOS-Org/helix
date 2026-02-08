//! # Apps Mlock Manager
//!
//! Memory locking management:
//! - mlock/munlock tracking per process
//! - RLIMIT_MEMLOCK enforcement
//! - Locked page accounting
//! - mlockall behavior tracking
//! - Hugetlb lock tracking
//! - Lock pressure monitoring

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Mlock type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MlockType {
    Mlock,
    Mlock2,
    MlockAll,
    Hugetlb,
}

/// Mlock flags for mlock2
#[derive(Debug, Clone, Copy)]
pub struct MlockFlags {
    pub bits: u32,
}

impl MlockFlags {
    pub const MLOCK_ONFAULT: u32 = 1;
    pub fn empty() -> Self { Self { bits: 0 } }
    pub fn new(bits: u32) -> Self { Self { bits } }
    pub fn is_onfault(&self) -> bool { self.bits & Self::MLOCK_ONFAULT != 0 }
}

/// Locked region
#[derive(Debug, Clone)]
pub struct LockedRegion {
    pub start: u64,
    pub length: usize,
    pub lock_type: MlockType,
    pub flags: MlockFlags,
    pub locked_ts: u64,
    pub resident_pages: u64,
    pub total_pages: u64,
}

impl LockedRegion {
    pub fn new(start: u64, len: usize, lock_type: MlockType, flags: MlockFlags, ts: u64) -> Self {
        let pages = ((len + 4095) / 4096) as u64;
        Self { start, length: len, lock_type, flags, locked_ts: ts, resident_pages: pages, total_pages: pages }
    }

    pub fn pages(&self) -> u64 { self.total_pages }
    pub fn overlaps(&self, other_start: u64, other_len: usize) -> bool {
        let end = self.start + self.length as u64;
        let other_end = other_start + other_len as u64;
        self.start < other_end && other_start < end
    }
}

/// Per-process mlock state
#[derive(Debug, Clone)]
pub struct ProcessMlockState {
    pub pid: u64,
    pub regions: Vec<LockedRegion>,
    pub locked_bytes: usize,
    pub limit_bytes: usize,
    pub mlockall_active: bool,
    pub mlockall_flags: u32,
    pub lock_count: u64,
    pub unlock_count: u64,
    pub denied_count: u64,
    pub peak_locked: usize,
}

impl ProcessMlockState {
    pub fn new(pid: u64, limit: usize) -> Self {
        Self {
            pid, regions: Vec::new(), locked_bytes: 0, limit_bytes: limit,
            mlockall_active: false, mlockall_flags: 0,
            lock_count: 0, unlock_count: 0, denied_count: 0, peak_locked: 0,
        }
    }

    pub fn mlock(&mut self, start: u64, len: usize, lock_type: MlockType, flags: MlockFlags, ts: u64) -> bool {
        if self.locked_bytes + len > self.limit_bytes {
            self.denied_count += 1;
            return false;
        }
        // Merge with existing or add new
        self.regions.push(LockedRegion::new(start, len, lock_type, flags, ts));
        self.locked_bytes += len;
        self.lock_count += 1;
        if self.locked_bytes > self.peak_locked { self.peak_locked = self.locked_bytes; }
        true
    }

    pub fn munlock(&mut self, start: u64, len: usize) -> bool {
        let before = self.regions.len();
        self.regions.retain(|r| !r.overlaps(start, len));
        if self.regions.len() < before {
            self.locked_bytes = self.regions.iter().map(|r| r.length).sum();
            self.unlock_count += 1;
            true
        } else { false }
    }

    pub fn mlockall(&mut self, flags: u32) -> bool {
        self.mlockall_active = true;
        self.mlockall_flags = flags;
        true
    }

    pub fn munlockall(&mut self) {
        self.mlockall_active = false;
        self.regions.clear();
        self.locked_bytes = 0;
        self.unlock_count += 1;
    }

    pub fn usage_ratio(&self) -> f64 {
        if self.limit_bytes == 0 { return 0.0; }
        self.locked_bytes as f64 / self.limit_bytes as f64
    }

    pub fn region_count(&self) -> usize { self.regions.len() }
}

/// Mlock manager stats
#[derive(Debug, Clone, Default)]
pub struct MlockMgrStats {
    pub tracked_processes: usize,
    pub total_locked_bytes: usize,
    pub total_regions: usize,
    pub total_locks: u64,
    pub total_unlocks: u64,
    pub total_denied: u64,
    pub mlockall_processes: usize,
    pub high_usage_processes: usize,
}

/// Apps mlock manager
pub struct AppsMlockMgr {
    processes: BTreeMap<u64, ProcessMlockState>,
    default_limit: usize,
    stats: MlockMgrStats,
}

impl AppsMlockMgr {
    pub fn new(default_limit: usize) -> Self {
        Self { processes: BTreeMap::new(), default_limit, stats: MlockMgrStats::default() }
    }

    pub fn register_process(&mut self, pid: u64, limit: Option<usize>) {
        let lim = limit.unwrap_or(self.default_limit);
        self.processes.insert(pid, ProcessMlockState::new(pid, lim));
    }

    pub fn mlock(&mut self, pid: u64, start: u64, len: usize, lock_type: MlockType, flags: MlockFlags, ts: u64) -> bool {
        let proc_state = self.processes.entry(pid).or_insert_with(|| ProcessMlockState::new(pid, self.default_limit));
        proc_state.mlock(start, len, lock_type, flags, ts)
    }

    pub fn munlock(&mut self, pid: u64, start: u64, len: usize) -> bool {
        if let Some(p) = self.processes.get_mut(&pid) { p.munlock(start, len) } else { false }
    }

    pub fn mlockall(&mut self, pid: u64, flags: u32) -> bool {
        if let Some(p) = self.processes.get_mut(&pid) { p.mlockall(flags) } else { false }
    }

    pub fn munlockall(&mut self, pid: u64) {
        if let Some(p) = self.processes.get_mut(&pid) { p.munlockall(); }
    }

    pub fn process_exit(&mut self, pid: u64) { self.processes.remove(&pid); }

    pub fn recompute(&mut self) {
        self.stats.tracked_processes = self.processes.len();
        self.stats.total_locked_bytes = self.processes.values().map(|p| p.locked_bytes).sum();
        self.stats.total_regions = self.processes.values().map(|p| p.region_count()).sum();
        self.stats.total_locks = self.processes.values().map(|p| p.lock_count).sum();
        self.stats.total_unlocks = self.processes.values().map(|p| p.unlock_count).sum();
        self.stats.total_denied = self.processes.values().map(|p| p.denied_count).sum();
        self.stats.mlockall_processes = self.processes.values().filter(|p| p.mlockall_active).count();
        self.stats.high_usage_processes = self.processes.values().filter(|p| p.usage_ratio() > 0.8).count();
    }

    pub fn process(&self, pid: u64) -> Option<&ProcessMlockState> { self.processes.get(&pid) }
    pub fn stats(&self) -> &MlockMgrStats { &self.stats }
}
