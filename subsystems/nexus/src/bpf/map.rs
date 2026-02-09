//! BPF Map
//!
//! BPF map structures and management.

use alloc::string::String;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{BpfMapId, BpfMapType, BpfProgId};

/// BPF map flags
#[derive(Debug, Clone, Copy, Default)]
pub struct BpfMapFlags {
    /// No prealloc
    pub no_prealloc: bool,
    /// No common LRU
    pub no_common_lru: bool,
    /// NUMA node
    pub numa_node: bool,
    /// Read only for prog
    pub rdonly_prog: bool,
    /// Write only for prog
    pub wronly_prog: bool,
    /// Clone map
    pub clone: bool,
    /// Memory mapped
    pub mmapable: bool,
    /// Preserve on update
    pub preserve_elems: bool,
    /// Inner map
    pub inner_map: bool,
}

/// BPF map info
#[derive(Debug)]
pub struct BpfMapInfo {
    /// Map ID
    pub id: BpfMapId,
    /// Map type
    pub map_type: BpfMapType,
    /// Map name
    pub name: String,
    /// Key size
    pub key_size: u32,
    /// Value size
    pub value_size: u32,
    /// Max entries
    pub max_entries: u32,
    /// Current entries
    pub current_entries: AtomicU64,
    /// Map flags
    pub flags: BpfMapFlags,
    /// Created timestamp
    pub created_at: u64,
    /// Lookup count
    pub lookup_count: AtomicU64,
    /// Update count
    pub update_count: AtomicU64,
    /// Delete count
    pub delete_count: AtomicU64,
    /// BTF key type ID
    pub btf_key_type_id: Option<u32>,
    /// BTF value type ID
    pub btf_value_type_id: Option<u32>,
    /// Owner program
    pub owner_prog: Option<BpfProgId>,
}

impl BpfMapInfo {
    /// Create new map info
    pub fn new(
        id: BpfMapId,
        map_type: BpfMapType,
        name: String,
        key_size: u32,
        value_size: u32,
        max_entries: u32,
        timestamp: u64,
    ) -> Self {
        Self {
            id,
            map_type,
            name,
            key_size,
            value_size,
            max_entries,
            current_entries: AtomicU64::new(0),
            flags: BpfMapFlags::default(),
            created_at: timestamp,
            lookup_count: AtomicU64::new(0),
            update_count: AtomicU64::new(0),
            delete_count: AtomicU64::new(0),
            btf_key_type_id: None,
            btf_value_type_id: None,
            owner_prog: None,
        }
    }

    /// Record lookup
    #[inline(always)]
    pub fn record_lookup(&self) {
        self.lookup_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record update
    #[inline(always)]
    pub fn record_update(&self) {
        self.update_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record delete
    #[inline(always)]
    pub fn record_delete(&self) {
        self.delete_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get lookup count
    #[inline(always)]
    pub fn get_lookup_count(&self) -> u64 {
        self.lookup_count.load(Ordering::Relaxed)
    }

    /// Get update count
    #[inline(always)]
    pub fn get_update_count(&self) -> u64 {
        self.update_count.load(Ordering::Relaxed)
    }

    /// Get fill ratio
    #[inline(always)]
    pub fn fill_ratio(&self) -> f32 {
        let current = self.current_entries.load(Ordering::Relaxed);
        current as f32 / self.max_entries as f32
    }

    /// Estimated memory usage
    #[inline]
    pub fn estimated_memory(&self) -> u64 {
        let entry_size = self.key_size as u64 + self.value_size as u64;
        let entries = self.current_entries.load(Ordering::Relaxed);
        entry_size * entries
    }
}
