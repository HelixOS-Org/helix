//! # Firmware Abstraction Module
//!
//! This module provides firmware abstractions for ACPI, SMBIOS, UEFI,
//! and Device Tree support.
//!
//! ## Features
//!
//! - ACPI table parsing (RSDT, XSDT, FADT, MADT, etc.)
//! - SMBIOS structure enumeration
//! - UEFI runtime services interface
//! - Device Tree blob parsing

use core::slice;

extern crate alloc;

// =============================================================================
// ACPI Support
// =============================================================================

/// ACPI RSDP (Root System Description Pointer) structure
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Rsdp {
    /// Signature "RSD PTR "
    pub signature: [u8; 8],
    /// Checksum of first 20 bytes
    pub checksum: u8,
    /// OEM ID
    pub oem_id: [u8; 6],
    /// ACPI revision (0 = 1.0, 2 = 2.0+)
    pub revision: u8,
    /// Physical address of RSDT
    pub rsdt_address: u32,
}

/// ACPI 2.0+ XSDP (Extended System Description Pointer)
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Xsdp {
    /// Base RSDP fields
    pub rsdp: Rsdp,
    /// Length of table
    pub length: u32,
    /// Physical address of XSDT
    pub xsdt_address: u64,
    /// Checksum of entire table
    pub extended_checksum: u8,
    /// Reserved
    pub reserved: [u8; 3],
}

/// ACPI table header (common to all ACPI tables)
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct AcpiSdtHeader {
    /// Table signature (4 ASCII chars)
    pub signature: [u8; 4],
    /// Table length including header
    pub length: u32,
    /// ACPI revision
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

impl AcpiSdtHeader {
    /// Get signature as string slice
    pub fn signature_str(&self) -> &str {
        core::str::from_utf8(&self.signature).unwrap_or("????")
    }

    /// Validate checksum
    pub fn validate(&self) -> bool {
        let bytes = unsafe {
            slice::from_raw_parts((self as *const Self).cast::<u8>(), self.length as usize)
        };
        bytes.iter().fold(0u8, |sum, &b| sum.wrapping_add(b)) == 0
    }
}

/// RSDT (Root System Description Table) - 32-bit addresses
pub struct Rsdt {
    #[allow(dead_code)]
    header: &'static AcpiSdtHeader,
    entries: &'static [u32],
}

impl Rsdt {
    /// Create RSDT from physical address
    ///
    /// # Safety
    ///
    /// Address must point to valid RSDT table.
    pub unsafe fn from_address(addr: u64) -> Option<Self> {
        let header = &*(addr as *const AcpiSdtHeader);
        if &header.signature != b"RSDT" {
            return None;
        }

        let entry_count = (header.length as usize - core::mem::size_of::<AcpiSdtHeader>()) / 4;
        // SAFETY: ACPI addresses are valid on 64-bit systems
        #[allow(clippy::cast_possible_truncation)]
        let entries_ptr = (addr as usize + core::mem::size_of::<AcpiSdtHeader>()) as *const u32;
        let entries = slice::from_raw_parts(entries_ptr, entry_count);

        Some(Self { header, entries })
    }

    /// Get number of table entries
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Get table address by index
    pub fn get_entry(&self, index: usize) -> Option<u64> {
        self.entries.get(index).map(|&addr| u64::from(addr))
    }

    /// Find table by signature
    pub fn find_table(&self, signature: &[u8; 4]) -> Option<u64> {
        for &addr in self.entries {
            let header = unsafe { &*(addr as *const AcpiSdtHeader) };
            if &header.signature == signature {
                return Some(u64::from(addr));
            }
        }
        None
    }

    /// Iterate over all table addresses
    pub fn iter(&self) -> impl Iterator<Item = u64> + '_ {
        self.entries.iter().map(|&addr| u64::from(addr))
    }
}

/// XSDT (Extended System Description Table) - 64-bit addresses
pub struct Xsdt {
    #[allow(dead_code)]
    header: &'static AcpiSdtHeader,
    entries: &'static [u64],
}

impl Xsdt {
    /// Create XSDT from physical address
    ///
    /// # Safety
    ///
    /// Address must point to valid XSDT table.
    pub unsafe fn from_address(addr: u64) -> Option<Self> {
        let header = &*(addr as *const AcpiSdtHeader);
        if &header.signature != b"XSDT" {
            return None;
        }

        let entry_count = (header.length as usize - core::mem::size_of::<AcpiSdtHeader>()) / 8;
        // SAFETY: ACPI addresses are valid on 64-bit systems
        #[allow(clippy::cast_possible_truncation)]
        let entries_ptr = (addr as usize + core::mem::size_of::<AcpiSdtHeader>()) as *const u64;
        let entries = slice::from_raw_parts(entries_ptr, entry_count);

        Some(Self { header, entries })
    }

    /// Get number of table entries
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Get table address by index
    pub fn get_entry(&self, index: usize) -> Option<u64> {
        self.entries.get(index).copied()
    }

    /// Find table by signature
    pub fn find_table(&self, signature: &[u8; 4]) -> Option<u64> {
        for &addr in self.entries {
            let header = unsafe { &*(addr as *const AcpiSdtHeader) };
            if &header.signature == signature {
                return Some(addr);
            }
        }
        None
    }

    /// Iterate over all table addresses
    pub fn iter(&self) -> impl Iterator<Item = u64> + '_ {
        self.entries.iter().copied()
    }
}

/// ACPI table finder
pub struct AcpiFinder {
    is_xsdt: bool,
    table_addr: u64,
}

impl AcpiFinder {
    /// Create from RSDP address
    ///
    /// # Safety
    ///
    /// Address must point to valid RSDP.
    pub unsafe fn from_rsdp(rsdp_addr: u64) -> Option<Self> {
        let rsdp = &*(rsdp_addr as *const Rsdp);

        // Validate signature
        if &rsdp.signature != b"RSD PTR " {
            return None;
        }

        // ACPI 2.0+ uses XSDT
        if rsdp.revision >= 2 {
            let xsdp = &*(rsdp_addr as *const Xsdp);
            Some(Self {
                is_xsdt: true,
                table_addr: xsdp.xsdt_address,
            })
        } else {
            Some(Self {
                is_xsdt: false,
                table_addr: u64::from(rsdp.rsdt_address),
            })
        }
    }

    /// Find ACPI table by signature
    pub fn find(&self, signature: &[u8; 4]) -> Option<u64> {
        unsafe {
            if self.is_xsdt {
                Xsdt::from_address(self.table_addr)?.find_table(signature)
            } else {
                Rsdt::from_address(self.table_addr)?.find_table(signature)
            }
        }
    }

    /// Find FADT (Fixed ACPI Description Table)
    pub fn find_fadt(&self) -> Option<u64> {
        self.find(b"FACP")
    }

    /// Find MADT (Multiple APIC Description Table)
    pub fn find_madt(&self) -> Option<u64> {
        self.find(b"APIC")
    }

    /// Find HPET table
    pub fn find_hpet(&self) -> Option<u64> {
        self.find(b"HPET")
    }

    /// Find MCFG (PCI Express configuration space)
    pub fn find_mcfg(&self) -> Option<u64> {
        self.find(b"MCFG")
    }

    /// Find SRAT (System Resource Affinity Table)
    pub fn find_srat(&self) -> Option<u64> {
        self.find(b"SRAT")
    }

    /// Find SLIT (System Locality Information Table)
    pub fn find_slit(&self) -> Option<u64> {
        self.find(b"SLIT")
    }

    /// Find BGRT (Boot Graphics Resource Table)
    pub fn find_bgrt(&self) -> Option<u64> {
        self.find(b"BGRT")
    }
}

// =============================================================================
// MADT Parsing
// =============================================================================

/// MADT (Multiple APIC Description Table) header
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct MadtHeader {
    /// Common SDT header
    pub header: AcpiSdtHeader,
    /// Local APIC address
    pub local_apic_address: u32,
    /// Flags
    pub flags: u32,
}

/// MADT entry header
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct MadtEntryHeader {
    /// Entry type
    pub entry_type: u8,
    /// Entry length
    pub length: u8,
}

/// MADT entry types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MadtEntryType {
    /// Processor Local APIC (type 0)
    LocalApic         = 0,
    /// I/O APIC (type 1)
    IoApic            = 1,
    /// Interrupt Source Override (type 2)
    InterruptOverride = 2,
    /// Non-Maskable Interrupt Source (type 3)
    NmiSource         = 3,
    /// Local APIC NMI (type 4)
    LocalApicNmi      = 4,
    /// Local APIC Address Override (type 5)
    LocalApicOverride = 5,
    /// I/O SAPIC (type 6)
    IoSapic           = 6,
    /// Local SAPIC (type 7)
    LocalSapic        = 7,
    /// Platform Interrupt Sources (type 8)
    PlatformInterrupt = 8,
    /// Processor Local x2APIC (type 9)
    LocalX2Apic       = 9,
    /// Local x2APIC NMI (type 10)
    LocalX2ApicNmi    = 10,
    /// GIC CPU Interface (type 11)
    GicCpuInterface   = 11,
    /// GIC Distributor (type 12)
    GicDistributor    = 12,
    /// GIC MSI Frame (type 13)
    GicMsiFrame       = 13,
    /// GIC Redistributor (type 14)
    GicRedistributor  = 14,
    /// GIC Interrupt Translation Service (type 15)
    GicIts            = 15,
}

/// Local APIC entry
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct MadtLocalApic {
    /// Entry header containing type and length
    pub header: MadtEntryHeader,
    /// ACPI processor ID
    pub processor_id: u8,
    /// Processor's local APIC ID
    pub apic_id: u8,
    /// Flags (bit 0: enabled, bit 1: online capable)
    pub flags: u32,
}

impl MadtLocalApic {
    /// Check if this processor is enabled
    pub fn is_enabled(&self) -> bool {
        self.flags & 1 != 0
    }

    /// Check if this processor can be enabled
    pub fn is_online_capable(&self) -> bool {
        self.flags & 2 != 0
    }
}

/// I/O APIC entry
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct MadtIoApic {
    /// Entry header containing type and length
    pub header: MadtEntryHeader,
    /// I/O APIC's ID
    pub io_apic_id: u8,
    /// Reserved byte
    pub reserved: u8,
    /// 32-bit physical address of I/O APIC
    pub io_apic_address: u32,
    /// Global system interrupt number where this I/O APIC's inputs start
    pub global_system_interrupt_base: u32,
}

/// Interrupt source override
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct MadtInterruptOverride {
    /// Entry header containing type and length
    pub header: MadtEntryHeader,
    /// Bus (0 = ISA)
    pub bus: u8,
    /// Bus-relative interrupt source (IRQ)
    pub source: u8,
    /// Global system interrupt that this bus-relative source will signal
    pub global_system_interrupt: u32,
    /// MPS INTI flags
    pub flags: u16,
}

/// Local APIC NMI
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct MadtLocalApicNmi {
    /// Entry header containing type and length
    pub header: MadtEntryHeader,
    /// ACPI processor ID (0xFF means all processors)
    pub processor_id: u8,
    /// MPS INTI flags
    pub flags: u16,
    /// Local APIC LINT pin (0 or 1)
    pub lint: u8,
}

/// MADT iterator
pub struct MadtIterator {
    current: *const u8,
    end: *const u8,
}

impl MadtIterator {
    /// Create iterator from MADT address
    ///
    /// # Safety
    ///
    /// Address must point to valid MADT.
    pub unsafe fn new(madt_addr: u64) -> Option<Self> {
        let header = &*(madt_addr as *const MadtHeader);
        if &header.header.signature != b"APIC" {
            return None;
        }

        // SAFETY: MADT addresses are valid on 64-bit systems
        #[allow(clippy::cast_possible_truncation)]
        let current = (madt_addr as usize + core::mem::size_of::<MadtHeader>()) as *const u8;
        #[allow(clippy::cast_possible_truncation)]
        let end = (madt_addr as usize + header.header.length as usize) as *const u8;

        Some(Self { current, end })
    }
}

/// MADT entry enum
#[derive(Debug)]
pub enum MadtEntry {
    /// Processor local APIC structure
    LocalApic(MadtLocalApic),
    /// I/O APIC structure
    IoApic(MadtIoApic),
    /// Interrupt source override structure
    InterruptOverride(MadtInterruptOverride),
    /// Local APIC NMI structure
    LocalApicNmi(MadtLocalApicNmi),
    /// Unknown entry type with raw type ID
    Unknown(u8),
}

impl Iterator for MadtIterator {
    type Item = MadtEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.end {
            return None;
        }

        unsafe {
            let header = &*(self.current.cast::<MadtEntryHeader>());
            let entry = match header.entry_type {
                0 => MadtEntry::LocalApic(*self.current.cast::<MadtLocalApic>()),
                1 => MadtEntry::IoApic(*self.current.cast::<MadtIoApic>()),
                2 => MadtEntry::InterruptOverride(*self.current.cast::<MadtInterruptOverride>()),
                4 => MadtEntry::LocalApicNmi(*self.current.cast::<MadtLocalApicNmi>()),
                t => MadtEntry::Unknown(t),
            };

            self.current = self.current.add(header.length as usize);
            Some(entry)
        }
    }
}

// =============================================================================
// SMBIOS Support
// =============================================================================

/// SMBIOS 2.x entry point
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Smbios2EntryPoint {
    /// Signature "_SM_"
    pub signature: [u8; 4],
    /// Checksum
    pub checksum: u8,
    /// Entry point length
    pub length: u8,
    /// Major version
    pub major_version: u8,
    /// Minor version
    pub minor_version: u8,
    /// Maximum structure size
    pub max_structure_size: u16,
    /// Entry point revision
    pub revision: u8,
    /// Formatted area
    pub formatted_area: [u8; 5],
    /// Intermediate signature "_DMI_"
    pub intermediate_signature: [u8; 5],
    /// Intermediate checksum
    pub intermediate_checksum: u8,
    /// Structure table length
    pub table_length: u16,
    /// Structure table address
    pub table_address: u32,
    /// Number of structures
    pub structure_count: u16,
    /// BCD revision
    pub bcd_revision: u8,
}

/// SMBIOS 3.x entry point
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Smbios3EntryPoint {
    /// Signature "_SM3_"
    pub signature: [u8; 5],
    /// Checksum
    pub checksum: u8,
    /// Entry point length
    pub length: u8,
    /// Major version
    pub major_version: u8,
    /// Minor version
    pub minor_version: u8,
    /// Docrev
    pub docrev: u8,
    /// Entry point revision
    pub revision: u8,
    /// Reserved
    pub reserved: u8,
    /// Maximum structure size
    pub max_structure_size: u32,
    /// Structure table address
    pub table_address: u64,
}

/// SMBIOS structure header
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct SmbiosHeader {
    /// Structure type
    pub structure_type: u8,
    /// Structure length
    pub length: u8,
    /// Handle
    pub handle: u16,
}

/// Common SMBIOS structure types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SmbiosType {
    /// BIOS Information (Type 0)
    BiosInfo            = 0,
    /// System Information (Type 1)
    SystemInfo          = 1,
    /// Baseboard/Module Information (Type 2)
    BaseboardInfo       = 2,
    /// System Enclosure/Chassis (Type 3)
    ChassisInfo         = 3,
    /// Processor Information (Type 4)
    ProcessorInfo       = 4,
    /// Cache Information (Type 7)
    CacheInfo           = 7,
    /// System Slots (Type 9)
    SystemSlots         = 9,
    /// Physical Memory Array (Type 16)
    PhysicalMemoryArray = 16,
    /// Memory Device (Type 17)
    MemoryDevice        = 17,
    /// Memory Array Mapped Address (Type 19)
    MemoryArrayMappedAddress = 19,
    /// System Boot Information (Type 32)
    SystemBoot          = 32,
    /// End-of-Table (Type 127)
    EndOfTable          = 127,
}

/// SMBIOS structure iterator
pub struct SmbiosIterator {
    current: *const u8,
    end: *const u8,
}

impl SmbiosIterator {
    /// Create from SMBIOS 2.x entry point
    ///
    /// # Safety
    ///
    /// Entry point must be valid.
    pub unsafe fn from_v2(entry: &Smbios2EntryPoint) -> Self {
        let current = entry.table_address as *const u8;
        let end = current.add(entry.table_length as usize);
        Self { current, end }
    }

    /// Create from SMBIOS 3.x entry point
    ///
    /// # Safety
    ///
    /// Entry point must be valid.
    pub unsafe fn from_v3(entry: &Smbios3EntryPoint) -> Self {
        let current = entry.table_address as *const u8;
        // Note: SMBIOS 3.x doesn't have a table length field, but we can use
        // max_structure_size as a reasonable upper bound for iteration
        let end = current.add(entry.max_structure_size as usize);
        Self { current, end }
    }
}

/// SMBIOS structure with strings
pub struct SmbiosStructure {
    header: &'static SmbiosHeader,
    strings_start: *const u8,
    strings_end: *const u8,
}

impl SmbiosStructure {
    /// Get structure type
    pub fn structure_type(&self) -> u8 {
        self.header.structure_type
    }

    /// Get structure handle
    pub fn handle(&self) -> u16 {
        self.header.handle
    }

    /// Get formatted portion as bytes
    pub fn formatted(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(
                (self.header as *const SmbiosHeader).cast::<u8>(),
                self.header.length as usize,
            )
        }
    }

    /// Get string by 1-based index
    pub fn get_string(&self, index: u8) -> Option<&'static str> {
        if index == 0 {
            return None;
        }

        let mut current = self.strings_start;
        let mut count = 1u8;

        unsafe {
            while current < self.strings_end {
                // Find string end
                let mut end = current;
                while end < self.strings_end && *end != 0 {
                    end = end.add(1);
                }

                if count == index {
                    let len = end as usize - current as usize;
                    let bytes = slice::from_raw_parts(current, len);
                    return core::str::from_utf8(bytes).ok();
                }

                // Move past null terminator
                current = end.add(1);

                // Check for double null (end of strings)
                if current < self.strings_end && *current == 0 {
                    break;
                }

                count += 1;
            }
        }

        None
    }
}

impl Iterator for SmbiosIterator {
    type Item = SmbiosStructure;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.end {
            return None;
        }

        unsafe {
            let header = &*(self.current.cast::<SmbiosHeader>());

            // End of table marker
            if header.structure_type == 127 {
                return None;
            }

            // Find strings section
            let strings_start = self.current.add(header.length as usize);
            let mut strings_end = strings_start;

            // Find double null terminator
            while strings_end < self.end {
                if *strings_end == 0 && *strings_end.add(1) == 0 {
                    strings_end = strings_end.add(2);
                    break;
                }
                strings_end = strings_end.add(1);
            }

            let structure = SmbiosStructure {
                header,
                strings_start,
                strings_end,
            };

            self.current = strings_end;
            Some(structure)
        }
    }
}

// =============================================================================
// UEFI Support
// =============================================================================

/// EFI GUID (Globally Unique Identifier)
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct EfiGuid {
    /// First part of the GUID (time-low)
    pub data1: u32,
    /// Second part of the GUID (time-mid)
    pub data2: u16,
    /// Third part of the GUID (time-hi-and-version)
    pub data3: u16,
    /// Fourth part of the GUID (clock-seq and node)
    pub data4: [u8; 8],
}

impl EfiGuid {
    /// Create GUID from components
    pub const fn new(data1: u32, data2: u16, data3: u16, data4: [u8; 8]) -> Self {
        Self {
            data1,
            data2,
            data3,
            data4,
        }
    }

    /// ACPI 2.0 table GUID
    pub const ACPI_20_TABLE: Self = Self::new(0x8868_e871, 0xe4f1, 0x11d3, [
        0xbc, 0x22, 0x00, 0x80, 0xc7, 0x3c, 0x88, 0x81,
    ]);

    /// SMBIOS 3.0 table GUID
    pub const SMBIOS3_TABLE: Self = Self::new(0xf2fd_1544, 0x9794, 0x4a2c, [
        0x99, 0x2e, 0xe5, 0xbb, 0xcf, 0x20, 0xe3, 0x94,
    ]);

    /// SMBIOS table GUID
    pub const SMBIOS_TABLE: Self = Self::new(0xeb9d_2d31, 0x2d88, 0x11d3, [
        0x9a, 0x16, 0x00, 0x90, 0x27, 0x3f, 0xc1, 0x4d,
    ]);
}

impl core::fmt::Debug for EfiGuid {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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

/// EFI System Table
#[derive(Debug)]
#[repr(C)]
pub struct EfiSystemTable {
    /// Table header with signature and revision
    pub header: EfiTableHeader,
    /// Pointer to null-terminated UCS-2 firmware vendor string
    pub firmware_vendor: *const u16,
    /// Firmware vendor-specific revision
    pub firmware_revision: u32,
    /// Handle for the active console input device
    pub console_in_handle: *const (),
    /// Pointer to `EFI_SIMPLE_TEXT_INPUT_PROTOCOL`
    pub con_in: *const (),
    /// Handle for the active console output device
    pub console_out_handle: *const (),
    /// Pointer to `EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL`
    pub con_out: *const (),
    /// Handle for the active standard error device
    pub standard_error_handle: *const (),
    /// Pointer to `EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL` for stderr
    pub std_err: *const (),
    /// Pointer to `EFI_RUNTIME_SERVICES`
    pub runtime_services: *const EfiRuntimeServices,
    /// Pointer to `EFI_BOOT_SERVICES` (null after `ExitBootServices`)
    pub boot_services: *const (),
    /// Number of entries in the configuration table
    pub number_of_table_entries: usize,
    /// Pointer to the configuration table array
    pub configuration_table: *const EfiConfigurationTable,
}

/// EFI Table Header
#[derive(Debug)]
#[repr(C)]
pub struct EfiTableHeader {
    /// Unique signature identifying the table type
    pub signature: u64,
    /// EFI specification version (major.minor)
    pub revision: u32,
    /// Size of the entire table including header
    pub header_size: u32,
    /// CRC32 checksum of the entire table
    pub crc32: u32,
    /// Reserved, must be zero
    pub reserved: u32,
}

/// EFI Configuration Table Entry
#[derive(Debug)]
#[repr(C)]
pub struct EfiConfigurationTable {
    /// GUID that uniquely identifies the configuration table
    pub vendor_guid: EfiGuid,
    /// Pointer to the configuration table data
    pub vendor_table: *const (),
}

/// EFI Runtime Services Table
#[derive(Debug)]
#[repr(C)]
pub struct EfiRuntimeServices {
    /// Table header with signature and revision
    pub header: EfiTableHeader,
    /// `GetTime` runtime service function pointer
    pub get_time: *const (),
    /// `SetTime` runtime service function pointer
    pub set_time: *const (),
    /// `GetWakeupTime` runtime service function pointer
    pub get_wakeup_time: *const (),
    /// `SetWakeupTime` runtime service function pointer
    pub set_wakeup_time: *const (),
    /// `SetVirtualAddressMap` runtime service function pointer
    pub set_virtual_address_map: *const (),
    /// `ConvertPointer` runtime service function pointer
    pub convert_pointer: *const (),
    /// `GetVariable` runtime service function pointer
    pub get_variable: *const (),
    /// `GetNextVariableName` runtime service function pointer
    pub get_next_variable_name: *const (),
    /// `SetVariable` runtime service function pointer
    pub set_variable: *const (),
    /// `GetNextHighMonotonicCount` runtime service function pointer
    pub get_next_high_monotonic_count: *const (),
    /// `ResetSystem` runtime service function pointer
    pub reset_system: *const (),
    /// `UpdateCapsule` runtime service function pointer
    pub update_capsule: *const (),
    /// `QueryCapsuleCapabilities` runtime service function pointer
    pub query_capsule_capabilities: *const (),
    /// `QueryVariableInfo` runtime service function pointer
    pub query_variable_info: *const (),
}

/// EFI memory type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum EfiMemoryType {
    /// Reserved by firmware
    ReservedMemoryType  = 0,
    /// UEFI OS loader code
    LoaderCode          = 1,
    /// UEFI OS loader data
    LoaderData          = 2,
    /// Boot services code
    BootServicesCode    = 3,
    /// Boot services data
    BootServicesData    = 4,
    /// Runtime services code
    RuntimeServicesCode = 5,
    /// Runtime services data
    RuntimeServicesData = 6,
    /// Free usable memory
    ConventionalMemory  = 7,
    /// Memory with errors
    UnusableMemory      = 8,
    /// ACPI reclaim memory
    AcpiReclaimMemory   = 9,
    /// ACPI NVS memory
    AcpiMemoryNvs       = 10,
    /// Memory-mapped I/O
    MemoryMappedIo      = 11,
    /// Memory-mapped I/O port space
    MemoryMappedIoPortSpace = 12,
    /// PA code
    PalCode             = 13,
    /// Persistent memory
    PersistentMemory    = 14,
}

/// EFI memory descriptor
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct EfiMemoryDescriptor {
    /// Type of memory region (see `EfiMemoryType`)
    pub memory_type: u32,
    /// Physical start address of the memory region
    pub physical_start: u64,
    /// Virtual start address of the memory region
    pub virtual_start: u64,
    /// Number of 4KB pages in the memory region
    pub number_of_pages: u64,
    /// Memory attributes (e.g., cacheable, executable)
    pub attribute: u64,
}

impl EfiMemoryDescriptor {
    /// Get memory type enum
    pub fn get_type(&self) -> Option<EfiMemoryType> {
        match self.memory_type {
            0 => Some(EfiMemoryType::ReservedMemoryType),
            1 => Some(EfiMemoryType::LoaderCode),
            2 => Some(EfiMemoryType::LoaderData),
            3 => Some(EfiMemoryType::BootServicesCode),
            4 => Some(EfiMemoryType::BootServicesData),
            5 => Some(EfiMemoryType::RuntimeServicesCode),
            6 => Some(EfiMemoryType::RuntimeServicesData),
            7 => Some(EfiMemoryType::ConventionalMemory),
            8 => Some(EfiMemoryType::UnusableMemory),
            9 => Some(EfiMemoryType::AcpiReclaimMemory),
            10 => Some(EfiMemoryType::AcpiMemoryNvs),
            11 => Some(EfiMemoryType::MemoryMappedIo),
            12 => Some(EfiMemoryType::MemoryMappedIoPortSpace),
            13 => Some(EfiMemoryType::PalCode),
            14 => Some(EfiMemoryType::PersistentMemory),
            _ => None,
        }
    }

    /// Check if memory is usable after `ExitBootServices`
    pub fn is_usable(&self) -> bool {
        matches!(
            self.get_type(),
            Some(
                EfiMemoryType::ConventionalMemory
                    | EfiMemoryType::BootServicesCode
                    | EfiMemoryType::BootServicesData
                    | EfiMemoryType::LoaderCode
                    | EfiMemoryType::LoaderData
            )
        )
    }

    /// Size in bytes
    pub fn size(&self) -> u64 {
        self.number_of_pages * 4096
    }
}

// =============================================================================
// Device Tree Support
// =============================================================================

/// Device Tree Blob header
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FdtHeader {
    /// Magic number (0xd00dfeed)
    pub magic: u32,
    /// Total size of DTB
    pub totalsize: u32,
    /// Offset to structure block
    pub off_dt_struct: u32,
    /// Offset to strings block
    pub off_dt_strings: u32,
    /// Offset to memory reservation block
    pub off_mem_rsvmap: u32,
    /// DTB version
    pub version: u32,
    /// Last compatible version
    pub last_comp_version: u32,
    /// Boot CPU ID
    pub boot_cpuid_phys: u32,
    /// Size of strings block
    pub size_dt_strings: u32,
    /// Size of structure block
    pub size_dt_struct: u32,
}

impl FdtHeader {
    /// Magic value for valid DTB
    pub const MAGIC: u32 = 0xd00d_feed;

    /// Validate header
    pub fn is_valid(&self) -> bool {
        u32::from_be(self.magic) == Self::MAGIC
    }

    /// Get total size
    pub fn total_size(&self) -> u32 {
        u32::from_be(self.totalsize)
    }

    /// Get structure offset
    pub fn struct_offset(&self) -> u32 {
        u32::from_be(self.off_dt_struct)
    }

    /// Get strings offset
    pub fn strings_offset(&self) -> u32 {
        u32::from_be(self.off_dt_strings)
    }

    /// Get version
    pub fn version(&self) -> u32 {
        u32::from_be(self.version)
    }
}

/// DTB structure tokens
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum FdtToken {
    /// Start of a new node
    BeginNode = 1,
    /// End of current node
    EndNode   = 2,
    /// Property definition
    Prop      = 3,
    /// No-op token
    Nop       = 4,
    /// End of DTB structure
    End       = 9,
}

/// Device tree parser
pub struct DeviceTree {
    base: *const u8,
    header: FdtHeader,
}

impl DeviceTree {
    /// Parse device tree from address
    ///
    /// # Safety
    ///
    /// Address must point to valid DTB.
    pub unsafe fn from_address(addr: u64) -> Option<Self> {
        let header = *(addr as *const FdtHeader);
        if !header.is_valid() {
            return None;
        }

        Some(Self {
            base: addr as *const u8,
            header,
        })
    }

    /// Get DTB version
    pub fn version(&self) -> u32 {
        self.header.version()
    }

    /// Get total size
    pub fn total_size(&self) -> u32 {
        self.header.total_size()
    }

    /// Get string at offset
    pub fn get_string(&self, offset: u32) -> Option<&str> {
        unsafe {
            let strings_base = self.base.add(self.header.strings_offset() as usize);
            let str_ptr = strings_base.add(offset as usize);

            // Find null terminator
            let mut len = 0;
            while *str_ptr.add(len) != 0 {
                len += 1;
            }

            let bytes = slice::from_raw_parts(str_ptr, len);
            core::str::from_utf8(bytes).ok()
        }
    }

    /// Get raw DTB data
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.base, self.header.total_size() as usize) }
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;

    use super::*;

    #[test]
    fn test_efi_guid_debug() {
        let guid = EfiGuid::ACPI_20_TABLE;
        let debug = format!("{:?}", guid);
        assert!(debug.contains("8868e871"));
    }

    #[test]
    fn test_efi_memory_type() {
        let desc = EfiMemoryDescriptor {
            memory_type: 7,
            physical_start: 0,
            virtual_start: 0,
            number_of_pages: 10,
            attribute: 0,
        };
        assert_eq!(desc.get_type(), Some(EfiMemoryType::ConventionalMemory));
        assert!(desc.is_usable());
        assert_eq!(desc.size(), 40960);
    }

    #[test]
    fn test_fdt_header() {
        let header = FdtHeader {
            magic: 0xedfe0dd0, // Little-endian of 0xd00dfeed
            totalsize: 0,
            off_dt_struct: 0,
            off_dt_strings: 0,
            off_mem_rsvmap: 0,
            version: 0,
            last_comp_version: 0,
            boot_cpuid_phys: 0,
            size_dt_strings: 0,
            size_dt_struct: 0,
        };
        assert!(header.is_valid());
    }
}
