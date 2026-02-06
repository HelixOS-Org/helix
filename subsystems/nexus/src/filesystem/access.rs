//! File access tracking.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::types::{Inode, IoPatternType};
use crate::core::NexusTimestamp;

// ============================================================================
// ACCESS RECORD
// ============================================================================

/// Access record
#[derive(Debug, Clone, Copy)]
struct AccessRecord {
    /// Offset
    offset: u64,
    /// Size
    size: u32,
    /// Was it a read?
    is_read: bool,
    /// Timestamp
    timestamp: u64,
}

// ============================================================================
// FILE ACCESS TRACKER
// ============================================================================

/// Tracks file access patterns
pub struct FileAccessTracker {
    /// Access history per file
    history: BTreeMap<Inode, Vec<AccessRecord>>,
    /// Max history per file
    max_history: usize,
    /// Total accesses
    total_accesses: AtomicU64,
    /// Access patterns
    patterns: BTreeMap<Inode, IoPatternType>,
}

impl FileAccessTracker {
    /// Create new tracker
    pub fn new(max_history: usize) -> Self {
        Self {
            history: BTreeMap::new(),
            max_history,
            total_accesses: AtomicU64::new(0),
            patterns: BTreeMap::new(),
        }
    }

    /// Record access
    pub fn record(&mut self, inode: Inode, offset: u64, size: u32, is_read: bool) {
        self.total_accesses.fetch_add(1, Ordering::Relaxed);

        let history = self.history.entry(inode).or_default();
        history.push(AccessRecord {
            offset,
            size,
            is_read,
            timestamp: NexusTimestamp::now().raw(),
        });

        if history.len() > self.max_history {
            history.remove(0);
        }

        // Update pattern detection
        if history.len() >= 20 {
            self.detect_pattern(inode);
        }
    }

    /// Detect access pattern
    fn detect_pattern(&mut self, inode: Inode) {
        let history = match self.history.get(&inode) {
            Some(h) if h.len() >= 10 => h,
            _ => return,
        };

        let mut sequential_reads = 0;
        let mut sequential_writes = 0;
        let mut random_reads = 0;
        let mut random_writes = 0;

        for i in 1..history.len() {
            let prev = &history[i - 1];
            let curr = &history[i];

            let is_sequential = curr.offset == prev.offset + prev.size as u64
                || curr.offset == prev.offset.saturating_sub(curr.size as u64);

            match (is_sequential, curr.is_read) {
                (true, true) => sequential_reads += 1,
                (true, false) => sequential_writes += 1,
                (false, true) => random_reads += 1,
                (false, false) => random_writes += 1,
            }
        }

        let total = (history.len() - 1) as f64;
        let seq_read_ratio = sequential_reads as f64 / total;
        let seq_write_ratio = sequential_writes as f64 / total;
        let rand_read_ratio = random_reads as f64 / total;
        let rand_write_ratio = random_writes as f64 / total;

        let pattern = if seq_read_ratio > 0.6 {
            IoPatternType::SequentialRead
        } else if seq_write_ratio > 0.6 {
            IoPatternType::SequentialWrite
        } else if rand_read_ratio > 0.6 {
            IoPatternType::RandomRead
        } else if rand_write_ratio > 0.6 {
            IoPatternType::RandomWrite
        } else {
            IoPatternType::Mixed
        };

        self.patterns.insert(inode, pattern);
    }

    /// Get pattern for file
    pub fn get_pattern(&self, inode: Inode) -> Option<IoPatternType> {
        self.patterns.get(&inode).copied()
    }

    /// Get total accesses
    pub fn total_accesses(&self) -> u64 {
        self.total_accesses.load(Ordering::Relaxed)
    }

    /// Predict next access offset
    pub fn predict_next(&self, inode: Inode) -> Option<u64> {
        let history = self.history.get(&inode)?;
        let last = history.last()?;
        let pattern = self.patterns.get(&inode)?;

        match pattern {
            IoPatternType::SequentialRead | IoPatternType::SequentialWrite => {
                Some(last.offset + last.size as u64)
            },
            _ => None,
        }
    }

    /// Get prefetch suggestions
    pub fn prefetch_suggestions(&self, inode: Inode, count: usize) -> Vec<(u64, u32)> {
        let mut suggestions = Vec::new();

        let pattern = match self.patterns.get(&inode) {
            Some(p) => p,
            None => return suggestions,
        };

        let history = match self.history.get(&inode) {
            Some(h) if !h.is_empty() => h,
            _ => return suggestions,
        };

        let last = history.last().unwrap();

        if pattern == &IoPatternType::SequentialRead {
            for i in 1..=count {
                suggestions.push((last.offset + last.size as u64 * i as u64, last.size));
            }
        }

        suggestions
    }

    /// Clear history
    pub fn clear(&mut self) {
        self.history.clear();
        self.patterns.clear();
    }
}

impl Default for FileAccessTracker {
    fn default() -> Self {
        Self::new(1000)
    }
}
