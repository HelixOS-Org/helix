//! Hot page tracking and temperature analysis.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::core::NexusTimestamp;

// ============================================================================
// HOT PAGE TRACKER
// ============================================================================

/// Tracks frequently accessed pages
pub struct HotPageTracker {
    /// Page access counts
    access_counts: BTreeMap<u64, PageStats>,
    /// Hot page threshold
    hot_threshold: u64,
    /// Cold page threshold
    cold_threshold: u64,
    /// Decay interval
    decay_interval: u64,
    /// Last decay time
    last_decay: NexusTimestamp,
}

/// Per-page statistics
#[derive(Debug, Clone, Default)]
struct PageStats {
    /// Read count
    reads: u64,
    /// Write count
    writes: u64,
    /// Last access time
    #[allow(dead_code)]
    last_access: u64,
    /// Hotness score
    hotness: f64,
}

impl HotPageTracker {
    /// Create new hot page tracker
    pub fn new() -> Self {
        Self {
            access_counts: BTreeMap::new(),
            hot_threshold: 100,
            cold_threshold: 5,
            decay_interval: 1_000_000_000, // 1 second
            last_decay: NexusTimestamp::now(),
        }
    }

    /// Record page access
    pub fn record_access(&mut self, page_addr: u64, is_write: bool) {
        let now = NexusTimestamp::now();

        // Maybe decay counts
        if now.duration_since(self.last_decay) > self.decay_interval {
            self.decay_counts();
            self.last_decay = now;
        }

        let stats = self.access_counts.entry(page_addr).or_default();
        if is_write {
            stats.writes += 1;
        } else {
            stats.reads += 1;
        }
        stats.last_access = now.raw();

        // Update hotness score
        let total = stats.reads + stats.writes;
        stats.hotness = total as f64 * 0.9 + stats.hotness * 0.1;
    }

    /// Decay access counts
    fn decay_counts(&mut self) {
        for stats in self.access_counts.values_mut() {
            stats.reads = stats.reads * 9 / 10;
            stats.writes = stats.writes * 9 / 10;
            stats.hotness *= 0.9;
        }

        // Remove cold pages
        self.access_counts.retain(|_, s| s.hotness > 0.1);
    }

    /// Get hot pages
    pub fn hot_pages(&self) -> Vec<u64> {
        self.access_counts
            .iter()
            .filter(|(_, s)| s.reads + s.writes >= self.hot_threshold)
            .map(|(&addr, _)| addr)
            .collect()
    }

    /// Get cold pages
    pub fn cold_pages(&self) -> Vec<u64> {
        self.access_counts
            .iter()
            .filter(|(_, s)| s.reads + s.writes <= self.cold_threshold)
            .map(|(&addr, _)| addr)
            .collect()
    }

    /// Get pages by temperature (hottest first)
    pub fn pages_by_temperature(&self) -> Vec<(u64, f64)> {
        let mut pages: Vec<_> = self
            .access_counts
            .iter()
            .map(|(&addr, s)| (addr, s.hotness))
            .collect();
        pages.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        pages
    }

    /// Is page hot?
    pub fn is_hot(&self, page_addr: u64) -> bool {
        self.access_counts
            .get(&page_addr)
            .map(|s| s.reads + s.writes >= self.hot_threshold)
            .unwrap_or(false)
    }

    /// Get page temperature
    pub fn get_temperature(&self, page_addr: u64) -> f64 {
        self.access_counts
            .get(&page_addr)
            .map(|s| s.hotness)
            .unwrap_or(0.0)
    }

    /// Set thresholds
    pub fn set_thresholds(&mut self, hot: u64, cold: u64) {
        self.hot_threshold = hot;
        self.cold_threshold = cold;
    }

    /// Get statistics
    pub fn stats(&self) -> HotPageStats {
        let total = self.access_counts.len();
        let hot = self.hot_pages().len();
        let cold = self.cold_pages().len();

        HotPageStats {
            total_pages: total,
            hot_pages: hot,
            cold_pages: cold,
            warm_pages: total - hot - cold,
        }
    }
}

impl Default for HotPageTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Hot page statistics
#[derive(Debug, Clone)]
pub struct HotPageStats {
    /// Total pages tracked
    pub total_pages: usize,
    /// Hot pages
    pub hot_pages: usize,
    /// Cold pages
    pub cold_pages: usize,
    /// Warm pages
    pub warm_pages: usize,
}
