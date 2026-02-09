//! # Application File Descriptor Tracker
//!
//! File descriptor usage analysis:
//! - FD table tracking
//! - FD leak detection
//! - FD type classification
//! - Inheritance tracking
//! - Close-on-exec analysis

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// FD TYPES
// ============================================================================

/// File descriptor type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FdType {
    /// Regular file
    RegularFile,
    /// Directory
    Directory,
    /// Pipe (read end)
    PipeRead,
    /// Pipe (write end)
    PipeWrite,
    /// Socket (TCP)
    SocketTcp,
    /// Socket (UDP)
    SocketUdp,
    /// Socket (Unix)
    SocketUnix,
    /// Eventfd
    EventFd,
    /// Timerfd
    TimerFd,
    /// Signalfd
    SignalFd,
    /// Epoll
    Epoll,
    /// Device
    Device,
    /// Other
    Other,
}

/// FD flags
#[derive(Debug, Clone, Copy)]
pub struct FdFlags {
    /// Close-on-exec
    pub cloexec: bool,
    /// Non-blocking
    pub nonblock: bool,
    /// Append mode
    pub append: bool,
}

impl FdFlags {
    pub fn new() -> Self {
        Self {
            cloexec: false,
            nonblock: false,
            append: false,
        }
    }
}

// ============================================================================
// FD ENTRY
// ============================================================================

/// File descriptor entry
#[derive(Debug, Clone)]
pub struct FdEntry {
    /// FD number
    pub fd: i32,
    /// Type
    pub fd_type: FdType,
    /// Flags
    pub flags: FdFlags,
    /// Opened at timestamp
    pub opened_at: u64,
    /// Read bytes
    pub read_bytes: u64,
    /// Written bytes
    pub write_bytes: u64,
    /// Operation count
    pub op_count: u64,
    /// Is inherited from parent
    pub inherited: bool,
    /// Reference count (dup'd)
    pub ref_count: u32,
}

impl FdEntry {
    pub fn new(fd: i32, fd_type: FdType, now: u64) -> Self {
        Self {
            fd,
            fd_type,
            flags: FdFlags::new(),
            opened_at: now,
            read_bytes: 0,
            write_bytes: 0,
            op_count: 0,
            inherited: false,
            ref_count: 1,
        }
    }

    /// Record I/O
    #[inline]
    pub fn record_io(&mut self, read: u64, write: u64) {
        self.read_bytes += read;
        self.write_bytes += write;
        self.op_count += 1;
    }

    /// Total bytes
    #[inline(always)]
    pub fn total_bytes(&self) -> u64 {
        self.read_bytes + self.write_bytes
    }

    /// Is active? (had I/O)
    #[inline(always)]
    pub fn is_active(&self) -> bool {
        self.op_count > 0
    }

    /// Age (ns)
    #[inline(always)]
    pub fn age_ns(&self, now: u64) -> u64 {
        now.saturating_sub(self.opened_at)
    }
}

// ============================================================================
// FD TABLE
// ============================================================================

/// Process FD table
#[derive(Debug)]
pub struct FdTable {
    /// FD entries
    entries: BTreeMap<i32, FdEntry>,
    /// High watermark
    pub high_watermark: i32,
    /// Total opened
    pub total_opened: u64,
    /// Total closed
    pub total_closed: u64,
}

impl FdTable {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            high_watermark: 0,
            total_opened: 0,
            total_closed: 0,
        }
    }

    /// Open FD
    #[inline]
    pub fn open(&mut self, fd: i32, fd_type: FdType, now: u64) {
        let entry = FdEntry::new(fd, fd_type, now);
        self.entries.insert(fd, entry);
        if fd > self.high_watermark {
            self.high_watermark = fd;
        }
        self.total_opened += 1;
    }

    /// Close FD
    #[inline(always)]
    pub fn close(&mut self, fd: i32) -> Option<FdEntry> {
        self.total_closed += 1;
        self.entries.remove(&fd)
    }

    /// Get entry
    #[inline(always)]
    pub fn get(&self, fd: i32) -> Option<&FdEntry> {
        self.entries.get(&fd)
    }

    /// Get mutable entry
    #[inline(always)]
    pub fn get_mut(&mut self, fd: i32) -> Option<&mut FdEntry> {
        self.entries.get_mut(&fd)
    }

    /// Current count
    #[inline(always)]
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// FDs by type
    #[inline]
    pub fn by_type(&self, fd_type: FdType) -> Vec<i32> {
        self.entries
            .iter()
            .filter(|(_, e)| e.fd_type == fd_type)
            .map(|(&fd, _)| fd)
            .collect()
    }

    /// Inactive FDs (potential leaks, older than threshold)
    #[inline]
    pub fn potential_leaks(&self, now: u64, min_age_ns: u64) -> Vec<i32> {
        self.entries
            .iter()
            .filter(|(_, e)| !e.is_active() && e.age_ns(now) > min_age_ns)
            .map(|(&fd, _)| fd)
            .collect()
    }

    /// FDs without cloexec
    #[inline]
    pub fn missing_cloexec(&self) -> Vec<i32> {
        self.entries
            .iter()
            .filter(|(_, e)| !e.flags.cloexec && !e.inherited)
            .map(|(&fd, _)| fd)
            .collect()
    }

    /// Socket count
    #[inline]
    pub fn socket_count(&self) -> usize {
        self.entries
            .values()
            .filter(|e| {
                matches!(
                    e.fd_type,
                    FdType::SocketTcp | FdType::SocketUdp | FdType::SocketUnix
                )
            })
            .count()
    }
}

// ============================================================================
// FD TRACKER ENGINE
// ============================================================================

/// FD stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppFdStats {
    /// Tracked processes
    pub tracked_processes: usize,
    /// Total open FDs
    pub total_open_fds: usize,
    /// Potential leaks detected
    pub potential_leaks: usize,
    /// Missing cloexec count
    pub missing_cloexec: usize,
}

/// App FD tracker
pub struct AppFdTracker {
    /// Per-process FD tables
    tables: BTreeMap<u64, FdTable>,
    /// Stats
    stats: AppFdStats,
}

impl AppFdTracker {
    pub fn new() -> Self {
        Self {
            tables: BTreeMap::new(),
            stats: AppFdStats::default(),
        }
    }

    /// Register process
    #[inline(always)]
    pub fn register(&mut self, pid: u64) {
        self.tables.insert(pid, FdTable::new());
        self.update_stats();
    }

    /// Open FD
    #[inline]
    pub fn open(&mut self, pid: u64, fd: i32, fd_type: FdType, now: u64) {
        if let Some(table) = self.tables.get_mut(&pid) {
            table.open(fd, fd_type, now);
        }
        self.update_stats();
    }

    /// Close FD
    #[inline]
    pub fn close(&mut self, pid: u64, fd: i32) {
        if let Some(table) = self.tables.get_mut(&pid) {
            table.close(fd);
        }
        self.update_stats();
    }

    /// Record I/O on FD
    #[inline]
    pub fn record_io(&mut self, pid: u64, fd: i32, read: u64, write: u64) {
        if let Some(table) = self.tables.get_mut(&pid) {
            if let Some(entry) = table.get_mut(fd) {
                entry.record_io(read, write);
            }
        }
    }

    /// Remove process
    #[inline(always)]
    pub fn remove(&mut self, pid: u64) {
        self.tables.remove(&pid);
        self.update_stats();
    }

    /// Get FD table
    #[inline(always)]
    pub fn table(&self, pid: u64) -> Option<&FdTable> {
        self.tables.get(&pid)
    }

    /// Check for leaks across all processes
    #[inline]
    pub fn check_leaks(&self, now: u64, min_age_ns: u64) -> Vec<(u64, Vec<i32>)> {
        self.tables
            .iter()
            .map(|(&pid, table)| (pid, table.potential_leaks(now, min_age_ns)))
            .filter(|(_, leaks)| !leaks.is_empty())
            .collect()
    }

    fn update_stats(&mut self) {
        self.stats.tracked_processes = self.tables.len();
        self.stats.total_open_fds = self.tables.values().map(|t| t.count()).sum();
        // Leak detection deferred to explicit check
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &AppFdStats {
        &self.stats
    }
}
