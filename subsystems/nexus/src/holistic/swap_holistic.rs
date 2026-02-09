// SPDX-License-Identifier: MIT
//! # Holistic Swap Optimization
//!
//! System-wide swap strategy:
//! - Global swap utilization dashboard
//! - Multi-device swap balancing
//! - zswap global compression analytics
//! - Swap I/O bandwidth allocation
//! - Predictive swap space management

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapDeviceType { SSD, HDD, NVMe, ZRAM, Network }

impl SwapDeviceType {
    pub fn latency_estimate_ns(&self) -> u64 {
        match self {
            Self::NVMe => 10_000,
            Self::SSD => 100_000,
            Self::ZRAM => 1_000,
            Self::HDD => 5_000_000,
            Self::Network => 10_000_000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SwapDevice {
    pub device_id: u64,
    pub device_type: SwapDeviceType,
    pub total_slots: u64,
    pub used_slots: u64,
    pub read_bandwidth_bps: u64,
    pub write_bandwidth_bps: u64,
    pub priority: i32,
    pub io_queue_depth: u32,
}

impl SwapDevice {
    pub fn utilization(&self) -> f64 {
        if self.total_slots == 0 { return 0.0; }
        self.used_slots as f64 / self.total_slots as f64
    }
    pub fn free_slots(&self) -> u64 { self.total_slots.saturating_sub(self.used_slots) }
    pub fn is_congested(&self) -> bool { self.io_queue_depth > 32 }
}

#[derive(Debug, Clone)]
pub struct ZswapGlobalStats {
    pub pool_size_bytes: u64,
    pub stored_pages: u64,
    pub compressed_bytes: u64,
    pub original_bytes: u64,
    pub reject_count: u64,
    pub writeback_count: u64,
}

impl ZswapGlobalStats {
    pub fn compression_ratio(&self) -> f64 {
        if self.compressed_bytes == 0 { return 1.0; }
        self.original_bytes as f64 / self.compressed_bytes as f64
    }
    pub fn pool_utilization(&self) -> f64 {
        if self.pool_size_bytes == 0 { return 0.0; }
        self.compressed_bytes as f64 / self.pool_size_bytes as f64
    }
}

#[derive(Debug, Clone, Default)]
pub struct SwapHolisticStats {
    pub total_swap_space: u64,
    pub total_used: u64,
    pub total_devices: u64,
    pub read_throughput: u64,
    pub write_throughput: u64,
    pub global_compression_ratio: f64,
    pub predicted_exhaustion_ns: Option<u64>,
}

pub struct SwapHolisticManager {
    devices: BTreeMap<u64, SwapDevice>,
    zswap: ZswapGlobalStats,
    /// Swap usage history for prediction
    usage_history: Vec<(u64, u64)>, // (timestamp, used_slots)
    stats: SwapHolisticStats,
}

impl SwapHolisticManager {
    pub fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
            zswap: ZswapGlobalStats {
                pool_size_bytes: 0, stored_pages: 0,
                compressed_bytes: 0, original_bytes: 0,
                reject_count: 0, writeback_count: 0,
            },
            usage_history: Vec::new(),
            stats: SwapHolisticStats::default(),
        }
    }

    pub fn add_device(&mut self, device: SwapDevice) {
        self.stats.total_swap_space += device.total_slots;
        self.stats.total_devices += 1;
        self.devices.insert(device.device_id, device);
    }

    /// Balance I/O across swap devices
    pub fn select_device_for_write(&self, pages: u64) -> Option<u64> {
        self.devices.iter()
            .filter(|(_, d)| d.free_slots() >= pages && !d.is_congested())
            .max_by_key(|(_, d)| {
                // Score: prefer fastest device with most free space
                let speed_score = d.write_bandwidth_bps / 1_000_000;
                let space_score = d.free_slots() / 100;
                let priority_score = (d.priority + 100) as u64;
                speed_score + space_score + priority_score
            })
            .map(|(id, _)| *id)
    }

    /// Balance swap reads across devices
    pub fn select_device_for_read(&self) -> Option<u64> {
        self.devices.iter()
            .filter(|(_, d)| d.used_slots > 0 && !d.is_congested())
            .min_by_key(|(_, d)| d.device_type.latency_estimate_ns())
            .map(|(id, _)| *id)
    }

    /// Update device usage
    pub fn update_usage(&mut self, device_id: u64, used_delta: i64, now: u64) {
        if let Some(dev) = self.devices.get_mut(&device_id) {
            if used_delta > 0 { dev.used_slots += used_delta as u64; }
            else { dev.used_slots = dev.used_slots.saturating_sub((-used_delta) as u64); }
        }

        // Update global stats
        self.stats.total_used = self.devices.values().map(|d| d.used_slots).sum();
        self.usage_history.push((now, self.stats.total_used));
        if self.usage_history.len() > 256 { self.usage_history.drain(..128); }
    }

    /// Update zswap statistics
    pub fn update_zswap(&mut self, zswap: ZswapGlobalStats) {
        self.stats.global_compression_ratio = zswap.compression_ratio();
        self.zswap = zswap;
    }

    /// Predict when swap will be exhausted
    pub fn predict_exhaustion(&mut self) -> Option<u64> {
        if self.usage_history.len() < 10 { return None; }

        let recent = &self.usage_history[self.usage_history.len() - 10..];
        let dt = recent.last().unwrap().0.saturating_sub(recent.first().unwrap().0);
        let du = recent.last().unwrap().1 as i64 - recent.first().unwrap().1 as i64;

        if dt == 0 || du <= 0 { return None; }

        let remaining = self.stats.total_swap_space.saturating_sub(self.stats.total_used);
        let rate = du as u64;
        let time = (remaining * dt) / rate.max(1);
        self.stats.predicted_exhaustion_ns = Some(time);
        Some(time)
    }

    /// Global swap utilization
    pub fn utilization(&self) -> f64 {
        if self.stats.total_swap_space == 0 { return 0.0; }
        self.stats.total_used as f64 / self.stats.total_swap_space as f64
    }

    pub fn device(&self, id: u64) -> Option<&SwapDevice> { self.devices.get(&id) }
    pub fn zswap_stats(&self) -> &ZswapGlobalStats { &self.zswap }
    pub fn stats(&self) -> &SwapHolisticStats { &self.stats }
}
