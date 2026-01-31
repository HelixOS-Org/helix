//! BPF Program
//!
//! BPF program state and information structures.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{BpfMapId, BpfProgId, BpfProgType, BtfId};

/// BPF program state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpfProgState {
    /// Created but not loaded
    Created,
    /// Loaded and verified
    Loaded,
    /// JIT compiled
    JitCompiled,
    /// Attached to hook
    Attached,
    /// Detached
    Detached,
    /// Error state
    Error,
}

/// BPF program info
#[derive(Debug)]
pub struct BpfProgInfo {
    /// Program ID
    pub id: BpfProgId,
    /// Program type
    pub prog_type: BpfProgType,
    /// Program name
    pub name: String,
    /// Program tag (hash)
    pub tag: [u8; 8],
    /// Current state
    pub state: BpfProgState,
    /// Number of instructions
    pub insn_count: u32,
    /// JIT compiled size
    pub jit_size: u32,
    /// Created timestamp
    pub created_at: u64,
    /// Last run timestamp
    pub last_run: u64,
    /// Run count
    pub run_count: AtomicU64,
    /// Run time total (ns)
    pub run_time_ns: AtomicU64,
    /// Associated maps
    pub maps: Vec<BpfMapId>,
    /// BTF ID (if present)
    pub btf_id: Option<BtfId>,
    /// License
    pub license: String,
    /// Verified
    pub verified: bool,
}

impl BpfProgInfo {
    /// Create new program info
    pub fn new(id: BpfProgId, prog_type: BpfProgType, name: String, timestamp: u64) -> Self {
        Self {
            id,
            prog_type,
            name,
            tag: [0; 8],
            state: BpfProgState::Created,
            insn_count: 0,
            jit_size: 0,
            created_at: timestamp,
            last_run: 0,
            run_count: AtomicU64::new(0),
            run_time_ns: AtomicU64::new(0),
            maps: Vec::new(),
            btf_id: None,
            license: String::from("GPL"),
            verified: false,
        }
    }

    /// Record run
    pub fn record_run(&self, duration_ns: u64, _timestamp: u64) {
        self.run_count.fetch_add(1, Ordering::Relaxed);
        self.run_time_ns.fetch_add(duration_ns, Ordering::Relaxed);
    }

    /// Get run count
    pub fn get_run_count(&self) -> u64 {
        self.run_count.load(Ordering::Relaxed)
    }

    /// Get total run time
    pub fn get_run_time(&self) -> u64 {
        self.run_time_ns.load(Ordering::Relaxed)
    }

    /// Get average run time
    pub fn avg_run_time(&self) -> f32 {
        let count = self.get_run_count();
        if count == 0 {
            return 0.0;
        }
        self.get_run_time() as f32 / count as f32
    }
}
