// SPDX-License-Identifier: MIT
//! # Application Page-Out Policy Manager
//!
//! Per-application page eviction and reclaim policy:
//! - Working set size estimation per epoch
//! - Page age histogram for LRU/Clock decisions
//! - Application memory pressure scoring
//! - Proactive page-out scheduling for idle apps
//! - Memory balloon integration for guest VMs

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Page age bucket (ticks since last access)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PageAgeBucket {
    Hot,    // < 100ms
    Warm,   // 100ms - 1s
    Cool,   // 1s - 10s
    Cold,   // 10s - 60s
    Frozen, // > 60s
}

impl PageAgeBucket {
    #[inline]
    pub fn from_age_ticks(ticks: u64, ticks_per_sec: u64) -> Self {
        let ms = ticks * 1000 / ticks_per_sec.max(1);
        match ms {
            0..=99 => Self::Hot,
            100..=999 => Self::Warm,
            1000..=9999 => Self::Cool,
            10000..=59999 => Self::Cold,
            _ => Self::Frozen,
        }
    }
}

/// Page age histogram for an application
#[derive(Debug, Clone)]
pub struct PageAgeHistogram {
    pub hot: u64,
    pub warm: u64,
    pub cool: u64,
    pub cold: u64,
    pub frozen: u64,
}

impl PageAgeHistogram {
    pub fn new() -> Self {
        Self {
            hot: 0,
            warm: 0,
            cool: 0,
            cold: 0,
            frozen: 0,
        }
    }

    #[inline]
    pub fn record(&mut self, bucket: PageAgeBucket) {
        match bucket {
            PageAgeBucket::Hot => self.hot += 1,
            PageAgeBucket::Warm => self.warm += 1,
            PageAgeBucket::Cool => self.cool += 1,
            PageAgeBucket::Cold => self.cold += 1,
            PageAgeBucket::Frozen => self.frozen += 1,
        }
    }

    #[inline(always)]
    pub fn total(&self) -> u64 {
        self.hot + self.warm + self.cool + self.cold + self.frozen
    }

    /// Estimated working set: hot + warm pages
    #[inline(always)]
    pub fn working_set(&self) -> u64 {
        self.hot + self.warm
    }

    /// Pages safe to evict without impact
    #[inline(always)]
    pub fn evictable(&self) -> u64 {
        self.cold + self.frozen
    }

    /// Ratio of actively-used pages
    #[inline]
    pub fn active_ratio(&self) -> f64 {
        let total = self.total();
        if total == 0 {
            return 0.0;
        }
        (self.hot + self.warm) as f64 / total as f64
    }

    #[inline]
    pub fn reset(&mut self) {
        self.hot = 0;
        self.warm = 0;
        self.cool = 0;
        self.cold = 0;
        self.frozen = 0;
    }
}

/// Memory pressure level for an application
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemPressure {
    None,
    Low,
    Medium,
    High,
    Critical,
}

/// Per-app page-out policy
#[derive(Debug, Clone)]
pub struct AppPagePolicy {
    pub app_id: u64,
    pub histogram: PageAgeHistogram,
    /// Estimated working set in pages
    pub working_set_pages: u64,
    /// RSS (Resident Set Size) in pages
    pub rss_pages: u64,
    /// Memory limit (0 = unlimited)
    pub mem_limit_pages: u64,
    /// Current pressure level
    pub pressure: MemPressure,
    /// Number of page-outs performed
    pub pageouts: u64,
    /// Number of page-ins (refaults)
    pub refaults: u64,
    /// Is this app a candidate for proactive pageout?
    pub proactive_eligible: bool,
    /// Last activity timestamp
    pub last_active: u64,
}

impl AppPagePolicy {
    pub fn new(app_id: u64) -> Self {
        Self {
            app_id,
            histogram: PageAgeHistogram::new(),
            working_set_pages: 0,
            rss_pages: 0,
            mem_limit_pages: 0,
            pressure: MemPressure::None,
            pageouts: 0,
            refaults: 0,
            proactive_eligible: false,
            last_active: 0,
        }
    }

    /// Recompute pressure based on RSS vs limit and working set
    pub fn recompute_pressure(&mut self) {
        if self.mem_limit_pages == 0 {
            self.pressure = MemPressure::None;
            return;
        }

        let usage_ratio = self.rss_pages as f64 / self.mem_limit_pages as f64;
        self.pressure = match () {
            _ if usage_ratio > 0.95 => MemPressure::Critical,
            _ if usage_ratio > 0.85 => MemPressure::High,
            _ if usage_ratio > 0.70 => MemPressure::Medium,
            _ if usage_ratio > 0.50 => MemPressure::Low,
            _ => MemPressure::None,
        };
    }

    /// How many pages should we try to reclaim from this app?
    #[inline]
    pub fn reclaim_target(&self) -> u64 {
        match self.pressure {
            MemPressure::None | MemPressure::Low => 0,
            MemPressure::Medium => self.histogram.cool / 2,
            MemPressure::High => self.histogram.cool + self.histogram.cold / 2,
            MemPressure::Critical => self.histogram.evictable(),
        }
    }

    #[inline]
    pub fn refault_rate(&self) -> f64 {
        if self.pageouts == 0 {
            return 0.0;
        }
        self.refaults as f64 / self.pageouts as f64
    }
}

/// Pageout manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct PageoutAppStats {
    pub apps_tracked: u64,
    pub total_pageouts: u64,
    pub total_refaults: u64,
    pub proactive_pageouts: u64,
    pub pressure_events: u64,
}

/// Per-application page-out manager
pub struct PageoutAppManager {
    apps: BTreeMap<u64, AppPagePolicy>,
    stats: PageoutAppStats,
    ticks_per_sec: u64,
    /// Idle threshold for proactive pageout (ticks)
    idle_threshold: u64,
}

impl PageoutAppManager {
    pub fn new(ticks_per_sec: u64) -> Self {
        Self {
            apps: BTreeMap::new(),
            stats: PageoutAppStats::default(),
            ticks_per_sec,
            idle_threshold: ticks_per_sec * 30, // 30 seconds idle
        }
    }

    #[inline]
    pub fn register_app(&mut self, app_id: u64) {
        self.apps.entry(app_id).or_insert_with(|| {
            self.stats.apps_tracked += 1;
            AppPagePolicy::new(app_id)
        });
    }

    #[inline]
    pub fn update_histogram(&mut self, app_id: u64, histogram: PageAgeHistogram) {
        if let Some(policy) = self.apps.get_mut(&app_id) {
            policy.working_set_pages = histogram.working_set();
            policy.histogram = histogram;
            policy.recompute_pressure();
        }
    }

    #[inline]
    pub fn record_pageout(&mut self, app_id: u64, pages: u64) {
        if let Some(policy) = self.apps.get_mut(&app_id) {
            policy.pageouts += pages;
            policy.rss_pages = policy.rss_pages.saturating_sub(pages);
            policy.recompute_pressure();
            self.stats.total_pageouts += pages;
        }
    }

    #[inline]
    pub fn record_refault(&mut self, app_id: u64) {
        if let Some(policy) = self.apps.get_mut(&app_id) {
            policy.refaults += 1;
            policy.rss_pages += 1;
            policy.last_active = 0; // mark active
            self.stats.total_refaults += 1;
        }
    }

    /// Find apps eligible for proactive pageout (idle > threshold)
    pub fn proactive_candidates(&self, now: u64) -> Vec<(u64, u64)> {
        let mut candidates = Vec::new();
        for (_, policy) in &self.apps {
            if now.saturating_sub(policy.last_active) > self.idle_threshold {
                let target = policy.histogram.evictable();
                if target > 0 {
                    candidates.push((policy.app_id, target));
                }
            }
        }
        candidates.sort_by(|a, b| b.1.cmp(&a.1));
        candidates
    }

    /// Get apps sorted by pressure (highest first)
    #[inline]
    pub fn pressure_ranking(&self) -> Vec<(u64, MemPressure, u64)> {
        let mut ranking: Vec<(u64, MemPressure, u64)> = self
            .apps
            .values()
            .filter(|p| p.pressure >= MemPressure::Medium)
            .map(|p| (p.app_id, p.pressure, p.reclaim_target()))
            .collect();
        ranking.sort_by(|a, b| b.1.cmp(&a.1));
        ranking
    }

    #[inline(always)]
    pub fn stats(&self) -> &PageoutAppStats {
        &self.stats
    }
}
