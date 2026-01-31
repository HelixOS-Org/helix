//! # Intent Engine - Core of Intent-Based Scheduling
//!
//! The Intent Engine is the revolutionary core of DIS. Instead of assigning
//! static priorities, tasks declare their **intentions** and the scheduler
//! optimizes globally.
//!
//! ## Intent Classes
//!
//! - **RealTime**: Hard deadlines, guaranteed execution
//! - **Interactive**: Low latency, responsive to user input
//! - **Throughput**: Maximum work done, batch processing
//! - **Background**: Run when nothing else needs CPU
//! - **Idle**: Only run when system is truly idle
//! - **Critical**: System-critical, cannot be preempted
//!
//! ## Intent Properties
//!
//! - Latency target (e.g., 16ms for 60 FPS)
//! - CPU budget (percentage or quota)
//! - Memory budget
//! - I/O budget
//! - Deadline (for real-time)
//! - Energy preference (performance vs power saving)
//!
//! ## How It Works
//!
//! ```text
//! Task Intent Declaration:
//!     "I am an interactive task"
//!     "I need <16ms latency"
//!     "I use ~30% CPU"
//!     "I'm I/O bound"
//!         │
//!         ▼
//! ┌─────────────────────────────────────────┐
//! │           INTENT ENGINE                  │
//! │                                          │
//! │  1. Parse and validate intent            │
//! │  2. Categorize (RT/Interactive/Batch)    │
//! │  3. Compute base priority                │
//! │  4. Set time slice                       │
//! │  5. Configure resource limits            │
//! │  6. Assign to appropriate queue          │
//! │                                          │
//! └─────────────────────────────────────────┘
//!         │
//!         ▼
//! Optimized Scheduling Decision
//! ```

use alloc::string::String;
use alloc::vec::Vec;
use alloc::boxed::Box;

use super::{
    Nanoseconds, CpuBudget, MemoryBudget, IoBudget, 
    TaskFlags, DISError, DISResult
};

// =============================================================================
// Intent Identification
// =============================================================================

/// Unique identifier for an intent
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct IntentId(pub u64);

impl IntentId {
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
    
    pub const fn raw(&self) -> u64 {
        self.0
    }
    
    pub const NULL: Self = Self(0);
}

// =============================================================================
// Intent Classes
// =============================================================================

/// Classification of task intent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum IntentClass {
    /// Hard real-time with deadlines
    /// Examples: Audio processing, motor control
    RealTime = 0,
    
    /// Soft real-time, low latency preferred
    /// Examples: UI rendering, games
    SoftRealTime = 1,
    
    /// Interactive, user-facing
    /// Examples: Shell, text editor
    Interactive = 2,
    
    /// Server workload, request-response
    /// Examples: Web server, database
    Server = 3,
    
    /// Throughput-oriented batch processing
    /// Examples: Compilation, data processing
    Batch = 4,
    
    /// Background tasks
    /// Examples: Indexing, backups
    Background = 5,
    
    /// Idle tasks, lowest priority
    /// Examples: Screen saver, garbage collection
    Idle = 6,
    
    /// System-critical, cannot be delayed
    /// Examples: OOM killer, watchdog
    Critical = 7,
}

impl Default for IntentClass {
    fn default() -> Self {
        Self::Interactive
    }
}

impl IntentClass {
    /// Get the base priority for this class (lower = higher priority)
    pub fn base_priority(&self) -> i32 {
        match self {
            Self::Critical => -100,
            Self::RealTime => -50,
            Self::SoftRealTime => -25,
            Self::Interactive => 0,
            Self::Server => 10,
            Self::Batch => 50,
            Self::Background => 100,
            Self::Idle => 127,
        }
    }
    
    /// Get the default time slice for this class
    pub fn default_time_slice(&self) -> Nanoseconds {
        match self {
            Self::Critical => Nanoseconds::from_millis(1),
            Self::RealTime => Nanoseconds::from_millis(2),
            Self::SoftRealTime => Nanoseconds::from_millis(4),
            Self::Interactive => Nanoseconds::from_millis(10),
            Self::Server => Nanoseconds::from_millis(20),
            Self::Batch => Nanoseconds::from_millis(100),
            Self::Background => Nanoseconds::from_millis(200),
            Self::Idle => Nanoseconds::from_millis(1000),
        }
    }
    
    /// Can this class preempt another?
    pub fn can_preempt(&self, other: &Self) -> bool {
        (*self as u8) < (*other as u8)
    }
    
    /// Is this a real-time class?
    pub fn is_realtime(&self) -> bool {
        matches!(self, Self::RealTime | Self::SoftRealTime | Self::Critical)
    }
    
    /// Is this an interactive class?
    pub fn is_interactive(&self) -> bool {
        matches!(self, Self::Interactive | Self::SoftRealTime)
    }
    
    /// Is this a batch class?
    pub fn is_batch(&self) -> bool {
        matches!(self, Self::Batch | Self::Background | Self::Idle)
    }
}

// =============================================================================
// Intent Flags
// =============================================================================

bitflags::bitflags! {
    /// Flags that modify intent behavior
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct IntentFlags: u64 {
        // Scheduling behavior
        /// Prefer low latency over throughput
        const LOW_LATENCY = 1 << 0;
        /// Prefer throughput over latency
        const HIGH_THROUGHPUT = 1 << 1;
        /// Task is I/O bound
        const IO_BOUND = 1 << 2;
        /// Task is CPU bound
        const CPU_BOUND = 1 << 3;
        /// Task is memory intensive
        const MEMORY_INTENSIVE = 1 << 4;
        
        // CPU affinity
        /// Pin to specific CPU
        const CPU_PINNED = 1 << 8;
        /// Prefer NUMA-local memory
        const NUMA_AWARE = 1 << 9;
        /// Can migrate between CPUs
        const MIGRATABLE = 1 << 10;
        
        // Preemption behavior
        /// Cannot be preempted
        const NO_PREEMPT = 1 << 16;
        /// Preempts others aggressively
        const PREEMPTIVE = 1 << 17;
        /// Yields voluntarily
        const COOPERATIVE = 1 << 18;
        
        // Energy management
        /// Prefer performance
        const PERF_PREFER = 1 << 24;
        /// Prefer power saving
        const POWER_PREFER = 1 << 25;
        /// Thermal-aware
        const THERMAL_AWARE = 1 << 26;
        
        // Security
        /// Runs in secure enclave
        const SECURE = 1 << 32;
        /// Isolated from other tasks
        const ISOLATED = 1 << 33;
        /// Has elevated privileges
        const PRIVILEGED = 1 << 34;
        
        // Deadline handling
        /// Has hard deadline
        const HARD_DEADLINE = 1 << 40;
        /// Has soft deadline
        const SOFT_DEADLINE = 1 << 41;
        /// Deadline can be extended
        const EXTENSIBLE_DEADLINE = 1 << 42;
        
        // Resource accounting
        /// Track resource usage precisely
        const PRECISE_ACCOUNTING = 1 << 48;
        /// Share resources with group
        const GROUP_SHARE = 1 << 49;
        /// Can borrow unused resources
        const RESOURCE_BORROW = 1 << 50;
    }
}

impl Default for IntentFlags {
    fn default() -> Self {
        Self::MIGRATABLE | Self::COOPERATIVE
    }
}

// =============================================================================
// Latency Target
// =============================================================================

/// Latency requirement specification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatencyTarget {
    /// No specific latency requirement
    None,
    /// Best effort, minimize when possible
    BestEffort,
    /// Target latency (soft)
    Target(Nanoseconds),
    /// Maximum allowed latency (hard)
    Maximum(Nanoseconds),
    /// Both target and maximum
    Bounded { target: Nanoseconds, max: Nanoseconds },
}

impl Default for LatencyTarget {
    fn default() -> Self {
        Self::None
    }
}

impl LatencyTarget {
    /// Get the target latency if specified
    pub fn target(&self) -> Option<Nanoseconds> {
        match self {
            Self::None | Self::BestEffort => None,
            Self::Target(t) | Self::Bounded { target: t, .. } => Some(*t),
            Self::Maximum(m) => Some(*m),
        }
    }
    
    /// Get the maximum latency if specified
    pub fn maximum(&self) -> Option<Nanoseconds> {
        match self {
            Self::None | Self::BestEffort | Self::Target(_) => None,
            Self::Maximum(m) | Self::Bounded { max: m, .. } => Some(*m),
        }
    }
    
    /// Common latency targets
    pub const REALTIME_AUDIO: Self = Self::Maximum(Nanoseconds::from_millis(5));
    pub const GAMING_60FPS: Self = Self::Target(Nanoseconds::from_millis(16));
    pub const GAMING_144FPS: Self = Self::Target(Nanoseconds::from_millis(7));
    pub const INTERACTIVE: Self = Self::Target(Nanoseconds::from_millis(50));
    pub const RESPONSIVE: Self = Self::Target(Nanoseconds::from_millis(100));
}

// =============================================================================
// Energy Preference
// =============================================================================

/// Energy/performance preference
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EnergyPreference {
    /// Maximum performance, ignore power
    Performance = 0,
    /// Slight bias towards performance
    BalancedPerformance = 1,
    /// Balance performance and power
    Balanced = 2,
    /// Slight bias towards power saving
    BalancedPower = 3,
    /// Maximum power saving
    PowerSave = 4,
}

impl Default for EnergyPreference {
    fn default() -> Self {
        Self::Balanced
    }
}

// =============================================================================
// Intent Structure
// =============================================================================

/// A complete intent specification
#[derive(Debug, Clone)]
pub struct Intent {
    /// Unique identifier
    pub id: IntentId,
    /// Intent class
    pub class: IntentClass,
    /// Intent flags
    pub flags: IntentFlags,
    /// Latency requirement
    pub latency: LatencyTarget,
    /// CPU budget
    pub cpu_budget: CpuBudget,
    /// Memory budget
    pub memory_budget: MemoryBudget,
    /// I/O budget
    pub io_budget: IoBudget,
    /// Energy preference
    pub energy: EnergyPreference,
    /// Deadline (for real-time tasks)
    pub deadline: Option<Nanoseconds>,
    /// Period (for periodic tasks)
    pub period: Option<Nanoseconds>,
    /// Execution time estimate
    pub exec_time_estimate: Option<Nanoseconds>,
    /// Nice value adjustment (-20 to +19)
    pub nice: i8,
    /// CPU affinity mask
    pub affinity_mask: Option<u64>,
    /// NUMA node preference
    pub numa_node: Option<u8>,
    /// Scheduling group ID
    pub group_id: Option<u64>,
    /// Description for debugging
    pub description: Option<String>,
}

impl Default for Intent {
    fn default() -> Self {
        Self {
            id: IntentId::NULL,
            class: IntentClass::Interactive,
            flags: IntentFlags::default(),
            latency: LatencyTarget::None,
            cpu_budget: CpuBudget::Unlimited,
            memory_budget: MemoryBudget::Unlimited,
            io_budget: IoBudget::Unlimited,
            energy: EnergyPreference::Balanced,
            deadline: None,
            period: None,
            exec_time_estimate: None,
            nice: 0,
            affinity_mask: None,
            numa_node: None,
            group_id: None,
            description: None,
        }
    }
}

impl Intent {
    /// Create a new intent with default values
    pub fn new() -> Self {
        Self {
            id: super::generate_intent_id(),
            ..Default::default()
        }
    }
    
    /// Create a builder for constructing intents
    pub fn builder() -> IntentBuilder {
        IntentBuilder::new()
    }
    
    /// Compute the base priority from this intent
    pub fn compute_base_priority(&self) -> i32 {
        let mut priority = self.class.base_priority();
        
        // Apply nice value
        priority = priority.saturating_add(self.nice as i32 * 2);
        
        // Low latency tasks get priority boost
        if self.flags.contains(IntentFlags::LOW_LATENCY) {
            priority = priority.saturating_sub(10);
        }
        
        // High throughput tasks get priority penalty
        if self.flags.contains(IntentFlags::HIGH_THROUGHPUT) {
            priority = priority.saturating_add(10);
        }
        
        // I/O bound tasks get slight boost
        if self.flags.contains(IntentFlags::IO_BOUND) {
            priority = priority.saturating_sub(5);
        }
        
        priority.clamp(-100, 127)
    }
    
    /// Compute flags for the task from this intent
    pub fn compute_flags(&self) -> TaskFlags {
        let mut flags = TaskFlags::empty();
        
        if self.class.is_realtime() {
            flags |= TaskFlags::REALTIME;
        }
        
        if self.class.is_interactive() {
            flags |= TaskFlags::INTERACTIVE;
        }
        
        if self.class.is_batch() {
            flags |= TaskFlags::BATCH;
        }
        
        if self.flags.contains(IntentFlags::NO_PREEMPT) {
            flags |= TaskFlags::NO_PREEMPT;
        }
        
        if self.flags.contains(IntentFlags::CPU_PINNED) {
            flags |= TaskFlags::PINNED;
        }
        
        if self.flags.contains(IntentFlags::IO_BOUND) {
            flags |= TaskFlags::IO_BOUND;
        }
        
        if self.flags.contains(IntentFlags::CPU_BOUND) {
            flags |= TaskFlags::CPU_BOUND;
        }
        
        if self.flags.contains(IntentFlags::LOW_LATENCY) {
            flags |= TaskFlags::LOW_LATENCY;
        }
        
        flags
    }
    
    /// Compute the time slice for this intent
    pub fn compute_time_slice(&self) -> Nanoseconds {
        let mut slice = self.class.default_time_slice();
        
        // Adjust based on latency target
        if let Some(target) = self.latency.target() {
            // Time slice should be smaller than latency target
            slice = slice.min(Nanoseconds::new(target.raw() / 2));
        }
        
        // Adjust based on nice value
        let nice_factor = if self.nice < 0 {
            100 + (-self.nice as u64 * 5) // Higher nice = more time
        } else {
            (100 - self.nice as u64 * 3).max(20) // Lower nice = less time
        };
        
        Nanoseconds::new(slice.raw() * nice_factor / 100)
    }
    
    /// Get the deadline if any
    pub fn deadline(&self) -> Option<Nanoseconds> {
        self.deadline.or_else(|| self.latency.maximum())
    }
    
    /// Validate this intent
    pub fn validate(&self) -> DISResult<()> {
        // Check for conflicting flags
        if self.flags.contains(IntentFlags::LOW_LATENCY) && 
           self.flags.contains(IntentFlags::HIGH_THROUGHPUT) {
            return Err(DISError::InvalidIntent);
        }
        
        if self.flags.contains(IntentFlags::PERF_PREFER) && 
           self.flags.contains(IntentFlags::POWER_PREFER) {
            return Err(DISError::InvalidIntent);
        }
        
        // Validate CPU budget
        if let CpuBudget::Percent(p) = self.cpu_budget {
            if p > 100 {
                return Err(DISError::InvalidIntent);
            }
        }
        
        // Validate nice value
        if self.nice < -20 || self.nice > 19 {
            return Err(DISError::InvalidIntent);
        }
        
        // Real-time tasks should have deadline
        if self.class == IntentClass::RealTime && 
           self.deadline.is_none() && 
           self.latency.maximum().is_none() {
            return Err(DISError::InvalidIntent);
        }
        
        Ok(())
    }
    
    /// Check if this intent is compatible with another
    pub fn compatible_with(&self, other: &Intent) -> bool {
        // Same group can coexist
        if self.group_id.is_some() && self.group_id == other.group_id {
            return true;
        }
        
        // Check for CPU conflicts
        if let (Some(my_affinity), Some(other_affinity)) = (self.affinity_mask, other.affinity_mask) {
            if my_affinity & other_affinity == 0 {
                return true; // Different CPUs
            }
        }
        
        // Critical tasks might conflict with real-time
        if self.class == IntentClass::Critical && other.class.is_realtime() {
            return false;
        }
        
        true
    }
}

// =============================================================================
// Intent Builder
// =============================================================================

/// Builder for constructing Intent objects
#[derive(Debug, Clone)]
pub struct IntentBuilder {
    intent: Intent,
}

impl IntentBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            intent: Intent::new(),
        }
    }
    
    /// Set the intent class
    pub fn class(mut self, class: IntentClass) -> Self {
        self.intent.class = class;
        self
    }
    
    /// Set intent flags
    pub fn flags(mut self, flags: IntentFlags) -> Self {
        self.intent.flags = flags;
        self
    }
    
    /// Add intent flags
    pub fn add_flags(mut self, flags: IntentFlags) -> Self {
        self.intent.flags |= flags;
        self
    }
    
    /// Set latency target
    pub fn latency(mut self, latency: LatencyTarget) -> Self {
        self.intent.latency = latency;
        self
    }
    
    /// Set latency target in milliseconds
    pub fn latency_ms(mut self, ms: u64) -> Self {
        self.intent.latency = LatencyTarget::Target(Nanoseconds::from_millis(ms));
        self
    }
    
    /// Set maximum latency in milliseconds
    pub fn max_latency_ms(mut self, ms: u64) -> Self {
        self.intent.latency = LatencyTarget::Maximum(Nanoseconds::from_millis(ms));
        self
    }
    
    /// Set CPU budget
    pub fn cpu_budget(mut self, budget: CpuBudget) -> Self {
        self.intent.cpu_budget = budget;
        self
    }
    
    /// Set CPU budget as percentage
    pub fn cpu_percent(mut self, percent: u8) -> Self {
        self.intent.cpu_budget = CpuBudget::Percent(percent.min(100));
        self
    }
    
    /// Set memory budget
    pub fn memory_budget(mut self, budget: MemoryBudget) -> Self {
        self.intent.memory_budget = budget;
        self
    }
    
    /// Set memory limit in bytes
    pub fn memory_limit(mut self, bytes: u64) -> Self {
        self.intent.memory_budget = MemoryBudget::Limit(bytes);
        self
    }
    
    /// Set I/O budget
    pub fn io_budget(mut self, budget: IoBudget) -> Self {
        self.intent.io_budget = budget;
        self
    }
    
    /// Set energy preference
    pub fn energy(mut self, energy: EnergyPreference) -> Self {
        self.intent.energy = energy;
        self
    }
    
    /// Set deadline
    pub fn deadline(mut self, deadline: Nanoseconds) -> Self {
        self.intent.deadline = Some(deadline);
        self
    }
    
    /// Set deadline in milliseconds
    pub fn deadline_ms(mut self, ms: u64) -> Self {
        self.intent.deadline = Some(Nanoseconds::from_millis(ms));
        self
    }
    
    /// Set period for periodic tasks
    pub fn period(mut self, period: Nanoseconds) -> Self {
        self.intent.period = Some(period);
        self
    }
    
    /// Set period in milliseconds
    pub fn period_ms(mut self, ms: u64) -> Self {
        self.intent.period = Some(Nanoseconds::from_millis(ms));
        self
    }
    
    /// Set execution time estimate
    pub fn exec_time(mut self, time: Nanoseconds) -> Self {
        self.intent.exec_time_estimate = Some(time);
        self
    }
    
    /// Set nice value
    pub fn nice(mut self, nice: i8) -> Self {
        self.intent.nice = nice.clamp(-20, 19);
        self
    }
    
    /// Set CPU affinity mask
    pub fn affinity(mut self, mask: u64) -> Self {
        self.intent.affinity_mask = Some(mask);
        if mask.count_ones() == 1 {
            self.intent.flags |= IntentFlags::CPU_PINNED;
        }
        self
    }
    
    /// Set NUMA node
    pub fn numa_node(mut self, node: u8) -> Self {
        self.intent.numa_node = Some(node);
        self.intent.flags |= IntentFlags::NUMA_AWARE;
        self
    }
    
    /// Set scheduling group
    pub fn group(mut self, group_id: u64) -> Self {
        self.intent.group_id = Some(group_id);
        self
    }
    
    /// Set description
    pub fn description(mut self, desc: &str) -> Self {
        self.intent.description = Some(String::from(desc));
        self
    }
    
    /// Mark as real-time
    pub fn realtime(mut self) -> Self {
        self.intent.class = IntentClass::RealTime;
        self.intent.flags |= IntentFlags::HARD_DEADLINE | IntentFlags::NO_PREEMPT;
        self
    }
    
    /// Mark as soft real-time
    pub fn soft_realtime(mut self) -> Self {
        self.intent.class = IntentClass::SoftRealTime;
        self.intent.flags |= IntentFlags::SOFT_DEADLINE | IntentFlags::LOW_LATENCY;
        self
    }
    
    /// Mark as interactive
    pub fn interactive(mut self) -> Self {
        self.intent.class = IntentClass::Interactive;
        self.intent.flags |= IntentFlags::LOW_LATENCY;
        self
    }
    
    /// Mark as server workload
    pub fn server(mut self) -> Self {
        self.intent.class = IntentClass::Server;
        self.intent.flags |= IntentFlags::HIGH_THROUGHPUT;
        self
    }
    
    /// Mark as batch processing
    pub fn batch(mut self) -> Self {
        self.intent.class = IntentClass::Batch;
        self.intent.flags |= IntentFlags::HIGH_THROUGHPUT | IntentFlags::CPU_BOUND;
        self
    }
    
    /// Mark as background
    pub fn background(mut self) -> Self {
        self.intent.class = IntentClass::Background;
        self.intent.flags |= IntentFlags::COOPERATIVE;
        self
    }
    
    /// Mark as idle
    pub fn idle(mut self) -> Self {
        self.intent.class = IntentClass::Idle;
        self.intent.flags |= IntentFlags::COOPERATIVE | IntentFlags::POWER_PREFER;
        self
    }
    
    /// Mark as I/O bound
    pub fn io_bound(mut self) -> Self {
        self.intent.flags |= IntentFlags::IO_BOUND;
        self.intent.flags.remove(IntentFlags::CPU_BOUND);
        self
    }
    
    /// Mark as CPU bound
    pub fn cpu_bound(mut self) -> Self {
        self.intent.flags |= IntentFlags::CPU_BOUND;
        self.intent.flags.remove(IntentFlags::IO_BOUND);
        self
    }
    
    /// Build and validate the intent
    pub fn build(self) -> DISResult<Intent> {
        self.intent.validate()?;
        Ok(self.intent)
    }
    
    /// Build without validation
    pub fn build_unchecked(self) -> Intent {
        self.intent
    }
}

impl Default for IntentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Intent Engine
// =============================================================================

/// The Intent Engine processes and manages task intents
pub struct IntentEngine {
    /// Registered intents
    intents: spin::RwLock<alloc::collections::BTreeMap<IntentId, Intent>>,
    /// Intent statistics
    intent_stats: spin::RwLock<IntentStats>,
    /// Intent validation rules
    validation_rules: Vec<Box<dyn Fn(&Intent) -> DISResult<()> + Send + Sync>>,
}

/// Statistics about intent usage
#[derive(Debug, Default)]
pub struct IntentStats {
    /// Total intents registered
    pub total_registered: u64,
    /// Active intents
    pub active: u64,
    /// Intents by class
    pub by_class: [u64; 8],
    /// Validation failures
    pub validation_failures: u64,
    /// Intent conflicts detected
    pub conflicts: u64,
}

impl IntentEngine {
    /// Create a new intent engine
    pub fn new() -> Self {
        Self {
            intents: spin::RwLock::new(alloc::collections::BTreeMap::new()),
            intent_stats: spin::RwLock::new(IntentStats::default()),
            validation_rules: Vec::new(),
        }
    }
    
    /// Register an intent
    pub fn register(&self, intent: Intent) -> DISResult<IntentId> {
        // Validate
        intent.validate()?;
        
        // Run custom validation rules
        for rule in &self.validation_rules {
            rule(&intent)?;
        }
        
        let id = intent.id;
        let class = intent.class as usize;
        
        self.intents.write().insert(id, intent);
        
        let mut stats = self.intent_stats.write();
        stats.total_registered += 1;
        stats.active += 1;
        if class < 8 {
            stats.by_class[class] += 1;
        }
        
        Ok(id)
    }
    
    /// Get an intent by ID
    pub fn get(&self, id: IntentId) -> Option<Intent> {
        self.intents.read().get(&id).cloned()
    }
    
    /// Unregister an intent
    pub fn unregister(&self, id: IntentId) -> Option<Intent> {
        let intent = self.intents.write().remove(&id)?;
        
        let class = intent.class as usize;
        let mut stats = self.intent_stats.write();
        stats.active = stats.active.saturating_sub(1);
        if class < 8 {
            stats.by_class[class] = stats.by_class[class].saturating_sub(1);
        }
        
        Some(intent)
    }
    
    /// Add a custom validation rule
    pub fn add_validation_rule<F>(&mut self, rule: F)
    where
        F: Fn(&Intent) -> DISResult<()> + Send + Sync + 'static,
    {
        self.validation_rules.push(Box::new(rule));
    }
    
    /// Get intent statistics
    pub fn stats(&self) -> IntentStats {
        IntentStats {
            total_registered: self.intent_stats.read().total_registered,
            active: self.intent_stats.read().active,
            by_class: self.intent_stats.read().by_class,
            validation_failures: self.intent_stats.read().validation_failures,
            conflicts: self.intent_stats.read().conflicts,
        }
    }
    
    /// Classify an intent automatically based on its properties
    pub fn auto_classify(intent: &mut Intent) {
        // If no class set, try to infer from flags and properties
        
        if intent.flags.contains(IntentFlags::HARD_DEADLINE) {
            intent.class = IntentClass::RealTime;
            return;
        }
        
        if intent.flags.contains(IntentFlags::SOFT_DEADLINE) {
            intent.class = IntentClass::SoftRealTime;
            return;
        }
        
        if let Some(latency) = intent.latency.target() {
            if latency.as_millis() < 10 {
                intent.class = IntentClass::RealTime;
            } else if latency.as_millis() < 50 {
                intent.class = IntentClass::SoftRealTime;
            } else if latency.as_millis() < 200 {
                intent.class = IntentClass::Interactive;
            }
            return;
        }
        
        if intent.flags.contains(IntentFlags::HIGH_THROUGHPUT) {
            if intent.flags.contains(IntentFlags::CPU_BOUND) {
                intent.class = IntentClass::Batch;
            } else {
                intent.class = IntentClass::Server;
            }
            return;
        }
        
        if intent.flags.contains(IntentFlags::COOPERATIVE) {
            intent.class = IntentClass::Background;
            return;
        }
        
        // Default to interactive
        intent.class = IntentClass::Interactive;
    }
    
    /// Check for conflicts between intents
    pub fn check_conflicts(&self, new_intent: &Intent) -> Vec<IntentId> {
        let mut conflicts = Vec::new();
        
        for (id, existing) in self.intents.read().iter() {
            if !new_intent.compatible_with(existing) {
                conflicts.push(*id);
            }
        }
        
        if !conflicts.is_empty() {
            self.intent_stats.write().conflicts += 1;
        }
        
        conflicts
    }
}

impl Default for IntentEngine {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Predefined Intent Templates
// =============================================================================

/// Common intent templates for quick use
pub mod templates {
    use super::*;
    
    /// Intent for audio processing (low latency)
    pub fn audio() -> Intent {
        Intent::builder()
            .realtime()
            .latency_ms(5)
            .cpu_percent(20)
            .io_bound()
            .description("Audio processing task")
            .build_unchecked()
    }
    
    /// Intent for video rendering (60 FPS)
    pub fn video_60fps() -> Intent {
        Intent::builder()
            .soft_realtime()
            .latency_ms(16)
            .cpu_percent(50)
            .cpu_bound()
            .description("60 FPS video rendering")
            .build_unchecked()
    }
    
    /// Intent for video rendering (144 FPS)
    pub fn video_144fps() -> Intent {
        Intent::builder()
            .soft_realtime()
            .latency_ms(7)
            .cpu_percent(70)
            .cpu_bound()
            .description("144 FPS video rendering")
            .build_unchecked()
    }
    
    /// Intent for UI/shell interaction
    pub fn ui() -> Intent {
        Intent::builder()
            .interactive()
            .latency_ms(50)
            .io_bound()
            .description("UI interaction task")
            .build_unchecked()
    }
    
    /// Intent for web server
    pub fn webserver() -> Intent {
        Intent::builder()
            .server()
            .latency_ms(100)
            .io_bound()
            .description("Web server request handler")
            .build_unchecked()
    }
    
    /// Intent for database
    pub fn database() -> Intent {
        Intent::builder()
            .server()
            .latency_ms(50)
            .io_bound()
            .cpu_percent(40)
            .description("Database query handler")
            .build_unchecked()
    }
    
    /// Intent for compilation
    pub fn compilation() -> Intent {
        Intent::builder()
            .batch()
            .cpu_bound()
            .cpu_percent(80)
            .description("Compilation task")
            .build_unchecked()
    }
    
    /// Intent for backup/sync
    pub fn backup() -> Intent {
        Intent::builder()
            .background()
            .io_bound()
            .energy(EnergyPreference::PowerSave)
            .description("Backup/sync task")
            .build_unchecked()
    }
    
    /// Intent for garbage collection
    pub fn gc() -> Intent {
        Intent::builder()
            .idle()
            .cpu_percent(10)
            .energy(EnergyPreference::PowerSave)
            .description("Garbage collection")
            .build_unchecked()
    }
    
    /// Intent for system monitoring
    pub fn monitoring() -> Intent {
        Intent::builder()
            .background()
            .period_ms(1000)
            .cpu_percent(5)
            .description("System monitoring")
            .build_unchecked()
    }
    
    /// Intent for batch processing
    pub fn batch() -> Intent {
        Intent::builder()
            .batch()
            .cpu_bound()
            .cpu_percent(80)
            .description("Batch processing task")
            .build_unchecked()
    }
    
    /// Intent for background tasks
    pub fn background() -> Intent {
        Intent::builder()
            .background()
            .energy(EnergyPreference::PowerSave)
            .description("Background task")
            .build_unchecked()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_intent_builder() {
        let intent = Intent::builder()
            .class(IntentClass::Interactive)
            .latency_ms(16)
            .cpu_percent(30)
            .build()
            .expect("Valid intent");
        
        assert_eq!(intent.class, IntentClass::Interactive);
        assert!(matches!(intent.cpu_budget, CpuBudget::Percent(30)));
    }
    
    #[test]
    fn test_intent_priority() {
        let rt = Intent::builder().realtime().deadline_ms(10).build().unwrap();
        let interactive = Intent::builder().interactive().build().unwrap();
        let batch = Intent::builder().batch().build().unwrap();
        
        assert!(rt.compute_base_priority() < interactive.compute_base_priority());
        assert!(interactive.compute_base_priority() < batch.compute_base_priority());
    }
    
    #[test]
    fn test_intent_validation() {
        // Valid intent
        assert!(Intent::builder()
            .interactive()
            .latency_ms(50)
            .build()
            .is_ok());
        
        // Invalid: conflicting flags
        let invalid = Intent::builder()
            .flags(IntentFlags::LOW_LATENCY | IntentFlags::HIGH_THROUGHPUT)
            .build();
        assert!(invalid.is_err());
    }
    
    #[test]
    fn test_intent_templates() {
        let audio = templates::audio();
        assert_eq!(audio.class, IntentClass::RealTime);
        
        let video = templates::video_60fps();
        assert_eq!(video.class, IntentClass::SoftRealTime);
        
        let batch = templates::compilation();
        assert_eq!(batch.class, IntentClass::Batch);
    }
}
