// SPDX-License-Identifier: MIT
//! # Cooperative Swap Management
//!
//! Multi-process swap coordination:
//! - Swap slot allocation fairness across process groups
//! - Cooperative zswap pool sharing
//! - Swap priority negotiation
//! - Group-level swap budget management
//! - Pre-swap notification protocol

extern crate alloc;
use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapPriority {
    Critical,
    High,
    Normal,
    Low,
    Idle,
}

impl SwapPriority {
    #[inline]
    pub fn weight(&self) -> u32 {
        match self {
            Self::Critical => 0, // never swap
            Self::High => 1,
            Self::Normal => 4,
            Self::Low => 8,
            Self::Idle => 16,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SwapBudget {
    pub group_id: u64,
    pub allocated_slots: u64,
    pub used_slots: u64,
    pub max_slots: u64,
    pub members: Vec<(u64, SwapPriority)>,
}

impl SwapBudget {
    #[inline(always)]
    pub fn remaining(&self) -> u64 {
        self.allocated_slots.saturating_sub(self.used_slots)
    }
    #[inline]
    pub fn utilization(&self) -> f64 {
        if self.allocated_slots == 0 {
            return 0.0;
        }
        self.used_slots as f64 / self.allocated_slots as f64
    }
}

#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ZswapPool {
    pub pool_id: u64,
    pub capacity_bytes: u64,
    pub used_bytes: u64,
    pub compressed_bytes: u64,
    pub original_bytes: u64,
    pub participants: Vec<u64>,
}

impl ZswapPool {
    #[inline]
    pub fn compression_ratio(&self) -> f64 {
        if self.compressed_bytes == 0 {
            return 1.0;
        }
        self.original_bytes as f64 / self.compressed_bytes as f64
    }
    #[inline(always)]
    pub fn free_bytes(&self) -> u64 {
        self.capacity_bytes.saturating_sub(self.used_bytes)
    }
}

#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct SwapCoopStats {
    pub slots_allocated: u64,
    pub slots_freed: u64,
    pub budget_exceeded: u64,
    pub priority_negotiations: u64,
    pub zswap_saves: u64,
    pub pre_swap_notifications: u64,
}

pub struct SwapCoopManager {
    budgets: BTreeMap<u64, SwapBudget>,
    pools: BTreeMap<u64, ZswapPool>,
    /// pid → group_id
    pid_groups: LinearMap<u64, 64>,
    total_swap_slots: u64,
    next_pool: u64,
    stats: SwapCoopStats,
}

impl SwapCoopManager {
    pub fn new(total_swap_slots: u64) -> Self {
        Self {
            budgets: BTreeMap::new(),
            pools: BTreeMap::new(),
            pid_groups: LinearMap::new(),
            total_swap_slots,
            next_pool: 1,
            stats: SwapCoopStats::default(),
        }
    }

    /// Allocate swap budget for a group based on member priorities
    pub fn allocate_budget(&mut self, group_id: u64, members: Vec<(u64, SwapPriority)>) {
        let total_weight: u32 = members.iter().map(|(_, p)| p.weight()).sum();
        let fair_share = if total_weight > 0 {
            self.total_swap_slots / (total_weight as u64 + 1)
        } else {
            self.total_swap_slots / 10
        };

        for &(pid, _) in &members {
            self.pid_groups.insert(pid, group_id);
        }
        self.budgets.insert(group_id, SwapBudget {
            group_id,
            allocated_slots: fair_share,
            used_slots: 0,
            max_slots: fair_share * 2,
            members,
        });
    }

    /// Request swap slots for a process
    pub fn request_slots(&mut self, pid: u64, count: u64) -> u64 {
        let group_id = match self.pid_groups.get(pid) {
            Some(g) => *g,
            None => return 0,
        };
        let budget = match self.budgets.get_mut(&group_id) {
            Some(b) => b,
            None => return 0,
        };

        let available = budget.remaining().min(count);
        if available < count {
            self.stats.budget_exceeded += 1;
        }
        budget.used_slots += available;
        self.stats.slots_allocated += available;
        available
    }

    /// Release swap slots
    #[inline]
    pub fn release_slots(&mut self, pid: u64, count: u64) {
        let group_id = match self.pid_groups.get(pid) {
            Some(g) => *g,
            None => return,
        };
        if let Some(budget) = self.budgets.get_mut(&group_id) {
            budget.used_slots = budget.used_slots.saturating_sub(count);
            self.stats.slots_freed += count;
        }
    }

    /// Create a shared zswap pool
    pub fn create_zswap_pool(&mut self, capacity: u64, participants: Vec<u64>) -> u64 {
        let id = self.next_pool;
        self.next_pool += 1;
        self.pools.insert(id, ZswapPool {
            pool_id: id,
            capacity_bytes: capacity,
            used_bytes: 0,
            compressed_bytes: 0,
            original_bytes: 0,
            participants,
        });
        id
    }

    /// Store compressed data in zswap pool
    pub fn zswap_store(&mut self, pool_id: u64, original: u64, compressed: u64) -> bool {
        let pool = match self.pools.get_mut(&pool_id) {
            Some(p) => p,
            None => return false,
        };
        if pool.free_bytes() < compressed {
            return false;
        }
        pool.used_bytes += compressed;
        pool.compressed_bytes += compressed;
        pool.original_bytes += original;
        self.stats.zswap_saves += 1;
        true
    }

    /// Rebalance budgets based on current usage patterns
    pub fn rebalance(&mut self) {
        let total_used: u64 = self.budgets.values().map(|b| b.used_slots).sum();
        let total_allocated: u64 = self.budgets.values().map(|b| b.allocated_slots).sum();

        if total_allocated == 0 {
            return;
        }
        let usage_ratio = total_used as f64 / total_allocated as f64;

        for budget in self.budgets.values_mut() {
            let group_usage = budget.utilization();
            if group_usage < usage_ratio * 0.5 && budget.allocated_slots > 1 {
                // Under-using: shrink allocation
                budget.allocated_slots = (budget.allocated_slots * 3) / 4;
            } else if group_usage > 0.9 && budget.allocated_slots < budget.max_slots {
                // Heavy use: grow allocation
                budget.allocated_slots = (budget.allocated_slots * 5) / 4;
            }
        }
    }

    #[inline(always)]
    pub fn budget(&self, group_id: u64) -> Option<&SwapBudget> {
        self.budgets.get(&group_id)
    }
    #[inline(always)]
    pub fn stats(&self) -> &SwapCoopStats {
        &self.stats
    }
}
