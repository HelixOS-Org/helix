//! Jitter Analyzer
//!
//! Analyzes timer jitter and timing precision.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::core::NexusTimestamp;

use super::TimerId;

/// Jitter statistics
#[derive(Debug, Clone, Default)]
pub struct JitterStats {
    /// Timer ID
    pub timer_id: TimerId,
    /// Average jitter (ns)
    pub avg_jitter_ns: f64,
    /// Maximum jitter (ns)
    pub max_jitter_ns: i64,
    /// Minimum jitter (ns)
    pub min_jitter_ns: i64,
    /// Standard deviation
    pub std_dev_ns: f64,
    /// Sample count
    pub samples: u64,
}

impl JitterStats {
    /// Record jitter
    pub fn record(&mut self, jitter_ns: i64) {
        self.samples += 1;

        if jitter_ns > self.max_jitter_ns {
            self.max_jitter_ns = jitter_ns;
        }
        if jitter_ns < self.min_jitter_ns {
            self.min_jitter_ns = jitter_ns;
        }

        let alpha = 0.1;
        self.avg_jitter_ns = alpha * jitter_ns as f64 + (1.0 - alpha) * self.avg_jitter_ns;
    }

    /// Jitter range
    pub fn range(&self) -> i64 {
        self.max_jitter_ns - self.min_jitter_ns
    }
}

/// Jitter sample
#[derive(Debug, Clone, Copy)]
struct JitterSample {
    /// Timestamp
    timestamp: u64,
    /// Expected deadline
    expected: u64,
    /// Actual time
    actual: u64,
    /// Jitter (actual - expected)
    jitter: i64,
}

/// Analyzes timer jitter
pub struct JitterAnalyzer {
    /// Per-timer jitter stats
    stats: BTreeMap<TimerId, JitterStats>,
    /// Jitter samples
    samples: BTreeMap<TimerId, Vec<JitterSample>>,
    /// Max samples
    max_samples: usize,
    /// High jitter threshold (ns)
    threshold_ns: u64,
}

impl JitterAnalyzer {
    /// Create new analyzer
    pub fn new() -> Self {
        Self {
            stats: BTreeMap::new(),
            samples: BTreeMap::new(),
            max_samples: 1000,
            threshold_ns: 1_000_000, // 1ms default
        }
    }

    /// Set threshold
    pub fn set_threshold(&mut self, threshold_ns: u64) {
        self.threshold_ns = threshold_ns;
    }

    /// Record timer event
    pub fn record(&mut self, timer_id: TimerId, expected_ns: u64, actual_ns: u64) {
        let jitter = actual_ns as i64 - expected_ns as i64;

        let sample = JitterSample {
            timestamp: NexusTimestamp::now().raw(),
            expected: expected_ns,
            actual: actual_ns,
            jitter,
        };

        let samples = self.samples.entry(timer_id).or_default();
        samples.push(sample);
        if samples.len() > self.max_samples {
            samples.remove(0);
        }

        let stats = self.stats.entry(timer_id).or_insert_with(|| JitterStats {
            timer_id,
            min_jitter_ns: i64::MAX,
            ..Default::default()
        });
        stats.record(jitter);
    }

    /// Get stats
    pub fn get_stats(&self, timer_id: TimerId) -> Option<&JitterStats> {
        self.stats.get(&timer_id)
    }

    /// Get high jitter timers
    pub fn high_jitter_timers(&self) -> Vec<TimerId> {
        self.stats
            .iter()
            .filter(|(_, s)| s.avg_jitter_ns.abs() > self.threshold_ns as f64)
            .map(|(&id, _)| id)
            .collect()
    }

    /// Get jitter histogram for timer
    pub fn histogram(&self, timer_id: TimerId, buckets: usize) -> Vec<(i64, u64)> {
        let samples = match self.samples.get(&timer_id) {
            Some(s) if !s.is_empty() => s,
            _ => return Vec::new(),
        };

        let min = samples.iter().map(|s| s.jitter).min().unwrap_or(0);
        let max = samples.iter().map(|s| s.jitter).max().unwrap_or(0);
        let range = max - min;
        let bucket_size = if range > 0 { range / buckets as i64 } else { 1 };

        let mut histogram = vec![0u64; buckets];

        for sample in samples {
            let bucket = ((sample.jitter - min) / bucket_size) as usize;
            let bucket = bucket.min(buckets - 1);
            histogram[bucket] += 1;
        }

        histogram
            .into_iter()
            .enumerate()
            .map(|(i, count)| (min + i as i64 * bucket_size, count))
            .collect()
    }
}

impl Default for JitterAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
