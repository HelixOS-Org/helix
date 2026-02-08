// SPDX-License-Identifier: GPL-2.0
//! Holistic workqueue — kernel workqueue management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Work item state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkItemState {
    Pending,
    Running,
    Completed,
    Cancelled,
    Delayed,
}

/// Work item priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WqPriority {
    HighPri,
    Normal,
    Long,
    Unbound,
    Freezable,
}

/// Work item
#[derive(Debug)]
pub struct WorkItem {
    pub id: u64,
    pub handler_hash: u64,
    pub state: WorkItemState,
    pub priority: WqPriority,
    pub cpu_bound: Option<u32>,
    pub submit_time: u64,
    pub start_time: u64,
    pub complete_time: u64,
    pub delay_ns: u64,
}

impl WorkItem {
    pub fn new(id: u64, handler: u64, prio: WqPriority, now: u64) -> Self {
        Self { id, handler_hash: handler, state: WorkItemState::Pending, priority: prio, cpu_bound: None, submit_time: now, start_time: 0, complete_time: 0, delay_ns: 0 }
    }

    pub fn latency_ns(&self) -> u64 { self.start_time.saturating_sub(self.submit_time) }
    pub fn exec_time_ns(&self) -> u64 { self.complete_time.saturating_sub(self.start_time) }
}

/// Worker pool
#[derive(Debug)]
pub struct WorkerPool {
    pub id: u32,
    pub cpu: Option<u32>,
    pub nr_workers: u32,
    pub nr_idle: u32,
    pub items: Vec<WorkItem>,
    pub total_executed: u64,
}

impl WorkerPool {
    pub fn new(id: u32, cpu: Option<u32>, workers: u32) -> Self {
        Self { id, cpu, nr_workers: workers, nr_idle: workers, items: Vec::new(), total_executed: 0 }
    }

    pub fn enqueue(&mut self, item: WorkItem) { self.items.push(item); }

    pub fn dequeue(&mut self, now: u64) -> Option<u64> {
        if let Some(item) = self.items.iter_mut().find(|i| i.state == WorkItemState::Pending) {
            item.state = WorkItemState::Running;
            item.start_time = now;
            if self.nr_idle > 0 { self.nr_idle -= 1; }
            Some(item.id)
        } else { None }
    }

    pub fn complete(&mut self, id: u64, now: u64) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.state = WorkItemState::Completed;
            item.complete_time = now;
            self.total_executed += 1;
            self.nr_idle = (self.nr_idle + 1).min(self.nr_workers);
        }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct WorkqueueStats {
    pub total_pools: u32,
    pub total_pending: u32,
    pub total_running: u32,
    pub total_executed: u64,
    pub avg_latency_ns: u64,
}

/// Main holistic workqueue
pub struct HolisticWorkqueue {
    pools: BTreeMap<u32, WorkerPool>,
}

impl HolisticWorkqueue {
    pub fn new() -> Self { Self { pools: BTreeMap::new() } }

    pub fn create_pool(&mut self, id: u32, cpu: Option<u32>, workers: u32) { self.pools.insert(id, WorkerPool::new(id, cpu, workers)); }

    pub fn enqueue(&mut self, pool_id: u32, item: WorkItem) { if let Some(p) = self.pools.get_mut(&pool_id) { p.enqueue(item); } }

    pub fn stats(&self) -> WorkqueueStats {
        let pending: u32 = self.pools.values().flat_map(|p| p.items.iter()).filter(|i| i.state == WorkItemState::Pending).count() as u32;
        let running: u32 = self.pools.values().flat_map(|p| p.items.iter()).filter(|i| i.state == WorkItemState::Running).count() as u32;
        let executed: u64 = self.pools.values().map(|p| p.total_executed).sum();
        let lats: Vec<u64> = self.pools.values().flat_map(|p| p.items.iter()).filter(|i| i.state == WorkItemState::Completed).map(|i| i.latency_ns()).collect();
        let avg = if lats.is_empty() { 0 } else { lats.iter().sum::<u64>() / lats.len() as u64 };
        WorkqueueStats { total_pools: self.pools.len() as u32, total_pending: pending, total_running: running, total_executed: executed, avg_latency_ns: avg }
    }
}

// ============================================================================
// Merged from workqueue_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WqV2Priority {
    Background,
    Low,
    Normal,
    High,
    Critical,
    RealTime,
}

/// Work item execution state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WqV2WorkState {
    Queued,
    Pending,
    Running,
    Completed,
    Cancelled,
    Failed,
    Deferred,
}

/// Workqueue type classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WqV2Type {
    BoundPerCpu,
    Unbound,
    Ordered,
    HighPriority,
    Freezable,
    MemReclaim,
    PowerEfficient,
}

/// Workqueue flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WqV2Flag {
    Draining,
    Rescuer,
    Unbound,
    HighPri,
    CpuIntensive,
    Freezable,
    MemReclaim,
    Sysfs,
}

/// An individual work item.
#[derive(Debug, Clone)]
pub struct WqV2WorkItem {
    pub id: u64,
    pub name: String,
    pub priority: WqV2Priority,
    pub state: WqV2WorkState,
    pub enqueue_time: u64,
    pub start_time: u64,
    pub complete_time: u64,
    pub worker_id: Option<u32>,
    pub cpu_affinity: Option<u32>,
    pub numa_node: Option<u32>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub is_ordered: bool,
    pub sequence_num: u64,
}

impl WqV2WorkItem {
    pub fn new(id: u64, name: String, priority: WqV2Priority) -> Self {
        Self {
            id,
            name,
            priority,
            state: WqV2WorkState::Queued,
            enqueue_time: 0,
            start_time: 0,
            complete_time: 0,
            worker_id: None,
            cpu_affinity: None,
            numa_node: None,
            retry_count: 0,
            max_retries: 3,
            is_ordered: false,
            sequence_num: 0,
        }
    }

    pub fn latency(&self) -> u64 {
        if self.complete_time > self.start_time {
            self.complete_time - self.start_time
        } else {
            0
        }
    }

    pub fn wait_time(&self) -> u64 {
        if self.start_time > self.enqueue_time {
            self.start_time - self.enqueue_time
        } else {
            0
        }
    }
}

/// Worker pool for a NUMA node.
#[derive(Debug, Clone)]
pub struct WqV2WorkerPool {
    pub pool_id: u32,
    pub numa_node: u32,
    pub max_workers: u32,
    pub active_workers: u32,
    pub idle_workers: u32,
    pub pending_work: u64,
    pub completed_work: u64,
    pub is_unbound: bool,
}

impl WqV2WorkerPool {
    pub fn new(pool_id: u32, numa_node: u32, max_workers: u32) -> Self {
        Self {
            pool_id,
            numa_node,
            max_workers,
            active_workers: 0,
            idle_workers: max_workers,
            pending_work: 0,
            completed_work: 0,
            is_unbound: false,
        }
    }

    pub fn try_dispatch(&mut self) -> bool {
        if self.idle_workers > 0 && self.pending_work > 0 {
            self.idle_workers -= 1;
            self.active_workers += 1;
            self.pending_work -= 1;
            true
        } else {
            false
        }
    }

    pub fn complete_work(&mut self) {
        if self.active_workers > 0 {
            self.active_workers -= 1;
            self.idle_workers += 1;
            self.completed_work += 1;
        }
    }

    pub fn utilization_percent(&self) -> f64 {
        if self.max_workers == 0 {
            return 0.0;
        }
        (self.active_workers as f64 / self.max_workers as f64) * 100.0
    }
}

/// A named workqueue instance.
#[derive(Debug, Clone)]
pub struct WqV2Instance {
    pub wq_id: u64,
    pub name: String,
    pub wq_type: WqV2Type,
    pub flags: Vec<WqV2Flag>,
    pub max_concurrency: u32,
    pub items: Vec<WqV2WorkItem>,
    pub total_enqueued: u64,
    pub total_completed: u64,
    pub total_failed: u64,
    pub ordered_sequence: u64,
}

impl WqV2Instance {
    pub fn new(wq_id: u64, name: String, wq_type: WqV2Type) -> Self {
        Self {
            wq_id,
            name,
            wq_type,
            flags: Vec::new(),
            max_concurrency: 256,
            items: Vec::new(),
            total_enqueued: 0,
            total_completed: 0,
            total_failed: 0,
            ordered_sequence: 0,
        }
    }

    pub fn enqueue(&mut self, mut item: WqV2WorkItem) -> u64 {
        if self.wq_type == WqV2Type::Ordered {
            self.ordered_sequence += 1;
            item.is_ordered = true;
            item.sequence_num = self.ordered_sequence;
        }
        let id = item.id;
        self.items.push(item);
        self.total_enqueued += 1;
        id
    }

    pub fn pending_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| i.state == WqV2WorkState::Queued || i.state == WqV2WorkState::Pending)
            .count()
    }
}

/// Statistics for the workqueue V2 manager.
#[derive(Debug, Clone)]
pub struct WorkqueueV2Stats {
    pub total_workqueues: u64,
    pub total_pools: u64,
    pub total_items_enqueued: u64,
    pub total_items_completed: u64,
    pub total_items_failed: u64,
    pub avg_wait_time: u64,
    pub avg_exec_time: u64,
    pub pool_rebalance_count: u64,
}

/// Main holistic workqueue V2 manager.
pub struct HolisticWorkqueueV2 {
    pub workqueues: BTreeMap<u64, WqV2Instance>,
    pub pools: BTreeMap<u32, WqV2WorkerPool>,
    pub next_wq_id: u64,
    pub next_item_id: u64,
    pub stats: WorkqueueV2Stats,
}

impl HolisticWorkqueueV2 {
    pub fn new() -> Self {
        Self {
            workqueues: BTreeMap::new(),
            pools: BTreeMap::new(),
            next_wq_id: 1,
            next_item_id: 1,
            stats: WorkqueueV2Stats {
                total_workqueues: 0,
                total_pools: 0,
                total_items_enqueued: 0,
                total_items_completed: 0,
                total_items_failed: 0,
                avg_wait_time: 0,
                avg_exec_time: 0,
                pool_rebalance_count: 0,
            },
        }
    }

    pub fn create_workqueue(&mut self, name: String, wq_type: WqV2Type) -> u64 {
        let id = self.next_wq_id;
        self.next_wq_id += 1;
        let wq = WqV2Instance::new(id, name, wq_type);
        self.workqueues.insert(id, wq);
        self.stats.total_workqueues += 1;
        id
    }

    pub fn create_pool(&mut self, pool_id: u32, numa_node: u32, max_workers: u32) {
        let pool = WqV2WorkerPool::new(pool_id, numa_node, max_workers);
        self.pools.insert(pool_id, pool);
        self.stats.total_pools += 1;
    }

    pub fn enqueue_work(
        &mut self,
        wq_id: u64,
        name: String,
        priority: WqV2Priority,
    ) -> Option<u64> {
        let item_id = self.next_item_id;
        self.next_item_id += 1;
        let item = WqV2WorkItem::new(item_id, name, priority);
        if let Some(wq) = self.workqueues.get_mut(&wq_id) {
            wq.enqueue(item);
            self.stats.total_items_enqueued += 1;
            Some(item_id)
        } else {
            None
        }
    }

    pub fn workqueue_count(&self) -> usize {
        self.workqueues.len()
    }

    pub fn pool_count(&self) -> usize {
        self.pools.len()
    }
}
