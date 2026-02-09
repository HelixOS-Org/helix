//! Filesystem workload classification.

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec::Vec;

use crate::core::NexusTimestamp;

// ============================================================================
// I/O OPERATION TYPE
// ============================================================================

/// I/O operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IoOpType {
    Read,
    Write,
    Metadata,
    Sync,
}

/// I/O operation
#[derive(Debug, Clone, Copy)]
struct IoOperation {
    /// Operation type
    op_type: IoOpType,
    /// Size
    size: u32,
    /// Timestamp
    timestamp: u64,
}

// ============================================================================
// WORKLOAD TYPE
// ============================================================================

/// Filesystem workload type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsWorkloadType {
    /// Database workload
    Database,
    /// Web server
    WebServer,
    /// File server
    FileServer,
    /// Build system
    BuildSystem,
    /// Logging
    Logging,
    /// Backup
    Backup,
    /// General purpose
    General,
}

// ============================================================================
// OPTIMAL SETTINGS
// ============================================================================

/// Optimal filesystem settings
#[derive(Debug, Clone, Copy)]
pub struct FsOptimalSettings {
    /// Read-ahead in KB
    pub read_ahead_kb: u32,
    /// Writeback delay in ms
    pub writeback_delay_ms: u32,
    /// Sync on close
    pub sync_on_close: bool,
    /// Cache priority (0.0 - 1.0)
    pub cache_priority: f64,
}

impl Default for FsOptimalSettings {
    fn default() -> Self {
        Self {
            read_ahead_kb: 128,
            writeback_delay_ms: 3000,
            sync_on_close: false,
            cache_priority: 0.7,
        }
    }
}

// ============================================================================
// WORKLOAD CLASSIFIER
// ============================================================================

/// Classifies filesystem workloads
pub struct FsWorkloadClassifier {
    /// Recent I/O operations
    recent_ops: VecDeque<IoOperation>,
    /// Max recent ops
    max_ops: usize,
    /// Current workload type
    current_workload: FsWorkloadType,
    /// Workload confidence
    confidence: f64,
}

impl FsWorkloadClassifier {
    /// Create new classifier
    pub fn new() -> Self {
        Self {
            recent_ops: VecDeque::new(),
            max_ops: 10000,
            current_workload: FsWorkloadType::General,
            confidence: 0.0,
        }
    }

    /// Record I/O operation
    fn record(&mut self, op: IoOpType, size: u32) {
        self.recent_ops.push_back(IoOperation {
            op_type: op,
            size,
            timestamp: NexusTimestamp::now().raw(),
        });

        if self.recent_ops.len() > self.max_ops {
            self.recent_ops.pop_front();
        }

        if self.recent_ops.len() >= 1000 && self.recent_ops.len() % 100 == 0 {
            self.classify();
        }
    }

    /// Record read
    #[inline(always)]
    pub fn record_read(&mut self, size: u32) {
        self.record(IoOpType::Read, size);
    }

    /// Record write
    #[inline(always)]
    pub fn record_write(&mut self, size: u32) {
        self.record(IoOpType::Write, size);
    }

    /// Record metadata operation
    #[inline(always)]
    pub fn record_metadata(&mut self) {
        self.record(IoOpType::Metadata, 0);
    }

    /// Classify workload
    fn classify(&mut self) {
        let len = self.recent_ops.len() as f64;

        let reads = self
            .recent_ops
            .iter()
            .filter(|o| o.op_type == IoOpType::Read)
            .count() as f64;
        let writes = self
            .recent_ops
            .iter()
            .filter(|o| o.op_type == IoOpType::Write)
            .count() as f64;
        let metadata = self
            .recent_ops
            .iter()
            .filter(|o| o.op_type == IoOpType::Metadata)
            .count() as f64;
        let syncs = self
            .recent_ops
            .iter()
            .filter(|o| o.op_type == IoOpType::Sync)
            .count() as f64;

        let read_ratio = reads / len;
        let write_ratio = writes / len;
        let meta_ratio = metadata / len;
        let sync_ratio = syncs / len;

        // Average size
        let avg_size: f64 = self
            .recent_ops
            .iter()
            .filter(|o| o.size > 0)
            .map(|o| o.size as f64)
            .sum::<f64>()
            / self.recent_ops.iter().filter(|o| o.size > 0).count().max(1) as f64;

        // Classify based on patterns
        let (workload, conf) = if sync_ratio > 0.1 && avg_size < 8192.0 {
            (FsWorkloadType::Database, 0.8)
        } else if read_ratio > 0.8 && avg_size > 4096.0 {
            (FsWorkloadType::WebServer, 0.7)
        } else if read_ratio > 0.9 && avg_size > 65536.0 {
            (FsWorkloadType::FileServer, 0.75)
        } else if meta_ratio > 0.3 {
            (FsWorkloadType::BuildSystem, 0.7)
        } else if write_ratio > 0.8 && avg_size < 1024.0 {
            (FsWorkloadType::Logging, 0.75)
        } else if write_ratio > 0.9 && avg_size > 65536.0 {
            (FsWorkloadType::Backup, 0.8)
        } else {
            (FsWorkloadType::General, 0.5)
        };

        self.current_workload = workload;
        self.confidence = conf;
    }

    /// Get current workload
    #[inline(always)]
    pub fn current_workload(&self) -> FsWorkloadType {
        self.current_workload
    }

    /// Get confidence
    #[inline(always)]
    pub fn confidence(&self) -> f64 {
        self.confidence
    }

    /// Get optimal settings for workload
    pub fn optimal_settings(&self) -> FsOptimalSettings {
        match self.current_workload {
            FsWorkloadType::Database => FsOptimalSettings {
                read_ahead_kb: 32,
                writeback_delay_ms: 0,
                sync_on_close: true,
                cache_priority: 0.9,
            },
            FsWorkloadType::WebServer => FsOptimalSettings {
                read_ahead_kb: 256,
                writeback_delay_ms: 5000,
                sync_on_close: false,
                cache_priority: 0.95,
            },
            FsWorkloadType::FileServer => FsOptimalSettings {
                read_ahead_kb: 512,
                writeback_delay_ms: 3000,
                sync_on_close: false,
                cache_priority: 0.8,
            },
            FsWorkloadType::BuildSystem => FsOptimalSettings {
                read_ahead_kb: 128,
                writeback_delay_ms: 1000,
                sync_on_close: false,
                cache_priority: 0.7,
            },
            FsWorkloadType::Logging => FsOptimalSettings {
                read_ahead_kb: 16,
                writeback_delay_ms: 100,
                sync_on_close: false,
                cache_priority: 0.3,
            },
            FsWorkloadType::Backup => FsOptimalSettings {
                read_ahead_kb: 1024,
                writeback_delay_ms: 10000,
                sync_on_close: false,
                cache_priority: 0.5,
            },
            FsWorkloadType::General => FsOptimalSettings::default(),
        }
    }
}

impl Default for FsWorkloadClassifier {
    fn default() -> Self {
        Self::new()
    }
}
