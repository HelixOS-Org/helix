//! # Memory Management Primitives
//!
//! Memory regions, pools, and GPU memory management abstractions.

use crate::error::Result;
use crate::types::*;

// =============================================================================
// MEMORY REGION
// =============================================================================

/// A contiguous region of GPU memory
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    /// Starting GPU address
    pub start: GpuAddr,
    /// Size in bytes
    pub size: ByteSize,
    /// Flags describing region properties
    pub flags: MemoryRegionFlags,
}

impl MemoryRegion {
    /// Create a new memory region
    pub const fn new(start: GpuAddr, size: ByteSize) -> Self {
        Self {
            start,
            size,
            flags: MemoryRegionFlags::empty(),
        }
    }

    /// Get end address (exclusive)
    pub fn end(&self) -> GpuAddr {
        self.start + self.size.as_bytes()
    }

    /// Check if address is within region
    pub fn contains(&self, addr: GpuAddr) -> bool {
        addr >= self.start && addr < self.end()
    }

    /// Check if regions overlap
    pub fn overlaps(&self, other: &MemoryRegion) -> bool {
        self.start < other.end() && other.start < self.end()
    }
}

bitflags::bitflags! {
    /// Memory region flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MemoryRegionFlags: u32 {
        /// Region is read-only
        const READ_ONLY = 1 << 0;
        /// Region is executable (for shaders)
        const EXECUTABLE = 1 << 1;
        /// Region is uncached
        const UNCACHED = 1 << 2;
        /// Region is write-combined
        const WRITE_COMBINE = 1 << 3;
        /// Region is protected
        const PROTECTED = 1 << 4;
        /// Region is mapped to CPU
        const CPU_MAPPED = 1 << 5;
    }
}

// =============================================================================
// MEMORY HEAP
// =============================================================================

/// GPU memory heap types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryHeapType {
    /// Device local (VRAM)
    DeviceLocal,
    /// Host visible (BAR aperture)
    HostVisible,
    /// Host cached
    HostCached,
    /// System memory (for DMA)
    SystemMemory,
}

/// GPU memory heap
#[derive(Debug)]
pub struct MemoryHeap {
    /// Heap type
    pub heap_type: MemoryHeapType,
    /// Base address
    pub base: GpuAddr,
    /// Total size
    pub size: ByteSize,
    /// Available size
    pub available: ByteSize,
    /// Allocation granularity
    pub granularity: u64,
}

// =============================================================================
// BUFFER OBJECT
// =============================================================================

/// Buffer usage flags
bitflags::bitflags! {
    /// How a buffer will be used
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct BufferUsage: u32 {
        /// Vertex data
        const VERTEX = 1 << 0;
        /// Index data
        const INDEX = 1 << 1;
        /// Uniform buffer
        const UNIFORM = 1 << 2;
        /// Storage buffer
        const STORAGE = 1 << 3;
        /// Indirect commands
        const INDIRECT = 1 << 4;
        /// Transfer source
        const TRANSFER_SRC = 1 << 5;
        /// Transfer destination
        const TRANSFER_DST = 1 << 6;
        /// Shader binding table (raytracing)
        const SHADER_BINDING_TABLE = 1 << 7;
        /// Acceleration structure
        const ACCELERATION_STRUCTURE = 1 << 8;
    }
}

/// Buffer creation descriptor
#[derive(Debug, Clone)]
pub struct BufferDesc {
    /// Size in bytes
    pub size: ByteSize,
    /// Usage flags
    pub usage: BufferUsage,
    /// Preferred heap type
    pub heap: MemoryHeapType,
    /// Debug name
    pub name: Option<&'static str>,
}

// =============================================================================
// IMAGE / TEXTURE
// =============================================================================

/// Image format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ImageFormat {
    /// Undefined format
    Undefined = 0,
    /// RGBA 8-bit unorm
    R8G8B8A8Unorm = 37,
    /// BGRA 8-bit unorm
    B8G8R8A8Unorm = 44,
    /// RGBA 8-bit SRGB
    R8G8B8A8Srgb = 43,
    /// BGRA 8-bit SRGB
    B8G8R8A8Srgb = 50,
    /// RGB10A2 unorm
    A2R10G10B10Unorm = 58,
    /// RGBA 16-bit float
    R16G16B16A16Sfloat = 97,
    /// RGBA 32-bit float
    R32G32B32A32Sfloat = 109,
    /// Depth 32-bit float
    D32Sfloat = 126,
    /// Depth 24 + Stencil 8
    D24UnormS8Uint = 129,
    /// BC1 (DXT1) compressed
    Bc1RgbaUnorm = 131,
    /// BC3 (DXT5) compressed
    Bc3RgbaUnorm = 137,
    /// BC7 compressed
    Bc7RgbaUnorm = 145,
}

/// Image dimensions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageDimension {
    /// 1D image
    D1,
    /// 2D image
    D2,
    /// 3D image
    D3,
}

/// Image usage flags
bitflags::bitflags! {
    /// How an image will be used
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ImageUsage: u32 {
        /// Sampled in shader
        const SAMPLED = 1 << 0;
        /// Storage image
        const STORAGE = 1 << 1;
        /// Color attachment
        const COLOR_ATTACHMENT = 1 << 2;
        /// Depth/stencil attachment
        const DEPTH_STENCIL = 1 << 3;
        /// Transfer source
        const TRANSFER_SRC = 1 << 4;
        /// Transfer destination
        const TRANSFER_DST = 1 << 5;
        /// Input attachment
        const INPUT_ATTACHMENT = 1 << 6;
    }
}

/// Image creation descriptor
#[derive(Debug, Clone)]
pub struct ImageDesc {
    /// Format
    pub format: ImageFormat,
    /// Dimensions
    pub dimension: ImageDimension,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Depth (for 3D) or array layers
    pub depth: u32,
    /// Mip levels
    pub mip_levels: u32,
    /// Sample count (for MSAA)
    pub samples: u32,
    /// Usage flags
    pub usage: ImageUsage,
    /// Debug name
    pub name: Option<&'static str>,
}

impl Default for ImageDesc {
    fn default() -> Self {
        Self {
            format: ImageFormat::R8G8B8A8Unorm,
            dimension: ImageDimension::D2,
            width: 1,
            height: 1,
            depth: 1,
            mip_levels: 1,
            samples: 1,
            usage: ImageUsage::SAMPLED,
            name: None,
        }
    }
}
