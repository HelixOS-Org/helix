// SPDX-License-Identifier: GPL-2.0
//! Holistic freelist — free page list management.

extern crate alloc;

use alloc::vec::Vec;

/// Freelist type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreelistType {
    Normal,
    Movable,
    Unmovable,
    Reclaimable,
    HighAtomic,
}

/// Free page entry
#[derive(Debug)]
pub struct FreePageEntry {
    pub pfn: u64,
    pub order: u32,
    pub list_type: FreelistType,
}

/// Free list for a specific order
#[derive(Debug)]
pub struct Freelist {
    pub order: u32,
    pub list_type: FreelistType,
    pub pages: Vec<u64>,
    pub nr_free: u64,
    pub total_alloc: u64,
    pub total_free: u64,
}

impl Freelist {
    pub fn new(order: u32, ltype: FreelistType) -> Self {
        Self { order, list_type: ltype, pages: Vec::new(), nr_free: 0, total_alloc: 0, total_free: 0 }
    }

    #[inline]
    pub fn add_page(&mut self, pfn: u64) {
        self.pages.push(pfn);
        self.nr_free += 1;
        self.total_free += 1;
    }

    #[inline]
    pub fn remove_page(&mut self) -> Option<u64> {
        if let Some(pfn) = self.pages.pop() {
            self.nr_free -= 1;
            self.total_alloc += 1;
            Some(pfn)
        } else { None }
    }

    #[inline(always)]
    pub fn count(&self) -> u64 { self.nr_free }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FreelistStats {
    pub total_lists: u32,
    pub total_free_pages: u64,
    pub total_allocs: u64,
    pub total_frees: u64,
}

/// Main holistic freelist manager
pub struct HolisticFreelist {
    lists: Vec<Freelist>,
}

impl HolisticFreelist {
    pub fn new() -> Self { Self { lists: Vec::new() } }

    #[inline]
    pub fn create(&mut self, order: u32, ltype: FreelistType) -> usize {
        let idx = self.lists.len();
        self.lists.push(Freelist::new(order, ltype));
        idx
    }

    #[inline(always)]
    pub fn add_page(&mut self, list: usize, pfn: u64) {
        if list < self.lists.len() { self.lists[list].add_page(pfn); }
    }

    #[inline(always)]
    pub fn remove_page(&mut self, list: usize) -> Option<u64> {
        if list < self.lists.len() { self.lists[list].remove_page() } else { None }
    }

    #[inline]
    pub fn stats(&self) -> FreelistStats {
        let free: u64 = self.lists.iter().map(|l| l.nr_free).sum();
        let allocs: u64 = self.lists.iter().map(|l| l.total_alloc).sum();
        let frees: u64 = self.lists.iter().map(|l| l.total_free).sum();
        FreelistStats { total_lists: self.lists.len() as u32, total_free_pages: free, total_allocs: allocs, total_frees: frees }
    }
}
