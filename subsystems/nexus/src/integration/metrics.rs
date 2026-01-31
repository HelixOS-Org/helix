//! Metrics export interface

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::core::NexusTimestamp;
use crate::error::NexusResult;

// ============================================================================
// METRIC VALUE
// ============================================================================

/// Metric value
#[derive(Debug, Clone)]
pub enum MetricValue {
    /// Counter (monotonically increasing)
    Counter(u64),
    /// Gauge (can go up or down)
    Gauge(f64),
    /// Histogram summary
    Histogram {
        count: u64,
        sum: f64,
        min: f64,
        max: f64,
        p50: f64,
        p95: f64,
        p99: f64,
    },
}

// ============================================================================
// METRIC
// ============================================================================

/// A metric
#[derive(Debug, Clone)]
pub struct Metric {
    /// Metric name
    pub name: String,
    /// Description
    pub description: String,
    /// Value
    pub value: MetricValue,
    /// Labels
    pub labels: Vec<(String, String)>,
    /// Timestamp
    pub timestamp: NexusTimestamp,
}

// ============================================================================
// METRIC EXPORTER TRAIT
// ============================================================================

/// Metric exporter interface
pub trait MetricExporter: Send + Sync {
    /// Export metrics
    fn export(&self, metrics: &[Metric]) -> NexusResult<()>;

    /// Get exporter name
    fn name(&self) -> &str;
}
