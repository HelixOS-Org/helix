//! Workload Management
//!
//! GPU workload distribution, balancing, and scheduling.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                    Workload Distribution                            │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │  ┌──────────────────────────────────────────────────────────────┐  │
//! │  │                   Workload Scheduler                          │  │
//! │  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐   │  │
//! │  │  │   Graphics  │  │   Compute   │  │      Transfer       │   │  │
//! │  │  │    Queue    │  │    Queue    │  │       Queue         │   │  │
//! │  │  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘   │  │
//! │  │         │                │                    │              │  │
//! │  │         ▼                ▼                    ▼              │  │
//! │  │  ┌────────────────────────────────────────────────────────┐  │  │
//! │  │  │              GPU Hardware Queues                       │  │  │
//! │  │  └────────────────────────────────────────────────────────┘  │  │
//! │  └──────────────────────────────────────────────────────────────┘  │
//! │                                                                     │
//! │  ┌──────────────────────────────────────────────────────────────┐  │
//! │  │                    Load Balancer                              │  │
//! │  │  • Automatic queue selection                                  │  │
//! │  │  • Workload prediction                                        │  │
//! │  │  • Stall avoidance                                           │  │
//! │  └──────────────────────────────────────────────────────────────┘  │
//! │                                                                     │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```

use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

// ============================================================================
// Workload Types
// ============================================================================

/// Workload type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkloadType {
    /// Graphics rendering.
    Graphics,
    /// General compute.
    Compute,
    /// Transfer/copy.
    Transfer,
    /// Ray tracing.
    RayTracing,
    /// Video decode.
    VideoDecode,
    /// Video encode.
    VideoEncode,
}

impl Default for WorkloadType {
    fn default() -> Self {
        WorkloadType::Graphics
    }
}

/// Workload priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorkloadPriority {
    /// Background (lowest).
    Background = 0,
    /// Low.
    Low        = 1,
    /// Normal.
    Normal     = 2,
    /// High.
    High       = 3,
    /// Realtime (highest).
    Realtime   = 4,
}

impl Default for WorkloadPriority {
    fn default() -> Self {
        WorkloadPriority::Normal
    }
}

/// Workload flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorkloadFlags(u32);

impl WorkloadFlags {
    /// No flags.
    pub const NONE: Self = Self(0);
    /// Can be split across frames.
    pub const SPLITTABLE: Self = Self(1 << 0);
    /// Should run async.
    pub const ASYNC: Self = Self(1 << 1);
    /// Time critical.
    pub const TIME_CRITICAL: Self = Self(1 << 2);
    /// Can be preempted.
    pub const PREEMPTABLE: Self = Self(1 << 3);
    /// Must complete this frame.
    pub const FRAME_BOUND: Self = Self(1 << 4);

    /// Combine flags.
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Check flag.
    pub const fn contains(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }
}

impl Default for WorkloadFlags {
    fn default() -> Self {
        Self::NONE
    }
}

// ============================================================================
// Workload Description
// ============================================================================

/// Workload handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorkloadHandle(pub u64);

impl WorkloadHandle {
    /// Invalid handle.
    pub const INVALID: Self = Self(u64::MAX);

    /// Check if valid.
    pub fn is_valid(&self) -> bool {
        self.0 != u64::MAX
    }
}

impl Default for WorkloadHandle {
    fn default() -> Self {
        Self::INVALID
    }
}

/// Workload description.
#[derive(Debug, Clone)]
pub struct WorkloadDesc {
    /// Debug name.
    pub name: Option<String>,
    /// Workload type.
    pub workload_type: WorkloadType,
    /// Priority.
    pub priority: WorkloadPriority,
    /// Flags.
    pub flags: WorkloadFlags,
    /// Estimated cost (arbitrary units).
    pub estimated_cost: u32,
    /// Dependencies.
    pub dependencies: Vec<WorkloadHandle>,
}

impl Default for WorkloadDesc {
    fn default() -> Self {
        Self {
            name: None,
            workload_type: WorkloadType::Graphics,
            priority: WorkloadPriority::Normal,
            flags: WorkloadFlags::NONE,
            estimated_cost: 100,
            dependencies: Vec::new(),
        }
    }
}

impl WorkloadDesc {
    /// Create new workload description.
    pub fn new(workload_type: WorkloadType) -> Self {
        Self {
            workload_type,
            ..Default::default()
        }
    }

    /// Set name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set priority.
    pub fn with_priority(mut self, priority: WorkloadPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set flags.
    pub fn with_flags(mut self, flags: WorkloadFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Set estimated cost.
    pub fn with_cost(mut self, cost: u32) -> Self {
        self.estimated_cost = cost;
        self
    }

    /// Add dependency.
    pub fn with_dependency(mut self, dep: WorkloadHandle) -> Self {
        self.dependencies.push(dep);
        self
    }

    /// Add multiple dependencies.
    pub fn with_dependencies(mut self, deps: impl IntoIterator<Item = WorkloadHandle>) -> Self {
        self.dependencies.extend(deps);
        self
    }
}

// ============================================================================
// Workload State
// ============================================================================

/// Workload state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkloadState {
    /// Pending (waiting for dependencies).
    Pending,
    /// Ready to execute.
    Ready,
    /// Currently executing.
    Executing,
    /// Completed successfully.
    Completed,
    /// Failed.
    Failed,
    /// Cancelled.
    Cancelled,
}

impl Default for WorkloadState {
    fn default() -> Self {
        WorkloadState::Pending
    }
}

/// Workload info.
#[derive(Debug, Clone)]
pub struct WorkloadInfo {
    /// Handle.
    pub handle: WorkloadHandle,
    /// Description.
    pub desc: WorkloadDesc,
    /// State.
    pub state: WorkloadState,
    /// Submit time.
    pub submit_time: u64,
    /// Start time.
    pub start_time: Option<u64>,
    /// End time.
    pub end_time: Option<u64>,
    /// Actual cost.
    pub actual_cost: Option<u32>,
}

// ============================================================================
// Queue Utilization
// ============================================================================

/// Queue utilization info.
#[derive(Debug, Clone, Copy, Default)]
pub struct QueueUtilization {
    /// Queue index.
    pub queue_index: u32,
    /// Queue type.
    pub queue_type: WorkloadType,
    /// Utilization percentage (0-100).
    pub utilization: f32,
    /// Pending workloads.
    pub pending_count: u32,
    /// Active workloads.
    pub active_count: u32,
    /// Completed this frame.
    pub completed_count: u32,
    /// Total cost pending.
    pub pending_cost: u32,
}

// ============================================================================
// Load Balancer
// ============================================================================

/// Load balancing strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LoadBalanceStrategy {
    /// Round robin.
    RoundRobin,
    /// Least loaded.
    LeastLoaded,
    /// Priority based.
    PriorityBased,
    /// Cost based.
    CostBased,
    /// Adaptive.
    Adaptive,
}

impl Default for LoadBalanceStrategy {
    fn default() -> Self {
        LoadBalanceStrategy::Adaptive
    }
}

/// Load balancer configuration.
#[derive(Debug, Clone)]
pub struct LoadBalancerConfig {
    /// Strategy.
    pub strategy: LoadBalanceStrategy,
    /// Target utilization percentage.
    pub target_utilization: f32,
    /// Max pending per queue.
    pub max_pending_per_queue: u32,
    /// Enable work stealing.
    pub work_stealing: bool,
}

impl Default for LoadBalancerConfig {
    fn default() -> Self {
        Self {
            strategy: LoadBalanceStrategy::Adaptive,
            target_utilization: 80.0,
            max_pending_per_queue: 32,
            work_stealing: true,
        }
    }
}

/// Load balancer.
pub struct LoadBalancer {
    /// Configuration.
    config: LoadBalancerConfig,
    /// Queue utilizations.
    utilizations: Vec<QueueUtilization>,
    /// Round robin counter.
    round_robin_counter: AtomicU32,
}

impl LoadBalancer {
    /// Create new load balancer.
    pub fn new(config: LoadBalancerConfig) -> Self {
        Self {
            config,
            utilizations: Vec::new(),
            round_robin_counter: AtomicU32::new(0),
        }
    }

    /// Initialize with queues.
    pub fn initialize(&mut self, queue_count: u32, queue_types: &[WorkloadType]) {
        self.utilizations.clear();
        for i in 0..queue_count {
            self.utilizations.push(QueueUtilization {
                queue_index: i,
                queue_type: queue_types.get(i as usize).copied().unwrap_or_default(),
                ..Default::default()
            });
        }
    }

    /// Select queue for workload.
    pub fn select_queue(&self, desc: &WorkloadDesc) -> Option<u32> {
        // Filter compatible queues
        let compatible: Vec<_> = self
            .utilizations
            .iter()
            .filter(|u| self.is_compatible(u.queue_type, desc.workload_type))
            .collect();

        if compatible.is_empty() {
            return None;
        }

        match self.config.strategy {
            LoadBalanceStrategy::RoundRobin => {
                let idx = self.round_robin_counter.fetch_add(1, Ordering::Relaxed) as usize;
                Some(compatible[idx % compatible.len()].queue_index)
            },
            LoadBalanceStrategy::LeastLoaded => compatible
                .iter()
                .min_by(|a, b| a.utilization.partial_cmp(&b.utilization).unwrap())
                .map(|u| u.queue_index),
            LoadBalanceStrategy::PriorityBased => {
                // High priority goes to dedicated queues
                if desc.priority >= WorkloadPriority::High {
                    compatible.first().map(|u| u.queue_index)
                } else {
                    compatible.last().map(|u| u.queue_index)
                }
            },
            LoadBalanceStrategy::CostBased => {
                // Choose queue with most headroom for this cost
                compatible
                    .iter()
                    .filter(|u| u.pending_count < self.config.max_pending_per_queue)
                    .min_by_key(|u| u.pending_cost)
                    .map(|u| u.queue_index)
            },
            LoadBalanceStrategy::Adaptive => {
                // Combine multiple factors
                let mut best_queue: Option<u32> = None;
                let mut best_score = f32::MIN;

                for util in &compatible {
                    if util.pending_count >= self.config.max_pending_per_queue {
                        continue;
                    }

                    let utilization_score = 1.0 - (util.utilization / 100.0);
                    let pending_score = 1.0
                        - (util.pending_count as f32 / self.config.max_pending_per_queue as f32);
                    let priority_bonus = if desc.priority >= WorkloadPriority::High {
                        1.0 - (util.queue_index as f32 * 0.1)
                    } else {
                        0.0
                    };

                    let score =
                        utilization_score * 0.4 + pending_score * 0.4 + priority_bonus * 0.2;

                    if score > best_score {
                        best_score = score;
                        best_queue = Some(util.queue_index);
                    }
                }

                best_queue
            },
        }
    }

    /// Check if queue type is compatible with workload type.
    fn is_compatible(&self, queue_type: WorkloadType, workload_type: WorkloadType) -> bool {
        match (queue_type, workload_type) {
            // Graphics queue can do everything
            (WorkloadType::Graphics, _) => true,
            // Compute queue can do compute and transfer
            (WorkloadType::Compute, WorkloadType::Compute) => true,
            (WorkloadType::Compute, WorkloadType::Transfer) => true,
            // Transfer queue only does transfer
            (WorkloadType::Transfer, WorkloadType::Transfer) => true,
            // Specialized queues
            (WorkloadType::RayTracing, WorkloadType::RayTracing) => true,
            (WorkloadType::VideoDecode, WorkloadType::VideoDecode) => true,
            (WorkloadType::VideoEncode, WorkloadType::VideoEncode) => true,
            _ => false,
        }
    }

    /// Update queue utilization.
    pub fn update_utilization(
        &mut self,
        queue_index: u32,
        utilization: f32,
        pending: u32,
        active: u32,
    ) {
        if let Some(util) = self
            .utilizations
            .iter_mut()
            .find(|u| u.queue_index == queue_index)
        {
            util.utilization = utilization;
            util.pending_count = pending;
            util.active_count = active;
        }
    }

    /// Get utilizations.
    pub fn utilizations(&self) -> &[QueueUtilization] {
        &self.utilizations
    }

    /// Check if any queue is overloaded.
    pub fn is_overloaded(&self) -> bool {
        self.utilizations
            .iter()
            .any(|u| u.utilization > self.config.target_utilization)
    }

    /// Get average utilization.
    pub fn average_utilization(&self) -> f32 {
        if self.utilizations.is_empty() {
            return 0.0;
        }
        self.utilizations.iter().map(|u| u.utilization).sum::<f32>()
            / self.utilizations.len() as f32
    }
}

impl Default for LoadBalancer {
    fn default() -> Self {
        Self::new(LoadBalancerConfig::default())
    }
}

// ============================================================================
// Workload Scheduler
// ============================================================================

/// Scheduler statistics.
#[derive(Debug, Clone, Copy, Default)]
pub struct SchedulerStatistics {
    /// Total workloads submitted.
    pub total_submitted: u64,
    /// Total workloads completed.
    pub total_completed: u64,
    /// Total workloads failed.
    pub total_failed: u64,
    /// Average wait time (us).
    pub avg_wait_time_us: f32,
    /// Average execution time (us).
    pub avg_exec_time_us: f32,
    /// Workloads this frame.
    pub frame_workloads: u32,
}

/// Workload scheduler.
pub struct WorkloadScheduler {
    /// Next handle.
    next_handle: AtomicU64,
    /// Pending workloads by priority.
    pending: [VecDeque<WorkloadInfo>; 5],
    /// Active workloads.
    active: Vec<WorkloadInfo>,
    /// Completed workloads (recent).
    completed: VecDeque<WorkloadInfo>,
    /// Load balancer.
    load_balancer: LoadBalancer,
    /// Statistics.
    statistics: SchedulerStatistics,
    /// Current time.
    current_time: u64,
    /// Max completed history.
    max_completed_history: usize,
}

impl WorkloadScheduler {
    /// Create new scheduler.
    pub fn new() -> Self {
        Self {
            next_handle: AtomicU64::new(1),
            pending: Default::default(),
            active: Vec::new(),
            completed: VecDeque::new(),
            load_balancer: LoadBalancer::default(),
            statistics: SchedulerStatistics::default(),
            current_time: 0,
            max_completed_history: 100,
        }
    }

    /// Initialize with queues.
    pub fn initialize(&mut self, queue_count: u32, queue_types: &[WorkloadType]) {
        self.load_balancer.initialize(queue_count, queue_types);
    }

    /// Submit workload.
    pub fn submit(&mut self, desc: WorkloadDesc) -> WorkloadHandle {
        let handle = WorkloadHandle(self.next_handle.fetch_add(1, Ordering::Relaxed));

        let info = WorkloadInfo {
            handle,
            desc: desc.clone(),
            state: WorkloadState::Pending,
            submit_time: self.current_time,
            start_time: None,
            end_time: None,
            actual_cost: None,
        };

        let priority_idx = desc.priority as usize;
        self.pending[priority_idx].push_back(info);

        self.statistics.total_submitted += 1;
        self.statistics.frame_workloads += 1;

        handle
    }

    /// Try to schedule pending workloads.
    pub fn schedule(&mut self) -> Vec<(WorkloadHandle, u32)> {
        let mut scheduled = Vec::new();

        // Process from highest to lowest priority
        for priority in (0..5).rev() {
            let queue = &mut self.pending[priority];
            let mut i = 0;

            while i < queue.len() {
                let info = &queue[i];

                // Check dependencies
                let deps_ready = info.desc.dependencies.iter().all(|dep| {
                    self.completed
                        .iter()
                        .any(|c| c.handle == *dep && c.state == WorkloadState::Completed)
                });

                if !deps_ready {
                    i += 1;
                    continue;
                }

                // Select queue
                if let Some(queue_idx) = self.load_balancer.select_queue(&info.desc) {
                    let mut info = queue.remove(i).unwrap();
                    info.state = WorkloadState::Ready;
                    scheduled.push((info.handle, queue_idx));
                    self.active.push(info);
                } else {
                    i += 1;
                }
            }
        }

        scheduled
    }

    /// Mark workload started.
    pub fn start_workload(&mut self, handle: WorkloadHandle) {
        if let Some(info) = self.active.iter_mut().find(|w| w.handle == handle) {
            info.state = WorkloadState::Executing;
            info.start_time = Some(self.current_time);
        }
    }

    /// Mark workload completed.
    pub fn complete_workload(&mut self, handle: WorkloadHandle, actual_cost: Option<u32>) {
        if let Some(idx) = self.active.iter().position(|w| w.handle == handle) {
            let mut info = self.active.remove(idx);
            info.state = WorkloadState::Completed;
            info.end_time = Some(self.current_time);
            info.actual_cost = actual_cost;

            self.statistics.total_completed += 1;

            // Update statistics
            if let (Some(start), Some(end)) = (info.start_time, info.end_time) {
                let exec_time = end - start;
                self.statistics.avg_exec_time_us =
                    self.statistics.avg_exec_time_us * 0.9 + exec_time as f32 * 0.1;
            }
            if let Some(start) = info.start_time {
                let wait_time = start - info.submit_time;
                self.statistics.avg_wait_time_us =
                    self.statistics.avg_wait_time_us * 0.9 + wait_time as f32 * 0.1;
            }

            // Add to completed history
            self.completed.push_back(info);
            while self.completed.len() > self.max_completed_history {
                self.completed.pop_front();
            }
        }
    }

    /// Mark workload failed.
    pub fn fail_workload(&mut self, handle: WorkloadHandle) {
        if let Some(idx) = self.active.iter().position(|w| w.handle == handle) {
            let mut info = self.active.remove(idx);
            info.state = WorkloadState::Failed;
            info.end_time = Some(self.current_time);

            self.statistics.total_failed += 1;
            self.completed.push_back(info);
        }
    }

    /// Cancel workload.
    pub fn cancel(&mut self, handle: WorkloadHandle) -> bool {
        // Check pending
        for queue in &mut self.pending {
            if let Some(idx) = queue.iter().position(|w| w.handle == handle) {
                let mut info = queue.remove(idx).unwrap();
                info.state = WorkloadState::Cancelled;
                self.completed.push_back(info);
                return true;
            }
        }

        // Check active (can't really cancel, but mark)
        if let Some(info) = self.active.iter_mut().find(|w| w.handle == handle) {
            info.state = WorkloadState::Cancelled;
            return true;
        }

        false
    }

    /// Get workload state.
    pub fn get_state(&self, handle: WorkloadHandle) -> Option<WorkloadState> {
        // Check pending
        for queue in &self.pending {
            if let Some(info) = queue.iter().find(|w| w.handle == handle) {
                return Some(info.state);
            }
        }

        // Check active
        if let Some(info) = self.active.iter().find(|w| w.handle == handle) {
            return Some(info.state);
        }

        // Check completed
        if let Some(info) = self.completed.iter().find(|w| w.handle == handle) {
            return Some(info.state);
        }

        None
    }

    /// Update time.
    pub fn update_time(&mut self, time: u64) {
        self.current_time = time;
    }

    /// Begin frame.
    pub fn begin_frame(&mut self) {
        self.statistics.frame_workloads = 0;
    }

    /// Get statistics.
    pub fn statistics(&self) -> &SchedulerStatistics {
        &self.statistics
    }

    /// Get load balancer.
    pub fn load_balancer(&self) -> &LoadBalancer {
        &self.load_balancer
    }

    /// Get load balancer mut.
    pub fn load_balancer_mut(&mut self) -> &mut LoadBalancer {
        &mut self.load_balancer
    }

    /// Get pending count.
    pub fn pending_count(&self) -> usize {
        self.pending.iter().map(|q| q.len()).sum()
    }

    /// Get active count.
    pub fn active_count(&self) -> usize {
        self.active.len()
    }
}

impl Default for WorkloadScheduler {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Work Stealing
// ============================================================================

/// Work stealing configuration.
#[derive(Debug, Clone)]
pub struct WorkStealingConfig {
    /// Enable work stealing.
    pub enabled: bool,
    /// Steal threshold (queue utilization difference).
    pub steal_threshold: f32,
    /// Max steal batch size.
    pub max_steal_batch: u32,
    /// Min items to trigger steal.
    pub min_items_to_steal: u32,
}

impl Default for WorkStealingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            steal_threshold: 30.0,
            max_steal_batch: 4,
            min_items_to_steal: 2,
        }
    }
}

/// Work stealing statistics.
#[derive(Debug, Clone, Copy, Default)]
pub struct WorkStealingStatistics {
    /// Total steal attempts.
    pub steal_attempts: u64,
    /// Successful steals.
    pub successful_steals: u64,
    /// Items stolen.
    pub items_stolen: u64,
}
