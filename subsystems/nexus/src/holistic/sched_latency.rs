//! # Holistic Scheduling Latency Tracker
//!
//! End-to-end scheduling latency measurement and optimization:
//! - Wakeup-to-run latency tracking
//! - Scheduling tail latency analysis
//! - Per-CPU scheduling delay histograms
//! - Latency target enforcement
//! - Context switch overhead measurement
//! - Scheduler queue depth monitoring

extern crate alloc;

use crate::fast::array_map::ArrayMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Latency category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatencyCategory {
    /// < 1μs
    SubMicrosecond,
    /// 1-10μs
    Microsecond,
    /// 10-100μs
    TensMicrosecond,
    /// 100μs - 1ms
    SubMillisecond,
    /// 1-10ms
    Millisecond,
    /// > 10ms
    TenPlusMs,
}

impl LatencyCategory {
    #[inline]
    pub fn from_ns(ns: u64) -> Self {
        match ns {
            0..=999 => Self::SubMicrosecond,
            1_000..=9_999 => Self::Microsecond,
            10_000..=99_999 => Self::TensMicrosecond,
            100_000..=999_999 => Self::SubMillisecond,
            1_000_000..=9_999_999 => Self::Millisecond,
            _ => Self::TenPlusMs,
        }
    }
}

/// Scheduling event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedEventType {
    WakeUp,
    RunqueueEnqueue,
    ContextSwitchIn,
    ContextSwitchOut,
    Migration,
    Preempted,
    Yielded,
}

/// Scheduling latency sample
#[derive(Debug, Clone)]
pub struct LatencySample {
    pub task_id: u64,
    pub cpu_id: u32,
    pub wakeup_ts: u64,
    pub enqueue_ts: u64,
    pub run_ts: u64,
    pub wakeup_to_run_ns: u64,
    pub queue_wait_ns: u64,
}

/// Per-task latency state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TaskLatencyState {
    pub task_id: u64,
    pub priority: i32,
    pub samples: VecDeque<u64>, // wakeup-to-run latencies
    pub max_samples: usize,
    pub total_wakeups: u64,
    pub total_latency_ns: u64,
    pub min_latency_ns: u64,
    pub max_latency_ns: u64,
    pub latency_target_ns: Option<u64>,
    pub target_violations: u64,
}

impl TaskLatencyState {
    pub fn new(task: u64, prio: i32, max: usize) -> Self {
        Self {
            task_id: task, priority: prio, samples: VecDeque::new(),
            max_samples: max, total_wakeups: 0, total_latency_ns: 0,
            min_latency_ns: u64::MAX, max_latency_ns: 0,
            latency_target_ns: None, target_violations: 0,
        }
    }

    #[inline]
    pub fn record(&mut self, latency_ns: u64) {
        self.samples.push_back(latency_ns);
        if self.samples.len() > self.max_samples { self.samples.pop_front(); }
        self.total_wakeups += 1;
        self.total_latency_ns += latency_ns;
        if latency_ns < self.min_latency_ns { self.min_latency_ns = latency_ns; }
        if latency_ns > self.max_latency_ns { self.max_latency_ns = latency_ns; }
        if let Some(target) = self.latency_target_ns {
            if latency_ns > target { self.target_violations += 1; }
        }
    }

    #[inline(always)]
    pub fn avg_latency_ns(&self) -> f64 {
        if self.total_wakeups == 0 { return 0.0; }
        self.total_latency_ns as f64 / self.total_wakeups as f64
    }

    #[inline(always)]
    pub fn p50_ns(&self) -> u64 { self.percentile(50) }
    #[inline(always)]
    pub fn p99_ns(&self) -> u64 { self.percentile(99) }

    fn percentile(&self, pct: u32) -> u64 {
        if self.samples.is_empty() { return 0; }
        let mut sorted = self.samples.clone();
        sorted.sort_unstable();
        let idx = ((pct as usize * sorted.len()) / 100).min(sorted.len() - 1);
        sorted[idx]
    }

    #[inline(always)]
    pub fn violation_rate(&self) -> f64 {
        if self.total_wakeups == 0 { return 0.0; }
        self.target_violations as f64 / self.total_wakeups as f64
    }
}

/// Per-CPU scheduling queue state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CpuSchedState {
    pub cpu_id: u32,
    pub queue_depth: u32,
    pub total_switches: u64,
    pub total_migrations_in: u64,
    pub total_migrations_out: u64,
    pub avg_switch_overhead_ns: f64,
    pub total_idle_ns: u64,
    pub total_busy_ns: u64,
    pub histogram: ArrayMap<u64, 32>, // bucket_us -> count
}

impl CpuSchedState {
    pub fn new(cpu: u32) -> Self {
        Self {
            cpu_id: cpu, queue_depth: 0, total_switches: 0,
            total_migrations_in: 0, total_migrations_out: 0,
            avg_switch_overhead_ns: 0.0, total_idle_ns: 0,
            total_busy_ns: 0, histogram: ArrayMap::new(0),
        }
    }

    #[inline]
    pub fn record_latency(&mut self, latency_ns: u64) {
        let bucket = (latency_ns / 1000) as u32; // microsecond buckets
        let bucket = bucket.min(10_000); // cap at 10ms
        self.histogram.add(bucket as usize, 1);
    }

    #[inline]
    pub fn utilization(&self) -> f64 {
        let total = self.total_idle_ns + self.total_busy_ns;
        if total == 0 { return 0.0; }
        self.total_busy_ns as f64 / total as f64
    }
}

/// Context switch measurement
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ContextSwitchInfo {
    pub from_task: u64,
    pub to_task: u64,
    pub cpu_id: u32,
    pub overhead_ns: u64,
    pub timestamp_ns: u64,
    pub voluntary: bool,
}

/// Sched latency stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct SchedLatencyStats {
    pub total_tasks: usize,
    pub total_cpus: usize,
    pub total_wakeups: u64,
    pub avg_latency_ns: f64,
    pub p50_latency_ns: u64,
    pub p99_latency_ns: u64,
    pub max_latency_ns: u64,
    pub total_switches: u64,
    pub avg_switch_overhead_ns: f64,
    pub target_violations: u64,
    pub avg_queue_depth: f64,
}

/// Holistic scheduling latency tracker
pub struct HolisticSchedLatency {
    tasks: BTreeMap<u64, TaskLatencyState>,
    cpus: BTreeMap<u32, CpuSchedState>,
    recent_switches: VecDeque<ContextSwitchInfo>,
    max_switch_history: usize,
    all_samples: VecDeque<u64>,
    max_global_samples: usize,
    stats: SchedLatencyStats,
}

impl HolisticSchedLatency {
    pub fn new() -> Self {
        Self {
            tasks: BTreeMap::new(), cpus: BTreeMap::new(),
            recent_switches: VecDeque::new(), max_switch_history: 1000,
            all_samples: VecDeque::new(), max_global_samples: 10_000,
            stats: SchedLatencyStats::default(),
        }
    }

    #[inline(always)]
    pub fn register_task(&mut self, task: u64, prio: i32) {
        self.tasks.insert(task, TaskLatencyState::new(task, prio, 200));
    }

    #[inline(always)]
    pub fn init_cpu(&mut self, cpu: u32) { self.cpus.insert(cpu, CpuSchedState::new(cpu)); }

    #[inline(always)]
    pub fn set_latency_target(&mut self, task: u64, target_ns: u64) {
        if let Some(t) = self.tasks.get_mut(&task) { t.latency_target_ns = Some(target_ns); }
    }

    #[inline]
    pub fn record_wakeup_to_run(&mut self, task: u64, cpu: u32, latency_ns: u64) {
        if let Some(t) = self.tasks.get_mut(&task) { t.record(latency_ns); }
        if let Some(c) = self.cpus.get_mut(&cpu) { c.record_latency(latency_ns); }
        self.all_samples.push_back(latency_ns);
        if self.all_samples.len() > self.max_global_samples { self.all_samples.pop_front().unwrap(); }
    }

    #[inline]
    pub fn record_switch(&mut self, info: ContextSwitchInfo) {
        if let Some(c) = self.cpus.get_mut(&info.cpu_id) { c.total_switches += 1; }
        self.recent_switches.push_back(info);
        if self.recent_switches.len() > self.max_switch_history { self.recent_switches.pop_front(); }
    }

    #[inline(always)]
    pub fn update_queue_depth(&mut self, cpu: u32, depth: u32) {
        if let Some(c) = self.cpus.get_mut(&cpu) { c.queue_depth = depth; }
    }

    #[inline]
    pub fn worst_tasks(&self, n: usize) -> Vec<(u64, f64)> {
        let mut sorted: Vec<(u64, f64)> = self.tasks.values().map(|t| (t.task_id, t.avg_latency_ns())).collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        sorted.truncate(n);
        sorted
    }

    pub fn recompute(&mut self) {
        self.stats.total_tasks = self.tasks.len();
        self.stats.total_cpus = self.cpus.len();
        self.stats.total_wakeups = self.tasks.values().map(|t| t.total_wakeups).sum();
        if !self.all_samples.is_empty() {
            self.stats.avg_latency_ns = self.all_samples.iter().sum::<u64>() as f64 / self.all_samples.len() as f64;
            let mut sorted = self.all_samples.clone();
            sorted.sort_unstable();
            let n = sorted.len();
            self.stats.p50_latency_ns = sorted[n / 2];
            self.stats.p99_latency_ns = sorted[(n * 99) / 100];
            self.stats.max_latency_ns = sorted[n - 1];
        }
        self.stats.total_switches = self.cpus.values().map(|c| c.total_switches).sum();
        if !self.recent_switches.is_empty() {
            self.stats.avg_switch_overhead_ns = self.recent_switches.iter().map(|s| s.overhead_ns as f64).sum::<f64>() / self.recent_switches.len() as f64;
        }
        self.stats.target_violations = self.tasks.values().map(|t| t.target_violations).sum();
        if !self.cpus.is_empty() {
            self.stats.avg_queue_depth = self.cpus.values().map(|c| c.queue_depth as f64).sum::<f64>() / self.cpus.len() as f64;
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &SchedLatencyStats { &self.stats }
}
