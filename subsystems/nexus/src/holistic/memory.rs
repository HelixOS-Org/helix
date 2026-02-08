//! # Holistic Memory Management
//!
//! System-wide memory optimization:
//! - Global memory pressure management
//! - Page reclamation policy
//! - Memory zone management
//! - Transparent huge page management
//! - KSM (Kernel Same-page Merging) policy
//! - Memory cgroup integration
//! - OOM policy and victim selection
//! - Memory compaction triggers

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// MEMORY ZONES
// ============================================================================

/// Memory zone type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryZone {
    /// DMA zone (0-16MB)
    Dma,
    /// DMA32 zone (0-4GB)
    Dma32,
    /// Normal zone
    Normal,
    /// High memory zone
    HighMem,
    /// Movable zone (for hotplug)
    Movable,
    /// Device zone (PMEM, etc.)
    Device,
}

/// Memory zone statistics
#[derive(Debug, Clone)]
pub struct ZoneStats {
    /// Zone type
    pub zone: MemoryZone,
    /// Total pages
    pub total_pages: u64,
    /// Free pages
    pub free_pages: u64,
    /// Active file pages
    pub active_file: u64,
    /// Inactive file pages
    pub inactive_file: u64,
    /// Active anonymous pages
    pub active_anon: u64,
    /// Inactive anonymous pages
    pub inactive_anon: u64,
    /// Slab reclaimable
    pub slab_reclaimable: u64,
    /// Slab unreclaimable
    pub slab_unreclaimable: u64,
    /// Page table pages
    pub page_tables: u64,
    /// Dirty pages
    pub dirty_pages: u64,
    /// Writeback pages
    pub writeback_pages: u64,
    /// Low watermark
    pub watermark_low: u64,
    /// High watermark
    pub watermark_high: u64,
    /// Min watermark
    pub watermark_min: u64,
}

impl ZoneStats {
    pub fn new(zone: MemoryZone, total_pages: u64) -> Self {
        Self {
            zone,
            total_pages,
            free_pages: total_pages,
            active_file: 0,
            inactive_file: 0,
            active_anon: 0,
            inactive_anon: 0,
            slab_reclaimable: 0,
            slab_unreclaimable: 0,
            page_tables: 0,
            dirty_pages: 0,
            writeback_pages: 0,
            watermark_low: total_pages / 20,     // 5%
            watermark_high: total_pages / 10,    // 10%
            watermark_min: total_pages / 40,     // 2.5%
        }
    }

    /// Is zone under pressure?
    pub fn under_pressure(&self) -> bool {
        self.free_pages < self.watermark_low
    }

    /// Is zone critically low?
    pub fn critical(&self) -> bool {
        self.free_pages < self.watermark_min
    }

    /// Reclaimable pages estimate
    pub fn reclaimable(&self) -> u64 {
        self.inactive_file + self.slab_reclaimable + self.inactive_anon / 2
    }

    /// Utilization (0.0 - 1.0)
    pub fn utilization(&self) -> f64 {
        if self.total_pages == 0 {
            return 0.0;
        }
        1.0 - (self.free_pages as f64 / self.total_pages as f64)
    }
}

// ============================================================================
// PRESSURE LEVELS
// ============================================================================

/// Memory pressure level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryPressure {
    /// No pressure
    None,
    /// Low pressure (proactive reclaim)
    Low,
    /// Medium pressure (aggressive reclaim)
    Medium,
    /// High pressure (urgent reclaim)
    High,
    /// Critical (OOM imminent)
    Critical,
}

impl MemoryPressure {
    pub fn from_free_ratio(free_ratio: f64) -> Self {
        if free_ratio > 0.20 {
            Self::None
        } else if free_ratio > 0.10 {
            Self::Low
        } else if free_ratio > 0.05 {
            Self::Medium
        } else if free_ratio > 0.02 {
            Self::High
        } else {
            Self::Critical
        }
    }
}

// ============================================================================
// RECLAMATION POLICY
// ============================================================================

/// Reclamation target type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReclaimTarget {
    /// Reclaim file pages
    FilePages,
    /// Reclaim anonymous pages (swap)
    AnonPages,
    /// Reclaim slab
    Slab,
    /// Reclaim page tables
    PageTables,
    /// Drop dentries/inodes
    DentryCache,
    /// Compact memory
    Compact,
}

/// Reclamation action
#[derive(Debug, Clone)]
pub struct ReclaimAction {
    /// Target
    pub target: ReclaimTarget,
    /// Pages to reclaim
    pub pages: u64,
    /// Priority (0 = most urgent)
    pub priority: u8,
    /// Zone to reclaim from
    pub zone: MemoryZone,
}

/// Reclamation policy engine
pub struct ReclaimPolicy {
    /// Swappiness (0-100)
    pub swappiness: u32,
    /// Cache pressure (0-1000)
    pub vfs_cache_pressure: u32,
    /// Dirty ratio threshold (percent)
    pub dirty_ratio: u32,
    /// Dirty background ratio
    pub dirty_background_ratio: u32,
    /// Compact threshold pages
    pub compact_threshold: u64,
    /// Min free pages target
    pub min_free_kbytes: u64,
}

impl Default for ReclaimPolicy {
    fn default() -> Self {
        Self {
            swappiness: 60,
            vfs_cache_pressure: 100,
            dirty_ratio: 20,
            dirty_background_ratio: 10,
            compact_threshold: 1024,
            min_free_kbytes: 65536,
        }
    }
}

impl ReclaimPolicy {
    /// Generate reclamation actions based on pressure
    pub fn generate_actions(
        &self,
        pressure: MemoryPressure,
        zone: &ZoneStats,
    ) -> Vec<ReclaimAction> {
        let mut actions = Vec::new();

        match pressure {
            MemoryPressure::None => {}
            MemoryPressure::Low => {
                // Gentle reclaim of file pages
                if zone.inactive_file > zone.watermark_high {
                    actions.push(ReclaimAction {
                        target: ReclaimTarget::FilePages,
                        pages: zone.inactive_file / 4,
                        priority: 12,
                        zone: zone.zone,
                    });
                }
            }
            MemoryPressure::Medium => {
                // More aggressive
                actions.push(ReclaimAction {
                    target: ReclaimTarget::FilePages,
                    pages: zone.inactive_file / 2,
                    priority: 8,
                    zone: zone.zone,
                });
                if zone.slab_reclaimable > 1024 {
                    actions.push(ReclaimAction {
                        target: ReclaimTarget::Slab,
                        pages: zone.slab_reclaimable / 4,
                        priority: 8,
                        zone: zone.zone,
                    });
                }
                if self.swappiness > 0 && zone.inactive_anon > 0 {
                    let swap_pages =
                        zone.inactive_anon * self.swappiness as u64 / 100;
                    actions.push(ReclaimAction {
                        target: ReclaimTarget::AnonPages,
                        pages: swap_pages,
                        priority: 10,
                        zone: zone.zone,
                    });
                }
            }
            MemoryPressure::High | MemoryPressure::Critical => {
                // Aggressive reclaim everything
                actions.push(ReclaimAction {
                    target: ReclaimTarget::FilePages,
                    pages: zone.inactive_file + zone.active_file / 2,
                    priority: 2,
                    zone: zone.zone,
                });
                actions.push(ReclaimAction {
                    target: ReclaimTarget::Slab,
                    pages: zone.slab_reclaimable,
                    priority: 2,
                    zone: zone.zone,
                });
                actions.push(ReclaimAction {
                    target: ReclaimTarget::DentryCache,
                    pages: 0, // Flush all
                    priority: 2,
                    zone: zone.zone,
                });
                if self.swappiness > 0 {
                    actions.push(ReclaimAction {
                        target: ReclaimTarget::AnonPages,
                        pages: zone.inactive_anon,
                        priority: 4,
                        zone: zone.zone,
                    });
                }
            }
        }

        actions
    }
}

// ============================================================================
// OOM POLICY
// ============================================================================

/// OOM score for a process
#[derive(Debug, Clone)]
pub struct OomScore {
    /// PID
    pub pid: u64,
    /// OOM score (0-1000, higher = more likely to kill)
    pub score: u32,
    /// User-set adjustment (-1000 to 1000)
    pub adj: i32,
    /// Memory usage (bytes)
    pub memory_usage: u64,
    /// Is kernel thread?
    pub is_kernel: bool,
    /// Is critical system service?
    pub is_critical: bool,
}

impl OomScore {
    /// Calculate effective OOM score
    pub fn effective_score(&self) -> i32 {
        if self.is_kernel || self.is_critical {
            return -1000; // Never kill
        }
        let base = self.score as i32;
        (base + self.adj).max(-1000).min(1000)
    }
}

/// OOM killer policy
pub struct OomPolicy {
    /// Process scores
    scores: BTreeMap<u64, OomScore>,
    /// OOM events
    pub oom_events: u64,
    /// Processes killed
    pub processes_killed: u64,
    /// Memory freed by OOM (bytes)
    pub memory_freed: u64,
}

impl OomPolicy {
    pub fn new() -> Self {
        Self {
            scores: BTreeMap::new(),
            oom_events: 0,
            processes_killed: 0,
            memory_freed: 0,
        }
    }

    /// Update OOM score
    pub fn update_score(&mut self, score: OomScore) {
        self.scores.insert(score.pid, score);
    }

    /// Remove process
    pub fn remove(&mut self, pid: u64) {
        self.scores.remove(&pid);
    }

    /// Select OOM victim
    pub fn select_victim(&self) -> Option<u64> {
        self.scores
            .values()
            .filter(|s| !s.is_kernel && !s.is_critical && s.effective_score() > -1000)
            .max_by_key(|s| s.effective_score())
            .map(|s| s.pid)
    }

    /// Record OOM event
    pub fn record_oom(&mut self, victim_pid: u64, freed_bytes: u64) {
        self.oom_events += 1;
        self.processes_killed += 1;
        self.memory_freed += freed_bytes;
        self.scores.remove(&victim_pid);
    }

    /// Score count
    pub fn score_count(&self) -> usize {
        self.scores.len()
    }
}

// ============================================================================
// HOLISTIC MEMORY MANAGER
// ============================================================================

/// Memory manager statistics
#[derive(Debug, Clone)]
pub struct MemoryManagerStats {
    /// Total system memory (pages)
    pub total_pages: u64,
    /// Free pages
    pub free_pages: u64,
    /// Current pressure
    pub pressure: MemoryPressure,
    /// Reclaim actions pending
    pub pending_reclaims: usize,
    /// OOM events
    pub oom_events: u64,
    /// Pages reclaimed total
    pub pages_reclaimed: u64,
}

/// System-wide memory manager
pub struct HolisticMemoryManager {
    /// Zone statistics
    zones: BTreeMap<u8, ZoneStats>,
    /// Reclaim policy
    reclaim_policy: ReclaimPolicy,
    /// OOM policy
    oom_policy: OomPolicy,
    /// Current pressure
    pressure: MemoryPressure,
    /// Pending reclaim actions
    pending_actions: Vec<ReclaimAction>,
    /// Total pages reclaimed
    pub pages_reclaimed: u64,
    /// Total compactions
    pub compactions: u64,
}

impl HolisticMemoryManager {
    pub fn new() -> Self {
        Self {
            zones: BTreeMap::new(),
            reclaim_policy: ReclaimPolicy::default(),
            oom_policy: OomPolicy::new(),
            pressure: MemoryPressure::None,
            pending_actions: Vec::new(),
            pages_reclaimed: 0,
            compactions: 0,
        }
    }

    /// Add a zone
    pub fn add_zone(&mut self, zone: MemoryZone, total_pages: u64) {
        self.zones
            .insert(zone as u8, ZoneStats::new(zone, total_pages));
    }

    /// Update zone stats
    pub fn update_zone(&mut self, stats: ZoneStats) {
        self.zones.insert(stats.zone as u8, stats);
    }

    /// Evaluate pressure and generate actions
    pub fn evaluate(&mut self) -> MemoryPressure {
        let total_free: u64 = self.zones.values().map(|z| z.free_pages).sum();
        let total_pages: u64 = self.zones.values().map(|z| z.total_pages).sum();

        let free_ratio = if total_pages > 0 {
            total_free as f64 / total_pages as f64
        } else {
            1.0
        };

        self.pressure = MemoryPressure::from_free_ratio(free_ratio);

        // Generate reclaim actions for pressured zones
        self.pending_actions.clear();
        let zones: Vec<ZoneStats> = self.zones.values().cloned().collect();
        for zone in &zones {
            if zone.under_pressure() {
                let actions = self.reclaim_policy.generate_actions(self.pressure, zone);
                self.pending_actions.extend(actions);
            }
        }

        // Sort by priority (lower = more urgent)
        self.pending_actions.sort_by_key(|a| a.priority);

        self.pressure
    }

    /// Get pending reclaim actions
    pub fn pending_actions(&self) -> &[ReclaimAction] {
        &self.pending_actions
    }

    /// Get current pressure
    pub fn pressure(&self) -> MemoryPressure {
        self.pressure
    }

    /// Get OOM policy
    pub fn oom_policy(&self) -> &OomPolicy {
        &self.oom_policy
    }

    /// Get mutable OOM policy
    pub fn oom_policy_mut(&mut self) -> &mut OomPolicy {
        &mut self.oom_policy
    }

    /// Get stats
    pub fn stats(&self) -> MemoryManagerStats {
        MemoryManagerStats {
            total_pages: self.zones.values().map(|z| z.total_pages).sum(),
            free_pages: self.zones.values().map(|z| z.free_pages).sum(),
            pressure: self.pressure,
            pending_reclaims: self.pending_actions.len(),
            oom_events: self.oom_policy.oom_events,
            pages_reclaimed: self.pages_reclaimed,
        }
    }

    /// Zone count
    pub fn zone_count(&self) -> usize {
        self.zones.len()
    }
}
