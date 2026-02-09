//! Cgroup Core Types
//!
//! Fundamental types for cgroup management.

/// Cgroup identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CgroupId(pub u64);

impl CgroupId {
    /// Create a new cgroup ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    #[inline(always)]
    pub const fn raw(&self) -> u64 {
        self.0
    }

    /// Root cgroup ID
    pub const ROOT: Self = Self(0);
}

/// Process identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcessId(pub u64);

impl ProcessId {
    /// Create a new process ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    #[inline(always)]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Cgroup version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupVersion {
    /// Cgroup v1
    V1,
    /// Cgroup v2 (unified hierarchy)
    V2,
    /// Hybrid mode
    Hybrid,
}

/// Controller type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ControllerType {
    /// CPU controller
    Cpu,
    /// CPU accounting
    CpuAcct,
    /// CPU set (pinning)
    Cpuset,
    /// Memory controller
    Memory,
    /// Block I/O controller
    Blkio,
    /// Network controller
    Net,
    /// PIDs controller
    Pids,
    /// Freezer controller
    Freezer,
    /// Devices controller
    Devices,
    /// Perf events
    PerfEvent,
    /// RDMA controller
    Rdma,
    /// Hugetlb controller
    Hugetlb,
    /// Misc controller
    Misc,
}

impl ControllerType {
    /// Get controller name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::CpuAcct => "cpuacct",
            Self::Cpuset => "cpuset",
            Self::Memory => "memory",
            Self::Blkio => "blkio",
            Self::Net => "net_cls,net_prio",
            Self::Pids => "pids",
            Self::Freezer => "freezer",
            Self::Devices => "devices",
            Self::PerfEvent => "perf_event",
            Self::Rdma => "rdma",
            Self::Hugetlb => "hugetlb",
            Self::Misc => "misc",
        }
    }

    /// Check if controller is v2 only
    #[inline(always)]
    pub fn is_v2_only(&self) -> bool {
        matches!(self, Self::Misc)
    }
}

/// Cgroup state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupState {
    /// Active and running
    Active,
    /// Frozen
    Frozen,
    /// Freezing in progress
    Freezing,
    /// Being destroyed
    Dying,
    /// Destroyed
    Dead,
}
