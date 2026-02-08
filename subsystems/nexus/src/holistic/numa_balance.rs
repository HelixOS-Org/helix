// SPDX-License-Identifier: GPL-2.0
//! Holistic numa_balance — NUMA balancing and page migration.

extern crate alloc;

use alloc::collections::BTreeMap;

/// NUMA hint fault type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumaFaultType {
    Local,
    Remote,
    Group,
    Preferred,
}

/// NUMA balance task
#[derive(Debug)]
pub struct NumaBalanceTask {
    pub pid: u64,
    pub preferred_node: u32,
    pub current_node: u32,
    pub local_faults: u64,
    pub remote_faults: u64,
    pub migrations: u64,
    pub scan_period_ms: u32,
    pub total_scanned: u64,
}

impl NumaBalanceTask {
    pub fn new(pid: u64, node: u32) -> Self {
        Self { pid, preferred_node: node, current_node: node, local_faults: 0, remote_faults: 0, migrations: 0, scan_period_ms: 1000, total_scanned: 0 }
    }

    pub fn record_fault(&mut self, fault_type: NumaFaultType) {
        match fault_type {
            NumaFaultType::Local => self.local_faults += 1,
            NumaFaultType::Remote => self.remote_faults += 1,
            _ => {}
        }
    }

    pub fn locality_score(&self) -> f64 {
        let total = self.local_faults + self.remote_faults;
        if total == 0 { return 1.0; }
        self.local_faults as f64 / total as f64
    }
}

/// NUMA node stats
#[derive(Debug)]
pub struct NumaNodeInfo {
    pub node_id: u32,
    pub total_pages: u64,
    pub free_pages: u64,
    pub local_allocs: u64,
    pub remote_allocs: u64,
    pub migrations_in: u64,
    pub migrations_out: u64,
}

impl NumaNodeInfo {
    pub fn new(id: u32, total: u64) -> Self {
        Self { node_id: id, total_pages: total, free_pages: total, local_allocs: 0, remote_allocs: 0, migrations_in: 0, migrations_out: 0 }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct NumaBalanceStats {
    pub total_tasks: u32,
    pub total_nodes: u32,
    pub total_local_faults: u64,
    pub total_remote_faults: u64,
    pub total_migrations: u64,
}

/// Main holistic NUMA balance
pub struct HolisticNumaBalance {
    tasks: BTreeMap<u64, NumaBalanceTask>,
    nodes: BTreeMap<u32, NumaNodeInfo>,
}

impl HolisticNumaBalance {
    pub fn new() -> Self { Self { tasks: BTreeMap::new(), nodes: BTreeMap::new() } }

    pub fn add_node(&mut self, id: u32, pages: u64) { self.nodes.insert(id, NumaNodeInfo::new(id, pages)); }
    pub fn track_task(&mut self, pid: u64, node: u32) { self.tasks.insert(pid, NumaBalanceTask::new(pid, node)); }

    pub fn record_fault(&mut self, pid: u64, ftype: NumaFaultType) {
        if let Some(t) = self.tasks.get_mut(&pid) { t.record_fault(ftype); }
    }

    pub fn migrate(&mut self, pid: u64, to_node: u32) {
        if let Some(t) = self.tasks.get_mut(&pid) {
            let from = t.current_node;
            t.current_node = to_node;
            t.migrations += 1;
            if let Some(n) = self.nodes.get_mut(&from) { n.migrations_out += 1; }
            if let Some(n) = self.nodes.get_mut(&to_node) { n.migrations_in += 1; }
        }
    }

    pub fn stats(&self) -> NumaBalanceStats {
        let local: u64 = self.tasks.values().map(|t| t.local_faults).sum();
        let remote: u64 = self.tasks.values().map(|t| t.remote_faults).sum();
        let mig: u64 = self.tasks.values().map(|t| t.migrations).sum();
        NumaBalanceStats { total_tasks: self.tasks.len() as u32, total_nodes: self.nodes.len() as u32, total_local_faults: local, total_remote_faults: remote, total_migrations: mig }
    }
}
