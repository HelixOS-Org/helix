//! # Apps Wait Tracker
//!
//! Wait/waitpid tracking:
//! - wait4/waitpid/waitid operation tracking
//! - Zombie process detection
//! - Orphan process tracking
//! - Wait latency profiling
//! - Reaping pattern analysis
//! - SIGCHLD correlation

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Wait variant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitVariant {
    Wait,
    Waitpid,
    Wait4,
    Waitid,
}

/// Wait options (WNOHANG, etc.)
#[derive(Debug, Clone, Copy)]
pub struct WaitOptions {
    pub bits: u32,
}

impl WaitOptions {
    pub const WNOHANG: u32 = 1;
    pub const WUNTRACED: u32 = 2;
    pub const WCONTINUED: u32 = 8;
    pub const WEXITED: u32 = 4;
    pub const WSTOPPED: u32 = 2;
    pub const WNOWAIT: u32 = 0x01000000;

    pub fn new(bits: u32) -> Self { Self { bits } }
    pub fn is_nohang(&self) -> bool { self.bits & Self::WNOHANG != 0 }
    pub fn is_untraced(&self) -> bool { self.bits & Self::WUNTRACED != 0 }
}

/// Child exit status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildStatus {
    Exited(u8),
    Signaled(u8),
    Stopped(u8),
    Continued,
    Unknown,
}

impl ChildStatus {
    pub fn is_abnormal(&self) -> bool {
        matches!(self, Self::Signaled(_))
    }
    pub fn exit_code(&self) -> Option<u8> {
        match self {
            Self::Exited(c) => Some(*c),
            _ => None,
        }
    }
}

/// Wait event record
#[derive(Debug, Clone)]
pub struct WaitEvent {
    pub waiter_pid: u64,
    pub waited_pid: u64,
    pub variant: WaitVariant,
    pub options: WaitOptions,
    pub status: ChildStatus,
    pub timestamp: u64,
    pub wait_latency_ns: u64,
    pub was_nohang: bool,
    pub got_result: bool,
}

/// Zombie process entry
#[derive(Debug, Clone)]
pub struct ZombieEntry {
    pub pid: u64,
    pub parent_pid: u64,
    pub exit_status: ChildStatus,
    pub exit_ts: u64,
    pub zombie_duration_ns: u64,
}

impl ZombieEntry {
    pub fn new(pid: u64, parent: u64, status: ChildStatus, ts: u64) -> Self {
        Self { pid, parent_pid: parent, exit_status: status, exit_ts: ts, zombie_duration_ns: 0 }
    }

    pub fn update_duration(&mut self, now: u64) {
        self.zombie_duration_ns = now.saturating_sub(self.exit_ts);
    }

    pub fn is_long_zombie(&self, threshold_ns: u64) -> bool {
        self.zombie_duration_ns > threshold_ns
    }
}

/// Per-process wait pattern
#[derive(Debug, Clone)]
pub struct WaitPattern {
    pub pid: u64,
    pub total_waits: u64,
    pub nohang_waits: u64,
    pub blocking_waits: u64,
    pub total_wait_time_ns: u64,
    pub children_reaped: u64,
    pub abnormal_exits: u64,
    pub last_wait_ts: u64,
    pub avg_reap_latency_ns: u64,
}

impl WaitPattern {
    pub fn new(pid: u64) -> Self {
        Self {
            pid, total_waits: 0, nohang_waits: 0, blocking_waits: 0,
            total_wait_time_ns: 0, children_reaped: 0, abnormal_exits: 0,
            last_wait_ts: 0, avg_reap_latency_ns: 0,
        }
    }

    pub fn record_wait(&mut self, nohang: bool, latency_ns: u64, got_result: bool, status: ChildStatus, ts: u64) {
        self.total_waits += 1;
        self.last_wait_ts = ts;
        if nohang { self.nohang_waits += 1; }
        else { self.blocking_waits += 1; }
        if got_result {
            self.children_reaped += 1;
            self.total_wait_time_ns += latency_ns;
            if self.children_reaped > 0 {
                self.avg_reap_latency_ns = self.total_wait_time_ns / self.children_reaped;
            }
            if status.is_abnormal() { self.abnormal_exits += 1; }
        }
    }

    pub fn nohang_ratio(&self) -> f64 {
        if self.total_waits == 0 { return 0.0; }
        self.nohang_waits as f64 / self.total_waits as f64
    }

    pub fn abnormal_ratio(&self) -> f64 {
        if self.children_reaped == 0 { return 0.0; }
        self.abnormal_exits as f64 / self.children_reaped as f64
    }
}

/// Wait tracker stats
#[derive(Debug, Clone, Default)]
pub struct WaitTrackerStats {
    pub tracked_processes: usize,
    pub total_wait_events: u64,
    pub current_zombies: usize,
    pub long_zombies: usize,
    pub orphan_count: usize,
    pub total_reaped: u64,
    pub total_abnormal: u64,
    pub avg_zombie_duration_ns: u64,
}

/// Apps wait tracker
pub struct AppsWaitTracker {
    patterns: BTreeMap<u64, WaitPattern>,
    zombies: BTreeMap<u64, ZombieEntry>,
    orphans: Vec<u64>,
    events: Vec<WaitEvent>,
    max_events: usize,
    zombie_threshold_ns: u64,
    stats: WaitTrackerStats,
}

impl AppsWaitTracker {
    pub fn new() -> Self {
        Self {
            patterns: BTreeMap::new(), zombies: BTreeMap::new(),
            orphans: Vec::new(), events: Vec::new(),
            max_events: 512, zombie_threshold_ns: 10_000_000_000,
            stats: WaitTrackerStats::default(),
        }
    }

    pub fn record_child_exit(&mut self, pid: u64, parent: u64, status: ChildStatus, ts: u64) {
        self.zombies.insert(pid, ZombieEntry::new(pid, parent, status, ts));
    }

    pub fn record_wait(&mut self, waiter: u64, waited: u64, variant: WaitVariant, options: WaitOptions, status: ChildStatus, latency_ns: u64, got_result: bool, ts: u64) {
        let pattern = self.patterns.entry(waiter).or_insert_with(|| WaitPattern::new(waiter));
        pattern.record_wait(options.is_nohang(), latency_ns, got_result, status, ts);

        if got_result { self.zombies.remove(&waited); }

        self.events.push(WaitEvent {
            waiter_pid: waiter, waited_pid: waited, variant, options, status,
            timestamp: ts, wait_latency_ns: latency_ns, was_nohang: options.is_nohang(),
            got_result,
        });
        if self.events.len() > self.max_events { self.events.remove(0); }
    }

    pub fn record_orphan(&mut self, pid: u64) {
        if !self.orphans.contains(&pid) { self.orphans.push(pid); }
    }

    pub fn record_reparent(&mut self, pid: u64, new_parent: u64) {
        if let Some(z) = self.zombies.get_mut(&pid) { z.parent_pid = new_parent; }
        self.orphans.retain(|&o| o != pid);
    }

    pub fn process_exit(&mut self, pid: u64) {
        self.patterns.remove(&pid);
        self.orphans.retain(|&o| o != pid);
    }

    pub fn recompute(&mut self, now: u64) {
        for z in self.zombies.values_mut() { z.update_duration(now); }
        self.stats.tracked_processes = self.patterns.len();
        self.stats.total_wait_events = self.patterns.values().map(|p| p.total_waits).sum();
        self.stats.current_zombies = self.zombies.len();
        self.stats.long_zombies = self.zombies.values().filter(|z| z.is_long_zombie(self.zombie_threshold_ns)).count();
        self.stats.orphan_count = self.orphans.len();
        self.stats.total_reaped = self.patterns.values().map(|p| p.children_reaped).sum();
        self.stats.total_abnormal = self.patterns.values().map(|p| p.abnormal_exits).sum();
        if !self.zombies.is_empty() {
            let total: u64 = self.zombies.values().map(|z| z.zombie_duration_ns).sum();
            self.stats.avg_zombie_duration_ns = total / self.zombies.len() as u64;
        }
    }

    pub fn pattern(&self, pid: u64) -> Option<&WaitPattern> { self.patterns.get(&pid) }
    pub fn zombie(&self, pid: u64) -> Option<&ZombieEntry> { self.zombies.get(&pid) }
    pub fn stats(&self) -> &WaitTrackerStats { &self.stats }
}

// ============================================================================
// Merged from wait_v2_tracker
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitV2Target {
    AnyChild,
    SpecificPid(u64),
    ProcessGroup(u64),
    AnyInGroup,
}

/// Wait status type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitV2Status {
    Exited(i32),
    Signaled(u32),
    Stopped(u32),
    Continued,
    StillRunning,
}

/// Wait options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitV2Option {
    NoHang,
    Untraced,
    Continued,
    Clone,
    NoWait,
    All,
}

/// A wait record
#[derive(Debug, Clone)]
pub struct WaitV2Record {
    pub parent_pid: u64,
    pub child_pid: u64,
    pub target: WaitV2Target,
    pub status: WaitV2Status,
    pub rusage_utime_us: u64,
    pub rusage_stime_us: u64,
    pub rusage_maxrss: u64,
    pub waited_ticks: u64,
    pub tick: u64,
}

/// Per-process wait state
#[derive(Debug, Clone)]
pub struct ProcessWaitV2State {
    pub pid: u64,
    pub children: Vec<u64>,
    pub zombies: Vec<u64>,
    pub wait_calls: u64,
    pub reaped: u64,
    pub no_hang_empty: u64,
}

impl ProcessWaitV2State {
    pub fn new(pid: u64) -> Self {
        Self {
            pid, children: Vec::new(), zombies: Vec::new(),
            wait_calls: 0, reaped: 0, no_hang_empty: 0,
        }
    }

    pub fn add_child(&mut self, child: u64) {
        self.children.push(child);
    }

    pub fn child_exited(&mut self, child: u64) {
        self.children.retain(|c| *c != child);
        self.zombies.push(child);
    }

    pub fn reap(&mut self) -> Option<u64> {
        if let Some(z) = self.zombies.pop() {
            self.reaped += 1;
            Some(z)
        } else { None }
    }
}

/// Statistics for wait V2 tracker
#[derive(Debug, Clone)]
pub struct WaitV2TrackerStats {
    pub total_wait_calls: u64,
    pub waitpid_calls: u64,
    pub waitid_calls: u64,
    pub wait4_calls: u64,
    pub children_reaped: u64,
    pub no_hang_returns: u64,
    pub zombies_accumulated: u64,
    pub orphans_reparented: u64,
}

/// Main wait V2 tracker manager
#[derive(Debug)]
pub struct AppWaitV2Tracker {
    processes: BTreeMap<u64, ProcessWaitV2State>,
    history: Vec<WaitV2Record>,
    max_history: usize,
    stats: WaitV2TrackerStats,
}

impl AppWaitV2Tracker {
    pub fn new(max_history: usize) -> Self {
        Self {
            processes: BTreeMap::new(),
            history: Vec::new(),
            max_history,
            stats: WaitV2TrackerStats {
                total_wait_calls: 0, waitpid_calls: 0,
                waitid_calls: 0, wait4_calls: 0,
                children_reaped: 0, no_hang_returns: 0,
                zombies_accumulated: 0, orphans_reparented: 0,
            },
        }
    }

    pub fn register_process(&mut self, pid: u64) {
        self.processes.insert(pid, ProcessWaitV2State::new(pid));
    }

    pub fn add_child(&mut self, parent: u64, child: u64) {
        if let Some(proc) = self.processes.get_mut(&parent) {
            proc.add_child(child);
        }
    }

    pub fn child_exit(&mut self, parent: u64, child: u64) {
        if let Some(proc) = self.processes.get_mut(&parent) {
            proc.child_exited(child);
            self.stats.zombies_accumulated += 1;
        }
    }

    pub fn wait(&mut self, pid: u64, target: WaitV2Target, tick: u64) -> Option<u64> {
        self.stats.total_wait_calls += 1;
        self.stats.waitpid_calls += 1;
        if let Some(proc) = self.processes.get_mut(&pid) {
            proc.wait_calls += 1;
            if let Some(child) = proc.reap() {
                self.stats.children_reaped += 1;
                let record = WaitV2Record {
                    parent_pid: pid, child_pid: child,
                    target, status: WaitV2Status::Exited(0),
                    rusage_utime_us: 0, rusage_stime_us: 0,
                    rusage_maxrss: 0, waited_ticks: 0, tick,
                };
                if self.history.len() >= self.max_history {
                    self.history.remove(0);
                }
                self.history.push(record);
                return Some(child);
            }
            proc.no_hang_empty += 1;
            self.stats.no_hang_returns += 1;
        }
        None
    }

    pub fn stats(&self) -> &WaitV2TrackerStats {
        &self.stats
    }
}
