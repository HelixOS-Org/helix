//! # Bridge WorkQueue Proxy
//!
//! Bridges kernel workqueue operations:
//! - Workqueue creation and management
//! - Work item scheduling and cancellation
//! - Delayed work support
//! - CPU affinity and concurrency control
//! - Work item statistics and monitoring

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;

/// Workqueue flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WqFlag {
    Unbound,
    Freezable,
    MemReclaim,
    HighPri,
    CpuIntensive,
    Sysfs,
    PowerEfficient,
}

/// Work item state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkState {
    Pending,
    Running,
    Delayed,
    Cancelled,
    Done,
    Failed,
}

/// Work item priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorkPriority {
    Low,
    Normal,
    High,
    RealTime,
}

/// Work item
#[derive(Debug, Clone)]
pub struct WorkItem {
    pub id: u64,
    pub wq_id: u64,
    pub state: WorkState,
    pub priority: WorkPriority,
    pub cpu_affinity: Option<u32>,
    pub delay_ns: u64,
    pub enqueue_ts: u64,
    pub start_ts: u64,
    pub end_ts: u64,
    pub handler_id: u64,
    pub retries: u32,
    pub max_retries: u32,
}

impl WorkItem {
    pub fn new(id: u64, wq: u64, handler: u64, prio: WorkPriority) -> Self {
        Self {
            id, wq_id: wq, state: WorkState::Pending, priority: prio,
            cpu_affinity: None, delay_ns: 0, enqueue_ts: 0, start_ts: 0,
            end_ts: 0, handler_id: handler, retries: 0, max_retries: 3,
        }
    }

    #[inline(always)]
    pub fn set_delayed(&mut self, delay: u64) { self.delay_ns = delay; self.state = WorkState::Delayed; }

    #[inline(always)]
    pub fn start(&mut self, ts: u64) { self.state = WorkState::Running; self.start_ts = ts; }

    #[inline(always)]
    pub fn complete(&mut self, ts: u64) { self.state = WorkState::Done; self.end_ts = ts; }

    #[inline]
    pub fn fail(&mut self, ts: u64) {
        self.retries += 1;
        if self.retries >= self.max_retries { self.state = WorkState::Failed; self.end_ts = ts; }
        else { self.state = WorkState::Pending; }
    }

    #[inline(always)]
    pub fn cancel(&mut self) { self.state = WorkState::Cancelled; }

    #[inline(always)]
    pub fn latency_ns(&self) -> u64 { if self.start_ts > self.enqueue_ts { self.start_ts - self.enqueue_ts } else { 0 } }
    #[inline(always)]
    pub fn execution_ns(&self) -> u64 { if self.end_ts > self.start_ts { self.end_ts - self.start_ts } else { 0 } }
}

/// Workqueue descriptor
#[derive(Debug, Clone)]
pub struct Workqueue {
    pub id: u64,
    pub name: String,
    pub flags: Vec<WqFlag>,
    pub max_active: u32,
    pub current_active: u32,
    pub nr_pending: u32,
    pub nr_delayed: u32,
    pub total_executed: u64,
    pub total_failed: u64,
    pub total_cancelled: u64,
    pub cpu_mask: u64,
    pub nice: i8,
}

impl Workqueue {
    pub fn new(id: u64, name: String, max_active: u32) -> Self {
        Self {
            id, name, flags: Vec::new(), max_active, current_active: 0,
            nr_pending: 0, nr_delayed: 0, total_executed: 0,
            total_failed: 0, total_cancelled: 0, cpu_mask: u64::MAX, nice: 0,
        }
    }

    #[inline(always)]
    pub fn add_flag(&mut self, f: WqFlag) { if !self.flags.contains(&f) { self.flags.push(f); } }
    #[inline(always)]
    pub fn is_unbound(&self) -> bool { self.flags.contains(&WqFlag::Unbound) }
    #[inline(always)]
    pub fn is_high_pri(&self) -> bool { self.flags.contains(&WqFlag::HighPri) }
    #[inline(always)]
    pub fn can_schedule(&self) -> bool { self.current_active < self.max_active }
    #[inline(always)]
    pub fn utilization(&self) -> f64 { if self.max_active == 0 { 0.0 } else { self.current_active as f64 / self.max_active as f64 * 100.0 } }
}

/// Per-CPU worker pool
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct WorkerPool {
    pub cpu_id: u32,
    pub nr_workers: u32,
    pub nr_idle: u32,
    pub nr_running: u32,
    pub total_scheduled: u64,
}

impl WorkerPool {
    pub fn new(cpu: u32) -> Self {
        Self { cpu_id: cpu, nr_workers: 4, nr_idle: 4, nr_running: 0, total_scheduled: 0 }
    }
}

/// Workqueue proxy stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct WqProxyStats {
    pub total_wqs: usize,
    pub total_pending: u64,
    pub total_executed: u64,
    pub total_failed: u64,
    pub avg_latency_ns: u64,
    pub max_latency_ns: u64,
    pub worker_pools: usize,
}

/// Bridge workqueue proxy
#[repr(align(64))]
pub struct BridgeWqProxy {
    queues: BTreeMap<u64, Workqueue>,
    items: BTreeMap<u64, WorkItem>,
    pools: BTreeMap<u32, WorkerPool>,
    stats: WqProxyStats,
    next_wq: u64,
    next_item: u64,
}

impl BridgeWqProxy {
    pub fn new() -> Self {
        Self { queues: BTreeMap::new(), items: BTreeMap::new(), pools: BTreeMap::new(), stats: WqProxyStats::default(), next_wq: 1, next_item: 1 }
    }

    #[inline]
    pub fn create_wq(&mut self, name: String, max_active: u32) -> u64 {
        let id = self.next_wq; self.next_wq += 1;
        self.queues.insert(id, Workqueue::new(id, name, max_active));
        id
    }

    #[inline]
    pub fn destroy_wq(&mut self, id: u64) {
        self.queues.remove(&id);
        let to_cancel: Vec<u64> = self.items.iter().filter(|(_, w)| w.wq_id == id).map(|(&k, _)| k).collect();
        for wid in to_cancel { self.items.remove(&wid); }
    }

    #[inline]
    pub fn queue_work(&mut self, wq: u64, handler: u64, prio: WorkPriority, ts: u64) -> u64 {
        let id = self.next_item; self.next_item += 1;
        let mut item = WorkItem::new(id, wq, handler, prio);
        item.enqueue_ts = ts;
        self.items.insert(id, item);
        if let Some(q) = self.queues.get_mut(&wq) { q.nr_pending += 1; }
        id
    }

    #[inline]
    pub fn queue_delayed_work(&mut self, wq: u64, handler: u64, delay: u64, ts: u64) -> u64 {
        let id = self.next_item; self.next_item += 1;
        let mut item = WorkItem::new(id, wq, handler, WorkPriority::Normal);
        item.enqueue_ts = ts;
        item.set_delayed(delay);
        self.items.insert(id, item);
        if let Some(q) = self.queues.get_mut(&wq) { q.nr_delayed += 1; }
        id
    }

    #[inline]
    pub fn start_work(&mut self, item_id: u64, ts: u64) {
        if let Some(w) = self.items.get_mut(&item_id) {
            let wq = w.wq_id;
            w.start(ts);
            if let Some(q) = self.queues.get_mut(&wq) { q.nr_pending = q.nr_pending.saturating_sub(1); q.current_active += 1; }
        }
    }

    #[inline]
    pub fn complete_work(&mut self, item_id: u64, ts: u64) {
        if let Some(w) = self.items.get_mut(&item_id) {
            let wq = w.wq_id;
            w.complete(ts);
            if let Some(q) = self.queues.get_mut(&wq) { q.current_active = q.current_active.saturating_sub(1); q.total_executed += 1; }
        }
    }

    #[inline]
    pub fn fail_work(&mut self, item_id: u64, ts: u64) {
        if let Some(w) = self.items.get_mut(&item_id) {
            let wq = w.wq_id;
            w.fail(ts);
            if w.state == WorkState::Failed { if let Some(q) = self.queues.get_mut(&wq) { q.total_failed += 1; q.current_active = q.current_active.saturating_sub(1); } }
        }
    }

    #[inline]
    pub fn cancel_work(&mut self, item_id: u64) {
        if let Some(w) = self.items.get_mut(&item_id) {
            let wq = w.wq_id;
            w.cancel();
            if let Some(q) = self.queues.get_mut(&wq) { q.total_cancelled += 1; q.nr_pending = q.nr_pending.saturating_sub(1); }
        }
    }

    #[inline(always)]
    pub fn add_pool(&mut self, cpu: u32) { self.pools.insert(cpu, WorkerPool::new(cpu)); }

    pub fn recompute(&mut self) {
        self.stats.total_wqs = self.queues.len();
        self.stats.total_pending = self.queues.values().map(|q| q.nr_pending as u64).sum();
        self.stats.total_executed = self.queues.values().map(|q| q.total_executed).sum();
        self.stats.total_failed = self.queues.values().map(|q| q.total_failed).sum();
        self.stats.worker_pools = self.pools.len();
        let done: Vec<&WorkItem> = self.items.values().filter(|w| w.state == WorkState::Done).collect();
        if !done.is_empty() {
            let total_lat: u64 = done.iter().map(|w| w.latency_ns()).sum();
            self.stats.avg_latency_ns = total_lat / done.len() as u64;
            self.stats.max_latency_ns = done.iter().map(|w| w.latency_ns()).max().unwrap_or(0);
        }
    }

    #[inline(always)]
    pub fn wq(&self, id: u64) -> Option<&Workqueue> { self.queues.get(&id) }
    #[inline(always)]
    pub fn work(&self, id: u64) -> Option<&WorkItem> { self.items.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &WqProxyStats { &self.stats }
}
