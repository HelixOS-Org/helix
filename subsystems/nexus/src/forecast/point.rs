//! Time point for time series data.

use crate::core::NexusTimestamp;

/// A point in a time series
#[derive(Debug, Clone, Copy)]
pub struct TimePoint {
    /// Timestamp (ticks)
    pub timestamp: u64,
    /// Value
    pub value: f64,
}

impl TimePoint {
    /// Create a new point
    pub fn new(timestamp: u64, value: f64) -> Self {
        Self { timestamp, value }
    }

    /// Create with current timestamp
    #[inline]
    pub fn now(value: f64) -> Self {
        Self {
            timestamp: NexusTimestamp::now().ticks(),
            value,
        }
    }
}
