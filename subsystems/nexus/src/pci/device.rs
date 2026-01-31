//! PCI device representation.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::bar::{Bar, BarType};
use super::capabilities::{CapabilityId, ExtCapability, ExtCapabilityId, PciCapability};
use super::pcie::{PcieLink, PowerState};
use super::types::{ClassCode, PciDeviceId, PciDeviceType, ProductId, VendorId};

// ============================================================================
// PCI DEVICE
// ============================================================================

/// PCI device
#[derive(Debug)]
pub struct PciDevice {
    /// Device ID (BDF)
    pub id: PciDeviceId,
    /// Vendor ID
    pub vendor: VendorId,
    /// Product ID
    pub product: ProductId,
    /// Class code
    pub class: ClassCode,
    /// Revision ID
    pub revision: u8,
    /// Device type
    pub device_type: PciDeviceType,
    /// Is multifunction
    pub multifunction: bool,
    /// BARs
    pub bars: Vec<Bar>,
    /// Capabilities
    pub capabilities: Vec<PciCapability>,
    /// Extended capabilities
    pub ext_capabilities: Vec<ExtCapability>,
    /// PCIe link (if PCIe)
    pub pcie_link: Option<PcieLink>,
    /// Power state
    pub power_state: PowerState,
    /// Has driver
    pub has_driver: bool,
    /// Driver name
    pub driver_name: Option<String>,
    /// Interrupt line
    pub irq: Option<u8>,
    /// MSI/MSI-X enabled
    pub msi_enabled: bool,
    /// IOMMU domain ID
    pub iommu_domain: Option<u64>,
    /// Parent bridge
    pub parent: Option<PciDeviceId>,
    /// Read count
    pub config_reads: AtomicU64,
    /// Write count
    pub config_writes: AtomicU64,
}

impl PciDevice {
    /// Create new device
    pub fn new(id: PciDeviceId, vendor: VendorId, product: ProductId, class: ClassCode) -> Self {
        Self {
            id,
            vendor,
            product,
            class,
            revision: 0,
            device_type: PciDeviceType::Unknown,
            multifunction: false,
            bars: Vec::new(),
            capabilities: Vec::new(),
            ext_capabilities: Vec::new(),
            pcie_link: None,
            power_state: PowerState::D0,
            has_driver: false,
            driver_name: None,
            irq: None,
            msi_enabled: false,
            iommu_domain: None,
            parent: None,
            config_reads: AtomicU64::new(0),
            config_writes: AtomicU64::new(0),
        }
    }

    /// Is PCIe device
    pub fn is_pcie(&self) -> bool {
        self.capabilities.iter().any(|c| c.id == CapabilityId::PCIE)
    }

    /// Has capability
    pub fn has_capability(&self, id: CapabilityId) -> bool {
        self.capabilities.iter().any(|c| c.id == id)
    }

    /// Has extended capability
    pub fn has_ext_capability(&self, id: ExtCapabilityId) -> bool {
        self.ext_capabilities.iter().any(|c| c.id == id)
    }

    /// Get total BAR memory
    pub fn total_bar_memory(&self) -> u64 {
        self.bars
            .iter()
            .filter(|b| b.bar_type.is_memory())
            .map(|b| b.size)
            .sum()
    }

    /// Get total BAR I/O
    pub fn total_bar_io(&self) -> u64 {
        self.bars
            .iter()
            .filter(|b| matches!(b.bar_type, BarType::Io))
            .map(|b| b.size)
            .sum()
    }

    /// Record config read
    pub fn record_read(&self) {
        self.config_reads.fetch_add(1, Ordering::Relaxed);
    }

    /// Record config write
    pub fn record_write(&self) {
        self.config_writes.fetch_add(1, Ordering::Relaxed);
    }

    /// Get class description
    pub fn class_description(&self) -> &'static str {
        match self.class.class {
            0x00 => "Unclassified",
            0x01 => "Mass Storage",
            0x02 => "Network",
            0x03 => "Display",
            0x04 => "Multimedia",
            0x05 => "Memory",
            0x06 => "Bridge",
            0x07 => "Communication",
            0x08 => "System Peripheral",
            0x09 => "Input",
            0x0a => "Docking",
            0x0b => "Processor",
            0x0c => "Serial Bus",
            0x0d => "Wireless",
            0x0e => "Intelligent I/O",
            0x0f => "Satellite",
            0x10 => "Encryption",
            0x11 => "Signal Processing",
            0x12 => "Processing Accelerator",
            0xff => "Vendor Specific",
            _ => "Unknown",
        }
    }
}
