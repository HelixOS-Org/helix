//! # Bridge Namespace Proxy
//!
//! Namespace-aware syscall proxying:
//! - PID namespace translation
//! - Mount namespace virtualization
//! - Network namespace routing
//! - User namespace UID/GID mapping
//! - IPC namespace isolation
//! - Cross-namespace reference management

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Namespace type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

/// Namespace descriptor
#[derive(Debug, Clone)]
pub struct NamespaceDesc {
    pub ns_id: u64,
    pub ns_type: NsType,
    pub parent_id: Option<u64>,
    pub owner_pid: u64,
    pub created_ns: u64,
    pub process_count: u32,
    pub depth: u8,
}

impl NamespaceDesc {
    pub fn new(ns_id: u64, ns_type: NsType, owner: u64, now: u64) -> Self {
        Self {
            ns_id,
            ns_type,
            parent_id: None,
            owner_pid: owner,
            created_ns: now,
            process_count: 0,
            depth: 0,
        }
    }
}

/// PID mapping (inner → outer)
#[derive(Debug, Clone)]
pub struct PidMapping {
    pub inner_pid: u64,
    pub outer_pid: u64,
    pub ns_id: u64,
}

/// UID/GID mapping
#[derive(Debug, Clone)]
pub struct IdMapping {
    pub inner_start: u32,
    pub outer_start: u32,
    pub count: u32,
    pub ns_id: u64,
}

impl IdMapping {
    #[inline]
    pub fn translate_to_outer(&self, inner: u32) -> Option<u32> {
        if inner >= self.inner_start && inner < self.inner_start + self.count {
            Some(self.outer_start + (inner - self.inner_start))
        } else { None }
    }

    #[inline]
    pub fn translate_to_inner(&self, outer: u32) -> Option<u32> {
        if outer >= self.outer_start && outer < self.outer_start + self.count {
            Some(self.inner_start + (outer - self.outer_start))
        } else { None }
    }
}

/// Cross-namespace reference
#[derive(Debug, Clone)]
pub struct NsReference {
    pub ref_id: u64,
    pub source_ns: u64,
    pub target_ns: u64,
    pub ref_type: NsRefType,
    pub resource_id: u64,
    pub created_ns: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NsRefType {
    FileDescriptor,
    Socket,
    PidRef,
    IpcKey,
    MountPoint,
}

/// Per-process namespace set
#[derive(Debug, Clone)]
pub struct ProcessNsSet {
    pub process_id: u64,
    pub pid_ns: u64,
    pub mount_ns: u64,
    pub net_ns: u64,
    pub user_ns: u64,
    pub ipc_ns: u64,
    pub uts_ns: u64,
    pub cgroup_ns: u64,
}

impl ProcessNsSet {
    pub fn new(pid: u64) -> Self {
        Self {
            process_id: pid,
            pid_ns: 1,
            mount_ns: 1,
            net_ns: 1,
            user_ns: 1,
            ipc_ns: 1,
            uts_ns: 1,
            cgroup_ns: 1,
        }
    }

    pub fn ns_for_type(&self, ns_type: NsType) -> u64 {
        match ns_type {
            NsType::Pid => self.pid_ns,
            NsType::Mount => self.mount_ns,
            NsType::Network => self.net_ns,
            NsType::User => self.user_ns,
            NsType::Ipc => self.ipc_ns,
            NsType::Uts => self.uts_ns,
            NsType::Cgroup => self.cgroup_ns,
            NsType::Time => 1,
        }
    }
}

/// Bridge Namespace Proxy stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BridgeNsProxyStats {
    pub total_namespaces: usize,
    pub total_pid_mappings: usize,
    pub total_id_mappings: usize,
    pub cross_ns_refs: usize,
    pub translation_count: u64,
}

/// Bridge Namespace Proxy
#[repr(align(64))]
pub struct BridgeNsProxy {
    namespaces: BTreeMap<u64, NamespaceDesc>,
    pid_mappings: Vec<PidMapping>,
    id_mappings: Vec<IdMapping>,
    references: Vec<NsReference>,
    process_ns: BTreeMap<u64, ProcessNsSet>,
    next_ns_id: u64,
    stats: BridgeNsProxyStats,
}

impl BridgeNsProxy {
    pub fn new() -> Self {
        Self {
            namespaces: BTreeMap::new(),
            pid_mappings: Vec::new(),
            id_mappings: Vec::new(),
            references: Vec::new(),
            process_ns: BTreeMap::new(),
            next_ns_id: 2,
            stats: BridgeNsProxyStats::default(),
        }
    }

    pub fn create_namespace(&mut self, ns_type: NsType, owner: u64, parent: Option<u64>, now: u64) -> u64 {
        let id = self.next_ns_id;
        self.next_ns_id += 1;
        let mut desc = NamespaceDesc::new(id, ns_type, owner, now);
        desc.parent_id = parent;
        desc.depth = parent
            .and_then(|pid| self.namespaces.get(&pid))
            .map(|p| p.depth + 1)
            .unwrap_or(0);
        self.namespaces.insert(id, desc);
        id
    }

    #[inline(always)]
    pub fn add_pid_mapping(&mut self, inner: u64, outer: u64, ns_id: u64) {
        self.pid_mappings.push(PidMapping { inner_pid: inner, outer_pid: outer, ns_id });
    }

    #[inline(always)]
    pub fn add_id_mapping(&mut self, mapping: IdMapping) {
        self.id_mappings.push(mapping);
    }

    #[inline(always)]
    pub fn set_process_ns(&mut self, pid: u64, ns_set: ProcessNsSet) {
        self.process_ns.insert(pid, ns_set);
    }

    /// Translate PID from one namespace to another
    pub fn translate_pid(&self, pid: u64, from_ns: u64, to_ns: u64) -> Option<u64> {
        if from_ns == to_ns { return Some(pid); }

        // Find outer PID from source ns
        let outer = self.pid_mappings.iter()
            .find(|m| m.inner_pid == pid && m.ns_id == from_ns)
            .map(|m| m.outer_pid)
            .unwrap_or(pid);

        // Find inner PID in target ns
        self.pid_mappings.iter()
            .find(|m| m.outer_pid == outer && m.ns_id == to_ns)
            .map(|m| m.inner_pid)
            .or(Some(outer))
    }

    /// Translate UID
    pub fn translate_uid(&self, uid: u32, ns_id: u64, to_outer: bool) -> Option<u32> {
        for mapping in &self.id_mappings {
            if mapping.ns_id == ns_id {
                if to_outer {
                    if let Some(outer) = mapping.translate_to_outer(uid) {
                        return Some(outer);
                    }
                } else {
                    if let Some(inner) = mapping.translate_to_inner(uid) {
                        return Some(inner);
                    }
                }
            }
        }
        None
    }

    #[inline(always)]
    pub fn add_reference(&mut self, ns_ref: NsReference) {
        self.references.push(ns_ref);
    }

    /// Check if two processes share a namespace
    #[inline]
    pub fn shares_namespace(&self, pid_a: u64, pid_b: u64, ns_type: NsType) -> bool {
        let ns_a = self.process_ns.get(&pid_a).map(|s| s.ns_for_type(ns_type));
        let ns_b = self.process_ns.get(&pid_b).map(|s| s.ns_for_type(ns_type));
        ns_a.is_some() && ns_a == ns_b
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_namespaces = self.namespaces.len();
        self.stats.total_pid_mappings = self.pid_mappings.len();
        self.stats.total_id_mappings = self.id_mappings.len();
        self.stats.cross_ns_refs = self.references.len();
    }

    #[inline(always)]
    pub fn namespace(&self, id: u64) -> Option<&NamespaceDesc> { self.namespaces.get(&id) }
    #[inline(always)]
    pub fn process_nset(&self, pid: u64) -> Option<&ProcessNsSet> { self.process_ns.get(&pid) }
    #[inline(always)]
    pub fn stats(&self) -> &BridgeNsProxyStats { &self.stats }
}
