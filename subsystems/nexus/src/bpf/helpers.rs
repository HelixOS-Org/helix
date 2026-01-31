//! BPF Helper Functions
//!
//! BPF helper function tracking and management.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::BpfProgType;

/// BPF helper function ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BpfHelperId(pub u32);

impl BpfHelperId {
    /// Create new helper ID
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get raw ID
    pub const fn raw(&self) -> u32 {
        self.0
    }
}

/// BPF helper info
#[derive(Debug)]
pub struct BpfHelperInfo {
    /// Helper ID
    pub id: BpfHelperId,
    /// Helper name
    pub name: String,
    /// Number of arguments
    pub num_args: u8,
    /// Return type
    pub return_type: String,
    /// Allowed program types
    pub allowed_prog_types: Vec<BpfProgType>,
    /// Call count
    pub call_count: AtomicU64,
    /// Total execution time (ns)
    pub total_time_ns: AtomicU64,
}

impl BpfHelperInfo {
    /// Create new helper info
    pub fn new(id: BpfHelperId, name: String, num_args: u8) -> Self {
        Self {
            id,
            name,
            num_args,
            return_type: String::from("u64"),
            allowed_prog_types: Vec::new(),
            call_count: AtomicU64::new(0),
            total_time_ns: AtomicU64::new(0),
        }
    }

    /// Record call
    pub fn record_call(&self, duration_ns: u64) {
        self.call_count.fetch_add(1, Ordering::Relaxed);
        self.total_time_ns.fetch_add(duration_ns, Ordering::Relaxed);
    }

    /// Get call count
    pub fn get_call_count(&self) -> u64 {
        self.call_count.load(Ordering::Relaxed)
    }

    /// Get total time
    pub fn get_total_time(&self) -> u64 {
        self.total_time_ns.load(Ordering::Relaxed)
    }

    /// Get average time
    pub fn avg_time(&self) -> f32 {
        let count = self.get_call_count();
        if count == 0 {
            return 0.0;
        }
        self.total_time_ns.load(Ordering::Relaxed) as f32 / count as f32
    }

    /// Add allowed program type
    pub fn add_allowed_prog_type(&mut self, prog_type: BpfProgType) {
        if !self.allowed_prog_types.contains(&prog_type) {
            self.allowed_prog_types.push(prog_type);
        }
    }

    /// Is allowed for program type
    pub fn is_allowed_for(&self, prog_type: BpfProgType) -> bool {
        self.allowed_prog_types.is_empty() || self.allowed_prog_types.contains(&prog_type)
    }
}
