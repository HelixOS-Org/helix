//! PCI I/O Protocol
//!
//! Provides access to PCI devices.

use core::fmt;

use crate::raw::types::*;

// =============================================================================
// PCI I/O PROTOCOL
// =============================================================================

/// PCI I/O Protocol
#[repr(C)]
pub struct EfiPciIoProtocol {
    /// Poll memory
    pub poll_mem: unsafe extern "efiapi" fn(
        this: *mut Self,
        width: EfiPciIoProtocolWidth,
        bar_index: u8,
        offset: u64,
        mask: u64,
        value: u64,
        delay: u64,
        result: *mut u64,
    ) -> Status,

    /// Poll I/O
    pub poll_io: unsafe extern "efiapi" fn(
        this: *mut Self,
        width: EfiPciIoProtocolWidth,
        bar_index: u8,
        offset: u64,
        mask: u64,
        value: u64,
        delay: u64,
        result: *mut u64,
    ) -> Status,

    /// Memory access
    pub mem: EfiPciIoProtocolAccess,

    /// I/O access
    pub io: EfiPciIoProtocolAccess,

    /// PCI configuration space access
    pub pci: EfiPciIoProtocolConfigAccess,

    /// Copy memory
    pub copy_mem: unsafe extern "efiapi" fn(
        this: *mut Self,
        width: EfiPciIoProtocolWidth,
        dest_bar_index: u8,
        dest_offset: u64,
        src_bar_index: u8,
        src_offset: u64,
        count: usize,
    ) -> Status,

    /// Map DMA buffer
    pub map: unsafe extern "efiapi" fn(
        this: *mut Self,
        operation: EfiPciIoProtocolOperation,
        host_address: *mut core::ffi::c_void,
        number_of_bytes: *mut usize,
        device_address: *mut PhysicalAddress,
        mapping: *mut *mut core::ffi::c_void,
    ) -> Status,

    /// Unmap DMA buffer
    pub unmap:
        unsafe extern "efiapi" fn(this: *mut Self, mapping: *mut core::ffi::c_void) -> Status,

    /// Allocate DMA buffer
    pub allocate_buffer: unsafe extern "efiapi" fn(
        this: *mut Self,
        alloc_type: AllocateType,
        memory_type: u32,
        pages: usize,
        host_address: *mut *mut core::ffi::c_void,
        attributes: u64,
    ) -> Status,

    /// Free DMA buffer
    pub free_buffer: unsafe extern "efiapi" fn(
        this: *mut Self,
        pages: usize,
        host_address: *mut core::ffi::c_void,
    ) -> Status,

    /// Flush DMA buffer
    pub flush: unsafe extern "efiapi" fn(this: *mut Self) -> Status,

    /// Get device location
    pub get_location: unsafe extern "efiapi" fn(
        this: *mut Self,
        segment_number: *mut usize,
        bus_number: *mut usize,
        device_number: *mut usize,
        function_number: *mut usize,
    ) -> Status,

    /// Get device attributes
    pub attributes: unsafe extern "efiapi" fn(
        this: *mut Self,
        operation: EfiPciIoProtocolAttributeOperation,
        attributes: u64,
        result: *mut u64,
    ) -> Status,

    /// Get BAR attributes
    pub get_bar_attributes: unsafe extern "efiapi" fn(
        this: *mut Self,
        bar_index: u8,
        supports: *mut u64,
        resources: *mut *mut core::ffi::c_void,
    ) -> Status,

    /// Set BAR attributes
    pub set_bar_attributes: unsafe extern "efiapi" fn(
        this: *mut Self,
        attributes: u64,
        bar_index: u8,
        offset: *mut u64,
        length: *mut u64,
    ) -> Status,

    /// ROM size
    pub rom_size: u64,

    /// ROM image
    pub rom_image: *mut core::ffi::c_void,
}

impl EfiPciIoProtocol {
    /// Protocol GUID
    pub const GUID: Guid = guids::PCI_IO_PROTOCOL;

    /// Get device location (segment, bus, device, function)
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn get_location(&self) -> Result<PciLocation, Status> {
        let mut segment = 0;
        let mut bus = 0;
        let mut device = 0;
        let mut function = 0;

        let status = (self.get_location)(
            self as *const _ as *mut _,
            &mut segment,
            &mut bus,
            &mut device,
            &mut function,
        );

        status.to_status_result_with(PciLocation {
            segment: segment as u16,
            bus: bus as u8,
            device: device as u8,
            function: function as u8,
        })
    }

    /// Read PCI configuration space
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn pci_read<T: Copy>(&mut self, offset: u32) -> Result<T, Status> {
        let width = match core::mem::size_of::<T>() {
            1 => EfiPciIoProtocolWidth::Uint8,
            2 => EfiPciIoProtocolWidth::Uint16,
            4 => EfiPciIoProtocolWidth::Uint32,
            8 => EfiPciIoProtocolWidth::Uint64,
            _ => return Err(Status::INVALID_PARAMETER),
        };

        let mut value = core::mem::MaybeUninit::<T>::uninit();
        let status = (self.pci.read)(
            self,
            width,
            offset,
            1,
            value.as_mut_ptr() as *mut core::ffi::c_void,
        );

        status.to_status_result_with(value.assume_init())
    }

    /// Write PCI configuration space
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn pci_write<T: Copy>(&mut self, offset: u32, value: T) -> Result<(), Status> {
        let width = match core::mem::size_of::<T>() {
            1 => EfiPciIoProtocolWidth::Uint8,
            2 => EfiPciIoProtocolWidth::Uint16,
            4 => EfiPciIoProtocolWidth::Uint32,
            8 => EfiPciIoProtocolWidth::Uint64,
            _ => return Err(Status::INVALID_PARAMETER),
        };

        let status = (self.pci.write)(
            self,
            width,
            offset,
            1,
            &value as *const T as *mut core::ffi::c_void,
        );

        status.to_status_result()
    }

    /// Read from memory BAR
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer and BAR are valid.
    pub unsafe fn mem_read<T: Copy>(&mut self, bar_index: u8, offset: u64) -> Result<T, Status> {
        let width = match core::mem::size_of::<T>() {
            1 => EfiPciIoProtocolWidth::Uint8,
            2 => EfiPciIoProtocolWidth::Uint16,
            4 => EfiPciIoProtocolWidth::Uint32,
            8 => EfiPciIoProtocolWidth::Uint64,
            _ => return Err(Status::INVALID_PARAMETER),
        };

        let mut value = core::mem::MaybeUninit::<T>::uninit();
        let status = (self.mem.read)(
            self,
            width,
            bar_index,
            offset,
            1,
            value.as_mut_ptr() as *mut core::ffi::c_void,
        );

        status.to_status_result_with(value.assume_init())
    }

    /// Write to memory BAR
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer and BAR are valid.
    pub unsafe fn mem_write<T: Copy>(
        &mut self,
        bar_index: u8,
        offset: u64,
        value: T,
    ) -> Result<(), Status> {
        let width = match core::mem::size_of::<T>() {
            1 => EfiPciIoProtocolWidth::Uint8,
            2 => EfiPciIoProtocolWidth::Uint16,
            4 => EfiPciIoProtocolWidth::Uint32,
            8 => EfiPciIoProtocolWidth::Uint64,
            _ => return Err(Status::INVALID_PARAMETER),
        };

        let status = (self.mem.write)(
            self,
            width,
            bar_index,
            offset,
            1,
            &value as *const T as *mut core::ffi::c_void,
        );

        status.to_status_result()
    }

    /// Get current attributes
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn get_attributes(&self) -> Result<u64, Status> {
        let mut result = 0;
        let status = (self.attributes)(
            self as *const _ as *mut _,
            EfiPciIoProtocolAttributeOperation::Get,
            0,
            &mut result,
        );
        status.to_status_result_with(result)
    }

    /// Enable attributes
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn enable_attributes(&mut self, attrs: u64) -> Result<(), Status> {
        let status = (self.attributes)(
            self,
            EfiPciIoProtocolAttributeOperation::Enable,
            attrs,
            core::ptr::null_mut(),
        );
        status.to_status_result()
    }

    /// Disable attributes
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn disable_attributes(&mut self, attrs: u64) -> Result<(), Status> {
        let status = (self.attributes)(
            self,
            EfiPciIoProtocolAttributeOperation::Disable,
            attrs,
            core::ptr::null_mut(),
        );
        status.to_status_result()
    }

    /// Get ROM if present
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn rom(&self) -> Option<&[u8]> {
        if self.rom_image.is_null() || self.rom_size == 0 {
            None
        } else {
            Some(core::slice::from_raw_parts(
                self.rom_image as *const u8,
                self.rom_size as usize,
            ))
        }
    }
}

impl fmt::Debug for EfiPciIoProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EfiPciIoProtocol")
            .field("rom_size", &self.rom_size)
            .finish()
    }
}

// =============================================================================
// PCI LOCATION
// =============================================================================

/// PCI device location
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PciLocation {
    /// Segment number
    pub segment: u16,
    /// Bus number
    pub bus: u8,
    /// Device number
    pub device: u8,
    /// Function number
    pub function: u8,
}

impl PciLocation {
    /// Create a new PCI location
    pub const fn new(segment: u16, bus: u8, device: u8, function: u8) -> Self {
        Self {
            segment,
            bus,
            device,
            function,
        }
    }

    /// Format as BDF (Bus:Device.Function)
    pub fn bdf(&self) -> u16 {
        ((self.bus as u16) << 8) | ((self.device as u16) << 3) | (self.function as u16)
    }
}

impl fmt::Display for PciLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:04x}:{:02x}:{:02x}.{}",
            self.segment, self.bus, self.device, self.function
        )
    }
}

// =============================================================================
// ACCESS WIDTH
// =============================================================================

/// PCI I/O access width
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum EfiPciIoProtocolWidth {
    /// 8-bit
    Uint8      = 0,
    /// 16-bit
    Uint16     = 1,
    /// 32-bit
    Uint32     = 2,
    /// 64-bit
    Uint64     = 3,
    /// FIFO 8-bit
    FifoUint8  = 4,
    /// FIFO 16-bit
    FifoUint16 = 5,
    /// FIFO 32-bit
    FifoUint32 = 6,
    /// FIFO 64-bit
    FifoUint64 = 7,
    /// Fill 8-bit
    FillUint8  = 8,
    /// Fill 16-bit
    FillUint16 = 9,
    /// Fill 32-bit
    FillUint32 = 10,
    /// Fill 64-bit
    FillUint64 = 11,
}

// =============================================================================
// DMA OPERATION
// =============================================================================

/// PCI DMA operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum EfiPciIoProtocolOperation {
    /// Bus master read
    BusMasterRead  = 0,
    /// Bus master write
    BusMasterWrite = 1,
    /// Bus master common buffer
    BusMasterCommonBuffer = 2,
}

// =============================================================================
// ATTRIBUTE OPERATION
// =============================================================================

/// Attribute operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum EfiPciIoProtocolAttributeOperation {
    /// Get current attributes
    Get       = 0,
    /// Set attributes
    Set       = 1,
    /// Enable attributes
    Enable    = 2,
    /// Disable attributes
    Disable   = 3,
    /// Get supported attributes
    Supported = 4,
}

// =============================================================================
// PCI ATTRIBUTES
// =============================================================================

/// PCI device attributes
pub mod pci_attributes {
    /// I/O space enable
    pub const IO: u64 = 0x0001;
    /// Memory space enable
    pub const MEMORY: u64 = 0x0002;
    /// Bus master enable
    pub const BUS_MASTER: u64 = 0x0004;
    /// Memory write and invalidate enable
    pub const MEMORY_WRITE_COMBINE: u64 = 0x0008;
    /// VGA palette snoop
    pub const VGA_PALETTE_IO: u64 = 0x0010;
    /// VGA memory
    pub const VGA_MEMORY: u64 = 0x0020;
    /// VGA I/O
    pub const VGA_IO: u64 = 0x0040;
    /// ISA I/O
    pub const ISA_IO: u64 = 0x0080;
    /// Dual address cycle
    pub const DUAL_ADDRESS_CYCLE: u64 = 0x8000;
    /// Clear interrupt
    pub const CLEAR_INTERRUPT: u64 = 0x10000;
}

// =============================================================================
// ACCESS FUNCTIONS
// =============================================================================

/// PCI I/O memory/IO access functions
#[repr(C)]
pub struct EfiPciIoProtocolAccess {
    /// Read
    pub read: unsafe extern "efiapi" fn(
        this: *mut EfiPciIoProtocol,
        width: EfiPciIoProtocolWidth,
        bar_index: u8,
        offset: u64,
        count: usize,
        buffer: *mut core::ffi::c_void,
    ) -> Status,

    /// Write
    pub write: unsafe extern "efiapi" fn(
        this: *mut EfiPciIoProtocol,
        width: EfiPciIoProtocolWidth,
        bar_index: u8,
        offset: u64,
        count: usize,
        buffer: *mut core::ffi::c_void,
    ) -> Status,
}

/// PCI configuration space access functions
#[repr(C)]
pub struct EfiPciIoProtocolConfigAccess {
    /// Read
    pub read: unsafe extern "efiapi" fn(
        this: *mut EfiPciIoProtocol,
        width: EfiPciIoProtocolWidth,
        offset: u32,
        count: usize,
        buffer: *mut core::ffi::c_void,
    ) -> Status,

    /// Write
    pub write: unsafe extern "efiapi" fn(
        this: *mut EfiPciIoProtocol,
        width: EfiPciIoProtocolWidth,
        offset: u32,
        count: usize,
        buffer: *mut core::ffi::c_void,
    ) -> Status,
}

// =============================================================================
// PCI ROOT BRIDGE I/O PROTOCOL
// =============================================================================

/// PCI Root Bridge I/O Protocol
#[repr(C)]
pub struct EfiPciRootBridgeIoProtocol {
    /// Parent handle
    pub parent_handle: Handle,

    /// Poll memory
    pub poll_mem: unsafe extern "efiapi" fn(
        this: *mut Self,
        width: EfiPciIoProtocolWidth,
        address: u64,
        mask: u64,
        value: u64,
        delay: u64,
        result: *mut u64,
    ) -> Status,

    /// Poll I/O
    pub poll_io: unsafe extern "efiapi" fn(
        this: *mut Self,
        width: EfiPciIoProtocolWidth,
        address: u64,
        mask: u64,
        value: u64,
        delay: u64,
        result: *mut u64,
    ) -> Status,

    /// Memory access
    pub mem: EfiPciRootBridgeIoProtocolAccess,

    /// I/O access
    pub io: EfiPciRootBridgeIoProtocolAccess,

    /// PCI configuration access
    pub pci: EfiPciRootBridgeIoProtocolAccess,

    /// Copy memory
    pub copy_mem: unsafe extern "efiapi" fn(
        this: *mut Self,
        width: EfiPciIoProtocolWidth,
        dest_address: u64,
        src_address: u64,
        count: usize,
    ) -> Status,

    /// Map DMA buffer
    pub map: unsafe extern "efiapi" fn(
        this: *mut Self,
        operation: EfiPciIoProtocolOperation,
        host_address: *mut core::ffi::c_void,
        number_of_bytes: *mut usize,
        device_address: *mut PhysicalAddress,
        mapping: *mut *mut core::ffi::c_void,
    ) -> Status,

    /// Unmap DMA buffer
    pub unmap:
        unsafe extern "efiapi" fn(this: *mut Self, mapping: *mut core::ffi::c_void) -> Status,

    /// Allocate DMA buffer
    pub allocate_buffer: unsafe extern "efiapi" fn(
        this: *mut Self,
        alloc_type: AllocateType,
        memory_type: u32,
        pages: usize,
        host_address: *mut *mut core::ffi::c_void,
        attributes: u64,
    ) -> Status,

    /// Free DMA buffer
    pub free_buffer: unsafe extern "efiapi" fn(
        this: *mut Self,
        pages: usize,
        host_address: *mut core::ffi::c_void,
    ) -> Status,

    /// Flush DMA buffer
    pub flush: unsafe extern "efiapi" fn(this: *mut Self) -> Status,

    /// Get attributes
    pub get_attributes: unsafe extern "efiapi" fn(
        this: *mut Self,
        supports: *mut u64,
        attributes: *mut u64,
    ) -> Status,

    /// Set attributes
    pub set_attributes: unsafe extern "efiapi" fn(
        this: *mut Self,
        attributes: u64,
        resource_base: *mut u64,
        resource_length: *mut u64,
    ) -> Status,

    /// Configuration
    pub configuration: unsafe extern "efiapi" fn(
        this: *mut Self,
        resources: *mut *mut core::ffi::c_void,
    ) -> Status,

    /// Segment number
    pub segment_number: u32,
}

impl EfiPciRootBridgeIoProtocol {
    /// Protocol GUID
    pub const GUID: Guid = guids::PCI_ROOT_BRIDGE_IO_PROTOCOL;
}

impl fmt::Debug for EfiPciRootBridgeIoProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EfiPciRootBridgeIoProtocol")
            .field("parent_handle", &self.parent_handle)
            .field("segment_number", &self.segment_number)
            .finish()
    }
}

/// PCI Root Bridge access functions
#[repr(C)]
pub struct EfiPciRootBridgeIoProtocolAccess {
    /// Read
    pub read: unsafe extern "efiapi" fn(
        this: *mut EfiPciRootBridgeIoProtocol,
        width: EfiPciIoProtocolWidth,
        address: u64,
        count: usize,
        buffer: *mut core::ffi::c_void,
    ) -> Status,

    /// Write
    pub write: unsafe extern "efiapi" fn(
        this: *mut EfiPciRootBridgeIoProtocol,
        width: EfiPciIoProtocolWidth,
        address: u64,
        count: usize,
        buffer: *mut core::ffi::c_void,
    ) -> Status,
}

// =============================================================================
// PCI CONFIGURATION SPACE
// =============================================================================

/// PCI configuration space header (common)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
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
    pub revision_id: u8,
    /// Programming interface
    pub prog_if: u8,
    /// Subclass
    pub subclass: u8,
    /// Class code
    pub class_code: u8,
    /// Cache line size
    pub cache_line_size: u8,
    /// Latency timer
    pub latency_timer: u8,
    /// Header type
    pub header_type: u8,
    /// BIST
    pub bist: u8,
}

impl PciConfigHeader {
    /// Offset of vendor ID
    pub const VENDOR_ID_OFFSET: u32 = 0x00;
    /// Offset of device ID
    pub const DEVICE_ID_OFFSET: u32 = 0x02;
    /// Offset of command register
    pub const COMMAND_OFFSET: u32 = 0x04;
    /// Offset of status register
    pub const STATUS_OFFSET: u32 = 0x06;
    /// Offset of class code
    pub const CLASS_CODE_OFFSET: u32 = 0x09;
    /// Offset of header type
    pub const HEADER_TYPE_OFFSET: u32 = 0x0E;
    /// Offset of BAR0
    pub const BAR0_OFFSET: u32 = 0x10;

    /// Check if this is a multi-function device
    pub fn is_multi_function(&self) -> bool {
        (self.header_type & 0x80) != 0
    }

    /// Get header type (0, 1, or 2)
    pub fn header_type_number(&self) -> u8 {
        self.header_type & 0x7F
    }

    /// Check if vendor ID is valid
    pub fn is_valid(&self) -> bool {
        self.vendor_id != 0xFFFF
    }
}

/// PCI command register bits
pub mod pci_command {
    /// I/O space enable
    pub const IO_SPACE: u16 = 0x0001;
    /// Memory space enable
    pub const MEMORY_SPACE: u16 = 0x0002;
    /// Bus master enable
    pub const BUS_MASTER: u16 = 0x0004;
    /// Special cycles
    pub const SPECIAL_CYCLES: u16 = 0x0008;
    /// Memory write and invalidate
    pub const MEM_WR_INVALIDATE: u16 = 0x0010;
    /// VGA palette snoop
    pub const VGA_PALETTE_SNOOP: u16 = 0x0020;
    /// Parity error response
    pub const PARITY_ERROR_RESPONSE: u16 = 0x0040;
    /// SERR# enable
    pub const SERR_ENABLE: u16 = 0x0100;
    /// Fast back-to-back enable
    pub const FAST_BACK_TO_BACK: u16 = 0x0200;
    /// Interrupt disable
    pub const INTERRUPT_DISABLE: u16 = 0x0400;
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_pci_location() {
        let loc = PciLocation::new(0, 0, 31, 3);
        assert_eq!(loc.bdf(), (0 << 8) | (31 << 3) | 3);
    }

    #[test]
    fn test_pci_config_header() {
        let header = PciConfigHeader {
            vendor_id: 0x8086,
            device_id: 0x1234,
            command: 0,
            status: 0,
            revision_id: 0,
            prog_if: 0,
            subclass: 0,
            class_code: 0,
            cache_line_size: 0,
            latency_timer: 0,
            header_type: 0x80,
            bist: 0,
        };

        assert!(header.is_valid());
        assert!(header.is_multi_function());
        assert_eq!(header.header_type_number(), 0);
    }
}
