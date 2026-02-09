//! Process Metrics
//!
//! Process performance metrics.

use crate::core::NexusTimestamp;
use super::ProcessId;

/// Process metrics snapshot
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ProcessMetrics {
    /// Process ID
    pub pid: ProcessId,
    /// Timestamp
    pub timestamp: NexusTimestamp,
    /// CPU time used (nanoseconds)
    pub cpu_time_ns: u64,
    /// User CPU time
    pub user_time_ns: u64,
    /// System/kernel CPU time
    pub system_time_ns: u64,
    /// Memory usage (bytes)
    pub memory_bytes: u64,
    /// Virtual memory size
    pub virtual_memory: u64,
    /// Page faults
    pub page_faults: u64,
    /// Minor page faults
    pub minor_faults: u64,
    /// Major page faults
    pub major_faults: u64,
    /// Voluntary context switches
    pub voluntary_switches: u64,
    /// Involuntary context switches
    pub involuntary_switches: u64,
    /// I/O read bytes
    pub io_read_bytes: u64,
    /// I/O write bytes
    pub io_write_bytes: u64,
    /// Thread count
    pub thread_count: u32,
    /// Open file count
    pub open_files: u32,
}

impl ProcessMetrics {
    /// Create new metrics
    pub fn new(pid: ProcessId) -> Self {
        Self {
            pid,
            timestamp: NexusTimestamp::now(),
            cpu_time_ns: 0,
            user_time_ns: 0,
            system_time_ns: 0,
            memory_bytes: 0,
            virtual_memory: 0,
            page_faults: 0,
            minor_faults: 0,
            major_faults: 0,
            voluntary_switches: 0,
            involuntary_switches: 0,
            io_read_bytes: 0,
            io_write_bytes: 0,
            thread_count: 1,
            open_files: 0,
        }
    }

    /// Calculate CPU usage ratio between two snapshots
    #[inline]
    pub fn cpu_usage(&self, previous: &Self, wall_time_ns: u64) -> f64 {
        if wall_time_ns == 0 {
            return 0.0;
        }

        let cpu_delta = self.cpu_time_ns.saturating_sub(previous.cpu_time_ns);
        (cpu_delta as f64 / wall_time_ns as f64).min(1.0)
    }

    /// Get memory delta
    #[inline(always)]
    pub fn memory_delta(&self, previous: &Self) -> i64 {
        self.memory_bytes as i64 - previous.memory_bytes as i64
    }

    /// Get I/O rate
    pub fn io_rate(&self, previous: &Self, duration_ns: u64) -> (f64, f64) {
        if duration_ns == 0 {
            return (0.0, 0.0);
        }

        let read_delta = self.io_read_bytes.saturating_sub(previous.io_read_bytes);
        let write_delta = self.io_write_bytes.saturating_sub(previous.io_write_bytes);

        let read_rate = read_delta as f64 * 1_000_000_000.0 / duration_ns as f64;
        let write_rate = write_delta as f64 * 1_000_000_000.0 / duration_ns as f64;

        (read_rate, write_rate)
    }
}
