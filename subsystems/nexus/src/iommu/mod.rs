//! IOMMU Intelligence Module
//!
//! This module provides AI-powered IOMMU (Input/Output Memory Management Unit) analysis
//! and DMA protection management. It includes domain tracking, DMA mapping analysis,
//! isolation verification, and intelligent security recommendations.
//!
//! # Submodules
//!
//! - `types` - Core type definitions (IommuId, DomainId, DeviceId, IommuType, IommuState)
//! - `capabilities` - IOMMU capabilities and features
//! - `domain` - IOMMU domain management
//! - `dma` - DMA mapping and tracking
//! - `fault` - IOMMU fault tracking
//! - `unit` - IOMMU unit representation
//! - `manager` - IOMMU manager
//! - `intelligence` - Security analysis and recommendations

#![allow(dead_code)]

extern crate alloc;

// ============================================================================
// SUBMODULES
// ============================================================================

mod capabilities;
mod dma;
mod domain;
mod fault;
mod intelligence;
mod manager;
mod types;
mod unit;

// ============================================================================
// RE-EXPORTS
// ============================================================================

pub use capabilities::IommuCapabilities;
pub use dma::{DmaDirection, DmaMapping, DmaMappingTracker};
pub use domain::{DomainType, IommuDomain};
pub use fault::{FaultTracker, FaultType, IommuFault};
pub use intelligence::{
    IommuAction, IommuAnalysis, IommuIntelligence, IommuIssue, IommuIssueType, IommuRecommendation,
};
pub use manager::IommuManager;
pub use types::{DeviceId, DomainId, IommuId, IommuState, IommuType};
pub use unit::IommuUnit;

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_id() {
        let dev = DeviceId::from_bdf(0x00, 0x1f, 0x00);
        assert_eq!(dev.bus, 0x00);
        assert_eq!(dev.device, 0x1f);
        assert_eq!(dev.function, 0x00);
    }

    #[test]
    fn test_iommu_domain() {
        let mut domain = IommuDomain::new(DomainId::new(1), DomainType::Dma, IommuId::new(1), 1000);

        let dev = DeviceId::from_bdf(0x00, 0x1f, 0x00);
        domain.attach_device(dev);
        assert_eq!(domain.device_count(), 1);

        domain.record_mapping(4096);
        assert_eq!(domain.mapping_count(), 1);
        assert_eq!(domain.mapped_bytes(), 4096);
    }

    #[test]
    fn test_dma_mapping_tracker() {
        let mut tracker = DmaMappingTracker::new();

        let mapping = DmaMapping::new(
            0x1000,
            0x10000,
            4096,
            DmaDirection::ToDevice,
            DeviceId::from_bdf(0, 0x1f, 0),
            DomainId::new(1),
            1000,
        );

        tracker.add_mapping(mapping);
        assert_eq!(tracker.total_mappings(), 1);

        tracker.remove_mapping(0x1000);
        assert_eq!(tracker.total_mappings(), 0);
    }

    #[test]
    fn test_iommu_unit() {
        let mut unit = IommuUnit::new(IommuId::new(1), IommuType::IntelVtd);

        let domain_id = unit.create_domain(DomainType::Dma, 1000);
        assert!(unit.get_domain(domain_id).is_some());

        let dev = DeviceId::from_bdf(0, 0x1f, 0);
        assert!(unit.attach_device(dev, domain_id));
    }

    #[test]
    fn test_iommu_intelligence() {
        let mut intel = IommuIntelligence::new();

        let id = intel.register_iommu(IommuType::IntelVtd);

        if let Some(unit) = intel.manager_mut().get_unit_mut(id) {
            unit.enable_translation();
        }

        let analysis = intel.analyze();
        assert!(analysis.security_score > 50.0);
    }
}
