//! # Holistic OOM Killer
//!
//! Out-of-memory killer with holistic awareness:
//! - Per-process OOM score calculation (RSS, oom_adj, nice, age)
//! - Cgroup-aware OOM handling
//! - Staged killing: reclaim → soft kill → hard kill
//! - Kill history and cooldown tracking
//! - Memory pressure integration
//! - Victim selection policy

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// OOM trigger reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OomTrigger {
    GlobalPressure,
    CgroupLimit,
    ZoneExhausted,
    SwapFull,
    HugePageDepletion,
    NurmaZone,
}

/// Kill stage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillStage {
    SoftReclaim,
    SigTerm,
    SigKill,
    CgroupFreeze,
    ForceReap,
}

/// OOM victim candidate
#[derive(Debug, Clone)]
pub struct OomCandidate {
    pub pid: u64,
    pub rss_pages: u64,
    pub swap_pages: u64,
    pub oom_adj: i16,
    pub nice: i8,
    pub age_ns: u64,
    pub cgroup_id: u64,
    pub oom_score: u64,
    pub protected: bool,
    pub essential: bool,
}

impl OomCandidate {
    pub fn new(pid: u64, rss: u64, swap: u64, adj: i16, nice: i8, age: u64, cgroup: u64) -> Self {
        let mut score = rss + swap / 2;
        // Adjust by oom_adj (-1000 to 1000 range)
        if adj > 0 { score = score.saturating_mul(adj as u64 + 1000) / 1000; }
        else if adj < 0 { score = score.saturating_mul(1000u64.saturating_sub((-adj) as u64)) / 1000; }
        // Older processes slightly less likely to be killed
        if age > 600_000_000_000 { score = score.saturating_mul(95) / 100; }
        Self { pid, rss_pages: rss, swap_pages: swap, oom_adj: adj, nice, age_ns: age, cgroup_id: cgroup, oom_score: score, protected: adj <= -998, essential: false }
    }
}

/// Kill record
#[derive(Debug, Clone)]
pub struct OomKillRecord {
    pub pid: u64,
    pub score: u64,
    pub rss_freed: u64,
    pub trigger: OomTrigger,
    pub stage: KillStage,
    pub ts: u64,
    pub cgroup_id: u64,
    pub latency_ns: u64,
}

/// Cgroup OOM state
#[derive(Debug, Clone)]
pub struct CgroupOomState {
    pub cgroup_id: u64,
    pub limit_pages: u64,
    pub usage_pages: u64,
    pub oom_count: u32,
    pub last_oom_ts: u64,
    pub cooldown_ns: u64,
}

impl CgroupOomState {
    pub fn new(id: u64, limit: u64) -> Self {
        Self { cgroup_id: id, limit_pages: limit, usage_pages: 0, oom_count: 0, last_oom_ts: 0, cooldown_ns: 5_000_000_000 }
    }

    pub fn is_over_limit(&self) -> bool { self.usage_pages > self.limit_pages }
    pub fn in_cooldown(&self, now: u64) -> bool { now.saturating_sub(self.last_oom_ts) < self.cooldown_ns }
}

/// OOM policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OomPolicy {
    KillLargest,
    KillNewest,
    KillLowestPriority,
    KillByCgroup,
    ReclaimFirst,
}

/// OOM stats
#[derive(Debug, Clone, Default)]
pub struct OomStats {
    pub total_kills: u64,
    pub pages_freed: u64,
    pub triggers_global: u64,
    pub triggers_cgroup: u64,
    pub avg_kill_latency_ns: u64,
    pub protected_saved: u64,
}

/// Holistic OOM killer
pub struct HolisticOomKiller {
    candidates: BTreeMap<u64, OomCandidate>,
    cgroups: BTreeMap<u64, CgroupOomState>,
    history: Vec<OomKillRecord>,
    stats: OomStats,
    policy: OomPolicy,
    min_free_pages: u64,
    kill_cooldown_ns: u64,
    last_kill_ts: u64,
}

impl HolisticOomKiller {
    pub fn new(policy: OomPolicy, min_free: u64, cooldown: u64) -> Self {
        Self {
            candidates: BTreeMap::new(), cgroups: BTreeMap::new(),
            history: Vec::new(), stats: OomStats::default(),
            policy, min_free_pages: min_free, kill_cooldown_ns: cooldown,
            last_kill_ts: 0,
        }
    }

    pub fn register(&mut self, c: OomCandidate) { self.candidates.insert(c.pid, c); }
    pub fn unregister(&mut self, pid: u64) { self.candidates.remove(&pid); }

    pub fn add_cgroup(&mut self, id: u64, limit: u64) { self.cgroups.insert(id, CgroupOomState::new(id, limit)); }
    pub fn update_cgroup_usage(&mut self, id: u64, usage: u64) { if let Some(c) = self.cgroups.get_mut(&id) { c.usage_pages = usage; } }

    pub fn select_victim(&self, trigger: OomTrigger) -> Option<u64> {
        let victims: Vec<&OomCandidate> = self.candidates.values()
            .filter(|c| !c.protected && !c.essential)
            .collect();
        if victims.is_empty() { return None; }
        match self.policy {
            OomPolicy::KillLargest | OomPolicy::ReclaimFirst => victims.iter().max_by_key(|c| c.oom_score).map(|c| c.pid),
            OomPolicy::KillNewest => victims.iter().min_by_key(|c| c.age_ns).map(|c| c.pid),
            OomPolicy::KillLowestPriority => victims.iter().max_by_key(|c| c.nice as i64 + 128).map(|c| c.pid),
            OomPolicy::KillByCgroup => {
                if let OomTrigger::CgroupLimit = trigger {
                    let over: Vec<u64> = self.cgroups.values().filter(|c| c.is_over_limit()).map(|c| c.cgroup_id).collect();
                    victims.iter().filter(|c| over.contains(&c.cgroup_id)).max_by_key(|c| c.oom_score).map(|c| c.pid)
                } else { victims.iter().max_by_key(|c| c.oom_score).map(|c| c.pid) }
            }
        }
    }

    pub fn kill(&mut self, pid: u64, trigger: OomTrigger, stage: KillStage, ts: u64) -> Option<u64> {
        if ts.saturating_sub(self.last_kill_ts) < self.kill_cooldown_ns { return None; }
        let c = self.candidates.remove(&pid)?;
        let freed = c.rss_pages;
        self.history.push(OomKillRecord {
            pid, score: c.oom_score, rss_freed: freed, trigger, stage, ts,
            cgroup_id: c.cgroup_id, latency_ns: 0,
        });
        self.stats.total_kills += 1;
        self.stats.pages_freed += freed;
        self.last_kill_ts = ts;
        match trigger {
            OomTrigger::GlobalPressure | OomTrigger::ZoneExhausted | OomTrigger::SwapFull => self.stats.triggers_global += 1,
            OomTrigger::CgroupLimit => self.stats.triggers_cgroup += 1,
            _ => {}
        }
        if let Some(cg) = self.cgroups.get_mut(&c.cgroup_id) {
            cg.oom_count += 1;
            cg.last_oom_ts = ts;
            cg.usage_pages = cg.usage_pages.saturating_sub(freed);
        }
        Some(freed)
    }

    pub fn auto_kill(&mut self, trigger: OomTrigger, ts: u64) -> Option<u64> {
        let victim = self.select_victim(trigger)?;
        self.kill(victim, trigger, KillStage::SigKill, ts)
    }

    pub fn cgroup_oom(&mut self, cgroup_id: u64, ts: u64) -> Option<u64> {
        if let Some(cg) = self.cgroups.get(&cgroup_id) {
            if cg.in_cooldown(ts) { return None; }
        }
        let victim = self.candidates.values()
            .filter(|c| c.cgroup_id == cgroup_id && !c.protected)
            .max_by_key(|c| c.oom_score)
            .map(|c| c.pid)?;
        self.kill(victim, OomTrigger::CgroupLimit, KillStage::SigKill, ts)
    }

    pub fn recompute(&mut self) {
        if !self.history.is_empty() {
            let total: u64 = self.history.iter().map(|r| r.latency_ns).sum();
            self.stats.avg_kill_latency_ns = total / self.history.len() as u64;
        }
        self.stats.protected_saved = self.candidates.values().filter(|c| c.protected).count() as u64;
    }

    pub fn candidate(&self, pid: u64) -> Option<&OomCandidate> { self.candidates.get(&pid) }
    pub fn stats(&self) -> &OomStats { &self.stats }
    pub fn history(&self) -> &[OomKillRecord] { &self.history }
}

// ============================================================================
// Merged from oom_killer_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OomPolicy {
    Global,
    CgroupLocal,
    Memcg,
    Priority,
    Custom,
}

/// OOM victim selection
#[derive(Debug, Clone)]
pub struct OomCandidate {
    pub pid: u64,
    pub uid: u32,
    pub oom_score: i32,
    pub oom_score_adj: i16,
    pub rss_pages: u64,
    pub swap_pages: u64,
    pub total_vm: u64,
    pub is_unkillable: bool,
    pub cgroup_id: u64,
}

impl OomCandidate {
    pub fn badness_score(&self) -> i64 {
        if self.is_unkillable { return -1000; }
        let points = self.rss_pages as i64 + self.swap_pages as i64;
        let adj = (points * self.oom_score_adj as i64) / 1000;
        (points + adj).max(1)
    }
}

/// OOM kill event
#[derive(Debug, Clone)]
pub struct OomKillEvent {
    pub id: u64,
    pub victim_pid: u64,
    pub victim_score: i64,
    pub freed_pages: u64,
    pub policy: OomPolicy,
    pub trigger_order: u32,
    pub timestamp: u64,
    pub duration_ns: u64,
    pub cgroup_id: u64,
}

/// Memory pressure level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemPressure {
    Low,
    Medium,
    Critical,
}

/// Stats
#[derive(Debug, Clone)]
pub struct OomKillerV2Stats {
    pub total_kills: u64,
    pub total_freed_pages: u64,
    pub global_kills: u64,
    pub cgroup_kills: u64,
    pub avg_response_ns: u64,
    pub current_pressure: u8,
}

/// Main OOM killer v2
pub struct HolisticOomKillerV2 {
    kills: Vec<OomKillEvent>,
    candidates: BTreeMap<u64, OomCandidate>,
    policy: OomPolicy,
    pressure: MemPressure,
    next_id: u64,
    max_events: usize,
}

impl HolisticOomKillerV2 {
    pub fn new() -> Self {
        Self { kills: Vec::new(), candidates: BTreeMap::new(), policy: OomPolicy::Global, pressure: MemPressure::Low, next_id: 1, max_events: 4096 }
    }

    pub fn register_process(&mut self, candidate: OomCandidate) { self.candidates.insert(candidate.pid, candidate); }
    pub fn unregister(&mut self, pid: u64) { self.candidates.remove(&pid); }
    pub fn set_policy(&mut self, policy: OomPolicy) { self.policy = policy; }
    pub fn set_pressure(&mut self, pressure: MemPressure) { self.pressure = pressure; }

    pub fn select_victim(&self) -> Option<u64> {
        self.candidates.values().filter(|c| !c.is_unkillable)
            .max_by_key(|c| c.badness_score()).map(|c| c.pid)
    }

    pub fn kill(&mut self, pid: u64, freed: u64, duration: u64, now: u64) {
        let id = self.next_id; self.next_id += 1;
        let score = self.candidates.get(&pid).map(|c| c.badness_score()).unwrap_or(0);
        let cg = self.candidates.get(&pid).map(|c| c.cgroup_id).unwrap_or(0);
        if self.kills.len() >= self.max_events { self.kills.drain(..self.max_events / 2); }
        self.kills.push(OomKillEvent { id, victim_pid: pid, victim_score: score, freed_pages: freed, policy: self.policy, trigger_order: 0, timestamp: now, duration_ns: duration, cgroup_id: cg });
        self.candidates.remove(&pid);
    }

    pub fn stats(&self) -> OomKillerV2Stats {
        let freed: u64 = self.kills.iter().map(|k| k.freed_pages).sum();
        let global = self.kills.iter().filter(|k| k.policy == OomPolicy::Global).count() as u64;
        let cgroup = self.kills.iter().filter(|k| matches!(k.policy, OomPolicy::CgroupLocal | OomPolicy::Memcg)).count() as u64;
        let durs: Vec<u64> = self.kills.iter().map(|k| k.duration_ns).collect();
        let avg = if durs.is_empty() { 0 } else { durs.iter().sum::<u64>() / durs.len() as u64 };
        OomKillerV2Stats { total_kills: self.kills.len() as u64, total_freed_pages: freed, global_kills: global, cgroup_kills: cgroup, avg_response_ns: avg, current_pressure: self.pressure as u8 }
    }
}
