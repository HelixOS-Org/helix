//! Cgroup Information
//!
//! Cgroup metadata and combined resource information.

use alloc::string::String;
use alloc::vec::Vec;

use super::{
    CgroupId, CgroupState, ControllerType, CpuLimits, CpuUsage, IoLimits, IoUsage, MemoryLimits,
    MemoryUsage, PidsLimits, ProcessId,
};

/// Cgroup information
#[derive(Debug, Clone)]
pub struct CgroupInfo {
    /// Cgroup ID
    pub id: CgroupId,
    /// Cgroup name
    pub name: String,
    /// Full path
    pub path: String,
    /// Parent cgroup
    pub parent: Option<CgroupId>,
    /// Children cgroups
    pub children: Vec<CgroupId>,
    /// Enabled controllers
    pub controllers: Vec<ControllerType>,
    /// Current state
    pub state: CgroupState,
    /// CPU limits
    pub cpu_limits: CpuLimits,
    /// Memory limits
    pub memory_limits: MemoryLimits,
    /// I/O limits
    pub io_limits: IoLimits,
    /// PIDs limits
    pub pids_limits: PidsLimits,
    /// CPU usage
    pub cpu_usage: CpuUsage,
    /// Memory usage
    pub memory_usage: MemoryUsage,
    /// I/O usage
    pub io_usage: IoUsage,
    /// Processes in cgroup
    pub processes: Vec<ProcessId>,
    /// Creation timestamp
    pub created_at: u64,
    /// Last update timestamp
    pub updated_at: u64,
}

impl CgroupInfo {
    /// Create new cgroup info
    pub fn new(id: CgroupId, name: String, path: String, timestamp: u64) -> Self {
        Self {
            id,
            name,
            path,
            parent: None,
            children: Vec::new(),
            controllers: Vec::new(),
            state: CgroupState::Active,
            cpu_limits: CpuLimits::default_limits(),
            memory_limits: MemoryLimits::default_limits(),
            io_limits: IoLimits::default_limits(),
            pids_limits: PidsLimits::default_limits(),
            cpu_usage: CpuUsage::default(),
            memory_usage: MemoryUsage::default(),
            io_usage: IoUsage::default(),
            processes: Vec::new(),
            created_at: timestamp,
            updated_at: timestamp,
        }
    }

    /// Check if cgroup is alive
    pub fn is_alive(&self) -> bool {
        matches!(
            self.state,
            CgroupState::Active | CgroupState::Frozen | CgroupState::Freezing
        )
    }

    /// Get process count
    pub fn process_count(&self) -> usize {
        self.processes.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.processes.is_empty() && self.children.is_empty()
    }

    /// Has controller
    pub fn has_controller(&self, controller: ControllerType) -> bool {
        self.controllers.contains(&controller)
    }
}
