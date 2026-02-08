// SPDX-License-Identifier: GPL-2.0
//! Apps open_app — file open/openat application layer.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Open flag
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenFlag {
    ReadOnly,
    WriteOnly,
    ReadWrite,
    Create,
    Excl,
    Trunc,
    Append,
    NonBlock,
    Sync,
    Direct,
    NoFollow,
    NoAtime,
    CloseOnExec,
    Path,
    TmpFile,
}

/// Open result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenResult {
    Success,
    NotFound,
    PermissionDenied,
    TooManyOpen,
    IsDirectory,
    NotDirectory,
    Loop,
    NameTooLong,
}

/// FD tracker entry
#[derive(Debug)]
pub struct FdEntry {
    pub fd: u64,
    pub inode_hash: u64,
    pub flags: u32,
    pub mode: u32,
    pub opened_at: u64,
    pub read_count: u64,
    pub write_count: u64,
    pub read_bytes: u64,
    pub write_bytes: u64,
}

impl FdEntry {
    pub fn new(fd: u64, inode: u64, flags: u32, mode: u32, now: u64) -> Self {
        Self { fd, inode_hash: inode, flags, mode, opened_at: now, read_count: 0, write_count: 0, read_bytes: 0, write_bytes: 0 }
    }
}

/// Process open tracker
#[derive(Debug)]
pub struct ProcessFdTracker {
    pub pid: u64,
    pub fds: BTreeMap<u64, FdEntry>,
    pub total_opens: u64,
    pub total_closes: u64,
    pub total_failures: u64,
}

impl ProcessFdTracker {
    pub fn new(pid: u64) -> Self {
        Self { pid, fds: BTreeMap::new(), total_opens: 0, total_closes: 0, total_failures: 0 }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct OpenAppStats {
    pub tracked_procs: u32,
    pub total_fds: u32,
    pub total_opens: u64,
    pub total_closes: u64,
    pub total_failures: u64,
}

/// Main app open
pub struct AppOpen {
    procs: BTreeMap<u64, ProcessFdTracker>,
}

impl AppOpen {
    pub fn new() -> Self { Self { procs: BTreeMap::new() } }
    pub fn track(&mut self, pid: u64) { self.procs.insert(pid, ProcessFdTracker::new(pid)); }

    pub fn open_file(&mut self, pid: u64, fd: u64, inode: u64, flags: u32, mode: u32, now: u64) {
        if let Some(p) = self.procs.get_mut(&pid) {
            p.total_opens += 1;
            p.fds.insert(fd, FdEntry::new(fd, inode, flags, mode, now));
        }
    }

    pub fn close_file(&mut self, pid: u64, fd: u64) {
        if let Some(p) = self.procs.get_mut(&pid) { p.fds.remove(&fd); p.total_closes += 1; }
    }

    pub fn record_failure(&mut self, pid: u64) {
        if let Some(p) = self.procs.get_mut(&pid) { p.total_failures += 1; }
    }

    pub fn untrack(&mut self, pid: u64) { self.procs.remove(&pid); }

    pub fn stats(&self) -> OpenAppStats {
        let fds: u32 = self.procs.values().map(|p| p.fds.len() as u32).sum();
        let opens: u64 = self.procs.values().map(|p| p.total_opens).sum();
        let closes: u64 = self.procs.values().map(|p| p.total_closes).sum();
        let fails: u64 = self.procs.values().map(|p| p.total_failures).sum();
        OpenAppStats { tracked_procs: self.procs.len() as u32, total_fds: fds, total_opens: opens, total_closes: closes, total_failures: fails }
    }
}

// ============================================================================
// Merged from open_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppOpenMode {
    ReadOnly,
    WriteOnly,
    ReadWrite,
    Append,
    Create,
    Truncate,
    Exclusive,
    NonBlock,
    Directory,
    NoFollow,
}

/// File descriptor tracking entry
#[derive(Debug, Clone)]
pub struct AppFileDescriptor {
    pub fd: u64,
    pub path: String,
    pub mode: AppOpenMode,
    pub offset: u64,
    pub is_directory: bool,
    pub reference_count: u32,
    pub creation_timestamp: u64,
}

/// Stats for open operations
#[derive(Debug, Clone)]
pub struct AppOpenStats {
    pub total_opens: u64,
    pub active_fds: u64,
    pub peak_fds: u64,
    pub failed_opens: u64,
    pub avg_open_latency_us: u64,
}

/// Manager for file open application operations
pub struct AppOpenV2Manager {
    descriptors: BTreeMap<u64, AppFileDescriptor>,
    next_fd: AtomicU64,
    stats: AppOpenStats,
    path_cache: BTreeMap<u64, String>,
}

impl AppOpenV2Manager {
    pub fn new() -> Self {
        Self {
            descriptors: BTreeMap::new(),
            next_fd: AtomicU64::new(3),
            stats: AppOpenStats {
                total_opens: 0,
                active_fds: 0,
                peak_fds: 0,
                failed_opens: 0,
                avg_open_latency_us: 0,
            },
            path_cache: BTreeMap::new(),
        }
    }

    fn hash_path(path: &str) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    pub fn open_file(&mut self, path: &str, mode: AppOpenMode) -> Option<u64> {
        let fd = self.next_fd.fetch_add(1, Ordering::SeqCst);
        let is_dir = mode == AppOpenMode::Directory;
        let desc = AppFileDescriptor {
            fd,
            path: String::from(path),
            mode,
            offset: 0,
            is_directory: is_dir,
            reference_count: 1,
            creation_timestamp: fd.wrapping_mul(17),
        };
        self.descriptors.insert(fd, desc);
        let hash = Self::hash_path(path);
        self.path_cache.insert(hash, String::from(path));
        self.stats.total_opens += 1;
        self.stats.active_fds += 1;
        if self.stats.active_fds > self.stats.peak_fds {
            self.stats.peak_fds = self.stats.active_fds;
        }
        Some(fd)
    }

    pub fn close_fd(&mut self, fd: u64) -> bool {
        if let Some(desc) = self.descriptors.remove(&fd) {
            let hash = Self::hash_path(&desc.path);
            self.path_cache.remove(&hash);
            self.stats.active_fds = self.stats.active_fds.saturating_sub(1);
            true
        } else {
            false
        }
    }

    pub fn duplicate_fd(&mut self, fd: u64) -> Option<u64> {
        if let Some(desc) = self.descriptors.get(&fd).cloned() {
            let new_fd = self.next_fd.fetch_add(1, Ordering::SeqCst);
            let mut new_desc = desc;
            new_desc.fd = new_fd;
            new_desc.reference_count = 1;
            self.descriptors.insert(new_fd, new_desc);
            self.stats.active_fds += 1;
            if self.stats.active_fds > self.stats.peak_fds {
                self.stats.peak_fds = self.stats.active_fds;
            }
            Some(new_fd)
        } else {
            None
        }
    }

    pub fn get_descriptor(&self, fd: u64) -> Option<&AppFileDescriptor> {
        self.descriptors.get(&fd)
    }

    pub fn stats(&self) -> &AppOpenStats {
        &self.stats
    }
}
