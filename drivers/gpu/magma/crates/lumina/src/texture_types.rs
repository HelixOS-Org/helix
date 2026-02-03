//! Texture Types for Lumina
//!
//! This module provides comprehensive texture types, views, sampling,
//! and texture management infrastructure.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Texture Handle
// ============================================================================

/// Texture handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TextureHandle(pub u64);

impl TextureHandle {
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

impl Default for TextureHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Texture view handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TextureViewHandle(pub u64);

impl TextureViewHandle {
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

impl Default for TextureViewHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Texture Create Info
// ============================================================================

/// Texture create info
#[derive(Clone, Debug)]
pub struct TextureCreateInfo {
    /// Texture type
    pub texture_type: TextureType,
    /// Format
    pub format: TextureFormat,
    /// Extent
    pub extent: Extent3D,
    /// Mip levels
    pub mip_levels: u32,
    /// Array layers
    pub array_layers: u32,
    /// Sample count
    pub samples: SampleCount,
    /// Tiling
    pub tiling: TextureTiling,
    /// Usage flags
    pub usage: TextureUsageFlags,
    /// Initial layout
    pub initial_layout: ImageLayout,
    /// Sharing mode
    pub sharing_mode: SharingMode,
    /// Queue family indices (for concurrent sharing)
    pub queue_family_indices: Vec<u32>,
    /// Create flags
    pub flags: TextureCreateFlags,
    /// Debug name
    pub debug_name: Option<String>,
}

impl TextureCreateInfo {
    /// Creates 1D texture
    pub fn d1(format: TextureFormat, width: u32) -> Self {
        Self {
            texture_type: TextureType::D1,
            format,
            extent: Extent3D::new(width, 1, 1),
            mip_levels: 1,
            array_layers: 1,
            samples: SampleCount::S1,
            tiling: TextureTiling::Optimal,
            usage: TextureUsageFlags::SAMPLED,
            initial_layout: ImageLayout::Undefined,
            sharing_mode: SharingMode::Exclusive,
            queue_family_indices: Vec::new(),
            flags: TextureCreateFlags::NONE,
            debug_name: None,
        }
    }

    /// Creates 2D texture
    pub fn d2(format: TextureFormat, width: u32, height: u32) -> Self {
        Self {
            texture_type: TextureType::D2,
            format,
            extent: Extent3D::new(width, height, 1),
            mip_levels: 1,
            array_layers: 1,
            samples: SampleCount::S1,
            tiling: TextureTiling::Optimal,
            usage: TextureUsageFlags::SAMPLED,
            initial_layout: ImageLayout::Undefined,
            sharing_mode: SharingMode::Exclusive,
            queue_family_indices: Vec::new(),
            flags: TextureCreateFlags::NONE,
            debug_name: None,
        }
    }

    /// Creates 3D texture
    pub fn d3(format: TextureFormat, width: u32, height: u32, depth: u32) -> Self {
        Self {
            texture_type: TextureType::D3,
            format,
            extent: Extent3D::new(width, height, depth),
            mip_levels: 1,
            array_layers: 1,
            samples: SampleCount::S1,
            tiling: TextureTiling::Optimal,
            usage: TextureUsageFlags::SAMPLED,
            initial_layout: ImageLayout::Undefined,
            sharing_mode: SharingMode::Exclusive,
            queue_family_indices: Vec::new(),
            flags: TextureCreateFlags::NONE,
            debug_name: None,
        }
    }

    /// Creates cubemap texture
    pub fn cube(format: TextureFormat, size: u32) -> Self {
        Self {
            texture_type: TextureType::D2,
            format,
            extent: Extent3D::new(size, size, 1),
            mip_levels: 1,
            array_layers: 6,
            samples: SampleCount::S1,
            tiling: TextureTiling::Optimal,
            usage: TextureUsageFlags::SAMPLED,
            initial_layout: ImageLayout::Undefined,
            sharing_mode: SharingMode::Exclusive,
            queue_family_indices: Vec::new(),
            flags: TextureCreateFlags::CUBE_COMPATIBLE,
            debug_name: None,
        }
    }

    /// Creates render target
    pub fn render_target(format: TextureFormat, width: u32, height: u32) -> Self {
        Self::d2(format, width, height)
            .with_usage(TextureUsageFlags::COLOR_ATTACHMENT | TextureUsageFlags::SAMPLED)
    }

    /// Creates depth buffer
    pub fn depth_buffer(format: TextureFormat, width: u32, height: u32) -> Self {
        Self::d2(format, width, height)
            .with_usage(TextureUsageFlags::DEPTH_STENCIL_ATTACHMENT | TextureUsageFlags::SAMPLED)
    }

    /// Creates storage texture
    pub fn storage(format: TextureFormat, width: u32, height: u32) -> Self {
        Self::d2(format, width, height)
            .with_usage(TextureUsageFlags::STORAGE | TextureUsageFlags::SAMPLED)
    }

    /// With mip levels
    pub fn with_mips(mut self, levels: u32) -> Self {
        self.mip_levels = levels;
        self
    }

    /// With auto mip levels
    pub fn with_auto_mips(mut self) -> Self {
        self.mip_levels = self.extent.mip_levels();
        self
    }

    /// With array layers
    pub fn with_layers(mut self, layers: u32) -> Self {
        self.array_layers = layers;
        self
    }

    /// With samples
    pub fn with_samples(mut self, samples: SampleCount) -> Self {
        self.samples = samples;
        self
    }

    /// With usage
    pub fn with_usage(mut self, usage: TextureUsageFlags) -> Self {
        self.usage = usage;
        self
    }

    /// With tiling
    pub fn with_tiling(mut self, tiling: TextureTiling) -> Self {
        self.tiling = tiling;
        self
    }

    /// With debug name
    pub fn with_name(mut self, name: &str) -> Self {
        self.debug_name = Some(String::from(name));
        self
    }

    /// With flags
    pub fn with_flags(mut self, flags: TextureCreateFlags) -> Self {
        self.flags = flags;
        self
    }
}

impl Default for TextureCreateInfo {
    fn default() -> Self {
        Self::d2(TextureFormat::Rgba8Unorm, 1, 1)
    }
}

// ============================================================================
// Texture Type
// ============================================================================

/// Texture type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum TextureType {
    /// 1D texture
    D1 = 0,
    /// 2D texture
    #[default]
    D2 = 1,
    /// 3D texture
    D3 = 2,
}

impl TextureType {
    /// Is 1D
    #[inline]
    pub const fn is_1d(&self) -> bool {
        matches!(self, Self::D1)
    }

    /// Is 2D
    #[inline]
    pub const fn is_2d(&self) -> bool {
        matches!(self, Self::D2)
    }

    /// Is 3D
    #[inline]
    pub const fn is_3d(&self) -> bool {
        matches!(self, Self::D3)
    }
}

// ============================================================================
// Texture Format
// ============================================================================

/// Texture format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum TextureFormat {
    /// R8 unorm
    R8Unorm = 9,
    /// R8 snorm
    R8Snorm = 10,
    /// R8 uint
    R8Uint = 13,
    /// R8 sint
    R8Sint = 14,
    /// RG8 unorm
    Rg8Unorm = 16,
    /// RG8 snorm
    Rg8Snorm = 17,
    /// RG8 uint
    Rg8Uint = 20,
    /// RG8 sint
    Rg8Sint = 21,
    /// RGBA8 unorm
    #[default]
    Rgba8Unorm = 37,
    /// RGBA8 sRGB
    Rgba8Srgb = 43,
    /// RGBA8 snorm
    Rgba8Snorm = 38,
    /// RGBA8 uint
    Rgba8Uint = 41,
    /// RGBA8 sint
    Rgba8Sint = 42,
    /// BGRA8 unorm
    Bgra8Unorm = 44,
    /// BGRA8 sRGB
    Bgra8Srgb = 50,
    /// R16 uint
    R16Uint = 74,
    /// R16 sint
    R16Sint = 75,
    /// R16 float
    R16Float = 76,
    /// RG16 uint
    Rg16Uint = 81,
    /// RG16 sint
    Rg16Sint = 82,
    /// RG16 float
    Rg16Float = 83,
    /// RGBA16 uint
    Rgba16Uint = 95,
    /// RGBA16 sint
    Rgba16Sint = 96,
    /// RGBA16 float
    Rgba16Float = 97,
    /// R32 uint
    R32Uint = 98,
    /// R32 sint
    R32Sint = 99,
    /// R32 float
    R32Float = 100,
    /// RG32 uint
    Rg32Uint = 101,
    /// RG32 sint
    Rg32Sint = 102,
    /// RG32 float
    Rg32Float = 103,
    /// RGBA32 uint
    Rgba32Uint = 107,
    /// RGBA32 sint
    Rgba32Sint = 108,
    /// RGBA32 float
    Rgba32Float = 109,
    /// RGB10A2 unorm
    Rgb10A2Unorm = 64,
    /// RGB10A2 uint
    Rgb10A2Uint = 65,
    /// R11G11B10 float
    R11G11B10Float = 58,
    /// Depth 16
    D16Unorm = 124,
    /// Depth 32 float
    D32Float = 126,
    /// Stencil 8
    S8Uint = 127,
    /// Depth 24 stencil 8
    D24UnormS8Uint = 129,
    /// Depth 32 float stencil 8
    D32FloatS8Uint = 130,
    /// BC1 RGBA unorm
    Bc1RgbaUnorm = 131,
    /// BC1 RGBA sRGB
    Bc1RgbaSrgb = 132,
    /// BC2 unorm
    Bc2Unorm = 135,
    /// BC2 sRGB
    Bc2Srgb = 136,
    /// BC3 unorm
    Bc3Unorm = 137,
    /// BC3 sRGB
    Bc3Srgb = 138,
    /// BC4 unorm
    Bc4Unorm = 139,
    /// BC4 snorm
    Bc4Snorm = 140,
    /// BC5 unorm
    Bc5Unorm = 141,
    /// BC5 snorm
    Bc5Snorm = 142,
    /// BC6H ufloat
    Bc6hUfloat = 143,
    /// BC6H sfloat
    Bc6hSfloat = 144,
    /// BC7 unorm
    Bc7Unorm = 145,
    /// BC7 sRGB
    Bc7Srgb = 146,
}

impl TextureFormat {
    /// Is depth format
    #[inline]
    pub const fn is_depth(&self) -> bool {
        matches!(
            self,
            Self::D16Unorm | Self::D32Float | Self::D24UnormS8Uint | Self::D32FloatS8Uint
        )
    }

    /// Is stencil format
    #[inline]
    pub const fn is_stencil(&self) -> bool {
        matches!(
            self,
            Self::S8Uint | Self::D24UnormS8Uint | Self::D32FloatS8Uint
        )
    }

    /// Is depth-stencil format
    #[inline]
    pub const fn is_depth_stencil(&self) -> bool {
        self.is_depth() || self.is_stencil()
    }

    /// Is color format
    #[inline]
    pub const fn is_color(&self) -> bool {
        !self.is_depth_stencil()
    }

    /// Is compressed format
    #[inline]
    pub const fn is_compressed(&self) -> bool {
        matches!(
            self,
            Self::Bc1RgbaUnorm
                | Self::Bc1RgbaSrgb
                | Self::Bc2Unorm
                | Self::Bc2Srgb
                | Self::Bc3Unorm
                | Self::Bc3Srgb
                | Self::Bc4Unorm
                | Self::Bc4Snorm
                | Self::Bc5Unorm
                | Self::Bc5Snorm
                | Self::Bc6hUfloat
                | Self::Bc6hSfloat
                | Self::Bc7Unorm
                | Self::Bc7Srgb
        )
    }

    /// Is sRGB format
    #[inline]
    pub const fn is_srgb(&self) -> bool {
        matches!(
            self,
            Self::Rgba8Srgb
                | Self::Bgra8Srgb
                | Self::Bc1RgbaSrgb
                | Self::Bc2Srgb
                | Self::Bc3Srgb
                | Self::Bc7Srgb
        )
    }

    /// Bytes per pixel (for uncompressed formats)
    pub const fn bytes_per_pixel(&self) -> Option<u32> {
        match self {
            Self::R8Unorm | Self::R8Snorm | Self::R8Uint | Self::R8Sint | Self::S8Uint => Some(1),
            Self::Rg8Unorm
            | Self::Rg8Snorm
            | Self::Rg8Uint
            | Self::Rg8Sint
            | Self::R16Uint
            | Self::R16Sint
            | Self::R16Float
            | Self::D16Unorm => Some(2),
            Self::Rgba8Unorm
            | Self::Rgba8Srgb
            | Self::Rgba8Snorm
            | Self::Rgba8Uint
            | Self::Rgba8Sint
            | Self::Bgra8Unorm
            | Self::Bgra8Srgb
            | Self::Rg16Uint
            | Self::Rg16Sint
            | Self::Rg16Float
            | Self::R32Uint
            | Self::R32Sint
            | Self::R32Float
            | Self::Rgb10A2Unorm
            | Self::Rgb10A2Uint
            | Self::R11G11B10Float
            | Self::D32Float
            | Self::D24UnormS8Uint => Some(4),
            Self::D32FloatS8Uint => Some(5),
            Self::Rgba16Uint
            | Self::Rgba16Sint
            | Self::Rgba16Float
            | Self::Rg32Uint
            | Self::Rg32Sint
            | Self::Rg32Float => Some(8),
            Self::Rgba32Uint | Self::Rgba32Sint | Self::Rgba32Float => Some(16),
            // Compressed formats - bytes per block
            _ => None,
        }
    }

    /// Block size for compressed formats (returns 4x4 for BC formats)
    pub const fn block_size(&self) -> (u32, u32) {
        if self.is_compressed() {
            (4, 4)
        } else {
            (1, 1)
        }
    }

    /// Bytes per block for compressed formats
    pub const fn bytes_per_block(&self) -> u32 {
        match self {
            Self::Bc1RgbaUnorm | Self::Bc1RgbaSrgb | Self::Bc4Unorm | Self::Bc4Snorm => 8,
            Self::Bc2Unorm
            | Self::Bc2Srgb
            | Self::Bc3Unorm
            | Self::Bc3Srgb
            | Self::Bc5Unorm
            | Self::Bc5Snorm
            | Self::Bc6hUfloat
            | Self::Bc6hSfloat
            | Self::Bc7Unorm
            | Self::Bc7Srgb => 16,
            _ => self.bytes_per_pixel().unwrap_or(4),
        }
    }

    /// Component count
    pub const fn component_count(&self) -> u32 {
        match self {
            Self::R8Unorm
            | Self::R8Snorm
            | Self::R8Uint
            | Self::R8Sint
            | Self::R16Uint
            | Self::R16Sint
            | Self::R16Float
            | Self::R32Uint
            | Self::R32Sint
            | Self::R32Float
            | Self::D16Unorm
            | Self::D32Float
            | Self::S8Uint
            | Self::Bc4Unorm
            | Self::Bc4Snorm => 1,
            Self::Rg8Unorm
            | Self::Rg8Snorm
            | Self::Rg8Uint
            | Self::Rg8Sint
            | Self::Rg16Uint
            | Self::Rg16Sint
            | Self::Rg16Float
            | Self::Rg32Uint
            | Self::Rg32Sint
            | Self::Rg32Float
            | Self::D24UnormS8Uint
            | Self::D32FloatS8Uint
            | Self::Bc5Unorm
            | Self::Bc5Snorm => 2,
            Self::R11G11B10Float | Self::Bc6hUfloat | Self::Bc6hSfloat => 3,
            _ => 4,
        }
    }
}

// ============================================================================
// Texture Usage Flags
// ============================================================================

/// Texture usage flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct TextureUsageFlags(pub u32);

impl TextureUsageFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Transfer source
    pub const TRANSFER_SRC: Self = Self(0x00000001);
    /// Transfer destination
    pub const TRANSFER_DST: Self = Self(0x00000002);
    /// Sampled
    pub const SAMPLED: Self = Self(0x00000004);
    /// Storage
    pub const STORAGE: Self = Self(0x00000008);
    /// Color attachment
    pub const COLOR_ATTACHMENT: Self = Self(0x00000010);
    /// Depth stencil attachment
    pub const DEPTH_STENCIL_ATTACHMENT: Self = Self(0x00000020);
    /// Transient attachment
    pub const TRANSIENT_ATTACHMENT: Self = Self(0x00000040);
    /// Input attachment
    pub const INPUT_ATTACHMENT: Self = Self(0x00000080);
    /// Fragment shading rate attachment
    pub const FRAGMENT_SHADING_RATE_ATTACHMENT: Self = Self(0x00000100);

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

impl core::ops::BitOr for TextureUsageFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

impl core::ops::BitOrAssign for TextureUsageFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

// ============================================================================
// Texture Create Flags
// ============================================================================

/// Texture create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct TextureCreateFlags(pub u32);

impl TextureCreateFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Sparse binding
    pub const SPARSE_BINDING: Self = Self(0x00000001);
    /// Sparse residency
    pub const SPARSE_RESIDENCY: Self = Self(0x00000002);
    /// Sparse aliased
    pub const SPARSE_ALIASED: Self = Self(0x00000004);
    /// Mutable format
    pub const MUTABLE_FORMAT: Self = Self(0x00000008);
    /// Cube compatible
    pub const CUBE_COMPATIBLE: Self = Self(0x00000010);
    /// Alias
    pub const ALIAS: Self = Self(0x00000400);
    /// Split instance bind regions
    pub const SPLIT_INSTANCE_BIND_REGIONS: Self = Self(0x00000040);
    /// 2D array compatible
    pub const D2_ARRAY_COMPATIBLE: Self = Self(0x00000020);
    /// Block texel view compatible
    pub const BLOCK_TEXEL_VIEW_COMPATIBLE: Self = Self(0x00000080);
    /// Extended usage
    pub const EXTENDED_USAGE: Self = Self(0x00000100);
    /// Protected
    pub const PROTECTED: Self = Self(0x00000800);
    /// Disjoint
    pub const DISJOINT: Self = Self(0x00000200);

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

impl core::ops::BitOr for TextureCreateFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

// ============================================================================
// Other Types
// ============================================================================

/// Sample count
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SampleCount {
    /// 1 sample
    #[default]
    S1 = 1,
    /// 2 samples
    S2 = 2,
    /// 4 samples
    S4 = 4,
    /// 8 samples
    S8 = 8,
    /// 16 samples
    S16 = 16,
    /// 32 samples
    S32 = 32,
    /// 64 samples
    S64 = 64,
}

impl SampleCount {
    /// To u32
    #[inline]
    pub const fn as_u32(&self) -> u32 {
        *self as u32
    }

    /// Is multisampled
    #[inline]
    pub const fn is_multisampled(&self) -> bool {
        !matches!(self, Self::S1)
    }
}

/// Texture tiling
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum TextureTiling {
    /// Optimal tiling
    #[default]
    Optimal = 0,
    /// Linear tiling
    Linear = 1,
}

/// Image layout
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ImageLayout {
    /// Undefined
    #[default]
    Undefined = 0,
    /// General
    General = 1,
    /// Color attachment optimal
    ColorAttachmentOptimal = 2,
    /// Depth stencil attachment optimal
    DepthStencilAttachmentOptimal = 3,
    /// Depth stencil read only optimal
    DepthStencilReadOnlyOptimal = 4,
    /// Shader read only optimal
    ShaderReadOnlyOptimal = 5,
    /// Transfer src optimal
    TransferSrcOptimal = 6,
    /// Transfer dst optimal
    TransferDstOptimal = 7,
    /// Preinitialized
    Preinitialized = 8,
    /// Present src
    PresentSrc = 1000001002,
    /// Depth read only stencil attachment optimal
    DepthReadOnlyStencilAttachmentOptimal = 1000117000,
    /// Depth attachment stencil read only optimal
    DepthAttachmentStencilReadOnlyOptimal = 1000117001,
    /// Depth attachment optimal
    DepthAttachmentOptimal = 1000241000,
    /// Depth read only optimal
    DepthReadOnlyOptimal = 1000241001,
    /// Stencil attachment optimal
    StencilAttachmentOptimal = 1000241002,
    /// Stencil read only optimal
    StencilReadOnlyOptimal = 1000241003,
    /// Read only optimal
    ReadOnlyOptimal = 1000314000,
    /// Attachment optimal
    AttachmentOptimal = 1000314001,
    /// Fragment shading rate attachment optimal
    FragmentShadingRateAttachmentOptimal = 1000164003,
}

/// Sharing mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SharingMode {
    /// Exclusive
    #[default]
    Exclusive = 0,
    /// Concurrent
    Concurrent = 1,
}

/// 3D extent
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(C)]
pub struct Extent3D {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Depth
    pub depth: u32,
}

impl Extent3D {
    /// Creates new extent
    #[inline]
    pub const fn new(width: u32, height: u32, depth: u32) -> Self {
        Self { width, height, depth }
    }

    /// Creates 2D extent
    #[inline]
    pub const fn d2(width: u32, height: u32) -> Self {
        Self::new(width, height, 1)
    }

    /// Total pixels
    #[inline]
    pub const fn volume(&self) -> u64 {
        self.width as u64 * self.height as u64 * self.depth as u64
    }

    /// Mip levels for this extent
    #[inline]
    pub const fn mip_levels(&self) -> u32 {
        let max_dim = if self.width > self.height {
            if self.width > self.depth {
                self.width
            } else {
                self.depth
            }
        } else if self.height > self.depth {
            self.height
        } else {
            self.depth
        };
        32 - max_dim.leading_zeros()
    }

    /// Mip extent at level
    pub const fn mip_extent(&self, level: u32) -> Self {
        let w = (self.width >> level).max(1);
        let h = (self.height >> level).max(1);
        let d = (self.depth >> level).max(1);
        Self::new(w, h, d)
    }
}

// ============================================================================
// Texture View Create Info
// ============================================================================

/// Texture view create info
#[derive(Clone, Debug)]
pub struct TextureViewCreateInfo {
    /// Texture handle
    pub texture: TextureHandle,
    /// View type
    pub view_type: TextureViewType,
    /// Format
    pub format: TextureFormat,
    /// Component mapping
    pub components: ComponentMapping,
    /// Subresource range
    pub subresource_range: ImageSubresourceRange,
    /// Create flags
    pub flags: TextureViewCreateFlags,
    /// Debug name
    pub debug_name: Option<String>,
}

impl TextureViewCreateInfo {
    /// Creates view for entire texture
    pub fn new(texture: TextureHandle, format: TextureFormat) -> Self {
        Self {
            texture,
            view_type: TextureViewType::D2,
            format,
            components: ComponentMapping::IDENTITY,
            subresource_range: ImageSubresourceRange::COLOR_ALL,
            flags: TextureViewCreateFlags::NONE,
            debug_name: None,
        }
    }

    /// Creates 1D view
    pub fn d1(texture: TextureHandle, format: TextureFormat) -> Self {
        Self::new(texture, format).with_view_type(TextureViewType::D1)
    }

    /// Creates 2D view
    pub fn d2(texture: TextureHandle, format: TextureFormat) -> Self {
        Self::new(texture, format).with_view_type(TextureViewType::D2)
    }

    /// Creates 3D view
    pub fn d3(texture: TextureHandle, format: TextureFormat) -> Self {
        Self::new(texture, format).with_view_type(TextureViewType::D3)
    }

    /// Creates cube view
    pub fn cube(texture: TextureHandle, format: TextureFormat) -> Self {
        Self::new(texture, format)
            .with_view_type(TextureViewType::Cube)
            .with_subresource_range(ImageSubresourceRange {
                aspect_mask: ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: !0,
                base_array_layer: 0,
                layer_count: 6,
            })
    }

    /// Creates 1D array view
    pub fn d1_array(texture: TextureHandle, format: TextureFormat, layers: u32) -> Self {
        Self::new(texture, format)
            .with_view_type(TextureViewType::D1Array)
            .with_layer_count(layers)
    }

    /// Creates 2D array view
    pub fn d2_array(texture: TextureHandle, format: TextureFormat, layers: u32) -> Self {
        Self::new(texture, format)
            .with_view_type(TextureViewType::D2Array)
            .with_layer_count(layers)
    }

    /// Creates cube array view
    pub fn cube_array(texture: TextureHandle, format: TextureFormat, cubes: u32) -> Self {
        Self::new(texture, format)
            .with_view_type(TextureViewType::CubeArray)
            .with_layer_count(cubes * 6)
    }

    /// With view type
    pub fn with_view_type(mut self, view_type: TextureViewType) -> Self {
        self.view_type = view_type;
        self
    }

    /// With component mapping
    pub fn with_components(mut self, components: ComponentMapping) -> Self {
        self.components = components;
        self
    }

    /// With subresource range
    pub fn with_subresource_range(mut self, range: ImageSubresourceRange) -> Self {
        self.subresource_range = range;
        self
    }

    /// With mip range
    pub fn with_mip_range(mut self, base: u32, count: u32) -> Self {
        self.subresource_range.base_mip_level = base;
        self.subresource_range.level_count = count;
        self
    }

    /// With layer range
    pub fn with_layer_range(mut self, base: u32, count: u32) -> Self {
        self.subresource_range.base_array_layer = base;
        self.subresource_range.layer_count = count;
        self
    }

    /// With layer count
    pub fn with_layer_count(mut self, count: u32) -> Self {
        self.subresource_range.layer_count = count;
        self
    }

    /// With debug name
    pub fn with_name(mut self, name: &str) -> Self {
        self.debug_name = Some(String::from(name));
        self
    }
}

impl Default for TextureViewCreateInfo {
    fn default() -> Self {
        Self {
            texture: TextureHandle::NULL,
            view_type: TextureViewType::D2,
            format: TextureFormat::Rgba8Unorm,
            components: ComponentMapping::IDENTITY,
            subresource_range: ImageSubresourceRange::COLOR_ALL,
            flags: TextureViewCreateFlags::NONE,
            debug_name: None,
        }
    }
}

/// Texture view type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum TextureViewType {
    /// 1D
    D1 = 0,
    /// 2D
    #[default]
    D2 = 1,
    /// 3D
    D3 = 2,
    /// Cube
    Cube = 3,
    /// 1D array
    D1Array = 4,
    /// 2D array
    D2Array = 5,
    /// Cube array
    CubeArray = 6,
}

/// Texture view create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct TextureViewCreateFlags(pub u32);

impl TextureViewCreateFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Fragment density map dynamic
    pub const FRAGMENT_DENSITY_MAP_DYNAMIC: Self = Self(0x00000001);
    /// Fragment density map deferred
    pub const FRAGMENT_DENSITY_MAP_DEFERRED: Self = Self(0x00000002);
}

// ============================================================================
// Component Mapping
// ============================================================================

/// Component mapping
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct ComponentMapping {
    /// R component
    pub r: ComponentSwizzle,
    /// G component
    pub g: ComponentSwizzle,
    /// B component
    pub b: ComponentSwizzle,
    /// A component
    pub a: ComponentSwizzle,
}

impl ComponentMapping {
    /// Identity mapping
    pub const IDENTITY: Self = Self {
        r: ComponentSwizzle::Identity,
        g: ComponentSwizzle::Identity,
        b: ComponentSwizzle::Identity,
        a: ComponentSwizzle::Identity,
    };

    /// RGBA mapping
    pub const RGBA: Self = Self {
        r: ComponentSwizzle::R,
        g: ComponentSwizzle::G,
        b: ComponentSwizzle::B,
        a: ComponentSwizzle::A,
    };

    /// Creates new mapping
    #[inline]
    pub const fn new(
        r: ComponentSwizzle,
        g: ComponentSwizzle,
        b: ComponentSwizzle,
        a: ComponentSwizzle,
    ) -> Self {
        Self { r, g, b, a }
    }
}

impl Default for ComponentMapping {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// Component swizzle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ComponentSwizzle {
    /// Identity
    #[default]
    Identity = 0,
    /// Zero
    Zero = 1,
    /// One
    One = 2,
    /// R
    R = 3,
    /// G
    G = 4,
    /// B
    B = 5,
    /// A
    A = 6,
}

// ============================================================================
// Image Subresource Range
// ============================================================================

/// Image subresource range
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct ImageSubresourceRange {
    /// Aspect mask
    pub aspect_mask: ImageAspectFlags,
    /// Base mip level
    pub base_mip_level: u32,
    /// Level count
    pub level_count: u32,
    /// Base array layer
    pub base_array_layer: u32,
    /// Layer count
    pub layer_count: u32,
}

impl ImageSubresourceRange {
    /// All color mips and layers
    pub const COLOR_ALL: Self = Self {
        aspect_mask: ImageAspectFlags::COLOR,
        base_mip_level: 0,
        level_count: !0,
        base_array_layer: 0,
        layer_count: !0,
    };

    /// All depth mips and layers
    pub const DEPTH_ALL: Self = Self {
        aspect_mask: ImageAspectFlags::DEPTH,
        base_mip_level: 0,
        level_count: !0,
        base_array_layer: 0,
        layer_count: !0,
    };

    /// All depth-stencil mips and layers
    pub const DEPTH_STENCIL_ALL: Self = Self {
        aspect_mask: ImageAspectFlags::DEPTH_STENCIL,
        base_mip_level: 0,
        level_count: !0,
        base_array_layer: 0,
        layer_count: !0,
    };

    /// Creates new range
    #[inline]
    pub const fn new(aspect: ImageAspectFlags) -> Self {
        Self {
            aspect_mask: aspect,
            base_mip_level: 0,
            level_count: !0,
            base_array_layer: 0,
            layer_count: !0,
        }
    }

    /// Single mip level
    #[inline]
    pub const fn single_mip(self, level: u32) -> Self {
        Self {
            base_mip_level: level,
            level_count: 1,
            ..self
        }
    }

    /// Single layer
    #[inline]
    pub const fn single_layer(self, layer: u32) -> Self {
        Self {
            base_array_layer: layer,
            layer_count: 1,
            ..self
        }
    }

    /// Mip range
    #[inline]
    pub const fn mip_range(self, base: u32, count: u32) -> Self {
        Self {
            base_mip_level: base,
            level_count: count,
            ..self
        }
    }

    /// Layer range
    #[inline]
    pub const fn layer_range(self, base: u32, count: u32) -> Self {
        Self {
            base_array_layer: base,
            layer_count: count,
            ..self
        }
    }
}

impl Default for ImageSubresourceRange {
    fn default() -> Self {
        Self::COLOR_ALL
    }
}

/// Image aspect flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ImageAspectFlags(pub u32);

impl ImageAspectFlags {
    /// Color
    pub const COLOR: Self = Self(0x00000001);
    /// Depth
    pub const DEPTH: Self = Self(0x00000002);
    /// Stencil
    pub const STENCIL: Self = Self(0x00000004);
    /// Metadata
    pub const METADATA: Self = Self(0x00000008);
    /// Plane 0
    pub const PLANE_0: Self = Self(0x00000010);
    /// Plane 1
    pub const PLANE_1: Self = Self(0x00000020);
    /// Plane 2
    pub const PLANE_2: Self = Self(0x00000040);
    /// Memory plane 0
    pub const MEMORY_PLANE_0: Self = Self(0x00000080);
    /// Memory plane 1
    pub const MEMORY_PLANE_1: Self = Self(0x00000100);
    /// Memory plane 2
    pub const MEMORY_PLANE_2: Self = Self(0x00000200);
    /// Memory plane 3
    pub const MEMORY_PLANE_3: Self = Self(0x00000400);
    /// Depth and stencil
    pub const DEPTH_STENCIL: Self = Self(0x00000006);

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

impl core::ops::BitOr for ImageAspectFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

// ============================================================================
// Texture Data
// ============================================================================

/// Texture data for uploads
#[derive(Clone, Debug)]
pub struct TextureData<'a> {
    /// Pixel data
    pub data: &'a [u8],
    /// Row pitch in bytes
    pub row_pitch: u32,
    /// Slice pitch in bytes (for 3D textures)
    pub slice_pitch: u32,
    /// Mip level
    pub mip_level: u32,
    /// Array layer
    pub array_layer: u32,
}

impl<'a> TextureData<'a> {
    /// Creates new texture data
    pub fn new(data: &'a [u8], row_pitch: u32) -> Self {
        Self {
            data,
            row_pitch,
            slice_pitch: 0,
            mip_level: 0,
            array_layer: 0,
        }
    }

    /// For mip level
    pub fn at_mip(mut self, level: u32) -> Self {
        self.mip_level = level;
        self
    }

    /// For array layer
    pub fn at_layer(mut self, layer: u32) -> Self {
        self.array_layer = layer;
        self
    }

    /// With slice pitch
    pub fn with_slice_pitch(mut self, pitch: u32) -> Self {
        self.slice_pitch = pitch;
        self
    }
}

// ============================================================================
// Texture Copy Info
// ============================================================================

/// Buffer to image copy info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct BufferImageCopy {
    /// Buffer offset
    pub buffer_offset: u64,
    /// Buffer row length (0 for tightly packed)
    pub buffer_row_length: u32,
    /// Buffer image height (0 for tightly packed)
    pub buffer_image_height: u32,
    /// Image subresource
    pub image_subresource: ImageSubresourceLayers,
    /// Image offset
    pub image_offset: Offset3D,
    /// Image extent
    pub image_extent: Extent3D,
}

/// Image subresource layers
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ImageSubresourceLayers {
    /// Aspect mask
    pub aspect_mask: ImageAspectFlags,
    /// Mip level
    pub mip_level: u32,
    /// Base array layer
    pub base_array_layer: u32,
    /// Layer count
    pub layer_count: u32,
}

impl ImageSubresourceLayers {
    /// Color layer
    pub const fn color(mip: u32, layer: u32) -> Self {
        Self {
            aspect_mask: ImageAspectFlags::COLOR,
            mip_level: mip,
            base_array_layer: layer,
            layer_count: 1,
        }
    }

    /// Depth layer
    pub const fn depth(mip: u32, layer: u32) -> Self {
        Self {
            aspect_mask: ImageAspectFlags::DEPTH,
            mip_level: mip,
            base_array_layer: layer,
            layer_count: 1,
        }
    }
}

/// 3D offset
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Offset3D {
    /// X
    pub x: i32,
    /// Y
    pub y: i32,
    /// Z
    pub z: i32,
}

impl Offset3D {
    /// Zero offset
    pub const ZERO: Self = Self { x: 0, y: 0, z: 0 };

    /// Creates new offset
    #[inline]
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

/// Image copy info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ImageCopy {
    /// Source subresource
    pub src_subresource: ImageSubresourceLayers,
    /// Source offset
    pub src_offset: Offset3D,
    /// Destination subresource
    pub dst_subresource: ImageSubresourceLayers,
    /// Destination offset
    pub dst_offset: Offset3D,
    /// Extent
    pub extent: Extent3D,
}

/// Image blit info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ImageBlit {
    /// Source subresource
    pub src_subresource: ImageSubresourceLayers,
    /// Source offsets (two corners)
    pub src_offsets: [Offset3D; 2],
    /// Destination subresource
    pub dst_subresource: ImageSubresourceLayers,
    /// Destination offsets (two corners)
    pub dst_offsets: [Offset3D; 2],
}

/// Image resolve info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ImageResolve {
    /// Source subresource
    pub src_subresource: ImageSubresourceLayers,
    /// Source offset
    pub src_offset: Offset3D,
    /// Destination subresource
    pub dst_subresource: ImageSubresourceLayers,
    /// Destination offset
    pub dst_offset: Offset3D,
    /// Extent
    pub extent: Extent3D,
}
