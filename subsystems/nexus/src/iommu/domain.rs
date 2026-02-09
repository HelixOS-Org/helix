//! IOMMU domain management.

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::types::{DeviceId, DomainId, IommuId};

// ============================================================================
// DOMAIN TYPE
// ============================================================================

/// Domain type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainType {
    /// Unmanaged (direct hardware)
    Unmanaged,
    /// DMA domain (kernel managed)
    Dma,
    /// DMA FQ (fault queue)
    DmaFq,
    /// Identity (passthrough)
    Identity,
    /// Blocked
    Blocked,
    /// SVA (Shared Virtual Addressing)
    Sva,
    /// Unknown
    Unknown,
}

impl DomainType {
    /// Get type name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Unmanaged => "unmanaged",
            Self::Dma => "dma",
            Self::DmaFq => "dma-fq",
            Self::Identity => "identity",
            Self::Blocked => "blocked",
            Self::Sva => "sva",
            Self::Unknown => "unknown",
        }
    }

    /// Has translation
    #[inline(always)]
    pub fn has_translation(&self) -> bool {
        matches!(self, Self::Dma | Self::DmaFq | Self::Sva)
    }

    /// Is isolated
    #[inline(always)]
    pub fn is_isolated(&self) -> bool {
        matches!(self, Self::Dma | Self::DmaFq | Self::Sva | Self::Blocked)
    }
}

// ============================================================================
// IOMMU DOMAIN
// ============================================================================

/// IOMMU domain
#[derive(Debug)]
pub struct IommuDomain {
    /// Domain ID
    pub id: DomainId,
    /// Domain type
    pub domain_type: DomainType,
    /// Parent IOMMU
    pub iommu_id: IommuId,
    /// Attached devices
    pub devices: Vec<DeviceId>,
    /// IOVA space start
    pub iova_start: u64,
    /// IOVA space end
    pub iova_end: u64,
    /// Page table address
    pub pgd: u64,
    /// Address width (bits)
    pub addr_width: u8,
    /// Mapping count
    pub mapping_count: AtomicU64,
    /// Total mapped bytes
    pub mapped_bytes: AtomicU64,
    /// Created timestamp
    pub created_at: u64,
    /// Is default domain
    pub is_default: bool,
}

impl IommuDomain {
    /// Create new domain
    pub fn new(id: DomainId, domain_type: DomainType, iommu_id: IommuId, timestamp: u64) -> Self {
        Self {
            id,
            domain_type,
            iommu_id,
            devices: Vec::new(),
            iova_start: 0,
            iova_end: 0,
            pgd: 0,
            addr_width: 48,
            mapping_count: AtomicU64::new(0),
            mapped_bytes: AtomicU64::new(0),
            created_at: timestamp,
            is_default: false,
        }
    }

    /// Attach device
    #[inline]
    pub fn attach_device(&mut self, device: DeviceId) {
        if !self.devices.contains(&device) {
            self.devices.push(device);
        }
    }

    /// Detach device
    #[inline]
    pub fn detach_device(&mut self, device: DeviceId) -> bool {
        if let Some(pos) = self.devices.iter().position(|d| *d == device) {
            self.devices.remove(pos);
            true
        } else {
            false
        }
    }

    /// Record mapping
    #[inline(always)]
    pub fn record_mapping(&self, bytes: u64) {
        self.mapping_count.fetch_add(1, Ordering::Relaxed);
        self.mapped_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record unmapping
    #[inline]
    pub fn record_unmapping(&self, bytes: u64) {
        let count = self.mapping_count.load(Ordering::Relaxed);
        if count > 0 {
            self.mapping_count.fetch_sub(1, Ordering::Relaxed);
        }
        let mapped = self.mapped_bytes.load(Ordering::Relaxed);
        if mapped >= bytes {
            self.mapped_bytes.fetch_sub(bytes, Ordering::Relaxed);
        }
    }

    /// Get mapping count
    #[inline(always)]
    pub fn mapping_count(&self) -> u64 {
        self.mapping_count.load(Ordering::Relaxed)
    }

    /// Get mapped bytes
    #[inline(always)]
    pub fn mapped_bytes(&self) -> u64 {
        self.mapped_bytes.load(Ordering::Relaxed)
    }

    /// Get device count
    #[inline(always)]
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }
}
