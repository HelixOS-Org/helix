// SPDX-License-Identifier: GPL-2.0
//! Coop coop_alloc — cooperative memory allocator with per-subsystem budgeting.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Allocation class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocClass {
    /// Small: <= 256 bytes
    Small,
    /// Medium: 256 bytes - 4KB
    Medium,
    /// Large: 4KB - 2MB
    Large,
    /// Huge: > 2MB
    Huge,
}

impl AllocClass {
    pub fn from_size(size: usize) -> Self {
        if size <= 256 { Self::Small }
        else if size <= 4096 { Self::Medium }
        else if size <= 2 * 1024 * 1024 { Self::Large }
        else { Self::Huge }
    }
}

/// Memory pool pressure level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolPressure {
    None,
    Low,
    Medium,
    High,
    Critical,
}

/// An allocation record
#[derive(Debug, Clone)]
pub struct CoopAllocRecord {
    pub id: u64,
    pub owner: u64,
    pub pool_id: u64,
    pub size: usize,
    pub class: AllocClass,
    pub alloc_ns: u64,
    pub alignment: usize,
    pub zeroed: bool,
}

/// A memory pool
#[derive(Debug)]
pub struct CoopPool {
    pub id: u64,
    pub name: String,
    pub total_capacity: usize,
    pub used_bytes: usize,
    pub peak_used: usize,
    pub alloc_count: u64,
    pub free_count: u64,
    pub alloc_fail_count: u64,
    pub fragmentation_ratio: f64,
    per_class: [ClassBucket; 4],
}

/// Per-class allocation bucket
#[derive(Debug, Clone, Copy)]
pub struct ClassBucket {
    pub active_count: u64,
    pub active_bytes: u64,
    pub total_allocs: u64,
    pub total_frees: u64,
}

impl ClassBucket {
    pub fn new() -> Self {
        Self { active_count: 0, active_bytes: 0, total_allocs: 0, total_frees: 0 }
    }

    pub fn alloc(&mut self, size: usize) {
        self.active_count += 1;
        self.active_bytes += size as u64;
        self.total_allocs += 1;
    }

    pub fn free(&mut self, size: usize) {
        self.active_count = self.active_count.saturating_sub(1);
        self.active_bytes = self.active_bytes.saturating_sub(size as u64);
        self.total_frees += 1;
    }
}

impl CoopPool {
    pub fn new(id: u64, name: String, capacity: usize) -> Self {
        Self {
            id,
            name,
            total_capacity: capacity,
            used_bytes: 0,
            peak_used: 0,
            alloc_count: 0,
            free_count: 0,
            alloc_fail_count: 0,
            fragmentation_ratio: 0.0,
            per_class: [ClassBucket::new(); 4],
        }
    }

    fn class_idx(class: AllocClass) -> usize {
        match class {
            AllocClass::Small => 0,
            AllocClass::Medium => 1,
            AllocClass::Large => 2,
            AllocClass::Huge => 3,
        }
    }

    pub fn allocate(&mut self, size: usize) -> bool {
        if self.used_bytes + size > self.total_capacity {
            self.alloc_fail_count += 1;
            return false;
        }
        self.used_bytes += size;
        if self.used_bytes > self.peak_used {
            self.peak_used = self.used_bytes;
        }
        self.alloc_count += 1;
        let class = AllocClass::from_size(size);
        self.per_class[Self::class_idx(class)].alloc(size);
        true
    }

    pub fn free(&mut self, size: usize) {
        self.used_bytes = self.used_bytes.saturating_sub(size);
        self.free_count += 1;
        let class = AllocClass::from_size(size);
        self.per_class[Self::class_idx(class)].free(size);
    }

    pub fn utilization(&self) -> f64 {
        if self.total_capacity == 0 { return 0.0; }
        self.used_bytes as f64 / self.total_capacity as f64
    }

    pub fn pressure(&self) -> PoolPressure {
        let util = self.utilization();
        if util > 0.95 { PoolPressure::Critical }
        else if util > 0.85 { PoolPressure::High }
        else if util > 0.70 { PoolPressure::Medium }
        else if util > 0.50 { PoolPressure::Low }
        else { PoolPressure::None }
    }

    pub fn remaining(&self) -> usize {
        self.total_capacity.saturating_sub(self.used_bytes)
    }

    pub fn fail_rate(&self) -> f64 {
        let total = self.alloc_count + self.alloc_fail_count;
        if total == 0 { return 0.0; }
        self.alloc_fail_count as f64 / total as f64
    }
}

/// Per-subsystem budget
#[derive(Debug)]
pub struct SubsystemBudget {
    pub name: String,
    pub budget_bytes: usize,
    pub used_bytes: usize,
    pub alloc_count: u64,
    pub denied_count: u64,
    pub pool_id: u64,
}

impl SubsystemBudget {
    pub fn new(name: String, budget: usize, pool_id: u64) -> Self {
        Self {
            name,
            budget_bytes: budget,
            used_bytes: 0,
            alloc_count: 0,
            denied_count: 0,
            pool_id,
        }
    }

    pub fn can_allocate(&self, size: usize) -> bool {
        self.used_bytes + size <= self.budget_bytes
    }

    pub fn allocate(&mut self, size: usize) -> bool {
        if !self.can_allocate(size) {
            self.denied_count += 1;
            return false;
        }
        self.used_bytes += size;
        self.alloc_count += 1;
        true
    }

    pub fn free(&mut self, size: usize) {
        self.used_bytes = self.used_bytes.saturating_sub(size);
    }

    pub fn utilization(&self) -> f64 {
        if self.budget_bytes == 0 { return 0.0; }
        self.used_bytes as f64 / self.budget_bytes as f64
    }
}

/// Allocator stats
#[derive(Debug, Clone)]
pub struct CoopAllocStats {
    pub total_pools: u64,
    pub total_budgets: u64,
    pub total_allocs: u64,
    pub total_frees: u64,
    pub total_bytes_allocated: u64,
    pub total_failures: u64,
    pub total_bytes_active: u64,
}

/// Main cooperative allocator
pub struct CoopAlloc {
    pools: BTreeMap<u64, CoopPool>,
    budgets: BTreeMap<String, SubsystemBudget>,
    alloc_records: BTreeMap<u64, CoopAllocRecord>,
    next_pool_id: u64,
    next_alloc_id: u64,
    stats: CoopAllocStats,
}

impl CoopAlloc {
    pub fn new() -> Self {
        Self {
            pools: BTreeMap::new(),
            budgets: BTreeMap::new(),
            alloc_records: BTreeMap::new(),
            next_pool_id: 1,
            next_alloc_id: 1,
            stats: CoopAllocStats {
                total_pools: 0,
                total_budgets: 0,
                total_allocs: 0,
                total_frees: 0,
                total_bytes_allocated: 0,
                total_failures: 0,
                total_bytes_active: 0,
            },
        }
    }

    pub fn create_pool(&mut self, name: String, capacity: usize) -> u64 {
        let id = self.next_pool_id;
        self.next_pool_id += 1;
        self.pools.insert(id, CoopPool::new(id, name, capacity));
        self.stats.total_pools += 1;
        id
    }

    pub fn create_budget(&mut self, subsystem: String, budget: usize, pool_id: u64) {
        self.budgets.insert(subsystem.clone(), SubsystemBudget::new(subsystem, budget, pool_id));
        self.stats.total_budgets += 1;
    }

    pub fn allocate(&mut self, subsystem: &str, owner: u64, size: usize, now_ns: u64) -> Option<u64> {
        // Check budget first
        let pool_id = if let Some(budget) = self.budgets.get_mut(subsystem) {
            if !budget.allocate(size) {
                self.stats.total_failures += 1;
                return None;
            }
            budget.pool_id
        } else {
            return None;
        };

        // Allocate from pool
        if let Some(pool) = self.pools.get_mut(&pool_id) {
            if !pool.allocate(size) {
                // Rollback budget
                if let Some(budget) = self.budgets.get_mut(subsystem) {
                    budget.free(size);
                }
                self.stats.total_failures += 1;
                return None;
            }
        } else {
            return None;
        }

        let alloc_id = self.next_alloc_id;
        self.next_alloc_id += 1;
        self.alloc_records.insert(alloc_id, CoopAllocRecord {
            id: alloc_id,
            owner,
            pool_id,
            size,
            class: AllocClass::from_size(size),
            alloc_ns: now_ns,
            alignment: 8,
            zeroed: false,
        });
        self.stats.total_allocs += 1;
        self.stats.total_bytes_allocated += size as u64;
        self.stats.total_bytes_active += size as u64;
        Some(alloc_id)
    }

    pub fn free(&mut self, alloc_id: u64) -> bool {
        if let Some(rec) = self.alloc_records.remove(&alloc_id) {
            if let Some(pool) = self.pools.get_mut(&rec.pool_id) {
                pool.free(rec.size);
            }
            // Find the budget for this pool
            for budget in self.budgets.values_mut() {
                if budget.pool_id == rec.pool_id {
                    budget.free(rec.size);
                    break;
                }
            }
            self.stats.total_frees += 1;
            self.stats.total_bytes_active = self.stats.total_bytes_active.saturating_sub(rec.size as u64);
            true
        } else {
            false
        }
    }

    pub fn pool_pressure(&self) -> Vec<(u64, PoolPressure)> {
        self.pools.iter().map(|(&id, p)| (id, p.pressure())).collect()
    }

    pub fn critical_pools(&self) -> Vec<u64> {
        self.pools.iter()
            .filter(|(_, p)| matches!(p.pressure(), PoolPressure::Critical | PoolPressure::High))
            .map(|(&id, _)| id)
            .collect()
    }

    pub fn budget_overcommit(&self) -> Vec<(&str, f64)> {
        self.budgets.iter()
            .filter(|(_, b)| b.utilization() > 0.9)
            .map(|(name, b)| (name.as_str(), b.utilization()))
            .collect()
    }

    pub fn get_pool(&self, id: u64) -> Option<&CoopPool> {
        self.pools.get(&id)
    }

    pub fn stats(&self) -> &CoopAllocStats {
        &self.stats
    }
}
