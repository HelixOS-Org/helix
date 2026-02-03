//! Format Properties and Capabilities for Lumina
//!
//! This module provides comprehensive format properties, capabilities,
//! and compatibility information for GPU image and buffer formats.

// ============================================================================
// Format Properties
// ============================================================================

/// Format properties
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct FormatProperties {
    /// Linear tiling features
    pub linear_tiling_features: FormatFeatureFlags,
    /// Optimal tiling features
    pub optimal_tiling_features: FormatFeatureFlags,
    /// Buffer features
    pub buffer_features: FormatFeatureFlags,
}

impl FormatProperties {
    /// Creates new format properties
    #[inline]
    pub const fn new(
        linear: FormatFeatureFlags,
        optimal: FormatFeatureFlags,
        buffer: FormatFeatureFlags,
    ) -> Self {
        Self {
            linear_tiling_features: linear,
            optimal_tiling_features: optimal,
            buffer_features: buffer,
        }
    }

    /// Supports optimal tiling
    #[inline]
    pub const fn supports_optimal_tiling(&self) -> bool {
        self.optimal_tiling_features.0 != 0
    }

    /// Supports linear tiling
    #[inline]
    pub const fn supports_linear_tiling(&self) -> bool {
        self.linear_tiling_features.0 != 0
    }

    /// Supports as sampled image
    #[inline]
    pub const fn supports_sampled_image(&self) -> bool {
        self.optimal_tiling_features
            .contains(FormatFeatureFlags::SAMPLED_IMAGE)
    }

    /// Supports as storage image
    #[inline]
    pub const fn supports_storage_image(&self) -> bool {
        self.optimal_tiling_features
            .contains(FormatFeatureFlags::STORAGE_IMAGE)
    }

    /// Supports as color attachment
    #[inline]
    pub const fn supports_color_attachment(&self) -> bool {
        self.optimal_tiling_features
            .contains(FormatFeatureFlags::COLOR_ATTACHMENT)
    }

    /// Supports as depth-stencil attachment
    #[inline]
    pub const fn supports_depth_stencil(&self) -> bool {
        self.optimal_tiling_features
            .contains(FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT)
    }

    /// Supports blending
    #[inline]
    pub const fn supports_blend(&self) -> bool {
        self.optimal_tiling_features
            .contains(FormatFeatureFlags::COLOR_ATTACHMENT_BLEND)
    }

    /// Supports as uniform texel buffer
    #[inline]
    pub const fn supports_uniform_texel_buffer(&self) -> bool {
        self.buffer_features
            .contains(FormatFeatureFlags::UNIFORM_TEXEL_BUFFER)
    }

    /// Supports as storage texel buffer
    #[inline]
    pub const fn supports_storage_texel_buffer(&self) -> bool {
        self.buffer_features
            .contains(FormatFeatureFlags::STORAGE_TEXEL_BUFFER)
    }

    /// Supports as vertex buffer
    #[inline]
    pub const fn supports_vertex_buffer(&self) -> bool {
        self.buffer_features
            .contains(FormatFeatureFlags::VERTEX_BUFFER)
    }
}

/// Format feature flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct FormatFeatureFlags(pub u64);

impl FormatFeatureFlags {
    /// No features
    pub const NONE: Self = Self(0);
    /// Can be sampled
    pub const SAMPLED_IMAGE: Self = Self(1 << 0);
    /// Can be used as storage image
    pub const STORAGE_IMAGE: Self = Self(1 << 1);
    /// Supports atomic operations on storage image
    pub const STORAGE_IMAGE_ATOMIC: Self = Self(1 << 2);
    /// Can be used as uniform texel buffer
    pub const UNIFORM_TEXEL_BUFFER: Self = Self(1 << 3);
    /// Can be used as storage texel buffer
    pub const STORAGE_TEXEL_BUFFER: Self = Self(1 << 4);
    /// Supports atomic operations on storage texel buffer
    pub const STORAGE_TEXEL_BUFFER_ATOMIC: Self = Self(1 << 5);
    /// Can be used as vertex buffer
    pub const VERTEX_BUFFER: Self = Self(1 << 6);
    /// Can be used as color attachment
    pub const COLOR_ATTACHMENT: Self = Self(1 << 7);
    /// Supports blending on color attachment
    pub const COLOR_ATTACHMENT_BLEND: Self = Self(1 << 8);
    /// Can be used as depth-stencil attachment
    pub const DEPTH_STENCIL_ATTACHMENT: Self = Self(1 << 9);
    /// Can be used as blit source
    pub const BLIT_SRC: Self = Self(1 << 10);
    /// Can be used as blit destination
    pub const BLIT_DST: Self = Self(1 << 11);
    /// Supports linear filtering
    pub const SAMPLED_IMAGE_FILTER_LINEAR: Self = Self(1 << 12);
    /// Can be used as transfer source
    pub const TRANSFER_SRC: Self = Self(1 << 14);
    /// Can be used as transfer destination
    pub const TRANSFER_DST: Self = Self(1 << 15);
    /// Supports minmax filter
    pub const SAMPLED_IMAGE_FILTER_MINMAX: Self = Self(1 << 16);
    /// Supports cubic filtering
    pub const SAMPLED_IMAGE_FILTER_CUBIC: Self = Self(1 << 17);
    /// Supports multisampling
    pub const MIDPOINT_CHROMA_SAMPLES: Self = Self(1 << 18);
    /// Sampled image ycbcr conversion linear filter
    pub const SAMPLED_IMAGE_YCBCR_CONVERSION_LINEAR_FILTER: Self = Self(1 << 19);
    /// Sampled image ycbcr conversion separate reconstruction filter
    pub const SAMPLED_IMAGE_YCBCR_CONVERSION_SEPARATE_RECONSTRUCTION_FILTER: Self = Self(1 << 20);
    /// Sampled image ycbcr conversion chroma reconstruction explicit
    pub const SAMPLED_IMAGE_YCBCR_CONVERSION_CHROMA_RECONSTRUCTION_EXPLICIT: Self = Self(1 << 21);
    /// Sampled image ycbcr conversion chroma reconstruction explicit forceable
    pub const SAMPLED_IMAGE_YCBCR_CONVERSION_CHROMA_RECONSTRUCTION_EXPLICIT_FORCEABLE: Self =
        Self(1 << 22);
    /// Disjoint format
    pub const DISJOINT: Self = Self(1 << 23);
    /// Cosited chroma samples
    pub const COSITED_CHROMA_SAMPLES: Self = Self(1 << 24);
    /// Fragment density map
    pub const FRAGMENT_DENSITY_MAP: Self = Self(1 << 25);
    /// Fragment shading rate attachment
    pub const FRAGMENT_SHADING_RATE_ATTACHMENT: Self = Self(1 << 26);
    /// Video decode output
    pub const VIDEO_DECODE_OUTPUT: Self = Self(1 << 27);
    /// Video decode DPB
    pub const VIDEO_DECODE_DPB: Self = Self(1 << 28);
    /// Video encode input
    pub const VIDEO_ENCODE_INPUT: Self = Self(1 << 29);
    /// Video encode DPB
    pub const VIDEO_ENCODE_DPB: Self = Self(1 << 30);
    /// Acceleration structure vertex buffer
    pub const ACCELERATION_STRUCTURE_VERTEX_BUFFER: Self = Self(1 << 31);
    /// Optical flow image
    pub const OPTICAL_FLOW_IMAGE: Self = Self(1 << 32);
    /// Optical flow vector
    pub const OPTICAL_FLOW_VECTOR: Self = Self(1 << 33);
    /// Optical flow cost
    pub const OPTICAL_FLOW_COST: Self = Self(1 << 34);

    /// All texture features
    pub const ALL_TEXTURE: Self = Self(
        Self::SAMPLED_IMAGE.0
            | Self::SAMPLED_IMAGE_FILTER_LINEAR.0
            | Self::TRANSFER_SRC.0
            | Self::TRANSFER_DST.0
            | Self::BLIT_SRC.0
            | Self::BLIT_DST.0,
    );

    /// All color attachment features
    pub const ALL_COLOR_ATTACHMENT: Self =
        Self(Self::COLOR_ATTACHMENT.0 | Self::COLOR_ATTACHMENT_BLEND.0 | Self::TRANSFER_DST.0);

    /// All depth-stencil features
    pub const ALL_DEPTH_STENCIL: Self =
        Self(Self::DEPTH_STENCIL_ATTACHMENT.0 | Self::TRANSFER_DST.0);

    /// Checks if flag is set
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Combines flags
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

// ============================================================================
// Format Information
// ============================================================================

/// Format information
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FormatInfo {
    /// Format type
    pub format_type: FormatType,
    /// Number of components
    pub component_count: u8,
    /// Bits per pixel (total)
    pub bits_per_pixel: u16,
    /// Block width (for compressed formats)
    pub block_width: u8,
    /// Block height (for compressed formats)
    pub block_height: u8,
    /// Block size in bytes (for compressed formats)
    pub block_size: u8,
    /// Aspect flags
    pub aspects: FormatAspectFlags,
    /// Component swizzle/format
    pub component_format: ComponentFormat,
}

impl FormatInfo {
    /// Creates new format info
    #[inline]
    pub const fn new(format_type: FormatType, component_count: u8, bits_per_pixel: u16) -> Self {
        Self {
            format_type,
            component_count,
            bits_per_pixel,
            block_width: 1,
            block_height: 1,
            block_size: (bits_per_pixel / 8) as u8,
            aspects: FormatAspectFlags::COLOR,
            component_format: ComponentFormat::UNorm,
        }
    }

    /// Creates compressed format info
    #[inline]
    pub const fn compressed(
        format_type: FormatType,
        block_width: u8,
        block_height: u8,
        block_size: u8,
    ) -> Self {
        Self {
            format_type,
            component_count: 4,
            bits_per_pixel: ((block_size as u16) * 8)
                / ((block_width as u16) * (block_height as u16)),
            block_width,
            block_height,
            block_size,
            aspects: FormatAspectFlags::COLOR,
            component_format: ComponentFormat::UNorm,
        }
    }

    /// Creates depth format info
    #[inline]
    pub const fn depth(bits: u16) -> Self {
        Self {
            format_type: FormatType::Depth,
            component_count: 1,
            bits_per_pixel: bits,
            block_width: 1,
            block_height: 1,
            block_size: (bits / 8) as u8,
            aspects: FormatAspectFlags::DEPTH,
            component_format: ComponentFormat::Float,
        }
    }

    /// Creates depth-stencil format info
    #[inline]
    pub const fn depth_stencil(depth_bits: u16, stencil_bits: u16) -> Self {
        Self {
            format_type: FormatType::DepthStencil,
            component_count: 2,
            bits_per_pixel: depth_bits + stencil_bits,
            block_width: 1,
            block_height: 1,
            block_size: ((depth_bits + stencil_bits) / 8) as u8,
            aspects: FormatAspectFlags::DEPTH_STENCIL,
            component_format: ComponentFormat::Float,
        }
    }

    /// Is compressed format
    #[inline]
    pub const fn is_compressed(&self) -> bool {
        self.block_width > 1 || self.block_height > 1
    }

    /// Is depth format
    #[inline]
    pub const fn has_depth(&self) -> bool {
        self.aspects.contains(FormatAspectFlags::DEPTH)
    }

    /// Is stencil format
    #[inline]
    pub const fn has_stencil(&self) -> bool {
        self.aspects.contains(FormatAspectFlags::STENCIL)
    }

    /// Bytes per pixel (for uncompressed formats)
    #[inline]
    pub const fn bytes_per_pixel(&self) -> u32 {
        (self.bits_per_pixel as u32 + 7) / 8
    }

    /// Calculate row pitch for width
    #[inline]
    pub const fn row_pitch(&self, width: u32) -> u32 {
        if self.is_compressed() {
            let blocks = (width + self.block_width as u32 - 1) / self.block_width as u32;
            blocks * self.block_size as u32
        } else {
            width * self.bytes_per_pixel()
        }
    }

    /// Calculate slice pitch for width and height
    #[inline]
    pub const fn slice_pitch(&self, width: u32, height: u32) -> u32 {
        if self.is_compressed() {
            let blocks_x = (width + self.block_width as u32 - 1) / self.block_width as u32;
            let blocks_y = (height + self.block_height as u32 - 1) / self.block_height as u32;
            blocks_x * blocks_y * self.block_size as u32
        } else {
            self.row_pitch(width) * height
        }
    }
}

/// Format type classification
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum FormatType {
    /// Undefined
    #[default]
    Undefined       = 0,
    /// Color format
    Color           = 1,
    /// Depth format
    Depth           = 2,
    /// Stencil format
    Stencil         = 3,
    /// Combined depth-stencil format
    DepthStencil    = 4,
    /// BC compressed format
    CompressedBC    = 5,
    /// ETC2 compressed format
    CompressedETC2  = 6,
    /// ASTC compressed format
    CompressedASTC  = 7,
    /// YCbCr format
    YCbCr           = 8,
    /// PVRTC compressed format
    CompressedPVRTC = 9,
}

impl FormatType {
    /// Name of format type
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Undefined => "Undefined",
            Self::Color => "Color",
            Self::Depth => "Depth",
            Self::Stencil => "Stencil",
            Self::DepthStencil => "Depth-Stencil",
            Self::CompressedBC => "BC Compressed",
            Self::CompressedETC2 => "ETC2 Compressed",
            Self::CompressedASTC => "ASTC Compressed",
            Self::YCbCr => "YCbCr",
            Self::CompressedPVRTC => "PVRTC Compressed",
        }
    }

    /// Is compressed type
    #[inline]
    pub const fn is_compressed(&self) -> bool {
        matches!(
            self,
            Self::CompressedBC
                | Self::CompressedETC2
                | Self::CompressedASTC
                | Self::CompressedPVRTC
        )
    }
}

/// Format aspect flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct FormatAspectFlags(pub u8);

impl FormatAspectFlags {
    /// No aspect
    pub const NONE: Self = Self(0);
    /// Color aspect
    pub const COLOR: Self = Self(1 << 0);
    /// Depth aspect
    pub const DEPTH: Self = Self(1 << 1);
    /// Stencil aspect
    pub const STENCIL: Self = Self(1 << 2);
    /// Metadata aspect
    pub const METADATA: Self = Self(1 << 3);
    /// Plane 0
    pub const PLANE_0: Self = Self(1 << 4);
    /// Plane 1
    pub const PLANE_1: Self = Self(1 << 5);
    /// Plane 2
    pub const PLANE_2: Self = Self(1 << 6);
    /// Memory plane 0
    pub const MEMORY_PLANE_0: Self = Self(1 << 7);

    /// Combined depth-stencil
    pub const DEPTH_STENCIL: Self = Self(Self::DEPTH.0 | Self::STENCIL.0);

    /// Checks if flag is set
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

/// Component format type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum ComponentFormat {
    /// Unsigned normalized [0, 1]
    #[default]
    UNorm   = 0,
    /// Signed normalized [-1, 1]
    SNorm   = 1,
    /// Unsigned scaled
    UScaled = 2,
    /// Signed scaled
    SScaled = 3,
    /// Unsigned integer
    UInt    = 4,
    /// Signed integer
    SInt    = 5,
    /// Unsigned float (special 10/11-bit formats)
    UFloat  = 6,
    /// Signed float
    Float   = 7,
    /// sRGB color space
    SRGB    = 8,
}

impl ComponentFormat {
    /// Name
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::UNorm => "UNorm",
            Self::SNorm => "SNorm",
            Self::UScaled => "UScaled",
            Self::SScaled => "SScaled",
            Self::UInt => "UInt",
            Self::SInt => "SInt",
            Self::UFloat => "UFloat",
            Self::Float => "Float",
            Self::SRGB => "sRGB",
        }
    }

    /// Is integer format
    #[inline]
    pub const fn is_integer(&self) -> bool {
        matches!(self, Self::UInt | Self::SInt)
    }

    /// Is normalized format
    #[inline]
    pub const fn is_normalized(&self) -> bool {
        matches!(self, Self::UNorm | Self::SNorm | Self::SRGB)
    }

    /// Is floating point format
    #[inline]
    pub const fn is_float(&self) -> bool {
        matches!(self, Self::Float | Self::UFloat)
    }
}

// ============================================================================
// Format Compatibility
// ============================================================================

/// Format compatibility class
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FormatCompatibilityClass {
    /// 8-bit formats
    Bit8        = 0,
    /// 16-bit formats
    Bit16       = 1,
    /// 24-bit formats
    Bit24       = 2,
    /// 32-bit formats
    Bit32       = 3,
    /// 48-bit formats
    Bit48       = 4,
    /// 64-bit formats
    Bit64       = 5,
    /// 96-bit formats
    Bit96       = 6,
    /// 128-bit formats
    Bit128      = 7,
    /// 192-bit formats
    Bit192      = 8,
    /// 256-bit formats
    Bit256      = 9,
    /// BC1 RGB
    BC1RGB      = 10,
    /// BC1 RGBA
    BC1RGBA     = 11,
    /// BC2
    BC2         = 12,
    /// BC3
    BC3         = 13,
    /// BC4
    BC4         = 14,
    /// BC5
    BC5         = 15,
    /// BC6H
    BC6H        = 16,
    /// BC7
    BC7         = 17,
    /// ETC2 RGB
    ETC2RGB     = 18,
    /// ETC2 RGBA
    ETC2RGBA    = 19,
    /// ETC2 EAC RGBA
    ETC2EACRGBA = 20,
    /// EAC R
    EACR        = 21,
    /// EAC RG
    EACRG       = 22,
    /// ASTC 4x4
    ASTC4x4     = 23,
    /// ASTC 5x4
    ASTC5x4     = 24,
    /// ASTC 5x5
    ASTC5x5     = 25,
    /// ASTC 6x5
    ASTC6x5     = 26,
    /// ASTC 6x6
    ASTC6x6     = 27,
    /// ASTC 8x5
    ASTC8x5     = 28,
    /// ASTC 8x6
    ASTC8x6     = 29,
    /// ASTC 8x8
    ASTC8x8     = 30,
    /// ASTC 10x5
    ASTC10x5    = 31,
    /// ASTC 10x6
    ASTC10x6    = 32,
    /// ASTC 10x8
    ASTC10x8    = 33,
    /// ASTC 10x10
    ASTC10x10   = 34,
    /// ASTC 12x10
    ASTC12x10   = 35,
    /// ASTC 12x12
    ASTC12x12   = 36,
    /// D16
    D16         = 37,
    /// D24
    D24         = 38,
    /// D32
    D32         = 39,
    /// S8
    S8          = 40,
    /// D16S8
    D16S8       = 41,
    /// D24S8
    D24S8       = 42,
    /// D32S8
    D32S8       = 43,
}

impl FormatCompatibilityClass {
    /// Get bit size for uncompressed classes
    #[inline]
    pub const fn bits(&self) -> Option<u32> {
        match self {
            Self::Bit8 => Some(8),
            Self::Bit16 => Some(16),
            Self::Bit24 => Some(24),
            Self::Bit32 => Some(32),
            Self::Bit48 => Some(48),
            Self::Bit64 => Some(64),
            Self::Bit96 => Some(96),
            Self::Bit128 => Some(128),
            Self::Bit192 => Some(192),
            Self::Bit256 => Some(256),
            Self::D16 | Self::D16S8 => Some(16),
            Self::D24 | Self::D24S8 => Some(32),
            Self::D32 | Self::D32S8 => Some(32),
            Self::S8 => Some(8),
            _ => None,
        }
    }
}

/// Checks if two formats are compatible for copy operations
#[inline]
pub const fn formats_are_size_compatible(
    class_a: FormatCompatibilityClass,
    class_b: FormatCompatibilityClass,
) -> bool {
    matches!(
        (class_a, class_b),
        (
            FormatCompatibilityClass::Bit8,
            FormatCompatibilityClass::Bit8
        ) | (
            FormatCompatibilityClass::Bit16,
            FormatCompatibilityClass::Bit16
        ) | (
            FormatCompatibilityClass::Bit24,
            FormatCompatibilityClass::Bit24
        ) | (
            FormatCompatibilityClass::Bit32,
            FormatCompatibilityClass::Bit32
        ) | (
            FormatCompatibilityClass::Bit48,
            FormatCompatibilityClass::Bit48
        ) | (
            FormatCompatibilityClass::Bit64,
            FormatCompatibilityClass::Bit64
        ) | (
            FormatCompatibilityClass::Bit96,
            FormatCompatibilityClass::Bit96
        ) | (
            FormatCompatibilityClass::Bit128,
            FormatCompatibilityClass::Bit128
        )
    )
}

// ============================================================================
// Surface Format
// ============================================================================

/// Surface format for presentation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct SurfaceFormat {
    /// Color format
    pub format: ColorFormat,
    /// Color space
    pub color_space: ColorSpace,
}

impl SurfaceFormat {
    /// Creates new surface format
    #[inline]
    pub const fn new(format: ColorFormat, color_space: ColorSpace) -> Self {
        Self {
            format,
            color_space,
        }
    }

    /// sRGB 8-bit BGRA
    pub const BGRA8_SRGB: Self = Self::new(ColorFormat::Bgra8Unorm, ColorSpace::SrgbNonlinear);

    /// Linear 8-bit BGRA
    pub const BGRA8_UNORM: Self = Self::new(ColorFormat::Bgra8Unorm, ColorSpace::PassThrough);

    /// HDR10 surface
    pub const HDR10: Self = Self::new(ColorFormat::Rgb10a2Unorm, ColorSpace::Hdr10St2084);

    /// Is HDR format
    #[inline]
    pub const fn is_hdr(&self) -> bool {
        self.color_space.is_hdr()
    }
}

impl Default for SurfaceFormat {
    fn default() -> Self {
        Self::BGRA8_SRGB
    }
}

/// Color format enumeration
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ColorFormat {
    /// Undefined
    #[default]
    Undefined       = 0,
    /// R8 UNorm
    R8Unorm         = 1,
    /// R8 SNorm
    R8Snorm         = 2,
    /// R8 UInt
    R8Uint          = 3,
    /// R8 SInt
    R8Sint          = 4,
    /// R8G8 UNorm
    Rg8Unorm        = 5,
    /// R8G8 SNorm
    Rg8Snorm        = 6,
    /// R8G8 UInt
    Rg8Uint         = 7,
    /// R8G8 SInt
    Rg8Sint         = 8,
    /// R8G8B8 UNorm
    Rgb8Unorm       = 9,
    /// R8G8B8 sRGB
    Rgb8Srgb        = 10,
    /// R8G8B8A8 UNorm
    Rgba8Unorm      = 11,
    /// R8G8B8A8 sRGB
    Rgba8Srgb       = 12,
    /// R8G8B8A8 SNorm
    Rgba8Snorm      = 13,
    /// R8G8B8A8 UInt
    Rgba8Uint       = 14,
    /// R8G8B8A8 SInt
    Rgba8Sint       = 15,
    /// B8G8R8A8 UNorm
    Bgra8Unorm      = 16,
    /// B8G8R8A8 sRGB
    Bgra8Srgb       = 17,
    /// R16 Float
    R16Float        = 18,
    /// R16 UNorm
    R16Unorm        = 19,
    /// R16 UInt
    R16Uint         = 20,
    /// R16 SInt
    R16Sint         = 21,
    /// R16G16 Float
    Rg16Float       = 22,
    /// R16G16 UNorm
    Rg16Unorm       = 23,
    /// R16G16 UInt
    Rg16Uint        = 24,
    /// R16G16 SInt
    Rg16Sint        = 25,
    /// R16G16B16A16 Float
    Rgba16Float     = 26,
    /// R16G16B16A16 UNorm
    Rgba16Unorm     = 27,
    /// R16G16B16A16 UInt
    Rgba16Uint      = 28,
    /// R16G16B16A16 SInt
    Rgba16Sint      = 29,
    /// R32 Float
    R32Float        = 30,
    /// R32 UInt
    R32Uint         = 31,
    /// R32 SInt
    R32Sint         = 32,
    /// R32G32 Float
    Rg32Float       = 33,
    /// R32G32 UInt
    Rg32Uint        = 34,
    /// R32G32 SInt
    Rg32Sint        = 35,
    /// R32G32B32 Float
    Rgb32Float      = 36,
    /// R32G32B32 UInt
    Rgb32Uint       = 37,
    /// R32G32B32 SInt
    Rgb32Sint       = 38,
    /// R32G32B32A32 Float
    Rgba32Float     = 39,
    /// R32G32B32A32 UInt
    Rgba32Uint      = 40,
    /// R32G32B32A32 SInt
    Rgba32Sint      = 41,
    /// R10G10B10A2 UNorm
    Rgb10a2Unorm    = 42,
    /// R10G10B10A2 UInt
    Rgb10a2Uint     = 43,
    /// R11G11B10 Float
    Rg11b10Float    = 44,
    /// R9G9B9E5 Shared Exponent
    Rgb9e5Float     = 45,
    /// D16 UNorm
    D16Unorm        = 46,
    /// D24 UNorm
    D24Unorm        = 47,
    /// D32 Float
    D32Float        = 48,
    /// S8 UInt
    S8Uint          = 49,
    /// D24 UNorm S8 UInt
    D24UnormS8Uint  = 50,
    /// D32 Float S8 UInt
    D32FloatS8Uint  = 51,
    /// BC1 RGB UNorm
    Bc1RgbUnorm     = 52,
    /// BC1 RGB sRGB
    Bc1RgbSrgb      = 53,
    /// BC1 RGBA UNorm
    Bc1RgbaUnorm    = 54,
    /// BC1 RGBA sRGB
    Bc1RgbaSrgb     = 55,
    /// BC2 UNorm
    Bc2Unorm        = 56,
    /// BC2 sRGB
    Bc2Srgb         = 57,
    /// BC3 UNorm
    Bc3Unorm        = 58,
    /// BC3 sRGB
    Bc3Srgb         = 59,
    /// BC4 UNorm
    Bc4Unorm        = 60,
    /// BC4 SNorm
    Bc4Snorm        = 61,
    /// BC5 UNorm
    Bc5Unorm        = 62,
    /// BC5 SNorm
    Bc5Snorm        = 63,
    /// BC6H UFloat
    Bc6hUfloat      = 64,
    /// BC6H SFloat
    Bc6hSfloat      = 65,
    /// BC7 UNorm
    Bc7Unorm        = 66,
    /// BC7 sRGB
    Bc7Srgb         = 67,
    /// ETC2 R8G8B8 UNorm
    Etc2Rgb8Unorm   = 68,
    /// ETC2 R8G8B8 sRGB
    Etc2Rgb8Srgb    = 69,
    /// ETC2 R8G8B8A1 UNorm
    Etc2Rgb8A1Unorm = 70,
    /// ETC2 R8G8B8A1 sRGB
    Etc2Rgb8A1Srgb  = 71,
    /// ETC2 R8G8B8A8 UNorm
    Etc2Rgba8Unorm  = 72,
    /// ETC2 R8G8B8A8 sRGB
    Etc2Rgba8Srgb   = 73,
    /// EAC R11 UNorm
    EacR11Unorm     = 74,
    /// EAC R11 SNorm
    EacR11Snorm     = 75,
    /// EAC R11G11 UNorm
    EacRg11Unorm    = 76,
    /// EAC R11G11 SNorm
    EacRg11Snorm    = 77,
    /// ASTC 4x4 UNorm
    Astc4x4Unorm    = 78,
    /// ASTC 4x4 sRGB
    Astc4x4Srgb     = 79,
    /// ASTC 5x4 UNorm
    Astc5x4Unorm    = 80,
    /// ASTC 5x4 sRGB
    Astc5x4Srgb     = 81,
    /// ASTC 5x5 UNorm
    Astc5x5Unorm    = 82,
    /// ASTC 5x5 sRGB
    Astc5x5Srgb     = 83,
    /// ASTC 6x5 UNorm
    Astc6x5Unorm    = 84,
    /// ASTC 6x5 sRGB
    Astc6x5Srgb     = 85,
    /// ASTC 6x6 UNorm
    Astc6x6Unorm    = 86,
    /// ASTC 6x6 sRGB
    Astc6x6Srgb     = 87,
    /// ASTC 8x5 UNorm
    Astc8x5Unorm    = 88,
    /// ASTC 8x5 sRGB
    Astc8x5Srgb     = 89,
    /// ASTC 8x6 UNorm
    Astc8x6Unorm    = 90,
    /// ASTC 8x6 sRGB
    Astc8x6Srgb     = 91,
    /// ASTC 8x8 UNorm
    Astc8x8Unorm    = 92,
    /// ASTC 8x8 sRGB
    Astc8x8Srgb     = 93,
    /// ASTC 10x5 UNorm
    Astc10x5Unorm   = 94,
    /// ASTC 10x5 sRGB
    Astc10x5Srgb    = 95,
    /// ASTC 10x6 UNorm
    Astc10x6Unorm   = 96,
    /// ASTC 10x6 sRGB
    Astc10x6Srgb    = 97,
    /// ASTC 10x8 UNorm
    Astc10x8Unorm   = 98,
    /// ASTC 10x8 sRGB
    Astc10x8Srgb    = 99,
    /// ASTC 10x10 UNorm
    Astc10x10Unorm  = 100,
    /// ASTC 10x10 sRGB
    Astc10x10Srgb   = 101,
    /// ASTC 12x10 UNorm
    Astc12x10Unorm  = 102,
    /// ASTC 12x10 sRGB
    Astc12x10Srgb   = 103,
    /// ASTC 12x12 UNorm
    Astc12x12Unorm  = 104,
    /// ASTC 12x12 sRGB
    Astc12x12Srgb   = 105,
}

impl ColorFormat {
    /// Bits per pixel
    #[inline]
    pub const fn bits_per_pixel(&self) -> u32 {
        match self {
            Self::Undefined => 0,
            Self::R8Unorm | Self::R8Snorm | Self::R8Uint | Self::R8Sint => 8,
            Self::Rg8Unorm | Self::Rg8Snorm | Self::Rg8Uint | Self::Rg8Sint => 16,
            Self::R16Float | Self::R16Unorm | Self::R16Uint | Self::R16Sint => 16,
            Self::Rgb8Unorm | Self::Rgb8Srgb => 24,
            Self::Rgba8Unorm
            | Self::Rgba8Srgb
            | Self::Rgba8Snorm
            | Self::Rgba8Uint
            | Self::Rgba8Sint
            | Self::Bgra8Unorm
            | Self::Bgra8Srgb => 32,
            Self::Rg16Float | Self::Rg16Unorm | Self::Rg16Uint | Self::Rg16Sint => 32,
            Self::R32Float | Self::R32Uint | Self::R32Sint => 32,
            Self::Rgb10a2Unorm | Self::Rgb10a2Uint | Self::Rg11b10Float | Self::Rgb9e5Float => 32,
            Self::Rgba16Float | Self::Rgba16Unorm | Self::Rgba16Uint | Self::Rgba16Sint => 64,
            Self::Rg32Float | Self::Rg32Uint | Self::Rg32Sint => 64,
            Self::Rgb32Float | Self::Rgb32Uint | Self::Rgb32Sint => 96,
            Self::Rgba32Float | Self::Rgba32Uint | Self::Rgba32Sint => 128,
            Self::D16Unorm => 16,
            Self::D24Unorm | Self::D24UnormS8Uint => 32,
            Self::D32Float | Self::D32FloatS8Uint => 32,
            Self::S8Uint => 8,
            // BC compressed
            Self::Bc1RgbUnorm | Self::Bc1RgbSrgb | Self::Bc1RgbaUnorm | Self::Bc1RgbaSrgb => 4,
            Self::Bc2Unorm | Self::Bc2Srgb | Self::Bc3Unorm | Self::Bc3Srgb => 8,
            Self::Bc4Unorm | Self::Bc4Snorm => 4,
            Self::Bc5Unorm | Self::Bc5Snorm => 8,
            Self::Bc6hUfloat | Self::Bc6hSfloat | Self::Bc7Unorm | Self::Bc7Srgb => 8,
            // ETC2/EAC
            Self::Etc2Rgb8Unorm
            | Self::Etc2Rgb8Srgb
            | Self::Etc2Rgb8A1Unorm
            | Self::Etc2Rgb8A1Srgb => 4,
            Self::Etc2Rgba8Unorm
            | Self::Etc2Rgba8Srgb
            | Self::EacRg11Unorm
            | Self::EacRg11Snorm => 8,
            Self::EacR11Unorm | Self::EacR11Snorm => 4,
            // ASTC
            _ => 16, // ASTC formats are 128-bit per block
        }
    }

    /// Is sRGB format
    #[inline]
    pub const fn is_srgb(&self) -> bool {
        matches!(
            self,
            Self::Rgb8Srgb
                | Self::Rgba8Srgb
                | Self::Bgra8Srgb
                | Self::Bc1RgbSrgb
                | Self::Bc1RgbaSrgb
                | Self::Bc2Srgb
                | Self::Bc3Srgb
                | Self::Bc7Srgb
                | Self::Etc2Rgb8Srgb
                | Self::Etc2Rgb8A1Srgb
                | Self::Etc2Rgba8Srgb
                | Self::Astc4x4Srgb
                | Self::Astc5x4Srgb
                | Self::Astc5x5Srgb
                | Self::Astc6x5Srgb
                | Self::Astc6x6Srgb
                | Self::Astc8x5Srgb
                | Self::Astc8x6Srgb
                | Self::Astc8x8Srgb
                | Self::Astc10x5Srgb
                | Self::Astc10x6Srgb
                | Self::Astc10x8Srgb
                | Self::Astc10x10Srgb
                | Self::Astc12x10Srgb
                | Self::Astc12x12Srgb
        )
    }

    /// Is depth format
    #[inline]
    pub const fn is_depth(&self) -> bool {
        matches!(
            self,
            Self::D16Unorm
                | Self::D24Unorm
                | Self::D32Float
                | Self::D24UnormS8Uint
                | Self::D32FloatS8Uint
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

    /// Is compressed format
    #[inline]
    pub const fn is_compressed(&self) -> bool {
        matches!(
            self,
            Self::Bc1RgbUnorm
                | Self::Bc1RgbSrgb
                | Self::Bc1RgbaUnorm
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
                | Self::Etc2Rgb8Unorm
                | Self::Etc2Rgb8Srgb
                | Self::Etc2Rgb8A1Unorm
                | Self::Etc2Rgb8A1Srgb
                | Self::Etc2Rgba8Unorm
                | Self::Etc2Rgba8Srgb
                | Self::EacR11Unorm
                | Self::EacR11Snorm
                | Self::EacRg11Unorm
                | Self::EacRg11Snorm
                | Self::Astc4x4Unorm
                | Self::Astc4x4Srgb
                | Self::Astc5x4Unorm
                | Self::Astc5x4Srgb
                | Self::Astc5x5Unorm
                | Self::Astc5x5Srgb
                | Self::Astc6x5Unorm
                | Self::Astc6x5Srgb
                | Self::Astc6x6Unorm
                | Self::Astc6x6Srgb
                | Self::Astc8x5Unorm
                | Self::Astc8x5Srgb
                | Self::Astc8x6Unorm
                | Self::Astc8x6Srgb
                | Self::Astc8x8Unorm
                | Self::Astc8x8Srgb
                | Self::Astc10x5Unorm
                | Self::Astc10x5Srgb
                | Self::Astc10x6Unorm
                | Self::Astc10x6Srgb
                | Self::Astc10x8Unorm
                | Self::Astc10x8Srgb
                | Self::Astc10x10Unorm
                | Self::Astc10x10Srgb
                | Self::Astc12x10Unorm
                | Self::Astc12x10Srgb
                | Self::Astc12x12Unorm
                | Self::Astc12x12Srgb
        )
    }
}

/// Color space enumeration
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ColorSpace {
    /// sRGB nonlinear
    #[default]
    SrgbNonlinear      = 0,
    /// Display P3 nonlinear
    DisplayP3Nonlinear = 1,
    /// Extended sRGB linear
    ExtendedSrgbLinear = 2,
    /// Display P3 linear
    DisplayP3Linear    = 3,
    /// DCI-P3 nonlinear
    DciP3Nonlinear     = 4,
    /// BT709 linear
    Bt709Linear        = 5,
    /// BT709 nonlinear
    Bt709Nonlinear     = 6,
    /// BT2020 linear
    Bt2020Linear       = 7,
    /// HDR10 ST2084
    Hdr10St2084        = 8,
    /// Dolby Vision
    DolbyVision        = 9,
    /// HDR10 HLG
    Hdr10Hlg           = 10,
    /// Adobe RGB linear
    AdobeRgbLinear     = 11,
    /// Adobe RGB nonlinear
    AdobeRgbNonlinear  = 12,
    /// Pass through
    PassThrough        = 13,
    /// Extended sRGB nonlinear
    ExtendedSrgbNonlinear = 14,
}

impl ColorSpace {
    /// Name
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::SrgbNonlinear => "sRGB Nonlinear",
            Self::DisplayP3Nonlinear => "Display P3",
            Self::ExtendedSrgbLinear => "Extended sRGB Linear",
            Self::DisplayP3Linear => "Display P3 Linear",
            Self::DciP3Nonlinear => "DCI-P3",
            Self::Bt709Linear => "BT.709 Linear",
            Self::Bt709Nonlinear => "BT.709",
            Self::Bt2020Linear => "BT.2020 Linear",
            Self::Hdr10St2084 => "HDR10 (ST.2084)",
            Self::DolbyVision => "Dolby Vision",
            Self::Hdr10Hlg => "HDR10 (HLG)",
            Self::AdobeRgbLinear => "Adobe RGB Linear",
            Self::AdobeRgbNonlinear => "Adobe RGB",
            Self::PassThrough => "Pass Through",
            Self::ExtendedSrgbNonlinear => "Extended sRGB",
        }
    }

    /// Is HDR color space
    #[inline]
    pub const fn is_hdr(&self) -> bool {
        matches!(
            self,
            Self::Hdr10St2084 | Self::DolbyVision | Self::Hdr10Hlg | Self::Bt2020Linear
        )
    }

    /// Is linear color space
    #[inline]
    pub const fn is_linear(&self) -> bool {
        matches!(
            self,
            Self::ExtendedSrgbLinear
                | Self::DisplayP3Linear
                | Self::Bt709Linear
                | Self::Bt2020Linear
                | Self::AdobeRgbLinear
        )
    }
}
