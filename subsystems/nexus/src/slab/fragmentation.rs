//! Fragmentation Analyzer
//!
//! This module provides fragmentation analysis for slab caches.

use alloc::collections::VecDeque;
use alloc::vec::Vec;

use super::SlabCacheId;

/// Fragmentation level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FragmentationLevel {
    /// No fragmentation
    None   = 0,
    /// Low fragmentation
    Low    = 1,
    /// Medium fragmentation
    Medium = 2,
    /// High fragmentation
    High   = 3,
    /// Severe fragmentation
    Severe = 4,
}

impl FragmentationLevel {
    /// Get level name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Severe => "severe",
        }
    }
}

/// Fragmentation sample
#[derive(Debug, Clone, Copy)]
pub struct FragmentationSample {
    /// Timestamp
    pub timestamp: u64,
    /// Internal fragmentation (%)
    pub internal_frag: f32,
    /// External fragmentation (%)
    pub external_frag: f32,
    /// Partial slabs count
    pub partial_slabs: u64,
    /// Full slabs count
    pub full_slabs: u64,
    /// Empty slabs count
    pub empty_slabs: u64,
}

/// Fragmentation analyzer
pub struct FragmentationAnalyzer {
    /// Cache ID
    cache_id: SlabCacheId,
    /// Historical samples
    samples: VecDeque<FragmentationSample>,
    /// Maximum samples
    max_samples: usize,
    /// Current fragmentation level
    current_level: FragmentationLevel,
    /// Internal fragmentation threshold (%)
    internal_threshold: f32,
    /// External fragmentation threshold (%)
    external_threshold: f32,
    /// Defragmentation operations
    defrag_count: u64,
}

impl FragmentationAnalyzer {
    /// Create new fragmentation analyzer
    pub fn new(cache_id: SlabCacheId) -> Self {
        Self {
            cache_id,
            samples: Vec::with_capacity(256),
            max_samples: 256,
            current_level: FragmentationLevel::None,
            internal_threshold: 10.0, // 10%
            external_threshold: 20.0, // 20%
            defrag_count: 0,
        }
    }

    /// Record fragmentation sample
    pub fn record_sample(&mut self, sample: FragmentationSample) {
        // Calculate new level
        let total_frag = sample.internal_frag + sample.external_frag;
        self.current_level = if total_frag < 5.0 {
            FragmentationLevel::None
        } else if total_frag < 15.0 {
            FragmentationLevel::Low
        } else if total_frag < 30.0 {
            FragmentationLevel::Medium
        } else if total_frag < 50.0 {
            FragmentationLevel::High
        } else {
            FragmentationLevel::Severe
        };

        // Store sample
        if self.samples.len() >= self.max_samples {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
    }

    /// Calculate internal fragmentation
    #[inline]
    pub fn calculate_internal_fragmentation(&self, object_size: usize, aligned_size: usize) -> f32 {
        if aligned_size == 0 {
            return 0.0;
        }
        let wasted = aligned_size - object_size;
        (wasted as f32 / aligned_size as f32) * 100.0
    }

    /// Calculate external fragmentation from slab states
    pub fn calculate_external_fragmentation(
        &self,
        partial_slabs: u64,
        full_slabs: u64,
        empty_slabs: u64,
        active_objects: u64,
        total_objects: u64,
    ) -> f32 {
        if total_objects == 0 {
            return 0.0;
        }

        let total_slabs = partial_slabs + full_slabs + empty_slabs;
        if total_slabs == 0 {
            return 0.0;
        }

        // External fragmentation based on partial slab waste
        let unused = total_objects - active_objects;
        let partial_waste = (partial_slabs as f32 / total_slabs as f32) * 50.0;
        let object_waste = (unused as f32 / total_objects as f32) * 50.0;

        partial_waste + object_waste
    }

    /// Get current fragmentation level
    #[inline(always)]
    pub fn current_level(&self) -> FragmentationLevel {
        self.current_level
    }

    /// Check if defragmentation is recommended
    #[inline(always)]
    pub fn recommend_defrag(&self) -> bool {
        self.current_level >= FragmentationLevel::High
    }

    /// Record defragmentation
    #[inline(always)]
    pub fn record_defrag(&mut self) {
        self.defrag_count += 1;
    }

    /// Get defrag count
    #[inline(always)]
    pub fn defrag_count(&self) -> u64 {
        self.defrag_count
    }

    /// Get cache ID
    #[inline(always)]
    pub fn cache_id(&self) -> SlabCacheId {
        self.cache_id
    }

    /// Set thresholds
    #[inline(always)]
    pub fn set_thresholds(&mut self, internal: f32, external: f32) {
        self.internal_threshold = internal;
        self.external_threshold = external;
    }

    /// Get sample count
    #[inline(always)]
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }
}
