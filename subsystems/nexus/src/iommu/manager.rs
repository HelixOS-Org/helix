//! IOMMU manager.

extern crate alloc;

use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::dma::{DmaMapping, DmaMappingTracker};
use super::fault::{FaultTracker, IommuFault};
use super::types::{DeviceId, IommuId, IommuType};
use super::unit::IommuUnit;

// ============================================================================
// IOMMU MANAGER
// ============================================================================

/// IOMMU manager
pub struct IommuManager {
    /// IOMMU units
    pub(crate) units: BTreeMap<IommuId, IommuUnit>,
    /// Mapping tracker
    mapping_tracker: DmaMappingTracker,
    /// Fault tracker
    pub(crate) fault_tracker: FaultTracker,
    /// Next unit ID
    next_unit_id: AtomicU64,
    /// Strict mode (fail if no IOMMU)
    strict_mode: AtomicBool,
}

impl IommuManager {
    /// Create new manager
    pub fn new() -> Self {
        Self {
            units: BTreeMap::new(),
            mapping_tracker: DmaMappingTracker::new(),
            fault_tracker: FaultTracker::new(10000),
            next_unit_id: AtomicU64::new(1),
            strict_mode: AtomicBool::new(false),
        }
    }

    /// Register IOMMU unit
    #[inline]
    pub fn register_unit(&mut self, iommu_type: IommuType) -> IommuId {
        let id = IommuId::new(self.next_unit_id.fetch_add(1, Ordering::Relaxed));
        let unit = IommuUnit::new(id, iommu_type);
        self.units.insert(id, unit);
        id
    }

    /// Get unit
    #[inline(always)]
    pub fn get_unit(&self, id: IommuId) -> Option<&IommuUnit> {
        self.units.get(&id)
    }

    /// Get unit mutably
    #[inline(always)]
    pub fn get_unit_mut(&mut self, id: IommuId) -> Option<&mut IommuUnit> {
        self.units.get_mut(&id)
    }

    /// Record DMA mapping
    #[inline]
    pub fn record_mapping(&mut self, mapping: DmaMapping) {
        if let Some(unit) = self.find_unit_for_device(mapping.device) {
            if let Some(domain) = unit.domains.get(&mapping.domain_id) {
                domain.record_mapping(mapping.size);
            }
        }
        self.mapping_tracker.add_mapping(mapping);
    }

    /// Record DMA unmap
    #[inline(always)]
    pub fn record_unmap(&mut self, iova: u64) {
        self.mapping_tracker.remove_mapping(iova);
    }

    /// Record fault
    #[inline(always)]
    pub fn record_fault(&mut self, fault: IommuFault) {
        self.fault_tracker.record(fault);
    }

    /// Find unit for device
    fn find_unit_for_device(&self, _device: DeviceId) -> Option<&IommuUnit> {
        // For now, return first unit
        self.units.values().next()
    }

    /// Get mapping tracker
    #[inline(always)]
    pub fn mapping_tracker(&self) -> &DmaMappingTracker {
        &self.mapping_tracker
    }

    /// Get fault tracker
    #[inline(always)]
    pub fn fault_tracker(&self) -> &FaultTracker {
        &self.fault_tracker
    }

    /// Set strict mode
    #[inline(always)]
    pub fn set_strict_mode(&self, strict: bool) {
        self.strict_mode.store(strict, Ordering::Relaxed);
    }

    /// Is strict mode
    #[inline(always)]
    pub fn is_strict_mode(&self) -> bool {
        self.strict_mode.load(Ordering::Relaxed)
    }

    /// Get unit count
    #[inline(always)]
    pub fn unit_count(&self) -> usize {
        self.units.len()
    }

    /// Has IOMMU support
    #[inline(always)]
    pub fn has_iommu(&self) -> bool {
        !self.units.is_empty()
    }
}

impl Default for IommuManager {
    fn default() -> Self {
        Self::new()
    }
}
