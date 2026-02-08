// SPDX-License-Identifier: GPL-2.0
//! Holistic huge_page_alloc — huge page allocation and management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Huge page size
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HugePageAllocSize {
    Size2M,
    Size1G,
    Size16K,
    Size64K,
    Size512M,
}

impl HugePageAllocSize {
    pub fn bytes(&self) -> u64 {
        match self {
            Self::Size16K => 16384,
            Self::Size64K => 65536,
            Self::Size2M => 2 * 1024 * 1024,
            Self::Size512M => 512 * 1024 * 1024,
            Self::Size1G => 1024 * 1024 * 1024,
        }
    }
}

/// Huge page entry
#[derive(Debug)]
pub struct HugePageEntry {
    pub pfn: u64,
    pub size: HugePageAllocSize,
    pub owner_pid: u64,
    pub allocated_at: u64,
    pub compound_order: u8,
    pub refcount: u32,
}

/// Huge page pool
#[derive(Debug)]
pub struct HugePageAllocPool {
    pub size: HugePageAllocSize,
    pub total: u32,
    pub free: u32,
    pub reserved: u32,
    pub surplus: u32,
    pub max_surplus: u32,
    pub pages: Vec<HugePageEntry>,
    pub alloc_failures: u64,
    pub total_allocated: u64,
    pub total_freed: u64,
}

impl HugePageAllocPool {
    pub fn new(size: HugePageAllocSize, total: u32) -> Self {
        Self { size, total, free: total, reserved: 0, surplus: 0, max_surplus: 0, pages: Vec::new(), alloc_failures: 0, total_allocated: 0, total_freed: 0 }
    }

    pub fn allocate(&mut self, pfn: u64, pid: u64, now: u64) -> bool {
        if self.free == 0 { self.alloc_failures += 1; return false; }
        self.free -= 1;
        self.total_allocated += 1;
        self.pages.push(HugePageEntry { pfn, size: self.size, owner_pid: pid, allocated_at: now, compound_order: 0, refcount: 1 });
        true
    }

    pub fn free_page(&mut self, pfn: u64) {
        if let Some(pos) = self.pages.iter().position(|p| p.pfn == pfn) {
            self.pages.remove(pos);
            self.free += 1;
            self.total_freed += 1;
        }
    }

    pub fn utilization(&self) -> f64 {
        if self.total == 0 { return 0.0; }
        (self.total - self.free) as f64 / self.total as f64
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct HugePageAllocStats {
    pub pools: u32,
    pub total_pages: u32,
    pub total_free: u32,
    pub total_allocated: u64,
    pub total_failures: u64,
}

/// Main holistic huge page allocator
pub struct HolisticHugePageAlloc {
    pools: BTreeMap<u8, HugePageAllocPool>,
}

impl HolisticHugePageAlloc {
    pub fn new() -> Self { Self { pools: BTreeMap::new() } }

    pub fn create_pool(&mut self, size: HugePageAllocSize, total: u32) {
        let key = size as u8;
        self.pools.insert(key, HugePageAllocPool::new(size, total));
    }

    pub fn allocate(&mut self, size: HugePageAllocSize, pfn: u64, pid: u64, now: u64) -> bool {
        let key = size as u8;
        if let Some(p) = self.pools.get_mut(&key) { p.allocate(pfn, pid, now) }
        else { false }
    }

    pub fn free_page(&mut self, size: HugePageAllocSize, pfn: u64) {
        let key = size as u8;
        if let Some(p) = self.pools.get_mut(&key) { p.free_page(pfn); }
    }

    pub fn stats(&self) -> HugePageAllocStats {
        let total: u32 = self.pools.values().map(|p| p.total).sum();
        let free: u32 = self.pools.values().map(|p| p.free).sum();
        let allocated: u64 = self.pools.values().map(|p| p.total_allocated).sum();
        let failures: u64 = self.pools.values().map(|p| p.alloc_failures).sum();
        HugePageAllocStats { pools: self.pools.len() as u32, total_pages: total, total_free: free, total_allocated: allocated, total_failures: failures }
    }
}
