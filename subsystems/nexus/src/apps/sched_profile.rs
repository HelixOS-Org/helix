//! # Application Scheduling Profiler
//!
//! Detailed CPU scheduling behavior analysis:
//! - Run queue latency tracking
//! - Voluntary/involuntary context switch analysis
//! - CPU burst pattern detection
//! - Wake-up chain analysis
//! - Migration tracking

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

// ============================================================================
// SCHEDULING TYPES
// ============================================================================

/// Context switch reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextSwitchReason {
    /// Voluntary (yield, sleep, wait)
    Voluntary,
    /// Involuntary (preempted)
    Involuntary,
    /// Migration to another CPU
    Migration,
    /// Timer interrupt
    Timer,
    /// I/O completion
    IoCompletion,
    /// Signal delivery
    Signal,
}

/// Run state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunState {
    /// Running on CPU
    Running,
    /// Runnable (waiting for CPU)
    Runnable,
    /// Sleeping (waiting for event)
    Sleeping,
    /// Blocked on I/O
    BlockedIo,
    /// Blocked on lock
    BlockedLock,
    /// Stopped
    Stopped,
}

/// CPU burst
#[derive(Debug, Clone)]
pub struct CpuBurst {
    /// Start time (ns)
    pub start: u64,
    /// Duration (ns)
    pub duration: u64,
    /// CPU id
    pub cpu: u32,
    /// Instructions retired (if available)
    pub instructions: u64,
}

// ============================================================================
// SCHEDULING PROFILE
// ============================================================================

/// Per-thread scheduling profile
#[derive(Debug)]
pub struct ThreadSchedProfile {
    /// Thread id
    pub tid: u64,
    /// Current state
    pub state: RunState,
    /// Current CPU
    pub current_cpu: u32,
    /// Voluntary switches
    pub voluntary_switches: u64,
    /// Involuntary switches
    pub involuntary_switches: u64,
    /// Total run time (ns)
    pub total_run_time_ns: u64,
    /// Total wait time (run queue latency, ns)
    pub total_wait_time_ns: u64,
    /// Total sleep time (ns)
    pub total_sleep_time_ns: u64,
    /// Last schedule-in timestamp
    last_sched_in: u64,
    /// Last enqueue timestamp
    last_enqueue: u64,
    /// CPU migrations
    pub migrations: u64,
    /// Burst history (recent)
    bursts: VecDeque<CpuBurst>,
    /// Max bursts to keep
    max_bursts: usize,
}

impl ThreadSchedProfile {
    pub fn new(tid: u64) -> Self {
        Self {
            tid,
            state: RunState::Sleeping,
            current_cpu: 0,
            voluntary_switches: 0,
            involuntary_switches: 0,
            total_run_time_ns: 0,
            total_wait_time_ns: 0,
            total_sleep_time_ns: 0,
            last_sched_in: 0,
            last_enqueue: 0,
            migrations: 0,
            bursts: VecDeque::new(),
            max_bursts: 64,
        }
    }

    /// Record schedule-in (start running)
    pub fn on_sched_in(&mut self, cpu: u32, now: u64) {
        if self.current_cpu != cpu && self.state != RunState::Sleeping {
            self.migrations += 1;
        }
        self.current_cpu = cpu;
        // Wait time
        if self.last_enqueue > 0 {
            self.total_wait_time_ns += now.saturating_sub(self.last_enqueue);
        }
        self.last_sched_in = now;
        self.state = RunState::Running;
    }

    /// Record schedule-out (stop running)
    pub fn on_sched_out(&mut self, reason: ContextSwitchReason, now: u64) {
        let run_time = now.saturating_sub(self.last_sched_in);
        self.total_run_time_ns += run_time;

        // Record burst
        if self.bursts.len() >= self.max_bursts {
            self.bursts.pop_front();
        }
        self.bursts.push_back(CpuBurst {
            start: self.last_sched_in,
            duration: run_time,
            cpu: self.current_cpu,
            instructions: 0,
        });

        match reason {
            ContextSwitchReason::Voluntary => {
                self.voluntary_switches += 1;
                self.state = RunState::Sleeping;
            },
            ContextSwitchReason::Involuntary | ContextSwitchReason::Timer => {
                self.involuntary_switches += 1;
                self.state = RunState::Runnable;
                self.last_enqueue = now;
            },
            ContextSwitchReason::Migration => {
                self.migrations += 1;
                self.state = RunState::Runnable;
                self.last_enqueue = now;
            },
            ContextSwitchReason::IoCompletion | ContextSwitchReason::Signal => {
                self.voluntary_switches += 1;
                self.state = RunState::Sleeping;
            },
        }
    }

    /// Record wakeup
    #[inline(always)]
    pub fn on_wakeup(&mut self, now: u64) {
        self.state = RunState::Runnable;
        self.last_enqueue = now;
    }

    /// Average burst duration (ns)
    #[inline]
    pub fn avg_burst_ns(&self) -> f64 {
        if self.bursts.is_empty() {
            return 0.0;
        }
        let sum: u64 = self.bursts.iter().map(|b| b.duration).sum();
        sum as f64 / self.bursts.len() as f64
    }

    /// Average run queue latency (ns)
    #[inline]
    pub fn avg_wait_ns(&self) -> f64 {
        let total_switches = self.voluntary_switches + self.involuntary_switches;
        if total_switches == 0 {
            return 0.0;
        }
        self.total_wait_time_ns as f64 / total_switches as f64
    }

    /// CPU utilization
    #[inline]
    pub fn cpu_utilization(&self) -> f64 {
        let total = self.total_run_time_ns + self.total_wait_time_ns + self.total_sleep_time_ns;
        if total == 0 {
            return 0.0;
        }
        self.total_run_time_ns as f64 / total as f64
    }

    /// Is CPU bound?
    #[inline(always)]
    pub fn is_cpu_bound(&self) -> bool {
        self.involuntary_switches > self.voluntary_switches * 2
    }

    /// Is I/O bound?
    #[inline(always)]
    pub fn is_io_bound(&self) -> bool {
        self.voluntary_switches > self.involuntary_switches * 3
    }
}

// ============================================================================
// WAKE-UP CHAIN
// ============================================================================

/// A wakeup event
#[derive(Debug, Clone)]
pub struct WakeupEvent {
    /// Waker tid
    pub waker: u64,
    /// Wakee tid
    pub wakee: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Wakeup latency until scheduled (ns)
    pub latency_ns: u64,
}

/// Wakeup chain tracker
#[derive(Debug)]
pub struct WakeupChainTracker {
    /// Recent wakeups
    wakeups: VecDeque<WakeupEvent>,
    /// Max entries
    max_entries: usize,
    /// Per-pair frequency: (waker, wakee) hash -> count
    frequencies: LinearMap<u64, 64>,
}

impl WakeupChainTracker {
    pub fn new(max_entries: usize) -> Self {
        Self {
            wakeups: VecDeque::new(),
            max_entries,
            frequencies: LinearMap::new(),
        }
    }

    fn pair_key(waker: u64, wakee: u64) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        hash ^= waker;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= wakee;
        hash = hash.wrapping_mul(0x100000001b3);
        hash
    }

    /// Record wakeup
    #[inline]
    pub fn record(&mut self, event: WakeupEvent) {
        let key = Self::pair_key(event.waker, event.wakee);
        self.frequencies.add(key, 1);
        if self.wakeups.len() >= self.max_entries {
            self.wakeups.pop_front();
        }
        self.wakeups.push_back(event);
    }
}

// ============================================================================
// SCHED ANALYZER ENGINE
// ============================================================================

/// Scheduling stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppSchedProfileStats {
    /// Tracked threads
    pub tracked_threads: usize,
    /// Total context switches
    pub total_switches: u64,
    /// CPU-bound threads
    pub cpu_bound_count: usize,
    /// IO-bound threads
    pub io_bound_count: usize,
}

/// App scheduling profiler
pub struct AppSchedProfiler {
    /// Per-thread profiles
    profiles: BTreeMap<u64, ThreadSchedProfile>,
    /// Wakeup tracker
    pub wakeups: WakeupChainTracker,
    /// Stats
    stats: AppSchedProfileStats,
}

impl AppSchedProfiler {
    pub fn new() -> Self {
        Self {
            profiles: BTreeMap::new(),
            wakeups: WakeupChainTracker::new(10000),
            stats: AppSchedProfileStats::default(),
        }
    }

    /// Register thread
    #[inline(always)]
    pub fn register(&mut self, tid: u64) {
        self.profiles.insert(tid, ThreadSchedProfile::new(tid));
        self.update_stats();
    }

    /// Schedule in
    #[inline]
    pub fn on_sched_in(&mut self, tid: u64, cpu: u32, now: u64) {
        if let Some(profile) = self.profiles.get_mut(&tid) {
            profile.on_sched_in(cpu, now);
        }
    }

    /// Schedule out
    #[inline]
    pub fn on_sched_out(&mut self, tid: u64, reason: ContextSwitchReason, now: u64) {
        if let Some(profile) = self.profiles.get_mut(&tid) {
            profile.on_sched_out(reason, now);
        }
        self.update_stats();
    }

    /// Wakeup
    #[inline]
    pub fn on_wakeup(&mut self, tid: u64, now: u64) {
        if let Some(profile) = self.profiles.get_mut(&tid) {
            profile.on_wakeup(now);
        }
    }

    /// Remove thread
    #[inline(always)]
    pub fn remove(&mut self, tid: u64) {
        self.profiles.remove(&tid);
        self.update_stats();
    }

    /// Get profile
    #[inline(always)]
    pub fn profile(&self, tid: u64) -> Option<&ThreadSchedProfile> {
        self.profiles.get(&tid)
    }

    fn update_stats(&mut self) {
        self.stats.tracked_threads = self.profiles.len();
        self.stats.total_switches = self
            .profiles
            .values()
            .map(|p| p.voluntary_switches + p.involuntary_switches)
            .sum();
        self.stats.cpu_bound_count = self.profiles.values().filter(|p| p.is_cpu_bound()).count();
        self.stats.io_bound_count = self.profiles.values().filter(|p| p.is_io_bound()).count();
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &AppSchedProfileStats {
        &self.stats
    }
}
