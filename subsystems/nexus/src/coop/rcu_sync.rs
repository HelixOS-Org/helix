// SPDX-License-Identifier: GPL-2.0
//! Coop rcu_sync — read-copy-update synchronization.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// RCU reader state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RcuReaderState {
    Inactive,
    Reading,
    QuiescentPending,
}

/// RCU reader
#[derive(Debug)]
pub struct RcuReader {
    pub thread_id: u64,
    pub state: RcuReaderState,
    pub nesting: u32,
    pub quiescent_count: u64,
    pub read_sections: u64,
}

impl RcuReader {
    pub fn new(tid: u64) -> Self {
        Self { thread_id: tid, state: RcuReaderState::Inactive, nesting: 0, quiescent_count: 0, read_sections: 0 }
    }

    #[inline(always)]
    pub fn enter(&mut self) { self.nesting += 1; self.state = RcuReaderState::Reading; self.read_sections += 1; }

    #[inline(always)]
    pub fn exit(&mut self) {
        if self.nesting > 0 { self.nesting -= 1; }
        if self.nesting == 0 { self.state = RcuReaderState::Inactive; self.quiescent_count += 1; }
    }
}

/// RCU callback
#[derive(Debug)]
pub struct RcuSyncCallback {
    pub id: u64,
    pub gp_number: u64,
    pub data_hash: u64,
}

/// Grace period tracker
#[derive(Debug)]
pub struct GracePeriodTracker {
    pub current_gp: u64,
    pub completed_gp: u64,
    pub readers: BTreeMap<u64, RcuReader>,
    pub pending_callbacks: Vec<RcuSyncCallback>,
    pub completed_callbacks: u64,
}

impl GracePeriodTracker {
    pub fn new() -> Self {
        Self { current_gp: 0, readers: BTreeMap::new(), pending_callbacks: Vec::new(), completed_gp: 0, completed_callbacks: 0 }
    }

    #[inline(always)]
    pub fn register_reader(&mut self, tid: u64) { self.readers.insert(tid, RcuReader::new(tid)); }

    #[inline(always)]
    pub fn read_lock(&mut self, tid: u64) { if let Some(r) = self.readers.get_mut(&tid) { r.enter(); } }
    #[inline(always)]
    pub fn read_unlock(&mut self, tid: u64) { if let Some(r) = self.readers.get_mut(&tid) { r.exit(); } }

    pub fn synchronize(&mut self) -> bool {
        let all_quiescent = self.readers.values().all(|r| r.state == RcuReaderState::Inactive);
        if all_quiescent {
            self.completed_gp = self.current_gp;
            self.current_gp += 1;
            let gp = self.completed_gp;
            let before = self.pending_callbacks.len();
            self.pending_callbacks.retain(|cb| cb.gp_number > gp);
            self.completed_callbacks += (before - self.pending_callbacks.len()) as u64;
            true
        } else { false }
    }

    #[inline(always)]
    pub fn call_rcu(&mut self, id: u64, data_hash: u64) {
        self.pending_callbacks.push(RcuSyncCallback { id, gp_number: self.current_gp, data_hash });
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct RcuSyncStats {
    pub current_gp: u64,
    pub completed_gp: u64,
    pub active_readers: u32,
    pub pending_callbacks: u32,
    pub completed_callbacks: u64,
}

/// Main coop RCU sync
pub struct CoopRcuSync {
    tracker: GracePeriodTracker,
}

impl CoopRcuSync {
    pub fn new() -> Self { Self { tracker: GracePeriodTracker::new() } }
    #[inline(always)]
    pub fn register(&mut self, tid: u64) { self.tracker.register_reader(tid); }
    #[inline(always)]
    pub fn read_lock(&mut self, tid: u64) { self.tracker.read_lock(tid); }
    #[inline(always)]
    pub fn read_unlock(&mut self, tid: u64) { self.tracker.read_unlock(tid); }
    #[inline(always)]
    pub fn synchronize(&mut self) -> bool { self.tracker.synchronize() }
    #[inline(always)]
    pub fn call_rcu(&mut self, id: u64, data_hash: u64) { self.tracker.call_rcu(id, data_hash); }

    #[inline(always)]
    pub fn stats(&self) -> RcuSyncStats {
        let active = self.tracker.readers.values().filter(|r| r.state == RcuReaderState::Reading).count() as u32;
        RcuSyncStats { current_gp: self.tracker.current_gp, completed_gp: self.tracker.completed_gp, active_readers: active, pending_callbacks: self.tracker.pending_callbacks.len() as u32, completed_callbacks: self.tracker.completed_callbacks }
    }
}
