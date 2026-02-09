//! Atomic counter and gauge metrics.

extern crate alloc;

use alloc::string::String;
use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// COUNTER
// ============================================================================

/// Atomic counter metric
#[repr(align(64))]
pub struct Counter {
    /// Name
    pub name: String,
    /// Value
    value: AtomicU64,
    /// Description
    description: String,
}

impl Counter {
    /// Create new counter
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: AtomicU64::new(0),
            description: String::new(),
        }
    }

    /// Set description
    #[inline(always)]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Increment by 1
    #[inline(always)]
    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment by amount
    #[inline(always)]
    pub fn add(&self, amount: u64) {
        self.value.fetch_add(amount, Ordering::Relaxed);
    }

    /// Get value
    #[inline(always)]
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    /// Reset to zero
    #[inline(always)]
    pub fn reset(&self) {
        self.value.store(0, Ordering::Relaxed);
    }
}

// ============================================================================
// GAUGE
// ============================================================================

/// Atomic gauge metric
pub struct Gauge {
    /// Name
    pub name: String,
    /// Value (stored as bits)
    value: AtomicU64,
    /// Description
    description: String,
}

impl Gauge {
    /// Create new gauge
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: AtomicU64::new(0),
            description: String::new(),
        }
    }

    /// Set description
    #[inline(always)]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set value
    #[inline(always)]
    pub fn set(&self, value: f64) {
        let bits = value.to_bits();
        self.value.store(bits, Ordering::Relaxed);
    }

    /// Get value
    #[inline(always)]
    pub fn get(&self) -> f64 {
        let bits = self.value.load(Ordering::Relaxed);
        f64::from_bits(bits)
    }

    /// Increment by amount
    pub fn add(&self, amount: f64) {
        loop {
            let old_bits = self.value.load(Ordering::Relaxed);
            let old_val = f64::from_bits(old_bits);
            let new_val = old_val + amount;
            let new_bits = new_val.to_bits();

            if self
                .value
                .compare_exchange_weak(old_bits, new_bits, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                break;
            }
        }
    }

    /// Decrement by amount
    #[inline(always)]
    pub fn sub(&self, amount: f64) {
        self.add(-amount);
    }

    /// Set to maximum of current and new value
    pub fn set_max(&self, value: f64) {
        loop {
            let old_bits = self.value.load(Ordering::Relaxed);
            let old_val = f64::from_bits(old_bits);
            if value <= old_val {
                break;
            }
            let new_bits = value.to_bits();

            if self
                .value
                .compare_exchange_weak(old_bits, new_bits, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                break;
            }
        }
    }

    /// Set to minimum of current and new value
    pub fn set_min(&self, value: f64) {
        loop {
            let old_bits = self.value.load(Ordering::Relaxed);
            let old_val = f64::from_bits(old_bits);
            if value >= old_val {
                break;
            }
            let new_bits = value.to_bits();

            if self
                .value
                .compare_exchange_weak(old_bits, new_bits, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                break;
            }
        }
    }
}
