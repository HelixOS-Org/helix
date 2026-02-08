// SPDX-License-Identifier: GPL-2.0
//! Coop epoch_gc — epoch-based garbage collection.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Epoch state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpochState {
    Active,
    Grace,
    Reclaimable,
    Reclaimed,
}

/// Deferred item
#[derive(Debug)]
pub struct DeferredItem {
    pub id: u64,
    pub retire_epoch: u64,
    pub size_bytes: u64,
    pub state: EpochState,
}

/// Thread epoch record
#[derive(Debug)]
pub struct ThreadEpochRecord {
    pub thread_id: u64,
    pub observed_epoch: u64,
    pub is_active: bool,
    pub pin_count: u64,
}

/// Epoch collector
#[derive(Debug)]
pub struct EpochCollector {
    pub global_epoch: u64,
    pub threads: BTreeMap<u64, ThreadEpochRecord>,
    pub deferred: Vec<DeferredItem>,
    pub reclaimed_bytes: u64,
    pub reclaimed_count: u64,
}

impl EpochCollector {
    pub fn new() -> Self {
        Self { global_epoch: 0, threads: BTreeMap::new(), deferred: Vec::new(), reclaimed_bytes: 0, reclaimed_count: 0 }
    }

    pub fn register_thread(&mut self, tid: u64) {
        self.threads.insert(tid, ThreadEpochRecord { thread_id: tid, observed_epoch: self.global_epoch, is_active: false, pin_count: 0 });
    }

    pub fn pin(&mut self, tid: u64) {
        if let Some(t) = self.threads.get_mut(&tid) { t.is_active = true; t.observed_epoch = self.global_epoch; t.pin_count += 1; }
    }

    pub fn unpin(&mut self, tid: u64) {
        if let Some(t) = self.threads.get_mut(&tid) { t.is_active = false; }
    }

    pub fn retire(&mut self, id: u64, size: u64) {
        self.deferred.push(DeferredItem { id, retire_epoch: self.global_epoch, size_bytes: size, state: EpochState::Active });
    }

    pub fn try_advance(&mut self) -> bool {
        let min_epoch = self.threads.values().filter(|t| t.is_active).map(|t| t.observed_epoch).min();
        if let Some(min) = min_epoch {
            if min >= self.global_epoch { self.global_epoch += 1; return true; }
        } else {
            self.global_epoch += 1; return true;
        }
        false
    }

    pub fn reclaim(&mut self) -> u64 {
        let safe_epoch = self.threads.values().filter(|t| t.is_active).map(|t| t.observed_epoch).min().unwrap_or(self.global_epoch);
        let mut reclaimed = 0u64;
        for item in self.deferred.iter_mut() {
            if item.state == EpochState::Active && item.retire_epoch + 2 <= safe_epoch {
                item.state = EpochState::Reclaimed;
                self.reclaimed_bytes += item.size_bytes;
                self.reclaimed_count += 1;
                reclaimed += item.size_bytes;
            }
        }
        self.deferred.retain(|i| i.state != EpochState::Reclaimed);
        reclaimed
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct EpochGcStats {
    pub global_epoch: u64,
    pub active_threads: u32,
    pub pending_items: u32,
    pub pending_bytes: u64,
    pub total_reclaimed_bytes: u64,
}

/// Main coop epoch gc
pub struct CoopEpochGc {
    collector: EpochCollector,
}

impl CoopEpochGc {
    pub fn new() -> Self { Self { collector: EpochCollector::new() } }
    pub fn register(&mut self, tid: u64) { self.collector.register_thread(tid); }
    pub fn pin(&mut self, tid: u64) { self.collector.pin(tid); }
    pub fn unpin(&mut self, tid: u64) { self.collector.unpin(tid); }
    pub fn retire(&mut self, id: u64, size: u64) { self.collector.retire(id, size); }
    pub fn collect(&mut self) -> u64 { self.collector.try_advance(); self.collector.reclaim() }

    pub fn stats(&self) -> EpochGcStats {
        let active = self.collector.threads.values().filter(|t| t.is_active).count() as u32;
        let pending_bytes: u64 = self.collector.deferred.iter().map(|d| d.size_bytes).sum();
        EpochGcStats { global_epoch: self.collector.global_epoch, active_threads: active, pending_items: self.collector.deferred.len() as u32, pending_bytes, total_reclaimed_bytes: self.collector.reclaimed_bytes }
    }
}
