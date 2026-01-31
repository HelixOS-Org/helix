//! Directory tree analysis.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::types::Inode;
use crate::core::NexusTimestamp;

// ============================================================================
// DIRECTORY INFO
// ============================================================================

/// Directory information
#[derive(Debug, Clone)]
pub struct DirectoryInfo {
    /// Inode
    pub inode: Inode,
    /// Child count
    pub child_count: u32,
    /// Subdirectory count
    pub subdir_count: u32,
    /// Total size of children
    pub total_size: u64,
    /// Access count
    pub access_count: u64,
    /// Depth from root
    pub depth: u32,
    /// Last access
    pub last_access: NexusTimestamp,
}

// ============================================================================
// DIRECTORY ANALYZER
// ============================================================================

/// Analyzes directory tree structure
pub struct DirectoryAnalyzer {
    /// Directory entries
    directories: BTreeMap<Inode, DirectoryInfo>,
    /// Hot directories
    hot_directories: Vec<Inode>,
    /// Deep directories (many levels)
    deep_directories: Vec<(Inode, u32)>,
}

impl DirectoryAnalyzer {
    /// Create new analyzer
    pub fn new() -> Self {
        Self {
            directories: BTreeMap::new(),
            hot_directories: Vec::new(),
            deep_directories: Vec::new(),
        }
    }

    /// Register directory
    pub fn register(&mut self, info: DirectoryInfo) {
        // Track deep directories
        if info.depth > 10 {
            self.deep_directories.push((info.inode, info.depth));
            self.deep_directories.sort_by(|a, b| b.1.cmp(&a.1));
            if self.deep_directories.len() > 100 {
                self.deep_directories.pop();
            }
        }

        self.directories.insert(info.inode, info);
    }

    /// Record directory access
    pub fn record_access(&mut self, inode: Inode) {
        if let Some(dir) = self.directories.get_mut(&inode) {
            dir.access_count += 1;
            dir.last_access = NexusTimestamp::now();

            // Update hot list
            if dir.access_count > 100 && !self.hot_directories.contains(&inode) {
                self.hot_directories.push(inode);
            }
        }
    }

    /// Get directory info
    pub fn get_info(&self, inode: Inode) -> Option<&DirectoryInfo> {
        self.directories.get(&inode)
    }

    /// Get hot directories
    pub fn hot_directories(&self) -> &[Inode] {
        &self.hot_directories
    }

    /// Get deepest directories
    pub fn deepest_directories(&self, n: usize) -> Vec<(Inode, u32)> {
        self.deep_directories.iter().take(n).cloned().collect()
    }

    /// Get largest directories
    pub fn largest_directories(&self, n: usize) -> Vec<&DirectoryInfo> {
        let mut dirs: Vec<_> = self.directories.values().collect();
        dirs.sort_by(|a, b| b.total_size.cmp(&a.total_size));
        dirs.truncate(n);
        dirs
    }

    /// Get most files directories
    pub fn busiest_directories(&self, n: usize) -> Vec<&DirectoryInfo> {
        let mut dirs: Vec<_> = self.directories.values().collect();
        dirs.sort_by(|a, b| b.child_count.cmp(&a.child_count));
        dirs.truncate(n);
        dirs
    }
}

impl Default for DirectoryAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
