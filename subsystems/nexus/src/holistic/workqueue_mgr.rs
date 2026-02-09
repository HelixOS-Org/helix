//! # Holistic Workqueue Manager
//!
//! Kernel workqueue management and optimization:
//! - Per-CPU and unbound workqueue tracking
//! - Worker thread pool management
//! - Work item latency and execution monitoring
//! - Concurrency level tuning
//! - Priority and nice value management
//! - CPU affinity for workqueues

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Workqueue type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WqType {
    BoundPerCpu,
    Unbound,
    Ordered,
    HighPriority,
    Freezable,
    ReclaimMem,
}

/// Work item state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkItemState {
    Queued,
    Running,
    Completed,
    Cancelled,
    Delayed,
    Failed,
}

/// Work item record
#[derive(Debug, Clone)]
pub struct WorkItem {
    pub item_id: u64,
    pub wq_name: String,
    pub state: WorkItemState,
    pub enqueue_ts: u64,
    pub start_ts: u64,
    pub end_ts: u64,
    pub cpu_id: u32,
    pub delay_ms: u64,
    pub priority: i32,
    pub retries: u32,
}

impl WorkItem {
    pub fn new(id: u64, wq_name: String, ts: u64) -> Self {
        Self {
            item_id: id, wq_name, state: WorkItemState::Queued,
            enqueue_ts: ts, start_ts: 0, end_ts: 0, cpu_id: 0,
            delay_ms: 0, priority: 0, retries: 0,
        }
    }

    #[inline]
    pub fn start(&mut self, cpu: u32, ts: u64) {
        self.state = WorkItemState::Running;
        self.start_ts = ts;
        self.cpu_id = cpu;
    }

    #[inline(always)]
    pub fn complete(&mut self, ts: u64) {
        self.state = WorkItemState::Completed;
        self.end_ts = ts;
    }

    #[inline]
    pub fn fail(&mut self, ts: u64) {
        self.state = WorkItemState::Failed;
        self.end_ts = ts;
        self.retries += 1;
    }

    #[inline(always)]
    pub fn queue_latency_ns(&self) -> u64 {
        if self.start_ts > self.enqueue_ts { self.start_ts - self.enqueue_ts } else { 0 }
    }

    #[inline(always)]
    pub fn exec_time_ns(&self) -> u64 {
        if self.end_ts > self.start_ts { self.end_ts - self.start_ts } else { 0 }
    }
}

/// Worker pool
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct WorkerPool {
    pub pool_id: u32,
    pub cpu_id: Option<u32>,
    pub nice: i32,
    pub nr_workers: u32,
    pub nr_idle: u32,
    pub nr_running: u32,
    pub max_active: u32,
    pub total_items_processed: u64,
    pub total_busy_ns: u64,
}

impl WorkerPool {
    pub fn new(pool_id: u32, cpu: Option<u32>, nice: i32) -> Self {
        Self {
            pool_id, cpu_id: cpu, nice, nr_workers: 0,
            nr_idle: 0, nr_running: 0, max_active: 256,
            total_items_processed: 0, total_busy_ns: 0,
        }
    }

    #[inline(always)]
    pub fn utilization(&self) -> f64 {
        if self.nr_workers == 0 { 0.0 }
        else { self.nr_running as f64 / self.nr_workers as f64 }
    }

    #[inline(always)]
    pub fn needs_more_workers(&self) -> bool {
        self.nr_idle == 0 && self.nr_running < self.max_active
    }
}

/// Workqueue descriptor
#[derive(Debug, Clone)]
pub struct WqDescriptor {
    pub name: String,
    pub wq_type: WqType,
    pub max_active: u32,
    pub nice: i32,
    pub pending_items: u32,
    pub active_items: u32,
    pub total_completed: u64,
    pub total_failed: u64,
    pub avg_queue_latency_ns: u64,
    pub avg_exec_time_ns: u64,
    pub p99_queue_latency_ns: u64,
    pub cpu_affinity_mask: u64,
}

impl WqDescriptor {
    pub fn new(name: String, wq_type: WqType) -> Self {
        Self {
            name, wq_type, max_active: 256, nice: 0,
            pending_items: 0, active_items: 0, total_completed: 0,
            total_failed: 0, avg_queue_latency_ns: 0, avg_exec_time_ns: 0,
            p99_queue_latency_ns: 0, cpu_affinity_mask: u64::MAX,
        }
    }

    pub fn update_latencies(&mut self, items: &[WorkItem]) {
        let completed: Vec<&WorkItem> = items.iter().filter(|i| i.state == WorkItemState::Completed && i.wq_name == self.name).collect();
        if completed.is_empty() { return; }
        let total_queue: u64 = completed.iter().map(|i| i.queue_latency_ns()).sum();
        let total_exec: u64 = completed.iter().map(|i| i.exec_time_ns()).sum();
        self.avg_queue_latency_ns = total_queue / completed.len() as u64;
        self.avg_exec_time_ns = total_exec / completed.len() as u64;
        let mut lats: Vec<u64> = completed.iter().map(|i| i.queue_latency_ns()).collect();
        lats.sort_unstable();
        let p99_idx = (lats.len() * 99 / 100).max(1) - 1;
        self.p99_queue_latency_ns = lats[p99_idx.min(lats.len() - 1)];
    }
}

/// Workqueue manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct WorkqueueStats {
    pub total_workqueues: usize,
    pub total_pools: usize,
    pub total_workers: u32,
    pub total_items_completed: u64,
    pub total_items_failed: u64,
    pub total_items_pending: u32,
    pub avg_pool_utilization: f64,
    pub avg_queue_latency_ns: u64,
    pub busiest_wq: u64,
}

/// Holistic workqueue manager
pub struct HolisticWorkqueueMgr {
    workqueues: BTreeMap<String, WqDescriptor>,
    pools: BTreeMap<u32, WorkerPool>,
    items: Vec<WorkItem>,
    max_items: usize,
    next_item_id: u64,
    stats: WorkqueueStats,
}

impl HolisticWorkqueueMgr {
    pub fn new() -> Self {
        Self {
            workqueues: BTreeMap::new(), pools: BTreeMap::new(),
            items: Vec::new(), max_items: 4096, next_item_id: 1,
            stats: WorkqueueStats::default(),
        }
    }

    #[inline(always)]
    pub fn register_wq(&mut self, desc: WqDescriptor) {
        self.workqueues.insert(desc.name.clone(), desc);
    }

    #[inline(always)]
    pub fn add_pool(&mut self, pool: WorkerPool) {
        self.pools.insert(pool.pool_id, pool);
    }

    pub fn enqueue_work(&mut self, wq_name: &str, ts: u64) -> u64 {
        let id = self.next_item_id;
        self.next_item_id += 1;
        let item = WorkItem::new(id, String::from(wq_name), ts);
        self.items.push(item);
        if let Some(wq) = self.workqueues.get_mut(&String::from(wq_name)) {
            wq.pending_items += 1;
        }
        if self.items.len() > self.max_items {
            self.items.retain(|i| i.state == WorkItemState::Queued || i.state == WorkItemState::Running);
        }
        id
    }

    #[inline]
    pub fn start_work(&mut self, item_id: u64, cpu: u32, ts: u64) {
        if let Some(item) = self.items.iter_mut().find(|i| i.item_id == item_id) {
            let wq_name = item.wq_name.clone();
            item.start(cpu, ts);
            if let Some(wq) = self.workqueues.get_mut(&wq_name) {
                wq.pending_items = wq.pending_items.saturating_sub(1);
                wq.active_items += 1;
            }
        }
    }

    #[inline]
    pub fn complete_work(&mut self, item_id: u64, ts: u64) {
        if let Some(item) = self.items.iter_mut().find(|i| i.item_id == item_id) {
            let wq_name = item.wq_name.clone();
            item.complete(ts);
            if let Some(wq) = self.workqueues.get_mut(&wq_name) {
                wq.active_items = wq.active_items.saturating_sub(1);
                wq.total_completed += 1;
            }
        }
    }

    #[inline]
    pub fn fail_work(&mut self, item_id: u64, ts: u64) {
        if let Some(item) = self.items.iter_mut().find(|i| i.item_id == item_id) {
            let wq_name = item.wq_name.clone();
            item.fail(ts);
            if let Some(wq) = self.workqueues.get_mut(&wq_name) {
                wq.active_items = wq.active_items.saturating_sub(1);
                wq.total_failed += 1;
            }
        }
    }

    #[inline]
    pub fn tune_concurrency(&mut self, wq_name: &str, max_active: u32) {
        if let Some(wq) = self.workqueues.get_mut(&String::from(wq_name)) {
            wq.max_active = max_active;
        }
    }

    pub fn recompute(&mut self) {
        self.stats.total_workqueues = self.workqueues.len();
        self.stats.total_pools = self.pools.len();
        self.stats.total_workers = self.pools.values().map(|p| p.nr_workers).sum();
        self.stats.total_items_completed = self.workqueues.values().map(|w| w.total_completed).sum();
        self.stats.total_items_failed = self.workqueues.values().map(|w| w.total_failed).sum();
        self.stats.total_items_pending = self.workqueues.values().map(|w| w.pending_items).sum();
        let utils: Vec<f64> = self.pools.values().map(|p| p.utilization()).collect();
        self.stats.avg_pool_utilization = if utils.is_empty() { 0.0 } else { utils.iter().sum::<f64>() / utils.len() as f64 };
        let lats: Vec<u64> = self.workqueues.values().map(|w| w.avg_queue_latency_ns).collect();
        self.stats.avg_queue_latency_ns = if lats.is_empty() { 0 } else { lats.iter().sum::<u64>() / lats.len() as u64 };
    }

    #[inline(always)]
    pub fn wq(&self, name: &str) -> Option<&WqDescriptor> { self.workqueues.get(&String::from(name)) }
    #[inline(always)]
    pub fn pool(&self, id: u32) -> Option<&WorkerPool> { self.pools.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &WorkqueueStats { &self.stats }
}
