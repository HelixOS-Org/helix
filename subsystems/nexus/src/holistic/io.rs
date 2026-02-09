//! # Holistic I/O Management
//!
//! System-wide I/O optimization:
//! - I/O scheduler integration
//! - Bandwidth allocation across processes
//! - I/O priority management
//! - Read-ahead tuning
//! - Write-back policy
//! - Device queue management
//! - I/O merging and coalescing
//! - Latency tracking per device

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// I/O DEVICE MODEL
// ============================================================================

/// Device type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeviceType {
    /// NVMe SSD
    NvmeSsd,
    /// SATA SSD
    SataSsd,
    /// SATA HDD
    SataHdd,
    /// RAM disk
    RamDisk,
    /// Network storage (NFS, iSCSI)
    Network,
    /// Virtual device
    Virtual,
    /// Unknown
    Unknown,
}

impl DeviceType {
    /// Typical max IOPS
    #[inline]
    pub fn typical_max_iops(&self) -> u64 {
        match self {
            Self::NvmeSsd => 500_000,
            Self::SataSsd => 80_000,
            Self::SataHdd => 200,
            Self::RamDisk => 10_000_000,
            Self::Network => 10_000,
            Self::Virtual => 100_000,
            Self::Unknown => 10_000,
        }
    }

    /// Typical max bandwidth (bytes/sec)
    #[inline]
    pub fn typical_max_bw(&self) -> u64 {
        match self {
            Self::NvmeSsd => 3_500_000_000,
            Self::SataSsd => 550_000_000,
            Self::SataHdd => 150_000_000,
            Self::RamDisk => 20_000_000_000,
            Self::Network => 125_000_000,
            Self::Virtual => 1_000_000_000,
            Self::Unknown => 100_000_000,
        }
    }

    /// Is rotational?
    #[inline(always)]
    pub fn is_rotational(&self) -> bool {
        matches!(self, Self::SataHdd)
    }
}

/// Device statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct DeviceStats {
    /// Device ID
    pub device_id: u64,
    /// Device type
    pub device_type: DeviceType,
    /// Current IOPS (reads)
    pub read_iops: u64,
    /// Current IOPS (writes)
    pub write_iops: u64,
    /// Read bandwidth (bytes/sec)
    pub read_bw: u64,
    /// Write bandwidth (bytes/sec)
    pub write_bw: u64,
    /// Queue depth
    pub queue_depth: u32,
    /// Average read latency (microseconds)
    pub avg_read_latency_us: u64,
    /// Average write latency (microseconds)
    pub avg_write_latency_us: u64,
    /// P99 read latency
    pub p99_read_latency_us: u64,
    /// P99 write latency
    pub p99_write_latency_us: u64,
    /// Utilization (percent * 100)
    pub utilization: u32,
    /// Errors
    pub errors: u64,
}

impl DeviceStats {
    pub fn new(device_id: u64, device_type: DeviceType) -> Self {
        Self {
            device_id,
            device_type,
            read_iops: 0,
            write_iops: 0,
            read_bw: 0,
            write_bw: 0,
            queue_depth: 0,
            avg_read_latency_us: 0,
            avg_write_latency_us: 0,
            p99_read_latency_us: 0,
            p99_write_latency_us: 0,
            utilization: 0,
            errors: 0,
        }
    }

    /// Total IOPS
    #[inline(always)]
    pub fn total_iops(&self) -> u64 {
        self.read_iops + self.write_iops
    }

    /// Total bandwidth
    #[inline(always)]
    pub fn total_bw(&self) -> u64 {
        self.read_bw + self.write_bw
    }

    /// Is saturated?
    #[inline(always)]
    pub fn is_saturated(&self) -> bool {
        self.utilization > 9000 // > 90%
    }
}

// ============================================================================
// I/O PRIORITY
// ============================================================================

/// I/O scheduling class
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IoSchedClass {
    /// Real-time (guaranteed bandwidth)
    RealTime,
    /// Best effort
    BestEffort,
    /// Idle (only when nothing else pending)
    Idle,
}

/// Per-process I/O priority
#[derive(Debug, Clone)]
pub struct IoProcessPriority {
    /// PID
    pub pid: u64,
    /// Scheduling class
    pub sched_class: IoSchedClass,
    /// Priority within class (0-7, lower = higher priority)
    pub priority: u8,
    /// Bandwidth limit (0 = unlimited, bytes/sec)
    pub bw_limit: u64,
    /// IOPS limit (0 = unlimited)
    pub iops_limit: u64,
    /// Current bandwidth usage
    pub current_bw: u64,
    /// Current IOPS
    pub current_iops: u64,
    /// Total bytes read
    pub total_read: u64,
    /// Total bytes written
    pub total_written: u64,
    /// Total I/O operations
    pub total_ops: u64,
}

impl IoProcessPriority {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            sched_class: IoSchedClass::BestEffort,
            priority: 4,
            bw_limit: 0,
            iops_limit: 0,
            current_bw: 0,
            current_iops: 0,
            total_read: 0,
            total_written: 0,
            total_ops: 0,
        }
    }

    /// Is throttled?
    #[inline(always)]
    pub fn is_throttled(&self) -> bool {
        (self.bw_limit > 0 && self.current_bw >= self.bw_limit)
            || (self.iops_limit > 0 && self.current_iops >= self.iops_limit)
    }
}

// ============================================================================
// I/O REQUEST MERGING
// ============================================================================

/// An I/O request
#[derive(Debug, Clone)]
pub struct IoRequest {
    /// Request ID
    pub id: u64,
    /// PID
    pub pid: u64,
    /// Device ID
    pub device_id: u64,
    /// Is write
    pub is_write: bool,
    /// Offset (bytes)
    pub offset: u64,
    /// Length (bytes)
    pub length: u32,
    /// Priority
    pub priority: u8,
    /// Submit time
    pub submit_time: u64,
}

/// I/O merge result
#[derive(Debug, Clone)]
pub struct MergedRequest {
    /// Original request IDs
    pub original_ids: Vec<u64>,
    /// Device ID
    pub device_id: u64,
    /// Is write
    pub is_write: bool,
    /// Merged offset
    pub offset: u64,
    /// Merged length
    pub length: u64,
    /// Best priority
    pub priority: u8,
}

/// I/O merge engine
pub struct IoMergeEngine {
    /// Pending requests per device
    pending: BTreeMap<u64, Vec<IoRequest>>,
    /// Max merge size (bytes)
    max_merge_size: u64,
    /// Max merge count
    max_merge_count: usize,
    /// Total merges performed
    pub total_merges: u64,
    /// Total requests processed
    pub total_requests: u64,
}

impl IoMergeEngine {
    pub fn new(max_merge_size: u64, max_merge_count: usize) -> Self {
        Self {
            pending: BTreeMap::new(),
            max_merge_size,
            max_merge_count,
            total_merges: 0,
            total_requests: 0,
        }
    }

    /// Submit request
    #[inline]
    pub fn submit(&mut self, request: IoRequest) {
        self.total_requests += 1;
        self.pending
            .entry(request.device_id)
            .or_insert_with(Vec::new)
            .push(request);
    }

    /// Try to merge pending requests for a device
    pub fn merge(&mut self, device_id: u64) -> Vec<MergedRequest> {
        let requests = match self.pending.get_mut(&device_id) {
            Some(r) => r,
            None => return Vec::new(),
        };

        if requests.is_empty() {
            return Vec::new();
        }

        // Sort by offset
        requests.sort_by_key(|r| r.offset);

        let mut merged = Vec::new();
        let mut current_ids = Vec::new();
        let mut current_offset = 0u64;
        let mut current_end = 0u64;
        let mut current_is_write = false;
        let mut current_priority = u8::MAX;

        for req in requests.iter() {
            if current_ids.is_empty() {
                // Start new merge group
                current_ids.push(req.id);
                current_offset = req.offset;
                current_end = req.offset + req.length as u64;
                current_is_write = req.is_write;
                current_priority = req.priority;
                continue;
            }

            // Check if can merge
            let contiguous = req.offset <= current_end;
            let same_type = req.is_write == current_is_write;
            let within_size = (req.offset + req.length as u64 - current_offset) <= self.max_merge_size;
            let within_count = current_ids.len() < self.max_merge_count;

            if contiguous && same_type && within_size && within_count {
                current_ids.push(req.id);
                let new_end = req.offset + req.length as u64;
                if new_end > current_end {
                    current_end = new_end;
                }
                if req.priority < current_priority {
                    current_priority = req.priority;
                }
                self.total_merges += 1;
            } else {
                // Emit current group
                merged.push(MergedRequest {
                    original_ids: core::mem::take(&mut current_ids),
                    device_id,
                    is_write: current_is_write,
                    offset: current_offset,
                    length: current_end - current_offset,
                    priority: current_priority,
                });

                // Start new group
                current_ids.push(req.id);
                current_offset = req.offset;
                current_end = req.offset + req.length as u64;
                current_is_write = req.is_write;
                current_priority = req.priority;
            }
        }

        // Emit last group
        if !current_ids.is_empty() {
            merged.push(MergedRequest {
                original_ids: current_ids,
                device_id,
                is_write: current_is_write,
                offset: current_offset,
                length: current_end - current_offset,
                priority: current_priority,
            });
        }

        // Clear pending
        if let Some(r) = self.pending.get_mut(&device_id) {
            r.clear();
        }

        merged
    }

    /// Pending request count
    #[inline(always)]
    pub fn pending_count(&self, device_id: u64) -> usize {
        self.pending.get(&device_id).map_or(0, |r| r.len())
    }
}

// ============================================================================
// HOLISTIC I/O MANAGER
// ============================================================================

/// Writeback policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WritebackPolicy {
    /// Write-through (sync)
    WriteThrough,
    /// Write-back (async, delayed)
    WriteBack,
    /// Write-back with periodic flush
    PeriodicFlush,
    /// No-write (read-only)
    NoWrite,
}

/// Read-ahead configuration
#[derive(Debug, Clone)]
pub struct ReadAheadConfig {
    /// Default read-ahead size (pages)
    pub default_pages: u32,
    /// Max read-ahead (pages)
    pub max_pages: u32,
    /// Sequential threshold (ops before activating)
    pub sequential_threshold: u32,
    /// Random read-ahead (pages)
    pub random_pages: u32,
}

impl Default for ReadAheadConfig {
    fn default() -> Self {
        Self {
            default_pages: 32,
            max_pages: 256,
            sequential_threshold: 4,
            random_pages: 4,
        }
    }
}

/// System-wide I/O manager
pub struct HolisticIoManager {
    /// Device statistics
    devices: BTreeMap<u64, DeviceStats>,
    /// Per-process I/O priorities
    priorities: BTreeMap<u64, IoProcessPriority>,
    /// Merge engine
    merge_engine: IoMergeEngine,
    /// Read-ahead config
    read_ahead: ReadAheadConfig,
    /// Writeback policy
    writeback: WritebackPolicy,
    /// Total I/O bytes
    pub total_bytes: u64,
    /// Total I/O operations
    pub total_ops: u64,
}

impl HolisticIoManager {
    pub fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
            priorities: BTreeMap::new(),
            merge_engine: IoMergeEngine::new(1024 * 1024, 64), // 1MB max merge
            read_ahead: ReadAheadConfig::default(),
            writeback: WritebackPolicy::WriteBack,
            total_bytes: 0,
            total_ops: 0,
        }
    }

    /// Register device
    #[inline(always)]
    pub fn register_device(&mut self, device_id: u64, device_type: DeviceType) {
        self.devices
            .insert(device_id, DeviceStats::new(device_id, device_type));
    }

    /// Update device stats
    #[inline(always)]
    pub fn update_device(&mut self, stats: DeviceStats) {
        self.devices.insert(stats.device_id, stats);
    }

    /// Register process
    #[inline]
    pub fn register_process(&mut self, pid: u64) {
        self.priorities
            .entry(pid)
            .or_insert_with(|| IoProcessPriority::new(pid));
    }

    /// Unregister process
    #[inline(always)]
    pub fn unregister_process(&mut self, pid: u64) {
        self.priorities.remove(&pid);
    }

    /// Set I/O priority
    #[inline]
    pub fn set_priority(&mut self, pid: u64, class: IoSchedClass, priority: u8) {
        if let Some(p) = self.priorities.get_mut(&pid) {
            p.sched_class = class;
            p.priority = priority.min(7);
        }
    }

    /// Set bandwidth limit
    #[inline]
    pub fn set_bw_limit(&mut self, pid: u64, limit: u64) {
        if let Some(p) = self.priorities.get_mut(&pid) {
            p.bw_limit = limit;
        }
    }

    /// Submit I/O request
    #[inline]
    pub fn submit_request(&mut self, request: IoRequest) {
        self.total_ops += 1;
        self.total_bytes += request.length as u64;
        self.merge_engine.submit(request);
    }

    /// Process merged requests for device
    #[inline(always)]
    pub fn process_device(&mut self, device_id: u64) -> Vec<MergedRequest> {
        self.merge_engine.merge(device_id)
    }

    /// Get device stats
    #[inline(always)]
    pub fn device_stats(&self, device_id: u64) -> Option<&DeviceStats> {
        self.devices.get(&device_id)
    }

    /// Get process priority
    #[inline(always)]
    pub fn process_priority(&self, pid: u64) -> Option<&IoProcessPriority> {
        self.priorities.get(&pid)
    }

    /// Get merge engine stats
    #[inline(always)]
    pub fn merge_stats(&self) -> (u64, u64) {
        (self.merge_engine.total_merges, self.merge_engine.total_requests)
    }

    /// Device count
    #[inline(always)]
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    /// Process count
    #[inline(always)]
    pub fn process_count(&self) -> usize {
        self.priorities.len()
    }
}
