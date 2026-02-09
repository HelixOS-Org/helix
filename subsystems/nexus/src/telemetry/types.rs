//! Core telemetry types and values.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// METRIC TYPES
// ============================================================================

/// Type of metric
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricType {
    /// Counter - monotonically increasing
    Counter,
    /// Gauge - can go up and down
    Gauge,
    /// Histogram - distribution of values
    Histogram,
    /// Summary - pre-computed quantiles
    Summary,
    /// Rate - derived rate metric
    Rate,
}

/// Metric value
#[derive(Debug, Clone, Copy)]
pub enum MetricValue {
    /// Integer value
    Int(i64),
    /// Unsigned integer
    Uint(u64),
    /// Float value
    Float(f64),
}

impl MetricValue {
    /// Convert to f64
    #[inline]
    pub fn as_f64(&self) -> f64 {
        match self {
            Self::Int(v) => *v as f64,
            Self::Uint(v) => *v as f64,
            Self::Float(v) => *v,
        }
    }

    /// Convert to u64
    #[inline]
    pub fn as_u64(&self) -> u64 {
        match self {
            Self::Int(v) => *v as u64,
            Self::Uint(v) => *v,
            Self::Float(v) => *v as u64,
        }
    }
}

impl Default for MetricValue {
    fn default() -> Self {
        Self::Float(0.0)
    }
}

// ============================================================================
// METRIC DEFINITION
// ============================================================================

/// Metric definition
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MetricDef {
    /// Metric name
    pub name: String,
    /// Type
    pub metric_type: MetricType,
    /// Description
    pub description: String,
    /// Unit
    pub unit: String,
    /// Labels
    pub labels: Vec<String>,
}

impl MetricDef {
    /// Create a new counter
    #[inline]
    pub fn counter(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            metric_type: MetricType::Counter,
            description: String::new(),
            unit: String::new(),
            labels: Vec::new(),
        }
    }

    /// Create a new gauge
    #[inline]
    pub fn gauge(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            metric_type: MetricType::Gauge,
            description: String::new(),
            unit: String::new(),
            labels: Vec::new(),
        }
    }

    /// Create a new histogram
    #[inline]
    pub fn histogram(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            metric_type: MetricType::Histogram,
            description: String::new(),
            unit: String::new(),
            labels: Vec::new(),
        }
    }

    /// Set description
    #[inline(always)]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set unit
    #[inline(always)]
    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = unit.into();
        self
    }

    /// Add label
    #[inline(always)]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.labels.push(label.into());
        self
    }
}
