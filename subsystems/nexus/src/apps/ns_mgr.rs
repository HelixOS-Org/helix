//! # Apps Namespace Manager
//!
//! Per-application namespace isolation tracking:
//! - PID namespace hierarchy
//! - Mount namespace management
//! - Network namespace isolation
//! - User namespace mapping
//! - IPC namespace state
//! - Cgroup namespace support

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Namespace type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NsType {
    Pid,
    Mount,
    Network,
    User,
    Ipc,
    Uts,
    Cgroup,
    Time,
}

/// Namespace state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NsState {
    Active,
    Zombie,
    Creating,
    Destroying,
}

/// ID mapping entry (user/group)
#[derive(Debug, Clone)]
pub struct IdMapping {
    pub inner_start: u32,
    pub outer_start: u32,
    pub count: u32,
}

/// Namespace descriptor
#[derive(Debug, Clone)]
pub struct NsDescriptor {
    pub id: u64,
    pub ns_type: NsType,
    pub state: NsState,
    pub parent_id: Option<u64>,
    pub children: Vec<u64>,
    pub owner_pid: u64,
    pub refcount: u32,
    pub created_at: u64,
    pub id_mappings: Vec<IdMapping>,
}

impl NsDescriptor {
    pub fn new(id: u64, ns_type: NsType, owner: u64, parent: Option<u64>) -> Self {
        Self {
            id, ns_type, state: NsState::Creating, parent_id: parent,
            children: Vec::new(), owner_pid: owner, refcount: 1, created_at: 0,
            id_mappings: Vec::new(),
        }
    }

    pub fn activate(&mut self) { self.state = NsState::Active; }
    pub fn add_ref(&mut self) { self.refcount += 1; }
    pub fn release(&mut self) -> bool { self.refcount = self.refcount.saturating_sub(1); self.refcount == 0 }
    pub fn add_mapping(&mut self, inner: u32, outer: u32, count: u32) { self.id_mappings.push(IdMapping { inner_start: inner, outer_start: outer, count }); }

    pub fn map_id(&self, inner: u32) -> Option<u32> {
        for m in &self.id_mappings {
            if inner >= m.inner_start && inner < m.inner_start + m.count {
                return Some(m.outer_start + (inner - m.inner_start));
            }
        }
        None
    }
}

/// Per-process namespace set
#[derive(Debug, Clone)]
pub struct ProcessNsSet {
    pub pid: u64,
    pub pid_ns: u64,
    pub mnt_ns: u64,
    pub net_ns: u64,
    pub user_ns: u64,
    pub ipc_ns: u64,
    pub uts_ns: u64,
    pub cgroup_ns: u64,
    pub time_ns: u64,
}

impl ProcessNsSet {
    pub fn new(pid: u64) -> Self {
        Self { pid, pid_ns: 0, mnt_ns: 0, net_ns: 0, user_ns: 0, ipc_ns: 0, uts_ns: 0, cgroup_ns: 0, time_ns: 0 }
    }

    pub fn get(&self, ns_type: NsType) -> u64 {
        match ns_type {
            NsType::Pid => self.pid_ns, NsType::Mount => self.mnt_ns,
            NsType::Network => self.net_ns, NsType::User => self.user_ns,
            NsType::Ipc => self.ipc_ns, NsType::Uts => self.uts_ns,
            NsType::Cgroup => self.cgroup_ns, NsType::Time => self.time_ns,
        }
    }

    pub fn set(&mut self, ns_type: NsType, id: u64) {
        match ns_type {
            NsType::Pid => self.pid_ns = id, NsType::Mount => self.mnt_ns = id,
            NsType::Network => self.net_ns = id, NsType::User => self.user_ns = id,
            NsType::Ipc => self.ipc_ns = id, NsType::Uts => self.uts_ns = id,
            NsType::Cgroup => self.cgroup_ns = id, NsType::Time => self.time_ns = id,
        }
    }
}

/// Namespace event
#[derive(Debug, Clone)]
pub struct NsEvent {
    pub ns_id: u64,
    pub ns_type: NsType,
    pub kind: NsEventKind,
    pub pid: u64,
    pub ts: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NsEventKind {
    Created,
    Entered,
    Exited,
    Destroyed,
    MappingAdded,
}

/// Namespace manager stats
#[derive(Debug, Clone, Default)]
pub struct NsMgrStats {
    pub total_namespaces: usize,
    pub active_namespaces: usize,
    pub tracked_processes: usize,
    pub pid_namespaces: usize,
    pub net_namespaces: usize,
    pub user_namespaces: usize,
    pub total_events: usize,
}

/// Apps namespace manager
pub struct AppsNsMgr {
    namespaces: BTreeMap<u64, NsDescriptor>,
    processes: BTreeMap<u64, ProcessNsSet>,
    events: Vec<NsEvent>,
    stats: NsMgrStats,
    next_id: u64,
}

impl AppsNsMgr {
    pub fn new() -> Self {
        let mut mgr = Self { namespaces: BTreeMap::new(), processes: BTreeMap::new(), events: Vec::new(), stats: NsMgrStats::default(), next_id: 1 };
        // Create initial namespaces (init ns)
        for &ns_type in &[NsType::Pid, NsType::Mount, NsType::Network, NsType::User, NsType::Ipc, NsType::Uts, NsType::Cgroup, NsType::Time] {
            let id = mgr.next_id; mgr.next_id += 1;
            let mut ns = NsDescriptor::new(id, ns_type, 1, None);
            ns.activate();
            mgr.namespaces.insert(id, ns);
        }
        mgr
    }

    pub fn create_ns(&mut self, ns_type: NsType, owner: u64, parent: Option<u64>, ts: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        let mut ns = NsDescriptor::new(id, ns_type, owner, parent);
        ns.created_at = ts;
        ns.activate();
        self.namespaces.insert(id, ns);
        if let Some(pid) = parent { if let Some(p) = self.namespaces.get_mut(&pid) { p.children.push(id); } }
        self.events.push(NsEvent { ns_id: id, ns_type, kind: NsEventKind::Created, pid: owner, ts });
        id
    }

    pub fn enter_ns(&mut self, pid: u64, ns_id: u64, ts: u64) {
        if let Some(ns) = self.namespaces.get_mut(&ns_id) {
            ns.add_ref();
            let ns_type = ns.ns_type;
            let proc_ns = self.processes.entry(pid).or_insert_with(|| ProcessNsSet::new(pid));
            proc_ns.set(ns_type, ns_id);
            self.events.push(NsEvent { ns_id, ns_type, kind: NsEventKind::Entered, pid, ts });
        }
    }

    pub fn exit_ns(&mut self, pid: u64, ns_type: NsType, ts: u64) {
        if let Some(proc_ns) = self.processes.get(&pid) {
            let ns_id = proc_ns.get(ns_type);
            if let Some(ns) = self.namespaces.get_mut(&ns_id) {
                let dead = ns.release();
                if dead { ns.state = NsState::Destroying; }
                self.events.push(NsEvent { ns_id, ns_type, kind: NsEventKind::Exited, pid, ts });
            }
        }
    }

    pub fn add_id_mapping(&mut self, ns_id: u64, inner: u32, outer: u32, count: u32) {
        if let Some(ns) = self.namespaces.get_mut(&ns_id) { ns.add_mapping(inner, outer, count); }
    }

    pub fn register_process(&mut self, pid: u64) { self.processes.entry(pid).or_insert_with(|| ProcessNsSet::new(pid)); }
    pub fn unregister_process(&mut self, pid: u64) { self.processes.remove(&pid); }

    pub fn same_ns(&self, pid1: u64, pid2: u64, ns_type: NsType) -> bool {
        let a = self.processes.get(&pid1).map(|p| p.get(ns_type));
        let b = self.processes.get(&pid2).map(|p| p.get(ns_type));
        a.is_some() && a == b
    }

    pub fn recompute(&mut self) {
        self.stats.total_namespaces = self.namespaces.len();
        self.stats.active_namespaces = self.namespaces.values().filter(|n| n.state == NsState::Active).count();
        self.stats.tracked_processes = self.processes.len();
        self.stats.pid_namespaces = self.namespaces.values().filter(|n| n.ns_type == NsType::Pid).count();
        self.stats.net_namespaces = self.namespaces.values().filter(|n| n.ns_type == NsType::Network).count();
        self.stats.user_namespaces = self.namespaces.values().filter(|n| n.ns_type == NsType::User).count();
        self.stats.total_events = self.events.len();
    }

    pub fn namespace(&self, id: u64) -> Option<&NsDescriptor> { self.namespaces.get(&id) }
    pub fn process_ns(&self, pid: u64) -> Option<&ProcessNsSet> { self.processes.get(&pid) }
    pub fn stats(&self) -> &NsMgrStats { &self.stats }
}
