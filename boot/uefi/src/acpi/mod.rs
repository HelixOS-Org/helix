//! ACPI Table Parser
//!
//! Comprehensive ACPI table discovery and parsing for system configuration.

use core::fmt;

// =============================================================================
// RSDP (ROOT SYSTEM DESCRIPTION POINTER)
// =============================================================================

/// RSDP signature "RSD PTR "
pub const RSDP_SIGNATURE: [u8; 8] = *b"RSD PTR ";

/// ACPI 1.0 RSDP
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Rsdp {
    /// Signature "RSD PTR "
    pub signature: [u8; 8],
    /// Checksum (ACPI 1.0)
    pub checksum: u8,
    /// OEM ID
    pub oem_id: [u8; 6],
    /// Revision (0 = ACPI 1.0, 2 = ACPI 2.0+)
    pub revision: u8,
    /// RSDT address (32-bit)
    pub rsdt_address: u32,
}

impl Rsdp {
    /// Size
    pub const SIZE: usize = 20;

    /// Parse from bytes
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        let signature: [u8; 8] = bytes[0..8].try_into().ok()?;
        if signature != RSDP_SIGNATURE {
            return None;
        }

        Some(Self {
            signature,
            checksum: bytes[8],
            oem_id: bytes[9..15].try_into().ok()?,
            revision: bytes[15],
            rsdt_address: u32::from_le_bytes(bytes[16..20].try_into().ok()?),
        })
    }

    /// Validate checksum
    #[must_use]
    pub fn validate_checksum(&self, bytes: &[u8]) -> bool {
        if bytes.len() < Self::SIZE {
            return false;
        }

        let sum: u8 = bytes[..Self::SIZE]
            .iter()
            .fold(0u8, |a, &b| a.wrapping_add(b));
        sum == 0
    }

    /// Get OEM ID as string
    #[must_use]
    pub fn oem_id_str(&self) -> &str {
        core::str::from_utf8(&self.oem_id)
            .unwrap_or("")
            .trim_end_matches('\0')
    }

    /// Is ACPI 2.0+
    #[must_use]
    pub const fn is_extended(&self) -> bool {
        self.revision >= 2
    }
}

/// ACPI 2.0+ extended RSDP
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct RsdpExtended {
    /// Base RSDP
    pub base: Rsdp,
    /// Length of table
    pub length: u32,
    /// XSDT address (64-bit)
    pub xsdt_address: u64,
    /// Extended checksum
    pub extended_checksum: u8,
    /// Reserved
    pub reserved: [u8; 3],
}

impl RsdpExtended {
    /// Size
    pub const SIZE: usize = 36;

    /// Parse from bytes
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        let base = Rsdp::from_bytes(bytes)?;
        if !base.is_extended() {
            return None;
        }

        Some(Self {
            base,
            length: u32::from_le_bytes(bytes[20..24].try_into().ok()?),
            xsdt_address: u64::from_le_bytes(bytes[24..32].try_into().ok()?),
            extended_checksum: bytes[32],
            reserved: [bytes[33], bytes[34], bytes[35]],
        })
    }

    /// Validate extended checksum
    #[must_use]
    pub fn validate_extended_checksum(&self, bytes: &[u8]) -> bool {
        if bytes.len() < Self::SIZE {
            return false;
        }

        let sum: u8 = bytes[..Self::SIZE]
            .iter()
            .fold(0u8, |a, &b| a.wrapping_add(b));
        sum == 0
    }
}

// =============================================================================
// SYSTEM DESCRIPTION TABLE HEADER
// =============================================================================

/// SDT header (common to all ACPI tables)
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct SdtHeader {
    /// Signature
    pub signature: [u8; 4],
    /// Length
    pub length: u32,
    /// Revision
    pub revision: u8,
    /// Checksum
    pub checksum: u8,
    /// OEM ID
    pub oem_id: [u8; 6],
    /// OEM table ID
    pub oem_table_id: [u8; 8],
    /// OEM revision
    pub oem_revision: u32,
    /// Creator ID
    pub creator_id: u32,
    /// Creator revision
    pub creator_revision: u32,
}

impl SdtHeader {
    /// Size
    pub const SIZE: usize = 36;

    /// Parse from bytes
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        Some(Self {
            signature: bytes[0..4].try_into().ok()?,
            length: u32::from_le_bytes(bytes[4..8].try_into().ok()?),
            revision: bytes[8],
            checksum: bytes[9],
            oem_id: bytes[10..16].try_into().ok()?,
            oem_table_id: bytes[16..24].try_into().ok()?,
            oem_revision: u32::from_le_bytes(bytes[24..28].try_into().ok()?),
            creator_id: u32::from_le_bytes(bytes[28..32].try_into().ok()?),
            creator_revision: u32::from_le_bytes(bytes[32..36].try_into().ok()?),
        })
    }

    /// Get signature as string
    #[must_use]
    pub fn signature_str(&self) -> &str {
        core::str::from_utf8(&self.signature).unwrap_or("")
    }

    /// Get OEM ID as string
    #[must_use]
    pub fn oem_id_str(&self) -> &str {
        core::str::from_utf8(&self.oem_id)
            .unwrap_or("")
            .trim_end_matches('\0')
    }

    /// Get OEM table ID as string
    #[must_use]
    pub fn oem_table_id_str(&self) -> &str {
        core::str::from_utf8(&self.oem_table_id)
            .unwrap_or("")
            .trim_end_matches('\0')
    }

    /// Validate checksum
    #[must_use]
    pub fn validate_checksum(&self, bytes: &[u8]) -> bool {
        let length = self.length as usize;
        if bytes.len() < length {
            return false;
        }

        let sum: u8 = bytes[..length].iter().fold(0u8, |a, &b| a.wrapping_add(b));
        sum == 0
    }
}

impl fmt::Debug for SdtHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let length = self.length;
        let revision = self.revision;
        let checksum = self.checksum;
        let oem_revision = self.oem_revision;
        let creator_id = self.creator_id;
        let creator_revision = self.creator_revision;
        f.debug_struct("SdtHeader")
            .field("signature", &self.signature_str())
            .field("length", &length)
            .field("revision", &revision)
            .field("checksum", &checksum)
            .field("oem_id", &self.oem_id_str())
            .field("oem_table_id", &self.oem_table_id_str())
            .field("oem_revision", &oem_revision)
            .field("creator_id", &creator_id)
            .field("creator_revision", &creator_revision)
            .finish()
    }
}

// =============================================================================
// TABLE SIGNATURES
// =============================================================================

/// Known ACPI table signatures
pub mod signature {
    /// Root System Description Table
    pub const RSDT: [u8; 4] = *b"RSDT";
    /// Extended System Description Table
    pub const XSDT: [u8; 4] = *b"XSDT";
    /// Fixed ACPI Description Table
    pub const FADT: [u8; 4] = *b"FACP";
    /// Firmware ACPI Control Structure
    pub const FACS: [u8; 4] = *b"FACS";
    /// Differentiated System Description Table
    pub const DSDT: [u8; 4] = *b"DSDT";
    /// Secondary System Description Table
    pub const SSDT: [u8; 4] = *b"SSDT";
    /// Multiple APIC Description Table
    pub const MADT: [u8; 4] = *b"APIC";
    /// Smart Battery Specification Table
    pub const SBST: [u8; 4] = *b"SBST";
    /// Extended System Configuration Data Table
    pub const ECDT: [u8; 4] = *b"ECDT";
    /// System Resource Affinity Table
    pub const SRAT: [u8; 4] = *b"SRAT";
    /// System Locality Distance Information Table
    pub const SLIT: [u8; 4] = *b"SLIT";
    /// High Precision Event Timer Table
    pub const HPET: [u8; 4] = *b"HPET";
    /// Boot Graphics Resource Table
    pub const BGRT: [u8; 4] = *b"BGRT";
    /// Memory Configuration Table
    pub const MCFG: [u8; 4] = *b"MCFG";
    /// Windows ACPI Emulated Devices Table
    pub const WAET: [u8; 4] = *b"WAET";
    /// Debug Port Table
    pub const DBGP: [u8; 4] = *b"DBGP";
    /// Debug Port Table 2
    pub const DBG2: [u8; 4] = *b"DBG2";
    /// DMA Remapping Table
    pub const DMAR: [u8; 4] = *b"DMAR";
    /// Trusted Computing Platform Alliance Table
    pub const TCPA: [u8; 4] = *b"TCPA";
    /// TPM 2.0 Table
    pub const TPM2: [u8; 4] = *b"TPM2";
    /// Core System Resource Table
    pub const CSRT: [u8; 4] = *b"CSRT";
    /// Low Power Idle Table
    pub const LPIT: [u8; 4] = *b"LPIT";
    /// Platform Runtime Mechanism Table
    pub const PRMT: [u8; 4] = *b"PRMT";
}

// =============================================================================
// RSDT / XSDT
// =============================================================================

/// RSDT (32-bit pointers)
pub struct Rsdt<'a> {
    header: SdtHeader,
    data: &'a [u8],
}

impl<'a> Rsdt<'a> {
    /// Parse RSDT
    #[must_use]
    pub fn parse(data: &'a [u8]) -> Option<Self> {
        let header = SdtHeader::from_bytes(data)?;

        if header.signature != signature::RSDT {
            return None;
        }

        Some(Self { header, data })
    }

    /// Get header
    pub fn header(&self) -> &SdtHeader {
        &self.header
    }

    /// Get entry count
    pub fn entry_count(&self) -> usize {
        let entries_size = self.header.length as usize - SdtHeader::SIZE;
        entries_size / 4
    }

    /// Get entry (table address)
    #[must_use]
    pub fn entry(&self, index: usize) -> Option<u32> {
        if index >= self.entry_count() {
            return None;
        }

        let offset = SdtHeader::SIZE + index * 4;
        if offset + 4 > self.data.len() {
            return None;
        }

        Some(u32::from_le_bytes(
            self.data[offset..offset + 4].try_into().ok()?,
        ))
    }

    /// Iterate entries
    pub fn entries(&self) -> impl Iterator<Item = u32> + 'a {
        let count = self.entry_count();
        let data = self.data;

        (0..count).filter_map(move |i| {
            let offset = SdtHeader::SIZE + i * 4;
            if offset + 4 > data.len() {
                return None;
            }

            Some(u32::from_le_bytes(
                data[offset..offset + 4].try_into().ok()?,
            ))
        })
    }
}

/// XSDT (64-bit pointers)
pub struct Xsdt<'a> {
    header: SdtHeader,
    data: &'a [u8],
}

impl<'a> Xsdt<'a> {
    /// Parse XSDT
    #[must_use]
    pub fn parse(data: &'a [u8]) -> Option<Self> {
        let header = SdtHeader::from_bytes(data)?;

        if header.signature != signature::XSDT {
            return None;
        }

        Some(Self { header, data })
    }

    /// Get header
    #[must_use]
    pub const fn header(&self) -> &SdtHeader {
        &self.header
    }

    /// Get entry count
    #[must_use]
    pub fn entry_count(&self) -> usize {
        let entries_size = self.header.length as usize - SdtHeader::SIZE;
        entries_size / 8
    }

    /// Get entry (table address)
    #[must_use]
    pub fn entry(&self, index: usize) -> Option<u64> {
        if index >= self.entry_count() {
            return None;
        }

        let offset = SdtHeader::SIZE + index * 8;
        if offset + 8 > self.data.len() {
            return None;
        }

        Some(u64::from_le_bytes(
            self.data[offset..offset + 8].try_into().ok()?,
        ))
    }

    /// Iterate entries
    pub fn entries(&self) -> impl Iterator<Item = u64> + 'a {
        let count = self.entry_count();
        let data = self.data;

        (0..count).filter_map(move |i| {
            let offset = SdtHeader::SIZE + i * 8;
            if offset + 8 > data.len() {
                return None;
            }

            Some(u64::from_le_bytes(
                data[offset..offset + 8].try_into().ok()?,
            ))
        })
    }
}

// =============================================================================
// MADT (MULTIPLE APIC DESCRIPTION TABLE)
// =============================================================================

/// MADT header
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct MadtHeader {
    /// SDT header
    pub header: SdtHeader,
    /// Local APIC address
    pub local_apic_address: u32,
    /// Flags
    pub flags: u32,
}

impl MadtHeader {
    /// Size
    pub const SIZE: usize = SdtHeader::SIZE + 8;

    /// Parse from bytes
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        let header = SdtHeader::from_bytes(bytes)?;
        if header.signature != signature::MADT {
            return None;
        }

        Some(Self {
            header,
            local_apic_address: u32::from_le_bytes(
                bytes[SdtHeader::SIZE..SdtHeader::SIZE + 4]
                    .try_into()
                    .ok()?,
            ),
            flags: u32::from_le_bytes(
                bytes[SdtHeader::SIZE + 4..SdtHeader::SIZE + 8]
                    .try_into()
                    .ok()?,
            ),
        })
    }

    /// Has legacy PICs
    #[must_use]
    pub const fn has_legacy_pics(&self) -> bool {
        self.flags & 0x01 != 0
    }
}

/// MADT entry types
pub mod madt_entry_type {
    /// Processor Local APIC
    pub const LOCAL_APIC: u8 = 0;
    /// I/O APIC
    pub const IO_APIC: u8 = 1;
    /// Interrupt Source Override
    pub const INTERRUPT_SOURCE_OVERRIDE: u8 = 2;
    /// NMI Source
    pub const NMI_SOURCE: u8 = 3;
    /// Local APIC NMI
    pub const LOCAL_APIC_NMI: u8 = 4;
    /// Local APIC Address Override
    pub const LOCAL_APIC_ADDRESS_OVERRIDE: u8 = 5;
    /// I/O SAPIC
    pub const IO_SAPIC: u8 = 6;
    /// Local SAPIC
    pub const LOCAL_SAPIC: u8 = 7;
    /// Platform Interrupt Sources
    pub const PLATFORM_INTERRUPT_SOURCES: u8 = 8;
    /// Processor Local x2APIC
    pub const LOCAL_X2APIC: u8 = 9;
    /// Local x2APIC NMI
    pub const LOCAL_X2APIC_NMI: u8 = 10;
    /// GIC CPU Interface
    pub const GIC_CPU: u8 = 11;
    /// GIC Distributor
    pub const GIC_DISTRIBUTOR: u8 = 12;
    /// GIC MSI Frame
    pub const GIC_MSI_FRAME: u8 = 13;
    /// GIC Redistributor
    pub const GIC_REDISTRIBUTOR: u8 = 14;
    /// GIC Interrupt Translation Service
    pub const GIC_ITS: u8 = 15;
}

/// MADT entry header
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct MadtEntryHeader {
    /// Entry type
    pub entry_type: u8,
    /// Entry length
    pub length: u8,
}

impl MadtEntryHeader {
    /// Parse from bytes
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 2 {
            return None;
        }

        Some(Self {
            entry_type: bytes[0],
            length: bytes[1],
        })
    }
}

/// Local APIC entry
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct MadtLocalApic {
    /// Header
    pub header: MadtEntryHeader,
    /// ACPI processor ID
    pub acpi_processor_id: u8,
    /// APIC ID
    pub apic_id: u8,
    /// Flags
    pub flags: u32,
}

impl MadtLocalApic {
    /// Parse from bytes
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 8 {
            return None;
        }

        let header = MadtEntryHeader::from_bytes(bytes)?;
        if header.entry_type != madt_entry_type::LOCAL_APIC {
            return None;
        }

        Some(Self {
            header,
            acpi_processor_id: bytes[2],
            apic_id: bytes[3],
            flags: u32::from_le_bytes(bytes[4..8].try_into().ok()?),
        })
    }

    /// Is enabled
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.flags & 0x01 != 0
    }

    /// Is online capable
    #[must_use]
    pub const fn is_online_capable(&self) -> bool {
        self.flags & 0x02 != 0
    }
}

/// I/O APIC entry
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct MadtIoApic {
    /// Header
    pub header: MadtEntryHeader,
    /// I/O APIC ID
    pub io_apic_id: u8,
    /// Reserved
    pub reserved: u8,
    /// I/O APIC address
    pub io_apic_address: u32,
    /// Global system interrupt base
    pub global_system_interrupt_base: u32,
}

impl MadtIoApic {
    /// Parse from bytes
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 12 {
            return None;
        }

        let header = MadtEntryHeader::from_bytes(bytes)?;
        if header.entry_type != madt_entry_type::IO_APIC {
            return None;
        }

        Some(Self {
            header,
            io_apic_id: bytes[2],
            reserved: bytes[3],
            io_apic_address: u32::from_le_bytes(bytes[4..8].try_into().ok()?),
            global_system_interrupt_base: u32::from_le_bytes(bytes[8..12].try_into().ok()?),
        })
    }
}

/// Interrupt source override
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct MadtInterruptOverride {
    /// Header
    pub header: MadtEntryHeader,
    /// Bus (always 0 = ISA)
    pub bus: u8,
    /// Source IRQ
    pub source: u8,
    /// Global system interrupt
    pub global_system_interrupt: u32,
    /// Flags
    pub flags: u16,
}

impl MadtInterruptOverride {
    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 10 {
            return None;
        }

        let header = MadtEntryHeader::from_bytes(bytes)?;
        if header.entry_type != madt_entry_type::INTERRUPT_SOURCE_OVERRIDE {
            return None;
        }

        Some(Self {
            header,
            bus: bytes[2],
            source: bytes[3],
            global_system_interrupt: u32::from_le_bytes(bytes[4..8].try_into().ok()?),
            flags: u16::from_le_bytes([bytes[8], bytes[9]]),
        })
    }

    /// Get polarity
    pub fn polarity(&self) -> Polarity {
        match self.flags & 0x03 {
            0 => Polarity::ConformToSpec,
            1 => Polarity::ActiveHigh,
            3 => Polarity::ActiveLow,
            _ => Polarity::Reserved,
        }
    }

    /// Get trigger mode
    pub fn trigger_mode(&self) -> TriggerMode {
        match (self.flags >> 2) & 0x03 {
            0 => TriggerMode::ConformToSpec,
            1 => TriggerMode::EdgeTriggered,
            3 => TriggerMode::LevelTriggered,
            _ => TriggerMode::Reserved,
        }
    }
}

/// Polarity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
    /// Conforms to the specifications of the bus
    ConformToSpec,
    /// Active high
    ActiveHigh,
    /// Active low
    ActiveLow,
    /// Reserved value
    Reserved,
}

/// Trigger mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerMode {
    /// Conforms to the specifications of the bus
    ConformToSpec,
    /// Edge-triggered
    EdgeTriggered,
    /// Level-triggered
    LevelTriggered,
    /// Reserved value
    Reserved,
}

/// Local APIC NMI
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct MadtLocalApicNmi {
    /// Header
    pub header: MadtEntryHeader,
    /// ACPI processor ID (0xFF = all)
    pub acpi_processor_id: u8,
    /// Flags
    pub flags: u16,
    /// Local APIC LINT#
    pub lint: u8,
}

impl MadtLocalApicNmi {
    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 6 {
            return None;
        }

        let header = MadtEntryHeader::from_bytes(bytes)?;
        if header.entry_type != madt_entry_type::LOCAL_APIC_NMI {
            return None;
        }

        Some(Self {
            header,
            acpi_processor_id: bytes[2],
            flags: u16::from_le_bytes([bytes[3], bytes[4]]),
            lint: bytes[5],
        })
    }

    /// Applies to all processors
    pub fn applies_to_all(&self) -> bool {
        self.acpi_processor_id == 0xFF
    }
}

/// Local x2APIC entry
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct MadtLocalX2Apic {
    /// Header
    pub header: MadtEntryHeader,
    /// Reserved
    pub reserved: [u8; 2],
    /// x2APIC ID
    pub x2apic_id: u32,
    /// Flags
    pub flags: u32,
    /// ACPI processor UID
    pub acpi_processor_uid: u32,
}

impl MadtLocalX2Apic {
    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 16 {
            return None;
        }

        let header = MadtEntryHeader::from_bytes(bytes)?;
        if header.entry_type != madt_entry_type::LOCAL_X2APIC {
            return None;
        }

        Some(Self {
            header,
            reserved: [bytes[2], bytes[3]],
            x2apic_id: u32::from_le_bytes(bytes[4..8].try_into().ok()?),
            flags: u32::from_le_bytes(bytes[8..12].try_into().ok()?),
            acpi_processor_uid: u32::from_le_bytes(bytes[12..16].try_into().ok()?),
        })
    }

    /// Is enabled
    pub fn is_enabled(&self) -> bool {
        self.flags & 0x01 != 0
    }
}

/// MADT parser
pub struct Madt<'a> {
    header: MadtHeader,
    data: &'a [u8],
}

impl<'a> Madt<'a> {
    /// Parse MADT
    pub fn parse(data: &'a [u8]) -> Option<Self> {
        let header = MadtHeader::from_bytes(data)?;
        Some(Self { header, data })
    }

    /// Get header
    pub fn header(&self) -> &MadtHeader {
        &self.header
    }

    /// Get local APIC address
    pub fn local_apic_address(&self) -> u32 {
        self.header.local_apic_address
    }

    /// Iterate entries
    pub fn entries(&self) -> MadtEntryIter<'a> {
        MadtEntryIter {
            data: self.data,
            offset: MadtHeader::SIZE,
            end: self.header.header.length as usize,
        }
    }

    /// Get all local APICs
    pub fn local_apics(&self) -> impl Iterator<Item = MadtLocalApic> + 'a {
        self.entries().filter_map(|e| {
            if e.header.entry_type == madt_entry_type::LOCAL_APIC {
                MadtLocalApic::from_bytes(e.data)
            } else {
                None
            }
        })
    }

    /// Get all I/O APICs
    pub fn io_apics(&self) -> impl Iterator<Item = MadtIoApic> + 'a {
        self.entries().filter_map(|e| {
            if e.header.entry_type == madt_entry_type::IO_APIC {
                MadtIoApic::from_bytes(e.data)
            } else {
                None
            }
        })
    }

    /// Get interrupt overrides
    pub fn interrupt_overrides(&self) -> impl Iterator<Item = MadtInterruptOverride> + 'a {
        self.entries().filter_map(|e| {
            if e.header.entry_type == madt_entry_type::INTERRUPT_SOURCE_OVERRIDE {
                MadtInterruptOverride::from_bytes(e.data)
            } else {
                None
            }
        })
    }

    /// Count enabled processors
    pub fn processor_count(&self) -> usize {
        let local_apic_count = self.local_apics().filter(MadtLocalApic::is_enabled).count();

        let x2apic_count = self
            .entries()
            .filter_map(|e| {
                if e.header.entry_type == madt_entry_type::LOCAL_X2APIC {
                    MadtLocalX2Apic::from_bytes(e.data)
                } else {
                    None
                }
            })
            .filter(MadtLocalX2Apic::is_enabled)
            .count();

        local_apic_count + x2apic_count
    }
}

/// MADT entry with raw data
pub struct MadtEntry<'a> {
    /// Entry header
    pub header: MadtEntryHeader,
    /// Raw entry data
    pub data: &'a [u8],
}

/// MADT entry iterator
pub struct MadtEntryIter<'a> {
    data: &'a [u8],
    offset: usize,
    end: usize,
}

impl<'a> Iterator for MadtEntryIter<'a> {
    type Item = MadtEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.end {
            return None;
        }

        let header = MadtEntryHeader::from_bytes(&self.data[self.offset..])?;
        let entry_end = self.offset + header.length as usize;

        if entry_end > self.end {
            return None;
        }

        let entry = MadtEntry {
            header,
            data: &self.data[self.offset..entry_end],
        };

        self.offset = entry_end;
        Some(entry)
    }
}

// =============================================================================
// FADT (FIXED ACPI DESCRIPTION TABLE)
// =============================================================================

/// FADT (partial - key fields only)
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Fadt {
    /// SDT header
    pub header: SdtHeader,
    /// FACS address (32-bit)
    pub firmware_ctrl: u32,
    /// DSDT address (32-bit)
    pub dsdt: u32,
    /// Reserved (was `INT_MODEL`)
    pub reserved1: u8,
    /// Preferred PM profile
    pub preferred_pm_profile: u8,
    /// SCI interrupt
    pub sci_int: u16,
    /// SMI command port
    pub smi_cmd: u32,
    /// ACPI enable value
    pub acpi_enable: u8,
    /// ACPI disable value
    pub acpi_disable: u8,
    /// S4BIOS request
    pub s4bios_req: u8,
    /// PSTATE control
    pub pstate_cnt: u8,
    /// `PM1a` event block
    pub pm1a_evt_blk: u32,
    /// `PM1b` event block
    pub pm1b_evt_blk: u32,
    /// `PM1a` control block
    pub pm1a_cnt_blk: u32,
    /// `PM1b` control block
    pub pm1b_cnt_blk: u32,
    /// PM2 control block
    pub pm2_cnt_blk: u32,
    /// PM timer block
    pub pm_tmr_blk: u32,
    /// GPE0 block
    pub gpe0_blk: u32,
    /// GPE1 block
    pub gpe1_blk: u32,
    /// PM1 event length
    pub pm1_evt_len: u8,
    /// PM1 control length
    pub pm1_cnt_len: u8,
    /// PM2 control length
    pub pm2_cnt_len: u8,
    /// PM timer length
    pub pm_tmr_len: u8,
    /// GPE0 block length
    pub gpe0_blk_len: u8,
    /// GPE1 block length
    pub gpe1_blk_len: u8,
    /// GPE1 base
    pub gpe1_base: u8,
    /// C-state control
    pub cst_cnt: u8,
    /// `P_LVL2` latency
    pub p_lvl2_lat: u16,
    /// `P_LVL3` latency
    pub p_lvl3_lat: u16,
    /// Flush size
    pub flush_size: u16,
    /// Flush stride
    pub flush_stride: u16,
    /// Duty offset
    pub duty_offset: u8,
    /// Duty width
    pub duty_width: u8,
    /// Day alarm
    pub day_alrm: u8,
    /// Month alarm
    pub mon_alrm: u8,
    /// Century
    pub century: u8,
    /// Boot architecture flags (ACPI 2.0+)
    pub iapc_boot_arch: u16,
    /// Reserved
    pub reserved2: u8,
    /// Flags
    pub flags: u32,
}

impl Fadt {
    /// Minimum size (ACPI 1.0)
    pub const MIN_SIZE: usize = 116;

    /// Parse from bytes (partial)
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::MIN_SIZE {
            return None;
        }

        let header = SdtHeader::from_bytes(bytes)?;
        if header.signature != signature::FADT {
            return None;
        }

        Some(Self {
            header,
            firmware_ctrl: u32::from_le_bytes(bytes[36..40].try_into().ok()?),
            dsdt: u32::from_le_bytes(bytes[40..44].try_into().ok()?),
            reserved1: bytes[44],
            preferred_pm_profile: bytes[45],
            sci_int: u16::from_le_bytes([bytes[46], bytes[47]]),
            smi_cmd: u32::from_le_bytes(bytes[48..52].try_into().ok()?),
            acpi_enable: bytes[52],
            acpi_disable: bytes[53],
            s4bios_req: bytes[54],
            pstate_cnt: bytes[55],
            pm1a_evt_blk: u32::from_le_bytes(bytes[56..60].try_into().ok()?),
            pm1b_evt_blk: u32::from_le_bytes(bytes[60..64].try_into().ok()?),
            pm1a_cnt_blk: u32::from_le_bytes(bytes[64..68].try_into().ok()?),
            pm1b_cnt_blk: u32::from_le_bytes(bytes[68..72].try_into().ok()?),
            pm2_cnt_blk: u32::from_le_bytes(bytes[72..76].try_into().ok()?),
            pm_tmr_blk: u32::from_le_bytes(bytes[76..80].try_into().ok()?),
            gpe0_blk: u32::from_le_bytes(bytes[80..84].try_into().ok()?),
            gpe1_blk: u32::from_le_bytes(bytes[84..88].try_into().ok()?),
            pm1_evt_len: bytes[88],
            pm1_cnt_len: bytes[89],
            pm2_cnt_len: bytes[90],
            pm_tmr_len: bytes[91],
            gpe0_blk_len: bytes[92],
            gpe1_blk_len: bytes[93],
            gpe1_base: bytes[94],
            cst_cnt: bytes[95],
            p_lvl2_lat: u16::from_le_bytes([bytes[96], bytes[97]]),
            p_lvl3_lat: u16::from_le_bytes([bytes[98], bytes[99]]),
            flush_size: u16::from_le_bytes([bytes[100], bytes[101]]),
            flush_stride: u16::from_le_bytes([bytes[102], bytes[103]]),
            duty_offset: bytes[104],
            duty_width: bytes[105],
            day_alrm: bytes[106],
            mon_alrm: bytes[107],
            century: bytes[108],
            iapc_boot_arch: u16::from_le_bytes([bytes[109], bytes[110]]),
            reserved2: bytes[111],
            flags: u32::from_le_bytes(bytes[112..116].try_into().ok()?),
        })
    }

    /// Has 8042 keyboard controller
    pub fn has_8042(&self) -> bool {
        self.iapc_boot_arch & 0x02 != 0
    }

    /// Has VGA not present
    pub fn vga_not_present(&self) -> bool {
        self.iapc_boot_arch & 0x04 != 0
    }

    /// Has MSI not supported
    pub fn msi_not_supported(&self) -> bool {
        self.iapc_boot_arch & 0x08 != 0
    }

    /// Has ASPM controls
    pub fn pcie_aspm_controls(&self) -> bool {
        self.iapc_boot_arch & 0x10 != 0
    }

    /// Has CMOS RTC not present
    pub fn cmos_rtc_not_present(&self) -> bool {
        self.iapc_boot_arch & 0x20 != 0
    }
}

/// Preferred PM profile
pub mod pm_profile {
    /// Unspecified profile
    pub const UNSPECIFIED: u8 = 0;
    /// Desktop profile
    pub const DESKTOP: u8 = 1;
    /// Mobile profile
    pub const MOBILE: u8 = 2;
    /// Workstation profile
    pub const WORKSTATION: u8 = 3;
    /// Enterprise server profile
    pub const ENTERPRISE_SERVER: u8 = 4;
    /// SOHO server profile
    pub const SOHO_SERVER: u8 = 5;
    /// Appliance PC profile
    pub const APPLIANCE_PC: u8 = 6;
    /// Performance server profile
    pub const PERFORMANCE_SERVER: u8 = 7;
    /// Tablet profile
    pub const TABLET: u8 = 8;
}

// =============================================================================
// HPET (HIGH PRECISION EVENT TIMER)
// =============================================================================

/// HPET table
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Hpet {
    /// SDT header
    pub header: SdtHeader,
    /// Event timer block ID
    pub event_timer_block_id: u32,
    /// Base address
    pub base_address: GenericAddress,
    /// HPET number
    pub hpet_number: u8,
    /// Minimum clock tick
    pub min_clock_tick: u16,
    /// Page protection
    pub page_protection: u8,
}

impl Hpet {
    /// Size
    pub const SIZE: usize = SdtHeader::SIZE + 20;

    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        let header = SdtHeader::from_bytes(bytes)?;
        if header.signature != signature::HPET {
            return None;
        }

        Some(Self {
            header,
            event_timer_block_id: u32::from_le_bytes(
                bytes[SdtHeader::SIZE..SdtHeader::SIZE + 4]
                    .try_into()
                    .ok()?,
            ),
            base_address: GenericAddress::from_bytes(
                &bytes[SdtHeader::SIZE + 4..SdtHeader::SIZE + 16],
            )?,
            hpet_number: bytes[SdtHeader::SIZE + 16],
            min_clock_tick: u16::from_le_bytes([
                bytes[SdtHeader::SIZE + 17],
                bytes[SdtHeader::SIZE + 18],
            ]),
            page_protection: bytes[SdtHeader::SIZE + 19],
        })
    }

    /// Get timer count
    pub fn timer_count(&self) -> u8 {
        ((self.event_timer_block_id >> 8) & 0x1F) as u8 + 1
    }

    /// Is 64-bit capable
    pub fn is_64bit(&self) -> bool {
        self.event_timer_block_id & (1 << 13) != 0
    }

    /// Supports legacy replacement
    pub fn legacy_replacement_capable(&self) -> bool {
        self.event_timer_block_id & (1 << 15) != 0
    }
}

// =============================================================================
// MCFG (MEMORY MAPPED CONFIGURATION)
// =============================================================================

/// MCFG table
pub struct Mcfg<'a> {
    header: SdtHeader,
    data: &'a [u8],
}

impl<'a> Mcfg<'a> {
    /// Parse MCFG
    pub fn parse(data: &'a [u8]) -> Option<Self> {
        let header = SdtHeader::from_bytes(data)?;
        if header.signature != signature::MCFG {
            return None;
        }

        Some(Self { header, data })
    }

    /// Get header
    pub fn header(&self) -> &SdtHeader {
        &self.header
    }

    /// Get entry count
    pub fn entry_count(&self) -> usize {
        let entries_size = self.header.length as usize - SdtHeader::SIZE - 8;
        entries_size / McfgEntry::SIZE
    }

    /// Get entry
    pub fn entry(&self, index: usize) -> Option<McfgEntry> {
        if index >= self.entry_count() {
            return None;
        }

        let offset = SdtHeader::SIZE + 8 + index * McfgEntry::SIZE;
        McfgEntry::from_bytes(&self.data[offset..])
    }

    /// Iterate entries
    pub fn entries(&self) -> impl Iterator<Item = McfgEntry> + 'a {
        let count = self.entry_count();
        let data = self.data;

        (0..count).filter_map(move |i| {
            let offset = SdtHeader::SIZE + 8 + i * McfgEntry::SIZE;
            McfgEntry::from_bytes(&data[offset..])
        })
    }
}

/// MCFG entry (PCI segment)
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct McfgEntry {
    /// Base address
    pub base_address: u64,
    /// Segment group number
    pub segment_group: u16,
    /// Start bus number
    pub start_bus: u8,
    /// End bus number
    pub end_bus: u8,
    /// Reserved
    pub reserved: u32,
}

impl McfgEntry {
    /// Size
    pub const SIZE: usize = 16;

    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        Some(Self {
            base_address: u64::from_le_bytes(bytes[0..8].try_into().ok()?),
            segment_group: u16::from_le_bytes([bytes[8], bytes[9]]),
            start_bus: bytes[10],
            end_bus: bytes[11],
            reserved: u32::from_le_bytes(bytes[12..16].try_into().ok()?),
        })
    }

    /// Calculate address for device
    pub fn device_address(&self, bus: u8, device: u8, function: u8) -> Option<u64> {
        if bus < self.start_bus || bus > self.end_bus {
            return None;
        }
        if device > 31 || function > 7 {
            return None;
        }

        let offset = ((u64::from(bus) - u64::from(self.start_bus)) << 20)
            | (u64::from(device) << 15)
            | (u64::from(function) << 12);

        Some(self.base_address + offset)
    }
}

// =============================================================================
// GENERIC ADDRESS STRUCTURE
// =============================================================================

/// Generic address structure
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct GenericAddress {
    /// Address space ID
    pub address_space_id: u8,
    /// Register bit width
    pub register_bit_width: u8,
    /// Register bit offset
    pub register_bit_offset: u8,
    /// Access size
    pub access_size: u8,
    /// Address
    pub address: u64,
}

impl GenericAddress {
    /// Size
    pub const SIZE: usize = 12;

    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        Some(Self {
            address_space_id: bytes[0],
            register_bit_width: bytes[1],
            register_bit_offset: bytes[2],
            access_size: bytes[3],
            address: u64::from_le_bytes(bytes[4..12].try_into().ok()?),
        })
    }

    /// Is memory mapped
    pub fn is_memory(&self) -> bool {
        self.address_space_id == address_space::SYSTEM_MEMORY
    }

    /// Is I/O port
    pub fn is_io(&self) -> bool {
        self.address_space_id == address_space::SYSTEM_IO
    }
}

/// Address space IDs
pub mod address_space {
    /// System memory address space
    pub const SYSTEM_MEMORY: u8 = 0;
    /// System I/O address space
    pub const SYSTEM_IO: u8 = 1;
    /// PCI configuration space
    pub const PCI_CONFIG: u8 = 2;
    /// Embedded controller
    pub const EMBEDDED_CONTROLLER: u8 = 3;
    /// SMBus address space
    pub const SMBUS: u8 = 4;
    /// CMOS address space
    pub const CMOS: u8 = 5;
    /// PCI BAR target
    pub const PCI_BAR_TARGET: u8 = 6;
    /// IPMI address space
    pub const IPMI: u8 = 7;
    /// GPIO address space
    pub const GPIO: u8 = 8;
    /// Generic serial bus
    pub const GENERIC_SERIAL_BUS: u8 = 9;
    /// Platform Communications Channel
    pub const PCC: u8 = 10;
    /// Functional fixed hardware
    pub const FUNCTIONAL_FIXED_HW: u8 = 127;
}

// =============================================================================
// SRAT (SYSTEM RESOURCE AFFINITY TABLE)
// =============================================================================

/// SRAT table
pub struct Srat<'a> {
    header: SdtHeader,
    data: &'a [u8],
}

impl<'a> Srat<'a> {
    /// Parse SRAT
    pub fn parse(data: &'a [u8]) -> Option<Self> {
        let header = SdtHeader::from_bytes(data)?;
        if header.signature != signature::SRAT {
            return None;
        }

        Some(Self { header, data })
    }

    /// Get header
    pub fn header(&self) -> &SdtHeader {
        &self.header
    }

    /// Iterate entries
    pub fn entries(&self) -> SratEntryIter<'a> {
        SratEntryIter {
            data: self.data,
            offset: SdtHeader::SIZE + 12, // Skip table revision and reserved
            end: self.header.length as usize,
        }
    }

    /// Get memory affinity entries
    pub fn memory_affinities(&self) -> impl Iterator<Item = SratMemoryAffinity> + 'a {
        self.entries().filter_map(|e| {
            if e.entry_type == srat_entry_type::MEMORY_AFFINITY {
                SratMemoryAffinity::from_bytes(e.data)
            } else {
                None
            }
        })
    }

    /// Get processor affinity entries
    pub fn processor_affinities(&self) -> impl Iterator<Item = SratProcessorAffinity> + 'a {
        self.entries().filter_map(|e| {
            if e.entry_type == srat_entry_type::PROCESSOR_LOCAL_APIC_AFFINITY {
                SratProcessorAffinity::from_bytes(e.data)
            } else {
                None
            }
        })
    }
}

/// SRAT entry types
pub mod srat_entry_type {
    /// Processor Local APIC/SAPIC Affinity
    pub const PROCESSOR_LOCAL_APIC_AFFINITY: u8 = 0;
    /// Memory Affinity
    pub const MEMORY_AFFINITY: u8 = 1;
    /// Processor Local x2APIC Affinity
    pub const PROCESSOR_LOCAL_X2APIC_AFFINITY: u8 = 2;
    /// GICC Affinity
    pub const GICC_AFFINITY: u8 = 3;
    /// GIC ITS Affinity
    pub const GIC_ITS_AFFINITY: u8 = 4;
    /// Generic Initiator Affinity
    pub const GENERIC_INITIATOR_AFFINITY: u8 = 5;
}

/// SRAT entry
pub struct SratEntry<'a> {
    /// Entry type
    pub entry_type: u8,
    /// Entry length
    pub length: u8,
    /// Raw entry data
    pub data: &'a [u8],
}

/// SRAT entry iterator
pub struct SratEntryIter<'a> {
    data: &'a [u8],
    offset: usize,
    end: usize,
}

impl<'a> Iterator for SratEntryIter<'a> {
    type Item = SratEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset + 2 > self.end {
            return None;
        }

        let entry_type = self.data[self.offset];
        let length = self.data[self.offset + 1];
        let entry_end = self.offset + length as usize;

        if entry_end > self.end || length < 2 {
            return None;
        }

        let entry = SratEntry {
            entry_type,
            length,
            data: &self.data[self.offset..entry_end],
        };

        self.offset = entry_end;
        Some(entry)
    }
}

/// SRAT memory affinity
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct SratMemoryAffinity {
    /// Entry type
    pub entry_type: u8,
    /// Length
    pub length: u8,
    /// Proximity domain
    pub proximity_domain: u32,
    /// Reserved
    pub reserved1: u16,
    /// Base address low
    pub base_address_low: u32,
    /// Base address high
    pub base_address_high: u32,
    /// Length low
    pub length_low: u32,
    /// Length high
    pub length_high: u32,
    /// Reserved
    pub reserved2: u32,
    /// Flags
    pub flags: u32,
    /// Reserved
    pub reserved3: u64,
}

impl SratMemoryAffinity {
    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 40 {
            return None;
        }

        Some(Self {
            entry_type: bytes[0],
            length: bytes[1],
            proximity_domain: u32::from_le_bytes(bytes[2..6].try_into().ok()?),
            reserved1: u16::from_le_bytes([bytes[6], bytes[7]]),
            base_address_low: u32::from_le_bytes(bytes[8..12].try_into().ok()?),
            base_address_high: u32::from_le_bytes(bytes[12..16].try_into().ok()?),
            length_low: u32::from_le_bytes(bytes[16..20].try_into().ok()?),
            length_high: u32::from_le_bytes(bytes[20..24].try_into().ok()?),
            reserved2: u32::from_le_bytes(bytes[24..28].try_into().ok()?),
            flags: u32::from_le_bytes(bytes[28..32].try_into().ok()?),
            reserved3: u64::from_le_bytes(bytes[32..40].try_into().ok()?),
        })
    }

    /// Get base address
    pub fn base_address(&self) -> u64 {
        (u64::from(self.base_address_high) << 32) | u64::from(self.base_address_low)
    }

    /// Get length
    pub fn length(&self) -> u64 {
        (u64::from(self.length_high) << 32) | u64::from(self.length_low)
    }

    /// Is enabled
    pub fn is_enabled(&self) -> bool {
        self.flags & 0x01 != 0
    }

    /// Is hot pluggable
    pub fn is_hot_pluggable(&self) -> bool {
        self.flags & 0x02 != 0
    }

    /// Is non-volatile
    pub fn is_non_volatile(&self) -> bool {
        self.flags & 0x04 != 0
    }
}

/// SRAT processor affinity
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct SratProcessorAffinity {
    /// Entry type
    pub entry_type: u8,
    /// Length
    pub length: u8,
    /// Proximity domain low
    pub proximity_domain_low: u8,
    /// APIC ID
    pub apic_id: u8,
    /// Flags
    pub flags: u32,
    /// Local SAPIC EID
    pub local_sapic_eid: u8,
    /// Proximity domain high
    pub proximity_domain_high: [u8; 3],
    /// Clock domain
    pub clock_domain: u32,
}

impl SratProcessorAffinity {
    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 16 {
            return None;
        }

        Some(Self {
            entry_type: bytes[0],
            length: bytes[1],
            proximity_domain_low: bytes[2],
            apic_id: bytes[3],
            flags: u32::from_le_bytes(bytes[4..8].try_into().ok()?),
            local_sapic_eid: bytes[8],
            proximity_domain_high: [bytes[9], bytes[10], bytes[11]],
            clock_domain: u32::from_le_bytes(bytes[12..16].try_into().ok()?),
        })
    }

    /// Get full proximity domain
    pub fn proximity_domain(&self) -> u32 {
        u32::from(self.proximity_domain_low)
            | (u32::from(self.proximity_domain_high[0]) << 8)
            | (u32::from(self.proximity_domain_high[1]) << 16)
            | (u32::from(self.proximity_domain_high[2]) << 24)
    }

    /// Is enabled
    pub fn is_enabled(&self) -> bool {
        self.flags & 0x01 != 0
    }
}

// =============================================================================
// ACPI ERROR
// =============================================================================

/// ACPI error
#[derive(Debug, Clone)]
pub enum AcpiError {
    /// Invalid RSDP
    InvalidRsdp,
    /// Invalid table
    InvalidTable,
    /// Table not found
    TableNotFound,
    /// Checksum error
    ChecksumError,
}

impl fmt::Display for AcpiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRsdp => write!(f, "invalid RSDP"),
            Self::InvalidTable => write!(f, "invalid ACPI table"),
            Self::TableNotFound => write!(f, "ACPI table not found"),
            Self::ChecksumError => write!(f, "ACPI checksum error"),
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_rsdp_signature() {
        assert_eq!(RSDP_SIGNATURE, *b"RSD PTR ");
    }

    #[test]
    fn test_table_signatures() {
        assert_eq!(&signature::MADT, b"APIC");
        assert_eq!(&signature::FADT, b"FACP");
        assert_eq!(&signature::HPET, b"HPET");
    }

    #[test]
    fn test_generic_address() {
        let gas = GenericAddress {
            address_space_id: address_space::SYSTEM_MEMORY,
            register_bit_width: 64,
            register_bit_offset: 0,
            access_size: 0,
            address: 0xFED00000,
        };

        assert!(gas.is_memory());
        assert!(!gas.is_io());
    }
}
