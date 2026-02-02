//! Device Path Protocol
//!
//! Extended device path types and utilities.

use core::fmt;

// =============================================================================
// RE-EXPORT FROM LOADED_IMAGE
// =============================================================================
pub use super::loaded_image::{
    acpi_subtype,
    hw_subtype,
    media_subtype,
    msg_subtype,
    // ACPI paths
    AcpiDevicePath,
    DevicePathEndSubType,
    DevicePathIter,
    DevicePathType,
    EfiDevicePathProtocol,
    EfiDevicePathUtilitiesProtocol,
    FilePathDevicePath,
    // Media paths
    HardDriveDevicePath,
    MacDevicePath,
    MemoryMappedDevicePath,
    NvmeDevicePath,
    // Hardware paths
    PciDevicePath,
    SataDevicePath,
    // Messaging paths
    UsbDevicePath,
};
use crate::raw::types::*;

// =============================================================================
// DEVICE PATH TO TEXT PROTOCOL
// =============================================================================

/// Device Path to Text Protocol
#[repr(C)]
pub struct EfiDevicePathToTextProtocol {
    /// Convert device node to text
    pub convert_device_node_to_text: unsafe extern "efiapi" fn(
        device_node: *const EfiDevicePathProtocol,
        display_only: bool,
        allow_shortcuts: bool,
    ) -> *mut u16,

    /// Convert device path to text
    pub convert_device_path_to_text: unsafe extern "efiapi" fn(
        device_path: *const EfiDevicePathProtocol,
        display_only: bool,
        allow_shortcuts: bool,
    ) -> *mut u16,
}

impl EfiDevicePathToTextProtocol {
    /// Protocol GUID
    pub const GUID: Guid = guids::DEVICE_PATH_TO_TEXT_PROTOCOL;

    /// Convert a device path to text
    ///
    /// # Safety
    /// The caller must ensure all pointers are valid and the returned
    /// string is freed with FreePool.
    pub unsafe fn device_path_to_text(
        &self,
        path: *const EfiDevicePathProtocol,
        display_only: bool,
    ) -> *mut u16 {
        (self.convert_device_path_to_text)(path, display_only, true)
    }

    /// Convert a single device node to text
    ///
    /// # Safety
    /// The caller must ensure all pointers are valid and the returned
    /// string is freed with FreePool.
    pub unsafe fn device_node_to_text(
        &self,
        node: *const EfiDevicePathProtocol,
        display_only: bool,
    ) -> *mut u16 {
        (self.convert_device_node_to_text)(node, display_only, true)
    }
}

impl fmt::Debug for EfiDevicePathToTextProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EfiDevicePathToTextProtocol").finish()
    }
}

// =============================================================================
// DEVICE PATH FROM TEXT PROTOCOL
// =============================================================================

/// Device Path from Text Protocol
#[repr(C)]
pub struct EfiDevicePathFromTextProtocol {
    /// Convert text to device node
    pub convert_text_to_device_node:
        unsafe extern "efiapi" fn(text_device_node: *const u16) -> *mut EfiDevicePathProtocol,

    /// Convert text to device path
    pub convert_text_to_device_path:
        unsafe extern "efiapi" fn(text_device_path: *const u16) -> *mut EfiDevicePathProtocol,
}

impl EfiDevicePathFromTextProtocol {
    /// Protocol GUID
    pub const GUID: Guid = guids::DEVICE_PATH_FROM_TEXT_PROTOCOL;

    /// Convert text to device path
    ///
    /// # Safety
    /// The caller must ensure the text pointer is valid and the returned
    /// path is freed with FreePool.
    pub unsafe fn text_to_device_path(&self, text: *const u16) -> *mut EfiDevicePathProtocol {
        (self.convert_text_to_device_path)(text)
    }

    /// Convert text to device node
    ///
    /// # Safety
    /// The caller must ensure the text pointer is valid and the returned
    /// node is freed with FreePool.
    pub unsafe fn text_to_device_node(&self, text: *const u16) -> *mut EfiDevicePathProtocol {
        (self.convert_text_to_device_node)(text)
    }
}

impl fmt::Debug for EfiDevicePathFromTextProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EfiDevicePathFromTextProtocol").finish()
    }
}

// =============================================================================
// EXTENDED HARDWARE DEVICE PATHS
// =============================================================================

/// Vendor device path (hardware)
/// Note: Vendor-defined data follows this struct
#[repr(C, packed)]
pub struct VendorDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// Vendor GUID
    pub vendor_guid: Guid,
}

impl VendorDevicePath {
    /// Get vendor data
    ///
    /// # Safety
    /// The caller must ensure the device path is valid.
    pub unsafe fn vendor_data(&self) -> &[u8] {
        let data_len = self.header.len() - 4 - 16; // header + guid
        let ptr = (self as *const Self as *const u8).add(4 + 16);
        core::slice::from_raw_parts(ptr, data_len)
    }
}

impl fmt::Debug for VendorDevicePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Copy field to avoid unaligned reference to packed struct
        let vendor_guid = self.vendor_guid;
        f.debug_struct("VendorDevicePath")
            .field("vendor_guid", &vendor_guid)
            .finish()
    }
}

/// Controller device path
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct ControllerDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// Controller number
    pub controller_number: u32,
}

/// BMC device path
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct BmcDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// Interface type
    pub interface_type: u8,
    /// Base address
    pub base_address: u64,
}

// =============================================================================
// EXTENDED ACPI DEVICE PATHS
// =============================================================================

/// Expanded ACPI device path
/// Note: HIDSTR, UIDSTR, CIDSTR follow (null-terminated ASCII)
#[repr(C, packed)]
pub struct ExpandedAcpiDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// HID
    pub hid: u32,
    /// UID
    pub uid: u32,
    /// CID
    pub cid: u32,
}

impl fmt::Debug for ExpandedAcpiDevicePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Copy fields to avoid unaligned reference to packed struct
        let hid = self.hid;
        let uid = self.uid;
        let cid = self.cid;
        f.debug_struct("ExpandedAcpiDevicePath")
            .field("hid", &hid)
            .field("uid", &uid)
            .field("cid", &cid)
            .finish()
    }
}

/// ACPI ADR device path
/// Note: Additional ADR values follow this struct
#[repr(C, packed)]
pub struct AcpiAdrDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// First ADR value
    pub adr: u32,
}

impl AcpiAdrDevicePath {
    /// Get number of ADR values
    pub fn adr_count(&self) -> usize {
        (self.header.len() - 4) / 4
    }

    /// Get ADR values
    ///
    /// # Safety
    /// The caller must ensure the device path is valid.
    pub unsafe fn adr_values(&self) -> &[u32] {
        let ptr = core::ptr::addr_of!(self.adr);
        core::slice::from_raw_parts(ptr, self.adr_count())
    }
}

impl fmt::Debug for AcpiAdrDevicePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Copy field to avoid unaligned reference to packed struct
        let adr = self.adr;
        f.debug_struct("AcpiAdrDevicePath")
            .field("adr", &adr)
            .finish()
    }
}

// =============================================================================
// EXTENDED MESSAGING DEVICE PATHS
// =============================================================================

/// ATAPI device path
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct AtapiDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// Primary or secondary
    pub primary_secondary: u8,
    /// Master or slave
    pub slave_master: u8,
    /// Logical unit number
    pub lun: u16,
}

/// SCSI device path
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct ScsiDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// Target ID
    pub target_id: u16,
    /// Logical unit number
    pub lun: u16,
}

/// Fibre Channel device path
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct FibreChannelDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// Reserved
    pub reserved: u32,
    /// World Wide Name
    pub wwn: u64,
    /// Logical unit number
    pub lun: u64,
}

/// 1394 device path
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Ieee1394DevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// Reserved
    pub reserved: u32,
    /// GUID
    pub guid: u64,
}

/// USB class device path
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct UsbClassDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// Vendor ID
    pub vendor_id: u16,
    /// Product ID
    pub product_id: u16,
    /// Device class
    pub device_class: u8,
    /// Device subclass
    pub device_subclass: u8,
    /// Device protocol
    pub device_protocol: u8,
}

/// USB WWID device path
/// Note: Serial number (UCS-2) follows this struct
#[repr(C, packed)]
pub struct UsbWwidDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// Interface number
    pub interface_number: u16,
    /// Vendor ID
    pub vendor_id: u16,
    /// Product ID
    pub product_id: u16,
}

impl fmt::Debug for UsbWwidDevicePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let interface_number = self.interface_number;
        let vendor_id = self.vendor_id;
        let product_id = self.product_id;
        f.debug_struct("UsbWwidDevicePath")
            .field("interface_number", &interface_number)
            .field("vendor_id", &vendor_id)
            .field("product_id", &product_id)
            .finish()
    }
}

/// I2O device path
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct I2oDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// Target ID
    pub tid: u32,
}

/// Infiniband device path
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct InfinibandDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// Resource flags
    pub resource_flags: u32,
    /// Port GID
    pub port_gid: [u8; 16],
    /// Service ID
    pub service_id: u64,
    /// Target port ID
    pub target_port_id: u64,
    /// Device ID
    pub device_id: u64,
}

impl fmt::Debug for InfinibandDevicePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let resource_flags = self.resource_flags;
        let service_id = self.service_id;
        f.debug_struct("InfinibandDevicePath")
            .field("resource_flags", &resource_flags)
            .field("service_id", &service_id)
            .finish()
    }
}

/// IPv4 device path
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct Ipv4DevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// Local IP address
    pub local_ip: [u8; 4],
    /// Remote IP address
    pub remote_ip: [u8; 4],
    /// Local port
    pub local_port: u16,
    /// Remote port
    pub remote_port: u16,
    /// Protocol (TCP=6, UDP=17)
    pub protocol: u16,
    /// Static IP address
    pub static_ip: u8,
    /// Gateway IP address
    pub gateway_ip: [u8; 4],
    /// Subnet mask
    pub subnet_mask: [u8; 4],
}

impl fmt::Debug for Ipv4DevicePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let local_ip = self.local_ip;
        let remote_ip = self.remote_ip;
        let local_port = self.local_port;
        let remote_port = self.remote_port;
        f.debug_struct("Ipv4DevicePath")
            .field(
                "local_ip",
                &format_args!(
                    "{}.{}.{}.{}",
                    local_ip[0], local_ip[1], local_ip[2], local_ip[3]
                ),
            )
            .field(
                "remote_ip",
                &format_args!(
                    "{}.{}.{}.{}",
                    remote_ip[0], remote_ip[1], remote_ip[2], remote_ip[3]
                ),
            )
            .field("local_port", &local_port)
            .field("remote_port", &remote_port)
            .finish()
    }
}

/// IPv6 device path
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct Ipv6DevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// Local IP address
    pub local_ip: [u8; 16],
    /// Remote IP address
    pub remote_ip: [u8; 16],
    /// Local port
    pub local_port: u16,
    /// Remote port
    pub remote_port: u16,
    /// Protocol
    pub protocol: u16,
    /// IP address origin
    pub ip_address_origin: u8,
    /// Prefix length
    pub prefix_length: u8,
    /// Gateway IP address
    pub gateway_ip: [u8; 16],
}

impl fmt::Debug for Ipv6DevicePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let local_port = self.local_port;
        let remote_port = self.remote_port;
        let prefix_length = self.prefix_length;
        f.debug_struct("Ipv6DevicePath")
            .field("local_port", &local_port)
            .field("remote_port", &remote_port)
            .field("prefix_length", &prefix_length)
            .finish()
    }
}

/// UART device path
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct UartDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// Reserved
    pub reserved: u32,
    /// Baud rate
    pub baud_rate: u64,
    /// Data bits
    pub data_bits: u8,
    /// Parity
    pub parity: u8,
    /// Stop bits
    pub stop_bits: u8,
}

/// iSCSI device path
/// Note: Target name follows this struct (iSCSI name)
#[repr(C, packed)]
pub struct IscsiDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// Protocol (0=TCP)
    pub protocol: u16,
    /// Options
    pub options: u16,
    /// Logical unit number
    pub lun: u64,
    /// Target portal group tag
    pub target_portal_group_tag: u16,
}

impl fmt::Debug for IscsiDevicePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let protocol = self.protocol;
        let lun = self.lun;
        f.debug_struct("IscsiDevicePath")
            .field("protocol", &protocol)
            .field("lun", &lun)
            .finish()
    }
}

/// VLAN device path
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct VlanDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// VLAN ID
    pub vlan_id: u16,
}

/// SAS Extended device path
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct SasExDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// SAS address
    pub sas_address: u64,
    /// Logical unit number
    pub lun: u64,
    /// Device and topology info
    pub device_topology_info: u16,
    /// Relative target port
    pub relative_target_port: u16,
}

impl fmt::Debug for SasExDevicePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sas_address = self.sas_address;
        let lun = self.lun;
        f.debug_struct("SasExDevicePath")
            .field("sas_address", &sas_address)
            .field("lun", &lun)
            .finish()
    }
}

/// URI device path
/// Note: URI follows this struct (ASCII string)
#[repr(C, packed)]
pub struct UriDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
}

impl UriDevicePath {
    /// Get URI string
    ///
    /// # Safety
    /// The caller must ensure the device path is valid.
    pub unsafe fn uri(&self) -> &[u8] {
        let len = self.header.len() - 4;
        let ptr = (self as *const Self as *const u8).add(4);
        core::slice::from_raw_parts(ptr, len)
    }
}

impl fmt::Debug for UriDevicePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UriDevicePath").finish()
    }
}

/// UFS device path
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct UfsDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// Target ID
    pub target_id: u8,
    /// LUN
    pub lun: u8,
}

/// SD device path
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct SdDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// Slot number
    pub slot_number: u8,
}

/// eMMC device path
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct EmmcDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// Slot number
    pub slot_number: u8,
}

/// Bluetooth device path
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct BluetoothDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// Bluetooth address
    pub device_address: [u8; 6],
}

impl fmt::Debug for BluetoothDevicePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BluetoothDevicePath")
            .field(
                "device_address",
                &format_args!(
                    "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                    self.device_address[0],
                    self.device_address[1],
                    self.device_address[2],
                    self.device_address[3],
                    self.device_address[4],
                    self.device_address[5]
                ),
            )
            .finish()
    }
}

/// Wi-Fi device path
#[repr(C, packed)]
pub struct WifiDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// SSID (32 bytes max)
    pub ssid: [u8; 32],
}

impl fmt::Debug for WifiDevicePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WifiDevicePath").finish()
    }
}

// =============================================================================
// EXTENDED MEDIA DEVICE PATHS
// =============================================================================

/// CD-ROM device path
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct CdromDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// Boot entry
    pub boot_entry: u32,
    /// Partition start
    pub partition_start: u64,
    /// Partition size
    pub partition_size: u64,
}

/// Relative offset range device path
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct RelativeOffsetRangeDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// Reserved
    pub reserved: u32,
    /// Starting offset
    pub starting_offset: u64,
    /// Ending offset
    pub ending_offset: u64,
}

/// RAM disk device path
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct RamDiskDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// Starting address
    pub starting_address: u64,
    /// Ending address
    pub ending_address: u64,
    /// Disk type GUID
    pub disk_type_guid: Guid,
    /// Disk instance
    pub disk_instance: u16,
}

impl fmt::Debug for RamDiskDevicePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let starting_address = self.starting_address;
        let ending_address = self.ending_address;
        let disk_type_guid = self.disk_type_guid;
        f.debug_struct("RamDiskDevicePath")
            .field("starting_address", &starting_address)
            .field("ending_address", &ending_address)
            .field("disk_type_guid", &disk_type_guid)
            .finish()
    }
}

/// RAM disk type GUIDs
pub mod ram_disk_type {
    use super::*;

    /// Virtual disk
    pub const VIRTUAL_DISK: Guid = Guid::new(0x77AB535A, 0x45FC, 0x624B, [
        0x55, 0x60, 0xF7, 0xB2, 0x81, 0xD1, 0xF9, 0x6E,
    ]);

    /// Virtual CD
    pub const VIRTUAL_CD: Guid = Guid::new(0x3D5ABD30, 0x4175, 0x87CE, [
        0x6D, 0x64, 0xD2, 0xAD, 0xE5, 0x23, 0xC4, 0xBB,
    ]);

    /// Persistent virtual disk
    pub const PERSISTENT_VIRTUAL_DISK: Guid = Guid::new(0x5CEA02C9, 0x4D07, 0x69D3, [
        0x26, 0x9F, 0x44, 0x96, 0xFB, 0xE0, 0x96, 0xF9,
    ]);

    /// Persistent virtual CD
    pub const PERSISTENT_VIRTUAL_CD: Guid = Guid::new(0x08018188, 0x42CD, 0xBB48, [
        0x10, 0x0F, 0x53, 0x87, 0xD5, 0x3D, 0xED, 0x3D,
    ]);
}

// =============================================================================
// BIOS BOOT SPECIFICATION DEVICE PATHS
// =============================================================================

/// BIOS boot specification device path
/// Note: Description string follows (null-terminated ASCII)
#[repr(C, packed)]
pub struct BbsDevicePath {
    /// Header
    pub header: EfiDevicePathProtocol,
    /// Device type
    pub device_type: u16,
    /// Status flag
    pub status_flag: u16,
}

impl BbsDevicePath {
    /// Get description string
    ///
    /// # Safety
    /// The caller must ensure the device path is valid.
    pub unsafe fn description(&self) -> &[u8] {
        let len = self.header.len() - 8;
        let ptr = (self as *const Self as *const u8).add(8);
        core::slice::from_raw_parts(ptr, len)
    }
}

impl fmt::Debug for BbsDevicePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let device_type = self.device_type;
        let status_flag = self.status_flag;
        f.debug_struct("BbsDevicePath")
            .field("device_type", &device_type)
            .field("status_flag", &status_flag)
            .finish()
    }
}

/// BBS device types
pub mod bbs_device_type {
    /// Floppy
    pub const FLOPPY: u16 = 0x01;
    /// Hard drive
    pub const HARD_DRIVE: u16 = 0x02;
    /// CD-ROM
    pub const CDROM: u16 = 0x03;
    /// PCMCIA
    pub const PCMCIA: u16 = 0x04;
    /// USB
    pub const USB: u16 = 0x05;
    /// Embedded network
    pub const EMBEDDED_NETWORK: u16 = 0x06;
    /// BEV (Bootstrap Entry Vector)
    pub const BEV: u16 = 0x80;
    /// Unknown
    pub const UNKNOWN: u16 = 0xFF;
}

// =============================================================================
// DEVICE PATH HELPER FUNCTIONS
// =============================================================================

/// Calculate device path size including end node
///
/// # Safety
/// The caller must ensure the device path is valid.
pub unsafe fn device_path_size(path: *const EfiDevicePathProtocol) -> usize {
    if path.is_null() {
        return 0;
    }

    let mut current = path;
    let mut size = 0usize;

    loop {
        let node = &*current;
        let node_len = node.len();
        size += node_len;

        if node.is_end_entire() {
            break;
        }

        current = node.next();
    }

    size
}

/// Count nodes in device path (excluding end node)
///
/// # Safety
/// The caller must ensure the device path is valid.
pub unsafe fn device_path_node_count(path: *const EfiDevicePathProtocol) -> usize {
    if path.is_null() {
        return 0;
    }

    let mut current = path;
    let mut count = 0usize;

    loop {
        let node = &*current;

        if node.is_end_entire() {
            break;
        }

        count += 1;
        current = node.next();
    }

    count
}

/// Check if device path is a single node
///
/// # Safety
/// The caller must ensure the device path is valid.
pub unsafe fn is_device_path_single_node(path: *const EfiDevicePathProtocol) -> bool {
    if path.is_null() {
        return false;
    }

    let node = &*path;
    if node.is_end() {
        return false;
    }

    let next = &*node.next();
    next.is_end_entire()
}

/// Get the last node before end (device node)
///
/// # Safety
/// The caller must ensure the device path is valid.
pub unsafe fn device_path_last_node(
    path: *const EfiDevicePathProtocol,
) -> *const EfiDevicePathProtocol {
    if path.is_null() {
        return core::ptr::null();
    }

    let mut current = path;
    let mut last = path;

    loop {
        let node = &*current;

        if node.is_end_entire() {
            break;
        }

        last = current;
        current = node.next();
    }

    last
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bbs_device_types() {
        assert_eq!(bbs_device_type::FLOPPY, 0x01);
        assert_eq!(bbs_device_type::HARD_DRIVE, 0x02);
        assert_eq!(bbs_device_type::CDROM, 0x03);
    }
}
