//! Central filesystem intelligence coordinator.

extern crate alloc;

use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};

use super::access::FileAccessTracker;
use super::cache::PageCacheAnalyzer;
use super::directory::DirectoryAnalyzer;
use super::fragmentation::FragmentationAnalyzer;
use super::metadata::FileMeta;
use super::types::{Inode, IoPatternType};
use super::workload::{FsOptimalSettings, FsWorkloadClassifier, FsWorkloadType};

// ============================================================================
// FILESYSTEM INTELLIGENCE
// ============================================================================

/// Central filesystem intelligence coordinator
pub struct FilesystemIntelligence {
    /// File access tracker
    access: FileAccessTracker,
    /// Directory analyzer
    directory: DirectoryAnalyzer,
    /// Fragmentation analyzer
    fragmentation: FragmentationAnalyzer,
    /// Page cache analyzer
    cache: PageCacheAnalyzer,
    /// Workload classifier
    workload: FsWorkloadClassifier,
    /// File metadata
    files: BTreeMap<Inode, FileMeta>,
    /// Total operations
    total_ops: AtomicU64,
}

impl FilesystemIntelligence {
    /// Create new filesystem intelligence
    pub fn new() -> Self {
        Self {
            access: FileAccessTracker::default(),
            directory: DirectoryAnalyzer::default(),
            fragmentation: FragmentationAnalyzer::default(),
            cache: PageCacheAnalyzer::default(),
            workload: FsWorkloadClassifier::default(),
            files: BTreeMap::new(),
            total_ops: AtomicU64::new(0),
        }
    }

    /// Register file
    pub fn register_file(&mut self, meta: FileMeta) {
        self.files.insert(meta.inode, meta);
    }

    /// Record read operation
    pub fn record_read(&mut self, inode: Inode, offset: u64, size: u32) {
        self.total_ops.fetch_add(1, Ordering::Relaxed);
        self.access.record(inode, offset, size, true);
        self.workload.record_read(size);

        if let Some(meta) = self.files.get_mut(&inode) {
            meta.record_access();
        }
    }

    /// Record write operation
    pub fn record_write(&mut self, inode: Inode, offset: u64, size: u32) {
        self.total_ops.fetch_add(1, Ordering::Relaxed);
        self.access.record(inode, offset, size, false);
        self.workload.record_write(size);

        if let Some(meta) = self.files.get_mut(&inode) {
            meta.record_access();
        }
    }

    /// Record cache hit
    pub fn record_cache_hit(&mut self, inode: Inode) {
        self.cache.record_hit(inode);
    }

    /// Record cache miss
    pub fn record_cache_miss(&mut self, inode: Inode) {
        self.cache.record_miss(inode);
    }

    /// Get file metadata
    pub fn get_file(&self, inode: Inode) -> Option<&FileMeta> {
        self.files.get(&inode)
    }

    /// Get access pattern
    pub fn get_access_pattern(&self, inode: Inode) -> Option<IoPatternType> {
        self.access.get_pattern(inode)
    }

    /// Get prefetch suggestions
    pub fn prefetch_suggestions(&self, inode: Inode, count: usize) -> alloc::vec::Vec<(u64, u32)> {
        self.access.prefetch_suggestions(inode, count)
    }

    /// Get cache hit rate
    pub fn cache_hit_rate(&self) -> f64 {
        self.cache.hit_rate()
    }

    /// Get filesystem health
    pub fn filesystem_health(&self) -> f64 {
        self.fragmentation.filesystem_health()
    }

    /// Get current workload type
    pub fn workload_type(&self) -> FsWorkloadType {
        self.workload.current_workload()
    }

    /// Get optimal settings
    pub fn optimal_settings(&self) -> FsOptimalSettings {
        self.workload.optimal_settings()
    }

    /// Get access tracker
    pub fn access(&self) -> &FileAccessTracker {
        &self.access
    }

    /// Get directory analyzer
    pub fn directory(&self) -> &DirectoryAnalyzer {
        &self.directory
    }

    /// Get mutable directory analyzer
    pub fn directory_mut(&mut self) -> &mut DirectoryAnalyzer {
        &mut self.directory
    }

    /// Get fragmentation analyzer
    pub fn fragmentation(&self) -> &FragmentationAnalyzer {
        &self.fragmentation
    }

    /// Get mutable fragmentation analyzer
    pub fn fragmentation_mut(&mut self) -> &mut FragmentationAnalyzer {
        &mut self.fragmentation
    }

    /// Get cache analyzer
    pub fn cache(&self) -> &PageCacheAnalyzer {
        &self.cache
    }

    /// Get mutable cache analyzer
    pub fn cache_mut(&mut self) -> &mut PageCacheAnalyzer {
        &mut self.cache
    }

    /// Get total operations
    pub fn total_operations(&self) -> u64 {
        self.total_ops.load(Ordering::Relaxed)
    }
}

impl Default for FilesystemIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
