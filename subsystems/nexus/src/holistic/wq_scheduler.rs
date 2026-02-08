// SPDX-License-Identifier: GPL-2.0
//! Holistic wq_scheduler — kernel workqueue scheduling and management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Workqueue type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WqType {
    Bound,
    Unbound,
    Ordered,
    HighPriority,
    Freezable,
    MemReclaim,
    Rescuer,
}

/// Work item state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkItemState {
    Pending,
    Running,
    Delayed,
    Cancelled,
    Completed,
    Failed,
}

/// Work item priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorkPriority {
    Realtime = 0,
    High = 1,
    Normal = 2,
    Low = 3,
    Idle = 4,
}

/// Work item flags
#[derive(Debug, Clone, Copy)]
pub struct WorkFlags {
    pub bits: u32,
}

impl WorkFlags {
    pub const HIGHPRI: u32 = 1 << 0;
    pub const CPU_INTENSIVE: u32 = 1 << 1;
    pub const MEM_RECLAIM: u32 = 1 << 2;
    pub const FREEZABLE: u32 = 1 << 3;
    pub const UNBOUND: u32 = 1 << 4;
    pub const ORDERED: u32 = 1 << 5;

    pub fn new(bits: u32) -> Self { Self { bits } }
    pub fn has(&self, flag: u32) -> bool { self.bits & flag != 0 }
}

/// Work item descriptor
#[derive(Debug, Clone)]
pub struct WorkItem {
    pub id: u64,
    pub wq_id: u32,
    pub state: WorkItemState,
    pub priority: WorkPriority,
    pub flags: WorkFlags,
    pub cpu_affinity: Option<u32>,
    pub delay_ns: u64,
    pub enqueued_at: u64,
    pub started_at: u64,
    pub completed_at: u64,
    pub execution_ns: u64,
    pub retries: u32,
}

impl WorkItem {
    pub fn new(id: u64, wq_id: u32, priority: WorkPriority) -> Self {
        Self {
            id, wq_id, state: WorkItemState::Pending,
            priority, flags: WorkFlags::new(0),
            cpu_affinity: None, delay_ns: 0,
            enqueued_at: 0, started_at: 0, completed_at: 0,
            execution_ns: 0, retries: 0,
        }
    }

    pub fn enqueue(&mut self, now: u64) {
        self.state = WorkItemState::Pending;
        self.enqueued_at = now;
    }

    pub fn start(&mut self, now: u64) {
        self.state = WorkItemState::Running;
        self.started_at = now;
    }

    pub fn complete(&mut self, now: u64) {
        self.state = WorkItemState::Completed;
        self.completed_at = now;
        self.execution_ns = now.saturating_sub(self.started_at);
    }

    pub fn fail(&mut self) {
        self.state = WorkItemState::Failed;
        self.retries += 1;
    }

    pub fn wait_time(&self, now: u64) -> u64 { now.saturating_sub(self.enqueued_at) }
    pub fn latency_ns(&self) -> u64 { if self.completed_at > 0 { self.completed_at.saturating_sub(self.enqueued_at) } else { 0 } }
}

/// Worker thread state
#[derive(Debug, Clone)]
pub struct WqWorker {
    pub id: u32,
    pub wq_id: u32,
    pub cpu_id: u32,
    pub active: bool,
    pub current_work: Option<u64>,
    pub items_processed: u64,
    pub total_exec_ns: u64,
    pub idle_since: u64,
}

impl WqWorker {
    pub fn new(id: u32, wq_id: u32, cpu_id: u32) -> Self {
        Self {
            id, wq_id, cpu_id, active: false, current_work: None,
            items_processed: 0, total_exec_ns: 0, idle_since: 0,
        }
    }

    pub fn assign(&mut self, work_id: u64) {
        self.current_work = Some(work_id);
        self.active = true;
    }

    pub fn finish(&mut self, exec_ns: u64, now: u64) {
        self.current_work = None;
        self.active = false;
        self.items_processed += 1;
        self.total_exec_ns += exec_ns;
        self.idle_since = now;
    }

    pub fn avg_exec_ns(&self) -> u64 {
        if self.items_processed == 0 { 0 } else { self.total_exec_ns / self.items_processed }
    }
}

/// Workqueue instance
#[derive(Debug)]
pub struct Workqueue {
    pub id: u32,
    pub wq_type: WqType,
    pub max_workers: u32,
    pub workers: Vec<WqWorker>,
    pub pending: Vec<WorkItem>,
    pub total_submitted: u64,
    pub total_completed: u64,
    pub total_failed: u64,
}

impl Workqueue {
    pub fn new(id: u32, wq_type: WqType, max_workers: u32) -> Self {
        Self {
            id, wq_type, max_workers, workers: Vec::new(),
            pending: Vec::new(), total_submitted: 0,
            total_completed: 0, total_failed: 0,
        }
    }

    pub fn submit(&mut self, mut item: WorkItem, now: u64) -> u64 {
        let id = item.id;
        item.enqueue(now);
        self.pending.push(item);
        self.total_submitted += 1;
        id
    }

    pub fn dispatch(&mut self, now: u64) -> Option<(u32, u64)> {
        let worker_id = self.workers.iter().position(|w| !w.active)?;
        let item = self.pending.pop()?;
        let work_id = item.id;
        self.workers[worker_id].assign(work_id);
        Some((self.workers[worker_id].id, work_id))
    }

    pub fn utilization(&self) -> f64 {
        if self.workers.is_empty() { return 0.0; }
        let active = self.workers.iter().filter(|w| w.active).count();
        active as f64 / self.workers.len() as f64
    }

    pub fn queue_depth(&self) -> u32 { self.pending.len() as u32 }
}

/// WQ scheduler stats
#[derive(Debug, Clone)]
pub struct WqSchedulerStats {
    pub total_workqueues: u32,
    pub total_workers: u32,
    pub active_workers: u32,
    pub total_pending: u64,
    pub total_submitted: u64,
    pub total_completed: u64,
    pub avg_latency_ns: u64,
}

/// Main workqueue scheduler
pub struct HolisticWqScheduler {
    workqueues: BTreeMap<u32, Workqueue>,
    completed_items: Vec<WorkItem>,
    next_wq_id: u32,
    next_work_id: u64,
    max_completed: usize,
}

impl HolisticWqScheduler {
    pub fn new() -> Self {
        Self {
            workqueues: BTreeMap::new(), completed_items: Vec::new(),
            next_wq_id: 1, next_work_id: 1, max_completed: 4096,
        }
    }

    pub fn create_workqueue(&mut self, wq_type: WqType, max_workers: u32) -> u32 {
        let id = self.next_wq_id;
        self.next_wq_id += 1;
        self.workqueues.insert(id, Workqueue::new(id, wq_type, max_workers));
        id
    }

    pub fn add_worker(&mut self, wq_id: u32, cpu_id: u32) -> Option<u32> {
        let wq = self.workqueues.get_mut(&wq_id)?;
        let worker_id = wq.workers.len() as u32;
        wq.workers.push(WqWorker::new(worker_id, wq_id, cpu_id));
        Some(worker_id)
    }

    pub fn submit(&mut self, wq_id: u32, priority: WorkPriority, now: u64) -> Option<u64> {
        let work_id = self.next_work_id;
        self.next_work_id += 1;
        let item = WorkItem::new(work_id, wq_id, priority);
        self.workqueues.get_mut(&wq_id)?.submit(item, now);
        Some(work_id)
    }

    pub fn stats(&self) -> WqSchedulerStats {
        let total_workers: u32 = self.workqueues.values().map(|wq| wq.workers.len() as u32).sum();
        let active: u32 = self.workqueues.values()
            .flat_map(|wq| wq.workers.iter())
            .filter(|w| w.active).count() as u32;
        let pending: u64 = self.workqueues.values().map(|wq| wq.pending.len() as u64).sum();
        let submitted: u64 = self.workqueues.values().map(|wq| wq.total_submitted).sum();
        let completed: u64 = self.workqueues.values().map(|wq| wq.total_completed).sum();
        let avg_lat = if self.completed_items.is_empty() { 0 } else {
            self.completed_items.iter().map(|i| i.latency_ns()).sum::<u64>() / self.completed_items.len() as u64
        };
        WqSchedulerStats {
            total_workqueues: self.workqueues.len() as u32,
            total_workers, active_workers: active,
            total_pending: pending, total_submitted: submitted,
            total_completed: completed, avg_latency_ns: avg_lat,
        }
    }
}
