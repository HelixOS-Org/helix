// SPDX-License-Identifier: GPL-2.0
//! Coop semaphore_v2 — counting semaphore v2.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Semaphore type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemTypeV2 {
    Counting,
    Binary,
    Weighted,
    Priority,
}

/// Waiter entry
#[derive(Debug)]
pub struct SemWaiter {
    pub tid: u64,
    pub count: u32,
    pub priority: i32,
    pub wait_start: u64,
    pub granted: bool,
}

impl SemWaiter {
    pub fn new(tid: u64, count: u32, prio: i32, now: u64) -> Self {
        Self { tid, count, priority: prio, wait_start: now, granted: false }
    }
}

/// Semaphore instance
#[derive(Debug)]
pub struct SemaphoreV2 {
    pub id: u64,
    pub sem_type: SemTypeV2,
    pub value: i32,
    pub max_value: i32,
    pub waiters: Vec<SemWaiter>,
    pub total_acquires: u64,
    pub total_releases: u64,
    pub total_timeouts: u64,
    pub contention_count: u64,
}

impl SemaphoreV2 {
    pub fn new(id: u64, stype: SemTypeV2, initial: i32, max: i32) -> Self {
        Self { id, sem_type: stype, value: initial, max_value: max, waiters: Vec::new(), total_acquires: 0, total_releases: 0, total_timeouts: 0, contention_count: 0 }
    }

    pub fn try_acquire(&mut self, count: u32) -> bool {
        if self.value >= count as i32 {
            self.value -= count as i32;
            self.total_acquires += 1;
            true
        } else { self.contention_count += 1; false }
    }

    pub fn release(&mut self, count: u32) {
        self.value = (self.value + count as i32).min(self.max_value);
        self.total_releases += 1;
        self.wake_waiters();
    }

    fn wake_waiters(&mut self) {
        if self.sem_type == SemTypeV2::Priority {
            self.waiters.sort_by(|a, b| b.priority.cmp(&a.priority));
        }
        for w in &mut self.waiters {
            if !w.granted && self.value >= w.count as i32 {
                self.value -= w.count as i32;
                w.granted = true;
                self.total_acquires += 1;
            }
        }
        self.waiters.retain(|w| !w.granted);
    }

    pub fn pending(&self) -> u32 { self.waiters.len() as u32 }
}

/// Stats
#[derive(Debug, Clone)]
pub struct SemaphoreV2Stats {
    pub total_semaphores: u32,
    pub total_acquires: u64,
    pub total_releases: u64,
    pub total_contention: u64,
    pub pending_waiters: u32,
    pub avg_value: f64,
}

/// Main semaphore v2 manager
pub struct CoopSemaphoreV2 {
    semaphores: BTreeMap<u64, SemaphoreV2>,
    next_id: u64,
}

impl CoopSemaphoreV2 {
    pub fn new() -> Self { Self { semaphores: BTreeMap::new(), next_id: 1 } }

    pub fn create(&mut self, stype: SemTypeV2, initial: i32, max: i32) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.semaphores.insert(id, SemaphoreV2::new(id, stype, initial, max));
        id
    }

    pub fn acquire(&mut self, sem: u64, count: u32) -> bool {
        self.semaphores.get_mut(&sem).map(|s| s.try_acquire(count)).unwrap_or(false)
    }

    pub fn release(&mut self, sem: u64, count: u32) {
        if let Some(s) = self.semaphores.get_mut(&sem) { s.release(count); }
    }

    pub fn stats(&self) -> SemaphoreV2Stats {
        let acq: u64 = self.semaphores.values().map(|s| s.total_acquires).sum();
        let rel: u64 = self.semaphores.values().map(|s| s.total_releases).sum();
        let cont: u64 = self.semaphores.values().map(|s| s.contention_count).sum();
        let pend: u32 = self.semaphores.values().map(|s| s.pending()).sum();
        let vals: Vec<f64> = self.semaphores.values().map(|s| s.value as f64).collect();
        let avg = if vals.is_empty() { 0.0 } else { vals.iter().sum::<f64>() / vals.len() as f64 };
        SemaphoreV2Stats { total_semaphores: self.semaphores.len() as u32, total_acquires: acq, total_releases: rel, total_contention: cont, pending_waiters: pend, avg_value: avg }
    }
}

// ============================================================================
// Merged from semaphore_v3
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemaphoreV3State {
    Available,
    Depleted,
    Overcommitted,
}

/// Semaphore v3 waiter
#[derive(Debug)]
pub struct SemV3Waiter {
    pub tid: u64,
    pub count: u32,
    pub enqueue_time: u64,
}

/// Semaphore v3 instance
#[derive(Debug)]
pub struct SemaphoreV3 {
    pub id: u64,
    pub count: i64,
    pub max_count: u32,
    pub waiters: Vec<SemV3Waiter>,
    pub total_acquires: u64,
    pub total_releases: u64,
    pub total_wait_ns: u64,
}

impl SemaphoreV3 {
    pub fn new(id: u64, initial: u32) -> Self {
        Self { id, count: initial as i64, max_count: initial, waiters: Vec::new(), total_acquires: 0, total_releases: 0, total_wait_ns: 0 }
    }

    pub fn try_acquire(&mut self, n: u32) -> bool {
        if self.count >= n as i64 {
            self.count -= n as i64;
            self.total_acquires += 1;
            true
        } else { false }
    }

    pub fn enqueue_waiter(&mut self, tid: u64, n: u32, now: u64) {
        self.waiters.push(SemV3Waiter { tid, count: n, enqueue_time: now });
    }

    pub fn release(&mut self, n: u32, now: u64) -> Vec<u64> {
        self.count += n as i64;
        self.total_releases += 1;
        let mut woken = Vec::new();
        while !self.waiters.is_empty() && self.count >= self.waiters[0].count as i64 {
            let w = self.waiters.remove(0);
            self.count -= w.count as i64;
            self.total_wait_ns += now.saturating_sub(w.enqueue_time);
            woken.push(w.tid);
        }
        woken
    }

    pub fn state(&self) -> SemaphoreV3State {
        if self.count > 0 { SemaphoreV3State::Available }
        else if self.count == 0 { SemaphoreV3State::Depleted }
        else { SemaphoreV3State::Overcommitted }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct SemaphoreV3Stats {
    pub total_semaphores: u32,
    pub total_acquires: u64,
    pub total_releases: u64,
    pub total_waiters: u32,
}

/// Main coop semaphore v3
pub struct CoopSemaphoreV3 {
    sems: BTreeMap<u64, SemaphoreV3>,
    next_id: u64,
}

impl CoopSemaphoreV3 {
    pub fn new() -> Self { Self { sems: BTreeMap::new(), next_id: 1 } }

    pub fn create(&mut self, initial: u32) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.sems.insert(id, SemaphoreV3::new(id, initial));
        id
    }

    pub fn try_acquire(&mut self, id: u64, n: u32) -> bool {
        if let Some(s) = self.sems.get_mut(&id) { s.try_acquire(n) } else { false }
    }

    pub fn release(&mut self, id: u64, n: u32, now: u64) -> Vec<u64> {
        if let Some(s) = self.sems.get_mut(&id) { s.release(n, now) } else { Vec::new() }
    }

    pub fn destroy(&mut self, id: u64) { self.sems.remove(&id); }

    pub fn stats(&self) -> SemaphoreV3Stats {
        let acq: u64 = self.sems.values().map(|s| s.total_acquires).sum();
        let rel: u64 = self.sems.values().map(|s| s.total_releases).sum();
        let wait: u32 = self.sems.values().map(|s| s.waiters.len() as u32).sum();
        SemaphoreV3Stats { total_semaphores: self.sems.len() as u32, total_acquires: acq, total_releases: rel, total_waiters: wait }
    }
}

// ============================================================================
// Merged from semaphore_v4
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SemaphoreV4Priority {
    Low,
    Normal,
    High,
    Realtime,
    Critical,
}

/// Semaphore wait result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemaphoreV4Result {
    Acquired,
    WouldBlock,
    Timeout,
    Interrupted,
    Destroyed,
}

/// A semaphore waiter.
#[derive(Debug, Clone)]
pub struct SemaphoreV4Waiter {
    pub waiter_id: u64,
    pub pid: u64,
    pub weight: u32,
    pub priority: SemaphoreV4Priority,
    pub deadline_ns: Option<u64>,
    pub enqueue_time: u64,
}

impl SemaphoreV4Waiter {
    pub fn new(waiter_id: u64, pid: u64, weight: u32) -> Self {
        Self {
            waiter_id,
            pid,
            weight,
            priority: SemaphoreV4Priority::Normal,
            deadline_ns: None,
            enqueue_time: 0,
        }
    }
}

/// A semaphore V4 instance.
#[derive(Debug, Clone)]
pub struct SemaphoreV4Instance {
    pub sem_id: u64,
    pub count: i64,
    pub max_count: i64,
    pub waiters: Vec<SemaphoreV4Waiter>,
    pub total_acquires: u64,
    pub total_releases: u64,
    pub total_timeouts: u64,
    pub weighted_acquires: u64,
    pub priority_donations: u64,
    pub peak_waiters: u32,
}

impl SemaphoreV4Instance {
    pub fn new(sem_id: u64, initial: i64) -> Self {
        Self {
            sem_id,
            count: initial,
            max_count: initial,
            waiters: Vec::new(),
            total_acquires: 0,
            total_releases: 0,
            total_timeouts: 0,
            weighted_acquires: 0,
            priority_donations: 0,
            peak_waiters: 0,
        }
    }

    pub fn try_acquire(&mut self, weight: u32) -> SemaphoreV4Result {
        if self.count >= weight as i64 {
            self.count -= weight as i64;
            self.total_acquires += 1;
            if weight > 1 {
                self.weighted_acquires += 1;
            }
            SemaphoreV4Result::Acquired
        } else {
            SemaphoreV4Result::WouldBlock
        }
    }

    pub fn release(&mut self, weight: u32) {
        self.count += weight as i64;
        if self.count > self.max_count {
            self.count = self.max_count;
        }
        self.total_releases += 1;
        // Try to wake priority-sorted waiters
        self.waiters.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    pub fn enqueue_waiter(&mut self, waiter: SemaphoreV4Waiter) {
        self.waiters.push(waiter);
        let len = self.waiters.len() as u32;
        if len > self.peak_waiters {
            self.peak_waiters = len;
        }
    }

    pub fn drain_acquirable(&mut self) -> Vec<SemaphoreV4Waiter> {
        let mut acquired = Vec::new();
        self.waiters.sort_by(|a, b| b.priority.cmp(&a.priority));
        let mut remaining = Vec::new();
        for w in self.waiters.drain(..) {
            if self.count >= w.weight as i64 {
                self.count -= w.weight as i64;
                self.total_acquires += 1;
                acquired.push(w);
            } else {
                remaining.push(w);
            }
        }
        self.waiters = remaining;
        acquired
    }

    pub fn utilization(&self) -> f64 {
        if self.max_count == 0 {
            return 0.0;
        }
        1.0 - (self.count as f64 / self.max_count as f64)
    }
}

/// Statistics for semaphore V4.
#[derive(Debug, Clone)]
pub struct SemaphoreV4Stats {
    pub total_semaphores: u64,
    pub total_acquires: u64,
    pub total_releases: u64,
    pub total_timeouts: u64,
    pub weighted_ops: u64,
    pub priority_donations: u64,
}

/// Main coop semaphore V4 manager.
pub struct CoopSemaphoreV4 {
    pub semaphores: BTreeMap<u64, SemaphoreV4Instance>,
    pub next_sem_id: u64,
    pub stats: SemaphoreV4Stats,
}

impl CoopSemaphoreV4 {
    pub fn new() -> Self {
        Self {
            semaphores: BTreeMap::new(),
            next_sem_id: 1,
            stats: SemaphoreV4Stats {
                total_semaphores: 0,
                total_acquires: 0,
                total_releases: 0,
                total_timeouts: 0,
                weighted_ops: 0,
                priority_donations: 0,
            },
        }
    }

    pub fn create_semaphore(&mut self, initial: i64) -> u64 {
        let id = self.next_sem_id;
        self.next_sem_id += 1;
        let sem = SemaphoreV4Instance::new(id, initial);
        self.semaphores.insert(id, sem);
        self.stats.total_semaphores += 1;
        id
    }

    pub fn semaphore_count(&self) -> usize {
        self.semaphores.len()
    }
}
