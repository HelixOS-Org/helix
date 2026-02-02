//! Metrics Types
//!
//! Types for representing measurements and telemetry data.

#![allow(dead_code)]

use alloc::string::String;

use super::temporal::Timestamp;

// ============================================================================
// METRIC
// ============================================================================

/// Metric value with metadata
#[derive(Debug, Clone)]
pub struct Metric {
    /// Metric name
    pub name: String,
    /// Current value
    pub value: MetricValue,
    /// Timestamp of measurement
    pub timestamp: Timestamp,
    /// Unit of measurement
    pub unit: MetricUnit,
}

impl Metric {
    /// Create new metric
    pub fn new(name: impl Into<String>, value: MetricValue, unit: MetricUnit) -> Self {
        Self {
            name: name.into(),
            value,
            timestamp: Timestamp::now(),
            unit,
        }
    }

    /// Create counter metric
    pub fn counter(name: impl Into<String>, value: u64) -> Self {
        Self::new(name, MetricValue::Counter(value), MetricUnit::Count)
    }

    /// Create gauge metric
    pub fn gauge(name: impl Into<String>, value: f64, unit: MetricUnit) -> Self {
        Self::new(name, MetricValue::Gauge(value), unit)
    }

    /// Create flag metric
    pub fn flag(name: impl Into<String>, value: bool) -> Self {
        Self::new(name, MetricValue::Flag(value), MetricUnit::None)
    }
}

// ============================================================================
// METRIC VALUE
// ============================================================================

/// Metric value variants
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MetricValue {
    /// Counter (monotonically increasing)
    Counter(u64),
    /// Gauge (can go up or down)
    Gauge(f64),
    /// Histogram bucket
    Histogram { sum: f64, count: u64 },
    /// Boolean flag
    Flag(bool),
}

impl MetricValue {
    /// As f64 (for comparison)
    pub fn as_f64(&self) -> f64 {
        match self {
            Self::Counter(v) => *v as f64,
            Self::Gauge(v) => *v,
            Self::Histogram { sum, count } => {
                if *count > 0 {
                    sum / *count as f64
                } else {
                    0.0
                }
            },
            Self::Flag(v) => {
                if *v {
                    1.0
                } else {
                    0.0
                }
            },
        }
    }

    /// Is this a counter?
    pub const fn is_counter(&self) -> bool {
        matches!(self, Self::Counter(_))
    }

    /// Is this a gauge?
    pub const fn is_gauge(&self) -> bool {
        matches!(self, Self::Gauge(_))
    }

    /// Is this a histogram?
    pub const fn is_histogram(&self) -> bool {
        matches!(self, Self::Histogram { .. })
    }

    /// Is this a flag?
    pub const fn is_flag(&self) -> bool {
        matches!(self, Self::Flag(_))
    }
}

impl Default for MetricValue {
    fn default() -> Self {
        Self::Gauge(0.0)
    }
}

// ============================================================================
// METRIC UNIT
// ============================================================================

/// Metric unit
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MetricUnit {
    /// No unit
    #[default]
    None,
    /// Bytes
    Bytes,
    /// Kilobytes
    Kilobytes,
    /// Megabytes
    Megabytes,
    /// Gigabytes
    Gigabytes,
    /// Nanoseconds
    Nanoseconds,
    /// Microseconds
    Microseconds,
    /// Milliseconds
    Milliseconds,
    /// Seconds
    Seconds,
    /// Count
    Count,
    /// Percentage (0-100)
    Percent,
    /// Ratio (0-1)
    Ratio,
    /// Hertz
    Hertz,
    /// Kilohertz
    Kilohertz,
    /// Megahertz
    Megahertz,
    /// Gigahertz
    Gigahertz,
    /// Bits per second
    BitsPerSecond,
    /// Bytes per second
    BytesPerSecond,
    /// Kilobytes per second
    KilobytesPerSecond,
    /// Megabytes per second
    MegabytesPerSecond,
    /// Operations per second
    OpsPerSecond,
    /// Requests per second
    RequestsPerSecond,
    /// Celsius
    Celsius,
    /// Fahrenheit
    Fahrenheit,
    /// Watts
    Watts,
    /// Milliwatts
    Milliwatts,
    /// Amperes
    Amperes,
    /// Volts
    Volts,
    /// Pages
    Pages,
    /// Blocks
    Blocks,
    /// Sectors
    Sectors,
}

impl MetricUnit {
    /// Get unit symbol
    pub const fn symbol(&self) -> &'static str {
        match self {
            Self::None => "",
            Self::Bytes => "B",
            Self::Kilobytes => "KB",
            Self::Megabytes => "MB",
            Self::Gigabytes => "GB",
            Self::Nanoseconds => "ns",
            Self::Microseconds => "µs",
            Self::Milliseconds => "ms",
            Self::Seconds => "s",
            Self::Count => "",
            Self::Percent => "%",
            Self::Ratio => "",
            Self::Hertz => "Hz",
            Self::Kilohertz => "KHz",
            Self::Megahertz => "MHz",
            Self::Gigahertz => "GHz",
            Self::BitsPerSecond => "bps",
            Self::BytesPerSecond => "B/s",
            Self::KilobytesPerSecond => "KB/s",
            Self::MegabytesPerSecond => "MB/s",
            Self::OpsPerSecond => "ops/s",
            Self::RequestsPerSecond => "req/s",
            Self::Celsius => "°C",
            Self::Fahrenheit => "°F",
            Self::Watts => "W",
            Self::Milliwatts => "mW",
            Self::Amperes => "A",
            Self::Volts => "V",
            Self::Pages => "pages",
            Self::Blocks => "blocks",
            Self::Sectors => "sectors",
        }
    }

    /// Get unit full name
    pub const fn name(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Bytes => "bytes",
            Self::Kilobytes => "kilobytes",
            Self::Megabytes => "megabytes",
            Self::Gigabytes => "gigabytes",
            Self::Nanoseconds => "nanoseconds",
            Self::Microseconds => "microseconds",
            Self::Milliseconds => "milliseconds",
            Self::Seconds => "seconds",
            Self::Count => "count",
            Self::Percent => "percent",
            Self::Ratio => "ratio",
            Self::Hertz => "hertz",
            Self::Kilohertz => "kilohertz",
            Self::Megahertz => "megahertz",
            Self::Gigahertz => "gigahertz",
            Self::BitsPerSecond => "bits_per_second",
            Self::BytesPerSecond => "bytes_per_second",
            Self::KilobytesPerSecond => "kilobytes_per_second",
            Self::MegabytesPerSecond => "megabytes_per_second",
            Self::OpsPerSecond => "operations_per_second",
            Self::RequestsPerSecond => "requests_per_second",
            Self::Celsius => "celsius",
            Self::Fahrenheit => "fahrenheit",
            Self::Watts => "watts",
            Self::Milliwatts => "milliwatts",
            Self::Amperes => "amperes",
            Self::Volts => "volts",
            Self::Pages => "pages",
            Self::Blocks => "blocks",
            Self::Sectors => "sectors",
        }
    }

    /// Is this a time unit?
    pub const fn is_time(&self) -> bool {
        matches!(
            self,
            Self::Nanoseconds | Self::Microseconds | Self::Milliseconds | Self::Seconds
        )
    }

    /// Is this a size unit?
    pub const fn is_size(&self) -> bool {
        matches!(
            self,
            Self::Bytes | Self::Kilobytes | Self::Megabytes | Self::Gigabytes
        )
    }

    /// Is this a rate unit?
    pub const fn is_rate(&self) -> bool {
        matches!(
            self,
            Self::BitsPerSecond
                | Self::BytesPerSecond
                | Self::KilobytesPerSecond
                | Self::MegabytesPerSecond
                | Self::OpsPerSecond
                | Self::RequestsPerSecond
        )
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_value() {
        let counter = MetricValue::Counter(100);
        assert_eq!(counter.as_f64(), 100.0);
        assert!(counter.is_counter());

        let gauge = MetricValue::Gauge(3.14);
        assert!((gauge.as_f64() - 3.14).abs() < 0.01);
        assert!(gauge.is_gauge());
    }

    #[test]
    fn test_metric_unit() {
        assert_eq!(MetricUnit::Bytes.symbol(), "B");
        assert_eq!(MetricUnit::Milliseconds.symbol(), "ms");
        assert!(MetricUnit::Seconds.is_time());
        assert!(MetricUnit::Megabytes.is_size());
        assert!(MetricUnit::BytesPerSecond.is_rate());
    }

    #[test]
    fn test_metric_creation() {
        let m = Metric::counter("requests", 42);
        assert_eq!(m.name, "requests");
        assert!(matches!(m.value, MetricValue::Counter(42)));
    }
}
