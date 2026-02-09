//! # Application Scheduler Latency Profiler
//!
//! Per-process scheduling latency measurement:
//! - Run-queue wait time tracking
//! - Wakeup-to-running latency
//! - Preemption latency
//! - Scheduling class transition tracking
//! - Tail-latency analysis (P95/P99)
//! - Latency budget enforcement

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Scheduling event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedEventKind {
    Enqueue,
    Dequeue,
    WakeUp,
    Sleep,
    Preempted,
    Yield,
    Migrated,
}

/// Latency bucket for histogram
#[derive(Debug, Clone)]
pub struct LatencyBucketApps {
    pub upper_bound_ns: u64,
    pub count: u64,
}

/// Scheduling latency histogram
#[derive(Debug, Clone)]
pub struct SchedLatencyHistogram {
    pub buckets: Vec<LatencyBucketApps>,
    pub total_count: u64,
    pub total_ns: u64,
    pub min_ns: u64,
    pub max_ns: u64,
}

impl SchedLatencyHistogram {
    pub fn new() -> Self {
        let boundaries = [
            1_000, 5_000, 10_000, 50_000, 100_000, 500_000,
            1_000_000, 5_000_000, 10_000_000, 50_000_000, 100_000_000,
            u64::MAX,
        ];
        let buckets = boundaries.iter().map(|&b| LatencyBucketApps { upper_bound_ns: b, count: 0 }).collect();
        Self {
            buckets,
            total_count: 0,
            total_ns: 0,
            min_ns: u64::MAX,
            max_ns: 0,
        }
    }

    pub fn record(&mut self, latency_ns: u64) {
        self.total_count += 1;
        self.total_ns += latency_ns;
        if latency_ns < self.min_ns { self.min_ns = latency_ns; }
        if latency_ns > self.max_ns { self.max_ns = latency_ns; }

        for bucket in &mut self.buckets {
            if latency_ns <= bucket.upper_bound_ns {
                bucket.count += 1;
                break;
            }
        }
    }

    #[inline(always)]
    pub fn avg_ns(&self) -> u64 {
        if self.total_count == 0 { return 0; }
        self.total_ns / self.total_count
    }

    /// Estimate percentile from histogram
    pub fn percentile(&self, p: f64) -> u64 {
        if self.total_count == 0 { return 0; }
        let target = (self.total_count as f64 * p) as u64;
        let mut cumulative = 0u64;
        for bucket in &self.buckets {
            cumulative += bucket.count;
            if cumulative >= target {
                return bucket.upper_bound_ns;
            }
        }
        self.max_ns
    }

    #[inline(always)]
    pub fn p50(&self) -> u64 { self.percentile(0.50) }
    #[inline(always)]
    pub fn p95(&self) -> u64 { self.percentile(0.95) }
    #[inline(always)]
    pub fn p99(&self) -> u64 { self.percentile(0.99) }
}

/// Per-thread scheduling state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ThreadSchedState {
    pub thread_id: u64,
    pub enqueue_ts: u64,
    pub last_run_ts: u64,
    pub total_wait_ns: u64,
    pub total_run_ns: u64,
    pub preempt_count: u64,
    pub yield_count: u64,
    pub wakeup_count: u64,
    pub latency_hist: SchedLatencyHistogram,
}

impl ThreadSchedState {
    pub fn new(thread_id: u64) -> Self {
        Self {
            thread_id,
            enqueue_ts: 0,
            last_run_ts: 0,
            total_wait_ns: 0,
            total_run_ns: 0,
            preempt_count: 0,
            yield_count: 0,
            wakeup_count: 0,
            latency_hist: SchedLatencyHistogram::new(),
        }
    }

    #[inline(always)]
    pub fn on_enqueue(&mut self, ts: u64) {
        self.enqueue_ts = ts;
    }

    #[inline]
    pub fn on_dequeue(&mut self, ts: u64) {
        if self.enqueue_ts > 0 {
            let wait = ts.saturating_sub(self.enqueue_ts);
            self.total_wait_ns += wait;
            self.latency_hist.record(wait);
        }
        self.last_run_ts = ts;
    }

    #[inline]
    pub fn on_preempt(&mut self, ts: u64) {
        self.preempt_count += 1;
        if self.last_run_ts > 0 {
            self.total_run_ns += ts.saturating_sub(self.last_run_ts);
        }
    }

    #[inline(always)]
    pub fn on_wakeup(&mut self, ts: u64) {
        self.wakeup_count += 1;
        self.enqueue_ts = ts;
    }

    #[inline]
    pub fn cpu_utilization(&self) -> f64 {
        let total = self.total_wait_ns + self.total_run_ns;
        if total == 0 { return 0.0; }
        self.total_run_ns as f64 / total as f64
    }
}

/// Per-process scheduling profile
#[derive(Debug, Clone)]
pub struct ProcessSchedProfile {
    pub pid: u64,
    pub threads: BTreeMap<u64, ThreadSchedState>,
    pub aggregate_hist: SchedLatencyHistogram,
    pub latency_budget_ns: u64,
    pub budget_violations: u64,
}

impl ProcessSchedProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            threads: BTreeMap::new(),
            aggregate_hist: SchedLatencyHistogram::new(),
            latency_budget_ns: 0,
            budget_violations: 0,
        }
    }

    #[inline(always)]
    pub fn register_thread(&mut self, thread_id: u64) {
        self.threads.entry(thread_id)
            .or_insert_with(|| ThreadSchedState::new(thread_id));
    }

    pub fn record_event(&mut self, thread_id: u64, event: SchedEventKind, ts: u64) {
        let thread = self.threads.entry(thread_id)
            .or_insert_with(|| ThreadSchedState::new(thread_id));

        match event {
            SchedEventKind::Enqueue => thread.on_enqueue(ts),
            SchedEventKind::Dequeue => {
                let wait = if thread.enqueue_ts > 0 { ts.saturating_sub(thread.enqueue_ts) } else { 0 };
                thread.on_dequeue(ts);
                if wait > 0 {
                    self.aggregate_hist.record(wait);
                    if self.latency_budget_ns > 0 && wait > self.latency_budget_ns {
                        self.budget_violations += 1;
                    }
                }
            }
            SchedEventKind::Preempted => thread.on_preempt(ts),
            SchedEventKind::WakeUp => thread.on_wakeup(ts),
            SchedEventKind::Yield => { thread.yield_count += 1; }
            SchedEventKind::Sleep | SchedEventKind::Migrated => {}
        }
    }

    #[inline]
    pub fn worst_p99_thread(&self) -> Option<u64> {
        self.threads.values()
            .filter(|t| t.latency_hist.total_count > 10)
            .max_by_key(|t| t.latency_hist.p99())
            .map(|t| t.thread_id)
    }
}

/// App scheduler latency profiler stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppSchedLatencyStats {
    pub total_processes: usize,
    pub total_threads: usize,
    pub total_events: u64,
    pub global_p50_ns: u64,
    pub global_p95_ns: u64,
    pub global_p99_ns: u64,
    pub budget_violations: u64,
}

/// Application Scheduler Latency Profiler
pub struct AppSchedLatencyProfiler {
    profiles: BTreeMap<u64, ProcessSchedProfile>,
    global_hist: SchedLatencyHistogram,
    stats: AppSchedLatencyStats,
}

impl AppSchedLatencyProfiler {
    pub fn new() -> Self {
        Self {
            profiles: BTreeMap::new(),
            global_hist: SchedLatencyHistogram::new(),
            stats: AppSchedLatencyStats::default(),
        }
    }

    #[inline(always)]
    pub fn register_process(&mut self, pid: u64) {
        self.profiles.entry(pid).or_insert_with(|| ProcessSchedProfile::new(pid));
    }

    #[inline]
    pub fn set_latency_budget(&mut self, pid: u64, budget_ns: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.latency_budget_ns = budget_ns;
        }
    }

    #[inline]
    pub fn record_event(&mut self, pid: u64, thread_id: u64, event: SchedEventKind, ts: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.record_event(thread_id, event, ts);
        }
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_processes = self.profiles.len();
        self.stats.total_threads = self.profiles.values().map(|p| p.threads.len()).sum();
        self.stats.total_events = self.profiles.values()
            .map(|p| p.aggregate_hist.total_count).sum();
        self.stats.budget_violations = self.profiles.values()
            .map(|p| p.budget_violations).sum();
        self.stats.global_p50_ns = self.global_hist.p50();
        self.stats.global_p95_ns = self.global_hist.p95();
        self.stats.global_p99_ns = self.global_hist.p99();
    }

    #[inline(always)]
    pub fn profile(&self, pid: u64) -> Option<&ProcessSchedProfile> {
        self.profiles.get(&pid)
    }

    #[inline(always)]
    pub fn stats(&self) -> &AppSchedLatencyStats {
        &self.stats
    }

    #[inline(always)]
    pub fn remove_process(&mut self, pid: u64) {
        self.profiles.remove(&pid);
        self.recompute();
    }
}
