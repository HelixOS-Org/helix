// SPDX-License-Identifier: GPL-2.0
//! NEXUS Bridge — Mlock (memory locking bridge)

extern crate alloc;
use alloc::collections::BTreeMap;

/// Mlock operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeMlockOp { Lock, LockAll, Unlock, UnlockAll }

/// Locked region
#[derive(Debug, Clone)]
pub struct BridgeMlockRegion { pub addr: u64, pub length: u64, pub onfault: bool }

/// Mlock stats
#[derive(Debug, Clone)]
pub struct BridgeMlockStats { pub total_ops: u64, pub locks: u64, pub unlocks: u64, pub locked_bytes: u64, pub peak_locked: u64 }

/// Manager for mlock bridge
pub struct BridgeMlockManager {
    locked: BTreeMap<u64, BridgeMlockRegion>,
    total_locked: u64,
    stats: BridgeMlockStats,
}

impl BridgeMlockManager {
    pub fn new() -> Self {
        Self { locked: BTreeMap::new(), total_locked: 0, stats: BridgeMlockStats { total_ops: 0, locks: 0, unlocks: 0, locked_bytes: 0, peak_locked: 0 } }
    }

    pub fn mlock(&mut self, addr: u64, length: u64, onfault: bool) {
        self.stats.total_ops += 1; self.stats.locks += 1;
        self.locked.insert(addr, BridgeMlockRegion { addr, length, onfault });
        self.total_locked += length; self.stats.locked_bytes = self.total_locked;
        if self.total_locked > self.stats.peak_locked { self.stats.peak_locked = self.total_locked; }
    }

    pub fn munlock(&mut self, addr: u64) {
        if let Some(r) = self.locked.remove(&addr) { self.total_locked -= r.length; self.stats.locked_bytes = self.total_locked; self.stats.unlocks += 1; }
        self.stats.total_ops += 1;
    }

    pub fn stats(&self) -> &BridgeMlockStats { &self.stats }
}
