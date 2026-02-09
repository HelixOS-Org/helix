// SPDX-License-Identifier: GPL-2.0
//! Holistic memcg_oom — memory cgroup OOM killer.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// OOM action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OomAction {
    Kill,
    OomScoreAdjust,
    Throttle,
    Reclaim,
    Panic,
}

/// OOM victim candidate
#[derive(Debug)]
pub struct OomVictim {
    pub pid: u64,
    pub oom_score: i32,
    pub oom_score_adj: i32,
    pub rss_bytes: u64,
    pub swap_bytes: u64,
    pub cgroup_id: u64,
    pub is_unkillable: bool,
}

impl OomVictim {
    #[inline(always)]
    pub fn effective_score(&self) -> i64 {
        (self.oom_score as i64 + self.oom_score_adj as i64).max(0)
    }
}

/// OOM event
#[derive(Debug)]
pub struct OomEvent {
    pub cgroup_id: u64,
    pub victim_pid: u64,
    pub action: OomAction,
    pub freed_bytes: u64,
    pub timestamp: u64,
}

/// Cgroup OOM state
#[derive(Debug)]
#[repr(align(64))]
pub struct CgroupOomState {
    pub cgroup_id: u64,
    pub oom_count: u64,
    pub last_oom_time: u64,
    pub oom_kill_disable: bool,
    pub total_killed: u64,
    pub total_freed_bytes: u64,
}

impl CgroupOomState {
    pub fn new(id: u64) -> Self {
        Self { cgroup_id: id, oom_count: 0, last_oom_time: 0, oom_kill_disable: false, total_killed: 0, total_freed_bytes: 0 }
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MemcgOomStats {
    pub total_cgroups: u32,
    pub total_oom_events: u64,
    pub total_killed: u64,
    pub total_freed_bytes: u64,
}

/// Main holistic memcg OOM
pub struct HolisticMemcgOom {
    cgroups: BTreeMap<u64, CgroupOomState>,
    events: Vec<OomEvent>,
}

impl HolisticMemcgOom {
    pub fn new() -> Self { Self { cgroups: BTreeMap::new(), events: Vec::new() } }

    #[inline(always)]
    pub fn register_cgroup(&mut self, id: u64) { self.cgroups.insert(id, CgroupOomState::new(id)); }

    #[inline(always)]
    pub fn select_victim(&self, candidates: &[OomVictim]) -> Option<u64> {
        candidates.iter().filter(|v| !v.is_unkillable).max_by_key(|v| v.effective_score()).map(|v| v.pid)
    }

    #[inline]
    pub fn oom_kill(&mut self, cgroup_id: u64, victim_pid: u64, freed: u64, now: u64) {
        if let Some(cg) = self.cgroups.get_mut(&cgroup_id) {
            cg.oom_count += 1; cg.last_oom_time = now; cg.total_killed += 1; cg.total_freed_bytes += freed;
        }
        self.events.push(OomEvent { cgroup_id, victim_pid, action: OomAction::Kill, freed_bytes: freed, timestamp: now });
    }

    #[inline]
    pub fn stats(&self) -> MemcgOomStats {
        let events: u64 = self.cgroups.values().map(|c| c.oom_count).sum();
        let killed: u64 = self.cgroups.values().map(|c| c.total_killed).sum();
        let freed: u64 = self.cgroups.values().map(|c| c.total_freed_bytes).sum();
        MemcgOomStats { total_cgroups: self.cgroups.len() as u32, total_oom_events: events, total_killed: killed, total_freed_bytes: freed }
    }
}
