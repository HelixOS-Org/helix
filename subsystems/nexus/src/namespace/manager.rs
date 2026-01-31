//! Namespace Manager
//!
//! Managing namespaces lifecycle.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{
    IdMapping, NamespaceId, NamespaceInfo, NamespaceType, NetNamespaceInfo, PidNamespaceInfo,
    ProcessId, UserNamespaceInfo, UserId,
};

/// Namespace creation options
#[derive(Debug, Clone, Default)]
pub struct NamespaceOptions {
    /// Clone flags
    pub flags: u64,
    /// Parent user namespace
    pub parent_user_ns: Option<NamespaceId>,
    /// UID mappings (for user ns)
    pub uid_map: Vec<IdMapping>,
    /// GID mappings (for user ns)
    pub gid_map: Vec<IdMapping>,
}

/// Namespace manager
pub struct NamespaceManager {
    /// Namespaces by ID
    namespaces: BTreeMap<NamespaceId, NamespaceInfo>,
    /// User namespace info
    user_ns_info: BTreeMap<NamespaceId, UserNamespaceInfo>,
    /// PID namespace info
    pid_ns_info: BTreeMap<NamespaceId, PidNamespaceInfo>,
    /// Network namespace info
    net_ns_info: BTreeMap<NamespaceId, NetNamespaceInfo>,
    /// Namespaces by type
    by_type: BTreeMap<NamespaceType, Vec<NamespaceId>>,
    /// Process namespace membership
    process_ns: BTreeMap<ProcessId, BTreeMap<NamespaceType, NamespaceId>>,
    /// Next namespace ID
    next_id: AtomicU64,
    /// Total namespaces created
    total_created: AtomicU64,
    /// Total namespaces destroyed
    total_destroyed: AtomicU64,
}

impl NamespaceManager {
    /// Create new namespace manager
    pub fn new() -> Self {
        Self {
            namespaces: BTreeMap::new(),
            user_ns_info: BTreeMap::new(),
            pid_ns_info: BTreeMap::new(),
            net_ns_info: BTreeMap::new(),
            by_type: BTreeMap::new(),
            process_ns: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            total_created: AtomicU64::new(0),
            total_destroyed: AtomicU64::new(0),
        }
    }

    /// Allocate namespace ID
    pub fn allocate_id(&self) -> NamespaceId {
        NamespaceId::new(self.next_id.fetch_add(1, Ordering::Relaxed))
    }

    /// Create namespace
    pub fn create_namespace(
        &mut self,
        ns_type: NamespaceType,
        creator: ProcessId,
        creator_uid: UserId,
        timestamp: u64,
        options: &NamespaceOptions,
    ) -> NamespaceId {
        let id = self.allocate_id();
        let mut info = NamespaceInfo::new(id, ns_type, creator, creator_uid, timestamp);
        info.user_ns = options.parent_user_ns;

        // Add to type index
        self.by_type.entry(ns_type).or_default().push(id);

        // Create type-specific info
        match ns_type {
            NamespaceType::User => {
                let mut user_info = UserNamespaceInfo::new(info.clone());
                user_info.uid_map = options.uid_map.clone();
                user_info.gid_map = options.gid_map.clone();
                self.user_ns_info.insert(id, user_info);
            }
            NamespaceType::Pid => {
                let pid_info = PidNamespaceInfo::new(info.clone());
                self.pid_ns_info.insert(id, pid_info);
            }
            NamespaceType::Net => {
                let net_info = NetNamespaceInfo::new(info.clone());
                self.net_ns_info.insert(id, net_info);
            }
            _ => {}
        }

        self.namespaces.insert(id, info);
        self.total_created.fetch_add(1, Ordering::Relaxed);

        id
    }

    /// Delete namespace
    pub fn delete_namespace(&mut self, id: NamespaceId) -> bool {
        let info = match self.namespaces.get(&id) {
            Some(i) => i,
            None => return false,
        };

        // Can't delete if processes still inside
        if !info.is_empty() {
            return false;
        }

        let ns_type = info.ns_type;

        // Remove from type index
        if let Some(list) = self.by_type.get_mut(&ns_type) {
            list.retain(|&x| x != id);
        }

        // Remove type-specific info
        match ns_type {
            NamespaceType::User => {
                self.user_ns_info.remove(&id);
            }
            NamespaceType::Pid => {
                self.pid_ns_info.remove(&id);
            }
            NamespaceType::Net => {
                self.net_ns_info.remove(&id);
            }
            _ => {}
        }

        self.namespaces.remove(&id);
        self.total_destroyed.fetch_add(1, Ordering::Relaxed);

        true
    }

    /// Get namespace
    pub fn get_namespace(&self, id: NamespaceId) -> Option<&NamespaceInfo> {
        self.namespaces.get(&id)
    }

    /// Get namespace mutably
    pub fn get_namespace_mut(&mut self, id: NamespaceId) -> Option<&mut NamespaceInfo> {
        self.namespaces.get_mut(&id)
    }

    /// Get user namespace info
    pub fn get_user_ns(&self, id: NamespaceId) -> Option<&UserNamespaceInfo> {
        self.user_ns_info.get(&id)
    }

    /// Get PID namespace info
    pub fn get_pid_ns(&self, id: NamespaceId) -> Option<&PidNamespaceInfo> {
        self.pid_ns_info.get(&id)
    }

    /// Get network namespace info
    pub fn get_net_ns(&self, id: NamespaceId) -> Option<&NetNamespaceInfo> {
        self.net_ns_info.get(&id)
    }

    /// Add process to namespace
    pub fn add_process(
        &mut self,
        ns_id: NamespaceId,
        pid: ProcessId,
        ns_type: NamespaceType,
    ) -> bool {
        if let Some(info) = self.namespaces.get_mut(&ns_id) {
            if !info.processes.contains(&pid) {
                info.processes.push(pid);
                info.refcount += 1;
            }

            self.process_ns
                .entry(pid)
                .or_default()
                .insert(ns_type, ns_id);
            return true;
        }
        false
    }

    /// Remove process from namespace
    pub fn remove_process(&mut self, ns_id: NamespaceId, pid: ProcessId) -> bool {
        if let Some(info) = self.namespaces.get_mut(&ns_id) {
            info.processes.retain(|&p| p != pid);
            info.refcount = info.refcount.saturating_sub(1);

            if let Some(proc_ns) = self.process_ns.get_mut(&pid) {
                proc_ns.remove(&info.ns_type);
            }

            return true;
        }
        false
    }

    /// Get process namespaces
    pub fn get_process_namespaces(
        &self,
        pid: ProcessId,
    ) -> Option<&BTreeMap<NamespaceType, NamespaceId>> {
        self.process_ns.get(&pid)
    }

    /// Get namespaces by type
    pub fn get_by_type(&self, ns_type: NamespaceType) -> Vec<&NamespaceInfo> {
        self.by_type
            .get(&ns_type)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.namespaces.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Count namespaces by type
    pub fn count_by_type(&self, ns_type: NamespaceType) -> usize {
        self.by_type.get(&ns_type).map(|v| v.len()).unwrap_or(0)
    }

    /// Get total namespaces
    pub fn total_namespaces(&self) -> usize {
        self.namespaces.len()
    }

    /// Get total created
    pub fn total_created(&self) -> u64 {
        self.total_created.load(Ordering::Relaxed)
    }

    /// Get total destroyed
    pub fn total_destroyed(&self) -> u64 {
        self.total_destroyed.load(Ordering::Relaxed)
    }
}

impl Default for NamespaceManager {
    fn default() -> Self {
        Self::new()
    }
}
