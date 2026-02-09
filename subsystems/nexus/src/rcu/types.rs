//! Core types for RCU Intelligence
//!
//! This module provides fundamental identifiers and enumerations for RCU management.

/// Unique RCU domain identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RcuDomainId(pub u64);

impl RcuDomainId {
    /// Create a new domain ID
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

/// Unique CPU identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CpuId(pub u32);

impl CpuId {
    /// Create a new CPU ID
    #[inline(always)]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    #[inline(always)]
    pub const fn raw(&self) -> u32 {
        self.0
    }
}

/// Grace period identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GracePeriodId(pub u64);

impl GracePeriodId {
    /// Create a new grace period ID
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

/// RCU callback identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CallbackId(pub u64);

impl CallbackId {
    /// Create a new callback ID
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

/// RCU flavor type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RcuFlavor {
    /// Classic RCU
    Classic,
    /// RCU BH (bottom half)
    Bh,
    /// RCU Sched
    Sched,
    /// SRCU (Sleepable RCU)
    Srcu,
    /// Tasks RCU
    Tasks,
    /// Tasks Rude RCU
    TasksRude,
    /// Tasks Trace RCU
    TasksTrace,
    /// Expedited RCU
    Expedited,
}

impl RcuFlavor {
    /// Get flavor name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Classic => "rcu",
            Self::Bh => "rcu_bh",
            Self::Sched => "rcu_sched",
            Self::Srcu => "srcu",
            Self::Tasks => "rcu_tasks",
            Self::TasksRude => "rcu_tasks_rude",
            Self::TasksTrace => "rcu_tasks_trace",
            Self::Expedited => "rcu_expedited",
        }
    }

    /// Check if flavor supports sleeping
    #[inline]
    pub fn supports_sleeping(&self) -> bool {
        matches!(
            self,
            Self::Srcu | Self::Tasks | Self::TasksRude | Self::TasksTrace
        )
    }
}

/// RCU domain state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RcuDomainState {
    /// Idle, no grace period in progress
    Idle,
    /// Grace period starting
    Starting,
    /// Grace period active
    Active,
    /// Waiting for quiescent states
    WaitingQs,
    /// Grace period ending
    Ending,
    /// Expedited grace period
    Expedited,
    /// Stalled (potential problem)
    Stalled,
}
