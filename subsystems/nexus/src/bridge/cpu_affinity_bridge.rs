// SPDX-License-Identifier: GPL-2.0
//! NEXUS Bridge — CPU Affinity (topology-aware affinity management)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AffinityScope {
    System,
    NumaNode,
    LlcDomain,
    CoreComplex,
    SmtGroup,
    Pinned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AffinityMigrationType {
    LoadBalance,
    NumaOptimize,
    ThermalThrottle,
    PowerSave,
    UserRequested,
    CgroupBound,
}

#[derive(Debug, Clone)]
pub struct CpuTopologyNode {
    pub cpu_id: u32,
    pub core_id: u32,
    pub package_id: u32,
    pub numa_node: u32,
    pub llc_id: u32,
    pub smt_sibling: Option<u32>,
    pub online: bool,
    pub frequency_khz: u32,
    pub capacity: u32,
}

impl CpuTopologyNode {
    #[inline(always)]
    pub fn is_smt_sibling_of(&self, other: &CpuTopologyNode) -> bool {
        self.core_id == other.core_id && self.cpu_id != other.cpu_id
    }

    #[inline(always)]
    pub fn shares_llc_with(&self, other: &CpuTopologyNode) -> bool {
        self.llc_id == other.llc_id
    }

    #[inline(always)]
    pub fn same_numa_node(&self, other: &CpuTopologyNode) -> bool {
        self.numa_node == other.numa_node
    }
}

#[derive(Debug, Clone)]
pub struct AffinityMask {
    bits: [u64; 4],
}

impl AffinityMask {
    pub fn new() -> Self {
        Self { bits: [0; 4] }
    }

    #[inline(always)]
    pub fn all() -> Self {
        Self { bits: [u64::MAX; 4] }
    }

    #[inline]
    pub fn set(&mut self, cpu: u32) {
        let idx = (cpu / 64) as usize;
        let bit = cpu % 64;
        if idx < 4 { self.bits[idx] |= 1u64 << bit; }
    }

    #[inline]
    pub fn clear(&mut self, cpu: u32) {
        let idx = (cpu / 64) as usize;
        let bit = cpu % 64;
        if idx < 4 { self.bits[idx] &= !(1u64 << bit); }
    }

    #[inline]
    pub fn is_set(&self, cpu: u32) -> bool {
        let idx = (cpu / 64) as usize;
        let bit = cpu % 64;
        if idx < 4 { (self.bits[idx] >> bit) & 1 == 1 } else { false }
    }

    #[inline(always)]
    pub fn count(&self) -> u32 {
        self.bits.iter().map(|b| b.count_ones()).sum()
    }

    #[inline]
    pub fn intersect(&self, other: &AffinityMask) -> AffinityMask {
        let mut r = AffinityMask::new();
        for i in 0..4 { r.bits[i] = self.bits[i] & other.bits[i]; }
        r
    }

    #[inline]
    pub fn union_with(&self, other: &AffinityMask) -> AffinityMask {
        let mut r = AffinityMask::new();
        for i in 0..4 { r.bits[i] = self.bits[i] | other.bits[i]; }
        r
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.bits.iter().all(|&b| b == 0)
    }
}

#[derive(Debug, Clone)]
pub struct ProcessAffinity {
    pub pid: u64,
    pub mask: AffinityMask,
    pub scope: AffinityScope,
    pub last_cpu: u32,
    pub migrations: u64,
    pub voluntary_switches: u64,
    pub involuntary_switches: u64,
}

impl ProcessAffinity {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            mask: AffinityMask::all(),
            scope: AffinityScope::System,
            last_cpu: 0,
            migrations: 0,
            voluntary_switches: 0,
            involuntary_switches: 0,
        }
    }

    #[inline]
    pub fn migrate_to(&mut self, cpu: u32, mig_type: AffinityMigrationType) {
        if self.mask.is_set(cpu) {
            self.last_cpu = cpu;
            self.migrations += 1;
        }
    }

    #[inline(always)]
    pub fn migration_rate(&self) -> u64 {
        let total = self.voluntary_switches + self.involuntary_switches;
        if total == 0 { 0 } else { (self.migrations * 100) / total }
    }
}

#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CpuAffinityBridgeStats {
    pub total_processes: u64,
    pub total_migrations: u64,
    pub numa_local_hits: u64,
    pub numa_remote_hits: u64,
    pub pinned_count: u64,
}

#[repr(align(64))]
pub struct BridgeCpuAffinity {
    processes: BTreeMap<u64, ProcessAffinity>,
    topology: Vec<CpuTopologyNode>,
    stats: CpuAffinityBridgeStats,
}

impl BridgeCpuAffinity {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            topology: Vec::new(),
            stats: CpuAffinityBridgeStats {
                total_processes: 0,
                total_migrations: 0,
                numa_local_hits: 0,
                numa_remote_hits: 0,
                pinned_count: 0,
            },
        }
    }

    #[inline(always)]
    pub fn add_cpu(&mut self, node: CpuTopologyNode) {
        self.topology.push(node);
    }

    #[inline(always)]
    pub fn register_process(&mut self, pid: u64) {
        self.processes.insert(pid, ProcessAffinity::new(pid));
        self.stats.total_processes += 1;
    }

    #[inline]
    pub fn pin_to_cpu(&mut self, pid: u64, cpu: u32) {
        if let Some(p) = self.processes.get_mut(&pid) {
            let mut mask = AffinityMask::new();
            mask.set(cpu);
            p.mask = mask;
            p.scope = AffinityScope::Pinned;
            self.stats.pinned_count += 1;
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &CpuAffinityBridgeStats {
        &self.stats
    }
}
