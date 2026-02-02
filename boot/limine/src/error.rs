//! # Error Types
//!
//! Comprehensive error types for the Limine crate.

use core::fmt;

/// Main error type for Limine operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A response was not provided by the bootloader
    NoResponse(&'static str),
    /// Invalid response data
    InvalidResponse {
        request: &'static str,
        reason: &'static str,
    },
    /// Feature not supported
    NotSupported(&'static str),
    /// Invalid parameter
    InvalidParameter {
        param: &'static str,
        reason: &'static str,
    },
    /// Memory error
    Memory(MemoryError),
    /// SMP error
    Smp(SmpError),
    /// Framebuffer error
    Framebuffer(FramebufferError),
    /// Firmware error
    Firmware(FirmwareError),
    /// Boot info error
    BootInfo(crate::boot_info::BootInfoError),
    /// Validation error
    Validation(crate::validate::ValidationError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoResponse(name) => write!(f, "No response for request: {}", name),
            Self::InvalidResponse { request, reason } => {
                write!(f, "Invalid response for {}: {}", request, reason)
            },
            Self::NotSupported(feature) => write!(f, "Feature not supported: {}", feature),
            Self::InvalidParameter { param, reason } => {
                write!(f, "Invalid parameter '{}': {}", param, reason)
            },
            Self::Memory(e) => write!(f, "Memory error: {}", e),
            Self::Smp(e) => write!(f, "SMP error: {}", e),
            Self::Framebuffer(e) => write!(f, "Framebuffer error: {}", e),
            Self::Firmware(e) => write!(f, "Firmware error: {}", e),
            Self::BootInfo(e) => write!(f, "Boot info error: {}", e),
            Self::Validation(e) => write!(f, "Validation error: {}", e),
        }
    }
}

impl From<MemoryError> for Error {
    fn from(e: MemoryError) -> Self {
        Self::Memory(e)
    }
}

impl From<SmpError> for Error {
    fn from(e: SmpError) -> Self {
        Self::Smp(e)
    }
}

impl From<FramebufferError> for Error {
    fn from(e: FramebufferError) -> Self {
        Self::Framebuffer(e)
    }
}

impl From<FirmwareError> for Error {
    fn from(e: FirmwareError) -> Self {
        Self::Firmware(e)
    }
}

impl From<crate::boot_info::BootInfoError> for Error {
    fn from(e: crate::boot_info::BootInfoError) -> Self {
        Self::BootInfo(e)
    }
}

impl From<crate::validate::ValidationError> for Error {
    fn from(e: crate::validate::ValidationError) -> Self {
        Self::Validation(e)
    }
}

/// Memory-related errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryError {
    /// Memory map not available
    NoMemoryMap,
    /// HHDM not available
    NoHhdm,
    /// No usable memory found
    NoUsableMemory,
    /// Insufficient memory for operation
    InsufficientMemory { required: u64, available: u64 },
    /// Address out of range
    AddressOutOfRange(u64),
    /// Invalid memory region
    InvalidRegion { base: u64, length: u64 },
    /// Overlapping memory regions
    OverlappingRegions,
    /// Memory region not found
    RegionNotFound,
    /// Allocation failed
    AllocationFailed { size: usize, align: usize },
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoMemoryMap => write!(f, "Memory map not available"),
            Self::NoHhdm => write!(f, "HHDM not available"),
            Self::NoUsableMemory => write!(f, "No usable memory found"),
            Self::InsufficientMemory {
                required,
                available,
            } => {
                write!(
                    f,
                    "Insufficient memory: need {} bytes, have {}",
                    required, available
                )
            },
            Self::AddressOutOfRange(addr) => write!(f, "Address out of range: {:#x}", addr),
            Self::InvalidRegion { base, length } => {
                write!(f, "Invalid memory region: {:#x}-{:#x}", base, base + length)
            },
            Self::OverlappingRegions => write!(f, "Overlapping memory regions"),
            Self::RegionNotFound => write!(f, "Memory region not found"),
            Self::AllocationFailed { size, align } => {
                write!(f, "Allocation failed: size={}, align={}", size, align)
            },
        }
    }
}

/// SMP-related errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmpError {
    /// SMP not available
    NotAvailable,
    /// CPU not found
    CpuNotFound(u32),
    /// CPU already started
    CpuAlreadyStarted(u32),
    /// Failed to start CPU
    StartupFailed(u32),
    /// Invalid CPU ID
    InvalidCpuId(u32),
    /// Too many CPUs
    TooManyCpus { count: usize, max: usize },
}

impl fmt::Display for SmpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAvailable => write!(f, "SMP not available"),
            Self::CpuNotFound(id) => write!(f, "CPU {} not found", id),
            Self::CpuAlreadyStarted(id) => write!(f, "CPU {} already started", id),
            Self::StartupFailed(id) => write!(f, "Failed to start CPU {}", id),
            Self::InvalidCpuId(id) => write!(f, "Invalid CPU ID: {}", id),
            Self::TooManyCpus { count, max } => {
                write!(f, "Too many CPUs: {} (max {})", count, max)
            },
        }
    }
}

/// Framebuffer-related errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FramebufferError {
    /// No framebuffer available
    NotAvailable,
    /// Framebuffer index out of range
    IndexOutOfRange(usize),
    /// Unsupported pixel format
    UnsupportedFormat { bpp: u16 },
    /// Invalid coordinates
    InvalidCoordinates { x: usize, y: usize },
    /// Buffer too small
    BufferTooSmall { required: usize, provided: usize },
    /// No video modes available
    NoVideoModes,
}

impl fmt::Display for FramebufferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAvailable => write!(f, "Framebuffer not available"),
            Self::IndexOutOfRange(idx) => write!(f, "Framebuffer index {} out of range", idx),
            Self::UnsupportedFormat { bpp } => write!(f, "Unsupported pixel format: {} bpp", bpp),
            Self::InvalidCoordinates { x, y } => write!(f, "Invalid coordinates: ({}, {})", x, y),
            Self::BufferTooSmall { required, provided } => {
                write!(
                    f,
                    "Buffer too small: need {} bytes, got {}",
                    required, provided
                )
            },
            Self::NoVideoModes => write!(f, "No video modes available"),
        }
    }
}

/// Firmware-related errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirmwareError {
    /// ACPI not available
    NoAcpi,
    /// Invalid RSDP
    InvalidRsdp,
    /// SMBIOS not available
    NoSmbios,
    /// Invalid SMBIOS tables
    InvalidSmbios,
    /// EFI not available
    NoEfi,
    /// Invalid EFI data
    InvalidEfi(&'static str),
    /// Device tree not available
    NoDeviceTree,
    /// Invalid device tree
    InvalidDeviceTree,
}

impl fmt::Display for FirmwareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoAcpi => write!(f, "ACPI not available"),
            Self::InvalidRsdp => write!(f, "Invalid RSDP"),
            Self::NoSmbios => write!(f, "SMBIOS not available"),
            Self::InvalidSmbios => write!(f, "Invalid SMBIOS tables"),
            Self::NoEfi => write!(f, "EFI not available"),
            Self::InvalidEfi(reason) => write!(f, "Invalid EFI data: {}", reason),
            Self::NoDeviceTree => write!(f, "Device tree not available"),
            Self::InvalidDeviceTree => write!(f, "Invalid device tree"),
        }
    }
}

/// Result type alias for Limine operations
pub type Result<T> = core::result::Result<T, Error>;

/// Result type for memory operations
pub type MemoryResult<T> = core::result::Result<T, MemoryError>;

/// Result type for SMP operations
pub type SmpResult<T> = core::result::Result<T, SmpError>;

/// Result type for framebuffer operations
pub type FramebufferResult<T> = core::result::Result<T, FramebufferError>;

/// Result type for firmware operations
pub type FirmwareResult<T> = core::result::Result<T, FirmwareError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = Error::NoResponse("memory_map");
        let s = alloc::format!("{}", error);
        assert!(s.contains("memory_map"));
    }

    #[test]
    fn test_error_conversion() {
        let mem_error = MemoryError::NoMemoryMap;
        let error: Error = mem_error.into();
        assert!(matches!(error, Error::Memory(MemoryError::NoMemoryMap)));
    }
}
