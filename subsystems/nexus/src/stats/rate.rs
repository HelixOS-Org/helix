//! Rate meter for measuring events per time unit

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// RATE METER
// ============================================================================

/// Meter for measuring rates (events per second, etc.)
pub struct RateMeter {
    /// Samples in the window
    samples: Vec<AtomicU64>,
    /// Current sample index
    current_index: AtomicU64,
    /// Window size
    window_size: usize,
    /// Sample interval in cycles
    sample_interval: u64,
    /// Last sample timestamp
    last_sample: AtomicU64,
}

impl RateMeter {
    /// Create a new rate meter
    pub fn new(window_size: usize, sample_interval: u64) -> Self {
        let mut samples = Vec::with_capacity(window_size);
        for _ in 0..window_size {
            samples.push(AtomicU64::new(0));
        }
        Self {
            samples,
            current_index: AtomicU64::new(0),
            window_size,
            sample_interval,
            last_sample: AtomicU64::new(0),
        }
    }

    /// Record an event
    #[inline(always)]
    pub fn mark(&self) {
        let index = self.current_index.load(Ordering::Relaxed) as usize % self.window_size;
        self.samples[index].fetch_add(1, Ordering::Relaxed);
    }

    /// Advance to next sample window if needed
    pub fn tick(&self, now: u64) {
        let last = self.last_sample.load(Ordering::Relaxed);
        if now - last >= self.sample_interval {
            if self
                .last_sample
                .compare_exchange(last, now, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
            {
                // Move to next index
                let new_index = self.current_index.fetch_add(1, Ordering::Relaxed) + 1;
                let next_slot = new_index as usize % self.window_size;
                self.samples[next_slot].store(0, Ordering::Relaxed);
            }
        }
    }

    /// Get current rate (events per sample interval)
    pub fn rate(&self) -> f64 {
        let mut total = 0u64;
        let mut count = 0u64;

        for sample in &self.samples {
            let v = sample.load(Ordering::Relaxed);
            if v > 0 {
                total += v;
                count += 1;
            }
        }

        if count == 0 {
            0.0
        } else {
            total as f64 / count as f64
        }
    }

    /// Get total events in window
    #[inline(always)]
    pub fn total(&self) -> u64 {
        self.samples.iter().map(|s| s.load(Ordering::Relaxed)).sum()
    }
}
