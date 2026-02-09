//! # Coop Partition
//!
//! Cooperative resource partitioning:
//! - Dynamic resource slicing across process groups
//! - Partition negotiation protocol
//! - Proportional share with minimum guarantees
//! - Partition borrowing and lending
//! - Hot partition detection
//! - Partition merge/split operations

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Partition resource type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PartitionResource {
    /// CPU time slice
    CpuTime,
    /// Memory pages
    MemoryPages,
    /// IO bandwidth
    IoBandwidth,
    /// Network bandwidth
    NetBandwidth,
    /// Cache ways (cache partitioning)
    CacheWays,
    /// TLB entries
    TlbEntries,
}

/// A resource partition
#[derive(Debug, Clone)]
pub struct Partition {
    pub partition_id: u64,
    pub resource: PartitionResource,
    /// Total capacity units
    pub capacity: u64,
    /// Currently allocated units
    pub allocated: u64,
    /// Minimum guaranteed allocation per member
    pub min_guarantee: u64,
    /// Maximum allowed per member
    pub max_cap: u64,
    /// Members (pid -> allocation)
    pub members: LinearMap<u64, 64>,
    /// Weight-based proportional share
    pub weights: LinearMap<u32, 64>,
    pub created_ns: u64,
    /// Utilization (0.0-1.0)
    pub utilization: f64,
    pub active: bool,
}

impl Partition {
    pub fn new(id: u64, resource: PartitionResource, capacity: u64, now_ns: u64) -> Self {
        Self {
            partition_id: id,
            resource,
            capacity,
            allocated: 0,
            min_guarantee: 0,
            max_cap: capacity,
            members: LinearMap::new(),
            weights: LinearMap::new(),
            created_ns: now_ns,
            utilization: 0.0,
            active: true,
        }
    }

    /// Add a member with weight
    #[inline(always)]
    pub fn add_member(&mut self, pid: u64, weight: u32) {
        self.weights.insert(pid, weight);
        self.recompute_allocations();
    }

    /// Remove a member
    #[inline]
    pub fn remove_member(&mut self, pid: u64) {
        self.weights.remove(pid);
        self.members.remove(pid);
        self.recompute_allocations();
    }

    /// Recompute proportional allocations
    fn recompute_allocations(&mut self) {
        let total_weight: u32 = self.weights.values().sum();
        if total_weight == 0 {
            self.members.clear();
            self.allocated = 0;
            return;
        }

        self.members.clear();
        self.allocated = 0;

        // First pass: give minimum guarantees
        let member_count = self.weights.len() as u64;
        let total_min = self.min_guarantee * member_count;
        let remaining = if total_min < self.capacity {
            self.capacity - total_min
        } else {
            0
        };

        // Second pass: distribute proportionally
        for (&pid, &weight) in &self.weights {
            let proportional = if remaining > 0 {
                (remaining as u128 * weight as u128 / total_weight as u128) as u64
            } else {
                0
            };
            let alloc = (self.min_guarantee + proportional).min(self.max_cap);
            self.members.insert(pid, alloc);
            self.allocated += alloc;
        }
    }

    /// Get allocation for a member
    #[inline(always)]
    pub fn get_allocation(&self, pid: u64) -> u64 {
        self.members.get(pid).copied().unwrap_or(0)
    }

    /// Free capacity
    #[inline(always)]
    pub fn free_capacity(&self) -> u64 {
        self.capacity.saturating_sub(self.allocated)
    }

    /// Fragmentation: how much of capacity is unused
    #[inline]
    pub fn fragmentation(&self) -> f64 {
        if self.capacity == 0 {
            0.0
        } else {
            self.free_capacity() as f64 / self.capacity as f64
        }
    }

    /// Update utilization based on actual usage
    #[inline]
    pub fn update_utilization(&mut self, actual_used: u64) {
        if self.allocated == 0 {
            self.utilization = 0.0;
        } else {
            self.utilization = actual_used as f64 / self.allocated as f64;
        }
    }

    /// Is this partition hot? (high utilization)
    #[inline(always)]
    pub fn is_hot(&self) -> bool {
        self.utilization > 0.85
    }

    /// Is this partition cold? (low utilization)
    #[inline(always)]
    pub fn is_cold(&self) -> bool {
        self.utilization < 0.2 && self.members.len() > 1
    }
}

/// Partition borrow request
#[derive(Debug, Clone)]
pub struct PartitionBorrow {
    pub borrower_partition: u64,
    pub lender_partition: u64,
    pub amount: u64,
    pub timestamp_ns: u64,
    pub return_by_ns: u64,
    pub active: bool,
}

/// Partition split/merge operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionOp {
    /// Split partition into two
    Split,
    /// Merge two partitions
    Merge,
    /// Resize partition
    Resize,
    /// Rebalance allocations
    Rebalance,
}

/// Partition manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CoopPartitionStats {
    pub total_partitions: usize,
    pub active_partitions: usize,
    pub total_capacity: u64,
    pub total_allocated: u64,
    pub hot_partitions: usize,
    pub cold_partitions: usize,
    pub active_borrows: usize,
    pub avg_utilization: f64,
}

/// Coop Partition Manager
pub struct CoopPartitionManager {
    partitions: BTreeMap<u64, Partition>,
    borrows: Vec<PartitionBorrow>,
    stats: CoopPartitionStats,
    next_id: u64,
}

impl CoopPartitionManager {
    pub fn new() -> Self {
        Self {
            partitions: BTreeMap::new(),
            borrows: Vec::new(),
            stats: CoopPartitionStats::default(),
            next_id: 1,
        }
    }

    /// Create a partition
    #[inline]
    pub fn create_partition(
        &mut self,
        resource: PartitionResource,
        capacity: u64,
        min_guarantee: u64,
        now_ns: u64,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let mut part = Partition::new(id, resource, capacity, now_ns);
        part.min_guarantee = min_guarantee;
        self.partitions.insert(id, part);
        self.update_stats();
        id
    }

    /// Add member to partition
    #[inline]
    pub fn add_member(&mut self, partition_id: u64, pid: u64, weight: u32) {
        if let Some(part) = self.partitions.get_mut(&partition_id) {
            part.add_member(pid, weight);
        }
        self.update_stats();
    }

    /// Remove member
    #[inline]
    pub fn remove_member(&mut self, partition_id: u64, pid: u64) {
        if let Some(part) = self.partitions.get_mut(&partition_id) {
            part.remove_member(pid);
        }
        self.update_stats();
    }

    /// Borrow capacity from one partition to another
    pub fn borrow(
        &mut self,
        from_id: u64,
        to_id: u64,
        amount: u64,
        now_ns: u64,
        duration_ns: u64,
    ) -> bool {
        let can_lend = self
            .partitions
            .get(&from_id)
            .map(|p| p.free_capacity() >= amount && p.is_cold())
            .unwrap_or(false);

        if !can_lend {
            return false;
        }

        // Transfer capacity
        if let Some(from) = self.partitions.get_mut(&from_id) {
            from.capacity -= amount;
            from.recompute_allocations();
        }
        if let Some(to) = self.partitions.get_mut(&to_id) {
            to.capacity += amount;
            to.recompute_allocations();
        }

        self.borrows.push(PartitionBorrow {
            borrower_partition: to_id,
            lender_partition: from_id,
            amount,
            timestamp_ns: now_ns,
            return_by_ns: now_ns + duration_ns,
            active: true,
        });
        self.update_stats();
        true
    }

    /// Return borrowed capacity
    pub fn return_borrows(&mut self, now_ns: u64) {
        for borrow in self.borrows.iter_mut() {
            if borrow.active && now_ns >= borrow.return_by_ns {
                borrow.active = false;
                // Return capacity
                if let Some(to) = self.partitions.get_mut(&borrow.borrower_partition) {
                    if to.capacity >= borrow.amount {
                        to.capacity -= borrow.amount;
                        to.recompute_allocations();
                    }
                }
                if let Some(from) = self.partitions.get_mut(&borrow.lender_partition) {
                    from.capacity += borrow.amount;
                    from.recompute_allocations();
                }
            }
        }
        self.update_stats();
    }

    /// Split a partition
    pub fn split(&mut self, partition_id: u64, split_ratio: f64, now_ns: u64) -> Option<u64> {
        let (resource, cap1, cap2) = {
            let part = self.partitions.get(&partition_id)?;
            let cap1 = (part.capacity as f64 * split_ratio) as u64;
            let cap2 = part.capacity - cap1;
            (part.resource, cap1, cap2)
        };

        // Resize original
        if let Some(part) = self.partitions.get_mut(&partition_id) {
            part.capacity = cap1;
            part.recompute_allocations();
        }

        // Create new partition
        let new_id = self.create_partition(resource, cap2, 0, now_ns);
        self.update_stats();
        Some(new_id)
    }

    fn update_stats(&mut self) {
        self.stats.total_partitions = self.partitions.len();
        self.stats.active_partitions = self.partitions.values().filter(|p| p.active).count();
        self.stats.total_capacity = self.partitions.values().map(|p| p.capacity).sum();
        self.stats.total_allocated = self.partitions.values().map(|p| p.allocated).sum();
        self.stats.hot_partitions = self.partitions.values().filter(|p| p.is_hot()).count();
        self.stats.cold_partitions = self.partitions.values().filter(|p| p.is_cold()).count();
        self.stats.active_borrows = self.borrows.iter().filter(|b| b.active).count();
        if !self.partitions.is_empty() {
            self.stats.avg_utilization =
                self.partitions.values().map(|p| p.utilization).sum::<f64>()
                    / self.partitions.len() as f64;
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &CoopPartitionStats {
        &self.stats
    }
}
