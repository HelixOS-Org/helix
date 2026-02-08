//! # Holistic KSM Deduplication
//!
//! Kernel Same-page Merging (KSM) deduplication management:
//! - Content-addressable page fingerprinting
//! - Stable/unstable tree management for merge candidates
//! - Copy-on-write tracking for shared pages
//! - Per-process dedup accounting
//! - Scan rate and sleep interval tuning
//! - Memory savings estimation and reporting

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Page fingerprint for content comparison
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PageFingerprint {
    pub hash_high: u64,
    pub hash_low: u64,
}

impl PageFingerprint {
    pub fn from_content(data: &[u8]) -> Self {
        let mut h1: u64 = 0xcbf29ce484222325;
        let mut h2: u64 = 0x6c62272e07bb0142;
        for &b in data {
            h1 ^= b as u64;
            h1 = h1.wrapping_mul(0x100000001b3);
            h2 ^= b as u64;
            h2 = h2.wrapping_mul(0x00000100000001b3);
        }
        Self { hash_high: h1, hash_low: h2 }
    }

    pub fn matches(&self, other: &Self) -> bool {
        self.hash_high == other.hash_high && self.hash_low == other.hash_low
    }
}

/// KSM page state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KsmPageState {
    Unscanned,
    Unstable,
    Stable,
    Shared,
    CowBroken,
}

/// KSM tracked page
#[derive(Debug, Clone)]
pub struct KsmPage {
    pub page_addr: u64,
    pub fingerprint: PageFingerprint,
    pub state: KsmPageState,
    pub share_count: u32,
    pub cow_breaks: u32,
    pub owner_process: u64,
    pub last_scan_ts: u64,
    pub stable_since: u64,
}

impl KsmPage {
    pub fn new(addr: u64, owner: u64) -> Self {
        Self {
            page_addr: addr, fingerprint: PageFingerprint { hash_high: 0, hash_low: 0 },
            state: KsmPageState::Unscanned, share_count: 1, cow_breaks: 0,
            owner_process: owner, last_scan_ts: 0, stable_since: 0,
        }
    }

    pub fn mark_stable(&mut self, ts: u64) {
        self.state = KsmPageState::Stable;
        self.stable_since = ts;
    }

    pub fn merge_with(&mut self) {
        self.state = KsmPageState::Shared;
        self.share_count += 1;
    }

    pub fn cow_break(&mut self) {
        self.cow_breaks += 1;
        self.share_count = self.share_count.saturating_sub(1);
        if self.share_count <= 1 {
            self.state = KsmPageState::CowBroken;
        }
    }

    pub fn savings_pages(&self) -> u32 {
        if self.share_count > 1 { self.share_count - 1 } else { 0 }
    }
}

/// Stable tree node
#[derive(Debug, Clone)]
pub struct StableTreeNode {
    pub fingerprint: PageFingerprint,
    pub representative_addr: u64,
    pub merged_pages: Vec<u64>,
    pub total_shares: u32,
}

impl StableTreeNode {
    pub fn new(fp: PageFingerprint, addr: u64) -> Self {
        Self { fingerprint: fp, representative_addr: addr, merged_pages: Vec::new(), total_shares: 1 }
    }

    pub fn add_page(&mut self, addr: u64) {
        self.merged_pages.push(addr);
        self.total_shares += 1;
    }

    pub fn savings(&self) -> u32 {
        if self.total_shares > 1 { self.total_shares - 1 } else { 0 }
    }
}

/// Per-process KSM accounting
#[derive(Debug, Clone)]
pub struct ProcessKsmInfo {
    pub process_id: u64,
    pub pages_scanned: u64,
    pub pages_shared: u64,
    pub pages_sharing: u64,
    pub cow_breaks: u64,
    pub savings_bytes: u64,
}

impl ProcessKsmInfo {
    pub fn new(pid: u64) -> Self {
        Self { process_id: pid, pages_scanned: 0, pages_shared: 0, pages_sharing: 0, cow_breaks: 0, savings_bytes: 0 }
    }
}

/// KSM scan configuration
#[derive(Debug, Clone)]
pub struct KsmScanConfig {
    pub pages_per_scan: u32,
    pub sleep_interval_ms: u64,
    pub max_merge_ratio: f64,
    pub cow_break_threshold: u32,
    pub adaptive_scan: bool,
    pub min_page_age_ms: u64,
}

impl Default for KsmScanConfig {
    fn default() -> Self {
        Self {
            pages_per_scan: 100, sleep_interval_ms: 200,
            max_merge_ratio: 0.5, cow_break_threshold: 10,
            adaptive_scan: true, min_page_age_ms: 1000,
        }
    }
}

/// KSM dedup stats
#[derive(Debug, Clone, Default)]
pub struct KsmDedupStats {
    pub total_pages_tracked: usize,
    pub stable_nodes: usize,
    pub unstable_pages: usize,
    pub shared_pages: u64,
    pub total_savings_pages: u64,
    pub total_cow_breaks: u64,
    pub scan_rounds: u64,
    pub merge_rate: f64,
    pub dedup_ratio: f64,
}

/// Holistic KSM deduplication manager
pub struct HolisticKsmDedup {
    pages: BTreeMap<u64, KsmPage>,
    stable_tree: BTreeMap<PageFingerprint, StableTreeNode>,
    unstable_pages: Vec<u64>,
    process_info: BTreeMap<u64, ProcessKsmInfo>,
    config: KsmScanConfig,
    stats: KsmDedupStats,
    scan_cursor: usize,
    current_ts: u64,
}

impl HolisticKsmDedup {
    pub fn new(config: KsmScanConfig) -> Self {
        Self {
            pages: BTreeMap::new(), stable_tree: BTreeMap::new(),
            unstable_pages: Vec::new(), process_info: BTreeMap::new(),
            config, stats: KsmDedupStats::default(),
            scan_cursor: 0, current_ts: 0,
        }
    }

    pub fn register_page(&mut self, addr: u64, owner: u64) {
        self.pages.insert(addr, KsmPage::new(addr, owner));
        self.process_info.entry(owner).or_insert_with(|| ProcessKsmInfo::new(owner));
    }

    pub fn scan_page(&mut self, addr: u64, content: &[u8], ts: u64) {
        self.current_ts = ts;
        let fp = PageFingerprint::from_content(content);
        if let Some(page) = self.pages.get_mut(&addr) {
            page.fingerprint = fp;
            page.last_scan_ts = ts;

            if let Some(pinfo) = self.process_info.get_mut(&page.owner_process) {
                pinfo.pages_scanned += 1;
            }

            if let Some(stable) = self.stable_tree.get_mut(&fp) {
                // Match in stable tree — merge
                page.merge_with();
                stable.add_page(addr);
                let owner = page.owner_process;
                if let Some(pinfo) = self.process_info.get_mut(&owner) {
                    pinfo.pages_sharing += 1;
                }
            } else {
                // Check unstable list for matching fingerprint
                let mut found_match = None;
                for (i, &uaddr) in self.unstable_pages.iter().enumerate() {
                    if let Some(upage) = self.pages.get(&uaddr) {
                        if upage.fingerprint.matches(&fp) && uaddr != addr {
                            found_match = Some((i, uaddr));
                            break;
                        }
                    }
                }

                if let Some((idx, match_addr)) = found_match {
                    // Promote both to stable
                    let mut node = StableTreeNode::new(fp, match_addr);
                    node.add_page(addr);
                    self.stable_tree.insert(fp, node);

                    if let Some(mp) = self.pages.get_mut(&match_addr) {
                        mp.mark_stable(ts);
                        mp.merge_with();
                    }
                    page.mark_stable(ts);
                    page.merge_with();
                    self.unstable_pages.swap_remove(idx);
                } else {
                    page.state = KsmPageState::Unstable;
                    self.unstable_pages.push(addr);
                }
            }
        }
    }

    pub fn record_cow_break(&mut self, addr: u64) {
        if let Some(page) = self.pages.get_mut(&addr) {
            page.cow_break();
            let owner = page.owner_process;
            if let Some(pinfo) = self.process_info.get_mut(&owner) {
                pinfo.cow_breaks += 1;
            }
            // Remove from stable tree if no longer shared
            if page.share_count <= 1 {
                self.stable_tree.remove(&page.fingerprint);
            }
        }
    }

    pub fn adapt_scan_rate(&mut self) {
        if !self.config.adaptive_scan { return; }
        let merge_rate = self.stats.merge_rate;
        if merge_rate > 0.3 {
            self.config.pages_per_scan = (self.config.pages_per_scan + 50).min(1000);
            self.config.sleep_interval_ms = (self.config.sleep_interval_ms.saturating_sub(50)).max(50);
        } else if merge_rate < 0.05 {
            self.config.pages_per_scan = (self.config.pages_per_scan.saturating_sub(20)).max(10);
            self.config.sleep_interval_ms = (self.config.sleep_interval_ms + 100).min(5000);
        }
    }

    pub fn recompute(&mut self) {
        self.stats.total_pages_tracked = self.pages.len();
        self.stats.stable_nodes = self.stable_tree.len();
        self.stats.unstable_pages = self.unstable_pages.len();
        let total_savings: u64 = self.stable_tree.values().map(|n| n.savings() as u64).sum();
        self.stats.total_savings_pages = total_savings;
        self.stats.shared_pages = self.pages.values().filter(|p| p.state == KsmPageState::Shared).count() as u64;
        self.stats.total_cow_breaks = self.pages.values().map(|p| p.cow_breaks as u64).sum();
        self.stats.scan_rounds += 1;
        let total = self.stats.total_pages_tracked as f64;
        if total > 0.0 {
            self.stats.merge_rate = self.stats.shared_pages as f64 / total;
            self.stats.dedup_ratio = total_savings as f64 / total;
        }
    }

    pub fn process_info(&self, pid: u64) -> Option<&ProcessKsmInfo> { self.process_info.get(&pid) }
    pub fn config(&self) -> &KsmScanConfig { &self.config }
    pub fn stats(&self) -> &KsmDedupStats { &self.stats }
}
