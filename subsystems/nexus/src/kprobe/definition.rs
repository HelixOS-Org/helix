//! Kprobe Definition
//!
//! Kprobe and kretprobe structures.

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{KprobeId, KprobeState, KretprobeId, ProbeAddress, SymbolInfo};

/// Kprobe definition
#[derive(Debug, Clone)]
pub struct KprobeDef {
    /// Kprobe ID
    pub id: KprobeId,
    /// Target address
    pub address: ProbeAddress,
    /// Symbol (if known)
    pub symbol: Option<SymbolInfo>,
    /// Offset into function
    pub offset: u64,
    /// Current state
    pub state: KprobeState,
    /// Original instruction bytes
    pub original_insn: Vec<u8>,
    /// Hit count
    pub hits: AtomicU64,
    /// Miss count (filtered)
    pub misses: AtomicU64,
    /// Registered timestamp
    pub registered_at: u64,
    /// Pre-handler present
    pub has_pre_handler: bool,
    /// Post-handler present
    pub has_post_handler: bool,
    /// Fault handler present
    pub has_fault_handler: bool,
}

impl KprobeDef {
    /// Create new kprobe definition
    pub fn new(id: KprobeId, address: ProbeAddress, timestamp: u64) -> Self {
        Self {
            id,
            address,
            symbol: None,
            offset: 0,
            state: KprobeState::Registered,
            original_insn: Vec::new(),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            registered_at: timestamp,
            has_pre_handler: false,
            has_post_handler: false,
            has_fault_handler: false,
        }
    }

    /// Record hit
    pub fn hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record miss
    pub fn miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Get hit count
    pub fn hit_count(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    /// Get miss count
    pub fn miss_count(&self) -> u64 {
        self.misses.load(Ordering::Relaxed)
    }

    /// Is armed
    pub fn is_armed(&self) -> bool {
        matches!(self.state, KprobeState::Armed)
    }
}

/// Kretprobe definition
#[derive(Debug, Clone)]
pub struct KretprobeDef {
    /// Kretprobe ID
    pub id: KretprobeId,
    /// Associated kprobe ID
    pub kprobe_id: KprobeId,
    /// Maximum active instances
    pub maxactive: u32,
    /// Current active instances
    pub active: AtomicU64,
    /// Number of missed (exhausted pool)
    pub nmissed: AtomicU64,
    /// Entry handler present
    pub has_entry_handler: bool,
    /// Return handler present
    pub has_return_handler: bool,
}

impl KretprobeDef {
    /// Create new kretprobe definition
    pub fn new(id: KretprobeId, kprobe_id: KprobeId, maxactive: u32) -> Self {
        Self {
            id,
            kprobe_id,
            maxactive,
            active: AtomicU64::new(0),
            nmissed: AtomicU64::new(0),
            has_entry_handler: false,
            has_return_handler: true,
        }
    }

    /// Acquire instance
    pub fn acquire(&self) -> bool {
        let current = self.active.load(Ordering::Relaxed);
        if current >= self.maxactive as u64 {
            self.nmissed.fetch_add(1, Ordering::Relaxed);
            return false;
        }
        self.active.fetch_add(1, Ordering::Relaxed);
        true
    }

    /// Release instance
    pub fn release(&self) {
        self.active.fetch_sub(1, Ordering::Relaxed);
    }

    /// Get active count
    pub fn active_count(&self) -> u64 {
        self.active.load(Ordering::Relaxed)
    }

    /// Get missed count
    pub fn missed_count(&self) -> u64 {
        self.nmissed.load(Ordering::Relaxed)
    }
}
