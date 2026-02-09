//! # Apps File Descriptor Manager
//!
//! Application file descriptor table management:
//! - Per-process FD table with CLOEXEC tracking
//! - FD duplication (dup/dup2/dup3)
//! - Close-on-exec flag management
//! - FD limit enforcement
//! - File description sharing across fork
//! - FD leak detection heuristics

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// File descriptor type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FdType {
    Regular,
    Directory,
    Socket,
    Pipe,
    Fifo,
    CharDevice,
    BlockDevice,
    Eventfd,
    Timerfd,
    Signalfd,
    Epoll,
    Inotify,
    Fanotify,
}

/// FD flags
#[derive(Debug, Clone, Copy, Default)]
pub struct FdFlags {
    pub cloexec: bool,
    pub nonblock: bool,
    pub append: bool,
    pub direct: bool,
    pub sync: bool,
    pub dsync: bool,
}

/// File description (shared across dup/fork)
#[derive(Debug, Clone)]
pub struct FileDescription {
    pub id: u64,
    pub fd_type: FdType,
    pub inode: u64,
    pub offset: u64,
    pub ref_count: u32,
    pub mode: u32,
    pub flags: FdFlags,
    pub path_hash: u64,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub read_ops: u64,
    pub write_ops: u64,
}

impl FileDescription {
    pub fn new(id: u64, ftype: FdType, inode: u64, mode: u32) -> Self {
        Self {
            id, fd_type: ftype, inode, offset: 0, ref_count: 1,
            mode, flags: FdFlags::default(), path_hash: 0,
            read_bytes: 0, write_bytes: 0, read_ops: 0, write_ops: 0,
        }
    }

    #[inline(always)]
    pub fn add_ref(&mut self) { self.ref_count += 1; }
    #[inline(always)]
    pub fn release(&mut self) -> bool {
        self.ref_count = self.ref_count.saturating_sub(1);
        self.ref_count == 0
    }
}

/// FD table entry
#[derive(Debug, Clone)]
pub struct FdEntry {
    pub fd: i32,
    pub description_id: u64,
    pub flags: FdFlags,
    pub opened_ns: u64,
    pub last_io_ns: u64,
}

/// Per-process FD table
#[derive(Debug, Clone)]
pub struct ProcessFdTable {
    pub process_id: u64,
    pub entries: BTreeMap<i32, FdEntry>,
    pub fd_limit: u32,
    pub next_fd: i32,
    pub total_opens: u64,
    pub total_closes: u64,
    pub peak_fd_count: usize,
}

impl ProcessFdTable {
    pub fn new(pid: u64, limit: u32) -> Self {
        Self {
            process_id: pid,
            entries: BTreeMap::new(),
            fd_limit: limit,
            next_fd: 3, // 0,1,2 reserved for stdin/stdout/stderr
            total_opens: 0,
            total_closes: 0,
            peak_fd_count: 0,
        }
    }

    pub fn alloc_fd(&mut self, desc_id: u64, flags: FdFlags, ts: u64) -> Option<i32> {
        if self.entries.len() >= self.fd_limit as usize { return None; }

        // Find lowest available FD
        let mut fd = 0i32;
        while self.entries.contains_key(&fd) { fd += 1; }

        self.entries.insert(fd, FdEntry {
            fd, description_id: desc_id, flags, opened_ns: ts, last_io_ns: 0,
        });
        self.total_opens += 1;
        if self.entries.len() > self.peak_fd_count {
            self.peak_fd_count = self.entries.len();
        }
        Some(fd)
    }

    #[inline]
    pub fn close_fd(&mut self, fd: i32) -> Option<u64> {
        if let Some(entry) = self.entries.remove(&fd) {
            self.total_closes += 1;
            Some(entry.description_id)
        } else { None }
    }

    #[inline]
    pub fn dup(&mut self, oldfd: i32, ts: u64) -> Option<i32> {
        let desc_id = self.entries.get(&oldfd)?.description_id;
        let flags = FdFlags::default(); // dup clears CLOEXEC
        self.alloc_fd(desc_id, flags, ts)
    }

    pub fn dup2(&mut self, oldfd: i32, newfd: i32, ts: u64) -> Option<i32> {
        if oldfd == newfd { return Some(newfd); }
        let desc_id = self.entries.get(&oldfd)?.description_id;

        // Close newfd if open
        self.entries.remove(&newfd);

        self.entries.insert(newfd, FdEntry {
            fd: newfd, description_id: desc_id,
            flags: FdFlags::default(), opened_ns: ts, last_io_ns: 0,
        });
        self.total_opens += 1;
        Some(newfd)
    }

    pub fn dup3(&mut self, oldfd: i32, newfd: i32, cloexec: bool, ts: u64) -> Option<i32> {
        if oldfd == newfd { return None; } // dup3 fails if equal
        let desc_id = self.entries.get(&oldfd)?.description_id;

        self.entries.remove(&newfd);

        let mut flags = FdFlags::default();
        flags.cloexec = cloexec;
        self.entries.insert(newfd, FdEntry {
            fd: newfd, description_id: desc_id,
            flags, opened_ns: ts, last_io_ns: 0,
        });
        self.total_opens += 1;
        Some(newfd)
    }

    #[inline]
    pub fn set_cloexec(&mut self, fd: i32, cloexec: bool) -> bool {
        if let Some(entry) = self.entries.get_mut(&fd) {
            entry.flags.cloexec = cloexec;
            true
        } else { false }
    }

    #[inline]
    pub fn close_cloexec(&mut self) -> Vec<i32> {
        let to_close: Vec<i32> = self.entries.iter()
            .filter(|(_, e)| e.flags.cloexec)
            .map(|(&fd, _)| fd)
            .collect();
        for fd in &to_close { self.entries.remove(fd); }
        to_close
    }

    /// Fork: copy entire FD table
    #[inline]
    pub fn fork_table(&self, child_pid: u64) -> ProcessFdTable {
        let mut child = ProcessFdTable::new(child_pid, self.fd_limit);
        child.entries = self.entries.clone();
        child.next_fd = self.next_fd;
        child
    }

    #[inline(always)]
    pub fn fd_count(&self) -> usize { self.entries.len() }
    #[inline(always)]
    pub fn get(&self, fd: i32) -> Option<&FdEntry> { self.entries.get(&fd) }
}

/// Leak detection heuristic
#[derive(Debug, Clone)]
pub struct FdLeakHeuristic {
    pub process_id: u64,
    pub suspected_leaks: Vec<i32>,
    pub growth_rate: f64,
    pub last_check_count: usize,
    pub check_interval_ns: u64,
}

/// Apps FD manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppsFdMgrStats {
    pub total_processes: usize,
    pub total_fds: usize,
    pub total_opens: u64,
    pub total_closes: u64,
    pub peak_fds: usize,
    pub suspected_leaks: usize,
}

/// Apps File Descriptor Manager
pub struct AppsFdMgr {
    tables: BTreeMap<u64, ProcessFdTable>,
    descriptions: BTreeMap<u64, FileDescription>,
    leak_checks: BTreeMap<u64, FdLeakHeuristic>,
    next_desc_id: u64,
    stats: AppsFdMgrStats,
}

impl AppsFdMgr {
    pub fn new() -> Self {
        Self {
            tables: BTreeMap::new(),
            descriptions: BTreeMap::new(),
            leak_checks: BTreeMap::new(),
            next_desc_id: 1,
            stats: AppsFdMgrStats::default(),
        }
    }

    #[inline(always)]
    pub fn register_process(&mut self, pid: u64, fd_limit: u32) {
        self.tables.entry(pid).or_insert_with(|| ProcessFdTable::new(pid, fd_limit));
    }

    #[inline]
    pub fn open(&mut self, pid: u64, ftype: FdType, inode: u64, mode: u32, flags: FdFlags, ts: u64) -> Option<i32> {
        let desc_id = self.next_desc_id;
        self.next_desc_id += 1;
        self.descriptions.insert(desc_id, FileDescription::new(desc_id, ftype, inode, mode));

        self.tables.get_mut(&pid)?.alloc_fd(desc_id, flags, ts)
    }

    pub fn close(&mut self, pid: u64, fd: i32) -> bool {
        if let Some(table) = self.tables.get_mut(&pid) {
            if let Some(desc_id) = table.close_fd(fd) {
                if let Some(desc) = self.descriptions.get_mut(&desc_id) {
                    if desc.release() {
                        self.descriptions.remove(&desc_id);
                    }
                }
                return true;
            }
        }
        false
    }

    #[inline]
    pub fn dup(&mut self, pid: u64, oldfd: i32, ts: u64) -> Option<i32> {
        let desc_id = self.tables.get(&pid)?.get(oldfd)?.description_id;
        if let Some(desc) = self.descriptions.get_mut(&desc_id) { desc.add_ref(); }
        self.tables.get_mut(&pid)?.dup(oldfd, ts)
    }

    pub fn fork_process(&mut self, parent_pid: u64, child_pid: u64) {
        if let Some(parent) = self.tables.get(&parent_pid) {
            let child_table = parent.fork_table(child_pid);
            // Increment ref counts for all shared descriptions
            for entry in child_table.entries.values() {
                if let Some(desc) = self.descriptions.get_mut(&entry.description_id) {
                    desc.add_ref();
                }
            }
            self.tables.insert(child_pid, child_table);
        }
    }

    #[inline]
    pub fn remove_process(&mut self, pid: u64) {
        if let Some(table) = self.tables.remove(&pid) {
            for entry in table.entries.values() {
                if let Some(desc) = self.descriptions.get_mut(&entry.description_id) {
                    if desc.release() { self.descriptions.remove(&entry.description_id); }
                }
            }
        }
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_processes = self.tables.len();
        self.stats.total_fds = self.tables.values().map(|t| t.fd_count()).sum();
        self.stats.total_opens = self.tables.values().map(|t| t.total_opens).sum();
        self.stats.total_closes = self.tables.values().map(|t| t.total_closes).sum();
        self.stats.peak_fds = self.tables.values().map(|t| t.peak_fd_count).max().unwrap_or(0);
        self.stats.suspected_leaks = self.leak_checks.values().map(|l| l.suspected_leaks.len()).sum();
    }

    #[inline(always)]
    pub fn table(&self, pid: u64) -> Option<&ProcessFdTable> { self.tables.get(&pid) }
    #[inline(always)]
    pub fn stats(&self) -> &AppsFdMgrStats { &self.stats }
}
