// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Link App (hard and symbolic link management)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Link type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkType {
    Hard,
    Symbolic,
}

/// Link result codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkResult {
    Success,
    SourceNotFound,
    DestExists,
    PermissionDenied,
    CrossDevice,
    IsDirectory,
    TooManyLinks,
    NameTooLong,
    NoSpace,
    IoError,
}

/// Unlink result codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnlinkResult {
    Success,
    NotFound,
    PermissionDenied,
    IsDirectory,
    Busy,
    IoError,
}

/// A link operation record
#[derive(Debug, Clone)]
pub struct LinkRecord {
    pub src_hash: u64,
    pub dst_hash: u64,
    pub link_type: LinkType,
    pub pid: u64,
    pub result: LinkResult,
    pub tick: u64,
}

/// An unlink operation record
#[derive(Debug, Clone)]
pub struct UnlinkRecord {
    pub path_hash: u64,
    pub pid: u64,
    pub result: UnlinkResult,
    pub was_last_link: bool,
    pub tick: u64,
}

/// Statistics for link app
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct LinkAppStats {
    pub hard_links_created: u64,
    pub symlinks_created: u64,
    pub unlinks: u64,
    pub link_failures: u64,
    pub unlink_failures: u64,
    pub readlink_calls: u64,
    pub last_link_removals: u64,
}

/// Main link app manager
#[derive(Debug)]
pub struct AppLink {
    link_history: VecDeque<LinkRecord>,
    unlink_history: VecDeque<UnlinkRecord>,
    max_history: usize,
    stats: LinkAppStats,
}

impl AppLink {
    pub fn new(max_history: usize) -> Self {
        Self {
            link_history: VecDeque::new(),
            unlink_history: VecDeque::new(),
            max_history,
            stats: LinkAppStats {
                hard_links_created: 0, symlinks_created: 0,
                unlinks: 0, link_failures: 0,
                unlink_failures: 0, readlink_calls: 0,
                last_link_removals: 0,
            },
        }
    }

    fn hash_path(path: &str) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    pub fn link(&mut self, src: &str, dst: &str, link_type: LinkType, pid: u64, tick: u64) -> LinkResult {
        match link_type {
            LinkType::Hard => self.stats.hard_links_created += 1,
            LinkType::Symbolic => self.stats.symlinks_created += 1,
        }
        let record = LinkRecord {
            src_hash: Self::hash_path(src),
            dst_hash: Self::hash_path(dst),
            link_type, pid, result: LinkResult::Success, tick,
        };
        if self.link_history.len() >= self.max_history {
            self.link_history.remove(0);
        }
        self.link_history.push_back(record);
        LinkResult::Success
    }

    pub fn unlink(&mut self, path: &str, pid: u64, was_last: bool, tick: u64) -> UnlinkResult {
        self.stats.unlinks += 1;
        if was_last { self.stats.last_link_removals += 1; }
        let record = UnlinkRecord {
            path_hash: Self::hash_path(path),
            pid, result: UnlinkResult::Success, was_last_link: was_last, tick,
        };
        if self.unlink_history.len() >= self.max_history {
            self.unlink_history.remove(0).unwrap();
        }
        self.unlink_history.push_back(record);
        UnlinkResult::Success
    }

    #[inline(always)]
    pub fn readlink(&mut self) {
        self.stats.readlink_calls += 1;
    }

    #[inline(always)]
    pub fn stats(&self) -> &LinkAppStats {
        &self.stats
    }
}

// ============================================================================
// Merged from link_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkV2Type {
    HardLink,
    HardLinkAt,
    CrossMount,
    EmptyPath,
}

/// Link v2 result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkV2Result {
    Success,
    CrossDevice,
    TooManyLinks,
    PermissionDenied,
    TargetExists,
    NotFound,
    IsDirectory,
    ReadOnlyFs,
    Error,
}

/// Link v2 flag
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkV2Flag {
    SymlinkFollow,
    EmptyPath,
}

/// Link operation record
#[derive(Debug, Clone)]
pub struct LinkV2Record {
    pub old_path_hash: u64,
    pub new_path_hash: u64,
    pub link_type: LinkV2Type,
    pub result: LinkV2Result,
    pub inode: u64,
    pub nlink_before: u32,
    pub nlink_after: u32,
    pub flags: u32,
    pub duration_ns: u64,
}

impl LinkV2Record {
    pub fn new(old_path: &[u8], new_path: &[u8], link_type: LinkV2Type) -> Self {
        let hash = |path: &[u8]| -> u64 {
            let mut h: u64 = 0xcbf29ce484222325;
            for b in path { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
            h
        };
        Self {
            old_path_hash: hash(old_path),
            new_path_hash: hash(new_path),
            link_type,
            result: LinkV2Result::Success,
            inode: 0,
            nlink_before: 0,
            nlink_after: 0,
            flags: 0,
            duration_ns: 0,
        }
    }

    #[inline(always)]
    pub fn link_count_changed(&self) -> bool {
        self.nlink_after != self.nlink_before
    }
}

/// Inode link tracker
#[derive(Debug, Clone)]
pub struct InodeLinkTracker {
    pub inode: u64,
    pub current_nlink: u32,
    pub max_nlink: u32,
    pub link_ops: u64,
    pub unlink_ops: u64,
}

impl InodeLinkTracker {
    pub fn new(inode: u64) -> Self {
        Self { inode, current_nlink: 1, max_nlink: 1, link_ops: 0, unlink_ops: 0 }
    }

    #[inline]
    pub fn link(&mut self) {
        self.current_nlink += 1;
        if self.current_nlink > self.max_nlink {
            self.max_nlink = self.current_nlink;
        }
        self.link_ops += 1;
    }

    #[inline(always)]
    pub fn unlink(&mut self) {
        self.current_nlink = self.current_nlink.saturating_sub(1);
        self.unlink_ops += 1;
    }
}

/// Link v2 app stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct LinkV2AppStats {
    pub total_ops: u64,
    pub hard_links_created: u64,
    pub cross_device_failures: u64,
    pub too_many_links: u64,
    pub failures: u64,
}

/// Main app link v2
#[derive(Debug)]
pub struct AppLinkV2 {
    pub inode_trackers: BTreeMap<u64, InodeLinkTracker>,
    pub stats: LinkV2AppStats,
}

impl AppLinkV2 {
    pub fn new() -> Self {
        Self {
            inode_trackers: BTreeMap::new(),
            stats: LinkV2AppStats {
                total_ops: 0,
                hard_links_created: 0,
                cross_device_failures: 0,
                too_many_links: 0,
                failures: 0,
            },
        }
    }

    pub fn record(&mut self, record: &LinkV2Record) {
        self.stats.total_ops += 1;
        match record.result {
            LinkV2Result::Success => {
                self.stats.hard_links_created += 1;
                let tracker = self.inode_trackers.entry(record.inode)
                    .or_insert_with(|| InodeLinkTracker::new(record.inode));
                tracker.link();
            }
            LinkV2Result::CrossDevice => { self.stats.cross_device_failures += 1; self.stats.failures += 1; }
            LinkV2Result::TooManyLinks => { self.stats.too_many_links += 1; self.stats.failures += 1; }
            _ => self.stats.failures += 1,
        }
    }

    #[inline(always)]
    pub fn success_rate(&self) -> f64 {
        if self.stats.total_ops == 0 { 0.0 }
        else { self.stats.hard_links_created as f64 / self.stats.total_ops as f64 }
    }
}
