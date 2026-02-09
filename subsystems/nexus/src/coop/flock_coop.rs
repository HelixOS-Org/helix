// SPDX-License-Identifier: GPL-2.0
//! Coop flock — cooperative file locking with fairness tracking

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Coop lock type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopLockType {
    SharedRead,
    ExclusiveWrite,
    IntentShared,
    IntentExclusive,
}

/// Coop lock state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopLockState {
    Granted,
    Waiting,
    Converting,
    Released,
}

/// Cooperative file lock
#[derive(Debug, Clone)]
pub struct CoopFileLock {
    pub inode: u64,
    pub owner_id: u64,
    pub lock_type: CoopLockType,
    pub state: CoopLockState,
    pub start: u64,
    pub end: u64,
    pub wait_ns: u64,
    pub grants: u64,
}

impl CoopFileLock {
    pub fn new(inode: u64, owner_id: u64, lock_type: CoopLockType) -> Self {
        Self { inode, owner_id, lock_type, state: CoopLockState::Waiting, start: 0, end: u64::MAX, wait_ns: 0, grants: 0 }
    }

    #[inline(always)]
    pub fn grant(&mut self) { self.state = CoopLockState::Granted; self.grants += 1; }
    #[inline(always)]
    pub fn release(&mut self) { self.state = CoopLockState::Released; }
    #[inline(always)]
    pub fn is_compatible(&self, other: &CoopFileLock) -> bool {
        matches!((self.lock_type, other.lock_type), (CoopLockType::SharedRead, CoopLockType::SharedRead) | (CoopLockType::IntentShared, CoopLockType::IntentShared))
    }
}

/// Fairness tracker
#[derive(Debug, Clone)]
pub struct CoopLockFairness {
    pub total_waits: u64,
    pub total_wait_ns: u64,
    pub max_wait_ns: u64,
    pub starvation_count: u64,
}

impl CoopLockFairness {
    pub fn new() -> Self { Self { total_waits: 0, total_wait_ns: 0, max_wait_ns: 0, starvation_count: 0 } }
    #[inline]
    pub fn record_wait(&mut self, wait_ns: u64) {
        self.total_waits += 1;
        self.total_wait_ns += wait_ns;
        if wait_ns > self.max_wait_ns { self.max_wait_ns = wait_ns; }
        if wait_ns > 10_000_000_000 { self.starvation_count += 1; }
    }
    #[inline(always)]
    pub fn avg_wait_ns(&self) -> u64 { if self.total_waits == 0 { 0 } else { self.total_wait_ns / self.total_waits } }
}

/// Coop flock stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CoopFlockStats {
    pub total_locks: u64,
    pub granted: u64,
    pub conflicts: u64,
    pub deadlocks_prevented: u64,
}

/// Main coop flock
#[derive(Debug)]
pub struct CoopFlock {
    pub fairness: CoopLockFairness,
    pub stats: CoopFlockStats,
}

impl CoopFlock {
    pub fn new() -> Self {
        Self { fairness: CoopLockFairness::new(), stats: CoopFlockStats { total_locks: 0, granted: 0, conflicts: 0, deadlocks_prevented: 0 } }
    }

    #[inline]
    pub fn request(&mut self, granted: bool, wait_ns: u64) {
        self.stats.total_locks += 1;
        if granted { self.stats.granted += 1; } else { self.stats.conflicts += 1; }
        if wait_ns > 0 { self.fairness.record_wait(wait_ns); }
    }
}

// ============================================================================
// Merged from flock_v2_coop
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopFlockV2Type {
    SharedRead,
    ExclusiveWrite,
    Advisory,
    Mandatory,
    Lease,
    OFDRead,
    OFDWrite,
}

/// Flock lock entry
#[derive(Debug, Clone)]
pub struct CoopFlockV2Entry {
    pub lock_id: u64,
    pub inode: u64,
    pub owner: u64,
    pub lock_type: CoopFlockV2Type,
    pub start: u64,
    pub length: u64,
    pub blocking: bool,
    pub timestamp: u64,
}

/// Stats for flock cooperation
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CoopFlockV2Stats {
    pub total_locks: u64,
    pub active_locks: u64,
    pub contentions: u64,
    pub deadlocks_detected: u64,
    pub upgrades: u64,
    pub downgrades: u64,
}

/// Manager for flock cooperative operations
pub struct CoopFlockV2Manager {
    locks: BTreeMap<u64, CoopFlockV2Entry>,
    inode_locks: BTreeMap<u64, Vec<u64>>,
    next_id: u64,
    stats: CoopFlockV2Stats,
}

impl CoopFlockV2Manager {
    pub fn new() -> Self {
        Self {
            locks: BTreeMap::new(),
            inode_locks: BTreeMap::new(),
            next_id: 1,
            stats: CoopFlockV2Stats {
                total_locks: 0,
                active_locks: 0,
                contentions: 0,
                deadlocks_detected: 0,
                upgrades: 0,
                downgrades: 0,
            },
        }
    }

    pub fn acquire(&mut self, inode: u64, owner: u64, lock_type: CoopFlockV2Type, start: u64, length: u64) -> Option<u64> {
        // Check for conflicts
        if let Some(existing) = self.inode_locks.get(&inode) {
            for &lid in existing {
                if let Some(lock) = self.locks.get(&lid) {
                    if lock.owner != owner && matches!(lock.lock_type, CoopFlockV2Type::ExclusiveWrite | CoopFlockV2Type::Mandatory) {
                        if self.ranges_overlap(lock.start, lock.length, start, length) {
                            self.stats.contentions += 1;
                            return None;
                        }
                    }
                }
            }
        }
        let id = self.next_id;
        self.next_id += 1;
        let entry = CoopFlockV2Entry {
            lock_id: id,
            inode,
            owner,
            lock_type,
            start,
            length,
            blocking: false,
            timestamp: id.wrapping_mul(41),
        };
        self.locks.insert(id, entry);
        self.inode_locks.entry(inode).or_insert_with(Vec::new).push(id);
        self.stats.total_locks += 1;
        self.stats.active_locks += 1;
        Some(id)
    }

    fn ranges_overlap(&self, s1: u64, l1: u64, s2: u64, l2: u64) -> bool {
        let e1 = s1.saturating_add(l1);
        let e2 = s2.saturating_add(l2);
        s1 < e2 && s2 < e1
    }

    #[inline]
    pub fn release(&mut self, lock_id: u64) -> bool {
        if let Some(entry) = self.locks.remove(&lock_id) {
            if let Some(list) = self.inode_locks.get_mut(&entry.inode) {
                list.retain(|&id| id != lock_id);
            }
            self.stats.active_locks = self.stats.active_locks.saturating_sub(1);
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &CoopFlockV2Stats {
        &self.stats
    }
}
