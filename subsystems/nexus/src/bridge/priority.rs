//! # Bridge Prioritization Engine
//!
//! Syscall priority management and scheduling:
//! - Multi-level priority queues
//! - Priority inheritance for syscalls
//! - Deadline-aware scheduling
//! - Starvation prevention
//! - Priority boosting

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

// ============================================================================
// PRIORITY TYPES
// ============================================================================

/// Priority class
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SyscallPriority {
    /// Idle (lowest)
    Idle,
    /// Background
    Background,
    /// Normal
    Normal,
    /// Elevated
    Elevated,
    /// High
    High,
    /// Realtime
    Realtime,
    /// Critical (highest)
    Critical,
}

impl SyscallPriority {
    /// Numeric value
    #[inline]
    pub fn value(&self) -> u32 {
        match self {
            SyscallPriority::Idle => 0,
            SyscallPriority::Background => 10,
            SyscallPriority::Normal => 20,
            SyscallPriority::Elevated => 30,
            SyscallPriority::High => 40,
            SyscallPriority::Realtime => 50,
            SyscallPriority::Critical => 60,
        }
    }
}

/// Boost reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoostReason {
    /// Priority inheritance (holding lock)
    Inheritance,
    /// Starvation prevention
    Starvation,
    /// Deadline approaching
    Deadline,
    /// Interactive boost
    Interactive,
    /// System critical
    SystemCritical,
}

// ============================================================================
// PRIORITY REQUEST
// ============================================================================

/// A prioritized syscall request
#[derive(Debug, Clone)]
pub struct PriorityRequest {
    /// Request id
    pub id: u64,
    /// Process id
    pub pid: u64,
    /// Syscall number
    pub syscall_nr: u32,
    /// Base priority
    pub base_priority: SyscallPriority,
    /// Effective priority (after boosts)
    pub effective_priority: SyscallPriority,
    /// Deadline (absolute ns, 0 = none)
    pub deadline_ns: u64,
    /// Enqueue time
    pub enqueue_time: u64,
    /// Wait time (ns)
    pub wait_time_ns: u64,
    /// Boost reasons
    pub boosts: Vec<BoostReason>,
}

impl PriorityRequest {
    pub fn new(
        id: u64,
        pid: u64,
        syscall_nr: u32,
        priority: SyscallPriority,
        now: u64,
    ) -> Self {
        Self {
            id,
            pid,
            syscall_nr,
            base_priority: priority,
            effective_priority: priority,
            deadline_ns: 0,
            enqueue_time: now,
            wait_time_ns: 0,
            boosts: Vec::new(),
        }
    }

    /// Apply boost
    #[inline]
    pub fn boost(&mut self, reason: BoostReason, new_priority: SyscallPriority) {
        if new_priority > self.effective_priority {
            self.effective_priority = new_priority;
        }
        if !self.boosts.contains(&reason) {
            self.boosts.push(reason);
        }
    }

    /// Update wait time
    #[inline(always)]
    pub fn update_wait(&mut self, now: u64) {
        self.wait_time_ns = now.saturating_sub(self.enqueue_time);
    }

    /// Is past deadline?
    #[inline(always)]
    pub fn is_past_deadline(&self, now: u64) -> bool {
        self.deadline_ns > 0 && now > self.deadline_ns
    }

    /// Time to deadline (ns)
    #[inline]
    pub fn time_to_deadline(&self, now: u64) -> Option<u64> {
        if self.deadline_ns == 0 {
            return None;
        }
        if now >= self.deadline_ns {
            Some(0)
        } else {
            Some(self.deadline_ns - now)
        }
    }
}

// ============================================================================
// PRIORITY QUEUE
// ============================================================================

/// Multi-level priority queue
#[derive(Debug)]
#[repr(align(64))]
pub struct PriorityQueue {
    /// Queues per priority level
    levels: BTreeMap<u32, Vec<PriorityRequest>>,
    /// Total items
    pub total_items: usize,
    /// Max items per level
    pub max_per_level: usize,
}

impl PriorityQueue {
    pub fn new(max_per_level: usize) -> Self {
        Self {
            levels: BTreeMap::new(),
            total_items: 0,
            max_per_level,
        }
    }

    /// Enqueue
    #[inline]
    pub fn enqueue(&mut self, request: PriorityRequest) -> bool {
        let level = request.effective_priority.value();
        let queue = self.levels.entry(level).or_insert_with(Vec::new);
        if queue.len() >= self.max_per_level {
            return false;
        }
        queue.push(request);
        self.total_items += 1;
        true
    }

    /// Dequeue highest priority (FIFO within same priority)
    pub fn dequeue(&mut self) -> Option<PriorityRequest> {
        // BTreeMap iterates in ascending order, we want highest
        let highest_key = self.levels.keys().next_back().copied();
        if let Some(key) = highest_key {
            if let Some(queue) = self.levels.get_mut(&key) {
                if !queue.is_empty() {
                    let req = queue.pop_front().unwrap();
                    self.total_items -= 1;
                    if queue.is_empty() {
                        self.levels.remove(&key);
                    }
                    return Some(req);
                }
            }
        }
        None
    }

    /// Peek highest priority
    #[inline]
    pub fn peek(&self) -> Option<&PriorityRequest> {
        let highest_key = self.levels.keys().next_back().copied();
        if let Some(key) = highest_key {
            self.levels.get(&key).and_then(|q| q.first())
        } else {
            None
        }
    }

    /// Items at priority
    #[inline(always)]
    pub fn count_at(&self, priority: SyscallPriority) -> usize {
        self.levels.get(&priority.value()).map(|q| q.len()).unwrap_or(0)
    }

    /// Is empty?
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.total_items == 0
    }
}

// ============================================================================
// STARVATION DETECTOR
// ============================================================================

/// Starvation detector
#[derive(Debug)]
pub struct StarvationDetector {
    /// Max wait time before boost (ns)
    pub max_wait_ns: u64,
    /// Boost target priority
    pub boost_priority: SyscallPriority,
}

impl StarvationDetector {
    pub fn new(max_wait_ns: u64) -> Self {
        Self {
            max_wait_ns,
            boost_priority: SyscallPriority::High,
        }
    }

    /// Check for starved requests and boost them
    pub fn check(&self, queue: &mut PriorityQueue, now: u64) -> u32 {
        let mut boosted = 0u32;
        for (_level, requests) in queue.levels.iter_mut() {
            for req in requests.iter_mut() {
                req.update_wait(now);
                if req.wait_time_ns > self.max_wait_ns {
                    req.boost(BoostReason::Starvation, self.boost_priority);
                    boosted += 1;
                }
            }
        }
        boosted
    }
}

// ============================================================================
// PRIORITY ENGINE
// ============================================================================

/// Priority stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BridgePriorityStats {
    /// Queued items
    pub queued_items: usize,
    /// Total enqueued
    pub total_enqueued: u64,
    /// Total dequeued
    pub total_dequeued: u64,
    /// Total boosted
    pub total_boosted: u64,
    /// Starvation events
    pub starvation_events: u64,
}

/// Bridge priority engine
#[repr(align(64))]
pub struct BridgePriorityEngine {
    /// Priority queue
    queue: PriorityQueue,
    /// Starvation detector
    starvation: StarvationDetector,
    /// Process base priorities
    process_priorities: BTreeMap<u64, SyscallPriority>,
    /// Next request id
    next_id: u64,
    /// Stats
    stats: BridgePriorityStats,
}

impl BridgePriorityEngine {
    pub fn new(max_per_level: usize, starvation_ns: u64) -> Self {
        Self {
            queue: PriorityQueue::new(max_per_level),
            starvation: StarvationDetector::new(starvation_ns),
            process_priorities: BTreeMap::new(),
            next_id: 1,
            stats: BridgePriorityStats::default(),
        }
    }

    /// Set process priority
    #[inline(always)]
    pub fn set_priority(&mut self, pid: u64, priority: SyscallPriority) {
        self.process_priorities.insert(pid, priority);
    }

    /// Submit syscall request
    pub fn submit(
        &mut self,
        pid: u64,
        syscall_nr: u32,
        now: u64,
    ) -> Option<u64> {
        let priority = self.process_priorities.get(&pid).copied()
            .unwrap_or(SyscallPriority::Normal);
        let id = self.next_id;
        self.next_id += 1;
        let request = PriorityRequest::new(id, pid, syscall_nr, priority, now);
        if self.queue.enqueue(request) {
            self.stats.total_enqueued += 1;
            self.stats.queued_items = self.queue.total_items;
            Some(id)
        } else {
            None
        }
    }

    /// Dispatch next request
    #[inline]
    pub fn dispatch(&mut self) -> Option<PriorityRequest> {
        let req = self.queue.dequeue();
        if req.is_some() {
            self.stats.total_dequeued += 1;
            self.stats.queued_items = self.queue.total_items;
        }
        req
    }

    /// Run starvation check
    #[inline]
    pub fn check_starvation(&mut self, now: u64) {
        let boosted = self.starvation.check(&mut self.queue, now);
        self.stats.total_boosted += boosted as u64;
        if boosted > 0 {
            self.stats.starvation_events += 1;
        }
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &BridgePriorityStats {
        &self.stats
    }
}
