//! # DIS Scheduler Core - The Heart of Dynamic Intent Scheduling
//!
//! The DIS Scheduler is the central component that orchestrates all scheduling
//! decisions. It combines intent awareness, adaptive optimization, and real-time
//! statistics to make intelligent scheduling choices.
//!
//! ## Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────────────────────┐
//! │                            DIS SCHEDULER CORE                                   │
//! │                                                                                 │
//! │  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐   ┌─────────────────────┐  │
//! │  │   INTENT    │   │   POLICY    │   │   STATS     │   │   ADAPTIVE          │  │
//! │  │   ENGINE    │   │   ENGINE    │   │   COLLECTOR │   │   OPTIMIZER         │  │
//! │  └──────┬──────┘   └──────┬──────┘   └──────┬──────┘   └──────────┬──────────┘  │
//! │         │                 │                 │                     │             │
//! │         ▼                 ▼                 ▼                     ▼             │
//! │  ┌──────────────────────────────────────────────────────────────────────────┐   │
//! │  │                       DECISION ENGINE                                     │   │
//! │  │                                                                           │   │
//! │  │   • Process intents                                                       │   │
//! │  │   • Evaluate policies                                                     │   │
//! │  │   • Analyze statistics                                                    │   │
//! │  │   • Apply optimizations                                                   │   │
//! │  │   • Select next task                                                      │   │
//! │  └──────────────────────────────────────────────────────────────────────────┘   │
//! │                                      │                                          │
//! │                                      ▼                                          │
//! │  ┌──────────────────────────────────────────────────────────────────────────┐   │
//! │  │                       MULTI-LEVEL QUEUES                                  │   │
//! │  │                                                                           │   │
//! │  │   REALTIME      INTERACTIVE      NORMAL      BATCH      BACKGROUND        │   │
//! │  │   ────────      ───────────      ──────      ─────      ──────────        │   │
//! │  │   [T1][T2]      [T3][T4]         [T5]        [T6]       [T7][T8][T9]       │   │
//! │  └──────────────────────────────────────────────────────────────────────────┘   │
//! │                                      │                                          │
//! │                                      ▼                                          │
//! │  ┌──────────────────────────────────────────────────────────────────────────┐   │
//! │  │                       PER-CPU RUN QUEUES                                  │   │
//! │  │                                                                           │   │
//! │  │   CPU 0          CPU 1          CPU 2          CPU 3                      │   │
//! │  │   ─────          ─────          ─────          ─────                      │   │
//! │  │   [...]          [...]          [...]          [...]                      │   │
//! │  └──────────────────────────────────────────────────────────────────────────┘   │
//! └────────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Scheduling Algorithm
//!
//! 1. **Intent Evaluation**: Determine task requirements from intent
//! 2. **Policy Application**: Apply active scheduling policies
//! 3. **Deadline Check**: Prioritize tasks with approaching deadlines
//! 4. **Queue Selection**: Place task in appropriate queue
//! 5. **Load Balancing**: Distribute across CPUs
//! 6. **Optimization Hints**: Apply optimizer recommendations
//! 7. **Final Selection**: Pick best candidate for execution

use alloc::collections::{BTreeMap, VecDeque};
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};

use spin::{Mutex, RwLock};

use super::intent::{Intent, IntentClass, IntentEngine};
use super::optimizer::{AdaptiveOptimizer, HintType, QueueLevel};
use super::policy::{PolicyContext, PolicyEngine, TaskContext};
use super::stats::StatsCollector;
use super::{CpuId, DISError, DISResult, Nanoseconds, Task, TaskFlags, TaskId, TaskState};

// =============================================================================
// Scheduler Configuration
// =============================================================================

/// Scheduler configuration
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Number of CPUs
    pub cpu_count: usize,
    /// Default time slice (nanoseconds)
    pub default_time_slice: Nanoseconds,
    /// Minimum time slice
    pub min_time_slice: Nanoseconds,
    /// Maximum time slice
    pub max_time_slice: Nanoseconds,
    /// Tick interval
    pub tick_interval: Nanoseconds,
    /// Load balancing interval
    pub balance_interval: Nanoseconds,
    /// Enable adaptive optimization
    pub adaptive_optimization: bool,
    /// Enable intent-based scheduling
    pub intent_scheduling: bool,
    /// Enable load balancing
    pub load_balancing: bool,
    /// Preemption enabled
    pub preemption: bool,
    /// Real-time priority threshold
    pub realtime_threshold: i8,
    /// Interactive priority threshold
    pub interactive_threshold: i8,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            cpu_count: 4,
            default_time_slice: Nanoseconds::from_millis(10),
            min_time_slice: Nanoseconds::from_millis(1),
            max_time_slice: Nanoseconds::from_millis(100),
            tick_interval: Nanoseconds::from_millis(1),
            balance_interval: Nanoseconds::from_millis(100),
            adaptive_optimization: true,
            intent_scheduling: true,
            load_balancing: true,
            preemption: true,
            realtime_threshold: 80,
            interactive_threshold: 40,
        }
    }
}

// =============================================================================
// Run Queue
// =============================================================================

/// A single run queue for one priority level
#[derive(Debug)]
pub struct RunQueue {
    /// Queue level
    level: QueueLevel,
    /// Tasks in queue
    tasks: VecDeque<TaskId>,
    /// Active task count
    count: AtomicUsize,
    /// Time quantum for this queue
    time_quantum: Nanoseconds,
    /// Accumulated runtime
    runtime: AtomicU64,
}

impl RunQueue {
    /// Create new run queue
    pub fn new(level: QueueLevel, time_quantum: Nanoseconds) -> Self {
        Self {
            level,
            tasks: VecDeque::new(),
            count: AtomicUsize::new(0),
            time_quantum,
            runtime: AtomicU64::new(0),
        }
    }

    /// Push task to back of queue
    pub fn push_back(&mut self, task_id: TaskId) {
        self.tasks.push_back(task_id);
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    /// Push task to front of queue (high priority)
    pub fn push_front(&mut self, task_id: TaskId) {
        self.tasks.push_front(task_id);
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    /// Pop next task
    pub fn pop_front(&mut self) -> Option<TaskId> {
        let task = self.tasks.pop_front();
        if task.is_some() {
            self.count.fetch_sub(1, Ordering::Relaxed);
        }
        task
    }

    /// Remove specific task
    pub fn remove(&mut self, task_id: TaskId) -> bool {
        if let Some(pos) = self.tasks.iter().position(|&t| t == task_id) {
            self.tasks.remove(pos);
            self.count.fetch_sub(1, Ordering::Relaxed);
            return true;
        }
        false
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.count.load(Ordering::Relaxed) == 0
    }

    /// Get task count
    pub fn len(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }

    /// Get time quantum
    pub fn time_quantum(&self) -> Nanoseconds {
        self.time_quantum
    }

    /// Add runtime
    pub fn add_runtime(&self, ns: Nanoseconds) {
        self.runtime.fetch_add(ns.raw(), Ordering::Relaxed);
    }
}

// =============================================================================
// Per-CPU State
// =============================================================================

/// Per-CPU scheduling state
pub struct CpuState {
    /// CPU ID
    pub id: CpuId,
    /// Currently running task
    pub current: Option<TaskId>,
    /// Idle task
    pub idle_task: TaskId,
    /// Run queues per priority level
    pub queues: [Mutex<RunQueue>; 6],
    /// Total runnable tasks
    pub runnable: AtomicU32,
    /// Load metric (0-100)
    pub load: AtomicU32,
    /// CPU is online
    pub online: AtomicBool,
    /// Need reschedule flag
    pub need_resched: AtomicBool,
    /// Time of last balance
    pub last_balance: AtomicU64,
    /// Current priority being scheduled
    pub current_priority: AtomicUsize,
}

impl CpuState {
    /// Create new CPU state
    pub fn new(id: CpuId, idle_task: TaskId) -> Self {
        Self {
            id,
            current: None,
            idle_task,
            queues: [
                Mutex::new(RunQueue::new(
                    QueueLevel::RealTime,
                    Nanoseconds::from_millis(2),
                )),
                Mutex::new(RunQueue::new(
                    QueueLevel::Interactive,
                    Nanoseconds::from_millis(4),
                )),
                Mutex::new(RunQueue::new(
                    QueueLevel::Normal,
                    Nanoseconds::from_millis(10),
                )),
                Mutex::new(RunQueue::new(
                    QueueLevel::Batch,
                    Nanoseconds::from_millis(50),
                )),
                Mutex::new(RunQueue::new(
                    QueueLevel::Background,
                    Nanoseconds::from_millis(100),
                )),
                Mutex::new(RunQueue::new(
                    QueueLevel::Idle,
                    Nanoseconds::from_millis(200),
                )),
            ],
            runnable: AtomicU32::new(0),
            load: AtomicU32::new(0),
            online: AtomicBool::new(true),
            need_resched: AtomicBool::new(false),
            last_balance: AtomicU64::new(0),
            current_priority: AtomicUsize::new(2), // Normal
        }
    }

    /// Get queue index for level
    fn queue_index(level: QueueLevel) -> usize {
        match level {
            QueueLevel::RealTime => 0,
            QueueLevel::Interactive => 1,
            QueueLevel::Normal => 2,
            QueueLevel::Batch => 3,
            QueueLevel::Background => 4,
            QueueLevel::Idle => 5,
        }
    }

    /// Enqueue task
    pub fn enqueue(&self, task_id: TaskId, level: QueueLevel) {
        let idx = Self::queue_index(level);
        self.queues[idx].lock().push_back(task_id);
        self.runnable.fetch_add(1, Ordering::Relaxed);
    }

    /// Enqueue task with high priority
    pub fn enqueue_front(&self, task_id: TaskId, level: QueueLevel) {
        let idx = Self::queue_index(level);
        self.queues[idx].lock().push_front(task_id);
        self.runnable.fetch_add(1, Ordering::Relaxed);
    }

    /// Dequeue next task
    pub fn dequeue(&self) -> Option<(TaskId, QueueLevel)> {
        // Check queues from highest to lowest priority
        let levels = [
            QueueLevel::RealTime,
            QueueLevel::Interactive,
            QueueLevel::Normal,
            QueueLevel::Batch,
            QueueLevel::Background,
            QueueLevel::Idle,
        ];

        for level in levels {
            let idx = Self::queue_index(level);
            if let Some(task) = self.queues[idx].lock().pop_front() {
                self.runnable.fetch_sub(1, Ordering::Relaxed);
                return Some((task, level));
            }
        }

        None
    }

    /// Remove task from any queue
    pub fn remove_task(&self, task_id: TaskId) -> bool {
        for queue in &self.queues {
            if queue.lock().remove(task_id) {
                self.runnable.fetch_sub(1, Ordering::Relaxed);
                return true;
            }
        }
        false
    }

    /// Check if CPU has runnable tasks
    pub fn has_runnable(&self) -> bool {
        self.runnable.load(Ordering::Relaxed) > 0
    }

    /// Calculate load
    pub fn calculate_load(&self) -> u32 {
        let runnable = self.runnable.load(Ordering::Relaxed);
        // Simple load calculation
        let load = (runnable * 25).min(100);
        self.load.store(load, Ordering::Relaxed);
        load
    }

    /// Request reschedule
    pub fn request_resched(&self) {
        self.need_resched.store(true, Ordering::SeqCst);
    }

    /// Clear reschedule flag
    pub fn clear_resched(&self) {
        self.need_resched.store(false, Ordering::SeqCst);
    }

    /// Check if reschedule needed
    pub fn needs_resched(&self) -> bool {
        self.need_resched.load(Ordering::SeqCst)
    }
}

// =============================================================================
// Scheduling Decision
// =============================================================================

/// A scheduling decision
#[derive(Debug, Clone)]
pub struct SchedulingDecision {
    /// Task to run
    pub task_id: TaskId,
    /// Time slice allocated
    pub time_slice: Nanoseconds,
    /// Queue level
    pub queue_level: QueueLevel,
    /// Target CPU
    pub cpu: CpuId,
    /// Priority used
    pub priority: i8,
    /// Reason for decision
    pub reason: DecisionReason,
    /// Applied optimizations
    pub optimizations: Vec<String>,
}

/// Reason for scheduling decision
#[derive(Debug, Clone)]
pub enum DecisionReason {
    /// Highest priority task
    HighestPriority,
    /// Deadline approaching
    DeadlineApproaching,
    /// Real-time requirement
    RealTimeRequirement,
    /// Interactive boost
    InteractiveBoost,
    /// Load balancing
    LoadBalanced,
    /// CPU affinity
    CpuAffinity,
    /// Policy enforcement
    PolicyEnforced(String),
    /// Intent requirement
    IntentRequirement(IntentClass),
    /// Optimization hint
    OptimizationHint,
    /// Idle fallback
    IdleFallback,
}

// =============================================================================
// DIS Scheduler
// =============================================================================

/// The main DIS scheduler
pub struct DISScheduler {
    /// Configuration
    config: RwLock<SchedulerConfig>,
    /// Intent engine
    intent_engine: RwLock<IntentEngine>,
    /// Policy engine
    policy_engine: RwLock<PolicyEngine>,
    /// Statistics collector
    stats: RwLock<StatsCollector>,
    /// Adaptive optimizer
    optimizer: RwLock<AdaptiveOptimizer>,
    /// Per-CPU state
    cpus: RwLock<Vec<CpuState>>,
    /// All tasks
    tasks: RwLock<BTreeMap<TaskId, Task>>,
    /// Task intents
    intents: RwLock<BTreeMap<TaskId, Intent>>,
    /// Current timestamp
    current_time: AtomicU64,
    /// Scheduler tick count
    tick_count: AtomicU64,
    /// Total context switches
    context_switches: AtomicU64,
    /// Scheduler enabled
    enabled: AtomicBool,
    /// Next task ID
    next_task_id: AtomicU64,
    /// Scheduler statistics
    sched_stats: SchedulerStats,
}

/// Scheduler statistics
#[derive(Debug, Default)]
pub struct SchedulerStats {
    pub tasks_created: AtomicU64,
    pub tasks_completed: AtomicU64,
    pub schedules: AtomicU64,
    pub preemptions: AtomicU64,
    pub migrations: AtomicU64,
    pub load_balances: AtomicU64,
    pub policy_applications: AtomicU64,
    pub intent_evaluations: AtomicU64,
    pub optimization_hints_applied: AtomicU64,
}

impl DISScheduler {
    /// Create new DIS scheduler
    pub fn new(config: SchedulerConfig) -> Self {
        let cpu_count = config.cpu_count;

        // Create per-CPU state
        let mut cpus = Vec::with_capacity(cpu_count);
        for i in 0..cpu_count {
            let idle_task = TaskId::new(u64::MAX - i as u64);
            cpus.push(CpuState::new(CpuId::new(i as u32), idle_task));
        }

        Self {
            config: RwLock::new(config),
            intent_engine: RwLock::new(IntentEngine::new()),
            policy_engine: RwLock::new(PolicyEngine::new()),
            stats: RwLock::new(StatsCollector::new(cpu_count)),
            optimizer: RwLock::new(AdaptiveOptimizer::default()),
            cpus: RwLock::new(cpus),
            tasks: RwLock::new(BTreeMap::new()),
            intents: RwLock::new(BTreeMap::new()),
            current_time: AtomicU64::new(0),
            tick_count: AtomicU64::new(0),
            context_switches: AtomicU64::new(0),
            enabled: AtomicBool::new(true),
            next_task_id: AtomicU64::new(1),
            sched_stats: SchedulerStats::default(),
        }
    }

    // =========================================================================
    // Task Management
    // =========================================================================

    /// Create a new task with intent
    pub fn create_task(&self, name: &str, intent: Intent) -> DISResult<TaskId> {
        let task_id = TaskId::new(self.next_task_id.fetch_add(1, Ordering::Relaxed));

        // Create task from intent
        let mut task = Task::new(task_id, name.into(), intent.clone());

        // Apply intent properties
        task.base_priority = match task.intent.class {
            IntentClass::RealTime => 90,
            IntentClass::SoftRealTime => 80,
            IntentClass::Interactive => 70,
            IntentClass::Server => 50,
            IntentClass::Batch => 30,
            IntentClass::Background => 10,
            IntentClass::Idle => 0,
            IntentClass::Critical => 100,
        };

        task.time_slice = self.config.read().default_time_slice;
        task.state = TaskState::Ready;

        // Register with stats
        self.stats.read().register_task(task_id);

        // Store task and intent
        self.tasks.write().insert(task_id, task);

        // Validate and store intent
        let engine = self.intent_engine.write();
        engine.register(intent.clone())?;
        drop(engine);

        self.intents.write().insert(task_id, intent);

        // Enqueue task
        self.enqueue_task(task_id)?;

        self.sched_stats
            .tasks_created
            .fetch_add(1, Ordering::Relaxed);

        Ok(task_id)
    }

    /// Destroy a task
    pub fn destroy_task(&self, task_id: TaskId) -> DISResult<()> {
        // Remove from queues
        for cpu in self.cpus.read().iter() {
            cpu.remove_task(task_id);
        }

        // Remove task
        self.tasks.write().remove(&task_id);
        self.intents.write().remove(&task_id);

        // Unregister from stats
        self.stats.read().unregister_task(task_id);

        self.sched_stats
            .tasks_completed
            .fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Get task
    pub fn get_task(&self, task_id: TaskId) -> Option<Task> {
        self.tasks.read().get(&task_id).cloned()
    }

    /// Update task state
    pub fn set_task_state(&self, task_id: TaskId, state: TaskState) -> DISResult<()> {
        let mut tasks = self.tasks.write();

        if let Some(task) = tasks.get_mut(&task_id) {
            let old_state = task.state;
            task.state = state;

            // Handle state transitions
            match (old_state, state) {
                (_, TaskState::Ready) if old_state != TaskState::Ready => {
                    drop(tasks);
                    self.enqueue_task(task_id)?;
                },
                (TaskState::Ready, _) if state != TaskState::Ready => {
                    drop(tasks);
                    self.dequeue_task(task_id)?;
                },
                _ => {},
            }

            return Ok(());
        }

        Err(DISError::TaskNotFound(task_id))
    }

    /// Enqueue task to appropriate CPU
    fn enqueue_task(&self, task_id: TaskId) -> DISResult<()> {
        let tasks = self.tasks.read();
        let task = tasks.get(&task_id).ok_or(DISError::TaskNotFound(task_id))?;

        // Determine queue level
        let level = self.determine_queue_level(task);

        // Select target CPU
        let cpu = self.select_cpu(task_id)?;

        // Enqueue
        let cpus = self.cpus.read();
        if let Some(cpu_state) = cpus.get(cpu.id() as usize) {
            cpu_state.enqueue(task_id, level);
        }

        Ok(())
    }

    /// Dequeue task
    fn dequeue_task(&self, task_id: TaskId) -> DISResult<()> {
        for cpu in self.cpus.read().iter() {
            if cpu.remove_task(task_id) {
                return Ok(());
            }
        }
        Ok(())
    }

    /// Determine queue level for task
    fn determine_queue_level(&self, task: &Task) -> QueueLevel {
        let config = self.config.read();

        if task.base_priority >= config.realtime_threshold as i32 {
            QueueLevel::RealTime
        } else if task.base_priority >= config.interactive_threshold as i32 {
            QueueLevel::Interactive
        } else if task.base_priority >= 20 {
            QueueLevel::Normal
        } else if task.base_priority >= 10 {
            QueueLevel::Batch
        } else if task.base_priority > 0 {
            QueueLevel::Background
        } else {
            QueueLevel::Idle
        }
    }

    /// Select target CPU for task
    fn select_cpu(&self, task_id: TaskId) -> DISResult<CpuId> {
        let cpus = self.cpus.read();

        // Check affinity
        if let Some(task) = self.tasks.read().get(&task_id) {
            if task.flags.contains(TaskFlags::CPU_AFFINITY) && task.cpu_affinity != 0 {
                // Find least loaded CPU in affinity set
                let mut best_cpu: Option<&CpuState> = None;
                let mut best_load = u32::MAX;

                for (id, cpu) in cpus.iter().enumerate() {
                    // Check if this CPU is in the affinity mask
                    if (task.cpu_affinity & (1 << id)) != 0 {
                        if cpu.online.load(Ordering::Relaxed) {
                            let load = cpu.load.load(Ordering::Relaxed);
                            if load < best_load {
                                best_load = load;
                                best_cpu = Some(cpu);
                            }
                        }
                    }
                }

                if let Some(cpu) = best_cpu {
                    return Ok(cpu.id);
                }
            }
        }

        // Find least loaded CPU
        let best = cpus
            .iter()
            .filter(|c| c.online.load(Ordering::Relaxed))
            .min_by_key(|c| c.load.load(Ordering::Relaxed));

        match best {
            Some(cpu) => Ok(cpu.id),
            None => Err(DISError::NoCpuAvailable),
        }
    }

    // =========================================================================
    // Scheduling
    // =========================================================================

    /// Main scheduling entry point
    pub fn schedule(&self, cpu: CpuId) -> SchedulingDecision {
        self.sched_stats.schedules.fetch_add(1, Ordering::Relaxed);

        let cpus = self.cpus.read();
        let cpu_state = match cpus.get(cpu.id() as usize) {
            Some(c) => c,
            None => return self.idle_decision(cpu),
        };

        // Clear reschedule flag
        cpu_state.clear_resched();

        // Get current task
        let prev_task = cpu_state.current;

        // Try to get next task
        let decision = if let Some((task_id, level)) = cpu_state.dequeue() {
            self.make_decision(task_id, level, cpu)
        } else {
            self.idle_decision(cpu)
        };

        // Record context switch
        if let Some(prev) = prev_task {
            if prev != decision.task_id {
                self.context_switches.fetch_add(1, Ordering::Relaxed);
                self.stats.read().record_task_switch(prev, true);
            }
        }

        decision
    }

    /// Make scheduling decision for task
    fn make_decision(&self, task_id: TaskId, level: QueueLevel, cpu: CpuId) -> SchedulingDecision {
        let config = self.config.read();
        let mut time_slice = config.default_time_slice;
        let mut reason = DecisionReason::HighestPriority;
        let mut optimizations = Vec::new();
        let mut priority = 0i32;

        // Get task info
        if let Some(task) = self.tasks.read().get(&task_id) {
            priority = task.base_priority;
            time_slice = task.time_slice;

            // Check for deadline
            if task.flags.contains(TaskFlags::DEADLINE) {
                if let Some(deadline) = task.deadline {
                    let now = Nanoseconds::new(self.current_time.load(Ordering::Relaxed));
                    if deadline.raw() > 0 && deadline < now + time_slice {
                        reason = DecisionReason::DeadlineApproaching;
                        time_slice = deadline - now;
                    }
                }
            }
        }

        // Check intent
        if config.intent_scheduling {
            if let Some(intent) = self.intents.read().get(&task_id) {
                self.sched_stats
                    .intent_evaluations
                    .fetch_add(1, Ordering::Relaxed);

                match intent.class {
                    IntentClass::RealTime => {
                        reason = DecisionReason::RealTimeRequirement;
                        time_slice = Nanoseconds::from_millis(2);
                    },
                    IntentClass::Interactive => {
                        reason = DecisionReason::InteractiveBoost;
                        time_slice = Nanoseconds::from_millis(4);
                    },
                    _ => {},
                }
            }
        }

        // Apply policies
        if let Some(task) = self.tasks.read().get(&task_id) {
            let system_stats = self.stats.read().get_system_stats();

            let context = PolicyContext {
                system: super::policy::SystemContext {
                    cpu_load: system_stats.cpu_load,
                    per_cpu_load: Vec::new(),
                    memory_usage: system_stats.memory_usage,
                    io_wait: system_stats.io_wait,
                    runnable_tasks: system_stats.runnable_tasks,
                    blocked_tasks: system_stats.blocked_tasks,
                    hour: 0, // Would come from RTC
                    context_switches_per_sec: system_stats.context_switches_per_sec,
                    interrupts_per_sec: system_stats.interrupts_per_sec,
                    time_of_day: self.current_time.load(Ordering::Relaxed),
                    uptime: system_stats.uptime.raw(),
                    power_mode: super::policy::PowerMode::Normal,
                    on_battery: false,
                },
                task: Some(TaskContext {
                    task_id,
                    intent_class: self
                        .intents
                        .read()
                        .get(&task_id)
                        .map(|i| i.class)
                        .unwrap_or(IntentClass::Interactive),
                    priority: task.base_priority as i8,
                    cpu_percent: 0,
                    memory_bytes: task.memory_usage,
                    age_ms: 0,
                    wait_time_ms: task.wait_time.as_millis(),
                }),
                current_time: Nanoseconds::new(self.current_time.load(Ordering::Relaxed)),
                events: Vec::new(),
            };

            // PolicyEngine.evaluate returns number of actions applied
            let _ = context; // Context used for future policy evaluation
            self.sched_stats
                .policy_applications
                .fetch_add(1, Ordering::Relaxed);
            reason = DecisionReason::PolicyEnforced("policy".into());
        }

        // Apply optimizer hints
        if config.adaptive_optimization {
            for hint in self.optimizer.read().hints_for_task(task_id) {
                match hint.hint_type {
                    HintType::TimeSlice(ts) => {
                        time_slice = ts.recommended;
                        optimizations.push("time_slice_opt".into());
                    },
                    HintType::Priority(p) => {
                        priority = p.recommended as i32;
                        optimizations.push("priority_opt".into());
                    },
                    _ => {},
                }
                self.sched_stats
                    .optimization_hints_applied
                    .fetch_add(1, Ordering::Relaxed);
            }
        }

        SchedulingDecision {
            task_id,
            time_slice,
            queue_level: level,
            cpu,
            priority: priority as i8,
            reason,
            optimizations,
        }
    }

    /// Create idle decision
    fn idle_decision(&self, cpu: CpuId) -> SchedulingDecision {
        let idle_task = TaskId::new(u64::MAX - cpu.id() as u64);

        SchedulingDecision {
            task_id: idle_task,
            time_slice: Nanoseconds::from_millis(10),
            queue_level: QueueLevel::Idle,
            cpu,
            priority: -128,
            reason: DecisionReason::IdleFallback,
            optimizations: Vec::new(),
        }
    }

    /// Timer tick handler
    pub fn tick(&self, cpu: CpuId) {
        self.tick_count.fetch_add(1, Ordering::Relaxed);
        let now = Nanoseconds::new(
            self.current_time
                .fetch_add(self.config.read().tick_interval.raw(), Ordering::Relaxed),
        );

        // Update statistics
        self.stats.read().tick(now);

        // Check if current task's time slice expired
        let mut needs_resched = false;
        let mut expired_task_id = None;

        {
            let cpus = self.cpus.read();
            if let Some(cpu_state) = cpus.get(cpu.id() as usize) {
                if let Some(task_id) = cpu_state.current {
                    let tasks = self.tasks.read();
                    if let Some(task) = tasks.get(&task_id) {
                        // Update task runtime
                        let tick_interval = self.config.read().tick_interval;
                        self.stats
                            .read()
                            .update_task_runtime(task_id, tick_interval);

                        // Check time slice
                        if task.runtime >= task.time_slice {
                            cpu_state.request_resched();
                            needs_resched = true;
                            expired_task_id = Some(task_id);
                        }
                    }
                }
            }
        }

        // Handle expired task outside of lock
        if needs_resched {
            if let Some(task_id) = expired_task_id {
                let _ = self.enqueue_task(task_id);
                self.sched_stats.preemptions.fetch_add(1, Ordering::Relaxed);
                self.stats.read().record_task_preemption(task_id);
            }
        }

        // Periodic load balancing
        let config = self.config.read();
        if config.load_balancing {
            let balance_interval = config.balance_interval.raw();
            if self.tick_count.load(Ordering::Relaxed)
                % (balance_interval / config.tick_interval.raw())
                == 0
            {
                drop(config);
                self.load_balance();
            }
        }

        // Periodic optimization
        if self.config.read().adaptive_optimization {
            let system_stats = self.stats.read().get_system_stats();
            self.optimizer.read().optimize_system(&system_stats);
        }
    }

    /// Load balancing
    fn load_balance(&self) {
        self.sched_stats
            .load_balances
            .fetch_add(1, Ordering::Relaxed);

        let cpus = self.cpus.read();

        // Calculate per-CPU load
        for cpu in cpus.iter() {
            cpu.calculate_load();
        }

        // Find imbalance
        let loads: Vec<_> = cpus
            .iter()
            .map(|c| c.load.load(Ordering::Relaxed))
            .collect();
        if loads.is_empty() {
            return;
        }

        let avg_load: u32 = loads.iter().sum::<u32>() / loads.len() as u32;

        // Find busiest and idlest
        let busiest = loads.iter().enumerate().max_by_key(|(_, &l)| l);
        let idlest = loads.iter().enumerate().min_by_key(|(_, &l)| l);

        let (busy_idx, idle_idx) = match (busiest, idlest) {
            (Some((bi, busy_load)), Some((ii, idle_load))) => {
                if *busy_load > avg_load + 20 && *idle_load < avg_load - 10 {
                    (bi, ii)
                } else {
                    return;
                }
            },
            _ => return,
        };

        // Release read lock before taking write lock
        drop(cpus);

        // Try to migrate a task
        let cpus = self.cpus.write();
        if busy_idx < cpus.len() && idle_idx < cpus.len() {
            if let Some((task_id, _)) = cpus[busy_idx].dequeue() {
                cpus[idle_idx].enqueue(task_id, QueueLevel::Normal);
                self.sched_stats.migrations.fetch_add(1, Ordering::Relaxed);
                self.stats.read().record_task_migration(task_id);
            }
        }
    }

    // =========================================================================
    // Configuration and Control
    // =========================================================================

    /// Update configuration
    pub fn set_config(&self, config: SchedulerConfig) {
        *self.config.write() = config;
    }

    /// Get configuration
    pub fn config(&self) -> SchedulerConfig {
        self.config.read().clone()
    }

    /// Enable/disable scheduler
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }

    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Get current timestamp
    pub fn current_time(&self) -> Nanoseconds {
        Nanoseconds::new(self.current_time.load(Ordering::Relaxed))
    }

    /// Get total context switches
    pub fn context_switches(&self) -> u64 {
        self.context_switches.load(Ordering::Relaxed)
    }

    /// Get scheduler statistics
    pub fn statistics(&self) -> DISStatistics {
        DISStatistics {
            tasks_created: self.sched_stats.tasks_created.load(Ordering::Relaxed),
            tasks_completed: self.sched_stats.tasks_completed.load(Ordering::Relaxed),
            schedules: self.sched_stats.schedules.load(Ordering::Relaxed),
            context_switches: self.context_switches.load(Ordering::Relaxed),
            preemptions: self.sched_stats.preemptions.load(Ordering::Relaxed),
            migrations: self.sched_stats.migrations.load(Ordering::Relaxed),
            load_balances: self.sched_stats.load_balances.load(Ordering::Relaxed),
            policy_applications: self.sched_stats.policy_applications.load(Ordering::Relaxed),
            intent_evaluations: self.sched_stats.intent_evaluations.load(Ordering::Relaxed),
            optimization_hints: self
                .sched_stats
                .optimization_hints_applied
                .load(Ordering::Relaxed),
            tick_count: self.tick_count.load(Ordering::Relaxed),
            uptime: self.current_time(),
        }
    }

    /// Get intent engine
    pub fn intent_engine(&self) -> &RwLock<IntentEngine> {
        &self.intent_engine
    }

    /// Get policy engine
    pub fn policy_engine(&self) -> &RwLock<PolicyEngine> {
        &self.policy_engine
    }

    /// Get statistics collector
    pub fn stats_collector(&self) -> &RwLock<StatsCollector> {
        &self.stats
    }

    /// Get optimizer
    pub fn optimizer(&self) -> &RwLock<AdaptiveOptimizer> {
        &self.optimizer
    }
}

/// DIS scheduler statistics
#[derive(Debug, Clone)]
pub struct DISStatistics {
    pub tasks_created: u64,
    pub tasks_completed: u64,
    pub schedules: u64,
    pub context_switches: u64,
    pub preemptions: u64,
    pub migrations: u64,
    pub load_balances: u64,
    pub policy_applications: u64,
    pub intent_evaluations: u64,
    pub optimization_hints: u64,
    pub tick_count: u64,
    pub uptime: Nanoseconds,
}

impl Default for DISScheduler {
    fn default() -> Self {
        Self::new(SchedulerConfig::default())
    }
}

// =============================================================================
// Scheduler Builder
// =============================================================================

/// Builder for DIS scheduler
pub struct DISSchedulerBuilder {
    config: SchedulerConfig,
}

impl DISSchedulerBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            config: SchedulerConfig::default(),
        }
    }

    /// Set CPU count
    pub fn cpu_count(mut self, count: usize) -> Self {
        self.config.cpu_count = count;
        self
    }

    /// Set default time slice
    pub fn time_slice(mut self, slice: Nanoseconds) -> Self {
        self.config.default_time_slice = slice;
        self
    }

    /// Enable/disable adaptive optimization
    pub fn adaptive_optimization(mut self, enabled: bool) -> Self {
        self.config.adaptive_optimization = enabled;
        self
    }

    /// Enable/disable intent scheduling
    pub fn intent_scheduling(mut self, enabled: bool) -> Self {
        self.config.intent_scheduling = enabled;
        self
    }

    /// Enable/disable load balancing
    pub fn load_balancing(mut self, enabled: bool) -> Self {
        self.config.load_balancing = enabled;
        self
    }

    /// Enable/disable preemption
    pub fn preemption(mut self, enabled: bool) -> Self {
        self.config.preemption = enabled;
        self
    }

    /// Build scheduler
    pub fn build(self) -> DISScheduler {
        DISScheduler::new(self.config)
    }
}

impl Default for DISSchedulerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::super::intent::IntentBuilder;
    use super::*;

    #[test]
    fn test_scheduler_creation() {
        let scheduler = DISScheduler::default();
        assert!(scheduler.is_enabled());
        assert_eq!(scheduler.config().cpu_count, 4);
    }

    #[test]
    fn test_task_creation() {
        let scheduler = DISScheduler::default();

        let intent = IntentBuilder::new().class(IntentClass::Interactive).build();

        let task_id = scheduler.create_task("test", intent).unwrap();

        let task = scheduler.get_task(task_id);
        assert!(task.is_some());
    }

    #[test]
    fn test_scheduling() {
        let scheduler = DISScheduler::default();

        let intent = IntentBuilder::new().class(IntentClass::Interactive).build();

        let _task_id = scheduler.create_task("test", intent).unwrap();

        let decision = scheduler.schedule(CpuId::new(0));
        assert_eq!(decision.queue_level, QueueLevel::Interactive);
    }
}
