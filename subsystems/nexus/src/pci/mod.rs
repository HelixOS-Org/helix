//! PCI Intelligence Module
//!
//! This module provides AI-powered PCI subsystem analysis including device enumeration,
//! configuration space management, BAR allocation, capability parsing, and intelligent
//! bus topology analysis.

mod bar;
mod bus;
mod capabilities;
mod device;
mod intelligence;
mod manager;
mod pcie;
mod types;

// Re-export types
pub use types::{classes, vendors, ClassCode, PciDeviceId, PciDeviceType, ProductId, VendorId};

// Re-export BAR
pub use bar::{Bar, BarFlags, BarType};

// Re-export capabilities
pub use capabilities::{CapabilityId, ExtCapability, ExtCapabilityId, PciCapability};

// Re-export PCIe
pub use pcie::{PcieLink, PcieLinkSpeed, PcieLinkWidth, PowerState};

// Re-export device
pub use device::PciDevice;

// Re-export bus
pub use bus::PciBus;

// Re-export manager
pub use manager::PciManager;

// Re-export intelligence
pub use intelligence::{
    PciAction, PciAnalysis, PciIntelligence, PciIssue, PciIssueType, PciRecommendation,
};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_id() {
        let dev = PciDeviceId::from_bdf(0x00, 0x1f, 0x03);
        assert_eq!(dev.bus, 0x00);
        assert_eq!(dev.device, 0x1f);
        assert_eq!(dev.function, 0x03);
    }

    #[test]
    fn test_pcie_link_bandwidth() {
        let link = PcieLink {
            speed: PcieLinkSpeed::Gen4,
            width: PcieLinkWidth::X16,
            max_speed: PcieLinkSpeed::Gen4,
            max_width: PcieLinkWidth::X16,
            active: true,
        };

        // Gen4 x16 = 1969 MB/s * 16 = 31504 MB/s
        assert!(link.bandwidth() > 30000);
    }

    #[test]
    fn test_pci_device() {
        let dev = PciDevice::new(
            PciDeviceId::from_bdf(0, 2, 0),
            vendors::NVIDIA,
            ProductId(0x2204),
            classes::GPU_3D,
        );

        assert_eq!(dev.class_description(), "Display");
    }

    #[test]
    fn test_pci_intelligence() {
        let mut intel = PciIntelligence::new();

        let dev = PciDevice::new(
            PciDeviceId::from_bdf(0, 2, 0),
            vendors::NVIDIA,
            ProductId(0x2204),
            classes::GPU_3D,
        );

        intel.register_device(dev);

        let analysis = intel.analyze();
        // No driver should be flagged
        assert!(analysis.issues.len() > 0);
    }
}
