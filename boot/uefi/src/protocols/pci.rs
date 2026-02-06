//! PCI Protocol
//!
//! High-level PCI device abstraction for device enumeration and access.
//!
//! # Safety
//! This module interfaces with UEFI firmware via FFI. All protocol pointers
//! are provided by the firmware and validated before use.
// codeql[rust/access-invalid-pointer] - UEFI FFI pointers validated by firmware

use super::{EnumerableProtocol, Protocol};
use crate::error::{Error, Result};
use crate::raw::protocols::pci::*;
use crate::raw::types::*;

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

/// PCI I/O Protocol GUID
const PCI_IO_PROTOCOL_GUID: Guid = guids::PCI_IO_PROTOCOL;

// =============================================================================
// PCI DEVICE
// =============================================================================

/// High-level PCI device abstraction
pub struct PciDevice {
    /// Raw protocol pointer
    protocol: *mut EfiPciIoProtocol,
    /// Handle
    _handle: Handle,
    /// Cached config header
    config: PciConfigHeader,
    /// Location
    location: PciLocation,
}

impl PciDevice {
    /// Create from raw protocol
    ///
    /// # Safety
    /// Protocol pointer must be valid and obtained from UEFI LocateProtocol
    /// or HandleProtocol calls. The pointer must remain valid for the
    /// lifetime of this PciDevice instance.
    ///
    /// # Errors
    /// Returns error if the PCI location cannot be read from the device.
    pub unsafe fn from_raw(protocol: *mut EfiPciIoProtocol, handle: Handle) -> Result<Self> {
        debug_assert!(!protocol.is_null(), "PciDevice protocol pointer is null");

        let mut segment = 0usize;
        let mut bus = 0usize;
        let mut device = 0usize;
        let mut function = 0usize;

        let result = ((*protocol).get_location)(
            protocol,
            &mut segment,
            &mut bus,
            &mut device,
            &mut function,
        );

        if result != Status::SUCCESS {
            return Err(Error::from_status(result));
        }

        let location = PciLocation {
            segment: segment as u16,
            bus: bus as u8,
            device: device as u8,
            function: function as u8,
        };

        // Read config header
        let mut header_data = [0u32; 16];
        let result = ((*protocol).pci.read)(
            protocol,
            EfiPciIoProtocolWidth::Uint32,
            0,
            16,
            header_data.as_mut_ptr() as *mut core::ffi::c_void,
        );

        let config = if result == Status::SUCCESS {
            PciConfigHeader::from_raw(&header_data)
        } else {
            PciConfigHeader::default()
        };

        Ok(Self {
            protocol,
            _handle: handle,
            config,
            location,
        })
    }

    /// Get device location
    pub fn location(&self) -> &PciLocation {
        &self.location
    }

    /// Get config header
    pub fn config(&self) -> &PciConfigHeader {
        &self.config
    }

    /// Get vendor ID
    pub fn vendor_id(&self) -> u16 {
        self.config.vendor_id
    }

    /// Get device ID
    pub fn device_id(&self) -> u16 {
        self.config.device_id
    }

    /// Get class code
    pub fn class(&self) -> PciClass {
        PciClass {
            base: self.config.base_class,
            sub: self.config.sub_class,
            interface: self.config.prog_if,
        }
    }

    /// Get revision ID
    pub fn revision(&self) -> u8 {
        self.config.revision
    }

    /// Get header type
    pub fn header_type(&self) -> u8 {
        self.config.header_type & 0x7F
    }

    /// Is multi-function device
    pub fn is_multifunction(&self) -> bool {
        (self.config.header_type & 0x80) != 0
    }

    /// Get BARs
    pub fn bars(&self) -> &[u32; 6] {
        &self.config.bars
    }

    // =========================================================================
    // CONFIG SPACE ACCESS
    // =========================================================================

    /// Read config byte
    pub fn read_config_byte(&self, offset: u32) -> Result<u8> {
        let mut value = 0u8;

        let result = unsafe {
            ((*self.protocol).pci.read)(
                self.protocol,
                EfiPciIoProtocolWidth::Uint8,
                offset,
                1,
                &mut value as *mut u8 as *mut core::ffi::c_void,
            )
        };

        if result == Status::SUCCESS {
            Ok(value)
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Read config word
    pub fn read_config_word(&self, offset: u32) -> Result<u16> {
        let mut value = 0u16;

        let result = unsafe {
            ((*self.protocol).pci.read)(
                self.protocol,
                EfiPciIoProtocolWidth::Uint16,
                offset,
                1,
                &mut value as *mut u16 as *mut core::ffi::c_void,
            )
        };

        if result == Status::SUCCESS {
            Ok(value)
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Read config dword
    pub fn read_config_dword(&self, offset: u32) -> Result<u32> {
        let mut value = 0u32;

        let result = unsafe {
            ((*self.protocol).pci.read)(
                self.protocol,
                EfiPciIoProtocolWidth::Uint32,
                offset,
                1,
                &mut value as *mut u32 as *mut core::ffi::c_void,
            )
        };

        if result == Status::SUCCESS {
            Ok(value)
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Write config byte
    pub fn write_config_byte(&self, offset: u32, value: u8) -> Result<()> {
        let result = unsafe {
            ((*self.protocol).pci.write)(
                self.protocol,
                EfiPciIoProtocolWidth::Uint8,
                offset,
                1,
                &value as *const u8 as *mut core::ffi::c_void,
            )
        };

        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Write config word
    pub fn write_config_word(&self, offset: u32, value: u16) -> Result<()> {
        let result = unsafe {
            ((*self.protocol).pci.write)(
                self.protocol,
                EfiPciIoProtocolWidth::Uint16,
                offset,
                1,
                &value as *const u16 as *mut core::ffi::c_void,
            )
        };

        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Write config dword
    pub fn write_config_dword(&self, offset: u32, value: u32) -> Result<()> {
        let result = unsafe {
            ((*self.protocol).pci.write)(
                self.protocol,
                EfiPciIoProtocolWidth::Uint32,
                offset,
                1,
                &value as *const u32 as *mut core::ffi::c_void,
            )
        };

        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(Error::from_status(result))
        }
    }

    // =========================================================================
    // CAPABILITY ACCESS
    // =========================================================================

    /// Find capability
    pub fn find_capability(&self, cap_id: u8) -> Option<u8> {
        // Check if capabilities are supported
        if (self.config.status & 0x10) == 0 {
            return None;
        }

        // Get capabilities pointer
        let cap_ptr = match self.read_config_byte(0x34) {
            Ok(ptr) => ptr & 0xFC,
            Err(_) => return None,
        };

        let mut offset = cap_ptr;
        let mut visited = 0;

        while offset != 0 && visited < 48 {
            visited += 1;

            let cap = match self.read_config_byte(offset as u32) {
                Ok(c) => c,
                Err(_) => return None,
            };

            if cap == cap_id {
                return Some(offset);
            }

            offset = match self.read_config_byte(offset as u32 + 1) {
                Ok(next) => next & 0xFC,
                Err(_) => return None,
            };
        }

        None
    }

    /// Check if device has MSI capability
    pub fn has_msi(&self) -> bool {
        self.find_capability(PCI_CAP_MSI).is_some()
    }

    /// Check if device has MSI-X capability
    pub fn has_msix(&self) -> bool {
        self.find_capability(PCI_CAP_MSIX).is_some()
    }

    /// Check if device has PCIe capability
    pub fn has_pcie(&self) -> bool {
        self.find_capability(PCI_CAP_PCIE).is_some()
    }

    /// Check if device has power management capability
    pub fn has_power_management(&self) -> bool {
        self.find_capability(PCI_CAP_PM).is_some()
    }

    // =========================================================================
    // MEMORY/IO ACCESS
    // =========================================================================

    /// Read from memory BAR
    pub fn mem_read<T: Copy>(&self, bar: u8, offset: u64) -> Result<T> {
        let mut value: T = unsafe { core::mem::zeroed() };

        let width = match core::mem::size_of::<T>() {
            1 => EfiPciIoProtocolWidth::Uint8,
            2 => EfiPciIoProtocolWidth::Uint16,
            4 => EfiPciIoProtocolWidth::Uint32,
            8 => EfiPciIoProtocolWidth::Uint64,
            _ => return Err(Error::InvalidParameter),
        };

        let result = unsafe {
            ((*self.protocol).mem.read)(
                self.protocol,
                width,
                bar,
                offset,
                1,
                &mut value as *mut T as *mut core::ffi::c_void,
            )
        };

        if result == Status::SUCCESS {
            Ok(value)
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Write to memory BAR
    pub fn mem_write<T: Copy>(&self, bar: u8, offset: u64, value: T) -> Result<()> {
        let width = match core::mem::size_of::<T>() {
            1 => EfiPciIoProtocolWidth::Uint8,
            2 => EfiPciIoProtocolWidth::Uint16,
            4 => EfiPciIoProtocolWidth::Uint32,
            8 => EfiPciIoProtocolWidth::Uint64,
            _ => return Err(Error::InvalidParameter),
        };

        let result = unsafe {
            ((*self.protocol).mem.write)(
                self.protocol,
                width,
                bar,
                offset,
                1,
                &value as *const T as *mut core::ffi::c_void,
            )
        };

        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Read from I/O BAR
    pub fn io_read<T: Copy>(&self, bar: u8, offset: u64) -> Result<T> {
        let mut value: T = unsafe { core::mem::zeroed() };

        let width = match core::mem::size_of::<T>() {
            1 => EfiPciIoProtocolWidth::Uint8,
            2 => EfiPciIoProtocolWidth::Uint16,
            4 => EfiPciIoProtocolWidth::Uint32,
            _ => return Err(Error::InvalidParameter),
        };

        let result = unsafe {
            ((*self.protocol).io.read)(
                self.protocol,
                width,
                bar,
                offset,
                1,
                &mut value as *mut T as *mut core::ffi::c_void,
            )
        };

        if result == Status::SUCCESS {
            Ok(value)
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Write to I/O BAR
    pub fn io_write<T: Copy>(&self, bar: u8, offset: u64, value: T) -> Result<()> {
        let width = match core::mem::size_of::<T>() {
            1 => EfiPciIoProtocolWidth::Uint8,
            2 => EfiPciIoProtocolWidth::Uint16,
            4 => EfiPciIoProtocolWidth::Uint32,
            _ => return Err(Error::InvalidParameter),
        };

        let result = unsafe {
            ((*self.protocol).io.write)(
                self.protocol,
                width,
                bar,
                offset,
                1,
                &value as *const T as *mut core::ffi::c_void,
            )
        };

        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(Error::from_status(result))
        }
    }

    // =========================================================================
    // DEVICE CONTROL
    // =========================================================================

    /// Get command register
    pub fn command(&self) -> Result<PciCommand> {
        Ok(PciCommand(self.read_config_word(4)?))
    }

    /// Set command register
    pub fn set_command(&self, command: PciCommand) -> Result<()> {
        self.write_config_word(4, command.0)
    }

    /// Enable bus master
    pub fn enable_bus_master(&self) -> Result<()> {
        let mut cmd = self.command()?;
        cmd.0 |= 0x04;
        self.set_command(cmd)
    }

    /// Disable bus master
    pub fn disable_bus_master(&self) -> Result<()> {
        let mut cmd = self.command()?;
        cmd.0 &= !0x04;
        self.set_command(cmd)
    }

    /// Enable memory space
    pub fn enable_memory(&self) -> Result<()> {
        let mut cmd = self.command()?;
        cmd.0 |= 0x02;
        self.set_command(cmd)
    }

    /// Enable I/O space
    pub fn enable_io(&self) -> Result<()> {
        let mut cmd = self.command()?;
        cmd.0 |= 0x01;
        self.set_command(cmd)
    }

    /// Get description
    pub fn description(&self) -> String {
        alloc::format!(
            "{:04X}:{:04X} {} ({:02X}:{:02X}:{:02X})",
            self.vendor_id(),
            self.device_id(),
            self.class().name(),
            self.location.bus,
            self.location.device,
            self.location.function
        )
    }
}

impl Protocol for PciDevice {
    const GUID: Guid = PCI_IO_PROTOCOL_GUID;

    fn open(handle: Handle) -> Result<Self> {
        use crate::services::boot_services;

        let bs = unsafe { boot_services() };
        let image = crate::services::image_handle().ok_or(Error::NotReady)?;

        let mut protocol: *mut core::ffi::c_void = core::ptr::null_mut();
        let result = unsafe {
            ((*bs).open_protocol)(
                handle,
                &Self::GUID as *const Guid,
                &mut protocol,
                image,
                Handle(core::ptr::null_mut()),
                0x0000_0002,
            )
        };

        if result != Status::SUCCESS {
            return Err(Error::from_status(result));
        }

        unsafe { Self::from_raw(protocol as *mut EfiPciIoProtocol, handle) }
    }
}

impl EnumerableProtocol for PciDevice {
    fn enumerate() -> Result<Vec<Self>> {
        let handles = super::ProtocolLocator::locate_all::<Self>()?;
        Ok(handles.into_iter().map(super::ProtocolHandle::leak).collect())
    }
}

// =============================================================================
// PCI LOCATION
// =============================================================================

/// PCI device location
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PciLocation {
    /// Segment (domain)
    pub segment: u16,
    /// Bus number
    pub bus: u8,
    /// Device number
    pub device: u8,
    /// Function number
    pub function: u8,
}

impl PciLocation {
    /// Create from BDF (Bus:Device:Function)
    pub fn from_bdf(bus: u8, device: u8, function: u8) -> Self {
        Self {
            segment: 0,
            bus,
            device,
            function,
        }
    }

    /// Get as BDF string
    pub fn to_bdf_string(&self) -> String {
        alloc::format!("{:02X}:{:02X}.{:X}", self.bus, self.device, self.function)
    }
}

impl core::fmt::Display for PciLocation {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:04X}:{:02X}:{:02X}.{:X}",
            self.segment, self.bus, self.device, self.function
        )
    }
}

// =============================================================================
// PCI CONFIG HEADER
// =============================================================================

/// PCI configuration header
#[derive(Debug, Clone, Default)]
pub struct PciConfigHeader {
    /// Vendor ID
    pub vendor_id: u16,
    /// Device ID
    pub device_id: u16,
    /// Command register
    pub command: u16,
    /// Status register
    pub status: u16,
    /// Revision ID
    pub revision: u8,
    /// Programming interface
    pub prog_if: u8,
    /// Sub-class
    pub sub_class: u8,
    /// Base class
    pub base_class: u8,
    /// Cache line size
    pub cache_line_size: u8,
    /// Latency timer
    pub latency_timer: u8,
    /// Header type
    pub header_type: u8,
    /// BIST
    pub bist: u8,
    /// BAR values
    pub bars: [u32; 6],
}

impl PciConfigHeader {
    fn from_raw(data: &[u32; 16]) -> Self {
        Self {
            vendor_id: (data[0] & 0xFFFF) as u16,
            device_id: ((data[0] >> 16) & 0xFFFF) as u16,
            command: (data[1] & 0xFFFF) as u16,
            status: ((data[1] >> 16) & 0xFFFF) as u16,
            revision: (data[2] & 0xFF) as u8,
            prog_if: ((data[2] >> 8) & 0xFF) as u8,
            sub_class: ((data[2] >> 16) & 0xFF) as u8,
            base_class: ((data[2] >> 24) & 0xFF) as u8,
            cache_line_size: (data[3] & 0xFF) as u8,
            latency_timer: ((data[3] >> 8) & 0xFF) as u8,
            header_type: ((data[3] >> 16) & 0xFF) as u8,
            bist: ((data[3] >> 24) & 0xFF) as u8,
            bars: [data[4], data[5], data[6], data[7], data[8], data[9]],
        }
    }
}

// =============================================================================
// PCI CONFIG (ALIAS)
// =============================================================================

/// PCI config (alias for PciConfigHeader)
pub type PciConfig = PciConfigHeader;

// =============================================================================
// PCI CLASS
// =============================================================================

/// PCI class code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PciClass {
    /// Base class
    pub base: u8,
    /// Sub-class
    pub sub: u8,
    /// Programming interface
    pub interface: u8,
}

impl PciClass {
    /// Get class name
    pub fn name(&self) -> &'static str {
        match self.base {
            0x00 => "Unclassified",
            0x01 => match self.sub {
                0x00 => "SCSI",
                0x01 => "IDE",
                0x02 => "Floppy",
                0x03 => "IPI",
                0x04 => "RAID",
                0x05 => "ATA",
                0x06 => "SATA",
                0x07 => "SAS",
                0x08 => "NVMe",
                _ => "Storage",
            },
            0x02 => match self.sub {
                0x00 => "Ethernet",
                0x01 => "Token Ring",
                0x02 => "FDDI",
                0x03 => "ATM",
                0x04 => "ISDN",
                0x80 => "Other Network",
                _ => "Network",
            },
            0x03 => match self.sub {
                0x00 => "VGA",
                0x01 => "XGA",
                0x02 => "3D",
                _ => "Display",
            },
            0x04 => match self.sub {
                0x00 => "Video",
                0x01 => "Audio",
                0x02 => "Telephony",
                0x03 => "Audio Device",
                _ => "Multimedia",
            },
            0x05 => "Memory",
            0x06 => match self.sub {
                0x00 => "Host Bridge",
                0x01 => "ISA Bridge",
                0x02 => "EISA Bridge",
                0x03 => "MCA Bridge",
                0x04 => "PCI-PCI Bridge",
                0x05 => "PCMCIA Bridge",
                0x06 => "NuBus Bridge",
                0x07 => "CardBus Bridge",
                0x08 => "RACEway Bridge",
                0x09 => "Semi PCI-PCI",
                0x0A => "InfiniBand",
                _ => "Bridge",
            },
            0x07 => "Communication",
            0x08 => "System Peripheral",
            0x09 => "Input",
            0x0A => "Docking Station",
            0x0B => "Processor",
            0x0C => match self.sub {
                0x00 => "FireWire",
                0x01 => "ACCESS.bus",
                0x02 => "SSA",
                0x03 => match self.interface {
                    0x00 => "UHCI",
                    0x10 => "OHCI",
                    0x20 => "EHCI",
                    0x30 => "xHCI",
                    _ => "USB",
                },
                0x04 => "Fibre Channel",
                0x05 => "SMBus",
                0x06 => "InfiniBand",
                0x07 => "IPMI",
                _ => "Serial Bus",
            },
            0x0D => "Wireless",
            0x0E => "Intelligent I/O",
            0x0F => "Satellite",
            0x10 => "Encryption",
            0x11 => "Signal Processing",
            0x12 => "Processing Accelerator",
            0x13 => "Non-Essential Instrumentation",
            0x40 => "Co-Processor",
            0xFF => "Unassigned",
            _ => "Unknown",
        }
    }

    /// Check if this is a bridge
    pub fn is_bridge(&self) -> bool {
        self.base == 0x06
    }

    /// Check if this is a display controller
    pub fn is_display(&self) -> bool {
        self.base == 0x03
    }

    /// Check if this is a network controller
    pub fn is_network(&self) -> bool {
        self.base == 0x02
    }

    /// Check if this is a storage controller
    pub fn is_storage(&self) -> bool {
        self.base == 0x01
    }

    /// Check if this is a USB controller
    pub fn is_usb(&self) -> bool {
        self.base == 0x0C && self.sub == 0x03
    }
}

// =============================================================================
// PCI COMMAND
// =============================================================================

/// PCI command register
#[derive(Debug, Clone, Copy)]
pub struct PciCommand(pub u16);

impl PciCommand {
    /// I/O space enable
    pub fn io_space(&self) -> bool {
        (self.0 & 0x0001) != 0
    }
    /// Memory space enable
    pub fn memory_space(&self) -> bool {
        (self.0 & 0x0002) != 0
    }
    /// Bus master enable
    pub fn bus_master(&self) -> bool {
        (self.0 & 0x0004) != 0
    }
    /// Special cycle enable
    pub fn special_cycles(&self) -> bool {
        (self.0 & 0x0008) != 0
    }
    /// Memory write and invalidate
    pub fn memory_write_invalidate(&self) -> bool {
        (self.0 & 0x0010) != 0
    }
    /// VGA palette snoop
    pub fn vga_palette_snoop(&self) -> bool {
        (self.0 & 0x0020) != 0
    }
    /// Parity error response
    pub fn parity_error_response(&self) -> bool {
        (self.0 & 0x0040) != 0
    }
    /// SERR# enable
    pub fn serr(&self) -> bool {
        (self.0 & 0x0100) != 0
    }
    /// Fast back-to-back enable
    pub fn fast_back_to_back(&self) -> bool {
        (self.0 & 0x0200) != 0
    }
    /// Interrupt disable
    pub fn interrupt_disable(&self) -> bool {
        (self.0 & 0x0400) != 0
    }
}

// =============================================================================
// CAPABILITY IDS
// =============================================================================

/// Power Management
const PCI_CAP_PM: u8 = 0x01;
/// AGP
const _PCI_CAP_AGP: u8 = 0x02;
/// VPD
const _PCI_CAP_VPD: u8 = 0x03;
/// Slot ID
const _PCI_CAP_SLOTID: u8 = 0x04;
/// MSI
const PCI_CAP_MSI: u8 = 0x05;
/// CompactPCI Hot Swap
const _PCI_CAP_CHSWP: u8 = 0x06;
/// PCI-X
const _PCI_CAP_PCIX: u8 = 0x07;
/// HyperTransport
const _PCI_CAP_HT: u8 = 0x08;
/// Vendor Specific
const _PCI_CAP_VNDR: u8 = 0x09;
/// Debug port
const _PCI_CAP_DBG: u8 = 0x0A;
/// CompactPCI Central Resource Control
const _PCI_CAP_CCRC: u8 = 0x0B;
/// Hot Plug
const _PCI_CAP_HOTPLUG: u8 = 0x0C;
/// Bridge Subsystem Vendor ID
const _PCI_CAP_SSVID: u8 = 0x0D;
/// AGP 8x
const _PCI_CAP_AGP3: u8 = 0x0E;
/// Secure Device
const _PCI_CAP_SECURE: u8 = 0x0F;
/// PCI Express
const PCI_CAP_PCIE: u8 = 0x10;
/// MSI-X
const PCI_CAP_MSIX: u8 = 0x11;
/// SATA
const _PCI_CAP_SATA: u8 = 0x12;
/// Advanced Features
const _PCI_CAP_AF: u8 = 0x13;

// =============================================================================
// ENUMERATION HELPERS
// =============================================================================

/// Enumerate all PCI devices
pub fn enumerate_pci() -> Result<Vec<PciDevice>> {
    PciDevice::enumerate()
}

/// Find devices by class
pub fn find_by_class(base_class: u8, sub_class: Option<u8>) -> Result<Vec<PciDevice>> {
    let devices = enumerate_pci()?;

    Ok(devices
        .into_iter()
        .filter(|d| {
            let c = d.class();
            c.base == base_class && sub_class.map_or(true, |s| c.sub == s)
        })
        .collect())
}

/// Find devices by vendor and device ID
pub fn find_by_id(vendor_id: u16, device_id: Option<u16>) -> Result<Vec<PciDevice>> {
    let devices = enumerate_pci()?;

    Ok(devices
        .into_iter()
        .filter(|d| {
            d.vendor_id() == vendor_id && device_id.map_or(true, |id| d.device_id() == id)
        })
        .collect())
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_pci_location() {
        let loc = PciLocation::from_bdf(0x00, 0x1F, 0x03);
        assert_eq!(loc.to_bdf_string(), "00:1F.3");
    }

    #[test]
    fn test_pci_class() {
        let class = PciClass {
            base: 0x02,
            sub: 0x00,
            interface: 0x00,
        };
        assert_eq!(class.name(), "Ethernet");
        assert!(class.is_network());
    }

    #[test]
    fn test_pci_command() {
        let cmd = PciCommand(0x0007);
        assert!(cmd.io_space());
        assert!(cmd.memory_space());
        assert!(cmd.bus_master());
    }
}
