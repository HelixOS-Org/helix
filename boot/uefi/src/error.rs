//! Error types for the UEFI boot platform
//!
//! This module provides comprehensive error handling for all UEFI operations,
//! mapping UEFI status codes to Rust-idiomatic error types.

use core::fmt;

use crate::raw::types::Status;

/// Result type for UEFI operations
pub type Result<T> = core::result::Result<T, Error>;

/// UEFI Error type
///
/// Comprehensive error type covering all possible failure modes
/// during UEFI boot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    // =========================================================================
    // UEFI Status Errors
    // =========================================================================
    /// The image failed to load
    LoadError,

    /// A parameter was incorrect
    InvalidParameter,

    /// The operation is not supported
    Unsupported,

    /// The buffer was too small
    BadBufferSize,

    /// The buffer is not large enough
    BufferTooSmall,

    /// There is no pending request
    NotReady,

    /// The physical device reported an error
    DeviceError,

    /// The device cannot be written to
    WriteProtected,

    /// A resource has run out
    OutOfResources,

    /// An inconsistency was detected
    VolumeCorrupted,

    /// There is no more space on the file system
    VolumeFull,

    /// The device does not contain any medium
    NoMedia,

    /// The medium in the device has changed
    MediaChanged,

    /// The item was not found
    NotFound,

    /// Access was denied
    AccessDenied,

    /// The server was not found
    NoResponse,

    /// A mapping to a device does not exist
    NoMapping,

    /// The timeout time expired
    Timeout,

    /// The protocol has not been started
    NotStarted,

    /// The protocol has already been started
    AlreadyStarted,

    /// The operation was aborted
    Aborted,

    /// An ICMP error occurred during the network operation
    IcmpError,

    /// A TFTP error occurred during the network operation
    TftpError,

    /// A protocol error occurred during the network operation
    ProtocolError,

    /// The function encountered an internal version that was
    /// incompatible with a version requested by the caller
    IncompatibleVersion,

    /// The function was not performed due to a security violation
    SecurityViolation,

    /// A CRC error was detected
    CrcError,

    /// Beginning or end of media was reached
    EndOfMedia,

    /// The end of the file was reached
    EndOfFile,

    /// The language specified was invalid
    InvalidLanguage,

    /// The security status of the data is unknown
    CompromisedData,

    /// There is an address conflict in the IP address
    IpAddressConflict,

    /// A HTTP error occurred during the network operation
    HttpError,

    // =========================================================================
    // Helix-Specific Errors
    // =========================================================================
    /// System table is invalid
    InvalidSystemTable,

    /// Boot services are not available (already exited)
    BootServicesUnavailable,

    /// Protocol not found
    ProtocolNotFound,

    /// Invalid memory map
    InvalidMemoryMap,

    /// Memory allocation failed
    AllocationFailed,

    /// Invalid ELF file
    InvalidElf,

    /// Invalid PE file
    InvalidPe,

    /// Kernel entry point not found
    KernelEntryNotFound,

    /// ACPI tables not found
    AcpiNotFound,

    /// Invalid ACPI table
    InvalidAcpi,

    /// SMBIOS tables not found
    SmbiosNotFound,

    /// Invalid SMBIOS table
    InvalidSmbios,

    /// Secure boot validation failed
    SecureBootViolation,

    /// Signature verification failed
    SignatureInvalid,

    /// TPM operation failed
    TpmError,

    /// File system error
    FileSystemError,

    /// File not found
    FileNotFound,

    /// Invalid file path
    InvalidPath,

    /// Graphics initialization failed
    GraphicsError,

    /// No suitable video mode found
    NoVideoMode,

    /// Serial I/O error
    SerialError,

    /// Configuration error
    ConfigError,

    /// Unsupported architecture
    UnsupportedArch,

    /// Memory region overlap
    MemoryOverlap,

    /// Exit boot services failed
    ExitBootServicesFailed,

    /// Invalid data encountered
    InvalidData,

    /// Relocation overflow during kernel loading
    RelocationOverflow,

    /// Unsupported format
    UnsupportedFormat,

    /// Not loaded
    NotLoaded,

    /// Item already exists
    AlreadyExists,

    /// Invalid memory address
    InvalidAddress,

    /// Invalid magic number
    InvalidMagic,

    /// Invalid signature
    InvalidSignature,

    /// Operation not supported
    NotSupported,

    /// Index or offset out of bounds
    OutOfBounds,

    /// Out of memory
    OutOfMemory,

    /// Unsupported CPU architecture
    UnsupportedArchitecture,

    /// Unsupported relocation type
    UnsupportedRelocation,

    /// Unknown error with status code
    Unknown(u64),
}

impl Error {
    /// Create error from UEFI status code
    pub fn from_status(status: Status) -> Self {
        match status.0 {
            0 => panic!("Cannot create error from success status"),
            1 => Self::LoadError,
            2 => Self::InvalidParameter,
            3 => Self::Unsupported,
            4 => Self::BadBufferSize,
            5 => Self::BufferTooSmall,
            6 => Self::NotReady,
            7 => Self::DeviceError,
            8 => Self::WriteProtected,
            9 => Self::OutOfResources,
            10 => Self::VolumeCorrupted,
            11 => Self::VolumeFull,
            12 => Self::NoMedia,
            13 => Self::MediaChanged,
            14 => Self::NotFound,
            15 => Self::AccessDenied,
            16 => Self::NoResponse,
            17 => Self::NoMapping,
            18 => Self::Timeout,
            19 => Self::NotStarted,
            20 => Self::AlreadyStarted,
            21 => Self::Aborted,
            22 => Self::IcmpError,
            23 => Self::TftpError,
            24 => Self::ProtocolError,
            25 => Self::IncompatibleVersion,
            26 => Self::SecurityViolation,
            27 => Self::CrcError,
            28 => Self::EndOfMedia,
            31 => Self::EndOfFile,
            32 => Self::InvalidLanguage,
            33 => Self::CompromisedData,
            34 => Self::IpAddressConflict,
            35 => Self::HttpError,
            code => Self::Unknown(code),
        }
    }

    /// Check if this is a not-found type error
    pub fn is_not_found(&self) -> bool {
        matches!(
            self,
            Self::NotFound
                | Self::FileNotFound
                | Self::ProtocolNotFound
                | Self::AcpiNotFound
                | Self::SmbiosNotFound
        )
    }

    /// Check if this is a resource error
    pub fn is_resource_error(&self) -> bool {
        matches!(
            self,
            Self::OutOfResources | Self::AllocationFailed | Self::VolumeFull | Self::BufferTooSmall
        )
    }

    /// Check if this is a security error
    pub fn is_security_error(&self) -> bool {
        matches!(
            self,
            Self::SecurityViolation
                | Self::SecureBootViolation
                | Self::SignatureInvalid
                | Self::CompromisedData
                | Self::AccessDenied
        )
    }

    /// Get error name as string
    pub fn name(&self) -> &'static str {
        match self {
            Self::LoadError => "LOAD_ERROR",
            Self::InvalidParameter => "INVALID_PARAMETER",
            Self::Unsupported => "UNSUPPORTED",
            Self::BadBufferSize => "BAD_BUFFER_SIZE",
            Self::BufferTooSmall => "BUFFER_TOO_SMALL",
            Self::NotReady => "NOT_READY",
            Self::DeviceError => "DEVICE_ERROR",
            Self::WriteProtected => "WRITE_PROTECTED",
            Self::OutOfResources => "OUT_OF_RESOURCES",
            Self::VolumeCorrupted => "VOLUME_CORRUPTED",
            Self::VolumeFull => "VOLUME_FULL",
            Self::NoMedia => "NO_MEDIA",
            Self::MediaChanged => "MEDIA_CHANGED",
            Self::NotFound => "NOT_FOUND",
            Self::AccessDenied => "ACCESS_DENIED",
            Self::NoResponse => "NO_RESPONSE",
            Self::NoMapping => "NO_MAPPING",
            Self::Timeout => "TIMEOUT",
            Self::NotStarted => "NOT_STARTED",
            Self::AlreadyStarted => "ALREADY_STARTED",
            Self::Aborted => "ABORTED",
            Self::IcmpError => "ICMP_ERROR",
            Self::TftpError => "TFTP_ERROR",
            Self::ProtocolError => "PROTOCOL_ERROR",
            Self::IncompatibleVersion => "INCOMPATIBLE_VERSION",
            Self::SecurityViolation => "SECURITY_VIOLATION",
            Self::CrcError => "CRC_ERROR",
            Self::EndOfMedia => "END_OF_MEDIA",
            Self::EndOfFile => "END_OF_FILE",
            Self::InvalidLanguage => "INVALID_LANGUAGE",
            Self::CompromisedData => "COMPROMISED_DATA",
            Self::IpAddressConflict => "IP_ADDRESS_CONFLICT",
            Self::HttpError => "HTTP_ERROR",
            Self::InvalidSystemTable => "INVALID_SYSTEM_TABLE",
            Self::BootServicesUnavailable => "BOOT_SERVICES_UNAVAILABLE",
            Self::ProtocolNotFound => "PROTOCOL_NOT_FOUND",
            Self::InvalidMemoryMap => "INVALID_MEMORY_MAP",
            Self::AllocationFailed => "ALLOCATION_FAILED",
            Self::InvalidElf => "INVALID_ELF",
            Self::InvalidPe => "INVALID_PE",
            Self::KernelEntryNotFound => "KERNEL_ENTRY_NOT_FOUND",
            Self::AcpiNotFound => "ACPI_NOT_FOUND",
            Self::InvalidAcpi => "INVALID_ACPI",
            Self::SmbiosNotFound => "SMBIOS_NOT_FOUND",
            Self::InvalidSmbios => "INVALID_SMBIOS",
            Self::SecureBootViolation => "SECURE_BOOT_VIOLATION",
            Self::SignatureInvalid => "SIGNATURE_INVALID",
            Self::TpmError => "TPM_ERROR",
            Self::FileSystemError => "FILE_SYSTEM_ERROR",
            Self::FileNotFound => "FILE_NOT_FOUND",
            Self::InvalidPath => "INVALID_PATH",
            Self::GraphicsError => "GRAPHICS_ERROR",
            Self::NoVideoMode => "NO_VIDEO_MODE",
            Self::SerialError => "SERIAL_ERROR",
            Self::ConfigError => "CONFIG_ERROR",
            Self::UnsupportedArch => "UNSUPPORTED_ARCH",
            Self::MemoryOverlap => "MEMORY_OVERLAP",
            Self::ExitBootServicesFailed => "EXIT_BOOT_SERVICES_FAILED",
            Self::InvalidData => "INVALID_DATA",
            Self::RelocationOverflow => "RELOCATION_OVERFLOW",
            Self::UnsupportedFormat => "UNSUPPORTED_FORMAT",
            Self::NotLoaded => "NOT_LOADED",
            Self::AlreadyExists => "ALREADY_EXISTS",
            Self::InvalidAddress => "INVALID_ADDRESS",
            Self::InvalidMagic => "INVALID_MAGIC",
            Self::InvalidSignature => "INVALID_SIGNATURE",
            Self::NotSupported => "NOT_SUPPORTED",
            Self::OutOfBounds => "OUT_OF_BOUNDS",
            Self::OutOfMemory => "OUT_OF_MEMORY",
            Self::UnsupportedArchitecture => "UNSUPPORTED_ARCHITECTURE",
            Self::UnsupportedRelocation => "UNSUPPORTED_RELOCATION",
            Self::Unknown(_) => "UNKNOWN",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(code) => write!(f, "UEFI Error: Unknown (0x{code:X})"),
            _ => write!(f, "UEFI Error: {}", self.name()),
        }
    }
}

impl From<Error> for Status {
    fn from(err: Error) -> Self {
        let code = match err {
            // Code 1: Load errors
            Error::LoadError
            | Error::InvalidElf
            | Error::InvalidPe
            | Error::RelocationOverflow
            | Error::NotLoaded => 1,

            // Code 2: Invalid parameter / data errors
            Error::InvalidParameter
            | Error::InvalidSystemTable
            | Error::InvalidMemoryMap
            | Error::InvalidAcpi
            | Error::InvalidSmbios
            | Error::InvalidPath
            | Error::ConfigError
            | Error::MemoryOverlap
            | Error::ExitBootServicesFailed
            | Error::InvalidData
            | Error::InvalidAddress
            | Error::InvalidMagic
            | Error::OutOfBounds => 2,

            // Code 3: Unsupported errors
            Error::Unsupported
            | Error::UnsupportedArch
            | Error::UnsupportedFormat
            | Error::NotSupported
            | Error::UnsupportedArchitecture
            | Error::UnsupportedRelocation => 3,

            Error::BadBufferSize => 4,
            Error::BufferTooSmall => 5,
            Error::NotReady => 6,

            // Code 7: Device errors
            Error::DeviceError
            | Error::TpmError
            | Error::FileSystemError
            | Error::GraphicsError
            | Error::SerialError => 7,

            Error::WriteProtected => 8,

            // Code 9: Resource errors
            Error::OutOfResources | Error::AllocationFailed | Error::OutOfMemory => 9,

            Error::VolumeCorrupted => 10,
            Error::VolumeFull => 11,
            Error::NoMedia => 12,
            Error::MediaChanged => 13,

            // Code 14: Not found errors
            Error::NotFound
            | Error::ProtocolNotFound
            | Error::KernelEntryNotFound
            | Error::AcpiNotFound
            | Error::SmbiosNotFound
            | Error::FileNotFound
            | Error::NoVideoMode => 14,

            Error::AccessDenied => 15,
            Error::NoResponse => 16,
            Error::NoMapping => 17,
            Error::Timeout => 18,

            // Code 19: Not started errors
            Error::NotStarted | Error::BootServicesUnavailable => 19,

            // Code 20: Already started/exists errors
            Error::AlreadyStarted | Error::AlreadyExists => 20,

            Error::Aborted => 21,
            Error::IcmpError => 22,
            Error::TftpError => 23,
            Error::ProtocolError => 24,
            Error::IncompatibleVersion => 25,

            // Code 26: Security errors
            Error::SecurityViolation
            | Error::SecureBootViolation
            | Error::SignatureInvalid
            | Error::InvalidSignature => 26,

            Error::CrcError => 27,
            Error::EndOfMedia => 28,
            Error::EndOfFile => 31,
            Error::InvalidLanguage => 32,
            Error::CompromisedData => 33,
            Error::IpAddressConflict => 34,
            Error::HttpError => 35,
            Error::Unknown(code) => code,
        };

        // UEFI error codes have the high bit set
        Status(code | (1u64 << 63))
    }
}

impl From<Status> for Result<()> {
    fn from(status: Status) -> Self {
        if status.is_success() {
            Ok(())
        } else {
            Err(Error::from_status(status))
        }
    }
}

/// Trait for converting results with additional context
pub trait ResultExt<T> {
    /// Add context to an error
    fn context(self, msg: &'static str) -> Result<T>;

    /// Convert not-found errors to None
    fn optional(self) -> Result<Option<T>>;
}

impl<T> ResultExt<T> for Result<T> {
    fn context(self, _msg: &'static str) -> Result<T> {
        // In no_std we can't easily add context, but this is a placeholder
        // for when we have a more sophisticated error type
        self
    }

    fn optional(self) -> Result<Option<T>> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(e) if e.is_not_found() => Ok(None),
            Err(e) => Err(e),
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_names() {
        assert_eq!(Error::NotFound.name(), "NOT_FOUND");
        assert_eq!(Error::OutOfResources.name(), "OUT_OF_RESOURCES");
    }

    #[test]
    fn test_error_categories() {
        assert!(Error::NotFound.is_not_found());
        assert!(Error::OutOfResources.is_resource_error());
        assert!(Error::SecurityViolation.is_security_error());
    }

    #[test]
    fn test_status_conversion() {
        let status: Status = Error::NotFound.into();
        assert!(!status.is_success());
    }
}
