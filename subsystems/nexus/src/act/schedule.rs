//! # Action Scheduling
//!
//! Schedules and manages action execution timing.
//! Handles priorities, deadlines, and resource constraints.
//!
//! Part of Year 2 COGNITION - Action Engine

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};

use crate::types::Timestamp;

// ============================================================================
// SCHEDULING TYPES
// ============================================================================

/// Scheduled task
#[derive(Debug, Clone)]
pub struct ScheduledTask {
    /// Task ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Priority
    pub priority: TaskPriority,
    /// Deadline
    pub deadline: Option<Timestamp>,
    /// Duration estimate (ns)
    pub duration_ns: u64,
    /// Dependencies
    pub dependencies: Vec<u64>,
    /// Resource requirements
    pub resources: Vec<ResourceRequirement>,
    /// Status
    pub status: TaskStatus,
    /// Created at
    pub created: Timestamp,
    /// Scheduled start
    pub scheduled_start: Option<Timestamp>,
}

/// Task priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Lowest = 0,
    Low = 1,
    Normal = 2,
    High = 3,
    Highest = 4,
    Critical = 5,
}

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Scheduled,
    Ready,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Resource requirement
#[derive(Debug, Clone)]
pub struct ResourceRequirement {
    /// Resource type
    pub resource_type: String,
    /// Amount needed
    pub amount: u64,
    /// Exclusive access
    pub exclusive: bool,
}

/// Resource
#[derive(Debug, Clone)]
pub struct Resource {
    /// Resource ID
    pub id: u64,
    /// Type
    pub resource_type: String,
    /// Total capacity
    pub capacity: u64,
    /// Available capacity
    pub available: u64,
    /// Locked by tasks
    pub locked_by: Vec<u64>,
}

/// Schedule
#[derive(Debug, Clone)]
pub struct Schedule {
    /// Schedule ID
    pub id: u64,
    /// Slots
    pub slots: Vec<ScheduleSlot>,
    /// Start time
    pub start_time: Timestamp,
    /// End time
    pub end_time: Timestamp,
}

/// Schedule slot
#[derive(Debug, Clone)]
pub struct ScheduleSlot {
    /// Task ID
    pub task_id: u64,
    /// Start time
    pub start: Timestamp,
    /// End time
    pub end: Timestamp,
}

// ============================================================================
// SCHEDULER
// ============================================================================

/// Action scheduler
pub struct ActionScheduler {
    /// Tasks
    tasks: BTreeMap<u64, ScheduledTask>,
    /// Resources
    resources: BTreeMap<u64, Resource>,
    /// Current schedule
    schedule: Option<Schedule>,
    /// Next ID
    next_id: AtomicU64,
    /// Running
    running: AtomicBool,
    /// Configuration
    config: SchedulerConfig,
    /// Statistics
    stats: SchedulerStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Scheduling algorithm
    pub algorithm: SchedulingAlgorithm,
    /// Time quantum (ns)
    pub time_quantum: u64,
    /// Preemption enabled
    pub preemption: bool,
    /// Maximum queue size
    pub max_queue_size: usize,
}

/// Scheduling algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingAlgorithm {
    /// First Come First Served
    FCFS,
    /// Shortest Job First
    SJF,
    /// Priority based
    Priority,
    /// Earliest Deadline First
    EDF,
    /// Round Robin
    RoundRobin,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            algorithm: SchedulingAlgorithm::Priority,
            time_quantum: 10_000_000, // 10ms
            preemption: true,
            max_queue_size: 1000,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct SchedulerStats {
    /// Tasks scheduled
    pub tasks_scheduled: u64,
    /// Tasks completed
    pub tasks_completed: u64,
    /// Deadline misses
    pub deadline_misses: u64,
    /// Average wait time
    pub avg_wait_time_ns: u64,
}

impl ActionScheduler {
    /// Create new scheduler
    pub fn new(config: SchedulerConfig) -> Self {
        Self {
            tasks: BTreeMap::new(),
            resources: BTreeMap::new(),
            schedule: None,
            next_id: AtomicU64::new(1),
            running: AtomicBool::new(false),
            config,
            stats: SchedulerStats::default(),
        }
    }

    /// Add task
    pub fn add_task(&mut self, name: &str, priority: TaskPriority) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let task = ScheduledTask {
            id,
            name: name.into(),
            priority,
            deadline: None,
            duration_ns: 0,
            dependencies: Vec::new(),
            resources: Vec::new(),
            status: TaskStatus::Pending,
            created: Timestamp::now(),
            scheduled_start: None,
        };

        self.tasks.insert(id, task);
        id
    }

    /// Set deadline
    pub fn set_deadline(&mut self, task_id: u64, deadline: Timestamp) {
        if let Some(task) = self.tasks.get_mut(&task_id) {
            task.deadline = Some(deadline);
        }
    }

    /// Set duration
    pub fn set_duration(&mut self, task_id: u64, duration_ns: u64) {
        if let Some(task) = self.tasks.get_mut(&task_id) {
            task.duration_ns = duration_ns;
        }
    }

    /// Add dependency
    pub fn add_dependency(&mut self, task_id: u64, dependency_id: u64) {
        if task_id != dependency_id {
            if let Some(task) = self.tasks.get_mut(&task_id) {
                if !task.dependencies.contains(&dependency_id) {
                    task.dependencies.push(dependency_id);
                }
            }
        }
    }

    /// Add resource requirement
    pub fn require_resource(&mut self, task_id: u64, resource: ResourceRequirement) {
        if let Some(task) = self.tasks.get_mut(&task_id) {
            task.resources.push(resource);
        }
    }

    /// Register resource
    pub fn register_resource(&mut self, resource_type: &str, capacity: u64) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let resource = Resource {
            id,
            resource_type: resource_type.into(),
            capacity,
            available: capacity,
            locked_by: Vec::new(),
        };

        self.resources.insert(id, resource);
        id
    }

    /// Build schedule
    pub fn build_schedule(&mut self) -> u64 {
        let schedule_id = self.next_id.fetch_add(1, Ordering::Relaxed);

        // Get ready tasks
        let ready_tasks: Vec<u64> = self.get_ready_tasks();

        // Sort by algorithm
        let sorted = self.sort_by_algorithm(ready_tasks);

        // Create slots
        let mut slots = Vec::new();
        let mut current_time = Timestamp::now();

        for task_id in sorted {
            if let Some(task) = self.tasks.get_mut(&task_id) {
                let start = current_time;
                let end = Timestamp(current_time.0 + task.duration_ns);

                slots.push(ScheduleSlot {
                    task_id,
                    start,
                    end,
                });

                task.status = TaskStatus::Scheduled;
                task.scheduled_start = Some(start);

                current_time = end;
                self.stats.tasks_scheduled += 1;
            }
        }

        let schedule = Schedule {
            id: schedule_id,
            slots: slots.clone(),
            start_time: slots.first().map(|s| s.start).unwrap_or(Timestamp::now()),
            end_time: slots.last().map(|s| s.end).unwrap_or(Timestamp::now()),
        };

        self.schedule = Some(schedule);

        schedule_id
    }

    fn get_ready_tasks(&self) -> Vec<u64> {
        self.tasks.values()
            .filter(|t| {
                t.status == TaskStatus::Pending &&
                self.dependencies_met(t.id)
            })
            .map(|t| t.id)
            .collect()
    }

    fn dependencies_met(&self, task_id: u64) -> bool {
        let task = match self.tasks.get(&task_id) {
            Some(t) => t,
            None => return false,
        };

        task.dependencies.iter().all(|dep_id| {
            self.tasks.get(dep_id)
                .map(|dep| dep.status == TaskStatus::Completed)
                .unwrap_or(true)
        })
    }

    fn sort_by_algorithm(&self, mut tasks: Vec<u64>) -> Vec<u64> {
        match self.config.algorithm {
            SchedulingAlgorithm::FCFS => {
                tasks.sort_by_key(|id| {
                    self.tasks.get(id).map(|t| t.created.0).unwrap_or(0)
                });
            }

            SchedulingAlgorithm::SJF => {
                tasks.sort_by_key(|id| {
                    self.tasks.get(id).map(|t| t.duration_ns).unwrap_or(u64::MAX)
                });
            }

            SchedulingAlgorithm::Priority => {
                tasks.sort_by(|a, b| {
                    let pa = self.tasks.get(a).map(|t| t.priority).unwrap_or(TaskPriority::Lowest);
                    let pb = self.tasks.get(b).map(|t| t.priority).unwrap_or(TaskPriority::Lowest);
                    pb.cmp(&pa) // Higher priority first
                });
            }

            SchedulingAlgorithm::EDF => {
                tasks.sort_by(|a, b| {
                    let da = self.tasks.get(a).and_then(|t| t.deadline).map(|d| d.0);
                    let db = self.tasks.get(b).and_then(|t| t.deadline).map(|d| d.0);

                    match (da, db) {
                        (Some(a), Some(b)) => a.cmp(&b),
                        (Some(_), None) => core::cmp::Ordering::Less,
                        (None, Some(_)) => core::cmp::Ordering::Greater,
                        (None, None) => core::cmp::Ordering::Equal,
                    }
                });
            }

            SchedulingAlgorithm::RoundRobin => {
                // Round robin just uses FCFS for initial ordering
                tasks.sort_by_key(|id| {
                    self.tasks.get(id).map(|t| t.created.0).unwrap_or(0)
                });
            }
        }

        tasks
    }

    /// Get next task
    pub fn next_task(&mut self) -> Option<u64> {
        let schedule = self.schedule.as_ref()?;
        let now = Timestamp::now();

        for slot in &schedule.slots {
            if let Some(task) = self.tasks.get(&slot.task_id) {
                if task.status == TaskStatus::Scheduled && slot.start.0 <= now.0 {
                    return Some(slot.task_id);
                }
            }
        }

        None
    }

    /// Start task
    pub fn start_task(&mut self, task_id: u64) -> bool {
        // Check resource availability
        if !self.acquire_resources(task_id) {
            return false;
        }

        if let Some(task) = self.tasks.get_mut(&task_id) {
            task.status = TaskStatus::Running;
            return true;
        }

        false
    }

    fn acquire_resources(&mut self, task_id: u64) -> bool {
        let requirements = match self.tasks.get(&task_id) {
            Some(t) => t.resources.clone(),
            None => return false,
        };

        // Check all requirements
        for req in &requirements {
            let available = self.resources.values()
                .filter(|r| r.resource_type == req.resource_type)
                .map(|r| r.available)
                .sum::<u64>();

            if available < req.amount {
                return false;
            }
        }

        // Acquire resources
        for req in &requirements {
            let mut remaining = req.amount;

            for resource in self.resources.values_mut() {
                if resource.resource_type == req.resource_type && remaining > 0 {
                    let take = remaining.min(resource.available);
                    resource.available -= take;
                    resource.locked_by.push(task_id);
                    remaining -= take;
                }
            }
        }

        true
    }

    /// Complete task
    pub fn complete_task(&mut self, task_id: u64) {
        self.release_resources(task_id);

        if let Some(task) = self.tasks.get_mut(&task_id) {
            task.status = TaskStatus::Completed;
            self.stats.tasks_completed += 1;

            // Check deadline
            if let Some(deadline) = task.deadline {
                if Timestamp::now().0 > deadline.0 {
                    self.stats.deadline_misses += 1;
                }
            }
        }
    }

    fn release_resources(&mut self, task_id: u64) {
        let requirements = match self.tasks.get(&task_id) {
            Some(t) => t.resources.clone(),
            None => return,
        };

        for req in &requirements {
            let mut remaining = req.amount;

            for resource in self.resources.values_mut() {
                if resource.resource_type == req.resource_type {
                    resource.locked_by.retain(|id| *id != task_id);
                    let release = remaining.min(resource.capacity - resource.available);
                    resource.available += release;
                    remaining -= release;
                }
            }
        }
    }

    /// Cancel task
    pub fn cancel_task(&mut self, task_id: u64) {
        if let Some(task) = self.tasks.get_mut(&task_id) {
            if task.status == TaskStatus::Running {
                self.release_resources(task_id);
            }
            task.status = TaskStatus::Cancelled;
        }
    }

    /// Get task
    pub fn get_task(&self, id: u64) -> Option<&ScheduledTask> {
        self.tasks.get(&id)
    }

    /// Get schedule
    pub fn get_schedule(&self) -> Option<&Schedule> {
        self.schedule.as_ref()
    }

    /// Get statistics
    pub fn stats(&self) -> &SchedulerStats {
        &self.stats
    }
}

impl Default for ActionScheduler {
    fn default() -> Self {
        Self::new(SchedulerConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_task() {
        let mut scheduler = ActionScheduler::default();

        let id = scheduler.add_task("Test", TaskPriority::Normal);
        assert!(scheduler.get_task(id).is_some());
    }

    #[test]
    fn test_set_deadline() {
        let mut scheduler = ActionScheduler::default();

        let id = scheduler.add_task("Test", TaskPriority::Normal);
        scheduler.set_deadline(id, Timestamp(1000000));

        let task = scheduler.get_task(id).unwrap();
        assert!(task.deadline.is_some());
    }

    #[test]
    fn test_dependency() {
        let mut scheduler = ActionScheduler::default();

        let t1 = scheduler.add_task("Task 1", TaskPriority::Normal);
        let t2 = scheduler.add_task("Task 2", TaskPriority::Normal);

        scheduler.complete_task(t1);
        scheduler.add_dependency(t2, t1);

        assert!(scheduler.dependencies_met(t2));
    }

    #[test]
    fn test_build_schedule() {
        let mut scheduler = ActionScheduler::default();

        scheduler.add_task("Task 1", TaskPriority::High);
        scheduler.add_task("Task 2", TaskPriority::Low);

        scheduler.build_schedule();

        let schedule = scheduler.get_schedule().unwrap();
        assert_eq!(schedule.slots.len(), 2);
    }

    #[test]
    fn test_priority_order() {
        let mut config = SchedulerConfig::default();
        config.algorithm = SchedulingAlgorithm::Priority;

        let mut scheduler = ActionScheduler::new(config);

        let low = scheduler.add_task("Low", TaskPriority::Low);
        scheduler.set_duration(low, 100);

        let high = scheduler.add_task("High", TaskPriority::High);
        scheduler.set_duration(high, 100);

        scheduler.build_schedule();

        let schedule = scheduler.get_schedule().unwrap();
        // High priority should be first
        assert_eq!(schedule.slots[0].task_id, high);
    }

    #[test]
    fn test_resources() {
        let mut scheduler = ActionScheduler::default();

        scheduler.register_resource("cpu", 4);

        let task = scheduler.add_task("Test", TaskPriority::Normal);
        scheduler.require_resource(task, ResourceRequirement {
            resource_type: "cpu".into(),
            amount: 2,
            exclusive: false,
        });

        scheduler.build_schedule();
        assert!(scheduler.start_task(task));
    }
}
