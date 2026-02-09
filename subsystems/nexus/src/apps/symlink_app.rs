// SPDX-License-Identifier: GPL-2.0
//! App symlink — symbolic link creation and resolution with loop detection

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Symlink result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymlinkResult {
    Success,
    PermissionDenied,
    TargetExists,
    NotFound,
    NameTooLong,
    NoSpace,
    ReadOnlyFs,
    LoopDetected,
    Error,
}

/// Symlink type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymlinkKind {
    Relative,
    Absolute,
    FastSymlink,
    PageSymlink,
}

/// Symlink resolution depth
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymlinkDepthLimit {
    Default,
    Custom(u32),
    Unlimited,
}

/// Symlink record
#[derive(Debug, Clone)]
pub struct SymlinkRecord {
    pub target_hash: u64,
    pub link_hash: u64,
    pub kind: SymlinkKind,
    pub result: SymlinkResult,
    pub inode: u64,
    pub target_len: u32,
    pub resolution_depth: u32,
    pub duration_ns: u64,
}

impl SymlinkRecord {
    pub fn new(target: &[u8], link_path: &[u8]) -> Self {
        let hash = |path: &[u8]| -> u64 {
            let mut h: u64 = 0xcbf29ce484222325;
            for b in path { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
            h
        };
        let kind = if target.first() == Some(&b'/') {
            SymlinkKind::Absolute
        } else {
            SymlinkKind::Relative
        };
        let target_len = target.len() as u32;
        Self {
            target_hash: hash(target),
            link_hash: hash(link_path),
            kind,
            result: SymlinkResult::Success,
            inode: 0,
            target_len,
            resolution_depth: 0,
            duration_ns: 0,
        }
    }

    #[inline(always)]
    pub fn is_fast_symlink(&self) -> bool {
        self.target_len <= 60
    }
}

/// Symlink resolution state
#[derive(Debug, Clone)]
pub struct SymlinkResolver {
    pub max_depth: u32,
    pub current_depth: u32,
    pub visited: Vec<u64>,
    pub total_resolutions: u64,
    pub loop_detections: u64,
}

impl SymlinkResolver {
    pub fn new(max_depth: u32) -> Self {
        Self {
            max_depth,
            current_depth: 0,
            visited: Vec::new(),
            total_resolutions: 0,
            loop_detections: 0,
        }
    }

    pub fn enter(&mut self, inode: u64) -> bool {
        if self.current_depth >= self.max_depth {
            self.loop_detections += 1;
            return false;
        }
        if self.visited.contains(&inode) {
            self.loop_detections += 1;
            return false;
        }
        self.visited.push(inode);
        self.current_depth += 1;
        true
    }

    #[inline]
    pub fn reset(&mut self) {
        self.current_depth = 0;
        self.visited.clear();
        self.total_resolutions += 1;
    }

    #[inline(always)]
    pub fn loop_rate(&self) -> f64 {
        if self.total_resolutions == 0 { 0.0 }
        else { self.loop_detections as f64 / self.total_resolutions as f64 }
    }
}

/// Symlink app stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SymlinkAppStats {
    pub total_created: u64,
    pub relative_links: u64,
    pub absolute_links: u64,
    pub fast_symlinks: u64,
    pub failures: u64,
    pub loop_detections: u64,
}

/// Main app symlink
#[derive(Debug)]
pub struct AppSymlink {
    pub resolver: SymlinkResolver,
    pub stats: SymlinkAppStats,
}

impl AppSymlink {
    pub fn new() -> Self {
        Self {
            resolver: SymlinkResolver::new(40),
            stats: SymlinkAppStats {
                total_created: 0,
                relative_links: 0,
                absolute_links: 0,
                fast_symlinks: 0,
                failures: 0,
                loop_detections: 0,
            },
        }
    }

    pub fn record(&mut self, record: &SymlinkRecord) {
        match record.result {
            SymlinkResult::Success => {
                self.stats.total_created += 1;
                match record.kind {
                    SymlinkKind::Relative => self.stats.relative_links += 1,
                    SymlinkKind::Absolute => self.stats.absolute_links += 1,
                    _ => {}
                }
                if record.is_fast_symlink() {
                    self.stats.fast_symlinks += 1;
                }
            }
            SymlinkResult::LoopDetected => {
                self.stats.failures += 1;
                self.stats.loop_detections += 1;
            }
            _ => self.stats.failures += 1,
        }
    }
}
