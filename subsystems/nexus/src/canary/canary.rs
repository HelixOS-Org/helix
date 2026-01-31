//! Canary values for memory corruption detection.

use core::sync::atomic::{AtomicU64, Ordering};

/// A canary value for memory corruption detection
#[derive(Debug)]
pub struct Canary {
    /// Expected value
    expected: u64,
    /// Current value
    pub(crate) value: AtomicU64,
}

impl Canary {
    /// Create a new canary
    pub fn new(value: u64) -> Self {
        Self {
            expected: value,
            value: AtomicU64::new(value),
        }
    }

    /// Check if canary is intact
    pub fn check(&self) -> bool {
        self.value.load(Ordering::SeqCst) == self.expected
    }

    /// Get the current value
    pub fn value(&self) -> u64 {
        self.value.load(Ordering::SeqCst)
    }

    /// Get expected value
    pub fn expected(&self) -> u64 {
        self.expected
    }

    /// Reset canary to expected value
    pub fn reset(&self) {
        self.value.store(self.expected, Ordering::SeqCst);
    }
}

impl Default for Canary {
    fn default() -> Self {
        Self::new(0xDEADBEEF_CAFEBABE)
    }
}
