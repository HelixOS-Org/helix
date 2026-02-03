//! Fence Primitives for Lumina
//!
//! This module provides fence synchronization types for CPU-GPU
//! synchronization and host-side waiting.

// ============================================================================
// Fence Handle
// ============================================================================

/// Fence handle for CPU-GPU synchronization
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct FenceHandle(pub u64);

impl FenceHandle {
    /// Null fence handle
    pub const NULL: Self = Self(0);

    /// Creates a new fence handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }

    /// Raw value
    #[inline]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

impl Default for FenceHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Fence Configuration
// ============================================================================

/// Fence creation configuration
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FenceConfig {
    /// Create fence in signaled state
    pub signaled: bool,
    /// Fence flags
    pub flags: FenceFlags,
    /// Debug name hash
    pub debug_name_hash: u32,
}

impl FenceConfig {
    /// Creates new fence config (unsignaled)
    #[inline]
    pub const fn new() -> Self {
        Self {
            signaled: false,
            flags: FenceFlags::NONE,
            debug_name_hash: 0,
        }
    }

    /// Creates fence config in signaled state
    #[inline]
    pub const fn signaled() -> Self {
        Self {
            signaled: true,
            flags: FenceFlags::NONE,
            debug_name_hash: 0,
        }
    }

    /// Creates fence config in unsignaled state
    #[inline]
    pub const fn unsignaled() -> Self {
        Self::new()
    }

    /// With signaled state
    #[inline]
    pub const fn with_signaled(mut self, signaled: bool) -> Self {
        self.signaled = signaled;
        self
    }

    /// With flags
    #[inline]
    pub const fn with_flags(mut self, flags: FenceFlags) -> Self {
        self.flags = flags;
        self
    }

    /// With debug name hash
    #[inline]
    pub const fn with_name(mut self, hash: u32) -> Self {
        self.debug_name_hash = hash;
        self
    }
}

impl Default for FenceConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Fence flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct FenceFlags(pub u32);

impl FenceFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Signaled on creation
    pub const SIGNALED: Self = Self(1 << 0);

    /// Contains flag
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

// ============================================================================
// Fence State
// ============================================================================

/// Fence state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum FenceState {
    /// Fence is unsignaled (not yet signaled by GPU)
    #[default]
    Unsignaled = 0,
    /// Fence is signaled (GPU has completed work)
    Signaled   = 1,
}

impl FenceState {
    /// Is signaled
    #[inline]
    pub const fn is_signaled(&self) -> bool {
        matches!(self, Self::Signaled)
    }

    /// Is unsignaled
    #[inline]
    pub const fn is_unsignaled(&self) -> bool {
        matches!(self, Self::Unsignaled)
    }
}

// ============================================================================
// Wait Configuration
// ============================================================================

/// Wait timeout configuration
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct WaitTimeout {
    /// Timeout in nanoseconds
    pub timeout_ns: u64,
}

impl WaitTimeout {
    /// Infinite timeout
    pub const INFINITE: Self = Self {
        timeout_ns: u64::MAX,
    };

    /// Zero timeout (poll)
    pub const POLL: Self = Self { timeout_ns: 0 };

    /// 1 millisecond
    pub const MS_1: Self = Self::ms(1);
    /// 10 milliseconds
    pub const MS_10: Self = Self::ms(10);
    /// 100 milliseconds
    pub const MS_100: Self = Self::ms(100);
    /// 1 second
    pub const SECOND_1: Self = Self::seconds(1);
    /// 5 seconds
    pub const SECOND_5: Self = Self::seconds(5);
    /// 10 seconds
    pub const SECOND_10: Self = Self::seconds(10);

    /// Creates timeout from nanoseconds
    #[inline]
    pub const fn ns(ns: u64) -> Self {
        Self { timeout_ns: ns }
    }

    /// Creates timeout from microseconds
    #[inline]
    pub const fn us(us: u64) -> Self {
        Self {
            timeout_ns: us * 1000,
        }
    }

    /// Creates timeout from milliseconds
    #[inline]
    pub const fn ms(ms: u64) -> Self {
        Self {
            timeout_ns: ms * 1_000_000,
        }
    }

    /// Creates timeout from seconds
    #[inline]
    pub const fn seconds(s: u64) -> Self {
        Self {
            timeout_ns: s * 1_000_000_000,
        }
    }

    /// Is infinite timeout
    #[inline]
    pub const fn is_infinite(&self) -> bool {
        self.timeout_ns == u64::MAX
    }

    /// Is zero timeout (poll)
    #[inline]
    pub const fn is_poll(&self) -> bool {
        self.timeout_ns == 0
    }

    /// Get timeout in milliseconds
    #[inline]
    pub const fn as_ms(&self) -> u64 {
        self.timeout_ns / 1_000_000
    }

    /// Get timeout in microseconds
    #[inline]
    pub const fn as_us(&self) -> u64 {
        self.timeout_ns / 1000
    }
}

impl Default for WaitTimeout {
    fn default() -> Self {
        Self::INFINITE
    }
}

// ============================================================================
// Wait Result
// ============================================================================

/// Result of waiting on a fence
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum WaitResult {
    /// Fence was signaled
    Success           = 0,
    /// Wait timed out
    Timeout           = 1,
    /// Device lost
    DeviceLost        = 2,
    /// Out of host memory
    OutOfHostMemory   = 3,
    /// Out of device memory
    OutOfDeviceMemory = 4,
    /// Unknown error
    Unknown           = 255,
}

impl WaitResult {
    /// Is success
    #[inline]
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }

    /// Is timeout
    #[inline]
    pub const fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout)
    }

    /// Is error
    #[inline]
    pub const fn is_error(&self) -> bool {
        matches!(
            self,
            Self::DeviceLost | Self::OutOfHostMemory | Self::OutOfDeviceMemory | Self::Unknown
        )
    }
}

impl Default for WaitResult {
    fn default() -> Self {
        Self::Success
    }
}

// ============================================================================
// Multiple Fence Wait
// ============================================================================

/// Wait mode for multiple fences
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum WaitMode {
    /// Wait for all fences to signal
    #[default]
    All = 0,
    /// Wait for any fence to signal
    Any = 1,
}

impl WaitMode {
    /// Name
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::All => "Wait All",
            Self::Any => "Wait Any",
        }
    }
}

/// Multiple fence wait info
#[derive(Clone, Debug)]
#[repr(C)]
pub struct MultiFenceWait {
    /// Fence handles
    pub fences: [FenceHandle; 16],
    /// Number of fences
    pub fence_count: u32,
    /// Wait mode
    pub wait_mode: WaitMode,
    /// Timeout
    pub timeout: WaitTimeout,
}

impl MultiFenceWait {
    /// Creates new multi-fence wait (wait all)
    #[inline]
    pub fn all(fences: &[FenceHandle], timeout: WaitTimeout) -> Self {
        let mut handles = [FenceHandle::NULL; 16];
        let count = fences.len().min(16);
        handles[..count].copy_from_slice(&fences[..count]);
        Self {
            fences: handles,
            fence_count: count as u32,
            wait_mode: WaitMode::All,
            timeout,
        }
    }

    /// Creates new multi-fence wait (wait any)
    #[inline]
    pub fn any(fences: &[FenceHandle], timeout: WaitTimeout) -> Self {
        let mut handles = [FenceHandle::NULL; 16];
        let count = fences.len().min(16);
        handles[..count].copy_from_slice(&fences[..count]);
        Self {
            fences: handles,
            fence_count: count as u32,
            wait_mode: WaitMode::Any,
            timeout,
        }
    }

    /// Creates infinite wait for all fences
    #[inline]
    pub fn all_infinite(fences: &[FenceHandle]) -> Self {
        Self::all(fences, WaitTimeout::INFINITE)
    }
}

impl Default for MultiFenceWait {
    fn default() -> Self {
        Self {
            fences: [FenceHandle::NULL; 16],
            fence_count: 0,
            wait_mode: WaitMode::All,
            timeout: WaitTimeout::INFINITE,
        }
    }
}

// ============================================================================
// Fence Pool
// ============================================================================

/// Fence pool configuration
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FencePoolConfig {
    /// Initial pool size
    pub initial_size: u32,
    /// Maximum pool size
    pub max_size: u32,
    /// Create fences signaled
    pub create_signaled: bool,
}

impl FencePoolConfig {
    /// Default pool (8 fences, max 64)
    pub const DEFAULT: Self = Self {
        initial_size: 8,
        max_size: 64,
        create_signaled: true,
    };

    /// Small pool (4 fences, max 16)
    pub const SMALL: Self = Self {
        initial_size: 4,
        max_size: 16,
        create_signaled: true,
    };

    /// Large pool (16 fences, max 128)
    pub const LARGE: Self = Self {
        initial_size: 16,
        max_size: 128,
        create_signaled: true,
    };

    /// Creates new pool config
    #[inline]
    pub const fn new(initial: u32, max: u32) -> Self {
        Self {
            initial_size: initial,
            max_size: max,
            create_signaled: true,
        }
    }

    /// With signaled state
    #[inline]
    pub const fn with_signaled(mut self, signaled: bool) -> Self {
        self.create_signaled = signaled;
        self
    }
}

impl Default for FencePoolConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Fence pool statistics
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct FencePoolStats {
    /// Total fences created
    pub total_created: u32,
    /// Currently available fences
    pub available: u32,
    /// Currently in use
    pub in_use: u32,
    /// Peak usage
    pub peak_usage: u32,
    /// Total acquires
    pub total_acquires: u64,
    /// Total releases
    pub total_releases: u64,
}

impl FencePoolStats {
    /// Creates empty stats
    #[inline]
    pub const fn new() -> Self {
        Self {
            total_created: 0,
            available: 0,
            in_use: 0,
            peak_usage: 0,
            total_acquires: 0,
            total_releases: 0,
        }
    }

    /// Utilization percentage
    #[inline]
    pub fn utilization(&self) -> f32 {
        if self.total_created == 0 {
            0.0
        } else {
            (self.in_use as f32 / self.total_created as f32) * 100.0
        }
    }
}

// ============================================================================
// External Fence
// ============================================================================

/// External fence handle type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ExternalFenceHandleType {
    /// No external handle
    #[default]
    None           = 0,
    /// POSIX file descriptor
    OpaqueFd       = 1,
    /// Windows handle
    OpaqueWin32    = 2,
    /// Windows handle (KMT)
    OpaqueWin32Kmt = 3,
    /// Sync file descriptor
    SyncFd         = 4,
}

impl ExternalFenceHandleType {
    /// Name
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::OpaqueFd => "Opaque FD",
            Self::OpaqueWin32 => "Opaque Win32",
            Self::OpaqueWin32Kmt => "Opaque Win32 KMT",
            Self::SyncFd => "Sync FD",
        }
    }

    /// Is POSIX type
    #[inline]
    pub const fn is_posix(&self) -> bool {
        matches!(self, Self::OpaqueFd | Self::SyncFd)
    }

    /// Is Windows type
    #[inline]
    pub const fn is_windows(&self) -> bool {
        matches!(self, Self::OpaqueWin32 | Self::OpaqueWin32Kmt)
    }
}

/// External fence features
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ExternalFenceFeatures(pub u32);

impl ExternalFenceFeatures {
    /// No features
    pub const NONE: Self = Self(0);
    /// Can export
    pub const EXPORTABLE: Self = Self(1 << 0);
    /// Can import
    pub const IMPORTABLE: Self = Self(1 << 1);

    /// Contains feature
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Can export
    #[inline]
    pub const fn can_export(&self) -> bool {
        self.contains(Self::EXPORTABLE)
    }

    /// Can import
    #[inline]
    pub const fn can_import(&self) -> bool {
        self.contains(Self::IMPORTABLE)
    }
}

/// External fence import info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ExternalFenceImportInfo {
    /// Handle type
    pub handle_type: ExternalFenceHandleType,
    /// Handle value (fd or HANDLE)
    pub handle: u64,
    /// Import temporary
    pub temporary: bool,
}

impl ExternalFenceImportInfo {
    /// Creates new import info
    #[inline]
    pub const fn new(handle_type: ExternalFenceHandleType, handle: u64) -> Self {
        Self {
            handle_type,
            handle,
            temporary: false,
        }
    }

    /// From file descriptor
    #[inline]
    pub const fn from_fd(fd: i32) -> Self {
        Self::new(ExternalFenceHandleType::OpaqueFd, fd as u64)
    }

    /// From sync fd
    #[inline]
    pub const fn from_sync_fd(fd: i32) -> Self {
        Self::new(ExternalFenceHandleType::SyncFd, fd as u64)
    }

    /// As temporary
    #[inline]
    pub const fn as_temporary(mut self) -> Self {
        self.temporary = true;
        self
    }
}

impl Default for ExternalFenceImportInfo {
    fn default() -> Self {
        Self::new(ExternalFenceHandleType::None, 0)
    }
}

/// External fence export info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ExternalFenceExportInfo {
    /// Handle type to export
    pub handle_type: ExternalFenceHandleType,
}

impl ExternalFenceExportInfo {
    /// Creates new export info
    #[inline]
    pub const fn new(handle_type: ExternalFenceHandleType) -> Self {
        Self { handle_type }
    }

    /// Export as file descriptor
    #[inline]
    pub const fn as_fd() -> Self {
        Self::new(ExternalFenceHandleType::OpaqueFd)
    }

    /// Export as sync fd
    #[inline]
    pub const fn as_sync_fd() -> Self {
        Self::new(ExternalFenceHandleType::SyncFd)
    }
}

impl Default for ExternalFenceExportInfo {
    fn default() -> Self {
        Self::new(ExternalFenceHandleType::None)
    }
}
