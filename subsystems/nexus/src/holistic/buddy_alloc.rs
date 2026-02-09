// SPDX-License-Identifier: GPL-2.0
//! Holistic buddy_alloc — buddy system page allocator.

extern crate alloc;

use alloc::vec::Vec;

/// Buddy block state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuddyState {
    Free,
    Allocated,
    Split,
}

/// Page order (0 = 4KB, 1 = 8KB, ... 10 = 4MB)
pub const MAX_ORDER: usize = 11;

/// Free list entry
#[derive(Debug, Clone)]
pub struct FreeBlock {
    pub pfn: u64,
    pub order: u8,
}

/// Buddy zone
#[derive(Debug)]
pub struct BuddyZone {
    pub id: u32,
    pub start_pfn: u64,
    pub end_pfn: u64,
    pub free_lists: [Vec<FreeBlock>; MAX_ORDER],
    pub total_pages: u64,
    pub free_pages: u64,
    pub alloc_count: u64,
    pub free_count: u64,
    pub split_count: u64,
    pub merge_count: u64,
}

impl BuddyZone {
    pub fn new(id: u32, start: u64, end: u64) -> Self {
        Self {
            id, start_pfn: start, end_pfn: end,
            free_lists: core::array::from_fn(|_| Vec::new()),
            total_pages: end - start, free_pages: end - start,
            alloc_count: 0, free_count: 0, split_count: 0, merge_count: 0,
        }
    }

    #[inline(always)]
    pub fn add_free(&mut self, pfn: u64, order: u8) {
        if (order as usize) < MAX_ORDER { self.free_lists[order as usize].push(FreeBlock { pfn, order }); }
    }

    pub fn alloc(&mut self, order: u8) -> Option<u64> {
        for o in (order as usize)..MAX_ORDER {
            if let Some(block) = self.free_lists[o].pop() {
                let pfn = block.pfn;
                let mut current_order = o;
                while current_order > order as usize {
                    current_order -= 1;
                    let buddy_pfn = pfn + (1u64 << current_order);
                    self.free_lists[current_order].push(FreeBlock { pfn: buddy_pfn, order: current_order as u8 });
                    self.split_count += 1;
                }
                let pages = 1u64 << order;
                self.free_pages -= pages;
                self.alloc_count += 1;
                return Some(pfn);
            }
        }
        None
    }

    pub fn free(&mut self, pfn: u64, order: u8) {
        let pages = 1u64 << order;
        self.free_pages += pages;
        self.free_count += 1;
        let mut current_pfn = pfn;
        let mut current_order = order as usize;
        while current_order < MAX_ORDER - 1 {
            let buddy_pfn = current_pfn ^ (1u64 << current_order);
            if let Some(pos) = self.free_lists[current_order].iter().position(|b| b.pfn == buddy_pfn) {
                self.free_lists[current_order].remove(pos);
                current_pfn = current_pfn.min(buddy_pfn);
                current_order += 1;
                self.merge_count += 1;
            } else { break; }
        }
        self.free_lists[current_order].push(FreeBlock { pfn: current_pfn, order: current_order as u8 });
    }

    #[inline]
    pub fn fragmentation(&self) -> f64 {
        if self.free_pages == 0 { return 0.0; }
        let largest_free_order = (0..MAX_ORDER).rev().find(|&o| !self.free_lists[o].is_empty()).unwrap_or(0);
        1.0 - ((1u64 << largest_free_order) as f64 / self.free_pages as f64)
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BuddyAllocStats {
    pub total_zones: u32,
    pub total_pages: u64,
    pub free_pages: u64,
    pub alloc_count: u64,
    pub free_count: u64,
    pub avg_fragmentation: f64,
}

/// Main buddy allocator
pub struct HolisticBuddyAlloc {
    zones: Vec<BuddyZone>,
}

impl HolisticBuddyAlloc {
    pub fn new() -> Self { Self { zones: Vec::new() } }

    #[inline]
    pub fn add_zone(&mut self, start: u64, end: u64) -> u32 {
        let id = self.zones.len() as u32;
        self.zones.push(BuddyZone::new(id, start, end));
        id
    }

    #[inline]
    pub fn alloc(&mut self, order: u8) -> Option<u64> {
        for zone in &mut self.zones {
            if let Some(pfn) = zone.alloc(order) { return Some(pfn); }
        }
        None
    }

    #[inline]
    pub fn stats(&self) -> BuddyAllocStats {
        let total: u64 = self.zones.iter().map(|z| z.total_pages).sum();
        let free: u64 = self.zones.iter().map(|z| z.free_pages).sum();
        let allocs: u64 = self.zones.iter().map(|z| z.alloc_count).sum();
        let frees: u64 = self.zones.iter().map(|z| z.free_count).sum();
        let frags: Vec<f64> = self.zones.iter().map(|z| z.fragmentation()).collect();
        let avg = if frags.is_empty() { 0.0 } else { frags.iter().sum::<f64>() / frags.len() as f64 };
        BuddyAllocStats { total_zones: self.zones.len() as u32, total_pages: total, free_pages: free, alloc_count: allocs, free_count: frees, avg_fragmentation: avg }
    }
}

// ============================================================================
// Merged from buddy_alloc_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuddyV2Order {
    Order0,
    Order1,
    Order2,
    Order3,
    Order4,
    Order5,
    Order6,
    Order7,
    Order8,
    Order9,
    Order10,
}

impl BuddyV2Order {
    #[inline(always)]
    pub fn pages(&self) -> u64 { 1 << (*self as u32) }
    #[inline(always)]
    pub fn as_u32(&self) -> u32 { *self as u32 }
}

/// Free block
#[derive(Debug)]
pub struct BuddyV2Block {
    pub pfn: u64,
    pub order: u32,
}

/// Buddy zone v2
#[derive(Debug)]
pub struct BuddyV2Zone {
    pub free_lists: [Vec<u64>; 11],
    pub total_pages: u64,
    pub free_pages: u64,
    pub alloc_count: u64,
    pub free_count: u64,
    pub split_count: u64,
    pub merge_count: u64,
}

impl BuddyV2Zone {
    pub fn new(total: u64) -> Self {
        Self { free_lists: Default::default(), total_pages: total, free_pages: total, alloc_count: 0, free_count: 0, split_count: 0, merge_count: 0 }
    }

    pub fn alloc(&mut self, order: u32) -> Option<u64> {
        let ord = order as usize;
        if ord > 10 { return None; }
        // Try exact order first
        if !self.free_lists[ord].is_empty() {
            self.alloc_count += 1;
            self.free_pages -= 1 << order;
            return self.free_lists[ord].pop();
        }
        // Split from higher order
        for hi in (ord + 1)..=10 {
            if !self.free_lists[hi].is_empty() {
                let pfn = self.free_lists[hi].pop().unwrap();
                for split_ord in (ord..hi).rev() {
                    let buddy = pfn + (1 << split_ord);
                    self.free_lists[split_ord].push(buddy);
                    self.split_count += 1;
                }
                self.alloc_count += 1;
                self.free_pages -= 1 << order;
                return Some(pfn);
            }
        }
        None
    }

    #[inline]
    pub fn free(&mut self, pfn: u64, order: u32) {
        if order as usize > 10 { return; }
        self.free_lists[order as usize].push(pfn);
        self.free_pages += 1 << order;
        self.free_count += 1;
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BuddyAllocV2Stats {
    pub total_pages: u64,
    pub free_pages: u64,
    pub allocs: u64,
    pub frees: u64,
    pub splits: u64,
    pub merges: u64,
}

/// Main holistic buddy alloc v2
pub struct HolisticBuddyAllocV2 {
    zone: BuddyV2Zone,
}

impl HolisticBuddyAllocV2 {
    pub fn new(total: u64) -> Self { Self { zone: BuddyV2Zone::new(total) } }
    #[inline(always)]
    pub fn alloc(&mut self, order: u32) -> Option<u64> { self.zone.alloc(order) }
    #[inline(always)]
    pub fn free(&mut self, pfn: u64, order: u32) { self.zone.free(pfn, order) }

    #[inline(always)]
    pub fn stats(&self) -> BuddyAllocV2Stats {
        BuddyAllocV2Stats { total_pages: self.zone.total_pages, free_pages: self.zone.free_pages, allocs: self.zone.alloc_count, frees: self.zone.free_count, splits: self.zone.split_count, merges: self.zone.merge_count }
    }
}
