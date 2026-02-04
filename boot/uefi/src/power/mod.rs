//! Advanced ACPI and Power Management for Helix UEFI Bootloader
//!
//! This module provides comprehensive ACPI table parsing and power
//! management capabilities for the UEFI boot environment.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                      ACPI Power Management                              │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌──────────────────────────────────────────────────────────────────┐  │
//! │  │                      ACPI Table Parser                           │  │
//! │  │  RSDP │ XSDT │ FADT │ MADT │ MCFG │ HPET │ SRAT │ SLIT │ ...    │  │
//! │  └──────────────────────────────────────────────────────────────────┘  │
//! │                              │                                         │
//! │         ┌────────────────────┼────────────────────┐                    │
//! │         ▼                    ▼                    ▼                    │
//! │  ┌────────────┐     ┌────────────────┐    ┌─────────────┐             │
//! │  │   Power    │     │    Thermal     │    │   Battery   │             │
//! │  │   States   │     │   Management   │    │   Status    │             │
//! │  └────────────┘     └────────────────┘    └─────────────┘             │
//! │         │                    │                    │                    │
//! │         ▼                    ▼                    ▼                    │
//! │  ┌──────────────────────────────────────────────────────────────────┐  │
//! │  │                      Hardware Control                            │  │
//! │  │  CPU P-States │ C-States │ Fan Control │ Sleep States            │  │
//! │  └──────────────────────────────────────────────────────────────────┘  │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Features
//!
//! - Complete ACPI table parsing
//! - Power state management (S0-S5)
//! - CPU power states (C-states, P-states)
//! - Thermal zone handling
//! - Battery information
//! - NUMA topology discovery

#![no_std]

use core::fmt;

// =============================================================================
// ACPI TABLE SIGNATURES
// =============================================================================

/// ACPI table signatures
pub mod signatures {
    /// Root System Description Pointer
    pub const RSDP: [u8; 8] = *b"RSD PTR ";
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
    /// Smart Battery Specification Table
    pub const SBST: [u8; 4] = *b"SBST";
    /// Embedded Controller Boot Resources Table
    pub const ECDT: [u8; 4] = *b"ECDT";
    /// System Locality Distance Information Table
    pub const SLIT: [u8; 4] = *b"SLIT";
    /// System Resource Affinity Table
    pub const SRAT: [u8; 4] = *b"SRAT";
    /// Corrected Platform Error Polling Table
    pub const CPEP: [u8; 4] = *b"CPEP";
    /// Memory-mapped Configuration Space Access Table
    pub const MCFG: [u8; 4] = *b"MCFG";
    /// High Precision Event Timer
    pub const HPET: [u8; 4] = *b"HPET";
    /// Boot Error Record Table
    pub const BERT: [u8; 4] = *b"BERT";
    /// Error Injection Table
    pub const EINJ: [u8; 4] = *b"EINJ";
    /// Error Record Serialization Table
    pub const ERST: [u8; 4] = *b"ERST";
    /// Hardware Error Source Table
    pub const HEST: [u8; 4] = *b"HEST";
    /// Microsoft Data Management Table
    pub const MSDM: [u8; 4] = *b"MSDM";
    /// Trusted Platform Module 2.0 Table
    pub const TPM2: [u8; 4] = *b"TPM2";
    /// Low Power Idle Table
    pub const LPIT: [u8; 4] = *b"LPIT";
    /// Watchdog Action Table
    pub const WDAT: [u8; 4] = *b"WDAT";
    /// Debug Port Table
    pub const DBGP: [u8; 4] = *b"DBGP";
    /// Debug Port Table 2
    pub const DBG2: [u8; 4] = *b"DBG2";
    /// Serial Port Console Redirection
    pub const SPCR: [u8; 4] = *b"SPCR";
    /// Windows ACPI Emulated Devices Table
    pub const WAET: [u8; 4] = *b"WAET";
    /// Windows Security Mitigations Table
    pub const WSMT: [u8; 4] = *b"WSMT";
    /// IPMI Information Table
    pub const SPMI: [u8; 4] = *b"SPMI";
    /// Server Platform Management Interface Table
    pub const SPCR2: [u8; 4] = *b"PCCT";
    /// DMA Remapping Table
    pub const DMAR: [u8; 4] = *b"DMAR";
    /// IVRS (I/O Virtualization Reporting Structure)
    pub const IVRS: [u8; 4] = *b"IVRS";
    /// ARM Performance Monitoring Unit Table
    pub const PPTT: [u8; 4] = *b"PPTT";
    /// Heterogeneous Memory Attribute Table
    pub const HMAT: [u8; 4] = *b"HMAT";
    /// Platform Communications Channel Table
    pub const PCCT: [u8; 4] = *b"PCCT";
    /// Platform Debug Trigger Table
    pub const PDTT: [u8; 4] = *b"PDTT";
    /// Processor Properties Topology Table
    pub const SDEI: [u8; 4] = *b"SDEI";
    /// BGRT (Boot Graphics Resource Table)
    pub const BGRT: [u8; 4] = *b"BGRT";
    /// FPDT (Firmware Performance Data Table)
    pub const FPDT: [u8; 4] = *b"FPDT";
    /// GTDT (Generic Timer Description Table)
    pub const GTDT: [u8; 4] = *b"GTDT";
    /// IORT (I/O Remapping Table)
    pub const IORT: [u8; 4] = *b"IORT";
    /// STAO (Status Override Table)
    pub const STAO: [u8; 4] = *b"STAO";
    /// NFIT (NVDIMM Firmware Interface Table)
    pub const NFIT: [u8; 4] = *b"NFIT";
    /// CSRT (Core System Resource Table)
    pub const CSRT: [u8; 4] = *b"CSRT";
}

// =============================================================================
// RSDP
// =============================================================================

/// ACPI 1.0 RSDP structure
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct RsdpV1 {
    /// "RSD PTR "
    pub signature: [u8; 8],
    /// Checksum
    pub checksum: u8,
    /// OEM ID
    pub oem_id: [u8; 6],
    /// Revision (0 for ACPI 1.0)
    pub revision: u8,
    /// RSDT physical address
    pub rsdt_address: u32,
}

impl RsdpV1 {
    /// Validate checksum
    pub fn validate(&self) -> bool {
        let bytes = unsafe { core::slice::from_raw_parts(self as *const _ as *const u8, 20) };
        bytes.iter().fold(0u8, |acc, &b| acc.wrapping_add(b)) == 0
    }

    /// Check signature
    pub fn is_valid_signature(&self) -> bool {
        self.signature == signatures::RSDP
    }
}

/// ACPI 2.0+ RSDP structure
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct RsdpV2 {
    /// ACPI 1.0 portion
    pub v1: RsdpV1,
    /// Length of RSDP
    pub length: u32,
    /// XSDT physical address
    pub xsdt_address: u64,
    /// Extended checksum
    pub extended_checksum: u8,
    /// Reserved
    pub reserved: [u8; 3],
}

impl RsdpV2 {
    /// Validate extended checksum
    pub fn validate(&self) -> bool {
        let bytes = unsafe {
            core::slice::from_raw_parts(self as *const _ as *const u8, self.length as usize)
        };
        bytes.iter().fold(0u8, |acc, &b| acc.wrapping_add(b)) == 0
    }

    /// Check if ACPI 2.0+
    pub const fn is_v2(&self) -> bool {
        self.v1.revision >= 2
    }
}

// =============================================================================
// COMMON TABLE HEADER
// =============================================================================

/// ACPI SDT header (common to all tables)
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct AcpiSdtHeader {
    /// Table signature
    pub signature: [u8; 4],
    /// Table length
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
    /// Header size
    pub const SIZE: usize = 36;

    /// Validate checksum
    pub fn validate(&self) -> bool {
        let bytes = unsafe {
            core::slice::from_raw_parts(self as *const _ as *const u8, self.length as usize)
        };
        bytes.iter().fold(0u8, |acc, &b| acc.wrapping_add(b)) == 0
    }

    /// Get signature as string
    pub fn signature_str(&self) -> &str {
        core::str::from_utf8(&self.signature).unwrap_or("????")
    }

    /// Get OEM ID as string
    pub fn oem_id_str(&self) -> &str {
        let end = self.oem_id.iter().position(|&b| b == 0).unwrap_or(6);
        core::str::from_utf8(&self.oem_id[..end]).unwrap_or("")
    }
}

// =============================================================================
// FADT (Fixed ACPI Description Table)
// =============================================================================

/// FADT flags
pub mod fadt_flags {
    pub const WBINVD: u32 = 1 << 0;
    pub const WBINVD_FLUSH: u32 = 1 << 1;
    pub const PROC_C1: u32 = 1 << 2;
    pub const P_LVL2_UP: u32 = 1 << 3;
    pub const PWR_BUTTON: u32 = 1 << 4;
    pub const SLP_BUTTON: u32 = 1 << 5;
    pub const FIX_RTC: u32 = 1 << 6;
    pub const RTC_S4: u32 = 1 << 7;
    pub const TMR_VAL_EXT: u32 = 1 << 8;
    pub const DCK_CAP: u32 = 1 << 9;
    pub const RESET_REG_SUP: u32 = 1 << 10;
    pub const SEALED_CASE: u32 = 1 << 11;
    pub const HEADLESS: u32 = 1 << 12;
    pub const CPU_SW_SLP: u32 = 1 << 13;
    pub const PCI_EXP_WAK: u32 = 1 << 14;
    pub const USE_PLATFORM_CLOCK: u32 = 1 << 15;
    pub const S4_RTC_STS_VALID: u32 = 1 << 16;
    pub const REMOTE_POWER_ON_CAPABLE: u32 = 1 << 17;
    pub const FORCE_APIC_CLUSTER_MODEL: u32 = 1 << 18;
    pub const FORCE_APIC_PHYSICAL_DEST_MODE: u32 = 1 << 19;
    pub const HW_REDUCED_ACPI: u32 = 1 << 20;
    pub const LOW_POWER_S0_IDLE_CAPABLE: u32 = 1 << 21;
}

/// Generic Address Structure
#[derive(Debug, Clone, Copy, Default)]
#[repr(C, packed)]
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
    /// System memory space
    pub const SPACE_SYSTEM_MEMORY: u8 = 0x00;
    /// System I/O space
    pub const SPACE_SYSTEM_IO: u8 = 0x01;
    /// PCI configuration space
    pub const SPACE_PCI_CONFIG: u8 = 0x02;
    /// Embedded controller
    pub const SPACE_EMBEDDED_CONTROLLER: u8 = 0x03;
    /// SMBus
    pub const SPACE_SMBUS: u8 = 0x04;
    /// Platform communications channel
    pub const SPACE_PCC: u8 = 0x0A;
    /// Functional fixed hardware
    pub const SPACE_FFH: u8 = 0x7F;

    /// Check if valid
    pub const fn is_valid(&self) -> bool {
        self.address != 0
    }
}

/// FADT (Fixed ACPI Description Table)
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Fadt {
    /// Header
    pub header: AcpiSdtHeader,
    /// FACS address (32-bit)
    pub facs_address: u32,
    /// DSDT address (32-bit)
    pub dsdt_address: u32,
    /// Reserved (ACPI 1.0 INT_MODEL)
    pub reserved: u8,
    /// Preferred power management profile
    pub preferred_pm_profile: u8,
    /// SCI interrupt
    pub sci_interrupt: u16,
    /// SMI command port
    pub smi_command: u32,
    /// ACPI enable command
    pub acpi_enable: u8,
    /// ACPI disable command
    pub acpi_disable: u8,
    /// S4BIOS request
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
    /// GPE0 block length
    pub gpe0_block_length: u8,
    /// GPE1 block length
    pub gpe1_block_length: u8,
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
    /// RTC day alarm
    pub day_alarm: u8,
    /// RTC month alarm
    pub month_alarm: u8,
    /// RTC century
    pub century: u8,
    /// Boot architecture flags
    pub boot_architecture_flags: u16,
    /// Reserved
    pub reserved2: u8,
    /// Flags
    pub flags: u32,
    /// Reset register
    pub reset_reg: GenericAddressStructure,
    /// Reset value
    pub reset_value: u8,
    /// ARM boot architecture flags
    pub arm_boot_flags: u16,
    /// FADT minor version
    pub fadt_minor_version: u8,
    /// FACS address (64-bit)
    pub x_facs_address: u64,
    /// DSDT address (64-bit)
    pub x_dsdt_address: u64,
    /// PM1a event block (64-bit)
    pub x_pm1a_event_block: GenericAddressStructure,
    /// PM1b event block (64-bit)
    pub x_pm1b_event_block: GenericAddressStructure,
    /// PM1a control block (64-bit)
    pub x_pm1a_control_block: GenericAddressStructure,
    /// PM1b control block (64-bit)
    pub x_pm1b_control_block: GenericAddressStructure,
    /// PM2 control block (64-bit)
    pub x_pm2_control_block: GenericAddressStructure,
    /// PM timer block (64-bit)
    pub x_pm_timer_block: GenericAddressStructure,
    /// GPE0 block (64-bit)
    pub x_gpe0_block: GenericAddressStructure,
    /// GPE1 block (64-bit)
    pub x_gpe1_block: GenericAddressStructure,
    /// Sleep control register
    pub sleep_control_reg: GenericAddressStructure,
    /// Sleep status register
    pub sleep_status_reg: GenericAddressStructure,
    /// Hypervisor vendor ID
    pub hypervisor_vendor_id: u64,
}

impl Fadt {
    /// Get DSDT address (prefer 64-bit if available)
    pub const fn dsdt_addr(&self) -> u64 {
        if self.x_dsdt_address != 0 {
            self.x_dsdt_address
        } else {
            self.dsdt_address as u64
        }
    }

    /// Get FACS address (prefer 64-bit if available)
    pub const fn facs_addr(&self) -> u64 {
        if self.x_facs_address != 0 {
            self.x_facs_address
        } else {
            self.facs_address as u64
        }
    }

    /// Check if hardware reduced ACPI
    pub const fn is_hw_reduced(&self) -> bool {
        (self.flags & fadt_flags::HW_REDUCED_ACPI) != 0
    }

    /// Check if low power S0 idle capable
    pub const fn is_low_power_s0_idle(&self) -> bool {
        (self.flags & fadt_flags::LOW_POWER_S0_IDLE_CAPABLE) != 0
    }

    /// Get PM profile
    pub const fn pm_profile(&self) -> PmProfile {
        match self.preferred_pm_profile {
            0 => PmProfile::Unspecified,
            1 => PmProfile::Desktop,
            2 => PmProfile::Mobile,
            3 => PmProfile::Workstation,
            4 => PmProfile::EnterpriseServer,
            5 => PmProfile::SohoServer,
            6 => PmProfile::AppliancePc,
            7 => PmProfile::PerformanceServer,
            8 => PmProfile::Tablet,
            _ => PmProfile::Unspecified,
        }
    }
}

/// PM Profile
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PmProfile {
    /// Unspecified
    Unspecified,
    /// Desktop
    Desktop,
    /// Mobile
    Mobile,
    /// Workstation
    Workstation,
    /// Enterprise Server
    EnterpriseServer,
    /// SOHO Server
    SohoServer,
    /// Appliance PC
    AppliancePc,
    /// Performance Server
    PerformanceServer,
    /// Tablet
    Tablet,
}

// =============================================================================
// MADT (Multiple APIC Description Table)
// =============================================================================

/// MADT entry types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MadtEntryType {
    /// Processor Local APIC
    LocalApic            = 0,
    /// I/O APIC
    IoApic               = 1,
    /// Interrupt Source Override
    InterruptSourceOverride = 2,
    /// NMI Source
    NmiSource            = 3,
    /// Local APIC NMI
    LocalApicNmi         = 4,
    /// Local APIC Address Override
    LocalApicAddressOverride = 5,
    /// I/O SAPIC
    IoSapic              = 6,
    /// Local SAPIC
    LocalSapic           = 7,
    /// Platform Interrupt Sources
    PlatformInterruptSources = 8,
    /// Processor Local x2APIC
    LocalX2Apic          = 9,
    /// Local x2APIC NMI
    LocalX2ApicNmi       = 10,
    /// GIC CPU Interface
    GicCpuInterface      = 11,
    /// GIC Distributor
    GicDistributor       = 12,
    /// GIC MSI Frame
    GicMsiFrame          = 13,
    /// GIC Redistributor
    GicRedistributor     = 14,
    /// GIC ITS
    GicIts               = 15,
    /// Multiprocessor Wakeup
    MultiprocessorWakeup = 16,
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

/// MADT header
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Madt {
    /// Common header
    pub header: AcpiSdtHeader,
    /// Local APIC address
    pub local_apic_address: u32,
    /// Flags
    pub flags: u32,
}

impl Madt {
    /// PC-AT compatible dual-8259 setup
    pub const FLAG_PCAT_COMPAT: u32 = 1 << 0;

    /// Check if dual-8259 compatible
    pub const fn has_8259(&self) -> bool {
        (self.flags & Self::FLAG_PCAT_COMPAT) != 0
    }
}

/// Local APIC entry
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct MadtLocalApic {
    /// Entry header
    pub header: MadtEntryHeader,
    /// ACPI processor UID
    pub acpi_processor_uid: u8,
    /// APIC ID
    pub apic_id: u8,
    /// Flags
    pub flags: u32,
}

impl MadtLocalApic {
    /// Processor is usable
    pub const FLAG_ENABLED: u32 = 1 << 0;
    /// Online capable
    pub const FLAG_ONLINE_CAPABLE: u32 = 1 << 1;

    /// Check if enabled
    pub const fn is_enabled(&self) -> bool {
        (self.flags & Self::FLAG_ENABLED) != 0
    }

    /// Check if online capable
    pub const fn is_online_capable(&self) -> bool {
        (self.flags & Self::FLAG_ONLINE_CAPABLE) != 0
    }
}

/// I/O APIC entry
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct MadtIoApic {
    /// Entry header
    pub header: MadtEntryHeader,
    /// I/O APIC ID
    pub io_apic_id: u8,
    /// Reserved
    pub reserved: u8,
    /// I/O APIC address
    pub io_apic_address: u32,
    /// Global System Interrupt base
    pub gsi_base: u32,
}

/// Interrupt Source Override entry
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct MadtInterruptSourceOverride {
    /// Entry header
    pub header: MadtEntryHeader,
    /// Bus (0 = ISA)
    pub bus: u8,
    /// Source IRQ
    pub source: u8,
    /// Global System Interrupt
    pub gsi: u32,
    /// Flags
    pub flags: u16,
}

impl MadtInterruptSourceOverride {
    /// Get polarity
    pub const fn polarity(&self) -> Polarity {
        match self.flags & 0x03 {
            0 => Polarity::Conforming,
            1 => Polarity::ActiveHigh,
            3 => Polarity::ActiveLow,
            _ => Polarity::Conforming,
        }
    }

    /// Get trigger mode
    pub const fn trigger_mode(&self) -> TriggerMode {
        match (self.flags >> 2) & 0x03 {
            0 => TriggerMode::Conforming,
            1 => TriggerMode::Edge,
            3 => TriggerMode::Level,
            _ => TriggerMode::Conforming,
        }
    }
}

/// Interrupt polarity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
    /// Conforms to bus specifications
    Conforming,
    /// Active high
    ActiveHigh,
    /// Active low
    ActiveLow,
}

/// Interrupt trigger mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerMode {
    /// Conforms to bus specifications
    Conforming,
    /// Edge triggered
    Edge,
    /// Level triggered
    Level,
}

/// Local x2APIC entry
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct MadtLocalX2Apic {
    /// Entry header
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

// =============================================================================
// MCFG (Memory-mapped Configuration Space)
// =============================================================================

/// MCFG entry
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct McfgEntry {
    /// Base address
    pub base_address: u64,
    /// PCI segment group number
    pub segment_group: u16,
    /// Start bus number
    pub start_bus: u8,
    /// End bus number
    pub end_bus: u8,
    /// Reserved
    pub reserved: u32,
}

impl McfgEntry {
    /// Calculate config space address for a device
    pub const fn config_address(&self, bus: u8, device: u8, function: u8, offset: u16) -> u64 {
        self.base_address
            + ((bus as u64 - self.start_bus as u64) << 20)
            + ((device as u64) << 15)
            + ((function as u64) << 12)
            + (offset as u64)
    }
}

// =============================================================================
// HPET (High Precision Event Timer)
// =============================================================================

/// HPET table
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Hpet {
    /// Common header
    pub header: AcpiSdtHeader,
    /// Event timer block ID
    pub event_timer_block_id: u32,
    /// Base address
    pub base_address: GenericAddressStructure,
    /// HPET sequence number
    pub hpet_number: u8,
    /// Minimum clock tick
    pub minimum_tick: u16,
    /// Page protection and OEM attributes
    pub page_protection: u8,
}

impl Hpet {
    /// Get hardware revision
    pub const fn revision(&self) -> u8 {
        (self.event_timer_block_id & 0xFF) as u8
    }

    /// Get number of comparators
    pub const fn num_comparators(&self) -> u8 {
        (((self.event_timer_block_id >> 8) & 0x1F) + 1) as u8
    }

    /// Get counter size (true = 64-bit)
    pub const fn is_64bit(&self) -> bool {
        (self.event_timer_block_id >> 13) & 1 != 0
    }

    /// Check if legacy replacement capable
    pub const fn legacy_capable(&self) -> bool {
        (self.event_timer_block_id >> 15) & 1 != 0
    }

    /// Get vendor ID
    pub const fn vendor_id(&self) -> u16 {
        (self.event_timer_block_id >> 16) as u16
    }
}

// =============================================================================
// SRAT (System Resource Affinity Table)
// =============================================================================

/// SRAT entry types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SratEntryType {
    /// Processor Local APIC/SAPIC Affinity
    LocalApicAffinity   = 0,
    /// Memory Affinity
    MemoryAffinity      = 1,
    /// Processor Local x2APIC Affinity
    LocalX2ApicAffinity = 2,
    /// GICC Affinity
    GiccAffinity        = 3,
    /// GIC ITS Affinity
    GicItsAffinity      = 4,
    /// Generic Initiator Affinity
    GenericInitiatorAffinity = 5,
}

/// Memory affinity entry
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct SratMemoryAffinity {
    /// Type (1)
    pub entry_type: u8,
    /// Length
    pub length: u8,
    /// Proximity domain
    pub proximity_domain: u32,
    /// Reserved
    pub reserved: u16,
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
    /// Entry is enabled
    pub const FLAG_ENABLED: u32 = 1 << 0;
    /// Hot pluggable
    pub const FLAG_HOT_PLUGGABLE: u32 = 1 << 1;
    /// Non-volatile
    pub const FLAG_NON_VOLATILE: u32 = 1 << 2;

    /// Get base address
    pub const fn base_address(&self) -> u64 {
        (self.base_address_high as u64) << 32 | self.base_address_low as u64
    }

    /// Get length
    pub const fn length(&self) -> u64 {
        (self.length_high as u64) << 32 | self.length_low as u64
    }

    /// Check if enabled
    pub const fn is_enabled(&self) -> bool {
        (self.flags & Self::FLAG_ENABLED) != 0
    }

    /// Check if hot pluggable
    pub const fn is_hot_pluggable(&self) -> bool {
        (self.flags & Self::FLAG_HOT_PLUGGABLE) != 0
    }

    /// Check if non-volatile
    pub const fn is_non_volatile(&self) -> bool {
        (self.flags & Self::FLAG_NON_VOLATILE) != 0
    }
}

// =============================================================================
// POWER STATES
// =============================================================================

/// System power state (S-states)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SleepState {
    /// S0 - Working
    S0,
    /// S1 - Sleep (CPU cache maintained)
    S1,
    /// S2 - Sleep (CPU context lost)
    S2,
    /// S3 - Suspend to RAM
    S3,
    /// S4 - Suspend to disk (hibernate)
    S4,
    /// S5 - Soft off
    S5,
}

impl SleepState {
    /// Get sleep type for PM1a/PM1b control
    pub const fn sleep_type_a(&self) -> u8 {
        match self {
            SleepState::S0 => 0,
            SleepState::S1 => 1,
            SleepState::S2 => 2,
            SleepState::S3 => 3,
            SleepState::S4 => 4,
            SleepState::S5 => 5,
        }
    }

    /// Get state name
    pub const fn name(&self) -> &'static str {
        match self {
            SleepState::S0 => "S0 (Working)",
            SleepState::S1 => "S1 (Sleep)",
            SleepState::S2 => "S2 (Sleep)",
            SleepState::S3 => "S3 (Suspend to RAM)",
            SleepState::S4 => "S4 (Hibernate)",
            SleepState::S5 => "S5 (Soft Off)",
        }
    }
}

/// CPU C-state (processor power state)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CState {
    /// C0 - Active
    C0,
    /// C1 - Halt
    C1,
    /// C1E - Enhanced halt
    C1E,
    /// C2 - Stop clock
    C2,
    /// C3 - Deep sleep
    C3,
    /// C6 - Deep power down
    C6,
    /// C7 - Deeper power down
    C7,
    /// C8 - Deepest power down
    C8,
    /// C9 - Ultra deep
    C9,
    /// C10 - Package level
    C10,
}

impl CState {
    /// Get exit latency in microseconds (typical)
    pub const fn exit_latency_us(&self) -> u32 {
        match self {
            CState::C0 => 0,
            CState::C1 | CState::C1E => 1,
            CState::C2 => 10,
            CState::C3 => 100,
            CState::C6 => 200,
            CState::C7 => 300,
            CState::C8 => 400,
            CState::C9 => 500,
            CState::C10 => 1000,
        }
    }
}

/// CPU P-state (performance state)
#[derive(Debug, Clone, Copy)]
pub struct PState {
    /// Frequency in MHz
    pub frequency_mhz: u32,
    /// Voltage in mV
    pub voltage_mv: u32,
    /// Power in mW
    pub power_mw: u32,
    /// Transition latency in us
    pub latency_us: u32,
}

impl PState {
    /// Create new P-state
    pub const fn new(frequency_mhz: u32, voltage_mv: u32, power_mw: u32) -> Self {
        Self {
            frequency_mhz,
            voltage_mv,
            power_mw,
            latency_us: 10,
        }
    }
}

// =============================================================================
// THERMAL
// =============================================================================

/// Thermal zone information
#[derive(Debug, Clone, Copy)]
pub struct ThermalZone {
    /// Zone ID
    pub id: u32,
    /// Current temperature (tenths of Kelvin)
    pub temperature: u32,
    /// Critical trip point (tenths of Kelvin)
    pub critical: u32,
    /// Hot trip point (tenths of Kelvin)
    pub hot: u32,
    /// Passive cooling trip point
    pub passive: u32,
    /// Active cooling trip points
    pub active: [u32; 5],
}

impl ThermalZone {
    /// Convert from tenths of Kelvin to Celsius
    pub const fn kelvin_to_celsius(tenths_k: u32) -> i32 {
        (tenths_k as i32 - 2732) / 10
    }

    /// Get temperature in Celsius
    pub fn temperature_celsius(&self) -> i32 {
        Self::kelvin_to_celsius(self.temperature)
    }

    /// Check if temperature is critical
    pub const fn is_critical(&self) -> bool {
        self.temperature >= self.critical
    }

    /// Check if temperature is hot
    pub const fn is_hot(&self) -> bool {
        self.temperature >= self.hot
    }
}

// =============================================================================
// BATTERY
// =============================================================================

/// Battery status
#[derive(Debug, Clone, Copy)]
pub struct BatteryStatus {
    /// Battery present
    pub present: bool,
    /// Is charging
    pub charging: bool,
    /// Is discharging
    pub discharging: bool,
    /// Is critical
    pub critical: bool,
    /// Charge percentage (0-100)
    pub charge_percent: u8,
    /// Remaining capacity in mWh
    pub remaining_mwh: u32,
    /// Full charge capacity in mWh
    pub full_mwh: u32,
    /// Design capacity in mWh
    pub design_mwh: u32,
    /// Current rate in mW
    pub current_rate_mw: u32,
    /// Voltage in mV
    pub voltage_mv: u32,
    /// Time to empty in minutes
    pub time_to_empty_min: u32,
    /// Time to full in minutes
    pub time_to_full_min: u32,
}

impl BatteryStatus {
    /// Create new empty status
    pub const fn new() -> Self {
        Self {
            present: false,
            charging: false,
            discharging: false,
            critical: false,
            charge_percent: 0,
            remaining_mwh: 0,
            full_mwh: 0,
            design_mwh: 0,
            current_rate_mw: 0,
            voltage_mv: 0,
            time_to_empty_min: 0,
            time_to_full_min: 0,
        }
    }

    /// Calculate health percentage
    pub fn health_percent(&self) -> u8 {
        if self.design_mwh == 0 {
            100
        } else {
            ((self.full_mwh * 100) / self.design_mwh).min(100) as u8
        }
    }
}

impl Default for BatteryStatus {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// ERROR TYPES
// =============================================================================

/// ACPI error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcpiError {
    /// RSDP not found
    RsdpNotFound,
    /// Invalid RSDP checksum
    InvalidRsdpChecksum,
    /// Table not found
    TableNotFound,
    /// Invalid table checksum
    InvalidTableChecksum,
    /// Invalid table signature
    InvalidTableSignature,
    /// Unsupported revision
    UnsupportedRevision,
    /// Parse error
    ParseError,
    /// Entry not found
    EntryNotFound,
    /// Buffer too small
    BufferTooSmall,
}

impl fmt::Display for AcpiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AcpiError::RsdpNotFound => write!(f, "RSDP not found"),
            AcpiError::InvalidRsdpChecksum => write!(f, "Invalid RSDP checksum"),
            AcpiError::TableNotFound => write!(f, "Table not found"),
            AcpiError::InvalidTableChecksum => write!(f, "Invalid table checksum"),
            AcpiError::InvalidTableSignature => write!(f, "Invalid table signature"),
            AcpiError::UnsupportedRevision => write!(f, "Unsupported revision"),
            AcpiError::ParseError => write!(f, "Parse error"),
            AcpiError::EntryNotFound => write!(f, "Entry not found"),
            AcpiError::BufferTooSmall => write!(f, "Buffer too small"),
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
    fn test_sleep_state() {
        assert_eq!(SleepState::S3.name(), "S3 (Suspend to RAM)");
        assert_eq!(SleepState::S3.sleep_type_a(), 3);
    }

    #[test]
    fn test_cstate_latency() {
        assert_eq!(CState::C0.exit_latency_us(), 0);
        assert!(CState::C3.exit_latency_us() > CState::C1.exit_latency_us());
    }

    #[test]
    fn test_thermal_conversion() {
        // 25°C = 298.2K = 2982 tenths of Kelvin
        let temp = 2982u32;
        let celsius = ThermalZone::kelvin_to_celsius(temp);
        assert!(celsius >= 24 && celsius <= 26);
    }

    #[test]
    fn test_battery_health() {
        let battery = BatteryStatus {
            present: true,
            design_mwh: 50000,
            full_mwh: 45000,
            ..Default::default()
        };
        assert_eq!(battery.health_percent(), 90);
    }

    #[test]
    fn test_mcfg_address() {
        let entry = McfgEntry {
            base_address: 0xE000_0000,
            segment_group: 0,
            start_bus: 0,
            end_bus: 255,
            reserved: 0,
        };
        // Bus 0, Device 0, Function 0, Offset 0
        assert_eq!(entry.config_address(0, 0, 0, 0), 0xE000_0000);
        // Bus 1, Device 0, Function 0, Offset 0
        assert_eq!(entry.config_address(1, 0, 0, 0), 0xE010_0000);
    }
}
