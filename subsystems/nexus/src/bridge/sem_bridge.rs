//! # Bridge SEM Bridge
//!
//! System V semaphore (semget/semop/semctl) bridging:
//! - Semaphore set creation and management
//! - Atomic semaphore operations (wait/signal/zero-wait)
//! - Undo tracking for process cleanup
//! - Deadlock-aware blocking semantics
//! - Per-semaphore statistics and contention tracking
//! - IPC namespace isolation

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Semaphore permission
#[derive(Debug, Clone, Copy)]
pub struct SemPerm {
    pub uid: u32,
    pub gid: u32,
    pub cuid: u32,
    pub cgid: u32,
    pub mode: u16,
}

impl SemPerm {
    pub fn new(uid: u32, gid: u32, mode: u16) -> Self {
        Self { uid, gid, cuid: uid, cgid: gid, mode }
    }
}

/// Single semaphore value with tracking
#[derive(Debug, Clone)]
pub struct Semaphore {
    pub value: i32,
    pub last_pid: u64,
    pub wait_count: u32,
    pub total_ops: u64,
    pub peak_value: i32,
    pub zero_wait_count: u32,
}

impl Semaphore {
    pub fn new() -> Self {
        Self { value: 0, last_pid: 0, wait_count: 0, total_ops: 0, peak_value: 0, zero_wait_count: 0 }
    }

    #[inline]
    pub fn set_value(&mut self, val: i32, pid: u64) {
        self.value = val;
        self.last_pid = pid;
        if val > self.peak_value { self.peak_value = val; }
    }
}

/// Semaphore operation
#[derive(Debug, Clone, Copy)]
pub struct SemOp {
    pub sem_num: u16,
    pub sem_op: i16,
    pub sem_flg: u16,
}

impl SemOp {
    pub const IPC_NOWAIT: u16 = 0x800;
    pub const SEM_UNDO: u16 = 0x1000;

    pub fn new(num: u16, op: i16, flg: u16) -> Self {
        Self { sem_num: num, sem_op: op, sem_flg: flg }
    }

    #[inline(always)]
    pub fn is_nowait(&self) -> bool { self.sem_flg & Self::IPC_NOWAIT != 0 }
    #[inline(always)]
    pub fn has_undo(&self) -> bool { self.sem_flg & Self::SEM_UNDO != 0 }
    #[inline(always)]
    pub fn is_wait(&self) -> bool { self.sem_op < 0 }
    #[inline(always)]
    pub fn is_signal(&self) -> bool { self.sem_op > 0 }
    #[inline(always)]
    pub fn is_zero_wait(&self) -> bool { self.sem_op == 0 }
}

/// Undo entry for a process
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SemUndo {
    pub pid: u64,
    pub adjustments: BTreeMap<u16, i16>,
}

impl SemUndo {
    pub fn new(pid: u64) -> Self {
        Self { pid, adjustments: BTreeMap::new() }
    }

    #[inline(always)]
    pub fn record(&mut self, sem_num: u16, delta: i16) {
        let entry = self.adjustments.entry(sem_num).or_insert(0);
        *entry = entry.wrapping_add(-delta);
    }
}

/// Pending wait entry
#[derive(Debug, Clone)]
pub struct SemWaiter {
    pub pid: u64,
    pub sem_num: u16,
    pub op: i16,
    pub enqueue_ts: u64,
}

/// Semaphore set
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SemaphoreSet {
    pub sem_id: u32,
    pub key: i32,
    pub perm: SemPerm,
    pub sems: Vec<Semaphore>,
    pub nsems: u16,
    pub undos: BTreeMap<u64, SemUndo>,
    pub waiters: Vec<SemWaiter>,
    pub creator_pid: u64,
    pub change_time: u64,
    pub op_time: u64,
    pub total_ops: u64,
    pub ns_id: u64,
}

impl SemaphoreSet {
    pub fn new(id: u32, key: i32, perm: SemPerm, nsems: u16, pid: u64, ts: u64) -> Self {
        let mut sems = Vec::new();
        for _ in 0..nsems {
            sems.push(Semaphore::new());
        }
        Self {
            sem_id: id, key, perm, sems, nsems, undos: BTreeMap::new(),
            waiters: Vec::new(), creator_pid: pid, change_time: ts,
            op_time: 0, total_ops: 0, ns_id: 0,
        }
    }

    /// Try to apply a single operation
    pub fn try_op(&mut self, op: &SemOp, pid: u64, ts: u64) -> Result<(), bool> {
        let idx = op.sem_num as usize;
        if idx >= self.sems.len() { return Err(true); } // invalid

        let sem = &self.sems[idx];
        if op.is_zero_wait() {
            if sem.value != 0 {
                return if op.is_nowait() { Err(true) } else { Err(false) }; // would block
            }
        } else if op.is_wait() {
            let needed = (-op.sem_op) as i32;
            if sem.value < needed {
                return if op.is_nowait() { Err(true) } else { Err(false) };
            }
        }

        // Apply
        let sem = &mut self.sems[idx];
        sem.value += op.sem_op as i32;
        sem.last_pid = pid;
        sem.total_ops += 1;
        if sem.value > sem.peak_value { sem.peak_value = sem.value; }

        if op.has_undo() {
            let undo = self.undos.entry(pid).or_insert_with(|| SemUndo::new(pid));
            undo.record(op.sem_num, op.sem_op);
        }

        self.op_time = ts;
        self.total_ops += 1;
        Ok(())
    }

    /// Try to apply a batch of operations atomically
    pub fn try_ops(&mut self, ops: &[SemOp], pid: u64, ts: u64) -> Result<(), bool> {
        // Check all ops first
        for op in ops {
            let idx = op.sem_num as usize;
            if idx >= self.sems.len() { return Err(true); }
            let sem = &self.sems[idx];
            if op.is_zero_wait() && sem.value != 0 {
                return if op.is_nowait() { Err(true) } else { Err(false) };
            }
            if op.is_wait() {
                let needed = (-op.sem_op) as i32;
                if sem.value < needed {
                    return if op.is_nowait() { Err(true) } else { Err(false) };
                }
            }
        }
        // Apply all
        for op in ops {
            let _ = self.try_op(op, pid, ts);
        }
        Ok(())
    }

    /// Apply undo adjustments when process exits
    pub fn apply_undo(&mut self, pid: u64, ts: u64) {
        if let Some(undo) = self.undos.remove(&pid) {
            for (&sem_num, &adj) in &undo.adjustments {
                let idx = sem_num as usize;
                if idx < self.sems.len() {
                    self.sems[idx].value += adj as i32;
                    self.sems[idx].last_pid = pid;
                }
            }
            self.op_time = ts;
        }
        self.waiters.retain(|w| w.pid != pid);
    }

    #[inline(always)]
    pub fn total_waiters(&self) -> usize { self.waiters.len() }
    #[inline]
    pub fn contention_score(&self) -> f64 {
        let total_waits: u32 = self.sems.iter().map(|s| s.wait_count).sum();
        if self.total_ops == 0 { return 0.0; }
        total_waits as f64 / self.total_ops as f64
    }
}

/// SEM bridge stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct SemBridgeStats {
    pub total_sets: usize,
    pub total_semaphores: usize,
    pub total_waiters: usize,
    pub total_undo_entries: usize,
    pub total_ops: u64,
    pub peak_sets: usize,
    pub high_contention_sets: usize,
}

/// Bridge semaphore manager
#[repr(align(64))]
pub struct BridgeSemBridge {
    sets: BTreeMap<u32, SemaphoreSet>,
    key_to_id: BTreeMap<i32, u32>,
    next_id: u32,
    stats: SemBridgeStats,
}

impl BridgeSemBridge {
    pub fn new() -> Self {
        Self {
            sets: BTreeMap::new(), key_to_id: BTreeMap::new(),
            next_id: 1, stats: SemBridgeStats::default(),
        }
    }

    pub fn semget(&mut self, key: i32, nsems: u16, uid: u32, gid: u32, mode: u16, pid: u64, ts: u64) -> u32 {
        if key != 0 {
            if let Some(&existing) = self.key_to_id.get(&key) {
                return existing;
            }
        }
        let id = self.next_id;
        self.next_id += 1;
        let set = SemaphoreSet::new(id, key, SemPerm::new(uid, gid, mode), nsems, pid, ts);
        self.sets.insert(id, set);
        if key != 0 { self.key_to_id.insert(key, id); }
        id
    }

    #[inline]
    pub fn semop(&mut self, sem_id: u32, ops: &[SemOp], pid: u64, ts: u64) -> Result<(), bool> {
        if let Some(set) = self.sets.get_mut(&sem_id) {
            set.try_ops(ops, pid, ts)
        } else { Err(true) }
    }

    #[inline]
    pub fn semctl_rmid(&mut self, sem_id: u32) -> bool {
        if let Some(set) = self.sets.get(&sem_id) {
            let key = set.key;
            self.sets.remove(&sem_id);
            if key != 0 { self.key_to_id.remove(&key); }
            true
        } else { false }
    }

    #[inline]
    pub fn semctl_setval(&mut self, sem_id: u32, sem_num: u16, val: i32, pid: u64) {
        if let Some(set) = self.sets.get_mut(&sem_id) {
            let idx = sem_num as usize;
            if idx < set.sems.len() {
                set.sems[idx].set_value(val, pid);
            }
        }
    }

    #[inline]
    pub fn process_exit(&mut self, pid: u64, ts: u64) {
        let ids: Vec<u32> = self.sets.keys().copied().collect();
        for id in ids {
            if let Some(set) = self.sets.get_mut(&id) {
                set.apply_undo(pid, ts);
            }
        }
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_sets = self.sets.len();
        self.stats.total_semaphores = self.sets.values().map(|s| s.nsems as usize).sum();
        self.stats.total_waiters = self.sets.values().map(|s| s.waiters.len()).sum();
        self.stats.total_undo_entries = self.sets.values().map(|s| s.undos.len()).sum();
        self.stats.total_ops = self.sets.values().map(|s| s.total_ops).sum();
        if self.stats.total_sets > self.stats.peak_sets { self.stats.peak_sets = self.stats.total_sets; }
        self.stats.high_contention_sets = self.sets.values().filter(|s| s.contention_score() > 0.5).count();
    }

    #[inline(always)]
    pub fn set(&self, id: u32) -> Option<&SemaphoreSet> { self.sets.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &SemBridgeStats { &self.stats }
}
