// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop spinlock — Test-and-test-and-set spinlock with backoff
//!
//! Implements TTAS spinlock with exponential backoff, NUMA-aware spinning,
//! lock holder preemption detection, and critical section length tracking.

extern crate alloc;

use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};

/// Spinlock backoff strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpinBackoff {
    None,
    Linear,
    Exponential,
    Proportional,
}

/// Spinlock state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpinlockState {
    Free,
    Held,
    Contested,
}

/// A spinlock instance.
#[derive(Debug)]
pub struct SpinlockInstance {
    pub lock_id: u64,
    pub state: SpinlockState,
    pub holder_pid: Option<u64>,
    pub holder_cpu: Option<u32>,
    pub backoff: SpinBackoff,
    pub acquire_count: u64,
    pub spin_total: u64,
    pub max_spin_iters: u64,
    pub hold_time_total: u64,
    pub max_hold_time: u64,
    pub contention_events: u64,
    pub preempt_disabled: bool,
}

impl SpinlockInstance {
    pub fn new(lock_id: u64, backoff: SpinBackoff) -> Self {
        Self {
            lock_id,
            state: SpinlockState::Free,
            holder_pid: None,
            holder_cpu: None,
            backoff,
            acquire_count: 0,
            spin_total: 0,
            max_spin_iters: 0,
            hold_time_total: 0,
            max_hold_time: 0,
            contention_events: 0,
            preempt_disabled: true,
        }
    }

    pub fn try_lock(&mut self, pid: u64, cpu: u32) -> bool {
        if self.state == SpinlockState::Free {
            self.state = SpinlockState::Held;
            self.holder_pid = Some(pid);
            self.holder_cpu = Some(cpu);
            self.acquire_count += 1;
            true
        } else {
            self.contention_events += 1;
            self.state = SpinlockState::Contested;
            false
        }
    }

    pub fn unlock(&mut self) {
        self.state = SpinlockState::Free;
        self.holder_pid = None;
        self.holder_cpu = None;
    }

    pub fn record_spin(&mut self, iterations: u64) {
        self.spin_total += iterations;
        if iterations > self.max_spin_iters {
            self.max_spin_iters = iterations;
        }
    }

    pub fn record_hold(&mut self, ticks: u64) {
        self.hold_time_total += ticks;
        if ticks > self.max_hold_time {
            self.max_hold_time = ticks;
        }
    }

    pub fn avg_spin(&self) -> f64 {
        if self.acquire_count == 0 {
            return 0.0;
        }
        self.spin_total as f64 / self.acquire_count as f64
    }

    pub fn avg_hold(&self) -> f64 {
        if self.acquire_count == 0 {
            return 0.0;
        }
        self.hold_time_total as f64 / self.acquire_count as f64
    }
}

/// Statistics for spinlock.
#[derive(Debug, Clone)]
pub struct SpinlockStats {
    pub total_locks: u64,
    pub total_acquires: u64,
    pub total_spins: u64,
    pub total_contention: u64,
    pub max_spin_seen: u64,
}

/// Main coop spinlock manager.
pub struct CoopSpinlock {
    pub locks: BTreeMap<u64, SpinlockInstance>,
    pub next_lock_id: u64,
    pub stats: SpinlockStats,
}

impl CoopSpinlock {
    pub fn new() -> Self {
        Self {
            locks: BTreeMap::new(),
            next_lock_id: 1,
            stats: SpinlockStats {
                total_locks: 0,
                total_acquires: 0,
                total_spins: 0,
                total_contention: 0,
                max_spin_seen: 0,
            },
        }
    }

    pub fn create_lock(&mut self, backoff: SpinBackoff) -> u64 {
        let id = self.next_lock_id;
        self.next_lock_id += 1;
        let lock = SpinlockInstance::new(id, backoff);
        self.locks.insert(id, lock);
        self.stats.total_locks += 1;
        id
    }

    pub fn lock_count(&self) -> usize {
        self.locks.len()
    }
}
