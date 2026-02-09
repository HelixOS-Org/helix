// SPDX-License-Identifier: GPL-2.0
//! NEXUS Bridge — Cpuset (CPU and memory partitioning)

extern crate alloc;
use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpusetPartition {
    Root,
    Member,
    Isolated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpusetDistribution {
    Spread,
    Cluster,
    Pack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpusetMemPolicy {
    Default,
    HardWall,
    MemMigrate,
    MemSpread,
}

#[derive(Debug, Clone)]
pub struct CpusetMask {
    bits: [u64; 4],
}

impl CpusetMask {
    pub fn new() -> Self { Self { bits: [0; 4] } }
    #[inline(always)]
    pub fn all() -> Self { Self { bits: [u64::MAX; 4] } }

    #[inline]
    pub fn set(&mut self, id: u32) {
        let idx = (id / 64) as usize;
        let bit = id % 64;
        if idx < 4 { self.bits[idx] |= 1u64 << bit; }
    }

    #[inline]
    pub fn clear(&mut self, id: u32) {
        let idx = (id / 64) as usize;
        let bit = id % 64;
        if idx < 4 { self.bits[idx] &= !(1u64 << bit); }
    }

    #[inline]
    pub fn is_set(&self, id: u32) -> bool {
        let idx = (id / 64) as usize;
        let bit = id % 64;
        if idx < 4 { (self.bits[idx] >> bit) & 1 == 1 } else { false }
    }

    #[inline(always)]
    pub fn count(&self) -> u32 { self.bits.iter().map(|b| b.count_ones()).sum() }

    #[inline]
    pub fn intersect(&self, other: &CpusetMask) -> CpusetMask {
        let mut r = CpusetMask::new();
        for i in 0..4 { r.bits[i] = self.bits[i] & other.bits[i]; }
        r
    }

    #[inline]
    pub fn is_subset_of(&self, other: &CpusetMask) -> bool {
        for i in 0..4 {
            if self.bits[i] & !other.bits[i] != 0 { return false; }
        }
        true
    }
}

#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CpusetGroup {
    pub id: u64,
    pub cpus: CpusetMask,
    pub mems: CpusetMask,
    pub partition: CpusetPartition,
    pub distribution: CpusetDistribution,
    pub mem_policy: CpusetMemPolicy,
    pub exclusive: bool,
    pub sched_load_balance: bool,
    pub nr_tasks: u32,
    pub parent_id: Option<u64>,
}

impl CpusetGroup {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            cpus: CpusetMask::all(),
            mems: CpusetMask::all(),
            partition: CpusetPartition::Member,
            distribution: CpusetDistribution::Spread,
            mem_policy: CpusetMemPolicy::Default,
            exclusive: false,
            sched_load_balance: true,
            nr_tasks: 0,
            parent_id: None,
        }
    }

    #[inline(always)]
    pub fn is_valid_for_task(&self) -> bool {
        self.cpus.count() > 0 && self.mems.count() > 0
    }

    #[inline(always)]
    pub fn effective_cpus(&self, parent_cpus: &CpusetMask) -> CpusetMask {
        self.cpus.intersect(parent_cpus)
    }
}

#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CpusetBridgeStats {
    pub total_groups: u64,
    pub total_tasks: u64,
    pub isolated_cpus: u32,
    pub exclusive_groups: u64,
    pub rebalance_events: u64,
}

#[repr(align(64))]
pub struct BridgeCpuset {
    groups: BTreeMap<u64, CpusetGroup>,
    task_to_group: LinearMap<u64, 64>,
    next_id: AtomicU64,
    stats: CpusetBridgeStats,
}

impl BridgeCpuset {
    pub fn new() -> Self {
        Self {
            groups: BTreeMap::new(),
            task_to_group: LinearMap::new(),
            next_id: AtomicU64::new(1),
            stats: CpusetBridgeStats {
                total_groups: 0,
                total_tasks: 0,
                isolated_cpus: 0,
                exclusive_groups: 0,
                rebalance_events: 0,
            },
        }
    }

    #[inline]
    pub fn create_group(&mut self, parent: Option<u64>) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let mut group = CpusetGroup::new(id);
        group.parent_id = parent;
        self.groups.insert(id, group);
        self.stats.total_groups += 1;
        id
    }

    #[inline]
    pub fn set_cpus(&mut self, group_id: u64, cpus: CpusetMask) {
        if let Some(g) = self.groups.get_mut(&group_id) {
            g.cpus = cpus;
        }
    }

    #[inline]
    pub fn attach_task(&mut self, group_id: u64, pid: u64) {
        if let Some(g) = self.groups.get_mut(&group_id) {
            g.nr_tasks += 1;
            self.task_to_group.insert(pid, group_id);
            self.stats.total_tasks += 1;
        }
    }

    #[inline]
    pub fn set_exclusive(&mut self, group_id: u64) {
        if let Some(g) = self.groups.get_mut(&group_id) {
            g.exclusive = true;
            self.stats.exclusive_groups += 1;
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &CpusetBridgeStats {
        &self.stats
    }
}
