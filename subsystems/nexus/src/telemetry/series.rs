//! Data points and time series storage.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

use crate::core::NexusTimestamp;
use crate::math;

// ============================================================================
// DATA POINT
// ============================================================================

/// A single data point
#[derive(Debug, Clone, Copy)]
pub struct DataPoint {
    /// Timestamp
    pub timestamp: u64,
    /// Value
    pub value: f64,
}

impl DataPoint {
    /// Create new data point
    pub fn new(timestamp: u64, value: f64) -> Self {
        Self { timestamp, value }
    }

    /// Create with current timestamp
    #[inline]
    pub fn now(value: f64) -> Self {
        Self {
            timestamp: NexusTimestamp::now().raw(),
            value,
        }
    }
}

// ============================================================================
// TIME SERIES
// ============================================================================

/// A time series of data points
#[derive(Debug, Clone)]
pub struct TimeSeries {
    /// Metric name
    pub name: String,
    /// Labels
    pub labels: BTreeMap<String, String>,
    /// Data points
    points: VecDeque<DataPoint>,
    /// Maximum points to retain
    max_points: usize,
    /// Aggregation interval (for downsampling)
    aggregation_interval: u64,
}

impl TimeSeries {
    /// Create new time series
    pub fn new(name: impl Into<String>, max_points: usize) -> Self {
        Self {
            name: name.into(),
            labels: BTreeMap::new(),
            points: VecDeque::new(),
            max_points,
            aggregation_interval: 0,
        }
    }

    /// Add label
    #[inline(always)]
    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    /// Add data point
    #[inline]
    pub fn add(&mut self, point: DataPoint) {
        self.points.push_back(point);

        // Evict old points if over limit
        if self.points.len() > self.max_points {
            self.points.pop_front();
        }
    }

    /// Add value with current timestamp
    #[inline(always)]
    pub fn add_value(&mut self, value: f64) {
        self.add(DataPoint::now(value));
    }

    /// Get all points
    #[inline(always)]
    pub fn points(&self) -> &[DataPoint] {
        &self.points
    }

    /// Get points in time range
    #[inline]
    pub fn range(&self, start: u64, end: u64) -> Vec<DataPoint> {
        self.points
            .iter()
            .filter(|p| p.timestamp >= start && p.timestamp <= end)
            .copied()
            .collect()
    }

    /// Get latest value
    #[inline(always)]
    pub fn latest(&self) -> Option<f64> {
        self.points.back().map(|p| p.value)
    }

    /// Get latest N values
    #[inline(always)]
    pub fn latest_n(&self, n: usize) -> Vec<f64> {
        self.points.iter().rev().take(n).map(|p| p.value).collect()
    }

    /// Calculate mean
    #[inline]
    pub fn mean(&self) -> f64 {
        if self.points.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.points.iter().map(|p| p.value).sum();
        sum / self.points.len() as f64
    }

    /// Calculate min
    #[inline]
    pub fn min(&self) -> f64 {
        self.points
            .iter()
            .map(|p| p.value)
            .fold(f64::INFINITY, f64::min)
    }

    /// Calculate max
    #[inline]
    pub fn max(&self) -> f64 {
        self.points
            .iter()
            .map(|p| p.value)
            .fold(f64::NEG_INFINITY, f64::max)
    }

    /// Calculate standard deviation
    pub fn std_dev(&self) -> f64 {
        if self.points.len() < 2 {
            return 0.0;
        }

        let mean = self.mean();
        let variance: f64 = self
            .points
            .iter()
            .map(|p| {
                let diff = p.value - mean;
                diff * diff
            })
            .sum::<f64>()
            / (self.points.len() - 1) as f64;

        math::sqrt(variance)
    }

    /// Calculate rate of change per second
    pub fn rate(&self) -> f64 {
        if self.points.len() < 2 {
            return 0.0;
        }

        let first = &self.points[0];
        let last = self.points.back().unwrap();

        let time_diff = (last.timestamp - first.timestamp) as f64;
        if time_diff == 0.0 {
            return 0.0;
        }

        (last.value - first.value) / time_diff
    }

    /// Downsample to lower resolution
    pub fn downsample(&self, interval: u64) -> Vec<DataPoint> {
        if self.points.is_empty() || interval == 0 {
            return Vec::new();
        }

        let mut result = Vec::new();
        let mut current_bucket_start = self.points[0].timestamp;
        let mut bucket_sum = 0.0;
        let mut bucket_count = 0;

        for point in &self.points {
            if point.timestamp >= current_bucket_start + interval {
                // Emit bucket
                if bucket_count > 0 {
                    result.push(DataPoint::new(
                        current_bucket_start,
                        bucket_sum / bucket_count as f64,
                    ));
                }

                // Start new bucket
                current_bucket_start = (point.timestamp / interval) * interval;
                bucket_sum = 0.0;
                bucket_count = 0;
            }

            bucket_sum += point.value;
            bucket_count += 1;
        }

        // Emit last bucket
        if bucket_count > 0 {
            result.push(DataPoint::new(
                current_bucket_start,
                bucket_sum / bucket_count as f64,
            ));
        }

        result
    }

    /// Number of points
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Is empty?
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Clear all points
    #[inline(always)]
    pub fn clear(&mut self) {
        self.points.clear();
    }
}

impl Default for TimeSeries {
    fn default() -> Self {
        Self::new("", 1000)
    }
}
