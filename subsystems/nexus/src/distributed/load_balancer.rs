//! # Load Balancer
//!
//! Year 3 EVOLUTION - Load balancing for distributed evolution

#![allow(dead_code)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// LOAD BALANCER TYPES
// ============================================================================

/// Worker ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkerId(pub u64);

/// Task ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(pub u64);

static TASK_COUNTER: AtomicU64 = AtomicU64::new(1);

impl TaskId {
    pub fn generate() -> Self {
        Self(TASK_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

/// Worker information
#[derive(Debug, Clone)]
pub struct WorkerInfo {
    /// ID
    pub id: WorkerId,
    /// Name
    pub name: String,
    /// Capacity (max concurrent tasks)
    pub capacity: usize,
    /// Current load
    pub current_load: usize,
    /// Weight (for weighted algorithms)
    pub weight: u32,
    /// Is healthy
    pub healthy: bool,
    /// Last heartbeat
    pub last_heartbeat: u64,
    /// Total tasks processed
    pub tasks_processed: u64,
    /// Average task time (ns)
    pub avg_task_time: u64,
    /// Current tasks
    pub current_tasks: Vec<TaskId>,
}

impl WorkerInfo {
    pub fn new(id: WorkerId, name: String, capacity: usize, weight: u32) -> Self {
        Self {
            id,
            name,
            capacity,
            current_load: 0,
            weight,
            healthy: true,
            last_heartbeat: 0,
            tasks_processed: 0,
            avg_task_time: 0,
            current_tasks: Vec::new(),
        }
    }

    /// Get load ratio
    pub fn load_ratio(&self) -> f64 {
        if self.capacity == 0 {
            1.0
        } else {
            self.current_load as f64 / self.capacity as f64
        }
    }

    /// Is available?
    pub fn is_available(&self) -> bool {
        self.healthy && self.current_load < self.capacity
    }
}

/// Task information
#[derive(Debug, Clone)]
pub struct TaskInfo {
    /// ID
    pub id: TaskId,
    /// Priority
    pub priority: u32,
    /// Estimated duration (ns)
    pub estimated_duration: u64,
    /// Assigned worker
    pub assigned_to: Option<WorkerId>,
    /// Status
    pub status: TaskStatus,
    /// Created at
    pub created_at: u64,
    /// Started at
    pub started_at: Option<u64>,
    /// Completed at
    pub completed_at: Option<u64>,
    /// Retries
    pub retries: u32,
    /// Max retries
    pub max_retries: u32,
    /// Affinity (preferred workers)
    pub affinity: Vec<WorkerId>,
}

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Assigned,
    Running,
    Completed,
    Failed,
    Cancelled,
}

// ============================================================================
// LOAD BALANCING STRATEGIES
// ============================================================================

/// Load balancing strategy trait
pub trait LoadBalancingStrategy: Send + Sync {
    /// Select worker for task
    fn select_worker(&self, task: &TaskInfo, workers: &[&WorkerInfo]) -> Option<WorkerId>;

    /// Strategy name
    fn name(&self) -> &str;
}

/// Round-robin strategy
pub struct RoundRobinStrategy {
    counter: AtomicU64,
}

impl RoundRobinStrategy {
    pub fn new() -> Self {
        Self {
            counter: AtomicU64::new(0),
        }
    }
}

impl Default for RoundRobinStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl LoadBalancingStrategy for RoundRobinStrategy {
    fn select_worker(&self, _task: &TaskInfo, workers: &[&WorkerInfo]) -> Option<WorkerId> {
        let available: Vec<_> = workers.iter().filter(|w| w.is_available()).collect();
        if available.is_empty() {
            return None;
        }

        let idx = self.counter.fetch_add(1, Ordering::Relaxed) as usize % available.len();
        Some(available[idx].id)
    }

    fn name(&self) -> &str {
        "RoundRobin"
    }
}

/// Least connections strategy
pub struct LeastConnectionsStrategy;

impl LoadBalancingStrategy for LeastConnectionsStrategy {
    fn select_worker(&self, _task: &TaskInfo, workers: &[&WorkerInfo]) -> Option<WorkerId> {
        workers
            .iter()
            .filter(|w| w.is_available())
            .min_by_key(|w| w.current_load)
            .map(|w| w.id)
    }

    fn name(&self) -> &str {
        "LeastConnections"
    }
}

/// Weighted round-robin strategy
pub struct WeightedRoundRobinStrategy {
    counter: AtomicU64,
}

impl WeightedRoundRobinStrategy {
    pub fn new() -> Self {
        Self {
            counter: AtomicU64::new(0),
        }
    }
}

impl Default for WeightedRoundRobinStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl LoadBalancingStrategy for WeightedRoundRobinStrategy {
    fn select_worker(&self, _task: &TaskInfo, workers: &[&WorkerInfo]) -> Option<WorkerId> {
        let available: Vec<_> = workers.iter().filter(|w| w.is_available()).collect();
        if available.is_empty() {
            return None;
        }

        // Build weighted list
        let total_weight: u32 = available.iter().map(|w| w.weight).sum();
        if total_weight == 0 {
            return Some(available[0].id);
        }

        let counter = self.counter.fetch_add(1, Ordering::Relaxed);
        let target = (counter as u32) % total_weight;

        let mut cumulative = 0;
        for worker in available {
            cumulative += worker.weight;
            if target < cumulative {
                return Some(worker.id);
            }
        }

        None
    }

    fn name(&self) -> &str {
        "WeightedRoundRobin"
    }
}

/// Least response time strategy
pub struct LeastResponseTimeStrategy;

impl LoadBalancingStrategy for LeastResponseTimeStrategy {
    fn select_worker(&self, _task: &TaskInfo, workers: &[&WorkerInfo]) -> Option<WorkerId> {
        workers
            .iter()
            .filter(|w| w.is_available())
            .min_by_key(|w| w.avg_task_time)
            .map(|w| w.id)
    }

    fn name(&self) -> &str {
        "LeastResponseTime"
    }
}

/// Random strategy
pub struct RandomStrategy {
    random_state: AtomicU64,
}

impl RandomStrategy {
    pub fn new() -> Self {
        Self {
            random_state: AtomicU64::new(0xDEADBEEF),
        }
    }

    fn random(&self) -> u64 {
        let mut x = self.random_state.load(Ordering::Relaxed);
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.random_state.store(x, Ordering::Relaxed);
        x
    }
}

impl Default for RandomStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl LoadBalancingStrategy for RandomStrategy {
    fn select_worker(&self, _task: &TaskInfo, workers: &[&WorkerInfo]) -> Option<WorkerId> {
        let available: Vec<_> = workers.iter().filter(|w| w.is_available()).collect();
        if available.is_empty() {
            return None;
        }

        let idx = (self.random() as usize) % available.len();
        Some(available[idx].id)
    }

    fn name(&self) -> &str {
        "Random"
    }
}

/// Affinity-aware strategy
pub struct AffinityStrategy {
    fallback: Box<dyn LoadBalancingStrategy>,
}

impl AffinityStrategy {
    pub fn new(fallback: Box<dyn LoadBalancingStrategy>) -> Self {
        Self { fallback }
    }
}

impl LoadBalancingStrategy for AffinityStrategy {
    fn select_worker(&self, task: &TaskInfo, workers: &[&WorkerInfo]) -> Option<WorkerId> {
        // Try affinity workers first
        for &preferred in &task.affinity {
            if let Some(worker) = workers
                .iter()
                .find(|w| w.id == preferred && w.is_available())
            {
                return Some(worker.id);
            }
        }

        // Fallback to other strategy
        self.fallback.select_worker(task, workers)
    }

    fn name(&self) -> &str {
        "Affinity"
    }
}

/// Power of two choices strategy
pub struct PowerOfTwoChoicesStrategy {
    random_state: AtomicU64,
}

impl PowerOfTwoChoicesStrategy {
    pub fn new() -> Self {
        Self {
            random_state: AtomicU64::new(0xCAFEBABE),
        }
    }

    fn random(&self) -> u64 {
        let mut x = self.random_state.load(Ordering::Relaxed);
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.random_state.store(x, Ordering::Relaxed);
        x
    }
}

impl Default for PowerOfTwoChoicesStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl LoadBalancingStrategy for PowerOfTwoChoicesStrategy {
    fn select_worker(&self, _task: &TaskInfo, workers: &[&WorkerInfo]) -> Option<WorkerId> {
        let available: Vec<_> = workers.iter().filter(|w| w.is_available()).collect();
        if available.is_empty() {
            return None;
        }
        if available.len() == 1 {
            return Some(available[0].id);
        }

        // Pick two random workers
        let idx1 = (self.random() as usize) % available.len();
        let mut idx2 = (self.random() as usize) % available.len();
        if idx2 == idx1 {
            idx2 = (idx1 + 1) % available.len();
        }

        // Choose the one with lower load
        if available[idx1].current_load <= available[idx2].current_load {
            Some(available[idx1].id)
        } else {
            Some(available[idx2].id)
        }
    }

    fn name(&self) -> &str {
        "PowerOfTwoChoices"
    }
}

// ============================================================================
// LOAD BALANCER
// ============================================================================

/// Load balancer
pub struct LoadBalancer {
    /// Workers
    workers: BTreeMap<WorkerId, WorkerInfo>,
    /// Pending tasks
    pending_tasks: Vec<TaskInfo>,
    /// Active tasks
    active_tasks: BTreeMap<TaskId, TaskInfo>,
    /// Completed tasks
    completed_tasks: Vec<TaskInfo>,
    /// Strategy
    strategy: Box<dyn LoadBalancingStrategy>,
    /// Health check interval
    health_check_interval: u64,
    /// Current tick
    tick: u64,
    /// Stats
    stats: LoadBalancerStats,
}

/// Load balancer statistics
#[derive(Debug, Clone, Default)]
pub struct LoadBalancerStats {
    /// Total tasks assigned
    pub tasks_assigned: u64,
    /// Total tasks completed
    pub tasks_completed: u64,
    /// Total tasks failed
    pub tasks_failed: u64,
    /// Total tasks retried
    pub tasks_retried: u64,
    /// Average wait time
    pub avg_wait_time: u64,
    /// Average processing time
    pub avg_processing_time: u64,
}

impl LoadBalancer {
    pub fn new(strategy: Box<dyn LoadBalancingStrategy>) -> Self {
        Self {
            workers: BTreeMap::new(),
            pending_tasks: Vec::new(),
            active_tasks: BTreeMap::new(),
            completed_tasks: Vec::new(),
            strategy,
            health_check_interval: 100,
            tick: 0,
            stats: LoadBalancerStats::default(),
        }
    }

    /// Register worker
    pub fn register_worker(&mut self, worker: WorkerInfo) {
        self.workers.insert(worker.id, worker);
    }

    /// Unregister worker
    pub fn unregister_worker(&mut self, id: WorkerId) {
        // Reassign tasks
        if let Some(worker) = self.workers.get(&id) {
            for &task_id in &worker.current_tasks {
                if let Some(task) = self.active_tasks.remove(&task_id) {
                    self.pending_tasks.push(TaskInfo {
                        assigned_to: None,
                        status: TaskStatus::Pending,
                        ..task
                    });
                }
            }
        }

        self.workers.remove(&id);
    }

    /// Submit task
    pub fn submit(&mut self, task: TaskInfo) -> TaskId {
        let id = task.id;
        self.pending_tasks.push(task);
        id
    }

    /// Process pending tasks
    pub fn process(&mut self) -> Vec<TaskAssignment> {
        let mut assignments = Vec::new();

        // Get available workers
        let available_workers: Vec<_> =
            self.workers.values().filter(|w| w.is_available()).collect();

        if available_workers.is_empty() {
            return assignments;
        }

        // Sort pending by priority
        self.pending_tasks
            .sort_by(|a, b| b.priority.cmp(&a.priority));

        // Assign tasks
        let mut to_remove = Vec::new();

        for (idx, task) in self.pending_tasks.iter().enumerate() {
            let workers: Vec<_> = self.workers.values().filter(|w| w.is_available()).collect();

            if workers.is_empty() {
                break;
            }

            if let Some(worker_id) = self.strategy.select_worker(task, &workers) {
                let assignment = TaskAssignment {
                    task_id: task.id,
                    worker_id,
                };

                // Update worker
                if let Some(worker) = self.workers.get_mut(&worker_id) {
                    worker.current_load += 1;
                    worker.current_tasks.push(task.id);
                }

                // Update task
                let mut assigned_task = task.clone();
                assigned_task.assigned_to = Some(worker_id);
                assigned_task.status = TaskStatus::Assigned;
                assigned_task.started_at = Some(self.tick);

                self.active_tasks.insert(task.id, assigned_task);
                to_remove.push(idx);
                assignments.push(assignment);
                self.stats.tasks_assigned += 1;
            }
        }

        // Remove assigned tasks from pending
        for idx in to_remove.into_iter().rev() {
            self.pending_tasks.remove(idx);
        }

        assignments
    }

    /// Task completed
    pub fn complete(&mut self, task_id: TaskId, success: bool) {
        if let Some(mut task) = self.active_tasks.remove(&task_id) {
            task.completed_at = Some(self.tick);

            if success {
                task.status = TaskStatus::Completed;
                self.stats.tasks_completed += 1;

                // Update worker stats
                if let Some(worker_id) = task.assigned_to {
                    if let Some(worker) = self.workers.get_mut(&worker_id) {
                        worker.current_load = worker.current_load.saturating_sub(1);
                        worker.current_tasks.retain(|&id| id != task_id);
                        worker.tasks_processed += 1;

                        // Update average task time
                        if let (Some(start), Some(end)) = (task.started_at, task.completed_at) {
                            let duration = end - start;
                            let n = worker.tasks_processed;
                            worker.avg_task_time =
                                ((worker.avg_task_time * (n - 1)) + duration) / n;
                        }
                    }
                }
            } else {
                task.status = TaskStatus::Failed;
                self.stats.tasks_failed += 1;

                // Retry if possible
                if task.retries < task.max_retries {
                    task.retries += 1;
                    task.status = TaskStatus::Pending;
                    task.assigned_to = None;
                    task.started_at = None;
                    task.completed_at = None;
                    self.pending_tasks.push(task.clone());
                    self.stats.tasks_retried += 1;
                }

                // Update worker
                if let Some(worker_id) = task.assigned_to {
                    if let Some(worker) = self.workers.get_mut(&worker_id) {
                        worker.current_load = worker.current_load.saturating_sub(1);
                        worker.current_tasks.retain(|&id| id != task_id);
                    }
                }
            }

            self.completed_tasks.push(task);

            // Trim completed history
            if self.completed_tasks.len() > 10000 {
                self.completed_tasks.drain(0..5000);
            }
        }
    }

    /// Worker heartbeat
    pub fn heartbeat(&mut self, worker_id: WorkerId) {
        if let Some(worker) = self.workers.get_mut(&worker_id) {
            worker.last_heartbeat = self.tick;
            worker.healthy = true;
        }
    }

    /// Tick (health checks)
    pub fn tick(&mut self) {
        self.tick += 1;

        // Health check
        if self.tick % self.health_check_interval == 0 {
            let timeout = self.health_check_interval * 3;
            for worker in self.workers.values_mut() {
                if self.tick - worker.last_heartbeat > timeout {
                    worker.healthy = false;
                }
            }
        }
    }

    /// Get worker info
    pub fn get_worker(&self, id: WorkerId) -> Option<&WorkerInfo> {
        self.workers.get(&id)
    }

    /// Get all workers
    pub fn workers(&self) -> impl Iterator<Item = &WorkerInfo> {
        self.workers.values()
    }

    /// Get stats
    pub fn stats(&self) -> &LoadBalancerStats {
        &self.stats
    }

    /// Get pending count
    pub fn pending_count(&self) -> usize {
        self.pending_tasks.len()
    }

    /// Get active count
    pub fn active_count(&self) -> usize {
        self.active_tasks.len()
    }

    /// Get total load
    pub fn total_load(&self) -> (usize, usize) {
        let current: usize = self.workers.values().map(|w| w.current_load).sum();
        let capacity: usize = self.workers.values().map(|w| w.capacity).sum();
        (current, capacity)
    }
}

/// Task assignment
#[derive(Debug, Clone)]
pub struct TaskAssignment {
    /// Task ID
    pub task_id: TaskId,
    /// Worker ID
    pub worker_id: WorkerId,
}

// ============================================================================
// ADAPTIVE LOAD BALANCER
// ============================================================================

/// Adaptive load balancer that switches strategies
pub struct AdaptiveLoadBalancer {
    /// Base load balancer
    balancer: LoadBalancer,
    /// Available strategies
    strategies: Vec<Box<dyn LoadBalancingStrategy>>,
    /// Current strategy index
    current_strategy: usize,
    /// Performance per strategy
    strategy_performance: Vec<f64>,
    /// Switch interval
    switch_interval: u64,
    /// Last switch tick
    last_switch: u64,
}

impl AdaptiveLoadBalancer {
    pub fn new() -> Self {
        let strategies: Vec<Box<dyn LoadBalancingStrategy>> = vec![
            Box::new(LeastConnectionsStrategy),
            Box::new(RoundRobinStrategy::new()),
            Box::new(PowerOfTwoChoicesStrategy::new()),
        ];

        let num_strategies = strategies.len();

        Self {
            balancer: LoadBalancer::new(Box::new(LeastConnectionsStrategy)),
            strategies,
            current_strategy: 0,
            strategy_performance: vec![0.0; num_strategies],
            switch_interval: 1000,
            last_switch: 0,
        }
    }

    /// Evaluate and possibly switch strategy
    pub fn evaluate_and_switch(&mut self) {
        if self.balancer.tick - self.last_switch < self.switch_interval {
            return;
        }

        // Calculate current performance
        let stats = self.balancer.stats();
        let performance = if stats.tasks_completed > 0 {
            stats.tasks_completed as f64 / (stats.tasks_completed + stats.tasks_failed) as f64
        } else {
            0.0
        };

        self.strategy_performance[self.current_strategy] = performance;
        self.last_switch = self.balancer.tick;

        // Find best strategy
        let best_idx = self
            .strategy_performance
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0);

        if best_idx != self.current_strategy {
            self.current_strategy = best_idx;
        }
    }
}

impl Default for AdaptiveLoadBalancer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_robin() {
        let strategy = RoundRobinStrategy::new();

        let workers = vec![
            WorkerInfo::new(WorkerId(1), String::from("w1"), 10, 1),
            WorkerInfo::new(WorkerId(2), String::from("w2"), 10, 1),
            WorkerInfo::new(WorkerId(3), String::from("w3"), 10, 1),
        ];

        let refs: Vec<_> = workers.iter().collect();
        let task = TaskInfo {
            id: TaskId(1),
            priority: 1,
            estimated_duration: 1000,
            assigned_to: None,
            status: TaskStatus::Pending,
            created_at: 0,
            started_at: None,
            completed_at: None,
            retries: 0,
            max_retries: 3,
            affinity: vec![],
        };

        let w1 = strategy.select_worker(&task, &refs);
        let w2 = strategy.select_worker(&task, &refs);
        let w3 = strategy.select_worker(&task, &refs);

        // Should cycle through workers
        assert!(w1.is_some());
        assert!(w2.is_some());
        assert!(w3.is_some());
    }

    #[test]
    fn test_load_balancer() {
        let mut lb = LoadBalancer::new(Box::new(LeastConnectionsStrategy));

        lb.register_worker(WorkerInfo::new(WorkerId(1), String::from("w1"), 5, 1));
        lb.register_worker(WorkerInfo::new(WorkerId(2), String::from("w2"), 5, 1));

        let task = TaskInfo {
            id: TaskId::generate(),
            priority: 1,
            estimated_duration: 1000,
            assigned_to: None,
            status: TaskStatus::Pending,
            created_at: 0,
            started_at: None,
            completed_at: None,
            retries: 0,
            max_retries: 3,
            affinity: vec![],
        };

        let task_id = lb.submit(task);
        let assignments = lb.process();

        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0].task_id, task_id);

        lb.complete(task_id, true);
        assert_eq!(lb.stats().tasks_completed, 1);
    }

    #[test]
    fn test_power_of_two_choices() {
        let strategy = PowerOfTwoChoicesStrategy::new();

        let mut workers = vec![
            WorkerInfo::new(WorkerId(1), String::from("w1"), 10, 1),
            WorkerInfo::new(WorkerId(2), String::from("w2"), 10, 1),
        ];

        workers[0].current_load = 5;
        workers[1].current_load = 2;

        let refs: Vec<_> = workers.iter().collect();
        let task = TaskInfo {
            id: TaskId(1),
            priority: 1,
            estimated_duration: 1000,
            assigned_to: None,
            status: TaskStatus::Pending,
            created_at: 0,
            started_at: None,
            completed_at: None,
            retries: 0,
            max_retries: 3,
            affinity: vec![],
        };

        // Should prefer worker 2 (lower load)
        let selected = strategy.select_worker(&task, &refs);
        assert!(selected.is_some());
    }
}
