// SPDX-License-Identifier: GPL-2.0
//! Holistic rcu_tree — hierarchical RCU tree for scalable grace period detection.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// RCU flavor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RcuTreeFlavor {
    Preemptible,
    NonPreemptible,
    Expedited,
    Tasks,
    Rude,
}

/// RCU node state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RcuNodeState {
    Idle,
    WaitingQs,
    QuiescentReported,
    GpInProgress,
}

/// Per-CPU RCU data
#[derive(Debug, Clone)]
pub struct RcuCpuData {
    pub cpu_id: u32,
    pub node_id: u32,
    pub quiescent_state: bool,
    pub qs_pending: bool,
    pub gp_seq: u64,
    pub callbacks_ready: u32,
    pub callbacks_pending: u32,
    pub callbacks_invoked: u64,
    pub qs_reported: u64,
    pub last_qs: u64,
}

impl RcuCpuData {
    pub fn new(cpu_id: u32, node_id: u32) -> Self {
        Self {
            cpu_id, node_id, quiescent_state: false,
            qs_pending: true, gp_seq: 0,
            callbacks_ready: 0, callbacks_pending: 0,
            callbacks_invoked: 0, qs_reported: 0, last_qs: 0,
        }
    }

    pub fn report_qs(&mut self, now: u64) {
        self.quiescent_state = true;
        self.qs_pending = false;
        self.qs_reported += 1;
        self.last_qs = now;
    }

    pub fn new_gp(&mut self, gp_seq: u64) {
        self.gp_seq = gp_seq;
        self.quiescent_state = false;
        self.qs_pending = true;
        self.callbacks_ready += self.callbacks_pending;
        self.callbacks_pending = 0;
    }

    pub fn add_callback(&mut self) { self.callbacks_pending += 1; }

    pub fn invoke_callbacks(&mut self) -> u32 {
        let n = self.callbacks_ready;
        self.callbacks_invoked += n as u64;
        self.callbacks_ready = 0;
        n
    }
}

/// RCU tree node (internal fan-out node)
#[derive(Debug, Clone)]
pub struct RcuTreeNode {
    pub id: u32,
    pub level: u32,
    pub parent_id: Option<u32>,
    pub state: RcuNodeState,
    pub qsmask: u64,
    pub qsmaskinit: u64,
    pub gp_seq: u64,
    pub children: Vec<u32>,
    pub cpu_ids: Vec<u32>,
}

impl RcuTreeNode {
    pub fn new(id: u32, level: u32) -> Self {
        Self {
            id, level, parent_id: None,
            state: RcuNodeState::Idle,
            qsmask: 0, qsmaskinit: 0, gp_seq: 0,
            children: Vec::new(), cpu_ids: Vec::new(),
        }
    }

    pub fn add_child(&mut self, child_id: u32, bit: u32) {
        self.children.push(child_id);
        if bit < 64 {
            self.qsmaskinit |= 1 << bit;
            self.qsmask |= 1 << bit;
        }
    }

    pub fn report_qs(&mut self, bit: u32) -> bool {
        if bit < 64 { self.qsmask &= !(1 << bit); }
        self.qsmask == 0
    }

    pub fn new_gp(&mut self, gp_seq: u64) {
        self.gp_seq = gp_seq;
        self.qsmask = self.qsmaskinit;
        self.state = RcuNodeState::WaitingQs;
    }

    pub fn all_reported(&self) -> bool { self.qsmask == 0 }
}

/// Grace period state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpState {
    Idle,
    Init,
    WaitQs,
    Cleanup,
    Completed,
}

/// Grace period tracking
#[derive(Debug, Clone)]
pub struct GracePeriodInfo {
    pub seq: u64,
    pub state: GpState,
    pub started_at: u64,
    pub completed_at: u64,
    pub duration_ns: u64,
}

impl GracePeriodInfo {
    pub fn new(seq: u64, now: u64) -> Self {
        Self { seq, state: GpState::Init, started_at: now, completed_at: 0, duration_ns: 0 }
    }

    pub fn complete(&mut self, now: u64) {
        self.state = GpState::Completed;
        self.completed_at = now;
        self.duration_ns = now.saturating_sub(self.started_at);
    }
}

/// RCU tree stats
#[derive(Debug, Clone)]
pub struct RcuTreeStats {
    pub total_cpus: u32,
    pub total_nodes: u32,
    pub tree_levels: u32,
    pub current_gp: u64,
    pub completed_gps: u64,
    pub total_callbacks: u64,
    pub avg_gp_duration_ns: u64,
    pub expedited_gps: u64,
}

/// Main RCU tree manager
pub struct HolisticRcuTree {
    flavor: RcuTreeFlavor,
    nodes: BTreeMap<u32, RcuTreeNode>,
    cpu_data: BTreeMap<u32, RcuCpuData>,
    gp_seq: u64,
    gp_history: Vec<GracePeriodInfo>,
    current_gp: Option<GracePeriodInfo>,
    max_gp_history: usize,
    root_node: u32,
    next_node_id: u32,
}

impl HolisticRcuTree {
    pub fn new(flavor: RcuTreeFlavor) -> Self {
        let root = 1;
        let mut nodes = BTreeMap::new();
        nodes.insert(root, RcuTreeNode::new(root, 0));
        Self {
            flavor, nodes, cpu_data: BTreeMap::new(),
            gp_seq: 0, gp_history: Vec::new(), current_gp: None,
            max_gp_history: 2048, root_node: root, next_node_id: 2,
        }
    }

    pub fn add_node(&mut self, level: u32, parent_id: u32) -> u32 {
        let id = self.next_node_id;
        self.next_node_id += 1;
        let mut node = RcuTreeNode::new(id, level);
        node.parent_id = Some(parent_id);
        let bit = self.nodes.get(&parent_id).map(|p| p.children.len() as u32).unwrap_or(0);
        if let Some(parent) = self.nodes.get_mut(&parent_id) { parent.add_child(id, bit); }
        self.nodes.insert(id, node);
        id
    }

    pub fn add_cpu(&mut self, cpu_id: u32, node_id: u32) {
        self.cpu_data.insert(cpu_id, RcuCpuData::new(cpu_id, node_id));
        if let Some(node) = self.nodes.get_mut(&node_id) { node.cpu_ids.push(cpu_id); }
    }

    pub fn start_gp(&mut self, now: u64) -> u64 {
        self.gp_seq += 1;
        let gp = GracePeriodInfo::new(self.gp_seq, now);
        for node in self.nodes.values_mut() { node.new_gp(self.gp_seq); }
        for cpu in self.cpu_data.values_mut() { cpu.new_gp(self.gp_seq); }
        self.current_gp = Some(gp);
        self.gp_seq
    }

    pub fn report_qs(&mut self, cpu_id: u32, now: u64) {
        let node_id = match self.cpu_data.get_mut(&cpu_id) {
            Some(cpu) => { cpu.report_qs(now); cpu.node_id },
            None => return,
        };
        // Propagate up the tree
        let mut current = node_id;
        loop {
            let all_done = self.nodes.get_mut(&current).map(|n| n.report_qs(0)).unwrap_or(true);
            if !all_done { break; }
            let parent = self.nodes.get(&current).and_then(|n| n.parent_id);
            match parent { Some(p) => current = p, None => break }
        }
    }

    pub fn check_gp_complete(&mut self, now: u64) -> bool {
        let root_done = self.nodes.get(&self.root_node).map(|n| n.all_reported()).unwrap_or(false);
        if root_done {
            if let Some(mut gp) = self.current_gp.take() {
                gp.complete(now);
                if self.gp_history.len() >= self.max_gp_history { self.gp_history.drain(..self.max_gp_history / 4); }
                self.gp_history.push(gp);
            }
            // Invoke callbacks
            for cpu in self.cpu_data.values_mut() { cpu.invoke_callbacks(); }
            true
        } else { false }
    }

    pub fn stats(&self) -> RcuTreeStats {
        let max_level = self.nodes.values().map(|n| n.level).max().unwrap_or(0);
        let total_cbs: u64 = self.cpu_data.values().map(|c| c.callbacks_invoked).sum();
        let avg_dur = if self.gp_history.is_empty() { 0 } else {
            self.gp_history.iter().map(|g| g.duration_ns).sum::<u64>() / self.gp_history.len() as u64
        };
        RcuTreeStats {
            total_cpus: self.cpu_data.len() as u32,
            total_nodes: self.nodes.len() as u32,
            tree_levels: max_level + 1,
            current_gp: self.gp_seq,
            completed_gps: self.gp_history.len() as u64,
            total_callbacks: total_cbs,
            avg_gp_duration_ns: avg_dur,
            expedited_gps: 0,
        }
    }
}

// ============================================================================
// Merged from rcu_tree_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RcuNodeLevel {
    Leaf,
    Interior,
    Root,
}

/// RCU tree node
#[derive(Debug)]
pub struct RcuTreeNode {
    pub id: u32,
    pub level: RcuNodeLevel,
    pub qsmask: u64,
    pub qsmaskinit: u64,
    pub gp_seq: u64,
    pub parent_id: Option<u32>,
    pub children: Vec<u32>,
}

impl RcuTreeNode {
    pub fn new(id: u32, level: RcuNodeLevel) -> Self {
        Self { id, level, qsmask: 0, qsmaskinit: 0, gp_seq: 0, parent_id: None, children: Vec::new() }
    }

    pub fn report_qs(&mut self, cpu_bit: u64) -> bool {
        self.qsmask &= !cpu_bit;
        self.qsmask == 0
    }
}

/// Grace period state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpStateV2 {
    Idle,
    Init,
    FqsWait,
    FqsForce,
    Cleanup,
}

/// Grace period tracker v2
#[derive(Debug)]
pub struct GpTrackerV2 {
    pub gp_seq: u64,
    pub state: GpStateV2,
    pub start_time: u64,
    pub completed: u64,
    pub fqs_count: u64,
}

impl GpTrackerV2 {
    pub fn new() -> Self { Self { gp_seq: 0, state: GpStateV2::Idle, start_time: 0, completed: 0, fqs_count: 0 } }

    pub fn start(&mut self, now: u64) { self.gp_seq += 1; self.state = GpStateV2::Init; self.start_time = now; }
    pub fn complete(&mut self) { self.state = GpStateV2::Idle; self.completed += 1; }
}

/// Stats
#[derive(Debug, Clone)]
pub struct RcuTreeV2Stats {
    pub total_nodes: u32,
    pub current_gp: u64,
    pub completed_gps: u64,
    pub fqs_count: u64,
    pub gp_state: GpStateV2,
}

/// Main holistic RCU tree v2
pub struct HolisticRcuTreeV2 {
    nodes: BTreeMap<u32, RcuTreeNode>,
    gp: GpTrackerV2,
}

impl HolisticRcuTreeV2 {
    pub fn new() -> Self { Self { nodes: BTreeMap::new(), gp: GpTrackerV2::new() } }

    pub fn add_node(&mut self, id: u32, level: RcuNodeLevel) { self.nodes.insert(id, RcuTreeNode::new(id, level)); }

    pub fn start_gp(&mut self, now: u64) {
        self.gp.start(now);
        for n in self.nodes.values_mut() { n.qsmask = n.qsmaskinit; n.gp_seq = self.gp.gp_seq; }
    }

    pub fn report_qs(&mut self, node_id: u32, cpu_bit: u64) -> bool {
        if let Some(n) = self.nodes.get_mut(&node_id) { n.report_qs(cpu_bit) } else { false }
    }

    pub fn check_complete(&mut self) -> bool {
        let root_done = self.nodes.values().filter(|n| n.level == RcuNodeLevel::Root).all(|n| n.qsmask == 0);
        if root_done { self.gp.complete(); true } else { false }
    }

    pub fn stats(&self) -> RcuTreeV2Stats {
        RcuTreeV2Stats { total_nodes: self.nodes.len() as u32, current_gp: self.gp.gp_seq, completed_gps: self.gp.completed, fqs_count: self.gp.fqs_count, gp_state: self.gp.state }
    }
}

// ============================================================================
// Merged from rcu_tree_v3
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RcuV3GpState {
    Idle,
    Started,
    WaitingForQs,
    ForcingQs,
    Completing,
    Expedited,
    Cleanup,
}

/// RCU node role in the hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RcuV3NodeRole {
    Root,
    Interior,
    Leaf,
}

/// A callback queued for execution after a grace period.
#[derive(Debug, Clone)]
pub struct RcuV3Callback {
    pub id: u64,
    pub gp_sequence: u64,
    pub registered_at: u64,
    pub cpu_id: u32,
    pub is_lazy: bool,
    pub batch_id: u64,
}

impl RcuV3Callback {
    pub fn new(id: u64, gp_sequence: u64, cpu_id: u32) -> Self {
        Self {
            id,
            gp_sequence,
            registered_at: 0,
            cpu_id,
            is_lazy: false,
            batch_id: 0,
        }
    }
}

/// Per-CPU RCU data.
#[derive(Debug, Clone)]
pub struct RcuV3CpuData {
    pub cpu_id: u32,
    pub qs_pending: bool,
    pub qs_completed_gp: u64,
    pub callbacks: Vec<RcuV3Callback>,
    pub nocb_mode: bool,
    pub callback_count: u64,
    pub offloaded_count: u64,
    pub online: bool,
}

impl RcuV3CpuData {
    pub fn new(cpu_id: u32) -> Self {
        Self {
            cpu_id,
            qs_pending: false,
            qs_completed_gp: 0,
            callbacks: Vec::new(),
            nocb_mode: false,
            callback_count: 0,
            offloaded_count: 0,
            online: true,
        }
    }

    pub fn report_qs(&mut self, gp_seq: u64) {
        if self.qs_pending && gp_seq >= self.qs_completed_gp {
            self.qs_pending = false;
            self.qs_completed_gp = gp_seq;
        }
    }

    pub fn enqueue_callback(&mut self, cb: RcuV3Callback) {
        self.callback_count += 1;
        self.callbacks.push(cb);
    }

    pub fn drain_completed(&mut self, gp_seq: u64) -> Vec<RcuV3Callback> {
        let mut completed = Vec::new();
        let mut remaining = Vec::new();
        for cb in self.callbacks.drain(..) {
            if cb.gp_sequence <= gp_seq {
                completed.push(cb);
            } else {
                remaining.push(cb);
            }
        }
        self.callbacks = remaining;
        completed
    }
}

/// RCU tree node.
#[derive(Debug, Clone)]
pub struct RcuV3Node {
    pub node_id: u32,
    pub role: RcuV3NodeRole,
    pub parent_id: Option<u32>,
    pub children: Vec<u32>,
    pub qs_mask: u64,
    pub qs_completed_mask: u64,
    pub level: u32,
    pub cpu_range_start: u32,
    pub cpu_range_end: u32,
}

impl RcuV3Node {
    pub fn new(node_id: u32, role: RcuV3NodeRole, level: u32) -> Self {
        Self {
            node_id,
            role,
            parent_id: None,
            children: Vec::new(),
            qs_mask: 0,
            qs_completed_mask: 0,
            level,
            cpu_range_start: 0,
            cpu_range_end: 0,
        }
    }

    pub fn all_qs_reported(&self) -> bool {
        self.qs_mask != 0 && self.qs_completed_mask == self.qs_mask
    }

    pub fn report_child_qs(&mut self, child_bit: u64) {
        self.qs_completed_mask |= child_bit & self.qs_mask;
    }
}

/// Statistics for the RCU tree V3.
#[derive(Debug, Clone)]
pub struct RcuTreeV3Stats {
    pub grace_periods_completed: u64,
    pub expedited_gps: u64,
    pub qs_forced: u64,
    pub callbacks_invoked: u64,
    pub callbacks_offloaded: u64,
    pub nocb_cpus: u64,
    pub tree_depth: u32,
    pub total_nodes: u64,
}

/// Main holistic RCU tree V3 manager.
pub struct HolisticRcuTreeV3 {
    pub current_gp: AtomicU64,
    pub gp_state: RcuV3GpState,
    pub nodes: BTreeMap<u32, RcuV3Node>,
    pub cpu_data: BTreeMap<u32, RcuV3CpuData>,
    pub stats: RcuTreeV3Stats,
}

impl HolisticRcuTreeV3 {
    pub fn new() -> Self {
        Self {
            current_gp: AtomicU64::new(0),
            gp_state: RcuV3GpState::Idle,
            nodes: BTreeMap::new(),
            cpu_data: BTreeMap::new(),
            stats: RcuTreeV3Stats {
                grace_periods_completed: 0,
                expedited_gps: 0,
                qs_forced: 0,
                callbacks_invoked: 0,
                callbacks_offloaded: 0,
                nocb_cpus: 0,
                tree_depth: 0,
                total_nodes: 0,
            },
        }
    }

    pub fn add_node(&mut self, node: RcuV3Node) {
        let id = node.node_id;
        if node.level + 1 > self.stats.tree_depth {
            self.stats.tree_depth = node.level + 1;
        }
        self.nodes.insert(id, node);
        self.stats.total_nodes += 1;
    }

    pub fn register_cpu(&mut self, cpu_id: u32, nocb: bool) {
        let mut data = RcuV3CpuData::new(cpu_id);
        data.nocb_mode = nocb;
        if nocb {
            self.stats.nocb_cpus += 1;
        }
        self.cpu_data.insert(cpu_id, data);
    }

    pub fn start_grace_period(&mut self) -> u64 {
        let gp = self.current_gp.fetch_add(1, Ordering::SeqCst) + 1;
        self.gp_state = RcuV3GpState::Started;
        // Mark all CPUs as needing to report QS
        for data in self.cpu_data.values_mut() {
            if data.online {
                data.qs_pending = true;
            }
        }
        // Set QS masks on leaf nodes
        for node in self.nodes.values_mut() {
            if node.role == RcuV3NodeRole::Leaf {
                node.qs_completed_mask = 0;
            }
        }
        self.gp_state = RcuV3GpState::WaitingForQs;
        gp
    }

    pub fn start_expedited_gp(&mut self) -> u64 {
        let gp = self.start_grace_period();
        self.gp_state = RcuV3GpState::Expedited;
        self.stats.expedited_gps += 1;
        gp
    }

    pub fn report_qs(&mut self, cpu_id: u32) -> bool {
        let gp = self.current_gp.load(Ordering::SeqCst);
        if let Some(data) = self.cpu_data.get_mut(&cpu_id) {
            data.report_qs(gp);
            true
        } else {
            false
        }
    }

    pub fn check_gp_complete(&self) -> bool {
        self.cpu_data
            .values()
            .filter(|d| d.online)
            .all(|d| !d.qs_pending)
    }

    pub fn complete_grace_period(&mut self) -> u64 {
        let gp = self.current_gp.load(Ordering::SeqCst);
        let mut invoked = 0u64;
        for data in self.cpu_data.values_mut() {
            let completed = data.drain_completed(gp);
            invoked += completed.len() as u64;
        }
        self.gp_state = RcuV3GpState::Idle;
        self.stats.grace_periods_completed += 1;
        self.stats.callbacks_invoked += invoked;
        invoked
    }

    pub fn cpu_count(&self) -> usize {
        self.cpu_data.len()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}
