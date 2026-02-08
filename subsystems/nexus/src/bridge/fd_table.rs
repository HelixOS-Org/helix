//! # Bridge Fd Table Proxy
//!
//! File descriptor table intelligence:
//! - FD usage tracking per process
//! - FD leak detection
//! - FD inheritance analysis
//! - Close-on-exec tracking
//! - FD table pressure monitoring

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// FD type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FdType {
    Regular,
    Directory,
    Socket,
    Pipe,
    Epoll,
    EventFd,
    TimerFd,
    SignalFd,
    Inotify,
    Device,
    Unknown,
}

/// FD flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FdFlags {
    /// Close on exec
    pub cloexec: bool,
    /// Non-blocking
    pub nonblock: bool,
    /// Append
    pub append: bool,
    /// Direct I/O
    pub direct: bool,
}

impl FdFlags {
    pub fn new() -> Self {
        Self {
            cloexec: false,
            nonblock: false,
            append: false,
            direct: false,
        }
    }
}

/// FD entry
#[derive(Debug, Clone)]
pub struct FdEntry {
    /// File descriptor number
    pub fd: u32,
    /// Type
    pub fd_type: FdType,
    /// Flags
    pub flags: FdFlags,
    /// Open timestamp (ns)
    pub opened_ns: u64,
    /// Last activity (ns)
    pub last_activity_ns: u64,
    /// Read count
    pub reads: u64,
    /// Write count
    pub writes: u64,
    /// Bytes read
    pub bytes_read: u64,
    /// Bytes written
    pub bytes_written: u64,
    /// Inode hash (FNV-1a of path)
    pub inode_hash: u64,
}

impl FdEntry {
    pub fn new(fd: u32, fd_type: FdType, now_ns: u64) -> Self {
        Self {
            fd,
            fd_type,
            flags: FdFlags::new(),
            opened_ns: now_ns,
            last_activity_ns: now_ns,
            reads: 0,
            writes: 0,
            bytes_read: 0,
            bytes_written: 0,
            inode_hash: 0,
        }
    }

    /// Age in ns
    pub fn age_ns(&self, now_ns: u64) -> u64 {
        now_ns.saturating_sub(self.opened_ns)
    }

    /// Idle time (since last activity)
    pub fn idle_ns(&self, now_ns: u64) -> u64 {
        now_ns.saturating_sub(self.last_activity_ns)
    }

    /// Is potentially leaked? (old + no activity)
    pub fn is_leak_candidate(&self, now_ns: u64) -> bool {
        let age = self.age_ns(now_ns);
        let idle = self.idle_ns(now_ns);
        // Open for >60s and idle for >30s with no I/O
        age > 60_000_000_000 && idle > 30_000_000_000 && self.reads == 0 && self.writes == 0
    }
}

/// Per-process FD table
#[derive(Debug)]
pub struct ProcessFdTable {
    /// PID
    pub pid: u64,
    /// FD entries
    fds: BTreeMap<u32, FdEntry>,
    /// Max FDs observed
    pub max_fds_observed: u32,
    /// Total opens
    pub total_opens: u64,
    /// Total closes
    pub total_closes: u64,
    /// FD limit
    pub fd_limit: u32,
}

impl ProcessFdTable {
    pub fn new(pid: u64, fd_limit: u32) -> Self {
        Self {
            pid,
            fds: BTreeMap::new(),
            max_fds_observed: 0,
            total_opens: 0,
            total_closes: 0,
            fd_limit,
        }
    }

    /// Open FD
    pub fn open_fd(&mut self, fd: u32, fd_type: FdType, now_ns: u64) {
        self.fds.insert(fd, FdEntry::new(fd, fd_type, now_ns));
        self.total_opens += 1;
        let current = self.fds.len() as u32;
        if current > self.max_fds_observed {
            self.max_fds_observed = current;
        }
    }

    /// Close FD
    pub fn close_fd(&mut self, fd: u32) -> Option<FdEntry> {
        self.total_closes += 1;
        self.fds.remove(&fd)
    }

    /// Record activity
    pub fn record_activity(&mut self, fd: u32, is_read: bool, bytes: u64, now_ns: u64) {
        if let Some(entry) = self.fds.get_mut(&fd) {
            entry.last_activity_ns = now_ns;
            if is_read {
                entry.reads += 1;
                entry.bytes_read += bytes;
            } else {
                entry.writes += 1;
                entry.bytes_written += bytes;
            }
        }
    }

    /// Current open FDs
    pub fn open_count(&self) -> usize {
        self.fds.len()
    }

    /// Pressure (ratio of open to limit)
    pub fn pressure(&self) -> f64 {
        if self.fd_limit == 0 {
            return 0.0;
        }
        self.fds.len() as f64 / self.fd_limit as f64
    }

    /// Leak candidates
    pub fn leak_candidates(&self, now_ns: u64) -> Vec<u32> {
        self.fds.iter()
            .filter(|(_, e)| e.is_leak_candidate(now_ns))
            .map(|(&fd, _)| fd)
            .collect()
    }

    /// FDs by type
    pub fn count_by_type(&self) -> BTreeMap<u8, u32> {
        let mut counts = BTreeMap::new();
        for entry in self.fds.values() {
            *counts.entry(entry.fd_type as u8).or_insert(0) += 1;
        }
        counts
    }

    /// Close-on-exec count
    pub fn cloexec_count(&self) -> usize {
        self.fds.values().filter(|e| e.flags.cloexec).count()
    }
}

/// FD table proxy stats
#[derive(Debug, Clone, Default)]
pub struct BridgeFdTableStats {
    pub tracked_processes: usize,
    pub total_open_fds: usize,
    pub total_leak_candidates: usize,
    pub max_pressure: f64,
    pub total_opens: u64,
    pub total_closes: u64,
}

/// Bridge FD table proxy
pub struct BridgeFdTableProxy {
    /// Per-process tables
    tables: BTreeMap<u64, ProcessFdTable>,
    /// Stats
    stats: BridgeFdTableStats,
}

impl BridgeFdTableProxy {
    pub fn new() -> Self {
        Self {
            tables: BTreeMap::new(),
            stats: BridgeFdTableStats::default(),
        }
    }

    /// Register process
    pub fn register_process(&mut self, pid: u64, fd_limit: u32) {
        self.tables.insert(pid, ProcessFdTable::new(pid, fd_limit));
        self.update_stats();
    }

    /// Record FD open
    pub fn open_fd(&mut self, pid: u64, fd: u32, fd_type: FdType, now_ns: u64) {
        if let Some(table) = self.tables.get_mut(&pid) {
            table.open_fd(fd, fd_type, now_ns);
        }
        self.update_stats();
    }

    /// Record FD close
    pub fn close_fd(&mut self, pid: u64, fd: u32) -> Option<FdEntry> {
        let result = self.tables.get_mut(&pid).and_then(|t| t.close_fd(fd));
        self.update_stats();
        result
    }

    /// Scan for leaks
    pub fn scan_leaks(&self, now_ns: u64) -> Vec<(u64, Vec<u32>)> {
        self.tables.iter()
            .filter_map(|(&pid, table)| {
                let leaks = table.leak_candidates(now_ns);
                if leaks.is_empty() { None } else { Some((pid, leaks)) }
            })
            .collect()
    }

    fn update_stats(&mut self) {
        self.stats.tracked_processes = self.tables.len();
        self.stats.total_open_fds = self.tables.values().map(|t| t.open_count()).sum();
        self.stats.max_pressure = self.tables.values()
            .map(|t| t.pressure())
            .fold(0.0_f64, f64::max);
        self.stats.total_opens = self.tables.values().map(|t| t.total_opens).sum();
        self.stats.total_closes = self.tables.values().map(|t| t.total_closes).sum();
    }

    pub fn stats(&self) -> &BridgeFdTableStats {
        &self.stats
    }
}
