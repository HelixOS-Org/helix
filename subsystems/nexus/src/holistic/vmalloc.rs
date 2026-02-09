// SPDX-License-Identifier: GPL-2.0
//! Holistic vmalloc — virtual memory allocation management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Vmalloc area type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmallocAreaType {
    Normal,
    DMA,
    DMA32,
    Ioremap,
    VMap,
    UserMap,
}

/// Vmalloc area
#[derive(Debug)]
pub struct VmallocArea {
    pub addr: u64,
    pub size: u64,
    pub area_type: VmallocAreaType,
    pub pages: u32,
    pub phys_addr: u64,
    pub caller_hash: u64,
    pub allocated_at: u64,
    pub flags: u32,
}

impl VmallocArea {
    pub fn new(addr: u64, size: u64, atype: VmallocAreaType, caller: u64, now: u64) -> Self {
        let pages = ((size + 4095) / 4096) as u32;
        Self { addr, size, area_type: atype, pages, phys_addr: 0, caller_hash: caller, allocated_at: now, flags: 0 }
    }
}

/// Vmalloc free hole
#[derive(Debug)]
pub struct VmallocHole {
    pub addr: u64,
    pub size: u64,
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct VmallocStats {
    pub total_areas: u32,
    pub total_pages: u64,
    pub total_bytes: u64,
    pub largest_free_hole: u64,
    pub total_allocs: u64,
    pub total_frees: u64,
    pub fragmentation: f64,
}

/// Main holistic vmalloc manager
pub struct HolisticVmalloc {
    areas: BTreeMap<u64, VmallocArea>,
    free_holes: Vec<VmallocHole>,
    total_allocs: u64,
    total_frees: u64,
    total_space: u64,
}

impl HolisticVmalloc {
    pub fn new(space: u64) -> Self {
        let holes = alloc::vec![VmallocHole { addr: 0, size: space }];
        Self { areas: BTreeMap::new(), free_holes: holes, total_allocs: 0, total_frees: 0, total_space: space }
    }

    pub fn alloc(&mut self, size: u64, atype: VmallocAreaType, caller: u64, now: u64) -> Option<u64> {
        let aligned = (size + 4095) & !4095;
        for i in 0..self.free_holes.len() {
            if self.free_holes[i].size >= aligned {
                let addr = self.free_holes[i].addr;
                self.free_holes[i].addr += aligned;
                self.free_holes[i].size -= aligned;
                if self.free_holes[i].size == 0 { self.free_holes.remove(i); }
                self.areas.insert(addr, VmallocArea::new(addr, aligned, atype, caller, now));
                self.total_allocs += 1;
                return Some(addr);
            }
        }
        None
    }

    #[inline]
    pub fn free(&mut self, addr: u64) {
        if let Some(area) = self.areas.remove(&addr) {
            self.free_holes.push(VmallocHole { addr: area.addr, size: area.size });
            self.total_frees += 1;
            self.coalesce();
        }
    }

    fn coalesce(&mut self) {
        self.free_holes.sort_by_key(|h| h.addr);
        let mut i = 0;
        while i + 1 < self.free_holes.len() {
            if self.free_holes[i].addr + self.free_holes[i].size == self.free_holes[i + 1].addr {
                self.free_holes[i].size += self.free_holes[i + 1].size;
                self.free_holes.remove(i + 1);
            } else { i += 1; }
        }
    }

    #[inline]
    pub fn stats(&self) -> VmallocStats {
        let pages: u64 = self.areas.values().map(|a| a.pages as u64).sum();
        let bytes: u64 = self.areas.values().map(|a| a.size).sum();
        let largest = self.free_holes.iter().map(|h| h.size).max().unwrap_or(0);
        let free_total: u64 = self.free_holes.iter().map(|h| h.size).sum();
        let frag = if free_total == 0 { 0.0 }
            else { 1.0 - (largest as f64 / free_total as f64) };
        VmallocStats { total_areas: self.areas.len() as u32, total_pages: pages, total_bytes: bytes, largest_free_hole: largest, total_allocs: self.total_allocs, total_frees: self.total_frees, fragmentation: frag }
    }
}
