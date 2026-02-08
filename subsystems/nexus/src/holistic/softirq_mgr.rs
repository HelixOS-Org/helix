//! # Holistic Soft IRQ Manager
//!
//! Software interrupt (softirq) processing management:
//! - Per-CPU softirq accounting and budgets
//! - ksoftirqd thread scheduling decisions
//! - NET_RX/NET_TX softirq tracking
//! - Softirq time budget enforcement
//! - Burst detection and throttling
//! - Softirq-to-workqueue migration

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Softirq vector types (matches Linux)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SoftIrqType {
    HiPriority,      // HI_SOFTIRQ
    Timer,            // TIMER_SOFTIRQ
    NetTx,            // NET_TX_SOFTIRQ
    NetRx,            // NET_RX_SOFTIRQ
    Block,            // BLOCK_SOFTIRQ
    IrqPoll,          // IRQ_POLL_SOFTIRQ
    Tasklet,          // TASKLET_SOFTIRQ
    Sched,            // SCHED_SOFTIRQ
    HrTimer,          // HRTIMER_SOFTIRQ
    Rcu,              // RCU_SOFTIRQ
}

/// Softirq processing state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoftirqState {
    Pending,
    Running,
    Completed,
    Deferred,
}

/// Per-vector statistics
#[derive(Debug, Clone)]
pub struct SoftirqVectorStats {
    pub vector: SoftIrqType,
    pub raised_count: u64,
    pub serviced_count: u64,
    pub deferred_count: u64,
    pub total_time_ns: u64,
    pub max_time_ns: u64,
    pub last_run_ts: u64,
}

impl SoftirqVectorStats {
    pub fn new(vec: SoftIrqType) -> Self {
        Self {
            vector: vec, raised_count: 0, serviced_count: 0,
            deferred_count: 0, total_time_ns: 0, max_time_ns: 0,
            last_run_ts: 0,
        }
    }

    pub fn avg_time_ns(&self) -> f64 {
        if self.serviced_count == 0 { return 0.0; }
        self.total_time_ns as f64 / self.serviced_count as f64
    }

    pub fn defer_ratio(&self) -> f64 {
        if self.raised_count == 0 { return 0.0; }
        self.deferred_count as f64 / self.raised_count as f64
    }
}

/// Per-CPU softirq state
#[derive(Debug, Clone)]
pub struct CpuSoftirqState {
    pub cpu_id: u32,
    pub pending_mask: u16,
    pub vectors: BTreeMap<u8, SoftirqVectorStats>,
    pub time_budget_ns: u64,
    pub time_used_ns: u64,
    pub max_loops: u32,
    pub current_loop: u32,
    pub ksoftirqd_wakeups: u64,
    pub in_softirq: bool,
    pub total_bursts: u64,
}

impl CpuSoftirqState {
    pub fn new(cpu: u32, budget_ns: u64) -> Self {
        let mut vectors = BTreeMap::new();
        for i in 0..10u8 {
            let vec = match i {
                0 => SoftIrqType::HiPriority,
                1 => SoftIrqType::Timer,
                2 => SoftIrqType::NetTx,
                3 => SoftIrqType::NetRx,
                4 => SoftIrqType::Block,
                5 => SoftIrqType::IrqPoll,
                6 => SoftIrqType::Tasklet,
                7 => SoftIrqType::Sched,
                8 => SoftIrqType::HrTimer,
                _ => SoftIrqType::Rcu,
            };
            vectors.insert(i, SoftirqVectorStats::new(vec));
        }
        Self {
            cpu_id: cpu, pending_mask: 0, vectors, time_budget_ns: budget_ns,
            time_used_ns: 0, max_loops: 10, current_loop: 0,
            ksoftirqd_wakeups: 0, in_softirq: false, total_bursts: 0,
        }
    }

    pub fn raise(&mut self, vec_idx: u8) {
        self.pending_mask |= 1 << vec_idx;
        if let Some(v) = self.vectors.get_mut(&vec_idx) { v.raised_count += 1; }
    }

    pub fn begin_processing(&mut self) {
        self.in_softirq = true;
        self.current_loop = 0;
        self.time_used_ns = 0;
    }

    pub fn process_vector(&mut self, vec_idx: u8, duration_ns: u64, ts: u64) -> bool {
        if self.time_used_ns + duration_ns > self.time_budget_ns {
            // Defer to ksoftirqd
            if let Some(v) = self.vectors.get_mut(&vec_idx) { v.deferred_count += 1; }
            self.ksoftirqd_wakeups += 1;
            return false;
        }
        self.pending_mask &= !(1 << vec_idx);
        self.time_used_ns += duration_ns;
        if let Some(v) = self.vectors.get_mut(&vec_idx) {
            v.serviced_count += 1;
            v.total_time_ns += duration_ns;
            if duration_ns > v.max_time_ns { v.max_time_ns = duration_ns; }
            v.last_run_ts = ts;
        }
        true
    }

    pub fn end_processing(&mut self) {
        self.in_softirq = false;
        self.current_loop += 1;
        if self.pending_mask != 0 { self.total_bursts += 1; }
    }

    pub fn budget_utilization(&self) -> f64 {
        if self.time_budget_ns == 0 { return 0.0; }
        self.time_used_ns as f64 / self.time_budget_ns as f64
    }

    pub fn pending_count(&self) -> u32 {
        self.pending_mask.count_ones()
    }
}

/// Burst detection
#[derive(Debug, Clone)]
pub struct BurstDetector {
    pub window_ns: u64,
    pub threshold: u32,
    pub recent: Vec<u64>,
    pub bursts_detected: u64,
    pub current_burst: bool,
}

impl BurstDetector {
    pub fn new(window_ns: u64, threshold: u32) -> Self {
        Self { window_ns, threshold, recent: Vec::new(), bursts_detected: 0, current_burst: false }
    }

    pub fn record(&mut self, ts: u64) {
        self.recent.push(ts);
        let cutoff = ts.saturating_sub(self.window_ns);
        self.recent.retain(|&t| t >= cutoff);
        let was_burst = self.current_burst;
        self.current_burst = self.recent.len() as u32 >= self.threshold;
        if self.current_burst && !was_burst { self.bursts_detected += 1; }
    }

    pub fn rate(&self) -> f64 {
        if self.window_ns == 0 { return 0.0; }
        (self.recent.len() as f64 * 1_000_000_000.0) / self.window_ns as f64
    }
}

/// Softirq manager stats
#[derive(Debug, Clone, Default)]
pub struct SoftirqMgrStats {
    pub total_cpus: usize,
    pub total_raised: u64,
    pub total_serviced: u64,
    pub total_deferred: u64,
    pub total_ksoftirqd_wakeups: u64,
    pub avg_budget_utilization: f64,
    pub max_vector_time_ns: u64,
    pub busiest_vector: u8,
    pub burst_count: u64,
    pub cpus_in_softirq: usize,
}

/// Holistic soft IRQ manager
pub struct HolisticSoftirqMgr {
    cpus: BTreeMap<u32, CpuSoftirqState>,
    burst_detector: BurstDetector,
    default_budget_ns: u64,
    stats: SoftirqMgrStats,
}

impl HolisticSoftirqMgr {
    pub fn new(budget_ns: u64) -> Self {
        Self {
            cpus: BTreeMap::new(),
            burst_detector: BurstDetector::new(1_000_000_000, 10_000),
            default_budget_ns: budget_ns,
            stats: SoftirqMgrStats::default(),
        }
    }

    pub fn init_cpu(&mut self, cpu: u32) {
        self.cpus.insert(cpu, CpuSoftirqState::new(cpu, self.default_budget_ns));
    }

    pub fn raise(&mut self, cpu: u32, vec_idx: u8, ts: u64) {
        if let Some(c) = self.cpus.get_mut(&cpu) { c.raise(vec_idx); }
        self.burst_detector.record(ts);
    }

    pub fn begin_processing(&mut self, cpu: u32) {
        if let Some(c) = self.cpus.get_mut(&cpu) { c.begin_processing(); }
    }

    pub fn process_vector(&mut self, cpu: u32, vec_idx: u8, duration_ns: u64, ts: u64) -> bool {
        if let Some(c) = self.cpus.get_mut(&cpu) {
            c.process_vector(vec_idx, duration_ns, ts)
        } else {
            false
        }
    }

    pub fn end_processing(&mut self, cpu: u32) {
        if let Some(c) = self.cpus.get_mut(&cpu) { c.end_processing(); }
    }

    pub fn recompute(&mut self) {
        self.stats.total_cpus = self.cpus.len();
        self.stats.total_raised = self.cpus.values().flat_map(|c| c.vectors.values()).map(|v| v.raised_count).sum();
        self.stats.total_serviced = self.cpus.values().flat_map(|c| c.vectors.values()).map(|v| v.serviced_count).sum();
        self.stats.total_deferred = self.cpus.values().flat_map(|c| c.vectors.values()).map(|v| v.deferred_count).sum();
        self.stats.total_ksoftirqd_wakeups = self.cpus.values().map(|c| c.ksoftirqd_wakeups).sum();
        if !self.cpus.is_empty() {
            self.stats.avg_budget_utilization = self.cpus.values().map(|c| c.budget_utilization()).sum::<f64>() / self.cpus.len() as f64;
        }
        let mut busiest_idx = 0u8;
        let mut busiest_time = 0u64;
        for cpu in self.cpus.values() {
            for (&idx, v) in &cpu.vectors {
                if v.total_time_ns > busiest_time { busiest_time = v.total_time_ns; busiest_idx = idx; }
                if v.max_time_ns > self.stats.max_vector_time_ns { self.stats.max_vector_time_ns = v.max_time_ns; }
            }
        }
        self.stats.busiest_vector = busiest_idx;
        self.stats.burst_count = self.burst_detector.bursts_detected;
        self.stats.cpus_in_softirq = self.cpus.values().filter(|c| c.in_softirq).count();
    }

    pub fn stats(&self) -> &SoftirqMgrStats { &self.stats }
}
