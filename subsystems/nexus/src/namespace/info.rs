//! Namespace Information
//!
//! Namespace info structures and ID mappings.

use alloc::string::String;
use alloc::vec::Vec;

use super::{GroupId, NamespaceId, NamespaceState, NamespaceType, ProcessId, UserId};

/// Namespace information
#[derive(Debug, Clone)]
pub struct NamespaceInfo {
    /// Namespace ID
    pub id: NamespaceId,
    /// Namespace type
    pub ns_type: NamespaceType,
    /// Parent namespace (for hierarchical namespaces)
    pub parent: Option<NamespaceId>,
    /// Owner user namespace
    pub user_ns: Option<NamespaceId>,
    /// Current state
    pub state: NamespaceState,
    /// Reference count
    pub refcount: u32,
    /// Processes in namespace
    pub processes: Vec<ProcessId>,
    /// Creation timestamp
    pub created_at: u64,
    /// Creator process
    pub creator: ProcessId,
    /// Creator UID
    pub creator_uid: UserId,
}

impl NamespaceInfo {
    /// Create new namespace info
    pub fn new(
        id: NamespaceId,
        ns_type: NamespaceType,
        creator: ProcessId,
        creator_uid: UserId,
        timestamp: u64,
    ) -> Self {
        Self {
            id,
            ns_type,
            parent: None,
            user_ns: None,
            state: NamespaceState::Active,
            refcount: 1,
            processes: Vec::new(),
            created_at: timestamp,
            creator,
            creator_uid,
        }
    }

    /// Check if namespace is alive
    #[inline(always)]
    pub fn is_alive(&self) -> bool {
        matches!(self.state, NamespaceState::Active)
    }

    /// Check if namespace is empty
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.processes.is_empty()
    }

    /// Get process count
    #[inline(always)]
    pub fn process_count(&self) -> usize {
        self.processes.len()
    }
}

/// User namespace mapping
#[derive(Debug, Clone)]
pub struct IdMapping {
    /// Inside ID (in the namespace)
    pub inside_id: u32,
    /// Outside ID (in parent namespace)
    pub outside_id: u32,
    /// Count of IDs mapped
    pub count: u32,
}

impl IdMapping {
    /// Create new ID mapping
    pub fn new(inside_id: u32, outside_id: u32, count: u32) -> Self {
        Self {
            inside_id,
            outside_id,
            count,
        }
    }

    /// Check if ID is in mapping range
    #[inline(always)]
    pub fn contains(&self, id: u32) -> bool {
        id >= self.inside_id && id < self.inside_id + self.count
    }

    /// Map inside ID to outside ID
    #[inline]
    pub fn map_to_outside(&self, inside: u32) -> Option<u32> {
        if self.contains(inside) {
            Some(self.outside_id + (inside - self.inside_id))
        } else {
            None
        }
    }

    /// Map outside ID to inside ID
    #[inline]
    pub fn map_to_inside(&self, outside: u32) -> Option<u32> {
        if outside >= self.outside_id && outside < self.outside_id + self.count {
            Some(self.inside_id + (outside - self.outside_id))
        } else {
            None
        }
    }
}

/// User namespace extended info
#[derive(Debug, Clone)]
pub struct UserNamespaceInfo {
    /// Base namespace info
    pub base: NamespaceInfo,
    /// UID mappings
    pub uid_map: Vec<IdMapping>,
    /// GID mappings
    pub gid_map: Vec<IdMapping>,
    /// Projid mappings
    pub projid_map: Vec<IdMapping>,
    /// Level in user namespace hierarchy
    pub level: u32,
    /// Maximum depth allowed
    pub max_depth: u32,
}

impl UserNamespaceInfo {
    /// Create new user namespace info
    pub fn new(base: NamespaceInfo) -> Self {
        Self {
            base,
            uid_map: Vec::new(),
            gid_map: Vec::new(),
            projid_map: Vec::new(),
            level: 0,
            max_depth: 32,
        }
    }

    /// Map UID to parent namespace
    #[inline]
    pub fn map_uid(&self, uid: UserId) -> Option<UserId> {
        for mapping in &self.uid_map {
            if let Some(outside) = mapping.map_to_outside(uid.raw()) {
                return Some(UserId::new(outside));
            }
        }
        None
    }

    /// Map GID to parent namespace
    #[inline]
    pub fn map_gid(&self, gid: GroupId) -> Option<GroupId> {
        for mapping in &self.gid_map {
            if let Some(outside) = mapping.map_to_outside(gid.raw()) {
                return Some(GroupId::new(outside));
            }
        }
        None
    }

    /// Check if UID has mapping
    #[inline(always)]
    pub fn has_uid_mapping(&self, uid: UserId) -> bool {
        self.uid_map.iter().any(|m| m.contains(uid.raw()))
    }

    /// Check if GID has mapping
    #[inline(always)]
    pub fn has_gid_mapping(&self, gid: GroupId) -> bool {
        self.gid_map.iter().any(|m| m.contains(gid.raw()))
    }
}

/// PID namespace extended info
#[derive(Debug, Clone)]
pub struct PidNamespaceInfo {
    /// Base namespace info
    pub base: NamespaceInfo,
    /// Level in PID namespace hierarchy
    pub level: u32,
    /// PID allocator high watermark
    pub pid_max: u32,
    /// Last allocated PID
    pub last_pid: u32,
    /// Hide PID (for containers)
    pub hide_pid: u8,
    /// Init process in this namespace
    pub init_pid: Option<ProcessId>,
}

impl PidNamespaceInfo {
    /// Create new PID namespace info
    pub fn new(base: NamespaceInfo) -> Self {
        Self {
            base,
            level: 0,
            pid_max: 32768,
            last_pid: 0,
            hide_pid: 0,
            init_pid: None,
        }
    }

    /// Allocate PID
    #[inline]
    pub fn allocate_pid(&mut self) -> Option<u32> {
        if self.last_pid >= self.pid_max {
            // Wrap around (simplified)
            self.last_pid = 1;
        }
        self.last_pid += 1;
        Some(self.last_pid)
    }
}

/// Network namespace extended info
#[derive(Debug, Clone)]
pub struct NetNamespaceInfo {
    /// Base namespace info
    pub base: NamespaceInfo,
    /// Network devices
    pub devices: Vec<String>,
    /// Routing tables
    pub routing_tables: u32,
    /// Network namespaces linked via veth
    pub linked_ns: Vec<NamespaceId>,
}

impl NetNamespaceInfo {
    /// Create new network namespace info
    pub fn new(base: NamespaceInfo) -> Self {
        Self {
            base,
            devices: alloc::vec![String::from("lo")],
            routing_tables: 1,
            linked_ns: Vec::new(),
        }
    }

    /// Add network device
    #[inline]
    pub fn add_device(&mut self, name: String) {
        if !self.devices.contains(&name) {
            self.devices.push(name);
        }
    }

    /// Remove network device
    #[inline(always)]
    pub fn remove_device(&mut self, name: &str) {
        self.devices.retain(|d| d != name);
    }
}
