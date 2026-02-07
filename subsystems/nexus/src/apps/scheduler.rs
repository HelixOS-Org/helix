//! # Per-Application Scheduler Hints
//!
//! Generates scheduling hints based on application behavior analysis:
//! - CPU affinity recommendations
//! - Priority adjustments
//! - Time slice tuning
//! - NUMA placement hints
//! - Preemption policy recommendations
//! - Group scheduling for related processes

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// SCHEDULING HINTS
// ============================================================================

/// Scheduling hint for a process
#[derive(Debug, Clone)]
pub struct SchedulingHint {
    /// Target process
    pub pid: u64,
    /// Recommended CPU affinity mask
    pub cpu_affinity: Option<u64>,
    /// Recommended NUMA node
    pub numa_node: Option<u32>,
    /// Recommended priority adjustment
    pub priority_delta: i8,
    /// Recommended time slice (µs, 0 = default)
    pub time_slice_us: u64,
    /// Recommended scheduling class
    pub sched_class: Option<SchedClassHint>,
    /// Preemption policy
    pub preemption: PreemptionPolicy,
    /// Whether to pin to specific CPUs
    pub pin_recommended: bool,
    /// Confidence in hint (0.0 - 1.0)
    pub confidence: f64,
    /// Reason for the hint
    pub reason: HintReason,
}

/// Recommended scheduling class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedClassHint {
    /// Normal time-sharing
    Normal,
    /// Batch (non-interactive)
    Batch,
    /// Idle (only run when nothing else)
    Idle,
    /// Real-time (latency sensitive)
    Realtime,
    /// Deadline-based
    Deadline,
}

/// Preemption policy recommendation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreemptionPolicy {
    /// Default preemption
    Default,
    /// Avoid preemption (latency-sensitive)
    AvoidPreemption,
    /// Eager preemption (fairness-critical)
    EagerPreemption,
    /// Cooperative (yield-based)
    Cooperative,
}

/// Reason for scheduling hint
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HintReason {
    /// CPU-bound workload detected
    CpuBound,
    /// I/O-bound workload detected
    IoBound,
    /// Latency-sensitive workload
    LatencySensitive,
    /// Throughput-oriented workload
    ThroughputOriented,
    /// Interactive workload
    Interactive,
    /// Background task
    Background,
    /// NUMA locality optimization
    NumaLocality,
    /// Cache locality optimization
    CacheLocality,
    /// Power saving
    PowerSaving,
    /// Related process grouping
    ProcessGrouping,
}

// ============================================================================
// WORKLOAD ANALYZER FOR SCHEDULING
// ============================================================================

/// Analyzes process behavior for scheduling decisions
#[derive(Debug)]
pub struct SchedulingAnalyzer {
    /// Per-process analysis state
    states: BTreeMap<u64, ProcessSchedState>,
    /// Max processes
    max_processes: usize,
    /// NUMA topology info
    numa_nodes: u32,
    /// CPUs per NUMA node
    cpus_per_node: u32,
    /// Total hints generated
    pub hints_generated: u64,
}

/// Per-process scheduling analysis state
#[derive(Debug, Clone)]
struct ProcessSchedState {
    /// Process ID
    pid: u64,
    /// Recent CPU usage samples
    cpu_samples: Vec<f64>,
    /// Recent I/O wait samples
    io_wait_samples: Vec<f64>,
    /// Recent context switch rate
    ctx_switch_rates: Vec<f64>,
    /// Recent cache miss rates
    cache_miss_rates: Vec<f64>,
    /// Current CPU
    current_cpu: u32,
    /// Last NUMA node
    last_numa_node: u32,
    /// Migration count
    migrations: u64,
    /// Wakeup latency samples (µs)
    wakeup_latencies: Vec<f64>,
    /// Run queue time samples (µs)
    runqueue_times: Vec<f64>,
    /// Last hint generated
    last_hint: Option<SchedulingHint>,
    /// Last hint timestamp
    last_hint_time: u64,
}

impl ProcessSchedState {
    fn new(pid: u64) -> Self {
        Self {
            pid,
            cpu_samples: Vec::new(),
            io_wait_samples: Vec::new(),
            ctx_switch_rates: Vec::new(),
            cache_miss_rates: Vec::new(),
            current_cpu: 0,
            last_numa_node: 0,
            migrations: 0,
            wakeup_latencies: Vec::new(),
            runqueue_times: Vec::new(),
            last_hint: None,
            last_hint_time: 0,
        }
    }

    fn add_sample(&mut self, cpu: f64, io_wait: f64, ctx_switch: f64) {
        Self::push_bounded(&mut self.cpu_samples, cpu, 30);
        Self::push_bounded(&mut self.io_wait_samples, io_wait, 30);
        Self::push_bounded(&mut self.ctx_switch_rates, ctx_switch, 30);
    }

    fn push_bounded(vec: &mut Vec<f64>, value: f64, max: usize) {
        if vec.len() >= max {
            vec.remove(0);
        }
        vec.push(value);
    }

    fn avg(samples: &[f64]) -> f64 {
        if samples.is_empty() {
            0.0
        } else {
            samples.iter().sum::<f64>() / samples.len() as f64
        }
    }

    fn is_cpu_bound(&self) -> bool {
        let avg_cpu = Self::avg(&self.cpu_samples);
        let avg_io = Self::avg(&self.io_wait_samples);
        avg_cpu > 0.7 && avg_io < 0.1
    }

    fn is_io_bound(&self) -> bool {
        let avg_io = Self::avg(&self.io_wait_samples);
        avg_io > 0.3
    }

    fn is_latency_sensitive(&self) -> bool {
        // High context switch rate + low CPU = likely interactive/latency-sensitive
        let avg_ctx = Self::avg(&self.ctx_switch_rates);
        let avg_cpu = Self::avg(&self.cpu_samples);
        avg_ctx > 100.0 && avg_cpu < 0.5
    }

    fn is_interactive(&self) -> bool {
        let avg_cpu = Self::avg(&self.cpu_samples);
        let avg_ctx = Self::avg(&self.ctx_switch_rates);
        avg_cpu < 0.3 && avg_ctx > 50.0
    }
}

impl SchedulingAnalyzer {
    pub fn new(numa_nodes: u32, cpus_per_node: u32) -> Self {
        Self {
            states: BTreeMap::new(),
            max_processes: 4096,
            numa_nodes,
            cpus_per_node,
            hints_generated: 0,
        }
    }

    /// Update process scheduling metrics
    pub fn update(
        &mut self,
        pid: u64,
        cpu_usage: f64,
        io_wait: f64,
        ctx_switch_rate: f64,
        current_cpu: u32,
    ) {
        let state = self
            .states
            .entry(pid)
            .or_insert_with(|| ProcessSchedState::new(pid));

        let old_cpu = state.current_cpu;
        state.add_sample(cpu_usage, io_wait, ctx_switch_rate);
        state.current_cpu = current_cpu;

        // Track migrations
        let old_node = old_cpu / self.cpus_per_node.max(1);
        let new_node = current_cpu / self.cpus_per_node.max(1);
        if old_node != new_node {
            state.migrations += 1;
        }
        state.last_numa_node = new_node;
    }

    /// Generate scheduling hint for a process
    pub fn generate_hint(&mut self, pid: u64, timestamp: u64) -> Option<SchedulingHint> {
        let state = self.states.get(&pid)?;

        // Don't generate hints too frequently
        if timestamp.saturating_sub(state.last_hint_time) < 1000 {
            return state.last_hint.clone();
        }

        if state.cpu_samples.len() < 5 {
            return None;
        }

        let hint = if state.is_cpu_bound() {
            self.cpu_bound_hint(state)
        } else if state.is_io_bound() {
            self.io_bound_hint(state)
        } else if state.is_latency_sensitive() {
            self.latency_hint(state)
        } else if state.is_interactive() {
            self.interactive_hint(state)
        } else {
            self.default_hint(state)
        };

        self.hints_generated += 1;

        if let Some(st) = self.states.get_mut(&pid) {
            st.last_hint = Some(hint.clone());
            st.last_hint_time = timestamp;
        }

        Some(hint)
    }

    fn cpu_bound_hint(&self, state: &ProcessSchedState) -> SchedulingHint {
        // For CPU-bound: pin to a specific core, increase time slice
        let numa_node = state.last_numa_node;
        let base_cpu = numa_node * self.cpus_per_node;
        let affinity = if self.cpus_per_node > 0 {
            let mut mask = 0u64;
            for i in 0..self.cpus_per_node.min(4) {
                mask |= 1u64 << (base_cpu + i);
            }
            Some(mask)
        } else {
            None
        };

        SchedulingHint {
            pid: state.pid,
            cpu_affinity: affinity,
            numa_node: Some(numa_node),
            priority_delta: 0,
            time_slice_us: 10000, // 10ms (larger slice for compute)
            sched_class: Some(SchedClassHint::Normal),
            preemption: PreemptionPolicy::AvoidPreemption,
            pin_recommended: state.migrations > 10,
            confidence: 0.8,
            reason: HintReason::CpuBound,
        }
    }

    fn io_bound_hint(&self, state: &ProcessSchedState) -> SchedulingHint {
        SchedulingHint {
            pid: state.pid,
            cpu_affinity: None,
            numa_node: None,
            priority_delta: -2,  // Slightly higher priority
            time_slice_us: 2000, // 2ms (short slice, will yield on I/O anyway)
            sched_class: Some(SchedClassHint::Normal),
            preemption: PreemptionPolicy::Default,
            pin_recommended: false,
            confidence: 0.7,
            reason: HintReason::IoBound,
        }
    }

    fn latency_hint(&self, state: &ProcessSchedState) -> SchedulingHint {
        SchedulingHint {
            pid: state.pid,
            cpu_affinity: None,
            numa_node: None,
            priority_delta: -5,  // Higher priority for low latency
            time_slice_us: 1000, // 1ms
            sched_class: Some(SchedClassHint::Normal),
            preemption: PreemptionPolicy::EagerPreemption,
            pin_recommended: false,
            confidence: 0.75,
            reason: HintReason::LatencySensitive,
        }
    }

    fn interactive_hint(&self, state: &ProcessSchedState) -> SchedulingHint {
        SchedulingHint {
            pid: state.pid,
            cpu_affinity: None,
            numa_node: None,
            priority_delta: -3,
            time_slice_us: 3000, // 3ms
            sched_class: Some(SchedClassHint::Normal),
            preemption: PreemptionPolicy::EagerPreemption,
            pin_recommended: false,
            confidence: 0.7,
            reason: HintReason::Interactive,
        }
    }

    fn default_hint(&self, state: &ProcessSchedState) -> SchedulingHint {
        SchedulingHint {
            pid: state.pid,
            cpu_affinity: None,
            numa_node: None,
            priority_delta: 0,
            time_slice_us: 4000, // 4ms default
            sched_class: None,
            preemption: PreemptionPolicy::Default,
            pin_recommended: false,
            confidence: 0.5,
            reason: HintReason::ThroughputOriented,
        }
    }

    /// Remove process
    pub fn remove_process(&mut self, pid: u64) {
        self.states.remove(&pid);
    }

    /// Processes with excessive migrations
    pub fn excessive_migrations(&self, threshold: u64) -> Vec<(u64, u64)> {
        self.states
            .iter()
            .filter(|(_, s)| s.migrations > threshold)
            .map(|(&pid, s)| (pid, s.migrations))
            .collect()
    }

    /// Number of tracked processes
    pub fn tracked_count(&self) -> usize {
        self.states.len()
    }
}
