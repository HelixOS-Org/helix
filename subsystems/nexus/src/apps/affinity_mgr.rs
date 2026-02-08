// SPDX-License-Identifier: GPL-2.0
//! Apps affinity_mgr — CPU affinity management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Affinity policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AffinityPolicy {
    Strict,
    Preferred,
    Spread,
    Compact,
    NumaBound,
}

/// CPU set representation
#[derive(Debug, Clone)]
pub struct CpuSet {
    mask: [u64; 4],
}

impl CpuSet {
    pub fn new() -> Self { Self { mask: [0u64; 4] } }
    pub fn all(ncpus: u32) -> Self {
        let mut s = Self::new();
        for i in 0..ncpus.min(256) { s.set(i); }
        s
    }

    pub fn set(&mut self, cpu: u32) { if cpu < 256 { self.mask[(cpu / 64) as usize] |= 1u64 << (cpu % 64); } }
    pub fn clear(&mut self, cpu: u32) { if cpu < 256 { self.mask[(cpu / 64) as usize] &= !(1u64 << (cpu % 64)); } }
    pub fn has(&self, cpu: u32) -> bool { if cpu < 256 { self.mask[(cpu / 64) as usize] & (1u64 << (cpu % 64)) != 0 } else { false } }
    pub fn count(&self) -> u32 { self.mask.iter().map(|m| m.count_ones()).sum() }
    pub fn is_empty(&self) -> bool { self.mask.iter().all(|&m| m == 0) }

    pub fn intersect(&self, other: &Self) -> Self {
        let mut r = Self::new();
        for i in 0..4 { r.mask[i] = self.mask[i] & other.mask[i]; }
        r
    }

    pub fn union(&self, other: &Self) -> Self {
        let mut r = Self::new();
        for i in 0..4 { r.mask[i] = self.mask[i] | other.mask[i]; }
        r
    }
}

/// Thread affinity state
#[derive(Debug)]
pub struct ThreadAffinity {
    pub tid: u64,
    pub policy: AffinityPolicy,
    pub cpu_set: CpuSet,
    pub effective: CpuSet,
    pub migrations: u64,
    pub last_cpu: u32,
    pub pin_count: u32,
}

impl ThreadAffinity {
    pub fn new(tid: u64) -> Self {
        Self { tid, policy: AffinityPolicy::Preferred, cpu_set: CpuSet::all(256), effective: CpuSet::all(256), migrations: 0, last_cpu: 0, pin_count: 0 }
    }

    pub fn bind(&mut self, set: CpuSet) { self.cpu_set = set.clone(); self.effective = set; }
    pub fn migrate(&mut self, new_cpu: u32) { if new_cpu != self.last_cpu { self.migrations += 1; self.last_cpu = new_cpu; } }
    pub fn pin(&mut self, cpu: u32) { let mut s = CpuSet::new(); s.set(cpu); self.cpu_set = s.clone(); self.effective = s; self.pin_count += 1; }
}

/// Stats
#[derive(Debug, Clone)]
pub struct AffinityMgrStats {
    pub total_threads: u32,
    pub pinned_threads: u32,
    pub total_migrations: u64,
    pub avg_cpus_per_thread: f64,
}

/// Main manager
pub struct AppAffinityMgr {
    threads: BTreeMap<u64, ThreadAffinity>,
}

impl AppAffinityMgr {
    pub fn new() -> Self { Self { threads: BTreeMap::new() } }
    pub fn register(&mut self, tid: u64) { self.threads.insert(tid, ThreadAffinity::new(tid)); }
    pub fn set_affinity(&mut self, tid: u64, set: CpuSet) { if let Some(t) = self.threads.get_mut(&tid) { t.bind(set); } }

    pub fn stats(&self) -> AffinityMgrStats {
        let pinned = self.threads.values().filter(|t| t.cpu_set.count() == 1).count() as u32;
        let mig: u64 = self.threads.values().map(|t| t.migrations).sum();
        let cpus: Vec<f64> = self.threads.values().map(|t| t.cpu_set.count() as f64).collect();
        let avg = if cpus.is_empty() { 0.0 } else { cpus.iter().sum::<f64>() / cpus.len() as f64 };
        AffinityMgrStats { total_threads: self.threads.len() as u32, pinned_threads: pinned, total_migrations: mig, avg_cpus_per_thread: avg }
    }
}
