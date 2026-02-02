//! # Core AI Types and Traits
//!
//! Fundamental types, traits, and error handling for the Helix AI subsystem.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::sync::atomic::{AtomicU64, Ordering};

// =============================================================================
// AI Configuration
// =============================================================================

/// Configuration for the Helix AI subsystem
#[derive(Debug, Clone)]
pub struct AiConfig {
    /// Enable intent recognition engine
    pub intent_engine_enabled: bool,

    /// Enable self-optimization
    pub self_optimization_enabled: bool,

    /// Enable self-healing capabilities
    pub self_healing_enabled: bool,

    /// Enable predictive security
    pub predictive_security_enabled: bool,

    /// Enable continuous learning
    pub continuous_learning_enabled: bool,

    /// Maximum memory budget for AI operations (bytes)
    pub memory_budget: usize,

    /// Maximum CPU time budget per decision cycle (microseconds)
    pub cpu_budget_us: u64,

    /// Minimum confidence threshold for autonomous actions
    pub min_confidence_threshold: f32,

    /// Maximum actions per second (rate limiting)
    pub max_actions_per_second: u32,

    /// Enable GPU acceleration if available
    pub gpu_acceleration: bool,

    /// Enable NPU acceleration if available
    pub npu_acceleration: bool,

    /// Safety level (higher = more conservative)
    pub safety_level: SafetyLevel,

    /// Enable AI decision logging
    pub decision_logging: bool,

    /// Pattern history retention (number of patterns)
    pub pattern_history_size: usize,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            intent_engine_enabled: true,
            self_optimization_enabled: true,
            self_healing_enabled: true,
            predictive_security_enabled: true,
            continuous_learning_enabled: true,
            memory_budget: 64 * 1024 * 1024, // 64 MB
            cpu_budget_us: 1000,             // 1ms per decision
            min_confidence_threshold: 0.75,
            max_actions_per_second: 100,
            gpu_acceleration: false,
            npu_acceleration: false,
            safety_level: SafetyLevel::Standard,
            decision_logging: true,
            pattern_history_size: 10000,
        }
    }
}

impl AiConfig {
    /// Create a minimal configuration for resource-constrained systems
    pub fn minimal() -> Self {
        Self {
            intent_engine_enabled: false,
            self_optimization_enabled: true,
            self_healing_enabled: true,
            predictive_security_enabled: true,
            continuous_learning_enabled: false,
            memory_budget: 8 * 1024 * 1024, // 8 MB
            cpu_budget_us: 500,
            min_confidence_threshold: 0.85,
            max_actions_per_second: 10,
            gpu_acceleration: false,
            npu_acceleration: false,
            safety_level: SafetyLevel::Paranoid,
            decision_logging: false,
            pattern_history_size: 1000,
        }
    }

    /// Create maximum capability configuration
    pub fn full() -> Self {
        Self {
            intent_engine_enabled: true,
            self_optimization_enabled: true,
            self_healing_enabled: true,
            predictive_security_enabled: true,
            continuous_learning_enabled: true,
            memory_budget: 256 * 1024 * 1024, // 256 MB
            cpu_budget_us: 5000,
            min_confidence_threshold: 0.6,
            max_actions_per_second: 1000,
            gpu_acceleration: true,
            npu_acceleration: true,
            safety_level: SafetyLevel::Relaxed,
            decision_logging: true,
            pattern_history_size: 100000,
        }
    }

    /// Validate configuration
    pub fn is_valid(&self) -> bool {
        self.memory_budget > 0
            && self.cpu_budget_us > 0
            && self.min_confidence_threshold >= 0.0
            && self.min_confidence_threshold <= 1.0
            && self.max_actions_per_second > 0
    }
}

// =============================================================================
// Safety Levels
// =============================================================================

/// Safety level for AI operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum SafetyLevel {
    /// Minimal safety checks, maximum autonomy
    Relaxed  = 0,

    /// Standard safety with reasonable autonomy
    Standard = 1,

    /// Enhanced safety with limited autonomy
    Cautious = 2,

    /// Maximum safety, minimal autonomy
    Paranoid = 3,
}

impl Default for SafetyLevel {
    fn default() -> Self {
        Self::Standard
    }
}

// =============================================================================
// Confidence Score
// =============================================================================

/// Confidence score for AI predictions/decisions (0.0 to 1.0)
#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct Confidence(f32);

impl Confidence {
    /// Minimum confidence value
    pub const MIN: Self = Self(0.0);

    /// Maximum confidence value
    pub const MAX: Self = Self(1.0);

    /// Create a new confidence score, clamping to valid range
    pub fn new(value: f32) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    /// Get the raw confidence value
    pub fn value(self) -> f32 {
        self.0
    }

    /// Check if confidence meets a threshold
    pub fn meets_threshold(self, threshold: f32) -> bool {
        self.0 >= threshold
    }

    /// Combine confidences (multiplication for independence)
    pub fn combine(self, other: Self) -> Self {
        Self::new(self.0 * other.0)
    }

    /// Average of multiple confidences
    pub fn average(confidences: &[Self]) -> Self {
        if confidences.is_empty() {
            return Self::MIN;
        }
        let sum: f32 = confidences.iter().map(|c| c.0).sum();
        Self::new(sum / confidences.len() as f32)
    }

    /// High confidence (> 0.9)
    pub fn is_high(self) -> bool {
        self.0 > 0.9
    }

    /// Medium confidence (0.5 - 0.9)
    pub fn is_medium(self) -> bool {
        self.0 >= 0.5 && self.0 <= 0.9
    }

    /// Low confidence (< 0.5)
    pub fn is_low(self) -> bool {
        self.0 < 0.5
    }
}

impl fmt::Debug for Confidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Confidence({:.2}%)", self.0 * 100.0)
    }
}

impl fmt::Display for Confidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1}%", self.0 * 100.0)
    }
}

impl Default for Confidence {
    fn default() -> Self {
        Self(0.5)
    }
}

// =============================================================================
// AI State
// =============================================================================

/// Current state of the AI subsystem
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AiState {
    /// AI is initializing
    Initializing = 0,

    /// AI is idle, waiting for events
    Idle         = 1,

    /// AI is processing events
    Processing   = 2,

    /// AI is executing an action
    Acting       = 3,

    /// AI is learning from new data
    Learning     = 4,

    /// AI is in safe mode (limited functionality)
    SafeMode     = 5,

    /// AI is suspended
    Suspended    = 6,

    /// AI encountered an error
    Error        = 7,
}

impl Default for AiState {
    fn default() -> Self {
        Self::Initializing
    }
}

// =============================================================================
// AI Priority
// =============================================================================

/// Priority level for AI actions and decisions
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum AiPriority {
    /// Background priority - execute when system is idle
    Background = 0,

    /// Low priority - can be delayed
    Low        = 1,

    /// Normal priority - standard processing
    Normal     = 2,

    /// High priority - process soon
    High       = 3,

    /// Critical priority - immediate processing required
    Critical   = 4,

    /// Emergency - system safety at stake
    Emergency  = 5,
}

impl Default for AiPriority {
    fn default() -> Self {
        Self::Normal
    }
}

impl AiPriority {
    /// Check if this priority is higher than another
    pub fn is_higher_than(self, other: Self) -> bool {
        (self as u8) > (other as u8)
    }

    /// Get the maximum delay (in ms) for this priority
    pub fn max_delay_ms(self) -> u64 {
        match self {
            Self::Background => 10000,
            Self::Low => 1000,
            Self::Normal => 100,
            Self::High => 10,
            Self::Critical => 1,
            Self::Emergency => 0,
        }
    }
}

// =============================================================================
// AI Events
// =============================================================================

/// Events that the AI system can process
#[derive(Debug, Clone)]
pub enum AiEvent {
    // System events
    /// System boot completed
    SystemBoot,
    /// System shutting down
    SystemShutdown,
    /// System entering sleep mode
    SystemSleep,
    /// System waking from sleep
    SystemWake,

    // Performance events
    /// CPU usage threshold crossed.
    /// - `usage_percent`: CPU usage percentage.
    /// - `cpu_id`: CPU core identifier.
    CpuThreshold {
        /// CPU usage percentage.
        usage_percent: u8,
        /// CPU core ID.
        cpu_id: u32,
    },
    /// Memory pressure detected.
    /// - `available_percent`: Available memory percentage.
    MemoryPressure {
        /// Available memory percentage.
        available_percent: u8,
    },
    /// I/O bottleneck detected.
    /// - `device_id`: Device identifier.
    /// - `latency_us`: Latency in microseconds.
    IoBottleneck {
        /// Device ID.
        device_id: u32,
        /// Latency in microseconds.
        latency_us: u64,
    },
    /// Performance anomaly detected.
    /// - `component`: Component name.
    /// - `metric`: Metric name.
    /// - `deviation`: Deviation value.
    PerformanceAnomaly {
        /// Component name.
        component: String,
        /// Metric name.
        metric: String,
        /// Deviation value.
        deviation: f32,
    },

    // Process events
    /// New process spawned.
    /// - `pid`: Process ID.
    /// - `name`: Process name.
    ProcessSpawn {
        /// Process ID.
        pid: u64,
        /// Process name.
        name: String,
    },
    /// Process terminated.
    /// - `pid`: Process ID.
    /// - `exit_code`: Exit code.
    ProcessExit {
        /// Process ID.
        pid: u64,
        /// Exit code.
        exit_code: i32,
    },
    /// Process resource usage spike.
    /// - `pid`: Process ID.
    /// - `resource`: Resource type.
    ProcessResourceSpike {
        /// Process ID.
        pid: u64,
        /// Resource type.
        resource: ResourceType,
    },

    // Security events
    /// Anomalous behavior detected.
    /// - `source`: Source identifier.
    /// - `severity`: Severity level.
    AnomalyDetected {
        /// Source.
        source: String,
        /// Severity level.
        severity: u8,
    },
    /// Potential attack signature matched.
    /// - `signature_id`: Signature ID.
    /// - `confidence`: Confidence level.
    ThreatSignature {
        /// Signature ID.
        signature_id: u64,
        /// Confidence level.
        confidence: Confidence,
    },
    /// Permission violation attempt.
    /// - `pid`: Process ID.
    /// - `resource`: Resource name.
    PermissionViolation {
        /// Process ID.
        pid: u64,
        /// Resource name.
        resource: String,
    },

    // Module events
    /// Module loaded.
    /// - `module_id`: Module ID.
    /// - `name`: Module name.
    ModuleLoaded {
        /// Module ID.
        module_id: u64,
        /// Module name.
        name: String,
    },
    /// Module unloaded.
    /// - `module_id`: Module ID.
    ModuleUnloaded {
        /// Module ID.
        module_id: u64,
    },
    /// Module error.
    /// - `module_id`: Module ID.
    /// - `error`: Error message.
    ModuleError {
        /// Module ID.
        module_id: u64,
        /// Error message.
        error: String,
    },
    /// Module crash.
    /// - `module_id`: Module ID.
    /// - `name`: Module name.
    /// - `error`: Error message.
    ModuleCrash {
        /// Module ID.
        module_id: u64,
        /// Module name.
        name: String,
        /// Error message.
        error: String,
    },

    // Kernel/System fault events
    /// Kernel panic detected.
    /// - `reason`: Panic reason.
    /// - `address`: Fault address.
    KernelPanic {
        /// Panic reason.
        reason: String,
        /// Fault address.
        address: u64,
    },
    /// Critical system error.
    /// - `component`: Component name.
    /// - `error`: Error message.
    CriticalSystemError {
        /// Component name.
        component: String,
        /// Error message.
        error: String,
    },

    // Hardware events
    /// Device connected.
    /// - `device_type`: Device type.
    /// - `device_id`: Device ID.
    DeviceConnected {
        /// Device type.
        device_type: DeviceType,
        /// Device ID.
        device_id: u32,
    },
    /// Device disconnected.
    /// - `device_id`: Device ID.
    DeviceDisconnected {
        /// Device ID.
        device_id: u32,
    },
    /// Hardware error.
    /// - `device_id`: Device ID.
    /// - `error_code`: Error code.
    HardwareError {
        /// Device ID.
        device_id: u32,
        /// Error code.
        error_code: u32,
    },

    // User events (if intent engine enabled)
    /// User action detected.
    /// - `action_type`: Type of user action.
    /// - `context`: User context.
    UserAction {
        /// Action type.
        action_type: UserActionType,
        /// User context.
        context: UserContext,
    },
    /// User pattern detected.
    /// - `pattern_id`: Pattern identifier.
    UserPattern {
        /// Pattern ID.
        pattern_id: u64,
    },

    // Learning events
    /// New pattern discovered.
    /// - `pattern_id`: Pattern identifier.
    /// - `confidence`: Confidence level.
    PatternDiscovered {
        /// Pattern ID.
        pattern_id: u64,
        /// Confidence level.
        confidence: Confidence,
    },
    /// Model update available.
    /// - `component`: Component name.
    /// - `version`: Version number.
    ModelUpdate {
        /// Component name.
        component: String,
        /// Version number.
        version: u64,
    },

    // Custom event
    /// Application-specific event.
    /// - `event_id`: Event identifier.
    /// - `data`: Event data.
    Custom {
        /// Event ID.
        event_id: u64,
        /// Event data.
        data: Vec<u8>,
    },
}

/// Resource types for monitoring
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    /// CPU resource.
    Cpu,
    /// Memory resource.
    Memory,
    /// Disk resource.
    Disk,
    /// Network resource.
    Network,
    /// GPU resource.
    Gpu,
    /// NPU resource.
    Npu,
}

/// Device types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    /// Storage device.
    Storage,
    /// Network device.
    Network,
    /// Input device.
    Input,
    /// Display device.
    Display,
    /// Audio device.
    Audio,
    /// Accelerator device.
    Accelerator,
    /// Other device type.
    Other,
}

/// User action types (for intent engine)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserActionType {
    /// File operation action.
    FileOperation,
    /// Process launch action.
    ProcessLaunch,
    /// System setting change.
    SystemSetting,
    /// Network access action.
    NetworkAccess,
    /// Peripheral use action.
    PeripheralUse,
    /// Custom action with ID.
    Custom(u32),
}

/// User context for intent recognition
#[derive(Debug, Clone, Default)]
pub struct UserContext {
    /// Active application/process
    pub active_process: Option<u64>,
    /// Time of day (0-23)
    pub hour_of_day: u8,
    /// Day of week (0-6)
    pub day_of_week: u8,
    /// Session duration in minutes
    pub session_duration_min: u32,
    /// Recent action history (action type IDs)
    pub recent_actions: Vec<u32>,
    /// Current workload category
    pub workload_category: WorkloadCategory,
}

/// Workload categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkloadCategory {
    /// System is idle.
    #[default]
    Idle,
    /// Interactive workload.
    Interactive,
    /// Computation-heavy workload.
    Computation,
    /// I/O intensive workload.
    IoIntensive,
    /// Multimedia workload.
    Multimedia,
    /// Gaming workload.
    Gaming,
    /// Development workload.
    Development,
    /// Server workload.
    Server,
    /// Mixed or unknown workload.
    Mixed,
}

// =============================================================================
// AI Decisions
// =============================================================================

/// A decision made by the AI system
#[derive(Debug, Clone)]
pub struct AiDecision {
    /// Unique decision ID
    pub id: DecisionId,

    /// Timestamp when decision was made
    pub timestamp: u64,

    /// The action to take
    pub action: AiAction,

    /// Confidence in this decision
    pub confidence: Confidence,

    /// Priority of this decision
    pub priority: AiPriority,

    /// Reasoning chain (for auditing)
    pub reasoning: Vec<String>,

    /// Expected outcome
    pub expected_outcome: String,

    /// Rollback strategy if action fails
    pub rollback: Option<RollbackStrategy>,

    /// Context that led to this decision
    pub context: DecisionContext,
}

/// Unique decision identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DecisionId(u64);

impl DecisionId {
    /// Generate a new unique decision ID
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Get the raw ID value
    pub fn value(self) -> u64 {
        self.0
    }
}

impl Default for DecisionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Context for a decision
#[derive(Debug, Clone, Default)]
pub struct DecisionContext {
    /// Triggering event
    pub trigger_event: Option<String>,
    /// Current system state metrics
    pub system_metrics: SystemMetrics,
    /// Active constraints
    pub constraints: Vec<String>,
    /// Time budget for decision
    pub time_budget_us: u64,
    /// CPU usage as 0.0-1.0 fraction
    pub cpu_usage: f32,
    /// Memory usage as 0.0-1.0 fraction
    pub memory_usage: f32,
    /// Number of active processes
    pub active_processes: u32,
    /// Number of pending I/O operations
    pub io_pending: u32,
}

/// System metrics snapshot
#[derive(Debug, Clone, Default)]
pub struct SystemMetrics {
    /// CPU usage percentage.
    pub cpu_usage_percent: u8,
    /// Memory usage percentage.
    pub memory_usage_percent: u8,
    /// I/O wait percentage.
    pub io_wait_percent: u8,
    /// Number of processes.
    pub process_count: u32,
    /// Number of threads.
    pub thread_count: u32,
    /// Interrupt rate per second.
    pub interrupt_rate: u32,
    /// Context switch rate per second.
    pub context_switch_rate: u32,
}

/// Rollback strategy for reversible actions
#[derive(Debug, Clone)]
pub struct RollbackStrategy {
    /// Steps to undo the action
    pub steps: Vec<RollbackStep>,
    /// Maximum time to attempt rollback
    pub timeout_ms: u64,
    /// Whether rollback is guaranteed to succeed
    pub guaranteed: bool,
}

/// A single rollback step
#[derive(Debug, Clone)]
pub struct RollbackStep {
    /// Description of the rollback step.
    pub description: String,
    /// Action to perform for rollback.
    pub action: AiAction,
}

// =============================================================================
// AI Actions
// =============================================================================

/// Actions that the AI can perform
#[derive(Debug, Clone)]
pub enum AiAction {
    // No action
    /// Do nothing
    NoOp,

    // Optimization actions
    /// Adjust scheduler parameters (legacy)
    TuneScheduler {
        /// Time slice granularity in nanoseconds
        granularity_ns: u64,
        /// Enable preemption
        preemption: bool,
    },
    /// Adjust memory allocator
    TuneAllocator {
        /// Strategy name
        strategy: String,
    },
    /// Adjust I/O scheduler.
    /// - `parameter`: Parameter name.
    /// - `value`: Parameter value.
    TuneIoScheduler {
        /// Parameter name.
        parameter: String,
        /// Parameter value.
        value: i64,
    },
    /// Pre-allocate resources for predicted workload.
    /// - `resource`: Resource type.
    /// - `amount`: Amount to allocate.
    PreallocateResources {
        /// Resource type.
        resource: ResourceType,
        /// Amount.
        amount: u64,
    },
    /// Migrate process to different CPU.
    /// - `pid`: Process ID.
    /// - `from_cpu`: Source CPU.
    /// - `to_cpu`: Destination CPU.
    MigrateProcess {
        /// Process ID.
        pid: u64,
        /// Source CPU.
        from_cpu: u32,
        /// Destination CPU.
        to_cpu: u32,
    },
    /// Adjust process priority.
    /// - `pid`: Process ID.
    /// - `old_priority`: Previous priority.
    /// - `new_priority`: New priority.
    AdjustProcessPriority {
        /// Process ID.
        pid: u64,
        /// Old priority.
        old_priority: i32,
        /// New priority.
        new_priority: i32,
    },
    /// Force garbage collection
    ForceGarbageCollection,

    // Self-healing actions
    /// Restart a faulting module.
    /// - `module_id`: Module ID.
    /// - `module_name`: Module name.
    RestartModule {
        /// Module ID.
        module_id: u64,
        /// Module name.
        module_name: String,
    },
    /// Apply a hot patch.
    /// - `patch_id`: Patch ID.
    /// - `target`: Target component.
    ApplyPatch {
        /// Patch ID.
        patch_id: u64,
        /// Target.
        target: String,
    },
    /// Roll back to previous module version.
    /// - `module_id`: Module ID.
    /// - `target_version`: Target version.
    RollbackModule {
        /// Module ID.
        module_id: u64,
        /// Target version.
        target_version: u64,
    },
    /// Isolate misbehaving process.
    /// - `pid`: Process ID.
    /// - `isolation_level`: Isolation level.
    IsolateProcess {
        /// Process ID.
        pid: u64,
        /// Isolation level.
        isolation_level: u8,
    },
    /// Clear and reinitialize cache.
    /// - `cache_id`: Cache ID.
    ResetCache {
        /// Cache ID.
        cache_id: u32,
    },
    /// Terminate a process.
    /// - `pid`: Process ID.
    TerminateProcess {
        /// Process ID.
        pid: u64,
    },

    // Security actions
    /// Block suspicious process.
    /// - `pid`: Process ID.
    /// - `reason`: Block reason.
    BlockProcess {
        /// Process ID.
        pid: u64,
        /// Reason.
        reason: String,
    },
    /// Quarantine file.
    /// - `path`: File path.
    /// - `threat_id`: Threat ID.
    QuarantineFile {
        /// File path.
        path: String,
        /// Threat ID.
        threat_id: u64,
    },
    /// Block network connection.
    /// - `address`: Network address.
    /// - `port`: Port number.
    BlockConnection {
        /// Address.
        address: String,
        /// Port.
        port: u16,
    },
    /// Increase security level.
    /// - `from`: Previous level.
    /// - `to`: New level.
    EscalateSecurityLevel {
        /// From level.
        from: u8,
        /// To level.
        to: u8,
    },
    /// Trigger security scan.
    /// - `scope`: Scan scope.
    TriggerSecurityScan {
        /// Scan scope.
        scope: SecurityScanScope,
    },

    // Resource management
    /// Offload computation to GPU.
    /// - `task_id`: Task ID.
    /// - `kernel_name`: Kernel name.
    OffloadToGpu {
        /// Task ID.
        task_id: u64,
        /// Kernel name.
        kernel_name: String,
    },
    /// Offload to NPU.
    /// - `task_id`: Task ID.
    /// - `model_id`: Model ID.
    OffloadToNpu {
        /// Task ID.
        task_id: u64,
        /// Model ID.
        model_id: u64,
    },
    /// Adjust power profile.
    /// - `profile`: Power profile.
    SetPowerProfile {
        /// Power profile.
        profile: PowerProfile,
    },
    /// Suspend idle processes.
    /// - `threshold_seconds`: Idle threshold.
    SuspendIdleProcesses {
        /// Threshold in seconds.
        threshold_seconds: u32,
    },

    // Module management
    /// Load a module
    LoadModule {
        /// Module name.
        module_name: String,
        /// Configuration data.
        config: Vec<u8>,
    },
    /// Unload a module.
    /// - `module_id`: Module ID.
    UnloadModule {
        /// Module ID.
        module_id: u64,
    },
    /// Hot-reload module.
    /// - `module_id`: Module ID.
    /// - `new_version`: New version.
    HotReloadModule {
        /// Module ID.
        module_id: u64,
        /// New version.
        new_version: u64,
    },

    // Learning actions
    /// Update prediction model.
    /// - `model_id`: Model ID.
    /// - `delta`: Model update delta.
    UpdateModel {
        /// Model ID.
        model_id: u64,
        /// Delta.
        delta: Vec<u8>,
    },
    /// Record pattern for future use.
    /// - `pattern`: Pattern data.
    /// - `category`: Category name.
    RecordPattern {
        /// Pattern data.
        pattern: Vec<u8>,
        /// Category.
        category: String,
    },
    /// Invalidate outdated pattern.
    /// - `pattern_id`: Pattern ID.
    InvalidatePattern {
        /// Pattern ID.
        pattern_id: u64,
    },

    // Composite actions
    /// Execute multiple actions in sequence
    Sequence(Vec<AiAction>),
    /// Execute actions in parallel where safe
    Parallel(Vec<AiAction>),
    /// Conditional action.
    /// - `condition`: Condition expression.
    /// - `if_true`: Action if condition is true.
    /// - `if_false`: Action if condition is false.
    Conditional {
        /// Condition.
        condition: String,
        /// Action if true.
        if_true: Box<AiAction>,
        /// Action if false.
        if_false: Box<AiAction>,
    },
}

/// Security scan scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityScanScope {
    /// Quick scan.
    QuickScan,
    /// Full system scan.
    FullSystem,
    /// Memory scan.
    Memory,
    /// Process scan.
    Processes,
    /// Network scan.
    Network,
    /// File system scan.
    FileSystem,
}

/// Power profiles
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerProfile {
    /// High performance mode.
    Performance,
    /// Balanced mode.
    Balanced,
    /// Power saver mode.
    PowerSaver,
    /// Ultra power saver mode.
    UltraPowerSaver,
    /// Custom power profile.
    Custom(u8),
}

// =============================================================================
// AI Errors
// =============================================================================

/// Errors that can occur in the AI subsystem
#[derive(Debug, Clone)]
pub enum AiError {
    /// AI not initialized
    NotInitialized,

    /// Configuration error
    ConfigurationError(String),

    /// Resource exhaustion
    ResourceExhausted(String),

    /// Timeout exceeded.
    /// - `operation`: Operation that timed out.
    /// - `elapsed_ms`: Elapsed time in milliseconds.
    Timeout {
        /// Operation name.
        operation: String,
        /// Elapsed time.
        elapsed_ms: u64,
    },

    /// Safety constraint violated
    SafetyViolation(String),

    /// Action not permitted at current safety level.
    /// - `action`: Action that was denied.
    /// - `reason`: Reason for denial.
    ActionDenied {
        /// Action name.
        action: String,
        /// Denial reason.
        reason: String,
    },

    /// Rate limit exceeded.
    /// - `limit`: Rate limit.
    /// - `window_ms`: Time window in milliseconds.
    RateLimitExceeded {
        /// Rate limit.
        limit: u32,
        /// Time window.
        window_ms: u64,
    },

    /// Confidence too low.
    /// - `required`: Required confidence.
    /// - `actual`: Actual confidence.
    LowConfidence {
        /// Required confidence.
        required: f32,
        /// Actual confidence.
        actual: f32,
    },

    /// Rollback failed
    RollbackFailed(String),

    /// Component error.
    /// - `component`: Component name.
    /// - `error`: Error message.
    ComponentError {
        /// Component name.
        component: String,
        /// Error message.
        error: String,
    },

    /// Learning error
    LearningError(String),

    /// Hardware acceleration unavailable
    AcceleratorUnavailable(String),

    /// Internal error
    Internal(String),
}

impl fmt::Display for AiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotInitialized => write!(f, "AI subsystem not initialized"),
            Self::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            Self::ResourceExhausted(resource) => {
                write!(f, "Resource exhausted: {}", resource)
            },
            Self::Timeout {
                operation,
                elapsed_ms,
            } => {
                write!(f, "Timeout in {}: {}ms elapsed", operation, elapsed_ms)
            },
            Self::SafetyViolation(msg) => write!(f, "Safety violation: {}", msg),
            Self::ActionDenied { action, reason } => {
                write!(f, "Action '{}' denied: {}", action, reason)
            },
            Self::RateLimitExceeded { limit, window_ms } => {
                write!(f, "Rate limit {} per {}ms exceeded", limit, window_ms)
            },
            Self::LowConfidence { required, actual } => {
                write!(
                    f,
                    "Confidence too low: {:.1}% < {:.1}% required",
                    actual * 100.0,
                    required * 100.0
                )
            },
            Self::RollbackFailed(msg) => write!(f, "Rollback failed: {}", msg),
            Self::ComponentError { component, error } => {
                write!(f, "Component '{}' error: {}", component, error)
            },
            Self::LearningError(msg) => write!(f, "Learning error: {}", msg),
            Self::AcceleratorUnavailable(accel) => {
                write!(f, "Accelerator unavailable: {}", accel)
            },
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

/// Result type for AI operations
pub type AiResult<T> = Result<T, AiError>;

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_clamping() {
        assert_eq!(Confidence::new(-0.5).value(), 0.0);
        assert_eq!(Confidence::new(1.5).value(), 1.0);
        assert_eq!(Confidence::new(0.5).value(), 0.5);
    }

    #[test]
    fn test_confidence_combine() {
        let c1 = Confidence::new(0.8);
        let c2 = Confidence::new(0.9);
        let combined = c1.combine(c2);
        assert!((combined.value() - 0.72).abs() < 0.001);
    }

    #[test]
    fn test_priority_ordering() {
        assert!(AiPriority::Critical.is_higher_than(AiPriority::Normal));
        assert!(!AiPriority::Low.is_higher_than(AiPriority::High));
    }

    #[test]
    fn test_decision_id_uniqueness() {
        let id1 = DecisionId::new();
        let id2 = DecisionId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_config_validation() {
        let valid = AiConfig::default();
        assert!(valid.is_valid());

        let mut invalid = AiConfig::default();
        invalid.min_confidence_threshold = 1.5;
        assert!(!invalid.is_valid());
    }
}
