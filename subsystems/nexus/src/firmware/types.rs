//! Firmware Core Types
//!
//! Fundamental types for firmware intelligence.

use alloc::string::String;

/// Firmware type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirmwareType {
    /// UEFI firmware
    Uefi,
    /// Legacy BIOS
    LegacyBios,
    /// Coreboot
    Coreboot,
    /// U-Boot
    UBoot,
    /// Open Firmware (OpenBoot)
    OpenFirmware,
    /// ARM Trusted Firmware
    ArmTrustedFirmware,
    /// Unknown firmware type
    Unknown,
}

impl FirmwareType {
    /// Check if firmware supports runtime services
    pub fn has_runtime_services(&self) -> bool {
        matches!(self, Self::Uefi)
    }

    /// Check if firmware supports ACPI
    pub fn supports_acpi(&self) -> bool {
        matches!(self, Self::Uefi | Self::LegacyBios | Self::Coreboot)
    }

    /// Check if firmware supports device tree
    pub fn supports_device_tree(&self) -> bool {
        matches!(self, Self::UBoot | Self::OpenFirmware | Self::ArmTrustedFirmware)
    }
}

/// ACPI table signature
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AcpiSignature(pub [u8; 4]);

impl AcpiSignature {
    /// RSDP signature
    pub const RSDP: Self = Self(*b"RSD ");
    /// RSDT signature
    pub const RSDT: Self = Self(*b"RSDT");
    /// XSDT signature
    pub const XSDT: Self = Self(*b"XSDT");
    /// FADT signature
    pub const FADT: Self = Self(*b"FACP");
    /// MADT signature
    pub const MADT: Self = Self(*b"APIC");
    /// MCFG signature
    pub const MCFG: Self = Self(*b"MCFG");
    /// HPET signature
    pub const HPET: Self = Self(*b"HPET");
    /// SRAT signature
    pub const SRAT: Self = Self(*b"SRAT");
    /// SLIT signature
    pub const SLIT: Self = Self(*b"SLIT");
    /// DSDT signature
    pub const DSDT: Self = Self(*b"DSDT");
    /// SSDT signature
    pub const SSDT: Self = Self(*b"SSDT");
    /// BGRT signature
    pub const BGRT: Self = Self(*b"BGRT");
    /// DMAR signature
    pub const DMAR: Self = Self(*b"DMAR");

    /// Create from bytes
    pub const fn from_bytes(bytes: [u8; 4]) -> Self {
        Self(bytes)
    }

    /// Get as string
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.0).unwrap_or("????")
    }
}

/// ACPI revision
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AcpiRevision {
    /// Major version
    pub major: u8,
    /// Minor version
    pub minor: u8,
}

impl AcpiRevision {
    /// ACPI 1.0
    pub const V1_0: Self = Self { major: 1, minor: 0 };
    /// ACPI 2.0
    pub const V2_0: Self = Self { major: 2, minor: 0 };
    /// ACPI 3.0
    pub const V3_0: Self = Self { major: 3, minor: 0 };
    /// ACPI 4.0
    pub const V4_0: Self = Self { major: 4, minor: 0 };
    /// ACPI 5.0
    pub const V5_0: Self = Self { major: 5, minor: 0 };
    /// ACPI 6.0
    pub const V6_0: Self = Self { major: 6, minor: 0 };

    /// Create new revision
    pub const fn new(major: u8, minor: u8) -> Self {
        Self { major, minor }
    }
}

/// ACPI table information
#[derive(Debug, Clone)]
pub struct AcpiTableInfo {
    /// Table signature
    pub signature: AcpiSignature,
    /// Physical address
    pub address: u64,
    /// Table length
    pub length: u32,
    /// Revision
    pub revision: u8,
    /// Checksum valid
    pub checksum_valid: bool,
    /// OEM ID
    pub oem_id: String,
    /// OEM table ID
    pub oem_table_id: String,
    /// OEM revision
    pub oem_revision: u32,
}

impl AcpiTableInfo {
    /// Create new table info
    pub fn new(signature: AcpiSignature, address: u64, length: u32) -> Self {
        Self {
            signature,
            address,
            length,
            revision: 0,
            checksum_valid: false,
            oem_id: String::new(),
            oem_table_id: String::new(),
            oem_revision: 0,
        }
    }
}
