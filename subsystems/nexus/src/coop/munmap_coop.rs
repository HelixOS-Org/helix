// SPDX-License-Identifier: MIT
//! # Cooperative Memory Unmap
//!
//! Multi-process unmap coordination:
//! - Coordinated teardown of shared regions
//! - Reference counting for shared mappings
//! - Orphan page detection after process exit
//! - TLB shootdown batching across cores
//! - Deferred unmap for performance-critical paths

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnmapStrategy { Immediate, Deferred, Batched, Lazy }

#[derive(Debug, Clone)]
pub struct SharedUnmapRef {
    pub mapping_id: u64,
    pub base_addr: u64,
    pub size: u64,
    pub refcount: u32,
    pub pids: Vec<u64>,
    pub deferred_until: u64,
}

impl SharedUnmapRef {
    pub fn can_reclaim(&self) -> bool { self.refcount == 0 }
    pub fn page_count(&self) -> u64 { self.size / 4096 }
}

#[derive(Debug, Clone)]
pub struct TlbShootdownBatch {
    pub batch_id: u64,
    pub target_cpus: Vec<u32>,
    pub pages: Vec<u64>,
    pub initiated_at: u64,
    pub completed: bool,
}

impl TlbShootdownBatch {
    pub fn cost_estimate(&self) -> u64 {
        // Each IPI costs ~1μs, each page invalidation ~100ns
        self.target_cpus.len() as u64 * 1000 + self.pages.len() as u64 * 100
    }
}

#[derive(Debug, Clone, Default)]
pub struct MunmapCoopStats {
    pub coordinated_unmaps: u64,
    pub deferred_unmaps: u64,
    pub orphan_pages_reclaimed: u64,
    pub tlb_shootdowns: u64,
    pub tlb_pages_invalidated: u64,
    pub refcount_drops: u64,
}

pub struct MunmapCoopManager {
    shared_refs: BTreeMap<u64, SharedUnmapRef>,
    /// Deferred unmap queue
    deferred: Vec<(u64, u64, u64)>, // (mapping_id, pid, defer_until)
    /// Pending TLB shootdown batches
    shootdown_queue: Vec<TlbShootdownBatch>,
    next_batch: u64,
    stats: MunmapCoopStats,
}

impl MunmapCoopManager {
    pub fn new() -> Self {
        Self {
            shared_refs: BTreeMap::new(),
            deferred: Vec::new(),
            shootdown_queue: Vec::new(),
            next_batch: 1,
            stats: MunmapCoopStats::default(),
        }
    }

    /// Register a shared mapping for coordinated unmap
    pub fn register_shared(&mut self, mapping_id: u64, base: u64, size: u64, pids: Vec<u64>) {
        let refcount = pids.len() as u32;
        self.shared_refs.insert(mapping_id, SharedUnmapRef {
            mapping_id, base_addr: base, size,
            refcount, pids, deferred_until: 0,
        });
    }

    /// One process requests unmap of a shared region
    pub fn request_unmap(
        &mut self, mapping_id: u64, pid: u64, strategy: UnmapStrategy, now: u64,
    ) -> bool {
        let entry = match self.shared_refs.get_mut(&mapping_id) {
            Some(e) => e,
            None => return false,
        };

        match strategy {
            UnmapStrategy::Deferred => {
                entry.deferred_until = now + 10_000_000; // 10ms defer
                self.deferred.push((mapping_id, pid, entry.deferred_until));
                self.stats.deferred_unmaps += 1;
                return true;
            }
            UnmapStrategy::Immediate | UnmapStrategy::Batched | UnmapStrategy::Lazy => {}
        }

        // Drop refcount
        entry.pids.retain(|&p| p != pid);
        entry.refcount = entry.refcount.saturating_sub(1);
        self.stats.refcount_drops += 1;

        if entry.can_reclaim() {
            self.stats.coordinated_unmaps += 1;
        }
        true
    }

    /// Process deferred unmaps that have reached their deadline
    pub fn process_deferred(&mut self, now: u64) -> Vec<u64> {
        let mut ready = Vec::new();
        self.deferred.retain(|&(mapping_id, pid, deadline)| {
            if now >= deadline {
                if let Some(entry) = self.shared_refs.get_mut(&mapping_id) {
                    entry.pids.retain(|&p| p != pid);
                    entry.refcount = entry.refcount.saturating_sub(1);
                    if entry.can_reclaim() {
                        ready.push(mapping_id);
                    }
                }
                false
            } else {
                true
            }
        });
        ready
    }

    /// Create a batched TLB shootdown
    pub fn batch_tlb_shootdown(&mut self, cpus: Vec<u32>, pages: Vec<u64>, now: u64) -> u64 {
        let id = self.next_batch;
        self.next_batch += 1;
        let page_count = pages.len() as u64;
        self.shootdown_queue.push(TlbShootdownBatch {
            batch_id: id, target_cpus: cpus, pages,
            initiated_at: now, completed: false,
        });
        self.stats.tlb_shootdowns += 1;
        self.stats.tlb_pages_invalidated += page_count;
        id
    }

    /// Complete a TLB shootdown batch
    pub fn complete_shootdown(&mut self, batch_id: u64) {
        if let Some(batch) = self.shootdown_queue.iter_mut().find(|b| b.batch_id == batch_id) {
            batch.completed = true;
        }
        self.shootdown_queue.retain(|b| !b.completed);
    }

    /// Detect orphan pages from dead processes
    pub fn scan_orphans(&mut self, live_pids: &[u64]) -> Vec<u64> {
        let mut orphan_mappings = Vec::new();
        for (id, entry) in &self.shared_refs {
            let has_live = entry.pids.iter().any(|p| live_pids.contains(p));
            if !has_live && entry.refcount > 0 {
                orphan_mappings.push(*id);
            }
        }
        for &id in &orphan_mappings {
            if let Some(entry) = self.shared_refs.get(&id) {
                self.stats.orphan_pages_reclaimed += entry.page_count();
            }
            self.shared_refs.remove(&id);
        }
        orphan_mappings
    }

    pub fn reclaimable_mappings(&self) -> Vec<u64> {
        self.shared_refs.iter()
            .filter(|(_, r)| r.can_reclaim())
            .map(|(id, _)| *id)
            .collect()
    }

    pub fn stats(&self) -> &MunmapCoopStats { &self.stats }
}
