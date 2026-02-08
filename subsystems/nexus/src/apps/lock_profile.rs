//! # Application Lock Contention Profiler
//!
//! Per-process lock contention analysis:
//! - Mutex/spinlock/rwlock contention tracking
//! - Lock hold time analysis
//! - Deadlock potential detection
//! - Lock ordering violation alerts
//! - Priority inversion tracking
//! - Lock convoy detection

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Lock type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockType {
    Mutex,
    Spinlock,
    RwLockRead,
    RwLockWrite,
    Semaphore,
    Futex,
    Condvar,
}

/// Contention severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentionSeverity {
    None,
    Low,
    Moderate,
    High,
    Critical,
}

/// Lock acquisition state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockAcqState {
    Acquired,
    Waiting,
    Spinning,
    TimedOut,
    Failed,
}

/// Single lock instance stats
#[derive(Debug, Clone)]
pub struct LockProfile {
    pub lock_id: u64,
    pub lock_type: LockType,
    pub lock_addr: u64,
    pub acquisitions: u64,
    pub contentions: u64,
    pub total_hold_ns: u64,
    pub total_wait_ns: u64,
    pub max_hold_ns: u64,
    pub max_wait_ns: u64,
    pub spin_count: u64,
    pub current_holder: Option<u64>,
    pub waiters: Vec<u64>,
    pub order_id: u32,
}

impl LockProfile {
    pub fn new(lock_id: u64, lock_type: LockType, addr: u64) -> Self {
        Self {
            lock_id,
            lock_type,
            lock_addr: addr,
            acquisitions: 0,
            contentions: 0,
            total_hold_ns: 0,
            total_wait_ns: 0,
            max_hold_ns: 0,
            max_wait_ns: 0,
            spin_count: 0,
            current_holder: None,
            waiters: Vec::new(),
            order_id: 0,
        }
    }

    pub fn contention_rate(&self) -> f64 {
        if self.acquisitions == 0 { return 0.0; }
        self.contentions as f64 / self.acquisitions as f64
    }

    pub fn avg_hold_ns(&self) -> u64 {
        if self.acquisitions == 0 { return 0; }
        self.total_hold_ns / self.acquisitions
    }

    pub fn avg_wait_ns(&self) -> u64 {
        if self.contentions == 0 { return 0; }
        self.total_wait_ns / self.contentions
    }

    pub fn severity(&self) -> ContentionSeverity {
        let rate = self.contention_rate();
        if rate < 0.01 { ContentionSeverity::None }
        else if rate < 0.1 { ContentionSeverity::Low }
        else if rate < 0.3 { ContentionSeverity::Moderate }
        else if rate < 0.6 { ContentionSeverity::High }
        else { ContentionSeverity::Critical }
    }

    pub fn acquire(&mut self, thread_id: u64, wait_ns: u64) {
        self.acquisitions += 1;
        if wait_ns > 0 {
            self.contentions += 1;
            self.total_wait_ns += wait_ns;
            if wait_ns > self.max_wait_ns { self.max_wait_ns = wait_ns; }
        }
        self.current_holder = Some(thread_id);
        self.waiters.retain(|&t| t != thread_id);
    }

    pub fn release(&mut self, hold_ns: u64) {
        self.total_hold_ns += hold_ns;
        if hold_ns > self.max_hold_ns { self.max_hold_ns = hold_ns; }
        self.current_holder = None;
    }

    pub fn add_waiter(&mut self, thread_id: u64) {
        if !self.waiters.contains(&thread_id) {
            self.waiters.push(thread_id);
        }
    }
}

/// Lock ordering edge for deadlock detection
#[derive(Debug, Clone)]
pub struct LockOrderEdge {
    pub from_lock: u64,
    pub to_lock: u64,
    pub thread_id: u64,
    pub count: u64,
}

/// Priority inversion event
#[derive(Debug, Clone)]
pub struct PriorityInversion {
    pub lock_id: u64,
    pub high_prio_thread: u64,
    pub low_prio_holder: u64,
    pub high_priority: i32,
    pub low_priority: i32,
    pub wait_ns: u64,
    pub timestamp: u64,
}

/// Lock convoy detection
#[derive(Debug, Clone)]
pub struct LockConvoy {
    pub lock_id: u64,
    pub convoy_threads: Vec<u64>,
    pub avg_wait_ns: u64,
    pub duration_ns: u64,
    pub detected_at: u64,
}

/// Per-process lock contention profile
#[derive(Debug, Clone)]
pub struct ProcessLockProfile {
    pub pid: u64,
    pub locks: BTreeMap<u64, LockProfile>,
    pub order_edges: Vec<LockOrderEdge>,
    pub inversions: Vec<PriorityInversion>,
    pub convoys: Vec<LockConvoy>,
    pub total_contention_ns: u64,
    pub deadlock_risk: bool,
}

impl ProcessLockProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            locks: BTreeMap::new(),
            order_edges: Vec::new(),
            inversions: Vec::new(),
            convoys: Vec::new(),
            total_contention_ns: 0,
            deadlock_risk: false,
        }
    }

    pub fn register_lock(&mut self, lock_id: u64, lock_type: LockType, addr: u64) {
        self.locks.entry(lock_id).or_insert_with(|| LockProfile::new(lock_id, lock_type, addr));
    }

    pub fn hottest_lock(&self) -> Option<u64> {
        self.locks.values()
            .max_by_key(|l| l.contentions)
            .map(|l| l.lock_id)
    }

    pub fn critical_locks(&self) -> Vec<u64> {
        self.locks.values()
            .filter(|l| l.severity() == ContentionSeverity::High || l.severity() == ContentionSeverity::Critical)
            .map(|l| l.lock_id)
            .collect()
    }

    /// Simple cycle detection in lock order graph (DFS)
    pub fn detect_deadlock_potential(&mut self) {
        // Build adjacency list
        let mut adj: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
        for edge in &self.order_edges {
            adj.entry(edge.from_lock).or_insert_with(Vec::new).push(edge.to_lock);
        }

        // DFS cycle detection
        let mut visited = Vec::new();
        let mut stack = Vec::new();
        self.deadlock_risk = false;

        let keys: Vec<u64> = adj.keys().copied().collect();
        for start in &keys {
            stack.clear();
            visited.clear();
            stack.push(*start);
            while let Some(node) = stack.pop() {
                if visited.contains(&node) {
                    self.deadlock_risk = true;
                    return;
                }
                visited.push(node);
                if let Some(neighbors) = adj.get(&node) {
                    for &n in neighbors {
                        stack.push(n);
                    }
                }
            }
        }
    }
}

/// App lock contention profiler stats
#[derive(Debug, Clone, Default)]
pub struct AppLockProfilerStats {
    pub total_processes: usize,
    pub total_locks_tracked: usize,
    pub total_contentions: u64,
    pub total_inversions: usize,
    pub deadlock_risk_processes: usize,
    pub critical_contention_locks: usize,
}

/// Application Lock Contention Profiler
pub struct AppLockProfiler {
    profiles: BTreeMap<u64, ProcessLockProfile>,
    stats: AppLockProfilerStats,
}

impl AppLockProfiler {
    pub fn new() -> Self {
        Self {
            profiles: BTreeMap::new(),
            stats: AppLockProfilerStats::default(),
        }
    }

    pub fn register_process(&mut self, pid: u64) {
        self.profiles.entry(pid).or_insert_with(|| ProcessLockProfile::new(pid));
    }

    pub fn register_lock(&mut self, pid: u64, lock_id: u64, lock_type: LockType, addr: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.register_lock(lock_id, lock_type, addr);
        }
        self.recompute();
    }

    pub fn record_acquire(&mut self, pid: u64, lock_id: u64, thread_id: u64, wait_ns: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            if let Some(lock) = profile.locks.get_mut(&lock_id) {
                lock.acquire(thread_id, wait_ns);
                profile.total_contention_ns += wait_ns;
            }
        }
    }

    pub fn record_release(&mut self, pid: u64, lock_id: u64, hold_ns: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            if let Some(lock) = profile.locks.get_mut(&lock_id) {
                lock.release(hold_ns);
            }
        }
    }

    pub fn record_order(&mut self, pid: u64, from_lock: u64, to_lock: u64, thread_id: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            if let Some(edge) = profile.order_edges.iter_mut()
                .find(|e| e.from_lock == from_lock && e.to_lock == to_lock) {
                edge.count += 1;
            } else {
                profile.order_edges.push(LockOrderEdge {
                    from_lock, to_lock, thread_id, count: 1,
                });
            }
            profile.detect_deadlock_potential();
        }
        self.recompute();
    }

    pub fn record_inversion(&mut self, pid: u64, inversion: PriorityInversion) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.inversions.push(inversion);
            if profile.inversions.len() > 128 {
                profile.inversions.remove(0);
            }
        }
        self.recompute();
    }

    fn recompute(&mut self) {
        self.stats.total_processes = self.profiles.len();
        self.stats.total_locks_tracked = self.profiles.values().map(|p| p.locks.len()).sum();
        self.stats.total_contentions = self.profiles.values()
            .flat_map(|p| p.locks.values())
            .map(|l| l.contentions)
            .sum();
        self.stats.total_inversions = self.profiles.values().map(|p| p.inversions.len()).sum();
        self.stats.deadlock_risk_processes = self.profiles.values().filter(|p| p.deadlock_risk).count();
        self.stats.critical_contention_locks = self.profiles.values()
            .flat_map(|p| p.locks.values())
            .filter(|l| l.severity() == ContentionSeverity::Critical)
            .count();
    }

    pub fn profile(&self, pid: u64) -> Option<&ProcessLockProfile> {
        self.profiles.get(&pid)
    }

    pub fn stats(&self) -> &AppLockProfilerStats {
        &self.stats
    }

    pub fn remove_process(&mut self, pid: u64) {
        self.profiles.remove(&pid);
        self.recompute();
    }
}
