//! Atomic operations and memory prefetch.

use core::sync::atomic::{AtomicU64, Ordering};

/// Advanced atomic operations
pub struct AtomicOps {
    /// Supports 128-bit atomics
    has_cmpxchg16b: bool,
    /// Supports wait/wake
    has_wait: bool,
}

impl AtomicOps {
    /// Create new atomic ops
    pub fn new() -> Self {
        Self {
            has_cmpxchg16b: Self::detect_cmpxchg16b(),
            has_wait: Self::detect_wait(),
        }
    }

    fn detect_cmpxchg16b() -> bool {
        cfg!(target_arch = "x86_64")
    }

    fn detect_wait() -> bool {
        // Linux futex-style wait
        true
    }

    /// Has 128-bit CAS?
    #[inline(always)]
    pub fn has_cmpxchg16b(&self) -> bool {
        self.has_cmpxchg16b
    }

    /// Has wait/wake?
    #[inline(always)]
    pub fn has_wait(&self) -> bool {
        self.has_wait
    }

    /// Fetch-and-add
    #[inline(always)]
    pub fn fetch_add(val: &AtomicU64, add: u64, order: Ordering) -> u64 {
        val.fetch_add(add, order)
    }

    /// Compare-and-swap
    #[inline(always)]
    pub fn compare_exchange(
        val: &AtomicU64,
        current: u64,
        new: u64,
        success: Ordering,
        failure: Ordering,
    ) -> Result<u64, u64> {
        val.compare_exchange(current, new, success, failure)
    }

    /// Spin wait hint
    #[inline(always)]
    pub fn spin_hint() {
        #[cfg(target_arch = "x86_64")]
        {
            core::hint::spin_loop();
        }
        #[cfg(target_arch = "aarch64")]
        {
            core::hint::spin_loop();
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            core::hint::spin_loop();
        }
    }
}

impl Default for AtomicOps {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory prefetch hints
pub struct Prefetch;

impl Prefetch {
    /// Prefetch for read
    #[inline(always)]
    pub fn read<T>(ptr: *const T) {
        #[cfg(target_arch = "x86_64")]
        {
            // PREFETCHT0
            let _ = ptr; // Would use intrinsic
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            let _ = ptr;
        }
    }

    /// Prefetch for write
    #[inline(always)]
    pub fn write<T>(ptr: *mut T) {
        #[cfg(target_arch = "x86_64")]
        {
            // PREFETCHW
            let _ = ptr;
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            let _ = ptr;
        }
    }

    /// Prefetch non-temporal (streaming)
    #[inline(always)]
    pub fn non_temporal<T>(ptr: *const T) {
        #[cfg(target_arch = "x86_64")]
        {
            // PREFETCHNTA
            let _ = ptr;
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            let _ = ptr;
        }
    }
}
