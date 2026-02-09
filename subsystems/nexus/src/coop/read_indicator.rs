// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop — Read Indicator (distributed read-side indication for scalable locking)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

/// Read indicator state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadIndicatorState {
    Inactive,
    Reading,
    Quiescing,
}

/// Per-CPU read indicator slot
#[derive(Debug)]
pub struct ReadIndicatorSlot {
    pub cpu_id: u32,
    pub counter: AtomicU64,
    pub active_readers: u64,
    pub total_reads: u64,
}

impl ReadIndicatorSlot {
    pub fn new(cpu_id: u32) -> Self {
        Self {
            cpu_id,
            counter: AtomicU64::new(0),
            active_readers: 0,
            total_reads: 0,
        }
    }

    #[inline(always)]
    pub fn enter(&self) {
        self.counter.fetch_add(1, Ordering::Release);
    }

    #[inline(always)]
    pub fn exit(&self) {
        self.counter.fetch_sub(1, Ordering::Release);
    }

    #[inline(always)]
    pub fn is_active(&self) -> bool {
        self.counter.load(Ordering::Acquire) > 0
    }
}

/// A read indicator instance
#[derive(Debug)]
pub struct ReadIndicatorInstance {
    pub id: u64,
    pub slots: Vec<ReadIndicatorSlot>,
    pub quiesce_count: u64,
    pub total_enters: u64,
    pub total_exits: u64,
}

impl ReadIndicatorInstance {
    pub fn new(id: u64, num_cpus: u32) -> Self {
        let mut slots = Vec::new();
        for i in 0..num_cpus {
            slots.push(ReadIndicatorSlot::new(i));
        }
        Self {
            id, slots,
            quiesce_count: 0,
            total_enters: 0, total_exits: 0,
        }
    }

    #[inline]
    pub fn reader_enter(&mut self, cpu: u32) {
        if (cpu as usize) < self.slots.len() {
            self.slots[cpu as usize].enter();
            self.total_enters += 1;
        }
    }

    #[inline]
    pub fn reader_exit(&mut self, cpu: u32) {
        if (cpu as usize) < self.slots.len() {
            self.slots[cpu as usize].exit();
            self.total_exits += 1;
        }
    }

    #[inline(always)]
    pub fn is_quiescent(&self) -> bool {
        self.slots.iter().all(|s| !s.is_active())
    }

    #[inline(always)]
    pub fn wait_for_quiescence(&mut self) -> bool {
        self.quiesce_count += 1;
        self.is_quiescent()
    }
}

/// Statistics for read indicator
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ReadIndicatorStats {
    pub indicators_created: u64,
    pub total_enters: u64,
    pub total_exits: u64,
    pub quiesce_operations: u64,
    pub quiesce_successes: u64,
}

/// Main read indicator coop manager
#[derive(Debug)]
pub struct CoopReadIndicator {
    indicators: BTreeMap<u64, ReadIndicatorInstance>,
    next_id: u64,
    stats: ReadIndicatorStats,
}

impl CoopReadIndicator {
    pub fn new() -> Self {
        Self {
            indicators: BTreeMap::new(),
            next_id: 1,
            stats: ReadIndicatorStats {
                indicators_created: 0, total_enters: 0,
                total_exits: 0, quiesce_operations: 0,
                quiesce_successes: 0,
            },
        }
    }

    #[inline]
    pub fn create(&mut self, num_cpus: u32) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.indicators.insert(id, ReadIndicatorInstance::new(id, num_cpus));
        self.stats.indicators_created += 1;
        id
    }

    #[inline]
    pub fn reader_enter(&mut self, ind_id: u64, cpu: u32) {
        if let Some(ind) = self.indicators.get_mut(&ind_id) {
            ind.reader_enter(cpu);
            self.stats.total_enters += 1;
        }
    }

    #[inline]
    pub fn reader_exit(&mut self, ind_id: u64, cpu: u32) {
        if let Some(ind) = self.indicators.get_mut(&ind_id) {
            ind.reader_exit(cpu);
            self.stats.total_exits += 1;
        }
    }

    #[inline]
    pub fn wait_quiescence(&mut self, ind_id: u64) -> bool {
        self.stats.quiesce_operations += 1;
        if let Some(ind) = self.indicators.get_mut(&ind_id) {
            if ind.wait_for_quiescence() {
                self.stats.quiesce_successes += 1;
                return true;
            }
        }
        false
    }

    #[inline(always)]
    pub fn stats(&self) -> &ReadIndicatorStats {
        &self.stats
    }
}
