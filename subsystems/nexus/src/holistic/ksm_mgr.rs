// SPDX-License-Identifier: GPL-2.0
//! Holistic ksm_mgr — kernel samepage merging manager.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// KSM page state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KsmPageState {
    Unscanned,
    Unstable,
    Stable,
    Merged,
}

/// KSM page
#[derive(Debug)]
pub struct KsmPage {
    pub pfn: u64,
    pub hash: u64,
    pub state: KsmPageState,
    pub merge_count: u32,
    pub scan_count: u32,
}

impl KsmPage {
    pub fn new(pfn: u64, hash: u64) -> Self {
        Self { pfn, hash, state: KsmPageState::Unscanned, merge_count: 0, scan_count: 0 }
    }
}

/// Stable tree node (pages that have been merged)
#[derive(Debug)]
pub struct StableNode {
    pub hash: u64,
    pub base_pfn: u64,
    pub merged_count: u32,
}

/// Stats
#[derive(Debug, Clone)]
pub struct KsmMgrStats {
    pub total_pages: u32,
    pub stable_nodes: u32,
    pub pages_shared: u64,
    pub pages_sharing: u64,
    pub pages_unshared: u32,
    pub full_scans: u64,
    pub bytes_saved: u64,
}

/// Main holistic KSM manager
pub struct HolisticKsmMgr {
    pages: Vec<KsmPage>,
    stable_tree: BTreeMap<u64, StableNode>,
    full_scans: u64,
    page_size: u64,
}

impl HolisticKsmMgr {
    pub fn new(page_size: u64) -> Self { Self { pages: Vec::new(), stable_tree: BTreeMap::new(), full_scans: 0, page_size } }

    pub fn register_page(&mut self, pfn: u64, hash: u64) { self.pages.push(KsmPage::new(pfn, hash)); }

    pub fn scan(&mut self) {
        self.full_scans += 1;
        for page in self.pages.iter_mut() {
            page.scan_count += 1;
            if let Some(node) = self.stable_tree.get_mut(&page.hash) {
                page.state = KsmPageState::Merged;
                page.merge_count += 1;
                node.merged_count += 1;
            } else {
                page.state = KsmPageState::Unstable;
            }
        }
        // promote unstable to stable
        let mut hash_counts: BTreeMap<u64, u32> = BTreeMap::new();
        for p in self.pages.iter().filter(|p| p.state == KsmPageState::Unstable) {
            *hash_counts.entry(p.hash).or_insert(0) += 1;
        }
        for (hash, count) in hash_counts {
            if count >= 2 && !self.stable_tree.contains_key(&hash) {
                if let Some(base) = self.pages.iter().find(|p| p.hash == hash) {
                    self.stable_tree.insert(hash, StableNode { hash, base_pfn: base.pfn, merged_count: count });
                }
            }
        }
    }

    pub fn stats(&self) -> KsmMgrStats {
        let sharing: u64 = self.stable_tree.values().map(|n| n.merged_count as u64).sum();
        let shared = self.stable_tree.len() as u64;
        let unshared = self.pages.iter().filter(|p| p.state == KsmPageState::Unstable).count() as u32;
        let saved = sharing.saturating_sub(shared) * self.page_size;
        KsmMgrStats { total_pages: self.pages.len() as u32, stable_nodes: self.stable_tree.len() as u32, pages_shared: shared, pages_sharing: sharing, pages_unshared: unshared, full_scans: self.full_scans, bytes_saved: saved }
    }
}
