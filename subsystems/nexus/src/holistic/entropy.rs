//! # Holistic Entropy Tracker
//!
//! System-wide entropy and randomness management:
//! - Entropy pool monitoring
//! - Random number quality assessment
//! - Entropy source tracking
//! - Depletion prediction
//! - Entropy distribution across consumers

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// ENTROPY TYPES
// ============================================================================

/// Entropy source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntropySource {
    /// Hardware RNG (RDRAND/RDSEED)
    HardwareRng,
    /// Interrupt timing jitter
    InterruptJitter,
    /// Disk I/O timing
    DiskTiming,
    /// Network packet timing
    NetworkTiming,
    /// Keyboard/input events
    InputEvents,
    /// CPU cycle jitter
    CpuJitter,
    /// Platform TPM
    Tpm,
}

/// Pool health
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolHealth {
    /// Healthy (>75% full)
    Healthy,
    /// Adequate (50-75%)
    Adequate,
    /// Low (25-50%)
    Low,
    /// Critical (<25%)
    Critical,
    /// Depleted (empty)
    Depleted,
}

// ============================================================================
// ENTROPY SOURCE TRACKER
// ============================================================================

/// Per-source tracking
#[derive(Debug, Clone)]
pub struct SourceTracker {
    /// Source type
    pub source: EntropySource,
    /// Bits contributed
    pub total_bits: u64,
    /// Contribution rate (bits/sec EMA)
    pub rate_ema: f64,
    /// Quality estimate (0..1)
    pub quality: f64,
    /// Last contribution (ns)
    pub last_contribution_ns: u64,
    /// Active
    pub active: bool,
    /// Sample count
    pub samples: u64,
}

impl SourceTracker {
    pub fn new(source: EntropySource) -> Self {
        Self {
            source,
            total_bits: 0,
            rate_ema: 0.0,
            quality: 1.0,
            last_contribution_ns: 0,
            active: true,
            samples: 0,
        }
    }

    /// Record contribution
    pub fn contribute(&mut self, bits: u64, now: u64) {
        self.total_bits += bits;
        self.samples += 1;

        if self.last_contribution_ns > 0 {
            let interval = now.saturating_sub(self.last_contribution_ns) as f64 / 1_000_000_000.0;
            if interval > 0.0 {
                let rate = bits as f64 / interval;
                self.rate_ema = 0.9 * self.rate_ema + 0.1 * rate;
            }
        }
        self.last_contribution_ns = now;
    }

    /// Assess quality (chi-squared simplification)
    pub fn assess_quality(&mut self, sample_bytes: &[u8]) {
        if sample_bytes.len() < 16 {
            return;
        }
        // Simple byte frequency analysis
        let mut freq = [0u32; 256];
        for &b in sample_bytes {
            freq[b as usize] += 1;
        }
        let expected = sample_bytes.len() as f64 / 256.0;
        let mut chi_sq = 0.0;
        for &f in &freq {
            let diff = f as f64 - expected;
            chi_sq += diff * diff / expected.max(0.001);
        }
        // Normalize to 0..1 (lower chi_sq = better)
        // Perfect uniform: chi_sq ~= 255, bad: chi_sq >> 255
        self.quality = (1.0 - (chi_sq - 255.0) / 1000.0).max(0.0).min(1.0);
    }
}

// ============================================================================
// ENTROPY POOL
// ============================================================================

/// Entropy pool
#[derive(Debug)]
pub struct EntropyPool {
    /// Pool name hash (FNV-1a)
    pub name_hash: u64,
    /// Current entropy bits
    pub current_bits: u64,
    /// Capacity bits
    pub capacity_bits: u64,
    /// Total bits generated
    pub total_generated: u64,
    /// Total bits consumed
    pub total_consumed: u64,
    /// Consumers
    consumers: BTreeMap<u64, u64>,
}

impl EntropyPool {
    pub fn new(name: &str, capacity_bits: u64) -> Self {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in name.as_bytes() {
            hash ^= *b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        Self {
            name_hash: hash,
            current_bits: 0,
            capacity_bits,
            total_generated: 0,
            total_consumed: 0,
            consumers: BTreeMap::new(),
        }
    }

    /// Add entropy
    pub fn add(&mut self, bits: u64) {
        self.current_bits = (self.current_bits + bits).min(self.capacity_bits);
        self.total_generated += bits;
    }

    /// Consume entropy
    pub fn consume(&mut self, bits: u64, consumer_pid: u64) -> u64 {
        let available = self.current_bits.min(bits);
        self.current_bits -= available;
        self.total_consumed += available;
        *self.consumers.entry(consumer_pid).or_insert(0) += available;
        available
    }

    /// Fill ratio
    pub fn fill_ratio(&self) -> f64 {
        if self.capacity_bits == 0 {
            return 0.0;
        }
        self.current_bits as f64 / self.capacity_bits as f64
    }

    /// Health
    pub fn health(&self) -> PoolHealth {
        let ratio = self.fill_ratio();
        if ratio > 0.75 {
            PoolHealth::Healthy
        } else if ratio > 0.5 {
            PoolHealth::Adequate
        } else if ratio > 0.25 {
            PoolHealth::Low
        } else if self.current_bits > 0 {
            PoolHealth::Critical
        } else {
            PoolHealth::Depleted
        }
    }

    /// Depletion forecast (seconds until empty at current consumption rate)
    pub fn depletion_forecast(&self, consumption_rate_bps: f64) -> f64 {
        if consumption_rate_bps <= 0.0 {
            return f64::INFINITY;
        }
        self.current_bits as f64 / consumption_rate_bps
    }

    /// Top consumers
    pub fn top_consumers(&self, n: usize) -> Vec<(u64, u64)> {
        let mut sorted: Vec<(u64, u64)> = self.consumers.iter().map(|(&k, &v)| (k, v)).collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(n);
        sorted
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Entropy tracker stats
#[derive(Debug, Clone, Default)]
pub struct HolisticEntropyStats {
    /// Tracked sources
    pub tracked_sources: usize,
    /// Active sources
    pub active_sources: usize,
    /// Total pool bits
    pub total_pool_bits: u64,
    /// Total capacity bits
    pub total_capacity: u64,
    /// Average quality
    pub avg_quality: f64,
    /// Pool health
    pub pool_health: u8,
}

/// Holistic entropy tracker
pub struct HolisticEntropyTracker {
    /// Sources
    sources: BTreeMap<u8, SourceTracker>,
    /// Primary pool
    pub pool: EntropyPool,
    /// Stats
    stats: HolisticEntropyStats,
}

impl HolisticEntropyTracker {
    pub fn new(pool_capacity: u64) -> Self {
        Self {
            sources: BTreeMap::new(),
            pool: EntropyPool::new("primary", pool_capacity),
            stats: HolisticEntropyStats::default(),
        }
    }

    /// Register source
    pub fn register_source(&mut self, source: EntropySource) {
        self.sources.insert(source as u8, SourceTracker::new(source));
        self.update_stats();
    }

    /// Contribute entropy from source
    pub fn contribute(&mut self, source: EntropySource, bits: u64, now: u64) {
        if let Some(tracker) = self.sources.get_mut(&(source as u8)) {
            tracker.contribute(bits, now);
        }
        self.pool.add(bits);
        self.update_stats();
    }

    /// Consume entropy
    pub fn consume(&mut self, bits: u64, consumer_pid: u64) -> u64 {
        let actual = self.pool.consume(bits, consumer_pid);
        self.update_stats();
        actual
    }

    fn update_stats(&mut self) {
        self.stats.tracked_sources = self.sources.len();
        self.stats.active_sources = self.sources.values().filter(|s| s.active).count();
        self.stats.total_pool_bits = self.pool.current_bits;
        self.stats.total_capacity = self.pool.capacity_bits;
        if !self.sources.is_empty() {
            self.stats.avg_quality = self.sources.values()
                .map(|s| s.quality)
                .sum::<f64>() / self.sources.len() as f64;
        }
        self.stats.pool_health = self.pool.health() as u8;
    }

    /// Stats
    pub fn stats(&self) -> &HolisticEntropyStats {
        &self.stats
    }
}
