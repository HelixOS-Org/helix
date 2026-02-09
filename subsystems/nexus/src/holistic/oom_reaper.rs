// SPDX-License-Identifier: GPL-2.0
//! Holistic oom_reaper — OOM reaper and victim selection for memory recovery.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

/// OOM policy for victim selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OomPolicy {
    /// Standard badness score
    Badness,
    /// Kill youngest first
    Youngest,
    /// Kill largest RSS first
    LargestRss,
    /// Kill by cgroup limit
    CgroupFirst,
    /// Panic instead of killing
    Panic,
    /// User-defined priority
    UserDefined,
}

/// OOM kill reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OomReason {
    SystemWide,
    CgroupLimit,
    MemcgOom,
    NoPagesAvail,
    HugepageExhaust,
    CompactionFail,
}

/// OOM victim state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VictimState {
    Selected,
    SignalSent,
    Reaped,
    Survived,
    Escaped,
}

/// Task info for OOM scoring
#[derive(Debug, Clone)]
pub struct OomTaskInfo {
    pub pid: u32,
    pub comm: String,
    pub rss_pages: u64,
    pub swap_pages: u64,
    pub oom_score: i32,
    pub oom_score_adj: i16,
    pub cgroup_id: u64,
    pub is_unkillable: bool,
    pub child_count: u32,
    pub total_vm_pages: u64,
    pub pgtable_bytes: u64,
    pub start_time: u64,
}

impl OomTaskInfo {
    #[inline]
    pub fn badness_score(&self) -> i64 {
        if self.is_unkillable { return 0; }
        let base = self.rss_pages as i64 + self.swap_pages as i64
            + (self.pgtable_bytes / 4096) as i64;
        let adj = self.oom_score_adj as i64;
        let adjusted = base + (base * adj) / 1000;
        if adjusted < 1 && !self.is_unkillable { 1 } else { adjusted.max(0) }
    }

    #[inline(always)]
    pub fn memory_footprint(&self) -> u64 {
        (self.rss_pages + self.swap_pages) * 4096
    }

    #[inline(always)]
    pub fn memory_footprint_mb(&self) -> f64 {
        self.memory_footprint() as f64 / (1024.0 * 1024.0)
    }
}

/// Record of an OOM kill event
#[derive(Debug, Clone)]
pub struct OomKillRecord {
    pub victim_pid: u32,
    pub victim_comm: String,
    pub badness: i64,
    pub reason: OomReason,
    pub state: VictimState,
    pub freed_pages: u64,
    pub time_to_reap_us: u64,
    pub timestamp: u64,
    pub cgroup_id: Option<u64>,
    pub node_id: Option<u32>,
}

impl OomKillRecord {
    #[inline(always)]
    pub fn was_effective(&self) -> bool {
        self.state == VictimState::Reaped && self.freed_pages > 0
    }

    #[inline(always)]
    pub fn freed_mb(&self) -> f64 {
        (self.freed_pages * 4096) as f64 / (1024.0 * 1024.0)
    }
}

/// Memory pressure levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemPressureLevel {
    None,
    Low,
    Medium,
    Critical,
    Oom,
}

/// Per-cgroup OOM state
#[derive(Debug)]
#[repr(align(64))]
pub struct CgroupOomState {
    pub cgroup_id: u64,
    pub limit_pages: u64,
    pub usage_pages: u64,
    pub oom_kill_count: u64,
    pub last_oom_timestamp: u64,
    pub oom_group_kill: bool,
}

impl CgroupOomState {
    #[inline(always)]
    pub fn usage_ratio(&self) -> f64 {
        if self.limit_pages == 0 { return 0.0; }
        self.usage_pages as f64 / self.limit_pages as f64
    }

    #[inline(always)]
    pub fn headroom_pages(&self) -> u64 {
        self.limit_pages.saturating_sub(self.usage_pages)
    }
}

/// OOM stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct OomReaperStats {
    pub total_kills: u64,
    pub total_pages_freed: u64,
    pub avg_reap_time_us: u64,
    pub system_oom_count: u64,
    pub cgroup_oom_count: u64,
    pub ineffective_kills: u64,
    pub panic_count: u64,
}

/// Main OOM reaper
pub struct HolisticOomReaper {
    policy: OomPolicy,
    tasks: BTreeMap<u32, OomTaskInfo>,
    cgroup_states: BTreeMap<u64, CgroupOomState>,
    kill_history: VecDeque<OomKillRecord>,
    max_history: usize,
    stats: OomReaperStats,
    pressure: MemPressureLevel,
    oom_score_adj_min: i16,
}

impl HolisticOomReaper {
    pub fn new(policy: OomPolicy) -> Self {
        Self {
            policy,
            tasks: BTreeMap::new(),
            cgroup_states: BTreeMap::new(),
            kill_history: VecDeque::new(),
            max_history: 1024,
            stats: OomReaperStats {
                total_kills: 0, total_pages_freed: 0, avg_reap_time_us: 0,
                system_oom_count: 0, cgroup_oom_count: 0,
                ineffective_kills: 0, panic_count: 0,
            },
            pressure: MemPressureLevel::None,
            oom_score_adj_min: -1000,
        }
    }

    #[inline(always)]
    pub fn update_task(&mut self, info: OomTaskInfo) {
        self.tasks.insert(info.pid, info);
    }

    #[inline(always)]
    pub fn remove_task(&mut self, pid: u32) {
        self.tasks.remove(&pid);
    }

    #[inline(always)]
    pub fn set_pressure(&mut self, level: MemPressureLevel) {
        self.pressure = level;
    }

    pub fn select_victim(&self, reason: OomReason) -> Option<u32> {
        match self.policy {
            OomPolicy::Panic => None,
            OomPolicy::LargestRss => {
                self.tasks.values()
                    .filter(|t| !t.is_unkillable && t.oom_score_adj > self.oom_score_adj_min)
                    .max_by_key(|t| t.rss_pages)
                    .map(|t| t.pid)
            }
            OomPolicy::Youngest => {
                self.tasks.values()
                    .filter(|t| !t.is_unkillable && t.oom_score_adj > self.oom_score_adj_min)
                    .max_by_key(|t| t.start_time)
                    .map(|t| t.pid)
            }
            OomPolicy::CgroupFirst => {
                if let OomReason::CgroupLimit | OomReason::MemcgOom = reason {
                    // find highest-usage cgroup first
                    if let Some(cg) = self.cgroup_states.values()
                        .max_by(|a, b| a.usage_ratio().partial_cmp(&b.usage_ratio())
                            .unwrap_or(core::cmp::Ordering::Equal))
                    {
                        return self.tasks.values()
                            .filter(|t| t.cgroup_id == cg.cgroup_id && !t.is_unkillable)
                            .max_by_key(|t| t.badness_score())
                            .map(|t| t.pid);
                    }
                }
                self.select_by_badness()
            }
            _ => self.select_by_badness(),
        }
    }

    fn select_by_badness(&self) -> Option<u32> {
        self.tasks.values()
            .filter(|t| !t.is_unkillable && t.oom_score_adj > self.oom_score_adj_min)
            .max_by_key(|t| t.badness_score())
            .map(|t| t.pid)
    }

    pub fn record_kill(&mut self, record: OomKillRecord) {
        self.stats.total_kills += 1;
        self.stats.total_pages_freed += record.freed_pages;
        if !record.was_effective() { self.stats.ineffective_kills += 1; }
        match record.reason {
            OomReason::SystemWide | OomReason::NoPagesAvail => self.stats.system_oom_count += 1,
            OomReason::CgroupLimit | OomReason::MemcgOom => self.stats.cgroup_oom_count += 1,
            _ => {}
        }
        let n = self.stats.total_kills;
        self.stats.avg_reap_time_us =
            ((self.stats.avg_reap_time_us * (n - 1)) + record.time_to_reap_us) / n;

        if let Some(cg_id) = record.cgroup_id {
            if let Some(cg) = self.cgroup_states.get_mut(&cg_id) {
                cg.oom_kill_count += 1;
                cg.last_oom_timestamp = record.timestamp;
            }
        }

        if self.kill_history.len() >= self.max_history {
            self.kill_history.pop_front();
        }
        self.kill_history.push_back(record);
    }

    #[inline]
    pub fn top_victims(&self, n: usize) -> Vec<(u32, i64)> {
        let mut v: Vec<_> = self.tasks.values()
            .filter(|t| !t.is_unkillable)
            .map(|t| (t.pid, t.badness_score()))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.truncate(n);
        v
    }

    #[inline]
    pub fn repeat_offenders(&self) -> Vec<(String, u64)> {
        let mut counts: BTreeMap<String, u64> = BTreeMap::new();
        for rec in &self.kill_history {
            *counts.entry(rec.victim_comm.clone()).or_insert(0) += 1;
        }
        let mut v: Vec<_> = counts.into_iter().filter(|(_, c)| *c > 1).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v
    }

    #[inline(always)]
    pub fn update_cgroup(&mut self, state: CgroupOomState) {
        self.cgroup_states.insert(state.cgroup_id, state);
    }

    #[inline(always)]
    pub fn pressure(&self) -> MemPressureLevel {
        self.pressure
    }

    #[inline(always)]
    pub fn stats(&self) -> &OomReaperStats {
        &self.stats
    }
}

// ============================================================================
// Merged from oom_reaper_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OomV2Reason {
    MemoryExhausted,
    CgroupLimit,
    MemcgOom,
    MempolicyBind,
    Compaction,
}

/// OOM v2 victim
#[derive(Debug)]
pub struct OomV2Victim {
    pub pid: u64,
    pub oom_score: i32,
    pub oom_score_adj: i32,
    pub rss_pages: u64,
    pub swap_pages: u64,
    pub reason: OomV2Reason,
    pub killed_at: u64,
    pub reaped: bool,
    pub freed_pages: u64,
}

impl OomV2Victim {
    pub fn new(pid: u64, score: i32, adj: i32, rss: u64, reason: OomV2Reason, now: u64) -> Self {
        Self { pid, oom_score: score, oom_score_adj: adj, rss_pages: rss, swap_pages: 0, reason, killed_at: now, reaped: false, freed_pages: 0 }
    }
}

/// OOM v2 process info
#[derive(Debug)]
pub struct OomV2ProcessInfo {
    pub pid: u64,
    pub oom_score_adj: i32,
    pub rss_pages: u64,
    pub swap_pages: u64,
    pub is_unkillable: bool,
}

impl OomV2ProcessInfo {
    #[inline]
    pub fn effective_score(&self) -> i32 {
        if self.is_unkillable { return -1000; }
        let base = (self.rss_pages as i32).min(1000);
        (base + self.oom_score_adj).max(-1000).min(1000)
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct OomReaperV2Stats {
    pub total_kills: u64,
    pub total_reaped: u64,
    pub total_freed_pages: u64,
    pub tracked_procs: u32,
}

/// Main holistic OOM reaper v2
pub struct HolisticOomReaperV2 {
    procs: BTreeMap<u64, OomV2ProcessInfo>,
    victims: Vec<OomV2Victim>,
}

impl HolisticOomReaperV2 {
    pub fn new() -> Self { Self { procs: BTreeMap::new(), victims: Vec::new() } }

    #[inline(always)]
    pub fn track(&mut self, pid: u64, adj: i32, rss: u64) {
        self.procs.insert(pid, OomV2ProcessInfo { pid, oom_score_adj: adj, rss_pages: rss, swap_pages: 0, is_unkillable: false });
    }

    #[inline]
    pub fn select_victim(&self, reason: OomV2Reason) -> Option<u64> {
        self.procs.values()
            .filter(|p| !p.is_unkillable && p.oom_score_adj > -1000)
            .max_by_key(|p| p.effective_score())
            .map(|p| p.pid)
    }

    #[inline]
    pub fn kill(&mut self, pid: u64, reason: OomV2Reason, now: u64) {
        if let Some(p) = self.procs.get(&pid) {
            self.victims.push(OomV2Victim::new(pid, p.effective_score(), p.oom_score_adj, p.rss_pages, reason, now));
        }
    }

    #[inline]
    pub fn reap(&mut self, pid: u64, freed: u64) {
        for v in &mut self.victims {
            if v.pid == pid && !v.reaped { v.reaped = true; v.freed_pages = freed; break; }
        }
        self.procs.remove(&pid);
    }

    #[inline]
    pub fn stats(&self) -> OomReaperV2Stats {
        let reaped = self.victims.iter().filter(|v| v.reaped).count() as u64;
        let freed: u64 = self.victims.iter().map(|v| v.freed_pages).sum();
        OomReaperV2Stats { total_kills: self.victims.len() as u64, total_reaped: reaped, total_freed_pages: freed, tracked_procs: self.procs.len() as u32 }
    }
}
