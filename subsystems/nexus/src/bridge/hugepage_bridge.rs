// SPDX-License-Identifier: GPL-2.0
//! NEXUS Bridge — Hugepage (huge page bridge)

extern crate alloc;
use alloc::collections::BTreeMap;

/// Huge page size
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeHugepageSize { TwoMB, OneGB }

/// Huge page allocation
#[derive(Debug, Clone)]
pub struct BridgeHugepageAlloc { pub addr: u64, pub size: BridgeHugepageSize, pub count: u32, pub transparent: bool }

/// Hugepage stats
#[derive(Debug, Clone)]
pub struct BridgeHugepageStats {
    pub total_allocs: u64, pub two_mb_pages: u64, pub one_gb_pages: u64, pub transparent_promotions: u64,
    pub total_huge_bytes: u64, pub defrag_attempts: u64,
}

/// Manager for hugepage bridge
pub struct BridgeHugepageManager {
    allocs: BTreeMap<u64, BridgeHugepageAlloc>,
    pool_2mb: u32, pool_1gb: u32,
    stats: BridgeHugepageStats,
}

impl BridgeHugepageManager {
    pub fn new(pool_2mb: u32, pool_1gb: u32) -> Self {
        Self { allocs: BTreeMap::new(), pool_2mb, pool_1gb, stats: BridgeHugepageStats {
            total_allocs: 0, two_mb_pages: 0, one_gb_pages: 0, transparent_promotions: 0, total_huge_bytes: 0, defrag_attempts: 0
        }}
    }

    pub fn alloc(&mut self, addr: u64, size: BridgeHugepageSize, count: u32) -> bool {
        let (page_bytes, pool) = match size {
            BridgeHugepageSize::TwoMB => (2 * 1024 * 1024u64, &mut self.pool_2mb),
            BridgeHugepageSize::OneGB => (1024 * 1024 * 1024u64, &mut self.pool_1gb),
        };
        if *pool < count { return false; }
        *pool -= count;
        self.allocs.insert(addr, BridgeHugepageAlloc { addr, size, count, transparent: false });
        self.stats.total_allocs += 1;
        match size { BridgeHugepageSize::TwoMB => self.stats.two_mb_pages += count as u64, BridgeHugepageSize::OneGB => self.stats.one_gb_pages += count as u64 }
        self.stats.total_huge_bytes += page_bytes * count as u64;
        true
    }

    pub fn promote_transparent(&mut self, addr: u64) { self.stats.transparent_promotions += 1; }
    pub fn stats(&self) -> &BridgeHugepageStats { &self.stats }
}
