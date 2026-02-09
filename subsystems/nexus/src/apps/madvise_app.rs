// SPDX-License-Identifier: GPL-2.0
//! Apps madvise_app — madvise memory advice application layer.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Madvise advice type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MadviseAdvice {
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
    UnMergeable,
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

/// Madvise region
#[derive(Debug)]
pub struct MadviseRegion {
    pub addr: u64,
    pub length: u64,
    pub advice: MadviseAdvice,
    pub applied_at: u64,
}

/// Per-process madvise tracker
#[derive(Debug)]
pub struct ProcessMadvise {
    pub pid: u64,
    pub regions: Vec<MadviseRegion>,
    pub total_calls: u64,
    pub total_bytes: u64,
    pub ksm_regions: u32,
    pub thp_regions: u32,
}

impl ProcessMadvise {
    pub fn new(pid: u64) -> Self {
        Self { pid, regions: Vec::new(), total_calls: 0, total_bytes: 0, ksm_regions: 0, thp_regions: 0 }
    }

    pub fn advise(&mut self, addr: u64, len: u64, advice: MadviseAdvice, now: u64) {
        match advice {
            MadviseAdvice::Mergeable => self.ksm_regions += 1,
            MadviseAdvice::UnMergeable => { if self.ksm_regions > 0 { self.ksm_regions -= 1; } }
            MadviseAdvice::HugePage => self.thp_regions += 1,
            MadviseAdvice::NoHugePage => { if self.thp_regions > 0 { self.thp_regions -= 1; } }
            _ => {}
        }
        self.regions.push(MadviseRegion { addr, length: len, advice, applied_at: now });
        self.total_calls += 1;
        self.total_bytes += len;
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MadviseAppStats {
    pub tracked_procs: u32,
    pub total_calls: u64,
    pub total_bytes: u64,
    pub ksm_regions: u32,
    pub thp_regions: u32,
}

/// Main app madvise
pub struct AppMadvise {
    procs: BTreeMap<u64, ProcessMadvise>,
}

impl AppMadvise {
    pub fn new() -> Self { Self { procs: BTreeMap::new() } }

    #[inline(always)]
    pub fn track(&mut self, pid: u64) { self.procs.insert(pid, ProcessMadvise::new(pid)); }

    #[inline(always)]
    pub fn advise(&mut self, pid: u64, addr: u64, len: u64, advice: MadviseAdvice, now: u64) {
        if let Some(p) = self.procs.get_mut(&pid) { p.advise(addr, len, advice, now); }
    }

    #[inline(always)]
    pub fn untrack(&mut self, pid: u64) { self.procs.remove(&pid); }

    #[inline]
    pub fn stats(&self) -> MadviseAppStats {
        let calls: u64 = self.procs.values().map(|p| p.total_calls).sum();
        let bytes: u64 = self.procs.values().map(|p| p.total_bytes).sum();
        let ksm: u32 = self.procs.values().map(|p| p.ksm_regions).sum();
        let thp: u32 = self.procs.values().map(|p| p.thp_regions).sum();
        MadviseAppStats { tracked_procs: self.procs.len() as u32, total_calls: calls, total_bytes: bytes, ksm_regions: ksm, thp_regions: thp }
    }
}
