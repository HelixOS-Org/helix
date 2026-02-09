//! # Holistic Memory Reclamation Engine
//!
//! System-wide memory reclamation and pressure management:
//! - Multi-tier reclamation (soft→hard→OOM)
//! - Working set vs cache classification
//! - Reclamation fairness across processes
//! - Proactive reclamation based on trends
//! - NUMA-aware reclamation

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

// ============================================================================
// RECLAMATION TYPES
// ============================================================================

/// Reclamation urgency
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReclaimUrgency {
    /// Background (proactive)
    Background,
    /// Low pressure
    Low,
    /// Medium pressure
    Medium,
    /// High pressure
    High,
    /// Critical (OOM imminent)
    Critical,
}

/// Reclamation source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReclaimSource {
    /// Page cache (file-backed)
    PageCache,
    /// Anonymous pages (swap)
    Anonymous,
    /// Slab caches
    SlabCache,
    /// Buffer cache
    BufferCache,
    /// Dentries/inodes
    DentryInode,
    /// Per-CPU caches
    PerCpuCache,
    /// Compaction (not actual reclaim)
    Compaction,
}

/// Reclamation action
#[derive(Debug, Clone)]
pub struct ReclaimAction {
    /// Source to reclaim from
    pub source: ReclaimSource,
    /// Pages to reclaim
    pub pages_target: u64,
    /// NUMA node (-1 for any)
    pub numa_node: i32,
    /// Priority (higher = more aggressive)
    pub priority: u32,
    /// Process to target (0 for global)
    pub target_pid: u64,
}

// ============================================================================
// MEMORY ZONE
// ============================================================================

/// Zone type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReclaimZoneType {
    /// DMA zone
    Dma,
    /// DMA32
    Dma32,
    /// Normal
    Normal,
    /// Highmem
    HighMem,
    /// Movable
    Movable,
}

/// Memory zone state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ZoneState {
    /// Zone type
    pub zone_type: ReclaimZoneType,
    /// NUMA node
    pub numa_node: u32,
    /// Total pages
    pub total_pages: u64,
    /// Free pages
    pub free_pages: u64,
    /// File-backed pages
    pub file_pages: u64,
    /// Anonymous pages
    pub anon_pages: u64,
    /// Active pages
    pub active_pages: u64,
    /// Inactive pages
    pub inactive_pages: u64,
    /// High watermark
    pub high_wmark: u64,
    /// Low watermark
    pub low_wmark: u64,
    /// Min watermark
    pub min_wmark: u64,
}

impl ZoneState {
    pub fn new(zone_type: ReclaimZoneType, numa_node: u32, total_pages: u64) -> Self {
        let high = total_pages / 16;
        let low = high * 3 / 4;
        let min = high / 2;
        Self {
            zone_type,
            numa_node,
            total_pages,
            free_pages: total_pages,
            file_pages: 0,
            anon_pages: 0,
            active_pages: 0,
            inactive_pages: 0,
            high_wmark: high,
            low_wmark: low,
            min_wmark: min,
        }
    }

    /// Is below low watermark?
    #[inline(always)]
    pub fn below_low(&self) -> bool {
        self.free_pages < self.low_wmark
    }

    /// Is below min watermark?
    #[inline(always)]
    pub fn below_min(&self) -> bool {
        self.free_pages < self.min_wmark
    }

    /// Pressure level
    pub fn urgency(&self) -> ReclaimUrgency {
        if self.free_pages >= self.high_wmark {
            ReclaimUrgency::Background
        } else if self.free_pages >= self.low_wmark {
            ReclaimUrgency::Low
        } else if self.free_pages >= self.min_wmark {
            ReclaimUrgency::Medium
        } else if self.free_pages > 0 {
            ReclaimUrgency::High
        } else {
            ReclaimUrgency::Critical
        }
    }

    /// Free ratio
    #[inline]
    pub fn free_ratio(&self) -> f64 {
        if self.total_pages == 0 {
            return 0.0;
        }
        self.free_pages as f64 / self.total_pages as f64
    }

    /// Pages needed to reach high watermark
    #[inline(always)]
    pub fn deficit(&self) -> u64 {
        self.high_wmark.saturating_sub(self.free_pages)
    }
}

// ============================================================================
// PER-PROCESS RECLAIMABLE
// ============================================================================

/// Per-process reclaimable memory
#[derive(Debug, Clone)]
pub struct ProcessReclaimable {
    /// Process id
    pub pid: u64,
    /// File-backed pages
    pub file_pages: u64,
    /// Anonymous pages
    pub anon_pages: u64,
    /// Shared pages
    pub shared_pages: u64,
    /// Reclaimable slab
    pub slab_reclaimable: u64,
    /// OOM score
    pub oom_score: i32,
    /// RSS (resident set size)
    pub rss: u64,
    /// Is OOM-kill exempt?
    pub oom_exempt: bool,
}

impl ProcessReclaimable {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            file_pages: 0,
            anon_pages: 0,
            shared_pages: 0,
            slab_reclaimable: 0,
            oom_score: 0,
            rss: 0,
            oom_exempt: false,
        }
    }

    /// Total reclaimable
    #[inline(always)]
    pub fn total_reclaimable(&self) -> u64 {
        self.file_pages + self.slab_reclaimable
    }

    /// Total swappable
    #[inline(always)]
    pub fn total_swappable(&self) -> u64 {
        self.anon_pages
    }
}

// ============================================================================
// OOM KILLER
// ============================================================================

/// OOM kill candidate
#[derive(Debug, Clone)]
pub struct OomCandidate {
    /// Process id
    pub pid: u64,
    /// OOM score
    pub score: i32,
    /// RSS
    pub rss: u64,
    /// Would free pages
    pub would_free: u64,
}

/// OOM killer
#[derive(Debug)]
pub struct OomKiller {
    /// Total kills
    pub total_kills: u64,
    /// Last kill timestamp
    pub last_kill: u64,
    /// Kill cooldown (ns)
    pub cooldown_ns: u64,
}

impl OomKiller {
    pub fn new() -> Self {
        Self {
            total_kills: 0,
            last_kill: 0,
            cooldown_ns: 1_000_000_000, // 1s
        }
    }

    /// Select victim
    #[inline]
    pub fn select_victim(&self, candidates: &[OomCandidate]) -> Option<u64> {
        candidates.iter()
            .max_by_key(|c| c.score)
            .map(|c| c.pid)
    }

    /// Can kill now?
    #[inline(always)]
    pub fn can_kill(&self, now: u64) -> bool {
        now.saturating_sub(self.last_kill) >= self.cooldown_ns
    }

    /// Record kill
    #[inline(always)]
    pub fn record_kill(&mut self, now: u64) {
        self.total_kills += 1;
        self.last_kill = now;
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Reclamation stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticReclaimStats {
    /// Total zones
    pub total_zones: usize,
    /// Zones under pressure
    pub pressured_zones: usize,
    /// Total pages reclaimed
    pub total_reclaimed: u64,
    /// OOM kills
    pub oom_kills: u64,
    /// Current urgency
    pub max_urgency: u8,
}

/// Holistic reclamation engine
pub struct HolisticReclaimEngine {
    /// Zone states
    zones: Vec<ZoneState>,
    /// Per-process reclaimable
    processes: BTreeMap<u64, ProcessReclaimable>,
    /// OOM killer
    pub oom: OomKiller,
    /// Total reclaimed
    total_reclaimed: u64,
    /// Stats
    stats: HolisticReclaimStats,
}

impl HolisticReclaimEngine {
    pub fn new() -> Self {
        Self {
            zones: Vec::new(),
            processes: BTreeMap::new(),
            oom: OomKiller::new(),
            total_reclaimed: 0,
            stats: HolisticReclaimStats::default(),
        }
    }

    /// Add zone
    #[inline(always)]
    pub fn add_zone(&mut self, zone: ZoneState) {
        self.zones.push(zone);
        self.update_stats();
    }

    /// Register process
    #[inline(always)]
    pub fn register_process(&mut self, proc: ProcessReclaimable) {
        self.processes.insert(proc.pid, proc);
    }

    /// Update zone state
    #[inline]
    pub fn update_zone(&mut self, index: usize, free_pages: u64) {
        if let Some(zone) = self.zones.get_mut(index) {
            zone.free_pages = free_pages;
        }
        self.update_stats();
    }

    /// Plan reclamation
    pub fn plan_reclaim(&self) -> Vec<ReclaimAction> {
        let mut actions = Vec::new();

        for zone in &self.zones {
            let urgency = zone.urgency();
            if urgency <= ReclaimUrgency::Background {
                continue;
            }

            let deficit = zone.deficit();

            // File pages first
            if zone.file_pages > 0 {
                actions.push(ReclaimAction {
                    source: ReclaimSource::PageCache,
                    pages_target: deficit.min(zone.file_pages),
                    numa_node: zone.numa_node as i32,
                    priority: urgency as u32,
                    target_pid: 0,
                });
            }

            // Slab if still needed
            if deficit > zone.file_pages {
                actions.push(ReclaimAction {
                    source: ReclaimSource::SlabCache,
                    pages_target: deficit.saturating_sub(zone.file_pages),
                    numa_node: zone.numa_node as i32,
                    priority: urgency as u32,
                    target_pid: 0,
                });
            }
        }

        actions
    }

    /// Record reclaimed pages
    #[inline(always)]
    pub fn record_reclaimed(&mut self, pages: u64) {
        self.total_reclaimed += pages;
        self.update_stats();
    }

    /// Get OOM candidates (sorted by score)
    pub fn oom_candidates(&self) -> Vec<OomCandidate> {
        let mut candidates: Vec<OomCandidate> = self.processes.values()
            .filter(|p| !p.oom_exempt)
            .map(|p| OomCandidate {
                pid: p.pid,
                score: p.oom_score,
                rss: p.rss,
                would_free: p.rss,
            })
            .collect();
        candidates.sort_by(|a, b| b.score.cmp(&a.score));
        candidates
    }

    /// System urgency (max across zones)
    #[inline]
    pub fn system_urgency(&self) -> ReclaimUrgency {
        self.zones.iter()
            .map(|z| z.urgency())
            .max()
            .unwrap_or(ReclaimUrgency::Background)
    }

    fn update_stats(&mut self) {
        self.stats.total_zones = self.zones.len();
        self.stats.pressured_zones = self.zones.iter()
            .filter(|z| z.urgency() > ReclaimUrgency::Low).count();
        self.stats.total_reclaimed = self.total_reclaimed;
        self.stats.oom_kills = self.oom.total_kills;
        self.stats.max_urgency = self.system_urgency() as u8;
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &HolisticReclaimStats {
        &self.stats
    }
}

// ============================================================================
// Merged from reclaim_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReclaimUrgency {
    None,
    Background,
    Moderate,
    High,
    Critical,
    Oom,
}

/// Page generation (multi-gen LRU)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageGeneration {
    Young,
    Active,
    Inactive,
    Old,
    Stale,
}

/// Reclamation source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReclaimSource {
    Anon,
    FileCache,
    Slab,
    SwapCache,
    PageTablePages,
    KernelStacks,
}

/// Page age histogram bucket
#[derive(Debug, Clone)]
pub struct AgeHistogramBucket {
    pub min_age_ms: u64,
    pub max_age_ms: u64,
    pub page_count: u64,
    pub total_bytes: u64,
}

/// Per-zone watermark state
#[derive(Debug, Clone)]
pub struct ZoneWatermarks {
    pub zone_id: u32,
    pub min_pages: u64,
    pub low_pages: u64,
    pub high_pages: u64,
    pub free_pages: u64,
    pub file_pages: u64,
    pub anon_pages: u64,
    pub slab_reclaimable: u64,
}

impl ZoneWatermarks {
    pub fn new(zone_id: u32, min: u64, low: u64, high: u64) -> Self {
        Self {
            zone_id,
            min_pages: min,
            low_pages: low,
            high_pages: high,
            free_pages: high,
            file_pages: 0,
            anon_pages: 0,
            slab_reclaimable: 0,
        }
    }

    #[inline]
    pub fn urgency(&self) -> ReclaimUrgency {
        if self.free_pages <= self.min_pages {
            ReclaimUrgency::Critical
        } else if self.free_pages <= self.low_pages {
            ReclaimUrgency::High
        } else if self.free_pages <= self.high_pages {
            ReclaimUrgency::Moderate
        } else {
            ReclaimUrgency::None
        }
    }

    #[inline(always)]
    pub fn reclaimable_estimate(&self) -> u64 {
        self.file_pages + self.slab_reclaimable
    }
}

/// Per-cgroup memory pressure
#[derive(Debug, Clone)]
pub struct CgroupMemPressure {
    pub cgroup_id: u64,
    pub memory_limit: u64,
    pub memory_usage: u64,
    pub swap_usage: u64,
    pub reclaim_count: u64,
    pub oom_kills: u32,
    pub pressure_some_us: u64,
    pub pressure_full_us: u64,
}

impl CgroupMemPressure {
    pub fn new(cgroup_id: u64, limit: u64) -> Self {
        Self {
            cgroup_id,
            memory_limit: limit,
            memory_usage: 0,
            swap_usage: 0,
            reclaim_count: 0,
            oom_kills: 0,
            pressure_some_us: 0,
            pressure_full_us: 0,
        }
    }

    #[inline(always)]
    pub fn usage_ratio(&self) -> f64 {
        if self.memory_limit == 0 { return 0.0; }
        self.memory_usage as f64 / self.memory_limit as f64
    }

    #[inline(always)]
    pub fn is_near_limit(&self) -> bool {
        self.usage_ratio() > 0.9
    }
}

/// Working set estimator
#[derive(Debug, Clone)]
pub struct WorkingSetEstimatorV2 {
    pub process_id: u64,
    pub sampled_pages: u64,
    pub accessed_pages: u64,
    pub working_set_pages: u64,
    pub growth_rate: f64,
    pub decay_factor: f64,
    samples: VecDeque<(u64, u64)>, // (timestamp_ns, accessed)
}

impl WorkingSetEstimatorV2 {
    pub fn new(process_id: u64) -> Self {
        Self {
            process_id,
            sampled_pages: 0,
            accessed_pages: 0,
            working_set_pages: 0,
            growth_rate: 0.0,
            decay_factor: 0.95,
            samples: VecDeque::new(),
        }
    }

    pub fn record_sample(&mut self, ts: u64, total_pages: u64, accessed: u64) {
        self.sampled_pages = total_pages;
        self.accessed_pages = accessed;

        // EWMA update
        let ratio = if total_pages > 0 { accessed as f64 / total_pages as f64 } else { 0.0 };
        let estimated = (total_pages as f64 * ratio) as u64;
        self.working_set_pages = ((self.working_set_pages as f64 * self.decay_factor)
            + (estimated as f64 * (1.0 - self.decay_factor))) as u64;

        self.samples.push_back((ts, accessed));
        if self.samples.len() > 64 {
            self.samples.pop_front();
        }

        // Compute growth rate
        if self.samples.len() >= 2 {
            let first = self.samples[0].1 as f64;
            let last = self.samples[self.samples.len() - 1].1 as f64;
            self.growth_rate = if first > 0.0 { (last - first) / first } else { 0.0 };
        }
    }

    #[inline(always)]
    pub fn is_growing(&self) -> bool { self.growth_rate > 0.05 }
    #[inline(always)]
    pub fn is_shrinking(&self) -> bool { self.growth_rate < -0.05 }
}

/// Reclaim event record
#[derive(Debug, Clone)]
pub struct ReclaimEvent {
    pub timestamp_ns: u64,
    pub source: ReclaimSource,
    pub pages_scanned: u64,
    pub pages_reclaimed: u64,
    pub urgency: ReclaimUrgency,
    pub duration_ns: u64,
}

/// Holistic Memory Reclaim V2 stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticReclaimV2Stats {
    pub total_reclaims: u64,
    pub total_pages_reclaimed: u64,
    pub total_pages_scanned: u64,
    pub avg_efficiency: f64,
    pub oom_events: u32,
    pub current_urgency: u8,
}

/// Holistic Memory Reclaim V2
pub struct HolisticReclaimV2 {
    zones: BTreeMap<u32, ZoneWatermarks>,
    cgroups: BTreeMap<u64, CgroupMemPressure>,
    working_sets: BTreeMap<u64, WorkingSetEstimatorV2>,
    events: VecDeque<ReclaimEvent>,
    max_events: usize,
    stats: HolisticReclaimV2Stats,
}

impl HolisticReclaimV2 {
    pub fn new(max_events: usize) -> Self {
        Self {
            zones: BTreeMap::new(),
            cgroups: BTreeMap::new(),
            working_sets: BTreeMap::new(),
            events: VecDeque::new(),
            max_events,
            stats: HolisticReclaimV2Stats::default(),
        }
    }

    #[inline(always)]
    pub fn add_zone(&mut self, zone: ZoneWatermarks) {
        self.zones.insert(zone.zone_id, zone);
    }

    #[inline(always)]
    pub fn add_cgroup(&mut self, cg: CgroupMemPressure) {
        self.cgroups.insert(cg.cgroup_id, cg);
    }

    #[inline]
    pub fn update_working_set(&mut self, pid: u64, ts: u64, total: u64, accessed: u64) {
        self.working_sets.entry(pid)
            .or_insert_with(|| WorkingSetEstimatorV2::new(pid))
            .record_sample(ts, total, accessed);
    }

    #[inline]
    pub fn global_urgency(&self) -> ReclaimUrgency {
        self.zones.values()
            .map(|z| z.urgency())
            .max()
            .unwrap_or(ReclaimUrgency::None)
    }

    /// Determine which cgroups need reclaim
    #[inline]
    pub fn cgroups_needing_reclaim(&self) -> Vec<u64> {
        self.cgroups.values()
            .filter(|cg| cg.is_near_limit())
            .map(|cg| cg.cgroup_id)
            .collect()
    }

    /// Record a reclaim event
    pub fn record_reclaim(&mut self, event: ReclaimEvent) {
        self.stats.total_reclaims += 1;
        self.stats.total_pages_reclaimed += event.pages_reclaimed;
        self.stats.total_pages_scanned += event.pages_scanned;

        let eff = if event.pages_scanned > 0 {
            event.pages_reclaimed as f64 / event.pages_scanned as f64
        } else { 0.0 };
        self.stats.avg_efficiency = self.stats.avg_efficiency * 0.9 + eff * 0.1;
        self.stats.current_urgency = self.global_urgency() as u8;

        self.events.push_back(event);
        while self.events.len() > self.max_events {
            self.events.pop_front();
        }
    }

    #[inline(always)]
    pub fn total_free_pages(&self) -> u64 {
        self.zones.values().map(|z| z.free_pages).sum()
    }

    #[inline(always)]
    pub fn total_reclaimable(&self) -> u64 {
        self.zones.values().map(|z| z.reclaimable_estimate()).sum()
    }

    #[inline(always)]
    pub fn stats(&self) -> &HolisticReclaimV2Stats { &self.stats }
}
