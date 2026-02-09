//! # Holistic Scheduler
//!
//! System-wide scheduling decisions integrating all subsystems:
//! - Global scheduling policy enforcement
//! - Cross-process fairness
//! - Latency-sensitive task prioritization
//! - NUMA-aware scheduling
//! - Energy-efficient scheduling
//! - Real-time task management
//! - Scheduling decision history

extern crate alloc;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::vec::Vec;

// ============================================================================
// SCHEDULING CLASSES
// ============================================================================

/// Global scheduling class
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GlobalSchedClass {
    /// Real-time FIFO
    RealtimeFifo,
    /// Real-time round-robin
    RealtimeRR,
    /// Interactive (latency-sensitive)
    Interactive,
    /// Normal timesharing
    Normal,
    /// Batch processing
    Batch,
    /// Idle (only when nothing else to run)
    Idle,
    /// Background (lowest priority)
    Background,
}

impl GlobalSchedClass {
    /// Base time slice (microseconds)
    #[inline]
    pub fn base_timeslice_us(&self) -> u64 {
        match self {
            Self::RealtimeFifo => u64::MAX, // Runs until yield
            Self::RealtimeRR => 5_000,
            Self::Interactive => 10_000,
            Self::Normal => 20_000,
            Self::Batch => 50_000,
            Self::Idle => 100_000,
            Self::Background => 100_000,
        }
    }

    /// Whether class is preemptible
    #[inline]
    pub fn preemptible(&self) -> bool {
        match self {
            Self::RealtimeFifo => false,
            _ => true,
        }
    }
}

/// NUMA node affinity
#[derive(Debug, Clone)]
pub struct NumaAffinity {
    /// Preferred NUMA nodes (bitmask)
    pub preferred_nodes: u64,
    /// Strict affinity (must run on preferred)
    pub strict: bool,
    /// Migration penalty threshold (microseconds)
    pub migration_penalty_us: u64,
}

impl Default for NumaAffinity {
    fn default() -> Self {
        Self {
            preferred_nodes: u64::MAX, // All nodes
            strict: false,
            migration_penalty_us: 5000,
        }
    }
}

/// Per-process scheduling parameters
#[derive(Debug, Clone)]
pub struct ProcessSchedParams {
    /// PID
    pub pid: u64,
    /// Scheduling class
    pub sched_class: GlobalSchedClass,
    /// Nice value (-20 to 19)
    pub nice: i8,
    /// CPU affinity mask
    pub cpu_affinity: u64,
    /// NUMA affinity
    pub numa_affinity: NumaAffinity,
    /// Time slice adjustment factor (basis points, 10000 = 1.0x)
    pub timeslice_factor: u32,
    /// Energy preference (0 = performance, 100 = efficiency)
    pub energy_preference: u8,
    /// Whether process is latency-sensitive
    pub latency_sensitive: bool,
    /// Last scheduled core
    pub last_core: Option<u32>,
    /// Voluntary context switches
    pub voluntary_switches: u64,
    /// Involuntary context switches
    pub involuntary_switches: u64,
    /// Total CPU time (microseconds)
    pub cpu_time_us: u64,
    /// Wait time (microseconds)
    pub wait_time_us: u64,
}

impl ProcessSchedParams {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            sched_class: GlobalSchedClass::Normal,
            nice: 0,
            cpu_affinity: u64::MAX,
            numa_affinity: NumaAffinity::default(),
            timeslice_factor: 10000,
            energy_preference: 50,
            latency_sensitive: false,
            last_core: None,
            voluntary_switches: 0,
            involuntary_switches: 0,
            cpu_time_us: 0,
            wait_time_us: 0,
        }
    }

    /// Effective timeslice (us)
    #[inline]
    pub fn effective_timeslice(&self) -> u64 {
        let base = self.sched_class.base_timeslice_us();
        if base == u64::MAX {
            return base;
        }
        (base * self.timeslice_factor as u64) / 10000
    }

    /// CPU-to-wait ratio
    #[inline]
    pub fn cpu_ratio(&self) -> f64 {
        let total = self.cpu_time_us + self.wait_time_us;
        if total == 0 {
            return 0.5;
        }
        self.cpu_time_us as f64 / total as f64
    }
}

// ============================================================================
// SCHEDULING DECISIONS
// ============================================================================

/// Scheduling decision type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedDecision {
    /// Run on specified core
    RunOn(u32),
    /// Preempt current process
    Preempt,
    /// Yield to higher priority
    Yield,
    /// Migrate to different core
    Migrate(u32),
    /// Put to sleep
    Sleep,
    /// Boost priority temporarily
    Boost(u32),
    /// Throttle (reduce timeslice)
    Throttle,
    /// No change
    NoChange,
}

/// Reason for scheduling decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedReason {
    /// Timeslice expired
    TimesliceExpired,
    /// Higher priority task ready
    HigherPriorityReady,
    /// Load balancing
    LoadBalance,
    /// NUMA migration
    NumaMigration,
    /// Energy optimization
    EnergyOptimization,
    /// Fairness enforcement
    Fairness,
    /// Latency requirement
    LatencyRequirement,
    /// Cooperation bonus
    CooperationBonus,
    /// Thermal throttling
    ThermalThrottle,
    /// User request
    UserRequest,
}

/// A scheduling decision record
#[derive(Debug, Clone)]
pub struct SchedRecord {
    /// PID affected
    pub pid: u64,
    /// Decision
    pub decision: SchedDecision,
    /// Reason
    pub reason: SchedReason,
    /// Timestamp
    pub timestamp: u64,
    /// Previous core
    pub prev_core: Option<u32>,
}

// ============================================================================
// LOAD BALANCER
// ============================================================================

/// Per-core load information
#[derive(Debug, Clone)]
pub struct CoreLoad {
    /// Core ID
    pub core_id: u32,
    /// NUMA node
    pub numa_node: u32,
    /// Run queue length
    pub run_queue_len: u32,
    /// CPU utilization (percent * 100)
    pub utilization: u32,
    /// Current frequency (MHz)
    pub frequency_mhz: u32,
    /// Temperature (Celsius * 10)
    pub temperature: u32,
    /// Processes assigned
    pub process_count: u32,
    /// Total load weight
    pub load_weight: u64,
}

/// Load balancer
pub struct LoadBalancer {
    /// Per-core loads
    core_loads: Vec<CoreLoad>,
    /// Average load
    avg_load: f64,
    /// Imbalance threshold (percent)
    imbalance_threshold: f64,
    /// Migration count
    pub migrations: u64,
}

impl LoadBalancer {
    pub fn new(num_cores: usize) -> Self {
        let mut core_loads = Vec::with_capacity(num_cores);
        for i in 0..num_cores {
            core_loads.push(CoreLoad {
                core_id: i as u32,
                numa_node: 0,
                run_queue_len: 0,
                utilization: 0,
                frequency_mhz: 0,
                temperature: 0,
                process_count: 0,
                load_weight: 0,
            });
        }
        Self {
            core_loads,
            avg_load: 0.0,
            imbalance_threshold: 25.0,
            migrations: 0,
        }
    }

    /// Update core load
    #[inline]
    pub fn update_core(&mut self, core_id: u32, load: CoreLoad) {
        if let Some(cl) = self.core_loads.get_mut(core_id as usize) {
            *cl = load;
        }
        self.recalculate_avg();
    }

    /// Recalculate average load
    fn recalculate_avg(&mut self) {
        if self.core_loads.is_empty() {
            self.avg_load = 0.0;
            return;
        }
        let sum: u64 = self.core_loads.iter().map(|c| c.load_weight).sum();
        self.avg_load = sum as f64 / self.core_loads.len() as f64;
    }

    /// Find lightest loaded core
    #[inline]
    pub fn lightest_core(&self) -> Option<u32> {
        self.core_loads
            .iter()
            .min_by_key(|c| c.load_weight)
            .map(|c| c.core_id)
    }

    /// Find lightest core on NUMA node
    #[inline]
    pub fn lightest_core_on_node(&self, node: u32) -> Option<u32> {
        self.core_loads
            .iter()
            .filter(|c| c.numa_node == node)
            .min_by_key(|c| c.load_weight)
            .map(|c| c.core_id)
    }

    /// Check if rebalancing is needed
    pub fn needs_rebalance(&self) -> bool {
        if self.core_loads.len() < 2 || self.avg_load < 1.0 {
            return false;
        }
        let max_load = self
            .core_loads
            .iter()
            .map(|c| c.load_weight)
            .max()
            .unwrap_or(0);
        let min_load = self
            .core_loads
            .iter()
            .map(|c| c.load_weight)
            .min()
            .unwrap_or(0);

        if max_load == 0 {
            return false;
        }
        let imbalance = ((max_load - min_load) as f64 / max_load as f64) * 100.0;
        imbalance > self.imbalance_threshold
    }

    /// Suggest migration: returns (pid_placeholder, from_core, to_core)
    pub fn suggest_migration(&self) -> Option<(u32, u32)> {
        if !self.needs_rebalance() {
            return None;
        }
        let heaviest = self
            .core_loads
            .iter()
            .max_by_key(|c| c.load_weight)?
            .core_id;
        let lightest = self.lightest_core()?;
        if heaviest != lightest {
            Some((heaviest, lightest))
        } else {
            None
        }
    }

    /// Core count
    #[inline(always)]
    pub fn core_count(&self) -> usize {
        self.core_loads.len()
    }

    /// Get core load
    #[inline(always)]
    pub fn get_core(&self, core_id: u32) -> Option<&CoreLoad> {
        self.core_loads.get(core_id as usize)
    }
}

// ============================================================================
// HOLISTIC SCHEDULER
// ============================================================================

/// Global scheduling engine
pub struct HolisticScheduler {
    /// Per-process scheduling params
    processes: BTreeMap<u64, ProcessSchedParams>,
    /// Load balancer
    load_balancer: LoadBalancer,
    /// Decision history
    history: VecDeque<SchedRecord>,
    /// Max history
    max_history: usize,
    /// Total decisions made
    pub total_decisions: u64,
    /// Total migrations
    pub total_migrations: u64,
    /// Total preemptions
    pub total_preemptions: u64,
}

impl HolisticScheduler {
    pub fn new(num_cores: usize) -> Self {
        Self {
            processes: BTreeMap::new(),
            load_balancer: LoadBalancer::new(num_cores),
            history: VecDeque::new(),
            max_history: 1000,
            total_decisions: 0,
            total_migrations: 0,
            total_preemptions: 0,
        }
    }

    /// Register process
    #[inline]
    pub fn register(&mut self, pid: u64) {
        self.processes
            .entry(pid)
            .or_insert_with(|| ProcessSchedParams::new(pid));
    }

    /// Unregister process
    #[inline(always)]
    pub fn unregister(&mut self, pid: u64) {
        self.processes.remove(&pid);
    }

    /// Set scheduling class
    #[inline]
    pub fn set_class(&mut self, pid: u64, class: GlobalSchedClass) {
        if let Some(p) = self.processes.get_mut(&pid) {
            p.sched_class = class;
        }
    }

    /// Set nice value
    #[inline]
    pub fn set_nice(&mut self, pid: u64, nice: i8) {
        if let Some(p) = self.processes.get_mut(&pid) {
            p.nice = nice.max(-20).min(19);
        }
    }

    /// Make scheduling decision for a process
    pub fn schedule(&mut self, pid: u64, timestamp: u64) -> SchedDecision {
        let params = match self.processes.get(&pid) {
            Some(p) => p.clone(),
            None => return SchedDecision::NoChange,
        };

        self.total_decisions += 1;

        // Simple decision logic
        let decision = if params.latency_sensitive {
            // Prefer sticking to same core for cache
            if let Some(core) = params.last_core {
                SchedDecision::RunOn(core)
            } else if let Some(core) = self.load_balancer.lightest_core() {
                SchedDecision::RunOn(core)
            } else {
                SchedDecision::NoChange
            }
        } else if self.load_balancer.needs_rebalance() {
            if let Some((_from, to)) = self.load_balancer.suggest_migration() {
                self.total_migrations += 1;
                SchedDecision::Migrate(to)
            } else {
                SchedDecision::NoChange
            }
        } else {
            SchedDecision::NoChange
        };

        // Record decision
        let record = SchedRecord {
            pid,
            decision,
            reason: SchedReason::LoadBalance,
            timestamp,
            prev_core: params.last_core,
        };

        self.history.push_back(record);
        if self.history.len() > self.max_history {
            self.history.pop_front();
        }

        decision
    }

    /// Get process parameters
    #[inline(always)]
    pub fn get_params(&self, pid: u64) -> Option<&ProcessSchedParams> {
        self.processes.get(&pid)
    }

    /// Get load balancer
    #[inline(always)]
    pub fn load_balancer(&self) -> &LoadBalancer {
        &self.load_balancer
    }

    /// Get mutable load balancer
    #[inline(always)]
    pub fn load_balancer_mut(&mut self) -> &mut LoadBalancer {
        &mut self.load_balancer
    }

    /// Process count
    #[inline(always)]
    pub fn process_count(&self) -> usize {
        self.processes.len()
    }
}
