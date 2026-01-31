//! Hierarchy Manager
//!
//! Cgroup hierarchy management and process placement.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{CgroupId, CgroupInfo, CgroupVersion, ProcessId};

/// Hierarchy statistics
#[derive(Debug, Clone, Default)]
pub struct HierarchyStats {
    /// Total cgroups
    pub total_cgroups: u64,
    /// Maximum depth
    pub max_depth: u32,
    /// Total processes
    pub total_processes: u64,
    /// Empty cgroups
    pub empty_cgroups: u64,
}

/// Hierarchy manager
pub struct HierarchyManager {
    /// Cgroups by ID
    cgroups: BTreeMap<CgroupId, CgroupInfo>,
    /// Cgroups by path
    by_path: BTreeMap<String, CgroupId>,
    /// Process to cgroup mapping
    process_cgroup: BTreeMap<ProcessId, CgroupId>,
    /// Next cgroup ID
    next_id: AtomicU64,
    /// Cgroup version
    version: CgroupVersion,
    /// Root cgroup
    root: Option<CgroupId>,
}

impl HierarchyManager {
    /// Create new hierarchy manager
    pub fn new(version: CgroupVersion) -> Self {
        Self {
            cgroups: BTreeMap::new(),
            by_path: BTreeMap::new(),
            process_cgroup: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            version,
            root: None,
        }
    }

    /// Initialize root cgroup
    pub fn init_root(&mut self, timestamp: u64) -> CgroupId {
        let id = CgroupId::ROOT;
        let info = CgroupInfo::new(id, String::from("/"), String::from("/"), timestamp);

        self.cgroups.insert(id, info);
        self.by_path.insert(String::from("/"), id);
        self.root = Some(id);

        id
    }

    /// Allocate cgroup ID
    pub fn allocate_id(&self) -> CgroupId {
        CgroupId::new(self.next_id.fetch_add(1, Ordering::Relaxed))
    }

    /// Create cgroup
    pub fn create_cgroup(
        &mut self,
        parent: CgroupId,
        name: String,
        timestamp: u64,
    ) -> Option<CgroupId> {
        let parent_info = self.cgroups.get(&parent)?;
        let path = if parent_info.path == "/" {
            alloc::format!("/{}", name)
        } else {
            alloc::format!("{}/{}", parent_info.path, name)
        };

        if self.by_path.contains_key(&path) {
            return None;
        }

        let id = self.allocate_id();
        let mut info = CgroupInfo::new(id, name, path.clone(), timestamp);
        info.parent = Some(parent);
        info.controllers = parent_info.controllers.clone();

        self.cgroups.insert(id, info);
        self.by_path.insert(path, id);

        if let Some(parent_info) = self.cgroups.get_mut(&parent) {
            parent_info.children.push(id);
        }

        Some(id)
    }

    /// Delete cgroup
    pub fn delete_cgroup(&mut self, id: CgroupId) -> bool {
        let info = match self.cgroups.get(&id) {
            Some(i) => i,
            None => return false,
        };

        if !info.is_empty() {
            return false;
        }

        let path = info.path.clone();
        let parent = info.parent;

        if let Some(parent_id) = parent {
            if let Some(parent_info) = self.cgroups.get_mut(&parent_id) {
                parent_info.children.retain(|c| *c != id);
            }
        }

        self.cgroups.remove(&id);
        self.by_path.remove(&path);

        true
    }

    /// Get cgroup by ID
    pub fn get_cgroup(&self, id: CgroupId) -> Option<&CgroupInfo> {
        self.cgroups.get(&id)
    }

    /// Get cgroup by ID mutably
    pub fn get_cgroup_mut(&mut self, id: CgroupId) -> Option<&mut CgroupInfo> {
        self.cgroups.get_mut(&id)
    }

    /// Get cgroup by path
    pub fn get_by_path(&self, path: &str) -> Option<&CgroupInfo> {
        let id = self.by_path.get(path)?;
        self.cgroups.get(id)
    }

    /// Add process to cgroup
    pub fn add_process(&mut self, cgroup: CgroupId, pid: ProcessId) -> bool {
        if let Some(old_cgroup) = self.process_cgroup.remove(&pid) {
            if let Some(info) = self.cgroups.get_mut(&old_cgroup) {
                info.processes.retain(|p| *p != pid);
            }
        }

        if let Some(info) = self.cgroups.get_mut(&cgroup) {
            info.processes.push(pid);
            info.pids_limits.current = info.processes.len() as u64;
            self.process_cgroup.insert(pid, cgroup);
            return true;
        }

        false
    }

    /// Remove process from cgroup
    pub fn remove_process(&mut self, pid: ProcessId) -> bool {
        if let Some(cgroup) = self.process_cgroup.remove(&pid) {
            if let Some(info) = self.cgroups.get_mut(&cgroup) {
                info.processes.retain(|p| *p != pid);
                info.pids_limits.current = info.processes.len() as u64;
                return true;
            }
        }
        false
    }

    /// Get cgroup for process
    pub fn get_process_cgroup(&self, pid: ProcessId) -> Option<CgroupId> {
        self.process_cgroup.get(&pid).copied()
    }

    /// Get hierarchy depth
    pub fn get_depth(&self, id: CgroupId) -> u32 {
        let mut depth = 0;
        let mut current = id;

        while let Some(info) = self.cgroups.get(&current) {
            if let Some(parent) = info.parent {
                depth += 1;
                current = parent;
            } else {
                break;
            }
        }

        depth
    }

    /// Get hierarchy statistics
    pub fn stats(&self) -> HierarchyStats {
        let mut stats = HierarchyStats::default();
        stats.total_cgroups = self.cgroups.len() as u64;

        for info in self.cgroups.values() {
            stats.total_processes += info.processes.len() as u64;
            if info.is_empty() {
                stats.empty_cgroups += 1;
            }
            let depth = self.get_depth(info.id);
            if depth > stats.max_depth {
                stats.max_depth = depth;
            }
        }

        stats
    }

    /// Get version
    pub fn version(&self) -> CgroupVersion {
        self.version
    }

    /// List all cgroups
    pub fn list_cgroups(&self) -> Vec<&CgroupInfo> {
        self.cgroups.values().collect()
    }

    /// Cgroup count
    pub fn cgroup_count(&self) -> usize {
        self.cgroups.len()
    }
}

impl Default for HierarchyManager {
    fn default() -> Self {
        Self::new(CgroupVersion::V2)
    }
}
