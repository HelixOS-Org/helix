//! # Filesystem Intelligence Module
//!
//! AI-powered filesystem analysis and optimization.
//!
//! ## Key Features
//!
//! - **Access Prediction**: Predict file access patterns
//! - **Layout Optimization**: Optimize file placement
//! - **Defragmentation**: Smart defragmentation
//! - **Cache Management**: Intelligent page cache
//! - **Metadata Optimization**: Optimize metadata access
//! - **Workload Analysis**: Analyze I/O workloads
//!
//! # Submodules
//!
//! - `types` - Core type definitions (FileType, AccessMode, IoPatternType)
//! - `metadata` - File metadata for analysis
//! - `access` - File access tracking
//! - `directory` - Directory tree analysis
//! - `fragmentation` - Fragmentation analysis
//! - `cache` - Page cache analysis
//! - `workload` - Workload classification
//! - `intelligence` - Central coordinator

#![allow(dead_code)]

extern crate alloc;

// ============================================================================
// SUBMODULES
// ============================================================================

mod access;
mod cache;
mod directory;
mod fragmentation;
mod intelligence;
mod metadata;
mod types;
mod workload;

// ============================================================================
// RE-EXPORTS
// ============================================================================

pub use access::FileAccessTracker;
pub use cache::{CachedFileInfo, PageCacheAnalyzer};
pub use directory::{DirectoryAnalyzer, DirectoryInfo};
pub use fragmentation::{DefragPriority, FragmentationAnalyzer, FragmentationScore};
pub use intelligence::FilesystemIntelligence;
pub use metadata::FileMeta;
pub use types::{AccessMode, BlockNum, FileDescriptor, FileType, Inode, IoPatternType};
pub use workload::{FsOptimalSettings, FsWorkloadClassifier, FsWorkloadType};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_meta() {
        let mut meta = FileMeta::new(1, FileType::Regular, 4096);
        assert_eq!(meta.access_count, 0);

        meta.record_access();
        assert_eq!(meta.access_count, 1);
    }

    #[test]
    fn test_file_access_tracker() {
        let mut tracker = FileAccessTracker::default();

        // Sequential reads
        for i in 0..50 {
            tracker.record(1, i * 4096, 4096, true);
        }

        assert_eq!(tracker.get_pattern(1), Some(IoPatternType::SequentialRead));
    }

    #[test]
    fn test_fragmentation_analyzer() {
        let mut analyzer = FragmentationAnalyzer::default();

        analyzer.analyze_file(1, 1024 * 1024, 1, false);
        analyzer.analyze_file(2, 1024 * 1024, 100, true);

        let score1 = analyzer.get_score(1).unwrap();
        let score2 = analyzer.get_score(2).unwrap();

        assert!(score1.score < score2.score);
    }

    #[test]
    fn test_page_cache() {
        let mut cache = PageCacheAnalyzer::new(1024 * 1024);

        cache.add_to_cache(1, 10, 100);
        cache.record_hit(1);
        cache.record_hit(1);
        cache.record_miss(2);

        assert!(cache.hit_rate() > 0.5);
    }

    #[test]
    fn test_workload_classifier() {
        let mut classifier = FsWorkloadClassifier::default();

        // Simulate logging workload
        for _ in 0..2000 {
            classifier.record_write(512);
        }

        assert_eq!(classifier.current_workload(), FsWorkloadType::Logging);
    }
}
