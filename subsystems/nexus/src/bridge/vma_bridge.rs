// SPDX-License-Identifier: GPL-2.0
//! NEXUS Bridge — VMA (virtual memory area bridge)

extern crate alloc;
use alloc::collections::BTreeMap;

/// VMA type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeVmaType { Code, Data, Heap, Stack, SharedLib, Mmap, Vdso, Anonymous }

/// VMA flags
#[derive(Debug, Clone, Copy)]
#[repr(align(64))]
pub struct BridgeVmaFlags { pub read: bool, pub write: bool, pub exec: bool, pub shared: bool, pub growsdown: bool }

/// VMA entry
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BridgeVmaEntry { pub start: u64, pub end: u64, pub vma_type: BridgeVmaType, pub flags: BridgeVmaFlags, pub rss_pages: u32 }

/// VMA stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BridgeVmaStats { pub total_vmas: u64, pub total_virtual_bytes: u64, pub total_rss_pages: u64, pub code_vmas: u64, pub data_vmas: u64 }

/// Manager for VMA bridge
#[repr(align(64))]
pub struct BridgeVmaManager {
    vmas: BTreeMap<u64, BridgeVmaEntry>,
    stats: BridgeVmaStats,
}

impl BridgeVmaManager {
    pub fn new() -> Self {
        Self { vmas: BTreeMap::new(), stats: BridgeVmaStats { total_vmas: 0, total_virtual_bytes: 0, total_rss_pages: 0, code_vmas: 0, data_vmas: 0 } }
    }

    #[inline]
    pub fn add_vma(&mut self, start: u64, end: u64, vma_type: BridgeVmaType, flags: BridgeVmaFlags) {
        let entry = BridgeVmaEntry { start, end, vma_type, flags, rss_pages: 0 };
        self.vmas.insert(start, entry);
        self.stats.total_vmas += 1;
        self.stats.total_virtual_bytes += end - start;
        match vma_type { BridgeVmaType::Code => self.stats.code_vmas += 1, BridgeVmaType::Data | BridgeVmaType::Heap => self.stats.data_vmas += 1, _ => {} }
    }

    #[inline]
    pub fn fault_page(&mut self, addr: u64) {
        for (_, vma) in self.vmas.iter_mut() {
            if addr >= vma.start && addr < vma.end { vma.rss_pages += 1; self.stats.total_rss_pages += 1; break; }
        }
    }

    #[inline(always)]
    pub fn remove_vma(&mut self, start: u64) -> bool { self.vmas.remove(&start).is_some() }
    #[inline(always)]
    pub fn vma_count(&self) -> usize { self.vmas.len() }
    #[inline(always)]
    pub fn stats(&self) -> &BridgeVmaStats { &self.stats }
}
