//! # Apps Thread Pool
//!
//! Thread pool detection and behavioral analysis:
//! - Automatic pool detection from thread naming/behavior
//! - Pool sizing analysis (over/under provisioned)
//! - Worker utilization tracking
//! - Queue depth estimation
//! - Task distribution fairness
//! - Pool type classification (CPU-bound, IO-bound, mixed)

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

/// Pool type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolType {
    /// Compute-bound workers
    CpuBound,
    /// IO-bound workers
    IoBound,
    /// Mixed workload
    Mixed,
    /// Event-loop style (few threads, high throughput)
    EventLoop,
    /// Unknown/unclassified
    Unknown,
}

/// Worker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerState {
    /// Running user code
    Active,
    /// Waiting for work (idle in pool)
    Idle,
    /// Blocked on IO
    BlockedIo,
    /// Blocked on sync
    BlockedSync,
    /// Parked (not in pool queue)
    Parked,
}

/// Individual worker stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct WorkerStats {
    pub tid: u64,
    pub name: String,
    pub state: WorkerState,
    pub tasks_completed: u64,
    pub total_busy_ns: u64,
    pub total_idle_ns: u64,
    pub total_blocked_ns: u64,
    pub current_task_start_ns: u64,
    /// Voluntary context switches
    pub vol_switches: u64,
    /// Involuntary context switches
    pub invol_switches: u64,
    pub cpu_id: u32,
    pub last_state_change_ns: u64,
}

impl WorkerStats {
    pub fn new(tid: u64) -> Self {
        Self {
            tid,
            name: String::new(),
            state: WorkerState::Idle,
            tasks_completed: 0,
            total_busy_ns: 0,
            total_idle_ns: 0,
            total_blocked_ns: 0,
            current_task_start_ns: 0,
            vol_switches: 0,
            invol_switches: 0,
            cpu_id: 0,
            last_state_change_ns: 0,
        }
    }

    /// Utilization ratio
    #[inline]
    pub fn utilization(&self) -> f64 {
        let total = self.total_busy_ns + self.total_idle_ns + self.total_blocked_ns;
        if total == 0 {
            return 0.0;
        }
        self.total_busy_ns as f64 / total as f64
    }

    /// Block ratio
    #[inline]
    pub fn block_ratio(&self) -> f64 {
        let total = self.total_busy_ns + self.total_idle_ns + self.total_blocked_ns;
        if total == 0 {
            return 0.0;
        }
        self.total_blocked_ns as f64 / total as f64
    }

    /// Is this worker a CPU-bound profile?
    #[inline(always)]
    pub fn is_cpu_bound(&self) -> bool {
        self.utilization() > 0.7 && self.block_ratio() < 0.1
    }

    /// Is this worker an IO-bound profile?
    #[inline(always)]
    pub fn is_io_bound(&self) -> bool {
        self.block_ratio() > 0.4
    }

    /// Transition state
    pub fn transition(&mut self, new_state: WorkerState, now_ns: u64) {
        let elapsed = now_ns.saturating_sub(self.last_state_change_ns);
        match self.state {
            WorkerState::Active => self.total_busy_ns += elapsed,
            WorkerState::Idle | WorkerState::Parked => self.total_idle_ns += elapsed,
            WorkerState::BlockedIo | WorkerState::BlockedSync => self.total_blocked_ns += elapsed,
        }
        if self.state == WorkerState::Active && new_state != WorkerState::Active {
            self.tasks_completed += 1;
        }
        self.state = new_state;
        self.last_state_change_ns = now_ns;
    }
}

/// Detected thread pool
#[derive(Debug)]
#[repr(align(64))]
pub struct DetectedPool {
    pub pool_id: u64,
    pub name: String,
    pub pool_type: PoolType,
    workers: BTreeMap<u64, WorkerStats>,
    /// Estimated queue depth
    pub queue_depth: u64,
    /// Peak queue depth observed
    pub peak_queue_depth: u64,
    pub creation_ns: u64,
}

impl DetectedPool {
    pub fn new(pool_id: u64, name: String) -> Self {
        Self {
            pool_id,
            name,
            pool_type: PoolType::Unknown,
            workers: BTreeMap::new(),
            queue_depth: 0,
            peak_queue_depth: 0,
            creation_ns: 0,
        }
    }

    #[inline(always)]
    pub fn add_worker(&mut self, tid: u64) {
        self.workers.entry(tid).or_insert_with(|| WorkerStats::new(tid));
    }

    #[inline(always)]
    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    #[inline(always)]
    pub fn active_workers(&self) -> usize {
        self.workers.values().filter(|w| w.state == WorkerState::Active).count()
    }

    #[inline]
    pub fn idle_workers(&self) -> usize {
        self.workers.values()
            .filter(|w| matches!(w.state, WorkerState::Idle | WorkerState::Parked))
            .count()
    }

    /// Average worker utilization
    #[inline]
    pub fn avg_utilization(&self) -> f64 {
        if self.workers.is_empty() {
            return 0.0;
        }
        self.workers.values().map(|w| w.utilization()).sum::<f64>()
            / self.workers.len() as f64
    }

    /// Task distribution fairness (Jain's fairness index)
    pub fn task_fairness(&self) -> f64 {
        if self.workers.is_empty() {
            return 1.0;
        }
        let n = self.workers.len() as f64;
        let tasks: Vec<f64> = self.workers.values()
            .map(|w| w.tasks_completed as f64)
            .collect();
        let sum: f64 = tasks.iter().sum();
        let sum_sq: f64 = tasks.iter().map(|t| t * t).sum();
        if sum_sq == 0.0 {
            return 1.0;
        }
        (sum * sum) / (n * sum_sq)
    }

    /// Classify pool type based on worker behavior
    pub fn classify(&mut self) {
        if self.workers.is_empty() {
            self.pool_type = PoolType::Unknown;
            return;
        }
        let cpu_bound = self.workers.values().filter(|w| w.is_cpu_bound()).count();
        let io_bound = self.workers.values().filter(|w| w.is_io_bound()).count();
        let total = self.workers.len();

        if self.workers.len() <= 2 {
            let total_tasks: u64 = self.workers.values().map(|w| w.tasks_completed).sum();
            if total_tasks > 1000 {
                self.pool_type = PoolType::EventLoop;
                return;
            }
        }

        if cpu_bound * 2 > total {
            self.pool_type = PoolType::CpuBound;
        } else if io_bound * 2 > total {
            self.pool_type = PoolType::IoBound;
        } else {
            self.pool_type = PoolType::Mixed;
        }
    }

    /// Is the pool over-provisioned?
    #[inline(always)]
    pub fn is_over_provisioned(&self) -> bool {
        self.avg_utilization() < 0.3 && self.workers.len() > 2
    }

    /// Is the pool under-provisioned?
    #[inline(always)]
    pub fn is_under_provisioned(&self) -> bool {
        self.avg_utilization() > 0.9 && self.queue_depth > self.workers.len() as u64
    }

    /// Suggested pool size
    pub fn suggested_size(&self) -> usize {
        if self.workers.is_empty() {
            return 1;
        }
        let util = self.avg_utilization();
        let current = self.workers.len();
        if util < 0.2 {
            let new_size = (current as f64 * util * 2.0) as usize;
            if new_size < 1 { 1 } else { new_size }
        } else if util > 0.9 {
            let growth = 1.0 + (self.queue_depth as f64 / current as f64).min(2.0);
            (current as f64 * growth) as usize
        } else {
            current
        }
    }

    #[inline(always)]
    pub fn get_worker(&mut self, tid: u64) -> Option<&mut WorkerStats> {
        self.workers.get_mut(&tid)
    }
}

/// Thread pool profiler stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppThreadPoolStats {
    pub detected_pools: usize,
    pub total_workers: usize,
    pub avg_utilization: f64,
    pub over_provisioned: usize,
    pub under_provisioned: usize,
    pub cpu_bound_pools: usize,
    pub io_bound_pools: usize,
}

/// App Thread Pool Profiler
#[repr(align(64))]
pub struct AppThreadPoolProfiler {
    pools: BTreeMap<u64, DetectedPool>,
    /// TID to pool mapping
    tid_to_pool: LinearMap<u64, 64>,
    stats: AppThreadPoolStats,
    next_pool_id: u64,
}

impl AppThreadPoolProfiler {
    pub fn new() -> Self {
        Self {
            pools: BTreeMap::new(),
            tid_to_pool: LinearMap::new(),
            stats: AppThreadPoolStats::default(),
            next_pool_id: 1,
        }
    }

    /// Register a pool
    #[inline]
    pub fn register_pool(&mut self, name: String) -> u64 {
        let id = self.next_pool_id;
        self.next_pool_id += 1;
        self.pools.insert(id, DetectedPool::new(id, name));
        id
    }

    /// Add worker to pool
    #[inline]
    pub fn add_worker(&mut self, pool_id: u64, tid: u64) {
        if let Some(pool) = self.pools.get_mut(&pool_id) {
            pool.add_worker(tid);
            self.tid_to_pool.insert(tid, pool_id);
        }
    }

    /// Record worker state transition
    #[inline]
    pub fn worker_transition(&mut self, tid: u64, state: WorkerState, now_ns: u64) {
        if let Some(&pool_id) = self.tid_to_pool.get(tid) {
            if let Some(pool) = self.pools.get_mut(&pool_id) {
                if let Some(worker) = pool.get_worker(tid) {
                    worker.transition(state, now_ns);
                }
            }
        }
    }

    /// Classify all pools
    #[inline]
    pub fn classify_all(&mut self) {
        for pool in self.pools.values_mut() {
            pool.classify();
        }
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.detected_pools = self.pools.len();
        self.stats.total_workers = self.pools.values().map(|p| p.worker_count()).sum();
        if !self.pools.is_empty() {
            self.stats.avg_utilization = self.pools.values()
                .map(|p| p.avg_utilization())
                .sum::<f64>() / self.pools.len() as f64;
        }
        self.stats.over_provisioned = self.pools.values()
            .filter(|p| p.is_over_provisioned()).count();
        self.stats.under_provisioned = self.pools.values()
            .filter(|p| p.is_under_provisioned()).count();
        self.stats.cpu_bound_pools = self.pools.values()
            .filter(|p| p.pool_type == PoolType::CpuBound).count();
        self.stats.io_bound_pools = self.pools.values()
            .filter(|p| p.pool_type == PoolType::IoBound).count();
    }

    #[inline(always)]
    pub fn stats(&self) -> &AppThreadPoolStats {
        &self.stats
    }
}

// ============================================================================
// Merged from thread_pool_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerStateV2 {
    Idle,
    Running,
    Parked,
    Stealing,
    Terminating,
    Terminated,
}

/// Task priority for the thread pool
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PoolTaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolTaskStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Thread pool task
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PoolTask {
    pub id: u64,
    pub priority: PoolTaskPriority,
    pub status: PoolTaskStatus,
    pub submit_ns: u64,
    pub start_ns: u64,
    pub complete_ns: u64,
    pub affinity_hint: Option<u32>,
    pub estimated_cost_us: u64,
}

impl PoolTask {
    pub fn new(id: u64, priority: PoolTaskPriority, ts: u64) -> Self {
        Self {
            id, priority, status: PoolTaskStatus::Queued,
            submit_ns: ts, start_ns: 0, complete_ns: 0,
            affinity_hint: None, estimated_cost_us: 0,
        }
    }

    #[inline(always)]
    pub fn start(&mut self, ts: u64) { self.status = PoolTaskStatus::Running; self.start_ns = ts; }
    #[inline(always)]
    pub fn complete(&mut self, ts: u64) { self.status = PoolTaskStatus::Completed; self.complete_ns = ts; }
    #[inline(always)]
    pub fn fail(&mut self, ts: u64) { self.status = PoolTaskStatus::Failed; self.complete_ns = ts; }

    #[inline(always)]
    pub fn queue_latency(&self) -> u64 {
        if self.start_ns > self.submit_ns { self.start_ns - self.submit_ns } else { 0 }
    }
    #[inline(always)]
    pub fn execution_time(&self) -> u64 {
        if self.complete_ns > self.start_ns { self.complete_ns - self.start_ns } else { 0 }
    }
}

/// Worker thread (V2)
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PoolWorkerV2 {
    pub id: u32,
    pub thread_id: u64,
    pub state: WorkerStateV2,
    pub cpu_affinity: Option<u32>,
    pub local_queue: VecDeque<PoolTask>,
    pub tasks_completed: u64,
    pub tasks_stolen: u64,
    pub total_busy_ns: u64,
    pub total_idle_ns: u64,
    pub last_state_change_ns: u64,
    pub current_task: Option<u64>,
}

impl PoolWorkerV2 {
    pub fn new(id: u32, tid: u64, ts: u64) -> Self {
        Self {
            id, thread_id: tid, state: WorkerStateV2::Idle,
            cpu_affinity: None, local_queue: VecDeque::new(),
            tasks_completed: 0, tasks_stolen: 0,
            total_busy_ns: 0, total_idle_ns: 0,
            last_state_change_ns: ts, current_task: None,
        }
    }

    #[inline(always)]
    pub fn push_task(&mut self, task: PoolTask) { self.local_queue.push_back(task); }

    #[inline]
    pub fn pop_task(&mut self) -> Option<PoolTask> {
        if self.local_queue.is_empty() { return None; }
        let mut best_idx = 0;
        for i in 1..self.local_queue.len() {
            if self.local_queue[i].priority > self.local_queue[best_idx].priority { best_idx = i; }
        }
        Some(self.local_queue.remove(best_idx))
    }

    #[inline(always)]
    pub fn steal_task(&mut self) -> Option<PoolTask> {
        if self.local_queue.is_empty() { return None; }
        self.local_queue.pop_front()
    }

    #[inline]
    pub fn update_state(&mut self, new_state: WorkerStateV2, ts: u64) {
        let elapsed = ts.saturating_sub(self.last_state_change_ns);
        match self.state {
            WorkerStateV2::Running | WorkerStateV2::Stealing => self.total_busy_ns += elapsed,
            WorkerStateV2::Idle | WorkerStateV2::Parked => self.total_idle_ns += elapsed,
            _ => {}
        }
        self.state = new_state;
        self.last_state_change_ns = ts;
    }

    #[inline]
    pub fn utilization(&self) -> f64 {
        let total = self.total_busy_ns + self.total_idle_ns;
        if total == 0 { return 0.0; }
        self.total_busy_ns as f64 / total as f64
    }

    #[inline(always)]
    pub fn queue_depth(&self) -> usize { self.local_queue.len() }
}

/// Pool sizing strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolSizingStrategy {
    Fixed,
    Adaptive,
    CorePerWorker,
    LoadBased,
}

/// Thread pool stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppsThreadPoolV2Stats {
    pub worker_count: usize,
    pub active_workers: usize,
    pub parked_workers: usize,
    pub global_queue_depth: usize,
    pub total_tasks_submitted: u64,
    pub total_tasks_completed: u64,
    pub total_tasks_stolen: u64,
    pub avg_utilization: f64,
}

/// Apps Thread Pool Manager V2
#[repr(align(64))]
pub struct AppsThreadPoolV2 {
    workers: BTreeMap<u32, PoolWorkerV2>,
    global_queue: Vec<PoolTask>,
    min_workers: u32,
    max_workers: u32,
    sizing: PoolSizingStrategy,
    global_cap: usize,
    stats: AppsThreadPoolV2Stats,
    next_task_id: u64,
    next_worker_id: u32,
    total_submitted: u64,
}

impl AppsThreadPoolV2 {
    pub fn new(min: u32, max: u32, sizing: PoolSizingStrategy, global_cap: usize) -> Self {
        Self {
            workers: BTreeMap::new(),
            global_queue: Vec::new(),
            min_workers: min,
            max_workers: max,
            sizing,
            global_cap,
            stats: AppsThreadPoolV2Stats::default(),
            next_task_id: 1,
            next_worker_id: 0,
            total_submitted: 0,
        }
    }

    #[inline]
    pub fn spawn_worker(&mut self, tid: u64, ts: u64) -> u32 {
        let id = self.next_worker_id;
        self.next_worker_id += 1;
        self.workers.insert(id, PoolWorkerV2::new(id, tid, ts));
        id
    }

    pub fn submit(&mut self, priority: PoolTaskPriority, ts: u64) -> u64 {
        let id = self.next_task_id;
        self.next_task_id += 1;
        let task = PoolTask::new(id, priority, ts);
        self.total_submitted += 1;

        let idle_worker = self.workers.values().find(|w| w.state == WorkerStateV2::Idle).map(|w| w.id);
        if let Some(wid) = idle_worker {
            if let Some(worker) = self.workers.get_mut(&wid) {
                worker.push_task(task);
                return id;
            }
        }
        if self.global_queue.len() < self.global_cap {
            self.global_queue.push(task);
        }
        id
    }

    pub fn worker_pick_task(&mut self, worker_id: u32, ts: u64) -> Option<u64> {
        if let Some(worker) = self.workers.get_mut(&worker_id) {
            if let Some(mut task) = worker.pop_task() {
                task.start(ts);
                let id = task.id;
                worker.current_task = Some(id);
                worker.update_state(WorkerStateV2::Running, ts);
                return Some(id);
            }
        }
        if !self.global_queue.is_empty() {
            let mut best = 0;
            for i in 1..self.global_queue.len() {
                if self.global_queue[i].priority > self.global_queue[best].priority { best = i; }
            }
            let mut task = self.global_queue.remove(best);
            task.start(ts);
            let id = task.id;
            if let Some(worker) = self.workers.get_mut(&worker_id) {
                worker.current_task = Some(id);
                worker.update_state(WorkerStateV2::Running, ts);
            }
            return Some(id);
        }
        let other_ids: Vec<u32> = self.workers.keys().filter(|&&wid| wid != worker_id).copied().collect();
        for oid in other_ids {
            let stolen = self.workers.get_mut(&oid).and_then(|w| w.steal_task());
            if let Some(mut task) = stolen {
                task.start(ts);
                let id = task.id;
                if let Some(worker) = self.workers.get_mut(&worker_id) {
                    worker.current_task = Some(id);
                    worker.tasks_stolen += 1;
                    worker.update_state(WorkerStateV2::Stealing, ts);
                }
                return Some(id);
            }
        }
        None
    }

    #[inline]
    pub fn worker_complete_task(&mut self, worker_id: u32, ts: u64) {
        if let Some(worker) = self.workers.get_mut(&worker_id) {
            worker.tasks_completed += 1;
            worker.current_task = None;
            worker.update_state(WorkerStateV2::Idle, ts);
        }
    }

    #[inline(always)]
    pub fn park_worker(&mut self, worker_id: u32, ts: u64) {
        if let Some(w) = self.workers.get_mut(&worker_id) { w.update_state(WorkerStateV2::Parked, ts); }
    }

    #[inline(always)]
    pub fn unpark_worker(&mut self, worker_id: u32, ts: u64) {
        if let Some(w) = self.workers.get_mut(&worker_id) { w.update_state(WorkerStateV2::Idle, ts); }
    }

    pub fn recompute(&mut self) {
        self.stats.worker_count = self.workers.len();
        self.stats.active_workers = self.workers.values()
            .filter(|w| w.state == WorkerStateV2::Running || w.state == WorkerStateV2::Stealing).count();
        self.stats.parked_workers = self.workers.values().filter(|w| w.state == WorkerStateV2::Parked).count();
        self.stats.global_queue_depth = self.global_queue.len();
        self.stats.total_tasks_submitted = self.total_submitted;
        self.stats.total_tasks_completed = self.workers.values().map(|w| w.tasks_completed).sum();
        self.stats.total_tasks_stolen = self.workers.values().map(|w| w.tasks_stolen).sum();
        let utils: Vec<f64> = self.workers.values().map(|w| w.utilization()).collect();
        self.stats.avg_utilization = if utils.is_empty() { 0.0 }
            else { utils.iter().sum::<f64>() / utils.len() as f64 };
    }

    #[inline(always)]
    pub fn worker(&self, id: u32) -> Option<&PoolWorkerV2> { self.workers.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &AppsThreadPoolV2Stats { &self.stats }
}
