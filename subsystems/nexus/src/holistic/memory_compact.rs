// SPDX-License-Identifier: GPL-2.0
//! Holistic memory_compact — memory compaction and defragmentation engine.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Compaction mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactMode {
    /// Synchronous direct compaction
    Direct,
    /// Background kcompactd
    Background,
    /// Proactive compaction
    Proactive,
    /// CMA compaction for contiguous allocations
    Cma,
    /// Deferred compaction on first failure
    Deferred,
}

impl CompactMode {
    pub fn is_blocking(&self) -> bool {
        matches!(self, Self::Direct | Self::Cma)
    }

    pub fn priority(&self) -> u8 {
        match self {
            Self::Direct => 5,
            Self::Cma => 4,
            Self::Deferred => 3,
            Self::Proactive => 2,
            Self::Background => 1,
        }
    }
}

/// Compaction result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactResult {
    Success,
    Partial,
    Skipped,
    NoProgress,
    Deferred,
    ContiguousFail,
}

/// Migration type for movability
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageMobility {
    Unmovable,
    Movable,
    Reclaimable,
    HighAtomic,
    Isolate,
    Cma,
}

/// A zone's compaction state
#[derive(Debug)]
pub struct ZoneCompactState {
    pub zone_name: String,
    pub zone_id: u32,
    pub free_pages: u64,
    pub free_blocks_order: [u32; 11],
    pub fragmentation_index: f64,
    pub compact_considered: u64,
    pub compact_deferred: u64,
    pub compact_success: u64,
    pub compact_fail: u64,
    pub compact_pages_moved: u64,
    pub compact_pages_scanned: u64,
    pub suitable_for_order: u8,
    pub proactive_threshold: f64,
}

impl ZoneCompactState {
    pub fn new(zone_name: String, zone_id: u32) -> Self {
        Self {
            zone_name, zone_id,
            free_pages: 0,
            free_blocks_order: [0; 11],
            fragmentation_index: 0.0,
            compact_considered: 0,
            compact_deferred: 0,
            compact_success: 0,
            compact_fail: 0,
            compact_pages_moved: 0,
            compact_pages_scanned: 0,
            suitable_for_order: 0,
            proactive_threshold: 0.2,
        }
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.compact_success + self.compact_fail;
        if total == 0 { return 1.0; }
        self.compact_success as f64 / total as f64
    }

    pub fn efficiency(&self) -> f64 {
        if self.compact_pages_scanned == 0 { return 0.0; }
        self.compact_pages_moved as f64 / self.compact_pages_scanned as f64
    }

    pub fn needs_compaction(&self, order: u8) -> bool {
        self.fragmentation_index > self.proactive_threshold
            && (order as usize) < 11
            && self.free_blocks_order[order as usize] == 0
    }

    pub fn largest_free_order(&self) -> u8 {
        for i in (0..11).rev() {
            if self.free_blocks_order[i] > 0 {
                return i as u8;
            }
        }
        0
    }

    pub fn external_fragmentation(&self) -> f64 {
        if self.free_pages == 0 { return 1.0; }
        let high_order: u64 = (4..11).map(|i| {
            (self.free_blocks_order[i] as u64) << i
        }).sum();
        1.0 - (high_order as f64 / self.free_pages as f64)
    }
}

/// Compaction event record
#[derive(Debug, Clone)]
pub struct CompactEvent {
    pub zone_id: u32,
    pub mode: CompactMode,
    pub order: u8,
    pub result: CompactResult,
    pub pages_moved: u64,
    pub pages_scanned: u64,
    pub duration_us: u64,
    pub timestamp: u64,
}

impl CompactEvent {
    pub fn throughput(&self) -> f64 {
        if self.duration_us == 0 { return 0.0; }
        self.pages_moved as f64 / (self.duration_us as f64 / 1_000_000.0)
    }
}

/// Migration scanner tracking
#[derive(Debug)]
pub struct MigrationScanner {
    pub zone_id: u32,
    pub pfn_migrate_start: u64,
    pub pfn_free_start: u64,
    pub pfn_migrate_end: u64,
    pub pfn_free_end: u64,
    pub pages_isolated_migrate: u64,
    pub pages_isolated_free: u64,
}

impl MigrationScanner {
    pub fn new(zone_id: u32, start: u64, end: u64) -> Self {
        Self {
            zone_id,
            pfn_migrate_start: start,
            pfn_free_start: end,
            pfn_migrate_end: start,
            pfn_free_end: end,
            pages_isolated_migrate: 0,
            pages_isolated_free: 0,
        }
    }

    pub fn progress(&self) -> f64 {
        let total = self.pfn_free_start.saturating_sub(self.pfn_migrate_start);
        if total == 0 { return 1.0; }
        let done = self.pfn_migrate_end.saturating_sub(self.pfn_migrate_start)
            + self.pfn_free_start.saturating_sub(self.pfn_free_end);
        (done as f64 / total as f64).min(1.0)
    }

    pub fn scanners_met(&self) -> bool {
        self.pfn_migrate_end >= self.pfn_free_end
    }
}

/// Compaction stats
#[derive(Debug, Clone)]
pub struct CompactStats {
    pub total_attempts: u64,
    pub total_success: u64,
    pub total_pages_moved: u64,
    pub total_pages_scanned: u64,
    pub avg_duration_us: u64,
    pub proactive_triggered: u64,
    pub direct_stalls: u64,
}

/// Main memory compaction manager
pub struct HolisticMemoryCompact {
    zones: BTreeMap<u32, ZoneCompactState>,
    scanners: BTreeMap<u32, MigrationScanner>,
    history: Vec<CompactEvent>,
    max_history: usize,
    stats: CompactStats,
    proactive_enabled: bool,
    proactive_threshold: f64,
}

impl HolisticMemoryCompact {
    pub fn new() -> Self {
        Self {
            zones: BTreeMap::new(),
            scanners: BTreeMap::new(),
            history: Vec::new(),
            max_history: 2048,
            stats: CompactStats {
                total_attempts: 0, total_success: 0,
                total_pages_moved: 0, total_pages_scanned: 0,
                avg_duration_us: 0, proactive_triggered: 0,
                direct_stalls: 0,
            },
            proactive_enabled: true,
            proactive_threshold: 0.2,
        }
    }

    pub fn add_zone(&mut self, state: ZoneCompactState) {
        self.zones.insert(state.zone_id, state);
    }

    pub fn record_event(&mut self, event: CompactEvent) {
        self.stats.total_attempts += 1;
        if event.result == CompactResult::Success { self.stats.total_success += 1; }
        self.stats.total_pages_moved += event.pages_moved;
        self.stats.total_pages_scanned += event.pages_scanned;
        if event.mode == CompactMode::Direct { self.stats.direct_stalls += 1; }
        if event.mode == CompactMode::Proactive { self.stats.proactive_triggered += 1; }

        let n = self.stats.total_attempts;
        self.stats.avg_duration_us = ((self.stats.avg_duration_us * (n - 1)) + event.duration_us) / n;

        if let Some(zone) = self.zones.get_mut(&event.zone_id) {
            zone.compact_considered += 1;
            zone.compact_pages_moved += event.pages_moved;
            zone.compact_pages_scanned += event.pages_scanned;
            match event.result {
                CompactResult::Success | CompactResult::Partial => zone.compact_success += 1,
                CompactResult::Deferred => zone.compact_deferred += 1,
                _ => zone.compact_fail += 1,
            }
        }

        if self.history.len() >= self.max_history {
            self.history.remove(0);
        }
        self.history.push(event);
    }

    pub fn zones_needing_compaction(&self, order: u8) -> Vec<u32> {
        self.zones.iter()
            .filter(|(_, z)| z.needs_compaction(order))
            .map(|(&id, _)| id)
            .collect()
    }

    pub fn most_fragmented_zones(&self, n: usize) -> Vec<(u32, f64)> {
        let mut v: Vec<_> = self.zones.iter()
            .map(|(&id, z)| (id, z.external_fragmentation()))
            .collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        v.truncate(n);
        v
    }

    pub fn set_proactive(&mut self, enabled: bool, threshold: f64) {
        self.proactive_enabled = enabled;
        self.proactive_threshold = threshold;
        for zone in self.zones.values_mut() {
            zone.proactive_threshold = threshold;
        }
    }

    pub fn proactive_candidates(&self) -> Vec<u32> {
        if !self.proactive_enabled { return Vec::new(); }
        self.zones.iter()
            .filter(|(_, z)| z.fragmentation_index > self.proactive_threshold)
            .map(|(&id, _)| id)
            .collect()
    }

    pub fn stats(&self) -> &CompactStats {
        &self.stats
    }
}
