//! # Holistic Deduplication Engine
//!
//! System-wide memory deduplication (KSM-like):
//! - Content-based page fingerprinting
//! - Copy-on-write merge
//! - Cross-process dedup detection
//! - Per-NUMA dedup pools
//! - Dedup benefit/cost analysis

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// DEDUP TYPES
// ============================================================================

/// Dedup state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DedupState {
    /// Page is unique
    Unique,
    /// Page has potential duplicates
    Candidate,
    /// Page is merged (COW)
    Merged,
    /// Page was unmerged (COW break)
    Unmerged,
}

/// Scan priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScanPriority {
    /// Idle scanning
    Idle,
    /// Background
    Background,
    /// Normal
    Normal,
    /// Aggressive
    Aggressive,
}

// ============================================================================
// PAGE FINGERPRINT
// ============================================================================

/// Page fingerprint for content matching
#[derive(Debug, Clone)]
pub struct PageFingerprint {
    /// Page frame number
    pub pfn: u64,
    /// Owner process
    pub owner: u64,
    /// Content hash (full page)
    pub full_hash: u64,
    /// Partial hash (first 256 bytes, for quick compare)
    pub partial_hash: u64,
    /// Last scan time
    pub last_scan: u64,
    /// State
    pub state: DedupState,
    /// Merge group id (if merged)
    pub merge_group: Option<u64>,
}

impl PageFingerprint {
    pub fn new(pfn: u64, owner: u64, content_hash: u64) -> Self {
        Self {
            pfn,
            owner,
            full_hash: content_hash,
            partial_hash: content_hash >> 32,
            last_scan: 0,
            state: DedupState::Unique,
            merge_group: None,
        }
    }
}

// ============================================================================
// MERGE GROUP
// ============================================================================

/// Merge group (pages with identical content)
#[derive(Debug)]
pub struct MergeGroup {
    /// Group id
    pub id: u64,
    /// Content hash
    pub content_hash: u64,
    /// Canonical PFN (the shared page)
    pub canonical_pfn: u64,
    /// Merged page entries: owner pid -> pfn
    pub members: BTreeMap<u64, u64>,
    /// NUMA node of canonical
    pub numa_node: u32,
    /// Creation time
    pub created_at: u64,
    /// COW breaks
    pub cow_breaks: u64,
}

impl MergeGroup {
    pub fn new(id: u64, content_hash: u64, canonical_pfn: u64, numa: u32, now: u64) -> Self {
        Self {
            id,
            content_hash,
            canonical_pfn,
            members: BTreeMap::new(),
            numa_node: numa,
            created_at: now,
            cow_breaks: 0,
        }
    }

    /// Add member
    pub fn add_member(&mut self, pid: u64, pfn: u64) {
        self.members.insert(pid, pfn);
    }

    /// Remove member (COW break)
    pub fn remove_member(&mut self, pid: u64) -> bool {
        if self.members.remove(&pid).is_some() {
            self.cow_breaks += 1;
            true
        } else {
            false
        }
    }

    /// Pages saved
    pub fn pages_saved(&self) -> u64 {
        if self.members.is_empty() {
            0
        } else {
            self.members.len() as u64 - 1
        }
    }

    /// Is stable? (no recent COW breaks)
    pub fn is_stable(&self) -> bool {
        self.cow_breaks == 0 || self.members.len() > self.cow_breaks as usize
    }
}

// ============================================================================
// SCANNER
// ============================================================================

/// Scan statistics
#[derive(Debug, Clone, Default)]
pub struct ScanStats {
    /// Pages scanned
    pub pages_scanned: u64,
    /// Duplicates found
    pub duplicates_found: u64,
    /// Pages merged
    pub pages_merged: u64,
    /// COW breaks
    pub cow_breaks: u64,
    /// Scan duration (ns)
    pub scan_duration_ns: u64,
}

/// Dedup scanner
#[derive(Debug)]
pub struct DedupScanner {
    /// Scan priority
    pub priority: ScanPriority,
    /// Pages per scan batch
    pub batch_size: usize,
    /// Scan interval (ns)
    pub interval_ns: u64,
    /// Last scan time
    pub last_scan: u64,
    /// Total scans
    pub total_scans: u64,
    /// Stats
    pub stats: ScanStats,
}

impl DedupScanner {
    pub fn new() -> Self {
        Self {
            priority: ScanPriority::Background,
            batch_size: 256,
            interval_ns: 100_000_000, // 100ms
            last_scan: 0,
            total_scans: 0,
            stats: ScanStats::default(),
        }
    }

    /// Should scan now?
    pub fn should_scan(&self, now: u64) -> bool {
        now.saturating_sub(self.last_scan) >= self.interval_ns
    }

    /// Record scan
    pub fn record_scan(&mut self, scanned: u64, found: u64, merged: u64, duration_ns: u64) {
        self.stats.pages_scanned += scanned;
        self.stats.duplicates_found += found;
        self.stats.pages_merged += merged;
        self.stats.scan_duration_ns += duration_ns;
        self.total_scans += 1;
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Dedup engine stats
#[derive(Debug, Clone, Default)]
pub struct HolisticDedupStats {
    /// Tracked pages
    pub tracked_pages: usize,
    /// Merge groups
    pub merge_groups: usize,
    /// Pages saved
    pub pages_saved: u64,
    /// Memory saved (bytes, 4K pages)
    pub memory_saved_bytes: u64,
    /// Total COW breaks
    pub total_cow_breaks: u64,
    /// Dedup ratio
    pub dedup_ratio: f64,
}

/// Holistic deduplication engine
pub struct HolisticDedupEngine {
    /// Page fingerprints: pfn -> fingerprint
    pages: BTreeMap<u64, PageFingerprint>,
    /// Hash index: content_hash -> group_id
    hash_index: BTreeMap<u64, u64>,
    /// Merge groups
    groups: BTreeMap<u64, MergeGroup>,
    /// Scanner
    pub scanner: DedupScanner,
    /// Next group id
    next_group_id: u64,
    /// Stats
    stats: HolisticDedupStats,
}

impl HolisticDedupEngine {
    pub fn new() -> Self {
        Self {
            pages: BTreeMap::new(),
            hash_index: BTreeMap::new(),
            groups: BTreeMap::new(),
            scanner: DedupScanner::new(),
            next_group_id: 1,
            stats: HolisticDedupStats::default(),
        }
    }

    /// Register page for dedup scanning
    pub fn register_page(&mut self, pfn: u64, owner: u64, content_hash: u64) {
        let fp = PageFingerprint::new(pfn, owner, content_hash);
        self.pages.insert(pfn, fp);
    }

    /// Scan for duplicates
    pub fn scan(&mut self, now: u64) -> u64 {
        let mut merged = 0u64;

        let pfns: Vec<u64> = self.pages.keys().copied()
            .take(self.scanner.batch_size)
            .collect();

        let mut new_merges: Vec<(u64, u64, u64, u64)> = Vec::new(); // (pfn, owner, hash, group_id)

        for &pfn in &pfns {
            if let Some(fp) = self.pages.get(&pfn) {
                if fp.state == DedupState::Merged {
                    continue;
                }
                let hash = fp.full_hash;
                let owner = fp.owner;

                if let Some(&group_id) = self.hash_index.get(&hash) {
                    new_merges.push((pfn, owner, hash, group_id));
                    merged += 1;
                }
            }
        }

        for (pfn, owner, hash, group_id) in new_merges {
            if let Some(group) = self.groups.get_mut(&group_id) {
                group.add_member(owner, pfn);
            }
            if let Some(fp) = self.pages.get_mut(&pfn) {
                fp.state = DedupState::Merged;
                fp.merge_group = Some(group_id);
                fp.last_scan = now;
            }
        }

        // Find new duplicate pairs among unmerged
        let unmerged: Vec<(u64, u64, u64)> = self.pages.values()
            .filter(|fp| fp.state == DedupState::Unique)
            .take(self.scanner.batch_size)
            .map(|fp| (fp.pfn, fp.owner, fp.full_hash))
            .collect();

        let mut hash_groups: BTreeMap<u64, Vec<(u64, u64)>> = BTreeMap::new();
        for &(pfn, owner, hash) in &unmerged {
            hash_groups.entry(hash).or_insert_with(Vec::new).push((pfn, owner));
        }

        for (hash, group) in &hash_groups {
            if group.len() < 2 {
                continue;
            }
            if self.hash_index.contains_key(hash) {
                continue;
            }
            // Create new merge group
            let gid = self.next_group_id;
            self.next_group_id += 1;
            let canonical = group[0].0;
            let mut mg = MergeGroup::new(gid, *hash, canonical, 0, now);
            for &(pfn, owner) in group {
                mg.add_member(owner, pfn);
                if let Some(fp) = self.pages.get_mut(&pfn) {
                    fp.state = DedupState::Merged;
                    fp.merge_group = Some(gid);
                    fp.last_scan = now;
                }
                merged += 1;
            }
            self.hash_index.insert(*hash, gid);
            self.groups.insert(gid, mg);
        }

        self.scanner.last_scan = now;
        self.scanner.record_scan(pfns.len() as u64 + unmerged.len() as u64, merged, merged, 0);
        self.update_stats();
        merged
    }

    /// COW break (page was written)
    pub fn cow_break(&mut self, pfn: u64) {
        if let Some(fp) = self.pages.get_mut(&pfn) {
            if let Some(gid) = fp.merge_group.take() {
                fp.state = DedupState::Unmerged;
                if let Some(group) = self.groups.get_mut(&gid) {
                    group.remove_member(fp.owner);
                }
            }
        }
        self.update_stats();
    }

    /// Remove page
    pub fn remove_page(&mut self, pfn: u64) {
        if let Some(fp) = self.pages.remove(&pfn) {
            if let Some(gid) = fp.merge_group {
                if let Some(group) = self.groups.get_mut(&gid) {
                    group.remove_member(fp.owner);
                }
            }
        }
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.tracked_pages = self.pages.len();
        self.stats.merge_groups = self.groups.len();
        self.stats.pages_saved = self.groups.values().map(|g| g.pages_saved()).sum();
        self.stats.memory_saved_bytes = self.stats.pages_saved * 4096;
        self.stats.total_cow_breaks = self.groups.values().map(|g| g.cow_breaks).sum();
        if self.stats.tracked_pages > 0 {
            self.stats.dedup_ratio = self.stats.pages_saved as f64 / self.stats.tracked_pages as f64;
        }
    }

    /// Stats
    pub fn stats(&self) -> &HolisticDedupStats {
        &self.stats
    }
}
