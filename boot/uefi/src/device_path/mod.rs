//! UEFI Device Path Utilities
//!
//! Device path parsing, building, and manipulation for UEFI.


// =============================================================================
// DEVICE PATH TYPES
// =============================================================================

/// Device path type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DevicePathType {
    /// Hardware device path
    Hardware = 0x01,
    /// ACPI device path
    Acpi = 0x02,
    /// Messaging device path
    Messaging = 0x03,
    /// Media device path
    Media = 0x04,
    /// BIOS boot specification device path
    BiosBootSpec = 0x05,
    /// End of hardware device path
    End = 0x7F,
}

impl DevicePathType {
    /// From raw value
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x01 => Some(Self::Hardware),
            0x02 => Some(Self::Acpi),
            0x03 => Some(Self::Messaging),
            0x04 => Some(Self::Media),
            0x05 => Some(Self::BiosBootSpec),
            0x7F => Some(Self::End),
            _ => None,
        }
    }
}

/// Hardware device path subtypes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HardwareSubtype {
    /// PCI device
    Pci = 0x01,
    /// PCI card
    PciCard = 0x02,
    /// Memory mapped
    MemoryMapped = 0x03,
    /// Vendor defined
    Vendor = 0x04,
    /// Controller
    Controller = 0x05,
    /// BMC
    Bmc = 0x06,
}

/// ACPI device path subtypes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AcpiSubtype {
    /// Standard ACPI
    Acpi = 0x01,
    /// Expanded ACPI
    ExpandedAcpi = 0x02,
    /// ADR ACPI
    Adr = 0x03,
    /// NVDIMM
    Nvdimm = 0x04,
}

/// Messaging device path subtypes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessagingSubtype {
    /// ATAPI
    Atapi = 0x01,
    /// SCSI
    Scsi = 0x02,
    /// Fibre Channel
    FibreChannel = 0x03,
    /// 1394 (FireWire)
    FireWire = 0x04,
    /// USB
    Usb = 0x05,
    /// I2O
    I2o = 0x06,
    /// InfiniBand
    InfiniBand = 0x09,
    /// Vendor defined
    Vendor = 0x0A,
    /// MAC address
    MacAddress = 0x0B,
    /// IPv4
    Ipv4 = 0x0C,
    /// IPv6
    Ipv6 = 0x0D,
    /// UART
    Uart = 0x0E,
    /// USB class
    UsbClass = 0x0F,
    /// USB WWID
    UsbWwid = 0x10,
    /// Device logical unit
    DeviceLogicalUnit = 0x11,
    /// SATA
    Sata = 0x12,
    /// iSCSI
    Iscsi = 0x13,
    /// VLAN
    Vlan = 0x14,
    /// Fibre Channel Ex
    FibreChannelEx = 0x15,
    /// SAS Ex
    SasEx = 0x16,
    /// NVMe namespace
    NvmeNamespace = 0x17,
    /// URI
    Uri = 0x18,
    /// UFS
    Ufs = 0x19,
    /// SD
    Sd = 0x1A,
    /// Bluetooth
    Bluetooth = 0x1B,
    /// WiFi
    Wifi = 0x1C,
    /// eMMC
    Emmc = 0x1D,
    /// Bluetooth LE
    BluetoothLe = 0x1E,
    /// DNS
    Dns = 0x1F,
    /// NVMe over Fabric
    NvmeOfNs = 0x20,
    /// REST Service
    RestService = 0x21,
}

/// Media device path subtypes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MediaSubtype {
    /// Hard drive
    HardDrive = 0x01,
    /// CD-ROM
    CdRom = 0x02,
    /// Vendor defined
    Vendor = 0x03,
    /// File path
    FilePath = 0x04,
    /// Media protocol
    MediaProtocol = 0x05,
    /// PIWG firmware file
    PiwgFirmwareFile = 0x06,
    /// PIWG firmware volume
    PiwgFirmwareVolume = 0x07,
    /// Relative offset range
    RelativeOffsetRange = 0x08,
    /// RAM disk
    RamDisk = 0x09,
}

/// End device path subtypes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EndSubtype {
    /// End this instance
    EndThisInstance = 0x01,
    /// End entire path
    EndEntirePath = 0xFF,
}

// =============================================================================
// DEVICE PATH NODE
// =============================================================================

/// Device path node header
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct DevicePathNodeHeader {
    /// Type
    pub device_type: u8,
    /// Subtype
    pub sub_type: u8,
    /// Length (including header)
    pub length: [u8; 2],
}

impl DevicePathNodeHeader {
    /// Get length as u16
    pub fn length(&self) -> u16 {
        u16::from_le_bytes(self.length)
    }

    /// Create new header
    pub fn new(device_type: u8, sub_type: u8, length: u16) -> Self {
        Self {
            device_type,
            sub_type,
            length: length.to_le_bytes(),
        }
    }

    /// Check if this is an end node
    pub fn is_end(&self) -> bool {
        self.device_type == DevicePathType::End as u8
    }

    /// Check if this is end of entire path
    pub fn is_end_entire(&self) -> bool {
        self.device_type == DevicePathType::End as u8 &&
        self.sub_type == EndSubtype::EndEntirePath as u8
    }
}

/// Device path node
#[derive(Clone)]
pub struct DevicePathNode {
    /// Header
    pub header: DevicePathNodeHeader,
    /// Data (excluding header)
    pub data: [u8; 252], // Max node size is 256, minus 4 byte header
    pub data_len: usize,
}

impl DevicePathNode {
    /// Minimum node size
    pub const MIN_SIZE: usize = 4;

    /// Maximum node size
    pub const MAX_SIZE: usize = 256;

    /// Create end node
    pub fn end_entire() -> Self {
        Self {
            header: DevicePathNodeHeader::new(
                DevicePathType::End as u8,
                EndSubtype::EndEntirePath as u8,
                4,
            ),
            data: [0; 252],
            data_len: 0,
        }
    }

    /// Create end instance node
    pub fn end_instance() -> Self {
        Self {
            header: DevicePathNodeHeader::new(
                DevicePathType::End as u8,
                EndSubtype::EndThisInstance as u8,
                4,
            ),
            data: [0; 252],
            data_len: 0,
        }
    }

    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::MIN_SIZE {
            return None;
        }

        let header = DevicePathNodeHeader {
            device_type: bytes[0],
            sub_type: bytes[1],
            length: [bytes[2], bytes[3]],
        };

        let length = header.length() as usize;
        if length < Self::MIN_SIZE || length > bytes.len() {
            return None;
        }

        let data_len = (length - 4).min(252);
        let mut data = [0u8; 252];
        data[..data_len].copy_from_slice(&bytes[4..4 + data_len]);

        Some(Self { header, data, data_len })
    }

    /// Get type
    pub fn device_type(&self) -> Option<DevicePathType> {
        DevicePathType::from_u8(self.header.device_type)
    }

    /// Get length
    pub fn length(&self) -> u16 {
        self.header.length()
    }

    /// Is end node
    pub fn is_end(&self) -> bool {
        self.header.is_end()
    }

    /// To bytes
    pub fn to_bytes(&self, buffer: &mut [u8]) -> usize {
        let len = self.header.length() as usize;
        if buffer.len() < len {
            return 0;
        }

        buffer[0] = self.header.device_type;
        buffer[1] = self.header.sub_type;
        buffer[2..4].copy_from_slice(&self.header.length);

        if self.data_len > 0 {
            buffer[4..4 + self.data_len].copy_from_slice(&self.data[..self.data_len]);
        }

        len
    }
}

// =============================================================================
// SPECIFIC NODE TYPES
// =============================================================================

/// PCI device path node
#[repr(C, packed)]
pub struct PciDevicePath {
    pub header: DevicePathNodeHeader,
    pub function: u8,
    pub device: u8,
}

impl PciDevicePath {
    /// Create new PCI device path node
    pub fn new(device: u8, function: u8) -> Self {
        Self {
            header: DevicePathNodeHeader::new(
                DevicePathType::Hardware as u8,
                HardwareSubtype::Pci as u8,
                6,
            ),
            function,
            device,
        }
    }

    /// Parse from node
    pub fn from_node(node: &DevicePathNode) -> Option<Self> {
        if node.header.device_type != DevicePathType::Hardware as u8 ||
           node.header.sub_type != HardwareSubtype::Pci as u8 ||
           node.data_len < 2 {
            return None;
        }

        Some(Self {
            header: node.header,
            function: node.data[0],
            device: node.data[1],
        })
    }
}

/// ACPI device path node
#[repr(C, packed)]
pub struct AcpiDevicePath {
    pub header: DevicePathNodeHeader,
    pub hid: u32,
    pub uid: u32,
}

impl AcpiDevicePath {
    /// Create ACPI device path
    pub fn new(hid: u32, uid: u32) -> Self {
        Self {
            header: DevicePathNodeHeader::new(
                DevicePathType::Acpi as u8,
                AcpiSubtype::Acpi as u8,
                12,
            ),
            hid,
            uid,
        }
    }

    /// Parse from node
    pub fn from_node(node: &DevicePathNode) -> Option<Self> {
        if node.header.device_type != DevicePathType::Acpi as u8 ||
           node.header.sub_type != AcpiSubtype::Acpi as u8 ||
           node.data_len < 8 {
            return None;
        }

        let hid = u32::from_le_bytes([node.data[0], node.data[1], node.data[2], node.data[3]]);
        let uid = u32::from_le_bytes([node.data[4], node.data[5], node.data[6], node.data[7]]);

        Some(Self {
            header: node.header,
            hid,
            uid,
        })
    }

    /// Encode EISA ID
    pub fn eisa_id(vendor: &[u8; 3], product: u16) -> u32 {
        let v1 = ((vendor[0] as u32 - 0x40) & 0x1F) << 10;
        let v2 = ((vendor[1] as u32 - 0x40) & 0x1F) << 5;
        let v3 = (vendor[2] as u32 - 0x40) & 0x1F;
        let vid = v1 | v2 | v3;
        (vid << 16) | (product as u32)
    }
}

/// USB device path node
#[repr(C, packed)]
pub struct UsbDevicePath {
    pub header: DevicePathNodeHeader,
    pub parent_port: u8,
    pub interface: u8,
}

impl UsbDevicePath {
    /// Create USB device path
    pub fn new(parent_port: u8, interface: u8) -> Self {
        Self {
            header: DevicePathNodeHeader::new(
                DevicePathType::Messaging as u8,
                MessagingSubtype::Usb as u8,
                6,
            ),
            parent_port,
            interface,
        }
    }
}

/// SATA device path node
#[repr(C, packed)]
pub struct SataDevicePath {
    pub header: DevicePathNodeHeader,
    pub hba_port: u16,
    pub port_multiplier: u16,
    pub lun: u16,
}

impl SataDevicePath {
    /// Create SATA device path
    pub fn new(hba_port: u16, port_multiplier: u16, lun: u16) -> Self {
        Self {
            header: DevicePathNodeHeader::new(
                DevicePathType::Messaging as u8,
                MessagingSubtype::Sata as u8,
                10,
            ),
            hba_port,
            port_multiplier,
            lun,
        }
    }
}

/// NVMe namespace device path node
#[repr(C, packed)]
pub struct NvmeNamespacePath {
    pub header: DevicePathNodeHeader,
    pub namespace_id: u32,
    pub eui64: [u8; 8],
}

impl NvmeNamespacePath {
    /// Create NVMe namespace path
    pub fn new(namespace_id: u32, eui64: [u8; 8]) -> Self {
        Self {
            header: DevicePathNodeHeader::new(
                DevicePathType::Messaging as u8,
                MessagingSubtype::NvmeNamespace as u8,
                16,
            ),
            namespace_id,
            eui64,
        }
    }
}

/// MAC address device path node
#[repr(C, packed)]
pub struct MacAddressPath {
    pub header: DevicePathNodeHeader,
    pub mac_address: [u8; 32],
    pub if_type: u8,
}

impl MacAddressPath {
    /// Create MAC address path
    pub fn new(mac: [u8; 6], if_type: u8) -> Self {
        let mut mac_address = [0u8; 32];
        mac_address[..6].copy_from_slice(&mac);

        Self {
            header: DevicePathNodeHeader::new(
                DevicePathType::Messaging as u8,
                MessagingSubtype::MacAddress as u8,
                37,
            ),
            mac_address,
            if_type,
        }
    }
}

/// IPv4 device path node
#[repr(C, packed)]
pub struct Ipv4DevicePath {
    pub header: DevicePathNodeHeader,
    pub local_ip: [u8; 4],
    pub remote_ip: [u8; 4],
    pub local_port: u16,
    pub remote_port: u16,
    pub protocol: u16,
    pub static_ip: u8,
    pub gateway: [u8; 4],
    pub subnet_mask: [u8; 4],
}

impl Ipv4DevicePath {
    /// Create IPv4 device path
    pub fn new(
        local_ip: [u8; 4],
        remote_ip: [u8; 4],
        local_port: u16,
        remote_port: u16,
        protocol: u16,
        static_ip: bool,
    ) -> Self {
        Self {
            header: DevicePathNodeHeader::new(
                DevicePathType::Messaging as u8,
                MessagingSubtype::Ipv4 as u8,
                27,
            ),
            local_ip,
            remote_ip,
            local_port,
            remote_port,
            protocol,
            static_ip: static_ip as u8,
            gateway: [0; 4],
            subnet_mask: [255, 255, 255, 0],
        }
    }
}

/// IPv6 device path node
#[repr(C, packed)]
pub struct Ipv6DevicePath {
    pub header: DevicePathNodeHeader,
    pub local_ip: [u8; 16],
    pub remote_ip: [u8; 16],
    pub local_port: u16,
    pub remote_port: u16,
    pub protocol: u16,
    pub ip_address_origin: u8,
    pub prefix_length: u8,
    pub gateway: [u8; 16],
}

/// URI device path node
pub struct UriDevicePath {
    pub header: DevicePathNodeHeader,
    pub uri: [u8; 248],
    pub uri_len: usize,
}

impl UriDevicePath {
    /// Create URI device path
    pub fn new(uri: &str) -> Self {
        let uri_bytes = uri.as_bytes();
        let uri_len = uri_bytes.len().min(248);
        let mut uri_buf = [0u8; 248];
        uri_buf[..uri_len].copy_from_slice(&uri_bytes[..uri_len]);

        Self {
            header: DevicePathNodeHeader::new(
                DevicePathType::Messaging as u8,
                MessagingSubtype::Uri as u8,
                (4 + uri_len) as u16,
            ),
            uri: uri_buf,
            uri_len,
        }
    }

    /// Get URI string
    pub fn uri_str(&self) -> &str {
        core::str::from_utf8(&self.uri[..self.uri_len]).unwrap_or("")
    }
}

/// Hard drive device path node
#[repr(C, packed)]
pub struct HardDriveDevicePath {
    pub header: DevicePathNodeHeader,
    pub partition_number: u32,
    pub partition_start: u64,
    pub partition_size: u64,
    pub partition_signature: [u8; 16],
    pub partition_format: u8,
    pub signature_type: u8,
}

/// Partition format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PartitionFormat {
    /// Legacy MBR
    Mbr = 0x01,
    /// GPT
    Gpt = 0x02,
}

/// Signature type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SignatureType {
    /// No signature
    None = 0x00,
    /// 32-bit MBR signature
    Mbr = 0x01,
    /// GUID signature
    Guid = 0x02,
}

impl HardDriveDevicePath {
    /// Create hard drive device path (GPT)
    pub fn new_gpt(
        partition_number: u32,
        partition_start: u64,
        partition_size: u64,
        partition_guid: [u8; 16],
    ) -> Self {
        Self {
            header: DevicePathNodeHeader::new(
                DevicePathType::Media as u8,
                MediaSubtype::HardDrive as u8,
                42,
            ),
            partition_number,
            partition_start,
            partition_size,
            partition_signature: partition_guid,
            partition_format: PartitionFormat::Gpt as u8,
            signature_type: SignatureType::Guid as u8,
        }
    }

    /// Create hard drive device path (MBR)
    pub fn new_mbr(
        partition_number: u32,
        partition_start: u64,
        partition_size: u64,
        signature: u32,
    ) -> Self {
        let mut sig = [0u8; 16];
        sig[..4].copy_from_slice(&signature.to_le_bytes());

        Self {
            header: DevicePathNodeHeader::new(
                DevicePathType::Media as u8,
                MediaSubtype::HardDrive as u8,
                42,
            ),
            partition_number,
            partition_start,
            partition_size,
            partition_signature: sig,
            partition_format: PartitionFormat::Mbr as u8,
            signature_type: SignatureType::Mbr as u8,
        }
    }
}

/// File path device path node
pub struct FilePathDevicePath {
    pub header: DevicePathNodeHeader,
    pub path: [u16; 128],
    pub path_len: usize,
}

impl FilePathDevicePath {
    /// Create file path device path
    pub fn new(path: &str) -> Self {
        let mut path_buf = [0u16; 128];
        let mut len = 0;

        for c in path.chars() {
            if len >= 127 {
                break;
            }
            // Convert to UCS-2 (simple ASCII)
            path_buf[len] = c as u16;
            len += 1;
        }
        path_buf[len] = 0; // Null terminator

        Self {
            header: DevicePathNodeHeader::new(
                DevicePathType::Media as u8,
                MediaSubtype::FilePath as u8,
                (4 + (len + 1) * 2) as u16,
            ),
            path: path_buf,
            path_len: len,
        }
    }

    /// Get path string
    pub fn path_str(&self, buffer: &mut [u8]) -> usize {
        let mut pos = 0;
        for i in 0..self.path_len {
            if pos >= buffer.len() {
                break;
            }
            buffer[pos] = self.path[i] as u8;
            pos += 1;
        }
        pos
    }
}

/// RAM disk device path node
#[repr(C, packed)]
pub struct RamDiskDevicePath {
    pub header: DevicePathNodeHeader,
    pub starting_address: u64,
    pub ending_address: u64,
    pub disk_type: [u8; 16],
    pub disk_instance: u16,
}

/// RAM disk type GUIDs
pub mod ram_disk_type {
    /// Virtual disk
    pub const VIRTUAL_DISK: [u8; 16] = [
        0x5C, 0xED, 0x45, 0x77, 0xD4, 0x29, 0xC7, 0x4D,
        0xB7, 0x48, 0x39, 0x4A, 0xBC, 0x0A, 0x24, 0x38,
    ];

    /// Virtual CD
    pub const VIRTUAL_CD: [u8; 16] = [
        0x4E, 0x2C, 0x91, 0x38, 0x29, 0x41, 0x4B, 0x4A,
        0xAE, 0x38, 0x29, 0x45, 0x98, 0xBF, 0xB9, 0x87,
    ];

    /// Persistent virtual disk
    pub const PERSISTENT_VIRTUAL_DISK: [u8; 16] = [
        0x12, 0x1E, 0x82, 0x5D, 0x3F, 0x97, 0x4D, 0x46,
        0xB0, 0x33, 0xF4, 0xB4, 0x77, 0xD3, 0x43, 0x13,
    ];

    /// Persistent virtual CD
    pub const PERSISTENT_VIRTUAL_CD: [u8; 16] = [
        0x97, 0x79, 0x76, 0x5B, 0xF9, 0x18, 0xB0, 0x44,
        0xA4, 0x10, 0x57, 0xB5, 0x16, 0xD0, 0xE5, 0xC3,
    ];
}

impl RamDiskDevicePath {
    /// Create RAM disk device path
    pub fn new(
        starting_address: u64,
        ending_address: u64,
        disk_type: [u8; 16],
        disk_instance: u16,
    ) -> Self {
        Self {
            header: DevicePathNodeHeader::new(
                DevicePathType::Media as u8,
                MediaSubtype::RamDisk as u8,
                38,
            ),
            starting_address,
            ending_address,
            disk_type,
            disk_instance,
        }
    }
}

// =============================================================================
// DEVICE PATH
// =============================================================================

/// Complete device path
pub struct DevicePath {
    /// Nodes
    nodes: [DevicePathNode; 16],
    /// Node count
    node_count: usize,
}

impl DevicePath {
    /// Maximum nodes
    pub const MAX_NODES: usize = 16;

    /// Create empty device path
    pub fn new() -> Self {
        Self {
            nodes: core::array::from_fn(|_| DevicePathNode::end_entire()),
            node_count: 0,
        }
    }

    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut path = Self::new();
        let mut pos = 0;

        while pos < bytes.len() {
            if bytes.len() - pos < 4 {
                break;
            }

            let node = DevicePathNode::from_bytes(&bytes[pos..])?;
            let node_len = node.length() as usize;

            if node.is_end() {
                break;
            }

            if path.node_count >= Self::MAX_NODES {
                break;
            }

            path.nodes[path.node_count] = node;
            path.node_count += 1;
            pos += node_len;
        }

        Some(path)
    }

    /// Add node
    pub fn add_node(&mut self, node: DevicePathNode) -> bool {
        if self.node_count >= Self::MAX_NODES {
            return false;
        }

        self.nodes[self.node_count] = node;
        self.node_count += 1;
        true
    }

    /// Get nodes
    pub fn nodes(&self) -> &[DevicePathNode] {
        &self.nodes[..self.node_count]
    }

    /// Get node count
    pub fn node_count(&self) -> usize {
        self.node_count
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.node_count == 0
    }

    /// Get total length (including end node)
    pub fn length(&self) -> usize {
        let mut len = 0;
        for node in &self.nodes[..self.node_count] {
            len += node.length() as usize;
        }
        len + 4 // End node
    }

    /// To bytes
    pub fn to_bytes(&self, buffer: &mut [u8]) -> usize {
        let mut pos = 0;

        for node in &self.nodes[..self.node_count] {
            let written = node.to_bytes(&mut buffer[pos..]);
            pos += written;
        }

        // Write end node
        let end = DevicePathNode::end_entire();
        pos += end.to_bytes(&mut buffer[pos..]);

        pos
    }

    /// Append another path
    pub fn append(&mut self, other: &DevicePath) {
        for node in other.nodes() {
            if !self.add_node(node.clone()) {
                break;
            }
        }
    }

    /// Get last node
    pub fn last_node(&self) -> Option<&DevicePathNode> {
        if self.node_count > 0 {
            Some(&self.nodes[self.node_count - 1])
        } else {
            None
        }
    }

    /// Get parent path (without last node)
    pub fn parent(&self) -> Option<Self> {
        if self.node_count <= 1 {
            return None;
        }

        let mut parent = Self::new();
        for i in 0..self.node_count - 1 {
            parent.add_node(self.nodes[i].clone());
        }
        Some(parent)
    }
}

impl Default for DevicePath {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// DEVICE PATH BUILDER
// =============================================================================

/// Device path builder
pub struct DevicePathBuilder {
    path: DevicePath,
}

impl DevicePathBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            path: DevicePath::new(),
        }
    }

    /// Add PCI node
    pub fn pci(mut self, device: u8, function: u8) -> Self {
        let node = DevicePathNode {
            header: DevicePathNodeHeader::new(
                DevicePathType::Hardware as u8,
                HardwareSubtype::Pci as u8,
                6,
            ),
            data: {
                let mut d = [0u8; 252];
                d[0] = function;
                d[1] = device;
                d
            },
            data_len: 2,
        };
        self.path.add_node(node);
        self
    }

    /// Add ACPI node
    pub fn acpi(mut self, hid: u32, uid: u32) -> Self {
        let node = DevicePathNode {
            header: DevicePathNodeHeader::new(
                DevicePathType::Acpi as u8,
                AcpiSubtype::Acpi as u8,
                12,
            ),
            data: {
                let mut d = [0u8; 252];
                d[0..4].copy_from_slice(&hid.to_le_bytes());
                d[4..8].copy_from_slice(&uid.to_le_bytes());
                d
            },
            data_len: 8,
        };
        self.path.add_node(node);
        self
    }

    /// Add USB node
    pub fn usb(mut self, parent_port: u8, interface: u8) -> Self {
        let node = DevicePathNode {
            header: DevicePathNodeHeader::new(
                DevicePathType::Messaging as u8,
                MessagingSubtype::Usb as u8,
                6,
            ),
            data: {
                let mut d = [0u8; 252];
                d[0] = parent_port;
                d[1] = interface;
                d
            },
            data_len: 2,
        };
        self.path.add_node(node);
        self
    }

    /// Add SATA node
    pub fn sata(mut self, hba_port: u16, port_multiplier: u16, lun: u16) -> Self {
        let node = DevicePathNode {
            header: DevicePathNodeHeader::new(
                DevicePathType::Messaging as u8,
                MessagingSubtype::Sata as u8,
                10,
            ),
            data: {
                let mut d = [0u8; 252];
                d[0..2].copy_from_slice(&hba_port.to_le_bytes());
                d[2..4].copy_from_slice(&port_multiplier.to_le_bytes());
                d[4..6].copy_from_slice(&lun.to_le_bytes());
                d
            },
            data_len: 6,
        };
        self.path.add_node(node);
        self
    }

    /// Add NVMe node
    pub fn nvme(mut self, namespace_id: u32, eui64: [u8; 8]) -> Self {
        let node = DevicePathNode {
            header: DevicePathNodeHeader::new(
                DevicePathType::Messaging as u8,
                MessagingSubtype::NvmeNamespace as u8,
                16,
            ),
            data: {
                let mut d = [0u8; 252];
                d[0..4].copy_from_slice(&namespace_id.to_le_bytes());
                d[4..12].copy_from_slice(&eui64);
                d
            },
            data_len: 12,
        };
        self.path.add_node(node);
        self
    }

    /// Add hard drive partition node (GPT)
    pub fn gpt_partition(
        mut self,
        partition_number: u32,
        partition_start: u64,
        partition_size: u64,
        partition_guid: [u8; 16],
    ) -> Self {
        let node = DevicePathNode {
            header: DevicePathNodeHeader::new(
                DevicePathType::Media as u8,
                MediaSubtype::HardDrive as u8,
                42,
            ),
            data: {
                let mut d = [0u8; 252];
                d[0..4].copy_from_slice(&partition_number.to_le_bytes());
                d[4..12].copy_from_slice(&partition_start.to_le_bytes());
                d[12..20].copy_from_slice(&partition_size.to_le_bytes());
                d[20..36].copy_from_slice(&partition_guid);
                d[36] = PartitionFormat::Gpt as u8;
                d[37] = SignatureType::Guid as u8;
                d
            },
            data_len: 38,
        };
        self.path.add_node(node);
        self
    }

    /// Add file path node
    pub fn file_path(mut self, path: &str) -> Self {
        let mut data = [0u8; 252];
        let mut pos = 0;

        for c in path.chars() {
            if pos >= 250 {
                break;
            }
            // UCS-2 encoding (little-endian)
            data[pos] = c as u8;
            data[pos + 1] = 0;
            pos += 2;
        }
        // Null terminator
        data[pos] = 0;
        data[pos + 1] = 0;
        pos += 2;

        let node = DevicePathNode {
            header: DevicePathNodeHeader::new(
                DevicePathType::Media as u8,
                MediaSubtype::FilePath as u8,
                (4 + pos) as u16,
            ),
            data,
            data_len: pos,
        };
        self.path.add_node(node);
        self
    }

    /// Add URI node
    pub fn uri(mut self, uri: &str) -> Self {
        let uri_bytes = uri.as_bytes();
        let uri_len = uri_bytes.len().min(248);

        let mut data = [0u8; 252];
        data[..uri_len].copy_from_slice(&uri_bytes[..uri_len]);

        let node = DevicePathNode {
            header: DevicePathNodeHeader::new(
                DevicePathType::Messaging as u8,
                MessagingSubtype::Uri as u8,
                (4 + uri_len) as u16,
            ),
            data,
            data_len: uri_len,
        };
        self.path.add_node(node);
        self
    }

    /// Build device path
    pub fn build(self) -> DevicePath {
        self.path
    }
}

impl Default for DevicePathBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// DEVICE PATH TEXT CONVERSION
// =============================================================================

/// Device path to text converter
pub struct DevicePathToText;

impl DevicePathToText {
    /// Convert device path to text
    pub fn convert(path: &DevicePath, buffer: &mut [u8]) -> usize {
        let mut pos = 0;

        for (i, node) in path.nodes().iter().enumerate() {
            if i > 0 && pos < buffer.len() {
                buffer[pos] = b'/';
                pos += 1;
            }

            pos += Self::convert_node(node, &mut buffer[pos..]);
        }

        pos
    }

    /// Convert single node to text
    pub fn convert_node(node: &DevicePathNode, buffer: &mut [u8]) -> usize {
        let mut pos = 0;

        match (node.header.device_type, node.header.sub_type) {
            (0x01, 0x01) => {
                // PCI
                if node.data_len >= 2 {
                    pos += write_str(buffer, "Pci(");
                    pos += write_hex_u8(&mut buffer[pos..], node.data[1]);
                    if pos < buffer.len() { buffer[pos] = b','; pos += 1; }
                    pos += write_hex_u8(&mut buffer[pos..], node.data[0]);
                    if pos < buffer.len() { buffer[pos] = b')'; pos += 1; }
                }
            }
            (0x02, 0x01) => {
                // ACPI
                pos += write_str(buffer, "Acpi(...)");
            }
            (0x03, 0x05) => {
                // USB
                if node.data_len >= 2 {
                    pos += write_str(buffer, "Usb(");
                    pos += write_hex_u8(&mut buffer[pos..], node.data[0]);
                    if pos < buffer.len() { buffer[pos] = b','; pos += 1; }
                    pos += write_hex_u8(&mut buffer[pos..], node.data[1]);
                    if pos < buffer.len() { buffer[pos] = b')'; pos += 1; }
                }
            }
            (0x03, 0x12) => {
                // SATA
                pos += write_str(buffer, "Sata(...)");
            }
            (0x03, 0x17) => {
                // NVMe
                pos += write_str(buffer, "NVMe(...)");
            }
            (0x04, 0x01) => {
                // Hard Drive
                pos += write_str(buffer, "HD(...)");
            }
            (0x04, 0x04) => {
                // File Path
                pos += write_str(buffer, "File(");
                // Decode UCS-2
                let mut i = 0;
                while i + 1 < node.data_len && pos < buffer.len() - 1 {
                    let c = u16::from_le_bytes([node.data[i], node.data[i + 1]]);
                    if c == 0 { break; }
                    if c < 128 {
                        buffer[pos] = c as u8;
                        pos += 1;
                    }
                    i += 2;
                }
                if pos < buffer.len() { buffer[pos] = b')'; pos += 1; }
            }
            (0x03, 0x18) => {
                // URI
                pos += write_str(buffer, "Uri(");
                let uri_len = node.data_len.min(buffer.len() - pos - 1);
                buffer[pos..pos + uri_len].copy_from_slice(&node.data[..uri_len]);
                pos += uri_len;
                if pos < buffer.len() { buffer[pos] = b')'; pos += 1; }
            }
            _ => {
                pos += write_str(buffer, "Unknown");
            }
        }

        pos
    }
}

/// Write string to buffer
fn write_str(buffer: &mut [u8], s: &str) -> usize {
    let bytes = s.as_bytes();
    let len = bytes.len().min(buffer.len());
    buffer[..len].copy_from_slice(&bytes[..len]);
    len
}

/// Write hex u8
fn write_hex_u8(buffer: &mut [u8], v: u8) -> usize {
    if buffer.len() < 4 {
        return 0;
    }

    let hex = b"0123456789ABCDEF";
    buffer[0] = b'0';
    buffer[1] = b'x';
    buffer[2] = hex[(v >> 4) as usize];
    buffer[3] = hex[(v & 0xF) as usize];
    4
}

/// Text to device path parser
pub struct TextToDevicePath;

impl TextToDevicePath {
    /// Parse device path from text
    pub fn parse(text: &str) -> Option<DevicePath> {
        let mut path = DevicePath::new();

        for segment in text.split('/') {
            let segment = segment.trim();
            if segment.is_empty() {
                continue;
            }

            if let Some(node) = Self::parse_node(segment) {
                path.add_node(node);
            }
        }

        if path.is_empty() {
            None
        } else {
            Some(path)
        }
    }

    /// Parse single node
    fn parse_node(text: &str) -> Option<DevicePathNode> {
        // Extract function name and arguments
        let paren = text.find('(')?;
        let name = &text[..paren];
        let end = text.rfind(')')?;
        let _args = &text[paren + 1..end];

        match name {
            "Pci" => {
                // TODO: Parse PCI(device,function)
                Some(DevicePathNode::end_instance()) // Placeholder
            }
            "File" => {
                let path_text = &text[paren + 1..end];
                let fp = FilePathDevicePath::new(path_text);

                let mut node = DevicePathNode {
                    header: fp.header,
                    data: [0; 252],
                    data_len: 0,
                };

                // Encode path
                for (i, c) in path_text.chars().enumerate() {
                    if i * 2 + 1 >= 252 {
                        break;
                    }
                    node.data[i * 2] = c as u8;
                    node.data[i * 2 + 1] = 0;
                    node.data_len = (i + 1) * 2 + 2;
                }

                Some(node)
            }
            _ => None,
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_path_node() {
        let end = DevicePathNode::end_entire();
        assert!(end.is_end());
        assert_eq!(end.length(), 4);
    }

    #[test]
    fn test_pci_device_path() {
        let pci = PciDevicePath::new(0x1F, 0x03);
        assert_eq!(pci.device, 0x1F);
        assert_eq!(pci.function, 0x03);
    }

    #[test]
    fn test_device_path_builder() {
        let path = DevicePathBuilder::new()
            .pci(0x1F, 0x00)
            .sata(0, 0xFFFF, 0)
            .file_path("\\EFI\\BOOT\\BOOTX64.EFI")
            .build();

        assert_eq!(path.node_count(), 3);
    }

    #[test]
    fn test_acpi_eisa_id() {
        let id = AcpiDevicePath::eisa_id(b"PNP", 0x0A03);
        assert_eq!(id, 0x030AD041); // PNP0A03 = PCI host bridge
    }
}
