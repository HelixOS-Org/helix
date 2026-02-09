//! Performance configuration.

/// Configuration for performance optimizations
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    /// Enable SIMD optimizations
    pub enable_simd: bool,

    /// Enable lock-free data structures
    pub enable_lockfree: bool,

    /// Enable branch prediction hints
    pub enable_branch_hints: bool,

    /// Enable prefetching
    pub enable_prefetch: bool,

    /// Cache line size (bytes)
    pub cache_line_size: usize,

    /// Number of CPU cores to use (0 = all available)
    pub max_cores: usize,

    /// Enable NUMA awareness
    pub numa_aware: bool,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enable_simd: true,
            enable_lockfree: true,
            enable_branch_hints: true,
            enable_prefetch: true,
            cache_line_size: 64,
            max_cores: 0,
            numa_aware: false,
        }
    }
}

impl PerformanceConfig {
    /// Aggressive performance configuration
    #[inline]
    pub fn aggressive() -> Self {
        Self {
            enable_simd: true,
            enable_lockfree: true,
            enable_branch_hints: true,
            enable_prefetch: true,
            cache_line_size: 64,
            max_cores: 0,
            numa_aware: true,
        }
    }
}
