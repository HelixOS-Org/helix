// SPDX-License-Identifier: GPL-2.0
//! Coop ticket_lock — FIFO ticket-based spin lock.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Ticket lock
#[derive(Debug)]
pub struct TicketLock {
    pub id: u64,
    pub next_ticket: u64,
    pub now_serving: u64,
    pub owner_tid: Option<u64>,
    pub acquisitions: u64,
    pub contentions: u64,
    pub total_spin_ns: u64,
    pub max_spin_ns: u64,
}

impl TicketLock {
    pub fn new(id: u64) -> Self {
        Self { id, next_ticket: 0, now_serving: 0, owner_tid: None, acquisitions: 0, contentions: 0, total_spin_ns: 0, max_spin_ns: 0 }
    }

    pub fn take_ticket(&mut self) -> u64 {
        let ticket = self.next_ticket;
        self.next_ticket += 1;
        if ticket != self.now_serving { self.contentions += 1; }
        ticket
    }

    pub fn try_lock(&mut self, ticket: u64, tid: u64) -> bool {
        if ticket == self.now_serving {
            self.owner_tid = Some(tid);
            self.acquisitions += 1;
            true
        } else { false }
    }

    pub fn unlock(&mut self) {
        self.owner_tid = None;
        self.now_serving += 1;
    }

    pub fn waiters(&self) -> u64 { self.next_ticket - self.now_serving - if self.owner_tid.is_some() { 1 } else { 0 } }
    pub fn contention_rate(&self) -> f64 { if self.acquisitions == 0 { 0.0 } else { self.contentions as f64 / self.acquisitions as f64 } }
}

/// Stats
#[derive(Debug, Clone)]
pub struct TicketLockStats {
    pub total_locks: u32,
    pub total_acquisitions: u64,
    pub total_contentions: u64,
    pub total_waiters: u64,
    pub avg_contention_rate: f64,
}

/// Main ticket lock manager
pub struct CoopTicketLock {
    locks: BTreeMap<u64, TicketLock>,
    next_id: u64,
}

impl CoopTicketLock {
    pub fn new() -> Self { Self { locks: BTreeMap::new(), next_id: 1 } }

    pub fn create(&mut self) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.locks.insert(id, TicketLock::new(id));
        id
    }

    pub fn stats(&self) -> TicketLockStats {
        let acqs: u64 = self.locks.values().map(|l| l.acquisitions).sum();
        let conts: u64 = self.locks.values().map(|l| l.contentions).sum();
        let waiters: u64 = self.locks.values().map(|l| l.waiters()).sum();
        let rates: Vec<f64> = self.locks.values().map(|l| l.contention_rate()).collect();
        let avg = if rates.is_empty() { 0.0 } else { rates.iter().sum::<f64>() / rates.len() as f64 };
        TicketLockStats { total_locks: self.locks.len() as u32, total_acquisitions: acqs, total_contentions: conts, total_waiters: waiters, avg_contention_rate: avg }
    }
}

// ============================================================================
// Merged from ticket_lock_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketLockV2State {
    Free,
    Held,
    Contended,
}

/// Ticket lock v2
#[derive(Debug)]
pub struct TicketLockV2 {
    pub id: u64,
    pub next_ticket: u64,
    pub now_serving: u64,
    pub state: TicketLockV2State,
    pub owner_tid: u64,
    pub total_acquires: u64,
    pub total_releases: u64,
    pub total_spins: u64,
    pub max_wait_spins: u64,
    pub hold_time_total: u64,
}

impl TicketLockV2 {
    pub fn new(id: u64) -> Self {
        Self { id, next_ticket: 0, now_serving: 0, state: TicketLockV2State::Free, owner_tid: 0, total_acquires: 0, total_releases: 0, total_spins: 0, max_wait_spins: 0, hold_time_total: 0 }
    }

    pub fn acquire(&mut self, tid: u64, spins: u64) -> u64 {
        let ticket = self.next_ticket;
        self.next_ticket += 1;
        self.total_acquires += 1;
        self.total_spins += spins;
        if spins > self.max_wait_spins { self.max_wait_spins = spins; }
        self.now_serving = ticket + 1;
        self.owner_tid = tid;
        self.state = if self.next_ticket > self.now_serving { TicketLockV2State::Contended } else { TicketLockV2State::Held };
        ticket
    }

    pub fn release(&mut self, hold_ns: u64) {
        self.total_releases += 1;
        self.hold_time_total += hold_ns;
        self.owner_tid = 0;
        if self.now_serving >= self.next_ticket { self.state = TicketLockV2State::Free; }
        else { self.state = TicketLockV2State::Contended; }
    }

    pub fn avg_hold_ns(&self) -> u64 {
        if self.total_releases == 0 { 0 } else { self.hold_time_total / self.total_releases }
    }

    pub fn avg_spins(&self) -> f64 {
        if self.total_acquires == 0 { 0.0 } else { self.total_spins as f64 / self.total_acquires as f64 }
    }

    pub fn waiters(&self) -> u64 {
        if self.next_ticket > self.now_serving { self.next_ticket - self.now_serving }
        else { 0 }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct TicketLockV2Stats {
    pub total_locks: u32,
    pub total_acquires: u64,
    pub total_spins: u64,
    pub avg_spins: f64,
    pub contended: u32,
}

/// Main coop ticket lock v2 manager
pub struct CoopTicketLockV2 {
    locks: BTreeMap<u64, TicketLockV2>,
    next_id: u64,
}

impl CoopTicketLockV2 {
    pub fn new() -> Self { Self { locks: BTreeMap::new(), next_id: 1 } }

    pub fn create(&mut self) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.locks.insert(id, TicketLockV2::new(id));
        id
    }

    pub fn acquire(&mut self, lock_id: u64, tid: u64, spins: u64) -> u64 {
        if let Some(l) = self.locks.get_mut(&lock_id) { l.acquire(tid, spins) }
        else { 0 }
    }

    pub fn release(&mut self, lock_id: u64, hold_ns: u64) {
        if let Some(l) = self.locks.get_mut(&lock_id) { l.release(hold_ns); }
    }

    pub fn destroy(&mut self, lock_id: u64) { self.locks.remove(&lock_id); }

    pub fn stats(&self) -> TicketLockV2Stats {
        let acqs: u64 = self.locks.values().map(|l| l.total_acquires).sum();
        let spins: u64 = self.locks.values().map(|l| l.total_spins).sum();
        let avg = if acqs == 0 { 0.0 } else { spins as f64 / acqs as f64 };
        let contended = self.locks.values().filter(|l| l.state == TicketLockV2State::Contended).count() as u32;
        TicketLockV2Stats { total_locks: self.locks.len() as u32, total_acquires: acqs, total_spins: spins, avg_spins: avg, contended }
    }
}

// ============================================================================
// Merged from ticket_lock_v3
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketV3State {
    Free,
    Acquired,
    Contended,
}

/// A ticket lock V3 instance
#[derive(Debug)]
pub struct TicketLockV3Instance {
    pub id: u64,
    pub next_ticket: AtomicU64,
    pub now_serving: AtomicU64,
    pub holder_tid: Option<u64>,
    pub acquisitions: u64,
    pub contentions: u64,
    pub max_waiters: u64,
    pub total_wait_ticks: u64,
}

impl TicketLockV3Instance {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            next_ticket: AtomicU64::new(0),
            now_serving: AtomicU64::new(0),
            holder_tid: None,
            acquisitions: 0, contentions: 0,
            max_waiters: 0, total_wait_ticks: 0,
        }
    }

    pub fn acquire(&mut self, tid: u64) -> u64 {
        let ticket = self.next_ticket.fetch_add(1, Ordering::Relaxed);
        let serving = self.now_serving.load(Ordering::Acquire);
        if ticket != serving {
            self.contentions += 1;
            let waiters = ticket - serving;
            if waiters > self.max_waiters {
                self.max_waiters = waiters;
            }
        }
        self.holder_tid = Some(tid);
        self.acquisitions += 1;
        ticket
    }

    pub fn release(&mut self) {
        self.holder_tid = None;
        self.now_serving.fetch_add(1, Ordering::Release);
    }

    pub fn is_locked(&self) -> bool {
        self.next_ticket.load(Ordering::Relaxed) != self.now_serving.load(Ordering::Relaxed)
    }

    pub fn waiter_count(&self) -> u64 {
        let next = self.next_ticket.load(Ordering::Relaxed);
        let serving = self.now_serving.load(Ordering::Relaxed);
        if next > serving { next - serving - 1 } else { 0 }
    }
}

/// Statistics for ticket lock V3
#[derive(Debug, Clone)]
pub struct TicketLockV3Stats {
    pub locks_created: u64,
    pub total_acquisitions: u64,
    pub total_contentions: u64,
    pub max_queue_depth: u64,
    pub total_wait_ticks: u64,
}

/// Main ticket lock V3 coop manager
#[derive(Debug)]
pub struct CoopTicketLockV3 {
    locks: BTreeMap<u64, TicketLockV3Instance>,
    next_id: u64,
    stats: TicketLockV3Stats,
}

impl CoopTicketLockV3 {
    pub fn new() -> Self {
        Self {
            locks: BTreeMap::new(),
            next_id: 1,
            stats: TicketLockV3Stats {
                locks_created: 0, total_acquisitions: 0,
                total_contentions: 0, max_queue_depth: 0,
                total_wait_ticks: 0,
            },
        }
    }

    pub fn create(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.locks.insert(id, TicketLockV3Instance::new(id));
        self.stats.locks_created += 1;
        id
    }

    pub fn acquire(&mut self, lock_id: u64, tid: u64) -> Option<u64> {
        if let Some(lock) = self.locks.get_mut(&lock_id) {
            let ticket = lock.acquire(tid);
            self.stats.total_acquisitions += 1;
            if lock.contentions > 0 {
                self.stats.total_contentions = lock.contentions;
            }
            if lock.max_waiters > self.stats.max_queue_depth {
                self.stats.max_queue_depth = lock.max_waiters;
            }
            Some(ticket)
        } else { None }
    }

    pub fn release(&mut self, lock_id: u64) {
        if let Some(lock) = self.locks.get_mut(&lock_id) {
            lock.release();
        }
    }

    pub fn stats(&self) -> &TicketLockV3Stats {
        &self.stats
    }
}

// ============================================================================
// Merged from ticket_lock_v4
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketV4BackoffMode {
    None,
    Constant,
    Proportional,
    NumaAware,
}

/// Ticket lock state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketV4State {
    Free,
    Held,
    Contested,
}

/// A ticket lock V4 instance.
#[derive(Debug)]
pub struct TicketV4Instance {
    pub lock_id: u64,
    pub head: AtomicU64,
    pub tail: AtomicU64,
    pub state: TicketV4State,
    pub backoff_mode: TicketV4BackoffMode,
    pub holder_cpu: Option<u32>,
    pub total_acquires: u64,
    pub total_spins: u64,
    pub max_queue_depth: u64,
    pub handoff_count: u64,
    pub numa_local_acquires: u64,
    pub numa_remote_acquires: u64,
}

impl TicketV4Instance {
    pub fn new(lock_id: u64, backoff_mode: TicketV4BackoffMode) -> Self {
        Self {
            lock_id,
            head: AtomicU64::new(0),
            tail: AtomicU64::new(0),
            state: TicketV4State::Free,
            backoff_mode,
            holder_cpu: None,
            total_acquires: 0,
            total_spins: 0,
            max_queue_depth: 0,
            handoff_count: 0,
            numa_local_acquires: 0,
            numa_remote_acquires: 0,
        }
    }

    pub fn acquire(&mut self, cpu: u32, numa_node: u32) -> u64 {
        let ticket = self.tail.fetch_add(1, Ordering::AcqRel);
        let head = self.head.load(Ordering::Acquire);
        let depth = ticket.saturating_sub(head);
        if depth > self.max_queue_depth {
            self.max_queue_depth = depth;
        }
        if depth > 0 {
            self.state = TicketV4State::Contested;
            let spins = match self.backoff_mode {
                TicketV4BackoffMode::None => depth,
                TicketV4BackoffMode::Constant => depth * 10,
                TicketV4BackoffMode::Proportional => depth * depth,
                TicketV4BackoffMode::NumaAware => depth * 50,
            };
            self.total_spins += spins;
        }
        self.state = TicketV4State::Held;
        self.holder_cpu = Some(cpu);
        self.total_acquires += 1;
        // NUMA tracking
        if let Some(holder) = self.holder_cpu {
            if (holder / 8) == (cpu / 8) {
                self.numa_local_acquires += 1;
            } else {
                self.numa_remote_acquires += 1;
            }
        }
        ticket
    }

    pub fn release(&mut self) {
        self.head.fetch_add(1, Ordering::AcqRel);
        self.state = TicketV4State::Free;
        self.holder_cpu = None;
        self.handoff_count += 1;
    }

    pub fn queue_depth(&self) -> u64 {
        let tail = self.tail.load(Ordering::Acquire);
        let head = self.head.load(Ordering::Acquire);
        tail.saturating_sub(head)
    }

    pub fn numa_locality_rate(&self) -> f64 {
        let total = self.numa_local_acquires + self.numa_remote_acquires;
        if total == 0 {
            return 1.0;
        }
        self.numa_local_acquires as f64 / total as f64
    }
}

/// Statistics for ticket lock V4.
#[derive(Debug, Clone)]
pub struct TicketV4Stats {
    pub total_locks: u64,
    pub total_acquires: u64,
    pub total_spins: u64,
    pub max_queue_depth: u64,
    pub numa_local_pct: f64,
}

/// Main coop ticket lock V4 manager.
pub struct CoopTicketLockV4 {
    pub locks: BTreeMap<u64, TicketV4Instance>,
    pub next_lock_id: u64,
    pub stats: TicketV4Stats,
}

impl CoopTicketLockV4 {
    pub fn new() -> Self {
        Self {
            locks: BTreeMap::new(),
            next_lock_id: 1,
            stats: TicketV4Stats {
                total_locks: 0,
                total_acquires: 0,
                total_spins: 0,
                max_queue_depth: 0,
                numa_local_pct: 1.0,
            },
        }
    }

    pub fn create_lock(&mut self, backoff: TicketV4BackoffMode) -> u64 {
        let id = self.next_lock_id;
        self.next_lock_id += 1;
        let lock = TicketV4Instance::new(id, backoff);
        self.locks.insert(id, lock);
        self.stats.total_locks += 1;
        id
    }

    pub fn lock_count(&self) -> usize {
        self.locks.len()
    }
}
