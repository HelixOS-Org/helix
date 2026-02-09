//! # Holistic Swap Manager
//!
//! System-wide swap management with holistic optimization:
//! - Multi-device swap allocation with priority
//! - Swap space pressure tracking and prediction
//! - Zswap/zram compressed swap management
//! - Swap readahead with pattern detection
//! - Process-level swap accounting
//! - Swap device wear leveling for SSDs

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Swap device type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapDeviceType {
    Partition,
    File,
    Zram,
    Zswap,
    NetworkBlock,
}

/// Swap allocation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapAllocStrategy {
    RoundRobin,
    PriorityBased,
    LeastUsed,
    PerformanceTiered,
}

/// Per-swap-device state
#[derive(Debug, Clone)]
pub struct SwapDevice {
    pub device_id: u32,
    pub device_type: SwapDeviceType,
    pub priority: i16,
    pub total_pages: u64,
    pub used_pages: u64,
    pub reserved_pages: u64,
    pub write_count: u64,
    pub read_count: u64,
    pub write_bandwidth_bps: u64,
    pub read_bandwidth_bps: u64,
    pub avg_latency_ns: u64,
    pub compression_ratio: f64, // for zram/zswap
    pub wear_level_pct: f64,    // SSD wear
    pub bad_pages: u32,
}

impl SwapDevice {
    pub fn new(device_id: u32, dtype: SwapDeviceType, priority: i16, total: u64) -> Self {
        Self {
            device_id,
            device_type: dtype,
            priority,
            total_pages: total,
            used_pages: 0,
            reserved_pages: 0,
            write_count: 0,
            read_count: 0,
            write_bandwidth_bps: 0,
            read_bandwidth_bps: 0,
            avg_latency_ns: 0,
            compression_ratio: 1.0,
            wear_level_pct: 0.0,
            bad_pages: 0,
        }
    }

    #[inline(always)]
    pub fn free_pages(&self) -> u64 {
        self.total_pages.saturating_sub(self.used_pages + self.reserved_pages)
    }

    #[inline(always)]
    pub fn usage_ratio(&self) -> f64 {
        if self.total_pages == 0 { return 0.0; }
        self.used_pages as f64 / self.total_pages as f64
    }

    #[inline(always)]
    pub fn is_full(&self) -> bool {
        self.free_pages() == 0
    }

    #[inline]
    pub fn effective_capacity(&self) -> u64 {
        if self.compression_ratio > 0.01 {
            (self.total_pages as f64 / self.compression_ratio) as u64
        } else { self.total_pages }
    }

    #[inline(always)]
    pub fn is_healthy(&self) -> bool {
        self.wear_level_pct < 90.0 && self.bad_pages < 100
    }
}

/// Per-process swap accounting
#[derive(Debug, Clone)]
pub struct ProcessSwapUsage {
    pub process_id: u64,
    pub swapped_pages: u64,
    pub swap_ins: u64,
    pub swap_outs: u64,
    pub major_faults: u64,
    pub total_swap_latency_ns: u64,
    pub swap_limit: Option<u64>,
}

impl ProcessSwapUsage {
    pub fn new(process_id: u64) -> Self {
        Self {
            process_id,
            swapped_pages: 0,
            swap_ins: 0,
            swap_outs: 0,
            major_faults: 0,
            total_swap_latency_ns: 0,
            swap_limit: None,
        }
    }

    #[inline]
    pub fn avg_swap_latency(&self) -> u64 {
        let total_ops = self.swap_ins + self.swap_outs;
        if total_ops == 0 { return 0; }
        self.total_swap_latency_ns / total_ops
    }

    #[inline]
    pub fn is_thrashing(&self) -> bool {
        // Thrashing if high major fault rate and roughly equal in/out
        if self.swap_ins < 100 { return false; }
        let ratio = if self.swap_outs > 0 {
            self.swap_ins as f64 / self.swap_outs as f64
        } else { 0.0 };
        ratio > 0.5 && ratio < 2.0 && self.major_faults > 1000
    }

    #[inline]
    pub fn exceeds_limit(&self) -> bool {
        if let Some(limit) = self.swap_limit {
            self.swapped_pages > limit
        } else { false }
    }
}

/// Zswap/zram state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CompressedSwapState {
    pub stored_pages: u64,
    pub compressed_bytes: u64,
    pub original_bytes: u64,
    pub reject_count: u64,
    pub writeback_count: u64,
    pub pool_size_bytes: u64,
    pub pool_limit_bytes: u64,
}

impl CompressedSwapState {
    pub fn new(pool_limit: u64) -> Self {
        Self {
            stored_pages: 0,
            compressed_bytes: 0,
            original_bytes: 0,
            reject_count: 0,
            writeback_count: 0,
            pool_size_bytes: 0,
            pool_limit_bytes: pool_limit,
        }
    }

    #[inline(always)]
    pub fn compression_ratio(&self) -> f64 {
        if self.compressed_bytes == 0 { return 1.0; }
        self.original_bytes as f64 / self.compressed_bytes as f64
    }

    #[inline(always)]
    pub fn pool_usage_ratio(&self) -> f64 {
        if self.pool_limit_bytes == 0 { return 0.0; }
        self.pool_size_bytes as f64 / self.pool_limit_bytes as f64
    }

    #[inline(always)]
    pub fn is_pool_full(&self) -> bool {
        self.pool_usage_ratio() > 0.95
    }

    #[inline(always)]
    pub fn savings_bytes(&self) -> u64 {
        self.original_bytes.saturating_sub(self.compressed_bytes)
    }
}

/// Holistic Swap Manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticSwapMgrStats {
    pub total_devices: usize,
    pub total_swap_pages: u64,
    pub used_swap_pages: u64,
    pub global_swap_pressure: f64,
    pub thrashing_processes: usize,
    pub compressed_savings_bytes: u64,
}

/// Holistic Swap Manager
pub struct HolisticSwapMgr {
    devices: BTreeMap<u32, SwapDevice>,
    processes: BTreeMap<u64, ProcessSwapUsage>,
    compressed: Option<CompressedSwapState>,
    strategy: SwapAllocStrategy,
    stats: HolisticSwapMgrStats,
}

impl HolisticSwapMgr {
    pub fn new(strategy: SwapAllocStrategy) -> Self {
        Self {
            devices: BTreeMap::new(),
            processes: BTreeMap::new(),
            compressed: None,
            strategy,
            stats: HolisticSwapMgrStats::default(),
        }
    }

    #[inline(always)]
    pub fn add_device(&mut self, dev: SwapDevice) {
        self.devices.insert(dev.device_id, dev);
    }

    #[inline(always)]
    pub fn set_compressed(&mut self, state: CompressedSwapState) {
        self.compressed = Some(state);
    }

    #[inline(always)]
    pub fn register_process(&mut self, pid: u64) {
        self.processes.entry(pid)
            .or_insert_with(|| ProcessSwapUsage::new(pid));
    }

    /// Select best swap device for allocation
    pub fn select_device(&self, pages_needed: u64) -> Option<u32> {
        match self.strategy {
            SwapAllocStrategy::PriorityBased => {
                self.devices.values()
                    .filter(|d| d.free_pages() >= pages_needed && d.is_healthy())
                    .max_by_key(|d| d.priority)
                    .map(|d| d.device_id)
            }
            SwapAllocStrategy::LeastUsed => {
                self.devices.values()
                    .filter(|d| d.free_pages() >= pages_needed && d.is_healthy())
                    .min_by(|a, b| a.usage_ratio().partial_cmp(&b.usage_ratio())
                        .unwrap_or(core::cmp::Ordering::Equal))
                    .map(|d| d.device_id)
            }
            SwapAllocStrategy::PerformanceTiered => {
                // Prefer compressed swap first, then fastest device
                if let Some(ref cs) = self.compressed {
                    if !cs.is_pool_full() {
                        // Use zswap/zram — return special ID 0xFFFF
                        return Some(0xFFFF);
                    }
                }
                self.devices.values()
                    .filter(|d| d.free_pages() >= pages_needed && d.is_healthy())
                    .min_by_key(|d| d.avg_latency_ns)
                    .map(|d| d.device_id)
            }
            _ => {
                self.devices.values()
                    .filter(|d| d.free_pages() >= pages_needed)
                    .next()
                    .map(|d| d.device_id)
            }
        }
    }

    /// Record a swap-out event
    pub fn record_swap_out(&mut self, pid: u64, device_id: u32, pages: u64, latency_ns: u64) {
        if let Some(dev) = self.devices.get_mut(&device_id) {
            dev.used_pages += pages;
            dev.write_count += 1;
        }
        if let Some(proc) = self.processes.get_mut(&pid) {
            proc.swapped_pages += pages;
            proc.swap_outs += 1;
            proc.total_swap_latency_ns += latency_ns;
        }
        self.recompute();
    }

    /// Record a swap-in event
    pub fn record_swap_in(&mut self, pid: u64, device_id: u32, pages: u64, latency_ns: u64) {
        if let Some(dev) = self.devices.get_mut(&device_id) {
            dev.used_pages = dev.used_pages.saturating_sub(pages);
            dev.read_count += 1;
        }
        if let Some(proc) = self.processes.get_mut(&pid) {
            proc.swapped_pages = proc.swapped_pages.saturating_sub(pages);
            proc.swap_ins += 1;
            proc.major_faults += pages;
            proc.total_swap_latency_ns += latency_ns;
        }
        self.recompute();
    }

    fn recompute(&mut self) {
        self.stats.total_devices = self.devices.len();
        self.stats.total_swap_pages = self.devices.values().map(|d| d.total_pages).sum();
        self.stats.used_swap_pages = self.devices.values().map(|d| d.used_pages).sum();
        self.stats.global_swap_pressure = if self.stats.total_swap_pages > 0 {
            self.stats.used_swap_pages as f64 / self.stats.total_swap_pages as f64
        } else { 0.0 };
        self.stats.thrashing_processes = self.processes.values()
            .filter(|p| p.is_thrashing()).count();
        self.stats.compressed_savings_bytes = self.compressed.as_ref()
            .map(|c| c.savings_bytes()).unwrap_or(0);
    }

    #[inline(always)]
    pub fn device(&self, id: u32) -> Option<&SwapDevice> { self.devices.get(&id) }
    #[inline(always)]
    pub fn process_swap(&self, pid: u64) -> Option<&ProcessSwapUsage> { self.processes.get(&pid) }
    #[inline(always)]
    pub fn stats(&self) -> &HolisticSwapMgrStats { &self.stats }
    #[inline(always)]
    pub fn total_free_swap(&self) -> u64 { self.devices.values().map(|d| d.free_pages()).sum() }
}

// ============================================================================
// Merged from swap_mgr_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapAreaType {
    Partition,
    File,
    Zram,
    Network,
}

/// Swap area priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SwapPriority(pub i32);

/// Swap area
#[derive(Debug)]
pub struct SwapAreaV2 {
    pub id: u64,
    pub area_type: SwapAreaType,
    pub priority: SwapPriority,
    pub total_pages: u64,
    pub used_pages: u64,
    pub bad_pages: u64,
    pub swap_in_count: u64,
    pub swap_out_count: u64,
    pub active: bool,
}

impl SwapAreaV2 {
    pub fn new(id: u64, at: SwapAreaType, prio: i32, total: u64) -> Self {
        Self { id, area_type: at, priority: SwapPriority(prio), total_pages: total, used_pages: 0, bad_pages: 0, swap_in_count: 0, swap_out_count: 0, active: true }
    }

    #[inline(always)]
    pub fn usage_ratio(&self) -> f64 { if self.total_pages == 0 { 0.0 } else { self.used_pages as f64 / self.total_pages as f64 } }

    #[inline(always)]
    pub fn swap_out(&mut self) -> bool {
        if self.used_pages < self.total_pages - self.bad_pages { self.used_pages += 1; self.swap_out_count += 1; true }
        else { false }
    }

    #[inline(always)]
    pub fn swap_in(&mut self) {
        if self.used_pages > 0 { self.used_pages -= 1; self.swap_in_count += 1; }
    }
}

/// Swap entry
#[derive(Debug)]
pub struct SwapEntryV2 {
    pub area_id: u64,
    pub offset: u64,
    pub pid: u64,
    pub vaddr: u64,
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SwapMgrV2Stats {
    pub total_areas: u32,
    pub active_areas: u32,
    pub total_capacity_pages: u64,
    pub total_used_pages: u64,
    pub total_swap_in: u64,
    pub total_swap_out: u64,
    pub overall_usage_ratio: f64,
}

/// Main holistic swap manager v2
pub struct HolisticSwapMgrV2 {
    areas: BTreeMap<u64, SwapAreaV2>,
    entries: BTreeMap<u64, SwapEntryV2>,
    next_offset: u64,
}

impl HolisticSwapMgrV2 {
    pub fn new() -> Self { Self { areas: BTreeMap::new(), entries: BTreeMap::new(), next_offset: 0 } }

    #[inline(always)]
    pub fn add_area(&mut self, id: u64, at: SwapAreaType, prio: i32, total: u64) { self.areas.insert(id, SwapAreaV2::new(id, at, prio, total)); }

    pub fn swap_out(&mut self, pid: u64, vaddr: u64) -> Option<u64> {
        let mut sorted: Vec<&mut SwapAreaV2> = self.areas.values_mut().filter(|a| a.active).collect();
        sorted.sort_by(|a, b| b.priority.cmp(&a.priority));
        for area in sorted {
            if area.swap_out() {
                let offset = self.next_offset; self.next_offset += 1;
                self.entries.insert(offset, SwapEntryV2 { area_id: area.id, offset, pid, vaddr });
                return Some(offset);
            }
        }
        None
    }

    #[inline]
    pub fn swap_in(&mut self, offset: u64) -> bool {
        if let Some(entry) = self.entries.remove(&offset) {
            if let Some(area) = self.areas.get_mut(&entry.area_id) { area.swap_in(); }
            true
        } else { false }
    }

    #[inline]
    pub fn stats(&self) -> SwapMgrV2Stats {
        let active = self.areas.values().filter(|a| a.active).count() as u32;
        let cap: u64 = self.areas.values().map(|a| a.total_pages).sum();
        let used: u64 = self.areas.values().map(|a| a.used_pages).sum();
        let si: u64 = self.areas.values().map(|a| a.swap_in_count).sum();
        let so: u64 = self.areas.values().map(|a| a.swap_out_count).sum();
        let ratio = if cap == 0 { 0.0 } else { used as f64 / cap as f64 };
        SwapMgrV2Stats { total_areas: self.areas.len() as u32, active_areas: active, total_capacity_pages: cap, total_used_pages: used, total_swap_in: si, total_swap_out: so, overall_usage_ratio: ratio }
    }
}

// ============================================================================
// Merged from swap_mgr_v3
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapV3Compressor {
    Lzo,
    Lz4,
    Zstd,
    Deflate,
    Lz4hc,
    None,
}

/// Swap slot state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapV3SlotState {
    Free,
    Allocated,
    Compressed,
    Writeback,
    Bad,
    Reserved,
}

/// Swap device type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapV3DeviceType {
    Partition,
    File,
    Zram,
    Network,
}

/// A compressed swap cache entry.
#[derive(Debug, Clone)]
pub struct ZswapEntry {
    pub page_pfn: u64,
    pub compressed_size: u32,
    pub original_size: u32,
    pub compressor: SwapV3Compressor,
    pub pool_id: u64,
    pub lru_timestamp: u64,
    pub reference_count: u32,
}

impl ZswapEntry {
    pub fn new(page_pfn: u64, compressed_size: u32, compressor: SwapV3Compressor) -> Self {
        Self {
            page_pfn,
            compressed_size,
            original_size: 4096,
            compressor,
            pool_id: 0,
            lru_timestamp: 0,
            reference_count: 1,
        }
    }

    #[inline]
    pub fn compression_ratio(&self) -> f64 {
        if self.original_size == 0 {
            return 1.0;
        }
        self.compressed_size as f64 / self.original_size as f64
    }

    #[inline(always)]
    pub fn savings_bytes(&self) -> u32 {
        self.original_size.saturating_sub(self.compressed_size)
    }
}

/// A swap cluster (group of contiguous slots).
#[derive(Debug, Clone)]
pub struct SwapV3Cluster {
    pub cluster_id: u64,
    pub start_offset: u64,
    pub slot_count: u32,
    pub free_slots: u32,
    pub flags: u32,
    pub owner_cpu: Option<u32>,
}

impl SwapV3Cluster {
    pub fn new(cluster_id: u64, start_offset: u64, slot_count: u32) -> Self {
        Self {
            cluster_id,
            start_offset,
            slot_count,
            free_slots: slot_count,
            flags: 0,
            owner_cpu: None,
        }
    }

    #[inline]
    pub fn try_alloc(&mut self) -> Option<u64> {
        if self.free_slots > 0 {
            self.free_slots -= 1;
            let offset = self.start_offset + (self.slot_count - self.free_slots - 1) as u64;
            Some(offset)
        } else {
            None
        }
    }

    #[inline]
    pub fn free_slot(&mut self) {
        if self.free_slots < self.slot_count {
            self.free_slots += 1;
        }
    }

    #[inline(always)]
    pub fn is_full(&self) -> bool {
        self.free_slots == 0
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.free_slots == self.slot_count
    }
}

/// A swap device/area descriptor.
#[derive(Debug, Clone)]
pub struct SwapV3Area {
    pub area_id: u64,
    pub name: String,
    pub device_type: SwapV3DeviceType,
    pub total_slots: u64,
    pub free_slots: u64,
    pub priority: i32,
    pub clusters: Vec<SwapV3Cluster>,
    pub page_ins: u64,
    pub page_outs: u64,
    pub discard_supported: bool,
}

impl SwapV3Area {
    pub fn new(area_id: u64, name: String, device_type: SwapV3DeviceType, total_slots: u64) -> Self {
        Self {
            area_id,
            name,
            device_type,
            total_slots,
            free_slots: total_slots,
            priority: 0,
            clusters: Vec::new(),
            page_ins: 0,
            page_outs: 0,
            discard_supported: false,
        }
    }

    #[inline]
    pub fn utilization_percent(&self) -> f64 {
        if self.total_slots == 0 {
            return 0.0;
        }
        let used = self.total_slots - self.free_slots;
        (used as f64 / self.total_slots as f64) * 100.0
    }
}

/// Statistics for the swap manager V3.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SwapMgrV3Stats {
    pub total_areas: u64,
    pub total_clusters: u64,
    pub zswap_entries: u64,
    pub zswap_stored_bytes: u64,
    pub zswap_compressed_bytes: u64,
    pub page_ins: u64,
    pub page_outs: u64,
    pub zswap_hits: u64,
    pub zswap_misses: u64,
    pub zswap_evictions: u64,
    pub cluster_alloc_count: u64,
    pub slot_alloc_count: u64,
}

/// Main holistic swap manager V3.
pub struct HolisticSwapMgrV3 {
    pub areas: BTreeMap<u64, SwapV3Area>,
    pub zswap_cache: BTreeMap<u64, ZswapEntry>,
    pub zswap_max_bytes: u64,
    pub zswap_current_bytes: u64,
    pub default_compressor: SwapV3Compressor,
    pub next_area_id: u64,
    pub next_cluster_id: u64,
    pub stats: SwapMgrV3Stats,
}

impl HolisticSwapMgrV3 {
    pub fn new() -> Self {
        Self {
            areas: BTreeMap::new(),
            zswap_cache: BTreeMap::new(),
            zswap_max_bytes: 256 * 1024 * 1024, // 256 MB default
            zswap_current_bytes: 0,
            default_compressor: SwapV3Compressor::Lz4,
            next_area_id: 1,
            next_cluster_id: 1,
            stats: SwapMgrV3Stats {
                total_areas: 0,
                total_clusters: 0,
                zswap_entries: 0,
                zswap_stored_bytes: 0,
                zswap_compressed_bytes: 0,
                page_ins: 0,
                page_outs: 0,
                zswap_hits: 0,
                zswap_misses: 0,
                zswap_evictions: 0,
                cluster_alloc_count: 0,
                slot_alloc_count: 0,
            },
        }
    }

    #[inline]
    pub fn add_area(&mut self, name: String, device_type: SwapV3DeviceType, total_slots: u64) -> u64 {
        let id = self.next_area_id;
        self.next_area_id += 1;
        let area = SwapV3Area::new(id, name, device_type, total_slots);
        self.areas.insert(id, area);
        self.stats.total_areas += 1;
        id
    }

    pub fn zswap_store(&mut self, page_pfn: u64, compressed_size: u32) -> bool {
        let entry_bytes = compressed_size as u64;
        if self.zswap_current_bytes + entry_bytes > self.zswap_max_bytes {
            self.stats.zswap_evictions += 1;
            return false;
        }
        let entry = ZswapEntry::new(page_pfn, compressed_size, self.default_compressor);
        self.zswap_cache.insert(page_pfn, entry);
        self.zswap_current_bytes += entry_bytes;
        self.stats.zswap_entries += 1;
        self.stats.zswap_stored_bytes += 4096;
        self.stats.zswap_compressed_bytes += entry_bytes;
        true
    }

    #[inline]
    pub fn zswap_load(&mut self, page_pfn: u64) -> Option<&ZswapEntry> {
        if self.zswap_cache.contains_key(&page_pfn) {
            self.stats.zswap_hits += 1;
            self.zswap_cache.get(&page_pfn)
        } else {
            self.stats.zswap_misses += 1;
            None
        }
    }

    pub fn zswap_evict_lru(&mut self) -> u64 {
        let mut evicted = 0u64;
        let mut oldest_pfn = None;
        let mut oldest_ts = u64::MAX;
        for (pfn, entry) in &self.zswap_cache {
            if entry.lru_timestamp < oldest_ts {
                oldest_ts = entry.lru_timestamp;
                oldest_pfn = Some(*pfn);
            }
        }
        if let Some(pfn) = oldest_pfn {
            if let Some(entry) = self.zswap_cache.remove(&pfn) {
                self.zswap_current_bytes = self
                    .zswap_current_bytes
                    .saturating_sub(entry.compressed_size as u64);
                evicted += 1;
            }
        }
        self.stats.zswap_evictions += evicted;
        evicted
    }

    #[inline]
    pub fn zswap_compression_ratio(&self) -> f64 {
        if self.stats.zswap_stored_bytes == 0 {
            return 1.0;
        }
        self.stats.zswap_compressed_bytes as f64 / self.stats.zswap_stored_bytes as f64
    }

    #[inline(always)]
    pub fn area_count(&self) -> usize {
        self.areas.len()
    }

    #[inline(always)]
    pub fn zswap_entry_count(&self) -> usize {
        self.zswap_cache.len()
    }
}
