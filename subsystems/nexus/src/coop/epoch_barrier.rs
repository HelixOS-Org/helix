// SPDX-License-Identifier: GPL-2.0
//! Coop epoch_barrier — epoch-based reclamation barrier for lock-free data structures.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Epoch state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpochState {
    /// Thread is not in a critical section
    Idle,
    /// Thread is pinned to the current epoch
    Pinned,
    /// Thread has deferred cleanup
    Deferred,
}

/// An epoch value (wrapping counter)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Epoch(pub u64);

impl Epoch {
    #[inline(always)]
    pub fn next(self) -> Self {
        Epoch(self.0.wrapping_add(1))
    }

    #[inline(always)]
    pub fn distance(self, other: Epoch) -> u64 {
        self.0.wrapping_sub(other.0)
    }

    #[inline(always)]
    pub fn is_safe_to_reclaim(self, current: Epoch, grace: u64) -> bool {
        current.0.wrapping_sub(self.0) >= grace
    }
}

/// A deferred destruction entry
#[derive(Debug)]
pub struct DeferredDrop {
    pub id: u64,
    pub epoch: Epoch,
    pub size_bytes: usize,
    pub owner: u64,
    pub enqueue_ns: u64,
}

impl DeferredDrop {
    pub fn new(id: u64, epoch: Epoch, size: usize, owner: u64) -> Self {
        Self { id, epoch, size_bytes: size, owner, enqueue_ns: 0 }
    }
}

/// Per-thread epoch state
#[derive(Debug)]
pub struct ThreadEpoch {
    pub thread_id: u64,
    pub state: EpochState,
    pub local_epoch: Epoch,
    pub pin_count: u32,
    pub deferred: Vec<DeferredDrop>,
    pub total_pins: u64,
    pub total_reclaims: u64,
    pub total_deferred_bytes: u64,
}

impl ThreadEpoch {
    pub fn new(thread_id: u64) -> Self {
        Self {
            thread_id,
            state: EpochState::Idle,
            local_epoch: Epoch(0),
            pin_count: 0,
            deferred: Vec::new(),
            total_pins: 0,
            total_reclaims: 0,
            total_deferred_bytes: 0,
        }
    }

    #[inline]
    pub fn pin(&mut self, global_epoch: Epoch) {
        self.pin_count += 1;
        if self.pin_count == 1 {
            self.state = EpochState::Pinned;
            self.local_epoch = global_epoch;
            self.total_pins += 1;
        }
    }

    #[inline]
    pub fn unpin(&mut self) {
        self.pin_count = self.pin_count.saturating_sub(1);
        if self.pin_count == 0 {
            self.state = EpochState::Idle;
        }
    }

    #[inline]
    pub fn defer(&mut self, drop: DeferredDrop) {
        self.total_deferred_bytes += drop.size_bytes as u64;
        self.deferred.push(drop);
        if self.state != EpochState::Pinned {
            self.state = EpochState::Deferred;
        }
    }

    pub fn reclaim(&mut self, safe_epoch: Epoch) -> Vec<DeferredDrop> {
        let mut reclaimed = Vec::new();
        self.deferred.retain(|d| {
            if d.epoch.is_safe_to_reclaim(safe_epoch, 2) {
                reclaimed.push(DeferredDrop {
                    id: d.id,
                    epoch: d.epoch,
                    size_bytes: d.size_bytes,
                    owner: d.owner,
                    enqueue_ns: d.enqueue_ns,
                });
                false
            } else {
                true
            }
        });
        self.total_reclaims += reclaimed.len() as u64;
        reclaimed
    }

    #[inline(always)]
    pub fn pending_bytes(&self) -> u64 {
        self.deferred.iter().map(|d| d.size_bytes as u64).sum()
    }
}

/// Epoch barrier stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EpochBarrierStats {
    pub global_epoch: u64,
    pub total_threads: u64,
    pub total_pins: u64,
    pub total_unpins: u64,
    pub total_deferred: u64,
    pub total_reclaimed: u64,
    pub total_reclaimed_bytes: u64,
    pub pending_deferred: u64,
    pub pending_bytes: u64,
    pub epoch_advances: u64,
}

/// Main epoch barrier manager
pub struct CoopEpochBarrier {
    global_epoch: Epoch,
    threads: BTreeMap<u64, ThreadEpoch>,
    next_drop_id: u64,
    grace_epochs: u64,
    stats: EpochBarrierStats,
}

impl CoopEpochBarrier {
    pub fn new() -> Self {
        Self {
            global_epoch: Epoch(0),
            threads: BTreeMap::new(),
            next_drop_id: 1,
            grace_epochs: 2,
            stats: EpochBarrierStats {
                global_epoch: 0,
                total_threads: 0,
                total_pins: 0,
                total_unpins: 0,
                total_deferred: 0,
                total_reclaimed: 0,
                total_reclaimed_bytes: 0,
                pending_deferred: 0,
                pending_bytes: 0,
                epoch_advances: 0,
            },
        }
    }

    #[inline(always)]
    pub fn register_thread(&mut self, thread_id: u64) {
        self.threads.insert(thread_id, ThreadEpoch::new(thread_id));
        self.stats.total_threads += 1;
    }

    #[inline(always)]
    pub fn unregister_thread(&mut self, thread_id: u64) {
        self.threads.remove(&thread_id);
        self.stats.total_threads = self.stats.total_threads.saturating_sub(1);
    }

    #[inline]
    pub fn pin(&mut self, thread_id: u64) {
        if let Some(t) = self.threads.get_mut(&thread_id) {
            t.pin(self.global_epoch);
            self.stats.total_pins += 1;
        }
    }

    #[inline]
    pub fn unpin(&mut self, thread_id: u64) {
        if let Some(t) = self.threads.get_mut(&thread_id) {
            t.unpin();
            self.stats.total_unpins += 1;
        }
    }

    pub fn defer_drop(&mut self, thread_id: u64, size: usize, owner: u64) -> u64 {
        let id = self.next_drop_id;
        self.next_drop_id += 1;
        let drop_entry = DeferredDrop::new(id, self.global_epoch, size, owner);
        if let Some(t) = self.threads.get_mut(&thread_id) {
            t.defer(drop_entry);
            self.stats.total_deferred += 1;
            self.stats.pending_deferred += 1;
            self.stats.pending_bytes += size as u64;
        }
        id
    }

    pub fn try_advance(&mut self) -> bool {
        // Check if all threads have observed the current epoch
        let min_epoch = self.threads.values()
            .filter(|t| t.state == EpochState::Pinned)
            .map(|t| t.local_epoch)
            .min();

        match min_epoch {
            Some(min) if min == self.global_epoch => {
                // All pinned threads are at current epoch, safe to advance
                self.global_epoch = self.global_epoch.next();
                self.stats.global_epoch = self.global_epoch.0;
                self.stats.epoch_advances += 1;
                true
            }
            None => {
                // No pinned threads, safe to advance
                self.global_epoch = self.global_epoch.next();
                self.stats.global_epoch = self.global_epoch.0;
                self.stats.epoch_advances += 1;
                true
            }
            _ => false,
        }
    }

    pub fn reclaim_all(&mut self) -> (u64, u64) {
        let safe = self.global_epoch;
        let mut total_count = 0u64;
        let mut total_bytes = 0u64;
        let thread_ids: Vec<u64> = self.threads.keys().copied().collect();
        for tid in thread_ids {
            if let Some(t) = self.threads.get_mut(&tid) {
                let reclaimed = t.reclaim(safe);
                for d in &reclaimed {
                    total_bytes += d.size_bytes as u64;
                }
                total_count += reclaimed.len() as u64;
            }
        }
        self.stats.total_reclaimed += total_count;
        self.stats.total_reclaimed_bytes += total_bytes;
        self.stats.pending_deferred = self.stats.pending_deferred.saturating_sub(total_count);
        self.stats.pending_bytes = self.stats.pending_bytes.saturating_sub(total_bytes);
        (total_count, total_bytes)
    }

    #[inline(always)]
    pub fn set_grace_epochs(&mut self, grace: u64) {
        self.grace_epochs = grace;
    }

    #[inline]
    pub fn lagging_threads(&self) -> Vec<u64> {
        self.threads.iter()
            .filter(|(_, t)| {
                t.state == EpochState::Pinned && self.global_epoch.distance(t.local_epoch) > self.grace_epochs
            })
            .map(|(&tid, _)| tid)
            .collect()
    }

    #[inline]
    pub fn heaviest_threads(&self, top: usize) -> Vec<(u64, u64)> {
        let mut v: Vec<(u64, u64)> = self.threads.iter()
            .map(|(&tid, t)| (tid, t.pending_bytes()))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.truncate(top);
        v
    }

    #[inline(always)]
    pub fn current_epoch(&self) -> Epoch {
        self.global_epoch
    }

    #[inline(always)]
    pub fn stats(&self) -> &EpochBarrierStats {
        &self.stats
    }
}
