// SPDX-License-Identifier: GPL-2.0
//! Holistic cgroup_mem — cgroup memory controller.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Memory limit type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemLimitType {
    Hard,
    Soft,
    Swap,
    KernelMemory,
    TcpMemory,
}

/// Cgroup memory state
#[derive(Debug)]
#[repr(align(64))]
pub struct CgroupMemState {
    pub id: u64,
    pub usage_bytes: u64,
    pub limit_bytes: u64,
    pub swap_usage: u64,
    pub swap_limit: u64,
    pub kernel_usage: u64,
    pub cache_bytes: u64,
    pub rss_bytes: u64,
    pub mapped_file: u64,
    pub pgfault: u64,
    pub pgmajfault: u64,
    pub oom_kills: u64,
    pub under_oom: bool,
}

impl CgroupMemState {
    pub fn new(id: u64, limit: u64) -> Self {
        Self { id, usage_bytes: 0, limit_bytes: limit, swap_usage: 0, swap_limit: 0, kernel_usage: 0, cache_bytes: 0, rss_bytes: 0, mapped_file: 0, pgfault: 0, pgmajfault: 0, oom_kills: 0, under_oom: false }
    }

    #[inline(always)]
    pub fn charge(&mut self, bytes: u64) -> bool {
        if self.usage_bytes + bytes > self.limit_bytes { self.under_oom = true; self.oom_kills += 1; return false; }
        self.usage_bytes += bytes; true
    }

    #[inline(always)]
    pub fn uncharge(&mut self, bytes: u64) {
        self.usage_bytes = self.usage_bytes.saturating_sub(bytes);
        if self.usage_bytes < self.limit_bytes { self.under_oom = false; }
    }

    #[inline(always)]
    pub fn usage_ratio(&self) -> f64 { if self.limit_bytes == 0 { 0.0 } else { self.usage_bytes as f64 / self.limit_bytes as f64 } }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CgroupMemStats {
    pub total_cgroups: u32,
    pub total_usage_bytes: u64,
    pub total_limit_bytes: u64,
    pub under_oom_count: u32,
    pub total_oom_kills: u64,
}

/// Main holistic cgroup memory
pub struct HolisticCgroupMem {
    cgroups: BTreeMap<u64, CgroupMemState>,
}

impl HolisticCgroupMem {
    pub fn new() -> Self { Self { cgroups: BTreeMap::new() } }

    #[inline(always)]
    pub fn add(&mut self, id: u64, limit: u64) { self.cgroups.insert(id, CgroupMemState::new(id, limit)); }

    #[inline(always)]
    pub fn charge(&mut self, id: u64, bytes: u64) -> bool {
        self.cgroups.get_mut(&id).map_or(false, |c| c.charge(bytes))
    }

    #[inline(always)]
    pub fn uncharge(&mut self, id: u64, bytes: u64) {
        if let Some(c) = self.cgroups.get_mut(&id) { c.uncharge(bytes); }
    }

    #[inline]
    pub fn stats(&self) -> CgroupMemStats {
        let usage: u64 = self.cgroups.values().map(|c| c.usage_bytes).sum();
        let limit: u64 = self.cgroups.values().map(|c| c.limit_bytes).sum();
        let oom = self.cgroups.values().filter(|c| c.under_oom).count() as u32;
        let kills: u64 = self.cgroups.values().map(|c| c.oom_kills).sum();
        CgroupMemStats { total_cgroups: self.cgroups.len() as u32, total_usage_bytes: usage, total_limit_bytes: limit, under_oom_count: oom, total_oom_kills: kills }
    }
}
