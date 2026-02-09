//! Resource Scheduler
//!
//! Intelligent resource scheduling for virtualized workloads.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::VirtId;
use crate::core::NexusTimestamp;

/// Intelligent resource scheduler
pub struct VirtResourceScheduler {
    /// Scheduling policies
    policies: BTreeMap<VirtId, SchedulingPolicy>,
    /// Resource reservations
    reservations: BTreeMap<VirtId, ResourceReservation>,
    /// Allocation history
    allocations: Vec<AllocationEvent>,
}

/// Scheduling policy
#[derive(Debug, Clone)]
pub struct SchedulingPolicy {
    /// Workload ID
    pub workload_id: VirtId,
    /// CPU policy
    pub cpu_policy: CpuPolicy,
    /// Memory policy
    pub memory_policy: MemoryPolicy,
    /// IO policy
    pub io_policy: IoPolicy,
}

/// CPU scheduling policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuPolicy {
    /// Fair share
    FairShare,
    /// Reserved
    Reserved,
    /// Burst allowed
    Burst,
    /// Capped
    Capped,
}

/// Memory policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPolicy {
    /// Soft limit
    Soft,
    /// Hard limit
    Hard,
    /// Overcommit allowed
    Overcommit,
}

/// IO policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoPolicy {
    /// Best effort
    BestEffort,
    /// Throttled
    Throttled,
    /// Priority
    Priority,
}

/// Resource reservation
#[derive(Debug, Clone)]
pub struct ResourceReservation {
    /// Workload ID
    pub workload_id: VirtId,
    /// Reserved CPUs
    pub reserved_cpus: f64,
    /// Maximum CPUs
    pub max_cpus: f64,
    /// Reserved memory
    pub reserved_memory: u64,
    /// Maximum memory
    pub max_memory: u64,
    /// IO weight
    pub io_weight: u32,
}

/// Allocation event
#[derive(Debug, Clone)]
pub struct AllocationEvent {
    /// Workload ID
    pub workload_id: VirtId,
    /// Timestamp
    pub timestamp: NexusTimestamp,
    /// Event type
    pub event_type: AllocationEventType,
    /// Amount
    pub amount: u64,
}

/// Allocation event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationEventType {
    /// CPU allocated
    CpuAllocated,
    /// CPU released
    CpuReleased,
    /// Memory allocated
    MemoryAllocated,
    /// Memory released
    MemoryReleased,
    /// Limit changed
    LimitChanged,
}

impl VirtResourceScheduler {
    /// Create new scheduler
    pub fn new() -> Self {
        Self {
            policies: BTreeMap::new(),
            reservations: BTreeMap::new(),
            allocations: Vec::new(),
        }
    }

    /// Set policy
    #[inline(always)]
    pub fn set_policy(&mut self, policy: SchedulingPolicy) {
        self.policies.insert(policy.workload_id, policy);
    }

    /// Get policy
    #[inline(always)]
    pub fn get_policy(&self, workload_id: VirtId) -> Option<&SchedulingPolicy> {
        self.policies.get(&workload_id)
    }

    /// Set reservation
    #[inline(always)]
    pub fn set_reservation(&mut self, reservation: ResourceReservation) {
        self.reservations
            .insert(reservation.workload_id, reservation);
    }

    /// Get reservation
    #[inline(always)]
    pub fn get_reservation(&self, workload_id: VirtId) -> Option<&ResourceReservation> {
        self.reservations.get(&workload_id)
    }

    /// Record allocation event
    #[inline(always)]
    pub fn record_event(&mut self, event: AllocationEvent) {
        self.allocations.push(event);
    }

    /// Get total reserved CPUs
    #[inline(always)]
    pub fn total_reserved_cpus(&self) -> f64 {
        self.reservations.values().map(|r| r.reserved_cpus).sum()
    }

    /// Get total reserved memory
    #[inline(always)]
    pub fn total_reserved_memory(&self) -> u64 {
        self.reservations.values().map(|r| r.reserved_memory).sum()
    }

    /// Policy count
    #[inline(always)]
    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }

    /// Reservation count
    #[inline(always)]
    pub fn reservation_count(&self) -> usize {
        self.reservations.len()
    }
}

impl Default for VirtResourceScheduler {
    fn default() -> Self {
        Self::new()
    }
}
