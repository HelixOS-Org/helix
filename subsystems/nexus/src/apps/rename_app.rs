// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Rename App (file rename/move tracking)

extern crate alloc;
use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Rename flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameFlag {
    NoReplace,
    Exchange,
    Whiteout,
}

/// Rename result codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameResult {
    Success,
    SourceNotFound,
    DestExists,
    PermissionDenied,
    CrossDevice,
    IsDirectory,
    NotDirectory,
    NotEmpty,
    IoError,
}

/// A rename operation record
#[derive(Debug, Clone)]
pub struct RenameRecord {
    pub src_hash: u64,
    pub dst_hash: u64,
    pub flags: u32,
    pub pid: u64,
    pub result: RenameResult,
    pub is_directory: bool,
    pub tick: u64,
}

/// Statistics for rename app
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct RenameAppStats {
    pub total_calls: u64,
    pub successful: u64,
    pub failed: u64,
    pub cross_dir_moves: u64,
    pub exchanges: u64,
    pub overwrites: u64,
    pub dir_renames: u64,
}

/// Main rename app manager
#[derive(Debug)]
pub struct AppRename {
    history: VecDeque<RenameRecord>,
    max_history: usize,
    stats: RenameAppStats,
}

impl AppRename {
    pub fn new(max_history: usize) -> Self {
        Self {
            history: VecDeque::new(),
            max_history,
            stats: RenameAppStats {
                total_calls: 0, successful: 0, failed: 0,
                cross_dir_moves: 0, exchanges: 0,
                overwrites: 0, dir_renames: 0,
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

    pub fn rename(&mut self, src: &str, dst: &str, flags: u32, pid: u64, is_dir: bool, tick: u64) -> RenameResult {
        self.stats.total_calls += 1;
        let result = RenameResult::Success;
        self.stats.successful += 1;
        if is_dir {
            self.stats.dir_renames += 1;
        }
        if flags & 2 != 0 {
            self.stats.exchanges += 1;
        }

        let record = RenameRecord {
            src_hash: Self::hash_path(src),
            dst_hash: Self::hash_path(dst),
            flags, pid, result, is_directory: is_dir, tick,
        };
        if self.history.len() >= self.max_history {
            self.history.pop_front();
        }
        self.history.push_back(record);
        result
    }

    #[inline(always)]
    pub fn stats(&self) -> &RenameAppStats {
        &self.stats
    }
}

// ============================================================================
// Merged from rename_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameV2Flag {
    None,
    NoReplace,
    Exchange,
    Whiteout,
}

/// Rename v2 result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameV2Result {
    Success,
    CrossDevice,
    NotFound,
    PermissionDenied,
    IsDirectory,
    NotDirectory,
    NotEmpty,
    Busy,
    TargetExists,
    ReadOnlyFs,
    Error,
}

/// Rename operation record
#[derive(Debug, Clone)]
pub struct RenameV2Record {
    pub old_path_hash: u64,
    pub new_path_hash: u64,
    pub flag: RenameV2Flag,
    pub result: RenameV2Result,
    pub old_inode: u64,
    pub new_inode: u64,
    pub overwritten_inode: u64,
    pub same_dir: bool,
    pub duration_ns: u64,
}

impl RenameV2Record {
    pub fn new(old_path: &[u8], new_path: &[u8], flag: RenameV2Flag) -> Self {
        let hash = |path: &[u8]| -> u64 {
            let mut h: u64 = 0xcbf29ce484222325;
            for b in path { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
            h
        };
        Self {
            old_path_hash: hash(old_path),
            new_path_hash: hash(new_path),
            flag,
            result: RenameV2Result::Success,
            old_inode: 0,
            new_inode: 0,
            overwritten_inode: 0,
            same_dir: false,
            duration_ns: 0,
        }
    }

    #[inline(always)]
    pub fn was_exchange(&self) -> bool {
        self.flag == RenameV2Flag::Exchange
    }

    #[inline(always)]
    pub fn overwrote_target(&self) -> bool {
        self.overwritten_inode != 0
    }
}

/// Rename v2 app stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct RenameV2AppStats {
    pub total_ops: u64,
    pub same_dir_renames: u64,
    pub cross_dir_renames: u64,
    pub exchanges: u64,
    pub overwrites: u64,
    pub failures: u64,
}

/// Main app rename v2
#[derive(Debug)]
pub struct AppRenameV2 {
    pub stats: RenameV2AppStats,
}

impl AppRenameV2 {
    pub fn new() -> Self {
        Self {
            stats: RenameV2AppStats {
                total_ops: 0,
                same_dir_renames: 0,
                cross_dir_renames: 0,
                exchanges: 0,
                overwrites: 0,
                failures: 0,
            },
        }
    }

    pub fn record(&mut self, record: &RenameV2Record) {
        self.stats.total_ops += 1;
        match record.result {
            RenameV2Result::Success => {
                if record.same_dir { self.stats.same_dir_renames += 1; }
                else { self.stats.cross_dir_renames += 1; }
                if record.was_exchange() { self.stats.exchanges += 1; }
                if record.overwrote_target() { self.stats.overwrites += 1; }
            }
            _ => self.stats.failures += 1,
        }
    }

    #[inline(always)]
    pub fn success_rate(&self) -> f64 {
        if self.stats.total_ops == 0 { 0.0 }
        else { (self.stats.total_ops - self.stats.failures) as f64 / self.stats.total_ops as f64 }
    }
}

// ============================================================================
// Merged from rename_v3_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppRenameFlags {
    None,
    NoReplace,
    Exchange,
    Whiteout,
}

/// Rename result status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppRenameResult {
    Success,
    SourceNotFound,
    DestExists,
    CrossDevice,
    PermissionDenied,
    InvalidArgument,
}

/// Rename operation record
#[derive(Debug, Clone)]
pub struct AppRenameRecord {
    pub old_path: String,
    pub new_path: String,
    pub flags: AppRenameFlags,
    pub inode: u64,
    pub result: AppRenameResult,
    pub timestamp: u64,
}

/// Stats for rename operations
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AppRenameV3Stats {
    pub total_renames: u64,
    pub successful_renames: u64,
    pub failed_renames: u64,
    pub cross_dir_renames: u64,
    pub exchange_renames: u64,
}

/// Manager for rename app operations
pub struct AppRenameV3Manager {
    history: VecDeque<AppRenameRecord>,
    name_map: LinearMap<u64, 64>,
    stats: AppRenameV3Stats,
}

impl AppRenameV3Manager {
    pub fn new() -> Self {
        Self {
            history: VecDeque::new(),
            name_map: LinearMap::new(),
            stats: AppRenameV3Stats {
                total_renames: 0,
                successful_renames: 0,
                failed_renames: 0,
                cross_dir_renames: 0,
                exchange_renames: 0,
            },
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

    pub fn rename(&mut self, old_path: &str, new_path: &str, inode: u64, flags: AppRenameFlags) -> AppRenameResult {
        self.stats.total_renames += 1;
        let old_hash = Self::hash_path(old_path);
        let new_hash = Self::hash_path(new_path);
        if flags == AppRenameFlags::NoReplace && self.name_map.contains_key(new_hash) {
            self.stats.failed_renames += 1;
            let record = AppRenameRecord {
                old_path: String::from(old_path),
                new_path: String::from(new_path),
                flags,
                inode,
                result: AppRenameResult::DestExists,
                timestamp: self.stats.total_renames.wrapping_mul(53),
            };
            self.history.push_back(record);
            return AppRenameResult::DestExists;
        }
        if flags == AppRenameFlags::Exchange {
            self.stats.exchange_renames += 1;
        }
        self.name_map.remove(old_hash);
        self.name_map.insert(new_hash, inode);
        self.stats.successful_renames += 1;
        let record = AppRenameRecord {
            old_path: String::from(old_path),
            new_path: String::from(new_path),
            flags,
            inode,
            result: AppRenameResult::Success,
            timestamp: self.stats.total_renames.wrapping_mul(53),
        };
        self.history.push_back(record);
        AppRenameResult::Success
    }

    #[inline(always)]
    pub fn history_count(&self) -> usize {
        self.history.len()
    }

    #[inline(always)]
    pub fn stats(&self) -> &AppRenameV3Stats {
        &self.stats
    }
}
