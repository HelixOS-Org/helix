//! # Bridge Swap Bridge
//!
//! Swapon/swapoff syscall bridging and swap management:
//! - Swap device/file registration and priority
//! - Swap slot allocation and tracking
//! - Per-process swap usage accounting
//! - Swap readahead and clustering
//! - Swap pressure monitoring
//! - Swap migration statistics

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Swap area type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapAreaType {
    Partition,
    File,
    Zswap,
    Zram,
    Network,
}

/// Swap area state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapAreaState {
    Active,
    Draining,
    Inactive,
    Failed,
}

/// Swap cluster
#[derive(Debug, Clone)]
pub struct SwapCluster {
    pub start_offset: u64,
    pub count: u32,
    pub free: u32,
    pub sequential_alloc: u32,
}

impl SwapCluster {
    pub fn new(offset: u64, count: u32) -> Self {
        Self { start_offset: offset, count, free: count, sequential_alloc: 0 }
    }

    pub fn utilization(&self) -> f64 {
        if self.count == 0 { return 0.0; }
        1.0 - (self.free as f64 / self.count as f64)
    }

    pub fn try_allocate(&mut self) -> Option<u64> {
        if self.free == 0 { return None; }
        self.free -= 1;
        self.sequential_alloc += 1;
        Some(self.start_offset + (self.count - self.free - 1) as u64)
    }

    pub fn release(&mut self) {
        if self.free < self.count { self.free += 1; }
    }
}

/// Swap area descriptor
#[derive(Debug, Clone)]
pub struct SwapArea {
    pub id: u64,
    pub area_type: SwapAreaType,
    pub state: SwapAreaState,
    pub path: String,
    pub priority: i16,
    pub total_slots: u64,
    pub used_slots: u64,
    pub bad_slots: u64,
    pub clusters: Vec<SwapCluster>,
    pub pages_in: u64,
    pub pages_out: u64,
    pub io_errors: u64,
    pub activated_ts: u64,
}

impl SwapArea {
    pub fn new(id: u64, area_type: SwapAreaType, path: String, priority: i16, total: u64, ts: u64) -> Self {
        let cluster_size = 256u32;
        let n_clusters = (total / cluster_size as u64).max(1);
        let mut clusters = Vec::new();
        for i in 0..n_clusters {
            let count = if i == n_clusters - 1 { (total - i * cluster_size as u64) as u32 } else { cluster_size };
            clusters.push(SwapCluster::new(i * cluster_size as u64, count));
        }
        Self {
            id, area_type, state: SwapAreaState::Active, path, priority,
            total_slots: total, used_slots: 0, bad_slots: 0,
            clusters, pages_in: 0, pages_out: 0, io_errors: 0,
            activated_ts: ts,
        }
    }

    pub fn usage_ratio(&self) -> f64 {
        if self.total_slots == 0 { return 0.0; }
        self.used_slots as f64 / self.total_slots as f64
    }

    pub fn free_slots(&self) -> u64 { self.total_slots.saturating_sub(self.used_slots + self.bad_slots) }

    pub fn allocate_slot(&mut self) -> Option<u64> {
        for cluster in &mut self.clusters {
            if let Some(off) = cluster.try_allocate() {
                self.used_slots += 1;
                self.pages_in += 1;
                return Some(off);
            }
        }
        None
    }

    pub fn free_slot(&mut self, offset: u64) {
        let cluster_size = 256u64;
        let idx = (offset / cluster_size) as usize;
        if idx < self.clusters.len() {
            self.clusters[idx].release();
            self.used_slots = self.used_slots.saturating_sub(1);
            self.pages_out += 1;
        }
    }
}

/// Per-process swap info
#[derive(Debug, Clone)]
pub struct ProcessSwapInfo {
    pub pid: u64,
    pub swapped_pages: u64,
    pub swap_in_faults: u64,
    pub swap_out_events: u64,
    pub peak_swapped: u64,
    pub last_swap_ts: u64,
}

impl ProcessSwapInfo {
    pub fn new(pid: u64) -> Self {
        Self { pid, swapped_pages: 0, swap_in_faults: 0, swap_out_events: 0, peak_swapped: 0, last_swap_ts: 0 }
    }

    pub fn record_swap_out(&mut self, pages: u64, ts: u64) {
        self.swapped_pages += pages;
        self.swap_out_events += 1;
        if self.swapped_pages > self.peak_swapped { self.peak_swapped = self.swapped_pages; }
        self.last_swap_ts = ts;
    }

    pub fn record_swap_in(&mut self, pages: u64, ts: u64) {
        self.swapped_pages = self.swapped_pages.saturating_sub(pages);
        self.swap_in_faults += 1;
        self.last_swap_ts = ts;
    }
}

/// Swap bridge stats
#[derive(Debug, Clone, Default)]
pub struct SwapBridgeStats {
    pub total_areas: usize,
    pub active_areas: usize,
    pub total_capacity: u64,
    pub total_used: u64,
    pub total_pages_in: u64,
    pub total_pages_out: u64,
    pub total_io_errors: u64,
    pub tracked_processes: usize,
    pub pressure_percent: f64,
}

/// Bridge swap manager
pub struct BridgeSwapBridge {
    areas: BTreeMap<u64, SwapArea>,
    proc_swap: BTreeMap<u64, ProcessSwapInfo>,
    priority_order: Vec<u64>,
    next_area_id: u64,
    stats: SwapBridgeStats,
}

impl BridgeSwapBridge {
    pub fn new() -> Self {
        Self {
            areas: BTreeMap::new(), proc_swap: BTreeMap::new(),
            priority_order: Vec::new(), next_area_id: 1,
            stats: SwapBridgeStats::default(),
        }
    }

    pub fn swapon(&mut self, area_type: SwapAreaType, path: String, priority: i16, total: u64, ts: u64) -> u64 {
        let id = self.next_area_id;
        self.next_area_id += 1;
        self.areas.insert(id, SwapArea::new(id, area_type, path, priority, total, ts));
        self.rebuild_priority();
        id
    }

    pub fn swapoff(&mut self, area_id: u64) -> bool {
        if let Some(area) = self.areas.get_mut(&area_id) {
            if area.used_slots > 0 {
                area.state = SwapAreaState::Draining;
                false // need to drain first
            } else {
                area.state = SwapAreaState::Inactive;
                self.rebuild_priority();
                true
            }
        } else { false }
    }

    pub fn allocate(&mut self, pid: u64, ts: u64) -> Option<(u64, u64)> {
        for &area_id in &self.priority_order {
            if let Some(area) = self.areas.get_mut(&area_id) {
                if area.state == SwapAreaState::Active {
                    if let Some(offset) = area.allocate_slot() {
                        let info = self.proc_swap.entry(pid).or_insert_with(|| ProcessSwapInfo::new(pid));
                        info.record_swap_out(1, ts);
                        return Some((area_id, offset));
                    }
                }
            }
        }
        None
    }

    pub fn free(&mut self, area_id: u64, offset: u64, pid: u64, ts: u64) {
        if let Some(area) = self.areas.get_mut(&area_id) {
            area.free_slot(offset);
        }
        if let Some(info) = self.proc_swap.get_mut(&pid) {
            info.record_swap_in(1, ts);
        }
    }

    fn rebuild_priority(&mut self) {
        let mut ids: Vec<(i16, u64)> = self.areas.iter()
            .filter(|(_, a)| a.state == SwapAreaState::Active)
            .map(|(&id, a)| (a.priority, id))
            .collect();
        ids.sort_by(|a, b| b.0.cmp(&a.0)); // higher priority first
        self.priority_order = ids.into_iter().map(|(_, id)| id).collect();
    }

    pub fn recompute(&mut self) {
        self.stats.total_areas = self.areas.len();
        self.stats.active_areas = self.areas.values().filter(|a| a.state == SwapAreaState::Active).count();
        self.stats.total_capacity = self.areas.values().map(|a| a.total_slots).sum();
        self.stats.total_used = self.areas.values().map(|a| a.used_slots).sum();
        self.stats.total_pages_in = self.areas.values().map(|a| a.pages_in).sum();
        self.stats.total_pages_out = self.areas.values().map(|a| a.pages_out).sum();
        self.stats.total_io_errors = self.areas.values().map(|a| a.io_errors).sum();
        self.stats.tracked_processes = self.proc_swap.len();
        if self.stats.total_capacity > 0 {
            self.stats.pressure_percent = (self.stats.total_used as f64 / self.stats.total_capacity as f64) * 100.0;
        }
    }

    pub fn area(&self, id: u64) -> Option<&SwapArea> { self.areas.get(&id) }
    pub fn process_swap(&self, pid: u64) -> Option<&ProcessSwapInfo> { self.proc_swap.get(&pid) }
    pub fn stats(&self) -> &SwapBridgeStats { &self.stats }
}
