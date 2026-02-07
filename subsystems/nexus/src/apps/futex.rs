//! # Application Futex/Synchronization Profiling
//!
//! Futex and synchronization primitive analysis:
//! - Lock contention profiling
//! - Wait chain analysis
//! - Mutex/rwlock/condvar tracking
//! - Priority inversion detection
//! - Deadlock cycle detection

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// SYNC PRIMITIVE TYPES
// ============================================================================

/// Synchronization primitive type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SyncPrimitiveType {
    /// Futex
    Futex,
    /// Mutex
    Mutex,
    /// Read-write lock
    RwLock,
    /// Condition variable
    CondVar,
    /// Semaphore
    Semaphore,
    /// Spinlock
    Spinlock,
    /// Barrier
    Barrier,
}

/// Lock state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockState {
    /// Free
    Free,
    /// Held by single owner
    Held,
    /// Read-locked (multiple readers)
    ReadLocked,
    /// Write-locked
    WriteLocked,
    /// Destroyed
    Destroyed,
}

// ============================================================================
// LOCK DESCRIPTOR
// ============================================================================

/// Lock instance descriptor
#[derive(Debug, Clone)]
pub struct LockDescriptor {
    /// Lock address
    pub address: u64,
    /// Primitive type
    pub prim_type: SyncPrimitiveType,
    /// State
    pub state: LockState,
    /// Current owner (thread ID)
    pub owner: Option<u64>,
    /// Reader count (for RwLock)
    pub reader_count: u32,
    /// Waiters
    pub waiters: Vec<u64>,
    /// Acquisition count
    pub acquisition_count: u64,
    /// Total hold time (ns)
    pub total_hold_ns: u64,
    /// Total wait time across all waiters (ns)
    pub total_wait_ns: u64,
    /// Max hold time (ns)
    pub max_hold_ns: u64,
    /// Max wait time (ns)
    pub max_wait_ns: u64,
    /// Contention count
    pub contention_count: u64,
}

impl LockDescriptor {
    pub fn new(address: u64, prim_type: SyncPrimitiveType) -> Self {
        Self {
            address,
            prim_type,
            state: LockState::Free,
            owner: None,
            reader_count: 0,
            waiters: Vec::new(),
            acquisition_count: 0,
            total_hold_ns: 0,
            total_wait_ns: 0,
            max_hold_ns: 0,
            max_wait_ns: 0,
            contention_count: 0,
        }
    }

    /// Acquire lock
    pub fn acquire(&mut self, thread: u64, wait_ns: u64) {
        self.owner = Some(thread);
        self.state = LockState::Held;
        self.acquisition_count += 1;
        self.total_wait_ns += wait_ns;
        if wait_ns > self.max_wait_ns {
            self.max_wait_ns = wait_ns;
        }
        if wait_ns > 0 {
            self.contention_count += 1;
        }
        self.waiters.retain(|&t| t != thread);
    }

    /// Release lock
    pub fn release(&mut self, hold_ns: u64) {
        self.owner = None;
        self.state = LockState::Free;
        self.total_hold_ns += hold_ns;
        if hold_ns > self.max_hold_ns {
            self.max_hold_ns = hold_ns;
        }
    }

    /// Add waiter
    pub fn add_waiter(&mut self, thread: u64) {
        if !self.waiters.contains(&thread) {
            self.waiters.push(thread);
        }
    }

    /// Average hold time
    pub fn avg_hold_ns(&self) -> f64 {
        if self.acquisition_count == 0 {
            return 0.0;
        }
        self.total_hold_ns as f64 / self.acquisition_count as f64
    }

    /// Average wait time
    pub fn avg_wait_ns(&self) -> f64 {
        if self.contention_count == 0 {
            return 0.0;
        }
        self.total_wait_ns as f64 / self.contention_count as f64
    }

    /// Contention rate
    pub fn contention_rate(&self) -> f64 {
        if self.acquisition_count == 0 {
            return 0.0;
        }
        self.contention_count as f64 / self.acquisition_count as f64
    }
}

// ============================================================================
// WAIT CHAIN
// ============================================================================

/// Wait chain entry
#[derive(Debug, Clone)]
pub struct WaitChainEntry {
    /// Waiting thread
    pub thread: u64,
    /// Waiting on lock address
    pub lock_address: u64,
    /// Lock owner
    pub owner: Option<u64>,
    /// Wait time so far (ns)
    pub wait_ns: u64,
}

/// Wait chain (potentially circular = deadlock)
#[derive(Debug, Clone)]
pub struct WaitChain {
    /// Chain entries
    pub entries: Vec<WaitChainEntry>,
    /// Is cyclic (deadlock)
    pub is_deadlock: bool,
    /// Chain length
    pub length: usize,
}

impl WaitChain {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            is_deadlock: false,
            length: 0,
        }
    }

    /// Add entry
    pub fn push(&mut self, entry: WaitChainEntry) {
        self.entries.push(entry);
        self.length = self.entries.len();
    }

    /// Check if this chain forms a deadlock
    pub fn check_cycle(&mut self) -> bool {
        if self.entries.len() < 2 {
            self.is_deadlock = false;
            return false;
        }
        let first = self.entries[0].thread;
        let last_owner = self.entries.last().and_then(|e| e.owner);
        self.is_deadlock = last_owner == Some(first);
        self.is_deadlock
    }
}

// ============================================================================
// PRIORITY INVERSION
// ============================================================================

/// Priority inversion event
#[derive(Debug, Clone)]
pub struct PriorityInversion {
    /// High priority thread (blocked)
    pub high_prio_thread: u64,
    /// High priority level
    pub high_prio: u32,
    /// Low priority thread (holding lock)
    pub low_prio_thread: u64,
    /// Low priority level
    pub low_prio: u32,
    /// Lock address
    pub lock_address: u64,
    /// Duration (ns)
    pub duration_ns: u64,
    /// Medium priority threads that preempted the low-priority holder
    pub interfering_threads: Vec<u64>,
}

// ============================================================================
// PROCESS SYNC PROFILE
// ============================================================================

/// Per-process synchronization profile
#[derive(Debug, Clone)]
pub struct ProcessSyncProfile {
    /// Process ID
    pub pid: u64,
    /// Locks owned or tracked
    pub locks: BTreeMap<u64, LockDescriptor>,
    /// Thread to lock held mapping
    pub thread_locks: BTreeMap<u64, Vec<u64>>,
    /// Thread to lock waiting mapping
    pub thread_waiting: BTreeMap<u64, u64>,
    /// Priority inversions detected
    pub inversions: Vec<PriorityInversion>,
    /// Total lock time (ns)
    pub total_lock_time_ns: u64,
    /// Total wait time (ns)
    pub total_wait_time_ns: u64,
}

impl ProcessSyncProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            locks: BTreeMap::new(),
            thread_locks: BTreeMap::new(),
            thread_waiting: BTreeMap::new(),
            inversions: Vec::new(),
            total_lock_time_ns: 0,
            total_wait_time_ns: 0,
        }
    }

    /// Register lock
    pub fn register_lock(&mut self, address: u64, prim_type: SyncPrimitiveType) {
        self.locks
            .entry(address)
            .or_insert_with(|| LockDescriptor::new(address, prim_type));
    }

    /// Acquire lock
    pub fn acquire(&mut self, thread: u64, address: u64, wait_ns: u64) {
        if let Some(lock) = self.locks.get_mut(&address) {
            lock.acquire(thread, wait_ns);
        }
        self.thread_locks
            .entry(thread)
            .or_insert_with(Vec::new)
            .push(address);
        self.thread_waiting.remove(&thread);
        self.total_wait_time_ns += wait_ns;
    }

    /// Release lock
    pub fn release(&mut self, thread: u64, address: u64, hold_ns: u64) {
        if let Some(lock) = self.locks.get_mut(&address) {
            lock.release(hold_ns);
        }
        if let Some(held) = self.thread_locks.get_mut(&thread) {
            held.retain(|&a| a != address);
        }
        self.total_lock_time_ns += hold_ns;
    }

    /// Thread starts waiting
    pub fn start_wait(&mut self, thread: u64, address: u64) {
        self.thread_waiting.insert(thread, address);
        if let Some(lock) = self.locks.get_mut(&address) {
            lock.add_waiter(thread);
        }
    }

    /// Build wait chains
    pub fn build_wait_chains(&self) -> Vec<WaitChain> {
        let mut chains = Vec::new();

        for (&thread, &lock_addr) in &self.thread_waiting {
            let mut chain = WaitChain::new();
            let mut visited = Vec::new();
            let mut current_thread = thread;
            let mut current_lock = lock_addr;

            loop {
                if visited.contains(&current_thread) {
                    chain.check_cycle();
                    break;
                }
                visited.push(current_thread);

                let owner = self
                    .locks
                    .get(&current_lock)
                    .and_then(|l| l.owner);

                chain.push(WaitChainEntry {
                    thread: current_thread,
                    lock_address: current_lock,
                    owner,
                    wait_ns: 0,
                });

                match owner {
                    Some(o) => {
                        if let Some(&next_lock) = self.thread_waiting.get(&o) {
                            current_thread = o;
                            current_lock = next_lock;
                        } else {
                            break;
                        }
                    }
                    None => break,
                }
            }

            if chain.length >= 2 {
                chains.push(chain);
            }
        }

        chains
    }

    /// Most contended locks
    pub fn most_contended(&self, limit: usize) -> Vec<&LockDescriptor> {
        let mut locks: Vec<_> = self.locks.values().collect();
        locks.sort_by(|a, b| {
            b.contention_count
                .cmp(&a.contention_count)
        });
        locks.truncate(limit);
        locks
    }
}

// ============================================================================
// FUTEX ANALYZER
// ============================================================================

/// Futex analyzer stats
#[derive(Debug, Clone, Default)]
pub struct AppFutexStats {
    /// Tracked processes
    pub process_count: usize,
    /// Total locks tracked
    pub total_locks: usize,
    /// Total contentions
    pub total_contentions: u64,
    /// Deadlocks detected
    pub deadlocks_detected: u64,
    /// Priority inversions
    pub inversions_detected: u64,
}

/// Application futex/sync analyzer
pub struct AppFutexAnalyzer {
    /// Per-process profiles
    profiles: BTreeMap<u64, ProcessSyncProfile>,
    /// Stats
    stats: AppFutexStats,
}

impl AppFutexAnalyzer {
    pub fn new() -> Self {
        Self {
            profiles: BTreeMap::new(),
            stats: AppFutexStats::default(),
        }
    }

    /// Register process
    pub fn register_process(&mut self, pid: u64) {
        self.profiles.insert(pid, ProcessSyncProfile::new(pid));
        self.stats.process_count = self.profiles.len();
    }

    /// Register lock
    pub fn register_lock(&mut self, pid: u64, address: u64, prim_type: SyncPrimitiveType) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.register_lock(address, prim_type);
            self.stats.total_locks = self
                .profiles
                .values()
                .map(|p| p.locks.len())
                .sum();
        }
    }

    /// Record acquire
    pub fn record_acquire(&mut self, pid: u64, thread: u64, address: u64, wait_ns: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.acquire(thread, address, wait_ns);
            if wait_ns > 0 {
                self.stats.total_contentions += 1;
            }
        }
    }

    /// Record release
    pub fn record_release(&mut self, pid: u64, thread: u64, address: u64, hold_ns: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.release(thread, address, hold_ns);
        }
    }

    /// Record wait start
    pub fn record_wait(&mut self, pid: u64, thread: u64, address: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.start_wait(thread, address);
        }
    }

    /// Detect deadlocks for a process
    pub fn detect_deadlocks(&mut self, pid: u64) -> Vec<WaitChain> {
        let chains = match self.profiles.get(&pid) {
            Some(profile) => profile.build_wait_chains(),
            None => return Vec::new(),
        };
        let deadlocks: Vec<_> = chains.into_iter().filter(|c| c.is_deadlock).collect();
        self.stats.deadlocks_detected += deadlocks.len() as u64;
        deadlocks
    }

    /// Get profile
    pub fn profile(&self, pid: u64) -> Option<&ProcessSyncProfile> {
        self.profiles.get(&pid)
    }

    /// Stats
    pub fn stats(&self) -> &AppFutexStats {
        &self.stats
    }
}
