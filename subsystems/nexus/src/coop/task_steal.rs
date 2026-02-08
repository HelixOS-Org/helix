// SPDX-License-Identifier: GPL-2.0
//! Coop task_steal — work-stealing scheduler for cooperative parallelism.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Task priority for steal ordering
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StealPriority {
    Critical = 0,
    High = 1,
    Normal = 2,
    Low = 3,
    Background = 4,
}

/// Task state in the steal queue
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StealTaskState {
    Queued,
    Running,
    Stolen,
    Completed,
    Cancelled,
    Blocked,
}

/// Steal policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StealPolicy {
    HalfQueue,
    SingleTask,
    PriorityBased,
    LoadBalanced,
    Affinity,
}

/// Steal result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StealResult {
    Success,
    QueueEmpty,
    TargetBusy,
    AffinityConflict,
    PolicyDenied,
}

/// A stealable task descriptor
#[derive(Debug, Clone)]
pub struct StealTask {
    pub id: u64,
    pub priority: StealPriority,
    pub state: StealTaskState,
    pub cpu_affinity: Option<u32>,
    pub owner_queue: u32,
    pub steal_count: u32,
    pub estimated_ns: u64,
    pub enqueued_at: u64,
    pub started_at: u64,
    pub completed_at: u64,
}

impl StealTask {
    pub fn new(id: u64, priority: StealPriority, owner: u32, est_ns: u64, now: u64) -> Self {
        Self {
            id, priority, state: StealTaskState::Queued,
            cpu_affinity: None, owner_queue: owner, steal_count: 0,
            estimated_ns: est_ns, enqueued_at: now, started_at: 0, completed_at: 0,
        }
    }

    pub fn wait_time(&self, now: u64) -> u64 { now.saturating_sub(self.enqueued_at) }

    pub fn execution_time(&self) -> u64 {
        if self.completed_at > 0 && self.started_at > 0 {
            self.completed_at.saturating_sub(self.started_at)
        } else { 0 }
    }

    pub fn start(&mut self, now: u64) {
        self.state = StealTaskState::Running;
        self.started_at = now;
    }

    pub fn complete(&mut self, now: u64) {
        self.state = StealTaskState::Completed;
        self.completed_at = now;
    }

    pub fn mark_stolen(&mut self, new_owner: u32) {
        self.state = StealTaskState::Stolen;
        self.owner_queue = new_owner;
        self.steal_count += 1;
    }
}

/// Per-CPU work queue for the steal scheduler
#[derive(Debug)]
pub struct WorkQueue {
    pub cpu_id: u32,
    pub tasks: Vec<StealTask>,
    pub capacity: u32,
    pub total_enqueued: u64,
    pub total_dequeued: u64,
    pub total_stolen_from: u64,
    pub total_stolen_to: u64,
}

impl WorkQueue {
    pub fn new(cpu_id: u32, capacity: u32) -> Self {
        Self {
            cpu_id, tasks: Vec::new(), capacity,
            total_enqueued: 0, total_dequeued: 0,
            total_stolen_from: 0, total_stolen_to: 0,
        }
    }

    pub fn push(&mut self, task: StealTask) -> bool {
        if self.tasks.len() as u32 >= self.capacity { return false; }
        self.tasks.push(task);
        self.total_enqueued += 1;
        true
    }

    pub fn pop(&mut self) -> Option<StealTask> {
        if self.tasks.is_empty() { return None; }
        self.total_dequeued += 1;
        Some(self.tasks.remove(0))
    }

    pub fn steal_half(&mut self) -> Vec<StealTask> {
        let n = self.tasks.len() / 2;
        if n == 0 { return Vec::new(); }
        let stolen: Vec<_> = self.tasks.drain(self.tasks.len() - n..).collect();
        self.total_stolen_from += stolen.len() as u64;
        stolen
    }

    pub fn steal_one(&mut self) -> Option<StealTask> {
        if self.tasks.is_empty() { return None; }
        self.total_stolen_from += 1;
        Some(self.tasks.remove(self.tasks.len() - 1))
    }

    pub fn steal_by_priority(&mut self, min_priority: StealPriority) -> Vec<StealTask> {
        let mut stolen = Vec::new();
        let mut remaining = Vec::new();
        for t in self.tasks.drain(..) {
            if t.priority <= min_priority { stolen.push(t); }
            else { remaining.push(t); }
        }
        self.tasks = remaining;
        self.total_stolen_from += stolen.len() as u64;
        stolen
    }

    pub fn load(&self) -> u32 { self.tasks.len() as u32 }

    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 { return 0.0; }
        self.tasks.len() as f64 / self.capacity as f64
    }
}

/// Task steal stats
#[derive(Debug, Clone)]
pub struct TaskStealStats {
    pub total_queues: u32,
    pub total_tasks: u64,
    pub total_steals: u64,
    pub avg_queue_load: f64,
    pub max_queue_load: u32,
    pub min_queue_load: u32,
    pub load_imbalance: f64,
}

/// Main work-stealing scheduler
pub struct CoopTaskSteal {
    queues: BTreeMap<u32, WorkQueue>,
    completed: Vec<StealTask>,
    policy: StealPolicy,
    max_completed: usize,
    prng_state: u64,
}

impl CoopTaskSteal {
    pub fn new(policy: StealPolicy) -> Self {
        Self {
            queues: BTreeMap::new(),
            completed: Vec::new(),
            policy,
            max_completed: 4096,
            prng_state: 0xdeadbeefcafe1234,
        }
    }

    fn next_rand(&mut self) -> u64 {
        let mut x = self.prng_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.prng_state = x;
        x
    }

    pub fn add_queue(&mut self, cpu_id: u32, capacity: u32) {
        self.queues.entry(cpu_id).or_insert_with(|| WorkQueue::new(cpu_id, capacity));
    }

    pub fn submit(&mut self, cpu_id: u32, task: StealTask) -> bool {
        self.queues.get_mut(&cpu_id).map(|q| q.push(task)).unwrap_or(false)
    }

    pub fn dequeue(&mut self, cpu_id: u32) -> Option<StealTask> {
        if let Some(task) = self.queues.get_mut(&cpu_id).and_then(|q| q.pop()) {
            return Some(task);
        }
        self.try_steal(cpu_id)
    }

    pub fn try_steal(&mut self, thief_cpu: u32) -> Option<StealTask> {
        let victim_cpu = self.find_victim(thief_cpu)?;
        match self.policy {
            StealPolicy::SingleTask | StealPolicy::Affinity => {
                let mut task = self.queues.get_mut(&victim_cpu)?.steal_one()?;
                task.mark_stolen(thief_cpu);
                if let Some(q) = self.queues.get_mut(&thief_cpu) {
                    q.total_stolen_to += 1;
                }
                Some(task)
            }
            StealPolicy::HalfQueue => {
                let stolen = self.queues.get_mut(&victim_cpu)?.steal_half();
                if stolen.is_empty() { return None; }
                let first = stolen.into_iter().next();
                if let Some(q) = self.queues.get_mut(&thief_cpu) {
                    q.total_stolen_to += 1;
                }
                first.map(|mut t| { t.mark_stolen(thief_cpu); t })
            }
            _ => {
                let mut task = self.queues.get_mut(&victim_cpu)?.steal_one()?;
                task.mark_stolen(thief_cpu);
                Some(task)
            }
        }
    }

    fn find_victim(&mut self, thief_cpu: u32) -> Option<u32> {
        let cpus: Vec<u32> = self.queues.keys().filter(|&&c| c != thief_cpu).copied().collect();
        if cpus.is_empty() { return None; }

        match self.policy {
            StealPolicy::LoadBalanced => {
                cpus.iter()
                    .max_by_key(|&&c| self.queues.get(&c).map(|q| q.load()).unwrap_or(0))
                    .copied()
            }
            _ => {
                let idx = (self.next_rand() as usize) % cpus.len();
                Some(cpus[idx])
            }
        }
    }

    pub fn complete_task(&mut self, mut task: StealTask, now: u64) {
        task.complete(now);
        if self.completed.len() >= self.max_completed {
            self.completed.drain(..self.max_completed / 4);
        }
        self.completed.push(task);
    }

    pub fn balance(&mut self) {
        let loads: Vec<(u32, u32)> = self.queues.iter()
            .map(|(&id, q)| (id, q.load())).collect();
        if loads.len() < 2 { return; }
        let avg = loads.iter().map(|(_, l)| *l as u64).sum::<u64>() / loads.len() as u64;
        let _ = avg; // used for future adaptive balancing
    }

    pub fn stats(&self) -> TaskStealStats {
        let loads: Vec<u32> = self.queues.values().map(|q| q.load()).collect();
        let total_tasks: u64 = loads.iter().map(|&l| l as u64).sum();
        let avg = if loads.is_empty() { 0.0 } else { total_tasks as f64 / loads.len() as f64 };
        let max_load = loads.iter().copied().max().unwrap_or(0);
        let min_load = loads.iter().copied().min().unwrap_or(0);
        let total_steals: u64 = self.queues.values().map(|q| q.total_stolen_from).sum();

        let variance = if loads.is_empty() { 0.0 } else {
            loads.iter().map(|&l| { let d = l as f64 - avg; d * d }).sum::<f64>() / loads.len() as f64
        };

        TaskStealStats {
            total_queues: self.queues.len() as u32,
            total_tasks,
            total_steals,
            avg_queue_load: avg,
            max_queue_load: max_load,
            min_queue_load: min_load,
            load_imbalance: libm::sqrt(variance),
        }
    }
}
