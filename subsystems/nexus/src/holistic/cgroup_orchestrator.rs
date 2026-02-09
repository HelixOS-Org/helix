// SPDX-License-Identifier: GPL-2.0
//! Holistic cgroup_orchestrator — cgroup hierarchy management and resource orchestration.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Cgroup controller type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupController {
    Cpu,
    Memory,
    Io,
    Pids,
    Cpuset,
    Hugetlb,
    Rdma,
    Misc,
    Freezer,
    Perf,
}

/// Cgroup version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupVersion {
    V1,
    V2,
    Hybrid,
}

/// Cgroup limit type
#[derive(Debug, Clone)]
pub struct CgroupLimits {
    pub cpu_weight: u32,
    pub cpu_max_us: u64,
    pub cpu_period_us: u64,
    pub memory_max: u64,
    pub memory_high: u64,
    pub memory_low: u64,
    pub io_weight: u32,
    pub pids_max: u64,
}

impl CgroupLimits {
    pub fn default_limits() -> Self {
        Self {
            cpu_weight: 100,
            cpu_max_us: 0, // unlimited
            cpu_period_us: 100_000,
            memory_max: u64::MAX,
            memory_high: u64::MAX,
            memory_low: 0,
            io_weight: 100,
            pids_max: u64::MAX,
        }
    }

    #[inline(always)]
    pub fn cpu_quota(&self) -> f64 {
        if self.cpu_max_us == 0 || self.cpu_period_us == 0 { return 1.0; }
        self.cpu_max_us as f64 / self.cpu_period_us as f64
    }
}

/// Cgroup resource usage snapshot
#[derive(Debug, Clone)]
pub struct CgroupUsage {
    pub cpu_usage_ns: u64,
    pub user_ns: u64,
    pub system_ns: u64,
    pub memory_current: u64,
    pub memory_swap: u64,
    pub io_read_bytes: u64,
    pub io_write_bytes: u64,
    pub pids_current: u64,
    pub nr_throttled: u64,
    pub throttled_ns: u64,
}

impl CgroupUsage {
    pub fn new() -> Self {
        Self {
            cpu_usage_ns: 0, user_ns: 0, system_ns: 0,
            memory_current: 0, memory_swap: 0,
            io_read_bytes: 0, io_write_bytes: 0,
            pids_current: 0, nr_throttled: 0, throttled_ns: 0,
        }
    }

    #[inline(always)]
    pub fn memory_utilization(&self, limit: u64) -> f64 {
        if limit == 0 || limit == u64::MAX { return 0.0; }
        self.memory_current as f64 / limit as f64
    }

    #[inline(always)]
    pub fn throttle_rate(&self, cpu_total_ns: u64) -> f64 {
        if cpu_total_ns == 0 { return 0.0; }
        self.throttled_ns as f64 / cpu_total_ns as f64
    }
}

/// A cgroup node in the hierarchy
#[derive(Debug)]
pub struct CgroupNode {
    pub id: u64,
    pub path: String,
    pub parent_id: Option<u64>,
    pub children: Vec<u64>,
    pub controllers: Vec<CgroupController>,
    pub limits: CgroupLimits,
    pub usage: CgroupUsage,
    pub frozen: bool,
    pub process_count: u32,
    pub depth: u32,
    pub created_ns: u64,
}

impl CgroupNode {
    pub fn new(id: u64, path: String, parent: Option<u64>, depth: u32) -> Self {
        Self {
            id, path, parent_id: parent,
            children: Vec::new(),
            controllers: Vec::new(),
            limits: CgroupLimits::default_limits(),
            usage: CgroupUsage::new(),
            frozen: false,
            process_count: 0,
            depth,
            created_ns: 0,
        }
    }

    #[inline]
    pub fn add_child(&mut self, child_id: u64) {
        if !self.children.contains(&child_id) {
            self.children.push(child_id);
        }
    }

    #[inline(always)]
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    #[inline(always)]
    pub fn is_root(&self) -> bool {
        self.parent_id.is_none()
    }

    #[inline]
    pub fn enable_controller(&mut self, ctrl: CgroupController) {
        if !self.controllers.contains(&ctrl) {
            self.controllers.push(ctrl);
        }
    }

    #[inline(always)]
    pub fn memory_pressure(&self) -> f64 {
        self.usage.memory_utilization(self.limits.memory_max)
    }

    #[inline(always)]
    pub fn is_throttled(&self) -> bool {
        self.usage.nr_throttled > 0
    }
}

/// Orchestration action
#[derive(Debug, Clone)]
pub enum OrchAction {
    AdjustCpuWeight { cgroup_id: u64, new_weight: u32 },
    AdjustMemLimit { cgroup_id: u64, new_max: u64 },
    FreezeGroup { cgroup_id: u64 },
    ThawGroup { cgroup_id: u64 },
    MigrateProcess { pid: u64, from: u64, to: u64 },
    Rebalance,
}

/// Orchestrator stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CgroupOrchStats {
    pub total_cgroups: u64,
    pub total_controllers: u64,
    pub total_actions: u64,
    pub total_rebalances: u64,
    pub frozen_count: u64,
    pub oom_kills: u64,
    pub max_depth: u32,
}

/// Main cgroup orchestrator
pub struct HolisticCgroupOrch {
    nodes: BTreeMap<u64, CgroupNode>,
    root_id: u64,
    version: CgroupVersion,
    next_id: u64,
    action_log: Vec<OrchAction>,
    stats: CgroupOrchStats,
}

impl HolisticCgroupOrch {
    pub fn new(version: CgroupVersion) -> Self {
        let mut nodes = BTreeMap::new();
        let root = CgroupNode::new(1, String::from("/"), None, 0);
        nodes.insert(1, root);
        Self {
            nodes,
            root_id: 1,
            version,
            next_id: 2,
            action_log: Vec::new(),
            stats: CgroupOrchStats {
                total_cgroups: 1,
                total_controllers: 0,
                total_actions: 0,
                total_rebalances: 0,
                frozen_count: 0,
                oom_kills: 0,
                max_depth: 0,
            },
        }
    }

    pub fn create_cgroup(&mut self, parent_id: u64, name: &str) -> Option<u64> {
        let parent_path = self.nodes.get(&parent_id)?.path.clone();
        let parent_depth = self.nodes.get(&parent_id)?.depth;
        let id = self.next_id;
        self.next_id += 1;

        let path = if parent_path == "/" {
            alloc::format!("/{}", name)
        } else {
            alloc::format!("{}/{}", parent_path, name)
        };

        let depth = parent_depth + 1;
        if depth > self.stats.max_depth { self.stats.max_depth = depth; }

        let node = CgroupNode::new(id, path, Some(parent_id), depth);
        self.nodes.insert(id, node);
        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            parent.add_child(id);
        }
        self.stats.total_cgroups += 1;
        Some(id)
    }

    #[inline]
    pub fn enable_controller(&mut self, cgroup_id: u64, ctrl: CgroupController) {
        if let Some(node) = self.nodes.get_mut(&cgroup_id) {
            node.enable_controller(ctrl);
            self.stats.total_controllers += 1;
        }
    }

    #[inline]
    pub fn set_limits(&mut self, cgroup_id: u64, limits: CgroupLimits) {
        if let Some(node) = self.nodes.get_mut(&cgroup_id) {
            node.limits = limits;
        }
    }

    #[inline]
    pub fn update_usage(&mut self, cgroup_id: u64, usage: CgroupUsage) {
        if let Some(node) = self.nodes.get_mut(&cgroup_id) {
            node.usage = usage;
        }
    }

    #[inline]
    pub fn freeze(&mut self, cgroup_id: u64) {
        if let Some(node) = self.nodes.get_mut(&cgroup_id) {
            node.frozen = true;
            self.stats.frozen_count += 1;
            self.action_log.push(OrchAction::FreezeGroup { cgroup_id });
            self.stats.total_actions += 1;
        }
    }

    #[inline]
    pub fn thaw(&mut self, cgroup_id: u64) {
        if let Some(node) = self.nodes.get_mut(&cgroup_id) {
            if node.frozen {
                node.frozen = false;
                self.stats.frozen_count = self.stats.frozen_count.saturating_sub(1);
                self.action_log.push(OrchAction::ThawGroup { cgroup_id });
                self.stats.total_actions += 1;
            }
        }
    }

    #[inline]
    pub fn high_pressure_groups(&self, threshold: f64) -> Vec<(u64, f64)> {
        self.nodes.iter()
            .filter_map(|(&id, n)| {
                let pressure = n.memory_pressure();
                if pressure > threshold { Some((id, pressure)) } else { None }
            })
            .collect()
    }

    #[inline]
    pub fn throttled_groups(&self) -> Vec<u64> {
        self.nodes.iter()
            .filter(|(_, n)| n.is_throttled())
            .map(|(&id, _)| id)
            .collect()
    }

    #[inline]
    pub fn subtree_process_count(&self, cgroup_id: u64) -> u32 {
        let mut total = 0u32;
        let mut stack = alloc::vec![cgroup_id];
        while let Some(id) = stack.pop() {
            if let Some(node) = self.nodes.get(&id) {
                total += node.process_count;
                stack.extend_from_slice(&node.children);
            }
        }
        total
    }

    #[inline(always)]
    pub fn get_node(&self, id: u64) -> Option<&CgroupNode> {
        self.nodes.get(&id)
    }

    #[inline(always)]
    pub fn stats(&self) -> &CgroupOrchStats {
        &self.stats
    }
}
