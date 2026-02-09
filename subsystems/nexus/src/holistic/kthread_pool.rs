//! # Holistic Kernel Thread Pool
//!
//! Kernel-level worker thread pool management:
//! - Dynamic worker scaling based on load
//! - Per-CPU and global workqueues
//! - Priority-based work scheduling
//! - Worker CPU affinity management
//! - Delayed work and timer-based scheduling
//! - Worker stall detection

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Worker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KWorkerState {
    Idle,
    Running,
    Sleeping,
    Blocked,
    Unbound,
    Dying,
}

/// Work priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KWorkPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    RealTime = 3,
}

/// Work item flags
#[derive(Debug, Clone, Copy)]
pub struct KWorkFlags {
    pub delayed: bool,
    pub cpu_intensive: bool,
    pub freezable: bool,
    pub mem_reclaim: bool,
    pub high_priority: bool,
}

impl KWorkFlags {
    #[inline(always)]
    pub fn default_flags() -> Self { Self { delayed: false, cpu_intensive: false, freezable: false, mem_reclaim: false, high_priority: false } }
}

/// Work item
#[derive(Debug, Clone)]
pub struct KWorkItem {
    pub id: u64,
    pub func_hash: u64,
    pub priority: KWorkPriority,
    pub flags: KWorkFlags,
    pub queued_ts: u64,
    pub start_ts: u64,
    pub end_ts: u64,
    pub worker_id: Option<u64>,
    pub cpu_affinity: Option<u32>,
    pub delay_ns: u64,
    pub status: KWorkStatus,
    pub retries: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KWorkStatus {
    Queued,
    Delayed,
    Running,
    Complete,
    Failed,
    Cancelled,
}

impl KWorkItem {
    pub fn new(id: u64, func_hash: u64, prio: KWorkPriority, flags: KWorkFlags, ts: u64) -> Self {
        Self {
            id, func_hash, priority: prio, flags, queued_ts: ts,
            start_ts: 0, end_ts: 0, worker_id: None,
            cpu_affinity: None, delay_ns: 0, status: KWorkStatus::Queued,
            retries: 0,
        }
    }

    #[inline(always)]
    pub fn delayed(mut self, delay: u64) -> Self { self.delay_ns = delay; self.status = KWorkStatus::Delayed; self }
    #[inline(always)]
    pub fn is_ready(&self, now: u64) -> bool { self.status == KWorkStatus::Queued || (self.status == KWorkStatus::Delayed && now.saturating_sub(self.queued_ts) >= self.delay_ns) }
    #[inline(always)]
    pub fn start(&mut self, worker: u64, ts: u64) { self.status = KWorkStatus::Running; self.worker_id = Some(worker); self.start_ts = ts; }
    #[inline(always)]
    pub fn complete(&mut self, ts: u64) { self.status = KWorkStatus::Complete; self.end_ts = ts; }
    #[inline(always)]
    pub fn fail(&mut self, ts: u64) { self.status = KWorkStatus::Failed; self.end_ts = ts; self.retries += 1; }
    #[inline(always)]
    pub fn cancel(&mut self) { self.status = KWorkStatus::Cancelled; }
    #[inline(always)]
    pub fn latency(&self) -> u64 { self.end_ts.saturating_sub(self.queued_ts) }
    #[inline(always)]
    pub fn exec_time(&self) -> u64 { self.end_ts.saturating_sub(self.start_ts) }
}

/// Worker thread
#[derive(Debug, Clone)]
pub struct KWorker {
    pub id: u64,
    pub state: KWorkerState,
    pub cpu: Option<u32>,
    pub current_work: Option<u64>,
    pub tasks_completed: u64,
    pub busy_ns: u64,
    pub idle_ns: u64,
    pub last_active_ts: u64,
    pub unbound: bool,
    pub rescuer: bool,
}

impl KWorker {
    pub fn new(id: u64, cpu: Option<u32>) -> Self {
        Self { id, state: KWorkerState::Idle, cpu, current_work: None, tasks_completed: 0, busy_ns: 0, idle_ns: 0, last_active_ts: 0, unbound: cpu.is_none(), rescuer: false }
    }

    #[inline(always)]
    pub fn assign(&mut self, work_id: u64, ts: u64) { self.state = KWorkerState::Running; self.current_work = Some(work_id); self.last_active_ts = ts; }
    #[inline(always)]
    pub fn finish(&mut self, ts: u64) { self.state = KWorkerState::Idle; self.current_work = None; self.tasks_completed += 1; self.busy_ns += ts.saturating_sub(self.last_active_ts); }
    #[inline(always)]
    pub fn utilization(&self) -> f64 { let total = self.busy_ns + self.idle_ns; if total == 0 { 0.0 } else { self.busy_ns as f64 / total as f64 } }
}

/// Workqueue
#[derive(Debug, Clone)]
pub struct KWorkqueue {
    pub id: u64,
    pub name_hash: u64,
    pub per_cpu: bool,
    pub max_workers: u32,
    pub min_workers: u32,
    pub ordered: bool,
    pub items_queued: u64,
    pub items_completed: u64,
}

impl KWorkqueue {
    pub fn new(id: u64, name_hash: u64, per_cpu: bool, max: u32) -> Self {
        Self { id, name_hash, per_cpu, max_workers: max, min_workers: 1, ordered: false, items_queued: 0, items_completed: 0 }
    }
}

/// Thread pool stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct KThreadPoolStats {
    pub total_workers: usize,
    pub idle_workers: usize,
    pub busy_workers: usize,
    pub pending_work: usize,
    pub total_completed: u64,
    pub avg_latency_ns: u64,
    pub avg_exec_ns: u64,
    pub workqueues: usize,
}

/// Holistic kernel thread pool
#[repr(align(64))]
pub struct HolisticKthreadPool {
    workers: BTreeMap<u64, KWorker>,
    work_items: BTreeMap<u64, KWorkItem>,
    workqueues: BTreeMap<u64, KWorkqueue>,
    stats: KThreadPoolStats,
    next_worker_id: u64,
    next_work_id: u64,
    next_wq_id: u64,
    stall_timeout_ns: u64,
}

impl HolisticKthreadPool {
    pub fn new(stall_timeout: u64) -> Self {
        Self {
            workers: BTreeMap::new(), work_items: BTreeMap::new(),
            workqueues: BTreeMap::new(), stats: KThreadPoolStats::default(),
            next_worker_id: 1, next_work_id: 1, next_wq_id: 1,
            stall_timeout_ns: stall_timeout,
        }
    }

    #[inline]
    pub fn create_workqueue(&mut self, name_hash: u64, per_cpu: bool, max_workers: u32) -> u64 {
        let id = self.next_wq_id; self.next_wq_id += 1;
        self.workqueues.insert(id, KWorkqueue::new(id, name_hash, per_cpu, max_workers));
        id
    }

    #[inline]
    pub fn spawn_worker(&mut self, cpu: Option<u32>) -> u64 {
        let id = self.next_worker_id; self.next_worker_id += 1;
        self.workers.insert(id, KWorker::new(id, cpu));
        id
    }

    #[inline]
    pub fn queue_work(&mut self, func_hash: u64, prio: KWorkPriority, flags: KWorkFlags, ts: u64) -> u64 {
        let id = self.next_work_id; self.next_work_id += 1;
        self.work_items.insert(id, KWorkItem::new(id, func_hash, prio, flags, ts));
        id
    }

    #[inline]
    pub fn queue_delayed(&mut self, func_hash: u64, prio: KWorkPriority, flags: KWorkFlags, delay: u64, ts: u64) -> u64 {
        let id = self.next_work_id; self.next_work_id += 1;
        let item = KWorkItem::new(id, func_hash, prio, flags, ts).delayed(delay);
        self.work_items.insert(id, item);
        id
    }

    pub fn schedule(&mut self, now: u64) -> Vec<(u64, u64)> {
        let mut assignments = Vec::new();
        let idle: Vec<u64> = self.workers.values().filter(|w| w.state == KWorkerState::Idle).map(|w| w.id).collect();
        let mut ready: Vec<u64> = self.work_items.values()
            .filter(|w| w.is_ready(now))
            .map(|w| w.id)
            .collect();
        // Sort by priority (higher first)
        ready.sort_by(|a, b| {
            let pa = self.work_items.get(a).map(|w| w.priority).unwrap_or(KWorkPriority::Normal);
            let pb = self.work_items.get(b).map(|w| w.priority).unwrap_or(KWorkPriority::Normal);
            pb.cmp(&pa)
        });
        for (wi, wk) in ready.iter().zip(idle.iter()) {
            if let Some(work) = self.work_items.get_mut(wi) { work.start(*wk, now); }
            if let Some(worker) = self.workers.get_mut(wk) { worker.assign(*wi, now); }
            assignments.push((*wk, *wi));
        }
        assignments
    }

    #[inline]
    pub fn complete_work(&mut self, work_id: u64, ts: u64) {
        if let Some(w) = self.work_items.get_mut(&work_id) {
            let wk = w.worker_id;
            w.complete(ts);
            if let Some(wk_id) = wk { if let Some(worker) = self.workers.get_mut(&wk_id) { worker.finish(ts); } }
        }
    }

    #[inline]
    pub fn detect_stalls(&self, now: u64) -> Vec<u64> {
        self.workers.values()
            .filter(|w| w.state == KWorkerState::Running && now.saturating_sub(w.last_active_ts) > self.stall_timeout_ns)
            .map(|w| w.id)
            .collect()
    }

    pub fn recompute(&mut self) {
        self.stats.total_workers = self.workers.len();
        self.stats.idle_workers = self.workers.values().filter(|w| w.state == KWorkerState::Idle).count();
        self.stats.busy_workers = self.workers.values().filter(|w| w.state == KWorkerState::Running).count();
        self.stats.pending_work = self.work_items.values().filter(|w| matches!(w.status, KWorkStatus::Queued | KWorkStatus::Delayed)).count();
        self.stats.total_completed = self.work_items.values().filter(|w| w.status == KWorkStatus::Complete).count() as u64;
        let done: Vec<&KWorkItem> = self.work_items.values().filter(|w| w.status == KWorkStatus::Complete).collect();
        if !done.is_empty() {
            self.stats.avg_latency_ns = done.iter().map(|w| w.latency()).sum::<u64>() / done.len() as u64;
            self.stats.avg_exec_ns = done.iter().map(|w| w.exec_time()).sum::<u64>() / done.len() as u64;
        }
        self.stats.workqueues = self.workqueues.len();
    }

    #[inline(always)]
    pub fn worker(&self, id: u64) -> Option<&KWorker> { self.workers.get(&id) }
    #[inline(always)]
    pub fn work(&self, id: u64) -> Option<&KWorkItem> { self.work_items.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &KThreadPoolStats { &self.stats }
}
