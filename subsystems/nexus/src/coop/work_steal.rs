//! # Cooperative Work-Stealing Scheduler
//!
//! Cooperative work-stealing for balanced task distribution:
//! - Per-worker double-ended queues
//! - Lock-free steal-from-tail protocol
//! - Locality-aware stealing (prefer nearby workers)
//! - Steal-half strategy for batch migration
//! - Adaptive steal intervals with back-off
//! - Worker state and throughput tracking

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Worker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerStateCoop {
    Active,
    Idle,
    Stealing,
    Sleeping,
    Shutdown,
}

/// Steal strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StealStrategyCoop {
    /// Steal one item at a time
    One,
    /// Steal half of the victim's deque
    Half,
    /// Steal up to N items
    Batch(u32),
    /// Adaptive based on load difference
    Adaptive,
}

/// Task item in the work deque
#[derive(Debug, Clone)]
pub struct WorkItem {
    pub id: u64,
    pub priority: i32,
    pub affinity_hint: Option<u32>,
    pub estimated_cost: u64,
    pub created_ns: u64,
}

/// Per-worker deque (double-ended queue)
#[derive(Debug, Clone)]
pub struct WorkerDeque {
    items: Vec<WorkItem>,
    capacity: usize,
}

impl WorkerDeque {
    pub fn new(capacity: usize) -> Self {
        Self { items: Vec::new(), capacity }
    }

    pub fn push_back(&mut self, item: WorkItem) -> bool {
        if self.items.len() >= self.capacity { return false; }
        self.items.push(item);
        true
    }

    pub fn pop_back(&mut self) -> Option<WorkItem> {
        self.items.pop()
    }

    pub fn pop_front(&mut self) -> Option<WorkItem> {
        if self.items.is_empty() { None }
        else { Some(self.items.remove(0)) }
    }

    /// Steal half of the deque items (from front)
    pub fn steal_half(&mut self) -> Vec<WorkItem> {
        let n = self.items.len() / 2;
        if n == 0 && !self.items.is_empty() {
            return alloc::vec![self.items.remove(0)];
        }
        let stolen: Vec<WorkItem> = self.items.drain(..n).collect();
        stolen
    }

    pub fn steal_n(&mut self, count: usize) -> Vec<WorkItem> {
        let n = if count > self.items.len() { self.items.len() } else { count };
        self.items.drain(..n).collect()
    }

    pub fn len(&self) -> usize { self.items.len() }
    pub fn is_empty(&self) -> bool { self.items.is_empty() }

    pub fn total_cost(&self) -> u64 {
        self.items.iter().map(|w| w.estimated_cost).sum()
    }
}

/// Per-worker statistics
#[derive(Debug, Clone, Default)]
pub struct WorkerStats {
    pub tasks_executed: u64,
    pub tasks_stolen_from: u64,
    pub tasks_stolen_to: u64,
    pub steal_attempts: u64,
    pub steal_failures: u64,
    pub idle_ns: u64,
    pub busy_ns: u64,
    pub last_steal_ns: u64,
}

impl WorkerStats {
    pub fn steal_success_rate(&self) -> f64 {
        if self.steal_attempts == 0 { return 0.0; }
        let successes = self.steal_attempts.saturating_sub(self.steal_failures);
        successes as f64 / self.steal_attempts as f64
    }

    pub fn utilization(&self) -> f64 {
        let total = self.busy_ns + self.idle_ns;
        if total == 0 { return 0.0; }
        self.busy_ns as f64 / total as f64
    }
}

/// Worker descriptor
#[derive(Debug, Clone)]
pub struct StealWorker {
    pub worker_id: u32,
    pub state: WorkerStateCoop,
    pub deque: WorkerDeque,
    pub stats: WorkerStats,
    pub numa_node: u32,
    pub backoff_ns: u64,
}

impl StealWorker {
    pub fn new(worker_id: u32, numa_node: u32, deque_capacity: usize) -> Self {
        Self {
            worker_id,
            state: WorkerStateCoop::Idle,
            deque: WorkerDeque::new(deque_capacity),
            stats: WorkerStats::default(),
            numa_node,
            backoff_ns: 1_000,
        }
    }
}

/// Cooperative work-stealing scheduler
pub struct CoopWorkStealer {
    workers: BTreeMap<u32, StealWorker>,
    strategy: StealStrategyCoop,
    max_backoff_ns: u64,
    total_tasks_submitted: u64,
    total_steals: u64,
    rng_state: u64,
}

impl CoopWorkStealer {
    pub fn new(strategy: StealStrategyCoop) -> Self {
        Self {
            workers: BTreeMap::new(),
            strategy,
            max_backoff_ns: 1_000_000,
            total_tasks_submitted: 0,
            total_steals: 0,
            rng_state: 0x123456789abcdef0,
        }
    }

    pub fn add_worker(&mut self, worker_id: u32, numa_node: u32, deque_cap: usize) {
        self.workers.entry(worker_id)
            .or_insert_with(|| StealWorker::new(worker_id, numa_node, deque_cap));
    }

    /// Submit a work item to a specific worker
    pub fn submit(&mut self, worker_id: u32, item: WorkItem) -> bool {
        if let Some(worker) = self.workers.get_mut(&worker_id) {
            if worker.deque.push_back(item) {
                self.total_tasks_submitted += 1;
                true
            } else { false }
        } else { false }
    }

    /// Submit with automatic load balancing — pick least loaded worker
    pub fn submit_balanced(&mut self, item: WorkItem) -> bool {
        let affinity = item.affinity_hint;
        let target = if let Some(hint) = affinity {
            // Prefer affinity-matching worker
            self.workers.values()
                .filter(|w| w.numa_node == hint)
                .min_by_key(|w| w.deque.len())
                .map(|w| w.worker_id)
        } else { None };

        let target = target.unwrap_or_else(|| {
            self.workers.values()
                .min_by_key(|w| w.deque.len())
                .map(|w| w.worker_id)
                .unwrap_or(0)
        });

        self.submit(target, item)
    }

    /// Worker tries to pop local work
    pub fn local_pop(&mut self, worker_id: u32) -> Option<WorkItem> {
        if let Some(worker) = self.workers.get_mut(&worker_id) {
            let item = worker.deque.pop_back();
            if item.is_some() {
                worker.stats.tasks_executed += 1;
                worker.state = WorkerStateCoop::Active;
            }
            item
        } else { None }
    }

    /// Worker attempts to steal from another worker
    pub fn try_steal(&mut self, thief_id: u32, now_ns: u64) -> Vec<WorkItem> {
        if let Some(thief) = self.workers.get_mut(&thief_id) {
            thief.state = WorkerStateCoop::Stealing;
            thief.stats.steal_attempts += 1;
        }

        // Pick a victim — prefer same NUMA first
        let thief_numa = self.workers.get(&thief_id).map(|w| w.numa_node).unwrap_or(0);

        // Collect victim candidates sorted by load (descending)
        let mut candidates: Vec<(u32, usize, u32)> = self.workers.iter()
            .filter(|(&wid, _)| wid != thief_id)
            .filter(|(_, w)| !w.deque.is_empty())
            .map(|(&wid, w)| (wid, w.deque.len(), w.numa_node))
            .collect();

        // Sort: same NUMA first, then by load descending
        candidates.sort_by(|a, b| {
            let a_local = if a.2 == thief_numa { 0 } else { 1 };
            let b_local = if b.2 == thief_numa { 0 } else { 1 };
            a_local.cmp(&b_local).then(b.1.cmp(&a.1))
        });

        if candidates.is_empty() {
            if let Some(thief) = self.workers.get_mut(&thief_id) {
                thief.stats.steal_failures += 1;
                thief.state = WorkerStateCoop::Idle;
                // Back off
                thief.backoff_ns = (thief.backoff_ns * 2).min(self.max_backoff_ns);
            }
            return Vec::new();
        }

        let victim_id = candidates[0].0;
        let stolen = if let Some(victim) = self.workers.get_mut(&victim_id) {
            let items = match self.strategy {
                StealStrategyCoop::One => {
                    if let Some(item) = victim.deque.pop_front() {
                        alloc::vec![item]
                    } else { Vec::new() }
                }
                StealStrategyCoop::Half => victim.deque.steal_half(),
                StealStrategyCoop::Batch(n) => victim.deque.steal_n(n as usize),
                StealStrategyCoop::Adaptive => {
                    let thief_load = self.workers.get(&thief_id).map(|w| w.deque.len()).unwrap_or(0);
                    let diff = victim.deque.len().saturating_sub(thief_load);
                    let count = diff / 2;
                    if count == 0 && !victim.deque.is_empty() {
                        alloc::vec![victim.deque.pop_front().unwrap()]
                    } else {
                        victim.deque.steal_n(count)
                    }
                }
            };
            victim.stats.tasks_stolen_from += items.len() as u64;
            items
        } else { Vec::new() };

        if stolen.is_empty() {
            if let Some(thief) = self.workers.get_mut(&thief_id) {
                thief.stats.steal_failures += 1;
                thief.state = WorkerStateCoop::Idle;
            }
        } else {
            let count = stolen.len() as u64;
            self.total_steals += count;
            if let Some(thief) = self.workers.get_mut(&thief_id) {
                thief.stats.tasks_stolen_to += count;
                thief.stats.last_steal_ns = now_ns;
                thief.backoff_ns = 1_000; // Reset backoff on success
                thief.state = WorkerStateCoop::Active;
            }
        }

        stolen
    }

    fn _next_rng(&mut self) -> u64 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        self.rng_state
    }

    pub fn worker(&self, id: u32) -> Option<&StealWorker> {
        self.workers.get(&id)
    }

    pub fn total_pending(&self) -> usize {
        self.workers.values().map(|w| w.deque.len()).sum()
    }

    pub fn total_steals(&self) -> u64 { self.total_steals }

    pub fn load_imbalance(&self) -> f64 {
        if self.workers.is_empty() { return 0.0; }
        let loads: Vec<usize> = self.workers.values().map(|w| w.deque.len()).collect();
        let mean = loads.iter().sum::<usize>() as f64 / loads.len() as f64;
        if mean < 0.001 { return 0.0; }
        let variance = loads.iter()
            .map(|&l| { let d = l as f64 - mean; d * d })
            .sum::<f64>() / loads.len() as f64;
        libm::sqrt(variance) / mean
    }
}

// ============================================================================
// Merged from work_steal_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkStealV2Policy {
    Random,
    NearestNeighbor,
    NumaLocal,
    LeastLoaded,
    Adaptive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkStealV2TaskState {
    Pending,
    Running,
    Stolen,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct WorkStealV2Task {
    pub id: u64,
    pub priority: u32,
    pub state: WorkStealV2TaskState,
    pub origin_worker: u32,
    pub current_worker: u32,
    pub steal_count: u32,
    pub estimated_cost: u64,
}

impl WorkStealV2Task {
    pub fn new(id: u64, priority: u32, worker: u32, cost: u64) -> Self {
        Self {
            id, priority, state: WorkStealV2TaskState::Pending,
            origin_worker: worker, current_worker: worker,
            steal_count: 0, estimated_cost: cost,
        }
    }

    pub fn steal_to(&mut self, new_worker: u32) {
        self.current_worker = new_worker;
        self.steal_count += 1;
        self.state = WorkStealV2TaskState::Stolen;
    }

    pub fn was_stolen(&self) -> bool { self.steal_count > 0 }
}

#[derive(Debug, Clone)]
pub struct WorkStealV2Deque {
    pub worker_id: u32,
    pub numa_node: u32,
    pub tasks: Vec<WorkStealV2Task>,
    pub total_pushed: u64,
    pub total_popped: u64,
    pub total_stolen_from: u64,
    pub total_stolen_to: u64,
}

impl WorkStealV2Deque {
    pub fn new(worker_id: u32, numa_node: u32) -> Self {
        Self {
            worker_id, numa_node, tasks: Vec::new(),
            total_pushed: 0, total_popped: 0,
            total_stolen_from: 0, total_stolen_to: 0,
        }
    }

    pub fn push(&mut self, task: WorkStealV2Task) {
        self.tasks.push(task);
        self.total_pushed += 1;
    }

    pub fn pop(&mut self) -> Option<WorkStealV2Task> {
        if let Some(t) = self.tasks.pop() {
            self.total_popped += 1;
            Some(t)
        } else { None }
    }

    pub fn steal(&mut self) -> Option<WorkStealV2Task> {
        if self.tasks.is_empty() { return None; }
        let mut task = self.tasks.remove(0);
        task.state = WorkStealV2TaskState::Stolen;
        self.total_stolen_from += 1;
        Some(task)
    }

    pub fn len(&self) -> usize { self.tasks.len() }
    pub fn is_empty(&self) -> bool { self.tasks.is_empty() }

    pub fn steal_rate(&self) -> u64 {
        let total = self.total_popped + self.total_stolen_from;
        if total == 0 { 0 } else { (self.total_stolen_from * 100) / total }
    }
}

#[derive(Debug, Clone)]
pub struct WorkStealV2Stats {
    pub total_workers: u32,
    pub total_tasks: u64,
    pub total_steals: u64,
    pub numa_local_steals: u64,
    pub numa_remote_steals: u64,
    pub avg_queue_depth: u64,
}

pub struct CoopWorkStealV2 {
    deques: BTreeMap<u32, WorkStealV2Deque>,
    policy: WorkStealV2Policy,
    stats: WorkStealV2Stats,
}

impl CoopWorkStealV2 {
    pub fn new(policy: WorkStealV2Policy) -> Self {
        Self {
            deques: BTreeMap::new(),
            policy,
            stats: WorkStealV2Stats {
                total_workers: 0, total_tasks: 0,
                total_steals: 0, numa_local_steals: 0,
                numa_remote_steals: 0, avg_queue_depth: 0,
            },
        }
    }

    pub fn add_worker(&mut self, worker_id: u32, numa_node: u32) {
        self.deques.insert(worker_id, WorkStealV2Deque::new(worker_id, numa_node));
        self.stats.total_workers += 1;
    }

    pub fn push_task(&mut self, worker_id: u32, task: WorkStealV2Task) {
        if let Some(d) = self.deques.get_mut(&worker_id) {
            d.push(task);
            self.stats.total_tasks += 1;
        }
    }

    pub fn try_steal(&mut self, thief_id: u32, victim_id: u32) -> Option<WorkStealV2Task> {
        let victim_numa = self.deques.get(&victim_id).map(|d| d.numa_node)?;
        let thief_numa = self.deques.get(&thief_id).map(|d| d.numa_node)?;
        if let Some(victim) = self.deques.get_mut(&victim_id) {
            if let Some(mut task) = victim.steal() {
                task.steal_to(thief_id);
                self.stats.total_steals += 1;
                if victim_numa == thief_numa {
                    self.stats.numa_local_steals += 1;
                } else {
                    self.stats.numa_remote_steals += 1;
                }
                return Some(task);
            }
        }
        None
    }

    pub fn stats(&self) -> &WorkStealV2Stats { &self.stats }
}
