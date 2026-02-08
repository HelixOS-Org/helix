//! # Apps IO Pattern Analyzer
//!
//! Advanced IO pattern detection:
//! - Sequential/random/strided pattern detection
//! - IO size distribution analysis
//! - Read/write ratio tracking
//! - Temporal IO pattern detection
//! - Prefetch hint generation

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use libm::sqrt;

/// IO access pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoPatternType {
    Sequential,
    Random,
    Strided,
    Mixed,
    Burst,
    Periodic,
}

/// IO operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoOpType {
    Read,
    Write,
    ReadWrite,
    Sync,
    Trim,
}

/// IO access record
#[derive(Debug, Clone)]
pub struct IoAccessRecord {
    pub offset: u64,
    pub size: u32,
    pub op: IoOpType,
    pub timestamp_ns: u64,
    pub latency_ns: u64,
}

/// IO size bucket
#[derive(Debug, Clone, Default)]
pub struct IoSizeBucket {
    pub count: u64,
    pub total_bytes: u64,
    pub total_latency_ns: u64,
}

/// Per-file IO pattern
#[derive(Debug)]
pub struct FileIoPattern {
    /// File identifier (inode hash)
    pub file_hash: u64,
    /// Detected pattern
    pub pattern: IoPatternType,
    /// Recent offsets for pattern detection
    recent_offsets: Vec<u64>,
    recent_pos: usize,
    /// Size distribution (bucket = log2(size))
    size_buckets: BTreeMap<u8, IoSizeBucket>,
    /// Read count
    pub reads: u64,
    /// Write count
    pub writes: u64,
    /// Total bytes read
    pub bytes_read: u64,
    /// Total bytes written
    pub bytes_written: u64,
    /// Average stride (for strided patterns)
    pub avg_stride: f64,
    /// Sequentiality score (0..1)
    pub sequentiality: f64,
}

impl FileIoPattern {
    pub fn new(file_hash: u64) -> Self {
        Self {
            file_hash,
            pattern: IoPatternType::Mixed,
            recent_offsets: alloc::vec![0; 64],
            recent_pos: 0,
            size_buckets: BTreeMap::new(),
            reads: 0,
            writes: 0,
            bytes_read: 0,
            bytes_written: 0,
            avg_stride: 0.0,
            sequentiality: 0.0,
        }
    }

    /// Record IO access
    pub fn record_access(&mut self, record: &IoAccessRecord) {
        match record.op {
            IoOpType::Read | IoOpType::ReadWrite => {
                self.reads += 1;
                self.bytes_read += record.size as u64;
            }
            IoOpType::Write => {
                self.writes += 1;
                self.bytes_written += record.size as u64;
            }
            _ => {}
        }

        // Track offset
        self.recent_offsets[self.recent_pos % self.recent_offsets.len()] = record.offset;
        self.recent_pos += 1;

        // Size bucket
        let bucket = if record.size == 0 { 0 } else { (record.size as f64).log2() as u8 };
        let entry = self.size_buckets.entry(bucket).or_insert_with(IoSizeBucket::default);
        entry.count += 1;
        entry.total_bytes += record.size as u64;
        entry.total_latency_ns += record.latency_ns;

        self.detect_pattern();
    }

    fn detect_pattern(&mut self) {
        let count = self.recent_pos.min(self.recent_offsets.len());
        if count < 4 {
            return;
        }

        // Calculate strides between consecutive accesses
        let mut strides = Vec::new();
        for i in 1..count {
            let prev = self.recent_offsets[(self.recent_pos - count + i - 1) % self.recent_offsets.len()];
            let curr = self.recent_offsets[(self.recent_pos - count + i) % self.recent_offsets.len()];
            let stride = if curr >= prev { curr - prev } else { prev - curr };
            strides.push(stride);
        }

        if strides.is_empty() {
            return;
        }

        // Check sequentiality
        let sequential_count = strides.iter().filter(|&&s| s <= 8192).count();
        self.sequentiality = sequential_count as f64 / strides.len() as f64;

        // Average stride
        let stride_sum: u64 = strides.iter().sum();
        self.avg_stride = stride_sum as f64 / strides.len() as f64;

        // Stride variance
        let stride_var: f64 = strides.iter()
            .map(|&s| {
                let diff = s as f64 - self.avg_stride;
                diff * diff
            })
            .sum::<f64>() / strides.len() as f64;
        let stride_stddev = sqrt(stride_var);
        let cv = if self.avg_stride > 0.0 { stride_stddev / self.avg_stride } else { 1.0 };

        self.pattern = if self.sequentiality > 0.8 {
            IoPatternType::Sequential
        } else if cv < 0.1 && self.avg_stride > 0.0 {
            IoPatternType::Strided
        } else if self.sequentiality < 0.2 {
            IoPatternType::Random
        } else {
            IoPatternType::Mixed
        };
    }

    /// Read/write ratio
    pub fn rw_ratio(&self) -> f64 {
        if self.writes == 0 {
            return f64::INFINITY;
        }
        self.reads as f64 / self.writes as f64
    }

    /// Dominant IO size
    pub fn dominant_size(&self) -> u32 {
        self.size_buckets.iter()
            .max_by_key(|(_, v)| v.count)
            .map(|(&bucket, _)| 1u32 << bucket)
            .unwrap_or(4096)
    }

    /// Prefetch recommendation (bytes ahead)
    pub fn prefetch_bytes(&self) -> u64 {
        match self.pattern {
            IoPatternType::Sequential => 256 * 1024,     // 256KB
            IoPatternType::Strided => self.avg_stride as u64 * 4,
            IoPatternType::Periodic => 128 * 1024,
            _ => 0,
        }
    }
}

/// IO pattern analyzer stats
#[derive(Debug, Clone, Default)]
pub struct AppIoPatternStats {
    pub tracked_files: usize,
    pub sequential_files: usize,
    pub random_files: usize,
    pub total_reads: u64,
    pub total_writes: u64,
    pub avg_sequentiality: f64,
}

/// App IO pattern analyzer
pub struct AppIoPatternAnalyzer {
    /// Per-file patterns
    files: BTreeMap<u64, FileIoPattern>,
    /// Stats
    stats: AppIoPatternStats,
}

impl AppIoPatternAnalyzer {
    pub fn new() -> Self {
        Self {
            files: BTreeMap::new(),
            stats: AppIoPatternStats::default(),
        }
    }

    /// Record IO access
    pub fn record(&mut self, file_hash: u64, record: &IoAccessRecord) {
        let pattern = self.files.entry(file_hash)
            .or_insert_with(|| FileIoPattern::new(file_hash));
        pattern.record_access(record);
        self.update_stats();
    }

    /// Get pattern for file
    pub fn get_pattern(&self, file_hash: u64) -> Option<IoPatternType> {
        self.files.get(&file_hash).map(|f| f.pattern)
    }

    fn update_stats(&mut self) {
        self.stats.tracked_files = self.files.len();
        self.stats.sequential_files = self.files.values()
            .filter(|f| matches!(f.pattern, IoPatternType::Sequential))
            .count();
        self.stats.random_files = self.files.values()
            .filter(|f| matches!(f.pattern, IoPatternType::Random))
            .count();
        self.stats.total_reads = self.files.values().map(|f| f.reads).sum();
        self.stats.total_writes = self.files.values().map(|f| f.writes).sum();
        if !self.files.is_empty() {
            self.stats.avg_sequentiality = self.files.values()
                .map(|f| f.sequentiality)
                .sum::<f64>() / self.files.len() as f64;
        }
    }

    pub fn stats(&self) -> &AppIoPatternStats {
        &self.stats
    }
}
