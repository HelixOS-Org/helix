//! I/O access pattern analysis.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::core::NexusTimestamp;

// ============================================================================
// I/O ACCESS RECORD
// ============================================================================

/// I/O access record
#[derive(Debug, Clone, Copy)]
struct IoAccessRecord {
    /// Offset accessed
    offset: u64,
    /// Size
    size: u32,
    /// Is read
    is_read: bool,
    /// Timestamp
    timestamp: u64,
}

// ============================================================================
// I/O PATTERN
// ============================================================================

/// Detected I/O pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoPattern {
    /// Sequential reads
    SequentialRead,
    /// Sequential writes
    SequentialWrite,
    /// Random access
    Random,
    /// Strided access
    Strided { stride: i64 },
    /// Mixed pattern
    Mixed,
}

// ============================================================================
// I/O PATTERN ANALYZER
// ============================================================================

/// Analyzes I/O access patterns
pub struct IoPatternAnalyzer {
    /// Recent access history
    history: Vec<IoAccessRecord>,
    /// Max history size
    max_history: usize,
    /// Detected stride
    detected_stride: Option<i64>,
    /// Pattern confidence
    confidence: f64,
    /// Total operations analyzed
    total_ops: AtomicU64,
}

impl IoPatternAnalyzer {
    /// Create new analyzer
    pub fn new(max_history: usize) -> Self {
        Self {
            history: Vec::with_capacity(max_history),
            max_history,
            detected_stride: None,
            confidence: 0.0,
            total_ops: AtomicU64::new(0),
        }
    }

    /// Record I/O access
    pub fn record(&mut self, offset: u64, size: u32, is_read: bool) {
        let record = IoAccessRecord {
            offset,
            size,
            is_read,
            timestamp: NexusTimestamp::now().raw(),
        };

        self.history.push(record);
        self.total_ops.fetch_add(1, Ordering::Relaxed);

        // Evict old entries
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }

        // Update pattern detection
        if self.history.len() >= 10 {
            self.update_pattern();
        }
    }

    /// Update pattern detection
    fn update_pattern(&mut self) {
        let len = self.history.len();
        if len < 3 {
            return;
        }

        // Calculate deltas
        let mut strides: BTreeMap<i64, u32> = BTreeMap::new();
        let mut sequential_count = 0;

        for i in 1..len {
            let delta = self.history[i].offset as i64 - self.history[i - 1].offset as i64;
            let expected_seq = self.history[i - 1].offset + self.history[i - 1].size as u64;

            if self.history[i].offset == expected_seq {
                sequential_count += 1;
            }

            *strides.entry(delta).or_insert(0) += 1;
        }

        // Check for sequential
        let seq_ratio = sequential_count as f64 / (len - 1) as f64;
        if seq_ratio > 0.7 {
            self.detected_stride = Some(0); // 0 indicates sequential
            self.confidence = seq_ratio;
            return;
        }

        // Check for strided
        if let Some((&stride, &count)) = strides.iter().max_by_key(|(_, &c)| c) {
            let stride_ratio = count as f64 / (len - 1) as f64;
            if stride_ratio > 0.6 && stride != 0 {
                self.detected_stride = Some(stride);
                self.confidence = stride_ratio;
                return;
            }
        }

        // Random or mixed
        self.detected_stride = None;
        self.confidence = 0.0;
    }

    /// Get detected pattern
    pub fn get_pattern(&self) -> IoPattern {
        if self.confidence < 0.5 {
            return IoPattern::Random;
        }

        match self.detected_stride {
            Some(0) => {
                // Check if reads or writes
                let reads = self.history.iter().filter(|r| r.is_read).count();
                if reads > self.history.len() / 2 {
                    IoPattern::SequentialRead
                } else {
                    IoPattern::SequentialWrite
                }
            },
            Some(stride) => IoPattern::Strided { stride },
            None => IoPattern::Random,
        }
    }

    /// Get confidence
    pub fn confidence(&self) -> f64 {
        self.confidence
    }

    /// Predict next access
    pub fn predict_next(&self) -> Option<u64> {
        if self.history.is_empty() || self.confidence < 0.5 {
            return None;
        }

        let last = self.history.last()?;

        match self.detected_stride {
            Some(0) => Some(last.offset + last.size as u64),
            Some(stride) => Some((last.offset as i64 + stride) as u64),
            None => None,
        }
    }

    /// Get prefetch recommendations
    pub fn prefetch_recommendations(&self, count: usize) -> Vec<u64> {
        let mut recommendations = Vec::new();

        if self.confidence < 0.5 {
            return recommendations;
        }

        if let Some(next) = self.predict_next() {
            let stride = match self.detected_stride {
                Some(0) => self.history.last().map(|r| r.size as i64).unwrap_or(4096),
                Some(s) => s,
                None => return recommendations,
            };

            for i in 0..count {
                recommendations.push((next as i64 + stride * i as i64) as u64);
            }
        }

        recommendations
    }

    /// Clear history
    pub fn clear(&mut self) {
        self.history.clear();
        self.detected_stride = None;
        self.confidence = 0.0;
    }

    /// Get total operations
    pub fn total_operations(&self) -> u64 {
        self.total_ops.load(Ordering::Relaxed)
    }
}

impl Default for IoPatternAnalyzer {
    fn default() -> Self {
        Self::new(1000)
    }
}
