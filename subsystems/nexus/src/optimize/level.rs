//! Optimization levels and targets

#![allow(dead_code)]

// ============================================================================
// OPTIMIZATION LEVEL
// ============================================================================

/// Optimization level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OptimizationLevel {
    /// No optimization (baseline)
    None       = 0,
    /// Light optimization (safe defaults)
    Light      = 1,
    /// Moderate optimization (balanced)
    Moderate   = 2,
    /// Aggressive optimization (performance focus)
    Aggressive = 3,
    /// Maximum optimization (all-out performance)
    Maximum    = 4,
}

// ============================================================================
// OPTIMIZATION TARGET
// ============================================================================

/// What to optimize for
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationTarget {
    /// Optimize for latency
    Latency,
    /// Optimize for throughput
    Throughput,
    /// Optimize for memory usage
    Memory,
    /// Optimize for power efficiency
    Power,
    /// Balanced optimization
    Balanced,
}
