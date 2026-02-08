//! # Apps Scheduler Policy Manager
//!
//! Per-application scheduler policy management:
//! - SCHED_FIFO/SCHED_RR/SCHED_DEADLINE policies
//! - CPU affinity and pinning
//! - Nice value management
//! - Bandwidth throttling
//! - Latency-sensitive task boosting
//! - Energy-aware scheduling hints

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Scheduling policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedPolicyType {
    Normal,
    Fifo,
    RoundRobin,
    Batch,
    Idle,
    Deadline,
    EnergyAware,
}

/// Scheduling class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedClass {
    Stop,
    Deadline,
    Realtime,
    Fair,
    Idle,
}

impl SchedClass {
    pub fn priority_order(&self) -> u8 {
        match self {
            Self::Stop => 0,
            Self::Deadline => 1,
            Self::Realtime => 2,
            Self::Fair => 3,
            Self::Idle => 4,
        }
    }
}

/// CPU affinity mask
#[derive(Debug, Clone)]
pub struct CpuAffinityMask {
    pub bits: Vec<u64>,
    pub nr_cpus: u32,
}

impl CpuAffinityMask {
    pub fn new(nr_cpus: u32) -> Self {
        let words = ((nr_cpus as usize) + 63) / 64;
        Self { bits: alloc::vec![0u64; words], nr_cpus }
    }

    pub fn set(&mut self, cpu: u32) {
        if cpu < self.nr_cpus {
            let word = (cpu / 64) as usize;
            let bit = cpu % 64;
            if word < self.bits.len() { self.bits[word] |= 1u64 << bit; }
        }
    }

    pub fn clear(&mut self, cpu: u32) {
        if cpu < self.nr_cpus {
            let word = (cpu / 64) as usize;
            let bit = cpu % 64;
            if word < self.bits.len() { self.bits[word] &= !(1u64 << bit); }
        }
    }

    pub fn is_set(&self, cpu: u32) -> bool {
        if cpu >= self.nr_cpus { return false; }
        let word = (cpu / 64) as usize;
        let bit = cpu % 64;
        word < self.bits.len() && (self.bits[word] & (1u64 << bit)) != 0
    }

    pub fn count(&self) -> u32 {
        self.bits.iter().map(|w| w.count_ones()).sum()
    }

    pub fn set_all(&mut self) {
        for i in 0..self.nr_cpus { self.set(i); }
    }
}

/// Deadline scheduling parameters
#[derive(Debug, Clone, Copy)]
pub struct DeadlineParams {
    pub runtime_ns: u64,
    pub deadline_ns: u64,
    pub period_ns: u64,
    pub flags: u32,
}

impl DeadlineParams {
    pub fn new(runtime: u64, deadline: u64, period: u64) -> Self {
        Self { runtime_ns: runtime, deadline_ns: deadline, period_ns: period, flags: 0 }
    }

    pub fn utilization(&self) -> f64 {
        if self.period_ns == 0 { return 0.0; }
        self.runtime_ns as f64 / self.period_ns as f64
    }
}

/// Bandwidth throttle
#[derive(Debug, Clone, Copy)]
pub struct BandwidthThrottle {
    pub quota_us: i64,
    pub period_us: u64,
    pub burst_us: u64,
    pub throttled_count: u64,
    pub throttled_time_ns: u64,
}

impl BandwidthThrottle {
    pub fn new(quota: i64, period: u64) -> Self {
        Self { quota_us: quota, period_us: period, burst_us: 0, throttled_count: 0, throttled_time_ns: 0 }
    }

    pub fn utilization_limit(&self) -> f64 {
        if self.period_us == 0 || self.quota_us <= 0 { return 1.0; }
        self.quota_us as f64 / self.period_us as f64
    }
}

/// Per-task scheduling state
#[derive(Debug, Clone)]
pub struct TaskSchedState {
    pub tid: u64,
    pub pid: u64,
    pub policy: SchedPolicyType,
    pub sched_class: SchedClass,
    pub nice: i8,
    pub rt_priority: u8,
    pub affinity: CpuAffinityMask,
    pub deadline: Option<DeadlineParams>,
    pub bandwidth: Option<BandwidthThrottle>,
    pub latency_nice: i8,
    pub energy_aware: bool,
    pub util_avg: u64,
    pub runnable_avg: u64,
    pub last_cpu: u32,
    pub nr_migrations: u64,
    pub nr_voluntary_switches: u64,
    pub nr_involuntary_switches: u64,
    pub total_runtime_ns: u64,
    pub total_wait_ns: u64,
}

impl TaskSchedState {
    pub fn new(tid: u64, pid: u64, nr_cpus: u32) -> Self {
        let mut affinity = CpuAffinityMask::new(nr_cpus);
        affinity.set_all();
        Self {
            tid, pid, policy: SchedPolicyType::Normal, sched_class: SchedClass::Fair,
            nice: 0, rt_priority: 0, affinity, deadline: None, bandwidth: None,
            latency_nice: 0, energy_aware: false, util_avg: 0, runnable_avg: 0,
            last_cpu: 0, nr_migrations: 0, nr_voluntary_switches: 0,
            nr_involuntary_switches: 0, total_runtime_ns: 0, total_wait_ns: 0,
        }
    }

    pub fn set_policy(&mut self, policy: SchedPolicyType, priority: u8) {
        self.policy = policy;
        self.sched_class = match policy {
            SchedPolicyType::Fifo | SchedPolicyType::RoundRobin => SchedClass::Realtime,
            SchedPolicyType::Deadline => SchedClass::Deadline,
            SchedPolicyType::Idle => SchedClass::Idle,
            _ => SchedClass::Fair,
        };
        self.rt_priority = priority;
    }

    pub fn avg_wait_pct(&self) -> f64 {
        let total = self.total_runtime_ns + self.total_wait_ns;
        if total == 0 { 0.0 } else { self.total_wait_ns as f64 / total as f64 * 100.0 }
    }
}

/// Process scheduling profile
#[derive(Debug, Clone)]
pub struct ProcessSchedProfile {
    pub pid: u64,
    pub tasks: Vec<u64>,
    pub default_policy: SchedPolicyType,
    pub default_nice: i8,
    pub total_runtime_ns: u64,
    pub total_wait_ns: u64,
    pub avg_util: u64,
}

impl ProcessSchedProfile {
    pub fn new(pid: u64) -> Self {
        Self { pid, tasks: Vec::new(), default_policy: SchedPolicyType::Normal, default_nice: 0, total_runtime_ns: 0, total_wait_ns: 0, avg_util: 0 }
    }
}

/// Sched policy stats
#[derive(Debug, Clone, Default)]
pub struct SchedPolicyStats {
    pub total_tasks: usize,
    pub total_processes: usize,
    pub rt_tasks: usize,
    pub dl_tasks: usize,
    pub idle_tasks: usize,
    pub fair_tasks: usize,
    pub total_throttled: u64,
    pub policy_changes: u64,
}

/// Apps scheduler policy manager
pub struct AppsSchedPolicy {
    tasks: BTreeMap<u64, TaskSchedState>,
    processes: BTreeMap<u64, ProcessSchedProfile>,
    stats: SchedPolicyStats,
    nr_cpus: u32,
}

impl AppsSchedPolicy {
    pub fn new(nr_cpus: u32) -> Self {
        Self { tasks: BTreeMap::new(), processes: BTreeMap::new(), stats: SchedPolicyStats::default(), nr_cpus }
    }

    pub fn add_task(&mut self, tid: u64, pid: u64) {
        let task = TaskSchedState::new(tid, pid, self.nr_cpus);
        self.tasks.insert(tid, task);
        self.processes.entry(pid).or_insert_with(|| ProcessSchedProfile::new(pid)).tasks.push(tid);
    }

    pub fn set_policy(&mut self, tid: u64, policy: SchedPolicyType, priority: u8) {
        if let Some(t) = self.tasks.get_mut(&tid) {
            t.set_policy(policy, priority);
            self.stats.policy_changes += 1;
        }
    }

    pub fn set_affinity(&mut self, tid: u64, cpus: &[u32]) {
        if let Some(t) = self.tasks.get_mut(&tid) {
            t.affinity = CpuAffinityMask::new(self.nr_cpus);
            for &cpu in cpus { t.affinity.set(cpu); }
        }
    }

    pub fn set_deadline(&mut self, tid: u64, runtime: u64, deadline: u64, period: u64) {
        if let Some(t) = self.tasks.get_mut(&tid) {
            t.policy = SchedPolicyType::Deadline;
            t.sched_class = SchedClass::Deadline;
            t.deadline = Some(DeadlineParams::new(runtime, deadline, period));
        }
    }

    pub fn set_bandwidth(&mut self, tid: u64, quota: i64, period: u64) {
        if let Some(t) = self.tasks.get_mut(&tid) {
            t.bandwidth = Some(BandwidthThrottle::new(quota, period));
        }
    }

    pub fn record_run(&mut self, tid: u64, runtime_ns: u64, wait_ns: u64, cpu: u32) {
        if let Some(t) = self.tasks.get_mut(&tid) {
            t.total_runtime_ns += runtime_ns;
            t.total_wait_ns += wait_ns;
            if t.last_cpu != cpu { t.nr_migrations += 1; }
            t.last_cpu = cpu;
        }
    }

    pub fn recompute(&mut self) {
        self.stats.total_tasks = self.tasks.len();
        self.stats.total_processes = self.processes.len();
        self.stats.rt_tasks = self.tasks.values().filter(|t| t.sched_class == SchedClass::Realtime).count();
        self.stats.dl_tasks = self.tasks.values().filter(|t| t.sched_class == SchedClass::Deadline).count();
        self.stats.idle_tasks = self.tasks.values().filter(|t| t.sched_class == SchedClass::Idle).count();
        self.stats.fair_tasks = self.tasks.values().filter(|t| t.sched_class == SchedClass::Fair).count();
        self.stats.total_throttled = self.tasks.values().filter_map(|t| t.bandwidth.as_ref()).map(|b| b.throttled_count).sum();
    }

    pub fn task(&self, tid: u64) -> Option<&TaskSchedState> { self.tasks.get(&tid) }
    pub fn process(&self, pid: u64) -> Option<&ProcessSchedProfile> { self.processes.get(&pid) }
    pub fn stats(&self) -> &SchedPolicyStats { &self.stats }
}

// ============================================================================
// Merged from sched_policy_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedPolicyType {
    Normal,
    Fifo,
    RoundRobin,
    Batch,
    Idle,
    Deadline,
    Ext,
}

/// Scheduling priority range
#[derive(Debug, Clone, Copy)]
pub struct SchedPriority {
    pub policy: SchedPolicyType,
    pub priority: i32,
    pub nice: i32,
}

impl SchedPriority {
    pub fn new(policy: SchedPolicyType, prio: i32, nice: i32) -> Self {
        Self { policy, priority: prio, nice }
    }

    pub fn effective_priority(&self) -> i32 {
        match self.policy {
            SchedPolicyType::Fifo | SchedPolicyType::RoundRobin => self.priority + 100,
            SchedPolicyType::Deadline => 200,
            _ => 120 + self.nice,
        }
    }
}

/// Deadline scheduling parameters
#[derive(Debug, Clone)]
pub struct DeadlineParams {
    pub runtime_ns: u64,
    pub deadline_ns: u64,
    pub period_ns: u64,
    pub flags: u32,
}

impl DeadlineParams {
    pub fn new(runtime: u64, deadline: u64, period: u64) -> Self {
        Self { runtime_ns: runtime, deadline_ns: deadline, period_ns: period, flags: 0 }
    }

    pub fn utilization(&self) -> f64 {
        if self.period_ns == 0 { return 0.0; }
        self.runtime_ns as f64 / self.period_ns as f64
    }
}

/// Process scheduling state
#[derive(Debug)]
pub struct ProcessSchedState {
    pub pid: u64,
    pub sched_prio: SchedPriority,
    pub deadline: Option<DeadlineParams>,
    pub cpu_affinity_mask: u64,
    pub runtime_ns: u64,
    pub wait_ns: u64,
    pub switches: u64,
    pub migrations: u64,
    pub preemptions: u64,
    pub overruns: u64,
}

impl ProcessSchedState {
    pub fn new(pid: u64, policy: SchedPolicyType) -> Self {
        Self {
            pid, sched_prio: SchedPriority::new(policy, 0, 0),
            deadline: None, cpu_affinity_mask: u64::MAX,
            runtime_ns: 0, wait_ns: 0, switches: 0,
            migrations: 0, preemptions: 0, overruns: 0,
        }
    }

    pub fn set_policy(&mut self, policy: SchedPolicyType, prio: i32) {
        self.sched_prio = SchedPriority::new(policy, prio, self.sched_prio.nice);
    }

    pub fn set_nice(&mut self, nice: i32) {
        self.sched_prio.nice = nice.max(-20).min(19);
    }

    pub fn context_switch(&mut self, now_runtime: u64) {
        self.switches += 1;
        self.runtime_ns = now_runtime;
    }

    pub fn avg_timeslice_ns(&self) -> u64 {
        if self.switches == 0 { return 0; }
        self.runtime_ns / self.switches
    }

    pub fn is_realtime(&self) -> bool {
        matches!(self.sched_prio.policy, SchedPolicyType::Fifo | SchedPolicyType::RoundRobin | SchedPolicyType::Deadline)
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct SchedPolicyV2Stats {
    pub total_processes: u32,
    pub normal_count: u32,
    pub rt_count: u32,
    pub deadline_count: u32,
    pub total_switches: u64,
    pub total_preemptions: u64,
    pub total_migrations: u64,
}

/// Main scheduling policy v2 manager
pub struct AppSchedPolicyV2 {
    processes: BTreeMap<u64, ProcessSchedState>,
}

impl AppSchedPolicyV2 {
    pub fn new() -> Self { Self { processes: BTreeMap::new() } }

    pub fn register(&mut self, pid: u64, policy: SchedPolicyType) {
        self.processes.insert(pid, ProcessSchedState::new(pid, policy));
    }

    pub fn set_policy(&mut self, pid: u64, policy: SchedPolicyType, prio: i32) {
        if let Some(p) = self.processes.get_mut(&pid) { p.set_policy(policy, prio); }
    }

    pub fn set_nice(&mut self, pid: u64, nice: i32) {
        if let Some(p) = self.processes.get_mut(&pid) { p.set_nice(nice); }
    }

    pub fn set_deadline(&mut self, pid: u64, runtime: u64, deadline: u64, period: u64) {
        if let Some(p) = self.processes.get_mut(&pid) {
            p.set_policy(SchedPolicyType::Deadline, 0);
            p.deadline = Some(DeadlineParams::new(runtime, deadline, period));
        }
    }

    pub fn stats(&self) -> SchedPolicyV2Stats {
        let normal = self.processes.values().filter(|p| matches!(p.sched_prio.policy, SchedPolicyType::Normal | SchedPolicyType::Batch | SchedPolicyType::Idle)).count() as u32;
        let rt = self.processes.values().filter(|p| matches!(p.sched_prio.policy, SchedPolicyType::Fifo | SchedPolicyType::RoundRobin)).count() as u32;
        let dl = self.processes.values().filter(|p| p.sched_prio.policy == SchedPolicyType::Deadline).count() as u32;
        let switches: u64 = self.processes.values().map(|p| p.switches).sum();
        let preemptions: u64 = self.processes.values().map(|p| p.preemptions).sum();
        let migrations: u64 = self.processes.values().map(|p| p.migrations).sum();
        SchedPolicyV2Stats {
            total_processes: self.processes.len() as u32, normal_count: normal,
            rt_count: rt, deadline_count: dl, total_switches: switches,
            total_preemptions: preemptions, total_migrations: migrations,
        }
    }
}
