// SPDX-License-Identifier: GPL-2.0
//! Apps brk manager — program break (heap) management and tracking.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Heap growth direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeapGrowth {
    /// Growing (brk increasing)
    Expanding,
    /// Stable (no recent changes)
    Stable,
    /// Shrinking (brk decreasing)
    Contracting,
}

/// Brk operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrkOp {
    /// Query current brk (brk(0))
    Query,
    /// Expand heap
    Expand,
    /// Contract heap
    Contract,
    /// Failed operation
    Failed,
}

/// A brk change record
#[derive(Debug, Clone)]
pub struct BrkChange {
    pub op: BrkOp,
    pub old_brk: u64,
    pub new_brk: u64,
    pub timestamp_ns: u64,
    pub pages_changed: u64,
}

impl BrkChange {
    pub fn size_delta(&self) -> i64 {
        self.new_brk as i64 - self.old_brk as i64
    }
}

/// Per-process brk state
#[derive(Debug)]
pub struct ProcessBrkState {
    pub pid: u64,
    pub initial_brk: u64,
    pub current_brk: u64,
    pub max_brk: u64,
    pub min_brk: u64,
    pub brk_limit: u64,
    pub growth: HeapGrowth,
    pub changes: Vec<BrkChange>,
    max_changes: usize,
    expand_count: u64,
    contract_count: u64,
    fail_count: u64,
    total_pages_allocated: u64,
    total_pages_freed: u64,
}

impl ProcessBrkState {
    pub fn new(pid: u64, initial_brk: u64, limit: u64) -> Self {
        Self {
            pid,
            initial_brk,
            current_brk: initial_brk,
            max_brk: initial_brk,
            min_brk: initial_brk,
            brk_limit: limit,
            growth: HeapGrowth::Stable,
            changes: Vec::new(),
            max_changes: 256,
            expand_count: 0,
            contract_count: 0,
            fail_count: 0,
            total_pages_allocated: 0,
            total_pages_freed: 0,
        }
    }

    pub fn heap_size(&self) -> u64 {
        self.current_brk.saturating_sub(self.initial_brk)
    }

    pub fn heap_pages(&self) -> u64 {
        (self.heap_size() + 4095) / 4096
    }

    pub fn watermark_ratio(&self) -> f64 {
        if self.max_brk == self.initial_brk { return 0.0; }
        let max_heap = self.max_brk - self.initial_brk;
        self.heap_size() as f64 / max_heap as f64
    }

    pub fn remaining_capacity(&self) -> u64 {
        self.brk_limit.saturating_sub(self.current_brk)
    }

    pub fn utilization(&self) -> f64 {
        if self.brk_limit == 0 { return 0.0; }
        self.current_brk as f64 / self.brk_limit as f64
    }

    fn record_change(&mut self, change: BrkChange) {
        if self.changes.len() >= self.max_changes {
            self.changes.remove(0);
        }
        self.changes.push(change);
    }

    pub fn apply_brk(&mut self, new_brk: u64, timestamp_ns: u64) -> BrkOp {
        if new_brk == 0 || new_brk == self.current_brk {
            let change = BrkChange {
                op: BrkOp::Query,
                old_brk: self.current_brk,
                new_brk: self.current_brk,
                timestamp_ns,
                pages_changed: 0,
            };
            self.record_change(change);
            return BrkOp::Query;
        }

        if new_brk > self.brk_limit {
            self.fail_count += 1;
            let change = BrkChange {
                op: BrkOp::Failed,
                old_brk: self.current_brk,
                new_brk: self.current_brk,
                timestamp_ns,
                pages_changed: 0,
            };
            self.record_change(change);
            return BrkOp::Failed;
        }

        if new_brk < self.initial_brk {
            self.fail_count += 1;
            return BrkOp::Failed;
        }

        let old_brk = self.current_brk;
        let op;
        let pages;

        if new_brk > self.current_brk {
            pages = (new_brk - self.current_brk + 4095) / 4096;
            self.total_pages_allocated += pages;
            self.expand_count += 1;
            self.growth = HeapGrowth::Expanding;
            op = BrkOp::Expand;
        } else {
            pages = (self.current_brk - new_brk + 4095) / 4096;
            self.total_pages_freed += pages;
            self.contract_count += 1;
            self.growth = HeapGrowth::Contracting;
            op = BrkOp::Contract;
        }

        self.current_brk = new_brk;
        if new_brk > self.max_brk {
            self.max_brk = new_brk;
        }
        if new_brk < self.min_brk {
            self.min_brk = new_brk;
        }

        self.record_change(BrkChange {
            op,
            old_brk,
            new_brk,
            timestamp_ns,
            pages_changed: pages,
        });

        op
    }

    pub fn avg_expand_size(&self) -> u64 {
        let expand_changes: Vec<&BrkChange> = self.changes.iter()
            .filter(|c| c.op == BrkOp::Expand)
            .collect();
        if expand_changes.is_empty() { return 0; }
        let total: u64 = expand_changes.iter().map(|c| c.pages_changed).sum();
        total / expand_changes.len() as u64
    }

    /// Predict next brk size based on recent growth pattern
    pub fn predict_next_size(&self) -> u64 {
        let recent_expands: Vec<u64> = self.changes.iter()
            .rev()
            .filter(|c| c.op == BrkOp::Expand)
            .take(8)
            .map(|c| c.pages_changed * 4096)
            .collect();
        if recent_expands.is_empty() { return 4096; }
        let avg = recent_expands.iter().sum::<u64>() / recent_expands.len() as u64;
        // Return 1.5x the average
        avg + avg / 2
    }
}

/// Brk manager stats
#[derive(Debug, Clone)]
pub struct BrkMgrStats {
    pub processes_tracked: u64,
    pub total_brk_calls: u64,
    pub total_expands: u64,
    pub total_contracts: u64,
    pub total_failures: u64,
    pub total_pages_allocated: u64,
    pub total_pages_freed: u64,
}

/// Main apps brk manager
pub struct AppBrkMgr {
    processes: BTreeMap<u64, ProcessBrkState>,
    default_limit: u64,
    stats: BrkMgrStats,
}

impl AppBrkMgr {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            default_limit: 128 * 1024 * 1024, // 128 MiB default limit
            stats: BrkMgrStats {
                processes_tracked: 0,
                total_brk_calls: 0,
                total_expands: 0,
                total_contracts: 0,
                total_failures: 0,
                total_pages_allocated: 0,
                total_pages_freed: 0,
            },
        }
    }

    pub fn register_process(&mut self, pid: u64, initial_brk: u64, limit: Option<u64>) {
        let lim = limit.unwrap_or(initial_brk.saturating_add(self.default_limit));
        self.processes.insert(pid, ProcessBrkState::new(pid, initial_brk, lim));
        self.stats.processes_tracked += 1;
    }

    pub fn unregister_process(&mut self, pid: u64) -> Option<(u64, u64)> {
        self.processes.remove(&pid).map(|p| (p.heap_size(), p.total_pages_allocated))
    }

    pub fn sys_brk(&mut self, pid: u64, new_brk: u64, timestamp_ns: u64) -> Option<u64> {
        let proc_state = self.processes.get_mut(&pid)?;
        let op = proc_state.apply_brk(new_brk, timestamp_ns);
        self.stats.total_brk_calls += 1;
        match op {
            BrkOp::Expand => {
                self.stats.total_expands += 1;
                self.stats.total_pages_allocated += proc_state.changes.last().map_or(0, |c| c.pages_changed);
            }
            BrkOp::Contract => {
                self.stats.total_contracts += 1;
                self.stats.total_pages_freed += proc_state.changes.last().map_or(0, |c| c.pages_changed);
            }
            BrkOp::Failed => {
                self.stats.total_failures += 1;
            }
            BrkOp::Query => {}
        }
        Some(proc_state.current_brk)
    }

    pub fn process_heap_size(&self, pid: u64) -> Option<u64> {
        self.processes.get(&pid).map(|p| p.heap_size())
    }

    pub fn largest_heaps(&self, top_n: usize) -> Vec<(u64, u64)> {
        let mut heaps: Vec<(u64, u64)> = self.processes.iter()
            .map(|(pid, p)| (*pid, p.heap_size()))
            .collect();
        heaps.sort_by(|a, b| b.1.cmp(&a.1));
        heaps.truncate(top_n);
        heaps
    }

    pub fn total_heap_bytes(&self) -> u64 {
        self.processes.values().map(|p| p.heap_size()).sum()
    }

    pub fn set_default_limit(&mut self, limit: u64) {
        self.default_limit = limit;
    }

    pub fn stats(&self) -> &BrkMgrStats {
        &self.stats
    }
}
