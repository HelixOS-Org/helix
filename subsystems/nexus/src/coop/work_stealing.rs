// SPDX-License-Identifier: GPL-2.0
//! Coop work_stealing — work-stealing task scheduler.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Task priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WsTaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Task state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsTaskState {
    Queued,
    Running,
    Stolen,
    Completed,
    Cancelled,
}

/// Work-stealing task
#[derive(Debug)]
pub struct WsTask {
    pub id: u64,
    pub priority: WsTaskPriority,
    pub state: WsTaskState,
    pub origin_worker: u64,
    pub current_worker: u64,
    pub stolen: bool,
    pub queued_at: u64,
    pub started_at: u64,
    pub completed_at: u64,
}

impl WsTask {
    pub fn new(id: u64, prio: WsTaskPriority, worker: u64, now: u64) -> Self {
        Self { id, priority: prio, state: WsTaskState::Queued, origin_worker: worker, current_worker: worker, stolen: false, queued_at: now, started_at: 0, completed_at: 0 }
    }

    #[inline(always)]
    pub fn start(&mut self, now: u64) { self.state = WsTaskState::Running; self.started_at = now; }
    #[inline(always)]
    pub fn complete(&mut self, now: u64) { self.state = WsTaskState::Completed; self.completed_at = now; }
    #[inline(always)]
    pub fn steal(&mut self, thief: u64) { self.stolen = true; self.current_worker = thief; self.state = WsTaskState::Stolen; }
    #[inline(always)]
    pub fn latency_ns(&self) -> u64 { if self.completed_at > 0 { self.completed_at - self.queued_at } else { 0 } }
}

/// Worker queue
#[derive(Debug)]
#[repr(align(64))]
pub struct WorkerQueue {
    pub id: u64,
    pub tasks: VecDeque<WsTask>,
    pub total_processed: u64,
    pub total_stolen_from: u64,
    pub total_stolen_to: u64,
    pub idle_ns: u64,
}

impl WorkerQueue {
    pub fn new(id: u64) -> Self {
        Self { id, tasks: VecDeque::new(), total_processed: 0, total_stolen_from: 0, total_stolen_to: 0, idle_ns: 0 }
    }

    #[inline(always)]
    pub fn push(&mut self, task: WsTask) { self.tasks.push_back(task); }
    #[inline(always)]
    pub fn pop(&mut self) -> Option<WsTask> { self.tasks.pop() }
    #[inline]
    pub fn steal(&mut self) -> Option<WsTask> {
        if self.tasks.is_empty() { return None; }
        self.total_stolen_from += 1;
        self.tasks.pop_front()
    }

    #[inline(always)]
    pub fn len(&self) -> usize { self.tasks.len() }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct WorkStealingStats {
    pub total_workers: u32,
    pub total_tasks: u64,
    pub total_steals: u64,
    pub total_processed: u64,
    pub avg_queue_depth: f64,
    pub steal_rate: f64,
}

/// Main work-stealing scheduler
pub struct CoopWorkStealing {
    workers: BTreeMap<u64, WorkerQueue>,
    next_task_id: u64,
    total_steals: u64,
    seed: u64,
}

impl CoopWorkStealing {
    pub fn new() -> Self { Self { workers: BTreeMap::new(), next_task_id: 1, total_steals: 0, seed: 0xabcdef0123456789 } }

    #[inline(always)]
    pub fn add_worker(&mut self, id: u64) { self.workers.insert(id, WorkerQueue::new(id)); }

    #[inline]
    pub fn submit(&mut self, worker: u64, prio: WsTaskPriority, now: u64) -> u64 {
        let id = self.next_task_id; self.next_task_id += 1;
        let task = WsTask::new(id, prio, worker, now);
        if let Some(w) = self.workers.get_mut(&worker) { w.push(task); }
        id
    }

    pub fn try_steal(&mut self, thief_id: u64) -> bool {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        let victims: Vec<u64> = self.workers.keys().filter(|&&id| id != thief_id).copied().collect();
        if victims.is_empty() { return false; }
        let victim_id = victims[self.seed as usize % victims.len()];
        if let Some(victim) = self.workers.get_mut(&victim_id) {
            if let Some(mut task) = victim.steal() {
                task.steal(thief_id);
                if let Some(thief) = self.workers.get_mut(&thief_id) {
                    thief.total_stolen_to += 1;
                    thief.push(task);
                    self.total_steals += 1;
                    return true;
                }
            }
        }
        false
    }

    #[inline]
    pub fn stats(&self) -> WorkStealingStats {
        let processed: u64 = self.workers.values().map(|w| w.total_processed).sum();
        let depths: Vec<f64> = self.workers.values().map(|w| w.len() as f64).collect();
        let avg = if depths.is_empty() { 0.0 } else { depths.iter().sum::<f64>() / depths.len() as f64 };
        let rate = if processed == 0 { 0.0 } else { self.total_steals as f64 / processed as f64 };
        WorkStealingStats { total_workers: self.workers.len() as u32, total_tasks: self.next_task_id - 1, total_steals: self.total_steals, total_processed: processed, avg_queue_depth: avg, steal_rate: rate }
    }
}

// ============================================================================
// Merged from work_stealing_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StealPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Steal task
#[derive(Debug)]
pub struct StealTask {
    pub id: u64,
    pub priority: StealPriority,
    pub cost_estimate: u64,
    pub created_at: u64,
    pub stolen: bool,
    pub steal_count: u32,
}

impl StealTask {
    pub fn new(id: u64, prio: StealPriority, cost: u64, now: u64) -> Self {
        Self { id, priority: prio, cost_estimate: cost, created_at: now, stolen: false, steal_count: 0 }
    }
}

/// Worker deque
#[derive(Debug)]
pub struct WorkerDeque {
    pub worker_id: u64,
    pub tasks: VecDeque<StealTask>,
    pub total_pushed: u64,
    pub total_popped: u64,
    pub total_stolen_from: u64,
    pub total_stolen_to: u64,
}

impl WorkerDeque {
    pub fn new(id: u64) -> Self {
        Self { worker_id: id, tasks: VecDeque::new(), total_pushed: 0, total_popped: 0, total_stolen_from: 0, total_stolen_to: 0 }
    }

    #[inline(always)]
    pub fn push(&mut self, task: StealTask) { self.total_pushed += 1; self.tasks.push_back(task); }

    #[inline(always)]
    pub fn pop(&mut self) -> Option<StealTask> { self.total_popped += 1; self.tasks.pop() }

    #[inline]
    pub fn steal(&mut self) -> Option<StealTask> {
        if self.tasks.is_empty() { return None; }
        self.total_stolen_from += 1;
        let mut task = self.tasks.pop_front().unwrap();
        task.stolen = true;
        task.steal_count += 1;
        Some(task)
    }

    #[inline(always)]
    pub fn load(&self) -> u64 { self.tasks.iter().map(|t| t.cost_estimate).sum() }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct WorkStealingV2Stats {
    pub total_workers: u32,
    pub total_tasks: u32,
    pub total_steals: u64,
    pub load_imbalance: f64,
    pub avg_deque_len: f64,
}

/// Main coop work stealing v2
pub struct CoopWorkStealingV2 {
    workers: BTreeMap<u64, WorkerDeque>,
}

impl CoopWorkStealingV2 {
    pub fn new() -> Self { Self { workers: BTreeMap::new() } }

    #[inline(always)]
    pub fn add_worker(&mut self, id: u64) { self.workers.insert(id, WorkerDeque::new(id)); }

    #[inline(always)]
    pub fn push(&mut self, worker: u64, task: StealTask) {
        if let Some(w) = self.workers.get_mut(&worker) { w.push(task); }
    }

    #[inline(always)]
    pub fn pop(&mut self, worker: u64) -> Option<StealTask> {
        self.workers.get_mut(&worker).and_then(|w| w.pop())
    }

    pub fn try_steal(&mut self, thief: u64) -> Option<StealTask> {
        let victim = {
            self.workers.iter()
                .filter(|(&id, _)| id != thief)
                .max_by_key(|(_, w)| w.tasks.len())
                .map(|(&id, _)| id)
        };
        if let Some(vid) = victim {
            if let Some(task) = self.workers.get_mut(&vid).and_then(|w| w.steal()) {
                if let Some(tw) = self.workers.get_mut(&thief) { tw.total_stolen_to += 1; }
                return Some(task);
            }
        }
        None
    }

    #[inline]
    pub fn stats(&self) -> WorkStealingV2Stats {
        let loads: Vec<u64> = self.workers.values().map(|w| w.load()).collect();
        let total_tasks: u32 = self.workers.values().map(|w| w.tasks.len() as u32).sum();
        let steals: u64 = self.workers.values().map(|w| w.total_stolen_from).sum();
        let avg_load = if loads.is_empty() { 0.0 } else { loads.iter().sum::<u64>() as f64 / loads.len() as f64 };
        let max_load = loads.iter().copied().max().unwrap_or(0) as f64;
        let imbalance = if avg_load < 1.0 { 0.0 } else { (max_load - avg_load) / avg_load };
        let avg_len = if self.workers.is_empty() { 0.0 } else { total_tasks as f64 / self.workers.len() as f64 };
        WorkStealingV2Stats { total_workers: self.workers.len() as u32, total_tasks, total_steals: steals, load_imbalance: imbalance, avg_deque_len: avg_len }
    }
}
