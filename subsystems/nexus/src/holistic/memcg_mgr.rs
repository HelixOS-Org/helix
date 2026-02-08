//! # Holistic Memory Cgroup Manager
//!
//! Memory cgroup (memcg) controller with holistic awareness:
//! - Per-cgroup memory limits (hard/soft/swap)
//! - Hierarchical accounting
//! - OOM scoring and notification
//! - Memory pressure events
//! - Reclaim statistics per cgroup
//! - LRU management per memcg

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Cgroup memory limit type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemcgLimit {
    Hard,
    Soft,
    Swap,
    KernelStack,
    TcpBuf,
}

/// Memory pressure level per cgroup
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemcgPressure {
    None,
    Low,
    Medium,
    Critical,
    Oom,
}

/// OOM control policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemcgOomPolicy {
    Kill,
    Pause,
    Notify,
    Migrate,
}

/// Per-cgroup memory counters
#[derive(Debug, Clone, Default)]
pub struct MemcgCounters {
    pub cache_bytes: u64,
    pub rss_bytes: u64,
    pub rss_huge_bytes: u64,
    pub shmem_bytes: u64,
    pub mapped_file_bytes: u64,
    pub dirty_bytes: u64,
    pub writeback_bytes: u64,
    pub swap_bytes: u64,
    pub pgfault: u64,
    pub pgmajfault: u64,
    pub pgrefill: u64,
    pub pgscan: u64,
    pub pgsteal: u64,
    pub pgactivate: u64,
    pub pgdeactivate: u64,
    pub pglazyfree: u64,
    pub thp_fault_alloc: u64,
    pub thp_collapse_alloc: u64,
    pub kernel_stack_bytes: u64,
    pub slab_reclaimable: u64,
    pub slab_unreclaimable: u64,
}

impl MemcgCounters {
    pub fn total_mem(&self) -> u64 { self.cache_bytes + self.rss_bytes + self.kernel_stack_bytes + self.slab_reclaimable + self.slab_unreclaimable }
    pub fn total_with_swap(&self) -> u64 { self.total_mem() + self.swap_bytes }
}

/// Single memcg
#[derive(Debug, Clone)]
pub struct Memcg {
    pub id: u64,
    pub name: String,
    pub parent_id: Option<u64>,
    pub children: Vec<u64>,
    pub hard_limit: u64,
    pub soft_limit: u64,
    pub swap_limit: u64,
    pub counters: MemcgCounters,
    pub usage_bytes: u64,
    pub max_usage_bytes: u64,
    pub watermark_low: u64,
    pub watermark_high: u64,
    pub pressure: MemcgPressure,
    pub oom_policy: MemcgOomPolicy,
    pub oom_kill_count: u64,
    pub failcnt: u64,
    pub move_charge_at_immigrate: bool,
    pub use_hierarchy: bool,
    pub created_at: u64,
}

impl Memcg {
    pub fn new(id: u64, name: String, parent: Option<u64>, hard: u64) -> Self {
        let soft = hard * 80 / 100;
        let wl = hard * 60 / 100;
        let wh = hard * 90 / 100;
        Self {
            id, name, parent_id: parent, children: Vec::new(),
            hard_limit: hard, soft_limit: soft, swap_limit: hard / 2,
            counters: MemcgCounters::default(), usage_bytes: 0, max_usage_bytes: 0,
            watermark_low: wl, watermark_high: wh,
            pressure: MemcgPressure::None, oom_policy: MemcgOomPolicy::Kill,
            oom_kill_count: 0, failcnt: 0,
            move_charge_at_immigrate: false, use_hierarchy: true, created_at: 0,
        }
    }

    pub fn charge(&mut self, bytes: u64) -> bool {
        if self.usage_bytes + bytes > self.hard_limit { self.failcnt += 1; return false; }
        self.usage_bytes += bytes;
        if self.usage_bytes > self.max_usage_bytes { self.max_usage_bytes = self.usage_bytes; }
        self.update_pressure();
        true
    }

    pub fn uncharge(&mut self, bytes: u64) {
        self.usage_bytes = self.usage_bytes.saturating_sub(bytes);
        self.update_pressure();
    }

    fn update_pressure(&mut self) {
        let pct = if self.hard_limit == 0 { 0 } else { (self.usage_bytes * 100) / self.hard_limit };
        self.pressure = match pct {
            0..=59 => MemcgPressure::None,
            60..=79 => MemcgPressure::Low,
            80..=94 => MemcgPressure::Medium,
            95..=99 => MemcgPressure::Critical,
            _ => MemcgPressure::Oom,
        };
    }

    pub fn usage_pct(&self) -> f64 { if self.hard_limit == 0 { 0.0 } else { self.usage_bytes as f64 / self.hard_limit as f64 * 100.0 } }
    pub fn above_soft(&self) -> bool { self.usage_bytes > self.soft_limit }
    pub fn above_high(&self) -> bool { self.usage_bytes > self.watermark_high }
}

/// Memcg event
#[derive(Debug, Clone)]
pub struct MemcgEvent {
    pub cgroup_id: u64,
    pub kind: MemcgEventKind,
    pub ts: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemcgEventKind {
    HardLimitHit,
    SoftLimitHit,
    OomTriggered,
    OomKill,
    PressureChanged,
    WatermarkLow,
    WatermarkHigh,
    Reclaim,
}

/// Reclaim info per cgroup
#[derive(Debug, Clone)]
pub struct MemcgReclaimInfo {
    pub cgroup_id: u64,
    pub pages_scanned: u64,
    pub pages_reclaimed: u64,
    pub scan_priority: u32,
    pub nr_attempts: u32,
    pub ts: u64,
}

/// Memcg stats
#[derive(Debug, Clone, Default)]
pub struct MemcgStats {
    pub total_cgroups: usize,
    pub total_usage_bytes: u64,
    pub total_limit_bytes: u64,
    pub total_oom_kills: u64,
    pub total_failcnt: u64,
    pub max_pressure: MemcgPressure,
    pub above_soft_count: usize,
}

impl Default for MemcgPressure {
    fn default() -> Self { MemcgPressure::None }
}

/// Holistic memcg manager
pub struct HolisticMemcgMgr {
    cgroups: BTreeMap<u64, Memcg>,
    events: Vec<MemcgEvent>,
    reclaim_log: Vec<MemcgReclaimInfo>,
    stats: MemcgStats,
    next_id: u64,
}

impl HolisticMemcgMgr {
    pub fn new() -> Self {
        Self { cgroups: BTreeMap::new(), events: Vec::new(), reclaim_log: Vec::new(), stats: MemcgStats::default(), next_id: 1 }
    }

    pub fn create_cgroup(&mut self, name: String, parent: Option<u64>, limit: u64, ts: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        let mut cg = Memcg::new(id, name, parent, limit);
        cg.created_at = ts;
        if let Some(pid) = parent {
            if let Some(p) = self.cgroups.get_mut(&pid) { p.children.push(id); }
        }
        self.cgroups.insert(id, cg);
        id
    }

    pub fn charge(&mut self, id: u64, bytes: u64, ts: u64) -> bool {
        let ok = if let Some(cg) = self.cgroups.get_mut(&id) { cg.charge(bytes) } else { return false; };
        if !ok {
            self.events.push(MemcgEvent { cgroup_id: id, kind: MemcgEventKind::HardLimitHit, ts });
        }
        ok
    }

    pub fn uncharge(&mut self, id: u64, bytes: u64) { if let Some(cg) = self.cgroups.get_mut(&id) { cg.uncharge(bytes); } }

    pub fn record_oom_kill(&mut self, id: u64, ts: u64) {
        if let Some(cg) = self.cgroups.get_mut(&id) {
            cg.oom_kill_count += 1;
            self.events.push(MemcgEvent { cgroup_id: id, kind: MemcgEventKind::OomKill, ts });
        }
    }

    pub fn record_reclaim(&mut self, id: u64, scanned: u64, reclaimed: u64, prio: u32, ts: u64) {
        if let Some(cg) = self.cgroups.get_mut(&id) {
            cg.counters.pgscan += scanned;
            cg.counters.pgsteal += reclaimed;
        }
        self.reclaim_log.push(MemcgReclaimInfo { cgroup_id: id, pages_scanned: scanned, pages_reclaimed: reclaimed, scan_priority: prio, nr_attempts: 1, ts });
    }

    pub fn set_limits(&mut self, id: u64, hard: u64, soft: u64, swap: u64) {
        if let Some(cg) = self.cgroups.get_mut(&id) { cg.hard_limit = hard; cg.soft_limit = soft; cg.swap_limit = swap; }
    }

    pub fn set_oom_policy(&mut self, id: u64, policy: MemcgOomPolicy) {
        if let Some(cg) = self.cgroups.get_mut(&id) { cg.oom_policy = policy; }
    }

    pub fn hierarchical_usage(&self, id: u64) -> u64 {
        let own = self.cgroups.get(&id).map(|c| c.usage_bytes).unwrap_or(0);
        let children_ids: Vec<u64> = self.cgroups.get(&id).map(|c| c.children.clone()).unwrap_or_default();
        let child_sum: u64 = children_ids.iter().map(|&cid| self.hierarchical_usage(cid)).sum();
        own + child_sum
    }

    pub fn above_soft_cgroups(&self) -> Vec<u64> { self.cgroups.values().filter(|c| c.above_soft()).map(|c| c.id).collect() }

    pub fn recompute(&mut self) {
        self.stats.total_cgroups = self.cgroups.len();
        self.stats.total_usage_bytes = self.cgroups.values().map(|c| c.usage_bytes).sum();
        self.stats.total_limit_bytes = self.cgroups.values().map(|c| c.hard_limit).sum();
        self.stats.total_oom_kills = self.cgroups.values().map(|c| c.oom_kill_count).sum();
        self.stats.total_failcnt = self.cgroups.values().map(|c| c.failcnt).sum();
        self.stats.max_pressure = self.cgroups.values().map(|c| c.pressure).max().unwrap_or(MemcgPressure::None);
        self.stats.above_soft_count = self.cgroups.values().filter(|c| c.above_soft()).count();
    }

    pub fn cgroup(&self, id: u64) -> Option<&Memcg> { self.cgroups.get(&id) }
    pub fn events(&self) -> &[MemcgEvent] { &self.events }
    pub fn stats(&self) -> &MemcgStats { &self.stats }
}
