//! Cache statistics tracking.

// ============================================================================
// CACHE STATISTICS
// ============================================================================

/// Cache statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CacheStats {
    /// Total accesses
    pub total_accesses: u64,
    /// Cache hits
    pub hits: u64,
    /// Cache misses
    pub misses: u64,
    /// Evictions
    pub evictions: u64,
    /// Insertions
    pub insertions: u64,
    /// Updates
    pub updates: u64,
    /// Current size (bytes)
    pub current_size: u64,
    /// Maximum size (bytes)
    pub max_size: u64,
    /// Current entries
    pub current_entries: u64,
    /// Read bytes
    pub read_bytes: u64,
    /// Write bytes
    pub write_bytes: u64,
}

impl CacheStats {
    /// Create new stats
    pub fn new(max_size: u64) -> Self {
        Self {
            max_size,
            ..Default::default()
        }
    }

    /// Get hit rate
    #[inline]
    pub fn hit_rate(&self) -> f64 {
        if self.total_accesses == 0 {
            0.0
        } else {
            self.hits as f64 / self.total_accesses as f64
        }
    }

    /// Get miss rate
    #[inline(always)]
    pub fn miss_rate(&self) -> f64 {
        1.0 - self.hit_rate()
    }

    /// Get fill ratio
    #[inline]
    pub fn fill_ratio(&self) -> f64 {
        if self.max_size == 0 {
            0.0
        } else {
            self.current_size as f64 / self.max_size as f64
        }
    }

    /// Record hit
    #[inline]
    pub fn record_hit(&mut self, bytes: u64) {
        self.total_accesses += 1;
        self.hits += 1;
        self.read_bytes += bytes;
    }

    /// Record miss
    #[inline(always)]
    pub fn record_miss(&mut self) {
        self.total_accesses += 1;
        self.misses += 1;
    }

    /// Record insertion
    #[inline]
    pub fn record_insertion(&mut self, size: u64) {
        self.insertions += 1;
        self.current_size += size;
        self.current_entries += 1;
        self.write_bytes += size;
    }

    /// Record eviction
    #[inline]
    pub fn record_eviction(&mut self, size: u64) {
        self.evictions += 1;
        self.current_size = self.current_size.saturating_sub(size);
        self.current_entries = self.current_entries.saturating_sub(1);
    }

    /// Record update
    #[inline]
    pub fn record_update(&mut self, old_size: u64, new_size: u64) {
        self.updates += 1;
        self.current_size = self.current_size.saturating_sub(old_size) + new_size;
        self.write_bytes += new_size;
    }
}
