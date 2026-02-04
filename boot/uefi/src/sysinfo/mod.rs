//! System Information and Diagnostics
//!
//! This module provides comprehensive system information gathering,
//! diagnostic capabilities, and hardware/software reporting.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                    System Information                                   │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────┐            │
//! │  │  CPU      │  │  Memory   │  │  Storage  │  │  Firmware │            │
//! │  │  Info     │  │  Map      │  │  Devices  │  │  Info     │            │
//! │  └───────────┘  └───────────┘  └───────────┘  └───────────┘            │
//! │                                                                         │
//! │  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────┐            │
//! │  │  Graphics │  │  Network  │  │  Security │  │  Boot     │            │
//! │  │  Info     │  │  Status   │  │  State    │  │  Status   │            │
//! │  └───────────┘  └───────────┘  └───────────┘  └───────────┘            │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

#![no_std]

use core::fmt;

// =============================================================================
// SYSTEM SUMMARY
// =============================================================================

/// System summary
#[derive(Debug, Clone, Copy)]
pub struct SystemSummary {
    /// CPU info
    pub cpu: CpuSummary,
    /// Memory info
    pub memory: MemorySummary,
    /// Storage info
    pub storage: StorageSummary,
    /// Firmware info
    pub firmware: FirmwareSummary,
    /// Graphics info
    pub graphics: GraphicsSummary,
    /// Boot info
    pub boot: BootSummary,
    /// System timestamp
    pub timestamp_us: u64,
}

impl Default for SystemSummary {
    fn default() -> Self {
        Self {
            cpu: CpuSummary::default(),
            memory: MemorySummary::default(),
            storage: StorageSummary::default(),
            firmware: FirmwareSummary::default(),
            graphics: GraphicsSummary::default(),
            boot: BootSummary::default(),
            timestamp_us: 0,
        }
    }
}

// =============================================================================
// CPU INFORMATION
// =============================================================================

/// CPU vendor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CpuVendor {
    /// Unknown vendor
    #[default]
    Unknown,
    /// Intel
    Intel,
    /// AMD
    Amd,
    /// ARM
    Arm,
    /// Apple
    Apple,
    /// Qualcomm
    Qualcomm,
    /// Other vendor
    Other,
}

impl fmt::Display for CpuVendor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CpuVendor::Unknown => write!(f, "Unknown"),
            CpuVendor::Intel => write!(f, "Intel"),
            CpuVendor::Amd => write!(f, "AMD"),
            CpuVendor::Arm => write!(f, "ARM"),
            CpuVendor::Apple => write!(f, "Apple"),
            CpuVendor::Qualcomm => write!(f, "Qualcomm"),
            CpuVendor::Other => write!(f, "Other"),
        }
    }
}

/// CPU architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CpuArch {
    /// Unknown architecture
    #[default]
    Unknown,
    /// x86 (32-bit)
    X86,
    /// x86-64 (64-bit)
    X86_64,
    /// ARM (32-bit)
    Arm32,
    /// ARM (64-bit)
    Arm64,
    /// RISC-V (32-bit)
    Riscv32,
    /// RISC-V (64-bit)
    Riscv64,
}

impl fmt::Display for CpuArch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CpuArch::Unknown => write!(f, "Unknown"),
            CpuArch::X86 => write!(f, "x86"),
            CpuArch::X86_64 => write!(f, "x86-64"),
            CpuArch::Arm32 => write!(f, "ARM32"),
            CpuArch::Arm64 => write!(f, "ARM64"),
            CpuArch::Riscv32 => write!(f, "RISC-V 32"),
            CpuArch::Riscv64 => write!(f, "RISC-V 64"),
        }
    }
}

/// CPU feature flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CpuFeatures(u64);

impl CpuFeatures {
    pub const NONE: CpuFeatures = CpuFeatures(0);
    pub const FPU: CpuFeatures = CpuFeatures(1 << 0);
    pub const SSE: CpuFeatures = CpuFeatures(1 << 1);
    pub const SSE2: CpuFeatures = CpuFeatures(1 << 2);
    pub const SSE3: CpuFeatures = CpuFeatures(1 << 3);
    pub const SSSE3: CpuFeatures = CpuFeatures(1 << 4);
    pub const SSE4_1: CpuFeatures = CpuFeatures(1 << 5);
    pub const SSE4_2: CpuFeatures = CpuFeatures(1 << 6);
    pub const AVX: CpuFeatures = CpuFeatures(1 << 7);
    pub const AVX2: CpuFeatures = CpuFeatures(1 << 8);
    pub const AVX512F: CpuFeatures = CpuFeatures(1 << 9);
    pub const AES_NI: CpuFeatures = CpuFeatures(1 << 10);
    pub const SHA: CpuFeatures = CpuFeatures(1 << 11);
    pub const RDRAND: CpuFeatures = CpuFeatures(1 << 12);
    pub const RDSEED: CpuFeatures = CpuFeatures(1 << 13);
    pub const TSC: CpuFeatures = CpuFeatures(1 << 14);
    pub const INVARIANT_TSC: CpuFeatures = CpuFeatures(1 << 15);
    pub const APIC: CpuFeatures = CpuFeatures(1 << 16);
    pub const X2APIC: CpuFeatures = CpuFeatures(1 << 17);
    pub const NX: CpuFeatures = CpuFeatures(1 << 18);
    pub const SMEP: CpuFeatures = CpuFeatures(1 << 19);
    pub const SMAP: CpuFeatures = CpuFeatures(1 << 20);
    pub const VMX: CpuFeatures = CpuFeatures(1 << 21);
    pub const SVM: CpuFeatures = CpuFeatures(1 << 22);
    pub const XSAVE: CpuFeatures = CpuFeatures(1 << 23);
    pub const LONG_MODE: CpuFeatures = CpuFeatures(1 << 24);
    pub const PAE: CpuFeatures = CpuFeatures(1 << 25);
    pub const HYPERVISOR: CpuFeatures = CpuFeatures(1 << 26);

    /// Get raw value
    pub const fn raw(&self) -> u64 {
        self.0
    }

    /// Check feature
    pub const fn has(&self, feature: CpuFeatures) -> bool {
        self.0 & feature.0 != 0
    }

    /// Set feature
    pub fn set(&mut self, feature: CpuFeatures) {
        self.0 |= feature.0;
    }

    /// Count features
    pub fn count(&self) -> u32 {
        self.0.count_ones()
    }
}

/// CPU summary
#[derive(Debug, Clone, Copy)]
pub struct CpuSummary {
    /// Vendor
    pub vendor: CpuVendor,
    /// Architecture
    pub arch: CpuArch,
    /// Brand string
    pub brand: [u8; 48],
    /// Brand length
    pub brand_len: usize,
    /// Family
    pub family: u8,
    /// Model
    pub model: u8,
    /// Stepping
    pub stepping: u8,
    /// Logical processors
    pub logical_cpus: u16,
    /// Physical cores
    pub physical_cores: u16,
    /// Packages/sockets
    pub packages: u8,
    /// Base frequency (MHz)
    pub base_freq_mhz: u32,
    /// Max frequency (MHz)
    pub max_freq_mhz: u32,
    /// Current frequency (MHz)
    pub current_freq_mhz: u32,
    /// Features
    pub features: CpuFeatures,
    /// L1 data cache (KB)
    pub l1d_cache_kb: u16,
    /// L1 instruction cache (KB)
    pub l1i_cache_kb: u16,
    /// L2 cache (KB)
    pub l2_cache_kb: u32,
    /// L3 cache (KB)
    pub l3_cache_kb: u32,
}

impl CpuSummary {
    /// Get brand as string
    pub fn brand_str(&self) -> &str {
        if self.brand_len > 0 {
            core::str::from_utf8(&self.brand[..self.brand_len]).unwrap_or("")
        } else {
            ""
        }
    }
}

impl Default for CpuSummary {
    fn default() -> Self {
        Self {
            vendor: CpuVendor::default(),
            arch: CpuArch::default(),
            brand: [0u8; 48],
            brand_len: 0,
            family: 0,
            model: 0,
            stepping: 0,
            logical_cpus: 0,
            physical_cores: 0,
            packages: 0,
            base_freq_mhz: 0,
            max_freq_mhz: 0,
            current_freq_mhz: 0,
            features: CpuFeatures::default(),
            l1d_cache_kb: 0,
            l1i_cache_kb: 0,
            l2_cache_kb: 0,
            l3_cache_kb: 0,
        }
    }
}

// =============================================================================
// MEMORY INFORMATION
// =============================================================================

/// Memory type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MemoryType {
    /// Unknown
    #[default]
    Unknown,
    /// DDR
    Ddr,
    /// DDR2
    Ddr2,
    /// DDR3
    Ddr3,
    /// DDR4
    Ddr4,
    /// DDR5
    Ddr5,
    /// LPDDR3
    Lpddr3,
    /// LPDDR4
    Lpddr4,
    /// LPDDR5
    Lpddr5,
}

impl fmt::Display for MemoryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryType::Unknown => write!(f, "Unknown"),
            MemoryType::Ddr => write!(f, "DDR"),
            MemoryType::Ddr2 => write!(f, "DDR2"),
            MemoryType::Ddr3 => write!(f, "DDR3"),
            MemoryType::Ddr4 => write!(f, "DDR4"),
            MemoryType::Ddr5 => write!(f, "DDR5"),
            MemoryType::Lpddr3 => write!(f, "LPDDR3"),
            MemoryType::Lpddr4 => write!(f, "LPDDR4"),
            MemoryType::Lpddr5 => write!(f, "LPDDR5"),
        }
    }
}

/// Memory region info
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryRegionInfo {
    /// Start address
    pub start: u64,
    /// Size
    pub size: u64,
    /// Type
    pub memory_type: MemoryRegionType,
    /// Usable
    pub usable: bool,
}

/// Memory region type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MemoryRegionType {
    /// Available
    #[default]
    Available,
    /// Reserved
    Reserved,
    /// ACPI reclaimable
    AcpiReclaimable,
    /// ACPI NVS
    AcpiNvs,
    /// Unusable
    Unusable,
    /// Disabled
    Disabled,
    /// Persistent
    Persistent,
}

/// Memory summary
#[derive(Debug, Clone, Copy, Default)]
pub struct MemorySummary {
    /// Total physical memory (bytes)
    pub total_physical: u64,
    /// Available memory (bytes)
    pub available: u64,
    /// Reserved memory (bytes)
    pub reserved: u64,
    /// ACPI memory (bytes)
    pub acpi: u64,
    /// Memory map entries
    pub map_entries: u32,
    /// Highest usable address
    pub highest_address: u64,
    /// Memory type
    pub memory_type: MemoryType,
    /// Memory speed (MHz)
    pub speed_mhz: u16,
    /// Number of DIMMs
    pub dimm_count: u8,
    /// Channels
    pub channels: u8,
}

impl MemorySummary {
    /// Get total in MB
    pub const fn total_mb(&self) -> u64 {
        self.total_physical / (1024 * 1024)
    }

    /// Get total in GB
    pub const fn total_gb(&self) -> u64 {
        self.total_physical / (1024 * 1024 * 1024)
    }

    /// Get available in MB
    pub const fn available_mb(&self) -> u64 {
        self.available / (1024 * 1024)
    }
}

// =============================================================================
// STORAGE INFORMATION
// =============================================================================

/// Storage device type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StorageType {
    /// Unknown
    #[default]
    Unknown,
    /// Hard disk
    Hdd,
    /// Solid state drive
    Ssd,
    /// NVMe SSD
    Nvme,
    /// USB flash drive
    UsbFlash,
    /// Memory card
    MemoryCard,
    /// CD/DVD
    Optical,
    /// Network storage
    Network,
    /// RAM disk
    RamDisk,
}

impl fmt::Display for StorageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageType::Unknown => write!(f, "Unknown"),
            StorageType::Hdd => write!(f, "HDD"),
            StorageType::Ssd => write!(f, "SSD"),
            StorageType::Nvme => write!(f, "NVMe"),
            StorageType::UsbFlash => write!(f, "USB"),
            StorageType::MemoryCard => write!(f, "Card"),
            StorageType::Optical => write!(f, "Optical"),
            StorageType::Network => write!(f, "Network"),
            StorageType::RamDisk => write!(f, "RAM"),
        }
    }
}

/// Storage device info
#[derive(Debug, Clone, Copy)]
pub struct StorageDeviceInfo {
    /// Device type
    pub device_type: StorageType,
    /// Device name
    pub name: [u8; 32],
    /// Name length
    pub name_len: usize,
    /// Model
    pub model: [u8; 40],
    /// Model length
    pub model_len: usize,
    /// Serial number
    pub serial: [u8; 20],
    /// Serial length
    pub serial_len: usize,
    /// Capacity (bytes)
    pub capacity: u64,
    /// Block size
    pub block_size: u32,
    /// Removable
    pub removable: bool,
    /// Read-only
    pub read_only: bool,
    /// Has ESP
    pub has_esp: bool,
    /// Boot device
    pub is_boot_device: bool,
}

impl Default for StorageDeviceInfo {
    fn default() -> Self {
        Self {
            device_type: StorageType::Unknown,
            name: [0u8; 32],
            name_len: 0,
            model: [0u8; 40],
            model_len: 0,
            serial: [0u8; 20],
            serial_len: 0,
            capacity: 0,
            block_size: 512,
            removable: false,
            read_only: false,
            has_esp: false,
            is_boot_device: false,
        }
    }
}

impl StorageDeviceInfo {
    /// Get capacity in MB
    pub const fn capacity_mb(&self) -> u64 {
        self.capacity / (1024 * 1024)
    }

    /// Get capacity in GB
    pub const fn capacity_gb(&self) -> u64 {
        self.capacity / (1024 * 1024 * 1024)
    }
}

/// Storage summary
#[derive(Debug, Clone, Copy, Default)]
pub struct StorageSummary {
    /// Number of storage devices
    pub device_count: u8,
    /// Number of partitions
    pub partition_count: u16,
    /// Number of ESP partitions
    pub esp_count: u8,
    /// Total capacity (bytes)
    pub total_capacity: u64,
    /// Boot device present
    pub boot_device_found: bool,
    /// Boot device index
    pub boot_device_index: u8,
}

// =============================================================================
// FIRMWARE INFORMATION
// =============================================================================

/// Firmware type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FirmwareType {
    /// Unknown
    #[default]
    Unknown,
    /// UEFI
    Uefi,
    /// Legacy BIOS
    LegacyBios,
}

impl fmt::Display for FirmwareType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FirmwareType::Unknown => write!(f, "Unknown"),
            FirmwareType::Uefi => write!(f, "UEFI"),
            FirmwareType::LegacyBios => write!(f, "Legacy BIOS"),
        }
    }
}

/// UEFI version
#[derive(Debug, Clone, Copy, Default)]
pub struct UefiVersion {
    /// Major version
    pub major: u16,
    /// Minor version (upper 8 bits: minor, lower 8 bits: revision)
    pub minor: u16,
}

impl UefiVersion {
    /// Create new version
    pub const fn new(major: u16, minor: u8, revision: u8) -> Self {
        Self {
            major,
            minor: ((minor as u16) << 8) | (revision as u16),
        }
    }

    /// Get minor
    pub const fn minor_version(&self) -> u8 {
        (self.minor >> 8) as u8
    }

    /// Get revision
    pub const fn revision(&self) -> u8 {
        (self.minor & 0xFF) as u8
    }
}

impl fmt::Display for UefiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{}.{}",
            self.major,
            self.minor_version(),
            self.revision()
        )
    }
}

/// Firmware summary
#[derive(Debug, Clone, Copy)]
pub struct FirmwareSummary {
    /// Firmware type
    pub firmware_type: FirmwareType,
    /// Vendor
    pub vendor: [u8; 32],
    /// Vendor length
    pub vendor_len: usize,
    /// Version string
    pub version: [u8; 32],
    /// Version length
    pub version_len: usize,
    /// UEFI version
    pub uefi_version: UefiVersion,
    /// Secure boot enabled
    pub secure_boot: bool,
    /// Secure boot in setup mode
    pub setup_mode: bool,
    /// ACPI version
    pub acpi_version: u8,
    /// SMBIOS version major
    pub smbios_major: u8,
    /// SMBIOS version minor
    pub smbios_minor: u8,
    /// Runtime services available
    pub runtime_services: bool,
    /// Boot services available
    pub boot_services: bool,
}

impl Default for FirmwareSummary {
    fn default() -> Self {
        Self {
            firmware_type: FirmwareType::Unknown,
            vendor: [0u8; 32],
            vendor_len: 0,
            version: [0u8; 32],
            version_len: 0,
            uefi_version: UefiVersion::default(),
            secure_boot: false,
            setup_mode: false,
            acpi_version: 0,
            smbios_major: 0,
            smbios_minor: 0,
            runtime_services: false,
            boot_services: true,
        }
    }
}

// =============================================================================
// GRAPHICS INFORMATION
// =============================================================================

/// Graphics output mode
#[derive(Debug, Clone, Copy, Default)]
pub struct GraphicsMode {
    /// Mode number
    pub mode: u32,
    /// Horizontal resolution
    pub width: u32,
    /// Vertical resolution
    pub height: u32,
    /// Pixels per scan line
    pub stride: u32,
    /// Pixel format
    pub format: PixelFormat,
}

/// Pixel format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PixelFormat {
    /// Unknown
    #[default]
    Unknown,
    /// RGB 32-bit
    Rgb32,
    /// BGR 32-bit
    Bgr32,
    /// Bit mask
    BitMask,
    /// BLT only
    BltOnly,
}

impl fmt::Display for PixelFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PixelFormat::Unknown => write!(f, "Unknown"),
            PixelFormat::Rgb32 => write!(f, "RGB32"),
            PixelFormat::Bgr32 => write!(f, "BGR32"),
            PixelFormat::BitMask => write!(f, "BitMask"),
            PixelFormat::BltOnly => write!(f, "BLT Only"),
        }
    }
}

/// Graphics summary
#[derive(Debug, Clone, Copy, Default)]
pub struct GraphicsSummary {
    /// Graphics available
    pub available: bool,
    /// Current mode
    pub current_mode: GraphicsMode,
    /// Number of modes
    pub mode_count: u32,
    /// Framebuffer address
    pub framebuffer_addr: u64,
    /// Framebuffer size
    pub framebuffer_size: u64,
    /// Is console mode
    pub console_mode: bool,
    /// Console columns
    pub console_cols: u32,
    /// Console rows
    pub console_rows: u32,
}

impl GraphicsSummary {
    /// Get resolution string
    pub fn resolution_string(&self) -> [u8; 16] {
        let mut buf = [0u8; 16];
        // Format: WIDTHxHEIGHT
        let w = self.current_mode.width;
        let h = self.current_mode.height;
        let mut idx = 0;

        // Width
        let mut w_digits = [0u8; 5];
        let mut w_len = 0;
        let mut temp = w;
        loop {
            w_digits[w_len] = b'0' + (temp % 10) as u8;
            temp /= 10;
            w_len += 1;
            if temp == 0 {
                break;
            }
        }
        for i in (0..w_len).rev() {
            if idx < 16 {
                buf[idx] = w_digits[i];
                idx += 1;
            }
        }

        if idx < 16 {
            buf[idx] = b'x';
            idx += 1;
        }

        // Height
        let mut h_digits = [0u8; 5];
        let mut h_len = 0;
        let mut temp = h;
        loop {
            h_digits[h_len] = b'0' + (temp % 10) as u8;
            temp /= 10;
            h_len += 1;
            if temp == 0 {
                break;
            }
        }
        for i in (0..h_len).rev() {
            if idx < 16 {
                buf[idx] = h_digits[i];
                idx += 1;
            }
        }

        buf
    }
}

// =============================================================================
// BOOT INFORMATION
// =============================================================================

/// Boot status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BootStatus {
    /// Not started
    #[default]
    NotStarted,
    /// In progress
    InProgress,
    /// Completed
    Completed,
    /// Failed
    Failed,
}

impl fmt::Display for BootStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BootStatus::NotStarted => write!(f, "Not Started"),
            BootStatus::InProgress => write!(f, "In Progress"),
            BootStatus::Completed => write!(f, "Completed"),
            BootStatus::Failed => write!(f, "Failed"),
        }
    }
}

/// Boot summary
#[derive(Debug, Clone, Copy)]
pub struct BootSummary {
    /// Boot status
    pub status: BootStatus,
    /// Boot device path
    pub boot_device: [u8; 64],
    /// Boot device length
    pub boot_device_len: usize,
    /// Boot entry count
    pub entry_count: u16,
    /// Default entry
    pub default_entry: u16,
    /// Selected entry
    pub selected_entry: u16,
    /// Timeout (seconds)
    pub timeout: u16,
    /// Quick boot
    pub quick_boot: bool,
    /// Safe mode
    pub safe_mode: bool,
    /// Debug mode
    pub debug_mode: bool,
    /// Boot time (milliseconds)
    pub boot_time_ms: u32,
    /// Last boot status
    pub last_boot_ok: bool,
}

impl Default for BootSummary {
    fn default() -> Self {
        Self {
            status: BootStatus::default(),
            boot_device: [0u8; 64],
            boot_device_len: 0,
            entry_count: 0,
            default_entry: 0,
            selected_entry: 0,
            timeout: 0,
            quick_boot: false,
            safe_mode: false,
            debug_mode: false,
            boot_time_ms: 0,
            last_boot_ok: false,
        }
    }
}

// =============================================================================
// NETWORK INFORMATION
// =============================================================================

/// Network interface info
#[derive(Debug, Clone, Copy)]
pub struct NetworkInterfaceInfo {
    /// Interface index
    pub index: u8,
    /// MAC address
    pub mac: [u8; 6],
    /// IPv4 address
    pub ipv4: [u8; 4],
    /// IPv4 mask
    pub ipv4_mask: [u8; 4],
    /// IPv4 gateway
    pub ipv4_gateway: [u8; 4],
    /// IPv6 address
    pub ipv6: [u8; 16],
    /// Link up
    pub link_up: bool,
    /// Speed (Mbps)
    pub speed_mbps: u32,
    /// PXE capable
    pub pxe_capable: bool,
}

impl Default for NetworkInterfaceInfo {
    fn default() -> Self {
        Self {
            index: 0,
            mac: [0u8; 6],
            ipv4: [0u8; 4],
            ipv4_mask: [0u8; 4],
            ipv4_gateway: [0u8; 4],
            ipv6: [0u8; 16],
            link_up: false,
            speed_mbps: 0,
            pxe_capable: false,
        }
    }
}

impl NetworkInterfaceInfo {
    /// Format MAC address
    pub fn mac_string(&self) -> [u8; 17] {
        let mut buf = [0u8; 17];
        const HEX: &[u8] = b"0123456789ABCDEF";
        let mut idx = 0;
        for i in 0..6 {
            buf[idx] = HEX[(self.mac[i] >> 4) as usize];
            buf[idx + 1] = HEX[(self.mac[i] & 0xF) as usize];
            idx += 2;
            if i < 5 {
                buf[idx] = b':';
                idx += 1;
            }
        }
        buf
    }

    /// Format IPv4 address
    pub fn ipv4_string(&self) -> [u8; 15] {
        let mut buf = [0u8; 15];
        let mut idx = 0;
        for i in 0..4 {
            let mut val = self.ipv4[i];
            let mut digits = [0u8; 3];
            let mut digit_count = 0;
            loop {
                digits[digit_count] = b'0' + (val % 10);
                val /= 10;
                digit_count += 1;
                if val == 0 {
                    break;
                }
            }
            for d in (0..digit_count).rev() {
                if idx < 15 {
                    buf[idx] = digits[d];
                    idx += 1;
                }
            }
            if i < 3 && idx < 15 {
                buf[idx] = b'.';
                idx += 1;
            }
        }
        buf
    }
}

/// Network summary
#[derive(Debug, Clone, Copy, Default)]
pub struct NetworkSummary {
    /// Interface count
    pub interface_count: u8,
    /// Any link up
    pub any_link_up: bool,
    /// IPv4 configured
    pub ipv4_configured: bool,
    /// IPv6 configured
    pub ipv6_configured: bool,
    /// PXE available
    pub pxe_available: bool,
    /// DNS servers configured
    pub dns_configured: bool,
}

// =============================================================================
// SECURITY INFORMATION
// =============================================================================

/// Security status
#[derive(Debug, Clone, Copy, Default)]
pub struct SecurityStatus {
    /// Secure boot enabled
    pub secure_boot_enabled: bool,
    /// Setup mode
    pub setup_mode: bool,
    /// Deployed mode
    pub deployed_mode: bool,
    /// TPM present
    pub tpm_present: bool,
    /// TPM version
    pub tpm_version: u8,
    /// TPM active
    pub tpm_active: bool,
    /// Password set
    pub password_set: bool,
    /// Admin password
    pub admin_password: bool,
    /// User password
    pub user_password: bool,
    /// Boot password
    pub boot_password: bool,
    /// Lockout active
    pub lockout_active: bool,
    /// Failed attempts
    pub failed_attempts: u8,
}

// =============================================================================
// DIAGNOSTIC REPORT
// =============================================================================

/// Diagnostic level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiagnosticLevel {
    /// Basic info only
    #[default]
    Basic,
    /// Standard detail
    Standard,
    /// Detailed
    Detailed,
    /// Full/verbose
    Full,
}

/// Maximum diagnostic entries
pub const MAX_DIAG_ENTRIES: usize = 32;

/// Diagnostic entry
#[derive(Debug, Clone, Copy)]
pub struct DiagnosticEntry {
    /// Category
    pub category: DiagnosticCategory,
    /// Status
    pub status: DiagnosticStatus,
    /// Code
    pub code: u16,
    /// Message
    pub message: [u8; 64],
    /// Message length
    pub message_len: usize,
    /// Value (if applicable)
    pub value: u64,
}

impl Default for DiagnosticEntry {
    fn default() -> Self {
        Self {
            category: DiagnosticCategory::System,
            status: DiagnosticStatus::Ok,
            code: 0,
            message: [0u8; 64],
            message_len: 0,
            value: 0,
        }
    }
}

/// Diagnostic category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiagnosticCategory {
    /// System
    #[default]
    System,
    /// CPU
    Cpu,
    /// Memory
    Memory,
    /// Storage
    Storage,
    /// Network
    Network,
    /// Graphics
    Graphics,
    /// Security
    Security,
    /// Boot
    Boot,
    /// Firmware
    Firmware,
}

/// Diagnostic status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiagnosticStatus {
    /// OK
    #[default]
    Ok,
    /// Warning
    Warning,
    /// Error
    Error,
    /// Critical
    Critical,
    /// Unknown
    Unknown,
}

/// Diagnostic report
#[derive(Debug)]
pub struct DiagnosticReport {
    /// Entries
    pub entries: [DiagnosticEntry; MAX_DIAG_ENTRIES],
    /// Entry count
    pub count: usize,
    /// Overall status
    pub overall: DiagnosticStatus,
    /// Warning count
    pub warnings: u16,
    /// Error count
    pub errors: u16,
    /// Timestamp
    pub timestamp_us: u64,
}

impl Default for DiagnosticReport {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagnosticReport {
    /// Create new report
    pub const fn new() -> Self {
        Self {
            entries: [DiagnosticEntry {
                category: DiagnosticCategory::System,
                status: DiagnosticStatus::Ok,
                code: 0,
                message: [0u8; 64],
                message_len: 0,
                value: 0,
            }; MAX_DIAG_ENTRIES],
            count: 0,
            overall: DiagnosticStatus::Ok,
            warnings: 0,
            errors: 0,
            timestamp_us: 0,
        }
    }

    /// Add entry
    pub fn add(&mut self, entry: DiagnosticEntry) -> bool {
        if self.count >= MAX_DIAG_ENTRIES {
            return false;
        }

        // Update counters
        match entry.status {
            DiagnosticStatus::Warning => self.warnings += 1,
            DiagnosticStatus::Error | DiagnosticStatus::Critical => self.errors += 1,
            _ => {},
        }

        self.entries[self.count] = entry;
        self.count += 1;
        true
    }

    /// Finalize report
    pub fn finalize(&mut self) {
        if self.errors > 0 {
            self.overall = DiagnosticStatus::Error;
        } else if self.warnings > 0 {
            self.overall = DiagnosticStatus::Warning;
        } else {
            self.overall = DiagnosticStatus::Ok;
        }
    }

    /// Get entry count
    pub const fn len(&self) -> usize {
        self.count
    }

    /// Check if empty
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_features() {
        let mut features = CpuFeatures::NONE;
        features.set(CpuFeatures::SSE);
        features.set(CpuFeatures::SSE2);
        assert!(features.has(CpuFeatures::SSE));
        assert!(features.has(CpuFeatures::SSE2));
        assert!(!features.has(CpuFeatures::AVX));
        assert_eq!(features.count(), 2);
    }

    #[test]
    fn test_uefi_version() {
        let ver = UefiVersion::new(2, 10, 0);
        assert_eq!(ver.major, 2);
        assert_eq!(ver.minor_version(), 10);
        assert_eq!(ver.revision(), 0);
    }

    #[test]
    fn test_memory_summary() {
        let mem = MemorySummary {
            total_physical: 16 * 1024 * 1024 * 1024, // 16 GB
            ..Default::default()
        };
        assert_eq!(mem.total_gb(), 16);
        assert_eq!(mem.total_mb(), 16384);
    }

    #[test]
    fn test_network_mac_string() {
        let iface = NetworkInterfaceInfo {
            mac: [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
            ..Default::default()
        };
        let mac = iface.mac_string();
        assert_eq!(&mac[..17], b"00:11:22:33:44:55");
    }

    #[test]
    fn test_diagnostic_report() {
        let mut report = DiagnosticReport::new();

        report.add(DiagnosticEntry {
            category: DiagnosticCategory::Memory,
            status: DiagnosticStatus::Ok,
            ..Default::default()
        });

        report.add(DiagnosticEntry {
            category: DiagnosticCategory::Storage,
            status: DiagnosticStatus::Warning,
            ..Default::default()
        });

        report.finalize();

        assert_eq!(report.len(), 2);
        assert_eq!(report.warnings, 1);
        assert_eq!(report.overall, DiagnosticStatus::Warning);
    }
}
