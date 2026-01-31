//! Workload Information
//!
//! Workload metadata and lifecycle.

use alloc::string::String;

use super::{VirtId, VirtType, WorkloadPriority, WorkloadState};
use crate::core::NexusTimestamp;

/// Workload information
#[derive(Debug, Clone)]
pub struct WorkloadInfo {
    /// Unique ID
    pub id: VirtId,
    /// Name
    pub name: String,
    /// Type
    pub virt_type: VirtType,
    /// State
    pub state: WorkloadState,
    /// Assigned CPUs
    pub vcpus: u32,
    /// Memory (bytes)
    pub memory: u64,
    /// Created timestamp
    pub created_at: NexusTimestamp,
    /// Host node (for distributed)
    pub host_node: Option<u32>,
    /// Parent (for nested virt)
    pub parent: Option<VirtId>,
    /// Priority
    pub priority: WorkloadPriority,
}

impl WorkloadInfo {
    /// Create new workload info
    pub fn new(id: VirtId, name: &str, virt_type: VirtType) -> Self {
        Self {
            id,
            name: String::from(name),
            virt_type,
            state: WorkloadState::Pending,
            vcpus: 1,
            memory: 512 * 1024 * 1024,
            created_at: NexusTimestamp::now(),
            host_node: None,
            parent: None,
            priority: WorkloadPriority::Normal,
        }
    }

    /// Set resources
    pub fn with_resources(mut self, vcpus: u32, memory: u64) -> Self {
        self.vcpus = vcpus;
        self.memory = memory;
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: WorkloadPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set host node
    pub fn on_host(mut self, node: u32) -> Self {
        self.host_node = Some(node);
        self
    }

    /// Is running?
    pub fn is_running(&self) -> bool {
        self.state == WorkloadState::Running
    }

    /// Is migratable?
    pub fn is_migratable(&self) -> bool {
        matches!(self.state, WorkloadState::Running | WorkloadState::Paused)
    }

    /// Get uptime
    pub fn uptime(&self) -> u64 {
        NexusTimestamp::now().duration_since(self.created_at)
    }
}
