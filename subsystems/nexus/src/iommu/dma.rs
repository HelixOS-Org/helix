//! DMA mapping management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::types::{DeviceId, DomainId};

// ============================================================================
// DMA DIRECTION
// ============================================================================

/// DMA direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaDirection {
    /// To device
    ToDevice,
    /// From device
    FromDevice,
    /// Bidirectional
    Bidirectional,
    /// None
    None,
}

impl DmaDirection {
    /// Get direction name
    pub fn name(&self) -> &'static str {
        match self {
            Self::ToDevice => "to_device",
            Self::FromDevice => "from_device",
            Self::Bidirectional => "bidirectional",
            Self::None => "none",
        }
    }
}

// ============================================================================
// DMA MAPPING
// ============================================================================

/// DMA mapping
#[derive(Debug, Clone)]
pub struct DmaMapping {
    /// IOVA (I/O Virtual Address)
    pub iova: u64,
    /// Physical address
    pub paddr: u64,
    /// Size in bytes
    pub size: u64,
    /// Direction
    pub direction: DmaDirection,
    /// Device
    pub device: DeviceId,
    /// Domain
    pub domain_id: DomainId,
    /// Created timestamp
    pub created_at: u64,
    /// Is coherent
    pub coherent: bool,
}

impl DmaMapping {
    /// Create new mapping
    pub fn new(
        iova: u64,
        paddr: u64,
        size: u64,
        direction: DmaDirection,
        device: DeviceId,
        domain_id: DomainId,
        timestamp: u64,
    ) -> Self {
        Self {
            iova,
            paddr,
            size,
            direction,
            device,
            domain_id,
            created_at: timestamp,
            coherent: true,
        }
    }

    /// Get end address (IOVA)
    pub fn iova_end(&self) -> u64 {
        self.iova + self.size
    }

    /// Get end address (physical)
    pub fn paddr_end(&self) -> u64 {
        self.paddr + self.size
    }

    /// Overlaps with another mapping
    pub fn overlaps(&self, other: &DmaMapping) -> bool {
        self.iova < other.iova_end() && other.iova < self.iova_end()
    }
}

// ============================================================================
// DMA MAPPING TRACKER
// ============================================================================

/// DMA mapping tracker
pub struct DmaMappingTracker {
    /// Mappings by IOVA
    mappings: BTreeMap<u64, DmaMapping>,
    /// Total mappings
    total_mappings: AtomicU64,
    /// Total bytes mapped
    total_bytes: AtomicU64,
    /// Mapping operations
    map_ops: AtomicU64,
    /// Unmap operations
    unmap_ops: AtomicU64,
    /// Enabled
    enabled: AtomicBool,
}

impl DmaMappingTracker {
    /// Create new tracker
    pub fn new() -> Self {
        Self {
            mappings: BTreeMap::new(),
            total_mappings: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
            map_ops: AtomicU64::new(0),
            unmap_ops: AtomicU64::new(0),
            enabled: AtomicBool::new(true),
        }
    }

    /// Add mapping
    pub fn add_mapping(&mut self, mapping: DmaMapping) {
        self.total_mappings.fetch_add(1, Ordering::Relaxed);
        self.total_bytes.fetch_add(mapping.size, Ordering::Relaxed);
        self.map_ops.fetch_add(1, Ordering::Relaxed);
        self.mappings.insert(mapping.iova, mapping);
    }

    /// Remove mapping by IOVA
    pub fn remove_mapping(&mut self, iova: u64) -> Option<DmaMapping> {
        if let Some(mapping) = self.mappings.remove(&iova) {
            self.total_mappings.fetch_sub(1, Ordering::Relaxed);
            self.total_bytes.fetch_sub(mapping.size, Ordering::Relaxed);
            self.unmap_ops.fetch_add(1, Ordering::Relaxed);
            Some(mapping)
        } else {
            None
        }
    }

    /// Get mapping
    pub fn get_mapping(&self, iova: u64) -> Option<&DmaMapping> {
        self.mappings.get(&iova)
    }

    /// Get mappings for device
    pub fn mappings_for_device(&self, device: DeviceId) -> Vec<&DmaMapping> {
        self.mappings
            .values()
            .filter(|m| m.device == device)
            .collect()
    }

    /// Get total mappings
    pub fn total_mappings(&self) -> u64 {
        self.total_mappings.load(Ordering::Relaxed)
    }

    /// Get total bytes
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes.load(Ordering::Relaxed)
    }

    /// Get map ops
    pub fn map_ops(&self) -> u64 {
        self.map_ops.load(Ordering::Relaxed)
    }

    /// Get unmap ops
    pub fn unmap_ops(&self) -> u64 {
        self.unmap_ops.load(Ordering::Relaxed)
    }

    /// Enable/disable
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }
}

impl Default for DmaMappingTracker {
    fn default() -> Self {
        Self::new()
    }
}
