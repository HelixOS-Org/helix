//! Interrupt coalescing optimization
//!
//! This module provides intelligent interrupt coalescing optimization
//! to balance between latency and throughput based on workload characteristics.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::types::Irq;

/// Coalescing settings
#[derive(Debug, Clone, Copy)]
pub struct CoalescingSettings {
    /// Maximum delay before interrupt (microseconds)
    pub max_delay_us: u32,
    /// Maximum packets to coalesce
    pub max_frames: u32,
    /// Use adaptive coalescing
    pub adaptive: bool,
}

impl Default for CoalescingSettings {
    fn default() -> Self {
        Self {
            max_delay_us: 100,
            max_frames: 64,
            adaptive: true,
        }
    }
}

/// Coalescing performance metrics
#[derive(Debug, Clone, Default)]
pub struct CoalescingMetrics {
    /// Average interrupts per second
    pub avg_rate: f64,
    /// Average latency
    pub avg_latency_us: f64,
    /// Throughput (bytes/sec)
    pub throughput: f64,
    /// Samples
    pub samples: u64,
}

/// Optimizes interrupt coalescing
pub struct CoalescingOptimizer {
    /// Coalescing settings per IRQ
    settings: BTreeMap<Irq, CoalescingSettings>,
    /// Performance metrics
    metrics: BTreeMap<Irq, CoalescingMetrics>,
    /// Optimization history
    history: Vec<(Irq, CoalescingSettings, alloc::string::String)>,
}

impl CoalescingOptimizer {
    /// Create new optimizer
    pub fn new() -> Self {
        Self {
            settings: BTreeMap::new(),
            metrics: BTreeMap::new(),
            history: Vec::new(),
        }
    }

    /// Set settings for IRQ
    pub fn set_settings(&mut self, irq: Irq, settings: CoalescingSettings) {
        self.settings.insert(irq, settings);
    }

    /// Get settings for IRQ
    pub fn get_settings(&self, irq: Irq) -> CoalescingSettings {
        self.settings.get(&irq).copied().unwrap_or_default()
    }

    /// Record metrics
    pub fn record_metrics(&mut self, irq: Irq, rate: f64, latency_us: f64, throughput: f64) {
        let metrics = self.metrics.entry(irq).or_default();
        let alpha = 0.1;

        metrics.avg_rate = alpha * rate + (1.0 - alpha) * metrics.avg_rate;
        metrics.avg_latency_us = alpha * latency_us + (1.0 - alpha) * metrics.avg_latency_us;
        metrics.throughput = alpha * throughput + (1.0 - alpha) * metrics.throughput;
        metrics.samples += 1;
    }

    /// Optimize settings for IRQ
    pub fn optimize(&mut self, irq: Irq) -> Option<CoalescingSettings> {
        let metrics = self.metrics.get(&irq)?;
        let current = self.get_settings(irq);

        if metrics.samples < 100 {
            return None; // Not enough data
        }

        let mut new_settings = current;

        // High rate + low latency = can increase coalescing
        if metrics.avg_rate > 10000.0 && metrics.avg_latency_us < 50.0 {
            new_settings.max_delay_us = (current.max_delay_us + 50).min(1000);
            new_settings.max_frames = (current.max_frames + 16).min(256);
        }
        // High latency = decrease coalescing
        else if metrics.avg_latency_us > 200.0 {
            new_settings.max_delay_us = (current.max_delay_us.saturating_sub(25)).max(10);
            new_settings.max_frames = (current.max_frames.saturating_sub(8)).max(4);
        }

        if new_settings.max_delay_us != current.max_delay_us
            || new_settings.max_frames != current.max_frames
        {
            self.history.push((
                irq,
                new_settings,
                alloc::format!(
                    "rate={:.0}, latency={:.0}us",
                    metrics.avg_rate,
                    metrics.avg_latency_us
                ),
            ));
            self.settings.insert(irq, new_settings);
            Some(new_settings)
        } else {
            None
        }
    }

    /// Get recommended settings for workload
    pub fn recommend_for_workload(
        &self,
        latency_sensitive: bool,
        high_throughput: bool,
    ) -> CoalescingSettings {
        match (latency_sensitive, high_throughput) {
            (true, false) => CoalescingSettings {
                max_delay_us: 10,
                max_frames: 4,
                adaptive: true,
            },
            (false, true) => CoalescingSettings {
                max_delay_us: 500,
                max_frames: 128,
                adaptive: true,
            },
            (true, true) => CoalescingSettings {
                max_delay_us: 50,
                max_frames: 32,
                adaptive: true,
            },
            (false, false) => CoalescingSettings::default(),
        }
    }

    /// Get optimization history
    pub fn history(&self) -> &[(Irq, CoalescingSettings, alloc::string::String)] {
        &self.history
    }

    /// Get metrics for IRQ
    pub fn get_metrics(&self, irq: Irq) -> Option<&CoalescingMetrics> {
        self.metrics.get(&irq)
    }
}

impl Default for CoalescingOptimizer {
    fn default() -> Self {
        Self::new()
    }
}
