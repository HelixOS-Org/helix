//! Compute shader support
//!
//! This module provides types and utilities for compute shader dispatch.

use crate::types::{BufferHandle, PipelineHandle, TextureHandle};

/// Compute pipeline descriptor
#[derive(Clone, Debug)]
pub struct ComputePipelineDesc<'a> {
    /// Label for debugging
    pub label: Option<&'a str>,
    /// Shader module
    pub shader: ShaderSource<'a>,
    /// Entry point function name
    pub entry_point: &'a str,
    /// Push constant range
    pub push_constant_size: u32,
    /// Bind group layouts
    pub bind_group_layouts: &'a [BindGroupLayoutDesc<'a>],
}

impl<'a> ComputePipelineDesc<'a> {
    /// Creates a new compute pipeline descriptor
    pub const fn new(shader: ShaderSource<'a>, entry_point: &'a str) -> Self {
        Self {
            label: None,
            shader,
            entry_point,
            push_constant_size: 0,
            bind_group_layouts: &[],
        }
    }

    /// Sets the label
    pub const fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// Sets push constant size
    pub const fn with_push_constants(mut self, size: u32) -> Self {
        self.push_constant_size = size;
        self
    }

    /// Sets bind group layouts
    pub const fn with_bind_groups(mut self, layouts: &'a [BindGroupLayoutDesc<'a>]) -> Self {
        self.bind_group_layouts = layouts;
        self
    }
}

/// Shader source
#[derive(Clone, Debug)]
pub enum ShaderSource<'a> {
    /// SPIR-V binary
    SpirV(&'a [u32]),
    /// GLSL source (will be compiled)
    Glsl { source: &'a str, stage: ShaderStage },
}

/// Shader stage
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShaderStage {
    /// Vertex shader
    Vertex,
    /// Fragment shader
    Fragment,
    /// Compute shader
    Compute,
    /// Geometry shader
    Geometry,
    /// Tessellation control shader
    TessControl,
    /// Tessellation evaluation shader
    TessEval,
}

/// Bind group layout descriptor
#[derive(Clone, Debug)]
pub struct BindGroupLayoutDesc<'a> {
    /// Label for debugging
    pub label: Option<&'a str>,
    /// Binding entries
    pub entries: &'a [BindGroupLayoutEntry],
}

/// Single entry in a bind group layout
#[derive(Clone, Copy, Debug)]
pub struct BindGroupLayoutEntry {
    /// Binding index
    pub binding: u32,
    /// Visibility in shader stages
    pub visibility: ShaderStageFlags,
    /// Type of binding
    pub ty: BindingType,
    /// Count (for arrays)
    pub count: Option<u32>,
}

/// Shader stage visibility flags
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct ShaderStageFlags(pub u32);

impl ShaderStageFlags {
    /// No stages
    pub const NONE: Self = Self(0);
    /// Vertex stage
    pub const VERTEX: Self = Self(0x01);
    /// Fragment stage
    pub const FRAGMENT: Self = Self(0x02);
    /// Compute stage
    pub const COMPUTE: Self = Self(0x04);
    /// All graphics stages
    pub const ALL_GRAPHICS: Self = Self(0x03);
    /// All stages
    pub const ALL: Self = Self(0x07);
}

impl core::ops::BitOr for ShaderStageFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Type of resource binding
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingType {
    /// Uniform buffer
    UniformBuffer,
    /// Storage buffer (read-only)
    StorageBufferReadOnly,
    /// Storage buffer (read-write)
    StorageBuffer,
    /// Sampled texture
    SampledTexture {
        /// Texture dimension
        dimension: TextureDimension,
        /// Component type
        sample_type: TextureSampleType,
        /// Is multisampled
        multisampled: bool,
    },
    /// Storage texture
    StorageTexture {
        /// Texture dimension
        dimension: TextureDimension,
        /// Access mode
        access: StorageTextureAccess,
        /// Texture format
        format: TextureFormat,
    },
    /// Sampler
    Sampler(SamplerBindingType),
}

/// Texture dimension for bindings
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureDimension {
    /// 1D texture
    D1,
    /// 2D texture
    D2,
    /// 3D texture
    D3,
    /// Cube texture
    Cube,
    /// 2D array texture
    D2Array,
    /// Cube array texture
    CubeArray,
}

/// Texture sample type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureSampleType {
    /// Float (filterable)
    Float { filterable: bool },
    /// Signed integer
    Sint,
    /// Unsigned integer
    Uint,
    /// Depth comparison
    Depth,
}

/// Storage texture access mode
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageTextureAccess {
    /// Read-only
    ReadOnly,
    /// Write-only
    WriteOnly,
    /// Read-write
    ReadWrite,
}

/// Sampler binding type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SamplerBindingType {
    /// Regular filtering sampler
    Filtering,
    /// Non-filtering sampler
    NonFiltering,
    /// Comparison sampler
    Comparison,
}

/// Texture format
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureFormat {
    // 8-bit formats
    /// R8 unsigned normalized
    R8Unorm,
    /// R8 signed normalized
    R8Snorm,
    /// R8 unsigned integer
    R8Uint,
    /// R8 signed integer
    R8Sint,

    // 16-bit formats
    /// R16 unsigned normalized
    R16Unorm,
    /// R16 signed normalized
    R16Snorm,
    /// R16 unsigned integer
    R16Uint,
    /// R16 signed integer
    R16Sint,
    /// R16 float
    R16Float,
    /// RG8 unsigned normalized
    Rg8Unorm,
    /// RG8 signed normalized
    Rg8Snorm,
    /// RG8 unsigned integer
    Rg8Uint,
    /// RG8 signed integer
    Rg8Sint,

    // 32-bit formats
    /// R32 unsigned integer
    R32Uint,
    /// R32 signed integer
    R32Sint,
    /// R32 float
    R32Float,
    /// RG16 unsigned normalized
    Rg16Unorm,
    /// RG16 signed normalized
    Rg16Snorm,
    /// RG16 unsigned integer
    Rg16Uint,
    /// RG16 signed integer
    Rg16Sint,
    /// RG16 float
    Rg16Float,
    /// RGBA8 unsigned normalized
    Rgba8Unorm,
    /// RGBA8 unsigned normalized sRGB
    Rgba8UnormSrgb,
    /// RGBA8 signed normalized
    Rgba8Snorm,
    /// RGBA8 unsigned integer
    Rgba8Uint,
    /// RGBA8 signed integer
    Rgba8Sint,
    /// BGRA8 unsigned normalized
    Bgra8Unorm,
    /// BGRA8 unsigned normalized sRGB
    Bgra8UnormSrgb,
    /// RGB10A2 unsigned normalized
    Rgb10a2Unorm,
    /// RG11B10 float
    Rg11b10Float,

    // 64-bit formats
    /// RG32 unsigned integer
    Rg32Uint,
    /// RG32 signed integer
    Rg32Sint,
    /// RG32 float
    Rg32Float,
    /// RGBA16 unsigned normalized
    Rgba16Unorm,
    /// RGBA16 signed normalized
    Rgba16Snorm,
    /// RGBA16 unsigned integer
    Rgba16Uint,
    /// RGBA16 signed integer
    Rgba16Sint,
    /// RGBA16 float
    Rgba16Float,

    // 128-bit formats
    /// RGBA32 unsigned integer
    Rgba32Uint,
    /// RGBA32 signed integer
    Rgba32Sint,
    /// RGBA32 float
    Rgba32Float,

    // Depth/stencil formats
    /// Depth 16-bit unsigned normalized
    Depth16Unorm,
    /// Depth 24-bit unsigned normalized
    Depth24Plus,
    /// Depth 24-bit + Stencil 8-bit
    Depth24PlusStencil8,
    /// Depth 32-bit float
    Depth32Float,
    /// Depth 32-bit float + Stencil 8-bit
    Depth32FloatStencil8,
    /// Stencil 8-bit
    Stencil8,

    // Compressed formats
    /// BC1 RGBA unsigned normalized
    Bc1RgbaUnorm,
    /// BC1 RGBA unsigned normalized sRGB
    Bc1RgbaUnormSrgb,
    /// BC2 RGBA unsigned normalized
    Bc2RgbaUnorm,
    /// BC2 RGBA unsigned normalized sRGB
    Bc2RgbaUnormSrgb,
    /// BC3 RGBA unsigned normalized
    Bc3RgbaUnorm,
    /// BC3 RGBA unsigned normalized sRGB
    Bc3RgbaUnormSrgb,
    /// BC4 R unsigned normalized
    Bc4RUnorm,
    /// BC4 R signed normalized
    Bc4RSnorm,
    /// BC5 RG unsigned normalized
    Bc5RgUnorm,
    /// BC5 RG signed normalized
    Bc5RgSnorm,
    /// BC6H RGB unsigned float
    Bc6hRgbUfloat,
    /// BC6H RGB signed float
    Bc6hRgbFloat,
    /// BC7 RGBA unsigned normalized
    Bc7RgbaUnorm,
    /// BC7 RGBA unsigned normalized sRGB
    Bc7RgbaUnormSrgb,
}

impl TextureFormat {
    /// Returns the size of one pixel in bytes
    pub const fn bytes_per_pixel(&self) -> u32 {
        match self {
            Self::R8Unorm | Self::R8Snorm | Self::R8Uint | Self::R8Sint | Self::Stencil8 => 1,
            Self::R16Unorm
            | Self::R16Snorm
            | Self::R16Uint
            | Self::R16Sint
            | Self::R16Float
            | Self::Rg8Unorm
            | Self::Rg8Snorm
            | Self::Rg8Uint
            | Self::Rg8Sint
            | Self::Depth16Unorm => 2,
            Self::Depth24Plus => 3,
            Self::R32Uint
            | Self::R32Sint
            | Self::R32Float
            | Self::Rg16Unorm
            | Self::Rg16Snorm
            | Self::Rg16Uint
            | Self::Rg16Sint
            | Self::Rg16Float
            | Self::Rgba8Unorm
            | Self::Rgba8UnormSrgb
            | Self::Rgba8Snorm
            | Self::Rgba8Uint
            | Self::Rgba8Sint
            | Self::Bgra8Unorm
            | Self::Bgra8UnormSrgb
            | Self::Rgb10a2Unorm
            | Self::Rg11b10Float
            | Self::Depth24PlusStencil8
            | Self::Depth32Float => 4,
            Self::Depth32FloatStencil8 => 5,
            Self::Rg32Uint
            | Self::Rg32Sint
            | Self::Rg32Float
            | Self::Rgba16Unorm
            | Self::Rgba16Snorm
            | Self::Rgba16Uint
            | Self::Rgba16Sint
            | Self::Rgba16Float => 8,
            Self::Rgba32Uint | Self::Rgba32Sint | Self::Rgba32Float => 16,
            // Compressed formats return block size / pixels per block
            _ => 0,
        }
    }

    /// Returns true if this is a depth format
    pub const fn is_depth(&self) -> bool {
        matches!(
            self,
            Self::Depth16Unorm
                | Self::Depth24Plus
                | Self::Depth24PlusStencil8
                | Self::Depth32Float
                | Self::Depth32FloatStencil8
        )
    }

    /// Returns true if this is a stencil format
    pub const fn is_stencil(&self) -> bool {
        matches!(
            self,
            Self::Depth24PlusStencil8 | Self::Depth32FloatStencil8 | Self::Stencil8
        )
    }

    /// Returns true if this is an sRGB format
    pub const fn is_srgb(&self) -> bool {
        matches!(
            self,
            Self::Rgba8UnormSrgb
                | Self::Bgra8UnormSrgb
                | Self::Bc1RgbaUnormSrgb
                | Self::Bc2RgbaUnormSrgb
                | Self::Bc3RgbaUnormSrgb
                | Self::Bc7RgbaUnormSrgb
        )
    }

    /// Returns true if this is a compressed format
    pub const fn is_compressed(&self) -> bool {
        matches!(
            self,
            Self::Bc1RgbaUnorm
                | Self::Bc1RgbaUnormSrgb
                | Self::Bc2RgbaUnorm
                | Self::Bc2RgbaUnormSrgb
                | Self::Bc3RgbaUnorm
                | Self::Bc3RgbaUnormSrgb
                | Self::Bc4RUnorm
                | Self::Bc4RSnorm
                | Self::Bc5RgUnorm
                | Self::Bc5RgSnorm
                | Self::Bc6hRgbUfloat
                | Self::Bc6hRgbFloat
                | Self::Bc7RgbaUnorm
                | Self::Bc7RgbaUnormSrgb
        )
    }
}

/// Compute dispatch descriptor
#[derive(Clone, Copy, Debug)]
pub struct DispatchDesc {
    /// Number of workgroups in X dimension
    pub x: u32,
    /// Number of workgroups in Y dimension
    pub y: u32,
    /// Number of workgroups in Z dimension
    pub z: u32,
}

impl DispatchDesc {
    /// Creates a 1D dispatch
    pub const fn d1(x: u32) -> Self {
        Self { x, y: 1, z: 1 }
    }

    /// Creates a 2D dispatch
    pub const fn d2(x: u32, y: u32) -> Self {
        Self { x, y, z: 1 }
    }

    /// Creates a 3D dispatch
    pub const fn d3(x: u32, y: u32, z: u32) -> Self {
        Self { x, y, z }
    }

    /// Calculates dispatch size for a given work size and local size
    pub const fn for_size(
        work_x: u32,
        work_y: u32,
        work_z: u32,
        local_x: u32,
        local_y: u32,
        local_z: u32,
    ) -> Self {
        Self {
            x: (work_x + local_x - 1) / local_x,
            y: (work_y + local_y - 1) / local_y,
            z: (work_z + local_z - 1) / local_z,
        }
    }
}

/// Indirect dispatch buffer
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DispatchIndirectCommand {
    /// Number of workgroups in X
    pub x: u32,
    /// Number of workgroups in Y
    pub y: u32,
    /// Number of workgroups in Z
    pub z: u32,
}

impl DispatchIndirectCommand {
    /// Creates a new indirect dispatch command
    pub const fn new(x: u32, y: u32, z: u32) -> Self {
        Self { x, y, z }
    }
}
