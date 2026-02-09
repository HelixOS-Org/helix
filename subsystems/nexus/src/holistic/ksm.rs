// SPDX-License-Identifier: GPL-2.0
//! Holistic ksm — Kernel Same-page Merging.

extern crate alloc;

use alloc::collections::BTreeMap;

/// KSM page state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KsmPageState {
    Unmerged,
    StableTree,
    UnstableTree,
    Volatile,
}

/// KSM page entry
#[derive(Debug)]
pub struct KsmPage {
    pub pfn: u64,
    pub checksum: u64,
    pub state: KsmPageState,
    pub merge_count: u32,
    pub rmap_count: u32,
}

impl KsmPage {
    pub fn new(pfn: u64, checksum: u64) -> Self {
        Self { pfn, checksum, state: KsmPageState::Unmerged, merge_count: 0, rmap_count: 1 }
    }
}

/// KSM scan info
#[derive(Debug)]
pub struct KsmScanInfo {
    pub pages_to_scan: u32,
    pub pages_scanned: u64,
    pub pages_shared: u64,
    pub pages_sharing: u64,
    pub pages_unshared: u64,
    pub full_scans: u64,
    pub sleep_ms: u32,
}

impl KsmScanInfo {
    pub fn new(pages_to_scan: u32, sleep: u32) -> Self {
        Self { pages_to_scan, pages_scanned: 0, pages_shared: 0, pages_sharing: 0, pages_unshared: 0, full_scans: 0, sleep_ms: sleep }
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct KsmStats {
    pub pages_shared: u64,
    pub pages_sharing: u64,
    pub pages_unshared: u64,
    pub pages_scanned: u64,
    pub memory_saved_pages: u64,
}

/// Main holistic KSM
pub struct HolisticKsm {
    stable_tree: BTreeMap<u64, KsmPage>,
    scan: KsmScanInfo,
}

impl HolisticKsm {
    pub fn new(pages_to_scan: u32, sleep: u32) -> Self {
        Self { stable_tree: BTreeMap::new(), scan: KsmScanInfo::new(pages_to_scan, sleep) }
    }

    #[inline]
    pub fn scan_page(&mut self, pfn: u64, checksum: u64) {
        self.scan.pages_scanned += 1;
        if let Some(existing) = self.stable_tree.get_mut(&checksum) {
            existing.merge_count += 1;
            existing.rmap_count += 1;
            self.scan.pages_sharing += 1;
        } else {
            self.stable_tree.insert(checksum, KsmPage::new(pfn, checksum));
            self.scan.pages_shared += 1;
        }
    }

    #[inline]
    pub fn unmerge(&mut self, checksum: u64) {
        if let Some(p) = self.stable_tree.get_mut(&checksum) {
            if p.rmap_count > 1 { p.rmap_count -= 1; self.scan.pages_sharing -= 1; }
            else { self.stable_tree.remove(&checksum); self.scan.pages_shared -= 1; }
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> KsmStats {
        let saved = self.scan.pages_sharing;
        KsmStats { pages_shared: self.scan.pages_shared, pages_sharing: self.scan.pages_sharing, pages_unshared: self.scan.pages_unshared, pages_scanned: self.scan.pages_scanned, memory_saved_pages: saved }
    }
}
