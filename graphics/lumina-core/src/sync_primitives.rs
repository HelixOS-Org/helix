//! Fence and Semaphore Types for Lumina
//!
//! This module provides synchronization primitives including fences,
//! semaphores, and events for GPU synchronization.

// ============================================================================
// Fence Handle
// ============================================================================

/// Fence handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct FenceHandle(pub u64);

impl FenceHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for FenceHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Semaphore Handle
// ============================================================================

/// Semaphore handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SemaphoreHandle(pub u64);

impl SemaphoreHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for SemaphoreHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Event Handle
// ============================================================================

/// Event handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct EventHandle(pub u64);

impl EventHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for EventHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Fence Create Info
// ============================================================================

/// Fence create info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FenceCreateInfo {
    /// Flags
    pub flags: FenceCreateFlags,
}

impl FenceCreateInfo {
    /// Creates new info (unsignaled)
    #[inline]
    pub const fn new() -> Self {
        Self {
            flags: FenceCreateFlags::NONE,
        }
    }

    /// Creates signaled fence
    #[inline]
    pub const fn signaled() -> Self {
        Self {
            flags: FenceCreateFlags::SIGNALED,
        }
    }

    /// Creates unsignaled fence
    #[inline]
    pub const fn unsignaled() -> Self {
        Self::new()
    }

    /// With flags
    #[inline]
    pub const fn with_flags(mut self, flags: FenceCreateFlags) -> Self {
        self.flags = flags;
        self
    }
}

impl Default for FenceCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Fence create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct FenceCreateFlags(pub u32);

impl FenceCreateFlags {
    /// No flags (unsignaled)
    pub const NONE: Self = Self(0);
    /// Create signaled
    pub const SIGNALED: Self = Self(1 << 0);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Is signaled flag set
    #[inline]
    pub const fn is_signaled(&self) -> bool {
        self.contains(Self::SIGNALED)
    }
}

// ============================================================================
// Semaphore Create Info
// ============================================================================

/// Semaphore create info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SemaphoreCreateInfo {
    /// Flags
    pub flags: SemaphoreCreateFlags,
    /// Semaphore type
    pub semaphore_type: SemaphoreType,
    /// Initial value (for timeline semaphores)
    pub initial_value: u64,
}

impl SemaphoreCreateInfo {
    /// Creates binary semaphore
    #[inline]
    pub const fn binary() -> Self {
        Self {
            flags: SemaphoreCreateFlags::NONE,
            semaphore_type: SemaphoreType::Binary,
            initial_value: 0,
        }
    }

    /// Creates timeline semaphore
    #[inline]
    pub const fn timeline(initial_value: u64) -> Self {
        Self {
            flags: SemaphoreCreateFlags::NONE,
            semaphore_type: SemaphoreType::Timeline,
            initial_value,
        }
    }

    /// With flags
    #[inline]
    pub const fn with_flags(mut self, flags: SemaphoreCreateFlags) -> Self {
        self.flags = flags;
        self
    }
}

impl Default for SemaphoreCreateInfo {
    fn default() -> Self {
        Self::binary()
    }
}

/// Semaphore create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct SemaphoreCreateFlags(pub u32);

impl SemaphoreCreateFlags {
    /// No flags
    pub const NONE: Self = Self(0);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Semaphore type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SemaphoreType {
    /// Binary semaphore
    #[default]
    Binary = 0,
    /// Timeline semaphore
    Timeline = 1,
}

impl SemaphoreType {
    /// Name
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Binary => "Binary",
            Self::Timeline => "Timeline",
        }
    }
}

// ============================================================================
// Event Create Info
// ============================================================================

/// Event create info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct EventCreateInfo {
    /// Flags
    pub flags: EventCreateFlags,
}

impl EventCreateInfo {
    /// Creates new info
    #[inline]
    pub const fn new() -> Self {
        Self {
            flags: EventCreateFlags::NONE,
        }
    }

    /// Creates device-only event
    #[inline]
    pub const fn device_only() -> Self {
        Self {
            flags: EventCreateFlags::DEVICE_ONLY,
        }
    }

    /// With flags
    #[inline]
    pub const fn with_flags(mut self, flags: EventCreateFlags) -> Self {
        self.flags = flags;
        self
    }
}

impl Default for EventCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Event create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct EventCreateFlags(pub u32);

impl EventCreateFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Device only (cannot be signaled/reset from host)
    pub const DEVICE_ONLY: Self = Self(1 << 0);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

// ============================================================================
// Wait Operations
// ============================================================================

/// Wait for fences info
#[derive(Clone, Debug)]
#[repr(C)]
pub struct WaitForFencesInfo<'a> {
    /// Fences to wait for
    pub fences: &'a [FenceHandle],
    /// Wait all (true) or any (false)
    pub wait_all: bool,
    /// Timeout in nanoseconds
    pub timeout: u64,
}

impl<'a> WaitForFencesInfo<'a> {
    /// Infinite timeout
    pub const INFINITE: u64 = u64::MAX;

    /// Creates new info (wait all)
    #[inline]
    pub const fn new(fences: &'a [FenceHandle], timeout: u64) -> Self {
        Self {
            fences,
            wait_all: true,
            timeout,
        }
    }

    /// Wait for all fences
    #[inline]
    pub const fn all(fences: &'a [FenceHandle]) -> Self {
        Self::new(fences, Self::INFINITE)
    }

    /// Wait for any fence
    #[inline]
    pub const fn any(fences: &'a [FenceHandle]) -> Self {
        Self {
            fences,
            wait_all: false,
            timeout: Self::INFINITE,
        }
    }

    /// With timeout
    #[inline]
    pub const fn with_timeout(mut self, timeout_ns: u64) -> Self {
        self.timeout = timeout_ns;
        self
    }

    /// With timeout in milliseconds
    #[inline]
    pub const fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout = timeout_ms * 1_000_000;
        self
    }
}

impl Default for WaitForFencesInfo<'_> {
    fn default() -> Self {
        Self {
            fences: &[],
            wait_all: true,
            timeout: Self::INFINITE,
        }
    }
}

/// Semaphore wait info
#[derive(Clone, Debug)]
#[repr(C)]
pub struct SemaphoreWaitInfo<'a> {
    /// Flags
    pub flags: SemaphoreWaitFlags,
    /// Semaphores
    pub semaphores: &'a [SemaphoreHandle],
    /// Values to wait for
    pub values: &'a [u64],
}

impl<'a> SemaphoreWaitInfo<'a> {
    /// Creates new info
    #[inline]
    pub const fn new(semaphores: &'a [SemaphoreHandle], values: &'a [u64]) -> Self {
        Self {
            flags: SemaphoreWaitFlags::NONE,
            semaphores,
            values,
        }
    }

    /// Wait for any
    #[inline]
    pub const fn any(semaphores: &'a [SemaphoreHandle], values: &'a [u64]) -> Self {
        Self {
            flags: SemaphoreWaitFlags::ANY,
            semaphores,
            values,
        }
    }

    /// With flags
    #[inline]
    pub const fn with_flags(mut self, flags: SemaphoreWaitFlags) -> Self {
        self.flags = flags;
        self
    }
}

impl Default for SemaphoreWaitInfo<'_> {
    fn default() -> Self {
        Self {
            flags: SemaphoreWaitFlags::NONE,
            semaphores: &[],
            values: &[],
        }
    }
}

/// Semaphore wait flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct SemaphoreWaitFlags(pub u32);

impl SemaphoreWaitFlags {
    /// No flags (wait all)
    pub const NONE: Self = Self(0);
    /// Wait for any
    pub const ANY: Self = Self(1 << 0);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Semaphore signal info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SemaphoreSignalInfo {
    /// Semaphore
    pub semaphore: SemaphoreHandle,
    /// Value to signal
    pub value: u64,
}

impl SemaphoreSignalInfo {
    /// Creates new info
    #[inline]
    pub const fn new(semaphore: SemaphoreHandle, value: u64) -> Self {
        Self { semaphore, value }
    }
}

impl Default for SemaphoreSignalInfo {
    fn default() -> Self {
        Self::new(SemaphoreHandle::NULL, 0)
    }
}

// ============================================================================
// Wait Results
// ============================================================================

/// Wait result
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum WaitResult {
    /// Success
    Success = 0,
    /// Timeout
    Timeout = 2,
    /// Device lost
    DeviceLost = -4,
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

    /// Is device lost
    #[inline]
    pub const fn is_device_lost(&self) -> bool {
        matches!(self, Self::DeviceLost)
    }

    /// Name
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Success => "Success",
            Self::Timeout => "Timeout",
            Self::DeviceLost => "Device Lost",
        }
    }
}

impl Default for WaitResult {
    fn default() -> Self {
        Self::Success
    }
}

// ============================================================================
// External Synchronization
// ============================================================================

/// External fence handle type flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ExternalFenceHandleTypeFlags(pub u32);

impl ExternalFenceHandleTypeFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Opaque FD
    pub const OPAQUE_FD: Self = Self(1 << 0);
    /// Opaque Win32
    pub const OPAQUE_WIN32: Self = Self(1 << 1);
    /// Opaque Win32 KMT
    pub const OPAQUE_WIN32_KMT: Self = Self(1 << 2);
    /// Sync FD
    pub const SYNC_FD: Self = Self(1 << 3);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// External semaphore handle type flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ExternalSemaphoreHandleTypeFlags(pub u32);

impl ExternalSemaphoreHandleTypeFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Opaque FD
    pub const OPAQUE_FD: Self = Self(1 << 0);
    /// Opaque Win32
    pub const OPAQUE_WIN32: Self = Self(1 << 1);
    /// Opaque Win32 KMT
    pub const OPAQUE_WIN32_KMT: Self = Self(1 << 2);
    /// D3D12 fence
    pub const D3D12_FENCE: Self = Self(1 << 3);
    /// Sync FD
    pub const SYNC_FD: Self = Self(1 << 4);
    /// Zircon event
    pub const ZIRCON_EVENT: Self = Self(1 << 7);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// External fence properties
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ExternalFenceProperties {
    /// Export from imported handle types
    pub export_from_imported_handle_types: ExternalFenceHandleTypeFlags,
    /// Compatible handle types
    pub compatible_handle_types: ExternalFenceHandleTypeFlags,
    /// External fence features
    pub external_fence_features: ExternalFenceFeatureFlags,
}

/// External fence feature flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ExternalFenceFeatureFlags(pub u32);

impl ExternalFenceFeatureFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Exportable
    pub const EXPORTABLE: Self = Self(1 << 0);
    /// Importable
    pub const IMPORTABLE: Self = Self(1 << 1);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// External semaphore properties
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ExternalSemaphoreProperties {
    /// Export from imported handle types
    pub export_from_imported_handle_types: ExternalSemaphoreHandleTypeFlags,
    /// Compatible handle types
    pub compatible_handle_types: ExternalSemaphoreHandleTypeFlags,
    /// External semaphore features
    pub external_semaphore_features: ExternalSemaphoreFeatureFlags,
}

/// External semaphore feature flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ExternalSemaphoreFeatureFlags(pub u32);

impl ExternalSemaphoreFeatureFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Exportable
    pub const EXPORTABLE: Self = Self(1 << 0);
    /// Importable
    pub const IMPORTABLE: Self = Self(1 << 1);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

// ============================================================================
// Import/Export Fence
// ============================================================================

/// Import fence FD info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ImportFenceFdInfo {
    /// Fence
    pub fence: FenceHandle,
    /// Flags
    pub flags: FenceImportFlags,
    /// Handle type
    pub handle_type: ExternalFenceHandleTypeFlags,
    /// FD
    pub fd: i32,
}

impl ImportFenceFdInfo {
    /// Creates new info
    #[inline]
    pub const fn new(
        fence: FenceHandle,
        handle_type: ExternalFenceHandleTypeFlags,
        fd: i32,
    ) -> Self {
        Self {
            fence,
            flags: FenceImportFlags::NONE,
            handle_type,
            fd,
        }
    }

    /// Temporary import
    #[inline]
    pub const fn temporary(mut self) -> Self {
        self.flags = self.flags.union(FenceImportFlags::TEMPORARY);
        self
    }
}

impl Default for ImportFenceFdInfo {
    fn default() -> Self {
        Self::new(FenceHandle::NULL, ExternalFenceHandleTypeFlags::OPAQUE_FD, -1)
    }
}

/// Fence import flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct FenceImportFlags(pub u32);

impl FenceImportFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Temporary
    pub const TEMPORARY: Self = Self(1 << 0);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Export fence create info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ExportFenceCreateInfo {
    /// Handle types
    pub handle_types: ExternalFenceHandleTypeFlags,
}

impl ExportFenceCreateInfo {
    /// Creates new info
    #[inline]
    pub const fn new(handle_types: ExternalFenceHandleTypeFlags) -> Self {
        Self { handle_types }
    }
}

// ============================================================================
// Import/Export Semaphore
// ============================================================================

/// Import semaphore FD info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ImportSemaphoreFdInfo {
    /// Semaphore
    pub semaphore: SemaphoreHandle,
    /// Flags
    pub flags: SemaphoreImportFlags,
    /// Handle type
    pub handle_type: ExternalSemaphoreHandleTypeFlags,
    /// FD
    pub fd: i32,
}

impl ImportSemaphoreFdInfo {
    /// Creates new info
    #[inline]
    pub const fn new(
        semaphore: SemaphoreHandle,
        handle_type: ExternalSemaphoreHandleTypeFlags,
        fd: i32,
    ) -> Self {
        Self {
            semaphore,
            flags: SemaphoreImportFlags::NONE,
            handle_type,
            fd,
        }
    }

    /// Temporary import
    #[inline]
    pub const fn temporary(mut self) -> Self {
        self.flags = self.flags.union(SemaphoreImportFlags::TEMPORARY);
        self
    }
}

impl Default for ImportSemaphoreFdInfo {
    fn default() -> Self {
        Self::new(
            SemaphoreHandle::NULL,
            ExternalSemaphoreHandleTypeFlags::OPAQUE_FD,
            -1,
        )
    }
}

/// Semaphore import flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct SemaphoreImportFlags(pub u32);

impl SemaphoreImportFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Temporary
    pub const TEMPORARY: Self = Self(1 << 0);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Export semaphore create info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ExportSemaphoreCreateInfo {
    /// Handle types
    pub handle_types: ExternalSemaphoreHandleTypeFlags,
}

impl ExportSemaphoreCreateInfo {
    /// Creates new info
    #[inline]
    pub const fn new(handle_types: ExternalSemaphoreHandleTypeFlags) -> Self {
        Self { handle_types }
    }
}

// ============================================================================
// Timeline Semaphore Helpers
// ============================================================================

/// Timeline semaphore value
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct TimelineValue {
    /// Current value
    pub value: u64,
}

impl TimelineValue {
    /// Creates new value
    #[inline]
    pub const fn new(value: u64) -> Self {
        Self { value }
    }

    /// Increment
    #[inline]
    pub const fn increment(&self) -> u64 {
        self.value + 1
    }

    /// Next value
    #[inline]
    pub const fn next(&self) -> Self {
        Self {
            value: self.value + 1,
        }
    }
}

/// Timeline synchronization pair
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct TimelineSync {
    /// Semaphore
    pub semaphore: SemaphoreHandle,
    /// Wait value
    pub wait_value: u64,
    /// Signal value
    pub signal_value: u64,
}

impl TimelineSync {
    /// Creates new sync
    #[inline]
    pub const fn new(semaphore: SemaphoreHandle, wait: u64, signal: u64) -> Self {
        Self {
            semaphore,
            wait_value: wait,
            signal_value: signal,
        }
    }

    /// Sequential (wait for previous, signal next)
    #[inline]
    pub const fn sequential(semaphore: SemaphoreHandle, value: u64) -> Self {
        Self::new(semaphore, value, value + 1)
    }
}
