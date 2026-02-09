// SPDX-License-Identifier: GPL-2.0
//! App unlink — file deletion with deferred removal and orphan inode tracking

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Unlink result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnlinkResult {
    Success,
    NotFound,
    PermissionDenied,
    IsDirectory,
    Busy,
    ReadOnlyFs,
    StickyBit,
    Error,
}

/// Unlink mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnlinkMode {
    Unlink,
    UnlinkAt,
    Remove,
}

/// Unlink record
#[derive(Debug, Clone)]
pub struct UnlinkRecord {
    pub path_hash: u64,
    pub mode: UnlinkMode,
    pub result: UnlinkResult,
    pub inode: u64,
    pub nlink_after: u32,
    pub size_bytes: u64,
    pub deferred: bool,
    pub orphan: bool,
    pub duration_ns: u64,
}

impl UnlinkRecord {
    pub fn new(path: &[u8], mode: UnlinkMode) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self {
            path_hash: h,
            mode,
            result: UnlinkResult::Success,
            inode: 0,
            nlink_after: 0,
            size_bytes: 0,
            deferred: false,
            orphan: false,
            duration_ns: 0,
        }
    }

    #[inline(always)]
    pub fn is_final_unlink(&self) -> bool {
        self.nlink_after == 0
    }
}

/// Orphan inode tracker
#[derive(Debug, Clone)]
pub struct OrphanInodeTracker {
    pub inode: u64,
    pub open_fds: u32,
    pub size_bytes: u64,
    pub unlinked_ns: u64,
}

impl OrphanInodeTracker {
    pub fn new(inode: u64, size_bytes: u64, ts_ns: u64) -> Self {
        Self { inode, open_fds: 1, size_bytes, unlinked_ns: ts_ns }
    }

    #[inline(always)]
    pub fn close_fd(&mut self) -> bool {
        self.open_fds = self.open_fds.saturating_sub(1);
        self.open_fds == 0
    }
}

/// Unlink app stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct UnlinkAppStats {
    pub total_ops: u64,
    pub files_removed: u64,
    pub bytes_freed: u64,
    pub orphan_inodes: u64,
    pub deferred_removals: u64,
    pub failures: u64,
}

/// Main app unlink
#[derive(Debug)]
pub struct AppUnlink {
    pub orphans: BTreeMap<u64, OrphanInodeTracker>,
    pub stats: UnlinkAppStats,
}

impl AppUnlink {
    pub fn new() -> Self {
        Self {
            orphans: BTreeMap::new(),
            stats: UnlinkAppStats {
                total_ops: 0,
                files_removed: 0,
                bytes_freed: 0,
                orphan_inodes: 0,
                deferred_removals: 0,
                failures: 0,
            },
        }
    }

    pub fn record(&mut self, record: &UnlinkRecord, ts_ns: u64) {
        self.stats.total_ops += 1;
        match record.result {
            UnlinkResult::Success => {
                self.stats.files_removed += 1;
                if record.is_final_unlink() {
                    self.stats.bytes_freed += record.size_bytes;
                }
                if record.orphan {
                    self.orphans.insert(record.inode,
                        OrphanInodeTracker::new(record.inode, record.size_bytes, ts_ns));
                    self.stats.orphan_inodes += 1;
                }
                if record.deferred {
                    self.stats.deferred_removals += 1;
                }
            }
            _ => self.stats.failures += 1,
        }
    }

    #[inline]
    pub fn close_orphan_fd(&mut self, inode: u64) -> bool {
        if let Some(orphan) = self.orphans.get_mut(&inode) {
            if orphan.close_fd() {
                self.stats.bytes_freed += orphan.size_bytes;
                self.orphans.remove(&inode);
                return true;
            }
        }
        false
    }
}

// ============================================================================
// Merged from unlink_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppUnlinkType {
    File,
    Directory,
    Symlink,
    HardLink,
    Forced,
}

/// Unlink result status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppUnlinkResult {
    Success,
    NotFound,
    PermissionDenied,
    Busy,
    DirectoryNotEmpty,
    IsDirectory,
}

/// Pending deletion entry
#[derive(Debug, Clone)]
pub struct AppUnlinkEntry {
    pub path: String,
    pub inode: u64,
    pub unlink_type: AppUnlinkType,
    pub link_count: u32,
    pub timestamp: u64,
}

/// Stats for unlink operations
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AppUnlinkV2Stats {
    pub total_unlinks: u64,
    pub successful_unlinks: u64,
    pub failed_unlinks: u64,
    pub deferred_unlinks: u64,
    pub orphan_inodes: u64,
}

/// Manager for unlink app operations
pub struct AppUnlinkV2Manager {
    pending_unlinks: BTreeMap<u64, AppUnlinkEntry>,
    orphan_list: Vec<u64>,
    stats: AppUnlinkV2Stats,
    next_id: u64,
}

impl AppUnlinkV2Manager {
    pub fn new() -> Self {
        Self {
            pending_unlinks: BTreeMap::new(),
            orphan_list: Vec::new(),
            stats: AppUnlinkV2Stats {
                total_unlinks: 0,
                successful_unlinks: 0,
                failed_unlinks: 0,
                deferred_unlinks: 0,
                orphan_inodes: 0,
            },
            next_id: 1,
        }
    }

    pub fn unlink(&mut self, path: &str, inode: u64, unlink_type: AppUnlinkType, link_count: u32) -> AppUnlinkResult {
        self.stats.total_unlinks += 1;
        if unlink_type == AppUnlinkType::Directory {
            self.stats.failed_unlinks += 1;
            return AppUnlinkResult::IsDirectory;
        }
        if link_count > 1 {
            self.stats.successful_unlinks += 1;
            return AppUnlinkResult::Success;
        }
        let entry = AppUnlinkEntry {
            path: String::from(path),
            inode,
            unlink_type,
            link_count,
            timestamp: self.next_id.wrapping_mul(47),
        };
        self.pending_unlinks.insert(self.next_id, entry);
        self.next_id += 1;
        self.orphan_list.push(inode);
        self.stats.orphan_inodes += 1;
        self.stats.successful_unlinks += 1;
        AppUnlinkResult::Success
    }

    #[inline]
    pub fn process_orphans(&mut self) -> usize {
        let count = self.orphan_list.len();
        self.orphan_list.clear();
        self.stats.orphan_inodes = 0;
        count
    }

    pub fn defer_unlink(&mut self, path: &str, inode: u64) {
        let entry = AppUnlinkEntry {
            path: String::from(path),
            inode,
            unlink_type: AppUnlinkType::File,
            link_count: 0,
            timestamp: self.next_id.wrapping_mul(47),
        };
        self.pending_unlinks.insert(self.next_id, entry);
        self.next_id += 1;
        self.stats.deferred_unlinks += 1;
    }

    #[inline(always)]
    pub fn stats(&self) -> &AppUnlinkV2Stats {
        &self.stats
    }
}
