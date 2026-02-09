// SPDX-License-Identifier: MIT
//! # Application Heap Allocation Profiler
//!
//! Per-application heap analysis and optimization:
//! - Allocation pattern recognition (slab-friendly, arena, random)
//! - Fragmentation index computation
//! - Peak usage tracking per epoch
//! - Allocation hot-path detection via call-site hashing
//! - Predictive pre-allocation hints

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// HEAP PROFILE TYPES
// ============================================================================

/// Allocation pattern classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocPattern {
    /// Fixed-size allocations (slab-friendly)
    SlabFriendly,
    /// Monotonically growing then bulk-free (arena)
    Arena,
    /// Random size / random lifetime
    Random,
    /// Many small short-lived allocations (generational)
    Generational,
    /// Large infrequent allocations
    LargeInfrequent,
    /// Mixed or unclassified
    Mixed,
}

/// Allocation site identifier (FNV-1a hash of caller address chain)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AllocSiteId(pub u64);

impl AllocSiteId {
    #[inline]
    pub fn from_caller_chain(callers: &[u64]) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for &addr in callers {
            h ^= addr;
            h = h.wrapping_mul(0x100000001b3);
        }
        Self(h)
    }
}

/// Single allocation record
#[derive(Debug, Clone)]
pub struct AllocRecord {
    pub site: AllocSiteId,
    pub size: usize,
    pub align: usize,
    pub timestamp: u64,
    pub freed_at: Option<u64>,
}

/// Per-site aggregate statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SiteStats {
    pub site: AllocSiteId,
    pub total_allocs: u64,
    pub total_frees: u64,
    pub total_bytes: u64,
    pub peak_live_bytes: u64,
    pub avg_lifetime_ticks: u64,
    pub min_size: usize,
    pub max_size: usize,
    pub pattern: AllocPattern,
}

impl SiteStats {
    pub fn new(site: AllocSiteId) -> Self {
        Self {
            site,
            total_allocs: 0,
            total_frees: 0,
            total_bytes: 0,
            peak_live_bytes: 0,
            avg_lifetime_ticks: 0,
            min_size: usize::MAX,
            max_size: 0,
            pattern: AllocPattern::Mixed,
        }
    }

    pub fn record_alloc(&mut self, size: usize) {
        self.total_allocs += 1;
        self.total_bytes += size as u64;
        if size < self.min_size {
            self.min_size = size;
        }
        if size > self.max_size {
            self.max_size = size;
        }
        let live = (self.total_allocs - self.total_frees) * (self.avg_size() as u64);
        if live > self.peak_live_bytes {
            self.peak_live_bytes = live;
        }
    }

    #[inline]
    pub fn record_free(&mut self, lifetime_ticks: u64) {
        self.total_frees += 1;
        // Exponential moving average of lifetime
        let alpha = 8; // 1/8 weight for new sample
        self.avg_lifetime_ticks =
            self.avg_lifetime_ticks - (self.avg_lifetime_ticks / alpha) + (lifetime_ticks / alpha);
    }

    #[inline]
    pub fn avg_size(&self) -> usize {
        if self.total_allocs == 0 {
            0
        } else {
            (self.total_bytes / self.total_allocs) as usize
        }
    }

    pub fn classify(&mut self) {
        let size_variance = self.max_size.saturating_sub(self.min_size);
        let avg = self.avg_size();

        self.pattern = if size_variance < avg / 10 + 1 {
            // Very uniform sizes → slab-friendly
            AllocPattern::SlabFriendly
        } else if self.total_frees < self.total_allocs / 4 {
            // Mostly allocations, few frees → arena
            AllocPattern::Arena
        } else if self.avg_lifetime_ticks < 1000 && avg < 256 {
            // Short-lived small objects → generational
            AllocPattern::Generational
        } else if avg > 4096 && self.total_allocs < 100 {
            // Large infrequent
            AllocPattern::LargeInfrequent
        } else {
            AllocPattern::Random
        };
    }
}

// ============================================================================
// FRAGMENTATION ANALYSIS
// ============================================================================

/// Fragmentation metrics for a heap region
#[derive(Debug, Clone)]
pub struct FragmentationIndex {
    /// Free bytes that cannot satisfy the most common allocation size
    pub unusable_free_bytes: u64,
    /// Total free bytes
    pub total_free_bytes: u64,
    /// Number of free fragments
    pub fragment_count: u64,
    /// Largest contiguous free block
    pub largest_free_block: u64,
    /// Fragmentation ratio (0.0 = perfect, 1.0 = fully fragmented)
    pub ratio: f64,
}

impl FragmentationIndex {
    pub fn compute(free_blocks: &[(u64, u64)], common_alloc_size: u64) -> Self {
        let mut total_free: u64 = 0;
        let mut unusable: u64 = 0;
        let mut largest: u64 = 0;

        for &(_, size) in free_blocks {
            total_free += size;
            if size < common_alloc_size {
                unusable += size;
            }
            if size > largest {
                largest = size;
            }
        }

        let ratio = if total_free == 0 {
            0.0
        } else {
            unusable as f64 / total_free as f64
        };

        Self {
            unusable_free_bytes: unusable,
            total_free_bytes: total_free,
            fragment_count: free_blocks.len() as u64,
            largest_free_block: largest,
            ratio,
        }
    }
}

// ============================================================================
// EPOCH TRACKING
// ============================================================================

/// Heap statistics for a time epoch
#[derive(Debug, Clone)]
pub struct HeapEpoch {
    pub epoch_id: u64,
    pub start_time: u64,
    pub end_time: u64,
    pub allocs_in_epoch: u64,
    pub frees_in_epoch: u64,
    pub peak_live_bytes: u64,
    pub fragmentation: f64,
}

// ============================================================================
// HEAP PROFILER
// ============================================================================

/// Application heap profiling stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HeapAppStats {
    pub apps_tracked: u64,
    pub total_sites: u64,
    pub total_allocs_profiled: u64,
    pub patterns_classified: u64,
    pub pre_alloc_hints_issued: u64,
}

/// Per-application heap profiler
pub struct HeapAppManager {
    /// Per-app site statistics: app_id → (site → stats)
    app_sites: BTreeMap<u64, BTreeMap<AllocSiteId, SiteStats>>,
    /// Per-app epoch history
    epochs: BTreeMap<u64, Vec<HeapEpoch>>,
    /// Current epoch per app
    current_epoch: LinearMap<u64, 64>,
    /// Pre-allocation hints generated
    hints: BTreeMap<u64, Vec<PreAllocHint>>,
    /// Global stats
    stats: HeapAppStats,
    /// Epoch duration in ticks
    epoch_duration: u64,
    /// Maximum epochs to retain
    max_epochs: usize,
}

/// Pre-allocation hint for an application
#[derive(Debug, Clone)]
pub struct PreAllocHint {
    pub site: AllocSiteId,
    pub predicted_size: usize,
    pub predicted_count: u64,
    pub confidence: f64,
}

impl HeapAppManager {
    pub fn new(epoch_duration: u64, max_epochs: usize) -> Self {
        Self {
            app_sites: BTreeMap::new(),
            epochs: BTreeMap::new(),
            current_epoch: LinearMap::new(),
            hints: BTreeMap::new(),
            stats: HeapAppStats::default(),
            epoch_duration,
            max_epochs,
        }
    }

    /// Record an allocation event from an application
    pub fn record_alloc(
        &mut self,
        app_id: u64,
        site: AllocSiteId,
        size: usize,
        _align: usize,
        now: u64,
    ) {
        let sites = self.app_sites.entry(app_id).or_insert_with(BTreeMap::new);
        let site_stats = sites.entry(site).or_insert_with(|| {
            self.stats.total_sites += 1;
            SiteStats::new(site)
        });
        site_stats.record_alloc(size);
        self.stats.total_allocs_profiled += 1;

        // Check epoch rollover
        self.maybe_rollover_epoch(app_id, now);
    }

    /// Record a free event
    #[inline]
    pub fn record_free(&mut self, app_id: u64, site: AllocSiteId, lifetime_ticks: u64) {
        if let Some(sites) = self.app_sites.get_mut(&app_id) {
            if let Some(site_stats) = sites.get_mut(&site) {
                site_stats.record_free(lifetime_ticks);
            }
        }
    }

    /// Classify all allocation patterns for an application
    #[inline]
    pub fn classify_app(&mut self, app_id: u64) {
        if let Some(sites) = self.app_sites.get_mut(&app_id) {
            for (_, site_stats) in sites.iter_mut() {
                site_stats.classify();
                self.stats.patterns_classified += 1;
            }
        }
    }

    /// Get the dominant allocation pattern for an application
    pub fn dominant_pattern(&self, app_id: u64) -> AllocPattern {
        let sites = match self.app_sites.get(&app_id) {
            Some(s) => s,
            None => return AllocPattern::Mixed,
        };

        let mut counts = [0u64; 6];
        for site in sites.values() {
            let idx = match site.pattern {
                AllocPattern::SlabFriendly => 0,
                AllocPattern::Arena => 1,
                AllocPattern::Random => 2,
                AllocPattern::Generational => 3,
                AllocPattern::LargeInfrequent => 4,
                AllocPattern::Mixed => 5,
            };
            counts[idx] += site.total_allocs;
        }

        let max_idx = counts
            .iter()
            .enumerate()
            .max_by_key(|(_, &c)| c)
            .map(|(i, _)| i)
            .unwrap_or(5);

        match max_idx {
            0 => AllocPattern::SlabFriendly,
            1 => AllocPattern::Arena,
            2 => AllocPattern::Random,
            3 => AllocPattern::Generational,
            4 => AllocPattern::LargeInfrequent,
            _ => AllocPattern::Mixed,
        }
    }

    /// Generate pre-allocation hints based on past patterns
    pub fn generate_hints(&mut self, app_id: u64) -> &[PreAllocHint] {
        let mut new_hints = Vec::new();

        if let Some(sites) = self.app_sites.get(&app_id) {
            for (_, site) in sites.iter() {
                if site.total_allocs < 100 {
                    continue; // Not enough data
                }
                // For slab-friendly patterns, predict same-size allocation
                if site.pattern == AllocPattern::SlabFriendly {
                    let avg = site.avg_size();
                    let predicted_count = site.total_allocs / 10; // 10% pre-alloc
                    if predicted_count > 0 {
                        new_hints.push(PreAllocHint {
                            site: site.site,
                            predicted_size: avg,
                            predicted_count,
                            confidence: 0.85,
                        });
                    }
                }
                // For generational, predict burst
                if site.pattern == AllocPattern::Generational {
                    new_hints.push(PreAllocHint {
                        site: site.site,
                        predicted_size: site.avg_size(),
                        predicted_count: 64, // Pre-alloc 64 objects
                        confidence: 0.70,
                    });
                }
            }
        }

        self.stats.pre_alloc_hints_issued += new_hints.len() as u64;
        self.hints.insert(app_id, new_hints);
        self.hints.get(&app_id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Rollover epoch if duration exceeded
    fn maybe_rollover_epoch(&mut self, app_id: u64, now: u64) {
        let epoch_id = self.current_epoch.entry(app_id).or_insert(0);
        let epoch_start = *epoch_id * self.epoch_duration;

        if now >= epoch_start + self.epoch_duration {
            let epoch = HeapEpoch {
                epoch_id: *epoch_id,
                start_time: epoch_start,
                end_time: now,
                allocs_in_epoch: 0,
                frees_in_epoch: 0,
                peak_live_bytes: 0,
                fragmentation: 0.0,
            };

            let epochs = self.epochs.entry(app_id).or_insert_with(Vec::new);
            epochs.push(epoch);
            if epochs.len() > self.max_epochs {
                epochs.pop_front();
            }

            *epoch_id += 1;
        }
    }

    /// Get top allocation sites for an app (by total bytes)
    #[inline]
    pub fn top_sites(&self, app_id: u64, n: usize) -> Vec<&SiteStats> {
        match self.app_sites.get(&app_id) {
            Some(sites) => {
                let mut sorted: Vec<&SiteStats> = sites.values().collect();
                sorted.sort_by(|a, b| b.total_bytes.cmp(&a.total_bytes));
                sorted.truncate(n);
                sorted
            },
            None => Vec::new(),
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &HeapAppStats {
        &self.stats
    }

    #[inline(always)]
    pub fn app_count(&self) -> usize {
        self.app_sites.len()
    }
}
