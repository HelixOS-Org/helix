//! Loaded Image Protocol
//!
//! Provides information about loaded UEFI images.

#![allow(non_camel_case_types)]

use core::fmt;

use crate::raw::memory::MemoryType;
use crate::raw::types::*;

// =============================================================================
// LOADED IMAGE PROTOCOL
// =============================================================================

/// Loaded Image Protocol
#[repr(C)]
pub struct EfiLoadedImageProtocol {
    /// Revision
    pub revision: u32,
    /// Parent handle
    pub parent_handle: Handle,
    /// System table
    pub system_table: *mut core::ffi::c_void,

    // Device location
    /// Device handle
    pub device_handle: Handle,
    /// File path
    pub file_path: *mut EfiDevicePathProtocol,
    /// Reserved
    pub reserved: *mut core::ffi::c_void,

    // Load options
    /// Load options size
    pub load_options_size: u32,
    /// Load options
    pub load_options: *mut core::ffi::c_void,

    // Image location
    /// Image base
    pub image_base: *mut core::ffi::c_void,
    /// Image size
    pub image_size: u64,
    /// Image code type
    pub image_code_type: MemoryType,
    /// Image data type
    pub image_data_type: MemoryType,

    /// Unload function
    pub unload: unsafe extern "efiapi" fn(image_handle: Handle) -> Status,
}

impl EfiLoadedImageProtocol {
    /// Protocol GUID
    pub const GUID: Guid = guids::LOADED_IMAGE_PROTOCOL;

    /// Revision
    pub const REVISION: u32 = 0x1000;

    /// Get parent handle
    pub fn parent(&self) -> Option<Handle> {
        if self.parent_handle.is_null() {
            None
        } else {
            Some(self.parent_handle)
        }
    }

    /// Get device handle
    pub fn device(&self) -> Option<Handle> {
        if self.device_handle.is_null() {
            None
        } else {
            Some(self.device_handle)
        }
    }

    /// Get file path
    ///
    /// # Safety
    /// The caller must ensure the file_path pointer is valid.
    pub unsafe fn file_path(&self) -> Option<&EfiDevicePathProtocol> {
        if self.file_path.is_null() {
            None
        } else {
            Some(&*self.file_path)
        }
    }

    /// Get load options as bytes
    ///
    /// # Safety
    /// The caller must ensure the load_options pointer is valid.
    pub unsafe fn load_options_bytes(&self) -> Option<&[u8]> {
        if self.load_options.is_null() || self.load_options_size == 0 {
            None
        } else {
            Some(core::slice::from_raw_parts(
                self.load_options as *const u8,
                self.load_options_size as usize,
            ))
        }
    }

    /// Get load options as UTF-16 string
    ///
    /// # Safety
    /// The caller must ensure the load_options pointer is valid.
    pub unsafe fn load_options_utf16(&self) -> Option<&[u16]> {
        if self.load_options.is_null() || self.load_options_size < 2 {
            None
        } else {
            let len = (self.load_options_size as usize) / 2;
            Some(core::slice::from_raw_parts(
                self.load_options as *const u16,
                len,
            ))
        }
    }

    /// Get image data as bytes
    ///
    /// # Safety
    /// The caller must ensure the image_base pointer is valid.
    pub unsafe fn image_data(&self) -> Option<&[u8]> {
        if self.image_base.is_null() || self.image_size == 0 {
            None
        } else {
            Some(core::slice::from_raw_parts(
                self.image_base as *const u8,
                self.image_size as usize,
            ))
        }
    }

    /// Get image base address
    pub fn image_base_address(&self) -> Option<*const u8> {
        if self.image_base.is_null() {
            None
        } else {
            Some(self.image_base as *const u8)
        }
    }

    /// Get image size
    pub fn image_size_bytes(&self) -> u64 {
        self.image_size
    }

    /// Unload the image
    ///
    /// # Safety
    /// The caller must ensure this is safe to do.
    pub unsafe fn unload_image(&self, handle: Handle) -> Result<(), Status> {
        let status = (self.unload)(handle);
        status.to_status_result()
    }
}

impl fmt::Debug for EfiLoadedImageProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EfiLoadedImageProtocol")
            .field("revision", &self.revision)
            .field("parent_handle", &self.parent_handle)
            .field("device_handle", &self.device_handle)
            .field("image_base", &self.image_base)
            .field("image_size", &self.image_size)
            .field("image_code_type", &self.image_code_type)
            .field("image_data_type", &self.image_data_type)
            .finish()
    }
}

// =============================================================================
// DEVICE PATH PROTOCOL
// =============================================================================

/// Device Path Protocol
#[derive(Clone, Copy)]
#[repr(C)]
pub struct EfiDevicePathProtocol {
    /// Type
    pub device_type: u8,
    /// Sub-type
    pub sub_type: u8,
    /// Length (including header)
    pub length: [u8; 2],
}

impl EfiDevicePathProtocol {
    /// Protocol GUID
    pub const GUID: Guid = guids::DEVICE_PATH_PROTOCOL;

    /// Get length
    pub fn len(&self) -> usize {
        u16::from_le_bytes(self.length) as usize
    }

    /// Check if this is an end node
    pub fn is_end(&self) -> bool {
        self.device_type == DevicePathType::END as u8
    }

    /// Check if this is the end of entire path
    pub fn is_end_entire(&self) -> bool {
        self.device_type == DevicePathType::END as u8
            && self.sub_type == DevicePathEndSubType::ENTIRE as u8
    }

    /// Check if this is end of instance
    pub fn is_end_instance(&self) -> bool {
        self.device_type == DevicePathType::END as u8
            && self.sub_type == DevicePathEndSubType::INSTANCE as u8
    }

    /// Get device type
    pub fn get_type(&self) -> DevicePathType {
        DevicePathType::from_u8(self.device_type)
    }

    /// Get next node
    ///
    /// # Safety
    /// The caller must ensure the device path is valid and not at the end.
    pub unsafe fn next(&self) -> *const Self {
        let ptr = self as *const Self as *const u8;
        ptr.add(self.len()) as *const Self
    }

    /// Iterate over all nodes
    ///
    /// # Safety
    /// The caller must ensure the device path is valid.
    pub unsafe fn iter(&self) -> DevicePathIter {
        DevicePathIter {
            current: self,
            phantom: core::marker::PhantomData,
        }
    }
}

impl fmt::Debug for EfiDevicePathProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EfiDevicePathProtocol")
            .field("type", &self.get_type())
            .field("sub_type", &self.sub_type)
            .field("length", &self.len())
            .finish()
    }
}

// =============================================================================
// DEVICE PATH ITERATOR
// =============================================================================

/// Iterator over device path nodes
pub struct DevicePathIter<'a> {
    current: *const EfiDevicePathProtocol,
    phantom: core::marker::PhantomData<&'a EfiDevicePathProtocol>,
}

impl<'a> DevicePathIter<'a> {
    /// Create a new iterator
    ///
    /// # Safety
    /// The caller must ensure the device path is valid.
    pub unsafe fn new(path: &'a EfiDevicePathProtocol) -> Self {
        Self {
            current: path,
            phantom: core::marker::PhantomData,
        }
    }
}

impl<'a> Iterator for DevicePathIter<'a> {
    type Item = &'a EfiDevicePathProtocol;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.is_null() {
            return None;
        }

        unsafe {
            let node = &*self.current;
            if node.is_end_entire() {
                self.current = core::ptr::null();
                None
            } else {
                self.current = node.next();
                Some(node)
            }
        }
    }
}

// =============================================================================
// DEVICE PATH TYPES
// =============================================================================

/// Device path type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DevicePathType {
    /// Hardware device path
    HARDWARE       = 0x01,
    /// ACPI device path
    ACPI           = 0x02,
    /// Messaging device path
    MESSAGING      = 0x03,
    /// Media device path
    MEDIA          = 0x04,
    /// BIOS boot specification device path
    BIOS_BOOT_SPEC = 0x05,
    /// End of device path
    END            = 0x7F,
    /// Unknown type
    Unknown        = 0xFF,
}

impl DevicePathType {
    /// Convert from u8
    pub fn from_u8(value: u8) -> Self {
        match value {
            0x01 => Self::HARDWARE,
            0x02 => Self::ACPI,
            0x03 => Self::MESSAGING,
            0x04 => Self::MEDIA,
            0x05 => Self::BIOS_BOOT_SPEC,
            0x7F => Self::END,
            _ => Self::Unknown,
        }
    }
}

/// Device path end sub-types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DevicePathEndSubType {
    /// End of entire path
    ENTIRE   = 0xFF,
    /// End of instance
    INSTANCE = 0x01,
}

// =============================================================================
// HARDWARE DEVICE PATH SUB-TYPES
// =============================================================================

/// Hardware device path sub-types
pub mod hw_subtype {
    /// PCI
    pub const PCI: u8 = 0x01;
    /// PCCARD
    pub const PCCARD: u8 = 0x02;
    /// Memory mapped
    pub const MEMORY_MAPPED: u8 = 0x03;
    /// Vendor
    pub const VENDOR: u8 = 0x04;
    /// Controller
    pub const CONTROLLER: u8 = 0x05;
    /// BMC
    pub const BMC: u8 = 0x06;
}

/// PCI device path
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct PciDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// Function
    pub function: u8,
    /// Device
    pub device: u8,
}

/// Memory mapped device path
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct MemoryMappedDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// Memory type
    pub memory_type: u32,
    /// Start address
    pub start_address: u64,
    /// End address
    pub end_address: u64,
}

// =============================================================================
// ACPI DEVICE PATH SUB-TYPES
// =============================================================================

/// ACPI device path sub-types
pub mod acpi_subtype {
    /// ACPI
    pub const ACPI: u8 = 0x01;
    /// Expanded ACPI
    pub const EXPANDED_ACPI: u8 = 0x02;
    /// ACPI ADR
    pub const ACPI_ADR: u8 = 0x03;
    /// NVDIMM
    pub const NVDIMM: u8 = 0x04;
}

/// ACPI device path
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct AcpiDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// HID
    pub hid: u32,
    /// UID
    pub uid: u32,
}

// =============================================================================
// MESSAGING DEVICE PATH SUB-TYPES
// =============================================================================

/// Messaging device path sub-types
pub mod msg_subtype {
    /// ATAPI
    pub const ATAPI: u8 = 0x01;
    /// SCSI
    pub const SCSI: u8 = 0x02;
    /// Fibre Channel
    pub const FIBRE_CHANNEL: u8 = 0x03;
    /// 1394
    pub const IEEE_1394: u8 = 0x04;
    /// USB
    pub const USB: u8 = 0x05;
    /// I2O
    pub const I2O: u8 = 0x06;
    /// Infiniband
    pub const INFINIBAND: u8 = 0x09;
    /// Vendor
    pub const VENDOR: u8 = 0x0A;
    /// MAC
    pub const MAC: u8 = 0x0B;
    /// IPv4
    pub const IPV4: u8 = 0x0C;
    /// IPv6
    pub const IPV6: u8 = 0x0D;
    /// UART
    pub const UART: u8 = 0x0E;
    /// USB class
    pub const USB_CLASS: u8 = 0x0F;
    /// USB WWID
    pub const USB_WWID: u8 = 0x10;
    /// Device logical unit
    pub const DEVICE_LOGICAL_UNIT: u8 = 0x11;
    /// SATA
    pub const SATA: u8 = 0x12;
    /// iSCSI
    pub const ISCSI: u8 = 0x13;
    /// VLAN
    pub const VLAN: u8 = 0x14;
    /// Fibre Channel Ex
    pub const FIBRE_CHANNEL_EX: u8 = 0x15;
    /// SAS Extended
    pub const SAS_EX: u8 = 0x16;
    /// NVMe
    pub const NVME: u8 = 0x17;
    /// URI
    pub const URI: u8 = 0x18;
    /// UFS
    pub const UFS: u8 = 0x19;
    /// SD
    pub const SD: u8 = 0x1A;
    /// Bluetooth
    pub const BLUETOOTH: u8 = 0x1B;
    /// Wi-Fi
    pub const WIFI: u8 = 0x1C;
    /// eMMC
    pub const EMMC: u8 = 0x1D;
    /// Bluetooth LE
    pub const BLUETOOTH_LE: u8 = 0x1E;
    /// DNS
    pub const DNS: u8 = 0x1F;
    /// NVDIMM
    pub const NVDIMM_NS: u8 = 0x20;
    /// REST Service
    pub const REST_SERVICE: u8 = 0x21;
}

/// USB device path
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct UsbDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// Parent port number
    pub parent_port_number: u8,
    /// Interface number
    pub interface_number: u8,
}

/// SATA device path
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct SataDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// HBA port number
    pub hba_port_number: u16,
    /// Port multiplier port number
    pub port_multiplier_port_number: u16,
    /// Logical unit number
    pub lun: u16,
}

/// NVMe device path
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct NvmeDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// Namespace identifier
    pub namespace_id: u32,
    /// IEEE extended unique identifier
    pub eui64: [u8; 8],
}

/// MAC address device path
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct MacDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// MAC address (32 bytes to accommodate any MAC type)
    pub mac_address: [u8; 32],
    /// Interface type
    pub if_type: u8,
}

impl fmt::Debug for MacDevicePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MacDevicePath")
            .field(
                "mac",
                &format_args!(
                    "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                    self.mac_address[0],
                    self.mac_address[1],
                    self.mac_address[2],
                    self.mac_address[3],
                    self.mac_address[4],
                    self.mac_address[5]
                ),
            )
            .field("if_type", &self.if_type)
            .finish()
    }
}

// =============================================================================
// MEDIA DEVICE PATH SUB-TYPES
// =============================================================================

/// Media device path sub-types
pub mod media_subtype {
    /// Hard drive
    pub const HARD_DRIVE: u8 = 0x01;
    /// CD-ROM
    pub const CDROM: u8 = 0x02;
    /// Vendor
    pub const VENDOR: u8 = 0x03;
    /// File path
    pub const FILE_PATH: u8 = 0x04;
    /// Media protocol
    pub const MEDIA_PROTOCOL: u8 = 0x05;
    /// PIWG firmware file
    pub const PIWG_FIRMWARE_FILE: u8 = 0x06;
    /// PIWG firmware volume
    pub const PIWG_FIRMWARE_VOLUME: u8 = 0x07;
    /// Relative offset range
    pub const RELATIVE_OFFSET_RANGE: u8 = 0x08;
    /// RAM disk
    pub const RAM_DISK: u8 = 0x09;
}

/// Hard drive device path
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct HardDriveDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// Partition number
    pub partition_number: u32,
    /// Partition start
    pub partition_start: u64,
    /// Partition size
    pub partition_size: u64,
    /// Partition signature
    pub partition_signature: [u8; 16],
    /// Partition format (MBR=1, GPT=2)
    pub partition_format: u8,
    /// Signature type
    pub signature_type: u8,
}

impl fmt::Debug for HardDriveDevicePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Copy fields to avoid unaligned reference to packed struct
        let partition_number = self.partition_number;
        let partition_start = self.partition_start;
        let partition_size = self.partition_size;
        let partition_format = self.partition_format;
        f.debug_struct("HardDriveDevicePath")
            .field("partition_number", &partition_number)
            .field("partition_start", &partition_start)
            .field("partition_size", &partition_size)
            .field("partition_format", &partition_format)
            .finish()
    }
}

impl HardDriveDevicePath {
    /// MBR partition format
    pub const PARTITION_FORMAT_MBR: u8 = 0x01;
    /// GPT partition format
    pub const PARTITION_FORMAT_GPT: u8 = 0x02;

    /// No signature
    pub const SIGNATURE_TYPE_NONE: u8 = 0x00;
    /// MBR signature (32-bit)
    pub const SIGNATURE_TYPE_MBR: u8 = 0x01;
    /// GPT signature (GUID)
    pub const SIGNATURE_TYPE_GUID: u8 = 0x02;

    /// Get partition GUID (if GPT)
    pub fn partition_guid(&self) -> Option<Guid> {
        if self.signature_type == Self::SIGNATURE_TYPE_GUID {
            Some(Guid::from_bytes(self.partition_signature))
        } else {
            None
        }
    }

    /// Get MBR signature (if MBR)
    pub fn mbr_signature(&self) -> Option<u32> {
        if self.signature_type == Self::SIGNATURE_TYPE_MBR {
            Some(u32::from_le_bytes([
                self.partition_signature[0],
                self.partition_signature[1],
                self.partition_signature[2],
                self.partition_signature[3],
            ]))
        } else {
            None
        }
    }
}

/// File path device path
#[repr(C, packed)]
pub struct FilePathDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// Path name (UCS-2 string)
    pub path_name: [u16; 0],
}

impl FilePathDevicePath {
    /// Get path as UTF-16 slice
    ///
    /// # Safety
    /// The caller must ensure the device path is valid.
    pub unsafe fn path(&self) -> &[u16] {
        let len = (self.header.len() - 4) / 2;
        let ptr = core::ptr::addr_of!(self.path_name) as *const u16;
        core::slice::from_raw_parts(ptr, len)
    }
}

// =============================================================================
// LOADED IMAGE DEVICE PATH PROTOCOL
// =============================================================================

/// Loaded Image Device Path Protocol
/// Same structure as device path, just different GUID
pub type EfiLoadedImageDevicePathProtocol = EfiDevicePathProtocol;

impl EfiLoadedImageDevicePathProtocol {
    /// Protocol GUID for loaded image device path
    pub const LOADED_IMAGE_DEVICE_PATH_GUID: Guid = guids::LOADED_IMAGE_DEVICE_PATH_PROTOCOL;
}

// =============================================================================
// DEVICE PATH UTILITIES
// =============================================================================

/// Device path utilities protocol
#[repr(C)]
pub struct EfiDevicePathUtilitiesProtocol {
    /// Get device path size
    pub get_device_path_size:
        unsafe extern "efiapi" fn(device_path: *const EfiDevicePathProtocol) -> usize,

    /// Duplicate device path
    pub duplicate_device_path: unsafe extern "efiapi" fn(
        device_path: *const EfiDevicePathProtocol,
    ) -> *mut EfiDevicePathProtocol,

    /// Append device path
    pub append_device_path: unsafe extern "efiapi" fn(
        src1: *const EfiDevicePathProtocol,
        src2: *const EfiDevicePathProtocol,
    ) -> *mut EfiDevicePathProtocol,

    /// Append device node
    pub append_device_node: unsafe extern "efiapi" fn(
        device_path: *const EfiDevicePathProtocol,
        device_node: *const EfiDevicePathProtocol,
    ) -> *mut EfiDevicePathProtocol,

    /// Append device path instance
    pub append_device_path_instance: unsafe extern "efiapi" fn(
        device_path: *const EfiDevicePathProtocol,
        device_path_instance: *const EfiDevicePathProtocol,
    ) -> *mut EfiDevicePathProtocol,

    /// Get next device path instance
    pub get_next_device_path_instance: unsafe extern "efiapi" fn(
        device_path_instance: *mut *mut EfiDevicePathProtocol,
        device_path_instance_size: *mut usize,
    ) -> *mut EfiDevicePathProtocol,

    /// Check if device path is multi-instance
    pub is_device_path_multi_instance:
        unsafe extern "efiapi" fn(device_path: *const EfiDevicePathProtocol) -> bool,

    /// Create device node
    pub create_device_node: unsafe extern "efiapi" fn(
        node_type: u8,
        node_sub_type: u8,
        node_length: u16,
    ) -> *mut EfiDevicePathProtocol,
}

impl EfiDevicePathUtilitiesProtocol {
    /// Protocol GUID
    pub const GUID: Guid = guids::DEVICE_PATH_UTILITIES_PROTOCOL;
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_device_path_type() {
        assert_eq!(DevicePathType::from_u8(0x01), DevicePathType::HARDWARE);
        assert_eq!(DevicePathType::from_u8(0x02), DevicePathType::ACPI);
        assert_eq!(DevicePathType::from_u8(0x7F), DevicePathType::END);
    }

    #[test]
    fn test_hard_drive_device_path() {
        // Verify partition format constants
        assert_eq!(HardDriveDevicePath::PARTITION_FORMAT_MBR, 1);
        assert_eq!(HardDriveDevicePath::PARTITION_FORMAT_GPT, 2);
    }
}
