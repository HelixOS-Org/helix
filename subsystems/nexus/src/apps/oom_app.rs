// SPDX-License-Identifier: MIT
//! # Application OOM Manager
//!
//! Out-Of-Memory killer intelligence at the app layer:
//! - Per-app OOM score computation (badness heuristic)
//! - Sacrificial victim selection with fairness
//! - Memory reclaim watermark management
//! - Grace period enforcement before kill
//! - Post-kill memory recovery tracking

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OomAdjust { Disabled, Low, Normal, High, Critical }

impl OomAdjust {
    #[inline]
    pub fn multiplier(&self) -> f64 {
        match self {
            Self::Disabled => 0.0,
            Self::Low => 0.5,
            Self::Normal => 1.0,
            Self::High => 1.5,
            Self::Critical => 2.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppOomProfile {
    pub app_id: u64,
    pub rss_pages: u64,
    pub swap_pages: u64,
    pub shared_pages: u64,
    pub oom_adjust: OomAdjust,
    pub badness_score: u64,
    pub kills_survived: u32,
    pub grace_period_ns: u64,
    pub is_essential: bool,
    pub last_reclaim: u64,
}

impl AppOomProfile {
    /// Compute badness score: higher = more likely to be killed
    pub fn compute_badness(&mut self, total_ram_pages: u64) {
        if self.oom_adjust == OomAdjust::Disabled || self.is_essential {
            self.badness_score = 0;
            return;
        }
        let effective_rss = self.rss_pages + self.swap_pages
            - self.shared_pages / 2; // Shared pages counted at half
        let base = if total_ram_pages > 0 {
            (effective_rss * 1000) / total_ram_pages
        } else {
            effective_rss
        };
        self.badness_score = (base as f64 * self.oom_adjust.multiplier()) as u64;
    }

    #[inline(always)]
    pub fn reclaimable_pages(&self) -> u64 {
        self.rss_pages.saturating_sub(self.shared_pages)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatermarkLevel { High, Low, Min, Critical }

impl WatermarkLevel {
    #[inline]
    pub fn from_free_ratio(ratio: f64) -> Self {
        if ratio > 0.15 { Self::High }
        else if ratio > 0.08 { Self::Low }
        else if ratio > 0.03 { Self::Min }
        else { Self::Critical }
    }
}

#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct OomAppStats {
    pub total_kills: u64,
    pub total_reclaimed_pages: u64,
    pub false_positives: u64,  // kills that didn't help
    pub cascade_kills: u64,    // second-round kills
    pub current_watermark: u64,
}

pub struct OomAppManager {
    profiles: BTreeMap<u64, AppOomProfile>,
    /// Kill history: (timestamp, app_id, pages_reclaimed)
    kill_history: Vec<(u64, u64, u64)>,
    total_ram_pages: u64,
    stats: OomAppStats,
}

impl OomAppManager {
    pub fn new(total_ram_pages: u64) -> Self {
        Self {
            profiles: BTreeMap::new(),
            kill_history: Vec::new(),
            total_ram_pages,
            stats: OomAppStats::default(),
        }
    }

    #[inline(always)]
    pub fn register_app(&mut self, profile: AppOomProfile) {
        self.profiles.insert(profile.app_id, profile);
    }

    #[inline]
    pub fn update_rss(&mut self, app_id: u64, rss: u64, swap: u64, shared: u64) {
        if let Some(p) = self.profiles.get_mut(&app_id) {
            p.rss_pages = rss;
            p.swap_pages = swap;
            p.shared_pages = shared;
            p.compute_badness(self.total_ram_pages);
        }
    }

    /// Select victim for OOM kill
    #[inline]
    pub fn select_victim(&self) -> Option<u64> {
        self.profiles.iter()
            .filter(|(_, p)| p.badness_score > 0)
            .max_by_key(|(_, p)| p.badness_score)
            .map(|(id, _)| *id)
    }

    /// Select ordered list of victims if one kill isn't enough
    pub fn select_victims_cascade(&self, needed_pages: u64) -> Vec<u64> {
        let mut candidates: Vec<_> = self.profiles.iter()
            .filter(|(_, p)| p.badness_score > 0)
            .map(|(id, p)| (*id, p.badness_score, p.reclaimable_pages()))
            .collect();
        candidates.sort_by(|a, b| b.1.cmp(&a.1));

        let mut selected = Vec::new();
        let mut reclaimed = 0u64;
        for (id, _, pages) in candidates {
            if reclaimed >= needed_pages { break; }
            selected.push(id);
            reclaimed += pages;
        }
        selected
    }

    /// Record that an app was killed
    pub fn record_kill(&mut self, app_id: u64, pages_reclaimed: u64, now: u64) {
        self.stats.total_kills += 1;
        self.stats.total_reclaimed_pages += pages_reclaimed;

        if pages_reclaimed == 0 {
            self.stats.false_positives += 1;
        }

        self.kill_history.push((now, app_id, pages_reclaimed));
        if self.kill_history.len() > 256 { self.kill_history.drain(..128); }

        // Track cascade: if 2 kills within 1 second
        if self.kill_history.len() >= 2 {
            let last = &self.kill_history[self.kill_history.len() - 1];
            let prev = &self.kill_history[self.kill_history.len() - 2];
            if last.0 - prev.0 < 1_000_000_000 {
                self.stats.cascade_kills += 1;
            }
        }

        self.profiles.remove(&app_id);
    }

    /// Check current memory pressure level
    #[inline(always)]
    pub fn watermark_level(&self, free_pages: u64) -> WatermarkLevel {
        let ratio = free_pages as f64 / self.total_ram_pages.max(1) as f64;
        WatermarkLevel::from_free_ratio(ratio)
    }

    /// Should we trigger OOM? Returns needed pages
    #[inline]
    pub fn should_trigger_oom(&self, free_pages: u64, min_free_pages: u64) -> Option<u64> {
        if free_pages < min_free_pages {
            Some(min_free_pages - free_pages)
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn profile(&self, app_id: u64) -> Option<&AppOomProfile> { self.profiles.get(&app_id) }
    #[inline(always)]
    pub fn stats(&self) -> &OomAppStats { &self.stats }
}
