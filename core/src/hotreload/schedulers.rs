//! # Hot-Reloadable Schedulers
//!
//! This module provides multiple scheduler implementations that can be
//! hot-swapped at runtime without rebooting.
//!
//! ## Available Schedulers
//!
//! - **RoundRobinScheduler**: Classic fair scheduler, equal time for all
//! - **PriorityScheduler**: Priority-based scheduling
//! - **RealtimeScheduler**: For time-critical tasks (future)

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::any::Any;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use spin::Mutex;

use crate::hotreload::{
    HotReloadError, HotReloadableModule, ModuleCategory, ModuleState, ModuleVersion,
};

/// Task ID type
pub type TaskId = u64;

/// Task priority (0 = highest, 255 = lowest)
pub type Priority = u8;

/// A minimal task representation for the scheduler
#[derive(Debug, Clone)]
pub struct SchedulableTask {
    /// Task ID
    pub id: TaskId,
    /// Task name
    pub name: String,
    /// Task priority
    pub priority: Priority,
    /// Accumulated runtime ticks
    pub runtime_ticks: u64,
    /// Task state
    pub state: TaskState,
}

/// Task state for scheduling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Ready to run
    Ready,
    /// Currently running
    Running,
    /// Blocked on something
    Blocked,
    /// Finished
    Dead,
}

/// Scheduler state for migration
#[derive(Debug)]
pub struct SchedulerState {
    /// All tasks
    tasks: Vec<SchedulableTask>,
    /// Current task ID (if any)
    current: Option<TaskId>,
    /// Total context switches
    context_switches: u64,
}

impl ModuleState for SchedulerState {
    fn export(&self) -> Vec<u8> {
        // Simple serialization (in real impl, use proper serialization)
        let mut data = Vec::new();

        // Number of tasks
        data.extend_from_slice(&(self.tasks.len() as u64).to_le_bytes());

        // Each task: id, priority, runtime, state
        for task in &self.tasks {
            data.extend_from_slice(&task.id.to_le_bytes());
            data.push(task.priority);
            data.extend_from_slice(&task.runtime_ticks.to_le_bytes());
            data.push(task.state as u8);
            // Name length + name
            data.extend_from_slice(&(task.name.len() as u32).to_le_bytes());
            data.extend_from_slice(task.name.as_bytes());
        }

        // Current task
        match self.current {
            Some(id) => {
                data.push(1);
                data.extend_from_slice(&id.to_le_bytes());
            },
            None => data.push(0),
        }

        // Context switches
        data.extend_from_slice(&self.context_switches.to_le_bytes());

        data
    }

    fn version(&self) -> u32 {
        1
    }
}

/// Common scheduler trait
pub trait Scheduler: Send + Sync {
    /// Add a task
    fn add_task(&mut self, task: SchedulableTask);

    /// Remove a task
    fn remove_task(&mut self, id: TaskId) -> Option<SchedulableTask>;

    /// Pick the next task to run
    fn pick_next(&mut self) -> Option<TaskId>;

    /// Get current task
    fn current(&self) -> Option<TaskId>;

    /// Yield current task
    fn yield_current(&mut self);

    /// Get scheduler name
    fn scheduler_name(&self) -> &'static str;

    /// Get statistics
    fn stats(&self) -> SchedulerStats;

    /// Export all tasks for migration
    fn export_tasks(&self) -> Vec<SchedulableTask>;

    /// Import tasks after migration
    fn import_tasks(&mut self, tasks: Vec<SchedulableTask>);

    /// Get context switch count
    fn context_switches(&self) -> u64;
}

/// Scheduler statistics
#[derive(Debug, Clone, Default)]
pub struct SchedulerStats {
    /// Total tasks
    pub total_tasks: usize,
    /// Ready tasks
    pub ready_tasks: usize,
    /// Context switches
    pub context_switches: u64,
    /// Current task ID
    pub current_task: Option<TaskId>,
}

// =============================================================================
// Round-Robin Scheduler
// =============================================================================

/// Classic Round-Robin Scheduler
///
/// All tasks get equal time slices, rotating in order.
pub struct RoundRobinScheduler {
    /// All tasks in queue order
    tasks: Vec<SchedulableTask>,
    /// Current task index
    current_index: usize,
    /// Context switch counter
    context_switches: AtomicU64,
}

impl RoundRobinScheduler {
    /// Create a new round-robin scheduler
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            current_index: 0,
            context_switches: AtomicU64::new(0),
        }
    }
}

impl Default for RoundRobinScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler for RoundRobinScheduler {
    fn add_task(&mut self, task: SchedulableTask) {
        self.tasks.push(task);
    }

    fn remove_task(&mut self, id: TaskId) -> Option<SchedulableTask> {
        if let Some(pos) = self.tasks.iter().position(|t| t.id == id) {
            let task = self.tasks.remove(pos);
            if pos < self.current_index && self.current_index > 0 {
                self.current_index -= 1;
            }
            Some(task)
        } else {
            None
        }
    }

    fn pick_next(&mut self) -> Option<TaskId> {
        if self.tasks.is_empty() {
            return None;
        }

        // Find next ready task
        let start = self.current_index;
        loop {
            self.current_index = (self.current_index + 1) % self.tasks.len();

            if self.tasks[self.current_index].state == TaskState::Ready {
                self.tasks[self.current_index].state = TaskState::Running;
                self.context_switches.fetch_add(1, Ordering::Relaxed);
                return Some(self.tasks[self.current_index].id);
            }

            // Wrapped around, no ready task
            if self.current_index == start {
                return None;
            }
        }
    }

    fn current(&self) -> Option<TaskId> {
        self.tasks
            .get(self.current_index)
            .filter(|t| t.state == TaskState::Running)
            .map(|t| t.id)
    }

    fn yield_current(&mut self) {
        if let Some(task) = self.tasks.get_mut(self.current_index) {
            if task.state == TaskState::Running {
                task.state = TaskState::Ready;
                task.runtime_ticks += 1;
            }
        }
    }

    fn scheduler_name(&self) -> &'static str {
        "RoundRobin"
    }

    fn stats(&self) -> SchedulerStats {
        SchedulerStats {
            total_tasks: self.tasks.len(),
            ready_tasks: self
                .tasks
                .iter()
                .filter(|t| t.state == TaskState::Ready)
                .count(),
            context_switches: self.context_switches.load(Ordering::Relaxed),
            current_task: self.current(),
        }
    }

    fn export_tasks(&self) -> Vec<SchedulableTask> {
        self.tasks.clone()
    }

    fn import_tasks(&mut self, tasks: Vec<SchedulableTask>) {
        self.tasks = tasks;
        self.current_index = 0;
    }

    fn context_switches(&self) -> u64 {
        self.context_switches.load(Ordering::Relaxed)
    }
}

impl HotReloadableModule for RoundRobinScheduler {
    fn name(&self) -> &'static str {
        "RoundRobinScheduler"
    }

    fn version(&self) -> ModuleVersion {
        ModuleVersion::new(1, 0, 0)
    }

    fn category(&self) -> ModuleCategory {
        ModuleCategory::Scheduler
    }

    fn init(&mut self) -> Result<(), HotReloadError> {
        log_scheduler("[SCHEDULER] RoundRobin initialized");
        Ok(())
    }

    fn prepare_unload(&mut self) -> Result<(), HotReloadError> {
        log_scheduler("[SCHEDULER] RoundRobin preparing to unload...");
        // Mark all running tasks as ready
        for task in &mut self.tasks {
            if task.state == TaskState::Running {
                task.state = TaskState::Ready;
            }
        }
        Ok(())
    }

    fn export_state(&self) -> Option<Box<dyn ModuleState>> {
        Some(Box::new(SchedulerState {
            tasks: self.export_tasks(),
            current: self.current(),
            context_switches: self.context_switches(),
        }))
    }

    fn import_state(&mut self, state: &dyn ModuleState) -> Result<(), HotReloadError> {
        // Deserialize tasks from state
        let data = state.export();
        if data.len() < 8 {
            return Err(HotReloadError::StateMigrationFailed);
        }

        // Parse number of tasks
        let num_tasks = u64::from_le_bytes(data[0..8].try_into().unwrap()) as usize;
        let mut offset = 8;

        for _ in 0..num_tasks {
            if offset + 18 > data.len() {
                break;
            }

            let id = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
            offset += 8;
            let priority = data[offset];
            offset += 1;
            let runtime = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
            offset += 8;
            let state_byte = data[offset];
            offset += 1;

            let name_len =
                u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
            offset += 4;

            let name = if offset + name_len <= data.len() {
                String::from_utf8_lossy(&data[offset..offset + name_len]).into_owned()
            } else {
                String::from("unknown")
            };
            offset += name_len;

            let state = match state_byte {
                0 => TaskState::Ready,
                1 => TaskState::Running,
                2 => TaskState::Blocked,
                _ => TaskState::Dead,
            };

            // Imported task is always set to Ready (we reset running state)
            self.tasks.push(SchedulableTask {
                id,
                name,
                priority,
                runtime_ticks: runtime,
                state: if state == TaskState::Running {
                    TaskState::Ready
                } else {
                    state
                },
            });
        }

        log_scheduler(&alloc::format!(
            "[SCHEDULER] Imported {} tasks",
            self.tasks.len()
        ));
        Ok(())
    }

    fn can_unload(&self) -> bool {
        // RoundRobin can always be unloaded
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// =============================================================================
// Priority Scheduler
// =============================================================================

/// Priority-Based Scheduler
///
/// Tasks with lower priority number run first.
/// This demonstrates a DIFFERENT scheduling algorithm that can be hot-swapped.
pub struct PriorityScheduler {
    /// Tasks organized by priority
    priority_queues: BTreeMap<Priority, Vec<SchedulableTask>>,
    /// Current running task
    current_task: Option<TaskId>,
    /// Context switches
    context_switches: AtomicU64,
}

impl PriorityScheduler {
    /// Create a new priority scheduler
    pub fn new() -> Self {
        Self {
            priority_queues: BTreeMap::new(),
            current_task: None,
            context_switches: AtomicU64::new(0),
        }
    }
}

impl Default for PriorityScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler for PriorityScheduler {
    fn add_task(&mut self, task: SchedulableTask) {
        let priority = task.priority;
        self.priority_queues
            .entry(priority)
            .or_insert_with(Vec::new)
            .push(task);
    }

    fn remove_task(&mut self, id: TaskId) -> Option<SchedulableTask> {
        for queue in self.priority_queues.values_mut() {
            if let Some(pos) = queue.iter().position(|t| t.id == id) {
                return Some(queue.remove(pos));
            }
        }
        None
    }

    fn pick_next(&mut self) -> Option<TaskId> {
        // Always pick from highest priority (lowest number) queue first
        for queue in self.priority_queues.values_mut() {
            for task in queue.iter_mut() {
                if task.state == TaskState::Ready {
                    task.state = TaskState::Running;
                    self.current_task = Some(task.id);
                    self.context_switches.fetch_add(1, Ordering::Relaxed);
                    return Some(task.id);
                }
            }
        }
        None
    }

    fn current(&self) -> Option<TaskId> {
        self.current_task
    }

    fn yield_current(&mut self) {
        if let Some(current_id) = self.current_task {
            for queue in self.priority_queues.values_mut() {
                for task in queue.iter_mut() {
                    if task.id == current_id && task.state == TaskState::Running {
                        task.state = TaskState::Ready;
                        task.runtime_ticks += 1;
                        self.current_task = None;
                        return;
                    }
                }
            }
        }
    }

    fn scheduler_name(&self) -> &'static str {
        "Priority"
    }

    fn stats(&self) -> SchedulerStats {
        let total: usize = self.priority_queues.values().map(|q| q.len()).sum();
        let ready: usize = self
            .priority_queues
            .values()
            .flat_map(|q| q.iter())
            .filter(|t| t.state == TaskState::Ready)
            .count();

        SchedulerStats {
            total_tasks: total,
            ready_tasks: ready,
            context_switches: self.context_switches.load(Ordering::Relaxed),
            current_task: self.current_task,
        }
    }

    fn export_tasks(&self) -> Vec<SchedulableTask> {
        self.priority_queues
            .values()
            .flat_map(|q| q.clone())
            .collect()
    }

    fn import_tasks(&mut self, tasks: Vec<SchedulableTask>) {
        self.priority_queues.clear();
        for task in tasks {
            self.add_task(task);
        }
    }

    fn context_switches(&self) -> u64 {
        self.context_switches.load(Ordering::Relaxed)
    }
}

impl HotReloadableModule for PriorityScheduler {
    fn name(&self) -> &'static str {
        "PriorityScheduler"
    }

    fn version(&self) -> ModuleVersion {
        ModuleVersion::new(1, 0, 0)
    }

    fn category(&self) -> ModuleCategory {
        ModuleCategory::Scheduler
    }

    fn init(&mut self) -> Result<(), HotReloadError> {
        log_scheduler("[SCHEDULER] Priority scheduler initialized");
        Ok(())
    }

    fn prepare_unload(&mut self) -> Result<(), HotReloadError> {
        log_scheduler("[SCHEDULER] Priority preparing to unload...");
        // Mark all running tasks as ready
        for queue in self.priority_queues.values_mut() {
            for task in queue.iter_mut() {
                if task.state == TaskState::Running {
                    task.state = TaskState::Ready;
                }
            }
        }
        self.current_task = None;
        Ok(())
    }

    fn export_state(&self) -> Option<Box<dyn ModuleState>> {
        Some(Box::new(SchedulerState {
            tasks: self.export_tasks(),
            current: self.current(),
            context_switches: self.context_switches(),
        }))
    }

    fn import_state(&mut self, state: &dyn ModuleState) -> Result<(), HotReloadError> {
        // Same deserialization as RoundRobin
        let data = state.export();
        if data.len() < 8 {
            return Err(HotReloadError::StateMigrationFailed);
        }

        let num_tasks = u64::from_le_bytes(data[0..8].try_into().unwrap()) as usize;
        let mut offset = 8;

        for _ in 0..num_tasks {
            if offset + 18 > data.len() {
                break;
            }

            let id = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
            offset += 8;
            let priority = data[offset];
            offset += 1;
            let runtime = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
            offset += 8;
            let state_byte = data[offset];
            offset += 1;

            let name_len =
                u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
            offset += 4;

            let name = if offset + name_len <= data.len() {
                String::from_utf8_lossy(&data[offset..offset + name_len]).into_owned()
            } else {
                String::from("unknown")
            };
            offset += name_len;

            let state = match state_byte {
                0 => TaskState::Ready,
                1 => TaskState::Running,
                2 => TaskState::Blocked,
                _ => TaskState::Dead,
            };

            self.add_task(SchedulableTask {
                id,
                name,
                priority,
                runtime_ticks: runtime,
                state: if state == TaskState::Running {
                    TaskState::Ready
                } else {
                    state
                },
            });
        }

        log_scheduler(&alloc::format!(
            "[SCHEDULER] Priority imported {} tasks",
            self.priority_queues
                .values()
                .map(|q| q.len())
                .sum::<usize>()
        ));
        Ok(())
    }

    fn can_unload(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// =============================================================================
// Helper
// =============================================================================

fn log_scheduler(msg: &str) {
    for &c in msg.as_bytes() {
        unsafe {
            core::arch::asm!(
                "out dx, al",
                in("dx") 0x3F8u16,
                in("al") c,
                options(nomem, nostack)
            );
        }
    }
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x3F8u16,
            in("al") b'\n',
            options(nomem, nostack)
        );
    }
}
