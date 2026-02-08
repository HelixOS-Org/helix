// SPDX-License-Identifier: MIT
//! # Cooperative Huge Page Management
//!
//! Multi-process huge page sharing and coordination:
//! - Huge page pool arbitration between competing processes
//! - THP promotion coordination to avoid fragmentation
//! - Shared huge page reference counting
//! - NUMA-aware huge page distribution
//! - Cooperative compaction scheduling

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HugePageSize {
    TwoMB,
    OneGB,
}

impl HugePageSize {
    pub fn bytes(&self) -> u64 {
        match self {
            Self::TwoMB => 2 * 1024 * 1024,
            Self::OneGB => 1024 * 1024 * 1024,
        }
    }
    pub fn base_pages(&self) -> u64 {
        self.bytes() / 4096
    }
}

#[derive(Debug, Clone)]
pub struct HugePagePool {
    pub total_2mb: u64,
    pub free_2mb: u64,
    pub total_1gb: u64,
    pub free_1gb: u64,
    pub reservations: BTreeMap<u64, u64>, // pid → reserved count
}

impl HugePagePool {
    pub fn utilization_2mb(&self) -> f64 {
        if self.total_2mb == 0 {
            return 0.0;
        }
        1.0 - (self.free_2mb as f64 / self.total_2mb as f64)
    }
    pub fn pressure(&self) -> f64 {
        let reserved: u64 = self.reservations.values().sum();
        if self.free_2mb == 0 {
            return 1.0;
        }
        (reserved as f64 / self.free_2mb as f64).min(1.0)
    }
}

#[derive(Debug, Clone)]
pub struct CompactionRequest {
    pub pid: u64,
    pub target_pages: u64,
    pub priority: u32,
    pub requested_at: u64,
    pub deadline_ns: u64,
}

#[derive(Debug, Clone, Default)]
pub struct HugePageCoopStats {
    pub promotions_coordinated: u64,
    pub demotions_forced: u64,
    pub compaction_runs: u64,
    pub reservation_denials: u64,
    pub numa_rebalances: u64,
    pub shared_huge_pages: u64,
}

pub struct HugePageCoopManager {
    pools: BTreeMap<u32, HugePagePool>, // numa_node → pool
    compaction_queue: Vec<CompactionRequest>,
    /// pid → (huge_page_addr, refcount)
    shared_refs: BTreeMap<u64, Vec<(u64, u32)>>,
    stats: HugePageCoopStats,
}

impl HugePageCoopManager {
    pub fn new() -> Self {
        Self {
            pools: BTreeMap::new(),
            compaction_queue: Vec::new(),
            shared_refs: BTreeMap::new(),
            stats: HugePageCoopStats::default(),
        }
    }

    pub fn init_pool(&mut self, numa_node: u32, total_2mb: u64, total_1gb: u64) {
        self.pools.insert(numa_node, HugePagePool {
            total_2mb,
            free_2mb: total_2mb,
            total_1gb,
            free_1gb: total_1gb,
            reservations: BTreeMap::new(),
        });
    }

    /// Try to reserve huge pages for a process
    pub fn reserve(&mut self, pid: u64, count: u64, numa_node: u32) -> bool {
        let pool = match self.pools.get_mut(&numa_node) {
            Some(p) => p,
            None => return false,
        };
        if pool.free_2mb < count {
            self.stats.reservation_denials += 1;
            return false;
        }
        pool.free_2mb -= count;
        *pool.reservations.entry(pid).or_insert(0) += count;
        true
    }

    /// Release reserved huge pages
    pub fn release(&mut self, pid: u64, count: u64, numa_node: u32) {
        if let Some(pool) = self.pools.get_mut(&numa_node) {
            pool.free_2mb += count;
            if let Some(r) = pool.reservations.get_mut(&pid) {
                *r = r.saturating_sub(count);
                if *r == 0 {
                    pool.reservations.remove(&pid);
                }
            }
        }
    }

    /// Coordinate THP promotion: check if promotion won't starve others
    pub fn can_promote(&self, pid: u64, numa_node: u32) -> bool {
        let pool = match self.pools.get(&numa_node) {
            Some(p) => p,
            None => return false,
        };
        // Allow promotion if pool isn't under pressure
        pool.pressure() < 0.8
    }

    /// Request compaction to free contiguous huge page frames
    pub fn request_compaction(&mut self, pid: u64, target: u64, priority: u32, now: u64) {
        self.compaction_queue.push(CompactionRequest {
            pid,
            target_pages: target,
            priority,
            requested_at: now,
            deadline_ns: now + 100_000_000, // 100ms deadline
        });
        self.compaction_queue
            .sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Process compaction queue
    pub fn run_compaction_round(&mut self, available_pages: u64) -> u64 {
        let mut compacted = 0u64;
        self.compaction_queue.retain(|req| {
            if compacted + req.target_pages <= available_pages {
                compacted += req.target_pages;
                false // remove fulfilled request
            } else {
                true
            }
        });
        self.stats.compaction_runs += 1;
        compacted
    }

    /// Rebalance huge pages across NUMA nodes
    pub fn rebalance_numa(&mut self) -> Vec<(u32, u32, u64)> {
        let mut moves = Vec::new();
        let pressures: Vec<(u32, f64)> =
            self.pools.iter().map(|(n, p)| (*n, p.pressure())).collect();

        for i in 0..pressures.len() {
            for j in (i + 1)..pressures.len() {
                let diff = (pressures[i].1 - pressures[j].1).abs();
                if diff > 0.3 {
                    let (from, to) = if pressures[i].1 < pressures[j].1 {
                        (pressures[i].0, pressures[j].0)
                    } else {
                        (pressures[j].0, pressures[i].0)
                    };
                    let amount = 4; // migrate 4 huge pages at a time
                    moves.push((from, to, amount));
                    self.stats.numa_rebalances += 1;
                }
            }
        }
        moves
    }

    pub fn pool(&self, numa_node: u32) -> Option<&HugePagePool> {
        self.pools.get(&numa_node)
    }
    pub fn stats(&self) -> &HugePageCoopStats {
        &self.stats
    }
}
