//! Virtualization Metrics
//!
//! Resource usage metrics and time series.

use alloc::collections::VecDeque;
use alloc::vec::Vec;

use crate::core::NexusTimestamp;

/// Resource usage metrics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct VirtMetrics {
    /// CPU usage (0.0 - 100.0 per vcpu)
    pub cpu_usage: f64,
    /// Memory used (bytes)
    pub memory_used: u64,
    /// Memory available
    pub memory_available: u64,
    /// Network RX bytes
    pub net_rx_bytes: u64,
    /// Network TX bytes
    pub net_tx_bytes: u64,
    /// Disk read bytes
    pub disk_read_bytes: u64,
    /// Disk write bytes
    pub disk_write_bytes: u64,
    /// IO operations
    pub io_ops: u64,
    /// Page faults
    pub page_faults: u64,
    /// Context switches
    pub context_switches: u64,
}

impl VirtMetrics {
    /// Memory usage ratio
    #[inline]
    pub fn memory_ratio(&self) -> f64 {
        if self.memory_available == 0 {
            0.0
        } else {
            self.memory_used as f64 / self.memory_available as f64
        }
    }

    /// Is memory constrained?
    #[inline(always)]
    pub fn is_memory_constrained(&self) -> bool {
        self.memory_ratio() > 0.9
    }

    /// Is CPU constrained?
    #[inline(always)]
    pub fn is_cpu_constrained(&self) -> bool {
        self.cpu_usage > 90.0
    }
}

/// Time-series metrics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MetricsSeries {
    /// Timestamps
    timestamps: VecDeque<u64>,
    /// CPU values
    cpu: VecDeque<f64>,
    /// Memory values
    memory: VecDeque<f64>,
    /// Max samples
    max_samples: usize,
}

impl MetricsSeries {
    /// Create new series
    pub fn new(max_samples: usize) -> Self {
        Self {
            timestamps: Vec::with_capacity(max_samples),
            cpu: Vec::with_capacity(max_samples),
            memory: Vec::with_capacity(max_samples),
            max_samples,
        }
    }

    /// Add sample
    pub fn add(&mut self, metrics: &VirtMetrics) {
        let timestamp = NexusTimestamp::now().raw();

        if self.timestamps.len() >= self.max_samples {
            self.timestamps.pop_front();
            self.cpu.pop_front();
            self.memory.pop_front();
        }

        self.timestamps.push_back(timestamp);
        self.cpu.push_back(metrics.cpu_usage);
        self.memory.push_back(metrics.memory_ratio() * 100.0);
    }

    /// Get average CPU
    #[inline]
    pub fn avg_cpu(&self) -> f64 {
        if self.cpu.is_empty() {
            0.0
        } else {
            self.cpu.iter().sum::<f64>() / self.cpu.len() as f64
        }
    }

    /// Get average memory
    #[inline]
    pub fn avg_memory(&self) -> f64 {
        if self.memory.is_empty() {
            0.0
        } else {
            self.memory.iter().sum::<f64>() / self.memory.len() as f64
        }
    }

    /// Get CPU trend
    #[inline(always)]
    pub fn cpu_trend(&self) -> f64 {
        self.calculate_trend(&self.cpu)
    }

    /// Get memory trend
    #[inline(always)]
    pub fn memory_trend(&self) -> f64 {
        self.calculate_trend(&self.memory)
    }

    /// Calculate trend using linear regression
    fn calculate_trend(&self, values: &[f64]) -> f64 {
        if values.len() < 2 {
            return 0.0;
        }

        let n = values.len() as f64;
        let x_mean = (n - 1.0) / 2.0;
        let y_mean = values.iter().sum::<f64>() / n;

        let mut numerator = 0.0;
        let mut denominator = 0.0;

        for (i, &y) in values.iter().enumerate() {
            let x = i as f64;
            numerator += (x - x_mean) * (y - y_mean);
            denominator += (x - x_mean) * (x - x_mean);
        }

        if denominator == 0.0 {
            0.0
        } else {
            numerator / denominator
        }
    }
}

impl Default for MetricsSeries {
    fn default() -> Self {
        Self::new(100)
    }
}
