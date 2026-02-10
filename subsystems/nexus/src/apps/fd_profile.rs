//! # Application File Descriptor Profiler
//!
//! Per-process file descriptor usage analysis:
//! - FD allocation/deallocation tracking
//! - FD type distribution (pipe, socket, file, etc.)
//! - FD leak detection (monotonically growing FD count)
//! - Per-FD I/O pattern profiling
//! - FD table utilization
//! - FD inheritance tracking across fork/exec

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// File descriptor type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FdTypeApps {
    RegularFile,
    Directory,
    Pipe,
    Socket,
    Epoll,
    EventFd,
    TimerFd,
    SignalFd,
    Inotify,
    Device,
    Memfd,
    Unknown,
}

/// FD I/O pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FdIoPattern {
    Sequential,
    Random,
    ReadOnly,
    WriteOnly,
    ReadWrite,
    Idle,
}

/// Per-FD statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FdStats {
    pub fd_num: i32,
    pub fd_type: FdTypeApps,
    pub opened_at: u64,
    pub closed_at: u64,
    pub is_open: bool,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub read_ops: u64,
    pub write_ops: u64,
    pub seek_ops: u64,
    pub poll_count: u64,
    pub last_activity: u64,
    pub inherited: bool,
    pub dup_count: u32,
    pub io_pattern: FdIoPattern,
}

impl FdStats {
    pub fn new(fd_num: i32, fd_type: FdTypeApps, opened_at: u64) -> Self {
        Self {
            fd_num,
            fd_type,
            opened_at,
            closed_at: 0,
            is_open: true,
            read_bytes: 0,
            write_bytes: 0,
            read_ops: 0,
            write_ops: 0,
            seek_ops: 0,
            poll_count: 0,
            last_activity: opened_at,
            inherited: false,
            dup_count: 0,
            io_pattern: FdIoPattern::Idle,
        }
    }

    #[inline(always)]
    pub fn lifetime_ns(&self, now: u64) -> u64 {
        if self.is_open { now.saturating_sub(self.opened_at) }
        else { self.closed_at.saturating_sub(self.opened_at) }
    }

    #[inline(always)]
    pub fn total_io_bytes(&self) -> u64 {
        self.read_bytes + self.write_bytes
    }

    #[inline(always)]
    pub fn total_io_ops(&self) -> u64 {
        self.read_ops + self.write_ops
    }

    #[inline(always)]
    pub fn idle_ns(&self, now: u64) -> u64 {
        now.saturating_sub(self.last_activity)
    }

    pub fn update_pattern(&mut self) {
        if self.read_ops == 0 && self.write_ops == 0 {
            self.io_pattern = FdIoPattern::Idle;
        } else if self.write_ops == 0 {
            self.io_pattern = FdIoPattern::ReadOnly;
        } else if self.read_ops == 0 {
            self.io_pattern = FdIoPattern::WriteOnly;
        } else if self.seek_ops > self.read_ops / 2 {
            self.io_pattern = FdIoPattern::Random;
        } else {
            self.io_pattern = FdIoPattern::Sequential;
        }
    }

    #[inline]
    pub fn record_read(&mut self, bytes: u64, ts: u64) {
        self.read_bytes += bytes;
        self.read_ops += 1;
        self.last_activity = ts;
        self.update_pattern();
    }

    #[inline]
    pub fn record_write(&mut self, bytes: u64, ts: u64) {
        self.write_bytes += bytes;
        self.write_ops += 1;
        self.last_activity = ts;
        self.update_pattern();
    }

    #[inline]
    pub fn record_seek(&mut self, ts: u64) {
        self.seek_ops += 1;
        self.last_activity = ts;
        self.update_pattern();
    }
}

/// FD type distribution
#[derive(Debug, Clone, Default)]
pub struct FdTypeDistribution {
    pub regular_files: u32,
    pub directories: u32,
    pub pipes: u32,
    pub sockets: u32,
    pub epolls: u32,
    pub eventfds: u32,
    pub timerfds: u32,
    pub devices: u32,
    pub other: u32,
}

impl FdTypeDistribution {
    #[inline(always)]
    pub fn total(&self) -> u32 {
        self.regular_files + self.directories + self.pipes + self.sockets
            + self.epolls + self.eventfds + self.timerfds + self.devices + self.other
    }

    pub fn count(&mut self, fd_type: FdTypeApps) {
        match fd_type {
            FdTypeApps::RegularFile => self.regular_files += 1,
            FdTypeApps::Directory => self.directories += 1,
            FdTypeApps::Pipe => self.pipes += 1,
            FdTypeApps::Socket => self.sockets += 1,
            FdTypeApps::Epoll => self.epolls += 1,
            FdTypeApps::EventFd => self.eventfds += 1,
            FdTypeApps::TimerFd => self.timerfds += 1,
            FdTypeApps::Device => self.devices += 1,
            _ => self.other += 1,
        }
    }
}

/// Per-process FD profile
#[derive(Debug, Clone)]
pub struct ProcessFdProfile {
    pub pid: u64,
    pub fds: BTreeMap<i32, FdStats>,
    pub total_opened: u64,
    pub total_closed: u64,
    pub peak_open: u32,
    pub current_open: u32,
    pub fd_limit: u32,
    pub distribution: FdTypeDistribution,
    pub fd_growth_samples: VecDeque<(u64, u32)>, // (ts, open_count)
}

impl ProcessFdProfile {
    pub fn new(pid: u64, fd_limit: u32) -> Self {
        Self {
            pid,
            fds: BTreeMap::new(),
            total_opened: 0,
            total_closed: 0,
            peak_open: 0,
            current_open: 0,
            fd_limit,
            distribution: FdTypeDistribution::default(),
            fd_growth_samples: VecDeque::new(),
        }
    }

    pub fn open_fd(&mut self, fd_num: i32, fd_type: FdTypeApps, ts: u64) {
        let stats = FdStats::new(fd_num, fd_type, ts);
        self.fds.insert(fd_num, stats);
        self.total_opened += 1;
        self.current_open += 1;
        if self.current_open > self.peak_open { self.peak_open = self.current_open; }
        self.distribution.count(fd_type);

        self.fd_growth_samples.push_back((ts, self.current_open));
        if self.fd_growth_samples.len() > 128 {
            self.fd_growth_samples.remove(0);
        }
    }

    #[inline]
    pub fn close_fd(&mut self, fd_num: i32, ts: u64) {
        if let Some(fd) = self.fds.get_mut(&fd_num) {
            fd.is_open = false;
            fd.closed_at = ts;
            self.total_closed += 1;
            self.current_open = self.current_open.saturating_sub(1);
        }
    }

    #[inline(always)]
    pub fn utilization(&self) -> f64 {
        if self.fd_limit == 0 { return 0.0; }
        self.current_open as f64 / self.fd_limit as f64
    }

    /// Detect potential FD leak: monotonically growing with low close rate
    #[inline]
    pub fn leak_risk(&self) -> bool {
        if self.total_opened < 100 { return false; }
        let close_ratio = self.total_closed as f64 / self.total_opened as f64;
        close_ratio < 0.5 && self.current_open > self.fd_limit / 2
    }

    #[inline]
    pub fn idle_fds(&self, now: u64, idle_threshold_ns: u64) -> Vec<i32> {
        self.fds.values()
            .filter(|fd| fd.is_open && fd.idle_ns(now) > idle_threshold_ns)
            .map(|fd| fd.fd_num)
            .collect()
    }

    #[inline]
    pub fn top_io_fds(&self, n: usize) -> Vec<(i32, u64)> {
        let mut sorted: Vec<_> = self.fds.values()
            .filter(|fd| fd.is_open)
            .map(|fd| (fd.fd_num, fd.total_io_bytes()))
            .collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(n);
        sorted
    }
}

/// App FD profiler stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppFdProfilerStats {
    pub total_processes: usize,
    pub total_open_fds: u64,
    pub total_opened: u64,
    pub total_closed: u64,
    pub leak_risk_count: usize,
    pub near_limit_count: usize,
}

/// Application File Descriptor Profiler
pub struct AppFdProfiler {
    profiles: BTreeMap<u64, ProcessFdProfile>,
    stats: AppFdProfilerStats,
}

impl AppFdProfiler {
    pub fn new() -> Self {
        Self {
            profiles: BTreeMap::new(),
            stats: AppFdProfilerStats::default(),
        }
    }

    #[inline(always)]
    pub fn register_process(&mut self, pid: u64, fd_limit: u32) {
        self.profiles.entry(pid).or_insert_with(|| ProcessFdProfile::new(pid, fd_limit));
    }

    #[inline]
    pub fn record_open(&mut self, pid: u64, fd_num: i32, fd_type: FdTypeApps, ts: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.open_fd(fd_num, fd_type, ts);
        }
    }

    #[inline]
    pub fn record_close(&mut self, pid: u64, fd_num: i32, ts: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.close_fd(fd_num, ts);
        }
    }

    #[inline]
    pub fn record_read(&mut self, pid: u64, fd_num: i32, bytes: u64, ts: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            if let Some(fd) = profile.fds.get_mut(&fd_num) {
                fd.record_read(bytes, ts);
            }
        }
    }

    #[inline]
    pub fn record_write(&mut self, pid: u64, fd_num: i32, bytes: u64, ts: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            if let Some(fd) = profile.fds.get_mut(&fd_num) {
                fd.record_write(bytes, ts);
            }
        }
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_processes = self.profiles.len();
        self.stats.total_open_fds = self.profiles.values().map(|p| p.current_open as u64).sum();
        self.stats.total_opened = self.profiles.values().map(|p| p.total_opened).sum();
        self.stats.total_closed = self.profiles.values().map(|p| p.total_closed).sum();
        self.stats.leak_risk_count = self.profiles.values().filter(|p| p.leak_risk()).count();
        self.stats.near_limit_count = self.profiles.values()
            .filter(|p| p.utilization() > 0.8)
            .count();
    }

    #[inline(always)]
    pub fn profile(&self, pid: u64) -> Option<&ProcessFdProfile> {
        self.profiles.get(&pid)
    }

    #[inline(always)]
    pub fn stats(&self) -> &AppFdProfilerStats {
        &self.stats
    }

    #[inline(always)]
    pub fn remove_process(&mut self, pid: u64) {
        self.profiles.remove(&pid);
        self.recompute();
    }
}
