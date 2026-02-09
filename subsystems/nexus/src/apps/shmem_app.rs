// SPDX-License-Identifier: MIT
//! # Application Shared Memory Manager
//!
//! Per-application shared memory tracking and optimization:
//! - IPC shared region lifecycle management
//! - Cross-process page sharing analytics
//! - Copy-on-write tracking per region
//! - Shared page deduplication scoring
//! - Access pattern correlation between sharers

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Shared region access mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShmAccessMode {
    ReadOnly,
    ReadWrite,
    ExecuteOnly,
    ReadExecute,
}

/// Copy-on-write state for a shared page
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CowState {
    /// Page is still shared (not yet written)
    Shared,
    /// Page was copied after a write
    Copied,
    /// Page was merged back after both copies became identical
    Merged,
}

/// A shared memory region between processes
#[derive(Debug, Clone)]
pub struct SharedRegion {
    pub region_id: u64,
    pub owner_app: u64,
    pub size_bytes: u64,
    pub mode: ShmAccessMode,
    pub created_at: u64,
    /// Processes currently attached
    pub attachers: Vec<u64>,
    /// Number of pages
    pub page_count: u64,
    /// Pages currently in COW state
    pub cow_pages: u64,
    /// Pages that were deduplicated (content-identical)
    pub deduped_pages: u64,
    /// Total read accesses observed
    pub total_reads: u64,
    /// Total write accesses (triggers COW)
    pub total_writes: u64,
    /// Access correlation score between sharers (0.0-1.0)
    pub access_correlation: f64,
}

impl SharedRegion {
    pub fn new(region_id: u64, owner: u64, size_bytes: u64, mode: ShmAccessMode, now: u64) -> Self {
        let page_count = (size_bytes + 4095) / 4096;
        Self {
            region_id,
            owner_app: owner,
            size_bytes,
            mode,
            created_at: now,
            attachers: Vec::new(),
            page_count,
            cow_pages: 0,
            deduped_pages: 0,
            total_reads: 0,
            total_writes: 0,
            access_correlation: 0.0,
        }
    }

    #[inline]
    pub fn attach(&mut self, app_id: u64) {
        if !self.attachers.contains(&app_id) {
            self.attachers.push(app_id);
        }
    }

    #[inline(always)]
    pub fn detach(&mut self, app_id: u64) {
        self.attachers.retain(|&id| id != app_id);
    }

    #[inline(always)]
    pub fn is_orphan(&self) -> bool {
        self.attachers.is_empty()
    }

    #[inline]
    pub fn sharing_ratio(&self) -> f64 {
        if self.page_count == 0 {
            return 0.0;
        }
        let shared = self.page_count.saturating_sub(self.cow_pages);
        shared as f64 / self.page_count as f64
    }

    #[inline]
    pub fn cow_ratio(&self) -> f64 {
        if self.page_count == 0 {
            return 0.0;
        }
        self.cow_pages as f64 / self.page_count as f64
    }

    #[inline(always)]
    pub fn record_read(&mut self) {
        self.total_reads += 1;
    }

    #[inline(always)]
    pub fn record_write(&mut self) {
        self.total_writes += 1;
    }

    #[inline(always)]
    pub fn record_cow(&mut self) {
        self.cow_pages += 1;
    }

    #[inline(always)]
    pub fn record_dedup(&mut self, pages: u64) {
        self.deduped_pages += pages;
    }

    /// Update access correlation between sharers
    /// A high correlation means processes access the same pages at similar times
    #[inline(always)]
    pub fn update_correlation(&mut self, sample: f64) {
        // EMA with alpha=0.1
        self.access_correlation = self.access_correlation * 0.9 + sample * 0.1;
    }

    #[inline]
    pub fn memory_saved_bytes(&self) -> u64 {
        // Shared pages × page_size + deduped pages × page_size
        let shared = self.page_count.saturating_sub(self.cow_pages);
        let savings = shared.saturating_sub(1) + self.deduped_pages; // -1 because 1 copy must exist
        savings * 4096
    }
}

/// Per-app shared memory access pattern
#[derive(Debug, Clone)]
pub struct AppShmProfile {
    pub app_id: u64,
    pub regions_owned: u64,
    pub regions_attached: u64,
    pub total_shared_bytes: u64,
    pub read_write_ratio: f64,
    pub avg_region_lifetime: u64,
}

/// Shared memory manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct ShmAppStats {
    pub regions_created: u64,
    pub regions_destroyed: u64,
    pub total_attachments: u64,
    pub total_cow_events: u64,
    pub total_dedup_pages: u64,
    pub total_memory_saved: u64,
}

/// Application shared memory manager
pub struct ShmAppManager {
    regions: BTreeMap<u64, SharedRegion>,
    /// Per-app region sets: app_id → [region_ids]
    app_regions: BTreeMap<u64, Vec<u64>>,
    next_id: u64,
    stats: ShmAppStats,
}

impl ShmAppManager {
    pub fn new() -> Self {
        Self {
            regions: BTreeMap::new(),
            app_regions: BTreeMap::new(),
            next_id: 1,
            stats: ShmAppStats::default(),
        }
    }

    pub fn create_region(
        &mut self,
        owner: u64,
        size: u64,
        mode: ShmAccessMode,
        now: u64,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let mut region = SharedRegion::new(id, owner, size, mode, now);
        region.attach(owner);

        self.regions.insert(id, region);
        self.app_regions.entry(owner).or_insert_with(Vec::new).push(id);
        self.stats.regions_created += 1;
        id
    }

    #[inline]
    pub fn attach(&mut self, region_id: u64, app_id: u64) -> bool {
        if let Some(region) = self.regions.get_mut(&region_id) {
            region.attach(app_id);
            self.app_regions.entry(app_id).or_insert_with(Vec::new).push(region_id);
            self.stats.total_attachments += 1;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn detach(&mut self, region_id: u64, app_id: u64) {
        if let Some(region) = self.regions.get_mut(&region_id) {
            region.detach(app_id);
        }
        if let Some(ids) = self.app_regions.get_mut(&app_id) {
            ids.retain(|&id| id != region_id);
        }
    }

    #[inline]
    pub fn record_cow_event(&mut self, region_id: u64) {
        if let Some(region) = self.regions.get_mut(&region_id) {
            region.record_cow();
            self.stats.total_cow_events += 1;
        }
    }

    #[inline]
    pub fn record_dedup(&mut self, region_id: u64, pages: u64) {
        if let Some(region) = self.regions.get_mut(&region_id) {
            region.record_dedup(pages);
            self.stats.total_dedup_pages += pages;
            self.stats.total_memory_saved += pages * 4096;
        }
    }

    pub fn cleanup_orphans(&mut self) -> u64 {
        let orphan_ids: Vec<u64> = self
            .regions
            .iter()
            .filter(|(_, r)| r.is_orphan())
            .map(|(&id, _)| id)
            .collect();

        let count = orphan_ids.len() as u64;
        for id in orphan_ids {
            self.regions.remove(&id);
            self.stats.regions_destroyed += 1;
        }
        count
    }

    pub fn app_profile(&self, app_id: u64) -> AppShmProfile {
        let region_ids = self.app_regions.get(&app_id);
        let mut owned = 0u64;
        let mut attached = 0u64;
        let mut total_bytes = 0u64;
        let mut total_reads = 0u64;
        let mut total_writes = 0u64;

        if let Some(ids) = region_ids {
            for &rid in ids {
                if let Some(region) = self.regions.get(&rid) {
                    if region.owner_app == app_id {
                        owned += 1;
                    } else {
                        attached += 1;
                    }
                    total_bytes += region.size_bytes;
                    total_reads += region.total_reads;
                    total_writes += region.total_writes;
                }
            }
        }

        let rw_ratio = if total_writes == 0 {
            f64::MAX
        } else {
            total_reads as f64 / total_writes as f64
        };

        AppShmProfile {
            app_id,
            regions_owned: owned,
            regions_attached: attached,
            total_shared_bytes: total_bytes,
            read_write_ratio: rw_ratio,
            avg_region_lifetime: 0,
        }
    }

    #[inline(always)]
    pub fn total_memory_saved(&self) -> u64 {
        self.regions.values().map(|r| r.memory_saved_bytes()).sum()
    }

    #[inline(always)]
    pub fn stats(&self) -> &ShmAppStats {
        &self.stats
    }
}
