//! Function information and tracking.

use alloc::string::String;
use core::sync::atomic::{AtomicU64, Ordering};

use super::types::FuncAddr;

// ============================================================================
// FUNCTION INFORMATION
// ============================================================================

/// Function info
#[derive(Debug)]
pub struct FunctionInfo {
    /// Address
    pub addr: FuncAddr,
    /// Name
    pub name: String,
    /// Module
    pub module: Option<String>,
    /// Size
    pub size: u32,
    /// Is inline
    pub is_inline: bool,
    /// Hit count
    hit_count: AtomicU64,
    /// Total time (ns)
    total_time_ns: AtomicU64,
}

impl FunctionInfo {
    /// Create new function info
    pub fn new(addr: FuncAddr, name: String) -> Self {
        Self {
            addr,
            name,
            module: None,
            size: 0,
            is_inline: false,
            hit_count: AtomicU64::new(0),
            total_time_ns: AtomicU64::new(0),
        }
    }

    /// Record hit
    pub fn record_hit(&self, duration_ns: u64) {
        self.hit_count.fetch_add(1, Ordering::Relaxed);
        self.total_time_ns.fetch_add(duration_ns, Ordering::Relaxed);
    }

    /// Get hit count
    pub fn hit_count(&self) -> u64 {
        self.hit_count.load(Ordering::Relaxed)
    }

    /// Get total time
    pub fn total_time_ns(&self) -> u64 {
        self.total_time_ns.load(Ordering::Relaxed)
    }

    /// Average time
    pub fn avg_time_ns(&self) -> u64 {
        let hits = self.hit_count();
        if hits == 0 {
            return 0;
        }
        self.total_time_ns() / hits
    }

    /// Full name
    pub fn full_name(&self) -> String {
        if let Some(ref module) = self.module {
            alloc::format!("{}:{}", module, self.name)
        } else {
            self.name.clone()
        }
    }
}
