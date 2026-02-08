// SPDX-License-Identifier: MIT
//! # Cooperative Memory Lock
//!
//! Multi-process mlock coordination:
//! - System-wide locked page budget arbitration
//! - Priority-based lock allocation between process groups
//! - Lock contention detection and mediation
//! - Lock preemption for higher-priority processes
//! - Cooperative lock release negotiation

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LockTier {
    Realtime,
    Interactive,
    Batch,
    Background,
}

impl LockTier {
    pub fn quota_fraction(&self) -> f64 {
        match self {
            Self::Realtime => 0.40,
            Self::Interactive => 0.30,
            Self::Batch => 0.20,
            Self::Background => 0.10,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LockAllocation {
    pub pid: u64,
    pub group_id: u64,
    pub tier: LockTier,
    pub locked_pages: u64,
    pub quota_pages: u64,
    pub preempted_count: u32,
}

impl LockAllocation {
    pub fn over_quota(&self) -> bool {
        self.locked_pages > self.quota_pages
    }
    pub fn excess_pages(&self) -> u64 {
        self.locked_pages.saturating_sub(self.quota_pages)
    }
}

#[derive(Debug, Clone)]
pub struct LockConflict {
    pub requester: u64,
    pub holders: Vec<u64>,
    pub pages_needed: u64,
    pub pages_held: u64,
    pub resolved: bool,
}

#[derive(Debug, Clone, Default)]
pub struct MlockCoopStats {
    pub allocations: u64,
    pub preemptions: u64,
    pub negotiations: u64,
    pub denials: u64,
    pub voluntary_releases: u64,
    pub total_locked_pages: u64,
}

pub struct MlockCoopManager {
    allocations: BTreeMap<u64, LockAllocation>,
    /// group_id → aggregated lock usage
    group_usage: BTreeMap<u64, u64>,
    conflicts: Vec<LockConflict>,
    system_lock_limit: u64,
    stats: MlockCoopStats,
}

impl MlockCoopManager {
    pub fn new(system_lock_limit: u64) -> Self {
        Self {
            allocations: BTreeMap::new(),
            group_usage: BTreeMap::new(),
            conflicts: Vec::new(),
            system_lock_limit,
            stats: MlockCoopStats::default(),
        }
    }

    /// Register a process with its lock tier
    pub fn register(&mut self, pid: u64, group_id: u64, tier: LockTier) {
        let quota = (self.system_lock_limit as f64 * tier.quota_fraction()) as u64;
        self.allocations.insert(pid, LockAllocation {
            pid,
            group_id,
            tier,
            locked_pages: 0,
            quota_pages: quota,
            preempted_count: 0,
        });
    }

    /// Request lock pages
    pub fn request_lock(&mut self, pid: u64, pages: u64) -> bool {
        let alloc = match self.allocations.get(&pid) {
            Some(a) => a.clone(),
            None => return false,
        };

        // Check system limit
        if self.stats.total_locked_pages + pages > self.system_lock_limit {
            // Try to preempt lower-priority holders
            if self.try_preempt(alloc.tier, pages) {
                // Preemption succeeded, try again
            } else {
                self.stats.denials += 1;
                return false;
            }
        }

        // Check per-process quota
        if alloc.locked_pages + pages > alloc.quota_pages * 2 {
            self.stats.denials += 1;
            return false;
        }

        // Grant
        if let Some(a) = self.allocations.get_mut(&pid) {
            a.locked_pages += pages;
            *self.group_usage.entry(a.group_id).or_insert(0) += pages;
            self.stats.total_locked_pages += pages;
            self.stats.allocations += 1;
        }
        true
    }

    /// Release locked pages
    pub fn release_lock(&mut self, pid: u64, pages: u64) {
        if let Some(a) = self.allocations.get_mut(&pid) {
            let freed = pages.min(a.locked_pages);
            a.locked_pages -= freed;
            if let Some(g) = self.group_usage.get_mut(&a.group_id) {
                *g = g.saturating_sub(freed);
            }
            self.stats.total_locked_pages = self.stats.total_locked_pages.saturating_sub(freed);
            self.stats.voluntary_releases += 1;
        }
    }

    /// Try to preempt lower-priority lock holders
    fn try_preempt(&mut self, requester_tier: LockTier, needed: u64) -> bool {
        let mut candidates: Vec<_> = self
            .allocations
            .values()
            .filter(|a| a.tier > requester_tier && a.locked_pages > 0)
            .map(|a| (a.pid, a.locked_pages, a.tier))
            .collect();
        // Sort by tier (lowest priority first) then by locked pages (most first)
        candidates.sort_by(|a, b| b.2.cmp(&a.2).then(b.1.cmp(&a.1)));

        let mut freed = 0u64;
        let mut preempted_pids = Vec::new();
        for (pid, pages, _) in &candidates {
            if freed >= needed {
                break;
            }
            preempted_pids.push(*pid);
            freed += pages;
        }

        if freed >= needed {
            for pid in preempted_pids {
                if let Some(a) = self.allocations.get_mut(&pid) {
                    let released = a.locked_pages;
                    a.locked_pages = 0;
                    a.preempted_count += 1;
                    if let Some(g) = self.group_usage.get_mut(&a.group_id) {
                        *g = g.saturating_sub(released);
                    }
                    self.stats.total_locked_pages =
                        self.stats.total_locked_pages.saturating_sub(released);
                }
                self.stats.preemptions += 1;
            }
            true
        } else {
            false
        }
    }

    /// Find processes over their quota that should be asked to release
    pub fn over_quota_processes(&self) -> Vec<(u64, u64)> {
        self.allocations
            .values()
            .filter(|a| a.over_quota())
            .map(|a| (a.pid, a.excess_pages()))
            .collect()
    }

    pub fn allocation(&self, pid: u64) -> Option<&LockAllocation> {
        self.allocations.get(&pid)
    }
    pub fn stats(&self) -> &MlockCoopStats {
        &self.stats
    }
}
