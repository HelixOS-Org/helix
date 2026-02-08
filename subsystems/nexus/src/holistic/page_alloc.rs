// SPDX-License-Identifier: GPL-2.0
//! Holistic page_alloc — page allocator zone management and buddy system tracking.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Zone type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PageZoneType {
    Dma,
    Dma32,
    Normal,
    HighMem,
    Movable,
    Device,
}

impl PageZoneType {
    pub fn can_reclaim(&self) -> bool {
        matches!(self, Self::Normal | Self::HighMem | Self::Movable)
    }
}

/// GFP flags simplified
#[derive(Debug, Clone, Copy)]
pub struct GfpFlags(pub u32);

impl GfpFlags {
    pub const KERNEL: Self = Self(0x01);
    pub const ATOMIC: Self = Self(0x02);
    pub const NOWAIT: Self = Self(0x04);
    pub const DMA: Self = Self(0x08);
    pub const HIGHMEM: Self = Self(0x10);
    pub const MOVABLE: Self = Self(0x20);
    pub const ZERO: Self = Self(0x40);
    pub const COMP: Self = Self(0x80);
    pub const NORETRY: Self = Self(0x100);
    pub const RETRY: Self = Self(0x200);
    pub const NOFAIL: Self = Self(0x400);

    pub fn contains(&self, flag: GfpFlags) -> bool {
        self.0 & flag.0 != 0
    }

    pub fn is_atomic(&self) -> bool {
        self.contains(Self::ATOMIC)
    }
}

/// Watermark levels
#[derive(Debug, Clone)]
pub struct ZoneWatermarks {
    pub min: u64,
    pub low: u64,
    pub high: u64,
    pub boost: u64,
}

impl ZoneWatermarks {
    pub fn current_level(&self, free: u64) -> WatermarkLevel {
        if free <= self.min { WatermarkLevel::Min }
        else if free <= self.low { WatermarkLevel::Low }
        else if free <= self.high { WatermarkLevel::High }
        else { WatermarkLevel::Above }
    }
}

/// Watermark level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WatermarkLevel {
    Min,
    Low,
    High,
    Above,
}

/// Buddy order stats
#[derive(Debug, Clone)]
pub struct BuddyOrderStats {
    pub free_count: [u64; 11],
    pub total_free_pages: u64,
}

impl BuddyOrderStats {
    pub fn new() -> Self {
        Self { free_count: [0; 11], total_free_pages: 0 }
    }

    pub fn recalculate(&mut self) {
        self.total_free_pages = (0..11).map(|i| self.free_count[i] << i).sum();
    }

    pub fn pages_at_order(&self, order: u8) -> u64 {
        if (order as usize) < 11 { self.free_count[order as usize] << order } else { 0 }
    }

    pub fn highest_available_order(&self) -> u8 {
        for i in (0..11).rev() {
            if self.free_count[i] > 0 { return i as u8; }
        }
        0
    }

    pub fn fragmentation_ratio(&self) -> f64 {
        if self.total_free_pages == 0 { return 1.0; }
        let high_order: u64 = (4..11).map(|i| self.free_count[i] << i).sum();
        1.0 - (high_order as f64 / self.total_free_pages as f64)
    }
}

/// Per-zone allocator state
#[derive(Debug)]
pub struct ZoneAllocState {
    pub zone_type: PageZoneType,
    pub zone_id: u32,
    pub zone_name: String,
    pub start_pfn: u64,
    pub spanned_pages: u64,
    pub present_pages: u64,
    pub managed_pages: u64,
    pub watermarks: ZoneWatermarks,
    pub buddy: BuddyOrderStats,
    pub nr_alloc: u64,
    pub nr_free: u64,
    pub nr_alloc_fail: u64,
    pub per_cpu_pages: u32,
    pub per_cpu_batch: u32,
}

impl ZoneAllocState {
    pub fn new(zone_type: PageZoneType, zone_id: u32, name: String) -> Self {
        Self {
            zone_type, zone_id, zone_name: name,
            start_pfn: 0, spanned_pages: 0, present_pages: 0, managed_pages: 0,
            watermarks: ZoneWatermarks { min: 0, low: 0, high: 0, boost: 0 },
            buddy: BuddyOrderStats::new(),
            nr_alloc: 0, nr_free: 0, nr_alloc_fail: 0,
            per_cpu_pages: 0, per_cpu_batch: 0,
        }
    }

    pub fn free_pages(&self) -> u64 {
        self.buddy.total_free_pages
    }

    pub fn utilization(&self) -> f64 {
        if self.managed_pages == 0 { return 0.0; }
        1.0 - (self.free_pages() as f64 / self.managed_pages as f64)
    }

    pub fn watermark_level(&self) -> WatermarkLevel {
        self.watermarks.current_level(self.free_pages())
    }

    pub fn alloc_fail_rate(&self) -> f64 {
        let total = self.nr_alloc + self.nr_alloc_fail;
        if total == 0 { return 0.0; }
        self.nr_alloc_fail as f64 / total as f64
    }

    pub fn can_satisfy(&self, order: u8) -> bool {
        self.buddy.highest_available_order() >= order
    }
}

/// Allocation request record
#[derive(Debug, Clone)]
pub struct AllocRequest {
    pub order: u8,
    pub gfp: GfpFlags,
    pub zone_preference: PageZoneType,
    pub pid: u32,
    pub success: bool,
    pub latency_ns: u64,
    pub timestamp: u64,
}

/// Page allocator stats
#[derive(Debug, Clone)]
pub struct PageAllocStats {
    pub total_allocs: u64,
    pub total_frees: u64,
    pub total_failures: u64,
    pub high_order_allocs: u64,
    pub high_order_failures: u64,
    pub avg_alloc_latency_ns: u64,
    pub zone_count: u32,
}

/// Main page allocator manager
pub struct HolisticPageAlloc {
    zones: BTreeMap<u32, ZoneAllocState>,
    recent_allocs: Vec<AllocRequest>,
    max_recent: usize,
    stats: PageAllocStats,
    zonelist: Vec<u32>,
}

impl HolisticPageAlloc {
    pub fn new() -> Self {
        Self {
            zones: BTreeMap::new(),
            recent_allocs: Vec::new(),
            max_recent: 4096,
            stats: PageAllocStats {
                total_allocs: 0, total_frees: 0, total_failures: 0,
                high_order_allocs: 0, high_order_failures: 0,
                avg_alloc_latency_ns: 0, zone_count: 0,
            },
            zonelist: Vec::new(),
        }
    }

    pub fn add_zone(&mut self, state: ZoneAllocState) {
        self.stats.zone_count += 1;
        let id = state.zone_id;
        self.zones.insert(id, state);
        self.zonelist.push(id);
    }

    pub fn record_alloc(&mut self, req: AllocRequest) {
        self.stats.total_allocs += 1;
        if req.order >= 4 { self.stats.high_order_allocs += 1; }
        if !req.success {
            self.stats.total_failures += 1;
            if req.order >= 4 { self.stats.high_order_failures += 1; }
        }
        let n = self.stats.total_allocs;
        self.stats.avg_alloc_latency_ns =
            ((self.stats.avg_alloc_latency_ns * (n - 1)) + req.latency_ns) / n;

        if let Some(zone) = self.zones.get_mut(&(req.zone_preference as u32)) {
            if req.success { zone.nr_alloc += 1; } else { zone.nr_alloc_fail += 1; }
        }

        if self.recent_allocs.len() >= self.max_recent {
            self.recent_allocs.remove(0);
        }
        self.recent_allocs.push(req);
    }

    pub fn record_free(&mut self, zone_id: u32, _order: u8) {
        self.stats.total_frees += 1;
        if let Some(zone) = self.zones.get_mut(&zone_id) {
            zone.nr_free += 1;
        }
    }

    pub fn find_zone_for(&self, order: u8, gfp: GfpFlags) -> Option<u32> {
        for &zid in &self.zonelist {
            if let Some(zone) = self.zones.get(&zid) {
                if gfp.contains(GfpFlags::DMA) && zone.zone_type != PageZoneType::Dma { continue; }
                if zone.can_satisfy(order)
                    && zone.watermark_level() >= WatermarkLevel::Low
                {
                    return Some(zid);
                }
            }
        }
        None
    }

    pub fn zones_below_watermark(&self, level: WatermarkLevel) -> Vec<u32> {
        self.zones.iter()
            .filter(|(_, z)| z.watermark_level() < level)
            .map(|(&id, _)| id)
            .collect()
    }

    pub fn total_free_pages(&self) -> u64 {
        self.zones.values().map(|z| z.free_pages()).sum()
    }

    pub fn total_managed_pages(&self) -> u64 {
        self.zones.values().map(|z| z.managed_pages).sum()
    }

    pub fn system_utilization(&self) -> f64 {
        let managed = self.total_managed_pages();
        if managed == 0 { return 0.0; }
        1.0 - (self.total_free_pages() as f64 / managed as f64)
    }

    pub fn stats(&self) -> &PageAllocStats {
        &self.stats
    }
}
