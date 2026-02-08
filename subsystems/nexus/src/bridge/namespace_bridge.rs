// SPDX-License-Identifier: GPL-2.0
//! Bridge namespace_bridge — Linux namespace management bridge.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Namespace type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NsType {
    Mount,
    Uts,
    Ipc,
    Network,
    Pid,
    User,
    Cgroup,
    Time,
}

/// Namespace
#[derive(Debug)]
pub struct Namespace {
    pub id: u64,
    pub ns_type: NsType,
    pub inum: u64,
    pub creator_pid: u64,
    pub ref_count: u32,
    pub processes: Vec<u64>,
    pub parent: Option<u64>,
    pub created_at: u64,
}

impl Namespace {
    pub fn new(id: u64, nt: NsType, creator: u64, now: u64) -> Self {
        Self { id, ns_type: nt, inum: id * 0x100000 + 0xF0000000, creator_pid: creator, ref_count: 1, processes: Vec::new(), parent: None, created_at: now }
    }
}

/// Namespace set for a process
#[derive(Debug)]
pub struct ProcessNsSet {
    pub pid: u64,
    pub mnt_ns: u64,
    pub uts_ns: u64,
    pub ipc_ns: u64,
    pub net_ns: u64,
    pub pid_ns: u64,
    pub user_ns: u64,
    pub cgroup_ns: u64,
    pub time_ns: u64,
}

/// Stats
#[derive(Debug, Clone)]
pub struct NamespaceBridgeStats {
    pub total_namespaces: u32,
    pub by_type: [u32; 8],
    pub total_processes: u32,
    pub max_depth: u32,
}

/// Main namespace bridge
pub struct BridgeNamespace {
    namespaces: BTreeMap<u64, Namespace>,
    next_id: u64,
}

impl BridgeNamespace {
    pub fn new() -> Self { Self { namespaces: BTreeMap::new(), next_id: 1 } }

    pub fn create(&mut self, nt: NsType, creator: u64, now: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.namespaces.insert(id, Namespace::new(id, nt, creator, now));
        id
    }

    pub fn enter(&mut self, ns_id: u64, pid: u64) {
        if let Some(ns) = self.namespaces.get_mut(&ns_id) { ns.processes.push(pid); ns.ref_count += 1; }
    }

    pub fn stats(&self) -> NamespaceBridgeStats {
        let mut by_type = [0u32; 8];
        for ns in self.namespaces.values() {
            let idx = match ns.ns_type { NsType::Mount => 0, NsType::Uts => 1, NsType::Ipc => 2, NsType::Network => 3, NsType::Pid => 4, NsType::User => 5, NsType::Cgroup => 6, NsType::Time => 7 };
            by_type[idx] += 1;
        }
        let procs: u32 = self.namespaces.values().map(|n| n.processes.len() as u32).sum();
        NamespaceBridgeStats { total_namespaces: self.namespaces.len() as u32, by_type, total_processes: procs, max_depth: 0 }
    }
}

// ============================================================================
// Merged from namespace_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NsV2Type {
    Pid,
    Net,
    Mnt,
    Uts,
    Ipc,
    User,
    Cgroup,
    Time,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NsV2State {
    Active,
    Draining,
    Zombie,
    Destroyed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NsV2Ownership {
    Root,
    UserOwned(u32),
    Delegated(u32, u32),
}

#[derive(Debug, Clone)]
pub struct NsV2IdMapping {
    pub inside_start: u32,
    pub outside_start: u32,
    pub count: u32,
}

impl NsV2IdMapping {
    pub fn translate_to_outside(&self, inside_id: u32) -> Option<u32> {
        if inside_id >= self.inside_start && inside_id < self.inside_start + self.count {
            Some(self.outside_start + (inside_id - self.inside_start))
        } else {
            None
        }
    }

    pub fn translate_to_inside(&self, outside_id: u32) -> Option<u32> {
        if outside_id >= self.outside_start && outside_id < self.outside_start + self.count {
            Some(self.inside_start + (outside_id - self.outside_start))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct NsV2Instance {
    pub id: u64,
    pub ns_type: NsV2Type,
    pub state: NsV2State,
    pub parent_id: Option<u64>,
    pub depth: u32,
    pub ownership: NsV2Ownership,
    pub uid_mappings: Vec<NsV2IdMapping>,
    pub gid_mappings: Vec<NsV2IdMapping>,
    pub nr_processes: u32,
    pub created_at: u64,
}

impl NsV2Instance {
    pub fn new(id: u64, ns_type: NsV2Type, parent: Option<u64>, depth: u32) -> Self {
        Self {
            id,
            ns_type,
            state: NsV2State::Active,
            parent_id: parent,
            depth,
            ownership: NsV2Ownership::Root,
            uid_mappings: Vec::new(),
            gid_mappings: Vec::new(),
            nr_processes: 0,
            created_at: 0,
        }
    }

    pub fn add_uid_mapping(&mut self, inside: u32, outside: u32, count: u32) {
        self.uid_mappings.push(NsV2IdMapping {
            inside_start: inside,
            outside_start: outside,
            count,
        });
    }

    pub fn translate_uid(&self, inside_uid: u32) -> Option<u32> {
        for map in &self.uid_mappings {
            if let Some(outside) = map.translate_to_outside(inside_uid) {
                return Some(outside);
            }
        }
        None
    }

    pub fn drain(&mut self) {
        if self.state == NsV2State::Active {
            self.state = NsV2State::Draining;
        }
    }

    pub fn is_ancestor_of(&self, child_depth: u32) -> bool {
        child_depth > self.depth
    }
}

#[derive(Debug, Clone)]
pub struct NsV2BridgeStats {
    pub total_namespaces: u64,
    pub active_namespaces: u64,
    pub max_nesting_depth: u32,
    pub total_id_mappings: u64,
    pub total_translations: u64,
}

pub struct BridgeNamespaceV2 {
    namespaces: BTreeMap<u64, NsV2Instance>,
    children: BTreeMap<u64, Vec<u64>>,
    next_id: AtomicU64,
    stats: NsV2BridgeStats,
}

impl BridgeNamespaceV2 {
    pub fn new() -> Self {
        Self {
            namespaces: BTreeMap::new(),
            children: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            stats: NsV2BridgeStats {
                total_namespaces: 0,
                active_namespaces: 0,
                max_nesting_depth: 0,
                total_id_mappings: 0,
                total_translations: 0,
            },
        }
    }

    pub fn create_namespace(&mut self, ns_type: NsV2Type, parent: Option<u64>) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let depth = parent
            .and_then(|pid| self.namespaces.get(&pid))
            .map(|p| p.depth + 1)
            .unwrap_or(0);
        let ns = NsV2Instance::new(id, ns_type, parent, depth);
        if depth > self.stats.max_nesting_depth {
            self.stats.max_nesting_depth = depth;
        }
        if let Some(pid) = parent {
            self.children.entry(pid).or_insert_with(Vec::new).push(id);
        }
        self.namespaces.insert(id, ns);
        self.stats.total_namespaces += 1;
        self.stats.active_namespaces += 1;
        id
    }

    pub fn destroy_namespace(&mut self, id: u64) {
        if let Some(ns) = self.namespaces.get_mut(&id) {
            ns.state = NsV2State::Destroyed;
            if self.stats.active_namespaces > 0 {
                self.stats.active_namespaces -= 1;
            }
        }
    }

    pub fn stats(&self) -> &NsV2BridgeStats {
        &self.stats
    }
}

// ============================================================================
// Merged from namespace_v3_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeNsV3Type {
    Pid,
    Net,
    Mount,
    User,
    Uts,
    Ipc,
    Cgroup,
    Time,
}

/// Namespace entry
#[derive(Debug, Clone)]
pub struct BridgeNsV3Entry {
    pub ns_id: u64,
    pub ns_type: BridgeNsV3Type,
    pub parent_ns: Option<u64>,
    pub owner_uid: u32,
    pub member_count: u32,
    pub creation_time: u64,
}

/// Namespace operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeNsV3Op {
    Create,
    Enter,
    Leave,
    Share,
    Destroy,
}

/// Stats for namespace operations
#[derive(Debug, Clone)]
pub struct BridgeNsV3Stats {
    pub total_namespaces: u64,
    pub active_namespaces: u64,
    pub entries: u64,
    pub leaves: u64,
    pub max_depth: u32,
}

/// Manager for namespace bridge operations
pub struct BridgeNamespaceV3Manager {
    namespaces: BTreeMap<u64, BridgeNsV3Entry>,
    pid_to_ns: BTreeMap<u64, Vec<u64>>,
    next_ns: u64,
    stats: BridgeNsV3Stats,
}

impl BridgeNamespaceV3Manager {
    pub fn new() -> Self {
        Self {
            namespaces: BTreeMap::new(),
            pid_to_ns: BTreeMap::new(),
            next_ns: 1,
            stats: BridgeNsV3Stats {
                total_namespaces: 0,
                active_namespaces: 0,
                entries: 0,
                leaves: 0,
                max_depth: 0,
            },
        }
    }

    pub fn create_ns(&mut self, ns_type: BridgeNsV3Type, parent: Option<u64>, owner_uid: u32) -> u64 {
        let id = self.next_ns;
        self.next_ns += 1;
        let entry = BridgeNsV3Entry {
            ns_id: id,
            ns_type,
            parent_ns: parent,
            owner_uid,
            member_count: 0,
            creation_time: id.wrapping_mul(53),
        };
        self.namespaces.insert(id, entry);
        self.stats.total_namespaces += 1;
        self.stats.active_namespaces += 1;
        id
    }

    pub fn enter_ns(&mut self, pid: u64, ns_id: u64) -> bool {
        if let Some(ns) = self.namespaces.get_mut(&ns_id) {
            ns.member_count += 1;
            self.pid_to_ns.entry(pid).or_insert_with(Vec::new).push(ns_id);
            self.stats.entries += 1;
            true
        } else {
            false
        }
    }

    pub fn leave_ns(&mut self, pid: u64, ns_id: u64) -> bool {
        if let Some(ns) = self.namespaces.get_mut(&ns_id) {
            ns.member_count = ns.member_count.saturating_sub(1);
            if let Some(list) = self.pid_to_ns.get_mut(&pid) {
                list.retain(|&id| id != ns_id);
            }
            self.stats.leaves += 1;
            true
        } else {
            false
        }
    }

    pub fn destroy_ns(&mut self, ns_id: u64) -> bool {
        if self.namespaces.remove(&ns_id).is_some() {
            self.stats.active_namespaces = self.stats.active_namespaces.saturating_sub(1);
            true
        } else {
            false
        }
    }

    pub fn stats(&self) -> &BridgeNsV3Stats {
        &self.stats
    }
}
