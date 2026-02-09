//! NUMA statistics tracking.

extern crate alloc;

// ============================================================================
// NUMA STATISTICS
// ============================================================================

/// NUMA access statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct NumaStats {
    /// Local accesses
    pub local_accesses: u64,
    /// Remote accesses
    pub remote_accesses: u64,
    /// Local hits
    pub local_hits: u64,
    /// Remote hits
    pub remote_hits: u64,
    /// Interleaved accesses
    pub interleaved: u64,
    /// Page migrations
    pub migrations: u64,
    /// Migration bytes
    pub migration_bytes: u64,
}

impl NumaStats {
    /// Local access ratio
    #[inline]
    pub fn local_ratio(&self) -> f64 {
        let total = self.local_accesses + self.remote_accesses;
        if total == 0 {
            1.0
        } else {
            self.local_accesses as f64 / total as f64
        }
    }

    /// Remote access ratio
    #[inline(always)]
    pub fn remote_ratio(&self) -> f64 {
        1.0 - self.local_ratio()
    }

    /// Record local access
    #[inline]
    pub fn record_local(&mut self, hit: bool) {
        self.local_accesses += 1;
        if hit {
            self.local_hits += 1;
        }
    }

    /// Record remote access
    #[inline]
    pub fn record_remote(&mut self, hit: bool) {
        self.remote_accesses += 1;
        if hit {
            self.remote_hits += 1;
        }
    }

    /// Record migration
    #[inline(always)]
    pub fn record_migration(&mut self, bytes: u64) {
        self.migrations += 1;
        self.migration_bytes += bytes;
    }
}

// ============================================================================
// PER-NODE STATISTICS
// ============================================================================

/// Per-node statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct NodeStats {
    /// Memory allocations
    pub allocations: u64,
    /// Memory frees
    pub frees: u64,
    /// Total allocated bytes
    pub allocated_bytes: u64,
    /// Total freed bytes
    pub freed_bytes: u64,
    /// Page faults
    pub page_faults: u64,
    /// TLB misses
    pub tlb_misses: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Average access latency
    pub avg_latency_ns: f64,
}

impl NodeStats {
    /// Record allocation
    #[inline(always)]
    pub fn record_alloc(&mut self, bytes: u64) {
        self.allocations += 1;
        self.allocated_bytes += bytes;
    }

    /// Record free
    #[inline(always)]
    pub fn record_free(&mut self, bytes: u64) {
        self.frees += 1;
        self.freed_bytes += bytes;
    }

    /// Record latency
    #[inline(always)]
    pub fn record_latency(&mut self, latency_ns: u64) {
        let alpha = 0.1;
        self.avg_latency_ns = alpha * latency_ns as f64 + (1.0 - alpha) * self.avg_latency_ns;
    }

    /// Net allocation
    #[inline(always)]
    pub fn net_allocation(&self) -> i64 {
        self.allocated_bytes as i64 - self.freed_bytes as i64
    }
}
