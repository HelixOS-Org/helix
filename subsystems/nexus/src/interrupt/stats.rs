//! IRQ statistics tracking
//!
//! This module provides the IrqStats struct for collecting and analyzing
//! interrupt statistics including per-CPU counts, latency metrics, and load balance.

#![allow(dead_code)]

extern crate alloc;
use alloc::collections::BTreeMap;

use super::record::InterruptRecord;
use super::types::CpuId;
use crate::math::F64Ext;

/// Statistics for an IRQ
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct IrqStats {
    /// Total count
    pub total: u64,
    /// Per-CPU count
    pub per_cpu: BTreeMap<CpuId, u64>,
    /// Average latency
    pub avg_latency_ns: f64,
    /// Max latency
    pub max_latency_ns: u64,
    /// Min latency
    pub min_latency_ns: u64,
    /// Frequency (per second)
    pub frequency: f64,
    /// Last timestamp
    last_time: u64,
    /// Storm count
    pub storm_count: u64,
}

impl IrqStats {
    /// Create new stats
    pub fn new() -> Self {
        Self {
            total: 0,
            per_cpu: BTreeMap::new(),
            avg_latency_ns: 0.0,
            max_latency_ns: 0,
            min_latency_ns: u64::MAX,
            frequency: 0.0,
            last_time: 0,
            storm_count: 0,
        }
    }

    /// Record interrupt
    #[inline]
    pub fn record(&mut self, record: &InterruptRecord) {
        self.total += 1;
        *self.per_cpu.entry(record.cpu).or_insert(0) += 1;

        // Update latency stats
        if record.latency_ns > 0 {
            let alpha = 0.1;
            self.avg_latency_ns =
                alpha * record.latency_ns as f64 + (1.0 - alpha) * self.avg_latency_ns;
            self.max_latency_ns = self.max_latency_ns.max(record.latency_ns);
            self.min_latency_ns = self.min_latency_ns.min(record.latency_ns);
        }

        // Update frequency
        if self.last_time > 0 {
            let delta = record.timestamp.saturating_sub(self.last_time);
            if delta > 0 {
                let instant_freq = 1_000_000_000.0 / delta as f64;
                let alpha = 0.1;
                self.frequency = alpha * instant_freq + (1.0 - alpha) * self.frequency;
            }
        }
        self.last_time = record.timestamp;
    }

    /// Calculate load imbalance (0 = perfect, 1 = all on one CPU)
    pub fn load_imbalance(&self) -> f64 {
        if self.per_cpu.is_empty() {
            return 0.0;
        }

        let count = self.per_cpu.len() as f64;
        let mean = self.total as f64 / count;

        if mean == 0.0 {
            return 0.0;
        }

        let variance: f64 = self
            .per_cpu
            .values()
            .map(|&c| {
                let diff = c as f64 - mean;
                diff * diff
            })
            .sum::<f64>()
            / count;

        (variance.sqrt() / mean).min(1.0)
    }
}

impl Default for IrqStats {
    fn default() -> Self {
        Self::new()
    }
}
