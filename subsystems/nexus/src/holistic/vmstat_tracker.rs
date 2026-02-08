//! # Holistic VMStat Tracker
//!
//! Virtual memory statistics tracking with holistic view:
//! - Per-zone page counters (free, active, inactive, dirty, writeback)
//! - Page allocation/free rate tracking
//! - Page reclaim statistics
//! - Swap activity tracking
//! - NUMA cross-node stats
//! - Historical trend tracking

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Memory zone
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmZone {
    Dma,
    Dma32,
    Normal,
    HighMem,
    Movable,
}

/// Page state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageState {
    Free,
    ActiveAnon,
    InactiveAnon,
    ActiveFile,
    InactiveFile,
    Unevictable,
    Dirty,
    Writeback,
    Slab,
    PageTable,
    Kernel,
}

/// Per-zone counters
#[derive(Debug, Clone, Default)]
pub struct ZoneCounters {
    pub free_pages: u64,
    pub active_anon: u64,
    pub inactive_anon: u64,
    pub active_file: u64,
    pub inactive_file: u64,
    pub unevictable: u64,
    pub dirty: u64,
    pub writeback: u64,
    pub slab_reclaimable: u64,
    pub slab_unreclaimable: u64,
    pub page_tables: u64,
    pub kernel_stack: u64,
    pub bounce: u64,
    pub mapped: u64,
    pub shmem: u64,
}

impl ZoneCounters {
    pub fn total_pages(&self) -> u64 {
        self.free_pages + self.active_anon + self.inactive_anon + self.active_file
            + self.inactive_file + self.unevictable + self.dirty + self.writeback
            + self.slab_reclaimable + self.slab_unreclaimable + self.page_tables
            + self.kernel_stack + self.bounce + self.mapped + self.shmem
    }

    pub fn reclaimable(&self) -> u64 { self.inactive_file + self.inactive_anon + self.slab_reclaimable }
    pub fn file_backed(&self) -> u64 { self.active_file + self.inactive_file }
    pub fn anon_pages(&self) -> u64 { self.active_anon + self.inactive_anon }
}

/// Zone descriptor with watermarks
#[derive(Debug, Clone)]
pub struct ZoneDesc {
    pub zone: VmZone,
    pub node: u32,
    pub counters: ZoneCounters,
    pub watermark_min: u64,
    pub watermark_low: u64,
    pub watermark_high: u64,
    pub managed_pages: u64,
}

impl ZoneDesc {
    pub fn new(zone: VmZone, node: u32, managed: u64) -> Self {
        let min = managed / 256;
        Self {
            zone, node, counters: ZoneCounters::default(),
            watermark_min: min, watermark_low: min * 2, watermark_high: min * 3,
            managed_pages: managed,
        }
    }

    pub fn is_below_min(&self) -> bool { self.counters.free_pages < self.watermark_min }
    pub fn is_below_low(&self) -> bool { self.counters.free_pages < self.watermark_low }
    pub fn is_below_high(&self) -> bool { self.counters.free_pages < self.watermark_high }
    pub fn pressure(&self) -> f64 { if self.managed_pages == 0 { 0.0 } else { 1.0 - (self.counters.free_pages as f64 / self.managed_pages as f64) } }
}

/// Swap stats
#[derive(Debug, Clone, Default)]
pub struct SwapCounters {
    pub total_pages: u64,
    pub used_pages: u64,
    pub swap_in: u64,
    pub swap_out: u64,
    pub swap_in_bytes: u64,
    pub swap_out_bytes: u64,
}

impl SwapCounters {
    pub fn usage(&self) -> f64 { if self.total_pages == 0 { 0.0 } else { self.used_pages as f64 / self.total_pages as f64 } }
}

/// Page allocation rate sample
#[derive(Debug, Clone)]
pub struct VmRateSample {
    pub ts: u64,
    pub alloc_rate: u64,
    pub free_rate: u64,
    pub fault_rate: u64,
    pub scan_rate: u64,
    pub steal_rate: u64,
}

/// Reclaim statistics
#[derive(Debug, Clone, Default)]
pub struct ReclaimCounters {
    pub pages_scanned: u64,
    pub pages_reclaimed: u64,
    pub pages_skipped: u64,
    pub kswapd_wake: u64,
    pub direct_reclaim: u64,
    pub compact_stall: u64,
    pub compact_success: u64,
    pub oom_kills: u64,
}

impl ReclaimCounters {
    pub fn reclaim_efficiency(&self) -> f64 { if self.pages_scanned == 0 { 0.0 } else { self.pages_reclaimed as f64 / self.pages_scanned as f64 } }
}

/// VMStat summary
#[derive(Debug, Clone, Default)]
pub struct VmStatSummary {
    pub total_memory_pages: u64,
    pub free_pages: u64,
    pub available_pages: u64,
    pub file_pages: u64,
    pub anon_pages: u64,
    pub dirty_pages: u64,
    pub writeback_pages: u64,
    pub slab_pages: u64,
    pub zones: usize,
    pub pressure_avg: f64,
}

/// Holistic VMStat tracker
pub struct HolisticVmstatTracker {
    zones: BTreeMap<u64, ZoneDesc>,
    swap: SwapCounters,
    reclaim: ReclaimCounters,
    rate_history: Vec<VmRateSample>,
    summary: VmStatSummary,
    next_zone_key: u64,
    max_history: usize,
}

impl HolisticVmstatTracker {
    pub fn new(max_history: usize) -> Self {
        Self {
            zones: BTreeMap::new(), swap: SwapCounters::default(),
            reclaim: ReclaimCounters::default(), rate_history: Vec::new(),
            summary: VmStatSummary::default(), next_zone_key: 1,
            max_history,
        }
    }

    pub fn add_zone(&mut self, zone: VmZone, node: u32, managed: u64) -> u64 {
        let key = self.next_zone_key; self.next_zone_key += 1;
        self.zones.insert(key, ZoneDesc::new(zone, node, managed));
        key
    }

    pub fn update_counters(&mut self, zone_key: u64, counters: ZoneCounters) {
        if let Some(z) = self.zones.get_mut(&zone_key) { z.counters = counters; }
    }

    pub fn update_swap(&mut self, total: u64, used: u64, sin: u64, sout: u64) {
        self.swap.total_pages = total;
        self.swap.used_pages = used;
        self.swap.swap_in += sin;
        self.swap.swap_out += sout;
        self.swap.swap_in_bytes += sin * 4096;
        self.swap.swap_out_bytes += sout * 4096;
    }

    pub fn record_reclaim(&mut self, scanned: u64, reclaimed: u64, skipped: u64) {
        self.reclaim.pages_scanned += scanned;
        self.reclaim.pages_reclaimed += reclaimed;
        self.reclaim.pages_skipped += skipped;
    }

    pub fn record_kswapd_wake(&mut self) { self.reclaim.kswapd_wake += 1; }
    pub fn record_direct_reclaim(&mut self) { self.reclaim.direct_reclaim += 1; }

    pub fn record_rate(&mut self, sample: VmRateSample) {
        self.rate_history.push(sample);
        if self.rate_history.len() > self.max_history { self.rate_history.remove(0); }
    }

    pub fn zones_under_pressure(&self) -> Vec<u64> {
        self.zones.iter().filter(|(_, z)| z.is_below_low()).map(|(&k, _)| k).collect()
    }

    pub fn recompute(&mut self) {
        self.summary.zones = self.zones.len();
        self.summary.total_memory_pages = self.zones.values().map(|z| z.managed_pages).sum();
        self.summary.free_pages = self.zones.values().map(|z| z.counters.free_pages).sum();
        self.summary.file_pages = self.zones.values().map(|z| z.counters.file_backed()).sum();
        self.summary.anon_pages = self.zones.values().map(|z| z.counters.anon_pages()).sum();
        self.summary.dirty_pages = self.zones.values().map(|z| z.counters.dirty).sum();
        self.summary.writeback_pages = self.zones.values().map(|z| z.counters.writeback).sum();
        self.summary.slab_pages = self.zones.values().map(|z| z.counters.slab_reclaimable + z.counters.slab_unreclaimable).sum();
        self.summary.available_pages = self.summary.free_pages + self.zones.values().map(|z| z.counters.reclaimable()).sum::<u64>();
        if !self.zones.is_empty() {
            self.summary.pressure_avg = self.zones.values().map(|z| z.pressure()).sum::<f64>() / self.zones.len() as f64;
        }
    }

    pub fn zone(&self, key: u64) -> Option<&ZoneDesc> { self.zones.get(&key) }
    pub fn swap(&self) -> &SwapCounters { &self.swap }
    pub fn reclaim(&self) -> &ReclaimCounters { &self.reclaim }
    pub fn summary(&self) -> &VmStatSummary { &self.summary }
    pub fn rate_history(&self) -> &[VmRateSample] { &self.rate_history }
}
