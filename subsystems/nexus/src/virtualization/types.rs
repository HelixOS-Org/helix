//! Virtualization Core Types
//!
//! Fundamental types for virtualization management.

/// VM/Container identifier
pub type VirtId = u64;

/// Virtualization technology
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtType {
    /// Full virtualization (KVM-like)
    FullVirt,
    /// Paravirtualization
    ParaVirt,
    /// Container (namespace isolation)
    Container,
    /// Microvm (lightweight VM)
    MicroVm,
    /// Sandbox (application sandbox)
    Sandbox,
    /// Unikernel
    Unikernel,
}

impl VirtType {
    /// Get isolation level
    pub fn isolation_level(&self) -> IsolationLevel {
        match self {
            Self::FullVirt | Self::ParaVirt => IsolationLevel::Full,
            Self::MicroVm | Self::Unikernel => IsolationLevel::Full,
            Self::Container => IsolationLevel::Partial,
            Self::Sandbox => IsolationLevel::Minimal,
        }
    }

    /// Is hardware-accelerated?
    pub fn is_hardware_accelerated(&self) -> bool {
        matches!(self, Self::FullVirt | Self::ParaVirt | Self::MicroVm)
    }

    /// Typical overhead
    pub fn typical_overhead_percent(&self) -> f64 {
        match self {
            Self::FullVirt => 5.0,
            Self::ParaVirt => 3.0,
            Self::MicroVm => 1.5,
            Self::Container => 0.5,
            Self::Sandbox => 0.2,
            Self::Unikernel => 0.5,
        }
    }
}

/// Isolation level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IsolationLevel {
    /// No isolation
    None    = 0,
    /// Minimal (sandbox)
    Minimal = 1,
    /// Partial (containers)
    Partial = 2,
    /// Full (VMs)
    Full    = 3,
}

/// Workload state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadState {
    /// Pending creation
    Pending,
    /// Creating
    Creating,
    /// Running
    Running,
    /// Paused
    Paused,
    /// Migrating
    Migrating,
    /// Stopping
    Stopping,
    /// Stopped
    Stopped,
    /// Error state
    Error,
}

/// Workload priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorkloadPriority {
    /// Background
    Background  = 0,
    /// Normal
    Normal      = 1,
    /// Interactive
    Interactive = 2,
    /// Realtime
    Realtime    = 3,
    /// Critical
    Critical    = 4,
}
