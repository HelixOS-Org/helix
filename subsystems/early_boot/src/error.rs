//! # Boot Error Handling
//!
//! Comprehensive error types and result handling for the early boot sequence.

use core::fmt;

/// Boot operation result type
pub type BootResult<T> = Result<T, BootError>;

/// Boot error enumeration
///
/// Covers all possible error conditions during the early boot process.
#[derive(Debug, Clone)]
pub enum BootError {
    // =========================================================================
    // General Errors
    // =========================================================================
    /// Generic failure with message
    Failed(&'static str),

    /// Operation not supported
    NotSupported(&'static str),

    /// Invalid parameter
    InvalidParameter(&'static str),

    /// Timeout occurred
    Timeout(&'static str),

    /// Resource not available
    ResourceUnavailable(&'static str),

    /// Already initialized
    AlreadyInitialized,

    /// Not initialized
    NotInitialized,

    // =========================================================================
    // Boot Info Errors
    // =========================================================================
    /// Invalid boot info structure
    InvalidBootInfo,

    /// Missing required boot info field
    MissingBootInfoField(&'static str),

    /// Unsupported boot protocol
    UnsupportedBootProtocol,

    /// Boot info magic mismatch
    BootInfoMagicMismatch,

    /// Boot info version mismatch
    BootInfoVersionMismatch,

    // =========================================================================
    // CPU Errors
    // =========================================================================
    /// Unsupported CPU
    UnsupportedCpu(&'static str),

    /// Required CPU feature missing
    MissingCpuFeature(&'static str),

    /// CPU initialization failed
    CpuInitFailed(&'static str),

    /// Invalid privilege level
    InvalidPrivilegeLevel,

    /// FPU/SIMD initialization failed
    FpuInitFailed,

    // =========================================================================
    // Memory Errors
    // =========================================================================
    /// No memory map available
    NoMemoryMap,

    /// Memory map is empty
    EmptyMemoryMap,

    /// Invalid memory map entry
    InvalidMemoryMapEntry,

    /// Insufficient physical memory
    InsufficientMemory,

    /// Page table allocation failed
    PageTableAllocationFailed,

    /// Page table setup failed
    PageTableSetupFailed(&'static str),

    /// Mapping failed
    MappingFailed {
        virt: u64,
        phys: u64,
        reason: &'static str,
    },

    /// Invalid address
    InvalidAddress(u64),

    /// Address not aligned
    AddressNotAligned { addr: u64, required: usize },

    /// Early heap setup failed
    EarlyHeapFailed,

    /// Out of memory
    OutOfMemory,

    // =========================================================================
    // Interrupt Errors
    // =========================================================================
    /// IDT setup failed
    IdtSetupFailed(&'static str),

    /// GDT setup failed
    GdtSetupFailed(&'static str),

    /// Exception handler registration failed
    ExceptionHandlerFailed(u8),

    /// IRQ controller not found
    IrqControllerNotFound,

    /// IRQ controller initialization failed
    IrqControllerInitFailed(&'static str),

    /// Invalid interrupt vector
    InvalidInterruptVector(u8),

    // =========================================================================
    // Timer Errors
    // =========================================================================
    /// No timer available
    NoTimerAvailable,

    /// Timer initialization failed
    TimerInitFailed(&'static str),

    /// Timer calibration failed
    TimerCalibrationFailed,

    /// Timer frequency detection failed
    TimerFrequencyDetectionFailed,

    // =========================================================================
    // SMP Errors
    // =========================================================================
    /// SMP not available
    SmpNotAvailable,

    /// AP startup failed
    ApStartupFailed { cpu_id: u32, reason: &'static str },

    /// Per-CPU data allocation failed
    PerCpuAllocationFailed,

    /// CPU topology detection failed
    TopologyDetectionFailed,

    /// Too many CPUs
    TooManyCpus { detected: usize, max: usize },

    // =========================================================================
    // Driver Errors
    // =========================================================================
    /// Serial port initialization failed
    SerialInitFailed(&'static str),

    /// Framebuffer initialization failed
    FramebufferInitFailed(&'static str),

    /// Console initialization failed
    ConsoleInitFailed(&'static str),

    /// Driver not found
    DriverNotFound(&'static str),

    // =========================================================================
    // ACPI/Firmware Errors
    // =========================================================================
    /// ACPI not available
    AcpiNotAvailable,

    /// ACPI table not found
    AcpiTableNotFound(&'static str),

    /// Invalid ACPI table
    InvalidAcpiTable(&'static str),

    /// SMBIOS not available
    SmbiosNotAvailable,

    /// Device tree not available
    DeviceTreeNotAvailable,

    /// Invalid device tree
    InvalidDeviceTree,

    // =========================================================================
    // Relocation Errors
    // =========================================================================
    /// Relocation failed
    RelocationFailed(&'static str),

    /// KASLR entropy generation failed
    KaslrEntropyFailed,

    /// Kernel too large
    KernelTooLarge { size: usize, max: usize },

    // =========================================================================
    // Handoff Errors
    // =========================================================================
    /// Kernel entry point not found
    KernelEntryNotFound,

    /// Invalid kernel entry point
    InvalidKernelEntry(u64),

    /// Handoff preparation failed
    HandoffPreparationFailed(&'static str),

    // =========================================================================
    // Architecture-Specific Errors
    // =========================================================================
    /// x86_64 specific error
    #[cfg(target_arch = "x86_64")]
    X86Error(X86BootError),

    /// AArch64 specific error
    #[cfg(target_arch = "aarch64")]
    ArmError(ArmBootError),

    /// RISC-V specific error
    #[cfg(target_arch = "riscv64")]
    RiscvError(RiscvBootError),
}

impl fmt::Display for BootError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Failed(msg) => write!(f, "Boot failed: {}", msg),
            Self::NotSupported(feature) => write!(f, "Not supported: {}", feature),
            Self::InvalidParameter(param) => write!(f, "Invalid parameter: {}", param),
            Self::Timeout(op) => write!(f, "Timeout during: {}", op),
            Self::ResourceUnavailable(res) => write!(f, "Resource unavailable: {}", res),
            Self::AlreadyInitialized => write!(f, "Already initialized"),
            Self::NotInitialized => write!(f, "Not initialized"),

            Self::InvalidBootInfo => write!(f, "Invalid boot info structure"),
            Self::MissingBootInfoField(field) => write!(f, "Missing boot info field: {}", field),
            Self::UnsupportedBootProtocol => write!(f, "Unsupported boot protocol"),
            Self::BootInfoMagicMismatch => write!(f, "Boot info magic mismatch"),
            Self::BootInfoVersionMismatch => write!(f, "Boot info version mismatch"),

            Self::UnsupportedCpu(reason) => write!(f, "Unsupported CPU: {}", reason),
            Self::MissingCpuFeature(feature) => write!(f, "Missing CPU feature: {}", feature),
            Self::CpuInitFailed(reason) => write!(f, "CPU init failed: {}", reason),
            Self::InvalidPrivilegeLevel => write!(f, "Invalid privilege level"),
            Self::FpuInitFailed => write!(f, "FPU initialization failed"),

            Self::NoMemoryMap => write!(f, "No memory map available"),
            Self::EmptyMemoryMap => write!(f, "Memory map is empty"),
            Self::InvalidMemoryMapEntry => write!(f, "Invalid memory map entry"),
            Self::InsufficientMemory => write!(f, "Insufficient physical memory"),
            Self::PageTableAllocationFailed => write!(f, "Page table allocation failed"),
            Self::PageTableSetupFailed(reason) => write!(f, "Page table setup failed: {}", reason),
            Self::MappingFailed { virt, phys, reason } => {
                write!(f, "Mapping failed: {:#x} -> {:#x}: {}", virt, phys, reason)
            },
            Self::InvalidAddress(addr) => write!(f, "Invalid address: {:#x}", addr),
            Self::AddressNotAligned { addr, required } => {
                write!(f, "Address {:#x} not aligned to {}", addr, required)
            },
            Self::EarlyHeapFailed => write!(f, "Early heap setup failed"),
            Self::OutOfMemory => write!(f, "Out of memory"),

            Self::IdtSetupFailed(reason) => write!(f, "IDT setup failed: {}", reason),
            Self::GdtSetupFailed(reason) => write!(f, "GDT setup failed: {}", reason),
            Self::ExceptionHandlerFailed(vec) => {
                write!(
                    f,
                    "Exception handler registration failed for vector {}",
                    vec
                )
            },
            Self::IrqControllerNotFound => write!(f, "IRQ controller not found"),
            Self::IrqControllerInitFailed(reason) => {
                write!(f, "IRQ controller init failed: {}", reason)
            },
            Self::InvalidInterruptVector(vec) => write!(f, "Invalid interrupt vector: {}", vec),

            Self::NoTimerAvailable => write!(f, "No timer available"),
            Self::TimerInitFailed(reason) => write!(f, "Timer init failed: {}", reason),
            Self::TimerCalibrationFailed => write!(f, "Timer calibration failed"),
            Self::TimerFrequencyDetectionFailed => write!(f, "Timer frequency detection failed"),

            Self::SmpNotAvailable => write!(f, "SMP not available"),
            Self::ApStartupFailed { cpu_id, reason } => {
                write!(f, "AP {} startup failed: {}", cpu_id, reason)
            },
            Self::PerCpuAllocationFailed => write!(f, "Per-CPU data allocation failed"),
            Self::TopologyDetectionFailed => write!(f, "CPU topology detection failed"),
            Self::TooManyCpus { detected, max } => {
                write!(f, "Too many CPUs: {} detected, max {}", detected, max)
            },

            Self::SerialInitFailed(reason) => write!(f, "Serial init failed: {}", reason),
            Self::FramebufferInitFailed(reason) => write!(f, "Framebuffer init failed: {}", reason),
            Self::ConsoleInitFailed(reason) => write!(f, "Console init failed: {}", reason),
            Self::DriverNotFound(driver) => write!(f, "Driver not found: {}", driver),

            Self::AcpiNotAvailable => write!(f, "ACPI not available"),
            Self::AcpiTableNotFound(table) => write!(f, "ACPI table not found: {}", table),
            Self::InvalidAcpiTable(table) => write!(f, "Invalid ACPI table: {}", table),
            Self::SmbiosNotAvailable => write!(f, "SMBIOS not available"),
            Self::DeviceTreeNotAvailable => write!(f, "Device tree not available"),
            Self::InvalidDeviceTree => write!(f, "Invalid device tree"),

            Self::RelocationFailed(reason) => write!(f, "Relocation failed: {}", reason),
            Self::KaslrEntropyFailed => write!(f, "KASLR entropy generation failed"),
            Self::KernelTooLarge { size, max } => {
                write!(f, "Kernel too large: {} bytes, max {}", size, max)
            },

            Self::KernelEntryNotFound => write!(f, "Kernel entry point not found"),
            Self::InvalidKernelEntry(addr) => write!(f, "Invalid kernel entry: {:#x}", addr),
            Self::HandoffPreparationFailed(reason) => {
                write!(f, "Handoff preparation failed: {}", reason)
            },

            #[cfg(target_arch = "x86_64")]
            Self::X86Error(e) => write!(f, "x86 error: {}", e),

            #[cfg(target_arch = "aarch64")]
            Self::ArmError(e) => write!(f, "ARM error: {}", e),

            #[cfg(target_arch = "riscv64")]
            Self::RiscvError(e) => write!(f, "RISC-V error: {}", e),
        }
    }
}

// =============================================================================
// Architecture-Specific Errors
// =============================================================================

/// x86_64 specific boot errors
#[cfg(target_arch = "x86_64")]
#[derive(Debug, Clone)]
pub enum X86BootError {
    /// Long mode not supported
    NoLongMode,
    /// PAE not supported
    NoPae,
    /// NX bit not supported
    NoNx,
    /// APIC not available
    NoApic,
    /// x2APIC not supported but required
    NoX2Apic,
    /// CPUID not supported
    NoCpuid,
    /// MSR read/write failed
    MsrAccessFailed(u32),
    /// CR4 feature not available
    Cr4FeatureUnavailable(&'static str),
    /// XSAVE area too small
    XsaveAreaTooSmall,
    /// TSS setup failed
    TssSetupFailed,
    /// APIC timer calibration failed
    ApicTimerCalibrationFailed,
    /// I/O APIC not found
    NoIoApic,
    /// HPET not found
    NoHpet,
}

#[cfg(target_arch = "x86_64")]
impl fmt::Display for X86BootError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoLongMode => write!(f, "Long mode not supported"),
            Self::NoPae => write!(f, "PAE not supported"),
            Self::NoNx => write!(f, "NX bit not supported"),
            Self::NoApic => write!(f, "APIC not available"),
            Self::NoX2Apic => write!(f, "x2APIC not supported"),
            Self::NoCpuid => write!(f, "CPUID not supported"),
            Self::MsrAccessFailed(msr) => write!(f, "MSR {:#x} access failed", msr),
            Self::Cr4FeatureUnavailable(feature) => {
                write!(f, "CR4 feature unavailable: {}", feature)
            },
            Self::XsaveAreaTooSmall => write!(f, "XSAVE area too small"),
            Self::TssSetupFailed => write!(f, "TSS setup failed"),
            Self::ApicTimerCalibrationFailed => write!(f, "APIC timer calibration failed"),
            Self::NoIoApic => write!(f, "I/O APIC not found"),
            Self::NoHpet => write!(f, "HPET not found"),
        }
    }
}

/// AArch64 specific boot errors
#[cfg(target_arch = "aarch64")]
#[derive(Debug, Clone)]
pub enum ArmBootError {
    /// Not running at EL1
    NotAtEl1,
    /// Cannot drop to EL1 from current EL
    CannotDropToEl1,
    /// GIC not found
    NoGic,
    /// GIC version not supported
    GicVersionNotSupported(u8),
    /// PSCI not available
    NoPsci,
    /// PSCI call failed
    PsciCallFailed { function: u32, error: i32 },
    /// Generic timer not available
    NoGenericTimer,
    /// SVE not supported but required
    NoSve,
    /// Granule size not supported
    GranuleNotSupported(usize),
    /// TCR configuration failed
    TcrConfigFailed,
    /// MAIR configuration failed
    MairConfigFailed,
}

#[cfg(target_arch = "aarch64")]
impl fmt::Display for ArmBootError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAtEl1 => write!(f, "Not running at EL1"),
            Self::CannotDropToEl1 => write!(f, "Cannot drop to EL1"),
            Self::NoGic => write!(f, "GIC not found"),
            Self::GicVersionNotSupported(v) => write!(f, "GIC v{} not supported", v),
            Self::NoPsci => write!(f, "PSCI not available"),
            Self::PsciCallFailed { function, error } => {
                write!(f, "PSCI function {:#x} failed: {}", function, error)
            },
            Self::NoGenericTimer => write!(f, "Generic timer not available"),
            Self::NoSve => write!(f, "SVE not supported"),
            Self::GranuleNotSupported(size) => write!(f, "{}KB granule not supported", size / 1024),
            Self::TcrConfigFailed => write!(f, "TCR configuration failed"),
            Self::MairConfigFailed => write!(f, "MAIR configuration failed"),
        }
    }
}

/// RISC-V specific boot errors
#[cfg(target_arch = "riscv64")]
#[derive(Debug, Clone)]
pub enum RiscvBootError {
    /// Not running in S-mode
    NotInSupervisorMode,
    /// SBI not available
    NoSbi,
    /// SBI call failed
    SbiCallFailed {
        extension: u32,
        function: u32,
        error: i64,
    },
    /// SBI extension not available
    SbiExtensionNotAvailable(u32),
    /// PLIC not found
    NoPlic,
    /// CLINT not found
    NoClint,
    /// Required ISA extension missing
    MissingIsaExtension(char),
    /// Sv39/48/57 not supported
    PagingModeNotSupported(&'static str),
    /// Hart ID invalid
    InvalidHartId(u64),
    /// Timer frequency unknown
    TimerFrequencyUnknown,
}

#[cfg(target_arch = "riscv64")]
impl fmt::Display for RiscvBootError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotInSupervisorMode => write!(f, "Not running in S-mode"),
            Self::NoSbi => write!(f, "SBI not available"),
            Self::SbiCallFailed {
                extension,
                function,
                error,
            } => {
                write!(
                    f,
                    "SBI call {:#x}:{:#x} failed: {}",
                    extension, function, error
                )
            },
            Self::SbiExtensionNotAvailable(ext) => {
                write!(f, "SBI extension {:#x} not available", ext)
            },
            Self::NoPlic => write!(f, "PLIC not found"),
            Self::NoClint => write!(f, "CLINT not found"),
            Self::MissingIsaExtension(ext) => write!(f, "Missing ISA extension: {}", ext),
            Self::PagingModeNotSupported(mode) => write!(f, "Paging mode {} not supported", mode),
            Self::InvalidHartId(id) => write!(f, "Invalid hart ID: {}", id),
            Self::TimerFrequencyUnknown => write!(f, "Timer frequency unknown"),
        }
    }
}

// =============================================================================
// ERROR UTILITIES
// =============================================================================

impl BootError {
    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            // Non-recoverable errors
            Self::UnsupportedCpu(_)
            | Self::MissingCpuFeature(_)
            | Self::NoMemoryMap
            | Self::InsufficientMemory
            | Self::PageTableSetupFailed(_)
            | Self::KernelEntryNotFound
            | Self::InvalidKernelEntry(_) => false,

            // Potentially recoverable
            Self::Timeout(_)
            | Self::TimerCalibrationFailed
            | Self::SmpNotAvailable
            | Self::ApStartupFailed { .. }
            | Self::SerialInitFailed(_)
            | Self::FramebufferInitFailed(_) => true,

            // Architecture-specific
            #[cfg(target_arch = "x86_64")]
            Self::X86Error(e) => matches!(e, X86BootError::NoHpet | X86BootError::NoIoApic),

            #[cfg(target_arch = "aarch64")]
            Self::ArmError(e) => matches!(
                e,
                ArmBootError::NoSve | ArmBootError::GranuleNotSupported(_)
            ),

            #[cfg(target_arch = "riscv64")]
            Self::RiscvError(e) => matches!(
                e,
                RiscvBootError::SbiExtensionNotAvailable(_) | RiscvBootError::TimerFrequencyUnknown
            ),

            // Default to non-recoverable
            _ => false,
        }
    }

    /// Get error code for this error
    pub fn error_code(&self) -> u32 {
        // Unique error codes for debugging
        match self {
            Self::Failed(_) => 0x0001,
            Self::NotSupported(_) => 0x0002,
            Self::InvalidParameter(_) => 0x0003,
            Self::Timeout(_) => 0x0004,
            Self::ResourceUnavailable(_) => 0x0005,
            Self::AlreadyInitialized => 0x0006,
            Self::NotInitialized => 0x0007,

            Self::InvalidBootInfo => 0x0100,
            Self::MissingBootInfoField(_) => 0x0101,
            Self::UnsupportedBootProtocol => 0x0102,
            Self::BootInfoMagicMismatch => 0x0103,
            Self::BootInfoVersionMismatch => 0x0104,

            Self::UnsupportedCpu(_) => 0x0200,
            Self::MissingCpuFeature(_) => 0x0201,
            Self::CpuInitFailed(_) => 0x0202,
            Self::InvalidPrivilegeLevel => 0x0203,
            Self::FpuInitFailed => 0x0204,

            Self::NoMemoryMap => 0x0300,
            Self::EmptyMemoryMap => 0x0301,
            Self::InvalidMemoryMapEntry => 0x0302,
            Self::InsufficientMemory => 0x0303,
            Self::PageTableAllocationFailed => 0x0304,
            Self::PageTableSetupFailed(_) => 0x0305,
            Self::MappingFailed { .. } => 0x0306,
            Self::InvalidAddress(_) => 0x0307,
            Self::AddressNotAligned { .. } => 0x0308,
            Self::EarlyHeapFailed => 0x0309,
            Self::OutOfMemory => 0x030A,

            Self::IdtSetupFailed(_) => 0x0400,
            Self::GdtSetupFailed(_) => 0x0401,
            Self::ExceptionHandlerFailed(_) => 0x0402,
            Self::IrqControllerNotFound => 0x0403,
            Self::IrqControllerInitFailed(_) => 0x0404,
            Self::InvalidInterruptVector(_) => 0x0405,

            Self::NoTimerAvailable => 0x0500,
            Self::TimerInitFailed(_) => 0x0501,
            Self::TimerCalibrationFailed => 0x0502,
            Self::TimerFrequencyDetectionFailed => 0x0503,

            Self::SmpNotAvailable => 0x0600,
            Self::ApStartupFailed { .. } => 0x0601,
            Self::PerCpuAllocationFailed => 0x0602,
            Self::TopologyDetectionFailed => 0x0603,
            Self::TooManyCpus { .. } => 0x0604,

            Self::SerialInitFailed(_) => 0x0700,
            Self::FramebufferInitFailed(_) => 0x0701,
            Self::ConsoleInitFailed(_) => 0x0702,
            Self::DriverNotFound(_) => 0x0703,

            Self::AcpiNotAvailable => 0x0800,
            Self::AcpiTableNotFound(_) => 0x0801,
            Self::InvalidAcpiTable(_) => 0x0802,
            Self::SmbiosNotAvailable => 0x0803,
            Self::DeviceTreeNotAvailable => 0x0804,
            Self::InvalidDeviceTree => 0x0805,

            Self::RelocationFailed(_) => 0x0900,
            Self::KaslrEntropyFailed => 0x0901,
            Self::KernelTooLarge { .. } => 0x0902,

            Self::KernelEntryNotFound => 0x0A00,
            Self::InvalidKernelEntry(_) => 0x0A01,
            Self::HandoffPreparationFailed(_) => 0x0A02,

            #[cfg(target_arch = "x86_64")]
            Self::X86Error(_) => 0x1000,

            #[cfg(target_arch = "aarch64")]
            Self::ArmError(_) => 0x2000,

            #[cfg(target_arch = "riscv64")]
            Self::RiscvError(_) => 0x3000,
        }
    }
}

/// Macro for early boot assertions with custom error
#[macro_export]
macro_rules! boot_assert {
    ($cond:expr, $err:expr) => {
        if !($cond) {
            return Err($err);
        }
    };
    ($cond:expr) => {
        if !($cond) {
            return Err($crate::error::BootError::Failed("assertion failed"));
        }
    };
}

/// Macro for early boot require (unwrap with error context)
#[macro_export]
macro_rules! boot_require {
    ($opt:expr, $err:expr) => {
        match $opt {
            Some(v) => v,
            None => return Err($err),
        }
    };
}
