// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop — Fork (cooperative process forking)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Cooperative fork strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopForkStrategy {
    Eager,
    Lazy,
    Deferred,
    Batched,
    Speculative,
}

/// Fork cooperation record
#[derive(Debug, Clone)]
pub struct CoopForkRecord {
    pub parent: u64,
    pub child: u64,
    pub strategy: CoopForkStrategy,
    pub shared_pages: u64,
    pub cow_faults: u64,
    pub latency_us: u64,
}

/// Fork cooperation stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CoopForkStats {
    pub total_forks: u64,
    pub eager_forks: u64,
    pub lazy_forks: u64,
    pub batched_forks: u64,
    pub avg_shared_pages: u64,
    pub total_cow_faults: u64,
}

/// Manager for cooperative fork operations
pub struct CoopForkManager {
    records: Vec<CoopForkRecord>,
    parent_map: BTreeMap<u64, Vec<u64>>,
    strategy_override: BTreeMap<u64, CoopForkStrategy>,
    stats: CoopForkStats,
}

impl CoopForkManager {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            parent_map: BTreeMap::new(),
            strategy_override: BTreeMap::new(),
            stats: CoopForkStats {
                total_forks: 0,
                eager_forks: 0,
                lazy_forks: 0,
                batched_forks: 0,
                avg_shared_pages: 0,
                total_cow_faults: 0,
            },
        }
    }

    #[inline(always)]
    pub fn set_strategy(&mut self, pid: u64, strategy: CoopForkStrategy) {
        self.strategy_override.insert(pid, strategy);
    }

    pub fn cooperative_fork(&mut self, parent: u64, child: u64, pages: u64) {
        let strategy = self.strategy_override.get(&parent).cloned().unwrap_or(CoopForkStrategy::Lazy);
        let record = CoopForkRecord {
            parent,
            child,
            strategy,
            shared_pages: pages,
            cow_faults: 0,
            latency_us: match strategy {
                CoopForkStrategy::Eager => 200,
                CoopForkStrategy::Lazy => 50,
                CoopForkStrategy::Batched => 30,
                _ => 100,
            },
        };
        self.records.push(record);
        self.parent_map.entry(parent).or_insert_with(Vec::new).push(child);
        self.stats.total_forks += 1;
        match strategy {
            CoopForkStrategy::Eager => self.stats.eager_forks += 1,
            CoopForkStrategy::Lazy => self.stats.lazy_forks += 1,
            CoopForkStrategy::Batched => self.stats.batched_forks += 1,
            _ => {}
        }
    }

    #[inline]
    pub fn report_cow_fault(&mut self, child: u64) {
        for r in self.records.iter_mut().rev() {
            if r.child == child {
                r.cow_faults += 1;
                self.stats.total_cow_faults += 1;
                break;
            }
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &CoopForkStats {
        &self.stats
    }
}
