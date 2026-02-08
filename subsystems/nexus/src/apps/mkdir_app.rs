// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Mkdir App (directory creation tracking)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Mkdir result codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MkdirResult {
    Success,
    AlreadyExists,
    PermissionDenied,
    NoParent,
    NameTooLong,
    NoSpace,
    ReadOnlyFs,
    IoError,
}

/// A directory creation record
#[derive(Debug, Clone)]
pub struct MkdirRecord {
    pub path_hash: u64,
    pub mode: u32,
    pub pid: u64,
    pub uid: u32,
    pub gid: u32,
    pub result: MkdirResult,
    pub recursive: bool,
    pub dirs_created: u32,
    pub tick: u64,
}

/// Statistics for mkdir app
#[derive(Debug, Clone)]
pub struct MkdirAppStats {
    pub total_calls: u64,
    pub successful: u64,
    pub failed: u64,
    pub recursive_calls: u64,
    pub dirs_created: u64,
    pub already_exists: u64,
    pub permission_denied: u64,
}

/// Main mkdir app manager
#[derive(Debug)]
pub struct AppMkdir {
    history: Vec<MkdirRecord>,
    max_history: usize,
    stats: MkdirAppStats,
}

impl AppMkdir {
    pub fn new(max_history: usize) -> Self {
        Self {
            history: Vec::new(),
            max_history,
            stats: MkdirAppStats {
                total_calls: 0, successful: 0, failed: 0,
                recursive_calls: 0, dirs_created: 0,
                already_exists: 0, permission_denied: 0,
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

    pub fn mkdir(&mut self, path: &str, mode: u32, pid: u64, uid: u32, gid: u32, recursive: bool, tick: u64) -> MkdirResult {
        self.stats.total_calls += 1;
        if recursive {
            self.stats.recursive_calls += 1;
        }
        let result = MkdirResult::Success;
        let dirs_created = if recursive { 3 } else { 1 };
        self.stats.successful += 1;
        self.stats.dirs_created += dirs_created as u64;

        let record = MkdirRecord {
            path_hash: Self::hash_path(path),
            mode, pid, uid, gid,
            result,
            recursive,
            dirs_created,
            tick,
        };
        if self.history.len() >= self.max_history {
            self.history.remove(0);
        }
        self.history.push(record);
        result
    }

    pub fn history_count(&self) -> usize {
        self.history.len()
    }

    pub fn stats(&self) -> &MkdirAppStats {
        &self.stats
    }
}

// ============================================================================
// Merged from mkdir_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MkdirV2Mode {
    Single,
    Recursive,
    Atomic,
    TempRenamed,
}

/// Mkdir v2 result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MkdirV2Result {
    Success,
    AlreadyExists,
    PermissionDenied,
    NoSpace,
    ParentNotFound,
    NameTooLong,
    ReadOnlyFs,
    Error,
}

/// Directory creation record
#[derive(Debug, Clone)]
pub struct MkdirV2Record {
    pub path_hash: u64,
    pub mode: MkdirV2Mode,
    pub permissions: u16,
    pub result: MkdirV2Result,
    pub parent_inode: u64,
    pub new_inode: u64,
    pub depth: u32,
    pub dirs_created: u32,
    pub duration_ns: u64,
}

impl MkdirV2Record {
    pub fn new(path: &[u8], mode: MkdirV2Mode, permissions: u16) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        let depth = path.iter().filter(|&&b| b == b'/').count() as u32;
        Self {
            path_hash: h,
            mode,
            permissions,
            result: MkdirV2Result::Success,
            parent_inode: 0,
            new_inode: 0,
            depth,
            dirs_created: 1,
            duration_ns: 0,
        }
    }

    pub fn is_deep(&self) -> bool {
        self.depth > 10
    }
}

/// Mkdir v2 app stats
#[derive(Debug, Clone)]
pub struct MkdirV2AppStats {
    pub total_ops: u64,
    pub total_dirs_created: u64,
    pub recursive_ops: u64,
    pub failures: u64,
    pub max_depth: u32,
}

/// Main app mkdir v2
#[derive(Debug)]
pub struct AppMkdirV2 {
    pub stats: MkdirV2AppStats,
    pub recent_paths: BTreeMap<u64, u64>,
}

impl AppMkdirV2 {
    pub fn new() -> Self {
        Self {
            stats: MkdirV2AppStats {
                total_ops: 0,
                total_dirs_created: 0,
                recursive_ops: 0,
                failures: 0,
                max_depth: 0,
            },
            recent_paths: BTreeMap::new(),
        }
    }

    pub fn record(&mut self, record: &MkdirV2Record) {
        self.stats.total_ops += 1;
        self.stats.total_dirs_created += record.dirs_created as u64;
        if record.mode == MkdirV2Mode::Recursive {
            self.stats.recursive_ops += 1;
        }
        if record.result != MkdirV2Result::Success {
            self.stats.failures += 1;
        }
        if record.depth > self.stats.max_depth {
            self.stats.max_depth = record.depth;
        }
        *self.recent_paths.entry(record.path_hash).or_insert(0) += 1;
    }

    pub fn success_rate(&self) -> f64 {
        if self.stats.total_ops == 0 { 0.0 }
        else { (self.stats.total_ops - self.stats.failures) as f64 / self.stats.total_ops as f64 }
    }
}

// ============================================================================
// Merged from mkdir_v3_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMkdirMode {
    Normal,
    Recursive,
    Temporary,
    WithParents,
    Strict,
}

/// Created directory record
#[derive(Debug, Clone)]
pub struct AppDirRecord {
    pub path: String,
    pub permissions: u32,
    pub mode: AppMkdirMode,
    pub parent_inode: u64,
    pub inode: u64,
    pub creation_time: u64,
}

/// Stats for mkdir operations
#[derive(Debug, Clone)]
pub struct AppMkdirV3Stats {
    pub total_mkdirs: u64,
    pub recursive_mkdirs: u64,
    pub mkdir_errors: u64,
    pub directories_created: u64,
    pub permissions_denied: u64,
}

/// Manager for mkdir app operations
pub struct AppMkdirV3Manager {
    directories: BTreeMap<u64, AppDirRecord>,
    path_index: BTreeMap<u64, u64>,
    next_inode: u64,
    stats: AppMkdirV3Stats,
}

impl AppMkdirV3Manager {
    pub fn new() -> Self {
        Self {
            directories: BTreeMap::new(),
            path_index: BTreeMap::new(),
            next_inode: 1000,
            stats: AppMkdirV3Stats {
                total_mkdirs: 0,
                recursive_mkdirs: 0,
                mkdir_errors: 0,
                directories_created: 0,
                permissions_denied: 0,
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

    pub fn mkdir(&mut self, path: &str, permissions: u32, mode: AppMkdirMode) -> Option<u64> {
        self.stats.total_mkdirs += 1;
        let hash = Self::hash_path(path);
        if self.path_index.contains_key(&hash) {
            self.stats.mkdir_errors += 1;
            return None;
        }
        if matches!(mode, AppMkdirMode::Recursive | AppMkdirMode::WithParents) {
            self.stats.recursive_mkdirs += 1;
        }
        let inode = self.next_inode;
        self.next_inode += 1;
        let record = AppDirRecord {
            path: String::from(path),
            permissions,
            mode,
            parent_inode: inode.wrapping_sub(1),
            inode,
            creation_time: inode.wrapping_mul(43),
        };
        self.directories.insert(inode, record);
        self.path_index.insert(hash, inode);
        self.stats.directories_created += 1;
        Some(inode)
    }

    pub fn rmdir(&mut self, path: &str) -> bool {
        let hash = Self::hash_path(path);
        if let Some(inode) = self.path_index.remove(&hash) {
            self.directories.remove(&inode);
            true
        } else {
            false
        }
    }

    pub fn exists(&self, path: &str) -> bool {
        let hash = Self::hash_path(path);
        self.path_index.contains_key(&hash)
    }

    pub fn stats(&self) -> &AppMkdirV3Stats {
        &self.stats
    }
}
