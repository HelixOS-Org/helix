//! Resource Management - Transient Resources and Memory Pooling
//!
//! This module provides efficient resource management with:
//! - Transient resource allocation with automatic lifetime tracking
//! - Memory aliasing for reduced memory usage
//! - Resource pooling for fast allocation/deallocation
//! - Smart caching of frequently used resources

use alloc::{collections::BTreeMap, string::String, vec::Vec};
use core::{
    fmt,
    hash::{Hash, Hasher},
    sync::atomic::{AtomicU32, AtomicU64, Ordering},
};

use crate::barrier::{AccessFlags, PipelineStage};

/// Handle to a GPU texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureHandle(pub u32);

impl TextureHandle {
    /// Invalid handle.
    pub const INVALID: Self = Self(u32::MAX);

    /// Create new handle.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Check if valid.
    pub fn is_valid(&self) -> bool {
        self.0 != u32::MAX
    }

    /// Get raw ID.
    pub fn raw(&self) -> u32 {
        self.0
    }
}

/// Handle to a GPU buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferHandle(pub u32);

impl BufferHandle {
    /// Invalid handle.
    pub const INVALID: Self = Self(u32::MAX);

    /// Create new handle.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Check if valid.
    pub fn is_valid(&self) -> bool {
        self.0 != u32::MAX
    }

    /// Get raw ID.
    pub fn raw(&self) -> u32 {
        self.0
    }
}

/// Generic resource handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceHandle {
    /// Texture resource.
    Texture(TextureHandle),
    /// Buffer resource.
    Buffer(BufferHandle),
}

/// Texture description.
#[derive(Debug, Clone)]
pub struct TextureDesc {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Depth (for 3D textures).
    pub depth: u32,
    /// Mip levels.
    pub mip_levels: u32,
    /// Array layers.
    pub array_layers: u32,
    /// Texture format.
    pub format: TextureFormat,
    /// Sample count for MSAA.
    pub samples: SampleCount,
    /// Texture dimension.
    pub dimension: TextureDimension,
    /// Usage flags.
    pub usage: TextureUsageFlags,
    /// Memory priority.
    pub priority: ResourcePriority,
    /// Debug name.
    pub name: String,
}

impl Default for TextureDesc {
    fn default() -> Self {
        Self {
            width: 1,
            height: 1,
            depth: 1,
            mip_levels: 1,
            array_layers: 1,
            format: TextureFormat::RGBA8Unorm,
            samples: SampleCount::X1,
            dimension: TextureDimension::D2,
            usage: TextureUsageFlags::SAMPLED,
            priority: ResourcePriority::Normal,
            name: String::new(),
        }
    }
}

impl TextureDesc {
    /// Create 2D color texture.
    pub fn color_2d(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            format: TextureFormat::RGBA8Unorm,
            usage: TextureUsageFlags::COLOR_ATTACHMENT | TextureUsageFlags::SAMPLED,
            ..Default::default()
        }
    }

    /// Create 2D HDR color texture.
    pub fn hdr_2d(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            format: TextureFormat::RGBA16Float,
            usage: TextureUsageFlags::COLOR_ATTACHMENT | TextureUsageFlags::SAMPLED,
            ..Default::default()
        }
    }

    /// Create depth texture.
    pub fn depth(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            format: TextureFormat::D32Float,
            usage: TextureUsageFlags::DEPTH_STENCIL_ATTACHMENT | TextureUsageFlags::SAMPLED,
            ..Default::default()
        }
    }

    /// Create depth-stencil texture.
    pub fn depth_stencil(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            format: TextureFormat::D24UnormS8Uint,
            usage: TextureUsageFlags::DEPTH_STENCIL_ATTACHMENT | TextureUsageFlags::SAMPLED,
            ..Default::default()
        }
    }

    /// Create GBuffer texture set.
    pub fn gbuffer(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            format: TextureFormat::RGBA16Float,
            usage: TextureUsageFlags::COLOR_ATTACHMENT | TextureUsageFlags::SAMPLED,
            ..Default::default()
        }
    }

    /// Create shadow map.
    pub fn shadow_map(size: u32) -> Self {
        Self {
            width: size,
            height: size,
            format: TextureFormat::D32Float,
            usage: TextureUsageFlags::DEPTH_STENCIL_ATTACHMENT | TextureUsageFlags::SAMPLED,
            ..Default::default()
        }
    }

    /// Create shadow map array.
    pub fn shadow_map_array(size: u32, layers: u32) -> Self {
        Self {
            width: size,
            height: size,
            array_layers: layers,
            format: TextureFormat::D32Float,
            usage: TextureUsageFlags::DEPTH_STENCIL_ATTACHMENT | TextureUsageFlags::SAMPLED,
            ..Default::default()
        }
    }

    /// Create cubemap.
    pub fn cubemap(size: u32, format: TextureFormat) -> Self {
        Self {
            width: size,
            height: size,
            array_layers: 6,
            format,
            dimension: TextureDimension::Cube,
            usage: TextureUsageFlags::SAMPLED,
            ..Default::default()
        }
    }

    /// Create 3D texture.
    pub fn volume(width: u32, height: u32, depth: u32, format: TextureFormat) -> Self {
        Self {
            width,
            height,
            depth,
            format,
            dimension: TextureDimension::D3,
            usage: TextureUsageFlags::SAMPLED | TextureUsageFlags::STORAGE,
            ..Default::default()
        }
    }

    /// Create storage texture.
    pub fn storage_2d(width: u32, height: u32, format: TextureFormat) -> Self {
        Self {
            width,
            height,
            format,
            usage: TextureUsageFlags::STORAGE | TextureUsageFlags::SAMPLED,
            ..Default::default()
        }
    }

    /// With mip levels.
    pub fn with_mips(mut self, levels: u32) -> Self {
        self.mip_levels = levels;
        self
    }

    /// With auto mip levels.
    pub fn with_auto_mips(mut self) -> Self {
        self.mip_levels = self.calculate_mip_levels();
        self
    }

    /// With MSAA.
    pub fn with_samples(mut self, samples: SampleCount) -> Self {
        self.samples = samples;
        self
    }

    /// With debug name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Calculate mip level count.
    pub fn calculate_mip_levels(&self) -> u32 {
        let max_dim = self.width.max(self.height).max(self.depth);
        (32 - max_dim.leading_zeros()).max(1)
    }

    /// Calculate total size in bytes.
    pub fn calculate_size(&self) -> u64 {
        let bytes_per_pixel = self.format.bytes_per_pixel() as u64;
        let mut total = 0u64;

        for mip in 0..self.mip_levels {
            let mip_width = (self.width >> mip).max(1) as u64;
            let mip_height = (self.height >> mip).max(1) as u64;
            let mip_depth = (self.depth >> mip).max(1) as u64;

            total += mip_width * mip_height * mip_depth * bytes_per_pixel;
        }

        total * self.array_layers as u64 * self.samples.count() as u64
    }

    /// Get required alignment.
    pub fn alignment(&self) -> u64 {
        // Most textures require 256-byte alignment
        if self.usage.contains(TextureUsageFlags::COLOR_ATTACHMENT)
            || self.usage.contains(TextureUsageFlags::DEPTH_STENCIL_ATTACHMENT)
        {
            4096 // Render targets need larger alignment
        } else {
            256
        }
    }
}

/// Texture formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureFormat {
    // 8-bit formats
    R8Unorm,
    R8Snorm,
    R8Uint,
    R8Sint,

    // 16-bit formats
    R16Uint,
    R16Sint,
    R16Float,
    RG8Unorm,
    RG8Snorm,
    RG8Uint,
    RG8Sint,

    // 32-bit formats
    R32Uint,
    R32Sint,
    R32Float,
    RG16Uint,
    RG16Sint,
    RG16Float,
    RGBA8Unorm,
    RGBA8UnormSrgb,
    RGBA8Snorm,
    RGBA8Uint,
    RGBA8Sint,
    BGRA8Unorm,
    BGRA8UnormSrgb,
    RGB10A2Unorm,
    RG11B10Float,
    RGB9E5Float,

    // 64-bit formats
    RG32Uint,
    RG32Sint,
    RG32Float,
    RGBA16Uint,
    RGBA16Sint,
    RGBA16Float,

    // 128-bit formats
    RGBA32Uint,
    RGBA32Sint,
    RGBA32Float,

    // Depth formats
    D16Unorm,
    D24UnormS8Uint,
    D32Float,
    D32FloatS8Uint,

    // Compressed formats - BC
    BC1RGBAUnorm,
    BC1RGBAUnormSrgb,
    BC2RGBAUnorm,
    BC2RGBAUnormSrgb,
    BC3RGBAUnorm,
    BC3RGBAUnormSrgb,
    BC4RUnorm,
    BC4RSnorm,
    BC5RGUnorm,
    BC5RGSnorm,
    BC6HRGBUfloat,
    BC6HRGBFloat,
    BC7RGBAUnorm,
    BC7RGBAUnormSrgb,

    // Compressed formats - ASTC
    ASTC4x4Unorm,
    ASTC4x4UnormSrgb,
    ASTC5x5Unorm,
    ASTC5x5UnormSrgb,
    ASTC6x6Unorm,
    ASTC6x6UnormSrgb,
    ASTC8x8Unorm,
    ASTC8x8UnormSrgb,
}

impl TextureFormat {
    /// Get bytes per pixel (for uncompressed formats).
    pub fn bytes_per_pixel(&self) -> u32 {
        match self {
            // 8-bit
            Self::R8Unorm | Self::R8Snorm | Self::R8Uint | Self::R8Sint => 1,

            // 16-bit
            Self::R16Uint
            | Self::R16Sint
            | Self::R16Float
            | Self::RG8Unorm
            | Self::RG8Snorm
            | Self::RG8Uint
            | Self::RG8Sint
            | Self::D16Unorm => 2,

            // 32-bit
            Self::R32Uint
            | Self::R32Sint
            | Self::R32Float
            | Self::RG16Uint
            | Self::RG16Sint
            | Self::RG16Float
            | Self::RGBA8Unorm
            | Self::RGBA8UnormSrgb
            | Self::RGBA8Snorm
            | Self::RGBA8Uint
            | Self::RGBA8Sint
            | Self::BGRA8Unorm
            | Self::BGRA8UnormSrgb
            | Self::RGB10A2Unorm
            | Self::RG11B10Float
            | Self::RGB9E5Float
            | Self::D24UnormS8Uint
            | Self::D32Float => 4,

            // 64-bit
            Self::RG32Uint
            | Self::RG32Sint
            | Self::RG32Float
            | Self::RGBA16Uint
            | Self::RGBA16Sint
            | Self::RGBA16Float
            | Self::D32FloatS8Uint => 8,

            // 128-bit
            Self::RGBA32Uint | Self::RGBA32Sint | Self::RGBA32Float => 16,

            // Compressed - approximate based on block size
            Self::BC1RGBAUnorm | Self::BC1RGBAUnormSrgb | Self::BC4RUnorm | Self::BC4RSnorm => 1,
            Self::BC2RGBAUnorm
            | Self::BC2RGBAUnormSrgb
            | Self::BC3RGBAUnorm
            | Self::BC3RGBAUnormSrgb
            | Self::BC5RGUnorm
            | Self::BC5RGSnorm
            | Self::BC6HRGBUfloat
            | Self::BC6HRGBFloat
            | Self::BC7RGBAUnorm
            | Self::BC7RGBAUnormSrgb => 1,

            Self::ASTC4x4Unorm
            | Self::ASTC4x4UnormSrgb
            | Self::ASTC5x5Unorm
            | Self::ASTC5x5UnormSrgb
            | Self::ASTC6x6Unorm
            | Self::ASTC6x6UnormSrgb
            | Self::ASTC8x8Unorm
            | Self::ASTC8x8UnormSrgb => 1,
        }
    }

    /// Check if this is a depth format.
    pub fn is_depth(&self) -> bool {
        matches!(
            self,
            Self::D16Unorm | Self::D24UnormS8Uint | Self::D32Float | Self::D32FloatS8Uint
        )
    }

    /// Check if this has stencil.
    pub fn has_stencil(&self) -> bool {
        matches!(self, Self::D24UnormS8Uint | Self::D32FloatS8Uint)
    }

    /// Check if sRGB.
    pub fn is_srgb(&self) -> bool {
        matches!(
            self,
            Self::RGBA8UnormSrgb
                | Self::BGRA8UnormSrgb
                | Self::BC1RGBAUnormSrgb
                | Self::BC2RGBAUnormSrgb
                | Self::BC3RGBAUnormSrgb
                | Self::BC7RGBAUnormSrgb
                | Self::ASTC4x4UnormSrgb
                | Self::ASTC5x5UnormSrgb
                | Self::ASTC6x6UnormSrgb
                | Self::ASTC8x8UnormSrgb
        )
    }

    /// Check if compressed.
    pub fn is_compressed(&self) -> bool {
        matches!(
            self,
            Self::BC1RGBAUnorm
                | Self::BC1RGBAUnormSrgb
                | Self::BC2RGBAUnorm
                | Self::BC2RGBAUnormSrgb
                | Self::BC3RGBAUnorm
                | Self::BC3RGBAUnormSrgb
                | Self::BC4RUnorm
                | Self::BC4RSnorm
                | Self::BC5RGUnorm
                | Self::BC5RGSnorm
                | Self::BC6HRGBUfloat
                | Self::BC6HRGBFloat
                | Self::BC7RGBAUnorm
                | Self::BC7RGBAUnormSrgb
                | Self::ASTC4x4Unorm
                | Self::ASTC4x4UnormSrgb
                | Self::ASTC5x5Unorm
                | Self::ASTC5x5UnormSrgb
                | Self::ASTC6x6Unorm
                | Self::ASTC6x6UnormSrgb
                | Self::ASTC8x8Unorm
                | Self::ASTC8x8UnormSrgb
        )
    }

    /// Get sRGB variant.
    pub fn to_srgb(&self) -> Self {
        match self {
            Self::RGBA8Unorm => Self::RGBA8UnormSrgb,
            Self::BGRA8Unorm => Self::BGRA8UnormSrgb,
            Self::BC1RGBAUnorm => Self::BC1RGBAUnormSrgb,
            Self::BC2RGBAUnorm => Self::BC2RGBAUnormSrgb,
            Self::BC3RGBAUnorm => Self::BC3RGBAUnormSrgb,
            Self::BC7RGBAUnorm => Self::BC7RGBAUnormSrgb,
            Self::ASTC4x4Unorm => Self::ASTC4x4UnormSrgb,
            Self::ASTC5x5Unorm => Self::ASTC5x5UnormSrgb,
            Self::ASTC6x6Unorm => Self::ASTC6x6UnormSrgb,
            Self::ASTC8x8Unorm => Self::ASTC8x8UnormSrgb,
            other => *other,
        }
    }

    /// Get linear variant.
    pub fn to_linear(&self) -> Self {
        match self {
            Self::RGBA8UnormSrgb => Self::RGBA8Unorm,
            Self::BGRA8UnormSrgb => Self::BGRA8Unorm,
            Self::BC1RGBAUnormSrgb => Self::BC1RGBAUnorm,
            Self::BC2RGBAUnormSrgb => Self::BC2RGBAUnorm,
            Self::BC3RGBAUnormSrgb => Self::BC3RGBAUnorm,
            Self::BC7RGBAUnormSrgb => Self::BC7RGBAUnorm,
            Self::ASTC4x4UnormSrgb => Self::ASTC4x4Unorm,
            Self::ASTC5x5UnormSrgb => Self::ASTC5x5Unorm,
            Self::ASTC6x6UnormSrgb => Self::ASTC6x6Unorm,
            Self::ASTC8x8UnormSrgb => Self::ASTC8x8Unorm,
            other => *other,
        }
    }
}

/// Sample count for MSAA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SampleCount {
    /// No MSAA.
    X1,
    /// 2x MSAA.
    X2,
    /// 4x MSAA.
    X4,
    /// 8x MSAA.
    X8,
    /// 16x MSAA.
    X16,
    /// 32x MSAA.
    X32,
    /// 64x MSAA.
    X64,
}

impl SampleCount {
    /// Get count as integer.
    pub fn count(&self) -> u32 {
        match self {
            Self::X1 => 1,
            Self::X2 => 2,
            Self::X4 => 4,
            Self::X8 => 8,
            Self::X16 => 16,
            Self::X32 => 32,
            Self::X64 => 64,
        }
    }

    /// From integer.
    pub fn from_count(count: u32) -> Self {
        match count {
            1 => Self::X1,
            2 => Self::X2,
            4 => Self::X4,
            8 => Self::X8,
            16 => Self::X16,
            32 => Self::X32,
            64 => Self::X64,
            _ => Self::X1,
        }
    }
}

impl Default for SampleCount {
    fn default() -> Self {
        Self::X1
    }
}

/// Texture dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureDimension {
    /// 1D texture.
    D1,
    /// 2D texture.
    D2,
    /// 3D volume texture.
    D3,
    /// Cube texture.
    Cube,
}

/// Texture usage flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureUsageFlags(u32);

impl TextureUsageFlags {
    /// Copy source.
    pub const COPY_SRC: Self = Self(1 << 0);
    /// Copy destination.
    pub const COPY_DST: Self = Self(1 << 1);
    /// Sampled texture.
    pub const SAMPLED: Self = Self(1 << 2);
    /// Storage texture.
    pub const STORAGE: Self = Self(1 << 3);
    /// Color attachment.
    pub const COLOR_ATTACHMENT: Self = Self(1 << 4);
    /// Depth/stencil attachment.
    pub const DEPTH_STENCIL_ATTACHMENT: Self = Self(1 << 5);
    /// Transient attachment.
    pub const TRANSIENT: Self = Self(1 << 6);
    /// Input attachment.
    pub const INPUT_ATTACHMENT: Self = Self(1 << 7);

    /// Check if contains flags.
    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Combine flags.
    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl core::ops::BitOr for TextureUsageFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitOrAssign for TextureUsageFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Buffer description.
#[derive(Debug, Clone)]
pub struct BufferDesc {
    /// Size in bytes.
    pub size: u64,
    /// Usage flags.
    pub usage: BufferUsageFlags,
    /// Memory location.
    pub memory: MemoryLocation,
    /// Required alignment.
    pub alignment: u64,
    /// Memory priority.
    pub priority: ResourcePriority,
    /// Debug name.
    pub name: String,
}

impl Default for BufferDesc {
    fn default() -> Self {
        Self {
            size: 0,
            usage: BufferUsageFlags::UNIFORM,
            memory: MemoryLocation::GpuOnly,
            alignment: 256,
            priority: ResourcePriority::Normal,
            name: String::new(),
        }
    }
}

impl BufferDesc {
    /// Create a uniform buffer.
    pub fn uniform(size: u64) -> Self {
        Self {
            size,
            usage: BufferUsageFlags::UNIFORM,
            alignment: 256,
            ..Default::default()
        }
    }

    /// Create a storage buffer.
    pub fn storage(size: u64) -> Self {
        Self {
            size,
            usage: BufferUsageFlags::STORAGE,
            alignment: 16,
            ..Default::default()
        }
    }

    /// Create a vertex buffer.
    pub fn vertex(size: u64) -> Self {
        Self {
            size,
            usage: BufferUsageFlags::VERTEX,
            alignment: 4,
            ..Default::default()
        }
    }

    /// Create an index buffer.
    pub fn index(size: u64) -> Self {
        Self {
            size,
            usage: BufferUsageFlags::INDEX,
            alignment: 4,
            ..Default::default()
        }
    }

    /// Create an indirect buffer.
    pub fn indirect(size: u64) -> Self {
        Self {
            size,
            usage: BufferUsageFlags::INDIRECT | BufferUsageFlags::STORAGE,
            alignment: 4,
            ..Default::default()
        }
    }

    /// Create a staging buffer.
    pub fn staging(size: u64) -> Self {
        Self {
            size,
            usage: BufferUsageFlags::COPY_SRC | BufferUsageFlags::COPY_DST,
            memory: MemoryLocation::CpuToGpu,
            alignment: 4,
            ..Default::default()
        }
    }

    /// Create a readback buffer.
    pub fn readback(size: u64) -> Self {
        Self {
            size,
            usage: BufferUsageFlags::COPY_DST,
            memory: MemoryLocation::GpuToCpu,
            alignment: 4,
            ..Default::default()
        }
    }

    /// With debug name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
}

/// Buffer usage flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferUsageFlags(u32);

impl BufferUsageFlags {
    /// Copy source.
    pub const COPY_SRC: Self = Self(1 << 0);
    /// Copy destination.
    pub const COPY_DST: Self = Self(1 << 1);
    /// Uniform buffer.
    pub const UNIFORM: Self = Self(1 << 2);
    /// Storage buffer.
    pub const STORAGE: Self = Self(1 << 3);
    /// Index buffer.
    pub const INDEX: Self = Self(1 << 4);
    /// Vertex buffer.
    pub const VERTEX: Self = Self(1 << 5);
    /// Indirect buffer.
    pub const INDIRECT: Self = Self(1 << 6);
    /// Acceleration structure.
    pub const ACCELERATION_STRUCTURE: Self = Self(1 << 7);
    /// Shader binding table.
    pub const SHADER_BINDING_TABLE: Self = Self(1 << 8);

    /// Check if contains flags.
    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl core::ops::BitOr for BufferUsageFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

/// Memory location preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryLocation {
    /// GPU-only memory (fastest).
    GpuOnly,
    /// CPU to GPU (upload).
    CpuToGpu,
    /// GPU to CPU (readback).
    GpuToCpu,
    /// CPU-only (staging).
    CpuOnly,
}

/// Resource priority for memory management.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResourcePriority {
    /// Low priority - can be evicted first.
    Low,
    /// Normal priority.
    Normal,
    /// High priority - prefer to keep.
    High,
    /// Critical - never evict.
    Critical,
}

/// Resource usage type for barrier generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceUsage {
    /// Undefined/initial state.
    Undefined,
    /// Shader read.
    ShaderRead,
    /// Shader write.
    ShaderWrite,
    /// Shader read/write.
    ShaderReadWrite,
    /// Color attachment.
    ColorAttachment,
    /// Depth/stencil attachment.
    DepthStencilAttachment,
    /// Depth/stencil read-only.
    DepthStencilRead,
    /// Storage image.
    StorageImage,
    /// Uniform buffer.
    UniformBuffer,
    /// Storage buffer.
    StorageBuffer,
    /// Vertex buffer.
    VertexBuffer,
    /// Index buffer.
    IndexBuffer,
    /// Indirect buffer.
    IndirectBuffer,
    /// Copy source.
    CopySrc,
    /// Copy destination.
    CopyDst,
    /// Present.
    Present,
    /// Ray tracing acceleration structure.
    AccelerationStructure,
}

impl ResourceUsage {
    /// Convert to resource state.
    pub fn to_resource_state(self) -> ResourceState {
        match self {
            Self::Undefined => ResourceState::Undefined,
            Self::ShaderRead => ResourceState::ShaderRead,
            Self::ShaderWrite => ResourceState::ShaderWrite,
            Self::ShaderReadWrite => ResourceState::General,
            Self::ColorAttachment => ResourceState::ColorAttachment,
            Self::DepthStencilAttachment => ResourceState::DepthStencilAttachment,
            Self::DepthStencilRead => ResourceState::DepthStencilRead,
            Self::StorageImage => ResourceState::Storage,
            Self::UniformBuffer => ResourceState::UniformBuffer,
            Self::StorageBuffer => ResourceState::Storage,
            Self::VertexBuffer => ResourceState::VertexBuffer,
            Self::IndexBuffer => ResourceState::IndexBuffer,
            Self::IndirectBuffer => ResourceState::IndirectBuffer,
            Self::CopySrc => ResourceState::CopySrc,
            Self::CopyDst => ResourceState::CopyDst,
            Self::Present => ResourceState::Present,
            Self::AccelerationStructure => ResourceState::AccelerationStructure,
        }
    }
}

/// Resource state for synchronization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceState {
    /// Undefined state.
    Undefined,
    /// General layout (all operations).
    General,
    /// Shader read.
    ShaderRead,
    /// Shader write.
    ShaderWrite,
    /// Color attachment.
    ColorAttachment,
    /// Depth/stencil attachment.
    DepthStencilAttachment,
    /// Depth/stencil read-only.
    DepthStencilRead,
    /// Storage.
    Storage,
    /// Uniform buffer.
    UniformBuffer,
    /// Vertex buffer.
    VertexBuffer,
    /// Index buffer.
    IndexBuffer,
    /// Indirect buffer.
    IndirectBuffer,
    /// Copy source.
    CopySrc,
    /// Copy destination.
    CopyDst,
    /// Present.
    Present,
    /// Acceleration structure.
    AccelerationStructure,
}

impl ResourceState {
    /// Convert to pipeline stage.
    pub fn to_pipeline_stage(self) -> PipelineStage {
        match self {
            Self::Undefined => PipelineStage::TOP_OF_PIPE,
            Self::General => PipelineStage::ALL_COMMANDS,
            Self::ShaderRead | Self::ShaderWrite | Self::Storage => {
                PipelineStage::VERTEX_SHADER
                    | PipelineStage::FRAGMENT_SHADER
                    | PipelineStage::COMPUTE_SHADER
            }
            Self::ColorAttachment => PipelineStage::COLOR_ATTACHMENT_OUTPUT,
            Self::DepthStencilAttachment | Self::DepthStencilRead => {
                PipelineStage::EARLY_FRAGMENT_TESTS | PipelineStage::LATE_FRAGMENT_TESTS
            }
            Self::UniformBuffer | Self::VertexBuffer | Self::IndexBuffer => {
                PipelineStage::VERTEX_INPUT | PipelineStage::VERTEX_SHADER
            }
            Self::IndirectBuffer => PipelineStage::DRAW_INDIRECT,
            Self::CopySrc | Self::CopyDst => PipelineStage::TRANSFER,
            Self::Present => PipelineStage::BOTTOM_OF_PIPE,
            Self::AccelerationStructure => PipelineStage::ACCELERATION_STRUCTURE_BUILD,
        }
    }

    /// Convert to access flags.
    pub fn to_access_flags(self) -> AccessFlags {
        match self {
            Self::Undefined => AccessFlags::NONE,
            Self::General => AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            Self::ShaderRead => AccessFlags::SHADER_READ,
            Self::ShaderWrite => AccessFlags::SHADER_WRITE,
            Self::Storage => AccessFlags::SHADER_READ | AccessFlags::SHADER_WRITE,
            Self::ColorAttachment => {
                AccessFlags::COLOR_ATTACHMENT_READ | AccessFlags::COLOR_ATTACHMENT_WRITE
            }
            Self::DepthStencilAttachment => {
                AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                    | AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE
            }
            Self::DepthStencilRead => AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ,
            Self::UniformBuffer => AccessFlags::UNIFORM_READ,
            Self::VertexBuffer => AccessFlags::VERTEX_ATTRIBUTE_READ,
            Self::IndexBuffer => AccessFlags::INDEX_READ,
            Self::IndirectBuffer => AccessFlags::INDIRECT_COMMAND_READ,
            Self::CopySrc => AccessFlags::TRANSFER_READ,
            Self::CopyDst => AccessFlags::TRANSFER_WRITE,
            Self::Present => AccessFlags::NONE,
            Self::AccelerationStructure => {
                AccessFlags::ACCELERATION_STRUCTURE_READ | AccessFlags::ACCELERATION_STRUCTURE_WRITE
            }
        }
    }
}

/// Resource pool for managing transient resources.
pub struct ResourcePool {
    /// Texture cache.
    texture_cache: Vec<CachedTexture>,
    /// Buffer cache.
    buffer_cache: Vec<CachedBuffer>,
    /// Frame index for tracking.
    frame_index: u64,
    /// Maximum cached textures.
    max_cached_textures: usize,
    /// Maximum cached buffers.
    max_cached_buffers: usize,
    /// Total cached memory.
    cached_memory: u64,
    /// Maximum cached memory.
    max_cached_memory: u64,
}

impl ResourcePool {
    /// Create a new resource pool.
    pub fn new() -> Self {
        Self {
            texture_cache: Vec::new(),
            buffer_cache: Vec::new(),
            frame_index: 0,
            max_cached_textures: 128,
            max_cached_buffers: 256,
            cached_memory: 0,
            max_cached_memory: 512 * 1024 * 1024, // 512 MB
        }
    }

    /// Configure pool limits.
    pub fn with_limits(mut self, max_textures: usize, max_buffers: usize, max_memory: u64) -> Self {
        self.max_cached_textures = max_textures;
        self.max_cached_buffers = max_buffers;
        self.max_cached_memory = max_memory;
        self
    }

    /// Acquire a texture from pool or create new.
    pub fn acquire_texture(&mut self, desc: &TextureDesc) -> TextureHandle {
        // Try to find compatible cached texture
        if let Some(pos) = self.find_compatible_texture(desc) {
            let cached = self.texture_cache.remove(pos);
            self.cached_memory -= cached.size;
            return cached.handle;
        }

        // Create new (in real implementation)
        TextureHandle::new(self.texture_cache.len() as u32)
    }

    /// Release a texture back to pool.
    pub fn release_texture(&mut self, handle: TextureHandle, desc: TextureDesc) {
        let size = desc.calculate_size();

        // Check if we should cache
        if self.texture_cache.len() >= self.max_cached_textures
            || self.cached_memory + size > self.max_cached_memory
        {
            self.evict_textures(size);
        }

        self.texture_cache.push(CachedTexture {
            handle,
            desc,
            size,
            last_use: self.frame_index,
        });
        self.cached_memory += size;
    }

    /// Acquire a buffer from pool.
    pub fn acquire_buffer(&mut self, desc: &BufferDesc) -> BufferHandle {
        if let Some(pos) = self.find_compatible_buffer(desc) {
            let cached = self.buffer_cache.remove(pos);
            self.cached_memory -= cached.size;
            return cached.handle;
        }

        BufferHandle::new(self.buffer_cache.len() as u32)
    }

    /// Release a buffer back to pool.
    pub fn release_buffer(&mut self, handle: BufferHandle, desc: BufferDesc) {
        let size = desc.size;

        if self.buffer_cache.len() >= self.max_cached_buffers
            || self.cached_memory + size > self.max_cached_memory
        {
            self.evict_buffers(size);
        }

        self.buffer_cache.push(CachedBuffer {
            handle,
            desc,
            size,
            last_use: self.frame_index,
        });
        self.cached_memory += size;
    }

    /// Advance to next frame.
    pub fn next_frame(&mut self) {
        self.frame_index += 1;
        self.cleanup_old_resources();
    }

    /// Get cached memory usage.
    pub fn cached_memory(&self) -> u64 {
        self.cached_memory
    }

    /// Get cache stats.
    pub fn stats(&self) -> ResourcePoolStats {
        ResourcePoolStats {
            cached_textures: self.texture_cache.len(),
            cached_buffers: self.buffer_cache.len(),
            cached_memory: self.cached_memory,
            frame_index: self.frame_index,
        }
    }

    fn find_compatible_texture(&self, desc: &TextureDesc) -> Option<usize> {
        self.texture_cache.iter().position(|cached| {
            cached.desc.width == desc.width
                && cached.desc.height == desc.height
                && cached.desc.depth == desc.depth
                && cached.desc.mip_levels >= desc.mip_levels
                && cached.desc.array_layers == desc.array_layers
                && cached.desc.format == desc.format
                && cached.desc.samples == desc.samples
                && cached.desc.usage.contains(desc.usage)
        })
    }

    fn find_compatible_buffer(&self, desc: &BufferDesc) -> Option<usize> {
        self.buffer_cache.iter().position(|cached| {
            cached.desc.size >= desc.size
                && cached.desc.usage.contains(desc.usage)
                && cached.desc.memory == desc.memory
        })
    }

    fn evict_textures(&mut self, needed: u64) {
        // Sort by last use (oldest first)
        self.texture_cache.sort_by_key(|t| t.last_use);

        let mut freed = 0u64;
        while freed < needed && !self.texture_cache.is_empty() {
            if let Some(evicted) = self.texture_cache.pop() {
                freed += evicted.size;
                self.cached_memory -= evicted.size;
            }
        }
    }

    fn evict_buffers(&mut self, needed: u64) {
        self.buffer_cache.sort_by_key(|b| b.last_use);

        let mut freed = 0u64;
        while freed < needed && !self.buffer_cache.is_empty() {
            if let Some(evicted) = self.buffer_cache.pop() {
                freed += evicted.size;
                self.cached_memory -= evicted.size;
            }
        }
    }

    fn cleanup_old_resources(&mut self) {
        const MAX_AGE: u64 = 60; // Keep for 60 frames

        let cutoff = self.frame_index.saturating_sub(MAX_AGE);

        self.texture_cache.retain(|t| t.last_use >= cutoff);
        self.buffer_cache.retain(|b| b.last_use >= cutoff);

        // Recalculate cached memory
        self.cached_memory = self.texture_cache.iter().map(|t| t.size).sum::<u64>()
            + self.buffer_cache.iter().map(|b| b.size).sum::<u64>();
    }
}

impl Default for ResourcePool {
    fn default() -> Self {
        Self::new()
    }
}

/// Cached texture entry.
struct CachedTexture {
    handle: TextureHandle,
    desc: TextureDesc,
    size: u64,
    last_use: u64,
}

/// Cached buffer entry.
struct CachedBuffer {
    handle: BufferHandle,
    desc: BufferDesc,
    size: u64,
    last_use: u64,
}

/// Resource pool statistics.
#[derive(Debug, Clone)]
pub struct ResourcePoolStats {
    /// Number of cached textures.
    pub cached_textures: usize,
    /// Number of cached buffers.
    pub cached_buffers: usize,
    /// Total cached memory.
    pub cached_memory: u64,
    /// Current frame index.
    pub frame_index: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_texture_desc_size() {
        let desc = TextureDesc::color_2d(1920, 1080);
        let size = desc.calculate_size();
        assert_eq!(size, 1920 * 1080 * 4); // RGBA8 = 4 bytes per pixel
    }

    #[test]
    fn test_texture_format_depth() {
        assert!(TextureFormat::D32Float.is_depth());
        assert!(!TextureFormat::RGBA8Unorm.is_depth());
    }

    #[test]
    fn test_resource_pool() {
        let mut pool = ResourcePool::new();
        let desc = BufferDesc::uniform(1024);
        let handle = pool.acquire_buffer(&desc);
        pool.release_buffer(handle, desc.clone());
        assert_eq!(pool.buffer_cache.len(), 1);
    }
}
