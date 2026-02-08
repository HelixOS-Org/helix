//! # Holistic Memory Compression
//!
//! System-wide memory compression engine:
//! - Zswap-style compressed page cache
//! - Per-pool compression ratio tracking
//! - LRU eviction from compressed store
//! - Compression algorithm selection (LZ4, ZSTD, LZO simulated)
//! - Writeback to swap policy
//! - Compression ratio prediction for admission

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Compression algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressAlgorithm {
    Lz4,
    Zstd,
    Lzo,
    None,
}

/// Pool type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompressPoolType {
    /// Hot pages (recently accessed)
    Hot,
    /// Warm pages
    Warm,
    /// Cold pages (candidates for writeback)
    Cold,
}

/// Compressed page entry
#[derive(Debug, Clone)]
pub struct CompressedPage {
    pub page_pfn: u64,
    pub original_size: u32,
    pub compressed_size: u32,
    pub algorithm: CompressAlgorithm,
    pub pool: CompressPoolType,
    pub access_count: u32,
    pub last_access_ts: u64,
    pub owner_pid: u64,
}

impl CompressedPage {
    pub fn new(pfn: u64, original: u32, compressed: u32, algo: CompressAlgorithm, pid: u64) -> Self {
        Self {
            page_pfn: pfn,
            original_size: original,
            compressed_size: compressed,
            algorithm: algo,
            pool: CompressPoolType::Hot,
            access_count: 1,
            last_access_ts: 0,
            owner_pid: pid,
        }
    }

    pub fn ratio(&self) -> f64 {
        if self.compressed_size == 0 { return 0.0; }
        self.original_size as f64 / self.compressed_size as f64
    }

    pub fn savings_bytes(&self) -> u32 {
        self.original_size.saturating_sub(self.compressed_size)
    }
}

/// Compression pool
#[derive(Debug, Clone)]
pub struct CompressPool {
    pub pool_type: CompressPoolType,
    pub pages: BTreeMap<u64, CompressedPage>,
    pub max_pages: usize,
    pub total_original_bytes: u64,
    pub total_compressed_bytes: u64,
    pub total_accesses: u64,
    pub evictions: u64,
}

impl CompressPool {
    pub fn new(pool_type: CompressPoolType, max_pages: usize) -> Self {
        Self {
            pool_type,
            pages: BTreeMap::new(),
            max_pages,
            total_original_bytes: 0,
            total_compressed_bytes: 0,
            total_accesses: 0,
            evictions: 0,
        }
    }

    pub fn ratio(&self) -> f64 {
        if self.total_compressed_bytes == 0 { return 0.0; }
        self.total_original_bytes as f64 / self.total_compressed_bytes as f64
    }

    pub fn usage(&self) -> f64 {
        if self.max_pages == 0 { return 0.0; }
        self.pages.len() as f64 / self.max_pages as f64
    }

    pub fn insert(&mut self, page: CompressedPage) -> Option<CompressedPage> {
        self.total_original_bytes += page.original_size as u64;
        self.total_compressed_bytes += page.compressed_size as u64;

        let evicted = if self.pages.len() >= self.max_pages {
            self.evict_lru()
        } else { None };

        self.pages.insert(page.page_pfn, page);
        evicted
    }

    pub fn access(&mut self, pfn: u64, now: u64) -> bool {
        if let Some(page) = self.pages.get_mut(&pfn) {
            page.access_count += 1;
            page.last_access_ts = now;
            self.total_accesses += 1;
            true
        } else { false }
    }

    pub fn remove(&mut self, pfn: u64) -> Option<CompressedPage> {
        if let Some(page) = self.pages.remove(&pfn) {
            self.total_original_bytes -= page.original_size as u64;
            self.total_compressed_bytes -= page.compressed_size as u64;
            Some(page)
        } else { None }
    }

    fn evict_lru(&mut self) -> Option<CompressedPage> {
        // Find page with oldest last_access_ts
        let victim_pfn = self.pages.iter()
            .min_by_key(|(_, p)| p.last_access_ts)
            .map(|(&pfn, _)| pfn);

        if let Some(pfn) = victim_pfn {
            self.evictions += 1;
            self.remove(pfn)
        } else { None }
    }

    /// Get cold pages (candidates for writeback)
    pub fn cold_pages(&self, age_threshold: u64, now: u64) -> Vec<u64> {
        self.pages.iter()
            .filter(|(_, p)| now.saturating_sub(p.last_access_ts) > age_threshold)
            .map(|(&pfn, _)| pfn)
            .collect()
    }
}

/// Admission decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionDecision {
    /// Accept into compressed cache
    Accept,
    /// Reject (ratio too low)
    RejectLowRatio,
    /// Reject (pool full, better pages exist)
    RejectPoolFull,
    /// Direct to swap
    DirectToSwap,
}

/// Compression stats
#[derive(Debug, Clone, Default)]
pub struct HolisticMemCompressStats {
    pub total_pages: usize,
    pub hot_pages: usize,
    pub warm_pages: usize,
    pub cold_pages: usize,
    pub overall_ratio: f64,
    pub total_savings_mb: f64,
    pub total_evictions: u64,
    pub admission_rejects: u64,
    pub writeback_count: u64,
}

/// Holistic Memory Compression Engine
pub struct HolisticMemCompress {
    pools: BTreeMap<CompressPoolType, CompressPool>,
    min_ratio_threshold: f64,
    default_algo: CompressAlgorithm,
    admission_rejects: u64,
    writeback_count: u64,
    stats: HolisticMemCompressStats,
}

impl HolisticMemCompress {
    pub fn new(hot_max: usize, warm_max: usize, cold_max: usize) -> Self {
        let mut pools = BTreeMap::new();
        pools.insert(CompressPoolType::Hot, CompressPool::new(CompressPoolType::Hot, hot_max));
        pools.insert(CompressPoolType::Warm, CompressPool::new(CompressPoolType::Warm, warm_max));
        pools.insert(CompressPoolType::Cold, CompressPool::new(CompressPoolType::Cold, cold_max));

        Self {
            pools,
            min_ratio_threshold: 1.2,
            default_algo: CompressAlgorithm::Lz4,
            admission_rejects: 0,
            writeback_count: 0,
            stats: HolisticMemCompressStats::default(),
        }
    }

    /// Check if a page should be admitted
    pub fn admission_check(&self, original_size: u32, compressed_size: u32) -> AdmissionDecision {
        if compressed_size == 0 || original_size == 0 {
            return AdmissionDecision::RejectLowRatio;
        }
        let ratio = original_size as f64 / compressed_size as f64;

        if ratio < self.min_ratio_threshold {
            return AdmissionDecision::RejectLowRatio;
        }

        let hot_pool = self.pools.get(&CompressPoolType::Hot).unwrap();
        if hot_pool.usage() > 0.95 {
            // Pool near full — only accept high-ratio pages
            if ratio < 2.0 {
                return AdmissionDecision::RejectPoolFull;
            }
        }

        AdmissionDecision::Accept
    }

    /// Store a compressed page
    pub fn store(&mut self, pfn: u64, original_size: u32, compressed_size: u32, pid: u64) -> AdmissionDecision {
        let decision = self.admission_check(original_size, compressed_size);
        match decision {
            AdmissionDecision::Accept => {
                let page = CompressedPage::new(pfn, original_size, compressed_size, self.default_algo, pid);
                if let Some(pool) = self.pools.get_mut(&CompressPoolType::Hot) {
                    if let Some(evicted) = pool.insert(page) {
                        // Move evicted to warm
                        if let Some(warm) = self.pools.get_mut(&CompressPoolType::Warm) {
                            let mut moved = evicted;
                            moved.pool = CompressPoolType::Warm;
                            warm.insert(moved);
                        }
                    }
                }
                self.recompute();
                AdmissionDecision::Accept
            }
            other => {
                self.admission_rejects += 1;
                other
            }
        }
    }

    /// Access a compressed page (decompress)
    pub fn access(&mut self, pfn: u64, now: u64) -> Option<&CompressedPage> {
        // Try pools in order: hot, warm, cold
        for pool_type in &[CompressPoolType::Hot, CompressPoolType::Warm, CompressPoolType::Cold] {
            if let Some(pool) = self.pools.get_mut(pool_type) {
                if pool.access(pfn, now) {
                    // Promote to hot if not already
                    if *pool_type != CompressPoolType::Hot {
                        if let Some(page) = pool.remove(pfn) {
                            let mut promoted = page;
                            promoted.pool = CompressPoolType::Hot;
                            if let Some(hot) = self.pools.get_mut(&CompressPoolType::Hot) {
                                hot.insert(promoted);
                            }
                        }
                    }
                    // Return reference from hot pool
                    return self.pools.get(&CompressPoolType::Hot)
                        .and_then(|p| p.pages.get(&pfn));
                }
            }
        }
        None
    }

    /// Age pages: move hot→warm→cold based on access patterns
    pub fn age_pages(&mut self, now: u64, hot_age_ms: u64, warm_age_ms: u64) {
        let hot_threshold = hot_age_ms * 1_000_000;
        let warm_threshold = warm_age_ms * 1_000_000;

        // Hot → Warm
        if let Some(hot) = self.pools.get(&CompressPoolType::Hot) {
            let to_demote: Vec<u64> = hot.cold_pages(hot_threshold, now);
            for pfn in to_demote {
                if let Some(pool) = self.pools.get_mut(&CompressPoolType::Hot) {
                    if let Some(page) = pool.remove(pfn) {
                        let mut demoted = page;
                        demoted.pool = CompressPoolType::Warm;
                        if let Some(warm) = self.pools.get_mut(&CompressPoolType::Warm) {
                            warm.insert(demoted);
                        }
                    }
                }
            }
        }

        // Warm → Cold
        if let Some(warm) = self.pools.get(&CompressPoolType::Warm) {
            let to_demote: Vec<u64> = warm.cold_pages(warm_threshold, now);
            for pfn in to_demote {
                if let Some(pool) = self.pools.get_mut(&CompressPoolType::Warm) {
                    if let Some(page) = pool.remove(pfn) {
                        let mut demoted = page;
                        demoted.pool = CompressPoolType::Cold;
                        if let Some(cold) = self.pools.get_mut(&CompressPoolType::Cold) {
                            cold.insert(demoted);
                        }
                    }
                }
            }
        }

        self.recompute();
    }

    /// Writeback cold pages (return PFNs to write to swap)
    pub fn writeback(&mut self, max_pages: usize) -> Vec<u64> {
        let mut written = Vec::new();
        if let Some(cold) = self.pools.get_mut(&CompressPoolType::Cold) {
            let pfns: Vec<u64> = cold.pages.keys().copied().take(max_pages).collect();
            for pfn in pfns {
                cold.remove(pfn);
                written.push(pfn);
                self.writeback_count += 1;
            }
        }
        self.recompute();
        written
    }

    fn recompute(&mut self) {
        let hot = self.pools.get(&CompressPoolType::Hot);
        let warm = self.pools.get(&CompressPoolType::Warm);
        let cold = self.pools.get(&CompressPoolType::Cold);

        let hot_count = hot.map(|p| p.pages.len()).unwrap_or(0);
        let warm_count = warm.map(|p| p.pages.len()).unwrap_or(0);
        let cold_count = cold.map(|p| p.pages.len()).unwrap_or(0);

        let total_orig: u64 = self.pools.values().map(|p| p.total_original_bytes).sum();
        let total_comp: u64 = self.pools.values().map(|p| p.total_compressed_bytes).sum();
        let ratio = if total_comp > 0 { total_orig as f64 / total_comp as f64 } else { 0.0 };
        let savings = (total_orig - total_comp) as f64 / (1024.0 * 1024.0);
        let evictions: u64 = self.pools.values().map(|p| p.evictions).sum();

        self.stats = HolisticMemCompressStats {
            total_pages: hot_count + warm_count + cold_count,
            hot_pages: hot_count,
            warm_pages: warm_count,
            cold_pages: cold_count,
            overall_ratio: ratio,
            total_savings_mb: savings,
            total_evictions: evictions,
            admission_rejects: self.admission_rejects,
            writeback_count: self.writeback_count,
        };
    }

    pub fn stats(&self) -> &HolisticMemCompressStats {
        &self.stats
    }
}
