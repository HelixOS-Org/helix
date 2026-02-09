//! # Application Cache Profiler
//!
//! Per-process cache behavior analysis:
//! - L1/L2/L3 miss rate tracking
//! - Cache line utilization and sharing
//! - False sharing detection
//! - Prefetch effectiveness measurement
//! - Cache-aware scheduling hints

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Cache level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CacheProfileLevel {
    L1Data,
    L1Instruction,
    L2Unified,
    L3Unified,
    Tlb,
}

/// Cache event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheEventType {
    Hit,
    Miss,
    Prefetch,
    Eviction,
    Writeback,
    Invalidation,
}

/// False sharing severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FalseSharingSeverity {
    None,
    Low,
    Moderate,
    High,
    Critical,
}

/// Per-level cache statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CacheLevelStats {
    pub level: CacheProfileLevel,
    pub accesses: u64,
    pub hits: u64,
    pub misses: u64,
    pub prefetches: u64,
    pub evictions: u64,
    pub writebacks: u64,
    pub invalidations: u64,
    pub useful_prefetch: u64,
    pub wasted_prefetch: u64,
}

impl CacheLevelStats {
    pub fn new(level: CacheProfileLevel) -> Self {
        Self {
            level,
            accesses: 0,
            hits: 0,
            misses: 0,
            prefetches: 0,
            evictions: 0,
            writebacks: 0,
            invalidations: 0,
            useful_prefetch: 0,
            wasted_prefetch: 0,
        }
    }

    #[inline(always)]
    pub fn hit_rate(&self) -> f64 {
        if self.accesses == 0 { return 0.0; }
        self.hits as f64 / self.accesses as f64
    }

    #[inline(always)]
    pub fn miss_rate(&self) -> f64 {
        1.0 - self.hit_rate()
    }

    #[inline]
    pub fn prefetch_effectiveness(&self) -> f64 {
        let total = self.useful_prefetch + self.wasted_prefetch;
        if total == 0 { return 0.0; }
        self.useful_prefetch as f64 / total as f64
    }

    #[inline]
    pub fn record_event(&mut self, event: CacheEventType) {
        match event {
            CacheEventType::Hit => { self.accesses += 1; self.hits += 1; }
            CacheEventType::Miss => { self.accesses += 1; self.misses += 1; }
            CacheEventType::Prefetch => { self.prefetches += 1; }
            CacheEventType::Eviction => { self.evictions += 1; }
            CacheEventType::Writeback => { self.writebacks += 1; }
            CacheEventType::Invalidation => { self.invalidations += 1; }
        }
    }
}

/// Cache line sharing info
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CacheLineSharing {
    pub address: u64,
    pub line_size: u32,
    pub owning_threads: Vec<u64>,
    pub write_count: u64,
    pub read_count: u64,
    pub invalidation_count: u64,
    pub false_sharing_score: f64,
}

impl CacheLineSharing {
    pub fn new(address: u64, line_size: u32) -> Self {
        Self {
            address,
            line_size,
            owning_threads: Vec::new(),
            write_count: 0,
            read_count: 0,
            invalidation_count: 0,
            false_sharing_score: 0.0,
        }
    }

    #[inline]
    pub fn severity(&self) -> FalseSharingSeverity {
        if self.owning_threads.len() < 2 { return FalseSharingSeverity::None; }
        if self.false_sharing_score < 0.1 { return FalseSharingSeverity::Low; }
        if self.false_sharing_score < 0.3 { return FalseSharingSeverity::Moderate; }
        if self.false_sharing_score < 0.6 { return FalseSharingSeverity::High; }
        FalseSharingSeverity::Critical
    }

    pub fn add_access(&mut self, thread_id: u64, is_write: bool) {
        if !self.owning_threads.contains(&thread_id) {
            self.owning_threads.push(thread_id);
        }
        if is_write {
            self.write_count += 1;
        } else {
            self.read_count += 1;
        }
        // Recompute false sharing score
        let threads = self.owning_threads.len() as f64;
        let write_ratio = if self.write_count + self.read_count > 0 {
            self.write_count as f64 / (self.write_count + self.read_count) as f64
        } else { 0.0 };
        let inv_ratio = if self.write_count > 0 {
            self.invalidation_count as f64 / self.write_count as f64
        } else { 0.0 };
        self.false_sharing_score = (threads - 1.0) * write_ratio * inv_ratio;
        if self.false_sharing_score > 1.0 { self.false_sharing_score = 1.0; }
    }
}

/// Working set estimation
#[derive(Debug, Clone)]
pub struct WorkingSetEstimate {
    pub hot_pages: u64,
    pub warm_pages: u64,
    pub cold_pages: u64,
    pub estimated_bytes: u64,
    pub l1_fit: bool,
    pub l2_fit: bool,
    pub l3_fit: bool,
}

impl WorkingSetEstimate {
    #[inline(always)]
    pub fn total_pages(&self) -> u64 {
        self.hot_pages + self.warm_pages + self.cold_pages
    }

    #[inline]
    pub fn best_fit_level(&self) -> CacheProfileLevel {
        if self.l1_fit { CacheProfileLevel::L1Data }
        else if self.l2_fit { CacheProfileLevel::L2Unified }
        else { CacheProfileLevel::L3Unified }
    }
}

/// Per-process cache profile
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ProcessCacheProfile {
    pub pid: u64,
    pub level_stats: BTreeMap<u8, CacheLevelStats>,
    pub false_sharing_hotspots: Vec<CacheLineSharing>,
    pub working_set: WorkingSetEstimate,
    pub sample_count: u64,
    pub last_update_ts: u64,
}

impl ProcessCacheProfile {
    pub fn new(pid: u64) -> Self {
        let mut level_stats = BTreeMap::new();
        level_stats.insert(0, CacheLevelStats::new(CacheProfileLevel::L1Data));
        level_stats.insert(1, CacheLevelStats::new(CacheProfileLevel::L1Instruction));
        level_stats.insert(2, CacheLevelStats::new(CacheProfileLevel::L2Unified));
        level_stats.insert(3, CacheLevelStats::new(CacheProfileLevel::L3Unified));
        level_stats.insert(4, CacheLevelStats::new(CacheProfileLevel::Tlb));

        Self {
            pid,
            level_stats,
            false_sharing_hotspots: Vec::new(),
            working_set: WorkingSetEstimate {
                hot_pages: 0, warm_pages: 0, cold_pages: 0,
                estimated_bytes: 0, l1_fit: false, l2_fit: false, l3_fit: false,
            },
            sample_count: 0,
            last_update_ts: 0,
        }
    }

    #[inline]
    pub fn overall_miss_rate(&self) -> f64 {
        if let Some(l1) = self.level_stats.get(&0) {
            l1.miss_rate()
        } else { 0.0 }
    }

    #[inline]
    pub fn record_event(&mut self, level: u8, event: CacheEventType) {
        if let Some(stats) = self.level_stats.get_mut(&level) {
            stats.record_event(event);
        }
        self.sample_count += 1;
    }

    #[inline]
    pub fn worst_false_sharing_score(&self) -> f64 {
        self.false_sharing_hotspots.iter()
            .map(|h| h.false_sharing_score)
            .fold(0.0_f64, |a, b| if a > b { a } else { b })
    }
}

/// App Cache Profiler stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppCacheProfilerStats {
    pub total_processes: usize,
    pub total_samples: u64,
    pub false_sharing_hotspots: usize,
    pub avg_l1_miss_rate: f64,
    pub worst_miss_rate_pid: u64,
}

/// Application Cache Profiler
#[repr(align(64))]
pub struct AppCacheProfiler {
    profiles: BTreeMap<u64, ProcessCacheProfile>,
    global_samples: u64,
    cache_line_size: u32,
    stats: AppCacheProfilerStats,
}

impl AppCacheProfiler {
    pub fn new(cache_line_size: u32) -> Self {
        Self {
            profiles: BTreeMap::new(),
            global_samples: 0,
            cache_line_size,
            stats: AppCacheProfilerStats::default(),
        }
    }

    #[inline(always)]
    pub fn register_process(&mut self, pid: u64) {
        self.profiles.entry(pid).or_insert_with(|| ProcessCacheProfile::new(pid));
        self.recompute();
    }

    #[inline]
    pub fn record_event(&mut self, pid: u64, level: u8, event: CacheEventType) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.record_event(level, event);
            self.global_samples += 1;
        }
    }

    pub fn record_false_sharing(&mut self, pid: u64, address: u64, thread_id: u64, is_write: bool) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            let aligned = address & !(self.cache_line_size as u64 - 1);
            if let Some(hs) = profile.false_sharing_hotspots.iter_mut().find(|h| h.address == aligned) {
                hs.add_access(thread_id, is_write);
            } else {
                let mut sharing = CacheLineSharing::new(aligned, self.cache_line_size);
                sharing.add_access(thread_id, is_write);
                profile.false_sharing_hotspots.push(sharing);
                // Cap hotspot tracking
                if profile.false_sharing_hotspots.len() > 128 {
                    profile.false_sharing_hotspots.sort_by(|a, b| {
                        b.false_sharing_score.partial_cmp(&a.false_sharing_score).unwrap_or(core::cmp::Ordering::Equal)
                    });
                    profile.false_sharing_hotspots.truncate(64);
                }
            }
        }
        self.recompute();
    }

    pub fn update_working_set(&mut self, pid: u64, hot: u64, warm: u64, cold: u64,
                               l1_size: u64, l2_size: u64, l3_size: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            let total_bytes = (hot + warm + cold) * 4096;
            profile.working_set = WorkingSetEstimate {
                hot_pages: hot,
                warm_pages: warm,
                cold_pages: cold,
                estimated_bytes: total_bytes,
                l1_fit: total_bytes <= l1_size,
                l2_fit: total_bytes <= l2_size,
                l3_fit: total_bytes <= l3_size,
            };
        }
    }

    fn recompute(&mut self) {
        self.stats.total_processes = self.profiles.len();
        self.stats.total_samples = self.global_samples;
        self.stats.false_sharing_hotspots = self.profiles.values()
            .map(|p| p.false_sharing_hotspots.iter().filter(|h| h.severity() != FalseSharingSeverity::None).count())
            .sum();

        let (sum_miss, count, mut worst_pid, mut worst_rate) = self.profiles.values().fold(
            (0.0_f64, 0u32, 0u64, 0.0_f64),
            |(sum, cnt, wp, wr), p| {
                let mr = p.overall_miss_rate();
                let (new_wp, new_wr) = if mr > wr { (p.pid, mr) } else { (wp, wr) };
                (sum + mr, cnt + 1, new_wp, new_wr)
            },
        );
        let _ = (worst_pid, worst_rate);
        self.stats.avg_l1_miss_rate = if count > 0 { sum_miss / count as f64 } else { 0.0 };
        self.stats.worst_miss_rate_pid = worst_pid;
    }

    #[inline(always)]
    pub fn profile(&self, pid: u64) -> Option<&ProcessCacheProfile> {
        self.profiles.get(&pid)
    }

    #[inline(always)]
    pub fn stats(&self) -> &AppCacheProfilerStats {
        &self.stats
    }

    #[inline(always)]
    pub fn remove_process(&mut self, pid: u64) {
        self.profiles.remove(&pid);
        self.recompute();
    }
}
