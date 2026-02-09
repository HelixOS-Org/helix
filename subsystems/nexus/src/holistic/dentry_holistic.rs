// SPDX-License-Identifier: GPL-2.0
//! Holistic dentry — directory entry cache with negative dentry handling

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Dentry state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DentryState {
    Positive,
    Negative,
    New,
    Unhashed,
    Killed,
}

/// Dentry flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DentryFlag {
    Mounted,
    Disconnected,
    Automount,
    ManageTransit,
    LruReferenced,
    CaseSensitive,
}

/// Dentry cache entry
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct DentryCacheEntry {
    pub name_hash: u64,
    pub parent_hash: u64,
    pub inode: u64,
    pub state: DentryState,
    pub flags: u32,
    pub ref_count: u32,
    pub lookup_count: u64,
    pub child_count: u32,
    pub created_ns: u64,
}

impl DentryCacheEntry {
    pub fn new(name: &[u8], parent_hash: u64, inode: u64) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in name { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self {
            name_hash: h, parent_hash, inode,
            state: if inode == 0 { DentryState::Negative } else { DentryState::Positive },
            flags: 0, ref_count: 1, lookup_count: 0, child_count: 0, created_ns: 0,
        }
    }

    #[inline(always)]
    pub fn is_positive(&self) -> bool { self.state == DentryState::Positive }
    #[inline(always)]
    pub fn is_negative(&self) -> bool { self.state == DentryState::Negative }

    #[inline(always)]
    pub fn lookup(&mut self) { self.lookup_count += 1; }
    #[inline(always)]
    pub fn grab(&mut self) { self.ref_count += 1; }
    #[inline(always)]
    pub fn put(&mut self) { self.ref_count = self.ref_count.saturating_sub(1); }

    #[inline(always)]
    pub fn invalidate(&mut self) { self.state = DentryState::Unhashed; }
    #[inline(always)]
    pub fn kill(&mut self) { self.state = DentryState::Killed; }
}

/// Dentry LRU
#[derive(Debug, Clone)]
pub struct DentryLru {
    pub max_entries: u32,
    pub entries: Vec<u64>,
    pub negative_count: u32,
    pub shrink_count: u64,
}

impl DentryLru {
    pub fn new(max_entries: u32) -> Self {
        Self { max_entries, entries: Vec::new(), negative_count: 0, shrink_count: 0 }
    }

    #[inline(always)]
    pub fn add(&mut self, name_hash: u64, is_negative: bool) {
        self.entries.push(name_hash);
        if is_negative { self.negative_count += 1; }
    }

    #[inline]
    pub fn shrink(&mut self, count: u32) -> u32 {
        let removed = count.min(self.entries.len() as u32);
        self.entries.drain(..removed as usize);
        self.shrink_count += 1;
        removed
    }

    #[inline(always)]
    pub fn negative_ratio(&self) -> f64 {
        if self.entries.is_empty() { 0.0 } else { self.negative_count as f64 / self.entries.len() as f64 }
    }
}

/// Dentry holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct HolisticDentryStats {
    pub total_entries: u64,
    pub positive: u64,
    pub negative: u64,
    pub lookups: u64,
    pub cache_hits: u64,
    pub invalidations: u64,
}

/// Main holistic dentry
#[derive(Debug)]
pub struct HolisticDentry {
    pub cache: BTreeMap<u64, DentryCacheEntry>,
    pub lru: DentryLru,
    pub stats: HolisticDentryStats,
}

impl HolisticDentry {
    pub fn new(max_lru: u32) -> Self {
        Self {
            cache: BTreeMap::new(),
            lru: DentryLru::new(max_lru),
            stats: HolisticDentryStats { total_entries: 0, positive: 0, negative: 0, lookups: 0, cache_hits: 0, invalidations: 0 },
        }
    }

    #[inline]
    pub fn insert(&mut self, entry: DentryCacheEntry) {
        self.stats.total_entries += 1;
        if entry.is_positive() { self.stats.positive += 1; } else { self.stats.negative += 1; }
        self.lru.add(entry.name_hash, entry.is_negative());
        self.cache.insert(entry.name_hash, entry);
    }

    #[inline]
    pub fn lookup(&mut self, name_hash: u64) -> Option<&DentryCacheEntry> {
        self.stats.lookups += 1;
        if let Some(entry) = self.cache.get_mut(&name_hash) {
            entry.lookup();
            self.stats.cache_hits += 1;
        }
        self.cache.get(&name_hash)
    }

    #[inline(always)]
    pub fn hit_rate(&self) -> f64 {
        if self.stats.lookups == 0 { 0.0 } else { self.stats.cache_hits as f64 / self.stats.lookups as f64 }
    }
}

// ============================================================================
// Merged from dentry_v2_holistic
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolisticDentryV2Metric {
    CacheSize,
    HitRate,
    NegativeEntryRatio,
    LookupLatency,
    PruneFrequency,
    MemoryPressureEvents,
}

/// Dentry analysis sample
#[derive(Debug, Clone)]
pub struct HolisticDentryV2Sample {
    pub metric: HolisticDentryV2Metric,
    pub value: u64,
    pub timestamp: u64,
}

/// Dentry health assessment
#[derive(Debug, Clone)]
pub struct HolisticDentryV2Health {
    pub cache_efficiency: u64,
    pub memory_pressure: u64,
    pub lookup_performance: u64,
    pub overall: u64,
}

/// Stats for dentry holistic analysis
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct HolisticDentryV2Stats {
    pub samples: u64,
    pub analyses: u64,
    pub alerts: u64,
    pub cache_resize_suggestions: u64,
}

/// Manager for dentry holistic analysis
pub struct HolisticDentryV2Manager {
    samples: VecDeque<HolisticDentryV2Sample>,
    health: HolisticDentryV2Health,
    stats: HolisticDentryV2Stats,
    window_size: usize,
}

impl HolisticDentryV2Manager {
    pub fn new() -> Self {
        Self {
            samples: VecDeque::new(),
            health: HolisticDentryV2Health {
                cache_efficiency: 100,
                memory_pressure: 0,
                lookup_performance: 100,
                overall: 100,
            },
            stats: HolisticDentryV2Stats {
                samples: 0,
                analyses: 0,
                alerts: 0,
                cache_resize_suggestions: 0,
            },
            window_size: 1000,
        }
    }

    pub fn record(&mut self, metric: HolisticDentryV2Metric, value: u64) {
        let sample = HolisticDentryV2Sample {
            metric,
            value,
            timestamp: self.samples.len() as u64,
        };
        self.samples.push_back(sample);
        self.stats.samples += 1;
        if self.samples.len() > self.window_size {
            self.samples.pop_front();
        }
    }

    pub fn analyze(&mut self) -> &HolisticDentryV2Health {
        self.stats.analyses += 1;
        let hit_samples: VecDeque<&HolisticDentryV2Sample> = self.samples.iter()
            .filter(|s| matches!(s.metric, HolisticDentryV2Metric::HitRate))
            .collect();
        if !hit_samples.is_empty() {
            let sum: u64 = hit_samples.iter().map(|s| s.value).sum();
            self.health.cache_efficiency = sum / hit_samples.len() as u64;
        }
        self.health.overall = (self.health.cache_efficiency + self.health.lookup_performance) / 2;
        &self.health
    }

    #[inline(always)]
    pub fn set_window(&mut self, size: usize) {
        self.window_size = size;
    }

    #[inline(always)]
    pub fn stats(&self) -> &HolisticDentryV2Stats {
        &self.stats
    }
}
