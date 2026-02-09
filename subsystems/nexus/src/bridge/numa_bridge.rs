// SPDX-License-Identifier: GPL-2.0
//! NEXUS Bridge — NUMA (non-uniform memory access topology)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumaPolicy {
    Default,
    Bind,
    Interleave,
    Preferred,
    PreferredMany,
    Local,
    WeightedInterleave,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumaMigrateMode {
    Lazy,
    Eager,
    Cooperative,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumaHintType {
    AutoNumaBalancing,
    MigrationFault,
    ExplicitMove,
    CpuFollows,
}

#[derive(Debug, Clone)]
pub struct NumaNode {
    pub node_id: u32,
    pub total_pages: u64,
    pub free_pages: u64,
    pub nr_cpus: u32,
    pub distance_to: Vec<(u32, u32)>,
    pub nr_hugepages: u64,
    pub free_hugepages: u64,
    pub migration_in: u64,
    pub migration_out: u64,
}

impl NumaNode {
    pub fn new(node_id: u32, total_pages: u64) -> Self {
        Self {
            node_id,
            total_pages,
            free_pages: total_pages,
            nr_cpus: 0,
            distance_to: Vec::new(),
            nr_hugepages: 0,
            free_hugepages: 0,
            migration_in: 0,
            migration_out: 0,
        }
    }

    #[inline(always)]
    pub fn utilization_pct(&self) -> u64 {
        if self.total_pages == 0 { 0 }
        else { ((self.total_pages - self.free_pages) * 100) / self.total_pages }
    }

    #[inline]
    pub fn distance_to_node(&self, other: u32) -> u32 {
        for &(nid, dist) in &self.distance_to {
            if nid == other { return dist; }
        }
        u32::MAX
    }

    #[inline(always)]
    pub fn is_local(&self) -> bool {
        self.distance_to.iter().all(|&(_, d)| d >= 10)
    }

    #[inline(always)]
    pub fn migration_balance(&self) -> i64 {
        self.migration_in as i64 - self.migration_out as i64
    }
}

#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct NumaProcessState {
    pub pid: u64,
    pub policy: NumaPolicy,
    pub preferred_node: Option<u32>,
    pub nodemask: u64,
    pub total_faults: u64,
    pub local_faults: u64,
    pub remote_faults: u64,
    pub migrate_mode: NumaMigrateMode,
    pub pages_migrated: u64,
}

impl NumaProcessState {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            policy: NumaPolicy::Default,
            preferred_node: None,
            nodemask: u64::MAX,
            total_faults: 0,
            local_faults: 0,
            remote_faults: 0,
            migrate_mode: NumaMigrateMode::Lazy,
            pages_migrated: 0,
        }
    }

    #[inline(always)]
    pub fn record_fault(&mut self, is_local: bool) {
        self.total_faults += 1;
        if is_local { self.local_faults += 1; } else { self.remote_faults += 1; }
    }

    #[inline(always)]
    pub fn locality_pct(&self) -> u64 {
        if self.total_faults == 0 { 100 }
        else { (self.local_faults * 100) / self.total_faults }
    }
}

#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct NumaBridgeStats {
    pub total_nodes: u64,
    pub total_processes: u64,
    pub total_migrations: u64,
    pub total_faults: u64,
    pub avg_locality_pct: u64,
}

#[repr(align(64))]
pub struct BridgeNuma {
    nodes: BTreeMap<u32, NumaNode>,
    processes: BTreeMap<u64, NumaProcessState>,
    stats: NumaBridgeStats,
}

impl BridgeNuma {
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            processes: BTreeMap::new(),
            stats: NumaBridgeStats {
                total_nodes: 0,
                total_processes: 0,
                total_migrations: 0,
                total_faults: 0,
                avg_locality_pct: 100,
            },
        }
    }

    #[inline(always)]
    pub fn add_node(&mut self, node: NumaNode) {
        self.nodes.insert(node.node_id, node);
        self.stats.total_nodes += 1;
    }

    #[inline(always)]
    pub fn register_process(&mut self, pid: u64) {
        self.processes.insert(pid, NumaProcessState::new(pid));
        self.stats.total_processes += 1;
    }

    #[inline]
    pub fn set_policy(&mut self, pid: u64, policy: NumaPolicy, preferred: Option<u32>) {
        if let Some(p) = self.processes.get_mut(&pid) {
            p.policy = policy;
            p.preferred_node = preferred;
        }
    }

    #[inline]
    pub fn record_fault(&mut self, pid: u64, is_local: bool) {
        if let Some(p) = self.processes.get_mut(&pid) {
            p.record_fault(is_local);
            self.stats.total_faults += 1;
        }
    }

    pub fn migrate_pages(&mut self, pid: u64, from_node: u32, to_node: u32, pages: u64) {
        if let Some(src) = self.nodes.get_mut(&from_node) {
            src.migration_out += pages;
            src.free_pages += pages;
        }
        if let Some(dst) = self.nodes.get_mut(&to_node) {
            dst.migration_in += pages;
            if dst.free_pages >= pages { dst.free_pages -= pages; }
        }
        if let Some(p) = self.processes.get_mut(&pid) {
            p.pages_migrated += pages;
        }
        self.stats.total_migrations += 1;
    }

    #[inline(always)]
    pub fn stats(&self) -> &NumaBridgeStats {
        &self.stats
    }
}
