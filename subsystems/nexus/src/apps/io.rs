//! # Application I/O Analysis
//!
//! Deep I/O behavior analysis per application:
//! - I/O pattern detection (sequential, random, mixed)
//! - Hot file detection
//! - I/O scheduling hints
//! - Bandwidth estimation
//! - Latency profiling per I/O type
//! - I/O dependency graph construction

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// I/O PATTERN TYPES
// ============================================================================

/// I/O access pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoPattern {
    /// Sequential reads/writes
    Sequential,
    /// Random access
    Random,
    /// Mixed sequential and random
    Mixed,
    /// Append-only
    AppendOnly,
    /// Read-mostly
    ReadMostly,
    /// Write-mostly
    WriteMostly,
    /// Read-write balanced
    Balanced,
    /// Metadata-heavy (stat, readdir)
    MetadataHeavy,
    /// Memory-mapped I/O
    MemoryMapped,
}

/// I/O operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoOpType {
    Read,
    Write,
    Seek,
    Sync,
    Stat,
    Open,
    Close,
    Readdir,
    Mmap,
    Truncate,
}

/// An I/O operation record
#[derive(Debug, Clone)]
pub struct IoOperation {
    /// File descriptor
    pub fd: u64,
    /// Operation type
    pub op_type: IoOpType,
    /// Offset
    pub offset: u64,
    /// Size (bytes)
    pub size: u64,
    /// Latency (ns)
    pub latency_ns: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Whether it hit the page cache
    pub cache_hit: bool,
}

// ============================================================================
// PER-FILE I/O TRACKER
// ============================================================================

/// I/O tracking for a single file
#[derive(Debug)]
pub struct FileIoTracker {
    /// File descriptor
    pub fd: u64,
    /// File identifier (inode or path hash)
    pub file_id: u64,
    /// Total reads
    pub read_count: u64,
    /// Total writes
    pub write_count: u64,
    /// Total read bytes
    pub read_bytes: u64,
    /// Total write bytes
    pub write_bytes: u64,
    /// Last read offset
    pub last_read_offset: u64,
    /// Last write offset
    pub last_write_offset: u64,
    /// Sequential read count
    sequential_reads: u64,
    /// Random read count
    random_reads: u64,
    /// Read latency sum (ns)
    read_latency_sum: u64,
    /// Write latency sum (ns)
    write_latency_sum: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Total I/O ops
    pub total_ops: u64,
    /// Detected pattern
    pub pattern: IoPattern,
}

impl FileIoTracker {
    pub fn new(fd: u64, file_id: u64) -> Self {
        Self {
            fd,
            file_id,
            read_count: 0,
            write_count: 0,
            read_bytes: 0,
            write_bytes: 0,
            last_read_offset: 0,
            last_write_offset: 0,
            sequential_reads: 0,
            random_reads: 0,
            read_latency_sum: 0,
            write_latency_sum: 0,
            cache_hits: 0,
            total_ops: 0,
            pattern: IoPattern::Sequential,
        }
    }

    /// Record a read operation
    pub fn record_read(&mut self, offset: u64, size: u64, latency_ns: u64, cache_hit: bool) {
        // Check if sequential
        if offset == self.last_read_offset + size || self.read_count == 0 {
            self.sequential_reads += 1;
        } else {
            self.random_reads += 1;
        }

        self.read_count += 1;
        self.read_bytes += size;
        self.read_latency_sum += latency_ns;
        self.last_read_offset = offset;
        self.total_ops += 1;
        if cache_hit {
            self.cache_hits += 1;
        }

        self.update_pattern();
    }

    /// Record a write operation
    pub fn record_write(&mut self, offset: u64, size: u64, latency_ns: u64) {
        self.write_count += 1;
        self.write_bytes += size;
        self.write_latency_sum += latency_ns;
        self.last_write_offset = offset;
        self.total_ops += 1;

        self.update_pattern();
    }

    fn update_pattern(&mut self) {
        let total = self.read_count + self.write_count;
        if total < 5 {
            return;
        }

        let read_ratio = self.read_count as f64 / total as f64;
        let seq_ratio = if self.read_count > 0 {
            self.sequential_reads as f64 / self.read_count as f64
        } else {
            0.0
        };

        if read_ratio > 0.8 {
            if seq_ratio > 0.7 {
                self.pattern = IoPattern::Sequential;
            } else {
                self.pattern = IoPattern::ReadMostly;
            }
        } else if read_ratio < 0.2 {
            self.pattern = IoPattern::WriteMostly;
        } else if seq_ratio > 0.7 {
            self.pattern = IoPattern::Sequential;
        } else if seq_ratio < 0.3 {
            self.pattern = IoPattern::Random;
        } else {
            self.pattern = IoPattern::Mixed;
        }
    }

    /// Average read latency (ns)
    pub fn avg_read_latency(&self) -> u64 {
        if self.read_count == 0 {
            0
        } else {
            self.read_latency_sum / self.read_count
        }
    }

    /// Average write latency (ns)
    pub fn avg_write_latency(&self) -> u64 {
        if self.write_count == 0 {
            0
        } else {
            self.write_latency_sum / self.write_count
        }
    }

    /// Cache hit rate
    pub fn cache_hit_rate(&self) -> f64 {
        if self.total_ops == 0 {
            0.0
        } else {
            self.cache_hits as f64 / self.total_ops as f64
        }
    }

    /// Total bytes
    pub fn total_bytes(&self) -> u64 {
        self.read_bytes + self.write_bytes
    }
}

// ============================================================================
// I/O SCHEDULER HINTS
// ============================================================================

/// I/O scheduling hint
#[derive(Debug, Clone)]
pub struct IoSchedulingHint {
    /// Process ID
    pub pid: u64,
    /// Recommended I/O priority
    pub priority: IoPriority,
    /// Recommended read-ahead size
    pub readahead_bytes: u64,
    /// Whether to enable direct I/O
    pub suggest_direct_io: bool,
    /// Whether to batch writes
    pub batch_writes: bool,
    /// Recommended write-behind size
    pub writebehind_bytes: u64,
    /// Confidence
    pub confidence: f64,
}

/// I/O priority class
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IoPriority {
    /// Background (best effort, lowest)
    Background = 0,
    /// Normal
    Normal = 1,
    /// High (interactive)
    High = 2,
    /// Realtime (latency-critical)
    Realtime = 3,
}

// ============================================================================
// BANDWIDTH ESTIMATOR
// ============================================================================

/// I/O bandwidth estimator
#[derive(Debug)]
pub struct BandwidthEstimator {
    /// Recent read bandwidth samples (bytes/sec)
    read_samples: Vec<f64>,
    /// Recent write bandwidth samples (bytes/sec)
    write_samples: Vec<f64>,
    /// Max samples
    max_samples: usize,
    /// Window start
    window_start: u64,
    /// Window bytes read
    window_read_bytes: u64,
    /// Window bytes written
    window_write_bytes: u64,
    /// Window duration (ms)
    window_ms: u64,
}

impl BandwidthEstimator {
    pub fn new(window_ms: u64) -> Self {
        Self {
            read_samples: Vec::new(),
            write_samples: Vec::new(),
            max_samples: 60,
            window_start: 0,
            window_read_bytes: 0,
            window_write_bytes: 0,
            window_ms,
        }
    }

    /// Record I/O
    pub fn record(&mut self, is_read: bool, bytes: u64, timestamp: u64) {
        if self.window_start == 0 {
            self.window_start = timestamp;
        }

        if is_read {
            self.window_read_bytes += bytes;
        } else {
            self.window_write_bytes += bytes;
        }

        // Rotate window
        if timestamp.saturating_sub(self.window_start) >= self.window_ms {
            let duration_sec = self.window_ms as f64 / 1000.0;
            let read_bw = self.window_read_bytes as f64 / duration_sec;
            let write_bw = self.window_write_bytes as f64 / duration_sec;

            if self.read_samples.len() >= self.max_samples {
                self.read_samples.remove(0);
            }
            self.read_samples.push(read_bw);

            if self.write_samples.len() >= self.max_samples {
                self.write_samples.remove(0);
            }
            self.write_samples.push(write_bw);

            self.window_start = timestamp;
            self.window_read_bytes = 0;
            self.window_write_bytes = 0;
        }
    }

    /// Current read bandwidth estimate (bytes/sec)
    pub fn read_bandwidth(&self) -> f64 {
        if self.read_samples.is_empty() {
            0.0
        } else {
            let recent = &self.read_samples[self.read_samples.len().saturating_sub(3)..];
            recent.iter().sum::<f64>() / recent.len() as f64
        }
    }

    /// Current write bandwidth estimate (bytes/sec)
    pub fn write_bandwidth(&self) -> f64 {
        if self.write_samples.is_empty() {
            0.0
        } else {
            let recent = &self.write_samples[self.write_samples.len().saturating_sub(3)..];
            recent.iter().sum::<f64>() / recent.len() as f64
        }
    }

    /// Total bandwidth
    pub fn total_bandwidth(&self) -> f64 {
        self.read_bandwidth() + self.write_bandwidth()
    }
}

// ============================================================================
// PER-PROCESS I/O ANALYZER
// ============================================================================

/// Comprehensive I/O analysis for a process
pub struct ProcessIoAnalyzer {
    /// Process ID
    pub pid: u64,
    /// Per-file trackers
    file_trackers: BTreeMap<u64, FileIoTracker>,
    /// Bandwidth estimator
    pub bandwidth: BandwidthEstimator,
    /// Max files to track
    max_files: usize,
    /// Overall I/O pattern
    pub overall_pattern: IoPattern,
    /// Total I/O operations
    pub total_ops: u64,
    /// Total bytes
    pub total_bytes: u64,
}

impl ProcessIoAnalyzer {
    pub fn new(pid: u64, max_files: usize) -> Self {
        Self {
            pid,
            file_trackers: BTreeMap::new(),
            bandwidth: BandwidthEstimator::new(1000),
            max_files,
            overall_pattern: IoPattern::Sequential,
            total_ops: 0,
            total_bytes: 0,
        }
    }

    /// Record an I/O operation
    pub fn record(&mut self, op: &IoOperation) {
        self.total_ops += 1;
        self.total_bytes += op.size;

        // Update per-file tracker
        if !self.file_trackers.contains_key(&op.fd) && self.file_trackers.len() < self.max_files {
            self.file_trackers
                .insert(op.fd, FileIoTracker::new(op.fd, op.fd));
        }

        if let Some(tracker) = self.file_trackers.get_mut(&op.fd) {
            match op.op_type {
                IoOpType::Read => tracker.record_read(op.offset, op.size, op.latency_ns, op.cache_hit),
                IoOpType::Write => tracker.record_write(op.offset, op.size, op.latency_ns),
                _ => {}
            }
        }

        // Update bandwidth
        let is_read = op.op_type == IoOpType::Read;
        self.bandwidth.record(is_read, op.size, op.timestamp);
    }

    /// Generate I/O scheduling hint
    pub fn scheduling_hint(&self) -> IoSchedulingHint {
        let is_sequential = self.file_trackers.values().any(|t| t.pattern == IoPattern::Sequential);
        let is_random = self.file_trackers.values().any(|t| t.pattern == IoPattern::Random);
        let bw = self.bandwidth.total_bandwidth();

        let priority = if bw > 100_000_000.0 {
            // >100 MB/s
            IoPriority::High
        } else if bw > 10_000_000.0 {
            IoPriority::Normal
        } else {
            IoPriority::Background
        };

        let readahead = if is_sequential { 256 * 1024 } else { 16 * 1024 };

        IoSchedulingHint {
            pid: self.pid,
            priority,
            readahead_bytes: readahead,
            suggest_direct_io: is_random && bw > 50_000_000.0,
            batch_writes: true,
            writebehind_bytes: 64 * 1024,
            confidence: if self.total_ops > 100 { 0.8 } else { 0.5 },
        }
    }

    /// Hot files (most I/O)
    pub fn hot_files(&self, n: usize) -> Vec<(u64, u64)> {
        let mut files: Vec<(u64, u64)> = self
            .file_trackers
            .iter()
            .map(|(&fd, t)| (fd, t.total_bytes()))
            .collect();
        files.sort_by(|a, b| b.1.cmp(&a.1));
        files.truncate(n);
        files
    }

    /// Close file
    pub fn close_file(&mut self, fd: u64) {
        self.file_trackers.remove(&fd);
    }
}

// ============================================================================
// GLOBAL I/O ANALYZER
// ============================================================================

/// System-wide I/O analysis
pub struct IoAnalyzer {
    /// Per-process analyzers
    analyzers: BTreeMap<u64, ProcessIoAnalyzer>,
    /// Max processes
    max_processes: usize,
    /// Max files per process
    max_files_per_process: usize,
}

impl IoAnalyzer {
    pub fn new(max_processes: usize, max_files: usize) -> Self {
        Self {
            analyzers: BTreeMap::new(),
            max_processes,
            max_files_per_process: max_files,
        }
    }

    /// Get or create analyzer for a process
    pub fn get_or_create(&mut self, pid: u64) -> &mut ProcessIoAnalyzer {
        let max_files = self.max_files_per_process;
        if !self.analyzers.contains_key(&pid) && self.analyzers.len() < self.max_processes {
            self.analyzers
                .insert(pid, ProcessIoAnalyzer::new(pid, max_files));
        }
        self.analyzers
            .entry(pid)
            .or_insert_with(|| ProcessIoAnalyzer::new(pid, max_files))
    }

    /// Get analyzer
    pub fn get(&self, pid: u64) -> Option<&ProcessIoAnalyzer> {
        self.analyzers.get(&pid)
    }

    /// Remove process
    pub fn remove_process(&mut self, pid: u64) {
        self.analyzers.remove(&pid);
    }

    /// Top I/O consumers
    pub fn top_consumers(&self, n: usize) -> Vec<(u64, u64)> {
        let mut consumers: Vec<(u64, u64)> = self
            .analyzers
            .iter()
            .map(|(&pid, a)| (pid, a.total_bytes))
            .collect();
        consumers.sort_by(|a, b| b.1.cmp(&a.1));
        consumers.truncate(n);
        consumers
    }
}
