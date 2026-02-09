// SPDX-License-Identifier: GPL-2.0
//! Apps madvise_v2 — advanced memory advisory hints manager.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Madvise behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MadvBehavior {
    Normal,
    Random,
    Sequential,
    WillNeed,
    DontNeed,
    Free,
    Remove,
    DontFork,
    DoFork,
    Mergeable,
    Unmergeable,
    HugePage,
    NoHugePage,
    DontDump,
    DoDump,
    WipeOnFork,
    KeepOnFork,
    Cold,
    PageOut,
    PopulateRead,
    PopulateWrite,
}

/// Advisory region
#[derive(Debug, Clone)]
pub struct MadvRegion {
    pub start: u64,
    pub len: u64,
    pub behavior: MadvBehavior,
    pub applied_at: u64,
    pub pages_affected: u64,
}

impl MadvRegion {
    pub fn new(start: u64, len: u64, behavior: MadvBehavior, now: u64) -> Self {
        Self { start, len, behavior, applied_at: now, pages_affected: len / 4096 }
    }

    #[inline(always)]
    pub fn overlaps(&self, addr: u64, size: u64) -> bool {
        self.start < addr + size && addr < self.start + self.len
    }
}

/// Process advisory state
#[derive(Debug)]
#[repr(align(64))]
pub struct ProcessMadvState {
    pub pid: u64,
    pub regions: Vec<MadvRegion>,
    pub total_advisories: u64,
    pub pages_freed: u64,
    pub pages_populated: u64,
    pub thp_enabled: bool,
    pub ksm_enabled: bool,
}

impl ProcessMadvState {
    pub fn new(pid: u64) -> Self {
        Self {
            pid, regions: Vec::new(), total_advisories: 0,
            pages_freed: 0, pages_populated: 0,
            thp_enabled: true, ksm_enabled: false,
        }
    }

    pub fn apply(&mut self, start: u64, len: u64, behavior: MadvBehavior, now: u64) {
        let region = MadvRegion::new(start, len, behavior, now);
        self.total_advisories += 1;
        match behavior {
            MadvBehavior::DontNeed | MadvBehavior::Free | MadvBehavior::PageOut | MadvBehavior::Cold => {
                self.pages_freed += region.pages_affected;
            }
            MadvBehavior::WillNeed | MadvBehavior::PopulateRead | MadvBehavior::PopulateWrite => {
                self.pages_populated += region.pages_affected;
            }
            MadvBehavior::HugePage => self.thp_enabled = true,
            MadvBehavior::NoHugePage => self.thp_enabled = false,
            MadvBehavior::Mergeable => self.ksm_enabled = true,
            MadvBehavior::Unmergeable => self.ksm_enabled = false,
            _ => {}
        }
        self.regions.push(region);
    }

    #[inline(always)]
    pub fn active_regions(&self) -> usize { self.regions.len() }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MadviseV2Stats {
    pub total_processes: u32,
    pub total_advisories: u64,
    pub total_pages_freed: u64,
    pub total_pages_populated: u64,
    pub thp_enabled_count: u32,
    pub ksm_enabled_count: u32,
}

/// Main madvise v2 manager
pub struct AppMadviseV2 {
    processes: BTreeMap<u64, ProcessMadvState>,
}

impl AppMadviseV2 {
    pub fn new() -> Self { Self { processes: BTreeMap::new() } }

    #[inline(always)]
    pub fn advise(&mut self, pid: u64, start: u64, len: u64, behavior: MadvBehavior, now: u64) {
        let state = self.processes.entry(pid).or_insert_with(|| ProcessMadvState::new(pid));
        state.apply(start, len, behavior, now);
    }

    pub fn stats(&self) -> MadviseV2Stats {
        let advisories: u64 = self.processes.values().map(|p| p.total_advisories).sum();
        let freed: u64 = self.processes.values().map(|p| p.pages_freed).sum();
        let populated: u64 = self.processes.values().map(|p| p.pages_populated).sum();
        let thp = self.processes.values().filter(|p| p.thp_enabled).count() as u32;
        let ksm = self.processes.values().filter(|p| p.ksm_enabled).count() as u32;
        MadviseV2Stats {
            total_processes: self.processes.len() as u32, total_advisories: advisories,
            total_pages_freed: freed, total_pages_populated: populated,
            thp_enabled_count: thp, ksm_enabled_count: ksm,
        }
    }
}
