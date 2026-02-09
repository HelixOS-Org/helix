//! RwLock Optimizer
//!
//! Optimizes read-write lock usage patterns.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::LockId;

/// RwLock statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct RwLockStats {
    /// Lock ID
    pub lock_id: LockId,
    /// Read acquisitions
    pub reads: u64,
    /// Write acquisitions
    pub writes: u64,
    /// Total read hold time (ns)
    pub read_time_ns: u64,
    /// Total write hold time (ns)
    pub write_time_ns: u64,
    /// Reader count sum (for average)
    pub reader_count_sum: u64,
    /// Samples for reader count
    pub reader_samples: u64,
}

impl RwLockStats {
    /// Record read
    #[inline]
    pub fn record_read(&mut self, hold_time_ns: u64, concurrent_readers: u32) {
        self.reads += 1;
        self.read_time_ns += hold_time_ns;
        self.reader_count_sum += concurrent_readers as u64;
        self.reader_samples += 1;
    }

    /// Record write
    #[inline(always)]
    pub fn record_write(&mut self, hold_time_ns: u64) {
        self.writes += 1;
        self.write_time_ns += hold_time_ns;
    }

    /// Read ratio
    #[inline]
    pub fn read_ratio(&self) -> f64 {
        let total = self.reads + self.writes;
        if total == 0 {
            0.0
        } else {
            self.reads as f64 / total as f64
        }
    }

    /// Average readers
    #[inline]
    pub fn avg_readers(&self) -> f64 {
        if self.reader_samples == 0 {
            0.0
        } else {
            self.reader_count_sum as f64 / self.reader_samples as f64
        }
    }
}

/// Read/write pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RwPattern {
    /// Mostly reads
    ReadHeavy,
    /// Mostly writes
    WriteHeavy,
    /// Balanced
    Balanced,
    /// Read-write-read burst
    Bursty,
}

/// RwLock recommendation
#[derive(Debug, Clone)]
pub struct RwRecommendation {
    /// Lock ID
    pub lock_id: LockId,
    /// Recommendation type
    pub recommendation: RwRecommendationType,
    /// Reason
    pub reason: String,
}

/// Recommendation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RwRecommendationType {
    /// Keep as RwLock
    Keep,
    /// Convert to Mutex
    ConvertToMutex,
    /// Use read-copy-update
    UseRcu,
    /// Use seqlock
    UseSeqlock,
}

/// Optimizes read-write lock usage
pub struct RwLockOptimizer {
    /// Per-lock stats
    stats: BTreeMap<LockId, RwLockStats>,
    /// Read/write patterns
    patterns: BTreeMap<LockId, RwPattern>,
    /// Recommendations
    recommendations: Vec<RwRecommendation>,
}

impl RwLockOptimizer {
    /// Create new optimizer
    pub fn new() -> Self {
        Self {
            stats: BTreeMap::new(),
            patterns: BTreeMap::new(),
            recommendations: Vec::new(),
        }
    }

    /// Record read
    #[inline]
    pub fn record_read(&mut self, lock_id: LockId, hold_time_ns: u64, concurrent_readers: u32) {
        let stats = self.stats.entry(lock_id).or_insert_with(|| RwLockStats {
            lock_id,
            ..Default::default()
        });
        stats.record_read(hold_time_ns, concurrent_readers);

        self.update_pattern(lock_id);
    }

    /// Record write
    #[inline]
    pub fn record_write(&mut self, lock_id: LockId, hold_time_ns: u64) {
        let stats = self.stats.entry(lock_id).or_insert_with(|| RwLockStats {
            lock_id,
            ..Default::default()
        });
        stats.record_write(hold_time_ns);

        self.update_pattern(lock_id);
    }

    /// Update pattern
    fn update_pattern(&mut self, lock_id: LockId) {
        let stats = match self.stats.get(&lock_id) {
            Some(s) => s,
            None => return,
        };

        let read_ratio = stats.read_ratio();

        let pattern = if read_ratio > 0.9 {
            RwPattern::ReadHeavy
        } else if read_ratio < 0.3 {
            RwPattern::WriteHeavy
        } else {
            RwPattern::Balanced
        };

        self.patterns.insert(lock_id, pattern);
    }

    /// Generate recommendations
    pub fn analyze(&mut self) {
        self.recommendations.clear();

        for (&lock_id, stats) in &self.stats {
            let pattern = self
                .patterns
                .get(&lock_id)
                .copied()
                .unwrap_or(RwPattern::Balanced);

            let (rec_type, reason) = match pattern {
                RwPattern::WriteHeavy => (
                    RwRecommendationType::ConvertToMutex,
                    alloc::format!(
                        "Write ratio {:.0}% - Mutex may be more efficient",
                        (1.0 - stats.read_ratio()) * 100.0
                    ),
                ),
                RwPattern::ReadHeavy if stats.avg_readers() < 1.5 => (
                    RwRecommendationType::ConvertToMutex,
                    String::from("Few concurrent readers - Mutex may be simpler"),
                ),
                RwPattern::ReadHeavy if stats.avg_readers() > 10.0 => (
                    RwRecommendationType::UseRcu,
                    String::from("Many concurrent readers - RCU may scale better"),
                ),
                _ => (
                    RwRecommendationType::Keep,
                    String::from("Current usage is appropriate"),
                ),
            };

            self.recommendations.push(RwRecommendation {
                lock_id,
                recommendation: rec_type,
                reason,
            });
        }
    }

    /// Get stats
    #[inline(always)]
    pub fn get_stats(&self, lock_id: LockId) -> Option<&RwLockStats> {
        self.stats.get(&lock_id)
    }

    /// Get pattern
    #[inline(always)]
    pub fn get_pattern(&self, lock_id: LockId) -> Option<RwPattern> {
        self.patterns.get(&lock_id).copied()
    }

    /// Get recommendations
    #[inline(always)]
    pub fn recommendations(&self) -> &[RwRecommendation] {
        &self.recommendations
    }
}

impl Default for RwLockOptimizer {
    fn default() -> Self {
        Self::new()
    }
}
