// SPDX-License-Identifier: GPL-2.0
//! Apps mlock_v2 — advanced memory locking manager.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Mlock mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MlockMode {
    Lock,
    LockOnFault,
    LockAll,
    LockAllOnFault,
}

/// Munlock mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MunlockMode {
    Unlock,
    UnlockAll,
}

/// Lock region state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockRegionState {
    Pending,
    Locked,
    Faulting,
    Partial,
    Unlocked,
}

/// Locked region
#[derive(Debug, Clone)]
pub struct LockedRegion {
    pub start: u64,
    pub len: u64,
    pub mode: MlockMode,
    pub state: LockRegionState,
    pub pages_locked: u64,
    pub pages_total: u64,
    pub locked_at: u64,
    pub faults_needed: u64,
}

impl LockedRegion {
    pub fn new(start: u64, len: u64, mode: MlockMode, now: u64) -> Self {
        let pages = (len + 4095) / 4096;
        let initially_locked = if matches!(mode, MlockMode::LockOnFault | MlockMode::LockAllOnFault) { 0 } else { pages };
        Self {
            start, len, mode, state: if initially_locked == pages { LockRegionState::Locked } else { LockRegionState::Pending },
            pages_locked: initially_locked, pages_total: pages,
            locked_at: now, faults_needed: pages - initially_locked,
        }
    }

    pub fn fault_in(&mut self, pages: u64) {
        self.pages_locked = (self.pages_locked + pages).min(self.pages_total);
        self.faults_needed = self.pages_total.saturating_sub(self.pages_locked);
        self.state = if self.pages_locked == self.pages_total { LockRegionState::Locked } else { LockRegionState::Faulting };
    }

    pub fn unlock(&mut self) { self.state = LockRegionState::Unlocked; self.pages_locked = 0; }

    pub fn locked_bytes(&self) -> u64 { self.pages_locked * 4096 }

    pub fn completion(&self) -> f64 {
        if self.pages_total == 0 { return 1.0; }
        self.pages_locked as f64 / self.pages_total as f64
    }

    pub fn overlaps(&self, addr: u64, size: u64) -> bool {
        self.start < addr + size && addr < self.start + self.len
    }
}

/// Per-process mlock state
#[derive(Debug)]
pub struct ProcessMlockState {
    pub pid: u64,
    pub regions: Vec<LockedRegion>,
    pub locked_pages: u64,
    pub limit_pages: u64,
    pub all_locked: bool,
}

impl ProcessMlockState {
    pub fn new(pid: u64, limit_pages: u64) -> Self {
        Self { pid, regions: Vec::new(), locked_pages: 0, limit_pages, all_locked: false }
    }

    pub fn lock(&mut self, start: u64, len: u64, mode: MlockMode, now: u64) -> bool {
        let pages = (len + 4095) / 4096;
        if self.locked_pages + pages > self.limit_pages { return false; }
        let region = LockedRegion::new(start, len, mode, now);
        self.locked_pages += region.pages_locked;
        self.regions.push(region);
        if matches!(mode, MlockMode::LockAll | MlockMode::LockAllOnFault) { self.all_locked = true; }
        true
    }

    pub fn unlock(&mut self, start: u64, len: u64) {
        for region in &mut self.regions {
            if region.overlaps(start, len) && region.state != LockRegionState::Unlocked {
                self.locked_pages = self.locked_pages.saturating_sub(region.pages_locked);
                region.unlock();
            }
        }
    }

    pub fn locked_bytes(&self) -> u64 { self.locked_pages * 4096 }
}

/// Stats
#[derive(Debug, Clone)]
pub struct MlockV2Stats {
    pub total_processes: u32,
    pub total_regions: u32,
    pub total_locked_pages: u64,
    pub total_locked_bytes: u64,
    pub all_locked_count: u32,
}

/// Main mlock v2 manager
pub struct AppMlockV2 {
    processes: BTreeMap<u64, ProcessMlockState>,
}

impl AppMlockV2 {
    pub fn new() -> Self { Self { processes: BTreeMap::new() } }

    pub fn mlock(&mut self, pid: u64, start: u64, len: u64, mode: MlockMode, now: u64) -> bool {
        let state = self.processes.entry(pid).or_insert_with(|| ProcessMlockState::new(pid, 65536));
        state.lock(start, len, mode, now)
    }

    pub fn munlock(&mut self, pid: u64, start: u64, len: u64) {
        if let Some(state) = self.processes.get_mut(&pid) { state.unlock(start, len); }
    }

    pub fn stats(&self) -> MlockV2Stats {
        let regions: u32 = self.processes.values().map(|p| p.regions.len() as u32).sum();
        let pages: u64 = self.processes.values().map(|p| p.locked_pages).sum();
        let all_locked = self.processes.values().filter(|p| p.all_locked).count() as u32;
        MlockV2Stats {
            total_processes: self.processes.len() as u32, total_regions: regions,
            total_locked_pages: pages, total_locked_bytes: pages * 4096,
            all_locked_count: all_locked,
        }
    }
}
