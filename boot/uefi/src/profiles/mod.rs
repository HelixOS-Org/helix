//! Boot Profiles and Presets Management
//!
//! This module provides comprehensive profile management for different
//! boot configurations, including hardware profiles and user presets.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                      Profile Management                                 │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Hardware Profiles                              │   │
//! │  │  Desktop │ Laptop │ Server │ Embedded │ VM │ Custom             │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                                                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   User Presets                                   │   │
//! │  │  Quick │ Full │ Debug │ Recovery │ Safe │ Gaming │ Minimal      │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                                                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Environment Presets                            │   │
//! │  │  Development │ Production │ Testing │ Staging │ Demo            │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

use core::fmt;

// =============================================================================
// HARDWARE PROFILE
// =============================================================================

/// Hardware platform type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HardwarePlatform {
    /// Generic desktop PC
    Desktop,
    /// Laptop/notebook
    Laptop,
    /// Server/datacenter
    Server,
    /// Workstation
    Workstation,
    /// Embedded system
    Embedded,
    /// Virtual machine
    VirtualMachine,
    /// Cloud instance
    Cloud,
    /// Raspberry Pi / ARM SBC
    ArmSbc,
    /// Apple Silicon Mac
    AppleSilicon,
    /// Unknown platform
    #[default]
    Unknown,
}

impl fmt::Display for HardwarePlatform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HardwarePlatform::Desktop => write!(f, "Desktop"),
            HardwarePlatform::Laptop => write!(f, "Laptop"),
            HardwarePlatform::Server => write!(f, "Server"),
            HardwarePlatform::Workstation => write!(f, "Workstation"),
            HardwarePlatform::Embedded => write!(f, "Embedded"),
            HardwarePlatform::VirtualMachine => write!(f, "Virtual Machine"),
            HardwarePlatform::Cloud => write!(f, "Cloud Instance"),
            HardwarePlatform::ArmSbc => write!(f, "ARM SBC"),
            HardwarePlatform::AppleSilicon => write!(f, "Apple Silicon"),
            HardwarePlatform::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Virtual machine type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VmType {
    /// Not a VM
    #[default]
    None,
    /// VMware
    VMware,
    /// VirtualBox
    VirtualBox,
    /// QEMU/KVM
    QemuKvm,
    /// Hyper-V
    HyperV,
    /// Xen
    Xen,
    /// Parallels
    Parallels,
    /// Bhyve (FreeBSD)
    Bhyve,
    /// Amazon EC2
    Ec2,
    /// Google Cloud
    Gce,
    /// Microsoft Azure
    Azure,
    /// Unknown VM
    Unknown,
}

impl VmType {
    /// Check if this is a VM
    pub const fn is_vm(&self) -> bool {
        !matches!(self, VmType::None)
    }

    /// Check if cloud instance
    pub const fn is_cloud(&self) -> bool {
        matches!(self, VmType::Ec2 | VmType::Gce | VmType::Azure)
    }
}

/// CPU architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CpuArch {
    /// x86-64 / AMD64
    X86_64,
    /// AArch64 / ARM64
    Aarch64,
    /// RISC-V 64-bit
    Riscv64,
    /// LoongArch64
    Loongarch64,
    /// Unknown
    #[default]
    Unknown,
}

impl fmt::Display for CpuArch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CpuArch::X86_64 => write!(f, "x86-64"),
            CpuArch::Aarch64 => write!(f, "AArch64"),
            CpuArch::Riscv64 => write!(f, "RISC-V 64"),
            CpuArch::Loongarch64 => write!(f, "LoongArch64"),
            CpuArch::Unknown => write!(f, "Unknown"),
        }
    }
}

/// CPU vendor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CpuVendor {
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
    /// NVIDIA
    Nvidia,
    /// Ampere
    Ampere,
    /// Unknown
    #[default]
    Unknown,
}

/// Firmware type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FirmwareType {
    /// UEFI
    Uefi,
    /// UEFI with Secure Boot
    UefiSecure,
    /// Legacy BIOS (via CSM)
    LegacyCsm,
    /// coreboot
    Coreboot,
    /// U-Boot
    UBoot,
    /// Unknown
    #[default]
    Unknown,
}

/// Hardware capabilities
#[derive(Debug, Clone, Copy, Default)]
pub struct HardwareCaps {
    /// Has TPM
    pub tpm: bool,
    /// TPM version (12 = 1.2, 20 = 2.0)
    pub tpm_version: u8,
    /// Has Secure Boot
    pub secure_boot: bool,
    /// Has IOMMU (VT-d / AMD-Vi)
    pub iommu: bool,
    /// Has hardware virtualization
    pub virtualization: bool,
    /// Has NVMe
    pub nvme: bool,
    /// Has USB 3.x
    pub usb3: bool,
    /// Has Thunderbolt
    pub thunderbolt: bool,
    /// Has network boot capability
    pub pxe: bool,
    /// Has WiFi
    pub wifi: bool,
    /// Has Bluetooth
    pub bluetooth: bool,
    /// Has GPU
    pub gpu: bool,
    /// GPU is discrete
    pub discrete_gpu: bool,
    /// Has battery (laptop/mobile)
    pub battery: bool,
    /// Has touchscreen
    pub touchscreen: bool,
}

/// Hardware profile
#[derive(Debug, Clone, Copy)]
pub struct HardwareProfile {
    /// Platform type
    pub platform: HardwarePlatform,
    /// CPU architecture
    pub arch: CpuArch,
    /// CPU vendor
    pub vendor: CpuVendor,
    /// VM type (if applicable)
    pub vm_type: VmType,
    /// Firmware type
    pub firmware: FirmwareType,
    /// Hardware capabilities
    pub caps: HardwareCaps,
    /// CPU count
    pub cpu_count: u16,
    /// RAM size (MB)
    pub ram_mb: u32,
    /// System vendor ID
    pub system_vendor: [u8; 32],
    /// System vendor length
    pub system_vendor_len: u8,
    /// System product name
    pub product_name: [u8; 64],
    /// Product name length
    pub product_name_len: u8,
}

impl HardwareProfile {
    /// Create new profile
    pub const fn new() -> Self {
        Self {
            platform: HardwarePlatform::Unknown,
            arch: CpuArch::Unknown,
            vendor: CpuVendor::Unknown,
            vm_type: VmType::None,
            firmware: FirmwareType::Unknown,
            caps: HardwareCaps {
                tpm: false,
                tpm_version: 0,
                secure_boot: false,
                iommu: false,
                virtualization: false,
                nvme: false,
                usb3: false,
                thunderbolt: false,
                pxe: false,
                wifi: false,
                bluetooth: false,
                gpu: false,
                discrete_gpu: false,
                battery: false,
                touchscreen: false,
            },
            cpu_count: 0,
            ram_mb: 0,
            system_vendor: [0u8; 32],
            system_vendor_len: 0,
            product_name: [0u8; 64],
            product_name_len: 0,
        }
    }

    /// Check if mobile platform
    pub const fn is_mobile(&self) -> bool {
        matches!(self.platform, HardwarePlatform::Laptop) || self.caps.battery
    }

    /// Check if virtual platform
    pub const fn is_virtual(&self) -> bool {
        self.vm_type.is_vm()
    }
}

impl Default for HardwareProfile {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// BOOT PRESET
// =============================================================================

/// Boot preset type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PresetType {
    /// Quick boot (minimal checks)
    Quick,
    /// Normal boot (default)
    #[default]
    Normal,
    /// Full diagnostic boot
    Full,
    /// Debug boot
    Debug,
    /// Safe mode
    Safe,
    /// Recovery mode
    Recovery,
    /// Minimal boot
    Minimal,
    /// Gaming optimized
    Gaming,
    /// Power saving
    PowerSave,
    /// Custom preset
    Custom,
}

impl fmt::Display for PresetType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PresetType::Quick => write!(f, "Quick Boot"),
            PresetType::Normal => write!(f, "Normal"),
            PresetType::Full => write!(f, "Full Diagnostic"),
            PresetType::Debug => write!(f, "Debug Mode"),
            PresetType::Safe => write!(f, "Safe Mode"),
            PresetType::Recovery => write!(f, "Recovery"),
            PresetType::Minimal => write!(f, "Minimal"),
            PresetType::Gaming => write!(f, "Gaming"),
            PresetType::PowerSave => write!(f, "Power Save"),
            PresetType::Custom => write!(f, "Custom"),
        }
    }
}

/// Boot preset flags
#[derive(Debug, Clone, Copy, Default)]
pub struct PresetFlags {
    bits: u32,
}

impl PresetFlags {
    // Initialization flags
    pub const SKIP_MEMORY_TEST: u32 = 1 << 0;
    pub const SKIP_DEVICE_ENUM: u32 = 1 << 1;
    pub const SKIP_PCI_SCAN: u32 = 1 << 2;
    pub const SKIP_USB_INIT: u32 = 1 << 3;
    pub const SKIP_NETWORK: u32 = 1 << 4;
    pub const SKIP_GRAPHICS: u32 = 1 << 5;

    // Feature flags
    pub const ENABLE_DEBUG: u32 = 1 << 8;
    pub const ENABLE_LOGGING: u32 = 1 << 9;
    pub const ENABLE_SERIAL: u32 = 1 << 10;
    pub const ENABLE_TIMING: u32 = 1 << 11;

    // Mode flags
    pub const SAFE_MODE: u32 = 1 << 16;
    pub const RECOVERY_MODE: u32 = 1 << 17;
    pub const SINGLE_USER: u32 = 1 << 18;
    pub const VERBOSE: u32 = 1 << 19;
    pub const QUIET: u32 = 1 << 20;

    /// Create new flags
    pub const fn new(bits: u32) -> Self {
        Self { bits }
    }

    /// Empty flags
    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    /// Check flag
    pub const fn contains(&self, flag: u32) -> bool {
        (self.bits & flag) != 0
    }

    /// Set flag
    pub fn set(&mut self, flag: u32) {
        self.bits |= flag;
    }

    /// Clear flag
    pub fn clear(&mut self, flag: u32) {
        self.bits &= !flag;
    }

    /// Get bits
    pub const fn bits(&self) -> u32 {
        self.bits
    }
}

/// Boot preset configuration
#[derive(Debug, Clone, Copy)]
pub struct BootPreset {
    /// Preset type
    pub preset_type: PresetType,
    /// Preset name
    pub name: [u8; 32],
    /// Name length
    pub name_len: usize,
    /// Flags
    pub flags: PresetFlags,
    /// Timeout override (-1 = use default)
    pub timeout: i16,
    /// Log level override
    pub log_level: u8,
    /// Additional kernel parameters
    pub extra_params: [u8; 256],
    /// Extra params length
    pub extra_params_len: usize,
}

impl Default for BootPreset {
    fn default() -> Self {
        Self::normal()
    }
}

impl BootPreset {
    /// Create quick boot preset
    pub const fn quick() -> Self {
        Self {
            preset_type: PresetType::Quick,
            name: *b"Quick Boot\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
            name_len: 10,
            flags: PresetFlags::new(
                PresetFlags::SKIP_MEMORY_TEST | PresetFlags::SKIP_DEVICE_ENUM | PresetFlags::QUIET,
            ),
            timeout: 0,
            log_level: 1, // Error only
            extra_params: [0u8; 256],
            extra_params_len: 0,
        }
    }

    /// Create normal boot preset
    pub const fn normal() -> Self {
        Self {
            preset_type: PresetType::Normal,
            name: *b"Normal Boot\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
            name_len: 11,
            flags: PresetFlags::empty(),
            timeout: -1,  // Use default
            log_level: 2, // Warning
            extra_params: [0u8; 256],
            extra_params_len: 0,
        }
    }

    /// Create debug preset
    pub const fn debug() -> Self {
        Self {
            preset_type: PresetType::Debug,
            name: *b"Debug Mode\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
            name_len: 10,
            flags: PresetFlags::new(
                PresetFlags::ENABLE_DEBUG
                    | PresetFlags::ENABLE_LOGGING
                    | PresetFlags::ENABLE_SERIAL
                    | PresetFlags::ENABLE_TIMING
                    | PresetFlags::VERBOSE,
            ),
            timeout: 30,
            log_level: 5, // Trace
            extra_params: [0u8; 256],
            extra_params_len: 0,
        }
    }

    /// Create safe mode preset
    pub const fn safe() -> Self {
        Self {
            preset_type: PresetType::Safe,
            name: *b"Safe Mode\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
            name_len: 9,
            flags: PresetFlags::new(PresetFlags::SAFE_MODE | PresetFlags::SKIP_USB_INIT),
            timeout: 10,
            log_level: 3, // Info
            extra_params: [0u8; 256],
            extra_params_len: 0,
        }
    }

    /// Create recovery preset
    pub const fn recovery() -> Self {
        Self {
            preset_type: PresetType::Recovery,
            name: *b"Recovery Mode\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
            name_len: 13,
            flags: PresetFlags::new(
                PresetFlags::RECOVERY_MODE | PresetFlags::SINGLE_USER | PresetFlags::ENABLE_LOGGING,
            ),
            timeout: 15,
            log_level: 4, // Debug
            extra_params: [0u8; 256],
            extra_params_len: 0,
        }
    }

    /// Create minimal preset
    pub const fn minimal() -> Self {
        Self {
            preset_type: PresetType::Minimal,
            name: *b"Minimal\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
            name_len: 7,
            flags: PresetFlags::new(
                PresetFlags::SKIP_MEMORY_TEST
                    | PresetFlags::SKIP_DEVICE_ENUM
                    | PresetFlags::SKIP_PCI_SCAN
                    | PresetFlags::SKIP_USB_INIT
                    | PresetFlags::SKIP_NETWORK
                    | PresetFlags::QUIET,
            ),
            timeout: 0,
            log_level: 0, // Off
            extra_params: [0u8; 256],
            extra_params_len: 0,
        }
    }

    /// Get preset name as str
    pub fn name_str(&self) -> &str {
        if self.name_len > 0 {
            core::str::from_utf8(&self.name[..self.name_len]).unwrap_or("")
        } else {
            ""
        }
    }
}

// =============================================================================
// ENVIRONMENT PRESET
// =============================================================================

/// Environment type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    /// Development environment
    Development,
    /// Testing/QA environment
    Testing,
    /// Staging/pre-production
    Staging,
    /// Production
    Production,
    /// Demo/showcase
    Demo,
    /// Benchmark/performance testing
    Benchmark,
}

impl Default for Environment {
    fn default() -> Self {
        Environment::Production
    }
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Environment::Development => write!(f, "Development"),
            Environment::Testing => write!(f, "Testing"),
            Environment::Staging => write!(f, "Staging"),
            Environment::Production => write!(f, "Production"),
            Environment::Demo => write!(f, "Demo"),
            Environment::Benchmark => write!(f, "Benchmark"),
        }
    }
}

/// Environment configuration
#[derive(Debug, Clone, Copy)]
pub struct EnvConfig {
    /// Environment type
    pub env_type: Environment,
    /// Enable debug features
    pub debug_enabled: bool,
    /// Enable assertions
    pub assertions: bool,
    /// Enable profiling
    pub profiling: bool,
    /// Log level
    pub log_level: u8,
    /// Enable crash dumps
    pub crash_dumps: bool,
    /// Enable telemetry
    pub telemetry: bool,
    /// Enable auto-update
    pub auto_update: bool,
    /// Config server URL enabled
    pub remote_config: bool,
}

impl Default for EnvConfig {
    fn default() -> Self {
        Self::production()
    }
}

impl EnvConfig {
    /// Development environment
    pub const fn development() -> Self {
        Self {
            env_type: Environment::Development,
            debug_enabled: true,
            assertions: true,
            profiling: true,
            log_level: 5, // Trace
            crash_dumps: true,
            telemetry: false,
            auto_update: false,
            remote_config: false,
        }
    }

    /// Testing environment
    pub const fn testing() -> Self {
        Self {
            env_type: Environment::Testing,
            debug_enabled: true,
            assertions: true,
            profiling: true,
            log_level: 4, // Debug
            crash_dumps: true,
            telemetry: false,
            auto_update: false,
            remote_config: false,
        }
    }

    /// Staging environment
    pub const fn staging() -> Self {
        Self {
            env_type: Environment::Staging,
            debug_enabled: false,
            assertions: true,
            profiling: true,
            log_level: 3, // Info
            crash_dumps: true,
            telemetry: true,
            auto_update: false,
            remote_config: true,
        }
    }

    /// Production environment
    pub const fn production() -> Self {
        Self {
            env_type: Environment::Production,
            debug_enabled: false,
            assertions: false,
            profiling: false,
            log_level: 2, // Warning
            crash_dumps: false,
            telemetry: true,
            auto_update: true,
            remote_config: true,
        }
    }

    /// Benchmark environment
    pub const fn benchmark() -> Self {
        Self {
            env_type: Environment::Benchmark,
            debug_enabled: false,
            assertions: false,
            profiling: true,
            log_level: 1, // Error
            crash_dumps: false,
            telemetry: false,
            auto_update: false,
            remote_config: false,
        }
    }
}

// =============================================================================
// KERNEL PARAMETERS
// =============================================================================

/// Common kernel parameter presets
pub mod kernel_params {
    /// Quiet boot
    pub const QUIET: &str = "quiet";

    /// Splash screen
    pub const SPLASH: &str = "splash";

    /// No splash
    pub const NOSPLASH: &str = "nosplash";

    /// Single user mode
    pub const SINGLE: &str = "single";

    /// Init=/bin/bash
    pub const INIT_BASH: &str = "init=/bin/bash";

    /// Emergency mode
    pub const EMERGENCY: &str = "emergency";

    /// No modeset (disable KMS)
    pub const NOMODESET: &str = "nomodeset";

    /// Disable ACPI
    pub const ACPI_OFF: &str = "acpi=off";

    /// Force ACPI
    pub const ACPI_FORCE: &str = "acpi=force";

    /// No IOMMU
    pub const IOMMU_OFF: &str = "iommu=off";

    /// Enable IOMMU passthrough
    pub const IOMMU_PT: &str = "iommu=pt";

    /// Intel IOMMU
    pub const INTEL_IOMMU: &str = "intel_iommu=on";

    /// AMD IOMMU
    pub const AMD_IOMMU: &str = "amd_iommu=on";

    /// Disable nouveau (NVIDIA open driver)
    pub const NOUVEAU_OFF: &str = "nouveau.modeset=0";

    /// Enable debug shell
    pub const DEBUG_SHELL: &str = "rd.shell";

    /// Break at initramfs
    pub const BREAK_INITRD: &str = "rd.break";

    /// Rescue target
    pub const RESCUE: &str = "systemd.unit=rescue.target";

    /// Multi-user target
    pub const MULTI_USER: &str = "systemd.unit=multi-user.target";

    /// Graphical target
    pub const GRAPHICAL: &str = "systemd.unit=graphical.target";

    /// Memory limit
    pub const MEM_LIMIT: &str = "mem=";

    /// Max CPUs
    pub const MAX_CPUS: &str = "maxcpus=";

    /// No randomize VA space
    pub const NO_ASLR: &str = "nokaslr";

    /// Disable spectre mitigations
    pub const NO_SPECTRE: &str = "nospectre_v1 nospectre_v2";

    /// Disable all mitigations
    pub const MITIGATIONS_OFF: &str = "mitigations=off";
}

/// Parameter set for specific use cases
#[derive(Debug, Clone, Copy)]
pub struct ParamSet {
    /// Name
    pub name: &'static str,
    /// Parameters
    pub params: &'static str,
    /// Description
    pub description: &'static str,
}

/// Predefined parameter sets
pub const PARAM_SETS: &[ParamSet] = &[
    ParamSet {
        name: "silent",
        params: "quiet splash loglevel=0",
        description: "Silent boot with splash screen",
    },
    ParamSet {
        name: "verbose",
        params: "nosplash loglevel=7",
        description: "Verbose boot with all messages",
    },
    ParamSet {
        name: "rescue",
        params: "single init=/bin/bash",
        description: "Rescue mode with shell",
    },
    ParamSet {
        name: "safe_graphics",
        params: "nomodeset nouveau.modeset=0",
        description: "Safe graphics mode",
    },
    ParamSet {
        name: "performance",
        params: "mitigations=off nokaslr",
        description: "Maximum performance (less secure)",
    },
    ParamSet {
        name: "virtualization",
        params: "intel_iommu=on iommu=pt",
        description: "Enable IOMMU for virtualization",
    },
    ParamSet {
        name: "debug_early",
        params: "rd.shell rd.break earlyprintk=serial",
        description: "Debug early boot",
    },
];

// =============================================================================
// PROFILE MANAGER
// =============================================================================

/// Maximum user profiles
pub const MAX_PROFILES: usize = 16;

/// Profile manager
#[derive(Debug, Clone)]
pub struct ProfileManager {
    /// Hardware profile (detected)
    pub hardware: HardwareProfile,
    /// Current boot preset
    pub current_preset: BootPreset,
    /// Environment config
    pub env: EnvConfig,
    /// User profiles
    pub user_profiles: [BootPreset; MAX_PROFILES],
    /// User profile count
    pub profile_count: usize,
    /// Active profile index
    pub active_profile: usize,
}

impl Default for ProfileManager {
    fn default() -> Self {
        Self {
            hardware: HardwareProfile::new(),
            current_preset: BootPreset::normal(),
            env: EnvConfig::production(),
            user_profiles: core::array::from_fn(|_| BootPreset::normal()),
            profile_count: 0,
            active_profile: 0,
        }
    }
}

impl ProfileManager {
    /// Create new profile manager
    pub const fn new() -> Self {
        Self {
            hardware: HardwareProfile::new(),
            current_preset: BootPreset::normal(),
            env: EnvConfig::production(),
            user_profiles: [BootPreset::normal(); MAX_PROFILES],
            profile_count: 0,
            active_profile: 0,
        }
    }

    /// Add user profile
    pub fn add_profile(&mut self, profile: BootPreset) -> bool {
        if self.profile_count < MAX_PROFILES {
            self.user_profiles[self.profile_count] = profile;
            self.profile_count += 1;
            true
        } else {
            false
        }
    }

    /// Get active profile
    pub fn active(&self) -> &BootPreset {
        if self.active_profile < self.profile_count {
            &self.user_profiles[self.active_profile]
        } else {
            &self.current_preset
        }
    }

    /// Select built-in preset
    pub fn select_preset(&mut self, preset_type: PresetType) {
        self.current_preset = match preset_type {
            PresetType::Quick => BootPreset::quick(),
            PresetType::Normal => BootPreset::normal(),
            PresetType::Debug => BootPreset::debug(),
            PresetType::Safe => BootPreset::safe(),
            PresetType::Recovery => BootPreset::recovery(),
            PresetType::Minimal => BootPreset::minimal(),
            _ => BootPreset::normal(),
        };
    }

    /// Get recommended preset based on hardware
    pub fn recommended_preset(&self) -> PresetType {
        match self.hardware.platform {
            HardwarePlatform::Server => PresetType::Normal,
            HardwarePlatform::Embedded => PresetType::Minimal,
            HardwarePlatform::Laptop if self.hardware.caps.battery => PresetType::PowerSave,
            HardwarePlatform::VirtualMachine => PresetType::Quick,
            _ => PresetType::Normal,
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
    fn test_hardware_profile() {
        let profile = HardwareProfile::new();
        assert!(!profile.is_mobile());
        assert!(!profile.is_virtual());
    }

    #[test]
    fn test_preset_flags() {
        let mut flags = PresetFlags::new(PresetFlags::ENABLE_DEBUG);
        assert!(flags.contains(PresetFlags::ENABLE_DEBUG));

        flags.set(PresetFlags::VERBOSE);
        assert!(flags.contains(PresetFlags::VERBOSE));

        flags.clear(PresetFlags::ENABLE_DEBUG);
        assert!(!flags.contains(PresetFlags::ENABLE_DEBUG));
    }

    #[test]
    fn test_boot_presets() {
        let quick = BootPreset::quick();
        assert!(quick.flags.contains(PresetFlags::SKIP_MEMORY_TEST));

        let debug = BootPreset::debug();
        assert!(debug.flags.contains(PresetFlags::ENABLE_DEBUG));
        assert!(debug.flags.contains(PresetFlags::VERBOSE));
    }

    #[test]
    fn test_env_config() {
        let dev = EnvConfig::development();
        assert!(dev.debug_enabled);
        assert!(dev.assertions);

        let prod = EnvConfig::production();
        assert!(!prod.debug_enabled);
        assert!(prod.auto_update);
    }

    #[test]
    fn test_profile_manager() {
        let mut manager = ProfileManager::new();

        manager.select_preset(PresetType::Debug);
        assert_eq!(manager.current_preset.preset_type, PresetType::Debug);

        assert!(manager.add_profile(BootPreset::safe()));
        assert_eq!(manager.profile_count, 1);
    }

    #[test]
    fn test_vm_type() {
        assert!(!VmType::None.is_vm());
        assert!(VmType::QemuKvm.is_vm());
        assert!(VmType::Ec2.is_cloud());
        assert!(!VmType::VMware.is_cloud());
    }
}
