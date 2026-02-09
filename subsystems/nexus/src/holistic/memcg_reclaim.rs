// SPDX-License-Identifier: GPL-2.0
//! NEXUS Holistic Memory Cgroup Reclaim — Proactive and reactive reclaim
//!
//! Implements per-cgroup memory reclamation with proactive scanning,
//! generation-based LRU, soft limit enforcement, and NUMA-aware reclaim.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Reclaim urgency level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemcgReclaimUrgency {
    None,
    Low,
    Medium,
    High,
    Critical,
    Oom,
}

/// LRU list type for reclaim scanning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemcgLruType {
    InactiveAnon,
    ActiveAnon,
    InactiveFile,
    ActiveFile,
    Unevictable,
}

/// Reclaim scan result for a single pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemcgScanResult {
    Reclaimed,
    Skipped,
    Failed,
    Writeback,
    Referenced,
    Mapped,
}

/// Per-LRU scan state.
#[derive(Debug, Clone)]
pub struct MemcgLruScan {
    pub lru_type: MemcgLruType,
    pub nr_scanned: u64,
    pub nr_reclaimed: u64,
    pub nr_skipped: u64,
    pub nr_writeback: u64,
    pub nr_referenced: u64,
    pub scan_priority: u32,
    pub generation: u64,
}

impl MemcgLruScan {
    pub fn new(lru_type: MemcgLruType) -> Self {
        Self {
            lru_type,
            nr_scanned: 0,
            nr_reclaimed: 0,
            nr_skipped: 0,
            nr_writeback: 0,
            nr_referenced: 0,
            scan_priority: 12,
            generation: 0,
        }
    }

    #[inline]
    pub fn reclaim_efficiency(&self) -> f64 {
        if self.nr_scanned == 0 {
            return 0.0;
        }
        (self.nr_reclaimed as f64 / self.nr_scanned as f64) * 100.0
    }

    #[inline]
    pub fn advance_generation(&mut self) {
        self.generation += 1;
        self.nr_scanned = 0;
        self.nr_reclaimed = 0;
        self.nr_skipped = 0;
        self.nr_writeback = 0;
        self.nr_referenced = 0;
    }
}

/// Memory cgroup reclaim context.
#[derive(Debug, Clone)]
pub struct MemcgReclaimCtx {
    pub cgroup_id: u64,
    pub name: String,
    pub memory_usage: u64,
    pub memory_limit: u64,
    pub soft_limit: u64,
    pub swap_usage: u64,
    pub swap_limit: u64,
    pub lru_scans: Vec<MemcgLruScan>,
    pub urgency: MemcgReclaimUrgency,
    pub reclaim_target: u64,
    pub total_reclaimed: u64,
    pub oom_kill_count: u64,
    pub proactive_reclaim_enabled: bool,
    pub proactive_target_percent: u32,
    pub children: Vec<u64>,
    pub parent_id: Option<u64>,
}

impl MemcgReclaimCtx {
    pub fn new(cgroup_id: u64, name: String) -> Self {
        let lru_scans = alloc::vec![
            MemcgLruScan::new(MemcgLruType::InactiveAnon),
            MemcgLruScan::new(MemcgLruType::ActiveAnon),
            MemcgLruScan::new(MemcgLruType::InactiveFile),
            MemcgLruScan::new(MemcgLruType::ActiveFile),
            MemcgLruScan::new(MemcgLruType::Unevictable),
        ];
        Self {
            cgroup_id,
            name,
            memory_usage: 0,
            memory_limit: u64::MAX,
            soft_limit: u64::MAX,
            swap_usage: 0,
            swap_limit: u64::MAX,
            lru_scans,
            urgency: MemcgReclaimUrgency::None,
            reclaim_target: 0,
            total_reclaimed: 0,
            oom_kill_count: 0,
            proactive_reclaim_enabled: false,
            proactive_target_percent: 80,
            children: Vec::new(),
            parent_id: None,
        }
    }

    pub fn compute_urgency(&mut self) {
        if self.memory_limit == u64::MAX {
            self.urgency = MemcgReclaimUrgency::None;
            return;
        }
        let usage_percent = (self.memory_usage as f64 / self.memory_limit as f64) * 100.0;
        self.urgency = if usage_percent >= 100.0 {
            MemcgReclaimUrgency::Oom
        } else if usage_percent >= 95.0 {
            MemcgReclaimUrgency::Critical
        } else if usage_percent >= 85.0 {
            MemcgReclaimUrgency::High
        } else if usage_percent >= 70.0 {
            MemcgReclaimUrgency::Medium
        } else if usage_percent >= 50.0 {
            MemcgReclaimUrgency::Low
        } else {
            MemcgReclaimUrgency::None
        };
    }

    #[inline]
    pub fn proactive_reclaim_target(&self) -> u64 {
        if !self.proactive_reclaim_enabled || self.memory_limit == u64::MAX {
            return 0;
        }
        let target_usage = (self.memory_limit * self.proactive_target_percent as u64) / 100;
        if self.memory_usage > target_usage {
            self.memory_usage - target_usage
        } else {
            0
        }
    }

    #[inline]
    pub fn record_scan(&mut self, lru_type: MemcgLruType, scanned: u64, reclaimed: u64) {
        for scan in &mut self.lru_scans {
            if scan.lru_type == lru_type {
                scan.nr_scanned += scanned;
                scan.nr_reclaimed += reclaimed;
            }
        }
        self.total_reclaimed += reclaimed;
        self.memory_usage = self.memory_usage.saturating_sub(reclaimed * 4096);
    }

    #[inline(always)]
    pub fn above_soft_limit(&self) -> bool {
        self.soft_limit != u64::MAX && self.memory_usage > self.soft_limit
    }
}

/// Statistics for the memcg reclaim controller.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MemcgReclaimStats {
    pub total_cgroups: u64,
    pub total_pages_scanned: u64,
    pub total_pages_reclaimed: u64,
    pub proactive_reclaim_runs: u64,
    pub soft_limit_reclaims: u64,
    pub oom_kills: u64,
    pub scan_efficiency: f64,
    pub total_swap_used: u64,
}

/// Main holistic memory cgroup reclaim manager.
pub struct HolisticMemcgReclaim {
    pub cgroups: BTreeMap<u64, MemcgReclaimCtx>,
    pub next_id: u64,
    pub global_reclaim_count: u64,
    pub stats: MemcgReclaimStats,
}

impl HolisticMemcgReclaim {
    pub fn new() -> Self {
        Self {
            cgroups: BTreeMap::new(),
            next_id: 1,
            global_reclaim_count: 0,
            stats: MemcgReclaimStats {
                total_cgroups: 0,
                total_pages_scanned: 0,
                total_pages_reclaimed: 0,
                proactive_reclaim_runs: 0,
                soft_limit_reclaims: 0,
                oom_kills: 0,
                scan_efficiency: 0.0,
                total_swap_used: 0,
            },
        }
    }

    pub fn create_cgroup(&mut self, name: String, parent: Option<u64>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let mut ctx = MemcgReclaimCtx::new(id, name);
        ctx.parent_id = parent;
        if let Some(pid) = parent {
            if let Some(p) = self.cgroups.get_mut(&pid) {
                p.children.push(id);
            }
        }
        self.cgroups.insert(id, ctx);
        self.stats.total_cgroups += 1;
        id
    }

    #[inline]
    pub fn set_limits(&mut self, cgroup_id: u64, memory_limit: u64, soft_limit: u64) -> bool {
        if let Some(cg) = self.cgroups.get_mut(&cgroup_id) {
            cg.memory_limit = memory_limit;
            cg.soft_limit = soft_limit;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn charge_memory(&mut self, cgroup_id: u64, pages: u64) -> bool {
        if let Some(cg) = self.cgroups.get_mut(&cgroup_id) {
            cg.memory_usage += pages * 4096;
            cg.compute_urgency();
            true
        } else {
            false
        }
    }

    pub fn run_reclaim(&mut self, cgroup_id: u64, target_pages: u64) -> u64 {
        let reclaimed;
        if let Some(cg) = self.cgroups.get_mut(&cgroup_id) {
            let actual = core::cmp::min(target_pages, cg.memory_usage / 4096);
            cg.record_scan(MemcgLruType::InactiveFile, actual * 2, actual);
            reclaimed = actual;
        } else {
            return 0;
        }
        self.global_reclaim_count += 1;
        self.stats.total_pages_reclaimed += reclaimed;
        reclaimed
    }

    #[inline(always)]
    pub fn cgroup_count(&self) -> usize {
        self.cgroups.len()
    }
}
