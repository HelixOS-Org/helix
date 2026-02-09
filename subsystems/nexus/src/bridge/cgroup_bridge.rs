//! # Bridge Cgroup Bridge
//!
//! Bridges cgroup operations between kernel and userspace:
//! - Cgroup hierarchy traversal
//! - Controller attachment/detachment
//! - Resource limit forwarding
//! - Event notification relay
//! - Migration coordination

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Cgroup controller type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupController {
    Cpu,
    CpuSet,
    Memory,
    Io,
    Pids,
    Rdma,
    Hugetlb,
    Cpuacct,
    Devices,
    Freezer,
    NetCls,
    NetPrio,
    PerfEvent,
}

/// Cgroup version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupVersion {
    V1,
    V2,
    Hybrid,
}

/// Operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupOp {
    Create,
    Destroy,
    Attach,
    Detach,
    SetLimit,
    GetStat,
    Freeze,
    Unfreeze,
    Migrate,
    Notify,
}

/// Cgroup node in hierarchy
#[derive(Debug, Clone)]
pub struct CgroupNode {
    pub id: u64,
    pub name: String,
    pub parent_id: Option<u64>,
    pub children: Vec<u64>,
    pub controllers: Vec<CgroupController>,
    pub nr_tasks: u32,
    pub frozen: bool,
    pub depth: u16,
}

impl CgroupNode {
    pub fn new(id: u64, name: String, parent: Option<u64>, depth: u16) -> Self {
        Self { id, name, parent_id: parent, children: Vec::new(), controllers: Vec::new(), nr_tasks: 0, frozen: false, depth }
    }

    #[inline(always)]
    pub fn add_child(&mut self, child_id: u64) { self.children.push(child_id); }
    #[inline(always)]
    pub fn attach_controller(&mut self, ctrl: CgroupController) { if !self.controllers.contains(&ctrl) { self.controllers.push(ctrl); } }
    #[inline(always)]
    pub fn detach_controller(&mut self, ctrl: CgroupController) { self.controllers.retain(|c| c != &ctrl); }
}

/// Controller limit
#[derive(Debug, Clone)]
pub struct ControllerLimit {
    pub controller: CgroupController,
    pub cgroup_id: u64,
    pub param_name: String,
    pub value: u64,
    pub max_value: u64,
}

/// Cgroup event
#[derive(Debug, Clone)]
pub struct CgroupEvent {
    pub cgroup_id: u64,
    pub op: CgroupOp,
    pub controller: Option<CgroupController>,
    pub ts: u64,
    pub result: i32,
}

/// Migration request
#[derive(Debug, Clone)]
pub struct CgroupMigration {
    pub task_id: u64,
    pub from_cgroup: u64,
    pub to_cgroup: u64,
    pub ts: u64,
    pub charge_migrate: bool,
}

/// Cgroup bridge stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CgroupBridgeStats {
    pub total_cgroups: usize,
    pub total_ops: u64,
    pub total_migrations: u64,
    pub failed_ops: u64,
    pub frozen_count: usize,
    pub max_depth: u16,
}

/// Bridge cgroup manager
#[repr(align(64))]
pub struct BridgeCgroupBridge {
    nodes: BTreeMap<u64, CgroupNode>,
    limits: Vec<ControllerLimit>,
    events: Vec<CgroupEvent>,
    migrations: Vec<CgroupMigration>,
    version: CgroupVersion,
    stats: CgroupBridgeStats,
    next_id: u64,
}

impl BridgeCgroupBridge {
    pub fn new(ver: CgroupVersion) -> Self {
        let mut nodes = BTreeMap::new();
        nodes.insert(0, CgroupNode::new(0, String::from("/"), None, 0));
        Self { nodes, limits: Vec::new(), events: Vec::new(), migrations: Vec::new(), version: ver, stats: CgroupBridgeStats::default(), next_id: 1 }
    }

    #[inline]
    pub fn create_cgroup(&mut self, name: String, parent: u64, ts: u64) -> Option<u64> {
        let depth = self.nodes.get(&parent).map(|p| p.depth + 1)?;
        let id = self.next_id; self.next_id += 1;
        let node = CgroupNode::new(id, name, Some(parent), depth);
        self.nodes.insert(id, node);
        if let Some(p) = self.nodes.get_mut(&parent) { p.add_child(id); }
        self.events.push(CgroupEvent { cgroup_id: id, op: CgroupOp::Create, controller: None, ts, result: 0 });
        Some(id)
    }

    #[inline]
    pub fn destroy_cgroup(&mut self, id: u64, ts: u64) -> bool {
        if id == 0 { return false; }
        let ok = if let Some(n) = self.nodes.get(&id) { n.children.is_empty() && n.nr_tasks == 0 } else { return false; };
        if !ok { self.events.push(CgroupEvent { cgroup_id: id, op: CgroupOp::Destroy, controller: None, ts, result: -1 }); return false; }
        let parent = self.nodes.get(&id).and_then(|n| n.parent_id);
        self.nodes.remove(&id);
        if let Some(pid) = parent { if let Some(p) = self.nodes.get_mut(&pid) { p.children.retain(|&c| c != id); } }
        self.events.push(CgroupEvent { cgroup_id: id, op: CgroupOp::Destroy, controller: None, ts, result: 0 });
        true
    }

    #[inline(always)]
    pub fn attach_controller(&mut self, id: u64, ctrl: CgroupController, ts: u64) {
        if let Some(n) = self.nodes.get_mut(&id) { n.attach_controller(ctrl); }
        self.events.push(CgroupEvent { cgroup_id: id, op: CgroupOp::Attach, controller: Some(ctrl), ts, result: 0 });
    }

    #[inline(always)]
    pub fn set_limit(&mut self, id: u64, ctrl: CgroupController, param: String, val: u64, max: u64, ts: u64) {
        self.limits.push(ControllerLimit { controller: ctrl, cgroup_id: id, param_name: param, value: val, max_value: max });
        self.events.push(CgroupEvent { cgroup_id: id, op: CgroupOp::SetLimit, controller: Some(ctrl), ts, result: 0 });
    }

    #[inline]
    pub fn migrate_task(&mut self, task: u64, from: u64, to: u64, charge: bool, ts: u64) {
        if let Some(n) = self.nodes.get_mut(&from) { n.nr_tasks = n.nr_tasks.saturating_sub(1); }
        if let Some(n) = self.nodes.get_mut(&to) { n.nr_tasks += 1; }
        self.migrations.push(CgroupMigration { task_id: task, from_cgroup: from, to_cgroup: to, ts, charge_migrate: charge });
        self.events.push(CgroupEvent { cgroup_id: to, op: CgroupOp::Migrate, controller: None, ts, result: 0 });
    }

    #[inline(always)]
    pub fn freeze(&mut self, id: u64, ts: u64) {
        if let Some(n) = self.nodes.get_mut(&id) { n.frozen = true; }
        self.events.push(CgroupEvent { cgroup_id: id, op: CgroupOp::Freeze, controller: None, ts, result: 0 });
    }

    #[inline(always)]
    pub fn unfreeze(&mut self, id: u64, ts: u64) {
        if let Some(n) = self.nodes.get_mut(&id) { n.frozen = false; }
        self.events.push(CgroupEvent { cgroup_id: id, op: CgroupOp::Unfreeze, controller: None, ts, result: 0 });
    }

    #[inline]
    pub fn descendants(&self, id: u64) -> Vec<u64> {
        let mut result = Vec::new();
        let mut stack = alloc::vec![id];
        while let Some(cur) = stack.pop() {
            if cur != id { result.push(cur); }
            if let Some(n) = self.nodes.get(&cur) { stack.extend_from_slice(&n.children); }
        }
        result
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_cgroups = self.nodes.len();
        self.stats.total_ops = self.events.len() as u64;
        self.stats.total_migrations = self.migrations.len() as u64;
        self.stats.failed_ops = self.events.iter().filter(|e| e.result < 0).count() as u64;
        self.stats.frozen_count = self.nodes.values().filter(|n| n.frozen).count();
        self.stats.max_depth = self.nodes.values().map(|n| n.depth).max().unwrap_or(0);
    }

    #[inline(always)]
    pub fn node(&self, id: u64) -> Option<&CgroupNode> { self.nodes.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &CgroupBridgeStats { &self.stats }
}

// ============================================================================
// Merged from cgroup_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupV2Controller {
    Cpu,
    Memory,
    Io,
    Pids,
    Rdma,
    Hugetlb,
    Cpuset,
    Misc,
    Perf,
}

/// Resource distribution model
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistributionModel {
    Weight,
    Max,
    Burst,
    Idle,
}

/// Memory protection type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryProtection {
    Min,
    Low,
    High,
    Max,
    Swap,
}

/// Cgroup v2 node in the hierarchy
#[derive(Debug, Clone)]
pub struct CgroupV2Node {
    pub id: u64,
    pub name: String,
    pub parent_id: Option<u64>,
    pub children: Vec<u64>,
    pub controllers: Vec<CgroupV2Controller>,
    pub depth: u32,
    pub processes: u32,
    pub threads: u32,
    pub frozen: bool,
    pub populated: bool,
}

impl CgroupV2Node {
    pub fn new(id: u64, name: String, parent: Option<u64>, depth: u32) -> Self {
        Self {
            id, name, parent_id: parent, children: Vec::new(),
            controllers: Vec::new(), depth, processes: 0, threads: 0,
            frozen: false, populated: false,
        }
    }

    #[inline(always)]
    pub fn add_child(&mut self, child_id: u64) { self.children.push(child_id); }
    #[inline(always)]
    pub fn enable_controller(&mut self, ctrl: CgroupV2Controller) { self.controllers.push(ctrl); }
    #[inline(always)]
    pub fn is_leaf(&self) -> bool { self.children.is_empty() }
}

/// CPU weight configuration
#[derive(Debug, Clone)]
pub struct CpuWeightConfig {
    pub weight: u32,
    pub weight_nice: i32,
    pub max_usec: u64,
    pub max_period: u64,
    pub burst_usec: u64,
}

impl CpuWeightConfig {
    #[inline(always)]
    pub fn default_config() -> Self {
        Self { weight: 100, weight_nice: 0, max_usec: u64::MAX, max_period: 100_000, burst_usec: 0 }
    }
}

/// Memory limits configuration
#[derive(Debug, Clone)]
pub struct MemoryLimits {
    pub min_bytes: u64,
    pub low_bytes: u64,
    pub high_bytes: u64,
    pub max_bytes: u64,
    pub swap_max: u64,
    pub current: u64,
    pub oom_kills: u64,
}

impl MemoryLimits {
    #[inline(always)]
    pub fn unlimited() -> Self {
        Self { min_bytes: 0, low_bytes: 0, high_bytes: u64::MAX, max_bytes: u64::MAX, swap_max: u64::MAX, current: 0, oom_kills: 0 }
    }

    #[inline(always)]
    pub fn utilization(&self) -> f64 {
        if self.max_bytes == u64::MAX || self.max_bytes == 0 { return 0.0; }
        self.current as f64 / self.max_bytes as f64
    }
}

/// IO weight/max config
#[derive(Debug, Clone)]
pub struct IoConfig {
    pub weight: u32,
    pub rbps_max: u64,
    pub wbps_max: u64,
    pub riops_max: u64,
    pub wiops_max: u64,
}

impl IoConfig {
    #[inline(always)]
    pub fn default_config() -> Self {
        Self { weight: 100, rbps_max: u64::MAX, wbps_max: u64::MAX, riops_max: u64::MAX, wiops_max: u64::MAX }
    }
}

/// Cgroup v2 event
#[derive(Debug, Clone)]
pub struct CgroupV2Event {
    pub cgroup_id: u64,
    pub event_type: CgroupV2EventType,
    pub timestamp: u64,
}

/// Event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupV2EventType {
    Populated,
    Empty,
    Frozen,
    Thawed,
    OomKill,
    MemoryHigh,
    MemoryMax,
    IoLatency,
}

/// Bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CgroupV2BridgeStats {
    pub total_cgroups: u32,
    pub total_events: u64,
    pub oom_kills: u64,
    pub frozen_cgroups: u32,
    pub max_depth: u32,
}

/// Main cgroup v2 bridge
#[repr(align(64))]
pub struct BridgeCgroupV2 {
    nodes: BTreeMap<u64, CgroupV2Node>,
    cpu_configs: BTreeMap<u64, CpuWeightConfig>,
    mem_limits: BTreeMap<u64, MemoryLimits>,
    io_configs: BTreeMap<u64, IoConfig>,
    events: Vec<CgroupV2Event>,
    next_id: u64,
    max_events: usize,
}

impl BridgeCgroupV2 {
    pub fn new() -> Self {
        let mut nodes = BTreeMap::new();
        let root = CgroupV2Node::new(1, String::from("/"), None, 0);
        nodes.insert(1, root);
        Self {
            nodes, cpu_configs: BTreeMap::new(),
            mem_limits: BTreeMap::new(), io_configs: BTreeMap::new(),
            events: Vec::new(), next_id: 2, max_events: 4096,
        }
    }

    #[inline]
    pub fn create_cgroup(&mut self, name: String, parent_id: u64) -> Option<u64> {
        let depth = self.nodes.get(&parent_id)?.depth + 1;
        let id = self.next_id;
        self.next_id += 1;
        self.nodes.insert(id, CgroupV2Node::new(id, name, Some(parent_id), depth));
        if let Some(parent) = self.nodes.get_mut(&parent_id) { parent.add_child(id); }
        Some(id)
    }

    #[inline(always)]
    pub fn set_cpu_config(&mut self, id: u64, config: CpuWeightConfig) {
        self.cpu_configs.insert(id, config);
    }

    #[inline(always)]
    pub fn set_memory_limits(&mut self, id: u64, limits: MemoryLimits) {
        self.mem_limits.insert(id, limits);
    }

    #[inline(always)]
    pub fn set_io_config(&mut self, id: u64, config: IoConfig) {
        self.io_configs.insert(id, config);
    }

    #[inline]
    pub fn stats(&self) -> CgroupV2BridgeStats {
        let frozen = self.nodes.values().filter(|n| n.frozen).count() as u32;
        let max_depth = self.nodes.values().map(|n| n.depth).max().unwrap_or(0);
        let oom: u64 = self.mem_limits.values().map(|m| m.oom_kills).sum();
        CgroupV2BridgeStats {
            total_cgroups: self.nodes.len() as u32,
            total_events: self.events.len() as u64,
            oom_kills: oom, frozen_cgroups: frozen, max_depth,
        }
    }
}

// ============================================================================
// Merged from cgroup_v3_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupV3Controller {
    Cpu,
    Memory,
    Io,
    Pids,
    Cpuset,
    Rdma,
    Hugetlb,
    Misc,
}

/// Cgroup v3 node
#[derive(Debug)]
pub struct CgroupV3Node {
    pub id: u64,
    pub name: String,
    pub parent_id: Option<u64>,
    pub children: Vec<u64>,
    pub controllers: Vec<CgroupV3Controller>,
    pub processes: Vec<u64>,
    pub cpu_weight: u32,
    pub cpu_max_us: u64,
    pub memory_max: u64,
    pub memory_current: u64,
    pub pids_max: u32,
    pub pids_current: u32,
}

impl CgroupV3Node {
    pub fn new(id: u64, name: String, parent: Option<u64>) -> Self {
        Self { id, name, parent_id: parent, children: Vec::new(), controllers: Vec::new(), processes: Vec::new(), cpu_weight: 100, cpu_max_us: u64::MAX, memory_max: u64::MAX, memory_current: 0, pids_max: u32::MAX, pids_current: 0 }
    }

    #[inline(always)]
    pub fn memory_pressure(&self) -> f64 { if self.memory_max == u64::MAX || self.memory_max == 0 { 0.0 } else { self.memory_current as f64 / self.memory_max as f64 } }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CgroupV3BridgeStats {
    pub total_groups: u32,
    pub total_processes: u32,
    pub total_controllers: u32,
    pub avg_memory_pressure: f64,
}

/// Main cgroup v3 bridge
#[repr(align(64))]
pub struct BridgeCgroupV3 {
    groups: BTreeMap<u64, CgroupV3Node>,
    next_id: u64,
}

impl BridgeCgroupV3 {
    pub fn new() -> Self {
        let mut groups = BTreeMap::new();
        groups.insert(0, CgroupV3Node::new(0, String::from("/"), None));
        Self { groups, next_id: 1 }
    }

    #[inline]
    pub fn create(&mut self, name: String, parent: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.groups.insert(id, CgroupV3Node::new(id, name, Some(parent)));
        if let Some(p) = self.groups.get_mut(&parent) { p.children.push(id); }
        id
    }

    #[inline(always)]
    pub fn attach_process(&mut self, cg: u64, pid: u64) {
        if let Some(g) = self.groups.get_mut(&cg) { g.processes.push(pid); g.pids_current += 1; }
    }

    #[inline]
    pub fn stats(&self) -> CgroupV3BridgeStats {
        let procs: u32 = self.groups.values().map(|g| g.processes.len() as u32).sum();
        let ctrls: u32 = self.groups.values().map(|g| g.controllers.len() as u32).sum();
        let pressures: Vec<f64> = self.groups.values().map(|g| g.memory_pressure()).collect();
        let avg = if pressures.is_empty() { 0.0 } else { pressures.iter().sum::<f64>() / pressures.len() as f64 };
        CgroupV3BridgeStats { total_groups: self.groups.len() as u32, total_processes: procs, total_controllers: ctrls, avg_memory_pressure: avg }
    }
}

// ============================================================================
// Merged from cgroup_v4_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupV4Controller {
    Cpu,
    Memory,
    Io,
    Pids,
    Cpuset,
    Hugetlb,
    Rdma,
    Misc,
    DeviceFilter,
    PressureStall,
    NetPrio,
    NetCls,
    Freezer,
    Perf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupV4PressureKind {
    Some10,
    Some60,
    Some300,
    Full10,
    Full60,
    Full300,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupV4DelegateScope {
    None,
    Thread,
    Domain,
    DomainThreaded,
}

#[derive(Debug, Clone)]
pub struct CgroupV4Pressure {
    pub kind: CgroupV4PressureKind,
    pub avg_pct: u64,
    pub total_us: u64,
}

#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CgroupV4ControllerState {
    pub controller: CgroupV4Controller,
    pub enabled: bool,
    pub weight: u32,
    pub max_limit: u64,
    pub current_usage: u64,
    pub peak_usage: u64,
}

impl CgroupV4ControllerState {
    pub fn new(controller: CgroupV4Controller) -> Self {
        Self {
            controller,
            enabled: false,
            weight: 100,
            max_limit: u64::MAX,
            current_usage: 0,
            peak_usage: 0,
        }
    }

    #[inline]
    pub fn update_usage(&mut self, usage: u64) {
        self.current_usage = usage;
        if usage > self.peak_usage {
            self.peak_usage = usage;
        }
    }

    #[inline]
    pub fn utilization_pct(&self) -> u64 {
        if self.max_limit == 0 || self.max_limit == u64::MAX {
            return 0;
        }
        (self.current_usage * 100) / self.max_limit
    }
}

#[derive(Debug, Clone)]
pub struct CgroupV4Group {
    pub id: u64,
    pub path_hash: u64,
    pub delegate_scope: CgroupV4DelegateScope,
    pub controllers: Vec<CgroupV4ControllerState>,
    pub pressures: Vec<CgroupV4Pressure>,
    pub nr_procs: u32,
    pub nr_threads: u32,
    pub frozen: bool,
    pub kill_requested: bool,
}

impl CgroupV4Group {
    pub fn new(id: u64, path: &[u8]) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in path {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        Self {
            id,
            path_hash: h,
            delegate_scope: CgroupV4DelegateScope::None,
            controllers: Vec::new(),
            pressures: Vec::new(),
            nr_procs: 0,
            nr_threads: 0,
            frozen: false,
            kill_requested: false,
        }
    }

    #[inline]
    pub fn enable_controller(&mut self, ctrl: CgroupV4Controller) {
        if !self.controllers.iter().any(|c| c.controller == ctrl) {
            let mut state = CgroupV4ControllerState::new(ctrl);
            state.enabled = true;
            self.controllers.push(state);
        }
    }

    #[inline]
    pub fn total_pressure(&self) -> u64 {
        let mut total = 0u64;
        for p in &self.pressures {
            total = total.wrapping_add(p.total_us);
        }
        total
    }
}

#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CgroupV4BridgeStats {
    pub total_groups: u64,
    pub total_controllers_enabled: u64,
    pub total_frozen: u64,
    pub total_kills: u64,
    pub pressure_events: u64,
}

#[repr(align(64))]
pub struct BridgeCgroupV4 {
    groups: BTreeMap<u64, CgroupV4Group>,
    next_id: AtomicU64,
    stats: CgroupV4BridgeStats,
}

impl BridgeCgroupV4 {
    pub fn new() -> Self {
        Self {
            groups: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            stats: CgroupV4BridgeStats {
                total_groups: 0,
                total_controllers_enabled: 0,
                total_frozen: 0,
                total_kills: 0,
                pressure_events: 0,
            },
        }
    }

    #[inline]
    pub fn create_group(&mut self, path: &[u8]) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let group = CgroupV4Group::new(id, path);
        self.groups.insert(id, group);
        self.stats.total_groups += 1;
        id
    }

    #[inline]
    pub fn enable_controller(&mut self, group_id: u64, ctrl: CgroupV4Controller) {
        if let Some(g) = self.groups.get_mut(&group_id) {
            g.enable_controller(ctrl);
            self.stats.total_controllers_enabled += 1;
        }
    }

    #[inline]
    pub fn freeze_group(&mut self, group_id: u64) {
        if let Some(g) = self.groups.get_mut(&group_id) {
            if !g.frozen {
                g.frozen = true;
                self.stats.total_frozen += 1;
            }
        }
    }

    #[inline]
    pub fn kill_group(&mut self, group_id: u64) {
        if let Some(g) = self.groups.get_mut(&group_id) {
            g.kill_requested = true;
            self.stats.total_kills += 1;
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &CgroupV4BridgeStats {
        &self.stats
    }
}

// ============================================================================
// Merged from cgroup_v5_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeCgroupV5Controller {
    Cpu,
    Memory,
    Io,
    Pids,
    Cpuset,
    Hugetlb,
    Rdma,
    Misc,
}

/// Cgroup resource limit
#[derive(Debug, Clone)]
pub struct BridgeCgroupV5Limit {
    pub controller: BridgeCgroupV5Controller,
    pub max_value: u64,
    pub current_value: u64,
    pub soft_limit: u64,
}

/// Cgroup entry
#[derive(Debug, Clone)]
pub struct BridgeCgroupV5Entry {
    pub cg_id: u64,
    pub path: String,
    pub parent_id: Option<u64>,
    pub limits: Vec<BridgeCgroupV5Limit>,
    pub member_pids: u32,
}

/// Stats for cgroup operations
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BridgeCgroupV5Stats {
    pub total_cgroups: u64,
    pub active_cgroups: u64,
    pub limit_exceeded: u64,
    pub migrations: u64,
    pub oom_events: u64,
}

/// Manager for cgroup bridge operations
#[repr(align(64))]
pub struct BridgeCgroupV5Manager {
    cgroups: BTreeMap<u64, BridgeCgroupV5Entry>,
    pid_cgroup: LinearMap<u64, 64>,
    next_id: u64,
    stats: BridgeCgroupV5Stats,
}

impl BridgeCgroupV5Manager {
    pub fn new() -> Self {
        Self {
            cgroups: BTreeMap::new(),
            pid_cgroup: LinearMap::new(),
            next_id: 1,
            stats: BridgeCgroupV5Stats {
                total_cgroups: 0,
                active_cgroups: 0,
                limit_exceeded: 0,
                migrations: 0,
                oom_events: 0,
            },
        }
    }

    pub fn create_cgroup(&mut self, path: &str, parent: Option<u64>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let entry = BridgeCgroupV5Entry {
            cg_id: id,
            path: String::from(path),
            parent_id: parent,
            limits: Vec::new(),
            member_pids: 0,
        };
        self.cgroups.insert(id, entry);
        self.stats.total_cgroups += 1;
        self.stats.active_cgroups += 1;
        id
    }

    #[inline]
    pub fn set_limit(&mut self, cg_id: u64, controller: BridgeCgroupV5Controller, max_val: u64, soft: u64) {
        if let Some(cg) = self.cgroups.get_mut(&cg_id) {
            let limit = BridgeCgroupV5Limit {
                controller,
                max_value: max_val,
                current_value: 0,
                soft_limit: soft,
            };
            cg.limits.push(limit);
        }
    }

    pub fn attach_pid(&mut self, cg_id: u64, pid: u64) -> bool {
        if let Some(old_cg) = self.pid_cgroup.get(pid).cloned() {
            if let Some(cg) = self.cgroups.get_mut(&old_cg) {
                cg.member_pids = cg.member_pids.saturating_sub(1);
            }
            self.stats.migrations += 1;
        }
        if let Some(cg) = self.cgroups.get_mut(&cg_id) {
            cg.member_pids += 1;
            self.pid_cgroup.insert(pid, cg_id);
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn destroy_cgroup(&mut self, cg_id: u64) -> bool {
        if let Some(cg) = self.cgroups.remove(&cg_id) {
            self.stats.active_cgroups = self.stats.active_cgroups.saturating_sub(1);
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &BridgeCgroupV5Stats {
        &self.stats
    }
}
