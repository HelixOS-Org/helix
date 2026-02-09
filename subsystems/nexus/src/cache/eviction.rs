//! Cache eviction optimization.

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

use super::entry::CacheEntry;
use super::types::{CacheKey, EvictionPolicy};
use crate::core::NexusTimestamp;
use crate::math;

// ============================================================================
// EVICTION OPTIMIZER
// ============================================================================

/// Optimizes cache eviction decisions
pub struct EvictionOptimizer {
    /// Current policy
    policy: EvictionPolicy,
    /// Policy statistics
    policy_stats: BTreeMap<EvictionPolicy, PolicyStats>,
    /// ARC parameters
    arc_p: f64, // Target size for T1
    /// Eviction history
    eviction_history: VecDeque<EvictionRecord>,
    /// Max history
    max_history: usize,
}

/// Policy statistics
#[derive(Debug, Clone, Default)]
struct PolicyStats {
    /// Evictions made
    evictions: u64,
    /// Subsequent hits (evicted but would have been hit)
    regrets: u64,
    /// Good evictions (never accessed again)
    good_evictions: u64,
}

/// Eviction record
#[derive(Debug, Clone)]
struct EvictionRecord {
    /// Evicted key
    key: CacheKey,
    /// Eviction time
    time: NexusTimestamp,
    /// Entry stats at eviction
    access_count: u32,
    frequency: f64,
}

impl EvictionOptimizer {
    /// Create new optimizer
    pub fn new(policy: EvictionPolicy) -> Self {
        Self {
            policy,
            policy_stats: BTreeMap::new(),
            arc_p: 0.5,
            eviction_history: VecDeque::new(),
            max_history: 10000,
        }
    }

    /// Select entry to evict
    pub fn select_victim<'a>(&mut self, entries: &'a [CacheEntry]) -> Option<&'a CacheEntry> {
        if entries.is_empty() {
            return None;
        }

        match self.policy {
            EvictionPolicy::Lru => self.select_lru(entries),
            EvictionPolicy::Lfu => self.select_lfu(entries),
            EvictionPolicy::Arc => self.select_arc(entries),
            EvictionPolicy::Clock => self.select_clock(entries),
            EvictionPolicy::Random => self.select_random(entries),
            EvictionPolicy::AiOptimized => self.select_ai(entries),
        }
    }

    /// LRU selection
    fn select_lru<'a>(&self, entries: &'a [CacheEntry]) -> Option<&'a CacheEntry> {
        entries.iter().min_by_key(|e| e.last_access.raw())
    }

    /// LFU selection
    fn select_lfu<'a>(&self, entries: &'a [CacheEntry]) -> Option<&'a CacheEntry> {
        entries
            .iter()
            .min_by(|a, b| a.frequency.partial_cmp(&b.frequency).unwrap())
    }

    /// ARC selection
    fn select_arc<'a>(&self, entries: &'a [CacheEntry]) -> Option<&'a CacheEntry> {
        // Simplified ARC: balance recency and frequency
        let len = entries.len();
        let t1_size = (len as f64 * self.arc_p) as usize;

        // Sort by recency
        let mut by_recency: Vec<_> = entries.iter().collect();
        by_recency.sort_by_key(|e| e.last_access.raw());

        // T1 candidates (recent but low frequency)
        for entry in by_recency.iter().take(t1_size) {
            if entry.access_count < 2 {
                return Some(entry);
            }
        }

        // Fall back to LRU
        by_recency.first().copied()
    }

    /// Clock selection
    fn select_clock<'a>(&self, entries: &'a [CacheEntry]) -> Option<&'a CacheEntry> {
        // Simplified clock: find entry with lowest combined score
        entries.iter().min_by_key(|e| {
            let recency = e.idle_ticks();
            let accessed = if e.access_count > 1 { 0 } else { 1 };
            (accessed, recency)
        })
    }

    /// Random selection
    fn select_random<'a>(&self, entries: &'a [CacheEntry]) -> Option<&'a CacheEntry> {
        if entries.is_empty() {
            None
        } else {
            // Pseudo-random based on current time
            let idx = (NexusTimestamp::now().raw() as usize) % entries.len();
            entries.get(idx)
        }
    }

    /// AI-optimized selection
    fn select_ai<'a>(&self, entries: &'a [CacheEntry]) -> Option<&'a CacheEntry> {
        // Score each entry
        entries.iter().min_by(|a, b| {
            let score_a = self.calculate_eviction_score(a);
            let score_b = self.calculate_eviction_score(b);
            score_b.partial_cmp(&score_a).unwrap()
        })
    }

    /// Calculate eviction score (higher = more likely to evict)
    fn calculate_eviction_score(&self, entry: &CacheEntry) -> f64 {
        let mut score = 0.0;

        // Recency (older = higher score)
        let age_seconds = entry.idle_ticks() as f64 / 1_000_000_000.0;
        score += math::ln(age_seconds + 1.0);

        // Frequency (lower = higher score)
        score += 10.0 / (entry.frequency + 1.0);

        // Size (larger = higher score)
        score += entry.size as f64 / 4096.0;

        // Predicted future access
        if let Some(predicted) = entry.predicted_next_access {
            let now = NexusTimestamp::now().raw();
            if predicted > now {
                let wait_time = (predicted - now) as f64 / 1_000_000_000.0;
                score += wait_time;
            } else {
                score -= 5.0; // Likely to be accessed soon
            }
        }

        score
    }

    /// Record eviction
    pub fn record_eviction(&mut self, entry: &CacheEntry) {
        let record = EvictionRecord {
            key: entry.key,
            time: NexusTimestamp::now(),
            access_count: entry.access_count,
            frequency: entry.frequency,
        };

        self.eviction_history.push_back(record);
        if self.eviction_history.len() > self.max_history {
            self.eviction_history.pop_front();
        }

        let stats = self.policy_stats.entry(self.policy).or_default();
        stats.evictions += 1;
    }

    /// Record regret (evicted entry was needed)
    #[inline]
    pub fn record_regret(&mut self, key: CacheKey) {
        // Check if in eviction history
        for record in &self.eviction_history {
            if record.key == key {
                let stats = self.policy_stats.entry(self.policy).or_default();
                stats.regrets += 1;
                break;
            }
        }
    }

    /// Set policy
    #[inline(always)]
    pub fn set_policy(&mut self, policy: EvictionPolicy) {
        self.policy = policy;
    }

    /// Get policy
    #[inline(always)]
    pub fn policy(&self) -> EvictionPolicy {
        self.policy
    }

    /// Get regret rate
    #[inline]
    pub fn regret_rate(&self) -> f64 {
        let stats = self.policy_stats.get(&self.policy);
        match stats {
            Some(s) if s.evictions > 0 => s.regrets as f64 / s.evictions as f64,
            _ => 0.0,
        }
    }

    /// Adapt ARC parameter
    #[inline]
    pub fn adapt_arc(&mut self, hit_in_t1: bool) {
        if hit_in_t1 {
            // Increase T1 size
            self.arc_p = (self.arc_p + 0.1).min(0.9);
        } else {
            // Decrease T1 size
            self.arc_p = (self.arc_p - 0.1).max(0.1);
        }
    }
}

impl Default for EvictionOptimizer {
    fn default() -> Self {
        Self::new(EvictionPolicy::AiOptimized)
    }
}
