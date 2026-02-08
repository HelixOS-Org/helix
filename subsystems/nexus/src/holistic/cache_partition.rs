// SPDX-License-Identifier: GPL-2.0
//! Holistic cache_partition — hardware cache partitioning (CAT/CDP).

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Cache level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheLevel {
    L2,
    L3,
}

/// Partition type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionType {
    SharedAll,
    Exclusive,
    CodeDataPriority,
    Overlapping,
}

/// CLOS (Class of Service) entry
#[derive(Debug, Clone)]
pub struct ClosEntry {
    pub id: u32,
    pub bitmask: u64,
    pub level: CacheLevel,
    pub ways_allocated: u32,
    pub total_ways: u32,
    pub processes: Vec<u64>,
}

impl ClosEntry {
    pub fn new(id: u32, level: CacheLevel, total_ways: u32) -> Self {
        Self {
            id, bitmask: (1u64 << total_ways) - 1, level,
            ways_allocated: total_ways, total_ways, processes: Vec::new(),
        }
    }

    pub fn set_ways(&mut self, mask: u64) {
        self.bitmask = mask;
        self.ways_allocated = mask.count_ones();
    }

    pub fn assign_process(&mut self, pid: u64) {
        if !self.processes.contains(&pid) { self.processes.push(pid); }
    }

    pub fn remove_process(&mut self, pid: u64) {
        self.processes.retain(|&p| p != pid);
    }

    pub fn utilization(&self) -> f64 {
        if self.total_ways == 0 { return 0.0; }
        self.ways_allocated as f64 / self.total_ways as f64
    }
}

/// Cache monitoring data
#[derive(Debug, Clone)]
pub struct CacheMonitorData {
    pub clos_id: u32,
    pub occupancy_bytes: u64,
    pub bandwidth_bytes: u64,
    pub miss_rate: f64,
    pub timestamp: u64,
}

/// CDP (Code Data Priority) config
#[derive(Debug, Clone)]
pub struct CdpConfig {
    pub code_mask: u64,
    pub data_mask: u64,
    pub enabled: bool,
}

impl CdpConfig {
    pub fn new(code_mask: u64, data_mask: u64) -> Self {
        Self { code_mask, data_mask, enabled: true }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct CachePartitionStats {
    pub total_clos: u32,
    pub active_processes: u32,
    pub total_ways: u32,
    pub allocated_ways: u32,
    pub cdp_enabled: bool,
    pub avg_miss_rate: f64,
}

/// Main cache partition manager
pub struct HolisticCachePartition {
    clos_entries: BTreeMap<u32, ClosEntry>,
    monitor_data: Vec<CacheMonitorData>,
    cdp: Option<CdpConfig>,
    cache_level: CacheLevel,
    total_ways: u32,
    next_clos: u32,
}

impl HolisticCachePartition {
    pub fn new(level: CacheLevel, total_ways: u32) -> Self {
        Self {
            clos_entries: BTreeMap::new(), monitor_data: Vec::new(),
            cdp: None, cache_level: level, total_ways, next_clos: 0,
        }
    }

    pub fn create_clos(&mut self) -> u32 {
        let id = self.next_clos;
        self.next_clos += 1;
        self.clos_entries.insert(id, ClosEntry::new(id, self.cache_level, self.total_ways));
        id
    }

    pub fn set_clos_mask(&mut self, clos: u32, mask: u64) {
        if let Some(entry) = self.clos_entries.get_mut(&clos) { entry.set_ways(mask); }
    }

    pub fn assign_process(&mut self, clos: u32, pid: u64) {
        if let Some(entry) = self.clos_entries.get_mut(&clos) { entry.assign_process(pid); }
    }

    pub fn enable_cdp(&mut self, code_mask: u64, data_mask: u64) {
        self.cdp = Some(CdpConfig::new(code_mask, data_mask));
    }

    pub fn record_monitor(&mut self, clos: u32, occ: u64, bw: u64, miss: f64, now: u64) {
        self.monitor_data.push(CacheMonitorData { clos_id: clos, occupancy_bytes: occ, bandwidth_bytes: bw, miss_rate: miss, timestamp: now });
        if self.monitor_data.len() > 4096 { self.monitor_data.drain(..2048); }
    }

    pub fn stats(&self) -> CachePartitionStats {
        let procs: u32 = self.clos_entries.values().map(|c| c.processes.len() as u32).sum();
        let alloc: u32 = self.clos_entries.values().map(|c| c.ways_allocated).sum();
        let rates: Vec<f64> = self.monitor_data.iter().map(|m| m.miss_rate).collect();
        let avg_miss = if rates.is_empty() { 0.0 } else { rates.iter().sum::<f64>() / rates.len() as f64 };
        CachePartitionStats {
            total_clos: self.clos_entries.len() as u32, active_processes: procs,
            total_ways: self.total_ways, allocated_ways: alloc,
            cdp_enabled: self.cdp.is_some(), avg_miss_rate: avg_miss,
        }
    }
}
