//! # Holistic Resource Pool
//!
//! Global resource pooling and sharing:
//! - Unified resource abstraction
//! - Pool partitioning and isolation
//! - Dynamic pool resizing
//! - Resource lending across pools
//! - Utilization tracking
//! - Fragmentation analysis

extern crate alloc;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::vec::Vec;

// ============================================================================
// POOL RESOURCE
// ============================================================================

/// Resource type for pooling
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PoolResourceType {
    /// CPU cores
    CpuCores,
    /// Physical memory pages
    MemoryPages,
    /// I/O bandwidth tokens
    IoBandwidth,
    /// Network bandwidth tokens
    NetworkBandwidth,
    /// IPC channel slots
    IpcSlots,
    /// File descriptors
    FileDescriptors,
    /// Timer slots
    TimerSlots,
    /// DMA buffers
    DmaBuffers,
}

/// Pool partition mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionMode {
    /// Fixed partitions
    Fixed,
    /// Proportional share
    Proportional,
    /// Dynamic (resize based on demand)
    Dynamic,
    /// Hierarchical (nested pools)
    Hierarchical,
}

// ============================================================================
// POOL PARTITION
// ============================================================================

/// A partition within a pool
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PoolPartition {
    /// Partition ID
    pub id: u64,
    /// Owner (process group or subsystem)
    pub owner: u64,
    /// Allocated capacity
    pub capacity: u64,
    /// Currently used
    pub used: u64,
    /// Reserved (guaranteed minimum)
    pub reserved: u64,
    /// Lent to other partitions
    pub lent: u64,
    /// Borrowed from other partitions
    pub borrowed: u64,
    /// Priority (higher = more important)
    pub priority: u32,
}

impl PoolPartition {
    pub fn new(id: u64, owner: u64, capacity: u64) -> Self {
        Self {
            id,
            owner,
            capacity,
            used: 0,
            reserved: 0,
            lent: 0,
            borrowed: 0,
            priority: 0,
        }
    }

    /// Available to allocate
    #[inline(always)]
    pub fn available(&self) -> u64 {
        let total = self.capacity + self.borrowed;
        total.saturating_sub(self.used + self.lent)
    }

    /// Available to lend
    #[inline(always)]
    pub fn lendable(&self) -> u64 {
        let effective = self.capacity.saturating_sub(self.lent);
        effective.saturating_sub(self.reserved.max(self.used))
    }

    /// Utilization
    #[inline]
    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 {
            return 0.0;
        }
        self.used as f64 / self.capacity as f64
    }

    /// Effective capacity (with borrows)
    #[inline(always)]
    pub fn effective_capacity(&self) -> u64 {
        self.capacity + self.borrowed - self.lent
    }

    /// Allocate
    #[inline]
    pub fn allocate(&mut self, amount: u64) -> bool {
        if self.available() >= amount {
            self.used += amount;
            true
        } else {
            false
        }
    }

    /// Release
    #[inline(always)]
    pub fn release(&mut self, amount: u64) {
        self.used = self.used.saturating_sub(amount);
    }

    /// Lend resources
    #[inline]
    pub fn lend(&mut self, amount: u64) -> bool {
        if self.lendable() >= amount {
            self.lent += amount;
            true
        } else {
            false
        }
    }

    /// Return lent resources
    #[inline(always)]
    pub fn return_lent(&mut self, amount: u64) {
        self.lent = self.lent.saturating_sub(amount);
    }

    /// Receive borrowed resources
    #[inline(always)]
    pub fn receive_borrow(&mut self, amount: u64) {
        self.borrowed += amount;
    }

    /// Return borrowed resources
    #[inline(always)]
    pub fn return_borrowed(&mut self, amount: u64) {
        self.borrowed = self.borrowed.saturating_sub(amount);
    }
}

// ============================================================================
// RESOURCE POOL
// ============================================================================

/// Pool state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolState {
    /// Normal operation
    Normal,
    /// Under pressure
    Pressure,
    /// Overcommitted
    Overcommitted,
    /// Critical shortage
    Critical,
}

/// A resource pool
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ResourcePool {
    /// Pool ID
    pub id: u64,
    /// Resource type
    pub resource_type: PoolResourceType,
    /// Total capacity
    pub total_capacity: u64,
    /// Partition mode
    pub mode: PartitionMode,
    /// Partitions
    pub partitions: BTreeMap<u64, PoolPartition>,
    /// Overcommit ratio (1.0 = no overcommit)
    pub overcommit_ratio: f64,
    /// State
    pub state: PoolState,
    /// Allocations count
    pub allocation_count: u64,
    /// Failure count
    pub failure_count: u64,
}

impl ResourcePool {
    pub fn new(
        id: u64,
        resource_type: PoolResourceType,
        total_capacity: u64,
        mode: PartitionMode,
    ) -> Self {
        Self {
            id,
            resource_type,
            total_capacity,
            mode,
            partitions: BTreeMap::new(),
            overcommit_ratio: 1.0,
            state: PoolState::Normal,
            allocation_count: 0,
            failure_count: 0,
        }
    }

    /// Add partition
    #[inline(always)]
    pub fn add_partition(&mut self, partition: PoolPartition) {
        self.partitions.insert(partition.id, partition);
    }

    /// Remove partition
    #[inline(always)]
    pub fn remove_partition(&mut self, id: u64) -> Option<PoolPartition> {
        self.partitions.remove(&id)
    }

    /// Total allocated across partitions
    #[inline(always)]
    pub fn total_allocated(&self) -> u64 {
        self.partitions.values().map(|p| p.capacity).sum()
    }

    /// Total used across partitions
    #[inline(always)]
    pub fn total_used(&self) -> u64 {
        self.partitions.values().map(|p| p.used).sum()
    }

    /// Unpartitioned capacity
    #[inline(always)]
    pub fn unpartitioned(&self) -> u64 {
        let virtual_cap = (self.total_capacity as f64 * self.overcommit_ratio) as u64;
        virtual_cap.saturating_sub(self.total_allocated())
    }

    /// Overall utilization
    #[inline]
    pub fn utilization(&self) -> f64 {
        if self.total_capacity == 0 {
            return 0.0;
        }
        self.total_used() as f64 / self.total_capacity as f64
    }

    /// Update pool state
    pub fn update_state(&mut self) {
        let util = self.utilization();
        self.state = if util > 0.95 {
            PoolState::Critical
        } else if self.total_allocated() > self.total_capacity {
            PoolState::Overcommitted
        } else if util > 0.8 {
            PoolState::Pressure
        } else {
            PoolState::Normal
        };
    }

    /// Allocate from partition
    #[inline]
    pub fn allocate(&mut self, partition_id: u64, amount: u64) -> bool {
        if let Some(partition) = self.partitions.get_mut(&partition_id) {
            if partition.allocate(amount) {
                self.allocation_count += 1;
                self.update_state();
                return true;
            }
        }
        self.failure_count += 1;
        false
    }

    /// Release from partition
    #[inline]
    pub fn release(&mut self, partition_id: u64, amount: u64) {
        if let Some(partition) = self.partitions.get_mut(&partition_id) {
            partition.release(amount);
            self.update_state();
        }
    }

    /// Transfer resources between partitions
    pub fn transfer(&mut self, from_id: u64, to_id: u64, amount: u64) -> bool {
        // Check lendable
        let lendable = self
            .partitions
            .get(&from_id)
            .map(|p| p.lendable())
            .unwrap_or(0);

        if lendable < amount {
            return false;
        }

        if let Some(from) = self.partitions.get_mut(&from_id) {
            if !from.lend(amount) {
                return false;
            }
        } else {
            return false;
        }

        if let Some(to) = self.partitions.get_mut(&to_id) {
            to.receive_borrow(amount);
        } else {
            // Rollback
            if let Some(from) = self.partitions.get_mut(&from_id) {
                from.return_lent(amount);
            }
            return false;
        }

        true
    }

    /// Failure rate
    #[inline]
    pub fn failure_rate(&self) -> f64 {
        let total = self.allocation_count + self.failure_count;
        if total == 0 {
            return 0.0;
        }
        self.failure_count as f64 / total as f64
    }
}

// ============================================================================
// FRAGMENTATION ANALYSIS
// ============================================================================

/// Fragmentation metrics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FragmentationMetrics {
    /// External fragmentation (0.0-1.0)
    pub external: f64,
    /// Internal fragmentation (0.0-1.0)
    pub internal: f64,
    /// Number of fragments
    pub fragment_count: usize,
    /// Largest contiguous free
    pub largest_free: u64,
    /// Smallest fragment
    pub smallest_fragment: u64,
}

impl FragmentationMetrics {
    pub fn new() -> Self {
        Self {
            external: 0.0,
            internal: 0.0,
            fragment_count: 0,
            largest_free: 0,
            smallest_fragment: u64::MAX,
        }
    }

    /// Compute for a pool
    pub fn compute(pool: &ResourcePool) -> Self {
        let total_free = pool.total_capacity.saturating_sub(pool.total_used());
        let mut metrics = Self::new();

        if pool.partitions.is_empty() || total_free == 0 {
            return metrics;
        }

        // External fragmentation: how spread out is free space
        let partition_frees: Vec<u64> = pool.partitions.values().map(|p| p.available()).collect();

        let max_free = partition_frees.iter().copied().max().unwrap_or(0);
        metrics.largest_free = max_free;
        metrics.smallest_fragment = partition_frees.iter().copied().min().unwrap_or(0);
        metrics.fragment_count = partition_frees.iter().filter(|&&f| f > 0).count();

        if total_free > 0 {
            metrics.external = 1.0 - (max_free as f64 / total_free as f64);
        }

        // Internal fragmentation: allocated but unused capacity
        let total_capacity: u64 = pool.partitions.values().map(|p| p.capacity).sum();
        let total_used: u64 = pool.partitions.values().map(|p| p.used).sum();
        if total_capacity > 0 {
            metrics.internal = 1.0 - (total_used as f64 / total_capacity as f64);
        }

        metrics
    }
}

// ============================================================================
// POOL MANAGER
// ============================================================================

/// Pool manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct ResourcePoolStats {
    /// Total pools
    pub pool_count: usize,
    /// Pools under pressure
    pub pressured_pools: usize,
    /// Total allocation failures
    pub total_failures: u64,
    /// Active transfers
    pub active_transfers: u64,
    /// Average utilization
    pub avg_utilization: f64,
}

/// Global resource pool manager
#[repr(align(64))]
pub struct HolisticResourcePoolManager {
    /// Pools by ID
    pools: BTreeMap<u64, ResourcePool>,
    /// Resource type → pool ID mapping
    type_mapping: BTreeMap<u8, Vec<u64>>,
    /// Transfer history (from, to, amount)
    transfer_history: VecDeque<(u64, u64, u64)>,
    /// Next pool ID
    next_id: u64,
    /// Stats
    stats: ResourcePoolStats,
}

impl HolisticResourcePoolManager {
    pub fn new() -> Self {
        Self {
            pools: BTreeMap::new(),
            type_mapping: BTreeMap::new(),
            transfer_history: VecDeque::new(),
            next_id: 1,
            stats: ResourcePoolStats::default(),
        }
    }

    /// Create pool
    pub fn create_pool(
        &mut self,
        resource_type: PoolResourceType,
        capacity: u64,
        mode: PartitionMode,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let pool = ResourcePool::new(id, resource_type, capacity, mode);
        self.pools.insert(id, pool);
        self.type_mapping
            .entry(resource_type as u8)
            .or_insert_with(Vec::new)
            .push(id);
        self.update_stats();
        id
    }

    /// Get pool
    #[inline(always)]
    pub fn pool(&self, id: u64) -> Option<&ResourcePool> {
        self.pools.get(&id)
    }

    /// Get pool mut
    #[inline(always)]
    pub fn pool_mut(&mut self, id: u64) -> Option<&mut ResourcePool> {
        self.pools.get_mut(&id)
    }

    /// Find pools by resource type
    #[inline]
    pub fn pools_for_type(&self, resource_type: PoolResourceType) -> Vec<u64> {
        self.type_mapping
            .get(&(resource_type as u8))
            .cloned()
            .unwrap_or_default()
    }

    /// Allocate from pool
    #[inline]
    pub fn allocate(&mut self, pool_id: u64, partition_id: u64, amount: u64) -> bool {
        if let Some(pool) = self.pools.get_mut(&pool_id) {
            let result = pool.allocate(partition_id, amount);
            self.update_stats();
            result
        } else {
            false
        }
    }

    /// Release from pool
    #[inline]
    pub fn release(&mut self, pool_id: u64, partition_id: u64, amount: u64) {
        if let Some(pool) = self.pools.get_mut(&pool_id) {
            pool.release(partition_id, amount);
            self.update_stats();
        }
    }

    /// Transfer within pool
    pub fn transfer_within_pool(
        &mut self,
        pool_id: u64,
        from_partition: u64,
        to_partition: u64,
        amount: u64,
    ) -> bool {
        if let Some(pool) = self.pools.get_mut(&pool_id) {
            let result = pool.transfer(from_partition, to_partition, amount);
            if result {
                self.transfer_history
                    .push_back((from_partition, to_partition, amount));
                if self.transfer_history.len() > 1000 {
                    self.transfer_history.pop_front();
                }
            }
            self.update_stats();
            result
        } else {
            false
        }
    }

    /// Analyze fragmentation
    #[inline]
    pub fn analyze_fragmentation(&self, pool_id: u64) -> Option<FragmentationMetrics> {
        self.pools.get(&pool_id).map(FragmentationMetrics::compute)
    }

    fn update_stats(&mut self) {
        self.stats.pool_count = self.pools.len();
        self.stats.pressured_pools = self
            .pools
            .values()
            .filter(|p| matches!(p.state, PoolState::Pressure | PoolState::Critical))
            .count();
        self.stats.total_failures = self.pools.values().map(|p| p.failure_count).sum();

        if self.pools.is_empty() {
            self.stats.avg_utilization = 0.0;
        } else {
            let total_util: f64 = self.pools.values().map(|p| p.utilization()).sum();
            self.stats.avg_utilization = total_util / self.pools.len() as f64;
        }
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &ResourcePoolStats {
        &self.stats
    }
}
