//! Disk Statistics
//!
//! Block device performance statistics.

/// Disk statistics (like /proc/diskstats)
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct DiskStats {
    /// Reads completed
    pub reads_completed: u64,
    /// Reads merged
    pub reads_merged: u64,
    /// Sectors read
    pub sectors_read: u64,
    /// Read time (ms)
    pub read_time_ms: u64,
    /// Writes completed
    pub writes_completed: u64,
    /// Writes merged
    pub writes_merged: u64,
    /// Sectors written
    pub sectors_written: u64,
    /// Write time (ms)
    pub write_time_ms: u64,
    /// I/Os in progress
    pub ios_in_progress: u64,
    /// I/O time (ms)
    pub io_time_ms: u64,
    /// Weighted I/O time (ms)
    pub weighted_io_time_ms: u64,
    /// Discards completed
    pub discards_completed: u64,
    /// Discards merged
    pub discards_merged: u64,
    /// Sectors discarded
    pub sectors_discarded: u64,
    /// Discard time (ms)
    pub discard_time_ms: u64,
}

impl DiskStats {
    /// Create new stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Total I/O operations
    #[inline(always)]
    pub fn total_ios(&self) -> u64 {
        self.reads_completed + self.writes_completed + self.discards_completed
    }

    /// Total bytes read
    #[inline(always)]
    pub fn bytes_read(&self) -> u64 {
        self.sectors_read * 512
    }

    /// Total bytes written
    #[inline(always)]
    pub fn bytes_written(&self) -> u64 {
        self.sectors_written * 512
    }

    /// Utilization (0-1)
    #[inline]
    pub fn utilization(&self, time_window_ms: u64) -> f32 {
        if time_window_ms > 0 {
            (self.io_time_ms as f32 / time_window_ms as f32).min(1.0)
        } else {
            0.0
        }
    }

    /// Average I/O size (bytes)
    #[inline]
    pub fn avg_io_size(&self) -> u64 {
        let total_ios = self.total_ios();
        if total_ios > 0 {
            (self.sectors_read + self.sectors_written) * 512 / total_ios
        } else {
            0
        }
    }
}
