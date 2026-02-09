//! # Bridge Dedup Engine
//!
//! Syscall deduplication and memoization:
//! - Identical syscall detection
//! - Result caching for pure syscalls
//! - Redundant call elimination
//! - Dedup statistics and hit rates
//! - TTL-based cache invalidation
//! - Content-addressed dedup

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// DEDUP KEY
// ============================================================================

/// Syscall signature for dedup matching
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SyscallSignature {
    /// Syscall number
    pub syscall_nr: u32,
    /// Process ID
    pub pid: u64,
    /// Argument hash
    pub arg_hash: u64,
}

impl SyscallSignature {
    pub fn new(syscall_nr: u32, pid: u64, arg_hash: u64) -> Self {
        Self {
            syscall_nr,
            pid,
            arg_hash,
        }
    }

    /// FNV-1a style hash combining
    #[inline]
    pub fn compute_hash(syscall_nr: u32, args: &[u64]) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        hash ^= syscall_nr as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        for &arg in args {
            hash ^= arg;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }
}

// ============================================================================
// CACHED RESULT
// ============================================================================

/// Cached syscall result
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CachedResult {
    /// Return value
    pub return_value: u64,
    /// Was error
    pub is_error: bool,
    /// Cache time
    pub cached_at: u64,
    /// TTL (ns)
    pub ttl_ns: u64,
    /// Hit count
    pub hits: u64,
    /// Original duration (ns)
    pub original_duration_ns: u64,
}

impl CachedResult {
    pub fn new(return_value: u64, is_error: bool, cached_at: u64, ttl_ns: u64) -> Self {
        Self {
            return_value,
            is_error,
            cached_at,
            ttl_ns,
            hits: 0,
            original_duration_ns: 0,
        }
    }

    #[inline(always)]
    pub fn with_duration(mut self, duration_ns: u64) -> Self {
        self.original_duration_ns = duration_ns;
        self
    }

    /// Is expired
    #[inline(always)]
    pub fn is_expired(&self, now: u64) -> bool {
        now.saturating_sub(self.cached_at) > self.ttl_ns
    }

    /// Record hit
    #[inline(always)]
    pub fn hit(&mut self) {
        self.hits += 1;
    }

    /// Saved time (estimated)
    #[inline(always)]
    pub fn saved_ns(&self) -> u64 {
        self.hits * self.original_duration_ns
    }
}

// ============================================================================
// DEDUP POLICY
// ============================================================================

/// Which syscalls are safe to dedup
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DedupSafety {
    /// Always safe (pure reads)
    AlwaysSafe,
    /// Safe within process
    ProcessLocal,
    /// Safe within same arguments
    SameArgs,
    /// Never safe (side effects)
    Never,
}

/// Per-syscall dedup policy
#[derive(Debug, Clone)]
pub struct SyscallDedupPolicy {
    /// Syscall number
    pub syscall_nr: u32,
    /// Safety level
    pub safety: DedupSafety,
    /// Default TTL (ns)
    pub default_ttl_ns: u64,
    /// Max cache entries
    pub max_entries: usize,
    /// Enabled
    pub enabled: bool,
}

impl SyscallDedupPolicy {
    pub fn new(syscall_nr: u32, safety: DedupSafety) -> Self {
        Self {
            syscall_nr,
            safety,
            default_ttl_ns: 1_000_000_000, // 1s
            max_entries: 64,
            enabled: true,
        }
    }

    #[inline(always)]
    pub fn with_ttl(mut self, ttl_ns: u64) -> Self {
        self.default_ttl_ns = ttl_ns;
        self
    }

    #[inline(always)]
    pub fn with_max_entries(mut self, max: usize) -> Self {
        self.max_entries = max;
        self
    }
}

// ============================================================================
// REDUNDANCY DETECTOR
// ============================================================================

/// Redundancy pattern
#[derive(Debug, Clone)]
pub struct RedundancyPattern {
    /// Syscall number
    pub syscall_nr: u32,
    /// Process
    pub pid: u64,
    /// Consecutive identical calls
    pub consecutive_count: u64,
    /// Total redundant calls
    pub total_redundant: u64,
    /// Wasted time (ns)
    pub wasted_ns: u64,
}

impl RedundancyPattern {
    pub fn new(syscall_nr: u32, pid: u64) -> Self {
        Self {
            syscall_nr,
            pid,
            consecutive_count: 0,
            total_redundant: 0,
            wasted_ns: 0,
        }
    }

    #[inline]
    pub fn record_redundant(&mut self, duration_ns: u64) {
        self.consecutive_count += 1;
        self.total_redundant += 1;
        self.wasted_ns += duration_ns;
    }

    #[inline(always)]
    pub fn reset_consecutive(&mut self) {
        self.consecutive_count = 0;
    }
}

// ============================================================================
// DEDUP CACHE
// ============================================================================

/// Per-syscall cache
#[derive(Debug)]
struct SyscallCache {
    /// Cached results (signature → result)
    entries: BTreeMap<u64, CachedResult>,
    /// Policy
    policy: SyscallDedupPolicy,
    /// Total lookups
    lookups: u64,
    /// Cache hits
    hits: u64,
}

impl SyscallCache {
    fn new(policy: SyscallDedupPolicy) -> Self {
        Self {
            entries: BTreeMap::new(),
            policy,
            lookups: 0,
            hits: 0,
        }
    }

    fn lookup(&mut self, arg_hash: u64, now: u64) -> Option<u64> {
        self.lookups += 1;
        if let Some(entry) = self.entries.get_mut(&arg_hash) {
            if entry.is_expired(now) {
                self.entries.remove(&arg_hash);
                return None;
            }
            entry.hit();
            self.hits += 1;
            return Some(entry.return_value);
        }
        None
    }

    fn insert(&mut self, arg_hash: u64, result: CachedResult) {
        // Evict if full
        while self.entries.len() >= self.policy.max_entries {
            // Remove oldest
            if let Some(&oldest_key) = self.entries.keys().next() {
                self.entries.remove(&oldest_key);
            } else {
                break;
            }
        }
        self.entries.insert(arg_hash, result);
    }

    fn evict_expired(&mut self, now: u64) {
        self.entries.retain(|_, v| !v.is_expired(now));
    }

    fn hit_rate(&self) -> f64 {
        if self.lookups == 0 {
            return 0.0;
        }
        self.hits as f64 / self.lookups as f64
    }
}

// ============================================================================
// DEDUP MANAGER
// ============================================================================

/// Dedup statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct DedupStats {
    /// Total lookups
    pub total_lookups: u64,
    /// Total hits
    pub total_hits: u64,
    /// Hit rate
    pub hit_rate: f64,
    /// Estimated time saved (ns)
    pub time_saved_ns: u64,
    /// Cache size (entries)
    pub cache_size: usize,
    /// Redundant patterns detected
    pub redundant_patterns: usize,
}

/// Bridge dedup manager
#[repr(align(64))]
pub struct BridgeDedupManager {
    /// Per-syscall caches
    caches: BTreeMap<u32, SyscallCache>,
    /// Policies
    policies: BTreeMap<u32, SyscallDedupPolicy>,
    /// Redundancy detector state (pid, syscall_nr) → pattern
    redundancy: BTreeMap<(u64, u32), RedundancyPattern>,
    /// Last call hash per (pid, syscall_nr)
    last_call: BTreeMap<(u64, u32), u64>,
    /// Stats
    stats: DedupStats,
}

impl BridgeDedupManager {
    pub fn new() -> Self {
        Self {
            caches: BTreeMap::new(),
            policies: BTreeMap::new(),
            redundancy: BTreeMap::new(),
            last_call: BTreeMap::new(),
            stats: DedupStats::default(),
        }
    }

    /// Register dedup policy
    #[inline]
    pub fn register_policy(&mut self, policy: SyscallDedupPolicy) {
        let nr = policy.syscall_nr;
        if policy.enabled {
            self.caches.insert(nr, SyscallCache::new(policy.clone()));
        }
        self.policies.insert(nr, policy);
    }

    /// Try dedup lookup
    pub fn lookup(&mut self, pid: u64, syscall_nr: u32, args: &[u64], now: u64) -> Option<u64> {
        let arg_hash = SyscallSignature::compute_hash(syscall_nr, args);

        // Check redundancy
        let key = (pid, syscall_nr);
        if let Some(&last_hash) = self.last_call.get(&key) {
            if last_hash == arg_hash {
                let pattern = self
                    .redundancy
                    .entry(key)
                    .or_insert_with(|| RedundancyPattern::new(syscall_nr, pid));
                pattern.record_redundant(0);
            } else {
                if let Some(pattern) = self.redundancy.get_mut(&key) {
                    pattern.reset_consecutive();
                }
            }
        }
        self.last_call.insert(key, arg_hash);

        // Cache lookup
        let cache = self.caches.get_mut(&syscall_nr)?;
        let result = cache.lookup(arg_hash, now);

        self.stats.total_lookups += 1;
        if result.is_some() {
            self.stats.total_hits += 1;
        }
        self.update_stats();

        result
    }

    /// Cache result
    pub fn cache_result(
        &mut self,
        syscall_nr: u32,
        args: &[u64],
        return_value: u64,
        is_error: bool,
        duration_ns: u64,
        now: u64,
    ) {
        let Some(policy) = self.policies.get(&syscall_nr) else {
            return;
        };
        if !policy.enabled || policy.safety == DedupSafety::Never {
            return;
        }

        let arg_hash = SyscallSignature::compute_hash(syscall_nr, args);
        let result = CachedResult::new(return_value, is_error, now, policy.default_ttl_ns)
            .with_duration(duration_ns);

        if let Some(cache) = self.caches.get_mut(&syscall_nr) {
            cache.insert(arg_hash, result);
        }

        self.update_cache_size();
    }

    /// Evict expired entries
    #[inline]
    pub fn evict_expired(&mut self, now: u64) {
        for cache in self.caches.values_mut() {
            cache.evict_expired(now);
        }
        self.update_cache_size();
    }

    /// Get redundancy patterns
    #[inline]
    pub fn redundancy_patterns(&self) -> Vec<&RedundancyPattern> {
        self.redundancy
            .values()
            .filter(|p| p.total_redundant > 5)
            .collect()
    }

    fn update_stats(&mut self) {
        if self.stats.total_lookups > 0 {
            self.stats.hit_rate = self.stats.total_hits as f64 / self.stats.total_lookups as f64;
        }
        self.stats.redundant_patterns = self
            .redundancy
            .values()
            .filter(|p| p.total_redundant > 5)
            .count();
    }

    fn update_cache_size(&mut self) {
        self.stats.cache_size = self.caches.values().map(|c| c.entries.len()).sum();
        self.stats.time_saved_ns = self
            .caches
            .values()
            .flat_map(|c| c.entries.values())
            .map(|e| e.saved_ns())
            .sum();
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &DedupStats {
        &self.stats
    }
}
