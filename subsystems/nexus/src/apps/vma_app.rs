// SPDX-License-Identifier: MIT
//! # VMA Application Manager
//!
//! Virtual Memory Area management at the application layer:
//! - Per-app VMA tree with interval tracking
//! - Access permission audit trail
//! - Fragmentation scoring and defrag hints
//! - VMA merge/split cost estimation
//! - Guard page enforcement metrics

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VmaPermission { None, ReadOnly, ReadWrite, ReadExec, ReadWriteExec }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmaType { Anonymous, FileBacked, Shared, Stack, Heap, Mmap }

#[derive(Debug, Clone)]
pub struct VmaEntry {
    pub start: u64,
    pub end: u64,
    pub perm: VmaPermission,
    pub vma_type: VmaType,
    pub fault_count: u64,
    pub resident_pages: u64,
    pub dirty_pages: u64,
    pub guard_pages: u32,
    pub created_at: u64,
}

impl VmaEntry {
    #[inline(always)]
    pub fn size(&self) -> u64 { self.end.saturating_sub(self.start) }
    #[inline(always)]
    pub fn page_count(&self) -> u64 { self.size() / 4096 }
    #[inline(always)]
    pub fn residency_ratio(&self) -> f64 {
        if self.page_count() == 0 { return 0.0; }
        self.resident_pages as f64 / self.page_count() as f64
    }
    #[inline(always)]
    pub fn dirty_ratio(&self) -> f64 {
        if self.resident_pages == 0 { return 0.0; }
        self.dirty_pages as f64 / self.resident_pages as f64
    }
}

#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct VmaAppStats {
    pub total_vmas: u64,
    pub total_mapped_bytes: u64,
    pub total_resident_bytes: u64,
    pub fragmentation_score: f64,
    pub merge_opportunities: u64,
    pub guard_page_count: u64,
}

pub struct VmaAppManager {
    /// app_id → sorted VMA entries
    app_vmas: BTreeMap<u64, Vec<VmaEntry>>,
    /// app_id → permission change log: (timestamp, vma_start, old, new)
    perm_log: BTreeMap<u64, Vec<(u64, u64, VmaPermission, VmaPermission)>>,
    stats: VmaAppStats,
}

impl VmaAppManager {
    pub fn new() -> Self {
        Self {
            app_vmas: BTreeMap::new(),
            perm_log: BTreeMap::new(),
            stats: VmaAppStats::default(),
        }
    }

    #[inline]
    pub fn add_vma(&mut self, app_id: u64, entry: VmaEntry) {
        self.stats.total_mapped_bytes += entry.size();
        self.stats.total_resident_bytes += entry.resident_pages * 4096;
        self.stats.guard_page_count += entry.guard_pages as u64;
        self.stats.total_vmas += 1;

        let vmas = self.app_vmas.entry(app_id).or_insert_with(Vec::new);
        // Insert sorted by start address
        let pos = vmas.partition_point(|v| v.start < entry.start);
        vmas.insert(pos, entry);
    }

    #[inline]
    pub fn remove_vma(&mut self, app_id: u64, start: u64) -> Option<VmaEntry> {
        let vmas = self.app_vmas.get_mut(&app_id)?;
        let idx = vmas.iter().position(|v| v.start == start)?;
        let removed = vmas.remove(idx);
        self.stats.total_mapped_bytes = self.stats.total_mapped_bytes.saturating_sub(removed.size());
        self.stats.total_resident_bytes = self.stats.total_resident_bytes
            .saturating_sub(removed.resident_pages * 4096);
        self.stats.total_vmas = self.stats.total_vmas.saturating_sub(1);
        Some(removed)
    }

    /// Compute fragmentation score: ratio of gaps to total address range
    pub fn fragmentation_score(&self, app_id: u64) -> f64 {
        let vmas = match self.app_vmas.get(&app_id) {
            Some(v) if v.len() >= 2 => v,
            _ => return 0.0,
        };

        let range_start = vmas.first().unwrap().start;
        let range_end = vmas.last().unwrap().end;
        let total_range = range_end.saturating_sub(range_start);
        if total_range == 0 { return 0.0; }

        let mut gap_bytes: u64 = 0;
        for i in 1..vmas.len() {
            let gap = vmas[i].start.saturating_sub(vmas[i - 1].end);
            gap_bytes += gap;
        }

        gap_bytes as f64 / total_range as f64
    }

    /// Find adjacent VMAs with same permissions that could be merged
    pub fn find_merge_opportunities(&self, app_id: u64) -> Vec<(u64, u64)> {
        let vmas = match self.app_vmas.get(&app_id) {
            Some(v) if v.len() >= 2 => v,
            _ => return Vec::new(),
        };

        let mut merges = Vec::new();
        for i in 1..vmas.len() {
            let prev = &vmas[i - 1];
            let curr = &vmas[i];
            if prev.end == curr.start
                && prev.perm == curr.perm
                && prev.vma_type == curr.vma_type
            {
                merges.push((prev.start, curr.end));
            }
        }
        merges
    }

    /// Record a permission change for audit
    pub fn record_perm_change(
        &mut self, app_id: u64, vma_start: u64,
        old: VmaPermission, new: VmaPermission, now: u64,
    ) {
        let log = self.perm_log.entry(app_id).or_insert_with(Vec::new);
        log.push((now, vma_start, old, new));
        if log.len() > 1024 { log.drain(..512); }

        // Update actual VMA
        if let Some(vmas) = self.app_vmas.get_mut(&app_id) {
            if let Some(vma) = vmas.iter_mut().find(|v| v.start == vma_start) {
                vma.perm = new;
            }
        }
    }

    /// Estimate cost of defragmenting an app's address space
    #[inline]
    pub fn defrag_cost_estimate(&self, app_id: u64) -> u64 {
        let vmas = match self.app_vmas.get(&app_id) {
            Some(v) => v,
            None => return 0,
        };
        // Cost = total dirty pages that need copying + TLB flush overhead
        let dirty_total: u64 = vmas.iter().map(|v| v.dirty_pages).sum();
        let tlb_cost = vmas.len() as u64 * 50; // ~50ns per TLB invalidation
        dirty_total * 100 + tlb_cost // 100ns per page copy
    }

    #[inline(always)]
    pub fn app_vmas(&self, app_id: u64) -> &[VmaEntry] {
        self.app_vmas.get(&app_id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    #[inline(always)]
    pub fn stats(&self) -> &VmaAppStats { &self.stats }
}
