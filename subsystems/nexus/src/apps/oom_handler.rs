//! # Apps OOM Handler
//!
//! Application-level OOM handling and prevention:
//! - Per-process OOM score calculation
//! - OOM score adjustment tracking
//! - Memory pressure notification
//! - Proactive memory reclaim triggers
//! - Kill candidate selection
//! - Post-OOM recovery coordination

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// OOM score factor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OomFactor {
    RssSize,
    SwapUsage,
    OomScoreAdj,
    CpuUsage,
    Age,
    Priority,
    CgroupLimit,
}

/// Memory pressure level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AppMemPressure {
    None,
    Low,
    Medium,
    Critical,
    Oom,
}

impl Default for AppMemPressure {
    fn default() -> Self { AppMemPressure::None }
}

/// OOM kill reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OomKillReason {
    SystemWide,
    CgroupLimit,
    MemoryFragmentation,
    SwapExhausted,
    NoPagesAvailable,
}

/// Per-process OOM state
#[derive(Debug, Clone)]
pub struct ProcessOomState {
    pub pid: u64,
    pub rss_bytes: u64,
    pub swap_bytes: u64,
    pub oom_score: u32,
    pub oom_score_adj: i16,
    pub oom_badness: u64,
    pub is_unkillable: bool,
    pub cgroup_id: u64,
    pub last_reclaim_ts: u64,
    pub reclaim_attempts: u32,
    pub voluntary_reclaim_bytes: u64,
}

impl ProcessOomState {
    pub fn new(pid: u64) -> Self {
        Self {
            pid, rss_bytes: 0, swap_bytes: 0, oom_score: 0, oom_score_adj: 0,
            oom_badness: 0, is_unkillable: false, cgroup_id: 0,
            last_reclaim_ts: 0, reclaim_attempts: 0, voluntary_reclaim_bytes: 0,
        }
    }

    pub fn compute_badness(&mut self, total_mem: u64) {
        if self.is_unkillable || self.oom_score_adj == -1000 { self.oom_badness = 0; return; }
        let points = if total_mem == 0 { 0 } else { (self.rss_bytes + self.swap_bytes) * 1000 / total_mem };
        let adj = self.oom_score_adj as i64;
        let adjusted = (points as i64 + adj).max(0) as u64;
        self.oom_badness = adjusted.min(1000);
        self.oom_score = self.oom_badness as u32;
    }

    pub fn total_mem(&self) -> u64 { self.rss_bytes + self.swap_bytes }
}

/// OOM kill record
#[derive(Debug, Clone)]
pub struct AppOomKillRecord {
    pub victim_pid: u64,
    pub rss_freed: u64,
    pub swap_freed: u64,
    pub oom_score: u32,
    pub reason: OomKillReason,
    pub ts: u64,
    pub cgroup_id: u64,
}

/// OOM event
#[derive(Debug, Clone)]
pub struct OomEvent {
    pub kind: OomEventKind,
    pub ts: u64,
    pub pressure: AppMemPressure,
    pub available_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OomEventKind {
    PressureChange,
    ReclaimStart,
    ReclaimComplete,
    KillTriggered,
    KillComplete,
    Recovery,
}

/// OOM handler stats
#[derive(Debug, Clone, Default)]
pub struct AppOomStats {
    pub tracked_processes: usize,
    pub current_pressure: AppMemPressure,
    pub total_kills: u64,
    pub total_reclaims: u64,
    pub bytes_reclaimed: u64,
    pub bytes_killed: u64,
    pub highest_oom_score: u32,
}

/// Apps OOM handler
pub struct AppsOomHandler {
    processes: BTreeMap<u64, ProcessOomState>,
    kill_history: Vec<AppOomKillRecord>,
    events: Vec<OomEvent>,
    total_memory: u64,
    pressure: AppMemPressure,
    stats: AppOomStats,
}

impl AppsOomHandler {
    pub fn new(total_mem: u64) -> Self {
        Self {
            processes: BTreeMap::new(), kill_history: Vec::new(), events: Vec::new(),
            total_memory: total_mem, pressure: AppMemPressure::None, stats: AppOomStats::default(),
        }
    }

    pub fn track(&mut self, pid: u64) { self.processes.entry(pid).or_insert_with(|| ProcessOomState::new(pid)); }
    pub fn untrack(&mut self, pid: u64) { self.processes.remove(&pid); }

    pub fn update(&mut self, pid: u64, rss: u64, swap: u64) {
        if let Some(p) = self.processes.get_mut(&pid) {
            p.rss_bytes = rss; p.swap_bytes = swap;
            p.compute_badness(self.total_memory);
        }
    }

    pub fn set_adj(&mut self, pid: u64, adj: i16) {
        if let Some(p) = self.processes.get_mut(&pid) {
            p.oom_score_adj = adj.max(-1000).min(1000);
            p.compute_badness(self.total_memory);
        }
    }

    pub fn set_unkillable(&mut self, pid: u64, unkillable: bool) {
        if let Some(p) = self.processes.get_mut(&pid) { p.is_unkillable = unkillable; }
    }

    pub fn set_pressure(&mut self, pressure: AppMemPressure, available: u64, ts: u64) {
        if pressure != self.pressure {
            self.pressure = pressure;
            self.events.push(OomEvent { kind: OomEventKind::PressureChange, ts, pressure, available_bytes: available });
        }
    }

    pub fn select_victim(&self) -> Option<u64> {
        self.processes.values()
            .filter(|p| !p.is_unkillable && p.oom_score_adj != -1000)
            .max_by_key(|p| p.oom_badness)
            .map(|p| p.pid)
    }

    pub fn select_victims(&self, count: usize) -> Vec<u64> {
        let mut candidates: Vec<&ProcessOomState> = self.processes.values()
            .filter(|p| !p.is_unkillable && p.oom_score_adj != -1000)
            .collect();
        candidates.sort_by(|a, b| b.oom_badness.cmp(&a.oom_badness));
        candidates.iter().take(count).map(|p| p.pid).collect()
    }

    pub fn record_kill(&mut self, pid: u64, reason: OomKillReason, ts: u64) {
        if let Some(p) = self.processes.get(&pid) {
            self.kill_history.push(AppOomKillRecord {
                victim_pid: pid, rss_freed: p.rss_bytes, swap_freed: p.swap_bytes,
                oom_score: p.oom_score, reason, ts, cgroup_id: p.cgroup_id,
            });
        }
        self.processes.remove(&pid);
        self.events.push(OomEvent { kind: OomEventKind::KillComplete, ts, pressure: self.pressure, available_bytes: 0 });
    }

    pub fn recompute(&mut self) {
        self.stats.tracked_processes = self.processes.len();
        self.stats.current_pressure = self.pressure;
        self.stats.total_kills = self.kill_history.len() as u64;
        self.stats.bytes_killed = self.kill_history.iter().map(|k| k.rss_freed + k.swap_freed).sum();
        self.stats.highest_oom_score = self.processes.values().map(|p| p.oom_score).max().unwrap_or(0);
        self.stats.total_reclaims = self.processes.values().map(|p| p.reclaim_attempts as u64).sum();
        self.stats.bytes_reclaimed = self.processes.values().map(|p| p.voluntary_reclaim_bytes).sum();
    }

    pub fn process(&self, pid: u64) -> Option<&ProcessOomState> { self.processes.get(&pid) }
    pub fn kill_history(&self) -> &[AppOomKillRecord] { &self.kill_history }
    pub fn stats(&self) -> &AppOomStats { &self.stats }
}
