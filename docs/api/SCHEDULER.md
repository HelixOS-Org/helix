# Scheduler API Reference

<div align="center">

⚡ **Differentiated Intent Scheduler (DIS) Documentation**

*High-Performance Intent-Based Task Scheduling*

</div>

---

## Table of Contents

1. [Overview](#1-overview)
2. [DIS Architecture](#2-dis-architecture)
3. [Core Types](#3-core-types)
4. [Intent System](#4-intent-system)
5. [Task Management](#5-task-management)
6. [Scheduling Algorithms](#6-scheduling-algorithms)
7. [Queue Management](#7-queue-management)
8. [Executor](#8-executor)
9. [Statistics](#9-statistics)
10. [Module Schedulers](#10-module-schedulers)

---

## 1. Overview

### 1.1 What is DIS?

The Differentiated Intent Scheduler (DIS) is a novel scheduling approach that prioritizes tasks based on their declared **intent** rather than just priority levels.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    DIFFERENTIATED INTENT SCHEDULER                         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Traditional Scheduler:                                                     │
│  ═════════════════════                                                      │
│                                                                             │
│    Priority 0 ─────▶ [Task A] [Task B] [Task C]  ─▶ Run in order           │
│    Priority 1 ─────▶ [Task D] [Task E]           ─▶ Wait for P0            │
│    Priority 2 ─────▶ [Task F]                    ─▶ Wait for P0,P1         │
│                                                                             │
│                                                                             │
│  Intent-Based Scheduler (DIS):                                              │
│  ════════════════════════════                                               │
│                                                                             │
│    ┌──────────────┐     ┌──────────────┐     ┌──────────────┐              │
│    │   REALTIME   │     │ INTERACTIVE  │     │    BATCH     │              │
│    │              │     │              │     │              │              │
│    │ • Deadline   │     │ • Low latency│     │ • Throughput │              │
│    │ • Guaranteed │     │ • Responsive │     │ • Background │              │
│    │ • Hardware   │     │ • User input │     │ • Compute    │              │
│    └──────────────┘     └──────────────┘     └──────────────┘              │
│           │                    │                    │                       │
│           ▼                    ▼                    ▼                       │
│    ┌─────────────────────────────────────────────────────────┐             │
│    │              INTENT OPTIMIZER                           │             │
│    │  Analyzes intent + constraints + history                │             │
│    │  Produces optimal scheduling decisions                  │             │
│    └─────────────────────────────────────────────────────────┘             │
│                              │                                              │
│                              ▼                                              │
│    ┌─────────────────────────────────────────────────────────┐             │
│    │              EXECUTOR                                   │             │
│    │  Runs tasks on available CPUs                          │             │
│    └─────────────────────────────────────────────────────────┘             │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 1.2 Crate Structure

```
subsystems/dis/
├── mod.rs              # DIS crate root
├── api.rs              # Public API
├── executor.rs         # Task execution
├── intent.rs           # Intent definitions
├── ipc.rs              # IPC integration
├── isolation.rs        # Task isolation
├── optimizer.rs        # Scheduling optimization
├── policy.rs           # Scheduling policies
├── queues.rs           # Priority queues
├── scheduler.rs        # Core scheduler
└── stats.rs            # Statistics
```

---

## 2. DIS Architecture

### 2.1 Component Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         DIS COMPONENTS                                      │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                        API LAYER                                    │   │
│  │  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────┐        │   │
│  │  │  spawn()  │  │  yield()  │  │  sleep()  │  │   exit()  │        │   │
│  │  └───────────┘  └───────────┘  └───────────┘  └───────────┘        │   │
│  └────────────────────────────────┬────────────────────────────────────┘   │
│                                   │                                         │
│                                   ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                     INTENT ANALYZER                                 │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                 │   │
│  │  │   Intent    │  │ Constraints │  │  Deadline   │                 │   │
│  │  │  Classifier │  │   Parser    │  │  Calculator │                 │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘                 │   │
│  └────────────────────────────────┬────────────────────────────────────┘   │
│                                   │                                         │
│                                   ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                      SCHEDULER CORE                                 │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                 │   │
│  │  │   Policy    │  │  Optimizer  │  │   Queue     │                 │   │
│  │  │   Engine    │  │             │  │  Manager    │                 │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘                 │   │
│  └────────────────────────────────┬────────────────────────────────────┘   │
│                                   │                                         │
│                                   ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                       EXECUTOR                                      │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                 │   │
│  │  │   CPU 0     │  │   CPU 1     │  │   CPU N     │                 │   │
│  │  │  [Task A]   │  │  [Task B]   │  │  [Task C]   │                 │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘                 │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 Scheduling Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       SCHEDULING FLOW                                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                              ┌─────────────┐                                │
│                              │   spawn()   │                                │
│                              │   called    │                                │
│                              └──────┬──────┘                                │
│                                     │                                        │
│                                     ▼                                        │
│                         ┌───────────────────────┐                           │
│                         │  Create Task Control  │                           │
│                         │       Block (TCB)     │                           │
│                         └───────────┬───────────┘                           │
│                                     │                                        │
│                                     ▼                                        │
│                         ┌───────────────────────┐                           │
│                         │   Parse Intent and    │                           │
│                         │     Constraints       │                           │
│                         └───────────┬───────────┘                           │
│                                     │                                        │
│                                     ▼                                        │
│                         ┌───────────────────────┐                           │
│                         │   Select Queue based  │                           │
│                         │     on Intent Class   │                           │
│                         └───────────┬───────────┘                           │
│                                     │                                        │
│          ┌──────────────────────────┼──────────────────────────┐            │
│          │                          │                          │            │
│          ▼                          ▼                          ▼            │
│  ┌───────────────┐        ┌───────────────┐        ┌───────────────┐        │
│  │  RT Queue     │        │  Interactive  │        │  Batch Queue  │        │
│  │  (Deadline)   │        │    Queue      │        │               │        │
│  └───────┬───────┘        └───────┬───────┘        └───────┬───────┘        │
│          │                        │                        │                │
│          └──────────────────────────────────────────────────┘                │
│                                     │                                        │
│                                     ▼                                        │
│                         ┌───────────────────────┐                           │
│                         │     Optimizer         │                           │
│                         │  (Selects next task)  │                           │
│                         └───────────┬───────────┘                           │
│                                     │                                        │
│                                     ▼                                        │
│                         ┌───────────────────────┐                           │
│                         │  Context Switch to    │                           │
│                         │   Selected Task       │                           │
│                         └───────────────────────┘                           │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Core Types

### 3.1 Task Types

```rust
/// Task identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(pub u64);

impl TaskId {
    /// Invalid task ID
    pub const INVALID: TaskId = TaskId(0);
    
    /// Kernel idle task
    pub const IDLE: TaskId = TaskId(1);
    
    /// Generate new task ID
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(2);
        TaskId(COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

/// Task state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Ready to run
    Ready,
    /// Currently running
    Running,
    /// Blocked waiting for event
    Blocked,
    /// Sleeping for duration
    Sleeping,
    /// Terminated
    Terminated,
    /// Zombie (waiting for parent)
    Zombie,
}

/// Task Control Block
#[derive(Debug)]
pub struct Task {
    /// Task ID
    pub id: TaskId,
    
    /// Parent task ID
    pub parent: TaskId,
    
    /// Task name
    pub name: String,
    
    /// Current state
    pub state: TaskState,
    
    /// Intent declaration
    pub intent: Intent,
    
    /// CPU context
    pub context: TaskContext,
    
    /// Stack pointer
    pub stack: VirtAddr,
    
    /// Stack size
    pub stack_size: usize,
    
    /// Priority (computed from intent)
    pub priority: u8,
    
    /// CPU affinity mask
    pub affinity: CpuMask,
    
    /// Statistics
    pub stats: TaskStats,
    
    /// Deadline (for realtime tasks)
    pub deadline: Option<Timestamp>,
    
    /// Creation time
    pub created: Timestamp,
}

/// CPU context for context switching
#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct TaskContext {
    // General purpose registers
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub rsp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    
    // Segment registers
    pub cs: u64,
    pub ss: u64,
    pub ds: u64,
    pub es: u64,
    pub fs: u64,
    pub gs: u64,
    
    // Control registers
    pub rip: u64,
    pub rflags: u64,
    pub cr3: u64,
    
    // FPU/SSE state
    pub fpu_state: [u8; 512],
}

impl TaskContext {
    /// Create context for new task
    pub fn new(entry: fn(), stack_top: VirtAddr) -> Self {
        Self {
            rip: entry as u64,
            rsp: stack_top.as_u64(),
            rflags: 0x200, // Interrupts enabled
            cs: 0x08,      // Kernel code segment
            ss: 0x10,      // Kernel data segment
            ..Default::default()
        }
    }
}

/// Task statistics
#[derive(Debug, Clone, Default)]
pub struct TaskStats {
    /// Total CPU time used (nanoseconds)
    pub cpu_time: u64,
    
    /// User mode CPU time
    pub user_time: u64,
    
    /// System mode CPU time
    pub system_time: u64,
    
    /// Number of context switches
    pub context_switches: u64,
    
    /// Number of voluntary yields
    pub voluntary_yields: u64,
    
    /// Number of involuntary preemptions
    pub preemptions: u64,
    
    /// Times blocked
    pub blocks: u64,
    
    /// Times woken
    pub wakes: u64,
    
    /// Page faults
    pub page_faults: u64,
    
    /// Last scheduled time
    pub last_scheduled: Timestamp,
    
    /// Time slice remaining
    pub time_slice: u64,
}

/// CPU affinity mask
#[derive(Debug, Clone, Copy, Default)]
pub struct CpuMask(pub u64);

impl CpuMask {
    /// All CPUs
    pub const ALL: CpuMask = CpuMask(!0);
    
    /// Single CPU
    pub fn single(cpu: usize) -> Self {
        CpuMask(1 << cpu)
    }
    
    /// Check if CPU is in mask
    pub fn contains(&self, cpu: usize) -> bool {
        self.0 & (1 << cpu) != 0
    }
    
    /// Add CPU to mask
    pub fn add(&mut self, cpu: usize) {
        self.0 |= 1 << cpu;
    }
    
    /// Remove CPU from mask
    pub fn remove(&mut self, cpu: usize) {
        self.0 &= !(1 << cpu);
    }
    
    /// Count CPUs in mask
    pub fn count(&self) -> u32 {
        self.0.count_ones()
    }
}
```

### 3.2 Scheduler Error Types

```rust
/// Scheduler result type
pub type SchedResult<T> = Result<T, SchedError>;

/// Scheduler errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedError {
    /// Task not found
    TaskNotFound,
    /// Task already exists
    TaskExists,
    /// Invalid task state
    InvalidState,
    /// Invalid intent
    InvalidIntent,
    /// Deadline missed
    DeadlineMissed,
    /// No available CPU
    NoCpu,
    /// Queue full
    QueueFull,
    /// Resource exhausted
    ResourceExhausted,
    /// Permission denied
    PermissionDenied,
    /// Operation not supported
    NotSupported,
}

impl core::fmt::Display for SchedError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TaskNotFound => write!(f, "Task not found"),
            Self::TaskExists => write!(f, "Task already exists"),
            Self::InvalidState => write!(f, "Invalid task state"),
            Self::InvalidIntent => write!(f, "Invalid intent"),
            Self::DeadlineMissed => write!(f, "Deadline missed"),
            Self::NoCpu => write!(f, "No available CPU"),
            Self::QueueFull => write!(f, "Queue is full"),
            Self::ResourceExhausted => write!(f, "Resource exhausted"),
            Self::PermissionDenied => write!(f, "Permission denied"),
            Self::NotSupported => write!(f, "Operation not supported"),
        }
    }
}
```

---

## 4. Intent System

### 4.1 Intent Declaration

```rust
/// Task intent declaration
#[derive(Debug, Clone)]
pub struct Intent {
    /// Intent class
    pub class: IntentClass,
    
    /// Constraints
    pub constraints: IntentConstraints,
    
    /// Quality of Service parameters
    pub qos: QosParameters,
}

/// Intent class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentClass {
    /// Real-time with hard deadlines
    Realtime {
        /// Period in microseconds
        period_us: u64,
        /// Deadline relative to period start
        deadline_us: u64,
        /// Worst-case execution time
        wcet_us: u64,
    },
    
    /// Interactive/latency-sensitive
    Interactive {
        /// Target response time
        target_latency_us: u64,
    },
    
    /// Batch/throughput-oriented
    Batch,
    
    /// Background/idle priority
    Background,
    
    /// System/kernel task
    System,
}

impl IntentClass {
    /// Convert to priority level (lower = higher priority)
    pub fn to_priority(&self) -> u8 {
        match self {
            IntentClass::System => 0,
            IntentClass::Realtime { .. } => 10,
            IntentClass::Interactive { .. } => 50,
            IntentClass::Batch => 100,
            IntentClass::Background => 200,
        }
    }
    
    /// Get time quantum for this class
    pub fn time_quantum(&self) -> u64 {
        match self {
            IntentClass::System => 1_000_000,        // 1ms
            IntentClass::Realtime { wcet_us, .. } => *wcet_us,
            IntentClass::Interactive { .. } => 4_000_000,  // 4ms
            IntentClass::Batch => 10_000_000,        // 10ms
            IntentClass::Background => 50_000_000,   // 50ms
        }
    }
}

/// Intent constraints
#[derive(Debug, Clone, Default)]
pub struct IntentConstraints {
    /// Maximum memory usage
    pub max_memory: Option<usize>,
    
    /// CPU affinity requirement
    pub cpu_affinity: Option<CpuMask>,
    
    /// NUMA node preference
    pub numa_node: Option<u32>,
    
    /// I/O priority
    pub io_priority: IoPriority,
    
    /// Network priority
    pub net_priority: u8,
}

/// I/O priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IoPriority {
    /// Real-time I/O
    Realtime,
    /// Best-effort (default)
    #[default]
    BestEffort,
    /// Idle I/O
    Idle,
}

/// Quality of Service parameters
#[derive(Debug, Clone, Default)]
pub struct QosParameters {
    /// Minimum CPU share (0-1000 = 0-100%)
    pub cpu_min: u16,
    
    /// Maximum CPU share
    pub cpu_max: u16,
    
    /// Memory bandwidth limit (MB/s)
    pub mem_bandwidth: Option<u32>,
    
    /// I/O bandwidth limit (KB/s)
    pub io_bandwidth: Option<u32>,
}

impl Intent {
    /// Create real-time intent
    pub fn realtime(period_us: u64, deadline_us: u64, wcet_us: u64) -> Self {
        Self {
            class: IntentClass::Realtime {
                period_us,
                deadline_us,
                wcet_us,
            },
            constraints: IntentConstraints::default(),
            qos: QosParameters::default(),
        }
    }
    
    /// Create interactive intent
    pub fn interactive(target_latency_us: u64) -> Self {
        Self {
            class: IntentClass::Interactive { target_latency_us },
            constraints: IntentConstraints::default(),
            qos: QosParameters::default(),
        }
    }
    
    /// Create batch intent
    pub fn batch() -> Self {
        Self {
            class: IntentClass::Batch,
            constraints: IntentConstraints::default(),
            qos: QosParameters::default(),
        }
    }
    
    /// Create background intent
    pub fn background() -> Self {
        Self {
            class: IntentClass::Background,
            constraints: IntentConstraints::default(),
            qos: QosParameters::default(),
        }
    }
    
    /// Create system intent
    pub fn system() -> Self {
        Self {
            class: IntentClass::System,
            constraints: IntentConstraints::default(),
            qos: QosParameters::default(),
        }
    }
    
    /// Set CPU affinity
    pub fn with_affinity(mut self, mask: CpuMask) -> Self {
        self.constraints.cpu_affinity = Some(mask);
        self
    }
    
    /// Set memory limit
    pub fn with_memory_limit(mut self, limit: usize) -> Self {
        self.constraints.max_memory = Some(limit);
        self
    }
}
```

### 4.2 Intent Analyzer

```rust
/// Intent analyzer for scheduling decisions
pub struct IntentAnalyzer {
    /// Historical data for predictions
    history: HashMap<TaskId, TaskHistory>,
    
    /// System load information
    system_load: SystemLoad,
}

/// Task execution history
struct TaskHistory {
    /// Average execution time
    avg_execution_time: u64,
    
    /// Execution time variance
    variance: u64,
    
    /// Miss rate for deadlines
    deadline_miss_rate: f32,
    
    /// Samples collected
    sample_count: u32,
}

/// System load metrics
#[derive(Debug, Clone, Default)]
pub struct SystemLoad {
    /// Per-CPU load (0-100)
    pub cpu_load: [u8; 64],
    
    /// Active CPU count
    pub active_cpus: u32,
    
    /// Run queue length
    pub runqueue_length: u32,
    
    /// Memory pressure (0-100)
    pub memory_pressure: u8,
    
    /// I/O wait percentage
    pub io_wait: u8,
}

impl IntentAnalyzer {
    /// Create new analyzer
    pub fn new() -> Self {
        Self {
            history: HashMap::new(),
            system_load: SystemLoad::default(),
        }
    }
    
    /// Analyze intent and produce scheduling decision
    pub fn analyze(&self, task: &Task) -> SchedulingDecision {
        match task.intent.class {
            IntentClass::Realtime { period_us, deadline_us, wcet_us } => {
                self.analyze_realtime(task, period_us, deadline_us, wcet_us)
            }
            IntentClass::Interactive { target_latency_us } => {
                self.analyze_interactive(task, target_latency_us)
            }
            IntentClass::Batch => {
                self.analyze_batch(task)
            }
            IntentClass::Background => {
                self.analyze_background(task)
            }
            IntentClass::System => {
                SchedulingDecision {
                    queue: QueueType::System,
                    priority: 0,
                    time_slice: 1_000_000,
                    cpu_hint: None,
                }
            }
        }
    }
    
    fn analyze_realtime(&self, task: &Task, period: u64, deadline: u64, wcet: u64) -> SchedulingDecision {
        // EDF-like analysis
        let now = Timestamp::now();
        
        // Calculate absolute deadline
        let abs_deadline = if let Some(d) = task.deadline {
            d
        } else {
            Timestamp::from_nanos(now.as_nanos() + deadline * 1000)
        };
        
        // Find least-loaded CPU
        let cpu = self.find_best_cpu(&task.intent.constraints);
        
        SchedulingDecision {
            queue: QueueType::Realtime,
            priority: self.deadline_to_priority(abs_deadline),
            time_slice: wcet * 1000, // Convert to ns
            cpu_hint: cpu,
        }
    }
    
    fn analyze_interactive(&self, task: &Task, target_latency: u64) -> SchedulingDecision {
        // Boost priority based on how long task has waited
        let wait_time = self.get_wait_time(task);
        let dynamic_priority = self.compute_interactive_priority(target_latency, wait_time);
        
        SchedulingDecision {
            queue: QueueType::Interactive,
            priority: dynamic_priority,
            time_slice: 4_000_000, // 4ms
            cpu_hint: self.find_best_cpu(&task.intent.constraints),
        }
    }
    
    fn analyze_batch(&self, task: &Task) -> SchedulingDecision {
        SchedulingDecision {
            queue: QueueType::Batch,
            priority: 100,
            time_slice: 10_000_000, // 10ms
            cpu_hint: None,
        }
    }
    
    fn analyze_background(&self, task: &Task) -> SchedulingDecision {
        SchedulingDecision {
            queue: QueueType::Background,
            priority: 200,
            time_slice: 50_000_000, // 50ms
            cpu_hint: None,
        }
    }
    
    fn find_best_cpu(&self, constraints: &IntentConstraints) -> Option<u32> {
        // Find least-loaded CPU matching constraints
        let affinity = constraints.cpu_affinity.unwrap_or(CpuMask::ALL);
        
        let mut best_cpu = None;
        let mut lowest_load = 101u8;
        
        for i in 0..self.system_load.active_cpus as usize {
            if affinity.contains(i) && self.system_load.cpu_load[i] < lowest_load {
                lowest_load = self.system_load.cpu_load[i];
                best_cpu = Some(i as u32);
            }
        }
        
        best_cpu
    }
    
    fn deadline_to_priority(&self, deadline: Timestamp) -> u8 {
        // Earlier deadline = higher priority (lower number)
        let now = Timestamp::now();
        let remaining = deadline.as_nanos().saturating_sub(now.as_nanos());
        
        // Map to 0-50 range (realtime priorities)
        core::cmp::min((remaining / 1_000_000_000) as u8, 50)
    }
    
    fn get_wait_time(&self, task: &Task) -> u64 {
        let now = Timestamp::now();
        now.as_nanos().saturating_sub(task.stats.last_scheduled.as_nanos())
    }
    
    fn compute_interactive_priority(&self, target: u64, wait: u64) -> u8 {
        // Boost priority if waiting longer than target
        let base = 50u8;
        if wait > target * 1000 {
            let boost = ((wait - target * 1000) / 1_000_000) as u8;
            base.saturating_sub(core::cmp::min(boost, 40))
        } else {
            base
        }
    }
    
    /// Update system load
    pub fn update_load(&mut self, load: SystemLoad) {
        self.system_load = load;
    }
    
    /// Record task completion for history
    pub fn record_completion(&mut self, task_id: TaskId, execution_time: u64) {
        let history = self.history.entry(task_id).or_insert(TaskHistory {
            avg_execution_time: execution_time,
            variance: 0,
            deadline_miss_rate: 0.0,
            sample_count: 1,
        });
        
        // Update running average
        history.sample_count += 1;
        let delta = execution_time as i64 - history.avg_execution_time as i64;
        history.avg_execution_time = 
            ((history.avg_execution_time as i64 + delta / history.sample_count as i64) as u64);
    }
}

/// Scheduling decision output
#[derive(Debug, Clone)]
pub struct SchedulingDecision {
    /// Target queue
    pub queue: QueueType,
    /// Priority within queue
    pub priority: u8,
    /// Time slice in nanoseconds
    pub time_slice: u64,
    /// Preferred CPU
    pub cpu_hint: Option<u32>,
}

/// Queue types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueType {
    System,
    Realtime,
    Interactive,
    Batch,
    Background,
}
```

---

## 5. Task Management

### 5.1 Task API

```rust
/// Task management API
pub trait TaskApi {
    /// Spawn a new task
    fn spawn(&mut self, entry: fn(), intent: Intent) -> SchedResult<TaskId>;
    
    /// Spawn named task
    fn spawn_named(&mut self, name: &str, entry: fn(), intent: Intent) -> SchedResult<TaskId>;
    
    /// Get current task ID
    fn current(&self) -> TaskId;
    
    /// Get task by ID
    fn get_task(&self, id: TaskId) -> SchedResult<&Task>;
    
    /// Get mutable task
    fn get_task_mut(&mut self, id: TaskId) -> SchedResult<&mut Task>;
    
    /// Yield current task
    fn yield_now(&mut self);
    
    /// Sleep for duration
    fn sleep(&mut self, duration: Duration);
    
    /// Block current task
    fn block(&mut self, reason: BlockReason) -> SchedResult<()>;
    
    /// Wake a blocked task
    fn wake(&mut self, id: TaskId) -> SchedResult<()>;
    
    /// Exit current task
    fn exit(&mut self, code: i32) -> !;
    
    /// Kill a task
    fn kill(&mut self, id: TaskId) -> SchedResult<()>;
    
    /// Set task priority
    fn set_priority(&mut self, id: TaskId, priority: u8) -> SchedResult<()>;
    
    /// Set task affinity
    fn set_affinity(&mut self, id: TaskId, mask: CpuMask) -> SchedResult<()>;
}

/// Reason for blocking
#[derive(Debug, Clone, Copy)]
pub enum BlockReason {
    /// Waiting for I/O
    Io,
    /// Waiting for mutex
    Mutex,
    /// Waiting for semaphore
    Semaphore,
    /// Waiting for event
    Event,
    /// Waiting for child
    ChildExit,
    /// Waiting for IPC
    Ipc,
}

/// Sleep duration
#[derive(Debug, Clone, Copy)]
pub struct Duration {
    pub secs: u64,
    pub nanos: u32,
}

impl Duration {
    /// Create from milliseconds
    pub const fn from_millis(ms: u64) -> Self {
        Self {
            secs: ms / 1000,
            nanos: ((ms % 1000) * 1_000_000) as u32,
        }
    }
    
    /// Create from microseconds
    pub const fn from_micros(us: u64) -> Self {
        Self {
            secs: us / 1_000_000,
            nanos: ((us % 1_000_000) * 1000) as u32,
        }
    }
    
    /// Total nanoseconds
    pub const fn as_nanos(&self) -> u64 {
        self.secs * 1_000_000_000 + self.nanos as u64
    }
}
```

### 5.2 Task Manager Implementation

```rust
/// Task manager
pub struct TaskManager {
    /// All tasks
    tasks: HashMap<TaskId, Task>,
    
    /// Current task per CPU
    current: [TaskId; 64],
    
    /// Ready queues
    queues: SchedulerQueues,
    
    /// Intent analyzer
    analyzer: IntentAnalyzer,
    
    /// Statistics
    stats: SchedulerStats,
}

impl TaskManager {
    /// Create new task manager
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            current: [TaskId::IDLE; 64],
            queues: SchedulerQueues::new(),
            analyzer: IntentAnalyzer::new(),
            stats: SchedulerStats::default(),
        }
    }
    
    /// Initialize with idle task
    pub fn init(&mut self, cpu_count: usize) {
        // Create idle task for each CPU
        for cpu in 0..cpu_count {
            let idle = Task {
                id: TaskId::IDLE,
                parent: TaskId::INVALID,
                name: format!("idle/{}", cpu),
                state: TaskState::Ready,
                intent: Intent::background(),
                context: TaskContext::default(),
                stack: VirtAddr::new(0),
                stack_size: 0,
                priority: 255,
                affinity: CpuMask::single(cpu),
                stats: TaskStats::default(),
                deadline: None,
                created: Timestamp::now(),
            };
            
            // Don't add to queues, just keep as fallback
        }
    }
}

impl TaskApi for TaskManager {
    fn spawn(&mut self, entry: fn(), intent: Intent) -> SchedResult<TaskId> {
        self.spawn_named("", entry, intent)
    }
    
    fn spawn_named(&mut self, name: &str, entry: fn(), intent: Intent) -> SchedResult<TaskId> {
        let id = TaskId::new();
        
        // Allocate stack
        let stack_size = 64 * 1024; // 64KB
        let stack = self.allocate_stack(stack_size)?;
        
        // Create context
        let context = TaskContext::new(entry, stack + stack_size);
        
        // Analyze intent
        let decision = self.analyzer.analyze_intent(&intent);
        
        let task = Task {
            id,
            parent: self.current(),
            name: name.to_string(),
            state: TaskState::Ready,
            intent: intent.clone(),
            context,
            stack,
            stack_size,
            priority: decision.priority,
            affinity: intent.constraints.cpu_affinity.unwrap_or(CpuMask::ALL),
            stats: TaskStats {
                time_slice: decision.time_slice,
                ..Default::default()
            },
            deadline: self.compute_deadline(&intent),
            created: Timestamp::now(),
        };
        
        // Add to appropriate queue
        self.queues.enqueue(&task, decision.queue)?;
        
        // Store task
        self.tasks.insert(id, task);
        
        self.stats.tasks_created += 1;
        
        serial_println!("[SCHED] Spawned task {} '{}'", id.0, name);
        
        Ok(id)
    }
    
    fn current(&self) -> TaskId {
        let cpu = self.get_cpu_id();
        self.current[cpu]
    }
    
    fn get_task(&self, id: TaskId) -> SchedResult<&Task> {
        self.tasks.get(&id).ok_or(SchedError::TaskNotFound)
    }
    
    fn get_task_mut(&mut self, id: TaskId) -> SchedResult<&mut Task> {
        self.tasks.get_mut(&id).ok_or(SchedError::TaskNotFound)
    }
    
    fn yield_now(&mut self) {
        let current = self.current();
        
        if let Some(task) = self.tasks.get_mut(&current) {
            task.stats.voluntary_yields += 1;
            task.state = TaskState::Ready;
            
            // Re-enqueue
            let decision = self.analyzer.analyze(task);
            let _ = self.queues.enqueue(task, decision.queue);
        }
        
        // Trigger reschedule
        self.schedule();
    }
    
    fn sleep(&mut self, duration: Duration) {
        let current = self.current();
        let wake_time = Timestamp::now().as_nanos() + duration.as_nanos();
        
        if let Some(task) = self.tasks.get_mut(&current) {
            task.state = TaskState::Sleeping;
        }
        
        // Add to sleep queue
        self.queues.add_sleeper(current, wake_time);
        
        // Schedule another task
        self.schedule();
    }
    
    fn block(&mut self, reason: BlockReason) -> SchedResult<()> {
        let current = self.current();
        
        let task = self.tasks.get_mut(&current)
            .ok_or(SchedError::TaskNotFound)?;
        
        task.state = TaskState::Blocked;
        task.stats.blocks += 1;
        
        // Schedule another task
        self.schedule();
        
        Ok(())
    }
    
    fn wake(&mut self, id: TaskId) -> SchedResult<()> {
        let task = self.tasks.get_mut(&id)
            .ok_or(SchedError::TaskNotFound)?;
        
        if task.state != TaskState::Blocked && task.state != TaskState::Sleeping {
            return Err(SchedError::InvalidState);
        }
        
        task.state = TaskState::Ready;
        task.stats.wakes += 1;
        
        // Re-enqueue
        let decision = self.analyzer.analyze(task);
        self.queues.enqueue(task, decision.queue)?;
        
        Ok(())
    }
    
    fn exit(&mut self, code: i32) -> ! {
        let current = self.current();
        
        if let Some(task) = self.tasks.get_mut(&current) {
            task.state = TaskState::Zombie;
            serial_println!("[SCHED] Task {} exited with code {}", current.0, code);
        }
        
        self.stats.tasks_exited += 1;
        
        // Schedule another task
        self.schedule();
        
        // Should never return
        loop {
            unsafe { core::arch::asm!("hlt") };
        }
    }
    
    fn kill(&mut self, id: TaskId) -> SchedResult<()> {
        let task = self.tasks.get_mut(&id)
            .ok_or(SchedError::TaskNotFound)?;
        
        task.state = TaskState::Terminated;
        
        // Remove from queues
        self.queues.remove(id);
        
        // Free stack
        self.free_stack(task.stack, task.stack_size);
        
        self.stats.tasks_killed += 1;
        
        Ok(())
    }
    
    fn set_priority(&mut self, id: TaskId, priority: u8) -> SchedResult<()> {
        let task = self.tasks.get_mut(&id)
            .ok_or(SchedError::TaskNotFound)?;
        
        task.priority = priority;
        
        // May need to requeue
        if task.state == TaskState::Ready {
            self.queues.update_priority(id, priority);
        }
        
        Ok(())
    }
    
    fn set_affinity(&mut self, id: TaskId, mask: CpuMask) -> SchedResult<()> {
        let task = self.tasks.get_mut(&id)
            .ok_or(SchedError::TaskNotFound)?;
        
        task.affinity = mask;
        
        Ok(())
    }
}
```

---

## 6. Scheduling Algorithms

### 6.1 Policy Interface

```rust
/// Scheduling policy trait
pub trait SchedulingPolicy: Send + Sync {
    /// Policy name
    fn name(&self) -> &'static str;
    
    /// Select next task to run
    fn select(&mut self, ready: &[&Task], current: Option<&Task>) -> Option<TaskId>;
    
    /// Called when task becomes ready
    fn on_ready(&mut self, task: &Task);
    
    /// Called when task blocks
    fn on_block(&mut self, task: &Task);
    
    /// Called when time slice expires
    fn on_tick(&mut self, task: &mut Task) -> bool;
    
    /// Compute time slice for task
    fn time_slice(&self, task: &Task) -> u64;
}

/// Round-robin policy
pub struct RoundRobinPolicy {
    /// Time quantum
    quantum: u64,
}

impl RoundRobinPolicy {
    pub fn new(quantum: u64) -> Self {
        Self { quantum }
    }
}

impl SchedulingPolicy for RoundRobinPolicy {
    fn name(&self) -> &'static str {
        "Round Robin"
    }
    
    fn select(&mut self, ready: &[&Task], _current: Option<&Task>) -> Option<TaskId> {
        // Simple FIFO selection
        ready.first().map(|t| t.id)
    }
    
    fn on_ready(&mut self, _task: &Task) {
        // Nothing special
    }
    
    fn on_block(&mut self, _task: &Task) {
        // Nothing special
    }
    
    fn on_tick(&mut self, task: &mut Task) -> bool {
        // Decrement time slice
        if task.stats.time_slice > 0 {
            task.stats.time_slice -= 1;
        }
        
        // Preempt if exhausted
        task.stats.time_slice == 0
    }
    
    fn time_slice(&self, _task: &Task) -> u64 {
        self.quantum
    }
}

/// Priority-based policy
pub struct PriorityPolicy {
    /// Number of priority levels
    levels: u8,
}

impl PriorityPolicy {
    pub fn new(levels: u8) -> Self {
        Self { levels }
    }
}

impl SchedulingPolicy for PriorityPolicy {
    fn name(&self) -> &'static str {
        "Priority"
    }
    
    fn select(&mut self, ready: &[&Task], _current: Option<&Task>) -> Option<TaskId> {
        // Select highest priority (lowest number)
        ready.iter()
            .min_by_key(|t| t.priority)
            .map(|t| t.id)
    }
    
    fn on_ready(&mut self, _task: &Task) {}
    fn on_block(&mut self, _task: &Task) {}
    
    fn on_tick(&mut self, task: &mut Task) -> bool {
        task.stats.time_slice == 0
    }
    
    fn time_slice(&self, task: &Task) -> u64 {
        // Higher priority = longer slice
        let base = 10_000_000u64; // 10ms
        base * (256 - task.priority as u64) / 256
    }
}

/// Earliest Deadline First policy
pub struct EdfPolicy;

impl SchedulingPolicy for EdfPolicy {
    fn name(&self) -> &'static str {
        "Earliest Deadline First"
    }
    
    fn select(&mut self, ready: &[&Task], _current: Option<&Task>) -> Option<TaskId> {
        // Select task with earliest deadline
        ready.iter()
            .filter(|t| t.deadline.is_some())
            .min_by_key(|t| t.deadline.unwrap().as_nanos())
            .map(|t| t.id)
            .or_else(|| ready.first().map(|t| t.id))
    }
    
    fn on_ready(&mut self, _task: &Task) {}
    fn on_block(&mut self, _task: &Task) {}
    
    fn on_tick(&mut self, task: &mut Task) -> bool {
        // Check deadline
        if let Some(deadline) = task.deadline {
            if Timestamp::now() >= deadline {
                // Deadline missed!
                return true;
            }
        }
        task.stats.time_slice == 0
    }
    
    fn time_slice(&self, task: &Task) -> u64 {
        // Use WCET if available
        match task.intent.class {
            IntentClass::Realtime { wcet_us, .. } => wcet_us * 1000,
            _ => 10_000_000, // 10ms default
        }
    }
}

/// Completely Fair Scheduler (CFS-like)
pub struct FairPolicy {
    /// Minimum granularity
    min_granularity: u64,
    /// Target latency
    target_latency: u64,
}

impl FairPolicy {
    pub fn new() -> Self {
        Self {
            min_granularity: 1_000_000,  // 1ms
            target_latency: 20_000_000,  // 20ms
        }
    }
    
    fn virtual_runtime(&self, task: &Task) -> u64 {
        // vruntime = actual_runtime * (base_weight / task_weight)
        let weight = 256 - task.priority as u64;
        task.stats.cpu_time * 128 / weight.max(1)
    }
}

impl SchedulingPolicy for FairPolicy {
    fn name(&self) -> &'static str {
        "Completely Fair"
    }
    
    fn select(&mut self, ready: &[&Task], _current: Option<&Task>) -> Option<TaskId> {
        // Select task with minimum virtual runtime
        ready.iter()
            .min_by_key(|t| self.virtual_runtime(t))
            .map(|t| t.id)
    }
    
    fn on_ready(&mut self, _task: &Task) {}
    fn on_block(&mut self, _task: &Task) {}
    
    fn on_tick(&mut self, task: &mut Task) -> bool {
        task.stats.time_slice == 0
    }
    
    fn time_slice(&self, task: &Task) -> u64 {
        // Slice proportional to weight
        let weight = 256 - task.priority as u64;
        let slice = self.target_latency * weight / 256;
        core::cmp::max(slice, self.min_granularity)
    }
}
```

---

## 7. Queue Management

### 7.1 Scheduler Queues

```rust
/// Multi-level queue system
pub struct SchedulerQueues {
    /// System queue (highest priority)
    system: PriorityQueue<TaskId>,
    
    /// Realtime queue (EDF ordered)
    realtime: DeadlineQueue,
    
    /// Interactive queue
    interactive: PriorityQueue<TaskId>,
    
    /// Batch queue
    batch: FifoQueue<TaskId>,
    
    /// Background queue (lowest priority)
    background: FifoQueue<TaskId>,
    
    /// Sleep queue
    sleeping: BinaryHeap<SleepEntry>,
}

/// Priority queue implementation
pub struct PriorityQueue<T> {
    /// Buckets by priority
    buckets: [VecDeque<T>; 256],
    
    /// Bitmap of non-empty buckets
    bitmap: [u64; 4],
    
    /// Total count
    count: usize,
}

impl<T> PriorityQueue<T> {
    pub fn new() -> Self {
        Self {
            buckets: core::array::from_fn(|_| VecDeque::new()),
            bitmap: [0; 4],
            count: 0,
        }
    }
    
    /// Insert with priority
    pub fn insert(&mut self, item: T, priority: u8) {
        self.buckets[priority as usize].push_back(item);
        self.bitmap[priority as usize / 64] |= 1 << (priority % 64);
        self.count += 1;
    }
    
    /// Get highest priority item
    pub fn pop(&mut self) -> Option<T> {
        // Find first non-empty bucket
        for i in 0..4 {
            if self.bitmap[i] != 0 {
                let bit = self.bitmap[i].trailing_zeros() as usize;
                let priority = i * 64 + bit;
                
                let item = self.buckets[priority].pop_front();
                
                if self.buckets[priority].is_empty() {
                    self.bitmap[i] &= !(1 << bit);
                }
                
                self.count -= 1;
                return item;
            }
        }
        None
    }
    
    /// Peek highest priority
    pub fn peek(&self) -> Option<&T> {
        for i in 0..4 {
            if self.bitmap[i] != 0 {
                let bit = self.bitmap[i].trailing_zeros() as usize;
                let priority = i * 64 + bit;
                return self.buckets[priority].front();
            }
        }
        None
    }
    
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    
    pub fn len(&self) -> usize {
        self.count
    }
}

/// Deadline-ordered queue for realtime tasks
pub struct DeadlineQueue {
    /// Tasks ordered by deadline
    tasks: BinaryHeap<DeadlineEntry>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct DeadlineEntry {
    task_id: TaskId,
    deadline: Timestamp,
}

impl Ord for DeadlineEntry {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        // Reverse order: earliest deadline = highest priority
        other.deadline.as_nanos().cmp(&self.deadline.as_nanos())
    }
}

impl PartialOrd for DeadlineEntry {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl DeadlineQueue {
    pub fn new() -> Self {
        Self {
            tasks: BinaryHeap::new(),
        }
    }
    
    pub fn insert(&mut self, task_id: TaskId, deadline: Timestamp) {
        self.tasks.push(DeadlineEntry { task_id, deadline });
    }
    
    pub fn pop(&mut self) -> Option<TaskId> {
        self.tasks.pop().map(|e| e.task_id)
    }
    
    pub fn peek(&self) -> Option<TaskId> {
        self.tasks.peek().map(|e| e.task_id)
    }
}

/// Simple FIFO queue
pub struct FifoQueue<T> {
    queue: VecDeque<T>,
}

impl<T> FifoQueue<T> {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }
    
    pub fn push(&mut self, item: T) {
        self.queue.push_back(item);
    }
    
    pub fn pop(&mut self) -> Option<T> {
        self.queue.pop_front()
    }
    
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

/// Sleep queue entry
#[derive(Debug, Clone, Eq, PartialEq)]
struct SleepEntry {
    task_id: TaskId,
    wake_time: u64,
}

impl Ord for SleepEntry {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        other.wake_time.cmp(&self.wake_time)
    }
}

impl PartialOrd for SleepEntry {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl SchedulerQueues {
    pub fn new() -> Self {
        Self {
            system: PriorityQueue::new(),
            realtime: DeadlineQueue::new(),
            interactive: PriorityQueue::new(),
            batch: FifoQueue::new(),
            background: FifoQueue::new(),
            sleeping: BinaryHeap::new(),
        }
    }
    
    /// Enqueue task
    pub fn enqueue(&mut self, task: &Task, queue: QueueType) -> SchedResult<()> {
        match queue {
            QueueType::System => {
                self.system.insert(task.id, task.priority);
            }
            QueueType::Realtime => {
                let deadline = task.deadline.unwrap_or(Timestamp::from_nanos(u64::MAX));
                self.realtime.insert(task.id, deadline);
            }
            QueueType::Interactive => {
                self.interactive.insert(task.id, task.priority);
            }
            QueueType::Batch => {
                self.batch.push(task.id);
            }
            QueueType::Background => {
                self.background.push(task.id);
            }
        }
        Ok(())
    }
    
    /// Get next task to run
    pub fn next(&mut self) -> Option<TaskId> {
        // Check queues in priority order
        if let Some(id) = self.system.pop() {
            return Some(id);
        }
        
        if let Some(id) = self.realtime.pop() {
            return Some(id);
        }
        
        if let Some(id) = self.interactive.pop() {
            return Some(id);
        }
        
        if let Some(id) = self.batch.pop() {
            return Some(id);
        }
        
        if let Some(id) = self.background.pop() {
            return Some(id);
        }
        
        None
    }
    
    /// Add sleeper
    pub fn add_sleeper(&mut self, task_id: TaskId, wake_time: u64) {
        self.sleeping.push(SleepEntry { task_id, wake_time });
    }
    
    /// Wake expired sleepers
    pub fn wake_sleepers(&mut self, now: u64) -> Vec<TaskId> {
        let mut woken = Vec::new();
        
        while let Some(entry) = self.sleeping.peek() {
            if entry.wake_time <= now {
                let entry = self.sleeping.pop().unwrap();
                woken.push(entry.task_id);
            } else {
                break;
            }
        }
        
        woken
    }
    
    /// Remove task from all queues
    pub fn remove(&mut self, _task_id: TaskId) {
        // Would need to implement removal for each queue type
    }
    
    /// Update task priority
    pub fn update_priority(&mut self, _task_id: TaskId, _priority: u8) {
        // Would need to reposition in queue
    }
    
    /// Total tasks queued
    pub fn total_queued(&self) -> usize {
        self.system.len() + 
        self.interactive.len() + 
        self.batch.queue.len() +
        self.background.queue.len()
    }
}
```

---

## 8. Executor

### 8.1 Task Executor

```rust
/// Task executor for running tasks on CPUs
pub struct Executor {
    /// Per-CPU state
    cpu_state: [CpuState; 64],
    
    /// Number of CPUs
    cpu_count: usize,
    
    /// Task manager reference
    task_manager: Arc<Mutex<TaskManager>>,
}

/// Per-CPU state
struct CpuState {
    /// Currently running task
    current: TaskId,
    
    /// Idle task for this CPU
    idle: TaskId,
    
    /// Local run queue
    local_queue: VecDeque<TaskId>,
    
    /// Is CPU idle?
    idle_flag: bool,
}

impl Executor {
    /// Create new executor
    pub fn new(cpu_count: usize, task_manager: Arc<Mutex<TaskManager>>) -> Self {
        Self {
            cpu_state: core::array::from_fn(|_| CpuState {
                current: TaskId::IDLE,
                idle: TaskId::IDLE,
                idle_flag: true,
                local_queue: VecDeque::new(),
            }),
            cpu_count,
            task_manager,
        }
    }
    
    /// Run the scheduler on current CPU
    pub fn schedule(&mut self) {
        let cpu = self.get_cpu_id();
        
        // Update current task state
        let mut tm = self.task_manager.lock();
        
        // Check for tasks to wake
        let now = Timestamp::now().as_nanos();
        let woken = tm.queues.wake_sleepers(now);
        for id in woken {
            if let Ok(task) = tm.get_task_mut(id) {
                task.state = TaskState::Ready;
                let decision = tm.analyzer.analyze(task);
                let _ = tm.queues.enqueue(task, decision.queue);
            }
        }
        
        // Select next task
        let next_id = tm.queues.next().unwrap_or(TaskId::IDLE);
        let current_id = self.cpu_state[cpu].current;
        
        if next_id == current_id {
            // Same task, continue running
            return;
        }
        
        // Context switch needed
        self.context_switch(cpu, current_id, next_id, &mut tm);
    }
    
    /// Perform context switch
    fn context_switch(&mut self, cpu: usize, from: TaskId, to: TaskId, tm: &mut TaskManager) {
        serial_println!("[EXEC] CPU{}: {} -> {}", cpu, from.0, to.0);
        
        // Save current context
        if let Ok(current) = tm.get_task_mut(from) {
            if current.state == TaskState::Running {
                current.state = TaskState::Ready;
                // Save CPU context here
                // current.context = save_context();
            }
            current.stats.context_switches += 1;
        }
        
        // Load new context
        if let Ok(next) = tm.get_task_mut(to) {
            next.state = TaskState::Running;
            next.stats.last_scheduled = Timestamp::now();
            next.stats.time_slice = next.intent.class.time_quantum();
            
            // Restore CPU context here
            // restore_context(&next.context);
        }
        
        self.cpu_state[cpu].current = to;
        self.cpu_state[cpu].idle_flag = to == TaskId::IDLE;
        
        tm.stats.context_switches += 1;
    }
    
    /// Timer tick handler
    pub fn tick(&mut self) {
        let cpu = self.get_cpu_id();
        let current = self.cpu_state[cpu].current;
        
        let mut tm = self.task_manager.lock();
        
        if let Ok(task) = tm.get_task_mut(current) {
            // Update CPU time
            task.stats.cpu_time += 1_000_000; // 1ms per tick
            
            // Decrement time slice
            if task.stats.time_slice > 0 {
                task.stats.time_slice = task.stats.time_slice.saturating_sub(1_000_000);
            }
            
            // Check if preemption needed
            if task.stats.time_slice == 0 {
                task.stats.preemptions += 1;
                drop(tm);
                self.schedule();
            }
        }
    }
    
    /// Get current CPU ID
    fn get_cpu_id(&self) -> usize {
        // Would read from APIC or similar
        0
    }
    
    /// Load balance across CPUs
    pub fn load_balance(&mut self) {
        // Find busiest and least busy CPUs
        let mut busiest = 0usize;
        let mut busiest_load = 0;
        let mut least_busy = 0usize;
        let mut least_load = usize::MAX;
        
        for i in 0..self.cpu_count {
            let load = self.cpu_state[i].local_queue.len();
            if load > busiest_load {
                busiest = i;
                busiest_load = load;
            }
            if load < least_load {
                least_busy = i;
                least_load = load;
            }
        }
        
        // Migrate tasks if imbalance is significant
        if busiest_load > least_load + 2 {
            if let Some(task) = self.cpu_state[busiest].local_queue.pop_back() {
                self.cpu_state[least_busy].local_queue.push_back(task);
                serial_println!("[EXEC] Migrated task from CPU{} to CPU{}", busiest, least_busy);
            }
        }
    }
}
```

---

## 9. Statistics

### 9.1 Scheduler Statistics

```rust
/// Scheduler statistics
#[derive(Debug, Clone, Default)]
pub struct SchedulerStats {
    /// Total tasks created
    pub tasks_created: u64,
    
    /// Total tasks exited
    pub tasks_exited: u64,
    
    /// Total tasks killed
    pub tasks_killed: u64,
    
    /// Total context switches
    pub context_switches: u64,
    
    /// Voluntary context switches
    pub voluntary_switches: u64,
    
    /// Involuntary context switches (preemptions)
    pub involuntary_switches: u64,
    
    /// Realtime deadline misses
    pub deadline_misses: u64,
    
    /// Total CPU time used (all tasks)
    pub total_cpu_time: u64,
    
    /// Per-queue statistics
    pub queue_stats: QueueStats,
}

/// Per-queue statistics
#[derive(Debug, Clone, Default)]
pub struct QueueStats {
    pub system: QueueStat,
    pub realtime: QueueStat,
    pub interactive: QueueStat,
    pub batch: QueueStat,
    pub background: QueueStat,
}

/// Single queue statistics
#[derive(Debug, Clone, Default)]
pub struct QueueStat {
    /// Current queue length
    pub length: u32,
    
    /// Maximum observed length
    pub max_length: u32,
    
    /// Total tasks enqueued
    pub enqueued: u64,
    
    /// Total tasks dequeued
    pub dequeued: u64,
    
    /// Average wait time (nanoseconds)
    pub avg_wait_time: u64,
}

/// Statistics collector
pub struct StatsCollector {
    /// Current statistics
    stats: SchedulerStats,
    
    /// Sample interval
    sample_interval: Duration,
    
    /// History for averaging
    history: VecDeque<SchedulerStats>,
    
    /// Maximum history size
    max_history: usize,
}

impl StatsCollector {
    /// Create new collector
    pub fn new() -> Self {
        Self {
            stats: SchedulerStats::default(),
            sample_interval: Duration::from_millis(1000),
            history: VecDeque::new(),
            max_history: 60, // 1 minute of history
        }
    }
    
    /// Record context switch
    pub fn record_switch(&mut self, voluntary: bool) {
        self.stats.context_switches += 1;
        if voluntary {
            self.stats.voluntary_switches += 1;
        } else {
            self.stats.involuntary_switches += 1;
        }
    }
    
    /// Record deadline miss
    pub fn record_deadline_miss(&mut self) {
        self.stats.deadline_misses += 1;
    }
    
    /// Record task creation
    pub fn record_task_created(&mut self) {
        self.stats.tasks_created += 1;
    }
    
    /// Record task exit
    pub fn record_task_exit(&mut self) {
        self.stats.tasks_exited += 1;
    }
    
    /// Take sample
    pub fn sample(&mut self) {
        self.history.push_back(self.stats.clone());
        if self.history.len() > self.max_history {
            self.history.pop_front();
        }
    }
    
    /// Get current stats
    pub fn current(&self) -> &SchedulerStats {
        &self.stats
    }
    
    /// Get average over history
    pub fn average(&self) -> SchedulerStats {
        if self.history.is_empty() {
            return self.stats.clone();
        }
        
        let mut avg = SchedulerStats::default();
        let count = self.history.len() as u64;
        
        for sample in &self.history {
            avg.context_switches += sample.context_switches;
            avg.deadline_misses += sample.deadline_misses;
            // ... average other fields
        }
        
        avg.context_switches /= count;
        avg.deadline_misses /= count;
        
        avg
    }
    
    /// Print statistics
    pub fn print(&self) {
        serial_println!("=== Scheduler Statistics ===");
        serial_println!("Tasks: created={}, exited={}, killed={}",
            self.stats.tasks_created,
            self.stats.tasks_exited,
            self.stats.tasks_killed);
        serial_println!("Context switches: {} (voluntary: {}, involuntary: {})",
            self.stats.context_switches,
            self.stats.voluntary_switches,
            self.stats.involuntary_switches);
        serial_println!("Deadline misses: {}", self.stats.deadline_misses);
    }
}
```

---

## 10. Module Schedulers

### 10.1 Scheduler Module Interface

```rust
/// Scheduler module interface
pub trait SchedulerModule: Module {
    /// Get scheduling policy
    fn policy(&self) -> Box<dyn SchedulingPolicy>;
    
    /// Get supported intent classes
    fn supported_intents(&self) -> Vec<IntentClass>;
    
    /// Can handle task?
    fn can_schedule(&self, task: &Task) -> bool;
}

/// Round-robin scheduler module
pub struct RoundRobinScheduler {
    quantum: u64,
}

impl Module for RoundRobinScheduler {
    fn name(&self) -> &'static str {
        "round_robin"
    }
    
    fn version(&self) -> Version {
        Version::new(1, 0, 0)
    }
    
    fn init(&mut self) -> ModuleResult<()> {
        serial_println!("[SCHEDULER] Round-robin initialized with {}ms quantum", 
            self.quantum / 1_000_000);
        Ok(())
    }
    
    fn cleanup(&mut self) -> ModuleResult<()> {
        Ok(())
    }
}

impl SchedulerModule for RoundRobinScheduler {
    fn policy(&self) -> Box<dyn SchedulingPolicy> {
        Box::new(RoundRobinPolicy::new(self.quantum))
    }
    
    fn supported_intents(&self) -> Vec<IntentClass> {
        vec![IntentClass::Batch, IntentClass::Background]
    }
    
    fn can_schedule(&self, task: &Task) -> bool {
        matches!(task.intent.class, IntentClass::Batch | IntentClass::Background)
    }
}

/// Export scheduler module
#[macro_export]
macro_rules! export_scheduler {
    ($scheduler:ty) => {
        #[no_mangle]
        pub extern "C" fn _helix_scheduler_create() -> *mut dyn SchedulerModule {
            let scheduler = Box::new(<$scheduler>::default());
            Box::into_raw(scheduler)
        }
    };
}
```

### 10.2 Module Registration

```rust
/// Scheduler registry
pub struct SchedulerRegistry {
    /// Registered schedulers
    schedulers: HashMap<String, Arc<dyn SchedulerModule>>,
    
    /// Active scheduler per intent class
    active: HashMap<IntentClass, String>,
}

impl SchedulerRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            schedulers: HashMap::new(),
            active: HashMap::new(),
        }
    }
    
    /// Register scheduler
    pub fn register(&mut self, scheduler: Arc<dyn SchedulerModule>) -> SchedResult<()> {
        let name = scheduler.name().to_string();
        
        if self.schedulers.contains_key(&name) {
            return Err(SchedError::TaskExists);
        }
        
        // Set as active for supported intents if none set
        for intent in scheduler.supported_intents() {
            if !self.active.contains_key(&intent) {
                self.active.insert(intent, name.clone());
            }
        }
        
        self.schedulers.insert(name, scheduler);
        
        Ok(())
    }
    
    /// Get scheduler for intent
    pub fn get_for_intent(&self, intent: &IntentClass) -> Option<&Arc<dyn SchedulerModule>> {
        self.active.get(intent)
            .and_then(|name| self.schedulers.get(name))
    }
    
    /// Set active scheduler for intent
    pub fn set_active(&mut self, intent: IntentClass, name: &str) -> SchedResult<()> {
        if !self.schedulers.contains_key(name) {
            return Err(SchedError::TaskNotFound);
        }
        
        self.active.insert(intent, name.to_string());
        Ok(())
    }
    
    /// List all schedulers
    pub fn list(&self) -> Vec<&str> {
        self.schedulers.keys().map(|s| s.as_str()).collect()
    }
}
```

---

## Summary

The DIS (Differentiated Intent Scheduler) provides:

1. **Intent-Based Scheduling**: Tasks declare intent, not just priority
2. **Multiple Queue Types**: System, Realtime, Interactive, Batch, Background
3. **Pluggable Policies**: Round-robin, Priority, EDF, Fair
4. **Advanced Analysis**: Historical data for predictions
5. **Load Balancing**: Work stealing across CPUs
6. **Statistics**: Comprehensive scheduling metrics
7. **Module System**: Hot-reloadable scheduler modules

For implementation details, see [subsystems/dis/](../../subsystems/dis/).

---

<div align="center">

⚡ *Intent-driven scheduling for predictable performance* ⚡

</div>
