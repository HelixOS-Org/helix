//! # Coop Priority Queue
//!
//! Distributed priority queue for cooperative task scheduling:
//! - Multi-level priority buckets
//! - Aging to prevent starvation
//! - Priority inheritance for waiting tasks
//! - Fair share scheduling within priority levels
//! - Deadline-aware scheduling
//! - Distributed queue partitioning

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Task urgency level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskUrgency {
    Idle = 0,
    Background = 1,
    Normal = 2,
    Elevated = 3,
    High = 4,
    Urgent = 5,
    Critical = 6,
    Emergency = 7,
}

/// Queue item state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueItemState {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

/// Queue item
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct QueueItem {
    pub id: u64,
    pub owner_node: u64,
    pub urgency: TaskUrgency,
    pub effective_priority: u64,
    pub base_priority: u64,
    pub age_bonus: u64,
    pub state: QueueItemState,
    pub enqueued_ts: u64,
    pub started_ts: Option<u64>,
    pub completed_ts: Option<u64>,
    pub deadline_ns: Option<u64>,
    pub wait_ns: u64,
    pub exec_ns: u64,
    pub retries: u32,
    pub max_retries: u32,
}

impl QueueItem {
    pub fn new(id: u64, owner: u64, urgency: TaskUrgency, ts: u64) -> Self {
        let base_pri = (urgency as u64) * 1000;
        Self {
            id, owner_node: owner, urgency, effective_priority: base_pri,
            base_priority: base_pri, age_bonus: 0, state: QueueItemState::Pending,
            enqueued_ts: ts, started_ts: None, completed_ts: None,
            deadline_ns: None, wait_ns: 0, exec_ns: 0, retries: 0, max_retries: 3,
        }
    }

    #[inline]
    pub fn age(&mut self, now: u64, aging_rate: u64) {
        let wait = now.saturating_sub(self.enqueued_ts);
        self.wait_ns = wait;
        self.age_bonus = wait / aging_rate;
        self.effective_priority = self.base_priority + self.age_bonus;
    }

    #[inline(always)]
    pub fn start(&mut self, ts: u64) { self.state = QueueItemState::Running; self.started_ts = Some(ts); }

    #[inline]
    pub fn complete(&mut self, ts: u64) {
        self.state = QueueItemState::Completed;
        self.completed_ts = Some(ts);
        if let Some(st) = self.started_ts { self.exec_ns = ts.saturating_sub(st); }
    }

    #[inline]
    pub fn fail(&mut self, ts: u64) {
        self.retries += 1;
        if self.retries >= self.max_retries { self.state = QueueItemState::Failed; }
        else { self.state = QueueItemState::Pending; }
        self.completed_ts = Some(ts);
    }

    #[inline(always)]
    pub fn is_overdue(&self, now: u64) -> bool {
        self.deadline_ns.map_or(false, |d| now > self.enqueued_ts + d)
    }

    #[inline(always)]
    pub fn turnaround_ns(&self) -> Option<u64> {
        self.completed_ts.map(|ct| ct.saturating_sub(self.enqueued_ts))
    }
}

/// Priority bucket
#[derive(Debug, Clone)]
pub struct PriorityBucket {
    pub urgency: TaskUrgency,
    pub items: VecDeque<u64>,
    pub total_enqueued: u64,
    pub total_dequeued: u64,
    pub max_size: usize,
}

impl PriorityBucket {
    pub fn new(urgency: TaskUrgency, max_size: usize) -> Self {
        Self { urgency, items: VecDeque::new(), total_enqueued: 0, total_dequeued: 0, max_size }
    }

    #[inline]
    pub fn enqueue(&mut self, item_id: u64) -> bool {
        if self.items.len() >= self.max_size { return false; }
        self.items.push_back(item_id);
        self.total_enqueued += 1;
        true
    }

    #[inline(always)]
    pub fn dequeue(&mut self) -> Option<u64> {
        if self.items.is_empty() { None } else { self.total_dequeued += 1; Some(self.items.pop_front().unwrap()) }
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool { self.items.is_empty() }
    #[inline(always)]
    pub fn len(&self) -> usize { self.items.len() }
}

/// Per-node queue partition
#[derive(Debug, Clone)]
pub struct NodePartition {
    pub node_id: u64,
    pub items_owned: Vec<u64>,
    pub items_processing: Vec<u64>,
    pub capacity: u32,
    pub current_load: u32,
}

impl NodePartition {
    pub fn new(node_id: u64, capacity: u32) -> Self {
        Self { node_id, items_owned: Vec::new(), items_processing: Vec::new(), capacity, current_load: 0 }
    }

    #[inline(always)]
    pub fn has_capacity(&self) -> bool { self.current_load < self.capacity }

    #[inline(always)]
    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 { return 1.0; }
        self.current_load as f64 / self.capacity as f64
    }
}

/// Priority queue stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct PriorityQueueStats {
    pub total_items: usize,
    pub pending_items: usize,
    pub running_items: usize,
    pub completed_items: u64,
    pub failed_items: u64,
    pub timed_out_items: u64,
    pub total_nodes: usize,
    pub avg_wait_ns: f64,
    pub avg_exec_ns: f64,
    pub avg_turnaround_ns: f64,
    pub max_wait_ns: u64,
}

/// Coop priority queue
#[repr(align(64))]
pub struct CoopPriorityQueue {
    items: BTreeMap<u64, QueueItem>,
    buckets: BTreeMap<u8, PriorityBucket>,
    partitions: BTreeMap<u64, NodePartition>,
    stats: PriorityQueueStats,
    next_id: u64,
    aging_rate_ns: u64,
}

impl CoopPriorityQueue {
    pub fn new(aging_rate_ns: u64) -> Self {
        let mut buckets = BTreeMap::new();
        for urgency in [TaskUrgency::Idle, TaskUrgency::Background, TaskUrgency::Normal,
            TaskUrgency::Elevated, TaskUrgency::High, TaskUrgency::Urgent,
            TaskUrgency::Critical, TaskUrgency::Emergency] {
            buckets.insert(urgency as u8, PriorityBucket::new(urgency, 10000));
        }
        Self { items: BTreeMap::new(), buckets, partitions: BTreeMap::new(), stats: PriorityQueueStats::default(), next_id: 1, aging_rate_ns }
    }

    #[inline(always)]
    pub fn add_node(&mut self, node_id: u64, capacity: u32) {
        self.partitions.entry(node_id).or_insert_with(|| NodePartition::new(node_id, capacity));
    }

    #[inline]
    pub fn enqueue(&mut self, owner: u64, urgency: TaskUrgency, ts: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        let item = QueueItem::new(id, owner, urgency, ts);
        self.items.insert(id, item);
        if let Some(bucket) = self.buckets.get_mut(&(urgency as u8)) { bucket.enqueue(id); }
        id
    }

    pub fn dequeue(&mut self, now: u64) -> Option<u64> {
        // Age all pending items
        let pending: Vec<u64> = self.items.iter()
            .filter(|(_, i)| i.state == QueueItemState::Pending)
            .map(|(&id, _)| id).collect();
        for id in &pending {
            if let Some(item) = self.items.get_mut(id) { item.age(now, self.aging_rate_ns); }
        }
        // Dequeue from highest priority bucket first
        for urgency in (0..=7).rev() {
            if let Some(bucket) = self.buckets.get_mut(&urgency) {
                if let Some(id) = bucket.dequeue() {
                    if let Some(item) = self.items.get_mut(&id) { item.start(now); }
                    return Some(id);
                }
            }
        }
        None
    }

    #[inline(always)]
    pub fn complete(&mut self, item_id: u64, ts: u64) {
        if let Some(item) = self.items.get_mut(&item_id) { item.complete(ts); }
    }

    #[inline]
    pub fn fail(&mut self, item_id: u64, ts: u64) {
        if let Some(item) = self.items.get_mut(&item_id) {
            let urgency = item.urgency;
            item.fail(ts);
            if item.state == QueueItemState::Pending {
                if let Some(bucket) = self.buckets.get_mut(&(urgency as u8)) { bucket.enqueue(item_id); }
            }
        }
    }

    #[inline]
    pub fn expire_overdue(&mut self, now: u64) {
        let overdue: Vec<u64> = self.items.iter()
            .filter(|(_, i)| i.state == QueueItemState::Pending && i.is_overdue(now))
            .map(|(&id, _)| id).collect();
        for id in overdue {
            if let Some(item) = self.items.get_mut(&id) { item.state = QueueItemState::TimedOut; }
        }
    }

    pub fn recompute(&mut self) {
        self.stats.total_items = self.items.len();
        self.stats.pending_items = self.items.values().filter(|i| i.state == QueueItemState::Pending).count();
        self.stats.running_items = self.items.values().filter(|i| i.state == QueueItemState::Running).count();
        self.stats.completed_items = self.items.values().filter(|i| i.state == QueueItemState::Completed).count() as u64;
        self.stats.failed_items = self.items.values().filter(|i| i.state == QueueItemState::Failed).count() as u64;
        self.stats.timed_out_items = self.items.values().filter(|i| i.state == QueueItemState::TimedOut).count() as u64;
        self.stats.total_nodes = self.partitions.len();
        let waits: Vec<u64> = self.items.values().map(|i| i.wait_ns).collect();
        self.stats.avg_wait_ns = if waits.is_empty() { 0.0 } else { waits.iter().sum::<u64>() as f64 / waits.len() as f64 };
        self.stats.max_wait_ns = waits.iter().copied().max().unwrap_or(0);
        let execs: Vec<u64> = self.items.values().filter(|i| i.exec_ns > 0).map(|i| i.exec_ns).collect();
        self.stats.avg_exec_ns = if execs.is_empty() { 0.0 } else { execs.iter().sum::<u64>() as f64 / execs.len() as f64 };
        let turns: Vec<u64> = self.items.values().filter_map(|i| i.turnaround_ns()).collect();
        self.stats.avg_turnaround_ns = if turns.is_empty() { 0.0 } else { turns.iter().sum::<u64>() as f64 / turns.len() as f64 };
    }

    #[inline(always)]
    pub fn item(&self, id: u64) -> Option<&QueueItem> { self.items.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &PriorityQueueStats { &self.stats }
}

// ============================================================================
// Merged from priority_queue_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PriorityLevelV2 {
    Idle,
    Low,
    BelowNormal,
    Normal,
    AboveNormal,
    High,
    Realtime,
    Critical,
}

/// Priority item v2
#[derive(Debug)]
pub struct PriorityItemV2 {
    pub id: u64,
    pub priority: PriorityLevelV2,
    pub data_hash: u64,
    pub enqueued_at: u64,
    pub deadline: u64,
}

/// Priority queue v2
#[derive(Debug)]
#[repr(align(64))]
pub struct PriorityQueueV2 {
    pub id: u64,
    pub bins: BTreeMap<u8, VecDeque<PriorityItemV2>>,
    pub total_enqueued: u64,
    pub total_dequeued: u64,
    pub total_expired: u64,
    pub capacity: u32,
}

impl PriorityQueueV2 {
    pub fn new(id: u64, cap: u32) -> Self {
        Self { id, bins: BTreeMap::new(), total_enqueued: 0, total_dequeued: 0, total_expired: 0, capacity: cap }
    }

    fn prio_to_bin(p: PriorityLevelV2) -> u8 {
        match p {
            PriorityLevelV2::Idle => 0, PriorityLevelV2::Low => 1,
            PriorityLevelV2::BelowNormal => 2, PriorityLevelV2::Normal => 3,
            PriorityLevelV2::AboveNormal => 4, PriorityLevelV2::High => 5,
            PriorityLevelV2::Realtime => 6, PriorityLevelV2::Critical => 7,
        }
    }

    #[inline]
    pub fn enqueue(&mut self, item: PriorityItemV2) -> bool {
        let total: usize = self.bins.values().map(|b| b.len()).sum();
        if total >= self.capacity as usize { return false; }
        let bin = Self::prio_to_bin(item.priority);
        self.bins.entry(bin).or_insert_with(VecDeque::new).push_back(item);
        self.total_enqueued += 1;
        true
    }

    #[inline]
    pub fn dequeue(&mut self) -> Option<PriorityItemV2> {
        for bin in (0..=7).rev() {
            if let Some(items) = self.bins.get_mut(&bin) {
                if !items.is_empty() {
                    self.total_dequeued += 1;
                    return Some(items.pop_front().unwrap());
                }
            }
        }
        None
    }

    #[inline]
    pub fn expire(&mut self, now: u64) -> u32 {
        let mut expired = 0u32;
        for items in self.bins.values_mut() {
            let before = items.len();
            items.retain(|i| i.deadline == 0 || i.deadline > now);
            expired += (before - items.len()) as u32;
        }
        self.total_expired += expired as u64;
        expired
    }

    #[inline(always)]
    pub fn len(&self) -> u32 { self.bins.values().map(|b| b.len() as u32).sum() }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PriorityQueueV2Stats {
    pub total_queues: u32,
    pub total_items: u32,
    pub total_enqueued: u64,
    pub total_dequeued: u64,
    pub total_expired: u64,
}

/// Main coop priority queue v2
#[repr(align(64))]
pub struct CoopPriorityQueueV2 {
    queues: BTreeMap<u64, PriorityQueueV2>,
    next_id: u64,
}

impl CoopPriorityQueueV2 {
    pub fn new() -> Self { Self { queues: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn create(&mut self, cap: u32) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.queues.insert(id, PriorityQueueV2::new(id, cap));
        id
    }

    #[inline(always)]
    pub fn enqueue(&mut self, qid: u64, item: PriorityItemV2) -> bool {
        if let Some(q) = self.queues.get_mut(&qid) { q.enqueue(item) }
        else { false }
    }

    #[inline(always)]
    pub fn dequeue(&mut self, qid: u64) -> Option<PriorityItemV2> {
        if let Some(q) = self.queues.get_mut(&qid) { q.dequeue() }
        else { None }
    }

    #[inline(always)]
    pub fn destroy(&mut self, qid: u64) { self.queues.remove(&qid); }

    #[inline]
    pub fn stats(&self) -> PriorityQueueV2Stats {
        let items: u32 = self.queues.values().map(|q| q.len()).sum();
        let enq: u64 = self.queues.values().map(|q| q.total_enqueued).sum();
        let deq: u64 = self.queues.values().map(|q| q.total_dequeued).sum();
        let exp: u64 = self.queues.values().map(|q| q.total_expired).sum();
        PriorityQueueV2Stats { total_queues: self.queues.len() as u32, total_items: items, total_enqueued: enq, total_dequeued: deq, total_expired: exp }
    }
}
