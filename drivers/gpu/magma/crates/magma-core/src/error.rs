//! # MAGMA Error Handling
//!
//! Comprehensive error types for the GPU driver stack.
//!
//! Error handling in MAGMA follows these principles:
//! - Errors are typed and categorized
//! - No panics in production code paths
//! - Errors carry context for debugging
//! - Errors are `no_std` compatible

use core::fmt;

// =============================================================================
// RESULT TYPE
// =============================================================================

/// MAGMA Result type alias
pub type Result<T> = core::result::Result<T, Error>;

// =============================================================================
// ERROR ENUM
// =============================================================================

/// MAGMA unified error type
///
/// This enum covers all error conditions across the driver stack.
/// Errors are categorized by subsystem for easier debugging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    // =========================================================================
    // Generic Errors
    // =========================================================================
    /// Operation not yet implemented
    NotImplemented,
    /// Invalid parameter provided
    InvalidParameter,
    /// Resource not found
    NotFound,
    /// Operation timed out
    Timeout,
    /// Operation was interrupted
    Interrupted,
    /// Resource is busy
    Busy,
    /// Access denied
    AccessDenied,
    /// Operation not supported on this hardware
    NotSupported,

    // =========================================================================
    // Hardware Errors
    // =========================================================================
    /// GPU device not found
    GpuNotFound,
    /// GPU device is in a bad state
    GpuFault,
    /// GPU hang detected
    GpuHang,
    /// GPU reset required
    GpuReset,
    /// Invalid GPU generation/architecture
    InvalidGeneration,
    /// PCI configuration error
    PciError(PciError),
    /// BAR mapping failed
    BarMappingFailed,
    /// MMIO access error
    MmioError,

    // =========================================================================
    // Memory Errors
    // =========================================================================
    /// Out of VRAM
    OutOfVram,
    /// Out of system memory
    OutOfMemory,
    /// Invalid GPU address
    InvalidGpuAddress,
    /// Allocation failed
    AllocationFailed,
    /// Address not aligned
    MisalignedAddress,
    /// Memory mapping failed
    MappingFailed,
    /// Buffer overflow
    BufferOverflow,
    /// Buffer underflow
    BufferUnderflow,

    // =========================================================================
    // GSP/Firmware Errors
    // =========================================================================
    /// GSP firmware not found
    GspNotFound,
    /// GSP handshake failed
    GspHandshakeFailed,
    /// GSP authentication failed
    GspAuthFailed,
    /// GSP communication timeout
    GspTimeout,
    /// GSP returned error
    GspError(GspErrorCode),
    /// Invalid RPC message
    InvalidRpcMessage,
    /// RPC channel full
    RpcChannelFull,

    // =========================================================================
    // Command Submission Errors
    // =========================================================================
    /// Command buffer full
    CommandBufferFull,
    /// Invalid command
    InvalidCommand,
    /// Command submission failed
    SubmissionFailed,
    /// Fence wait timeout
    FenceTimeout,
    /// Ring buffer overflow
    RingOverflow,
    /// Push buffer error
    PushBufferError,

    // =========================================================================
    // Vulkan-Specific Errors
    // =========================================================================
    /// Vulkan instance creation failed
    VkInstanceFailed,
    /// Vulkan device creation failed
    VkDeviceFailed,
    /// Invalid Vulkan handle
    VkInvalidHandle,
    /// Vulkan extension not supported
    VkExtensionNotSupported,
    /// Vulkan feature not supported
    VkFeatureNotSupported,
    /// Shader compilation failed
    ShaderCompilationFailed,
    /// Pipeline creation failed
    PipelineCreationFailed,

    // =========================================================================
    // Display Errors
    // =========================================================================
    /// No display connected
    NoDisplay,
    /// Invalid display mode
    InvalidDisplayMode,
    /// Scanout failed
    ScanoutFailed,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Generic
            Self::NotImplemented => write!(f, "operation not implemented"),
            Self::InvalidParameter => write!(f, "invalid parameter"),
            Self::NotFound => write!(f, "resource not found"),
            Self::Timeout => write!(f, "operation timed out"),
            Self::Interrupted => write!(f, "operation interrupted"),
            Self::Busy => write!(f, "resource busy"),
            Self::AccessDenied => write!(f, "access denied"),
            Self::NotSupported => write!(f, "operation not supported"),

            // Hardware
            Self::GpuNotFound => write!(f, "GPU device not found"),
            Self::GpuFault => write!(f, "GPU fault detected"),
            Self::GpuHang => write!(f, "GPU hang detected"),
            Self::GpuReset => write!(f, "GPU reset required"),
            Self::InvalidGeneration => write!(f, "invalid GPU generation"),
            Self::PciError(e) => write!(f, "PCI error: {:?}", e),
            Self::BarMappingFailed => write!(f, "BAR mapping failed"),
            Self::MmioError => write!(f, "MMIO access error"),

            // Memory
            Self::OutOfVram => write!(f, "out of VRAM"),
            Self::OutOfMemory => write!(f, "out of memory"),
            Self::InvalidGpuAddress => write!(f, "invalid GPU address"),
            Self::AllocationFailed => write!(f, "allocation failed"),
            Self::MisalignedAddress => write!(f, "misaligned address"),
            Self::MappingFailed => write!(f, "memory mapping failed"),
            Self::BufferOverflow => write!(f, "buffer overflow"),
            Self::BufferUnderflow => write!(f, "buffer underflow"),

            // GSP
            Self::GspNotFound => write!(f, "GSP firmware not found"),
            Self::GspHandshakeFailed => write!(f, "GSP handshake failed"),
            Self::GspAuthFailed => write!(f, "GSP authentication failed"),
            Self::GspTimeout => write!(f, "GSP communication timeout"),
            Self::GspError(code) => write!(f, "GSP error: {:?}", code),
            Self::InvalidRpcMessage => write!(f, "invalid RPC message"),
            Self::RpcChannelFull => write!(f, "RPC channel full"),

            // Command
            Self::CommandBufferFull => write!(f, "command buffer full"),
            Self::InvalidCommand => write!(f, "invalid command"),
            Self::SubmissionFailed => write!(f, "submission failed"),
            Self::FenceTimeout => write!(f, "fence wait timeout"),
            Self::RingOverflow => write!(f, "ring buffer overflow"),
            Self::PushBufferError => write!(f, "push buffer error"),

            // Vulkan
            Self::VkInstanceFailed => write!(f, "Vulkan instance creation failed"),
            Self::VkDeviceFailed => write!(f, "Vulkan device creation failed"),
            Self::VkInvalidHandle => write!(f, "invalid Vulkan handle"),
            Self::VkExtensionNotSupported => write!(f, "Vulkan extension not supported"),
            Self::VkFeatureNotSupported => write!(f, "Vulkan feature not supported"),
            Self::ShaderCompilationFailed => write!(f, "shader compilation failed"),
            Self::PipelineCreationFailed => write!(f, "pipeline creation failed"),

            // Display
            Self::NoDisplay => write!(f, "no display connected"),
            Self::InvalidDisplayMode => write!(f, "invalid display mode"),
            Self::ScanoutFailed => write!(f, "scanout failed"),
        }
    }
}

// =============================================================================
// SUB-ERROR TYPES
// =============================================================================

/// PCI-specific error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PciError {
    /// Device not found at expected BDF
    DeviceNotFound,
    /// Invalid vendor ID
    InvalidVendorId,
    /// Invalid device ID
    InvalidDeviceId,
    /// Configuration space access failed
    ConfigAccessFailed,
    /// Capability not found
    CapabilityNotFound,
    /// Express capability missing
    NotPcieDevice,
}

/// GSP firmware error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GspErrorCode {
    /// Generic GSP failure
    GenericFailure,
    /// Firmware version mismatch
    VersionMismatch,
    /// Signature verification failed
    SignatureInvalid,
    /// Insufficient GSP memory
    OutOfGspMemory,
    /// Internal GSP error
    InternalError(u32),
}

// =============================================================================
// ERROR CONVERSION
// =============================================================================

impl From<PciError> for Error {
    fn from(e: PciError) -> Self {
        Error::PciError(e)
    }
}

impl From<GspErrorCode> for Error {
    fn from(e: GspErrorCode) -> Self {
        Error::GspError(e)
    }
}
