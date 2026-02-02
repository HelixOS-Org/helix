//! GUID Utilities
//!
//! GUID (Globally Unique Identifier) handling for UEFI protocols and services.

use core::fmt;

// =============================================================================
// GUID STRUCTURE
// =============================================================================

/// GUID (Globally Unique Identifier)
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Guid {
    /// Data 1 (time-low)
    pub data1: u32,
    /// Data 2 (time-mid)
    pub data2: u16,
    /// Data 3 (time-hi-and-version)
    pub data3: u16,
    /// Data 4 (clock-seq-hi-and-reserved, clock-seq-low, node)
    pub data4: [u8; 8],
}

impl Guid {
    /// Create a GUID from components
    pub const fn new(data1: u32, data2: u16, data3: u16, data4: [u8; 8]) -> Self {
        Self {
            data1,
            data2,
            data3,
            data4,
        }
    }

    /// Create from parts (more readable format)
    pub const fn from_parts(
        time_low: u32,
        time_mid: u16,
        time_hi_version: u16,
        clock_seq: u16,
        node: [u8; 6],
    ) -> Self {
        Self {
            data1: time_low,
            data2: time_mid,
            data3: time_hi_version,
            data4: [
                (clock_seq >> 8) as u8,
                clock_seq as u8,
                node[0],
                node[1],
                node[2],
                node[3],
                node[4],
                node[5],
            ],
        }
    }

    /// Create null GUID
    pub const fn null() -> Self {
        Self {
            data1: 0,
            data2: 0,
            data3: 0,
            data4: [0; 8],
        }
    }

    /// Check if null
    pub const fn is_null(&self) -> bool {
        self.data1 == 0
            && self.data2 == 0
            && self.data3 == 0
            && self.data4[0] == 0
            && self.data4[1] == 0
            && self.data4[2] == 0
            && self.data4[3] == 0
            && self.data4[4] == 0
            && self.data4[5] == 0
            && self.data4[6] == 0
            && self.data4[7] == 0
    }

    /// Convert to bytes (little-endian format)
    pub fn to_bytes_le(&self) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[0..4].copy_from_slice(&self.data1.to_le_bytes());
        bytes[4..6].copy_from_slice(&self.data2.to_le_bytes());
        bytes[6..8].copy_from_slice(&self.data3.to_le_bytes());
        bytes[8..16].copy_from_slice(&self.data4);
        bytes
    }

    /// Convert to bytes (mixed-endian, as stored in UEFI)
    pub fn to_bytes_mixed(&self) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[0..4].copy_from_slice(&self.data1.to_le_bytes());
        bytes[4..6].copy_from_slice(&self.data2.to_le_bytes());
        bytes[6..8].copy_from_slice(&self.data3.to_le_bytes());
        bytes[8..16].copy_from_slice(&self.data4);
        bytes
    }

    /// Create from bytes (little-endian)
    pub fn from_bytes_le(bytes: &[u8; 16]) -> Self {
        Self {
            data1: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            data2: u16::from_le_bytes([bytes[4], bytes[5]]),
            data3: u16::from_le_bytes([bytes[6], bytes[7]]),
            data4: [
                bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
                bytes[15],
            ],
        }
    }

    /// Create from bytes (big-endian, UUID format)
    pub fn from_bytes_be(bytes: &[u8; 16]) -> Self {
        Self {
            data1: u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            data2: u16::from_be_bytes([bytes[4], bytes[5]]),
            data3: u16::from_be_bytes([bytes[6], bytes[7]]),
            data4: [
                bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
                bytes[15],
            ],
        }
    }

    /// Get version (from data3)
    pub fn version(&self) -> u8 {
        ((self.data3 >> 12) & 0x0F) as u8
    }

    /// Get variant
    pub fn variant(&self) -> GuidVariant {
        let byte = self.data4[0];
        if (byte & 0x80) == 0 {
            GuidVariant::Ncs
        } else if (byte & 0xC0) == 0x80 {
            GuidVariant::Rfc4122
        } else if (byte & 0xE0) == 0xC0 {
            GuidVariant::Microsoft
        } else {
            GuidVariant::Reserved
        }
    }

    /// Compare GUIDs
    pub fn compare(&self, other: &Guid) -> core::cmp::Ordering {
        use core::cmp::Ordering;

        match self.data1.cmp(&other.data1) {
            Ordering::Equal => {},
            ord => return ord,
        }
        match self.data2.cmp(&other.data2) {
            Ordering::Equal => {},
            ord => return ord,
        }
        match self.data3.cmp(&other.data3) {
            Ordering::Equal => {},
            ord => return ord,
        }
        self.data4.cmp(&other.data4)
    }
}

impl fmt::Debug for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Guid({})", self)
    }
}

impl fmt::Display for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            self.data1,
            self.data2,
            self.data3,
            self.data4[0],
            self.data4[1],
            self.data4[2],
            self.data4[3],
            self.data4[4],
            self.data4[5],
            self.data4[6],
            self.data4[7]
        )
    }
}

impl fmt::UpperHex for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
            self.data1,
            self.data2,
            self.data3,
            self.data4[0],
            self.data4[1],
            self.data4[2],
            self.data4[3],
            self.data4[4],
            self.data4[5],
            self.data4[6],
            self.data4[7]
        )
    }
}

impl fmt::LowerHex for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Default for Guid {
    fn default() -> Self {
        Self::null()
    }
}

impl Ord for Guid {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.compare(other)
    }
}

impl PartialOrd for Guid {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// GUID variant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuidVariant {
    /// NCS (reserved for backward compatibility)
    Ncs,
    /// RFC 4122 (standard)
    Rfc4122,
    /// Microsoft (reserved)
    Microsoft,
    /// Reserved for future use
    Reserved,
}

// =============================================================================
// GUID PARSING
// =============================================================================

/// Parse GUID from string
pub fn parse_guid(s: &str) -> Option<Guid> {
    let s = s.trim();

    // Remove curly braces if present
    let s = if s.starts_with('{') && s.ends_with('}') {
        &s[1..s.len() - 1]
    } else {
        s
    };

    // Expected format: XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX
    if s.len() != 36 {
        return None;
    }

    // Verify dashes
    if s.as_bytes()[8] != b'-'
        || s.as_bytes()[13] != b'-'
        || s.as_bytes()[18] != b'-'
        || s.as_bytes()[23] != b'-'
    {
        return None;
    }

    // Parse components
    let data1 = parse_hex_u32(&s[0..8])?;
    let data2 = parse_hex_u16(&s[9..13])?;
    let data3 = parse_hex_u16(&s[14..18])?;
    let data4_0 = parse_hex_u8(&s[19..21])?;
    let data4_1 = parse_hex_u8(&s[21..23])?;
    let data4_2 = parse_hex_u8(&s[24..26])?;
    let data4_3 = parse_hex_u8(&s[26..28])?;
    let data4_4 = parse_hex_u8(&s[28..30])?;
    let data4_5 = parse_hex_u8(&s[30..32])?;
    let data4_6 = parse_hex_u8(&s[32..34])?;
    let data4_7 = parse_hex_u8(&s[34..36])?;

    Some(Guid {
        data1,
        data2,
        data3,
        data4: [
            data4_0, data4_1, data4_2, data4_3, data4_4, data4_5, data4_6, data4_7,
        ],
    })
}

fn parse_hex_u32(s: &str) -> Option<u32> {
    let mut result = 0u32;
    for c in s.chars() {
        let digit = match c {
            '0'..='9' => c as u32 - '0' as u32,
            'a'..='f' => c as u32 - 'a' as u32 + 10,
            'A'..='F' => c as u32 - 'A' as u32 + 10,
            _ => return None,
        };
        result = result.checked_mul(16)?.checked_add(digit)?;
    }
    Some(result)
}

fn parse_hex_u16(s: &str) -> Option<u16> {
    parse_hex_u32(s).and_then(|v| u16::try_from(v).ok())
}

fn parse_hex_u8(s: &str) -> Option<u8> {
    parse_hex_u32(s).and_then(|v| u8::try_from(v).ok())
}

// =============================================================================
// WELL-KNOWN GUIDS - EFI SYSTEM TABLE
// =============================================================================

/// EFI System Table signature
pub const EFI_SYSTEM_TABLE_SIGNATURE: u64 = 0x5453595320494249; // "IBI SYST"

// =============================================================================
// WELL-KNOWN GUIDS - CONFIGURATION TABLES
// =============================================================================

/// ACPI 1.0 RSDP GUID
pub const ACPI_10_TABLE_GUID: Guid = Guid::new(0xEB9D2D30, 0x2D88, 0x11D3, [
    0x9A, 0x16, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4D,
]);

/// ACPI 2.0+ RSDP GUID
pub const ACPI_20_TABLE_GUID: Guid = Guid::new(0x8868E871, 0xE4F1, 0x11D3, [
    0xBC, 0x22, 0x00, 0x80, 0xC7, 0x3C, 0x88, 0x81,
]);

/// SMBIOS Table GUID
pub const SMBIOS_TABLE_GUID: Guid = Guid::new(0xEB9D2D31, 0x2D88, 0x11D3, [
    0x9A, 0x16, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4D,
]);

/// SMBIOS 3.0 Table GUID
pub const SMBIOS3_TABLE_GUID: Guid = Guid::new(0xF2FD1544, 0x9794, 0x4A2C, [
    0x99, 0x2E, 0xE5, 0xBB, 0xCF, 0x20, 0xE3, 0x94,
]);

/// MPS Table GUID
pub const MPS_TABLE_GUID: Guid = Guid::new(0xEB9D2D2F, 0x2D88, 0x11D3, [
    0x9A, 0x16, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4D,
]);

/// SAL System Table GUID
pub const SAL_SYSTEM_TABLE_GUID: Guid = Guid::new(0xEB9D2D32, 0x2D88, 0x11D3, [
    0x9A, 0x16, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4D,
]);

/// Device Tree GUID
pub const DEVICE_TREE_GUID: Guid = Guid::new(0xB1B621D5, 0xF19C, 0x41A5, [
    0x83, 0x0B, 0xD9, 0x15, 0x2C, 0x69, 0xAA, 0xE0,
]);

// =============================================================================
// WELL-KNOWN GUIDS - PROTOCOLS
// =============================================================================

/// Loaded Image Protocol GUID
pub const LOADED_IMAGE_PROTOCOL_GUID: Guid = Guid::new(0x5B1B31A1, 0x9562, 0x11D2, [
    0x8E, 0x3F, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B,
]);

/// Device Path Protocol GUID
pub const DEVICE_PATH_PROTOCOL_GUID: Guid = Guid::new(0x09576E91, 0x6D3F, 0x11D2, [
    0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B,
]);

/// Simple File System Protocol GUID
pub const SIMPLE_FILE_SYSTEM_PROTOCOL_GUID: Guid = Guid::new(0x0964E5B22, 0x6459, 0x11D2, [
    0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B,
]);

/// File Info GUID
pub const FILE_INFO_GUID: Guid = Guid::new(0x09576E92, 0x6D3F, 0x11D2, [
    0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B,
]);

/// File System Info GUID
pub const FILE_SYSTEM_INFO_GUID: Guid = Guid::new(0x09576E93, 0x6D3F, 0x11D2, [
    0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B,
]);

/// Block I/O Protocol GUID
pub const BLOCK_IO_PROTOCOL_GUID: Guid = Guid::new(0x964E5B21, 0x6459, 0x11D2, [
    0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B,
]);

/// Disk I/O Protocol GUID
pub const DISK_IO_PROTOCOL_GUID: Guid = Guid::new(0xCE345171, 0xBA0B, 0x11D2, [
    0x8E, 0x4F, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B,
]);

/// Simple Text Input Protocol GUID
pub const SIMPLE_TEXT_INPUT_PROTOCOL_GUID: Guid = Guid::new(0x387477C1, 0x69C7, 0x11D2, [
    0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B,
]);

/// Simple Text Input Ex Protocol GUID
pub const SIMPLE_TEXT_INPUT_EX_PROTOCOL_GUID: Guid = Guid::new(0xDD9E7534, 0x7762, 0x4698, [
    0x8C, 0x14, 0xF5, 0x85, 0x17, 0xA6, 0x25, 0xAA,
]);

/// Simple Text Output Protocol GUID
pub const SIMPLE_TEXT_OUTPUT_PROTOCOL_GUID: Guid = Guid::new(0x387477C2, 0x69C7, 0x11D2, [
    0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B,
]);

/// Graphics Output Protocol GUID
pub const GRAPHICS_OUTPUT_PROTOCOL_GUID: Guid = Guid::new(0x9042A9DE, 0x23DC, 0x4A38, [
    0x96, 0xFB, 0x7A, 0xDE, 0xD0, 0x80, 0x51, 0x6A,
]);

/// EDID Active Protocol GUID
pub const EDID_ACTIVE_PROTOCOL_GUID: Guid = Guid::new(0xBD8C1056, 0x9F36, 0x44EC, [
    0x92, 0xA8, 0xA6, 0x33, 0x7F, 0x81, 0x79, 0x86,
]);

/// EDID Discovered Protocol GUID
pub const EDID_DISCOVERED_PROTOCOL_GUID: Guid = Guid::new(0x1C0C34F6, 0xD380, 0x41FA, [
    0xA0, 0x49, 0x8A, 0xD0, 0x6C, 0x1A, 0x66, 0xAA,
]);

/// Serial I/O Protocol GUID
pub const SERIAL_IO_PROTOCOL_GUID: Guid = Guid::new(0xBB25CF6F, 0xF1D4, 0x11D2, [
    0x9A, 0x0C, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0xFD,
]);

/// PCI I/O Protocol GUID
pub const PCI_IO_PROTOCOL_GUID: Guid = Guid::new(0x4CF5B200, 0x68B8, 0x4CA5, [
    0x9E, 0xEC, 0xB2, 0x3E, 0x3F, 0x50, 0x02, 0x9A,
]);

/// PCI Root Bridge I/O Protocol GUID
pub const PCI_ROOT_BRIDGE_IO_PROTOCOL_GUID: Guid = Guid::new(0x2F707EBB, 0x4A1A, 0x11D4, [
    0x9A, 0x38, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4D,
]);

/// USB I/O Protocol GUID
pub const USB_IO_PROTOCOL_GUID: Guid = Guid::new(0x2B2F68D6, 0x0CD2, 0x44CF, [
    0x8E, 0x8B, 0xBB, 0xA2, 0x0B, 0x1B, 0x5B, 0x75,
]);

/// Network Interface Identifier Protocol GUID
pub const NETWORK_INTERFACE_IDENTIFIER_PROTOCOL_GUID: Guid =
    Guid::new(0x1ACED566, 0x76ED, 0x4218, [
        0xBC, 0x81, 0x76, 0x7F, 0x1F, 0x97, 0x7A, 0x89,
    ]);

/// Simple Network Protocol GUID
pub const SIMPLE_NETWORK_PROTOCOL_GUID: Guid = Guid::new(0xA19832B9, 0xAC25, 0x11D3, [
    0x9A, 0x2D, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4D,
]);

/// Managed Network Protocol GUID
pub const MANAGED_NETWORK_PROTOCOL_GUID: Guid = Guid::new(0x7AB33A91, 0xACE5, 0x4326, [
    0xB5, 0x72, 0xE7, 0xEE, 0x33, 0xD3, 0x9F, 0x16,
]);

// =============================================================================
// WELL-KNOWN GUIDS - PARTITION TYPES
// =============================================================================

/// Unused partition entry
pub const PARTITION_TYPE_UNUSED: Guid = Guid::null();

/// EFI System Partition
pub const EFI_SYSTEM_PARTITION_GUID: Guid = Guid::new(0xC12A7328, 0xF81F, 0x11D2, [
    0xBA, 0x4B, 0x00, 0xA0, 0xC9, 0x3E, 0xC9, 0x3B,
]);

/// Microsoft Basic Data Partition
pub const MICROSOFT_BASIC_DATA_GUID: Guid = Guid::new(0xEBD0A0A2, 0xB9E5, 0x4433, [
    0x87, 0xC0, 0x68, 0xB6, 0xB7, 0x26, 0x99, 0xC7,
]);

/// Microsoft Reserved Partition
pub const MICROSOFT_RESERVED_GUID: Guid = Guid::new(0xE3C9E316, 0x0B5C, 0x4DB8, [
    0x81, 0x7D, 0xF9, 0x2D, 0xF0, 0x02, 0x15, 0xAE,
]);

/// Linux Filesystem Data
pub const LINUX_FILESYSTEM_DATA_GUID: Guid = Guid::new(0x0FC63DAF, 0x8483, 0x4772, [
    0x8E, 0x79, 0x3D, 0x69, 0xD8, 0x47, 0x7D, 0xE4,
]);

/// Linux Root x86-64
pub const LINUX_ROOT_X86_64_GUID: Guid = Guid::new(0x4F68BCE3, 0xE8CD, 0x4DB1, [
    0x96, 0xE7, 0xFB, 0xCA, 0xF9, 0x84, 0xB7, 0x09,
]);

/// Linux Home
pub const LINUX_HOME_GUID: Guid = Guid::new(0x933AC7E1, 0x2EB4, 0x4F13, [
    0xB8, 0x44, 0x0E, 0x14, 0xE2, 0xAE, 0xF9, 0x15,
]);

/// Linux Swap
pub const LINUX_SWAP_GUID: Guid = Guid::new(0x0657FD6D, 0xA4AB, 0x43C4, [
    0x84, 0xE5, 0x09, 0x33, 0xC8, 0x4B, 0x4F, 0x4F,
]);

// =============================================================================
// WELL-KNOWN GUIDS - SECURITY
// =============================================================================

/// EFI Global Variable GUID
pub const EFI_GLOBAL_VARIABLE_GUID: Guid = Guid::new(0x8BE4DF61, 0x93CA, 0x11D2, [
    0xAA, 0x0D, 0x00, 0xE0, 0x98, 0x03, 0x2B, 0x8C,
]);

/// EFI Image Security Database GUID
pub const EFI_IMAGE_SECURITY_DATABASE_GUID: Guid = Guid::new(0xD719B2CB, 0x3D3A, 0x4596, [
    0xA3, 0xBC, 0xDA, 0xD0, 0x0E, 0x67, 0x65, 0x6F,
]);

/// EFI Certificate X509 GUID
pub const EFI_CERT_X509_GUID: Guid = Guid::new(0xA5C059A1, 0x94E4, 0x4AA7, [
    0x87, 0xB5, 0xAB, 0x15, 0x5C, 0x2B, 0xF0, 0x72,
]);

/// EFI Certificate SHA256 GUID
pub const EFI_CERT_SHA256_GUID: Guid = Guid::new(0xC1C41626, 0x504C, 0x4092, [
    0xAC, 0xA9, 0x41, 0xF9, 0x36, 0x93, 0x43, 0x28,
]);

/// EFI Certificate RSA2048 GUID
pub const EFI_CERT_RSA2048_GUID: Guid = Guid::new(0x3C5766E8, 0x269C, 0x4E34, [
    0xAA, 0x14, 0xED, 0x77, 0x6E, 0x85, 0xB3, 0xB6,
]);

// =============================================================================
// GUID REGISTRY
// =============================================================================

/// GUID name entry
pub struct GuidEntry {
    pub guid: Guid,
    pub name: &'static str,
}

/// Look up GUID name
pub fn guid_name(guid: &Guid) -> Option<&'static str> {
    KNOWN_GUIDS.iter().find(|e| e.guid == *guid).map(|e| e.name)
}

/// Known GUIDs table
static KNOWN_GUIDS: &[GuidEntry] = &[
    GuidEntry {
        guid: ACPI_10_TABLE_GUID,
        name: "ACPI 1.0 Table",
    },
    GuidEntry {
        guid: ACPI_20_TABLE_GUID,
        name: "ACPI 2.0+ Table",
    },
    GuidEntry {
        guid: SMBIOS_TABLE_GUID,
        name: "SMBIOS Table",
    },
    GuidEntry {
        guid: SMBIOS3_TABLE_GUID,
        name: "SMBIOS 3.0 Table",
    },
    GuidEntry {
        guid: LOADED_IMAGE_PROTOCOL_GUID,
        name: "Loaded Image Protocol",
    },
    GuidEntry {
        guid: DEVICE_PATH_PROTOCOL_GUID,
        name: "Device Path Protocol",
    },
    GuidEntry {
        guid: SIMPLE_FILE_SYSTEM_PROTOCOL_GUID,
        name: "Simple File System Protocol",
    },
    GuidEntry {
        guid: BLOCK_IO_PROTOCOL_GUID,
        name: "Block I/O Protocol",
    },
    GuidEntry {
        guid: SIMPLE_TEXT_INPUT_PROTOCOL_GUID,
        name: "Simple Text Input Protocol",
    },
    GuidEntry {
        guid: SIMPLE_TEXT_OUTPUT_PROTOCOL_GUID,
        name: "Simple Text Output Protocol",
    },
    GuidEntry {
        guid: GRAPHICS_OUTPUT_PROTOCOL_GUID,
        name: "Graphics Output Protocol",
    },
    GuidEntry {
        guid: SERIAL_IO_PROTOCOL_GUID,
        name: "Serial I/O Protocol",
    },
    GuidEntry {
        guid: PCI_IO_PROTOCOL_GUID,
        name: "PCI I/O Protocol",
    },
    GuidEntry {
        guid: USB_IO_PROTOCOL_GUID,
        name: "USB I/O Protocol",
    },
    GuidEntry {
        guid: SIMPLE_NETWORK_PROTOCOL_GUID,
        name: "Simple Network Protocol",
    },
    GuidEntry {
        guid: EFI_SYSTEM_PARTITION_GUID,
        name: "EFI System Partition",
    },
    GuidEntry {
        guid: MICROSOFT_BASIC_DATA_GUID,
        name: "Microsoft Basic Data",
    },
    GuidEntry {
        guid: LINUX_FILESYSTEM_DATA_GUID,
        name: "Linux Filesystem",
    },
    GuidEntry {
        guid: EFI_GLOBAL_VARIABLE_GUID,
        name: "EFI Global Variable",
    },
];

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guid_creation() {
        let guid = Guid::new(0x12345678, 0xABCD, 0xEF01, [
            0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x01,
        ]);
        assert_eq!(guid.data1, 0x12345678);
        assert_eq!(guid.data2, 0xABCD);
        assert_eq!(guid.data3, 0xEF01);
    }

    #[test]
    fn test_guid_null() {
        let null = Guid::null();
        assert!(null.is_null());

        let not_null = Guid::new(1, 0, 0, [0; 8]);
        assert!(!not_null.is_null());
    }

    #[test]
    fn test_guid_display() {
        let guid = Guid::new(0x12345678, 0xABCD, 0xEF01, [
            0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x01,
        ]);
        let s = format!("{}", guid);
        assert_eq!(s, "12345678-abcd-ef01-2345-6789abcdef01");
    }

    #[test]
    fn test_guid_parse() {
        let s = "12345678-ABCD-EF01-2345-6789ABCDEF01";
        let guid = parse_guid(s).unwrap();
        assert_eq!(guid.data1, 0x12345678);
        assert_eq!(guid.data2, 0xABCD);
        assert_eq!(guid.data3, 0xEF01);
    }

    #[test]
    fn test_guid_parse_braces() {
        let s = "{12345678-ABCD-EF01-2345-6789ABCDEF01}";
        let guid = parse_guid(s).unwrap();
        assert_eq!(guid.data1, 0x12345678);
    }

    #[test]
    fn test_known_guids() {
        let name = guid_name(&ACPI_20_TABLE_GUID);
        assert_eq!(name, Some("ACPI 2.0+ Table"));
    }

    #[test]
    fn test_guid_version() {
        // Version 4 UUID
        let guid = Guid::new(0x12345678, 0x1234, 0x4234, [
            0x82, 0x34, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC,
        ]);
        assert_eq!(guid.version(), 4);
    }
}
