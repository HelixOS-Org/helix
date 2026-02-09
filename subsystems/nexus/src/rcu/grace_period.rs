//! Grace Period Tracking
//!
//! This module provides structures for tracking RCU grace periods.

use alloc::vec::Vec;
use super::{RcuDomainId, GracePeriodId, CpuId};

/// Grace period information
#[derive(Debug, Clone)]
pub struct GracePeriodInfo {
    /// Grace period ID
    pub id: GracePeriodId,
    /// Domain ID
    pub domain_id: RcuDomainId,
    /// Start timestamp (nanoseconds)
    pub start_ns: u64,
    /// End timestamp if completed (nanoseconds)
    pub end_ns: Option<u64>,
    /// Is expedited
    pub expedited: bool,
    /// CPUs that have passed quiescent state
    pub cpus_qs: Vec<CpuId>,
    /// CPUs still pending
    pub cpus_pending: Vec<CpuId>,
    /// Callbacks waiting for this GP
    pub pending_callbacks: u64,
    /// Was forced (timeout)
    pub forced: bool,
}

impl GracePeriodInfo {
    /// Create new grace period info
    pub fn new(id: GracePeriodId, domain_id: RcuDomainId, start_ns: u64) -> Self {
        Self {
            id,
            domain_id,
            start_ns,
            end_ns: None,
            expedited: false,
            cpus_qs: Vec::new(),
            cpus_pending: Vec::new(),
            pending_callbacks: 0,
            forced: false,
        }
    }

    /// Get duration in nanoseconds
    #[inline(always)]
    pub fn duration_ns(&self) -> Option<u64> {
        self.end_ns.map(|end| end.saturating_sub(self.start_ns))
    }

    /// Get completion percentage
    #[inline]
    pub fn completion_pct(&self) -> f32 {
        let total = self.cpus_qs.len() + self.cpus_pending.len();
        if total == 0 {
            return 100.0;
        }
        (self.cpus_qs.len() as f32 / total as f32) * 100.0
    }

    /// Check if completed
    #[inline(always)]
    pub fn is_completed(&self) -> bool {
        self.end_ns.is_some()
    }
}

/// Grace period statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct GracePeriodStats {
    /// Total grace periods completed
    pub total_completed: u64,
    /// Total expedited grace periods
    pub expedited_count: u64,
    /// Total forced grace periods
    pub forced_count: u64,
    /// Minimum duration (nanoseconds)
    pub min_duration_ns: u64,
    /// Maximum duration (nanoseconds)
    pub max_duration_ns: u64,
    /// Average duration (nanoseconds)
    pub avg_duration_ns: u64,
    /// Current ongoing grace period
    pub current_gp: Option<GracePeriodId>,
    /// Stall count
    pub stall_count: u64,
}
