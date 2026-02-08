//! # Holistic Page Cache Optimizer
//!
//! System-wide page cache management with holistic awareness:
//! - Working set estimation per process and cgroup
//! - Adaptive writeback tuning
//! - Dirty page throttling with fairness
//! - Cache partition hints for isolation
//! - Readahead feedback integration
//! - Thrashing detection and mitigation

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Cache eviction policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheEvictionPolicy {
    Lru,
    Lfu,
    Clock,
    Arc, // Adaptive Replacement Cache
    MultiGenLru,
}

/// Writeback mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WritebackMode {
    Background,
    Periodic,
    Forced,
    Throttled,
}

/// Per-inode cache state
#[derive(Debug, Clone)]
pub struct InodeCacheState {
    pub inode_id: u64,
    pub device_id: u32,
    pub cached_pages: u64,
    pub dirty_pages: u64,
    pub read_hits: u64,
    pub read_misses: u64,
    pub write_count: u64,
    pub last_access_ns: u64,
    pub last_writeback_ns: u64,
}

impl InodeCacheState {
    pub fn new(inode_id: u64, device_id: u32) -> Self {
        Self {
            inode_id,
            device_id,
            cached_pages: 0,
            dirty_pages: 0,
            read_hits: 0,
            read_misses: 0,
            write_count: 0,
            last_access_ns: 0,
            last_writeback_ns: 0,
        }
    }

    pub fn hit_ratio(&self) -> f64 {
        let total = self.read_hits + self.read_misses;
        if total == 0 { return 0.0; }
        self.read_hits as f64 / total as f64
    }

    pub fn dirty_ratio(&self) -> f64 {
        if self.cached_pages == 0 { return 0.0; }
        self.dirty_pages as f64 / self.cached_pages as f64
    }
}

/// Writeback tuning parameters
#[derive(Debug, Clone)]
pub struct WritebackParams {
    pub dirty_background_ratio: f64,
    pub dirty_ratio: f64,
    pub dirty_writeback_interval_ms: u64,
    pub dirty_expire_interval_ms: u64,
    pub nr_to_write: u64,
    pub bandwidth_bps: u64,
}

impl WritebackParams {
    pub fn default_params() -> Self {
        Self {
            dirty_background_ratio: 0.10,
            dirty_ratio: 0.20,
            dirty_writeback_interval_ms: 500,
            dirty_expire_interval_ms: 3000,
            nr_to_write: 1024,
            bandwidth_bps: 0,
        }
    }
}

/// Per-device writeback state
#[derive(Debug, Clone)]
pub struct DeviceWritebackState {
    pub device_id: u32,
    pub dirty_pages: u64,
    pub writeback_pages: u64,
    pub write_bandwidth_bps: u64,
    pub avg_write_latency_ns: u64,
    pub congested: bool,
    pub params: WritebackParams,
}

impl DeviceWritebackState {
    pub fn new(device_id: u32) -> Self {
        Self {
            device_id,
            dirty_pages: 0,
            writeback_pages: 0,
            write_bandwidth_bps: 0,
            avg_write_latency_ns: 0,
            congested: false,
            params: WritebackParams::default_params(),
        }
    }
}

/// Per-process cache usage
#[derive(Debug, Clone)]
pub struct ProcessCacheUsage {
    pub process_id: u64,
    pub cached_pages: u64,
    pub dirty_pages: u64,
    pub hit_ratio: f64,
    pub working_set_pages: u64,
    pub thrashing_score: f64,
    pub refault_count: u64,
}

impl ProcessCacheUsage {
    pub fn new(pid: u64) -> Self {
        Self {
            process_id: pid,
            cached_pages: 0,
            dirty_pages: 0,
            hit_ratio: 0.0,
            working_set_pages: 0,
            thrashing_score: 0.0,
            refault_count: 0,
        }
    }

    pub fn is_thrashing(&self) -> bool {
        self.thrashing_score > 0.5
    }

    pub fn update_thrashing(&mut self, refaults: u64, total_faults: u64) {
        self.refault_count += refaults;
        if total_faults > 0 {
            let new_score = refaults as f64 / total_faults as f64;
            self.thrashing_score = self.thrashing_score * 0.9 + new_score * 0.1;
        }
    }
}

/// Global page cache statistics
#[derive(Debug, Clone)]
pub struct GlobalCacheStats {
    pub total_cached_pages: u64,
    pub total_dirty_pages: u64,
    pub total_writeback_pages: u64,
    pub global_hit_ratio: f64,
    pub thrashing_processes: usize,
    pub total_memory_pages: u64,
}

impl GlobalCacheStats {
    pub fn cache_pressure(&self) -> f64 {
        if self.total_memory_pages == 0 { return 0.0; }
        self.total_cached_pages as f64 / self.total_memory_pages as f64
    }

    pub fn dirty_pressure(&self) -> f64 {
        if self.total_memory_pages == 0 { return 0.0; }
        self.total_dirty_pages as f64 / self.total_memory_pages as f64
    }
}

/// Holistic Page Cache Optimizer
pub struct HolisticPageCache {
    inodes: BTreeMap<u64, InodeCacheState>,
    devices: BTreeMap<u32, DeviceWritebackState>,
    processes: BTreeMap<u64, ProcessCacheUsage>,
    eviction_policy: CacheEvictionPolicy,
    total_memory_pages: u64,
    stats: GlobalCacheStats,
}

impl HolisticPageCache {
    pub fn new(total_memory_pages: u64, policy: CacheEvictionPolicy) -> Self {
        Self {
            inodes: BTreeMap::new(),
            devices: BTreeMap::new(),
            processes: BTreeMap::new(),
            eviction_policy: policy,
            total_memory_pages,
            stats: GlobalCacheStats {
                total_cached_pages: 0,
                total_dirty_pages: 0,
                total_writeback_pages: 0,
                global_hit_ratio: 0.0,
                thrashing_processes: 0,
                total_memory_pages,
            },
        }
    }

    pub fn register_device(&mut self, state: DeviceWritebackState) {
        self.devices.insert(state.device_id, state);
    }

    pub fn record_read(&mut self, inode_id: u64, device_id: u32, hit: bool, pid: u64, now_ns: u64) {
        let entry = self.inodes.entry(inode_id)
            .or_insert_with(|| InodeCacheState::new(inode_id, device_id));
        entry.last_access_ns = now_ns;
        if hit { entry.read_hits += 1; }
        else {
            entry.read_misses += 1;
            entry.cached_pages += 1;
        }

        let proc = self.processes.entry(pid)
            .or_insert_with(|| ProcessCacheUsage::new(pid));
        if !hit { proc.cached_pages += 1; }
        let proc_total = proc.cached_pages;
        let proc_hits = entry.read_hits;
        proc.hit_ratio = if proc_total > 0 {
            proc_hits as f64 / (proc_hits + entry.read_misses) as f64
        } else { 0.0 };
    }

    pub fn record_write(&mut self, inode_id: u64, device_id: u32, pages: u64) {
        let entry = self.inodes.entry(inode_id)
            .or_insert_with(|| InodeCacheState::new(inode_id, device_id));
        entry.dirty_pages += pages;
        entry.write_count += 1;

        if let Some(dev) = self.devices.get_mut(&device_id) {
            dev.dirty_pages += pages;
        }
    }

    pub fn record_writeback(&mut self, inode_id: u64, pages: u64, now_ns: u64) {
        if let Some(entry) = self.inodes.get_mut(&inode_id) {
            entry.dirty_pages = entry.dirty_pages.saturating_sub(pages);
            entry.last_writeback_ns = now_ns;
            if let Some(dev) = self.devices.get_mut(&entry.device_id) {
                dev.dirty_pages = dev.dirty_pages.saturating_sub(pages);
            }
        }
    }

    /// Determine writeback mode based on current pressure
    pub fn writeback_mode(&self) -> WritebackMode {
        let dirty = self.stats.dirty_pressure();
        if dirty > 0.20 { WritebackMode::Forced }
        else if dirty > 0.15 { WritebackMode::Throttled }
        else if dirty > 0.10 { WritebackMode::Periodic }
        else { WritebackMode::Background }
    }

    pub fn recompute(&mut self) {
        self.stats.total_cached_pages = self.inodes.values().map(|i| i.cached_pages).sum();
        self.stats.total_dirty_pages = self.inodes.values().map(|i| i.dirty_pages).sum();
        self.stats.total_writeback_pages = self.devices.values().map(|d| d.writeback_pages).sum();
        let total_hits: u64 = self.inodes.values().map(|i| i.read_hits).sum();
        let total_misses: u64 = self.inodes.values().map(|i| i.read_misses).sum();
        let total = total_hits + total_misses;
        self.stats.global_hit_ratio = if total > 0 { total_hits as f64 / total as f64 } else { 0.0 };
        self.stats.thrashing_processes = self.processes.values()
            .filter(|p| p.is_thrashing()).count();
    }

    pub fn inode_cache(&self, id: u64) -> Option<&InodeCacheState> { self.inodes.get(&id) }
    pub fn process_cache(&self, pid: u64) -> Option<&ProcessCacheUsage> { self.processes.get(&pid) }
    pub fn stats(&self) -> &GlobalCacheStats { &self.stats }
}

// ============================================================================
// Merged from page_cache_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageCacheStateV2 {
    Clean,
    Dirty,
    Writeback,
    Locked,
    Uptodate,
    Error,
}

/// Cached page
#[derive(Debug)]
pub struct CachedPageV2 {
    pub index: u64,
    pub inode: u64,
    pub state: PageCacheStateV2,
    pub flags: u32,
    pub access_count: u64,
    pub last_access: u64,
    pub dirty_since: u64,
}

impl CachedPageV2 {
    pub fn new(index: u64, inode: u64, now: u64) -> Self {
        Self { index, inode, state: PageCacheStateV2::Uptodate, flags: 0, access_count: 1, last_access: now, dirty_since: 0 }
    }

    pub fn mark_dirty(&mut self, now: u64) {
        self.state = PageCacheStateV2::Dirty;
        if self.dirty_since == 0 { self.dirty_since = now; }
    }

    pub fn access(&mut self, now: u64) { self.access_count += 1; self.last_access = now; }
}

/// Radix tree node for page lookup
#[derive(Debug)]
pub struct PageTreeNode {
    pub pages: BTreeMap<u64, CachedPageV2>,
    pub nr_pages: u64,
}

impl PageTreeNode {
    pub fn new() -> Self { Self { pages: BTreeMap::new(), nr_pages: 0 } }
}

/// Stats
#[derive(Debug, Clone)]
pub struct PageCacheV2Stats {
    pub total_pages: u64,
    pub dirty_pages: u64,
    pub writeback_pages: u64,
    pub hit_ratio: f64,
    pub total_lookups: u64,
    pub total_hits: u64,
}

/// Main holistic page cache v2
pub struct HolisticPageCacheV2 {
    inodes: BTreeMap<u64, PageTreeNode>,
    total_lookups: u64,
    total_hits: u64,
}

impl HolisticPageCacheV2 {
    pub fn new() -> Self { Self { inodes: BTreeMap::new(), total_lookups: 0, total_hits: 0 } }

    pub fn find_or_create(&mut self, inode: u64, index: u64, now: u64) -> bool {
        self.total_lookups += 1;
        let tree = self.inodes.entry(inode).or_insert_with(PageTreeNode::new);
        if let Some(page) = tree.pages.get_mut(&index) { page.access(now); self.total_hits += 1; true }
        else { tree.pages.insert(index, CachedPageV2::new(index, inode, now)); tree.nr_pages += 1; false }
    }

    pub fn mark_dirty(&mut self, inode: u64, index: u64, now: u64) {
        if let Some(tree) = self.inodes.get_mut(&inode) {
            if let Some(page) = tree.pages.get_mut(&index) { page.mark_dirty(now); }
        }
    }

    pub fn evict(&mut self, inode: u64, index: u64) -> bool {
        if let Some(tree) = self.inodes.get_mut(&inode) {
            if tree.pages.remove(&index).is_some() { tree.nr_pages -= 1; return true; }
        }
        false
    }

    pub fn stats(&self) -> PageCacheV2Stats {
        let total: u64 = self.inodes.values().map(|t| t.nr_pages).sum();
        let dirty: u64 = self.inodes.values().flat_map(|t| t.pages.values()).filter(|p| p.state == PageCacheStateV2::Dirty).count() as u64;
        let wb: u64 = self.inodes.values().flat_map(|t| t.pages.values()).filter(|p| p.state == PageCacheStateV2::Writeback).count() as u64;
        let ratio = if self.total_lookups == 0 { 0.0 } else { self.total_hits as f64 / self.total_lookups as f64 };
        PageCacheV2Stats { total_pages: total, dirty_pages: dirty, writeback_pages: wb, hit_ratio: ratio, total_lookups: self.total_lookups, total_hits: self.total_hits }
    }
}

// ============================================================================
// Merged from page_cache_v3
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageGeneration {
    Young,
    Warm,
    Old,
    Cold,
}

/// Folio order (log2 of pages in folio).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolioOrder {
    Order0,  // 1 page (4K)
    Order2,  // 4 pages (16K)
    Order4,  // 16 pages (64K)
    Order9,  // 512 pages (2M)
}

impl FolioOrder {
    pub fn pages(&self) -> u64 {
        match self {
            FolioOrder::Order0 => 1,
            FolioOrder::Order2 => 4,
            FolioOrder::Order4 => 16,
            FolioOrder::Order9 => 512,
        }
    }
}

/// A folio in the page cache.
#[derive(Debug, Clone)]
pub struct PageCacheV3Folio {
    pub folio_id: u64,
    pub inode: u64,
    pub offset: u64,
    pub order: FolioOrder,
    pub generation: PageGeneration,
    pub dirty: bool,
    pub writeback: bool,
    pub referenced: bool,
    pub locked: bool,
    pub memcg_id: Option<u64>,
    pub access_count: u64,
}

impl PageCacheV3Folio {
    pub fn new(folio_id: u64, inode: u64, offset: u64, order: FolioOrder) -> Self {
        Self {
            folio_id,
            inode,
            offset,
            order,
            generation: PageGeneration::Young,
            dirty: false,
            writeback: false,
            referenced: false,
            locked: false,
            memcg_id: None,
            access_count: 0,
        }
    }

    pub fn page_count(&self) -> u64 {
        self.order.pages()
    }

    pub fn access(&mut self) {
        self.referenced = true;
        self.access_count += 1;
    }

    pub fn age(&mut self) {
        if !self.referenced {
            self.generation = match self.generation {
                PageGeneration::Young => PageGeneration::Warm,
                PageGeneration::Warm => PageGeneration::Old,
                PageGeneration::Old => PageGeneration::Cold,
                PageGeneration::Cold => PageGeneration::Cold,
            };
        }
        self.referenced = false;
    }
}

/// Per-generation list stats.
#[derive(Debug, Clone)]
pub struct GenerationStats {
    pub folio_count: u64,
    pub page_count: u64,
    pub dirty_count: u64,
}

impl GenerationStats {
    pub fn new() -> Self {
        Self {
            folio_count: 0,
            page_count: 0,
            dirty_count: 0,
        }
    }
}

/// Statistics for page cache V3.
#[derive(Debug, Clone)]
pub struct PageCacheV3Stats {
    pub total_folios: u64,
    pub total_pages: u64,
    pub dirty_pages: u64,
    pub lookups: u64,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub gen_stats: BTreeMap<u8, GenerationStats>,
}

/// Main holistic page cache V3 manager.
pub struct HolisticPageCacheV3 {
    pub folios: BTreeMap<u64, PageCacheV3Folio>,
    pub inode_index: BTreeMap<u64, Vec<u64>>, // inode → [folio_ids]
    pub next_folio_id: u64,
    pub max_pages: u64,
    pub current_pages: u64,
    pub stats: PageCacheV3Stats,
}

impl HolisticPageCacheV3 {
    pub fn new(max_pages: u64) -> Self {
        let mut gen_stats = BTreeMap::new();
        gen_stats.insert(0, GenerationStats::new());
        gen_stats.insert(1, GenerationStats::new());
        gen_stats.insert(2, GenerationStats::new());
        gen_stats.insert(3, GenerationStats::new());
        Self {
            folios: BTreeMap::new(),
            inode_index: BTreeMap::new(),
            next_folio_id: 1,
            max_pages,
            current_pages: 0,
            stats: PageCacheV3Stats {
                total_folios: 0,
                total_pages: 0,
                dirty_pages: 0,
                lookups: 0,
                hits: 0,
                misses: 0,
                evictions: 0,
                gen_stats,
            },
        }
    }

    pub fn insert_folio(&mut self, inode: u64, offset: u64, order: FolioOrder) -> Option<u64> {
        let pages = order.pages();
        if self.current_pages + pages > self.max_pages {
            return None;
        }
        let id = self.next_folio_id;
        self.next_folio_id += 1;
        let folio = PageCacheV3Folio::new(id, inode, offset, order);
        self.inode_index.entry(inode).or_insert_with(Vec::new).push(id);
        self.folios.insert(id, folio);
        self.current_pages += pages;
        self.stats.total_folios += 1;
        self.stats.total_pages += pages;
        Some(id)
    }

    pub fn lookup(&mut self, inode: u64, offset: u64) -> Option<u64> {
        self.stats.lookups += 1;
        if let Some(ids) = self.inode_index.get(&inode) {
            for &id in ids {
                if let Some(folio) = self.folios.get(&id) {
                    let end = folio.offset + folio.page_count() * 4096;
                    if offset >= folio.offset && offset < end {
                        self.stats.hits += 1;
                        return Some(id);
                    }
                }
            }
        }
        self.stats.misses += 1;
        None
    }

    pub fn hit_rate(&self) -> f64 {
        if self.stats.lookups == 0 {
            return 0.0;
        }
        self.stats.hits as f64 / self.stats.lookups as f64
    }

    pub fn folio_count(&self) -> usize {
        self.folios.len()
    }
}
