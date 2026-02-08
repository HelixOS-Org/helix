// SPDX-License-Identifier: GPL-2.0
//! Holistic netstack — holistic network stack layer analysis

extern crate alloc;

/// Netstack layer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetstackLayer { Driver, L2, L3, L4, Socket }

/// Netstack holistic record
#[derive(Debug, Clone)]
pub struct NetstackHolisticRecord {
    pub layer: NetstackLayer,
    pub processing_ns: u64,
    pub drops: u32,
    pub queue_depth: u32,
}

impl NetstackHolisticRecord {
    pub fn new(layer: NetstackLayer) -> Self { Self { layer, processing_ns: 0, drops: 0, queue_depth: 0 } }
}

/// Netstack holistic stats
#[derive(Debug, Clone)]
pub struct NetstackHolisticStats { pub total_samples: u64, pub total_drops: u64, pub slowest_layer_ns: u64, pub peak_queue: u32 }

/// Main holistic netstack
#[derive(Debug)]
pub struct HolisticNetstack { pub stats: NetstackHolisticStats }

impl HolisticNetstack {
    pub fn new() -> Self { Self { stats: NetstackHolisticStats { total_samples: 0, total_drops: 0, slowest_layer_ns: 0, peak_queue: 0 } } }
    pub fn record(&mut self, rec: &NetstackHolisticRecord) {
        self.stats.total_samples += 1;
        self.stats.total_drops += rec.drops as u64;
        if rec.processing_ns > self.stats.slowest_layer_ns { self.stats.slowest_layer_ns = rec.processing_ns; }
        if rec.queue_depth > self.stats.peak_queue { self.stats.peak_queue = rec.queue_depth; }
    }
}
