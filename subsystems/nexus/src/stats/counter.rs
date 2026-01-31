//! Atomic counter for thread-safe statistics

#![allow(dead_code)]

use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// ATOMIC COUNTERS
// ============================================================================

/// Atomic counter for thread-safe statistics
#[repr(align(64))] // Cache line aligned
pub struct AtomicCounter {
    value: AtomicU64,
}

impl AtomicCounter {
    /// Create a new counter with value 0
    pub const fn new() -> Self {
        Self {
            value: AtomicU64::new(0),
        }
    }

    /// Create a counter with an initial value
    pub const fn with_value(value: u64) -> Self {
        Self {
            value: AtomicU64::new(value),
        }
    }

    /// Increment by 1
    #[inline]
    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    /// Add a value
    #[inline]
    pub fn add(&self, delta: u64) {
        self.value.fetch_add(delta, Ordering::Relaxed);
    }

    /// Get current value
    #[inline]
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    /// Reset to 0
    #[inline]
    pub fn reset(&self) {
        self.value.store(0, Ordering::Relaxed);
    }

    /// Set to a specific value
    #[inline]
    pub fn set(&self, value: u64) {
        self.value.store(value, Ordering::Relaxed);
    }
}

impl Default for AtomicCounter {
    fn default() -> Self {
        Self::new()
    }
}
