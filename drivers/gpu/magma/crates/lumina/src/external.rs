//! External memory and semaphore types
//!
//! This module provides types for cross-API and cross-process resource sharing.

extern crate alloc;
use alloc::vec::Vec;

/// External memory handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ExternalMemoryHandle(pub u64);

impl ExternalMemoryHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Invalid handle value
    pub const INVALID: Self = Self(u64::MAX);

    /// Checks if valid
    pub const fn is_valid(&self) -> bool {
        self.0 != 0 && self.0 != u64::MAX
    }
}

/// External semaphore handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ExternalSemaphoreHandle(pub u64);

impl ExternalSemaphoreHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

/// External fence handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ExternalFenceHandle(pub u64);

impl ExternalFenceHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

/// External memory handle type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ExternalMemoryHandleType {
    /// Opaque file descriptor (Linux)
    OpaqueFd        = 0x01,
    /// Opaque Win32 handle
    OpaqueWin32     = 0x02,
    /// Opaque Win32 KMT handle
    OpaqueWin32Kmt  = 0x04,
    /// D3D11 texture (Win32)
    D3D11Texture    = 0x08,
    /// D3D11 texture KMT
    D3D11TextureKmt = 0x10,
    /// D3D12 heap
    D3D12Heap       = 0x20,
    /// D3D12 resource
    D3D12Resource   = 0x40,
    /// DMA buf (Linux)
    DmaBuf          = 0x200,
    /// Android hardware buffer
    AndroidHardwareBuffer = 0x400,
    /// Host allocation
    HostAllocation  = 0x80,
    /// Host mapped foreign memory
    HostMappedForeignMemory = 0x100,
    /// Zircon VMO (Fuchsia)
    ZirconVmo       = 0x800,
    /// RDMA address (network)
    RdmaAddress     = 0x1000,
}

impl ExternalMemoryHandleType {
    /// Is Linux-specific
    pub const fn is_linux(&self) -> bool {
        matches!(self, Self::OpaqueFd | Self::DmaBuf)
    }

    /// Is Windows-specific
    pub const fn is_windows(&self) -> bool {
        matches!(
            self,
            Self::OpaqueWin32
                | Self::OpaqueWin32Kmt
                | Self::D3D11Texture
                | Self::D3D11TextureKmt
                | Self::D3D12Heap
                | Self::D3D12Resource
        )
    }

    /// Is cross-platform
    pub const fn is_cross_platform(&self) -> bool {
        matches!(self, Self::HostAllocation | Self::HostMappedForeignMemory)
    }
}

/// External memory handle type flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ExternalMemoryHandleTypeFlags(pub u32);

impl ExternalMemoryHandleTypeFlags {
    /// Opaque FD
    pub const OPAQUE_FD: Self = Self(0x01);
    /// Opaque Win32
    pub const OPAQUE_WIN32: Self = Self(0x02);
    /// DMA buf
    pub const DMA_BUF: Self = Self(0x200);
    /// D3D12 heap
    pub const D3D12_HEAP: Self = Self(0x20);

    /// Combines flags
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Checks if contains
    pub const fn contains(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }
}

/// External semaphore handle type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ExternalSemaphoreHandleType {
    /// Opaque FD
    OpaqueFd       = 0x01,
    /// Opaque Win32
    OpaqueWin32    = 0x02,
    /// Opaque Win32 KMT
    OpaqueWin32Kmt = 0x04,
    /// D3D12 fence
    D3D12Fence     = 0x08,
    /// Sync FD
    SyncFd         = 0x10,
    /// Zircon event
    ZirconEvent    = 0x80,
}

/// External fence handle type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ExternalFenceHandleType {
    /// Opaque FD
    OpaqueFd       = 0x01,
    /// Opaque Win32
    OpaqueWin32    = 0x02,
    /// Opaque Win32 KMT
    OpaqueWin32Kmt = 0x04,
    /// Sync FD
    SyncFd         = 0x08,
}

/// External memory properties
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ExternalMemoryProperties {
    /// Memory features
    pub memory_features: ExternalMemoryFeatureFlags,
    /// Export from imported types
    pub export_from_imported_handle_types: ExternalMemoryHandleTypeFlags,
    /// Compatible handle types
    pub compatible_handle_types: ExternalMemoryHandleTypeFlags,
}

/// External memory feature flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ExternalMemoryFeatureFlags(pub u32);

impl ExternalMemoryFeatureFlags {
    /// Dedicated only
    pub const DEDICATED_ONLY: Self = Self(1 << 0);
    /// Exportable
    pub const EXPORTABLE: Self = Self(1 << 1);
    /// Importable
    pub const IMPORTABLE: Self = Self(1 << 2);

    /// Checks if exportable
    pub const fn is_exportable(&self) -> bool {
        (self.0 & Self::EXPORTABLE.0) != 0
    }

    /// Checks if importable
    pub const fn is_importable(&self) -> bool {
        (self.0 & Self::IMPORTABLE.0) != 0
    }
}

/// External buffer create info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ExternalBufferCreateInfo {
    /// Handle types
    pub handle_types: ExternalMemoryHandleTypeFlags,
}

impl ExternalBufferCreateInfo {
    /// Creates for opaque FD
    pub const fn opaque_fd() -> Self {
        Self {
            handle_types: ExternalMemoryHandleTypeFlags::OPAQUE_FD,
        }
    }

    /// Creates for DMA buf
    pub const fn dma_buf() -> Self {
        Self {
            handle_types: ExternalMemoryHandleTypeFlags::DMA_BUF,
        }
    }
}

/// External image create info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ExternalImageCreateInfo {
    /// Handle types
    pub handle_types: ExternalMemoryHandleTypeFlags,
}

/// Import memory FD info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ImportMemoryFdInfo {
    /// Handle type
    pub handle_type: ExternalMemoryHandleType,
    /// File descriptor
    pub fd: i32,
}

impl ImportMemoryFdInfo {
    /// Creates from opaque FD
    pub const fn opaque_fd(fd: i32) -> Self {
        Self {
            handle_type: ExternalMemoryHandleType::OpaqueFd,
            fd,
        }
    }

    /// Creates from DMA buf
    pub const fn dma_buf(fd: i32) -> Self {
        Self {
            handle_type: ExternalMemoryHandleType::DmaBuf,
            fd,
        }
    }
}

/// Memory get FD info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MemoryGetFdInfo {
    /// Memory handle
    pub memory: u64,
    /// Handle type
    pub handle_type: ExternalMemoryHandleType,
}

/// Import semaphore FD info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ImportSemaphoreFdInfo {
    /// Semaphore handle
    pub semaphore: u64,
    /// Flags
    pub flags: SemaphoreImportFlags,
    /// Handle type
    pub handle_type: ExternalSemaphoreHandleType,
    /// File descriptor
    pub fd: i32,
}

/// Semaphore import flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct SemaphoreImportFlags(pub u32);

impl SemaphoreImportFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Temporary import
    pub const TEMPORARY: Self = Self(1 << 0);
}

/// Semaphore get FD info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SemaphoreGetFdInfo {
    /// Semaphore handle
    pub semaphore: u64,
    /// Handle type
    pub handle_type: ExternalSemaphoreHandleType,
}

/// Import fence FD info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ImportFenceFdInfo {
    /// Fence handle
    pub fence: u64,
    /// Flags
    pub flags: FenceImportFlags,
    /// Handle type
    pub handle_type: ExternalFenceHandleType,
    /// File descriptor
    pub fd: i32,
}

/// Fence import flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct FenceImportFlags(pub u32);

impl FenceImportFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Temporary import
    pub const TEMPORARY: Self = Self(1 << 0);
}

/// Fence get FD info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FenceGetFdInfo {
    /// Fence handle
    pub fence: u64,
    /// Handle type
    pub handle_type: ExternalFenceHandleType,
}

/// External semaphore properties
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ExternalSemaphoreProperties {
    /// Export from imported types
    pub export_from_imported_handle_types: ExternalSemaphoreHandleTypeFlags,
    /// Compatible types
    pub compatible_handle_types: ExternalSemaphoreHandleTypeFlags,
    /// Features
    pub features: ExternalSemaphoreFeatureFlags,
}

/// External semaphore handle type flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ExternalSemaphoreHandleTypeFlags(pub u32);

impl ExternalSemaphoreHandleTypeFlags {
    /// Opaque FD
    pub const OPAQUE_FD: Self = Self(0x01);
    /// Opaque Win32
    pub const OPAQUE_WIN32: Self = Self(0x02);
    /// Sync FD
    pub const SYNC_FD: Self = Self(0x10);
}

/// External semaphore feature flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ExternalSemaphoreFeatureFlags(pub u32);

impl ExternalSemaphoreFeatureFlags {
    /// Exportable
    pub const EXPORTABLE: Self = Self(1 << 0);
    /// Importable
    pub const IMPORTABLE: Self = Self(1 << 1);
}

/// External fence properties
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ExternalFenceProperties {
    /// Export from imported types
    pub export_from_imported_handle_types: ExternalFenceHandleTypeFlags,
    /// Compatible types
    pub compatible_handle_types: ExternalFenceHandleTypeFlags,
    /// Features
    pub features: ExternalFenceFeatureFlags,
}

/// External fence handle type flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ExternalFenceHandleTypeFlags(pub u32);

impl ExternalFenceHandleTypeFlags {
    /// Opaque FD
    pub const OPAQUE_FD: Self = Self(0x01);
    /// Sync FD
    pub const SYNC_FD: Self = Self(0x08);
}

/// External fence feature flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ExternalFenceFeatureFlags(pub u32);

impl ExternalFenceFeatureFlags {
    /// Exportable
    pub const EXPORTABLE: Self = Self(1 << 0);
    /// Importable
    pub const IMPORTABLE: Self = Self(1 << 1);
}

/// DMA buf format properties
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DrmFormatModifierProperties {
    /// DRM format modifier
    pub drm_format_modifier: u64,
    /// Number of planes
    pub drm_format_modifier_plane_count: u32,
    /// Compatible tiling features
    pub drm_format_modifier_tiling_features: FormatFeatureFlags,
}

/// Format feature flags for external memory
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct FormatFeatureFlags(pub u32);

impl FormatFeatureFlags {
    /// Sampled image
    pub const SAMPLED_IMAGE: Self = Self(1 << 0);
    /// Storage image
    pub const STORAGE_IMAGE: Self = Self(1 << 1);
    /// Color attachment
    pub const COLOR_ATTACHMENT: Self = Self(1 << 2);
    /// Depth stencil attachment
    pub const DEPTH_STENCIL_ATTACHMENT: Self = Self(1 << 3);
    /// Transfer source
    pub const TRANSFER_SRC: Self = Self(1 << 4);
    /// Transfer destination
    pub const TRANSFER_DST: Self = Self(1 << 5);
    /// Sampled image filter linear
    pub const SAMPLED_IMAGE_FILTER_LINEAR: Self = Self(1 << 6);

    /// Checks if contains
    pub const fn contains(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }
}

/// Image DRM format modifier info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ImageDrmFormatModifierInfo {
    /// DRM format modifier
    pub drm_format_modifier: u64,
    /// Sharing mode
    pub sharing_mode: SharingMode,
    /// Queue family index count
    pub queue_family_index_count: u32,
}

/// Sharing mode
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SharingMode {
    /// Exclusive access
    #[default]
    Exclusive,
    /// Concurrent access
    Concurrent,
}

/// Image DRM format modifier explicit info
#[derive(Clone, Debug)]
pub struct ImageDrmFormatModifierExplicitInfo {
    /// DRM format modifier
    pub drm_format_modifier: u64,
    /// Per-plane layouts
    pub plane_layouts: Vec<SubresourceLayout>,
}

/// Subresource layout
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SubresourceLayout {
    /// Offset
    pub offset: u64,
    /// Size
    pub size: u64,
    /// Row pitch
    pub row_pitch: u64,
    /// Array pitch
    pub array_pitch: u64,
    /// Depth pitch
    pub depth_pitch: u64,
}

/// Export memory info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ExportMemoryInfo {
    /// Handle types to export
    pub handle_types: ExternalMemoryHandleTypeFlags,
}

/// Export semaphore info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ExportSemaphoreInfo {
    /// Handle types
    pub handle_types: ExternalSemaphoreHandleTypeFlags,
}

/// Export fence info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ExportFenceInfo {
    /// Handle types
    pub handle_types: ExternalFenceHandleTypeFlags,
}

/// Cross-process shared resource
#[derive(Clone, Debug)]
pub struct SharedResource {
    /// Resource name
    pub name: Vec<u8>,
    /// Handle type
    pub handle_type: ExternalMemoryHandleType,
    /// Size in bytes
    pub size: u64,
    /// Memory type bits
    pub memory_type_bits: u32,
    /// Dedicated allocation required
    pub dedicated: bool,
}

impl SharedResource {
    /// Creates a new shared resource
    pub fn new(name: &[u8], handle_type: ExternalMemoryHandleType, size: u64) -> Self {
        Self {
            name: name.to_vec(),
            handle_type,
            size,
            memory_type_bits: 0,
            dedicated: false,
        }
    }
}

/// Memory plane layout
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct PlaneLayout {
    /// Offset from start of allocation
    pub offset: u64,
    /// Size of plane
    pub size: u64,
    /// Row stride
    pub row_stride: u64,
    /// Array stride
    pub array_stride: u64,
}

/// DRM modifier list
#[derive(Clone, Debug, Default)]
pub struct DrmModifierList {
    /// List of supported modifiers
    pub modifiers: Vec<u64>,
}

impl DrmModifierList {
    /// Linear modifier (no tiling)
    pub const LINEAR: u64 = 0;

    /// Intel X-tiling
    pub const I915_X_TILED: u64 = 0x0100000000000001;

    /// Intel Y-tiling
    pub const I915_Y_TILED: u64 = 0x0100000000000002;

    /// AMD GFX9 64K_S
    pub const AMD_GFX9_64K_S: u64 = 0x0200000000000001;

    /// Creates an empty list
    pub const fn new() -> Self {
        Self {
            modifiers: Vec::new(),
        }
    }

    /// Adds a modifier
    pub fn add(mut self, modifier: u64) -> Self {
        self.modifiers.push(modifier);
        self
    }

    /// Checks if contains modifier
    pub fn contains(&self, modifier: u64) -> bool {
        self.modifiers.contains(&modifier)
    }
}
