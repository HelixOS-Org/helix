//! # PCI Device Discovery and Configuration
//!
//! NVIDIA GPU PCI enumeration and configuration space access.

use magma_core::{Error, GpuDeviceId, GpuGeneration, PciAddr, Result};

// =============================================================================
// NVIDIA VENDOR ID
// =============================================================================

/// NVIDIA PCI Vendor ID
pub const NVIDIA_VENDOR_ID: u16 = 0x10DE;

/// Known NVIDIA GPU Device IDs (sample - real list is much larger)
pub mod device_ids {
    //! NVIDIA Device ID constants

    // Turing (RTX 20xx)
    /// TU102 - RTX 2080 Ti
    pub const TU102_RTX2080TI: u16 = 0x1E04;
    /// TU104 - RTX 2080
    pub const TU104_RTX2080: u16 = 0x1E82;
    /// TU106 - RTX 2070
    pub const TU106_RTX2070: u16 = 0x1F02;

    // Ampere (RTX 30xx)
    /// GA102 - RTX 3090
    pub const GA102_RTX3090: u16 = 0x2204;
    /// GA102 - RTX 3080
    pub const GA102_RTX3080: u16 = 0x2206;
    /// GA104 - RTX 3070
    pub const GA104_RTX3070: u16 = 0x2484;

    // Ada Lovelace (RTX 40xx)
    /// AD102 - RTX 4090
    pub const AD102_RTX4090: u16 = 0x2684;
    /// AD103 - RTX 4080
    pub const AD103_RTX4080: u16 = 0x2704;
    /// AD104 - RTX 4070 Ti
    pub const AD104_RTX4070TI: u16 = 0x2782;

    // Blackwell (RTX 50xx)
    /// GB202 - RTX 5090 (placeholder)
    pub const GB202_RTX5090: u16 = 0x2900;
}

// =============================================================================
// PCI CONFIGURATION SPACE
// =============================================================================

/// PCI configuration space registers
#[derive(Debug, Clone, Copy)]
#[repr(u16)]
pub enum PciConfigReg {
    /// Vendor ID (16-bit)
    VendorId          = 0x00,
    /// Device ID (16-bit)
    DeviceId          = 0x02,
    /// Command register
    Command           = 0x04,
    /// Status register
    Status            = 0x06,
    /// Revision ID
    RevisionId        = 0x08,
    /// Class code
    ClassCode         = 0x09,
    /// Cache line size
    CacheLineSize     = 0x0C,
    /// Latency timer
    LatencyTimer      = 0x0D,
    /// Header type
    HeaderType        = 0x0E,
    /// BAR0
    Bar0              = 0x10,
    /// BAR1
    Bar1              = 0x14,
    /// BAR2
    Bar2              = 0x18,
    /// BAR3
    Bar3              = 0x1C,
    /// BAR4
    Bar4              = 0x20,
    /// BAR5
    Bar5              = 0x24,
    /// Subsystem Vendor ID
    SubsystemVendorId = 0x2C,
    /// Subsystem ID
    SubsystemId       = 0x2E,
    /// Expansion ROM Base
    ExpansionRomBase  = 0x30,
    /// Capabilities pointer
    CapabilitiesPtr   = 0x34,
    /// Interrupt line
    InterruptLine     = 0x3C,
    /// Interrupt pin
    InterruptPin      = 0x3D,
}

/// PCI command register flags
bitflags::bitflags! {
    /// PCI command register bits
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PciCommand: u16 {
        /// I/O space enable
        const IO_SPACE = 1 << 0;
        /// Memory space enable
        const MEMORY_SPACE = 1 << 1;
        /// Bus master enable
        const BUS_MASTER = 1 << 2;
        /// Special cycles
        const SPECIAL_CYCLES = 1 << 3;
        /// Memory write and invalidate
        const MEM_WRITE_INVALIDATE = 1 << 4;
        /// VGA palette snoop
        const VGA_PALETTE_SNOOP = 1 << 5;
        /// Parity error response
        const PARITY_ERROR_RESPONSE = 1 << 6;
        /// SERR# enable
        const SERR_ENABLE = 1 << 8;
        /// Fast back-to-back enable
        const FAST_B2B = 1 << 9;
        /// Interrupt disable
        const INTERRUPT_DISABLE = 1 << 10;
    }
}

// =============================================================================
// PCI DEVICE INFO
// =============================================================================

/// Information about a discovered PCI device
#[derive(Debug, Clone)]
pub struct PciDeviceInfo {
    /// PCI address (bus:device:function)
    pub address: PciAddr,
    /// Vendor ID
    pub vendor_id: u16,
    /// Device ID
    pub device_id: u16,
    /// Revision ID
    pub revision: u8,
    /// Subsystem vendor ID
    pub subsystem_vendor: u16,
    /// Subsystem ID
    pub subsystem_id: u16,
    /// Detected GPU generation
    pub generation: Option<GpuGeneration>,
}

impl PciDeviceInfo {
    /// Check if this is an NVIDIA GPU
    pub fn is_nvidia_gpu(&self) -> bool {
        self.vendor_id == NVIDIA_VENDOR_ID
    }

    /// Get GPU device ID wrapper
    pub fn gpu_device_id(&self) -> GpuDeviceId {
        GpuDeviceId(self.device_id)
    }
}

// =============================================================================
// PCI DEVICE TRAIT
// =============================================================================

/// Trait for accessing a PCI device
pub trait PciDevice {
    /// Read 8-bit value from configuration space
    fn config_read8(&self, offset: u16) -> Result<u8>;

    /// Read 16-bit value from configuration space
    fn config_read16(&self, offset: u16) -> Result<u16>;

    /// Read 32-bit value from configuration space
    fn config_read32(&self, offset: u16) -> Result<u32>;

    /// Write 8-bit value to configuration space
    fn config_write8(&self, offset: u16, value: u8) -> Result<()>;

    /// Write 16-bit value to configuration space
    fn config_write16(&self, offset: u16, value: u16) -> Result<()>;

    /// Write 32-bit value to configuration space
    fn config_write32(&self, offset: u16, value: u32) -> Result<()>;

    /// Get device info
    fn info(&self) -> &PciDeviceInfo;

    /// Enable bus mastering
    fn enable_bus_master(&mut self) -> Result<()> {
        let cmd = self.config_read16(PciConfigReg::Command as u16)?;
        let new_cmd = cmd | PciCommand::BUS_MASTER.bits() | PciCommand::MEMORY_SPACE.bits();
        self.config_write16(PciConfigReg::Command as u16, new_cmd)
    }

    /// Find a PCI capability by ID
    fn find_capability(&self, cap_id: u8) -> Result<Option<u8>> {
        let status = self.config_read16(PciConfigReg::Status as u16)?;

        // Check if capabilities list is supported
        if (status & 0x10) == 0 {
            return Ok(None);
        }

        let mut cap_ptr = self.config_read8(PciConfigReg::CapabilitiesPtr as u16)?;

        while cap_ptr != 0 {
            let id = self.config_read8(cap_ptr as u16)?;
            if id == cap_id {
                return Ok(Some(cap_ptr));
            }
            cap_ptr = self.config_read8((cap_ptr + 1) as u16)?;
        }

        Ok(None)
    }

    /// Find MSI-X capability
    fn find_msix(&self) -> Result<Option<u8>> {
        self.find_capability(0x11) // MSI-X capability ID
    }

    /// Find PCIe capability
    fn find_pcie(&self) -> Result<Option<u8>> {
        self.find_capability(0x10) // PCI Express capability ID
    }
}

// =============================================================================
// PCI ENUMERATION
// =============================================================================

/// Enumerate NVIDIA GPUs on the PCI bus
pub trait PciEnumerator {
    /// Device type returned
    type Device: PciDevice;

    /// Enumerate all NVIDIA GPUs
    fn enumerate_nvidia_gpus(&self) -> Result<alloc::vec::Vec<Self::Device>>;

    /// Get device at specific address
    fn get_device(&self, addr: PciAddr) -> Result<Option<Self::Device>>;
}
