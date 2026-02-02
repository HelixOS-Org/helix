//! # Scheduler Metrics
//!
//! Metrics and statistics for scheduler performance monitoring.

use core::sync::atomic::{AtomicU64, Ordering};

/// Scheduler metrics
pub struct SchedulerMetrics {
    /// Total context switches
    context_switches: AtomicU64,
    /// Total timer ticks
    ticks: AtomicU64,
    /// Total voluntary yields
    voluntary_yields: AtomicU64,
    /// Total preemptions
    preemptions: AtomicU64,
    /// Total migrations
    migrations: AtomicU64,
    /// Total idle time (nanoseconds)
    idle_time: AtomicU64,
    /// Total run time (nanoseconds)
    run_time: AtomicU64,
    /// Total wait time (nanoseconds)
    wait_time: AtomicU64,
}

impl SchedulerMetrics {
    /// Create new metrics
    pub const fn new() -> Self {
        Self {
            context_switches: AtomicU64::new(0),
            ticks: AtomicU64::new(0),
            voluntary_yields: AtomicU64::new(0),
            preemptions: AtomicU64::new(0),
            migrations: AtomicU64::new(0),
            idle_time: AtomicU64::new(0),
            run_time: AtomicU64::new(0),
            wait_time: AtomicU64::new(0),
        }
    }

    /// Record a context switch
    pub fn record_context_switch(&self) {
        self.context_switches.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a timer tick
    pub fn record_tick(&self) {
        self.ticks.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a voluntary yield
    pub fn record_yield(&self) {
        self.voluntary_yields.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a preemption
    pub fn record_preemption(&self) {
        self.preemptions.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a migration
    pub fn record_migration(&self) {
        self.migrations.fetch_add(1, Ordering::Relaxed);
    }

    /// Record idle time
    pub fn record_idle_time(&self, ns: u64) {
        self.idle_time.fetch_add(ns, Ordering::Relaxed);
    }

    /// Record run time
    pub fn record_run_time(&self, ns: u64) {
        self.run_time.fetch_add(ns, Ordering::Relaxed);
    }

    /// Record wait time
    pub fn record_wait_time(&self, ns: u64) {
        self.wait_time.fetch_add(ns, Ordering::Relaxed);
    }

    /// Get total context switches
    pub fn context_switches(&self) -> u64 {
        self.context_switches.load(Ordering::Relaxed)
    }

    /// Get total ticks
    pub fn ticks(&self) -> u64 {
        self.ticks.load(Ordering::Relaxed)
    }

    /// Get voluntary yields
    pub fn voluntary_yields(&self) -> u64 {
        self.voluntary_yields.load(Ordering::Relaxed)
    }

    /// Get preemptions
    pub fn preemptions(&self) -> u64 {
        self.preemptions.load(Ordering::Relaxed)
    }

    /// Get migrations
    pub fn migrations(&self) -> u64 {
        self.migrations.load(Ordering::Relaxed)
    }

    /// Get idle time
    pub fn idle_time(&self) -> u64 {
        self.idle_time.load(Ordering::Relaxed)
    }

    /// Get run time
    pub fn run_time(&self) -> u64 {
        self.run_time.load(Ordering::Relaxed)
    }

    /// Get wait time
    pub fn wait_time(&self) -> u64 {
        self.wait_time.load(Ordering::Relaxed)
    }

    /// Get CPU utilization (percentage)
    pub fn cpu_utilization(&self) -> u8 {
        let run = self.run_time.load(Ordering::Relaxed);
        let idle = self.idle_time.load(Ordering::Relaxed);
        let total = run + idle;

        if total == 0 {
            0
        } else {
            ((run * 100) / total) as u8
        }
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.context_switches.store(0, Ordering::Relaxed);
        self.ticks.store(0, Ordering::Relaxed);
        self.voluntary_yields.store(0, Ordering::Relaxed);
        self.preemptions.store(0, Ordering::Relaxed);
        self.migrations.store(0, Ordering::Relaxed);
        self.idle_time.store(0, Ordering::Relaxed);
        self.run_time.store(0, Ordering::Relaxed);
        self.wait_time.store(0, Ordering::Relaxed);
    }
}
