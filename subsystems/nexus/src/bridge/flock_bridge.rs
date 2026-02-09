// SPDX-License-Identifier: GPL-2.0
//! Bridge flock_bridge — file locking (flock/POSIX/OFD) bridge.

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

/// Lock type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlockType {
    /// Shared (read) lock
    Shared,
    /// Exclusive (write) lock
    Exclusive,
    /// Unlock
    Unlock,
}

/// Lock mechanism
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockMechanism {
    /// BSD flock()
    Flock,
    /// POSIX fcntl() locks
    Posix,
    /// Open File Description locks
    Ofd,
    /// Mandatory locks
    Mandatory,
    /// Lease locks
    Lease,
}

/// Lock state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockState {
    Granted,
    Waiting,
    Blocked,
    Cancelled,
}

/// A file lock
#[derive(Debug, Clone)]
pub struct FileLock {
    pub id: u64,
    pub inode: u64,
    pub lock_type: FlockType,
    pub mechanism: LockMechanism,
    pub state: LockState,
    pub pid: u32,
    pub fd: i32,
    pub start: u64,
    pub end: u64,
    pub timestamp: u64,
    pub wait_time_ns: u64,
}

impl FileLock {
    pub fn new(id: u64, inode: u64, lock_type: FlockType, mechanism: LockMechanism, pid: u32) -> Self {
        Self {
            id, inode, lock_type, mechanism,
            state: LockState::Waiting,
            pid, fd: -1,
            start: 0, end: u64::MAX,
            timestamp: 0, wait_time_ns: 0,
        }
    }

    #[inline(always)]
    pub fn is_whole_file(&self) -> bool {
        self.start == 0 && self.end == u64::MAX
    }

    #[inline(always)]
    pub fn overlaps(&self, start: u64, end: u64) -> bool {
        self.start < end && start < self.end
    }

    #[inline]
    pub fn conflicts_with(&self, other: &FileLock) -> bool {
        if self.inode != other.inode { return false; }
        if !self.overlaps(other.start, other.end) { return false; }
        // shared-shared is ok
        if self.lock_type == FlockType::Shared && other.lock_type == FlockType::Shared {
            return false;
        }
        true
    }

    #[inline(always)]
    pub fn is_granted(&self) -> bool {
        self.state == LockState::Granted
    }
}

/// Lock operation record
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct LockOp {
    pub lock_id: u64,
    pub op: LockOpType,
    pub pid: u32,
    pub inode: u64,
    pub result: LockOpResult,
    pub latency_ns: u64,
    pub timestamp: u64,
}

/// Lock operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockOpType {
    Lock,
    TryLock,
    Unlock,
    Upgrade,
    Downgrade,
}

/// Lock operation result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockOpResult {
    Success,
    WouldBlock,
    Deadlock,
    Interrupted,
    BadFd,
    NoLock,
}

/// Per-inode lock state
#[derive(Debug)]
#[repr(align(64))]
pub struct InodeLockState {
    pub inode: u64,
    pub active_locks: Vec<u64>,
    pub waiting_locks: Vec<u64>,
    pub reader_count: u32,
    pub writer_count: u32,
    pub contention_count: u64,
}

impl InodeLockState {
    pub fn new(inode: u64) -> Self {
        Self {
            inode, active_locks: Vec::new(),
            waiting_locks: Vec::new(),
            reader_count: 0, writer_count: 0,
            contention_count: 0,
        }
    }

    #[inline(always)]
    pub fn is_contended(&self) -> bool {
        !self.waiting_locks.is_empty()
    }

    #[inline(always)]
    pub fn total_locks(&self) -> usize {
        self.active_locks.len() + self.waiting_locks.len()
    }
}

/// Flock bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FlockBridgeStats {
    pub active_locks: u64,
    pub waiting_locks: u64,
    pub total_lock_ops: u64,
    pub total_contention: u64,
    pub deadlocks_detected: u64,
    pub avg_wait_time_ns: u64,
    pub peak_wait_time_ns: u64,
}

/// Main flock bridge
#[repr(align(64))]
pub struct BridgeFlock {
    locks: BTreeMap<u64, FileLock>,
    inodes: BTreeMap<u64, InodeLockState>,
    ops: VecDeque<LockOp>,
    max_ops: usize,
    next_id: u64,
    stats: FlockBridgeStats,
}

impl BridgeFlock {
    pub fn new() -> Self {
        Self {
            locks: BTreeMap::new(),
            inodes: BTreeMap::new(),
            ops: VecDeque::new(),
            max_ops: 4096,
            next_id: 1,
            stats: FlockBridgeStats {
                active_locks: 0, waiting_locks: 0,
                total_lock_ops: 0, total_contention: 0,
                deadlocks_detected: 0, avg_wait_time_ns: 0,
                peak_wait_time_ns: 0,
            },
        }
    }

    pub fn request_lock(&mut self, inode: u64, lock_type: FlockType,
                         mechanism: LockMechanism, pid: u32, now: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let mut lock = FileLock::new(id, inode, lock_type, mechanism, pid);
        lock.timestamp = now;

        let inode_state = self.inodes.entry(inode)
            .or_insert_with(|| InodeLockState::new(inode));

        // check conflicts
        let has_conflict = inode_state.active_locks.iter().any(|&lid| {
            self.locks.get(&lid).map(|l| l.conflicts_with(&lock)).unwrap_or(false)
        });

        if has_conflict {
            lock.state = LockState::Waiting;
            inode_state.waiting_locks.push(id);
            inode_state.contention_count += 1;
            self.stats.waiting_locks += 1;
            self.stats.total_contention += 1;
        } else {
            lock.state = LockState::Granted;
            inode_state.active_locks.push(id);
            match lock_type {
                FlockType::Shared => inode_state.reader_count += 1,
                FlockType::Exclusive => inode_state.writer_count += 1,
                FlockType::Unlock => {}
            }
            self.stats.active_locks += 1;
        }

        self.locks.insert(id, lock);
        self.stats.total_lock_ops += 1;
        id
    }

    pub fn unlock(&mut self, lock_id: u64) -> bool {
        if let Some(lock) = self.locks.remove(&lock_id) {
            if lock.state == LockState::Granted && self.stats.active_locks > 0 {
                self.stats.active_locks -= 1;
            }
            if lock.state == LockState::Waiting && self.stats.waiting_locks > 0 {
                self.stats.waiting_locks -= 1;
            }
            if let Some(inode_state) = self.inodes.get_mut(&lock.inode) {
                inode_state.active_locks.retain(|&id| id != lock_id);
                inode_state.waiting_locks.retain(|&id| id != lock_id);
                match lock.lock_type {
                    FlockType::Shared => {
                        if inode_state.reader_count > 0 { inode_state.reader_count -= 1; }
                    }
                    FlockType::Exclusive => {
                        if inode_state.writer_count > 0 { inode_state.writer_count -= 1; }
                    }
                    FlockType::Unlock => {}
                }
            }
            true
        } else { false }
    }

    #[inline]
    pub fn record_op(&mut self, op: LockOp) {
        if op.result == LockOpResult::Deadlock {
            self.stats.deadlocks_detected += 1;
        }
        if op.latency_ns > self.stats.peak_wait_time_ns {
            self.stats.peak_wait_time_ns = op.latency_ns;
        }
        if self.ops.len() >= self.max_ops { self.ops.pop_front(); }
        self.ops.push_back(op);
    }

    #[inline]
    pub fn contended_inodes(&self) -> Vec<(u64, u64)> {
        self.inodes.iter()
            .filter(|(_, s)| s.is_contended())
            .map(|(&ino, s)| (ino, s.contention_count))
            .collect()
    }

    #[inline(always)]
    pub fn stats(&self) -> &FlockBridgeStats {
        &self.stats
    }
}

// ============================================================================
// Merged from flock_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlockV2Type {
    SharedRead,
    ExclusiveWrite,
    Unlock,
}

/// Lock state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlockV2State {
    Granted,
    Waiting,
    Blocked,
    Released,
}

/// POSIX lock range
#[derive(Debug, Clone)]
pub struct PosixLock {
    pub id: u64,
    pub pid: u64,
    pub fd: i32,
    pub inode: u64,
    pub lock_type: FlockV2Type,
    pub start: u64,
    pub length: u64,
    pub state: FlockV2State,
    pub wait_start: u64,
    pub granted_at: u64,
}

impl PosixLock {
    pub fn new(id: u64, pid: u64, fd: i32, inode: u64, ltype: FlockV2Type, start: u64, len: u64, now: u64) -> Self {
        Self { id, pid, fd, inode, lock_type: ltype, start, length: len, state: FlockV2State::Waiting, wait_start: now, granted_at: 0 }
    }

    #[inline(always)]
    pub fn grant(&mut self, now: u64) { self.state = FlockV2State::Granted; self.granted_at = now; }
    #[inline(always)]
    pub fn release(&mut self) { self.state = FlockV2State::Released; }

    #[inline]
    pub fn overlaps(&self, start: u64, len: u64) -> bool {
        let end = if self.length == 0 { u64::MAX } else { self.start + self.length };
        let other_end = if len == 0 { u64::MAX } else { start + len };
        self.start < other_end && start < end
    }

    #[inline]
    pub fn conflicts(&self, other: &PosixLock) -> bool {
        if self.inode != other.inode { return false; }
        if !self.overlaps(other.start, other.length) { return false; }
        matches!((self.lock_type, other.lock_type),
            (FlockV2Type::ExclusiveWrite, _) | (_, FlockV2Type::ExclusiveWrite))
    }

    #[inline(always)]
    pub fn wait_time_ns(&self, now: u64) -> u64 {
        if self.state == FlockV2State::Waiting { now.saturating_sub(self.wait_start) } else { 0 }
    }
}

/// Deadlock detection edge
#[derive(Debug, Clone)]
pub struct DeadlockEdge {
    pub waiter_pid: u64,
    pub holder_pid: u64,
    pub inode: u64,
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FlockV2BridgeStats {
    pub total_locks: u32,
    pub granted: u32,
    pub waiting: u32,
    pub total_granted: u64,
    pub total_released: u64,
    pub deadlocks_detected: u64,
    pub avg_wait_ns: u64,
}

/// Main flock v2 bridge
#[repr(align(64))]
pub struct BridgeFlockV2 {
    locks: BTreeMap<u64, PosixLock>,
    next_id: u64,
    deadlocks: u64,
}

impl BridgeFlockV2 {
    pub fn new() -> Self { Self { locks: BTreeMap::new(), next_id: 1, deadlocks: 0 } }

    pub fn lock(&mut self, pid: u64, fd: i32, inode: u64, ltype: FlockV2Type, start: u64, len: u64, now: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        let mut lock = PosixLock::new(id, pid, fd, inode, ltype, start, len, now);

        let conflicts: Vec<u64> = self.locks.values()
            .filter(|l| l.state == FlockV2State::Granted && l.pid != pid && lock.conflicts(l))
            .map(|l| l.id).collect();

        if conflicts.is_empty() { lock.grant(now); }
        else { lock.state = FlockV2State::Blocked; }

        self.locks.insert(id, lock);
        id
    }

    #[inline(always)]
    pub fn unlock(&mut self, id: u64) {
        if let Some(lock) = self.locks.get_mut(&id) { lock.release(); }
    }

    pub fn stats(&self) -> FlockV2BridgeStats {
        let granted = self.locks.values().filter(|l| l.state == FlockV2State::Granted).count() as u32;
        let waiting = self.locks.values().filter(|l| matches!(l.state, FlockV2State::Waiting | FlockV2State::Blocked)).count() as u32;
        let total_granted = self.locks.values().filter(|l| l.granted_at > 0).count() as u64;
        let released = self.locks.values().filter(|l| l.state == FlockV2State::Released).count() as u64;
        let waits: Vec<u64> = self.locks.values().filter(|l| l.granted_at > l.wait_start).map(|l| l.granted_at - l.wait_start).collect();
        let avg = if waits.is_empty() { 0 } else { waits.iter().sum::<u64>() / waits.len() as u64 };
        FlockV2BridgeStats {
            total_locks: self.locks.len() as u32, granted, waiting,
            total_granted, total_released: released,
            deadlocks_detected: self.deadlocks, avg_wait_ns: avg,
        }
    }
}

// ============================================================================
// Merged from flock_v3_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlockV3Type {
    SharedRead,
    ExclusiveWrite,
    Unlock,
}

/// Lock scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockScopeV3 {
    Flock,
    Posix,
    Ofd,
    Lease,
}

/// File lock v3
#[derive(Debug)]
pub struct FileLockV3 {
    pub id: u64,
    pub fd: u64,
    pub pid: u64,
    pub lock_type: FlockV3Type,
    pub scope: LockScopeV3,
    pub start: u64,
    pub end: u64,
    pub acquired_at: u64,
    pub blocking: bool,
}

impl FileLockV3 {
    pub fn new(id: u64, fd: u64, pid: u64, lt: FlockV3Type, scope: LockScopeV3, start: u64, end: u64, now: u64) -> Self {
        Self { id, fd, pid, lock_type: lt, scope, start, end, acquired_at: now, blocking: false }
    }
}

/// Deadlock detector
#[derive(Debug)]
#[repr(align(64))]
pub struct DeadlockDetector {
    pub wait_graph: LinearMap<u64, 64>,
    pub detected_cycles: u64,
}

impl DeadlockDetector {
    pub fn new() -> Self { Self { wait_graph: LinearMap::new(), detected_cycles: 0 } }

    pub fn add_wait(&mut self, waiter: u64, holder: u64) -> bool {
        self.wait_graph.insert(waiter, holder);
        let mut visited = Vec::new();
        let mut current = waiter;
        loop {
            if visited.contains(&current) { self.detected_cycles += 1; self.wait_graph.remove(waiter); return true; }
            visited.push(current);
            if let Some(&next) = self.wait_graph.get(current) { current = next; }
            else { break; }
        }
        false
    }

    #[inline(always)]
    pub fn remove_wait(&mut self, waiter: u64) { self.wait_graph.remove(waiter); }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FlockV3BridgeStats {
    pub active_locks: u32,
    pub shared_locks: u32,
    pub exclusive_locks: u32,
    pub deadlocks_detected: u64,
}

/// Main bridge flock v3
#[repr(align(64))]
pub struct BridgeFlockV3 {
    locks: BTreeMap<u64, FileLockV3>,
    detector: DeadlockDetector,
    next_id: u64,
}

impl BridgeFlockV3 {
    pub fn new() -> Self { Self { locks: BTreeMap::new(), detector: DeadlockDetector::new(), next_id: 1 } }

    #[inline]
    pub fn lock(&mut self, fd: u64, pid: u64, lt: FlockV3Type, scope: LockScopeV3, start: u64, end: u64, now: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.locks.insert(id, FileLockV3::new(id, fd, pid, lt, scope, start, end, now));
        id
    }

    #[inline(always)]
    pub fn unlock(&mut self, id: u64) { self.locks.remove(&id); }

    #[inline]
    pub fn stats(&self) -> FlockV3BridgeStats {
        let shared = self.locks.values().filter(|l| l.lock_type == FlockV3Type::SharedRead).count() as u32;
        let excl = self.locks.values().filter(|l| l.lock_type == FlockV3Type::ExclusiveWrite).count() as u32;
        FlockV3BridgeStats { active_locks: self.locks.len() as u32, shared_locks: shared, exclusive_locks: excl, deadlocks_detected: self.detector.detected_cycles }
    }
}

// ============================================================================
// Merged from flock_v4_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlockV4Op {
    FlockSh,
    FlockEx,
    FlockUn,
    FlockNb,
    OfdSetlk,
    OfdSetlkw,
    OfdGetlk,
    LeaseSet,
    LeaseGet,
    LeaseBreak,
}

/// Flock v4 result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlockV4Result {
    Granted,
    WouldBlock,
    Deadlock,
    Interrupted,
    BadFd,
    LeaseBreaking,
    Error,
}

/// Flock v4 bridge record
#[derive(Debug, Clone)]
pub struct FlockV4BridgeRecord {
    pub op: FlockV4Op,
    pub result: FlockV4Result,
    pub fd: i32,
    pub inode: u64,
    pub pid: u32,
    pub start: u64,
    pub end: u64,
    pub wait_ns: u64,
    pub lease_type: u8,
}

impl FlockV4BridgeRecord {
    pub fn new(op: FlockV4Op, fd: i32, inode: u64) -> Self {
        Self { op, result: FlockV4Result::Granted, fd, inode, pid: 0, start: 0, end: u64::MAX, wait_ns: 0, lease_type: 0 }
    }

    #[inline(always)]
    pub fn is_lease_op(&self) -> bool {
        matches!(self.op, FlockV4Op::LeaseSet | FlockV4Op::LeaseGet | FlockV4Op::LeaseBreak)
    }

    #[inline(always)]
    pub fn is_ofd(&self) -> bool {
        matches!(self.op, FlockV4Op::OfdSetlk | FlockV4Op::OfdSetlkw | FlockV4Op::OfdGetlk)
    }
}

/// Flock v4 bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FlockV4BridgeStats {
    pub total_ops: u64,
    pub locks_granted: u64,
    pub would_blocks: u64,
    pub deadlocks: u64,
    pub lease_ops: u64,
    pub lease_breaks: u64,
    pub total_wait_ns: u64,
}

/// Main bridge flock v4
#[derive(Debug)]
pub struct BridgeFlockV4 {
    pub stats: FlockV4BridgeStats,
}

impl BridgeFlockV4 {
    pub fn new() -> Self {
        Self { stats: FlockV4BridgeStats { total_ops: 0, locks_granted: 0, would_blocks: 0, deadlocks: 0, lease_ops: 0, lease_breaks: 0, total_wait_ns: 0 } }
    }

    pub fn record(&mut self, rec: &FlockV4BridgeRecord) {
        self.stats.total_ops += 1;
        self.stats.total_wait_ns += rec.wait_ns;
        if rec.is_lease_op() { self.stats.lease_ops += 1; }
        if matches!(rec.op, FlockV4Op::LeaseBreak) { self.stats.lease_breaks += 1; }
        match rec.result {
            FlockV4Result::Granted => self.stats.locks_granted += 1,
            FlockV4Result::WouldBlock => self.stats.would_blocks += 1,
            FlockV4Result::Deadlock => self.stats.deadlocks += 1,
            _ => {}
        }
    }

    #[inline(always)]
    pub fn contention_rate(&self) -> f64 {
        if self.stats.total_ops == 0 { 0.0 }
        else { (self.stats.would_blocks + self.stats.deadlocks) as f64 / self.stats.total_ops as f64 }
    }
}

// ============================================================================
// Merged from flock_v5_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlockV5Type { SharedRead, ExclusiveWrite, Unlock, TestLock }

/// Flock v5 record
#[derive(Debug, Clone)]
pub struct FlockV5Record {
    pub lock_type: FlockV5Type,
    pub fd: i32,
    pub inode: u64,
    pub start: u64,
    pub len: u64,
    pub pid: u32,
}

impl FlockV5Record {
    pub fn new(lock_type: FlockV5Type, fd: i32) -> Self { Self { lock_type, fd, inode: 0, start: 0, len: 0, pid: 0 } }
}

/// Flock v5 bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FlockV5BridgeStats { pub total_ops: u64, pub shared: u64, pub exclusive: u64, pub unlocks: u64 }

/// Main bridge flock v5
#[derive(Debug)]
pub struct BridgeFlockV5 { pub stats: FlockV5BridgeStats }

impl BridgeFlockV5 {
    pub fn new() -> Self { Self { stats: FlockV5BridgeStats { total_ops: 0, shared: 0, exclusive: 0, unlocks: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &FlockV5Record) {
        self.stats.total_ops += 1;
        match rec.lock_type {
            FlockV5Type::SharedRead => self.stats.shared += 1,
            FlockV5Type::ExclusiveWrite => self.stats.exclusive += 1,
            FlockV5Type::Unlock => self.stats.unlocks += 1,
            _ => {}
        }
    }
}
