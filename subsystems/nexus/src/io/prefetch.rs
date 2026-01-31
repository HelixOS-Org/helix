//! Intelligent prefetch engine.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::pattern::IoPatternAnalyzer;
use crate::core::NexusTimestamp;

// ============================================================================
// PREFETCH REQUEST
// ============================================================================

/// Prefetch request
#[derive(Debug, Clone)]
struct PrefetchRequest {
    /// Device ID
    device_id: u32,
    /// Offset
    offset: u64,
    /// Size
    size: u32,
    /// Process ID that triggered
    process_id: u64,
    /// Submitted at
    submitted_at: NexusTimestamp,
    /// Hit count (how many reads matched)
    hits: u32,
}

// ============================================================================
// PREFETCH CONFIG
// ============================================================================

/// Prefetch configuration
#[derive(Debug, Clone)]
pub struct PrefetchConfig {
    /// Maximum concurrent prefetches per device
    pub max_concurrent: usize,
    /// Prefetch size
    pub prefetch_size: u32,
    /// Prefetch depth (how many ahead)
    pub prefetch_depth: usize,
    /// Minimum confidence to trigger
    pub min_confidence: f64,
    /// Enabled
    pub enabled: bool,
}

impl Default for PrefetchConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 8,
            prefetch_size: 64 * 1024, // 64KB
            prefetch_depth: 4,
            min_confidence: 0.6,
            enabled: true,
        }
    }
}

// ============================================================================
// PREFETCH STATS
// ============================================================================

/// Prefetch statistics
#[derive(Debug, Clone, Default)]
pub struct PrefetchStats {
    /// Total prefetches issued
    pub prefetches_issued: u64,
    /// Prefetch hits
    pub hits: u64,
    /// Prefetch misses
    pub misses: u64,
    /// Bytes prefetched
    pub bytes_prefetched: u64,
}

// ============================================================================
// PREFETCH ENGINE
// ============================================================================

/// Intelligent prefetch engine
pub struct PrefetchEngine {
    /// Pattern analyzers by process
    analyzers: BTreeMap<u64, IoPatternAnalyzer>,
    /// Active prefetches
    active_prefetches: Vec<PrefetchRequest>,
    /// Prefetch configuration
    config: PrefetchConfig,
    /// Statistics
    stats: PrefetchStats,
}

impl PrefetchEngine {
    /// Create new prefetch engine
    pub fn new() -> Self {
        Self {
            analyzers: BTreeMap::new(),
            active_prefetches: Vec::new(),
            config: PrefetchConfig::default(),
            stats: PrefetchStats::default(),
        }
    }

    /// Record access and potentially trigger prefetch
    pub fn record_access(
        &mut self,
        device_id: u32,
        process_id: u64,
        offset: u64,
        size: u32,
        is_read: bool,
    ) -> Vec<(u64, u32)> {
        // Only prefetch for reads
        if !is_read || !self.config.enabled {
            return Vec::new();
        }

        // Check for prefetch hits
        self.check_prefetch_hits(offset, size);

        // Update pattern
        let analyzer = self.analyzers.entry(process_id).or_default();
        analyzer.record(offset, size, is_read);

        // Generate prefetch recommendations
        if analyzer.confidence() >= self.config.min_confidence {
            let recommendations = analyzer.prefetch_recommendations(self.config.prefetch_depth);

            let mut prefetches = Vec::new();
            for rec_offset in recommendations {
                if !self.is_prefetch_active(device_id, rec_offset) {
                    self.active_prefetches.push(PrefetchRequest {
                        device_id,
                        offset: rec_offset,
                        size: self.config.prefetch_size,
                        process_id,
                        submitted_at: NexusTimestamp::now(),
                        hits: 0,
                    });

                    prefetches.push((rec_offset, self.config.prefetch_size));
                    self.stats.prefetches_issued += 1;
                    self.stats.bytes_prefetched += self.config.prefetch_size as u64;
                }
            }

            // Limit active prefetches
            while self.active_prefetches.len() > self.config.max_concurrent * 4 {
                let removed = self.active_prefetches.remove(0);
                if removed.hits == 0 {
                    self.stats.misses += 1;
                }
            }

            return prefetches;
        }

        Vec::new()
    }

    /// Check if offset is in active prefetches
    fn is_prefetch_active(&self, device_id: u32, offset: u64) -> bool {
        self.active_prefetches.iter().any(|p| {
            p.device_id == device_id && p.offset <= offset && p.offset + p.size as u64 > offset
        })
    }

    /// Check for prefetch hits
    fn check_prefetch_hits(&mut self, offset: u64, size: u32) {
        for prefetch in &mut self.active_prefetches {
            if prefetch.offset <= offset
                && prefetch.offset + prefetch.size as u64 >= offset + size as u64
            {
                prefetch.hits += 1;
                self.stats.hits += 1;
            }
        }
    }

    /// Get hit ratio
    pub fn hit_ratio(&self) -> f64 {
        let total = self.stats.hits + self.stats.misses;
        if total == 0 {
            0.0
        } else {
            self.stats.hits as f64 / total as f64
        }
    }

    /// Set configuration
    pub fn set_config(&mut self, config: PrefetchConfig) {
        self.config = config;
    }

    /// Get statistics
    pub fn stats(&self) -> &PrefetchStats {
        &self.stats
    }
}

impl Default for PrefetchEngine {
    fn default() -> Self {
        Self::new()
    }
}
