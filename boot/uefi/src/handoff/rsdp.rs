//! RSDP and ACPI Table Handling
//!
//! Structures and utilities for ACPI Root System Description Pointer.

use crate::raw::types::*;

// =============================================================================
// RSDP SIGNATURE
// =============================================================================

/// RSDP signature bytes
pub const RSDP_SIGNATURE: [u8; 8] = *b"RSD PTR ";

/// RSDP search start (EBDA)
pub const RSDP_SEARCH_START: u64 = 0x000E0000;

/// RSDP search end
pub const RSDP_SEARCH_END: u64 = 0x000FFFFF;

/// RSDP alignment
pub const RSDP_ALIGNMENT: u64 = 16;

// =============================================================================
// RSDP V1 (ACPI 1.0)
// =============================================================================

/// RSDP version 1 structure (ACPI 1.0)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct RsdpV1 {
    /// "RSD PTR " signature
    pub signature: [u8; 8],
    /// Checksum for first 20 bytes
    pub checksum: u8,
    /// OEM ID string
    pub oem_id: [u8; 6],
    /// ACPI revision (0 for 1.0, 2 for 2.0+)
    pub revision: u8,
    /// Physical address of RSDT
    pub rsdt_address: u32,
}

impl RsdpV1 {
    /// Structure size
    pub const SIZE: usize = 20;

    /// Validate checksum
    pub fn validate_checksum(&self) -> bool {
        let bytes = unsafe {
            core::slice::from_raw_parts(
                self as *const _ as *const u8,
                Self::SIZE,
            )
        };

        bytes.iter().fold(0u8, |acc, &b| acc.wrapping_add(b)) == 0
    }

    /// Validate signature
    pub fn validate_signature(&self) -> bool {
        self.signature == RSDP_SIGNATURE
    }

    /// Full validation
    pub fn validate(&self) -> bool {
        self.validate_signature() && self.validate_checksum()
    }

    /// Get OEM ID as string
    pub fn oem_id_str(&self) -> &str {
        core::str::from_utf8(&self.oem_id)
            .unwrap_or("")
            .trim_end_matches('\0')
            .trim()
    }

    /// Get RSDT address
    pub fn rsdt(&self) -> PhysicalAddress {
        PhysicalAddress(self.rsdt_address as u64)
    }
}

// =============================================================================
// RSDP V2 (ACPI 2.0+)
// =============================================================================

/// RSDP version 2 structure (ACPI 2.0+)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct RsdpV2 {
    /// Version 1 fields
    pub v1: RsdpV1,

    /// Length of entire RSDP
    pub length: u32,
    /// Physical address of XSDT
    pub xsdt_address: u64,
    /// Extended checksum
    pub extended_checksum: u8,
    /// Reserved bytes
    pub reserved: [u8; 3],
}

impl RsdpV2 {
    /// Structure size
    pub const SIZE: usize = 36;

    /// Validate extended checksum
    pub fn validate_extended_checksum(&self) -> bool {
        let bytes = unsafe {
            core::slice::from_raw_parts(
                self as *const _ as *const u8,
                self.length as usize,
            )
        };

        bytes.iter().fold(0u8, |acc, &b| acc.wrapping_add(b)) == 0
    }

    /// Full validation
    pub fn validate(&self) -> bool {
        self.v1.validate() && self.validate_extended_checksum()
    }

    /// Get XSDT address
    pub fn xsdt(&self) -> PhysicalAddress {
        PhysicalAddress(self.xsdt_address)
    }

    /// Get RSDT address (fallback)
    pub fn rsdt(&self) -> PhysicalAddress {
        self.v1.rsdt()
    }

    /// Get preferred SDT address (XSDT if available)
    pub fn preferred_sdt(&self) -> PhysicalAddress {
        if self.xsdt_address != 0 {
            PhysicalAddress(self.xsdt_address)
        } else {
            self.rsdt()
        }
    }
}

// =============================================================================
// RSDP INFO
// =============================================================================

/// RSDP information
#[derive(Debug, Clone, Copy)]
pub struct RsdpInfo {
    /// RSDP physical address
    pub address: PhysicalAddress,
    /// ACPI revision
    pub revision: u8,
    /// RSDT address (ACPI 1.0)
    pub rsdt_address: PhysicalAddress,
    /// XSDT address (ACPI 2.0+)
    pub xsdt_address: Option<PhysicalAddress>,
    /// OEM ID
    pub oem_id: [u8; 6],
}

impl RsdpInfo {
    /// Create from V1 RSDP
    pub fn from_v1(address: PhysicalAddress, rsdp: &RsdpV1) -> Self {
        Self {
            address,
            revision: rsdp.revision,
            rsdt_address: rsdp.rsdt(),
            xsdt_address: None,
            oem_id: rsdp.oem_id,
        }
    }

    /// Create from V2 RSDP
    pub fn from_v2(address: PhysicalAddress, rsdp: &RsdpV2) -> Self {
        Self {
            address,
            revision: rsdp.v1.revision,
            rsdt_address: rsdp.rsdt(),
            xsdt_address: if rsdp.xsdt_address != 0 {
                Some(rsdp.xsdt())
            } else {
                None
            },
            oem_id: rsdp.v1.oem_id,
        }
    }

    /// Check if ACPI 2.0+
    pub fn is_acpi2(&self) -> bool {
        self.revision >= 2 && self.xsdt_address.is_some()
    }

    /// Get preferred SDT address
    pub fn sdt_address(&self) -> PhysicalAddress {
        self.xsdt_address.unwrap_or(self.rsdt_address)
    }

    /// Get OEM ID as string
    pub fn oem_id_str(&self) -> &str {
        core::str::from_utf8(&self.oem_id)
            .unwrap_or("")
            .trim_end_matches('\0')
            .trim()
    }
}

impl Default for RsdpInfo {
    fn default() -> Self {
        Self {
            address: PhysicalAddress(0),
            revision: 0,
            rsdt_address: PhysicalAddress(0),
            xsdt_address: None,
            oem_id: [0; 6],
        }
    }
}

// =============================================================================
// ACPI SDT HEADER
// =============================================================================

/// ACPI System Description Table header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct AcpiSdtHeader {
    /// Table signature (4 bytes)
    pub signature: [u8; 4],
    /// Total table length
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
    /// Header size
    pub const SIZE: usize = 36;

    /// Get signature as string
    pub fn signature_str(&self) -> &str {
        core::str::from_utf8(&self.signature)
            .unwrap_or("")
            .trim()
    }

    /// Get OEM ID as string
    pub fn oem_id_str(&self) -> &str {
        core::str::from_utf8(&self.oem_id)
            .unwrap_or("")
            .trim_end_matches('\0')
            .trim()
    }

    /// Get OEM table ID as string
    pub fn oem_table_id_str(&self) -> &str {
        core::str::from_utf8(&self.oem_table_id)
            .unwrap_or("")
            .trim_end_matches('\0')
            .trim()
    }

    /// Validate checksum
    pub fn validate_checksum(&self, data: &[u8]) -> bool {
        if data.len() < self.length as usize {
            return false;
        }

        data[..self.length as usize]
            .iter()
            .fold(0u8, |acc, &b| acc.wrapping_add(b)) == 0
    }
}

// =============================================================================
// COMMON ACPI TABLES
// =============================================================================

/// Known ACPI table signatures
pub mod signatures {
    /// Root System Description Table
    pub const RSDT: [u8; 4] = *b"RSDT";
    /// Extended System Description Table
    pub const XSDT: [u8; 4] = *b"XSDT";
    /// Fixed ACPI Description Table
    pub const FADT: [u8; 4] = *b"FACP";
    /// Differentiated System Description Table
    pub const DSDT: [u8; 4] = *b"DSDT";
    /// Secondary System Description Table
    pub const SSDT: [u8; 4] = *b"SSDT";
    /// Multiple APIC Description Table
    pub const MADT: [u8; 4] = *b"APIC";
    /// High Precision Event Timer
    pub const HPET: [u8; 4] = *b"HPET";
    /// Memory Configuration Table
    pub const MCFG: [u8; 4] = *b"MCFG";
    /// Boot Graphics Resource Table
    pub const BGRT: [u8; 4] = *b"BGRT";
    /// Serial Port Console Redirection
    pub const SPCR: [u8; 4] = *b"SPCR";
    /// NUMA Memory Affinity
    pub const SRAT: [u8; 4] = *b"SRAT";
    /// NUMA Locality
    pub const SLIT: [u8; 4] = *b"SLIT";
    /// Watchdog Timer
    pub const WDDT: [u8; 4] = *b"WDDT";
    /// Windows ACPI Emulated Devices
    pub const WAET: [u8; 4] = *b"WAET";
    /// Trusted Platform Module
    pub const TPM2: [u8; 4] = *b"TPM2";
    /// Platform Debug Trigger
    pub const PDTT: [u8; 4] = *b"PDTT";
}

/// ACPI table info
#[derive(Debug, Clone, Copy)]
pub struct AcpiTableInfo {
    /// Table signature
    pub signature: [u8; 4],
    /// Physical address
    pub address: PhysicalAddress,
    /// Table length
    pub length: u32,
    /// Revision
    pub revision: u8,
}

impl AcpiTableInfo {
    /// Get signature as string
    pub fn signature_str(&self) -> &str {
        core::str::from_utf8(&self.signature)
            .unwrap_or("")
            .trim()
    }
}

// =============================================================================
// ACPI TABLE FINDER
// =============================================================================

/// ACPI table finder
pub struct AcpiTableFinder {
    /// RSDP info
    rsdp: RsdpInfo,
    /// Found tables
    tables: [Option<AcpiTableInfo>; 32],
    /// Table count
    table_count: usize,
}

impl AcpiTableFinder {
    /// Create new finder
    pub fn new(rsdp: RsdpInfo) -> Self {
        Self {
            rsdp,
            tables: [None; 32],
            table_count: 0,
        }
    }

    /// Get RSDP info
    pub fn rsdp(&self) -> &RsdpInfo {
        &self.rsdp
    }

    /// Add table
    pub fn add_table(&mut self, info: AcpiTableInfo) {
        if self.table_count < 32 {
            self.tables[self.table_count] = Some(info);
            self.table_count += 1;
        }
    }

    /// Find table by signature
    pub fn find(&self, signature: &[u8; 4]) -> Option<&AcpiTableInfo> {
        self.tables[..self.table_count]
            .iter()
            .flatten()
            .find(|t| &t.signature == signature)
    }

    /// Find MADT (APIC table)
    pub fn find_madt(&self) -> Option<&AcpiTableInfo> {
        self.find(&signatures::MADT)
    }

    /// Find FADT
    pub fn find_fadt(&self) -> Option<&AcpiTableInfo> {
        self.find(&signatures::FADT)
    }

    /// Find HPET
    pub fn find_hpet(&self) -> Option<&AcpiTableInfo> {
        self.find(&signatures::HPET)
    }

    /// Find MCFG (PCI configuration)
    pub fn find_mcfg(&self) -> Option<&AcpiTableInfo> {
        self.find(&signatures::MCFG)
    }

    /// Find BGRT (boot graphics)
    pub fn find_bgrt(&self) -> Option<&AcpiTableInfo> {
        self.find(&signatures::BGRT)
    }

    /// Find all tables with signature
    pub fn find_all<'a>(&'a self, signature: &'a [u8; 4]) -> impl Iterator<Item = &'a AcpiTableInfo> + 'a {
        self.tables[..self.table_count]
            .iter()
            .flatten()
            .filter(move |t| &t.signature == signature)
    }

    /// Get all SSDTs
    pub fn ssdts(&self) -> impl Iterator<Item = &AcpiTableInfo> + '_ {
        self.find_all(&signatures::SSDT)
    }

    /// Iterate all tables
    pub fn iter(&self) -> impl Iterator<Item = &AcpiTableInfo> + '_ {
        self.tables[..self.table_count].iter().flatten()
    }

    /// Get table count
    pub fn count(&self) -> usize {
        self.table_count
    }
}

// =============================================================================
// MADT (APIC) STRUCTURES
// =============================================================================

/// MADT entry types
pub mod madt_types {
    /// Processor Local APIC
    pub const LOCAL_APIC: u8 = 0;
    /// I/O APIC
    pub const IO_APIC: u8 = 1;
    /// Interrupt Source Override
    pub const INTERRUPT_OVERRIDE: u8 = 2;
    /// NMI Source
    pub const NMI_SOURCE: u8 = 3;
    /// Local APIC NMI
    pub const LOCAL_APIC_NMI: u8 = 4;
    /// Local APIC Address Override
    pub const LOCAL_APIC_OVERRIDE: u8 = 5;
    /// I/O SAPIC
    pub const IO_SAPIC: u8 = 6;
    /// Local SAPIC
    pub const LOCAL_SAPIC: u8 = 7;
    /// Platform Interrupt Sources
    pub const PLATFORM_INTERRUPT: u8 = 8;
    /// Processor Local x2APIC
    pub const LOCAL_X2APIC: u8 = 9;
    /// Local x2APIC NMI
    pub const LOCAL_X2APIC_NMI: u8 = 10;
    /// GIC CPU Interface
    pub const GICC: u8 = 11;
    /// GIC Distributor
    pub const GICD: u8 = 12;
    /// GIC MSI Frame
    pub const GIC_MSI_FRAME: u8 = 13;
    /// GIC Redistributor
    pub const GICR: u8 = 14;
    /// GIC Interrupt Translation Service
    pub const GIC_ITS: u8 = 15;
    /// Multiprocessor Wakeup
    pub const MP_WAKEUP: u8 = 16;
}

/// MADT entry header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct MadtEntryHeader {
    /// Entry type
    pub entry_type: u8,
    /// Entry length
    pub length: u8,
}

/// Local APIC entry
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct MadtLocalApic {
    /// Header
    pub header: MadtEntryHeader,
    /// ACPI processor UID
    pub acpi_processor_id: u8,
    /// APIC ID
    pub apic_id: u8,
    /// Flags
    pub flags: u32,
}

impl MadtLocalApic {
    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        (self.flags & 1) != 0
    }

    /// Check if online capable
    pub fn is_online_capable(&self) -> bool {
        (self.flags & 2) != 0
    }
}

/// I/O APIC entry
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
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
    pub gsi_base: u32,
}

/// Interrupt source override entry
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct MadtInterruptOverride {
    /// Header
    pub header: MadtEntryHeader,
    /// Bus (0 = ISA)
    pub bus: u8,
    /// Source IRQ
    pub source: u8,
    /// Global system interrupt
    pub gsi: u32,
    /// Flags
    pub flags: u16,
}

impl MadtInterruptOverride {
    /// Get polarity
    pub fn polarity(&self) -> u8 {
        (self.flags & 0x03) as u8
    }

    /// Get trigger mode
    pub fn trigger_mode(&self) -> u8 {
        ((self.flags >> 2) & 0x03) as u8
    }
}

/// Local APIC NMI entry
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct MadtLocalApicNmi {
    /// Header
    pub header: MadtEntryHeader,
    /// ACPI processor UID
    pub acpi_processor_uid: u8,
    /// Flags
    pub flags: u16,
    /// LINT# (0 or 1)
    pub lint: u8,
}

/// Local x2APIC entry
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct MadtLocalX2Apic {
    /// Header
    pub header: MadtEntryHeader,
    /// Reserved
    pub reserved: u16,
    /// x2APIC ID
    pub x2apic_id: u32,
    /// Flags
    pub flags: u32,
    /// ACPI processor UID
    pub acpi_processor_uid: u32,
}

impl MadtLocalX2Apic {
    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        (self.flags & 1) != 0
    }
}

// =============================================================================
// CPU INFO FROM MADT
// =============================================================================

/// CPU information from MADT
#[derive(Debug, Clone, Copy, Default)]
pub struct CpuInfo {
    /// APIC ID
    pub apic_id: u32,
    /// ACPI processor UID
    pub acpi_uid: u32,
    /// Is BSP (Bootstrap Processor)
    pub is_bsp: bool,
    /// Is enabled
    pub enabled: bool,
    /// Is x2APIC
    pub x2apic: bool,
}

/// I/O APIC information
#[derive(Debug, Clone, Copy, Default)]
pub struct IoApicInfo {
    /// I/O APIC ID
    pub id: u8,
    /// Base address
    pub address: u32,
    /// GSI base
    pub gsi_base: u32,
}

/// IRQ override information
#[derive(Debug, Clone, Copy, Default)]
pub struct IrqOverrideInfo {
    /// Source IRQ (ISA)
    pub source_irq: u8,
    /// Destination GSI
    pub gsi: u32,
    /// Polarity (0=default, 1=high, 3=low)
    pub polarity: u8,
    /// Trigger (0=default, 1=edge, 3=level)
    pub trigger: u8,
}

/// MADT parsed information
#[derive(Debug, Clone)]
pub struct MadtInfo {
    /// Local APIC address
    pub local_apic_address: u32,
    /// CPUs
    pub cpus: [Option<CpuInfo>; 256],
    /// CPU count
    pub cpu_count: usize,
    /// I/O APICs
    pub io_apics: [Option<IoApicInfo>; 8],
    /// I/O APIC count
    pub io_apic_count: usize,
    /// IRQ overrides
    pub irq_overrides: [Option<IrqOverrideInfo>; 16],
    /// IRQ override count
    pub irq_override_count: usize,
    /// Flags
    pub flags: u32,
}

impl Default for MadtInfo {
    fn default() -> Self {
        const NONE_CPU: Option<CpuInfo> = None;
        const NONE_IOAPIC: Option<IoApicInfo> = None;
        const NONE_IRQ: Option<IrqOverrideInfo> = None;
        Self {
            local_apic_address: 0,
            cpus: [NONE_CPU; 256],
            cpu_count: 0,
            io_apics: [NONE_IOAPIC; 8],
            io_apic_count: 0,
            irq_overrides: [NONE_IRQ; 16],
            irq_override_count: 0,
            flags: 0,
        }
    }
}

impl MadtInfo {
    /// Check if 8259 PICs are present
    pub fn has_8259_pic(&self) -> bool {
        (self.flags & 1) != 0
    }

    /// Get BSP APIC ID
    pub fn bsp_apic_id(&self) -> Option<u32> {
        self.cpus[..self.cpu_count]
            .iter()
            .flatten()
            .find(|cpu| cpu.is_bsp)
            .map(|cpu| cpu.apic_id)
    }

    /// Get enabled CPU count
    pub fn enabled_cpu_count(&self) -> usize {
        self.cpus[..self.cpu_count]
            .iter()
            .flatten()
            .filter(|cpu| cpu.enabled)
            .count()
    }

    /// Find IRQ override
    pub fn find_irq_override(&self, irq: u8) -> Option<&IrqOverrideInfo> {
        self.irq_overrides[..self.irq_override_count]
            .iter()
            .flatten()
            .find(|o| o.source_irq == irq)
    }

    /// Map ISA IRQ to GSI
    pub fn irq_to_gsi(&self, irq: u8) -> u32 {
        self.find_irq_override(irq)
            .map(|o| o.gsi)
            .unwrap_or(irq as u32)
    }
}

// =============================================================================
// RSDP SEARCH
// =============================================================================

/// Search for RSDP in BIOS memory area
pub unsafe fn search_rsdp_bios() -> Option<RsdpInfo> {
    let mut addr = RSDP_SEARCH_START;

    while addr < RSDP_SEARCH_END {
        let ptr = addr as *const RsdpV1;
        let rsdp = &*ptr;

        if rsdp.validate() {
            if rsdp.revision >= 2 {
                let rsdp2 = &*(ptr as *const RsdpV2);
                if rsdp2.validate() {
                    return Some(RsdpInfo::from_v2(PhysicalAddress(addr), rsdp2));
                }
            }
            return Some(RsdpInfo::from_v1(PhysicalAddress(addr), rsdp));
        }

        addr += RSDP_ALIGNMENT;
    }

    None
}

/// Validate RSDP at given address
pub unsafe fn validate_rsdp(addr: PhysicalAddress) -> Option<RsdpInfo> {
    let ptr = addr.0 as *const RsdpV1;
    let rsdp = &*ptr;

    if rsdp.validate() {
        if rsdp.revision >= 2 {
            let rsdp2 = &*(ptr as *const RsdpV2);
            if rsdp2.validate() {
                return Some(RsdpInfo::from_v2(addr, rsdp2));
            }
        }
        return Some(RsdpInfo::from_v1(addr, rsdp));
    }

    None
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rsdp_v1_size() {
        assert_eq!(RsdpV1::SIZE, 20);
    }

    #[test]
    fn test_rsdp_v2_size() {
        assert_eq!(RsdpV2::SIZE, 36);
    }

    #[test]
    fn test_sdt_header_size() {
        assert_eq!(AcpiSdtHeader::SIZE, 36);
    }

    #[test]
    fn test_rsdp_info() {
        let info = RsdpInfo {
            address: 0xE0000,
            revision: 2,
            rsdt_address: 0x7FFE0000,
            xsdt_address: Some(0x7FFE0100),
            oem_id: *b"HELIX ",
        };

        assert!(info.is_acpi2());
        assert_eq!(info.sdt_address(), 0x7FFE0100);
        assert_eq!(info.oem_id_str(), "HELIX");
    }

    #[test]
    fn test_madt_local_apic() {
        let apic = MadtLocalApic {
            header: MadtEntryHeader {
                entry_type: madt_types::LOCAL_APIC,
                length: 8,
            },
            acpi_processor_id: 0,
            apic_id: 0,
            flags: 0x01, // Enabled
        };

        assert!(apic.is_enabled());
        assert!(!apic.is_online_capable());
    }

    #[test]
    fn test_irq_override() {
        let madt_info = MadtInfo {
            irq_overrides: {
                let mut arr: [Option<IrqOverrideInfo>; 16] = [None; 16];
                arr[0] = Some(IrqOverrideInfo {
                    source_irq: 0,
                    gsi: 2,
                    polarity: 0,
                    trigger: 0,
                });
                arr
            },
            irq_override_count: 1,
            ..Default::default()
        };

        assert_eq!(madt_info.irq_to_gsi(0), 2);
        assert_eq!(madt_info.irq_to_gsi(1), 1); // No override
    }
}
