//! Deadlock Detector
//!
//! Detects and prevents deadlocks.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::{LockId, ThreadId};
use crate::core::NexusTimestamp;

/// Deadlock information
#[derive(Debug, Clone)]
pub struct DeadlockInfo {
    /// Threads involved
    pub threads: Vec<ThreadId>,
    /// Locks involved
    pub locks: Vec<LockId>,
    /// Detection timestamp
    pub detected_at: NexusTimestamp,
    /// Cycle description
    pub cycle: Vec<(ThreadId, LockId)>,
}

/// Near miss (potential deadlock)
#[derive(Debug, Clone)]
pub struct NearMiss {
    /// Thread 1
    pub thread1: ThreadId,
    /// Thread 2
    pub thread2: ThreadId,
    /// Lock 1
    pub lock1: LockId,
    /// Lock 2
    pub lock2: LockId,
    /// Timestamp
    pub timestamp: NexusTimestamp,
}

/// Detects potential deadlocks
pub struct DeadlockDetector {
    /// Lock graph edges (thread -> locks held)
    held: BTreeMap<ThreadId, Vec<LockId>>,
    /// Waiting on (thread -> lock waiting for)
    waiting: BTreeMap<ThreadId, LockId>,
    /// Lock holders (lock -> holder thread)
    holders: BTreeMap<LockId, ThreadId>,
    /// Detected deadlocks
    deadlocks: Vec<DeadlockInfo>,
    /// Near misses
    near_misses: Vec<NearMiss>,
}

impl DeadlockDetector {
    /// Create new detector
    pub fn new() -> Self {
        Self {
            held: BTreeMap::new(),
            waiting: BTreeMap::new(),
            holders: BTreeMap::new(),
            deadlocks: Vec::new(),
            near_misses: Vec::new(),
        }
    }

    /// Record lock acquired
    pub fn lock_acquired(&mut self, thread: ThreadId, lock: LockId) {
        self.held.entry(thread).or_default().push(lock);
        self.holders.insert(lock, thread);
        self.waiting.remove(&thread);
    }

    /// Record lock released
    pub fn lock_released(&mut self, thread: ThreadId, lock: LockId) {
        if let Some(held) = self.held.get_mut(&thread) {
            held.retain(|&l| l != lock);
        }
        self.holders.remove(&lock);
    }

    /// Record waiting
    pub fn waiting_for(&mut self, thread: ThreadId, lock: LockId) -> Option<DeadlockInfo> {
        self.waiting.insert(thread, lock);
        self.detect_cycle(thread)
    }

    /// Detect cycle from starting thread
    fn detect_cycle(&self, start: ThreadId) -> Option<DeadlockInfo> {
        let mut visited = Vec::new();
        let mut current = start;
        let mut cycle = Vec::new();

        loop {
            if visited.contains(&current) {
                // Found cycle
                let cycle_start = visited.iter().position(|&t| t == current)?;
                let threads: Vec<_> = visited[cycle_start..].to_vec();
                let locks: Vec<_> = cycle[cycle_start..].iter().map(|(_, l)| *l).collect();

                return Some(DeadlockInfo {
                    threads: threads.clone(),
                    locks,
                    detected_at: NexusTimestamp::now(),
                    cycle: cycle[cycle_start..].to_vec(),
                });
            }

            visited.push(current);

            // What lock is current thread waiting for?
            let waiting_lock = self.waiting.get(&current)?;
            cycle.push((current, *waiting_lock));

            // Who holds that lock?
            let holder = self.holders.get(waiting_lock)?;
            current = *holder;
        }
    }

    /// Check for potential deadlock (AB-BA pattern)
    pub fn check_potential(&mut self, thread: ThreadId, lock: LockId) -> bool {
        let my_locks = match self.held.get(&thread) {
            Some(l) => l,
            None => return false,
        };

        // For each other thread
        for (&other_thread, other_locks) in &self.held {
            if other_thread == thread {
                continue;
            }

            // Check if they hold what we want and want what we hold
            if other_locks.contains(&lock) {
                for &my_lock in my_locks {
                    if self.waiting.get(&other_thread) == Some(&my_lock) {
                        self.near_misses.push(NearMiss {
                            thread1: thread,
                            thread2: other_thread,
                            lock1: lock,
                            lock2: my_lock,
                            timestamp: NexusTimestamp::now(),
                        });
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Get detected deadlocks
    pub fn deadlocks(&self) -> &[DeadlockInfo] {
        &self.deadlocks
    }

    /// Get near misses
    pub fn near_misses(&self) -> &[NearMiss] {
        &self.near_misses
    }

    /// Clear
    pub fn clear(&mut self) {
        self.held.clear();
        self.waiting.clear();
        self.holders.clear();
    }
}

impl Default for DeadlockDetector {
    fn default() -> Self {
        Self::new()
    }
}
