//! Advanced ACPI Table Parser
//!
//! Comprehensive ACPI table parsing for system configuration discovery.

use crate::error::{Error, Result};
use crate::raw::types::*;

extern crate alloc;
use alloc::vec::Vec;
use core::ptr;

// =============================================================================
// ACPI PARSER
// =============================================================================

/// Advanced ACPI parser
pub struct AcpiParser {
    /// RSDP address
    rsdp_address: PhysicalAddress,
    /// Revision (0 for ACPI 1.0, 2+ for ACPI 2.0+)
    revision: u8,
    /// RSDT address
    rsdt_address: Option<PhysicalAddress>,
    /// XSDT address
    xsdt_address: Option<PhysicalAddress>,
    /// Parsed tables
    tables: Vec<ParsedTable>,
    /// FADT info
    fadt: Option<FadtInfo>,
    /// MADT info
    madt: Option<MadtInfo>,
    /// HPET info
    hpet: Option<HpetInfo>,
    /// MCFG info
    mcfg: Option<McfgInfo>,
    /// BGRT info
    bgrt: Option<BgrtInfo>,
}

impl AcpiParser {
    /// Create new ACPI parser
    pub fn new() -> Self {
        Self {
            rsdp_address: PhysicalAddress(0),
            revision: 0,
            rsdt_address: None,
            xsdt_address: None,
            tables: Vec::new(),
            fadt: None,
            madt: None,
            hpet: None,
            mcfg: None,
            bgrt: None,
        }
    }

    /// Initialize from RSDP address
    pub unsafe fn init(&mut self, rsdp_address: PhysicalAddress) -> Result<()> {
        self.rsdp_address = rsdp_address;

        // Check ACPI 1.0 RSDP first
        let rsdp = &*(rsdp_address.0 as *const Rsdp);

        // Validate signature
        if &rsdp.signature != b"RSD PTR " {
            return Err(Error::InvalidParameter);
        }

        // Validate checksum
        if !validate_checksum(rsdp_address.0 as *const u8, 20) {
            return Err(Error::CrcError);
        }

        self.revision = rsdp.revision;
        self.rsdt_address = Some(PhysicalAddress(rsdp.rsdt_address as u64));

        // Check for ACPI 2.0+ RSDP
        if rsdp.revision >= 2 {
            let rsdp2 = &*(rsdp_address.0 as *const Rsdp2);

            // Validate extended checksum
            if !validate_checksum(rsdp_address.0 as *const u8, rsdp2.length as usize) {
                return Err(Error::CrcError);
            }

            if rsdp2.xsdt_address != 0 {
                self.xsdt_address = Some(PhysicalAddress(rsdp2.xsdt_address));
            }
        }

        // Parse root table
        self.parse_root_table()?;

        // Parse important tables
        self.parse_fadt()?;
        self.parse_madt()?;
        self.parse_hpet()?;
        self.parse_mcfg()?;
        self.parse_bgrt()?;

        Ok(())
    }

    /// Parse root table (XSDT or RSDT)
    unsafe fn parse_root_table(&mut self) -> Result<()> {
        if let Some(xsdt_addr) = self.xsdt_address {
            self.parse_xsdt(xsdt_addr)?;
        } else if let Some(rsdt_addr) = self.rsdt_address {
            self.parse_rsdt(rsdt_addr)?;
        }
        Ok(())
    }

    /// Parse RSDT (32-bit pointers)
    unsafe fn parse_rsdt(&mut self, addr: PhysicalAddress) -> Result<()> {
        let header = &*(addr.0 as *const SdtHeader);

        if &header.signature != b"RSDT" {
            return Err(Error::InvalidParameter);
        }

        if !validate_checksum(addr.0 as *const u8, header.length as usize) {
            return Err(Error::CrcError);
        }

        let entry_count = (header.length as usize - core::mem::size_of::<SdtHeader>()) / 4;
        let entries_ptr = (addr.0 as usize + core::mem::size_of::<SdtHeader>()) as *const u32;

        for i in 0..entry_count {
            let table_addr = PhysicalAddress(ptr::read_unaligned(entries_ptr.add(i)) as u64);
            if table_addr.0 != 0 {
                self.add_table(table_addr)?;
            }
        }

        Ok(())
    }

    /// Parse XSDT (64-bit pointers)
    unsafe fn parse_xsdt(&mut self, addr: PhysicalAddress) -> Result<()> {
        let header = &*(addr.0 as *const SdtHeader);

        if &header.signature != b"XSDT" {
            return Err(Error::InvalidParameter);
        }

        if !validate_checksum(addr.0 as *const u8, header.length as usize) {
            return Err(Error::CrcError);
        }

        let entry_count = (header.length as usize - core::mem::size_of::<SdtHeader>()) / 8;
        let entries_ptr = (addr.0 as usize + core::mem::size_of::<SdtHeader>()) as *const u64;

        for i in 0..entry_count {
            let table_addr = ptr::read_unaligned(entries_ptr.add(i));
            if table_addr != 0 {
                self.add_table(PhysicalAddress(table_addr))?;
            }
        }

        Ok(())
    }

    /// Add table to list
    unsafe fn add_table(&mut self, addr: PhysicalAddress) -> Result<()> {
        let header = &*(addr.0 as *const SdtHeader);

        self.tables.push(ParsedTable {
            signature: header.signature,
            address: addr,
            length: header.length,
            revision: header.revision,
            oem_id: header.oem_id,
            oem_table_id: header.oem_table_id,
        });

        Ok(())
    }

    /// Find table by signature
    pub fn find_table(&self, signature: &[u8; 4]) -> Option<&ParsedTable> {
        self.tables.iter().find(|t| &t.signature == signature)
    }

    /// Get all tables
    pub fn tables(&self) -> &[ParsedTable] {
        &self.tables
    }

    /// Parse FADT
    unsafe fn parse_fadt(&mut self) -> Result<()> {
        let fadt_entry = match self.find_table(b"FACP") {
            Some(e) => e.clone(),
            None => return Ok(()),
        };

        let fadt = &*(fadt_entry.address.0 as *const Fadt);

        self.fadt = Some(FadtInfo {
            sci_interrupt: fadt.sci_interrupt,
            smi_command: fadt.smi_command,
            acpi_enable: fadt.acpi_enable,
            acpi_disable: fadt.acpi_disable,
            pm1a_event_block: fadt.pm1a_event_block,
            pm1b_event_block: fadt.pm1b_event_block,
            pm1a_control_block: fadt.pm1a_control_block,
            pm1b_control_block: fadt.pm1b_control_block,
            pm2_control_block: fadt.pm2_control_block,
            pm_timer_block: fadt.pm_timer_block,
            gpe0_block: fadt.gpe0_block,
            gpe1_block: fadt.gpe1_block,
            pm_timer_length: fadt.pm_timer_length,
            flags: fadt.flags,
            reset_register: if fadt_entry.length >= 129 {
                Some(fadt.reset_reg)
            } else {
                None
            },
            reset_value: if fadt_entry.length >= 129 {
                fadt.reset_value
            } else {
                0
            },
            dsdt_address: if fadt_entry.length >= 148 && fadt.x_dsdt != 0 {
                fadt.x_dsdt
            } else {
                fadt.dsdt as u64
            },
            facs_address: if fadt_entry.length >= 140 && fadt.x_facs != 0 {
                fadt.x_facs
            } else {
                fadt.facs as u64
            },
            century_register: fadt.century,
            boot_flags: if fadt_entry.length >= 113 {
                fadt.boot_architecture_flags
            } else {
                0
            },
            hypervisor_vendor_id: None, // TODO: Parse from extended fields
        });

        Ok(())
    }

    /// Parse MADT
    unsafe fn parse_madt(&mut self) -> Result<()> {
        let madt_entry = match self.find_table(b"APIC") {
            Some(e) => e.clone(),
            None => return Ok(()),
        };

        let madt = &*(madt_entry.address.0 as *const Madt);

        let mut info = MadtInfo {
            local_apic_address: madt.local_apic_address as u64,
            flags: madt.flags,
            local_apics: Vec::new(),
            io_apics: Vec::new(),
            overrides: Vec::new(),
            nmis: Vec::new(),
            local_apic_nmis: Vec::new(),
            local_x2apics: Vec::new(),
        };

        // Parse entries
        let end = madt_entry.address + madt_entry.length as u64;
        let mut ptr = madt_entry.address + core::mem::size_of::<Madt>() as u64;

        while ptr + 2 <= end {
            let entry_type = *(ptr.0 as *const u8);
            let entry_len = *((ptr + 1).0 as *const u8);

            if entry_len < 2 || ptr + entry_len as u64 > end {
                break;
            }

            match entry_type {
                0 => {
                    // Local APIC
                    if entry_len >= 8 {
                        let entry = &*(ptr.0 as *const MadtLocalApicEntry);
                        info.local_apics.push(LocalApic {
                            processor_uid: entry.processor_id as u32,
                            apic_id: entry.apic_id as u32,
                            flags: entry.flags,
                        });
                    }
                },
                1 => {
                    // I/O APIC
                    if entry_len >= 12 {
                        let entry = &*(ptr.0 as *const MadtIoApicEntry);
                        info.io_apics.push(IoApic {
                            id: entry.io_apic_id,
                            address: entry.io_apic_address,
                            gsi_base: entry.gsi_base,
                        });
                    }
                },
                2 => {
                    // Interrupt Source Override
                    if entry_len >= 10 {
                        let entry = &*(ptr.0 as *const MadtOverrideEntry);
                        info.overrides.push(InterruptOverride {
                            bus: entry.bus,
                            source: entry.source,
                            gsi: entry.gsi,
                            flags: entry.flags,
                        });
                    }
                },
                3 => {
                    // NMI Source
                    if entry_len >= 8 {
                        let entry = &*(ptr.0 as *const MadtNmiEntry);
                        info.nmis.push(NmiSource {
                            flags: entry.flags,
                            gsi: entry.gsi,
                        });
                    }
                },
                4 => {
                    // Local APIC NMI
                    if entry_len >= 6 {
                        let entry = &*(ptr.0 as *const MadtLocalNmiEntry);
                        info.local_apic_nmis.push(LocalApicNmi {
                            processor_uid: entry.processor_id as u32,
                            flags: entry.flags,
                            lint: entry.lint,
                        });
                    }
                },
                5 => {
                    // Local APIC Address Override
                    if entry_len >= 12 {
                        let entry = &*(ptr.0 as *const MadtAddressOverrideEntry);
                        info.local_apic_address = entry.local_apic_address;
                    }
                },
                9 => {
                    // Local x2APIC
                    if entry_len >= 16 {
                        let entry = &*(ptr.0 as *const MadtX2ApicEntry);
                        info.local_x2apics.push(LocalX2Apic {
                            processor_uid: entry.processor_uid,
                            x2apic_id: entry.x2apic_id,
                            flags: entry.flags,
                        });
                    }
                },
                _ => {},
            }

            ptr += entry_len as u64;
        }

        self.madt = Some(info);
        Ok(())
    }

    /// Parse HPET
    unsafe fn parse_hpet(&mut self) -> Result<()> {
        let hpet_entry = match self.find_table(b"HPET") {
            Some(e) => e.clone(),
            None => return Ok(()),
        };

        let hpet = &*(hpet_entry.address.0 as *const HpetTable);

        self.hpet = Some(HpetInfo {
            hardware_rev_id: (hpet.event_timer_block_id & 0xFF) as u8,
            comparator_count: ((hpet.event_timer_block_id >> 8) & 0x1F) as u8,
            counter_size: ((hpet.event_timer_block_id >> 13) & 1) != 0,
            legacy_replacement: ((hpet.event_timer_block_id >> 15) & 1) != 0,
            pci_vendor_id: ((hpet.event_timer_block_id >> 16) & 0xFFFF) as u16,
            address: hpet.base_address.address,
            hpet_number: hpet.hpet_number,
            minimum_tick: hpet.minimum_tick,
            page_protection: hpet.page_protection,
        });

        Ok(())
    }

    /// Parse MCFG
    unsafe fn parse_mcfg(&mut self) -> Result<()> {
        let mcfg_entry = match self.find_table(b"MCFG") {
            Some(e) => e.clone(),
            None => return Ok(()),
        };

        let header = &*(mcfg_entry.address.0 as *const SdtHeader);
        let entry_count = (header.length as usize - 44) / 16;

        let mut segments = Vec::new();
        let entries_ptr = (mcfg_entry.address + 44).0 as *const McfgEntry;

        for i in 0..entry_count {
            let entry = &*entries_ptr.add(i);
            segments.push(PcieSegment {
                base_address: entry.base_address,
                segment_group: entry.segment_group,
                start_bus: entry.start_bus,
                end_bus: entry.end_bus,
            });
        }

        self.mcfg = Some(McfgInfo { segments });
        Ok(())
    }

    /// Parse BGRT (Boot Graphics Resource Table)
    unsafe fn parse_bgrt(&mut self) -> Result<()> {
        let bgrt_entry = match self.find_table(b"BGRT") {
            Some(e) => e.clone(),
            None => return Ok(()),
        };

        let bgrt = &*(bgrt_entry.address.0 as *const BgrtTable);

        self.bgrt = Some(BgrtInfo {
            version: bgrt.version,
            status: bgrt.status,
            image_type: bgrt.image_type,
            image_address: bgrt.image_address,
            image_offset_x: bgrt.image_offset_x,
            image_offset_y: bgrt.image_offset_y,
        });

        Ok(())
    }

    /// Get FADT info
    pub fn fadt(&self) -> Option<&FadtInfo> {
        self.fadt.as_ref()
    }

    /// Get MADT info
    pub fn madt(&self) -> Option<&MadtInfo> {
        self.madt.as_ref()
    }

    /// Get HPET info
    pub fn hpet(&self) -> Option<&HpetInfo> {
        self.hpet.as_ref()
    }

    /// Get MCFG info
    pub fn mcfg(&self) -> Option<&McfgInfo> {
        self.mcfg.as_ref()
    }

    /// Get BGRT info
    pub fn bgrt(&self) -> Option<&BgrtInfo> {
        self.bgrt.as_ref()
    }

    /// Get CPU count from MADT
    pub fn cpu_count(&self) -> usize {
        self.madt.as_ref().map_or(0, |m| {
            let lapic_count = m.local_apics.iter().filter(|a| (a.flags & 1) != 0).count();
            let x2apic_count = m
                .local_x2apics
                .iter()
                .filter(|a| (a.flags & 1) != 0)
                .count();
            lapic_count.max(x2apic_count)
        })
    }

    /// Get I/O APIC count
    pub fn io_apic_count(&self) -> usize {
        self.madt.as_ref().map_or(0, |m| m.io_apics.len())
    }

    /// Check if HPET is available
    pub fn has_hpet(&self) -> bool {
        self.hpet.is_some()
    }

    /// Get ACPI version from FADT revision
    pub fn version(&self) -> super::AcpiVersion {
        match self.revision {
            0 => super::AcpiVersion::V1_0,
            1 => super::AcpiVersion::V2_0,
            2 => super::AcpiVersion::V3_0,
            3 => super::AcpiVersion::V4_0,
            4 => super::AcpiVersion::V5_0,
            5 => super::AcpiVersion::V5_1,
            6 => super::AcpiVersion::V6_0,
            _ => super::AcpiVersion::new(6, self.revision - 5),
        }
    }
}

impl Default for AcpiParser {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Validate checksum
unsafe fn validate_checksum(ptr: *const u8, len: usize) -> bool {
    let mut sum = 0u8;
    for i in 0..len {
        sum = sum.wrapping_add(*ptr.add(i));
    }
    sum == 0
}

// =============================================================================
// RAW STRUCTURES
// =============================================================================

/// RSDP (ACPI 1.0)
#[repr(C, packed)]
struct Rsdp {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_address: u32,
}

/// RSDP (ACPI 2.0+)
#[repr(C, packed)]
struct Rsdp2 {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_address: u32,
    length: u32,
    xsdt_address: u64,
    extended_checksum: u8,
    reserved: [u8; 3],
}

/// SDT Header
#[repr(C, packed)]
struct SdtHeader {
    signature: [u8; 4],
    length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
}

/// FADT
#[repr(C, packed)]
struct Fadt {
    header: SdtHeader,
    facs: u32,
    dsdt: u32,
    reserved1: u8,
    preferred_pm_profile: u8,
    sci_interrupt: u16,
    smi_command: u32,
    acpi_enable: u8,
    acpi_disable: u8,
    s4bios_req: u8,
    pstate_control: u8,
    pm1a_event_block: u32,
    pm1b_event_block: u32,
    pm1a_control_block: u32,
    pm1b_control_block: u32,
    pm2_control_block: u32,
    pm_timer_block: u32,
    gpe0_block: u32,
    gpe1_block: u32,
    pm1_event_length: u8,
    pm1_control_length: u8,
    pm2_control_length: u8,
    pm_timer_length: u8,
    gpe0_length: u8,
    gpe1_length: u8,
    gpe1_base: u8,
    cstate_control: u8,
    worst_c2_latency: u16,
    worst_c3_latency: u16,
    flush_size: u16,
    flush_stride: u16,
    duty_offset: u8,
    duty_width: u8,
    day_alarm: u8,
    month_alarm: u8,
    century: u8,
    boot_architecture_flags: u16,
    reserved2: u8,
    flags: u32,
    reset_reg: GenericAddress,
    reset_value: u8,
    arm_boot_arch: u16,
    fadt_minor_version: u8,
    x_facs: u64,
    x_dsdt: u64,
    // Extended fields continue...
}

/// Generic Address Structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct GenericAddress {
    /// Address space
    pub address_space: u8,
    /// Bit width
    pub bit_width: u8,
    /// Bit offset
    pub bit_offset: u8,
    /// Access size
    pub access_size: u8,
    /// Address
    pub address: u64,
}

/// MADT
#[repr(C, packed)]
struct Madt {
    header: SdtHeader,
    local_apic_address: u32,
    flags: u32,
}

/// MADT Local APIC entry
#[repr(C, packed)]
struct MadtLocalApicEntry {
    entry_type: u8,
    length: u8,
    processor_id: u8,
    apic_id: u8,
    flags: u32,
}

/// MADT I/O APIC entry
#[repr(C, packed)]
struct MadtIoApicEntry {
    entry_type: u8,
    length: u8,
    io_apic_id: u8,
    reserved: u8,
    io_apic_address: u32,
    gsi_base: u32,
}

/// MADT Interrupt Override entry
#[repr(C, packed)]
struct MadtOverrideEntry {
    entry_type: u8,
    length: u8,
    bus: u8,
    source: u8,
    gsi: u32,
    flags: u16,
}

/// MADT NMI entry
#[repr(C, packed)]
struct MadtNmiEntry {
    entry_type: u8,
    length: u8,
    flags: u16,
    gsi: u32,
}

/// MADT Local APIC NMI entry
#[repr(C, packed)]
struct MadtLocalNmiEntry {
    entry_type: u8,
    length: u8,
    processor_id: u8,
    flags: u16,
    lint: u8,
}

/// MADT Local APIC Address Override entry
#[repr(C, packed)]
struct MadtAddressOverrideEntry {
    entry_type: u8,
    length: u8,
    reserved: u16,
    local_apic_address: u64,
}

/// MADT x2APIC entry
#[repr(C, packed)]
struct MadtX2ApicEntry {
    entry_type: u8,
    length: u8,
    reserved: u16,
    x2apic_id: u32,
    flags: u32,
    processor_uid: u32,
}

/// HPET table
#[repr(C, packed)]
struct HpetTable {
    header: SdtHeader,
    event_timer_block_id: u32,
    base_address: GenericAddress,
    hpet_number: u8,
    minimum_tick: u16,
    page_protection: u8,
}

/// MCFG entry
#[repr(C, packed)]
struct McfgEntry {
    base_address: u64,
    segment_group: u16,
    start_bus: u8,
    end_bus: u8,
    reserved: u32,
}

/// BGRT table
#[repr(C, packed)]
struct BgrtTable {
    header: SdtHeader,
    version: u16,
    status: u8,
    image_type: u8,
    image_address: u64,
    image_offset_x: u32,
    image_offset_y: u32,
}

// =============================================================================
// PARSED STRUCTURES
// =============================================================================

/// Parsed table info
#[derive(Debug, Clone)]
pub struct ParsedTable {
    /// Signature
    pub signature: [u8; 4],
    /// Address
    pub address: PhysicalAddress,
    /// Length
    pub length: u32,
    /// Revision
    pub revision: u8,
    /// OEM ID
    pub oem_id: [u8; 6],
    /// OEM Table ID
    pub oem_table_id: [u8; 8],
}

impl ParsedTable {
    /// Get signature as string
    pub fn signature_str(&self) -> &str {
        core::str::from_utf8(&self.signature).unwrap_or("????")
    }

    /// Get OEM ID as string
    pub fn oem_id_str(&self) -> &str {
        core::str::from_utf8(&self.oem_id)
            .unwrap_or("??????")
            .trim()
    }
}

/// FADT parsed info
#[derive(Debug, Clone)]
pub struct FadtInfo {
    /// SCI interrupt
    pub sci_interrupt: u16,
    /// SMI command port
    pub smi_command: u32,
    /// ACPI enable command
    pub acpi_enable: u8,
    /// ACPI disable command
    pub acpi_disable: u8,
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
    /// PM timer length
    pub pm_timer_length: u8,
    /// Flags
    pub flags: u32,
    /// Reset register
    pub reset_register: Option<GenericAddress>,
    /// Reset value
    pub reset_value: u8,
    /// DSDT address
    pub dsdt_address: u64,
    /// FACS address
    pub facs_address: u64,
    /// Century register
    pub century_register: u8,
    /// Boot architecture flags
    pub boot_flags: u16,
    /// Hypervisor vendor ID
    pub hypervisor_vendor_id: Option<u64>,
}

impl FadtInfo {
    /// Check if WBINVD instruction is supported
    pub fn wbinvd(&self) -> bool {
        (self.flags & (1 << 0)) != 0
    }

    /// Check if WBINVD flushes caches
    pub fn wbinvd_flush(&self) -> bool {
        (self.flags & (1 << 1)) != 0
    }

    /// Check if C1 power state is supported
    pub fn c1_supported(&self) -> bool {
        (self.flags & (1 << 2)) != 0
    }

    /// Check if C2 power state works on multiple processors
    pub fn c2_mp(&self) -> bool {
        (self.flags & (1 << 3)) != 0
    }

    /// Check if power button is a control method device
    pub fn power_button_is_method(&self) -> bool {
        (self.flags & (1 << 4)) != 0
    }

    /// Check if sleep button is a control method device
    pub fn sleep_button_is_method(&self) -> bool {
        (self.flags & (1 << 5)) != 0
    }

    /// Check if RTC is a control method device
    pub fn rtc_is_method(&self) -> bool {
        (self.flags & (1 << 6)) != 0
    }

    /// Check if RTC can wake from S4
    pub fn rtc_s4(&self) -> bool {
        (self.flags & (1 << 7)) != 0
    }

    /// Check if PM timer is 32-bit
    pub fn timer_32bit(&self) -> bool {
        (self.flags & (1 << 8)) != 0
    }

    /// Check if docking is supported
    pub fn docking_supported(&self) -> bool {
        (self.flags & (1 << 9)) != 0
    }

    /// Check if reset register is supported
    pub fn reset_supported(&self) -> bool {
        (self.flags & (1 << 10)) != 0
    }

    /// Check if sealed case
    pub fn sealed_case(&self) -> bool {
        (self.flags & (1 << 11)) != 0
    }

    /// Check if headless
    pub fn headless(&self) -> bool {
        (self.flags & (1 << 12)) != 0
    }

    /// Check if 8042 keyboard controller
    pub fn has_8042(&self) -> bool {
        (self.boot_flags & 1) != 0
    }

    /// Check if VGA not present
    pub fn no_vga(&self) -> bool {
        (self.boot_flags & 2) != 0
    }

    /// Check if MSI not supported
    pub fn no_msi(&self) -> bool {
        (self.boot_flags & 4) != 0
    }

    /// Check if PCIe ASPM controls
    pub fn pcie_aspm_controls(&self) -> bool {
        (self.boot_flags & 8) != 0
    }
}

/// MADT parsed info
#[derive(Debug, Clone)]
pub struct MadtInfo {
    /// Local APIC address
    pub local_apic_address: u64,
    /// Flags
    pub flags: u32,
    /// Local APICs
    pub local_apics: Vec<LocalApic>,
    /// I/O APICs
    pub io_apics: Vec<IoApic>,
    /// Interrupt overrides
    pub overrides: Vec<InterruptOverride>,
    /// NMI sources
    pub nmis: Vec<NmiSource>,
    /// Local APIC NMIs
    pub local_apic_nmis: Vec<LocalApicNmi>,
    /// Local x2APICs
    pub local_x2apics: Vec<LocalX2Apic>,
}

impl MadtInfo {
    /// Check if PC-AT compatible
    pub fn pcat_compatible(&self) -> bool {
        (self.flags & 1) != 0
    }

    /// Get enabled CPU count
    pub fn enabled_cpu_count(&self) -> usize {
        let lapic = self.local_apics.iter().filter(|a| a.is_enabled()).count();
        let x2apic = self.local_x2apics.iter().filter(|a| a.is_enabled()).count();
        lapic.max(x2apic)
    }

    /// Get BSP (Bootstrap Processor) APIC ID
    pub fn bsp_apic_id(&self) -> Option<u32> {
        self.local_apics
            .first()
            .filter(|a| a.is_enabled())
            .map(|a| a.apic_id)
            .or_else(|| {
                self.local_x2apics
                    .first()
                    .filter(|a| a.is_enabled())
                    .map(|a| a.x2apic_id)
            })
    }
}

/// Local APIC info
#[derive(Debug, Clone)]
pub struct LocalApic {
    /// Processor UID
    pub processor_uid: u32,
    /// APIC ID
    pub apic_id: u32,
    /// Flags
    pub flags: u32,
}

impl LocalApic {
    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        (self.flags & 1) != 0
    }

    /// Check if online capable
    pub fn is_online_capable(&self) -> bool {
        (self.flags & 2) != 0
    }
}

/// Local x2APIC info
#[derive(Debug, Clone)]
pub struct LocalX2Apic {
    /// Processor UID
    pub processor_uid: u32,
    /// x2APIC ID
    pub x2apic_id: u32,
    /// Flags
    pub flags: u32,
}

impl LocalX2Apic {
    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        (self.flags & 1) != 0
    }
}

/// I/O APIC info
#[derive(Debug, Clone)]
pub struct IoApic {
    /// I/O APIC ID
    pub id: u8,
    /// Address
    pub address: u32,
    /// GSI base
    pub gsi_base: u32,
}

/// Interrupt override info
#[derive(Debug, Clone)]
pub struct InterruptOverride {
    /// Bus
    pub bus: u8,
    /// Source IRQ
    pub source: u8,
    /// Global System Interrupt
    pub gsi: u32,
    /// Flags
    pub flags: u16,
}

impl InterruptOverride {
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
    pub fn trigger(&self) -> Trigger {
        match (self.flags >> 2) & 0x03 {
            0 => Trigger::ConformsBus,
            1 => Trigger::Edge,
            3 => Trigger::Level,
            _ => Trigger::Reserved,
        }
    }
}

/// Polarity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
    ConformsBus,
    ActiveHigh,
    ActiveLow,
    Reserved,
}

/// Trigger mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    ConformsBus,
    Edge,
    Level,
    Reserved,
}

/// NMI source info
#[derive(Debug, Clone)]
pub struct NmiSource {
    /// Flags
    pub flags: u16,
    /// Global System Interrupt
    pub gsi: u32,
}

/// Local APIC NMI info
#[derive(Debug, Clone)]
pub struct LocalApicNmi {
    /// Processor UID
    pub processor_uid: u32,
    /// Flags
    pub flags: u16,
    /// LINT#
    pub lint: u8,
}

/// HPET info
#[derive(Debug, Clone)]
pub struct HpetInfo {
    /// Hardware revision ID
    pub hardware_rev_id: u8,
    /// Number of comparators
    pub comparator_count: u8,
    /// Counter size (true = 64-bit)
    pub counter_size: bool,
    /// Legacy replacement capable
    pub legacy_replacement: bool,
    /// PCI vendor ID
    pub pci_vendor_id: u16,
    /// Base address
    pub address: u64,
    /// HPET number
    pub hpet_number: u8,
    /// Minimum tick
    pub minimum_tick: u16,
    /// Page protection
    pub page_protection: u8,
}

/// MCFG info
#[derive(Debug, Clone)]
pub struct McfgInfo {
    /// PCIe segments
    pub segments: Vec<PcieSegment>,
}

/// PCIe segment info
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
    /// Get config space address
    pub fn config_address(&self, bus: u8, device: u8, function: u8, offset: u16) -> u64 {
        self.base_address
            | ((bus as u64 - self.start_bus as u64) << 20)
            | ((device as u64) << 15)
            | ((function as u64) << 12)
            | (offset as u64)
    }
}

/// BGRT info (Boot Graphics Resource Table)
#[derive(Debug, Clone)]
pub struct BgrtInfo {
    /// Version
    pub version: u16,
    /// Status
    pub status: u8,
    /// Image type (0 = BMP)
    pub image_type: u8,
    /// Image address
    pub image_address: u64,
    /// Image X offset
    pub image_offset_x: u32,
    /// Image Y offset
    pub image_offset_y: u32,
}

impl BgrtInfo {
    /// Check if image is displayed
    pub fn is_displayed(&self) -> bool {
        (self.status & 1) != 0
    }

    /// Check if image is BMP
    pub fn is_bmp(&self) -> bool {
        self.image_type == 0
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_apic() {
        let apic = LocalApic {
            processor_uid: 0,
            apic_id: 0,
            flags: 0x01,
        };
        assert!(apic.is_enabled());
        assert!(!apic.is_online_capable());
    }

    #[test]
    fn test_interrupt_override() {
        let ovr = InterruptOverride {
            bus: 0,
            source: 0,
            gsi: 2,
            flags: 0x0F,
        };
        assert_eq!(ovr.polarity(), Polarity::ActiveLow);
        assert_eq!(ovr.trigger(), Trigger::Level);
    }

    #[test]
    fn test_pcie_segment() {
        let seg = PcieSegment {
            base_address: 0xE000_0000,
            segment_group: 0,
            start_bus: 0,
            end_bus: 255,
        };
        assert_eq!(seg.config_address(0, 0, 0, 0), 0xE000_0000);
    }
}
