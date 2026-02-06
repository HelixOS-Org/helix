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
        /// The name of the request that received an invalid response
        request: &'static str,
        /// The reason why the response is invalid
        reason: &'static str,
    },
    /// Feature not supported
    NotSupported(&'static str),
    /// Invalid parameter
    InvalidParameter {
        /// The name of the invalid parameter
        param: &'static str,
        /// The reason why the parameter is invalid
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
            Self::NoResponse(name) => write!(f, "No response for request: {name}"),
            Self::InvalidResponse { request, reason } => {
                write!(f, "Invalid response for {request}: {reason}")
            },
            Self::NotSupported(feature) => write!(f, "Feature not supported: {feature}"),
            Self::InvalidParameter { param, reason } => {
                write!(f, "Invalid parameter '{param}': {reason}")
            },
            Self::Memory(e) => write!(f, "Memory error: {e}"),
            Self::Smp(e) => write!(f, "SMP error: {e}"),
            Self::Framebuffer(e) => write!(f, "Framebuffer error: {e}"),
            Self::Firmware(e) => write!(f, "Firmware error: {e}"),
            Self::BootInfo(e) => write!(f, "Boot info error: {e}"),
            Self::Validation(e) => write!(f, "Validation error: {e}"),
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
    /// HHDM (Higher Half Direct Map) not available
    NoHhdm,
    /// No usable memory found
    NoUsableMemory,
    /// Insufficient memory for operation
    InsufficientMemory {
        /// The number of bytes required for the operation
        required: u64,
        /// The number of bytes currently available
        available: u64,
    },
    /// Address out of range
    AddressOutOfRange(u64),
    /// Invalid memory region
    InvalidRegion {
        /// Base address of the invalid region
        base: u64,
        /// Length of the invalid region in bytes
        length: u64,
    },
    /// Overlapping memory regions detected
    OverlappingRegions,
    /// Requested memory region not found
    RegionNotFound,
    /// Memory allocation failed
    AllocationFailed {
        /// Requested allocation size in bytes
        size: usize,
        /// Requested alignment in bytes
        align: usize,
    },
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
                    "Insufficient memory: need {required} bytes, have {available}"
                )
            },
            Self::AddressOutOfRange(addr) => write!(f, "Address out of range: {addr:#x}"),
            Self::InvalidRegion { base, length } => {
                let end = base + length;
                write!(f, "Invalid memory region: {base:#x}-{end:#x}")
            },
            Self::OverlappingRegions => write!(f, "Overlapping memory regions"),
            Self::RegionNotFound => write!(f, "Memory region not found"),
            Self::AllocationFailed { size, align } => {
                write!(f, "Allocation failed: size={size}, align={align}")
            },
        }
    }
}

/// SMP (Symmetric Multi-Processing) related errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmpError {
    /// SMP functionality is not available on this system
    NotAvailable,
    /// The specified CPU was not found in the system
    CpuNotFound(u32),
    /// The CPU has already been started and cannot be started again
    CpuAlreadyStarted(u32),
    /// Failed to start the specified CPU
    StartupFailed(u32),
    /// The provided CPU ID is invalid
    InvalidCpuId(u32),
    /// The number of CPUs exceeds the maximum supported
    TooManyCpus {
        /// The actual number of CPUs detected
        count: usize,
        /// The maximum number of CPUs supported
        max: usize,
    },
}

impl fmt::Display for SmpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAvailable => write!(f, "SMP not available"),
            Self::CpuNotFound(id) => write!(f, "CPU {id} not found"),
            Self::CpuAlreadyStarted(id) => write!(f, "CPU {id} already started"),
            Self::StartupFailed(id) => write!(f, "Failed to start CPU {id}"),
            Self::InvalidCpuId(id) => write!(f, "Invalid CPU ID: {id}"),
            Self::TooManyCpus { count, max } => {
                write!(f, "Too many CPUs: {count} (max {max})")
            },
        }
    }
}

/// Framebuffer-related errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FramebufferError {
    /// No framebuffer is available on this system
    NotAvailable,
    /// The requested framebuffer index is out of range
    IndexOutOfRange(usize),
    /// The pixel format is not supported
    UnsupportedFormat {
        /// Bits per pixel of the unsupported format
        bpp: u16,
    },
    /// The specified coordinates are outside the framebuffer bounds
    InvalidCoordinates {
        /// The x coordinate that was invalid
        x: usize,
        /// The y coordinate that was invalid
        y: usize,
    },
    /// The provided buffer is too small for the operation
    BufferTooSmall {
        /// The number of bytes required
        required: usize,
        /// The number of bytes provided
        provided: usize,
    },
    /// No video modes are available
    NoVideoModes,
}

impl fmt::Display for FramebufferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAvailable => write!(f, "Framebuffer not available"),
            Self::IndexOutOfRange(idx) => write!(f, "Framebuffer index {idx} out of range"),
            Self::UnsupportedFormat { bpp } => write!(f, "Unsupported pixel format: {bpp} bpp"),
            Self::InvalidCoordinates { x, y } => write!(f, "Invalid coordinates: ({x}, {y})"),
            Self::BufferTooSmall { required, provided } => {
                write!(f, "Buffer too small: need {required} bytes, got {provided}")
            },
            Self::NoVideoModes => write!(f, "No video modes available"),
        }
    }
}

/// Firmware-related errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirmwareError {
    /// ACPI tables are not available on this system
    NoAcpi,
    /// The RSDP (Root System Description Pointer) is invalid or malformed
    InvalidRsdp,
    /// SMBIOS tables are not available on this system
    NoSmbios,
    /// The SMBIOS tables are invalid or corrupted
    InvalidSmbios,
    /// EFI/UEFI services are not available
    NoEfi,
    /// EFI data is invalid or malformed
    InvalidEfi(&'static str),
    /// Device tree (FDT) is not available on this system
    NoDeviceTree,
    /// The device tree is invalid or malformed
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
            Self::InvalidEfi(reason) => write!(f, "Invalid EFI data: {reason}"),
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
