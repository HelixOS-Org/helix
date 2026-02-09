// SPDX-License-Identifier: MIT
//! # Holistic Memory Sync Analysis
//!
//! System-wide writeback and memory sync optimization:
//! - Global dirty page ratio monitoring
//! - Writeback bandwidth allocation
//! - Cross-device sync scheduling
//! - I/O congestion detection from sync storms
//! - Periodic vs on-demand flush strategy selection

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlushStrategy { Periodic, OnDemand, Adaptive, Aggressive }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CongestionLevel { None, Light, Medium, Heavy, Saturated }

impl CongestionLevel {
    #[inline]
    pub fn from_pending_ratio(ratio: f64) -> Self {
        if ratio < 0.2 { Self::None }
        else if ratio < 0.4 { Self::Light }
        else if ratio < 0.6 { Self::Medium }
        else if ratio < 0.8 { Self::Heavy }
        else { Self::Saturated }
    }
}

#[derive(Debug, Clone)]
pub struct DeviceSyncProfile {
    pub device_id: u64,
    pub dirty_pages: u64,
    pub total_pages: u64,
    pub write_bandwidth_bps: u64,
    pub pending_flushes: u64,
    pub avg_flush_latency_ns: u64,
    pub last_flush: u64,
}

impl DeviceSyncProfile {
    #[inline(always)]
    pub fn dirty_ratio(&self) -> f64 {
        if self.total_pages == 0 { return 0.0; }
        self.dirty_pages as f64 / self.total_pages as f64
    }
    #[inline(always)]
    pub fn estimated_flush_time_ns(&self) -> u64 {
        if self.write_bandwidth_bps == 0 { return u64::MAX; }
        (self.dirty_pages * 4096 * 1_000_000_000) / self.write_bandwidth_bps
    }
}

#[derive(Debug, Clone)]
pub struct WritebackSchedule {
    pub device_id: u64,
    pub pages_to_flush: u64,
    pub deadline_ns: u64,
    pub bandwidth_share: f64,
    pub priority: u32,
}

#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct MsyncHolisticStats {
    pub total_dirty_pages: u64,
    pub total_flushes: u64,
    pub total_flushed_bytes: u64,
    pub congestion_events: u64,
    pub strategy_switches: u64,
    pub avg_flush_latency: u64,
}

pub struct MsyncHolisticManager {
    devices: BTreeMap<u64, DeviceSyncProfile>,
    schedule: Vec<WritebackSchedule>,
    strategy: FlushStrategy,
    dirty_threshold: f64,
    congestion_threshold: f64,
    stats: MsyncHolisticStats,
}

impl MsyncHolisticManager {
    pub fn new(dirty_threshold: f64) -> Self {
        Self {
            devices: BTreeMap::new(),
            schedule: Vec::new(),
            strategy: FlushStrategy::Adaptive,
            dirty_threshold,
            congestion_threshold: 0.7,
            stats: MsyncHolisticStats::default(),
        }
    }

    #[inline(always)]
    pub fn register_device(&mut self, profile: DeviceSyncProfile) {
        self.stats.total_dirty_pages += profile.dirty_pages;
        self.devices.insert(profile.device_id, profile);
    }

    pub fn update_dirty(&mut self, device_id: u64, dirty_delta: i64) {
        if let Some(dev) = self.devices.get_mut(&device_id) {
            if dirty_delta > 0 {
                dev.dirty_pages += dirty_delta as u64;
                self.stats.total_dirty_pages += dirty_delta as u64;
            } else {
                let dec = (-dirty_delta) as u64;
                dev.dirty_pages = dev.dirty_pages.saturating_sub(dec);
                self.stats.total_dirty_pages = self.stats.total_dirty_pages.saturating_sub(dec);
            }
        }
    }

    /// Compute system-wide dirty ratio
    #[inline]
    pub fn global_dirty_ratio(&self) -> f64 {
        let total_pages: u64 = self.devices.values().map(|d| d.total_pages).sum();
        if total_pages == 0 { return 0.0; }
        self.stats.total_dirty_pages as f64 / total_pages as f64
    }

    /// Detect I/O congestion
    #[inline]
    pub fn congestion_level(&self) -> CongestionLevel {
        let total_pending: u64 = self.devices.values().map(|d| d.pending_flushes).sum();
        let total_capacity: u64 = self.devices.values().map(|d| d.total_pages / 100).sum();
        if total_capacity == 0 { return CongestionLevel::None; }
        CongestionLevel::from_pending_ratio(total_pending as f64 / total_capacity as f64)
    }

    /// Build writeback schedule based on current strategy
    pub fn build_schedule(&mut self, now: u64) -> Vec<WritebackSchedule> {
        let total_bw: u64 = self.devices.values().map(|d| d.write_bandwidth_bps).sum();
        self.schedule.clear();

        for (_, dev) in &self.devices {
            if dev.dirty_ratio() < self.dirty_threshold && self.strategy != FlushStrategy::Aggressive {
                continue;
            }
            let bw_share = if total_bw > 0 {
                dev.write_bandwidth_bps as f64 / total_bw as f64
            } else {
                1.0 / self.devices.len().max(1) as f64
            };
            let priority = if dev.dirty_ratio() > 0.8 { 0 }
                else if dev.dirty_ratio() > 0.5 { 1 }
                else { 2 };

            self.schedule.push(WritebackSchedule {
                device_id: dev.device_id,
                pages_to_flush: dev.dirty_pages,
                deadline_ns: now + dev.estimated_flush_time_ns(),
                bandwidth_share: bw_share,
                priority,
            });
        }
        self.schedule.sort_by_key(|s| s.priority);
        self.schedule.clone()
    }

    /// Adapt flush strategy based on system state
    pub fn adapt_strategy(&mut self) {
        let dirty_ratio = self.global_dirty_ratio();
        let congestion = self.congestion_level();
        let old = self.strategy;

        self.strategy = match (dirty_ratio > self.dirty_threshold, congestion) {
            (true, CongestionLevel::Heavy | CongestionLevel::Saturated) => FlushStrategy::Aggressive,
            (true, _) => FlushStrategy::OnDemand,
            (false, CongestionLevel::None) => FlushStrategy::Periodic,
            _ => FlushStrategy::Adaptive,
        };

        if old != self.strategy { self.stats.strategy_switches += 1; }
    }

    /// Record a completed flush
    #[inline]
    pub fn record_flush(&mut self, device_id: u64, pages: u64, latency: u64, now: u64) {
        if let Some(dev) = self.devices.get_mut(&device_id) {
            dev.dirty_pages = dev.dirty_pages.saturating_sub(pages);
            dev.last_flush = now;
            dev.avg_flush_latency_ns = (dev.avg_flush_latency_ns * 7 + latency) / 8;
        }
        self.stats.total_flushes += 1;
        self.stats.total_flushed_bytes += pages * 4096;
        self.stats.avg_flush_latency = (self.stats.avg_flush_latency * 15 + latency) / 16;
    }

    #[inline(always)]
    pub fn strategy(&self) -> FlushStrategy { self.strategy }
    #[inline(always)]
    pub fn stats(&self) -> &MsyncHolisticStats { &self.stats }
}
