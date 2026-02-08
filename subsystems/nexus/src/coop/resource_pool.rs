//! # Coop Resource Pool V2
//!
//! Second-generation cooperative resource pooling:
//! - Multi-tier pool hierarchy
//! - Adaptive pool sizing
//! - Cross-pool migration
//! - Pool fragmentation management
//! - Object lifecycle tracking
//! - Slab-style fast allocation

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Pool tier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PoolTier {
    /// Hot tier — fast, small capacity
    Hot,
    /// Warm tier — moderate speed/capacity
    Warm,
    /// Cold tier — slow, large capacity
    Cold,
    /// Overflow — emergency capacity
    Overflow,
}

/// Pool object state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolObjectState {
    /// Free — available for allocation
    Free,
    /// Allocated — in use
    Allocated,
    /// Reserved — held but not yet in use
    Reserved,
    /// Migrating — being moved between tiers
    Migrating,
}

/// A pool object
#[derive(Debug, Clone)]
pub struct PoolObject {
    pub object_id: u64,
    pub size: u32,
    pub state: PoolObjectState,
    pub owner_pid: Option<u64>,
    pub tier: PoolTier,
    pub alloc_ns: u64,
    pub last_access_ns: u64,
    pub access_count: u64,
}

impl PoolObject {
    pub fn new(id: u64, size: u32, tier: PoolTier) -> Self {
        Self {
            object_id: id,
            size,
            state: PoolObjectState::Free,
            owner_pid: None,
            tier,
            alloc_ns: 0,
            last_access_ns: 0,
            access_count: 0,
        }
    }

    pub fn allocate(&mut self, pid: u64, now_ns: u64) {
        self.state = PoolObjectState::Allocated;
        self.owner_pid = Some(pid);
        self.alloc_ns = now_ns;
        self.last_access_ns = now_ns;
        self.access_count = 1;
    }

    pub fn release(&mut self) {
        self.state = PoolObjectState::Free;
        self.owner_pid = None;
    }

    pub fn access(&mut self, now_ns: u64) {
        self.last_access_ns = now_ns;
        self.access_count += 1;
    }

    pub fn age_ns(&self, now_ns: u64) -> u64 {
        now_ns.saturating_sub(self.alloc_ns)
    }

    pub fn idle_ns(&self, now_ns: u64) -> u64 {
        now_ns.saturating_sub(self.last_access_ns)
    }
}

/// Slab for same-sized objects
#[derive(Debug)]
pub struct PoolSlab {
    pub slab_id: u64,
    pub object_size: u32,
    pub tier: PoolTier,
    objects: Vec<PoolObject>,
    pub capacity: usize,
    pub allocated_count: usize,
    next_obj_id: u64,
}

impl PoolSlab {
    pub fn new(id: u64, object_size: u32, capacity: usize, tier: PoolTier) -> Self {
        let mut objects = Vec::with_capacity(capacity);
        for i in 0..capacity {
            objects.push(PoolObject::new(i as u64, object_size, tier));
        }
        Self {
            slab_id: id,
            object_size,
            tier,
            objects,
            capacity,
            allocated_count: 0,
            next_obj_id: capacity as u64,
        }
    }

    pub fn allocate(&mut self, pid: u64, now_ns: u64) -> Option<u64> {
        for obj in self.objects.iter_mut() {
            if obj.state == PoolObjectState::Free {
                obj.allocate(pid, now_ns);
                self.allocated_count += 1;
                return Some(obj.object_id);
            }
        }
        None
    }

    pub fn release(&mut self, object_id: u64) -> bool {
        if let Some(obj) = self.objects.iter_mut().find(|o| o.object_id == object_id) {
            obj.release();
            if self.allocated_count > 0 {
                self.allocated_count -= 1;
            }
            true
        } else {
            false
        }
    }

    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 {
            0.0
        } else {
            self.allocated_count as f64 / self.capacity as f64
        }
    }

    pub fn free_count(&self) -> usize {
        self.capacity - self.allocated_count
    }

    /// Grow the slab
    pub fn grow(&mut self, additional: usize) {
        for _ in 0..additional {
            self.objects.push(PoolObject::new(
                self.next_obj_id,
                self.object_size,
                self.tier,
            ));
            self.next_obj_id += 1;
        }
        self.capacity += additional;
    }

    /// Shrink by removing free objects
    pub fn shrink(&mut self, target_free: usize) -> usize {
        let current_free = self.free_count();
        if current_free <= target_free {
            return 0;
        }
        let to_remove = current_free - target_free;
        let mut removed = 0;
        self.objects.retain(|obj| {
            if removed < to_remove && obj.state == PoolObjectState::Free {
                removed += 1;
                false
            } else {
                true
            }
        });
        self.capacity -= removed;
        removed
    }

    /// Cold objects (not accessed for a long time)
    pub fn cold_objects(&self, idle_threshold_ns: u64, now_ns: u64) -> Vec<u64> {
        self.objects
            .iter()
            .filter(|o| {
                o.state == PoolObjectState::Allocated && o.idle_ns(now_ns) > idle_threshold_ns
            })
            .map(|o| o.object_id)
            .collect()
    }
}

/// Resource pool stats
#[derive(Debug, Clone, Default)]
pub struct CoopResourcePoolV2Stats {
    pub total_slabs: usize,
    pub total_objects: usize,
    pub total_allocated: usize,
    pub total_free: usize,
    pub avg_utilization: f64,
    pub hot_slab_count: usize,
    pub cold_objects_count: usize,
}

/// Coop Resource Pool V2
pub struct CoopResourcePoolV2 {
    slabs: BTreeMap<u64, PoolSlab>,
    /// Size to slab mapping
    size_to_slab: BTreeMap<u32, Vec<u64>>,
    stats: CoopResourcePoolV2Stats,
    next_slab_id: u64,
}

impl CoopResourcePoolV2 {
    pub fn new() -> Self {
        Self {
            slabs: BTreeMap::new(),
            size_to_slab: BTreeMap::new(),
            stats: CoopResourcePoolV2Stats::default(),
            next_slab_id: 1,
        }
    }

    /// Create a slab for a specific object size
    pub fn create_slab(&mut self, object_size: u32, capacity: usize, tier: PoolTier) -> u64 {
        let id = self.next_slab_id;
        self.next_slab_id += 1;
        self.slabs
            .insert(id, PoolSlab::new(id, object_size, capacity, tier));
        self.size_to_slab
            .entry(object_size)
            .or_insert_with(Vec::new)
            .push(id);
        self.update_stats();
        id
    }

    /// Allocate from best-fit slab
    pub fn allocate(&mut self, size: u32, pid: u64, now_ns: u64) -> Option<(u64, u64)> {
        // Find slab with matching size
        if let Some(slab_ids) = self.size_to_slab.get(&size) {
            for &slab_id in slab_ids {
                if let Some(slab) = self.slabs.get_mut(&slab_id) {
                    if let Some(obj_id) = slab.allocate(pid, now_ns) {
                        self.update_stats();
                        return Some((slab_id, obj_id));
                    }
                }
            }
        }
        // No free objects — try growing
        if let Some(slab_ids) = self.size_to_slab.get(&size) {
            if let Some(&slab_id) = slab_ids.first() {
                if let Some(slab) = self.slabs.get_mut(&slab_id) {
                    slab.grow(16);
                    if let Some(obj_id) = slab.allocate(pid, now_ns) {
                        self.update_stats();
                        return Some((slab_id, obj_id));
                    }
                }
            }
        }
        None
    }

    /// Release an object
    pub fn release(&mut self, slab_id: u64, object_id: u64) -> bool {
        if let Some(slab) = self.slabs.get_mut(&slab_id) {
            let result = slab.release(object_id);
            self.update_stats();
            result
        } else {
            false
        }
    }

    /// Adaptive sizing — grow hot slabs, shrink cold ones
    pub fn rebalance(&mut self, now_ns: u64) {
        let hot_ids: Vec<u64> = self
            .slabs
            .iter()
            .filter(|(_, s)| s.utilization() > 0.9)
            .map(|(&id, _)| id)
            .collect();
        for id in hot_ids {
            if let Some(slab) = self.slabs.get_mut(&id) {
                let grow_by = slab.capacity / 4;
                slab.grow(grow_by.max(4));
            }
        }

        let cold_ids: Vec<u64> = self
            .slabs
            .iter()
            .filter(|(_, s)| s.utilization() < 0.2 && s.capacity > 16)
            .map(|(&id, _)| id)
            .collect();
        for id in cold_ids {
            if let Some(slab) = self.slabs.get_mut(&id) {
                slab.shrink(4);
            }
        }
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.total_slabs = self.slabs.len();
        self.stats.total_objects = self.slabs.values().map(|s| s.capacity).sum();
        self.stats.total_allocated = self.slabs.values().map(|s| s.allocated_count).sum();
        self.stats.total_free = self.stats.total_objects - self.stats.total_allocated;
        if !self.slabs.is_empty() {
            self.stats.avg_utilization =
                self.slabs.values().map(|s| s.utilization()).sum::<f64>() / self.slabs.len() as f64;
        }
        self.stats.hot_slab_count = self
            .slabs
            .values()
            .filter(|s| s.utilization() > 0.8)
            .count();
    }

    pub fn stats(&self) -> &CoopResourcePoolV2Stats {
        &self.stats
    }
}
