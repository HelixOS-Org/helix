// SPDX-License-Identifier: MIT
//! # Application Memory Lock Manager
//!
//! Per-application mlock/mlockall tracking:
//! - Locked region registry per app
//! - Lock budget enforcement (per-app and system-wide)
//! - Lock priority scoring (RT apps get more budget)
//! - Lock contention detection
//! - Auto-unlock suggestions for idle locked pages

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockPriority { Realtime, High, Normal, Low, Background }

impl LockPriority {
    #[inline]
    pub fn budget_multiplier(&self) -> f64 {
        match self {
            Self::Realtime => 4.0,
            Self::High => 2.0,
            Self::Normal => 1.0,
            Self::Low => 0.5,
            Self::Background => 0.25,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LockedRegion {
    pub start: u64,
    pub size: u64,
    pub locked_at: u64,
    pub last_access: u64,
    pub access_count: u64,
    pub fault_saved: u64, // page faults avoided by locking
}

impl LockedRegion {
    #[inline(always)]
    pub fn pages(&self) -> u64 { self.size / 4096 }
    #[inline(always)]
    pub fn idle_ticks(&self, now: u64) -> u64 { now.saturating_sub(self.last_access) }
    #[inline(always)]
    pub fn worth_locking(&self, now: u64, threshold: u64) -> bool {
        self.idle_ticks(now) < threshold && self.fault_saved > 0
    }
}

#[derive(Debug, Clone)]
pub struct AppLockProfile {
    pub app_id: u64,
    pub priority: LockPriority,
    pub locked_pages: u64,
    pub budget_pages: u64,
    pub regions: Vec<LockedRegion>,
    pub mlockall_active: bool,
    pub denied_requests: u64,
}

impl AppLockProfile {
    #[inline(always)]
    pub fn budget_remaining(&self) -> u64 { self.budget_pages.saturating_sub(self.locked_pages) }
    #[inline(always)]
    pub fn utilization(&self) -> f64 {
        if self.budget_pages == 0 { return 0.0; }
        self.locked_pages as f64 / self.budget_pages as f64
    }
}

#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct MlockAppStats {
    pub total_locked_pages: u64,
    pub total_budget_pages: u64,
    pub denied_requests: u64,
    pub auto_unlock_suggestions: u64,
    pub contention_events: u64,
}

pub struct MlockAppManager {
    profiles: BTreeMap<u64, AppLockProfile>,
    system_lock_limit: u64, // max pages locked system-wide
    idle_threshold: u64,
    stats: MlockAppStats,
}

impl MlockAppManager {
    pub fn new(system_lock_limit: u64, idle_threshold: u64) -> Self {
        Self {
            profiles: BTreeMap::new(),
            system_lock_limit,
            idle_threshold,
            stats: MlockAppStats::default(),
        }
    }

    #[inline]
    pub fn register_app(&mut self, app_id: u64, priority: LockPriority, base_budget: u64) {
        let budget = (base_budget as f64 * priority.budget_multiplier()) as u64;
        self.profiles.insert(app_id, AppLockProfile {
            app_id, priority, locked_pages: 0,
            budget_pages: budget,
            regions: Vec::new(),
            mlockall_active: false,
            denied_requests: 0,
        });
        self.stats.total_budget_pages += budget;
    }

    /// Try to lock a region for an app. Returns false if over budget.
    pub fn try_lock(
        &mut self, app_id: u64, start: u64, size: u64, now: u64,
    ) -> bool {
        let pages = size / 4096;

        // System-wide check
        if self.stats.total_locked_pages + pages > self.system_lock_limit {
            self.stats.contention_events += 1;
            if let Some(p) = self.profiles.get_mut(&app_id) {
                p.denied_requests += 1;
            }
            self.stats.denied_requests += 1;
            return false;
        }

        let profile = match self.profiles.get_mut(&app_id) {
            Some(p) => p,
            None => return false,
        };

        // Per-app budget check
        if profile.locked_pages + pages > profile.budget_pages {
            profile.denied_requests += 1;
            self.stats.denied_requests += 1;
            return false;
        }

        profile.regions.push(LockedRegion {
            start, size, locked_at: now, last_access: now,
            access_count: 0, fault_saved: 0,
        });
        profile.locked_pages += pages;
        self.stats.total_locked_pages += pages;
        true
    }

    /// Unlock a region
    pub fn unlock(&mut self, app_id: u64, start: u64) -> bool {
        let profile = match self.profiles.get_mut(&app_id) {
            Some(p) => p,
            None => return false,
        };

        if let Some(idx) = profile.regions.iter().position(|r| r.start == start) {
            let region = profile.regions.remove(idx);
            let pages = region.pages();
            profile.locked_pages = profile.locked_pages.saturating_sub(pages);
            self.stats.total_locked_pages = self.stats.total_locked_pages.saturating_sub(pages);
            true
        } else {
            false
        }
    }

    /// Find locked regions that are idle and could be unlocked
    #[inline]
    pub fn suggest_auto_unlock(&self, now: u64) -> Vec<(u64, u64)> {
        let mut suggestions = Vec::new();
        for (app_id, profile) in &self.profiles {
            for region in &profile.regions {
                if !region.worth_locking(now, self.idle_threshold) {
                    suggestions.push((*app_id, region.start));
                }
            }
        }
        suggestions
    }

    /// Record a page access in a locked region (fault was avoided)
    pub fn record_access(&mut self, app_id: u64, addr: u64, now: u64) {
        if let Some(profile) = self.profiles.get_mut(&app_id) {
            for region in &mut profile.regions {
                if addr >= region.start && addr < region.start + region.size {
                    region.last_access = now;
                    region.access_count += 1;
                    region.fault_saved += 1;
                    break;
                }
            }
        }
    }

    #[inline(always)]
    pub fn profile(&self, app_id: u64) -> Option<&AppLockProfile> { self.profiles.get(&app_id) }
    #[inline(always)]
    pub fn stats(&self) -> &MlockAppStats { &self.stats }
}
