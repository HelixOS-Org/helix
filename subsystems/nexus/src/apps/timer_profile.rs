//! # App Timer Profiler
//!
//! Timer and timeout profiling per application:
//! - High-resolution timer usage analysis
//! - Periodic timer pattern detection
//! - Timer slack and coalescing opportunities
//! - Timeout distribution tracking
//! - Timer wheel efficiency analysis

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// TIMER TYPES
// ============================================================================

/// Timer type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerType {
    /// One-shot timer
    OneShot,
    /// Periodic/interval timer
    Periodic,
    /// POSIX timer
    Posix,
    /// High-resolution timer
    Hrtimer,
    /// Watchdog timer
    Watchdog,
    /// Deadline timer
    Deadline,
}

/// Timer precision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerPrecision {
    /// Nanosecond precision
    Nanosecond,
    /// Microsecond precision
    Microsecond,
    /// Millisecond precision
    Millisecond,
    /// Second precision
    Second,
}

/// Timer state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerState {
    /// Armed
    Armed,
    /// Fired
    Fired,
    /// Cancelled
    Cancelled,
    /// Expired undelivered
    Expired,
}

// ============================================================================
// TIMER RECORD
// ============================================================================

/// Single timer record
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TimerRecord {
    /// Timer ID (FNV-1a)
    pub timer_id: u64,
    /// Timer type
    pub timer_type: TimerType,
    /// Precision
    pub precision: TimerPrecision,
    /// Requested interval (ns)
    pub interval_ns: u64,
    /// Actual fire count
    pub fire_count: u64,
    /// Missed fires
    pub missed_fires: u64,
    /// Total latency (ns) from expiry to handler
    pub total_latency_ns: u64,
    /// Max latency
    pub max_latency_ns: u64,
    /// State
    pub state: TimerState,
    /// Slack (ns)
    pub slack_ns: u64,
    /// Last fire (ns)
    pub last_fire_ns: u64,
}

impl TimerRecord {
    pub fn new(
        timer_id: u64,
        timer_type: TimerType,
        precision: TimerPrecision,
        interval_ns: u64,
    ) -> Self {
        Self {
            timer_id,
            timer_type,
            precision,
            interval_ns,
            fire_count: 0,
            missed_fires: 0,
            total_latency_ns: 0,
            max_latency_ns: 0,
            state: TimerState::Armed,
            slack_ns: 0,
            last_fire_ns: 0,
        }
    }

    /// Record fire event
    #[inline]
    pub fn record_fire(&mut self, latency_ns: u64, now: u64) {
        self.fire_count += 1;
        self.total_latency_ns += latency_ns;
        if latency_ns > self.max_latency_ns {
            self.max_latency_ns = latency_ns;
        }
        self.last_fire_ns = now;
        self.state = TimerState::Fired;
    }

    /// Record missed fire
    #[inline(always)]
    pub fn record_missed(&mut self) {
        self.missed_fires += 1;
    }

    /// Average latency
    #[inline]
    pub fn avg_latency_ns(&self) -> f64 {
        if self.fire_count == 0 {
            return 0.0;
        }
        self.total_latency_ns as f64 / self.fire_count as f64
    }

    /// Miss rate
    #[inline]
    pub fn miss_rate(&self) -> f64 {
        let total = self.fire_count + self.missed_fires;
        if total == 0 {
            return 0.0;
        }
        self.missed_fires as f64 / total as f64
    }

    /// Is coalescable (slack > 1ms)
    #[inline(always)]
    pub fn is_coalescable(&self) -> bool {
        self.slack_ns > 1_000_000
    }
}

// ============================================================================
// COALESCING OPPORTUNITY
// ============================================================================

/// Timer coalescing group
#[derive(Debug, Clone)]
pub struct CoalesceGroup {
    /// Group ID
    pub group_id: u64,
    /// Timer IDs in this group
    pub timer_ids: Vec<u64>,
    /// Common interval (ns)
    pub common_interval_ns: u64,
    /// Max drift tolerated
    pub max_drift_ns: u64,
    /// Estimated power savings (%)
    pub power_savings_pct: f64,
}

// ============================================================================
// PER-PROCESS TIMERS
// ============================================================================

/// Per-process timer tracking
#[derive(Debug)]
#[repr(align(64))]
pub struct ProcessTimerProfile {
    /// PID
    pub pid: u64,
    /// Active timers
    timers: BTreeMap<u64, TimerRecord>,
    /// Total timers created
    pub total_created: u64,
    /// Total timers cancelled
    pub total_cancelled: u64,
    /// Wakeups from timers
    pub wakeup_count: u64,
    /// Highest frequency timer interval
    pub min_interval_ns: u64,
}

impl ProcessTimerProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            timers: BTreeMap::new(),
            total_created: 0,
            total_cancelled: 0,
            wakeup_count: 0,
            min_interval_ns: u64::MAX,
        }
    }

    /// Create timer
    #[inline]
    pub fn create_timer(
        &mut self,
        timer_id: u64,
        timer_type: TimerType,
        precision: TimerPrecision,
        interval_ns: u64,
    ) -> &mut TimerRecord {
        self.total_created += 1;
        if interval_ns < self.min_interval_ns && interval_ns > 0 {
            self.min_interval_ns = interval_ns;
        }
        self.timers
            .entry(timer_id)
            .or_insert_with(|| TimerRecord::new(timer_id, timer_type, precision, interval_ns))
    }

    /// Cancel timer
    #[inline]
    pub fn cancel_timer(&mut self, timer_id: u64) {
        if let Some(t) = self.timers.get_mut(&timer_id) {
            t.state = TimerState::Cancelled;
            self.total_cancelled += 1;
        }
    }

    /// Fire timer
    #[inline]
    pub fn fire_timer(&mut self, timer_id: u64, latency_ns: u64, now: u64) {
        if let Some(t) = self.timers.get_mut(&timer_id) {
            t.record_fire(latency_ns, now);
            self.wakeup_count += 1;
        }
    }

    /// Timer rate (timers/sec)
    #[inline]
    pub fn timer_rate(&self, elapsed_ns: u64) -> f64 {
        if elapsed_ns == 0 {
            return 0.0;
        }
        self.wakeup_count as f64 / (elapsed_ns as f64 / 1_000_000_000.0)
    }

    /// Find coalescing opportunities
    pub fn find_coalesce_opportunities(&self) -> Vec<CoalesceGroup> {
        let mut groups: Vec<CoalesceGroup> = Vec::new();
        let periodic: Vec<&TimerRecord> = self
            .timers
            .values()
            .filter(|t| t.timer_type == TimerType::Periodic && t.state == TimerState::Armed)
            .collect();

        // Group timers with similar intervals (within 10%)
        for timer in &periodic {
            let mut found = false;
            for group in &mut groups {
                let ratio = timer.interval_ns as f64 / group.common_interval_ns as f64;
                if ratio > 0.9 && ratio < 1.1 {
                    group.timer_ids.push(timer.timer_id);
                    found = true;
                    break;
                }
            }
            if !found && timer.interval_ns > 0 {
                groups.push(CoalesceGroup {
                    group_id: timer.timer_id,
                    timer_ids: alloc::vec![timer.timer_id],
                    common_interval_ns: timer.interval_ns,
                    max_drift_ns: timer.interval_ns / 10,
                    power_savings_pct: 0.0,
                });
            }
        }

        // Estimate savings for multi-timer groups
        for group in &mut groups {
            if group.timer_ids.len() > 1 {
                // Coalescing N timers saves ~(N-1)/N wakeups
                let n = group.timer_ids.len() as f64;
                group.power_savings_pct = (n - 1.0) / n * 100.0;
            }
        }

        groups.retain(|g| g.timer_ids.len() > 1);
        groups
    }

    /// Count active timers
    #[inline]
    pub fn active_count(&self) -> usize {
        self.timers
            .values()
            .filter(|t| t.state == TimerState::Armed)
            .count()
    }
}

// ============================================================================
// TIMER WHEEL PROFILING
// ============================================================================

/// Timer wheel level stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct WheelLevelStats {
    /// Level number
    pub level: usize,
    /// Total slots
    pub total_slots: usize,
    /// Occupied slots
    pub occupied_slots: usize,
    /// Total timers in level
    pub timer_count: u64,
    /// Cascade events
    pub cascades: u64,
}

impl WheelLevelStats {
    #[inline]
    pub fn utilization(&self) -> f64 {
        if self.total_slots == 0 {
            return 0.0;
        }
        self.occupied_slots as f64 / self.total_slots as f64
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Timer profiler stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppTimerProfilerStats {
    /// Tracked processes
    pub tracked_processes: usize,
    /// Total active timers
    pub total_active_timers: usize,
    /// High-frequency timer processes
    pub high_freq_processes: usize,
    /// Coalescable timer groups
    pub coalescable_groups: usize,
}

/// App timer profiler
#[repr(align(64))]
pub struct AppTimerProfiler {
    /// Per-process profiles
    processes: BTreeMap<u64, ProcessTimerProfile>,
    /// Wheel stats
    pub wheel_levels: Vec<WheelLevelStats>,
    /// Stats
    stats: AppTimerProfilerStats,
}

impl AppTimerProfiler {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            wheel_levels: Vec::new(),
            stats: AppTimerProfilerStats::default(),
        }
    }

    /// Get/create process
    #[inline]
    pub fn process(&mut self, pid: u64) -> &mut ProcessTimerProfile {
        self.processes
            .entry(pid)
            .or_insert_with(|| ProcessTimerProfile::new(pid))
    }

    /// Create timer
    #[inline]
    pub fn create_timer(
        &mut self,
        pid: u64,
        timer_id: u64,
        timer_type: TimerType,
        precision: TimerPrecision,
        interval_ns: u64,
    ) {
        let proc = self
            .processes
            .entry(pid)
            .or_insert_with(|| ProcessTimerProfile::new(pid));
        proc.create_timer(timer_id, timer_type, precision, interval_ns);
        self.update_stats();
    }

    /// Fire timer
    #[inline]
    pub fn fire_timer(&mut self, pid: u64, timer_id: u64, latency_ns: u64, now: u64) {
        if let Some(proc) = self.processes.get_mut(&pid) {
            proc.fire_timer(timer_id, latency_ns, now);
        }
    }

    /// Remove process
    #[inline(always)]
    pub fn remove_process(&mut self, pid: u64) {
        self.processes.remove(&pid);
        self.update_stats();
    }

    /// Find all coalescing opportunities
    #[inline]
    pub fn find_all_coalesce(&self) -> Vec<(u64, Vec<CoalesceGroup>)> {
        let mut result = Vec::new();
        for (pid, proc) in &self.processes {
            let groups = proc.find_coalesce_opportunities();
            if !groups.is_empty() {
                result.push((*pid, groups));
            }
        }
        result
    }

    fn update_stats(&mut self) {
        self.stats.tracked_processes = self.processes.len();
        self.stats.total_active_timers = self.processes.values().map(|p| p.active_count()).sum();
        self.stats.high_freq_processes = self.processes.values()
            .filter(|p| p.min_interval_ns < 1_000_000) // < 1ms
            .count();
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &AppTimerProfilerStats {
        &self.stats
    }
}
