//! # Policy Engine - Adaptive Scheduling Policies
//!
//! The Policy Engine manages dynamic scheduling policies that can evolve
//! at runtime based on system behavior and optimization hints.
//!
//! ## Policy Types
//!
//! - **Static Policies**: Fixed rules that don't change
//! - **Adaptive Policies**: Rules that evolve based on statistics
//! - **Learning Policies**: Policies that learn from execution patterns
//!
//! ## Policy Rules
//!
//! Policies are expressed as rules with conditions and actions:
//!
//! ```text
//! RULE: high_cpu_load
//!   WHEN system.cpu_load > 80%
//!   AND  task.class == Interactive
//!   THEN boost_priority(task, +10)
//!   AND  reduce_timeslice(task, 50%)
//! ```
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     POLICY ENGINE                            │
//! │                                                              │
//! │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
//! │  │   Static     │  │  Adaptive    │  │  Learning    │       │
//! │  │   Policies   │  │  Policies    │  │  Policies    │       │
//! │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘       │
//! │         │                 │                 │                │
//! │         ▼                 ▼                 ▼                │
//! │  ┌──────────────────────────────────────────────────────┐   │
//! │  │              POLICY RULE EVALUATOR                    │   │
//! │  │                                                       │   │
//! │  │  Conditions    Actions     Priority    Cooldown       │   │
//! │  └──────────────────────────────────────────────────────┘   │
//! │                          │                                   │
//! │                          ▼                                   │
//! │  ┌──────────────────────────────────────────────────────┐   │
//! │  │              POLICY DECISION                          │   │
//! │  │                                                       │   │
//! │  │  • Priority adjustments                               │   │
//! │  │  • Time slice modifications                           │   │
//! │  │  • CPU affinity changes                               │   │
//! │  │  • Queue assignments                                  │   │
//! │  └──────────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────────┘
//! ```

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use spin::RwLock;

use super::{
    TaskId, Task, Nanoseconds,
    intent::{IntentClass, IntentFlags},
};

// =============================================================================
// Policy Identification
// =============================================================================

/// Unique identifier for a policy
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct PolicyId(pub u64);

impl PolicyId {
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
    
    pub const fn raw(&self) -> u64 {
        self.0
    }
    
    pub const NULL: Self = Self(0);
}

// =============================================================================
// Policy Conditions
// =============================================================================

/// Comparison operators for conditions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparison {
    Equal,
    NotEqual,
    LessThan,
    LessOrEqual,
    GreaterThan,
    GreaterOrEqual,
}

/// Logical operators for combining conditions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalOp {
    And,
    Or,
    Not,
}

/// A condition that can be evaluated
#[derive(Debug, Clone)]
pub enum Condition {
    // System-level conditions
    /// System CPU load (0-100)
    SystemCpuLoad { op: Comparison, threshold: u8 },
    /// System memory usage (0-100)
    SystemMemoryUsage { op: Comparison, threshold: u8 },
    /// System I/O wait (0-100)
    SystemIoWait { op: Comparison, threshold: u8 },
    /// Number of runnable tasks
    RunnableTasks { op: Comparison, count: u32 },
    /// Time of day (hour 0-23)
    TimeOfDay { op: Comparison, hour: u8 },
    
    // Task-level conditions
    /// Task class
    TaskClass { op: Comparison, class: IntentClass },
    /// Task priority
    TaskPriority { op: Comparison, priority: i32 },
    /// Task runtime
    TaskRuntime { op: Comparison, duration: Nanoseconds },
    /// Task wait time
    TaskWaitTime { op: Comparison, duration: Nanoseconds },
    /// Task CPU usage (0-100)
    TaskCpuUsage { op: Comparison, percent: u8 },
    /// Task has flag
    TaskHasFlag { flag: IntentFlags, expected: bool },
    /// Task age
    TaskAge { op: Comparison, duration: Nanoseconds },
    
    // Combined conditions
    /// All conditions must be true
    All(Vec<Condition>),
    /// Any condition must be true
    Any(Vec<Condition>),
    /// Condition must be false
    Not(Box<Condition>),
    
    // Custom condition
    Custom {
        name: String,
        evaluator: fn(&PolicyContext) -> bool,
    },
}

impl Condition {
    /// Evaluate this condition
    pub fn evaluate(&self, ctx: &PolicyContext) -> bool {
        match self {
            // System conditions
            Self::SystemCpuLoad { op, threshold } => {
                compare(*op, ctx.system.cpu_load, *threshold)
            }
            Self::SystemMemoryUsage { op, threshold } => {
                compare(*op, ctx.system.memory_usage, *threshold)
            }
            Self::SystemIoWait { op, threshold } => {
                compare(*op, ctx.system.io_wait, *threshold)
            }
            Self::RunnableTasks { op, count } => {
                compare(*op, ctx.system.runnable_tasks, *count)
            }
            Self::TimeOfDay { op, hour } => {
                compare(*op, ctx.system.hour, *hour)
            }
            
            // Task conditions
            Self::TaskClass { op, class } => {
                if let Some(task) = &ctx.task {
                    match op {
                        Comparison::Equal => task.intent_class == *class,
                        Comparison::NotEqual => task.intent_class != *class,
                        _ => false,
                    }
                } else {
                    false
                }
            }
            Self::TaskPriority { op, priority } => {
                if let Some(task) = &ctx.task {
                    compare(*op, task.priority as i32, *priority)
                } else {
                    false
                }
            }
            Self::TaskRuntime { op, duration } => {
                if let Some(task) = &ctx.task {
                    compare(*op, task.age_ms * 1_000_000, duration.raw())
                } else {
                    false
                }
            }
            Self::TaskWaitTime { op, duration } => {
                if let Some(task) = &ctx.task {
                    compare(*op, task.wait_time_ms * 1_000_000, duration.raw())
                } else {
                    false
                }
            }
            Self::TaskCpuUsage { op, percent } => {
                if let Some(task) = &ctx.task {
                    compare(*op, task.cpu_percent, *percent)
                } else {
                    false
                }
            }
            Self::TaskHasFlag { flag: _, expected } => {
                // Simplified: TaskContext doesn't have flags directly
                // Return expected == false as default
                !expected
            }
            Self::TaskAge { op, duration } => {
                if let Some(task) = &ctx.task {
                    compare(*op, task.age_ms * 1_000_000, duration.raw())
                } else {
                    false
                }
            }
            
            // Combined
            Self::All(conditions) => conditions.iter().all(|c| c.evaluate(ctx)),
            Self::Any(conditions) => conditions.iter().any(|c| c.evaluate(ctx)),
            Self::Not(condition) => !condition.evaluate(ctx),
            
            // Custom
            Self::Custom { evaluator, .. } => evaluator(ctx),
        }
    }
}

/// Compare two values using the given operator
fn compare<T: PartialOrd>(op: Comparison, a: T, b: T) -> bool {
    match op {
        Comparison::Equal => a == b,
        Comparison::NotEqual => a != b,
        Comparison::LessThan => a < b,
        Comparison::LessOrEqual => a <= b,
        Comparison::GreaterThan => a > b,
        Comparison::GreaterOrEqual => a >= b,
    }
}

// =============================================================================
// Policy Actions
// =============================================================================

/// Actions that can be taken by a policy
#[derive(Debug, Clone)]
pub enum PolicyAction {
    // Priority modifications
    /// Set absolute priority
    SetPriority(i32),
    /// Adjust priority by delta
    AdjustPriority(i32),
    /// Boost priority temporarily
    BoostPriority { amount: i32, duration: Nanoseconds },
    
    // Time slice modifications
    /// Set absolute time slice
    SetTimeSlice(Nanoseconds),
    /// Multiply time slice by factor
    ScaleTimeSlice(f32),
    
    // CPU affinity
    /// Set CPU affinity mask
    SetAffinity(u64),
    /// Pin to specific CPU
    PinToCpu(u32),
    /// Allow migration
    AllowMigration,
    
    // Queue assignment
    /// Move to specific queue
    MoveToQueue(QueueType),
    /// Change intent class
    ChangeClass(IntentClass),
    
    // Resource limits
    /// Set CPU budget
    SetCpuBudget(u8),
    /// Throttle task
    Throttle { percent: u8, duration: Nanoseconds },
    
    // Preemption
    /// Force preemption
    ForcePreempt,
    /// Disable preemption
    DisablePreempt(Nanoseconds),
    
    // Custom actions
    Custom {
        name: String,
        executor: fn(&mut Task, &PolicyContext),
    },
    
    // Composite actions
    Multiple(Vec<PolicyAction>),
}

/// Queue types for assignment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueType {
    RealTime,
    Interactive,
    Server,
    Batch,
    Background,
    Idle,
}

impl PolicyAction {
    /// Execute this action on a task
    pub fn execute(&self, task: &mut Task, ctx: &PolicyContext) {
        match self {
            Self::SetPriority(p) => {
                task.effective_priority = *p;
            }
            Self::AdjustPriority(delta) => {
                task.priority_adjustment = task.priority_adjustment.saturating_add(*delta);
                task.recalculate_priority();
            }
            Self::BoostPriority { amount, duration: _ } => {
                task.priority_adjustment = task.priority_adjustment.saturating_sub(*amount);
                task.recalculate_priority();
            }
            Self::SetTimeSlice(ts) => {
                task.time_slice = *ts;
            }
            Self::ScaleTimeSlice(factor) => {
                let new_slice = (task.time_slice.raw() as f32 * factor) as u64;
                task.time_slice = Nanoseconds::new(new_slice.max(1000)); // Min 1µs
            }
            Self::SetAffinity(mask) => {
                task.affinity = *mask;
            }
            Self::PinToCpu(cpu) => {
                task.affinity = 1u64 << cpu;
                task.flags |= super::TaskFlags::PINNED;
            }
            Self::AllowMigration => {
                task.affinity = u64::MAX;
                task.flags.remove(super::TaskFlags::PINNED);
            }
            Self::MoveToQueue(_queue) => {
                // Queue assignment is handled by scheduler
            }
            Self::ChangeClass(class) => {
                task.intent.class = *class;
                task.base_priority = class.base_priority();
                task.recalculate_priority();
            }
            Self::SetCpuBudget(_percent) => {
                // Resource accounting handled separately
            }
            Self::Throttle { .. } => {
                // Throttling handled by resource manager
            }
            Self::ForcePreempt => {
                task.preempt_count += 1;
            }
            Self::DisablePreempt(_) => {
                task.flags |= super::TaskFlags::NO_PREEMPT;
            }
            Self::Custom { executor, .. } => {
                executor(task, ctx);
            }
            Self::Multiple(actions) => {
                for action in actions {
                    action.execute(task, ctx);
                }
            }
        }
    }
}

// =============================================================================
// Policy Rule
// =============================================================================

/// A complete policy rule with condition and action
#[derive(Debug, Clone)]
pub struct PolicyRule {
    /// Rule name for debugging
    pub name: String,
    /// Rule priority (lower = evaluated first)
    pub priority: i32,
    /// Condition to evaluate
    pub condition: Condition,
    /// Action to take if condition is true
    pub action: PolicyAction,
    /// Whether rule is enabled
    pub enabled: bool,
    /// Cooldown between activations
    pub cooldown: Option<Nanoseconds>,
    /// Last activation time
    pub last_activated: Option<Nanoseconds>,
    /// Number of times activated
    pub activation_count: u64,
}

impl PolicyRule {
    /// Create a new policy rule
    pub fn new(name: &str, condition: Condition, action: PolicyAction) -> Self {
        Self {
            name: String::from(name),
            priority: 0,
            condition,
            action,
            enabled: true,
            cooldown: None,
            last_activated: None,
            activation_count: 0,
        }
    }
    
    /// Set rule priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
    
    /// Set cooldown
    pub fn with_cooldown(mut self, cooldown: Nanoseconds) -> Self {
        self.cooldown = Some(cooldown);
        self
    }
    
    /// Check if rule can be activated (respects cooldown)
    pub fn can_activate(&self, current_time: Nanoseconds) -> bool {
        if !self.enabled {
            return false;
        }
        
        if let (Some(cooldown), Some(last)) = (self.cooldown, self.last_activated) {
            return current_time >= last + cooldown;
        }
        
        true
    }
    
    /// Evaluate and potentially execute this rule
    pub fn evaluate(&mut self, task: &mut Task, ctx: &PolicyContext) -> bool {
        if !self.can_activate(ctx.current_time) {
            return false;
        }
        
        if self.condition.evaluate(ctx) {
            self.action.execute(task, ctx);
            self.last_activated = Some(ctx.current_time);
            self.activation_count += 1;
            return true;
        }
        
        false
    }
}

// =============================================================================
// Policy Context
// =============================================================================

/// Power mode for the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PowerMode {
    /// Performance mode
    Performance,
    /// Balanced mode
    #[default]
    Normal,
    /// Power saver mode
    PowerSaver,
    /// Low latency mode
    LowLatency,
}

/// Context for a specific task during policy evaluation
#[derive(Debug, Clone)]
pub struct TaskContext {
    /// Task ID
    pub task_id: TaskId,
    /// Intent class
    pub intent_class: IntentClass,
    /// Priority
    pub priority: i8,
    /// CPU usage percentage
    pub cpu_percent: u8,
    /// Memory usage in bytes
    pub memory_bytes: u64,
    /// Task age in milliseconds
    pub age_ms: u64,
    /// Wait time in milliseconds
    pub wait_time_ms: u64,
}

/// Context for policy evaluation
pub struct PolicyContext {
    /// Current system statistics
    pub system: SystemContext,
    /// Current task (if evaluating for a task)
    pub task: Option<TaskContext>,
    /// Current time
    pub current_time: Nanoseconds,
    /// Recent events
    pub events: Vec<PolicyEvent>,
}

/// System context for policy evaluation
#[derive(Debug, Clone, Default)]
pub struct SystemContext {
    /// CPU load (0-100)
    pub cpu_load: u8,
    /// Per-CPU load
    pub per_cpu_load: Vec<u8>,
    /// Memory usage (0-100)
    pub memory_usage: u8,
    /// I/O wait (0-100)
    pub io_wait: u8,
    /// Number of runnable tasks
    pub runnable_tasks: u32,
    /// Number of blocked tasks
    pub blocked_tasks: u32,
    /// Current hour (0-23)
    pub hour: u8,
    /// Context switches per second
    pub context_switches_per_sec: u64,
    /// Interrupts per second
    pub interrupts_per_sec: u64,
    /// Time of day (nanoseconds since epoch)
    pub time_of_day: u64,
    /// System uptime (nanoseconds)
    pub uptime: u64,
    /// Current power mode
    pub power_mode: PowerMode,
    /// Running on battery
    pub on_battery: bool,
}

/// Events that can trigger policy evaluation
#[derive(Debug, Clone)]
pub enum PolicyEvent {
    /// A task was created
    TaskCreated(TaskId),
    /// A task completed
    TaskCompleted(TaskId),
    /// A task was preempted
    TaskPreempted(TaskId),
    /// CPU load changed significantly
    CpuLoadChanged(u8),
    /// Memory pressure
    MemoryPressure(u8),
    /// Deadline was missed
    DeadlineMissed(TaskId),
}

// =============================================================================
// Policy
// =============================================================================

/// A complete scheduling policy
#[derive(Debug, Clone)]
pub struct Policy {
    /// Policy identifier
    pub id: PolicyId,
    /// Policy name
    pub name: String,
    /// Policy description
    pub description: String,
    /// Policy rules
    pub rules: Vec<PolicyRule>,
    /// Whether policy is enabled
    pub enabled: bool,
    /// Policy priority (lower = higher priority)
    pub priority: i32,
    /// Scope of the policy
    pub scope: PolicyScope,
    /// Statistics
    pub stats: PolicyStats,
}

/// Scope of a policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyScope {
    /// Applies to all tasks
    Global,
    /// Applies to specific intent class
    Class(IntentClass),
    /// Applies to specific task group
    Group(u64),
    /// Applies to specific task
    Task(TaskId),
}

/// Policy statistics
#[derive(Debug, Default, Clone)]
pub struct PolicyStats {
    /// Number of evaluations
    pub evaluations: u64,
    /// Number of matches
    pub matches: u64,
    /// Number of actions taken
    pub actions_taken: u64,
    /// Average evaluation time (ns)
    pub avg_eval_time: u64,
}

impl Policy {
    /// Create a new policy
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            id: super::generate_policy_id(),
            name: String::from(name),
            description: String::from(description),
            rules: Vec::new(),
            enabled: true,
            priority: 0,
            scope: PolicyScope::Global,
            stats: PolicyStats::default(),
        }
    }
    
    /// Add a rule to this policy
    pub fn add_rule(&mut self, rule: PolicyRule) {
        self.rules.push(rule);
        // Sort by priority
        self.rules.sort_by_key(|r| r.priority);
    }
    
    /// Evaluate this policy for a task
    pub fn evaluate(&mut self, task: &mut Task, ctx: &PolicyContext) -> u32 {
        if !self.enabled {
            return 0;
        }
        
        // Check scope
        match self.scope {
            PolicyScope::Global => {}
            PolicyScope::Class(class) => {
                if task.intent.class != class {
                    return 0;
                }
            }
            PolicyScope::Group(group) => {
                if task.intent.group_id != Some(group) {
                    return 0;
                }
            }
            PolicyScope::Task(id) => {
                if task.id != id {
                    return 0;
                }
            }
        }
        
        self.stats.evaluations += 1;
        
        let mut actions = 0;
        for rule in &mut self.rules {
            if rule.evaluate(task, ctx) {
                actions += 1;
            }
        }
        
        if actions > 0 {
            self.stats.matches += 1;
            self.stats.actions_taken += actions as u64;
        }
        
        actions
    }
}

// =============================================================================
// Policy Engine
// =============================================================================

/// The policy engine manages all policies
pub struct PolicyEngine {
    /// Registered policies
    policies: RwLock<BTreeMap<PolicyId, Policy>>,
    /// System context
    system_context: RwLock<SystemContext>,
    /// Event history
    events: RwLock<Vec<PolicyEvent>>,
    /// Engine statistics
    stats: RwLock<PolicyEngineStats>,
    /// Maximum events to keep
    max_events: usize,
}

/// Policy engine statistics
#[derive(Debug, Default)]
pub struct PolicyEngineStats {
    /// Total policy evaluations
    pub total_evaluations: u64,
    /// Total actions taken
    pub total_actions: u64,
    /// Average evaluation time
    pub avg_evaluation_time_ns: u64,
    /// Active policies
    pub active_policies: u32,
    /// Total rules
    pub total_rules: u32,
}

impl PolicyEngine {
    /// Create a new policy engine
    pub fn new() -> Self {
        let mut engine = Self {
            policies: RwLock::new(BTreeMap::new()),
            system_context: RwLock::new(SystemContext::default()),
            events: RwLock::new(Vec::new()),
            stats: RwLock::new(PolicyEngineStats::default()),
            max_events: 1000,
        };
        
        // Register default policies
        engine.register_default_policies();
        
        engine
    }
    
    /// Register a policy
    pub fn register(&self, policy: Policy) -> PolicyId {
        let id = policy.id;
        let rules_count = policy.rules.len();
        
        self.policies.write().insert(id, policy);
        
        let mut stats = self.stats.write();
        stats.active_policies += 1;
        stats.total_rules += rules_count as u32;
        
        id
    }
    
    /// Unregister a policy
    pub fn unregister(&self, id: PolicyId) -> Option<Policy> {
        let policy = self.policies.write().remove(&id)?;
        
        let mut stats = self.stats.write();
        stats.active_policies = stats.active_policies.saturating_sub(1);
        stats.total_rules = stats.total_rules.saturating_sub(policy.rules.len() as u32);
        
        Some(policy)
    }
    
    /// Get a policy by ID
    pub fn get(&self, id: PolicyId) -> Option<Policy> {
        self.policies.read().get(&id).cloned()
    }
    
    /// Update system context
    pub fn update_context(&self, ctx: SystemContext) {
        *self.system_context.write() = ctx;
    }
    
    /// Record an event
    pub fn record_event(&self, event: PolicyEvent) {
        let mut events = self.events.write();
        events.push(event);
        
        // Trim if too many
        if events.len() > self.max_events {
            let drain_count = events.len() - self.max_events;
            events.drain(0..drain_count);
        }
    }
    
    /// Evaluate all policies for a task
    pub fn evaluate(&self, task: &mut Task, current_time: Nanoseconds) -> u32 {
        let task_context = TaskContext {
            task_id: task.id,
            intent_class: task.intent.class,
            priority: task.base_priority as i8,
            cpu_percent: 0, // Computed from stats if available
            memory_bytes: task.memory_usage,
            age_ms: 0, // Computed from creation time
            wait_time_ms: 0, // Computed from wait start
        };
        
        let ctx = PolicyContext {
            system: self.system_context.read().clone(),
            task: Some(task_context),
            current_time,
            events: self.events.read().clone(),
        };
        
        let mut total_actions = 0;
        
        // Collect policies sorted by priority
        let mut policies: Vec<_> = self.policies.read()
            .iter()
            .map(|(id, p)| (*id, p.priority))
            .collect();
        policies.sort_by_key(|(_, p)| *p);
        
        // Evaluate in priority order
        for (id, _) in policies {
            if let Some(mut policy) = self.policies.write().remove(&id) {
                let actions = policy.evaluate(task, &ctx);
                total_actions += actions;
                self.policies.write().insert(id, policy);
            }
        }
        
        let mut stats = self.stats.write();
        stats.total_evaluations += 1;
        stats.total_actions += total_actions as u64;
        
        total_actions
    }
    
    /// Get engine statistics
    pub fn stats(&self) -> PolicyEngineStats {
        PolicyEngineStats {
            total_evaluations: self.stats.read().total_evaluations,
            total_actions: self.stats.read().total_actions,
            avg_evaluation_time_ns: self.stats.read().avg_evaluation_time_ns,
            active_policies: self.stats.read().active_policies,
            total_rules: self.stats.read().total_rules,
        }
    }
    
    /// Register default policies
    fn register_default_policies(&mut self) {
        // Policy: Boost starving tasks
        let mut starvation_policy = Policy::new(
            "starvation_prevention",
            "Boost priority of tasks that have been waiting too long",
        );
        starvation_policy.add_rule(PolicyRule::new(
            "boost_starving",
            Condition::TaskWaitTime {
                op: Comparison::GreaterThan,
                duration: Nanoseconds::from_millis(1000),
            },
            PolicyAction::AdjustPriority(-20),
        ).with_cooldown(Nanoseconds::from_millis(500)));
        
        // Policy: Penalize CPU hogs
        let mut cpu_hog_policy = Policy::new(
            "cpu_hog_penalty",
            "Reduce priority of tasks using too much CPU",
        );
        cpu_hog_policy.add_rule(PolicyRule::new(
            "penalize_cpu_hog",
            Condition::All(vec![
                Condition::TaskCpuUsage {
                    op: Comparison::GreaterThan,
                    percent: 90,
                },
                Condition::TaskHasFlag {
                    flag: IntentFlags::HIGH_THROUGHPUT,
                    expected: false,
                },
            ]),
            PolicyAction::AdjustPriority(10),
        ).with_cooldown(Nanoseconds::from_millis(1000)));
        
        // Policy: Boost interactive during low load
        let mut interactive_boost = Policy::new(
            "interactive_boost",
            "Boost interactive tasks when system load is low",
        );
        interactive_boost.scope = PolicyScope::Class(IntentClass::Interactive);
        interactive_boost.add_rule(PolicyRule::new(
            "boost_when_idle",
            Condition::SystemCpuLoad {
                op: Comparison::LessThan,
                threshold: 30,
            },
            PolicyAction::ScaleTimeSlice(2.0),
        ));
        
        // Policy: Throttle background during high load
        let mut background_throttle = Policy::new(
            "background_throttle",
            "Throttle background tasks during high load",
        );
        background_throttle.scope = PolicyScope::Class(IntentClass::Background);
        background_throttle.add_rule(PolicyRule::new(
            "throttle_background",
            Condition::SystemCpuLoad {
                op: Comparison::GreaterThan,
                threshold: 80,
            },
            PolicyAction::Multiple(vec![
                PolicyAction::AdjustPriority(20),
                PolicyAction::ScaleTimeSlice(0.25),
            ]),
        ));
        
        // Register all default policies
        self.register(starvation_policy);
        self.register(cpu_hog_policy);
        self.register(interactive_boost);
        self.register(background_throttle);
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Policy Builder
// =============================================================================

/// Builder for creating policies
pub struct PolicyBuilder {
    policy: Policy,
}

impl PolicyBuilder {
    /// Create a new policy builder
    pub fn new(name: &str) -> Self {
        Self {
            policy: Policy::new(name, ""),
        }
    }
    
    /// Set description
    pub fn description(mut self, desc: &str) -> Self {
        self.policy.description = String::from(desc);
        self
    }
    
    /// Set priority
    pub fn priority(mut self, priority: i32) -> Self {
        self.policy.priority = priority;
        self
    }
    
    /// Set scope
    pub fn scope(mut self, scope: PolicyScope) -> Self {
        self.policy.scope = scope;
        self
    }
    
    /// Add a rule
    pub fn rule(mut self, rule: PolicyRule) -> Self {
        self.policy.add_rule(rule);
        self
    }
    
    /// Add a simple rule (condition -> action)
    pub fn when(mut self, name: &str, condition: Condition, action: PolicyAction) -> Self {
        self.policy.add_rule(PolicyRule::new(name, condition, action));
        self
    }
    
    /// Build the policy
    pub fn build(self) -> Policy {
        self.policy
    }
}

// =============================================================================
// Predefined Policies
// =============================================================================

/// Predefined policy templates
pub mod templates {
    use super::*;
    
    /// Policy for real-time workloads
    pub fn realtime() -> Policy {
        PolicyBuilder::new("realtime")
            .description("Optimized for real-time workloads")
            .scope(PolicyScope::Class(IntentClass::RealTime))
            .when(
                "ensure_no_preempt",
                Condition::TaskHasFlag {
                    flag: IntentFlags::HARD_DEADLINE,
                    expected: true,
                },
                PolicyAction::DisablePreempt(Nanoseconds::from_millis(10)),
            )
            .build()
    }
    
    /// Policy for interactive workloads
    pub fn interactive() -> Policy {
        PolicyBuilder::new("interactive")
            .description("Optimized for interactive responsiveness")
            .scope(PolicyScope::Class(IntentClass::Interactive))
            .when(
                "boost_io_wait",
                Condition::TaskHasFlag {
                    flag: IntentFlags::IO_BOUND,
                    expected: true,
                },
                PolicyAction::AdjustPriority(-5),
            )
            .build()
    }
    
    /// Policy for server workloads
    pub fn server() -> Policy {
        PolicyBuilder::new("server")
            .description("Optimized for server request handling")
            .scope(PolicyScope::Class(IntentClass::Server))
            .when(
                "scale_with_load",
                Condition::SystemCpuLoad {
                    op: Comparison::GreaterThan,
                    threshold: 70,
                },
                PolicyAction::ScaleTimeSlice(0.75),
            )
            .build()
    }
    
    /// Policy for batch workloads
    pub fn batch() -> Policy {
        PolicyBuilder::new("batch")
            .description("Optimized for batch processing throughput")
            .scope(PolicyScope::Class(IntentClass::Batch))
            .when(
                "maximize_timeslice",
                Condition::SystemCpuLoad {
                    op: Comparison::LessThan,
                    threshold: 50,
                },
                PolicyAction::ScaleTimeSlice(4.0),
            )
            .build()
    }
    
    /// Policy for power saving
    pub fn power_save() -> Policy {
        PolicyBuilder::new("power_save")
            .description("Optimized for power saving")
            .when(
                "throttle_batch",
                Condition::All(vec![
                    Condition::TaskClass {
                        op: Comparison::Equal,
                        class: IntentClass::Batch,
                    },
                    Condition::SystemCpuLoad {
                        op: Comparison::GreaterThan,
                        threshold: 60,
                    },
                ]),
                PolicyAction::Throttle {
                    percent: 50,
                    duration: Nanoseconds::from_secs(1),
                },
            )
            .build()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_condition_evaluation() {
        let ctx = PolicyContext {
            system: SystemContext {
                cpu_load: 75,
                memory_usage: 50,
                ..Default::default()
            },
            task: None,
            current_time: Nanoseconds::ZERO,
            events: Vec::new(),
        };
        
        let cond = Condition::SystemCpuLoad {
            op: Comparison::GreaterThan,
            threshold: 50,
        };
        
        assert!(cond.evaluate(&ctx));
    }
    
    #[test]
    fn test_policy_builder() {
        let policy = PolicyBuilder::new("test")
            .description("Test policy")
            .priority(10)
            .scope(PolicyScope::Global)
            .when(
                "test_rule",
                Condition::SystemCpuLoad {
                    op: Comparison::GreaterThan,
                    threshold: 80,
                },
                PolicyAction::AdjustPriority(-10),
            )
            .build();
        
        assert_eq!(policy.name, "test");
        assert_eq!(policy.rules.len(), 1);
    }
}
