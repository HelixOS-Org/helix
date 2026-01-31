//! Memory intelligence subsystem.
//!
//! Provides intelligent memory management including:
//! - Access pattern detection (sequential, strided, random, etc.)
//! - Prefetch prediction for cache optimization
//! - Allocation intelligence with lifetime analysis
//! - Hot page tracking for NUMA optimization
//! - Cross-node access analysis

#![allow(dead_code)]

extern crate alloc;

// Submodules
mod allocation;
mod hotpage;
mod intelligence;
mod numa;
mod pattern;
mod prefetch;
mod types;

// Re-exports
pub use allocation::{AllocationIntelligence, AllocationStats, FragmentationMetrics};
pub use hotpage::{HotPageStats, HotPageTracker};
pub use intelligence::MemoryIntelligence;
pub use numa::NumaAnalyzer;
pub use pattern::{PatternDetector, PatternStats};
pub use prefetch::PrefetchPredictor;
pub use types::{AccessPattern, AccessRecord, AccessType};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequential_pattern() {
        let mut detector = PatternDetector::new(100);

        for i in 0..50 {
            detector.record(AccessRecord {
                address: 0x1000 + i * 8,
                access_type: AccessType::Read,
                size: 8,
                timestamp: i,
            });
        }

        let (pattern, confidence) = detector.detect_pattern();
        assert_eq!(pattern, AccessPattern::Sequential);
        assert!(confidence > 0.7);
    }

    #[test]
    fn test_strided_pattern() {
        let mut detector = PatternDetector::new(100);

        for i in 0..50 {
            detector.record(AccessRecord {
                address: 0x1000 + i * 256,
                access_type: AccessType::Read,
                size: 8,
                timestamp: i,
            });
        }

        let (pattern, _) = detector.detect_pattern();
        assert!(matches!(pattern, AccessPattern::Strided { .. }));
    }

    #[test]
    fn test_prefetch_predictor() {
        let mut predictor = PrefetchPredictor::new();

        for i in 0..100 {
            predictor.record_access(0x1000 + i * 8, AccessType::Read);
        }

        let prefetch = predictor.get_prefetch_addresses(0x1000 + 100 * 8, 4);
        assert!(!prefetch.is_empty());
    }

    #[test]
    fn test_hot_page_tracker() {
        let mut tracker = HotPageTracker::new();

        // Access same page many times
        for _ in 0..150 {
            tracker.record_access(0x1000, false);
        }

        assert!(tracker.is_hot(0x1000));
        assert!(!tracker.is_hot(0x2000));
    }

    #[test]
    fn test_numa_analyzer() {
        let mut analyzer = NumaAnalyzer::new(4);

        // Local accesses
        for _ in 0..100 {
            analyzer.record_access(1, 0, 0);
        }

        // Remote accesses
        for _ in 0..50 {
            analyzer.record_access(1, 0, 1);
        }

        let local_ratio = analyzer.local_ratio(0);
        assert!(local_ratio > 0.6 && local_ratio < 0.8);
    }

    #[test]
    fn test_allocation_intelligence() {
        let mut alloc = AllocationIntelligence::new();

        // Record allocations
        for i in 0..100 {
            alloc.record_alloc(i, 128, 0x1000 + i * 128, 1);
        }

        // Deallocate some
        for i in 0..50 {
            alloc.record_dealloc(i);
        }

        let stats = alloc.stats();
        assert_eq!(stats.live_allocations, 50);
    }
}
