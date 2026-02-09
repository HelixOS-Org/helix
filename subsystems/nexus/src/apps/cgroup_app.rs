// SPDX-License-Identifier: GPL-2.0
//! Apps cgroup_app — cgroup application-level integration.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Cgroup subsystem
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupSubsystem {
    Cpu,
    Memory,
    Io,
    Pids,
    Cpuset,
    Freezer,
    HugeTlb,
    Rdma,
    Misc,
}

/// Cgroup freeze state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupFreezeState {
    Thawed,
    Freezing,
    Frozen,
}

/// Resource limits
#[derive(Debug, Clone)]
pub struct CgroupLimits {
    pub cpu_max_us: u64,
    pub cpu_period_us: u64,
    pub mem_max: u64,
    pub mem_swap_max: u64,
    pub pids_max: u32,
    pub io_weight: u16,
    pub io_max_rbps: u64,
    pub io_max_wbps: u64,
}

impl CgroupLimits {
    #[inline(always)]
    pub fn unlimited() -> Self {
        Self { cpu_max_us: u64::MAX, cpu_period_us: 100_000, mem_max: u64::MAX, mem_swap_max: u64::MAX, pids_max: u32::MAX, io_weight: 100, io_max_rbps: u64::MAX, io_max_wbps: u64::MAX }
    }
}

/// Cgroup node
#[derive(Debug)]
pub struct CgroupNode {
    pub id: u64,
    pub parent_id: u64,
    pub depth: u8,
    pub limits: CgroupLimits,
    pub freeze: CgroupFreezeState,
    pub children: Vec<u64>,
    pub processes: Vec<u64>,
    pub cpu_usage_us: u64,
    pub mem_current: u64,
    pub io_bytes_read: u64,
    pub io_bytes_written: u64,
}

impl CgroupNode {
    pub fn new(id: u64, parent: u64, depth: u8) -> Self {
        Self {
            id, parent_id: parent, depth, limits: CgroupLimits::unlimited(),
            freeze: CgroupFreezeState::Thawed, children: Vec::new(),
            processes: Vec::new(), cpu_usage_us: 0, mem_current: 0,
            io_bytes_read: 0, io_bytes_written: 0,
        }
    }

    #[inline(always)]
    pub fn add_process(&mut self, pid: u64) { if !self.processes.contains(&pid) { self.processes.push(pid); } }
    #[inline(always)]
    pub fn remove_process(&mut self, pid: u64) { self.processes.retain(|&p| p != pid); }
    #[inline(always)]
    pub fn mem_utilization(&self) -> f64 { if self.limits.mem_max == u64::MAX { 0.0 } else { self.mem_current as f64 / self.limits.mem_max as f64 } }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CgroupAppStats {
    pub total_groups: u32,
    pub total_processes: u32,
    pub frozen_groups: u32,
    pub total_cpu_us: u64,
    pub total_memory: u64,
}

/// Main cgroup app
pub struct AppCgroup {
    groups: BTreeMap<u64, CgroupNode>,
    next_id: u64,
}

impl AppCgroup {
    pub fn new() -> Self {
        let mut g = BTreeMap::new();
        g.insert(0, CgroupNode::new(0, 0, 0));
        Self { groups: g, next_id: 1 }
    }

    #[inline]
    pub fn create(&mut self, parent: u64) -> Option<u64> {
        let depth = self.groups.get(&parent)?.depth + 1;
        let id = self.next_id; self.next_id += 1;
        self.groups.insert(id, CgroupNode::new(id, parent, depth));
        if let Some(p) = self.groups.get_mut(&parent) { p.children.push(id); }
        Some(id)
    }

    #[inline(always)]
    pub fn attach(&mut self, group: u64, pid: u64) {
        if let Some(g) = self.groups.get_mut(&group) { g.add_process(pid); }
    }

    #[inline]
    pub fn stats(&self) -> CgroupAppStats {
        let procs: u32 = self.groups.values().map(|g| g.processes.len() as u32).sum();
        let frozen = self.groups.values().filter(|g| g.freeze == CgroupFreezeState::Frozen).count() as u32;
        let cpu: u64 = self.groups.values().map(|g| g.cpu_usage_us).sum();
        let mem: u64 = self.groups.values().map(|g| g.mem_current).sum();
        CgroupAppStats { total_groups: self.groups.len() as u32, total_processes: procs, frozen_groups: frozen, total_cpu_us: cpu, total_memory: mem }
    }
}
