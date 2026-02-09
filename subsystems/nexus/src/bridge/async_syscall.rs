//! # Bridge Async Syscall Engine
//!
//! Asynchronous syscall submission and completion:
//! - io_uring-style submission/completion queue model
//! - Per-thread SQ/CQ ring management
//! - Linked syscall chains
//! - Timeout handling
//! - Cancellation support
//! - Batched completion notification

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Async syscall state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncSyscallState {
    Queued,
    Submitted,
    InProgress,
    Completed,
    Cancelled,
    TimedOut,
}

/// Async syscall flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncSyscallFlag {
    /// Normal submission
    None,
    /// Link to next entry (chain)
    Link,
    /// Hard link (cancel chain on failure)
    HardLink,
    /// Drain: wait for previous completions
    Drain,
    /// Fixed file descriptor
    FixedFile,
}

/// Submission queue entry
#[derive(Debug, Clone)]
pub struct SqEntry {
    pub user_data: u64,
    pub syscall_nr: u32,
    pub args: [u64; 6],
    pub flags: AsyncSyscallFlag,
    pub timeout_ns: u64,
    pub submit_ts: u64,
}

impl SqEntry {
    pub fn new(user_data: u64, syscall_nr: u32) -> Self {
        Self {
            user_data,
            syscall_nr,
            args: [0; 6],
            flags: AsyncSyscallFlag::None,
            timeout_ns: 0,
            submit_ts: 0,
        }
    }
}

/// Completion queue entry
#[derive(Debug, Clone)]
pub struct CqEntry {
    pub user_data: u64,
    pub result: i64,
    pub flags: u32,
    pub complete_ts: u64,
}

/// Ring buffer for SQ/CQ
#[derive(Debug, Clone)]
pub struct Ring<T: Clone> {
    pub entries: VecDeque<T>,
    pub head: u32,
    pub tail: u32,
    pub capacity: u32,
}

impl<T: Clone> Ring<T> {
    pub fn new(capacity: u32) -> Self {
        Self {
            entries: VecDeque::new(),
            head: 0,
            tail: 0,
            capacity,
        }
    }

    #[inline(always)]
    pub fn len(&self) -> u32 {
        self.tail.wrapping_sub(self.head)
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.head == self.tail
    }

    #[inline(always)]
    pub fn is_full(&self) -> bool {
        self.len() >= self.capacity
    }

    #[inline]
    pub fn push(&mut self, entry: T) -> bool {
        if self.is_full() { return false; }
        self.entries.push_back(entry);
        self.tail = self.tail.wrapping_add(1);
        true
    }

    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() { return None; }
        if self.entries.is_empty() { return None; }
        let entry = self.entries.pop_front().unwrap();
        self.head = self.head.wrapping_add(1);
        Some(entry)
    }
}

/// Per-thread async context
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AsyncContext {
    pub thread_id: u64,
    pub sq: Ring<SqEntry>,
    pub cq: Ring<CqEntry>,
    pub in_flight: u32,
    pub total_submitted: u64,
    pub total_completed: u64,
    pub total_cancelled: u64,
    pub total_timed_out: u64,
}

impl AsyncContext {
    pub fn new(thread_id: u64, sq_size: u32, cq_size: u32) -> Self {
        Self {
            thread_id,
            sq: Ring::new(sq_size),
            cq: Ring::new(cq_size),
            in_flight: 0,
            total_submitted: 0,
            total_completed: 0,
            total_cancelled: 0,
            total_timed_out: 0,
        }
    }

    #[inline]
    pub fn submit(&mut self, entry: SqEntry) -> bool {
        if self.sq.push(entry) {
            self.total_submitted += 1;
            true
        } else { false }
    }

    pub fn complete(&mut self, user_data: u64, result: i64, now: u64) -> bool {
        let cqe = CqEntry {
            user_data,
            result,
            flags: 0,
            complete_ts: now,
        };
        if self.cq.push(cqe) {
            self.in_flight = self.in_flight.saturating_sub(1);
            self.total_completed += 1;
            true
        } else { false }
    }

    #[inline(always)]
    pub fn reap(&mut self) -> Option<CqEntry> {
        self.cq.pop()
    }
}

/// In-flight syscall tracking
#[derive(Debug, Clone)]
pub struct InFlightSyscall {
    pub user_data: u64,
    pub thread_id: u64,
    pub syscall_nr: u32,
    pub state: AsyncSyscallState,
    pub submit_ts: u64,
    pub deadline_ns: u64,
    pub linked_next: Option<u64>,
}

/// Async syscall engine stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BridgeAsyncSyscallStats {
    pub active_contexts: usize,
    pub total_in_flight: u32,
    pub total_submitted: u64,
    pub total_completed: u64,
    pub total_cancelled: u64,
    pub total_timed_out: u64,
    pub avg_latency_ns: f64,
}

/// Bridge Async Syscall Engine
#[repr(align(64))]
pub struct BridgeAsyncSyscall {
    contexts: BTreeMap<u64, AsyncContext>,
    in_flight: BTreeMap<u64, InFlightSyscall>,
    default_sq_size: u32,
    default_cq_size: u32,
    latency_sum_ns: u64,
    latency_count: u64,
    stats: BridgeAsyncSyscallStats,
}

impl BridgeAsyncSyscall {
    pub fn new(sq_size: u32, cq_size: u32) -> Self {
        Self {
            contexts: BTreeMap::new(),
            in_flight: BTreeMap::new(),
            default_sq_size: sq_size,
            default_cq_size: cq_size,
            latency_sum_ns: 0,
            latency_count: 0,
            stats: BridgeAsyncSyscallStats::default(),
        }
    }

    #[inline]
    pub fn create_context(&mut self, thread_id: u64) {
        self.contexts.insert(thread_id, AsyncContext::new(
            thread_id, self.default_sq_size, self.default_cq_size));
        self.recompute();
    }

    pub fn destroy_context(&mut self, thread_id: u64) {
        self.contexts.remove(&thread_id);
        // Cancel all in-flight for this thread
        let to_cancel: Vec<u64> = self.in_flight.iter()
            .filter(|(_, s)| s.thread_id == thread_id)
            .map(|(&ud, _)| ud)
            .collect();
        for ud in to_cancel {
            self.in_flight.remove(&ud);
        }
        self.recompute();
    }

    /// Submit a syscall
    pub fn submit(&mut self, thread_id: u64, entry: SqEntry, now: u64) -> bool {
        let user_data = entry.user_data;
        let syscall_nr = entry.syscall_nr;
        let deadline = if entry.timeout_ns > 0 { now + entry.timeout_ns } else { 0 };
        let linked = match entry.flags {
            AsyncSyscallFlag::Link | AsyncSyscallFlag::HardLink => Some(user_data + 1),
            _ => None,
        };

        let ctx = match self.contexts.get_mut(&thread_id) {
            Some(c) => c,
            None => return false,
        };

        if !ctx.submit(entry) { return false; }
        ctx.in_flight += 1;

        self.in_flight.insert(user_data, InFlightSyscall {
            user_data,
            thread_id,
            syscall_nr,
            state: AsyncSyscallState::Submitted,
            submit_ts: now,
            deadline_ns: deadline,
            linked_next: linked,
        });

        self.recompute();
        true
    }

    /// Complete a syscall
    pub fn complete(&mut self, user_data: u64, result: i64, now: u64) -> bool {
        let inflight = match self.in_flight.remove(&user_data) {
            Some(s) => s,
            None => return false,
        };

        let latency = now.saturating_sub(inflight.submit_ts);
        self.latency_sum_ns += latency;
        self.latency_count += 1;

        if let Some(ctx) = self.contexts.get_mut(&inflight.thread_id) {
            ctx.complete(user_data, result, now);
        }

        self.recompute();
        true
    }

    /// Cancel a syscall
    pub fn cancel(&mut self, user_data: u64, now: u64) -> bool {
        if let Some(mut inflight) = self.in_flight.remove(&user_data) {
            inflight.state = AsyncSyscallState::Cancelled;
            if let Some(ctx) = self.contexts.get_mut(&inflight.thread_id) {
                ctx.total_cancelled += 1;
                ctx.in_flight = ctx.in_flight.saturating_sub(1);
                ctx.complete(user_data, -1, now);
            }
            self.recompute();
            true
        } else { false }
    }

    /// Check for timeouts
    pub fn check_timeouts(&mut self, now: u64) -> Vec<u64> {
        let timed_out: Vec<u64> = self.in_flight.iter()
            .filter(|(_, s)| s.deadline_ns > 0 && now > s.deadline_ns)
            .map(|(&ud, _)| ud)
            .collect();

        for &ud in &timed_out {
            if let Some(mut inflight) = self.in_flight.remove(&ud) {
                inflight.state = AsyncSyscallState::TimedOut;
                if let Some(ctx) = self.contexts.get_mut(&inflight.thread_id) {
                    ctx.total_timed_out += 1;
                    ctx.in_flight = ctx.in_flight.saturating_sub(1);
                    ctx.complete(ud, -110, now); // -ETIMEDOUT
                }
            }
        }

        if !timed_out.is_empty() { self.recompute(); }
        timed_out
    }

    fn recompute(&mut self) {
        let total_in_flight: u32 = self.contexts.values().map(|c| c.in_flight).sum();
        let total_sub: u64 = self.contexts.values().map(|c| c.total_submitted).sum();
        let total_comp: u64 = self.contexts.values().map(|c| c.total_completed).sum();
        let total_cancel: u64 = self.contexts.values().map(|c| c.total_cancelled).sum();
        let total_timeout: u64 = self.contexts.values().map(|c| c.total_timed_out).sum();
        let avg_lat = if self.latency_count > 0 {
            self.latency_sum_ns as f64 / self.latency_count as f64
        } else { 0.0 };

        self.stats = BridgeAsyncSyscallStats {
            active_contexts: self.contexts.len(),
            total_in_flight,
            total_submitted: total_sub,
            total_completed: total_comp,
            total_cancelled: total_cancel,
            total_timed_out: total_timeout,
            avg_latency_ns: avg_lat,
        };
    }

    #[inline(always)]
    pub fn stats(&self) -> &BridgeAsyncSyscallStats {
        &self.stats
    }
}
