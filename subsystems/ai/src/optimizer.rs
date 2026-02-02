//! # Self-Optimization Engine
//!
//! The Optimizer continuously analyzes system performance and automatically
//! adjusts kernel parameters for optimal operation.
//!
//! ## Capabilities
//!
//! - **Scheduler Tuning**: Adjust scheduling parameters based on workload
//! - **Memory Optimization**: Tune allocator and cache parameters
//! - **I/O Scheduling**: Optimize disk and network I/O patterns
//! - **Power Management**: Balance performance and power consumption
//! - **Workload Prediction**: Anticipate resource needs
//!
//! ## Architecture
//!
//! ```text
//!   System Metrics ───►┌─────────────────────────────────────┐
//!                      │          Optimizer                   │
//!   Events ───────────►│                                     │
//!                      │  ┌─────────────────────────────┐    │
//!   Historical Data ──►│  │    Metrics Analyzer         │    │
//!                      │  │  - CPU Analysis             │    │
//!                      │  │  - Memory Analysis          │    │
//!                      │  │  - I/O Analysis             │    │
//!                      │  └──────────────┬──────────────┘    │
//!                      │                 │                    │
//!                      │                 ▼                    │
//!                      │  ┌─────────────────────────────┐    │
//!                      │  │    Workload Classifier      │    │
//!                      │  │  - Interactive              │    │
//!                      │  │  - Batch                    │    │
//!                      │  │  - Real-time                │    │
//!                      │  └──────────────┬──────────────┘    │
//!                      │                 │                    │
//!                      │                 ▼                    │
//!                      │  ┌─────────────────────────────┐    │
//!                      │  │    Parameter Optimizer      │    │
//!                      │  │  - Scheduler Params         │    │
//!                      │  │  - Allocator Params         │    │
//!                      │  │  - I/O Scheduler            │    │
//!                      │  └──────────────┬──────────────┘    │
//!                      │                 │                    │
//!                      │                 ▼                    │
//!                      │  ┌─────────────────────────────┐    │
//!                      │  │    Action Generator         │───────► Tuning Actions
//!                      │  └─────────────────────────────┘    │
//!                      │                                     │
//!                      └─────────────────────────────────────┘
//! ```

use alloc::collections::VecDeque;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::{format, vec};
use core::sync::atomic::{AtomicU64, Ordering};

use spin::{Mutex, RwLock};

use crate::core::{
    AiAction, AiDecision, AiError, AiEvent, AiPriority, AiResult, Confidence, DecisionContext,
    DecisionId, PowerProfile, ResourceType, SystemMetrics, WorkloadCategory,
};

// =============================================================================
// Performance Profile
// =============================================================================

/// A performance profile with tuning parameters
#[derive(Debug, Clone)]
pub struct PerformanceProfile {
    /// Profile name
    pub name: String,

    /// Scheduler parameters
    pub scheduler: SchedulerParams,

    /// Memory allocator parameters
    pub allocator: AllocatorParams,

    /// I/O scheduler parameters
    pub io_scheduler: IoSchedulerParams,

    /// Power profile
    pub power: PowerProfile,

    /// Target workload
    pub target_workload: WorkloadCategory,
}

/// Scheduler tuning parameters
#[derive(Debug, Clone)]
pub struct SchedulerParams {
    /// Scheduling granularity (ns)
    pub sched_granularity_ns: u64,
    /// Wakeup granularity (ns)
    pub wakeup_granularity_ns: u64,
    /// Minimum preemption granularity (ns)
    pub min_granularity_ns: u64,
    /// Migration cost (ns)
    pub migration_cost_ns: u64,
    /// Number of CPU-bound processes per core threshold
    pub nr_latency: u32,
    /// Enable latency-nice for interactive tasks
    pub latency_nice_enabled: bool,
    /// Enable NUMA balancing
    pub numa_balancing: bool,
}

impl Default for SchedulerParams {
    fn default() -> Self {
        Self {
            sched_granularity_ns: 3_000_000,  // 3ms
            wakeup_granularity_ns: 4_000_000, // 4ms
            min_granularity_ns: 500_000,      // 0.5ms
            migration_cost_ns: 500_000,       // 0.5ms
            nr_latency: 8,
            latency_nice_enabled: true,
            numa_balancing: true,
        }
    }
}

/// Memory allocator parameters
#[derive(Debug, Clone)]
pub struct AllocatorParams {
    /// Enable transparent huge pages
    pub thp_enabled: bool,
    /// THP defrag mode
    pub thp_defrag: ThpDefrag,
    /// Swappiness (0-100)
    pub swappiness: u8,
    /// VFS cache pressure
    pub vfs_cache_pressure: u32,
    /// Dirty ratio (%)
    pub dirty_ratio: u8,
    /// Dirty background ratio (%)
    pub dirty_background_ratio: u8,
    /// Watermark scale factor
    pub watermark_scale_factor: u32,
    /// Enable zone reclaim
    pub zone_reclaim: bool,
}

impl Default for AllocatorParams {
    fn default() -> Self {
        Self {
            thp_enabled: true,
            thp_defrag: ThpDefrag::Defer,
            swappiness: 60,
            vfs_cache_pressure: 100,
            dirty_ratio: 20,
            dirty_background_ratio: 10,
            watermark_scale_factor: 10,
            zone_reclaim: false,
        }
    }
}

/// THP defrag modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThpDefrag {
    Always,
    Defer,
    DeferMadvise,
    Madvise,
    Never,
}

/// I/O scheduler parameters
#[derive(Debug, Clone)]
pub struct IoSchedulerParams {
    /// I/O scheduler type
    pub scheduler: IoScheduler,
    /// Read-ahead size (KB)
    pub read_ahead_kb: u32,
    /// Number of requests
    pub nr_requests: u32,
    /// Enable I/O merging
    pub io_merge: bool,
    /// I/O priority class boost for reads
    pub read_priority_boost: i8,
    /// Queue depth
    pub queue_depth: u32,
}

impl Default for IoSchedulerParams {
    fn default() -> Self {
        Self {
            scheduler: IoScheduler::Mq,
            read_ahead_kb: 128,
            nr_requests: 256,
            io_merge: true,
            read_priority_boost: 0,
            queue_depth: 64,
        }
    }
}

/// I/O scheduler types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoScheduler {
    /// No-op (passthrough)
    None,
    /// Multi-queue deadline
    Mq,
    /// Budget Fair Queueing
    Bfq,
    /// Kyber
    Kyber,
}

// =============================================================================
// Workload Analysis
// =============================================================================

/// Analysis of current workload characteristics
#[derive(Debug, Clone)]
pub struct WorkloadAnalysis {
    /// Primary workload type
    pub workload_type: WorkloadCategory,
    /// Confidence in classification
    pub confidence: Confidence,
    /// CPU characteristics
    pub cpu: CpuCharacteristics,
    /// Memory characteristics
    pub memory: MemoryCharacteristics,
    /// I/O characteristics
    pub io: IoCharacteristics,
    /// Predicted future workload
    pub predicted_workload: Option<WorkloadCategory>,
    /// Time until workload change (seconds)
    pub predicted_change_s: Option<u32>,
}

/// CPU workload characteristics
#[derive(Debug, Clone)]
pub struct CpuCharacteristics {
    /// Average CPU utilization (0-100)
    pub utilization: u8,
    /// User time percentage
    pub user_percent: u8,
    /// System time percentage
    pub system_percent: u8,
    /// I/O wait percentage
    pub iowait_percent: u8,
    /// Context switches per second
    pub context_switches_per_sec: u32,
    /// Is workload CPU-bound?
    pub cpu_bound: bool,
    /// Number of runnable tasks
    pub runnable_tasks: u32,
}

/// Memory workload characteristics
#[derive(Debug, Clone)]
pub struct MemoryCharacteristics {
    /// Memory utilization (0-100)
    pub utilization: u8,
    /// Page faults per second
    pub page_faults_per_sec: u32,
    /// Major faults (requiring I/O)
    pub major_faults_per_sec: u32,
    /// Cache hit ratio (0-100)
    pub cache_hit_ratio: u8,
    /// Memory pressure level
    pub pressure: MemoryPressure,
    /// Active anonymous memory (MB)
    pub active_anon_mb: u32,
    /// Active file cache (MB)
    pub active_file_mb: u32,
}

/// Memory pressure levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPressure {
    None,
    Low,
    Medium,
    High,
    Critical,
}

/// I/O workload characteristics
#[derive(Debug, Clone)]
pub struct IoCharacteristics {
    /// Read IOPS
    pub read_iops: u32,
    /// Write IOPS
    pub write_iops: u32,
    /// Read throughput (MB/s)
    pub read_mbps: u32,
    /// Write throughput (MB/s)
    pub write_mbps: u32,
    /// Average latency (us)
    pub avg_latency_us: u32,
    /// Is workload I/O-bound?
    pub io_bound: bool,
    /// Dominant access pattern
    pub pattern: IoPattern,
}

/// I/O access patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoPattern {
    Sequential,
    Random,
    Mixed,
}

// =============================================================================
// Optimization Hints
// =============================================================================

/// Hint about potential optimization
#[derive(Debug, Clone)]
pub struct OptimizationHint {
    /// What parameter to adjust
    pub parameter: String,
    /// Current value
    pub current_value: i64,
    /// Suggested value
    pub suggested_value: i64,
    /// Expected improvement
    pub expected_improvement: String,
    /// Confidence in suggestion
    pub confidence: Confidence,
    /// Priority of this optimization
    pub priority: AiPriority,
}

// =============================================================================
// Optimizer Engine
// =============================================================================

/// The Self-Optimization Engine
pub struct Optimizer {
    /// Whether optimization is enabled
    enabled: bool,

    /// Current performance profile
    current_profile: RwLock<PerformanceProfile>,

    /// Available profiles
    profiles: RwLock<Vec<PerformanceProfile>>,

    /// Metrics history
    metrics_history: Mutex<VecDeque<MetricsSnapshot>>,

    /// Applied optimizations history
    optimization_history: Mutex<VecDeque<AppliedOptimization>>,

    /// Statistics
    stats: OptimizerStats,
}

/// Snapshot of system metrics at a point in time
#[derive(Debug, Clone)]
struct MetricsSnapshot {
    timestamp: u64,
    metrics: SystemMetrics,
    workload: WorkloadCategory,
}

/// Record of an applied optimization
#[derive(Debug, Clone)]
struct AppliedOptimization {
    timestamp: u64,
    parameter: String,
    old_value: i64,
    new_value: i64,
    reason: String,
    outcome: OptimizationOutcome,
}

/// Outcome of an optimization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OptimizationOutcome {
    Pending,
    Improved,
    NoChange,
    Degraded,
    RolledBack,
}

struct OptimizerStats {
    optimizations_made: AtomicU64,
    optimizations_successful: AtomicU64,
    optimizations_rolled_back: AtomicU64,
    workload_transitions: AtomicU64,
    profile_switches: AtomicU64,
}

impl Default for OptimizerStats {
    fn default() -> Self {
        Self {
            optimizations_made: AtomicU64::new(0),
            optimizations_successful: AtomicU64::new(0),
            optimizations_rolled_back: AtomicU64::new(0),
            workload_transitions: AtomicU64::new(0),
            profile_switches: AtomicU64::new(0),
        }
    }
}

impl Optimizer {
    /// Maximum metrics history size
    const MAX_HISTORY: usize = 1000;

    /// Create a new Optimizer
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            current_profile: RwLock::new(Self::default_profile()),
            profiles: RwLock::new(Self::builtin_profiles()),
            metrics_history: Mutex::new(VecDeque::with_capacity(Self::MAX_HISTORY)),
            optimization_history: Mutex::new(VecDeque::with_capacity(Self::MAX_HISTORY)),
            stats: OptimizerStats::default(),
        }
    }

    /// Default balanced profile
    fn default_profile() -> PerformanceProfile {
        PerformanceProfile {
            name: "balanced".to_string(),
            scheduler: SchedulerParams::default(),
            allocator: AllocatorParams::default(),
            io_scheduler: IoSchedulerParams::default(),
            power: PowerProfile::Balanced,
            target_workload: WorkloadCategory::Interactive,
        }
    }

    /// Built-in performance profiles
    fn builtin_profiles() -> Vec<PerformanceProfile> {
        vec![
            // Balanced profile
            Self::default_profile(),
            // Interactive profile (low latency)
            PerformanceProfile {
                name: "interactive".to_string(),
                scheduler: SchedulerParams {
                    sched_granularity_ns: 1_000_000, // 1ms
                    wakeup_granularity_ns: 500_000,  // 0.5ms
                    min_granularity_ns: 250_000,     // 0.25ms
                    migration_cost_ns: 250_000,
                    nr_latency: 4,
                    latency_nice_enabled: true,
                    numa_balancing: false, // Disable for lower latency
                },
                allocator: AllocatorParams {
                    thp_enabled: true,
                    thp_defrag: ThpDefrag::DeferMadvise, // Don't block
                    swappiness: 10,                      // Avoid swapping
                    vfs_cache_pressure: 50,
                    dirty_ratio: 10,
                    dirty_background_ratio: 5,
                    watermark_scale_factor: 10,
                    zone_reclaim: false,
                },
                io_scheduler: IoSchedulerParams {
                    scheduler: IoScheduler::Kyber, // Low latency
                    read_ahead_kb: 64,
                    nr_requests: 128,
                    io_merge: true,
                    read_priority_boost: 4,
                    queue_depth: 32,
                },
                power: PowerProfile::Balanced,
                target_workload: WorkloadCategory::Interactive,
            },
            // Throughput profile (batch/computation)
            PerformanceProfile {
                name: "throughput".to_string(),
                scheduler: SchedulerParams {
                    sched_granularity_ns: 10_000_000, // 10ms
                    wakeup_granularity_ns: 8_000_000,
                    min_granularity_ns: 2_000_000,
                    migration_cost_ns: 1_000_000,
                    nr_latency: 16,
                    latency_nice_enabled: false,
                    numa_balancing: true,
                },
                allocator: AllocatorParams {
                    thp_enabled: true,
                    thp_defrag: ThpDefrag::Always, // Maximize huge pages
                    swappiness: 60,
                    vfs_cache_pressure: 100,
                    dirty_ratio: 40,
                    dirty_background_ratio: 20,
                    watermark_scale_factor: 10,
                    zone_reclaim: true,
                },
                io_scheduler: IoSchedulerParams {
                    scheduler: IoScheduler::Mq,
                    read_ahead_kb: 512,
                    nr_requests: 512,
                    io_merge: true,
                    read_priority_boost: 0,
                    queue_depth: 128,
                },
                power: PowerProfile::Performance,
                target_workload: WorkloadCategory::Computation,
            },
            // Power saving profile
            PerformanceProfile {
                name: "powersave".to_string(),
                scheduler: SchedulerParams {
                    sched_granularity_ns: 8_000_000,
                    wakeup_granularity_ns: 6_000_000,
                    min_granularity_ns: 1_000_000,
                    migration_cost_ns: 2_000_000, // Prefer not to migrate
                    nr_latency: 8,
                    latency_nice_enabled: false,
                    numa_balancing: false,
                },
                allocator: AllocatorParams {
                    thp_enabled: false, // Save memory
                    thp_defrag: ThpDefrag::Never,
                    swappiness: 80, // Swap more freely
                    vfs_cache_pressure: 150,
                    dirty_ratio: 5,
                    dirty_background_ratio: 2,
                    watermark_scale_factor: 10,
                    zone_reclaim: true,
                },
                io_scheduler: IoSchedulerParams {
                    scheduler: IoScheduler::Bfq, // Fair, not aggressive
                    read_ahead_kb: 32,
                    nr_requests: 64,
                    io_merge: true,
                    read_priority_boost: 0,
                    queue_depth: 16,
                },
                power: PowerProfile::PowerSaver,
                target_workload: WorkloadCategory::Idle,
            },
            // Real-time profile
            PerformanceProfile {
                name: "realtime".to_string(),
                scheduler: SchedulerParams {
                    sched_granularity_ns: 500_000,  // 0.5ms
                    wakeup_granularity_ns: 250_000, // 0.25ms
                    min_granularity_ns: 100_000,    // 0.1ms
                    migration_cost_ns: 100_000,
                    nr_latency: 2,
                    latency_nice_enabled: true,
                    numa_balancing: false,
                },
                allocator: AllocatorParams {
                    thp_enabled: false, // Avoid THP latency
                    thp_defrag: ThpDefrag::Never,
                    swappiness: 0, // Never swap
                    vfs_cache_pressure: 50,
                    dirty_ratio: 5,
                    dirty_background_ratio: 2,
                    watermark_scale_factor: 10,
                    zone_reclaim: false,
                },
                io_scheduler: IoSchedulerParams {
                    scheduler: IoScheduler::None, // Bypass I/O scheduler
                    read_ahead_kb: 0,
                    nr_requests: 64,
                    io_merge: false,
                    read_priority_boost: 7,
                    queue_depth: 8,
                },
                power: PowerProfile::Performance,
                target_workload: WorkloadCategory::Gaming,
            },
        ]
    }

    /// Check if optimizer is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Analyze an event and recommend optimizations
    pub fn analyze(
        &self,
        event: &AiEvent,
        context: &DecisionContext,
    ) -> Result<Option<(AiAction, Confidence, String)>, ()> {
        if !self.enabled {
            return Ok(None);
        }

        // Record metrics
        self.record_metrics(&context.system_metrics);

        match event {
            AiEvent::CpuThreshold {
                usage_percent,
                cpu_id,
            } => self.handle_cpu_threshold(*usage_percent, *cpu_id, context),
            AiEvent::MemoryPressure { available_percent } => {
                self.handle_memory_pressure(*available_percent, context)
            },
            AiEvent::IoBottleneck {
                device_id,
                latency_us,
            } => self.handle_io_bottleneck(*device_id, *latency_us, context),
            AiEvent::ProcessResourceSpike { pid, resource } => {
                self.handle_resource_spike(*pid, resource, context)
            },
            _ => Ok(None),
        }
    }

    /// Handle CPU threshold event
    fn handle_cpu_threshold(
        &self,
        usage: u8,
        cpu_id: u32,
        context: &DecisionContext,
    ) -> Result<Option<(AiAction, Confidence, String)>, ()> {
        if usage < 80 {
            return Ok(None);
        }

        // Analyze workload to determine best action
        let workload = self.classify_workload(context);

        let action = if usage > 95 {
            // Critical: reduce scheduler granularity
            AiAction::TuneScheduler {
                granularity_ns: 1_000_000,
                preemption: true,
            }
        } else if workload == WorkloadCategory::Interactive {
            // High but not critical on interactive: boost latency
            AiAction::TuneScheduler {
                granularity_ns: 2_000_000,
                preemption: true,
            }
        } else {
            // High on batch: increase granularity for throughput
            AiAction::TuneScheduler {
                granularity_ns: 8_000_000,
                preemption: false, // Disable preemption for throughput
            }
        };

        Ok(Some((
            action,
            Confidence::new(0.75),
            format!("CPU {} at {}%, optimizing scheduler", cpu_id, usage),
        )))
    }

    /// Handle memory pressure event
    fn handle_memory_pressure(
        &self,
        available: u8,
        context: &DecisionContext,
    ) -> Result<Option<(AiAction, Confidence, String)>, ()> {
        if available > 30 {
            return Ok(None);
        }

        let action = if available < 10 {
            // Critical: aggressive reclaim
            AiAction::Sequence(vec![
                AiAction::TuneAllocator {
                    strategy: String::from("aggressive_reclaim"),
                },
                AiAction::TuneAllocator {
                    strategy: String::from("high_cache_pressure"),
                },
                AiAction::SuspendIdleProcesses {
                    threshold_seconds: 60,
                },
            ])
        } else if available < 20 {
            // Low: increase swappiness
            AiAction::TuneAllocator {
                strategy: String::from("high_swappiness"),
            }
        } else {
            // Moderate: drop caches
            AiAction::TuneAllocator {
                strategy: String::from("moderate_cache_pressure"),
            }
        };

        Ok(Some((
            action,
            Confidence::new(0.8),
            format!("Memory pressure detected ({}% available)", available),
        )))
    }

    /// Handle I/O bottleneck event
    fn handle_io_bottleneck(
        &self,
        device_id: u32,
        latency_us: u64,
        context: &DecisionContext,
    ) -> Result<Option<(AiAction, Confidence, String)>, ()> {
        if latency_us < 50_000 {
            // Less than 50ms is acceptable
            return Ok(None);
        }

        let action = if latency_us > 200_000 {
            // Severe bottleneck: switch to low-latency scheduler
            AiAction::TuneIoScheduler {
                parameter: "scheduler".to_string(),
                value: 2, // Kyber
            }
        } else {
            // Moderate: increase read-ahead
            AiAction::TuneIoScheduler {
                parameter: "read_ahead_kb".to_string(),
                value: 256,
            }
        };

        Ok(Some((
            action,
            Confidence::new(0.7),
            format!(
                "I/O bottleneck on device {}: {}us latency",
                device_id, latency_us
            ),
        )))
    }

    /// Handle resource spike event
    fn handle_resource_spike(
        &self,
        pid: u64,
        resource: &ResourceType,
        context: &DecisionContext,
    ) -> Result<Option<(AiAction, Confidence, String)>, ()> {
        let action = match resource {
            ResourceType::Cpu => {
                AiAction::AdjustProcessPriority {
                    pid,
                    old_priority: 0,
                    new_priority: 10, // Lower priority
                }
            },
            ResourceType::Memory => {
                // Isolate memory-hungry process
                AiAction::IsolateProcess {
                    pid,
                    isolation_level: 1,
                }
            },
            _ => return Ok(None),
        };

        Ok(Some((
            action,
            Confidence::new(0.65),
            format!("Process {} resource spike: {:?}", pid, resource),
        )))
    }

    /// Proactive optimization check
    pub fn proactive_check(&self, context: &DecisionContext) -> AiResult<Option<AiDecision>> {
        if !self.enabled {
            return Ok(None);
        }

        // Analyze current workload
        let workload = self.classify_workload(context);
        let current_profile = self.current_profile.read();

        // Check if profile matches workload
        if current_profile.target_workload != workload {
            // Recommend profile switch
            if let Some(new_profile) = self.find_profile_for_workload(workload) {
                let actions = self.generate_profile_switch_actions(&current_profile, &new_profile);

                return Ok(Some(AiDecision {
                    id: DecisionId::new(),
                    timestamp: 0,
                    action: AiAction::Sequence(actions),
                    confidence: Confidence::new(0.7),
                    priority: AiPriority::Normal,
                    reasoning: vec![
                        format!(
                            "Workload changed from {:?} to {:?}",
                            current_profile.target_workload, workload
                        ),
                        format!(
                            "Switching profile from {} to {}",
                            current_profile.name, new_profile.name
                        ),
                    ],
                    expected_outcome: "Better performance for current workload".to_string(),
                    rollback: None,
                    context: context.clone(),
                }));
            }
        }

        // Check for micro-optimizations
        if let Some(hint) = self.analyze_for_hints(context) {
            return Ok(Some(AiDecision {
                id: DecisionId::new(),
                timestamp: 0,
                action: AiAction::TuneScheduler {
                    granularity_ns: hint.suggested_value as u64,
                    preemption: true,
                },
                confidence: hint.confidence,
                priority: hint.priority,
                reasoning: vec![hint.expected_improvement],
                expected_outcome: "Incremental performance improvement".to_string(),
                rollback: None,
                context: context.clone(),
            }));
        }

        Ok(None)
    }

    /// Classify current workload
    fn classify_workload(&self, context: &DecisionContext) -> WorkloadCategory {
        let metrics = &context.system_metrics;

        // Simple heuristic classification
        if metrics.cpu_usage_percent < 10 && metrics.io_wait_percent < 5 {
            WorkloadCategory::Idle
        } else if metrics.io_wait_percent > 30 {
            WorkloadCategory::IoIntensive
        } else if metrics.cpu_usage_percent > 80 && metrics.context_switch_rate < 1000 {
            WorkloadCategory::Computation
        } else if metrics.context_switch_rate > 10000 {
            WorkloadCategory::Interactive
        } else {
            WorkloadCategory::Interactive // Default
        }
    }

    /// Find a profile for the given workload
    fn find_profile_for_workload(&self, workload: WorkloadCategory) -> Option<PerformanceProfile> {
        let profiles = self.profiles.read();
        profiles
            .iter()
            .find(|p| p.target_workload == workload)
            .cloned()
    }

    /// Generate actions to switch profiles
    fn generate_profile_switch_actions(
        &self,
        from: &PerformanceProfile,
        to: &PerformanceProfile,
    ) -> Vec<AiAction> {
        let mut actions = Vec::new();

        // Scheduler changes
        if from.scheduler.sched_granularity_ns != to.scheduler.sched_granularity_ns {
            actions.push(AiAction::TuneScheduler {
                granularity_ns: to.scheduler.sched_granularity_ns,
                preemption: true,
            });
        }

        if from.scheduler.wakeup_granularity_ns != to.scheduler.wakeup_granularity_ns {
            actions.push(AiAction::TuneScheduler {
                granularity_ns: to.scheduler.wakeup_granularity_ns,
                preemption: true,
            });
        }

        // Memory changes
        if from.allocator.swappiness != to.allocator.swappiness {
            actions.push(AiAction::TuneAllocator {
                strategy: format!("swappiness_{}", to.allocator.swappiness),
            });
        }

        // Power profile
        if from.power != to.power {
            actions.push(AiAction::SetPowerProfile { profile: to.power });
        }

        actions
    }

    /// Analyze metrics for optimization hints
    fn analyze_for_hints(&self, context: &DecisionContext) -> Option<OptimizationHint> {
        let metrics = &context.system_metrics;
        let history = self.metrics_history.lock();

        if history.len() < 10 {
            return None; // Need more history
        }

        // Check for consistent high context switches
        let avg_ctx_switches: u32 = history
            .iter()
            .rev()
            .take(10)
            .map(|s| s.metrics.context_switch_rate)
            .sum::<u32>()
            / 10;

        if avg_ctx_switches > 20000 && metrics.cpu_usage_percent > 50 {
            return Some(OptimizationHint {
                parameter: "min_granularity_ns".to_string(),
                current_value: 500_000,
                suggested_value: 1_000_000,
                expected_improvement: "Reduce context switch overhead".to_string(),
                confidence: Confidence::new(0.6),
                priority: AiPriority::Low,
            });
        }

        None
    }

    /// Record metrics snapshot
    fn record_metrics(&self, metrics: &SystemMetrics) {
        let mut history = self.metrics_history.lock();

        while history.len() >= Self::MAX_HISTORY {
            history.pop_front();
        }

        history.push_back(MetricsSnapshot {
            timestamp: 0, // Would be real timestamp
            metrics: metrics.clone(),
            workload: WorkloadCategory::Interactive, // Would be classified
        });
    }

    /// Get current profile
    pub fn current_profile(&self) -> PerformanceProfile {
        self.current_profile.read().clone()
    }

    /// Set current profile by name
    pub fn set_profile(&self, name: &str) -> AiResult<()> {
        let profiles = self.profiles.read();
        if let Some(profile) = profiles.iter().find(|p| p.name == name) {
            *self.current_profile.write() = profile.clone();
            self.stats.profile_switches.fetch_add(1, Ordering::Relaxed);
            Ok(())
        } else {
            Err(AiError::ConfigurationError(format!(
                "Profile '{}' not found",
                name
            )))
        }
    }

    /// Add a custom profile
    pub fn add_profile(&self, profile: PerformanceProfile) {
        self.profiles.write().push(profile);
    }

    /// Get available profile names
    pub fn available_profiles(&self) -> Vec<String> {
        self.profiles
            .read()
            .iter()
            .map(|p| p.name.clone())
            .collect()
    }

    /// Get statistics
    pub fn statistics(&self) -> OptimizerStatistics {
        OptimizerStatistics {
            enabled: self.enabled,
            current_profile: self.current_profile.read().name.clone(),
            available_profiles: self.profiles.read().len(),
            optimizations_made: self.stats.optimizations_made.load(Ordering::Relaxed),
            optimizations_successful: self.stats.optimizations_successful.load(Ordering::Relaxed),
            optimizations_rolled_back: self.stats.optimizations_rolled_back.load(Ordering::Relaxed),
            workload_transitions: self.stats.workload_transitions.load(Ordering::Relaxed),
            profile_switches: self.stats.profile_switches.load(Ordering::Relaxed),
            metrics_history_size: self.metrics_history.lock().len(),
        }
    }
}

/// Public statistics structure
#[derive(Debug, Clone)]
pub struct OptimizerStatistics {
    pub enabled: bool,
    pub current_profile: String,
    pub available_profiles: usize,
    pub optimizations_made: u64,
    pub optimizations_successful: u64,
    pub optimizations_rolled_back: u64,
    pub workload_transitions: u64,
    pub profile_switches: u64,
    pub metrics_history_size: usize,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimizer_creation() {
        let optimizer = Optimizer::new(true);
        assert!(optimizer.is_enabled());

        let disabled = Optimizer::new(false);
        assert!(!disabled.is_enabled());
    }

    #[test]
    fn test_default_profiles() {
        let optimizer = Optimizer::new(true);
        let profiles = optimizer.available_profiles();

        assert!(profiles.contains(&"balanced".to_string()));
        assert!(profiles.contains(&"interactive".to_string()));
        assert!(profiles.contains(&"throughput".to_string()));
        assert!(profiles.contains(&"powersave".to_string()));
        assert!(profiles.contains(&"realtime".to_string()));
    }

    #[test]
    fn test_profile_switching() {
        let optimizer = Optimizer::new(true);

        let current = optimizer.current_profile();
        assert_eq!(current.name, "balanced");

        optimizer.set_profile("interactive").unwrap();
        let current = optimizer.current_profile();
        assert_eq!(current.name, "interactive");

        let result = optimizer.set_profile("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_workload_classification() {
        let optimizer = Optimizer::new(true);

        // Idle workload
        let context = DecisionContext {
            system_metrics: SystemMetrics {
                cpu_usage_percent: 5,
                io_wait_percent: 2,
                context_switch_rate: 100,
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(
            optimizer.classify_workload(&context),
            WorkloadCategory::Idle
        );

        // I/O intensive
        let context = DecisionContext {
            system_metrics: SystemMetrics {
                cpu_usage_percent: 30,
                io_wait_percent: 40,
                context_switch_rate: 1000,
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(
            optimizer.classify_workload(&context),
            WorkloadCategory::IoIntensive
        );

        // Computation
        let context = DecisionContext {
            system_metrics: SystemMetrics {
                cpu_usage_percent: 95,
                io_wait_percent: 1,
                context_switch_rate: 500,
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(
            optimizer.classify_workload(&context),
            WorkloadCategory::Computation
        );
    }

    #[test]
    fn test_custom_profile() {
        let optimizer = Optimizer::new(true);

        let custom = PerformanceProfile {
            name: "custom".to_string(),
            scheduler: SchedulerParams::default(),
            allocator: AllocatorParams::default(),
            io_scheduler: IoSchedulerParams::default(),
            power: PowerProfile::Balanced,
            target_workload: WorkloadCategory::Server,
        };

        optimizer.add_profile(custom);
        assert!(optimizer
            .available_profiles()
            .contains(&"custom".to_string()));
    }
}
