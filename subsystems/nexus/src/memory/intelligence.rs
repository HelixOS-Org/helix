//! Central memory intelligence coordinator.

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::allocation::AllocationIntelligence;
use super::hotpage::HotPageTracker;
use super::numa::NumaAnalyzer;
use super::pattern::PatternDetector;
use super::prefetch::PrefetchPredictor;
use super::types::{AccessPattern, AccessType};

// ============================================================================
// MEMORY INTELLIGENCE COORDINATOR
// ============================================================================

/// Central memory intelligence coordinator
pub struct MemoryIntelligence {
    /// Pattern detector
    pattern_detector: PatternDetector,
    /// Prefetch predictor
    prefetch: PrefetchPredictor,
    /// Allocation intelligence
    alloc: AllocationIntelligence,
    /// Hot page tracker
    hot_pages: HotPageTracker,
    /// NUMA analyzer
    numa: NumaAnalyzer,
    /// Total operations
    total_ops: AtomicU64,
}

impl MemoryIntelligence {
    /// Create new memory intelligence
    pub fn new(num_numa_nodes: u32) -> Self {
        Self {
            pattern_detector: PatternDetector::new(1000),
            prefetch: PrefetchPredictor::new(),
            alloc: AllocationIntelligence::new(),
            hot_pages: HotPageTracker::new(),
            numa: NumaAnalyzer::new(num_numa_nodes),
            total_ops: AtomicU64::new(0),
        }
    }

    /// Record memory access
    #[inline]
    pub fn record_access(&mut self, address: u64, is_write: bool) {
        let access_type = if is_write {
            AccessType::Write
        } else {
            AccessType::Read
        };

        self.prefetch.record_access(address, access_type);
        self.hot_pages.record_access(address / 4096, is_write);
        self.total_ops.fetch_add(1, Ordering::Relaxed);
    }

    /// Get prefetch addresses
    #[inline(always)]
    pub fn get_prefetch_addresses(&self, current: u64, count: usize) -> Vec<u64> {
        self.prefetch.get_prefetch_addresses(current, count)
    }

    /// Get current access pattern
    #[inline(always)]
    pub fn current_pattern(&self) -> (AccessPattern, f64) {
        self.prefetch.current_pattern()
    }

    /// Record allocation
    #[inline(always)]
    pub fn record_alloc(&mut self, id: u64, size: u64, address: u64, source: u64) {
        self.alloc.record_alloc(id, size, address, source);
    }

    /// Record deallocation
    #[inline(always)]
    pub fn record_dealloc(&mut self, id: u64) {
        self.alloc.record_dealloc(id);
    }

    /// Get allocation recommendation
    #[inline(always)]
    pub fn recommend_alloc_size(&self, requested: u64, source: u64) -> u64 {
        self.alloc.recommend_size(requested, source)
    }

    /// Check if page is hot
    #[inline(always)]
    pub fn is_hot_page(&self, page: u64) -> bool {
        self.hot_pages.is_hot(page)
    }

    /// Get hot pages
    #[inline(always)]
    pub fn get_hot_pages(&self) -> Vec<u64> {
        self.hot_pages.hot_pages()
    }

    /// Get NUMA efficiency
    #[inline(always)]
    pub fn numa_efficiency(&self) -> f64 {
        self.numa.numa_efficiency()
    }

    /// Get fragmentation level
    #[inline(always)]
    pub fn fragmentation_level(&self) -> f64 {
        self.alloc.fragmentation_level()
    }

    /// Get total operations
    #[inline(always)]
    pub fn total_operations(&self) -> u64 {
        self.total_ops.load(Ordering::Relaxed)
    }

    /// Get pattern detector
    #[inline(always)]
    pub fn pattern_detector(&self) -> &PatternDetector {
        &self.pattern_detector
    }

    /// Get prefetch predictor
    #[inline(always)]
    pub fn prefetch_predictor(&self) -> &PrefetchPredictor {
        &self.prefetch
    }

    /// Get allocation intelligence
    #[inline(always)]
    pub fn allocation_intelligence(&self) -> &AllocationIntelligence {
        &self.alloc
    }

    /// Get mutable allocation intelligence
    #[inline(always)]
    pub fn allocation_intelligence_mut(&mut self) -> &mut AllocationIntelligence {
        &mut self.alloc
    }

    /// Get hot page tracker
    #[inline(always)]
    pub fn hot_page_tracker(&self) -> &HotPageTracker {
        &self.hot_pages
    }

    /// Get NUMA analyzer
    #[inline(always)]
    pub fn numa_analyzer(&self) -> &NumaAnalyzer {
        &self.numa
    }

    /// Get mutable NUMA analyzer
    #[inline(always)]
    pub fn numa_analyzer_mut(&mut self) -> &mut NumaAnalyzer {
        &mut self.numa
    }
}
