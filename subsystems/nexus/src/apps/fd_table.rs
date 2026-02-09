// SPDX-License-Identifier: GPL-2.0
//! Apps fd_table — file descriptor table management per process.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
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
    EventFd,
    TimerFd,
    SignalFd,
    EpollFd,
    MemFd,
    PidFd,
    Inotify,
}

/// File descriptor flags
#[derive(Debug, Clone, Copy)]
pub struct FdFlags(pub u32);

impl FdFlags {
    pub const CLOEXEC: Self = Self(0x01);
    pub const NONBLOCK: Self = Self(0x02);
    pub const APPEND: Self = Self(0x04);
    pub const DIRECT: Self = Self(0x08);
    pub const NOATIME: Self = Self(0x10);
    pub const SYNC: Self = Self(0x20);
    pub const DSYNC: Self = Self(0x40);

    #[inline(always)]
    pub fn contains(&self, flag: Self) -> bool { self.0 & flag.0 != 0 }
    #[inline(always)]
    pub fn set(&mut self, flag: Self) { self.0 |= flag.0; }
    #[inline(always)]
    pub fn clear(&mut self, flag: Self) { self.0 &= !flag.0; }
}

/// File descriptor entry
#[derive(Debug, Clone)]
pub struct FdEntry {
    pub fd: i32,
    pub fd_type: FdType,
    pub flags: FdFlags,
    pub offset: u64,
    pub inode: u64,
    pub path: Option<String>,
    pub ref_count: u32,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub read_ops: u64,
    pub write_ops: u64,
    pub created_at: u64,
    pub last_access: u64,
}

impl FdEntry {
    pub fn new(fd: i32, fd_type: FdType, flags: FdFlags, now: u64) -> Self {
        Self {
            fd, fd_type, flags, offset: 0, inode: 0,
            path: None, ref_count: 1,
            read_bytes: 0, write_bytes: 0,
            read_ops: 0, write_ops: 0,
            created_at: now, last_access: now,
        }
    }

    #[inline]
    pub fn record_read(&mut self, bytes: u64, now: u64) {
        self.read_bytes += bytes;
        self.read_ops += 1;
        self.last_access = now;
    }

    #[inline]
    pub fn record_write(&mut self, bytes: u64, now: u64) {
        self.write_bytes += bytes;
        self.write_ops += 1;
        self.last_access = now;
    }

    #[inline(always)]
    pub fn total_io_bytes(&self) -> u64 { self.read_bytes + self.write_bytes }
    #[inline(always)]
    pub fn total_io_ops(&self) -> u64 { self.read_ops + self.write_ops }

    #[inline(always)]
    pub fn is_close_on_exec(&self) -> bool { self.flags.contains(FdFlags::CLOEXEC) }
    #[inline(always)]
    pub fn idle_time(&self, now: u64) -> u64 { now.saturating_sub(self.last_access) }
}

/// Process fd table
#[derive(Debug)]
pub struct ProcessFdTable {
    pub pid: u32,
    pub fds: BTreeMap<i32, FdEntry>,
    pub max_fds: i32,
    pub next_fd: i32,
    pub close_on_exec_count: u32,
}

impl ProcessFdTable {
    pub fn new(pid: u32, max_fds: i32) -> Self {
        Self { pid, fds: BTreeMap::new(), max_fds, next_fd: 0, close_on_exec_count: 0 }
    }

    #[inline]
    pub fn alloc_fd(&mut self, fd_type: FdType, flags: FdFlags, now: u64) -> Option<i32> {
        if self.fds.len() as i32 >= self.max_fds { return None; }
        let fd = self.find_lowest_free();
        let entry = FdEntry::new(fd, fd_type, flags, now);
        if flags.contains(FdFlags::CLOEXEC) { self.close_on_exec_count += 1; }
        self.fds.insert(fd, entry);
        Some(fd)
    }

    fn find_lowest_free(&self) -> i32 {
        let mut fd = 0i32;
        while self.fds.contains_key(&fd) { fd += 1; }
        fd
    }

    #[inline]
    pub fn close_fd(&mut self, fd: i32) -> Option<FdEntry> {
        if let Some(entry) = self.fds.remove(&fd) {
            if entry.is_close_on_exec() && self.close_on_exec_count > 0 {
                self.close_on_exec_count -= 1;
            }
            Some(entry)
        } else { None }
    }

    #[inline]
    pub fn dup_fd(&mut self, old_fd: i32, new_fd: Option<i32>, now: u64) -> Option<i32> {
        let entry = self.fds.get(&old_fd)?.clone();
        let target_fd = new_fd.unwrap_or_else(|| self.find_lowest_free());
        if target_fd >= self.max_fds { return None; }
        let mut new_entry = entry;
        new_entry.fd = target_fd;
        new_entry.created_at = now;
        self.fds.insert(target_fd, new_entry);
        Some(target_fd)
    }

    #[inline]
    pub fn exec_close(&mut self) -> u32 {
        let to_close: Vec<i32> = self.fds.iter()
            .filter(|(_, e)| e.is_close_on_exec())
            .map(|(&fd, _)| fd)
            .collect();
        let count = to_close.len() as u32;
        for fd in to_close { self.fds.remove(&fd); }
        self.close_on_exec_count = 0;
        count
    }

    #[inline(always)]
    pub fn used_count(&self) -> usize { self.fds.len() }
    #[inline(always)]
    pub fn available(&self) -> i32 { self.max_fds - self.fds.len() as i32 }

    #[inline]
    pub fn fds_by_type(&self, fd_type: FdType) -> Vec<i32> {
        self.fds.iter()
            .filter(|(_, e)| e.fd_type == fd_type)
            .map(|(&fd, _)| fd)
            .collect()
    }

    #[inline]
    pub fn io_heavy_fds(&self, threshold: u64) -> Vec<(i32, u64)> {
        self.fds.iter()
            .filter(|(_, e)| e.total_io_bytes() >= threshold)
            .map(|(&fd, e)| (fd, e.total_io_bytes()))
            .collect()
    }
}

/// Fd table manager stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FdTableStats {
    pub tracked_processes: u32,
    pub total_open_fds: u64,
    pub total_allocs: u64,
    pub total_closes: u64,
    pub total_dups: u64,
}

/// Main fd table manager
pub struct AppFdTable {
    tables: BTreeMap<u32, ProcessFdTable>,
    default_max_fds: i32,
    total_allocs: u64,
    total_closes: u64,
    total_dups: u64,
}

impl AppFdTable {
    pub fn new(default_max: i32) -> Self {
        Self {
            tables: BTreeMap::new(), default_max_fds: default_max,
            total_allocs: 0, total_closes: 0, total_dups: 0,
        }
    }

    #[inline(always)]
    pub fn create_table(&mut self, pid: u32) {
        self.tables.insert(pid, ProcessFdTable::new(pid, self.default_max_fds));
    }

    #[inline(always)]
    pub fn remove_table(&mut self, pid: u32) -> bool {
        self.tables.remove(&pid).is_some()
    }

    #[inline(always)]
    pub fn alloc(&mut self, pid: u32, fd_type: FdType, flags: FdFlags, now: u64) -> Option<i32> {
        self.total_allocs += 1;
        self.tables.get_mut(&pid)?.alloc_fd(fd_type, flags, now)
    }

    #[inline(always)]
    pub fn close(&mut self, pid: u32, fd: i32) -> Option<FdEntry> {
        self.total_closes += 1;
        self.tables.get_mut(&pid)?.close_fd(fd)
    }

    #[inline(always)]
    pub fn dup(&mut self, pid: u32, old_fd: i32, new_fd: Option<i32>, now: u64) -> Option<i32> {
        self.total_dups += 1;
        self.tables.get_mut(&pid)?.dup_fd(old_fd, new_fd, now)
    }

    #[inline]
    pub fn fork_table(&mut self, parent: u32, child: u32) -> bool {
        if let Some(parent_table) = self.tables.get(&parent) {
            let mut child_table = ProcessFdTable::new(child, parent_table.max_fds);
            for (&fd, entry) in &parent_table.fds {
                child_table.fds.insert(fd, entry.clone());
            }
            child_table.close_on_exec_count = parent_table.close_on_exec_count;
            self.tables.insert(child, child_table);
            true
        } else { false }
    }

    #[inline]
    pub fn stats(&self) -> FdTableStats {
        let total_fds: u64 = self.tables.values().map(|t| t.used_count() as u64).sum();
        FdTableStats {
            tracked_processes: self.tables.len() as u32,
            total_open_fds: total_fds,
            total_allocs: self.total_allocs,
            total_closes: self.total_closes,
            total_dups: self.total_dups,
        }
    }
}
