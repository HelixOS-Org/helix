// SPDX-License-Identifier: GPL-2.0
//! Holistic file lock — POSIX and flock with deadlock detection

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Lock type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileLockType {
    ReadLock,
    WriteLock,
    FlockShared,
    FlockExclusive,
    Ofd,
    Lease,
}

/// Lock state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileLockState {
    Granted,
    Waiting,
    Blocked,
    Deadlocked,
    Released,
}

/// File lock request
#[derive(Debug, Clone)]
pub struct FileLockRequest {
    pub lock_id: u64,
    pub inode: u64,
    pub pid: u32,
    pub lock_type: FileLockType,
    pub state: FileLockState,
    pub start: u64,
    pub end: u64,
    pub blocking_lock: Option<u64>,
    pub wait_ns: u64,
}

impl FileLockRequest {
    pub fn new(lock_id: u64, inode: u64, pid: u32, lock_type: FileLockType, start: u64, end: u64) -> Self {
        Self {
            lock_id, inode, pid, lock_type, state: FileLockState::Waiting,
            start, end, blocking_lock: None, wait_ns: 0,
        }
    }

    #[inline(always)]
    pub fn grant(&mut self) { self.state = FileLockState::Granted; }
    #[inline(always)]
    pub fn block(&mut self, blocker: u64) { self.state = FileLockState::Blocked; self.blocking_lock = Some(blocker); }
    #[inline(always)]
    pub fn release(&mut self) { self.state = FileLockState::Released; }

    #[inline(always)]
    pub fn overlaps(&self, start: u64, end: u64) -> bool {
        self.start < end && start < self.end
    }

    #[inline(always)]
    pub fn is_write(&self) -> bool {
        matches!(self.lock_type, FileLockType::WriteLock | FileLockType::FlockExclusive)
    }

    #[inline]
    pub fn is_compatible(&self, other: &Self) -> bool {
        if self.inode != other.inode { return true; }
        if !self.overlaps(other.start, other.end) { return true; }
        !self.is_write() && !other.is_write()
    }
}

/// Deadlock detector
#[derive(Debug, Clone)]
pub struct DeadlockDetector {
    pub wait_graph: BTreeMap<u32, Vec<u32>>,
    pub deadlocks_found: u64,
    pub checks: u64,
}

impl DeadlockDetector {
    pub fn new() -> Self {
        Self { wait_graph: BTreeMap::new(), deadlocks_found: 0, checks: 0 }
    }

    #[inline(always)]
    pub fn add_edge(&mut self, waiter: u32, holder: u32) {
        self.wait_graph.entry(waiter).or_insert_with(Vec::new).push(holder);
    }

    #[inline(always)]
    pub fn remove_edges(&mut self, pid: u32) {
        self.wait_graph.remove(&pid);
    }

    pub fn has_cycle(&mut self, start: u32) -> bool {
        self.checks += 1;
        let mut visited = Vec::new();
        let mut stack = alloc::vec![start];
        while let Some(node) = stack.pop() {
            if visited.contains(&node) {
                self.deadlocks_found += 1;
                return true;
            }
            visited.push(node);
            if let Some(edges) = self.wait_graph.get(&node) {
                for &next in edges { stack.push(next); }
            }
        }
        false
    }
}

/// File lock holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct HolisticFileLockStats {
    pub total_locks: u64,
    pub granted: u64,
    pub blocked: u64,
    pub deadlocks: u64,
    pub avg_wait_ns: u64,
}

/// Main holistic file lock manager
#[derive(Debug)]
pub struct HolisticFileLock {
    pub locks: BTreeMap<u64, FileLockRequest>,
    pub detector: DeadlockDetector,
    pub stats: HolisticFileLockStats,
}

impl HolisticFileLock {
    pub fn new() -> Self {
        Self {
            locks: BTreeMap::new(),
            detector: DeadlockDetector::new(),
            stats: HolisticFileLockStats { total_locks: 0, granted: 0, blocked: 0, deadlocks: 0, avg_wait_ns: 0 },
        }
    }

    pub fn request_lock(&mut self, mut req: FileLockRequest) {
        self.stats.total_locks += 1;
        let mut conflict = None;
        for (id, existing) in &self.locks {
            if existing.state == FileLockState::Granted && !existing.is_compatible(&req) {
                conflict = Some(*id);
                break;
            }
        }
        if let Some(blocker) = conflict {
            req.block(blocker);
            self.stats.blocked += 1;
        } else {
            req.grant();
            self.stats.granted += 1;
        }
        self.locks.insert(req.lock_id, req);
    }

    #[inline]
    pub fn release_lock(&mut self, lock_id: u64) {
        if let Some(lock) = self.locks.get_mut(&lock_id) {
            lock.release();
            self.detector.remove_edges(lock.pid);
        }
    }
}
