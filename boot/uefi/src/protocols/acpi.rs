//! ACPI Protocol
//!
//! High-level ACPI table access abstraction.

use super::Protocol;
use crate::error::{Error, Result};
use crate::raw::types::*;

extern crate alloc;
use alloc::vec::Vec;
use core::ptr;

// =============================================================================
// ACPI TABLE PROTOCOL
// =============================================================================

/// ACPI table protocol for accessing system ACPI tables
pub struct AcpiTables {
    /// Handle
    handle: Handle,
    /// RSDP address
    rsdp_address: PhysicalAddress,
    /// RSDT address (32-bit)
    rsdt_address: Option<PhysicalAddress>,
    /// XSDT address (64-bit)
    xsdt_address: Option<PhysicalAddress>,
    /// Cached table list
    tables: Vec<AcpiTableEntry>,
}

impl AcpiTables {
    /// Create new ACPI tables accessor
    pub fn new(handle: Handle) -> Self {
        Self {
            handle,
            rsdp_address: PhysicalAddress(0),
            rsdt_address: None,
            xsdt_address: None,
            tables: Vec::new(),
        }
    }

    /// Get RSDP address
    pub fn rsdp_address(&self) -> PhysicalAddress {
        self.rsdp_address
    }

    /// Get RSDT address
    pub fn rsdt_address(&self) -> Option<PhysicalAddress> {
        self.rsdt_address
    }

    /// Get XSDT address
    pub fn xsdt_address(&self) -> Option<PhysicalAddress> {
        self.xsdt_address
    }

    /// Initialize from RSDP pointer
    pub unsafe fn init_from_rsdp(&mut self, rsdp: *const Rsdp) -> Result<()> {
        if rsdp.is_null() {
            return Err(Error::InvalidParameter);
        }

        let rsdp_ref = &*rsdp;

        // Validate signature
        if &rsdp_ref.signature != b"RSD PTR " {
            return Err(Error::InvalidParameter);
        }

        // Validate checksum
        if !rsdp_ref.validate_checksum() {
            return Err(Error::CrcError);
        }

        self.rsdp_address = PhysicalAddress(rsdp as u64);

        // ACPI 1.0 - use RSDT
        if rsdp_ref.revision == 0 {
            self.rsdt_address = Some(PhysicalAddress(rsdp_ref.rsdt_address as u64));
        } else {
            // ACPI 2.0+ - use XSDT if available
            let rsdp2 = rsdp as *const Rsdp2;
            let rsdp2_ref = &*rsdp2;

            if rsdp2_ref.xsdt_address != 0 {
                self.xsdt_address = Some(PhysicalAddress(rsdp2_ref.xsdt_address));
            }
            self.rsdt_address = Some(PhysicalAddress(rsdp_ref.rsdt_address as u64));
        }

        // Parse table entries
        self.parse_tables()?;

        Ok(())
    }

    /// Parse tables from RSDT/XSDT
    unsafe fn parse_tables(&mut self) -> Result<()> {
        // Prefer XSDT over RSDT
        if let Some(xsdt_addr) = self.xsdt_address {
            self.parse_xsdt(xsdt_addr)?;
        } else if let Some(rsdt_addr) = self.rsdt_address {
            self.parse_rsdt(rsdt_addr)?;
        }

        Ok(())
    }

    /// Parse RSDT (32-bit pointers)
    unsafe fn parse_rsdt(&mut self, addr: PhysicalAddress) -> Result<()> {
        let header = &*(addr.0 as *const AcpiSdtHeader);

        if &header.signature != b"RSDT" {
            return Err(Error::InvalidParameter);
        }

        let num_entries = (header.length as usize - core::mem::size_of::<AcpiSdtHeader>()) / 4;
        let entries_ptr = (addr.0 as usize + core::mem::size_of::<AcpiSdtHeader>()) as *const u32;

        for i in 0..num_entries {
            let table_addr = PhysicalAddress(ptr::read_unaligned(entries_ptr.add(i)) as u64);
            self.add_table_entry(table_addr)?;
        }

        Ok(())
    }

    /// Parse XSDT (64-bit pointers)
    unsafe fn parse_xsdt(&mut self, addr: PhysicalAddress) -> Result<()> {
        let header = &*(addr.0 as *const AcpiSdtHeader);

        if &header.signature != b"XSDT" {
            return Err(Error::InvalidParameter);
        }

        let num_entries = (header.length as usize - core::mem::size_of::<AcpiSdtHeader>()) / 8;
        let entries_ptr = (addr.0 as usize + core::mem::size_of::<AcpiSdtHeader>()) as *const u64;

        for i in 0..num_entries {
            let table_addr = ptr::read_unaligned(entries_ptr.add(i));
            self.add_table_entry(PhysicalAddress(table_addr))?;
        }

        Ok(())
    }

    /// Add table entry
    unsafe fn add_table_entry(&mut self, addr: PhysicalAddress) -> Result<()> {
        if addr.0 == 0 {
            return Ok(());
        }

        let header = &*(addr.0 as *const AcpiSdtHeader);

        self.tables.push(AcpiTableEntry {
            signature: header.signature,
            address: addr,
            length: header.length,
            revision: header.revision,
        });

        Ok(())
    }

    /// Get all tables
    pub fn tables(&self) -> &[AcpiTableEntry] {
        &self.tables
    }

    /// Find table by signature
    pub fn find_table(&self, signature: &[u8; 4]) -> Option<&AcpiTableEntry> {
        self.tables.iter().find(|t| &t.signature == signature)
    }

    /// Get FADT
    pub fn fadt(&self) -> Option<&AcpiTableEntry> {
        self.find_table(b"FACP")
    }

    /// Get MADT
    pub fn madt(&self) -> Option<&AcpiTableEntry> {
        self.find_table(b"APIC")
    }

    /// Get HPET
    pub fn hpet(&self) -> Option<&AcpiTableEntry> {
        self.find_table(b"HPET")
    }

    /// Get MCFG
    pub fn mcfg(&self) -> Option<&AcpiTableEntry> {
        self.find_table(b"MCFG")
    }

    /// Get SRAT
    pub fn srat(&self) -> Option<&AcpiTableEntry> {
        self.find_table(b"SRAT")
    }

    /// Get SLIT
    pub fn slit(&self) -> Option<&AcpiTableEntry> {
        self.find_table(b"SLIT")
    }

    /// Get DSDT
    pub fn dsdt(&self) -> Option<PhysicalAddress> {
        // DSDT is referenced from FADT, not in RSDT/XSDT
        self.fadt().and_then(|fadt_entry| unsafe {
            let fadt = &*(fadt_entry.address.0 as *const Fadt);
            if fadt.header.length >= 148 && fadt.x_dsdt != 0 {
                Some(PhysicalAddress(fadt.x_dsdt))
            } else {
                Some(PhysicalAddress(fadt.dsdt as u64))
            }
        })
    }

    /// Parse MADT and return CPU info
    pub fn parse_madt(&self) -> Option<MadtInfo> {
        let madt_entry = self.madt()?;

        unsafe {
            let madt = &*(madt_entry.address.0 as *const Madt);
            let mut info = MadtInfo {
                local_apic_address: madt.local_apic_address as u64,
                flags: madt.flags,
                local_apics: Vec::new(),
                io_apics: Vec::new(),
                interrupt_overrides: Vec::new(),
                nmi_sources: Vec::new(),
                local_apic_nmis: Vec::new(),
            };

            // Parse entries
            let end = madt_entry.address + madt_entry.length as u64;
            let mut ptr = madt_entry.address + core::mem::size_of::<Madt>() as u64;

            while ptr < end {
                let entry_type = *(ptr.0 as *const u8);
                let entry_len = *((ptr + 1).0 as *const u8);

                if entry_len == 0 {
                    break;
                }

                match entry_type {
                    0 => {
                        // Processor Local APIC
                        let lapic = &*(ptr.0 as *const MadtLocalApic);
                        info.local_apics.push(LocalApicInfo {
                            processor_id: lapic.processor_id,
                            apic_id: lapic.apic_id,
                            flags: lapic.flags,
                        });
                    },
                    1 => {
                        // I/O APIC
                        let ioapic = &*(ptr.0 as *const MadtIoApic);
                        info.io_apics.push(IoApicInfo {
                            id: ioapic.io_apic_id,
                            address: ioapic.io_apic_address,
                            gsi_base: ioapic.global_system_interrupt_base,
                        });
                    },
                    2 => {
                        // Interrupt Source Override
                        let iso = &*(ptr.0 as *const MadtInterruptOverride);
                        info.interrupt_overrides.push(InterruptOverrideInfo {
                            bus: iso.bus,
                            source: iso.source,
                            gsi: iso.global_system_interrupt,
                            flags: iso.flags,
                        });
                    },
                    3 => {
                        // NMI Source
                        let nmi = &*(ptr.0 as *const MadtNmiSource);
                        info.nmi_sources.push(NmiSourceInfo {
                            flags: nmi.flags,
                            gsi: nmi.global_system_interrupt,
                        });
                    },
                    4 => {
                        // Local APIC NMI
                        let lapic_nmi = &*(ptr.0 as *const MadtLocalApicNmi);
                        info.local_apic_nmis.push(LocalApicNmiInfo {
                            processor_id: lapic_nmi.processor_id,
                            flags: lapic_nmi.flags,
                            lint: lapic_nmi.lint,
                        });
                    },
                    5 => {
                        // Local APIC Address Override
                        let addr_override = &*(ptr.0 as *const MadtLocalApicAddressOverride);
                        info.local_apic_address = addr_override.local_apic_address;
                    },
                    9 => {
                        // Processor Local x2APIC
                        // TODO: Handle x2APIC
                    },
                    _ => {
                        // Unknown entry type
                    },
                }

                ptr += entry_len as u64;
            }

            Some(info)
        }
    }

    /// Parse MCFG and return PCIe configuration space info
    pub fn parse_mcfg(&self) -> Option<McfgInfo> {
        let mcfg_entry = self.mcfg()?;

        unsafe {
            let header = &*(mcfg_entry.address.0 as *const AcpiSdtHeader);
            let num_entries = (header.length as usize - 44) / 16;

            let mut info = McfgInfo {
                entries: Vec::new(),
            };

            let entries_ptr = (mcfg_entry.address.0 + 44) as *const McfgEntry;

            for i in 0..num_entries {
                let entry = &*entries_ptr.add(i);
                info.entries.push(PcieSegment {
                    base_address: entry.base_address,
                    segment_group: entry.segment_group,
                    start_bus: entry.start_bus,
                    end_bus: entry.end_bus,
                });
            }

            Some(info)
        }
    }
}

impl Protocol for AcpiTables {
    const GUID: Guid = Guid::new(0xEB9D2D30, 0x2D88, 0x11D3, [
        0x9A, 0x16, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4D,
    ]);

    fn open(handle: Handle) -> Result<Self> {
        Ok(Self::new(handle))
    }
}

// =============================================================================
// ACPI TABLE ENTRY
// =============================================================================

/// ACPI table entry
#[derive(Debug, Clone)]
pub struct AcpiTableEntry {
    /// Signature (4 bytes)
    pub signature: [u8; 4],
    /// Physical address
    pub address: PhysicalAddress,
    /// Length
    pub length: u32,
    /// Revision
    pub revision: u8,
}

impl AcpiTableEntry {
    /// Get signature as string
    pub fn signature_str(&self) -> &str {
        core::str::from_utf8(&self.signature).unwrap_or("????")
    }

    /// Get table header
    pub unsafe fn header(&self) -> &AcpiSdtHeader {
        &*(self.address.0 as *const AcpiSdtHeader)
    }

    /// Get table data (after header)
    pub unsafe fn data(&self) -> &[u8] {
        let data_ptr = (self.address.0 + core::mem::size_of::<AcpiSdtHeader>() as u64) as *const u8;
        let data_len = self.length as usize - core::mem::size_of::<AcpiSdtHeader>();
        core::slice::from_raw_parts(data_ptr, data_len)
    }
}

// =============================================================================
// RSDP STRUCTURES
// =============================================================================

/// Root System Description Pointer (ACPI 1.0)
#[repr(C, packed)]
pub struct Rsdp {
    /// "RSD PTR "
    pub signature: [u8; 8],
    /// Checksum
    pub checksum: u8,
    /// OEM ID
    pub oem_id: [u8; 6],
    /// Revision (0 for ACPI 1.0, 2 for ACPI 2.0+)
    pub revision: u8,
    /// RSDT physical address
    pub rsdt_address: u32,
}

impl Rsdp {
    /// Validate checksum
    pub fn validate_checksum(&self) -> bool {
        let bytes = unsafe { core::slice::from_raw_parts(self as *const _ as *const u8, 20) };

        bytes.iter().fold(0u8, |acc, &b| acc.wrapping_add(b)) == 0
    }
}

/// Root System Description Pointer (ACPI 2.0+)
#[repr(C, packed)]
pub struct Rsdp2 {
    /// ACPI 1.0 part
    pub rsdp: Rsdp,
    /// Length of RSDP
    pub length: u32,
    /// XSDT physical address
    pub xsdt_address: u64,
    /// Extended checksum
    pub extended_checksum: u8,
    /// Reserved
    pub reserved: [u8; 3],
}

impl Rsdp2 {
    /// Validate extended checksum
    pub fn validate_extended_checksum(&self) -> bool {
        let bytes = unsafe {
            core::slice::from_raw_parts(self as *const _ as *const u8, self.length as usize)
        };

        bytes.iter().fold(0u8, |acc, &b| acc.wrapping_add(b)) == 0
    }
}

// =============================================================================
// SDT HEADER
// =============================================================================

/// System Description Table header
#[repr(C, packed)]
pub struct AcpiSdtHeader {
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

impl AcpiSdtHeader {
    /// Get signature as string
    pub fn signature_str(&self) -> &str {
        core::str::from_utf8(&self.signature).unwrap_or("????")
    }

    /// Get OEM ID as string
    pub fn oem_id_str(&self) -> &str {
        core::str::from_utf8(&self.oem_id).unwrap_or("??????")
    }

    /// Get OEM table ID as string
    pub fn oem_table_id_str(&self) -> &str {
        core::str::from_utf8(&self.oem_table_id).unwrap_or("????????")
    }

    /// Validate checksum
    pub fn validate_checksum(&self) -> bool {
        let bytes = unsafe {
            core::slice::from_raw_parts(self as *const _ as *const u8, self.length as usize)
        };

        bytes.iter().fold(0u8, |acc, &b| acc.wrapping_add(b)) == 0
    }
}

// =============================================================================
// FADT (Fixed ACPI Description Table)
// =============================================================================

/// Fixed ACPI Description Table
#[repr(C, packed)]
pub struct Fadt {
    /// Header
    pub header: AcpiSdtHeader,
    /// FACS address (32-bit)
    pub facs: u32,
    /// DSDT address (32-bit)
    pub dsdt: u32,
    /// Reserved (was INT_MODEL in ACPI 1.0)
    pub reserved1: u8,
    /// Preferred power management profile
    pub preferred_pm_profile: u8,
    /// SCI interrupt
    pub sci_interrupt: u16,
    /// SMI command port
    pub smi_command: u32,
    /// ACPI enable value
    pub acpi_enable: u8,
    /// ACPI disable value
    pub acpi_disable: u8,
    /// S4BIOS request value
    pub s4bios_req: u8,
    /// P-State control
    pub pstate_control: u8,
    /// PM1a event block
    pub pm1a_event_block: u32,
    /// PM1b event block
    pub pm1b_event_block: u32,
    /// PM1a control block
    pub pm1a_control_block: u32,
    /// PM1b control block
    pub pm1b_control_block: u32,
    /// PM2 control block
    pub pm2_control_block: u32,
    /// PM timer block
    pub pm_timer_block: u32,
    /// GPE0 block
    pub gpe0_block: u32,
    /// GPE1 block
    pub gpe1_block: u32,
    /// PM1 event length
    pub pm1_event_length: u8,
    /// PM1 control length
    pub pm1_control_length: u8,
    /// PM2 control length
    pub pm2_control_length: u8,
    /// PM timer length
    pub pm_timer_length: u8,
    /// GPE0 length
    pub gpe0_length: u8,
    /// GPE1 length
    pub gpe1_length: u8,
    /// GPE1 base
    pub gpe1_base: u8,
    /// C-State control
    pub cstate_control: u8,
    /// Worst C2 latency
    pub worst_c2_latency: u16,
    /// Worst C3 latency
    pub worst_c3_latency: u16,
    /// Flush size
    pub flush_size: u16,
    /// Flush stride
    pub flush_stride: u16,
    /// Duty offset
    pub duty_offset: u8,
    /// Duty width
    pub duty_width: u8,
    /// Day alarm
    pub day_alarm: u8,
    /// Month alarm
    pub month_alarm: u8,
    /// Century
    pub century: u8,
    /// Boot architecture flags (ACPI 2.0+)
    pub boot_architecture_flags: u16,
    /// Reserved
    pub reserved2: u8,
    /// Flags
    pub flags: u32,
    /// Reset register (ACPI 2.0+)
    pub reset_reg: GenericAddressStructure,
    /// Reset value
    pub reset_value: u8,
    /// ARM boot architecture flags (ACPI 5.1+)
    pub arm_boot_arch: u16,
    /// FADT minor version (ACPI 5.1+)
    pub fadt_minor_version: u8,
    /// X_FACS address (64-bit, ACPI 2.0+)
    pub x_facs: u64,
    /// X_DSDT address (64-bit, ACPI 2.0+)
    pub x_dsdt: u64,
    /// Extended PM1a event block
    pub x_pm1a_event_block: GenericAddressStructure,
    /// Extended PM1b event block
    pub x_pm1b_event_block: GenericAddressStructure,
    /// Extended PM1a control block
    pub x_pm1a_control_block: GenericAddressStructure,
    /// Extended PM1b control block
    pub x_pm1b_control_block: GenericAddressStructure,
    /// Extended PM2 control block
    pub x_pm2_control_block: GenericAddressStructure,
    /// Extended PM timer block
    pub x_pm_timer_block: GenericAddressStructure,
    /// Extended GPE0 block
    pub x_gpe0_block: GenericAddressStructure,
    /// Extended GPE1 block
    pub x_gpe1_block: GenericAddressStructure,
}

/// Generic Address Structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct GenericAddressStructure {
    /// Address space ID
    pub address_space: u8,
    /// Register bit width
    pub bit_width: u8,
    /// Register bit offset
    pub bit_offset: u8,
    /// Access size
    pub access_size: u8,
    /// Address
    pub address: u64,
}

impl GenericAddressStructure {
    /// Memory address space
    pub const SPACE_MEMORY: u8 = 0;
    /// I/O address space
    pub const SPACE_IO: u8 = 1;
    /// PCI configuration space
    pub const SPACE_PCI: u8 = 2;
    /// Embedded controller
    pub const SPACE_EMBEDDED: u8 = 3;
    /// SMBus
    pub const SPACE_SMBUS: u8 = 4;
    /// PCC
    pub const SPACE_PCC: u8 = 0x0A;
    /// Functional Fixed Hardware
    pub const SPACE_FFH: u8 = 0x7F;
}

// =============================================================================
// MADT (Multiple APIC Description Table)
// =============================================================================

/// Multiple APIC Description Table
#[repr(C, packed)]
pub struct Madt {
    /// Header
    pub header: AcpiSdtHeader,
    /// Local APIC address
    pub local_apic_address: u32,
    /// Flags
    pub flags: u32,
}

/// MADT Local APIC entry
#[repr(C, packed)]
pub struct MadtLocalApic {
    /// Entry type (0)
    pub entry_type: u8,
    /// Entry length
    pub length: u8,
    /// Processor ID
    pub processor_id: u8,
    /// APIC ID
    pub apic_id: u8,
    /// Flags
    pub flags: u32,
}

/// MADT I/O APIC entry
#[repr(C, packed)]
pub struct MadtIoApic {
    /// Entry type (1)
    pub entry_type: u8,
    /// Entry length
    pub length: u8,
    /// I/O APIC ID
    pub io_apic_id: u8,
    /// Reserved
    pub reserved: u8,
    /// I/O APIC address
    pub io_apic_address: u32,
    /// Global system interrupt base
    pub global_system_interrupt_base: u32,
}

/// MADT Interrupt Source Override entry
#[repr(C, packed)]
pub struct MadtInterruptOverride {
    /// Entry type (2)
    pub entry_type: u8,
    /// Entry length
    pub length: u8,
    /// Bus (always 0 for ISA)
    pub bus: u8,
    /// Source (IRQ number)
    pub source: u8,
    /// Global system interrupt
    pub global_system_interrupt: u32,
    /// Flags
    pub flags: u16,
}

/// MADT NMI Source entry
#[repr(C, packed)]
pub struct MadtNmiSource {
    /// Entry type (3)
    pub entry_type: u8,
    /// Entry length
    pub length: u8,
    /// Flags
    pub flags: u16,
    /// Global system interrupt
    pub global_system_interrupt: u32,
}

/// MADT Local APIC NMI entry
#[repr(C, packed)]
pub struct MadtLocalApicNmi {
    /// Entry type (4)
    pub entry_type: u8,
    /// Entry length
    pub length: u8,
    /// Processor ID (0xFF for all)
    pub processor_id: u8,
    /// Flags
    pub flags: u16,
    /// LINT#
    pub lint: u8,
}

/// MADT Local APIC Address Override entry
#[repr(C, packed)]
pub struct MadtLocalApicAddressOverride {
    /// Entry type (5)
    pub entry_type: u8,
    /// Entry length
    pub length: u8,
    /// Reserved
    pub reserved: u16,
    /// 64-bit Local APIC address
    pub local_apic_address: u64,
}

// =============================================================================
// MCFG (PCI Express Memory Mapped Configuration)
// =============================================================================

/// MCFG entry
#[repr(C, packed)]
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

// =============================================================================
// PARSED STRUCTURES
// =============================================================================

/// Parsed MADT information
#[derive(Debug, Clone)]
pub struct MadtInfo {
    /// Local APIC address
    pub local_apic_address: u64,
    /// Flags
    pub flags: u32,
    /// Local APICs
    pub local_apics: Vec<LocalApicInfo>,
    /// I/O APICs
    pub io_apics: Vec<IoApicInfo>,
    /// Interrupt overrides
    pub interrupt_overrides: Vec<InterruptOverrideInfo>,
    /// NMI sources
    pub nmi_sources: Vec<NmiSourceInfo>,
    /// Local APIC NMIs
    pub local_apic_nmis: Vec<LocalApicNmiInfo>,
}

impl MadtInfo {
    /// Check if PC-AT compatible
    pub fn pcat_compatible(&self) -> bool {
        (self.flags & 1) != 0
    }

    /// Get number of CPUs
    pub fn cpu_count(&self) -> usize {
        self.local_apics.iter().filter(|a| a.is_enabled()).count()
    }

    /// Get enabled APIC IDs
    pub fn enabled_apic_ids(&self) -> Vec<u8> {
        self.local_apics
            .iter()
            .filter(|a| a.is_enabled())
            .map(|a| a.apic_id)
            .collect()
    }
}

/// Local APIC information
#[derive(Debug, Clone)]
pub struct LocalApicInfo {
    /// Processor ID
    pub processor_id: u8,
    /// APIC ID
    pub apic_id: u8,
    /// Flags
    pub flags: u32,
}

impl LocalApicInfo {
    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        (self.flags & 1) != 0
    }

    /// Check if online capable
    pub fn is_online_capable(&self) -> bool {
        (self.flags & 2) != 0
    }
}

/// I/O APIC information
#[derive(Debug, Clone)]
pub struct IoApicInfo {
    /// I/O APIC ID
    pub id: u8,
    /// Address
    pub address: u32,
    /// GSI base
    pub gsi_base: u32,
}

/// Interrupt override information
#[derive(Debug, Clone)]
pub struct InterruptOverrideInfo {
    /// Bus (always 0 for ISA)
    pub bus: u8,
    /// Source IRQ
    pub source: u8,
    /// Global system interrupt
    pub gsi: u32,
    /// Flags
    pub flags: u16,
}

impl InterruptOverrideInfo {
    /// Get polarity
    pub fn polarity(&self) -> Polarity {
        match self.flags & 0x03 {
            0 => Polarity::ConformsBus,
            1 => Polarity::ActiveHigh,
            3 => Polarity::ActiveLow,
            _ => Polarity::Reserved,
        }
    }

    /// Get trigger mode
    pub fn trigger_mode(&self) -> TriggerMode {
        match (self.flags >> 2) & 0x03 {
            0 => TriggerMode::ConformsBus,
            1 => TriggerMode::Edge,
            3 => TriggerMode::Level,
            _ => TriggerMode::Reserved,
        }
    }
}

/// NMI source information
#[derive(Debug, Clone)]
pub struct NmiSourceInfo {
    /// Flags
    pub flags: u16,
    /// Global system interrupt
    pub gsi: u32,
}

/// Local APIC NMI information
#[derive(Debug, Clone)]
pub struct LocalApicNmiInfo {
    /// Processor ID (0xFF for all)
    pub processor_id: u8,
    /// Flags
    pub flags: u16,
    /// LINT#
    pub lint: u8,
}

/// Parsed MCFG information
#[derive(Debug, Clone)]
pub struct McfgInfo {
    /// PCIe segments
    pub entries: Vec<PcieSegment>,
}

/// PCIe segment
#[derive(Debug, Clone)]
pub struct PcieSegment {
    /// Base address
    pub base_address: u64,
    /// Segment group
    pub segment_group: u16,
    /// Start bus
    pub start_bus: u8,
    /// End bus
    pub end_bus: u8,
}

impl PcieSegment {
    /// Get configuration space address for device
    pub fn config_address(&self, bus: u8, device: u8, function: u8, offset: u16) -> u64 {
        self.base_address
            | ((bus as u64 - self.start_bus as u64) << 20)
            | ((device as u64) << 15)
            | ((function as u64) << 12)
            | (offset as u64)
    }
}

/// Polarity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
    /// Conforms to bus specifications
    ConformsBus,
    /// Active high
    ActiveHigh,
    /// Active low
    ActiveLow,
    /// Reserved
    Reserved,
}

/// Trigger mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerMode {
    /// Conforms to bus specifications
    ConformsBus,
    /// Edge triggered
    Edge,
    /// Level triggered
    Level,
    /// Reserved
    Reserved,
}

// =============================================================================
// ACPI CONSTANTS
// =============================================================================

/// ACPI GUIDs
pub mod acpi_guids {
    use super::*;

    /// ACPI 1.0 table GUID
    pub const ACPI_10: Guid = Guid::new(0xEB9D2D30, 0x2D88, 0x11D3, [
        0x9A, 0x16, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4D,
    ]);

    /// ACPI 2.0+ table GUID
    pub const ACPI_20: Guid = Guid::new(0x8868E871, 0xE4F1, 0x11D3, [
        0xBC, 0x22, 0x00, 0x80, 0xC7, 0x3C, 0x88, 0x81,
    ]);
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_apic_flags() {
        let apic = LocalApicInfo {
            processor_id: 0,
            apic_id: 0,
            flags: 0x01,
        };
        assert!(apic.is_enabled());
        assert!(!apic.is_online_capable());
    }

    #[test]
    fn test_interrupt_override() {
        let ovr = InterruptOverrideInfo {
            bus: 0,
            source: 0,
            gsi: 2,
            flags: 0x0F, // Active low, level triggered
        };
        assert_eq!(ovr.polarity(), Polarity::ActiveLow);
        assert_eq!(ovr.trigger_mode(), TriggerMode::Level);
    }

    #[test]
    fn test_pcie_config_address() {
        let segment = PcieSegment {
            base_address: 0xE000_0000,
            segment_group: 0,
            start_bus: 0,
            end_bus: 255,
        };

        // Bus 0, Device 0, Function 0, Offset 0
        assert_eq!(segment.config_address(0, 0, 0, 0), 0xE000_0000);

        // Bus 1, Device 2, Function 3, Offset 0x10
        let addr = segment.config_address(1, 2, 3, 0x10);
        assert_eq!(addr, 0xE000_0000 | (1 << 20) | (2 << 15) | (3 << 12) | 0x10);
    }
}
