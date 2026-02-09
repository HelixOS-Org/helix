//! # Bridge Wait Queue
//!
//! Kernel wait queue management for blocking syscalls:
//! - Per-waitqueue sleeping thread tracking
//! - Priority-ordered wakeup
//! - Exclusive vs shared wakeup modes
//! - Thundering herd prevention
//! - Wait timeout management
//! - Wait queue statistics

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Wait queue type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitQueueType {
    /// FIFO ordering
    Fifo,
    /// Priority ordering
    Priority,
    /// Exclusive wakeup (one waiter at a time)
    Exclusive,
}

/// Wait entry state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitState {
    Waiting,
    WokenUp,
    TimedOut,
    Interrupted,
}

/// Waiter entry
#[derive(Debug, Clone)]
pub struct WaitEntry {
    pub thread_id: u64,
    pub pid: u64,
    pub priority: i32,
    pub state: WaitState,
    pub enqueue_ts: u64,
    pub timeout_ns: u64,
    pub exclusive: bool,
    pub wait_reason: u32, // hash of reason
}

impl WaitEntry {
    pub fn new(thread_id: u64, pid: u64, priority: i32) -> Self {
        Self {
            thread_id,
            pid,
            priority,
            state: WaitState::Waiting,
            enqueue_ts: 0,
            timeout_ns: 0,
            exclusive: false,
            wait_reason: 0,
        }
    }

    #[inline(always)]
    pub fn is_expired(&self, now: u64) -> bool {
        self.timeout_ns > 0 && now > self.enqueue_ts + self.timeout_ns
    }

    #[inline(always)]
    pub fn wait_duration(&self, now: u64) -> u64 {
        now.saturating_sub(self.enqueue_ts)
    }
}

/// Wait queue
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct WaitQueue {
    pub queue_id: u64,
    pub queue_type: WaitQueueType,
    pub waiters: Vec<WaitEntry>,
    pub total_enqueues: u64,
    pub total_wakeups: u64,
    pub total_timeouts: u64,
    pub max_waiters: u32,
    pub max_wait_ns: u64,
}

impl WaitQueue {
    pub fn new(queue_id: u64, queue_type: WaitQueueType) -> Self {
        Self {
            queue_id,
            queue_type,
            waiters: Vec::new(),
            total_enqueues: 0,
            total_wakeups: 0,
            total_timeouts: 0,
            max_waiters: 0,
            max_wait_ns: 0,
        }
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.waiters.iter().all(|w| w.state != WaitState::Waiting)
    }

    #[inline(always)]
    pub fn waiting_count(&self) -> usize {
        self.waiters.iter().filter(|w| w.state == WaitState::Waiting).count()
    }

    /// Add a waiter
    pub fn enqueue(&mut self, mut entry: WaitEntry, now: u64) {
        entry.enqueue_ts = now;
        entry.state = WaitState::Waiting;
        self.total_enqueues += 1;

        // Insert sorted by priority for priority queues
        if self.queue_type == WaitQueueType::Priority {
            let pos = self.waiters.iter()
                .position(|w| w.priority < entry.priority)
                .unwrap_or(self.waiters.len());
            self.waiters.insert(pos, entry);
        } else {
            self.waiters.push(entry);
        }

        let wc = self.waiting_count() as u32;
        if wc > self.max_waiters {
            self.max_waiters = wc;
        }
    }

    /// Wake up waiters
    pub fn wakeup(&mut self, max: usize, now: u64) -> Vec<u64> {
        let mut woken = Vec::new();
        let mut count = 0;

        for waiter in &mut self.waiters {
            if count >= max { break; }
            if waiter.state != WaitState::Waiting { continue; }

            waiter.state = WaitState::WokenUp;
            woken.push(waiter.thread_id);
            self.total_wakeups += 1;
            count += 1;

            let wait_time = waiter.wait_duration(now);
            if wait_time > self.max_wait_ns {
                self.max_wait_ns = wait_time;
            }

            if self.queue_type == WaitQueueType::Exclusive {
                break; // Only wake one for exclusive
            }
        }

        // Clean up woken entries
        self.waiters.retain(|w| w.state == WaitState::Waiting);
        woken
    }

    /// Wake all waiters
    #[inline(always)]
    pub fn wakeup_all(&mut self, now: u64) -> Vec<u64> {
        self.wakeup(usize::MAX, now)
    }

    /// Check for timeouts
    pub fn check_timeouts(&mut self, now: u64) -> Vec<u64> {
        let mut timed_out = Vec::new();

        for waiter in &mut self.waiters {
            if waiter.state == WaitState::Waiting && waiter.is_expired(now) {
                waiter.state = WaitState::TimedOut;
                timed_out.push(waiter.thread_id);
                self.total_timeouts += 1;
            }
        }

        self.waiters.retain(|w| w.state == WaitState::Waiting);
        timed_out
    }

    /// Remove a specific waiter (e.g., signal interrupt)
    #[inline]
    pub fn remove_waiter(&mut self, thread_id: u64) -> bool {
        if let Some(pos) = self.waiters.iter().position(|w| w.thread_id == thread_id) {
            self.waiters[pos].state = WaitState::Interrupted;
            self.waiters.remove(pos);
            true
        } else { false }
    }
}

/// Wait queue manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BridgeWaitQueueStats {
    pub total_queues: usize,
    pub total_waiting: usize,
    pub total_enqueues: u64,
    pub total_wakeups: u64,
    pub total_timeouts: u64,
    pub max_queue_depth: u32,
    pub busiest_queue: u64,
}

/// Bridge Wait Queue Manager
#[repr(align(64))]
pub struct BridgeWaitQueueMgr {
    queues: BTreeMap<u64, WaitQueue>,
    next_queue_id: u64,
    stats: BridgeWaitQueueStats,
}

impl BridgeWaitQueueMgr {
    pub fn new() -> Self {
        Self {
            queues: BTreeMap::new(),
            next_queue_id: 1,
            stats: BridgeWaitQueueStats::default(),
        }
    }

    #[inline]
    pub fn create_queue(&mut self, queue_type: WaitQueueType) -> u64 {
        let id = self.next_queue_id;
        self.next_queue_id += 1;
        self.queues.insert(id, WaitQueue::new(id, queue_type));
        self.recompute();
        id
    }

    #[inline]
    pub fn destroy_queue(&mut self, queue_id: u64) -> Vec<u64> {
        if let Some(mut queue) = self.queues.remove(&queue_id) {
            let woken = queue.wakeup_all(0);
            self.recompute();
            woken
        } else { Vec::new() }
    }

    #[inline]
    pub fn enqueue(&mut self, queue_id: u64, entry: WaitEntry, now: u64) {
        if let Some(queue) = self.queues.get_mut(&queue_id) {
            queue.enqueue(entry, now);
        }
        self.recompute();
    }

    #[inline]
    pub fn wakeup(&mut self, queue_id: u64, max: usize, now: u64) -> Vec<u64> {
        let result = if let Some(queue) = self.queues.get_mut(&queue_id) {
            queue.wakeup(max, now)
        } else { Vec::new() };
        self.recompute();
        result
    }

    #[inline(always)]
    pub fn wakeup_all(&mut self, queue_id: u64, now: u64) -> Vec<u64> {
        self.wakeup(queue_id, usize::MAX, now)
    }

    /// Global timeout check across all queues
    pub fn check_all_timeouts(&mut self, now: u64) -> Vec<(u64, u64)> {
        let mut all_timeouts = Vec::new();
        let ids: Vec<u64> = self.queues.keys().copied().collect();
        for qid in ids {
            if let Some(queue) = self.queues.get_mut(&qid) {
                let timed_out = queue.check_timeouts(now);
                for tid in timed_out {
                    all_timeouts.push((qid, tid));
                }
            }
        }
        if !all_timeouts.is_empty() { self.recompute(); }
        all_timeouts
    }

    fn recompute(&mut self) {
        self.stats.total_queues = self.queues.len();
        self.stats.total_waiting = self.queues.values().map(|q| q.waiting_count()).sum();
        self.stats.total_enqueues = self.queues.values().map(|q| q.total_enqueues).sum();
        self.stats.total_wakeups = self.queues.values().map(|q| q.total_wakeups).sum();
        self.stats.total_timeouts = self.queues.values().map(|q| q.total_timeouts).sum();

        let (busiest, max_depth) = self.queues.values()
            .map(|q| (q.queue_id, q.waiting_count() as u32))
            .max_by_key(|&(_, depth)| depth)
            .unwrap_or((0, 0));

        self.stats.max_queue_depth = max_depth;
        self.stats.busiest_queue = busiest;
    }

    #[inline(always)]
    pub fn stats(&self) -> &BridgeWaitQueueStats {
        &self.stats
    }

    #[inline(always)]
    pub fn queue(&self, id: u64) -> Option<&WaitQueue> {
        self.queues.get(&id)
    }
}
