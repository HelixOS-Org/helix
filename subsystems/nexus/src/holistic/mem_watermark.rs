//! # Holistic Memory Watermark
//!
//! Memory zone watermark management:
//! - Min/low/high watermark calculation
//! - Per-zone free page tracking
//! - Watermark boost and throttling
//! - kswapd wake-up threshold control
//! - Direct reclaim trigger points
//! - Fragmentation-aware watermark adjustment

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Watermark level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WatermarkLevel {
    Min,
    Low,
    High,
    Preallocate,
}

/// Zone type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ZoneType {
    Dma,
    Dma32,
    Normal,
    HighMem,
    Movable,
}

/// Watermark thresholds for a zone
#[derive(Debug, Clone)]
pub struct ZoneWatermarks {
    pub min_pages: u64,
    pub low_pages: u64,
    pub high_pages: u64,
    pub preallocate_pages: u64,
    pub boost_pages: u64,
}

impl ZoneWatermarks {
    #[inline]
    pub fn calculate(total_pages: u64, managed_pages: u64) -> Self {
        let base = (managed_pages / 256).max(32);
        let min = base;
        let low = min + min / 4;
        let high = low + min / 2;
        let prealloc = high + min / 4;
        Self {
            min_pages: min, low_pages: low, high_pages: high,
            preallocate_pages: prealloc, boost_pages: 0,
        }
    }

    #[inline(always)]
    pub fn boosted_min(&self) -> u64 { self.min_pages + self.boost_pages }
    #[inline(always)]
    pub fn boosted_low(&self) -> u64 { self.low_pages + self.boost_pages }
}

/// Memory zone state
#[derive(Debug, Clone)]
pub struct MemZone {
    pub zone_type: ZoneType,
    pub node_id: u32,
    pub total_pages: u64,
    pub managed_pages: u64,
    pub free_pages: u64,
    pub reserved_pages: u64,
    pub watermarks: ZoneWatermarks,
    pub kswapd_active: bool,
    pub direct_reclaim_count: u64,
    pub kswapd_wake_count: u64,
    pub alloc_stall_count: u64,
    pub fragmentation_index: f64,
    pub last_boost_ts: u64,
}

impl MemZone {
    pub fn new(zone_type: ZoneType, node_id: u32, total_pages: u64) -> Self {
        let managed = total_pages;
        let wm = ZoneWatermarks::calculate(total_pages, managed);
        Self {
            zone_type, node_id, total_pages, managed_pages: managed,
            free_pages: total_pages, reserved_pages: 0, watermarks: wm,
            kswapd_active: false, direct_reclaim_count: 0,
            kswapd_wake_count: 0, alloc_stall_count: 0,
            fragmentation_index: 0.0, last_boost_ts: 0,
        }
    }

    #[inline]
    pub fn current_level(&self) -> WatermarkLevel {
        let free = self.free_pages;
        if free <= self.watermarks.boosted_min() { WatermarkLevel::Min }
        else if free <= self.watermarks.boosted_low() { WatermarkLevel::Low }
        else if free <= self.watermarks.high_pages { WatermarkLevel::High }
        else { WatermarkLevel::Preallocate }
    }

    #[inline(always)]
    pub fn needs_kswapd(&self) -> bool {
        self.free_pages <= self.watermarks.boosted_low()
    }

    #[inline(always)]
    pub fn needs_direct_reclaim(&self) -> bool {
        self.free_pages <= self.watermarks.boosted_min()
    }

    #[inline(always)]
    pub fn free_ratio(&self) -> f64 {
        if self.managed_pages == 0 { 0.0 } else { self.free_pages as f64 / self.managed_pages as f64 }
    }

    pub fn alloc_pages(&mut self, count: u64) -> bool {
        if self.free_pages < count {
            self.alloc_stall_count += 1;
            return false;
        }
        self.free_pages -= count;
        if self.needs_kswapd() && !self.kswapd_active {
            self.kswapd_active = true;
            self.kswapd_wake_count += 1;
        }
        if self.needs_direct_reclaim() {
            self.direct_reclaim_count += 1;
        }
        true
    }

    #[inline]
    pub fn free_allocated(&mut self, count: u64) {
        self.free_pages = (self.free_pages + count).min(self.managed_pages);
        if self.free_pages > self.watermarks.high_pages {
            self.kswapd_active = false;
        }
    }

    #[inline(always)]
    pub fn apply_boost(&mut self, pages: u64, ts: u64) {
        self.watermarks.boost_pages = pages;
        self.last_boost_ts = ts;
    }

    #[inline]
    pub fn decay_boost(&mut self, ts: u64) {
        if self.watermarks.boost_pages > 0 {
            let elapsed = ts.saturating_sub(self.last_boost_ts);
            let decay = elapsed / 1_000_000; // decay 1 page per ms
            self.watermarks.boost_pages = self.watermarks.boost_pages.saturating_sub(decay);
        }
    }
}

/// Order-based watermark for anti-fragmentation
#[derive(Debug, Clone)]
pub struct OrderWatermark {
    pub order: u8,
    pub min_free_pages: u64,
    pub current_free: u64,
    pub compaction_needed: bool,
}

impl OrderWatermark {
    pub fn new(order: u8, zone_managed: u64) -> Self {
        let base = zone_managed >> (order as u32 + 4);
        Self { order, min_free_pages: base.max(1), current_free: 0, compaction_needed: false }
    }

    #[inline(always)]
    pub fn update(&mut self, free: u64) {
        self.current_free = free;
        self.compaction_needed = free < self.min_free_pages;
    }
}

/// Watermark stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct WatermarkStats {
    pub total_zones: usize,
    pub zones_below_low: usize,
    pub zones_below_min: usize,
    pub total_kswapd_wakes: u64,
    pub total_direct_reclaims: u64,
    pub total_alloc_stalls: u64,
    pub avg_free_ratio: f64,
    pub worst_free_ratio: f64,
    pub total_boosted_zones: usize,
}

/// Holistic memory watermark manager
pub struct HolisticMemWatermark {
    zones: BTreeMap<(u32, ZoneType), MemZone>,
    order_watermarks: BTreeMap<(u32, ZoneType, u8), OrderWatermark>,
    stats: WatermarkStats,
    vm_min_free_kbytes: u64,
    watermark_scale_factor: u64,
}

impl HolisticMemWatermark {
    pub fn new(min_free_kb: u64) -> Self {
        Self {
            zones: BTreeMap::new(), order_watermarks: BTreeMap::new(),
            stats: WatermarkStats::default(),
            vm_min_free_kbytes: min_free_kb, watermark_scale_factor: 10,
        }
    }

    #[inline]
    pub fn add_zone(&mut self, zone: MemZone) {
        let key = (zone.node_id, zone.zone_type);
        for order in 0..11u8 {
            let owm = OrderWatermark::new(order, zone.managed_pages);
            self.order_watermarks.insert((zone.node_id, zone.zone_type, order), owm);
        }
        self.zones.insert(key, zone);
    }

    #[inline]
    pub fn alloc_pages(&mut self, node: u32, zone_type: ZoneType, count: u64) -> bool {
        if let Some(z) = self.zones.get_mut(&(node, zone_type)) {
            z.alloc_pages(count)
        } else { false }
    }

    #[inline(always)]
    pub fn free_pages(&mut self, node: u32, zone_type: ZoneType, count: u64) {
        if let Some(z) = self.zones.get_mut(&(node, zone_type)) { z.free_allocated(count); }
    }

    #[inline(always)]
    pub fn apply_boost(&mut self, node: u32, zone_type: ZoneType, pages: u64, ts: u64) {
        if let Some(z) = self.zones.get_mut(&(node, zone_type)) { z.apply_boost(pages, ts); }
    }

    #[inline]
    pub fn recalculate_watermarks(&mut self) {
        let keys: Vec<_> = self.zones.keys().cloned().collect();
        for key in keys {
            if let Some(z) = self.zones.get_mut(&key) {
                z.watermarks = ZoneWatermarks::calculate(z.total_pages, z.managed_pages);
                let scale_adj = z.managed_pages * self.watermark_scale_factor / 10000;
                z.watermarks.low_pages += scale_adj;
                z.watermarks.high_pages += scale_adj * 2;
            }
        }
    }

    pub fn recompute(&mut self) {
        self.stats.total_zones = self.zones.len();
        self.stats.zones_below_low = self.zones.values().filter(|z| z.current_level() <= WatermarkLevel::Low).count();
        self.stats.zones_below_min = self.zones.values().filter(|z| z.current_level() == WatermarkLevel::Min).count();
        self.stats.total_kswapd_wakes = self.zones.values().map(|z| z.kswapd_wake_count).sum();
        self.stats.total_direct_reclaims = self.zones.values().map(|z| z.direct_reclaim_count).sum();
        self.stats.total_alloc_stalls = self.zones.values().map(|z| z.alloc_stall_count).sum();
        let ratios: Vec<f64> = self.zones.values().map(|z| z.free_ratio()).collect();
        self.stats.avg_free_ratio = if ratios.is_empty() { 0.0 } else { ratios.iter().sum::<f64>() / ratios.len() as f64 };
        self.stats.worst_free_ratio = ratios.iter().cloned().fold(1.0_f64, f64::min);
        self.stats.total_boosted_zones = self.zones.values().filter(|z| z.watermarks.boost_pages > 0).count();
    }

    #[inline(always)]
    pub fn zone(&self, node: u32, zt: ZoneType) -> Option<&MemZone> { self.zones.get(&(node, zt)) }
    #[inline(always)]
    pub fn stats(&self) -> &WatermarkStats { &self.stats }
}
