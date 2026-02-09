// SPDX-License-Identifier: MIT
//! # Cooperative Memory Sync
//!
//! Multi-process memory synchronization:
//! - Dirty page tracking across shared regions
//! - Writeback scheduling coordination
//! - Consistency protocol (eventual, causal, sequential)
//! - Sync barrier management for process groups
//! - Conflict-free replicated data type support

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsistencyModel {
    Eventual,
    Causal,
    Sequential,
    Linearizable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncPhase {
    Idle,
    Collecting,
    Flushing,
    Verifying,
    Complete,
}

#[derive(Debug, Clone)]
pub struct SyncBarrier {
    pub barrier_id: u64,
    pub participants: Vec<u64>,
    pub arrived: Vec<u64>,
    pub phase: SyncPhase,
    pub created_at: u64,
    pub consistency: ConsistencyModel,
}

impl SyncBarrier {
    #[inline(always)]
    pub fn all_arrived(&self) -> bool {
        self.arrived.len() >= self.participants.len()
    }
    #[inline]
    pub fn progress(&self) -> f64 {
        if self.participants.is_empty() {
            return 1.0;
        }
        self.arrived.len() as f64 / self.participants.len() as f64
    }
}

#[derive(Debug, Clone)]
pub struct DirtyRegion {
    pub owner_pid: u64,
    pub base_addr: u64,
    pub dirty_pages: u64,
    pub total_pages: u64,
    pub last_flush: u64,
    pub flush_count: u64,
    pub writeback_bytes: u64,
}

impl DirtyRegion {
    #[inline]
    pub fn dirty_ratio(&self) -> f64 {
        if self.total_pages == 0 {
            return 0.0;
        }
        self.dirty_pages as f64 / self.total_pages as f64
    }
    #[inline(always)]
    pub fn needs_flush(&self, threshold: f64, max_age: u64, now: u64) -> bool {
        self.dirty_ratio() > threshold || now.saturating_sub(self.last_flush) > max_age
    }
}

#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct MsyncCoopStats {
    pub barriers_created: u64,
    pub barriers_completed: u64,
    pub dirty_flushes: u64,
    pub total_writeback_bytes: u64,
    pub stalled_barriers: u64,
    pub consistency_violations: u64,
}

pub struct MsyncCoopManager {
    barriers: BTreeMap<u64, SyncBarrier>,
    /// region_key → dirty tracking
    dirty_regions: BTreeMap<u64, DirtyRegion>,
    /// Vector clocks for causal consistency: pid → counter
    vector_clocks: BTreeMap<u64, BTreeMap<u64, u64>>,
    next_barrier: u64,
    dirty_threshold: f64,
    max_dirty_age: u64,
    stats: MsyncCoopStats,
}

impl MsyncCoopManager {
    pub fn new(dirty_threshold: f64, max_dirty_age: u64) -> Self {
        Self {
            barriers: BTreeMap::new(),
            dirty_regions: BTreeMap::new(),
            vector_clocks: BTreeMap::new(),
            next_barrier: 1,
            dirty_threshold,
            max_dirty_age,
            stats: MsyncCoopStats::default(),
        }
    }

    /// Create a sync barrier for a process group
    pub fn create_barrier(
        &mut self,
        participants: Vec<u64>,
        consistency: ConsistencyModel,
        now: u64,
    ) -> u64 {
        let id = self.next_barrier;
        self.next_barrier += 1;
        self.barriers.insert(id, SyncBarrier {
            barrier_id: id,
            participants,
            arrived: Vec::new(),
            phase: SyncPhase::Collecting,
            created_at: now,
            consistency,
        });
        self.stats.barriers_created += 1;
        id
    }

    /// Process arrives at barrier
    pub fn arrive_at_barrier(&mut self, barrier_id: u64, pid: u64) -> SyncPhase {
        let barrier = match self.barriers.get_mut(&barrier_id) {
            Some(b) => b,
            None => return SyncPhase::Idle,
        };
        if !barrier.arrived.contains(&pid) {
            barrier.arrived.push(pid);
        }
        if barrier.all_arrived() {
            barrier.phase = SyncPhase::Flushing;
        }
        barrier.phase
    }

    /// Advance barrier phase after flush
    #[inline]
    pub fn complete_barrier(&mut self, barrier_id: u64) {
        if let Some(barrier) = self.barriers.get_mut(&barrier_id) {
            barrier.phase = SyncPhase::Complete;
            self.stats.barriers_completed += 1;
        }
    }

    /// Track dirty pages in a shared region
    pub fn mark_dirty(&mut self, region_key: u64, pid: u64, pages: u64, total: u64) {
        let region = self.dirty_regions.entry(region_key).or_insert(DirtyRegion {
            owner_pid: pid,
            base_addr: region_key,
            dirty_pages: 0,
            total_pages: total,
            last_flush: 0,
            flush_count: 0,
            writeback_bytes: 0,
        });
        region.dirty_pages += pages;
    }

    /// Find regions that need flushing
    #[inline]
    pub fn regions_needing_flush(&self, now: u64) -> Vec<u64> {
        self.dirty_regions
            .iter()
            .filter(|(_, r)| r.needs_flush(self.dirty_threshold, self.max_dirty_age, now))
            .map(|(k, _)| *k)
            .collect()
    }

    /// Flush a dirty region
    pub fn flush_region(&mut self, region_key: u64, now: u64) -> u64 {
        if let Some(region) = self.dirty_regions.get_mut(&region_key) {
            let flushed = region.dirty_pages * 4096;
            region.writeback_bytes += flushed;
            region.dirty_pages = 0;
            region.last_flush = now;
            region.flush_count += 1;
            self.stats.dirty_flushes += 1;
            self.stats.total_writeback_bytes += flushed;
            flushed
        } else {
            0
        }
    }

    /// Update vector clock for causal consistency
    #[inline(always)]
    pub fn tick_clock(&mut self, pid: u64) {
        let clock = self.vector_clocks.entry(pid).or_insert_with(BTreeMap::new);
        *clock.entry(pid).or_insert(0) += 1;
    }

    /// Check causal ordering between two events
    pub fn causally_before(&self, pid_a: u64, pid_b: u64) -> bool {
        let clock_a = match self.vector_clocks.get(&pid_a) {
            Some(c) => c,
            None => return false,
        };
        let clock_b = match self.vector_clocks.get(&pid_b) {
            Some(c) => c,
            None => return false,
        };
        // a < b iff all a[i] <= b[i] and at least one a[i] < b[i]
        let mut all_leq = true;
        let mut some_lt = false;
        for (k, &va) in clock_a {
            let vb = clock_b.get(k).copied().unwrap_or(0);
            if va > vb {
                all_leq = false;
                break;
            }
            if va < vb {
                some_lt = true;
            }
        }
        all_leq && some_lt
    }

    #[inline(always)]
    pub fn barrier(&self, id: u64) -> Option<&SyncBarrier> {
        self.barriers.get(&id)
    }
    #[inline(always)]
    pub fn stats(&self) -> &MsyncCoopStats {
        &self.stats
    }
}
