//! Core types for Work Queue Intelligence
//!
//! This module provides fundamental identifiers and enumerations for work queue management.

/// Unique work queue identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkQueueId(pub u64);

impl WorkQueueId {
    /// Create a new work queue ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Unique work item identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkId(pub u64);

impl WorkId {
    /// Create a new work ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Unique CPU identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CpuId(pub u32);

impl CpuId {
    /// Create a new CPU ID
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    pub const fn raw(&self) -> u32 {
        self.0
    }
}

/// Work queue type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkQueueType {
    /// Bound to specific CPU
    Bound,
    /// Unbound, can run on any CPU
    Unbound,
    /// High priority work queue
    HighPriority,
    /// System work queue
    System,
    /// Freezable work queue
    Freezable,
    /// Memory reclaim work queue
    MemReclaim,
    /// Power efficient work queue
    PowerEfficient,
    /// CPU intensive work queue
    CpuIntensive,
    /// Ordered work queue (serialized execution)
    Ordered,
    /// Concurrent work queue
    Concurrent,
}

impl WorkQueueType {
    /// Get type name as string
    pub fn name(&self) -> &'static str {
        match self {
            Self::Bound => "bound",
            Self::Unbound => "unbound",
            Self::HighPriority => "highpri",
            Self::System => "system",
            Self::Freezable => "freezable",
            Self::MemReclaim => "memreclaim",
            Self::PowerEfficient => "power_efficient",
            Self::CpuIntensive => "cpu_intensive",
            Self::Ordered => "ordered",
            Self::Concurrent => "concurrent",
        }
    }
}

/// Work item priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorkPriority {
    /// Idle priority
    Idle     = 0,
    /// Low priority
    Low      = 1,
    /// Normal priority
    Normal   = 2,
    /// High priority
    High     = 3,
    /// Critical priority
    Critical = 4,
}

impl WorkPriority {
    /// Get priority as numeric value
    pub fn value(&self) -> u8 {
        *self as u8
    }

    /// Create from numeric value
    pub fn from_value(v: u8) -> Self {
        match v {
            0 => Self::Idle,
            1 => Self::Low,
            2 => Self::Normal,
            3 => Self::High,
            _ => Self::Critical,
        }
    }
}

/// Work item state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkState {
    /// Pending in queue
    Pending,
    /// Currently running
    Running,
    /// Completed successfully
    Completed,
    /// Cancelled
    Cancelled,
    /// Failed
    Failed,
    /// Delayed (waiting for timer)
    Delayed,
    /// Blocked (waiting for dependency)
    Blocked,
}
