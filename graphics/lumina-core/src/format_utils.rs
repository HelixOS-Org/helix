//! Format utilities and conversions
//!
//! This module provides comprehensive format information and conversion utilities.

/// Format information
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FormatInfo {
    /// Format
    pub format: Format,
    /// Block width (1 for non-compressed)
    pub block_width: u8,
    /// Block height (1 for non-compressed)
    pub block_height: u8,
    /// Block depth (1 for 2D formats)
    pub block_depth: u8,
    /// Bytes per block
    pub bytes_per_block: u8,
    /// Component count
    pub component_count: u8,
    /// Format class
    pub format_class: FormatClass,
    /// Format aspects
    pub aspects: FormatAspects,
}

impl FormatInfo {
    /// Gets format info for a format
    pub const fn for_format(format: Format) -> Self {
        match format {
            // 8-bit formats
            Format::R8_UNORM | Format::R8_SNORM | Format::R8_UINT | Format::R8_SINT => Self {
                format,
                block_width: 1,
                block_height: 1,
                block_depth: 1,
                bytes_per_block: 1,
                component_count: 1,
                format_class: FormatClass::R8,
                aspects: FormatAspects::COLOR,
            },
            Format::R8G8_UNORM | Format::R8G8_SNORM | Format::R8G8_UINT | Format::R8G8_SINT => {
                Self {
                    format,
                    block_width: 1,
                    block_height: 1,
                    block_depth: 1,
                    bytes_per_block: 2,
                    component_count: 2,
                    format_class: FormatClass::RG8,
                    aspects: FormatAspects::COLOR,
                }
            },
            Format::R8G8B8A8_UNORM
            | Format::R8G8B8A8_SNORM
            | Format::R8G8B8A8_UINT
            | Format::R8G8B8A8_SINT
            | Format::R8G8B8A8_SRGB => Self {
                format,
                block_width: 1,
                block_height: 1,
                block_depth: 1,
                bytes_per_block: 4,
                component_count: 4,
                format_class: FormatClass::RGBA8,
                aspects: FormatAspects::COLOR,
            },
            Format::B8G8R8A8_UNORM | Format::B8G8R8A8_SRGB => Self {
                format,
                block_width: 1,
                block_height: 1,
                block_depth: 1,
                bytes_per_block: 4,
                component_count: 4,
                format_class: FormatClass::RGBA8,
                aspects: FormatAspects::COLOR,
            },

            // 16-bit formats
            Format::R16_UNORM
            | Format::R16_SNORM
            | Format::R16_UINT
            | Format::R16_SINT
            | Format::R16_SFLOAT => Self {
                format,
                block_width: 1,
                block_height: 1,
                block_depth: 1,
                bytes_per_block: 2,
                component_count: 1,
                format_class: FormatClass::R16,
                aspects: FormatAspects::COLOR,
            },
            Format::R16G16_UNORM
            | Format::R16G16_SNORM
            | Format::R16G16_UINT
            | Format::R16G16_SINT
            | Format::R16G16_SFLOAT => Self {
                format,
                block_width: 1,
                block_height: 1,
                block_depth: 1,
                bytes_per_block: 4,
                component_count: 2,
                format_class: FormatClass::RG16,
                aspects: FormatAspects::COLOR,
            },
            Format::R16G16B16A16_UNORM
            | Format::R16G16B16A16_SNORM
            | Format::R16G16B16A16_UINT
            | Format::R16G16B16A16_SINT
            | Format::R16G16B16A16_SFLOAT => Self {
                format,
                block_width: 1,
                block_height: 1,
                block_depth: 1,
                bytes_per_block: 8,
                component_count: 4,
                format_class: FormatClass::RGBA16,
                aspects: FormatAspects::COLOR,
            },

            // 32-bit formats
            Format::R32_UINT | Format::R32_SINT | Format::R32_SFLOAT => Self {
                format,
                block_width: 1,
                block_height: 1,
                block_depth: 1,
                bytes_per_block: 4,
                component_count: 1,
                format_class: FormatClass::R32,
                aspects: FormatAspects::COLOR,
            },
            Format::R32G32_UINT | Format::R32G32_SINT | Format::R32G32_SFLOAT => Self {
                format,
                block_width: 1,
                block_height: 1,
                block_depth: 1,
                bytes_per_block: 8,
                component_count: 2,
                format_class: FormatClass::RG32,
                aspects: FormatAspects::COLOR,
            },
            Format::R32G32B32_UINT | Format::R32G32B32_SINT | Format::R32G32B32_SFLOAT => Self {
                format,
                block_width: 1,
                block_height: 1,
                block_depth: 1,
                bytes_per_block: 12,
                component_count: 3,
                format_class: FormatClass::RGB32,
                aspects: FormatAspects::COLOR,
            },
            Format::R32G32B32A32_UINT | Format::R32G32B32A32_SINT | Format::R32G32B32A32_SFLOAT => {
                Self {
                    format,
                    block_width: 1,
                    block_height: 1,
                    block_depth: 1,
                    bytes_per_block: 16,
                    component_count: 4,
                    format_class: FormatClass::RGBA32,
                    aspects: FormatAspects::COLOR,
                }
            },

            // Depth formats
            Format::D16_UNORM => Self {
                format,
                block_width: 1,
                block_height: 1,
                block_depth: 1,
                bytes_per_block: 2,
                component_count: 1,
                format_class: FormatClass::D16,
                aspects: FormatAspects::DEPTH,
            },
            Format::D32_SFLOAT => Self {
                format,
                block_width: 1,
                block_height: 1,
                block_depth: 1,
                bytes_per_block: 4,
                component_count: 1,
                format_class: FormatClass::D32,
                aspects: FormatAspects::DEPTH,
            },
            Format::D24_UNORM_S8_UINT => Self {
                format,
                block_width: 1,
                block_height: 1,
                block_depth: 1,
                bytes_per_block: 4,
                component_count: 2,
                format_class: FormatClass::D24S8,
                aspects: FormatAspects::DEPTH_STENCIL,
            },
            Format::D32_SFLOAT_S8_UINT => Self {
                format,
                block_width: 1,
                block_height: 1,
                block_depth: 1,
                bytes_per_block: 8,
                component_count: 2,
                format_class: FormatClass::D32S8,
                aspects: FormatAspects::DEPTH_STENCIL,
            },

            // BC compressed formats
            Format::BC1_RGB_UNORM | Format::BC1_RGB_SRGB => Self {
                format,
                block_width: 4,
                block_height: 4,
                block_depth: 1,
                bytes_per_block: 8,
                component_count: 3,
                format_class: FormatClass::BC1,
                aspects: FormatAspects::COLOR,
            },
            Format::BC1_RGBA_UNORM | Format::BC1_RGBA_SRGB => Self {
                format,
                block_width: 4,
                block_height: 4,
                block_depth: 1,
                bytes_per_block: 8,
                component_count: 4,
                format_class: FormatClass::BC1,
                aspects: FormatAspects::COLOR,
            },
            Format::BC2_UNORM | Format::BC2_SRGB => Self {
                format,
                block_width: 4,
                block_height: 4,
                block_depth: 1,
                bytes_per_block: 16,
                component_count: 4,
                format_class: FormatClass::BC2,
                aspects: FormatAspects::COLOR,
            },
            Format::BC3_UNORM | Format::BC3_SRGB => Self {
                format,
                block_width: 4,
                block_height: 4,
                block_depth: 1,
                bytes_per_block: 16,
                component_count: 4,
                format_class: FormatClass::BC3,
                aspects: FormatAspects::COLOR,
            },
            Format::BC4_UNORM | Format::BC4_SNORM => Self {
                format,
                block_width: 4,
                block_height: 4,
                block_depth: 1,
                bytes_per_block: 8,
                component_count: 1,
                format_class: FormatClass::BC4,
                aspects: FormatAspects::COLOR,
            },
            Format::BC5_UNORM | Format::BC5_SNORM => Self {
                format,
                block_width: 4,
                block_height: 4,
                block_depth: 1,
                bytes_per_block: 16,
                component_count: 2,
                format_class: FormatClass::BC5,
                aspects: FormatAspects::COLOR,
            },
            Format::BC6H_UFLOAT | Format::BC6H_SFLOAT => Self {
                format,
                block_width: 4,
                block_height: 4,
                block_depth: 1,
                bytes_per_block: 16,
                component_count: 3,
                format_class: FormatClass::BC6H,
                aspects: FormatAspects::COLOR,
            },
            Format::BC7_UNORM | Format::BC7_SRGB => Self {
                format,
                block_width: 4,
                block_height: 4,
                block_depth: 1,
                bytes_per_block: 16,
                component_count: 4,
                format_class: FormatClass::BC7,
                aspects: FormatAspects::COLOR,
            },

            _ => Self {
                format,
                block_width: 1,
                block_height: 1,
                block_depth: 1,
                bytes_per_block: 4,
                component_count: 4,
                format_class: FormatClass::Unknown,
                aspects: FormatAspects::COLOR,
            },
        }
    }

    /// Is this a compressed format
    pub const fn is_compressed(&self) -> bool {
        self.block_width > 1 || self.block_height > 1
    }

    /// Is this a depth format
    pub const fn is_depth(&self) -> bool {
        self.aspects.contains(FormatAspects::DEPTH)
    }

    /// Is this a stencil format
    pub const fn is_stencil(&self) -> bool {
        self.aspects.contains(FormatAspects::STENCIL)
    }

    /// Is this a depth-stencil format
    pub const fn is_depth_stencil(&self) -> bool {
        self.aspects
            .contains(FormatAspects::DEPTH.union(FormatAspects::STENCIL))
    }

    /// Calculates size for an image
    pub const fn calculate_size(&self, width: u32, height: u32, depth: u32) -> u64 {
        let blocks_x = (width + self.block_width as u32 - 1) / self.block_width as u32;
        let blocks_y = (height + self.block_height as u32 - 1) / self.block_height as u32;
        let blocks_z = (depth + self.block_depth as u32 - 1) / self.block_depth as u32;
        blocks_x as u64 * blocks_y as u64 * blocks_z as u64 * self.bytes_per_block as u64
    }

    /// Calculates row pitch
    pub const fn row_pitch(&self, width: u32) -> u32 {
        let blocks_x = (width + self.block_width as u32 - 1) / self.block_width as u32;
        blocks_x * self.bytes_per_block as u32
    }
}

/// Format enum
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Format {
    /// Undefined format
    #[default]
    Undefined           = 0,

    // 8-bit formats
    /// R8 unsigned normalized
    R8_UNORM            = 9,
    /// R8 signed normalized
    R8_SNORM            = 10,
    /// R8 unsigned integer
    R8_UINT             = 13,
    /// R8 signed integer
    R8_SINT             = 14,

    /// RG8 unsigned normalized
    R8G8_UNORM          = 16,
    /// RG8 signed normalized
    R8G8_SNORM          = 17,
    /// RG8 unsigned integer
    R8G8_UINT           = 20,
    /// RG8 signed integer
    R8G8_SINT           = 21,

    /// RGBA8 unsigned normalized
    R8G8B8A8_UNORM      = 37,
    /// RGBA8 signed normalized
    R8G8B8A8_SNORM      = 38,
    /// RGBA8 unsigned integer
    R8G8B8A8_UINT       = 41,
    /// RGBA8 signed integer
    R8G8B8A8_SINT       = 42,
    /// RGBA8 sRGB
    R8G8B8A8_SRGB       = 43,

    /// BGRA8 unsigned normalized
    B8G8R8A8_UNORM      = 44,
    /// BGRA8 sRGB
    B8G8R8A8_SRGB       = 50,

    // 16-bit formats
    /// R16 unsigned normalized
    R16_UNORM           = 70,
    /// R16 signed normalized
    R16_SNORM           = 71,
    /// R16 unsigned integer
    R16_UINT            = 74,
    /// R16 signed integer
    R16_SINT            = 75,
    /// R16 float
    R16_SFLOAT          = 76,

    /// RG16 unsigned normalized
    R16G16_UNORM        = 77,
    /// RG16 signed normalized
    R16G16_SNORM        = 78,
    /// RG16 unsigned integer
    R16G16_UINT         = 81,
    /// RG16 signed integer
    R16G16_SINT         = 82,
    /// RG16 float
    R16G16_SFLOAT       = 83,

    /// RGBA16 unsigned normalized
    R16G16B16A16_UNORM  = 91,
    /// RGBA16 signed normalized
    R16G16B16A16_SNORM  = 92,
    /// RGBA16 unsigned integer
    R16G16B16A16_UINT   = 95,
    /// RGBA16 signed integer
    R16G16B16A16_SINT   = 96,
    /// RGBA16 float
    R16G16B16A16_SFLOAT = 97,

    // 32-bit formats
    /// R32 unsigned integer
    R32_UINT            = 98,
    /// R32 signed integer
    R32_SINT            = 99,
    /// R32 float
    R32_SFLOAT          = 100,

    /// RG32 unsigned integer
    R32G32_UINT         = 101,
    /// RG32 signed integer
    R32G32_SINT         = 102,
    /// RG32 float
    R32G32_SFLOAT       = 103,

    /// RGB32 unsigned integer
    R32G32B32_UINT      = 104,
    /// RGB32 signed integer
    R32G32B32_SINT      = 105,
    /// RGB32 float
    R32G32B32_SFLOAT    = 106,

    /// RGBA32 unsigned integer
    R32G32B32A32_UINT   = 107,
    /// RGBA32 signed integer
    R32G32B32A32_SINT   = 108,
    /// RGBA32 float
    R32G32B32A32_SFLOAT = 109,

    // Depth formats
    /// D16 unsigned normalized
    D16_UNORM           = 124,
    /// D32 float
    D32_SFLOAT          = 126,
    /// D24 unsigned normalized + S8 unsigned integer
    D24_UNORM_S8_UINT   = 129,
    /// D32 float + S8 unsigned integer
    D32_SFLOAT_S8_UINT  = 130,

    // BC compressed formats
    /// BC1 RGB unsigned normalized
    BC1_RGB_UNORM       = 131,
    /// BC1 RGB sRGB
    BC1_RGB_SRGB        = 132,
    /// BC1 RGBA unsigned normalized
    BC1_RGBA_UNORM      = 133,
    /// BC1 RGBA sRGB
    BC1_RGBA_SRGB       = 134,
    /// BC2 unsigned normalized
    BC2_UNORM           = 135,
    /// BC2 sRGB
    BC2_SRGB            = 136,
    /// BC3 unsigned normalized
    BC3_UNORM           = 137,
    /// BC3 sRGB
    BC3_SRGB            = 138,
    /// BC4 unsigned normalized
    BC4_UNORM           = 139,
    /// BC4 signed normalized
    BC4_SNORM           = 140,
    /// BC5 unsigned normalized
    BC5_UNORM           = 141,
    /// BC5 signed normalized
    BC5_SNORM           = 142,
    /// BC6H unsigned float
    BC6H_UFLOAT         = 143,
    /// BC6H signed float
    BC6H_SFLOAT         = 144,
    /// BC7 unsigned normalized
    BC7_UNORM           = 145,
    /// BC7 sRGB
    BC7_SRGB            = 146,

    // ETC2 formats
    /// ETC2 RGB8 unsigned normalized
    ETC2_R8G8B8_UNORM   = 147,
    /// ETC2 RGB8 sRGB
    ETC2_R8G8B8_SRGB    = 148,
    /// ETC2 RGB8A1 unsigned normalized
    ETC2_R8G8B8A1_UNORM = 149,
    /// ETC2 RGB8A1 sRGB
    ETC2_R8G8B8A1_SRGB  = 150,
    /// ETC2 RGBA8 unsigned normalized
    ETC2_R8G8B8A8_UNORM = 151,
    /// ETC2 RGBA8 sRGB
    ETC2_R8G8B8A8_SRGB  = 152,

    // ASTC formats
    /// ASTC 4x4 unsigned normalized
    ASTC_4x4_UNORM      = 157,
    /// ASTC 4x4 sRGB
    ASTC_4x4_SRGB       = 158,
    /// ASTC 5x4 unsigned normalized
    ASTC_5x4_UNORM      = 159,
    /// ASTC 5x4 sRGB
    ASTC_5x4_SRGB       = 160,
    /// ASTC 5x5 unsigned normalized
    ASTC_5x5_UNORM      = 161,
    /// ASTC 5x5 sRGB
    ASTC_5x5_SRGB       = 162,
    /// ASTC 6x5 unsigned normalized
    ASTC_6x5_UNORM      = 163,
    /// ASTC 6x5 sRGB
    ASTC_6x5_SRGB       = 164,
    /// ASTC 6x6 unsigned normalized
    ASTC_6x6_UNORM      = 165,
    /// ASTC 6x6 sRGB
    ASTC_6x6_SRGB       = 166,
    /// ASTC 8x5 unsigned normalized
    ASTC_8x5_UNORM      = 167,
    /// ASTC 8x5 sRGB
    ASTC_8x5_SRGB       = 168,
    /// ASTC 8x6 unsigned normalized
    ASTC_8x6_UNORM      = 169,
    /// ASTC 8x6 sRGB
    ASTC_8x6_SRGB       = 170,
    /// ASTC 8x8 unsigned normalized
    ASTC_8x8_UNORM      = 171,
    /// ASTC 8x8 sRGB
    ASTC_8x8_SRGB       = 172,
    /// ASTC 10x5 unsigned normalized
    ASTC_10x5_UNORM     = 173,
    /// ASTC 10x5 sRGB
    ASTC_10x5_SRGB      = 174,
    /// ASTC 10x6 unsigned normalized
    ASTC_10x6_UNORM     = 175,
    /// ASTC 10x6 sRGB
    ASTC_10x6_SRGB      = 176,
    /// ASTC 10x8 unsigned normalized
    ASTC_10x8_UNORM     = 177,
    /// ASTC 10x8 sRGB
    ASTC_10x8_SRGB      = 178,
    /// ASTC 10x10 unsigned normalized
    ASTC_10x10_UNORM    = 179,
    /// ASTC 10x10 sRGB
    ASTC_10x10_SRGB     = 180,
    /// ASTC 12x10 unsigned normalized
    ASTC_12x10_UNORM    = 181,
    /// ASTC 12x10 sRGB
    ASTC_12x10_SRGB     = 182,
    /// ASTC 12x12 unsigned normalized
    ASTC_12x12_UNORM    = 183,
    /// ASTC 12x12 sRGB
    ASTC_12x12_SRGB     = 184,
}

impl Format {
    /// Gets format info
    pub const fn info(self) -> FormatInfo {
        FormatInfo::for_format(self)
    }

    /// Is this an sRGB format
    pub const fn is_srgb(self) -> bool {
        matches!(
            self,
            Self::R8G8B8A8_SRGB
                | Self::B8G8R8A8_SRGB
                | Self::BC1_RGB_SRGB
                | Self::BC1_RGBA_SRGB
                | Self::BC2_SRGB
                | Self::BC3_SRGB
                | Self::BC7_SRGB
                | Self::ETC2_R8G8B8_SRGB
                | Self::ETC2_R8G8B8A1_SRGB
                | Self::ETC2_R8G8B8A8_SRGB
                | Self::ASTC_4x4_SRGB
                | Self::ASTC_5x4_SRGB
                | Self::ASTC_5x5_SRGB
                | Self::ASTC_6x5_SRGB
                | Self::ASTC_6x6_SRGB
                | Self::ASTC_8x5_SRGB
                | Self::ASTC_8x6_SRGB
                | Self::ASTC_8x8_SRGB
                | Self::ASTC_10x5_SRGB
                | Self::ASTC_10x6_SRGB
                | Self::ASTC_10x8_SRGB
                | Self::ASTC_10x10_SRGB
                | Self::ASTC_12x10_SRGB
                | Self::ASTC_12x12_SRGB
        )
    }

    /// Gets non-sRGB version of this format
    pub const fn to_linear(self) -> Self {
        match self {
            Self::R8G8B8A8_SRGB => Self::R8G8B8A8_UNORM,
            Self::B8G8R8A8_SRGB => Self::B8G8R8A8_UNORM,
            Self::BC1_RGB_SRGB => Self::BC1_RGB_UNORM,
            Self::BC1_RGBA_SRGB => Self::BC1_RGBA_UNORM,
            Self::BC2_SRGB => Self::BC2_UNORM,
            Self::BC3_SRGB => Self::BC3_UNORM,
            Self::BC7_SRGB => Self::BC7_UNORM,
            _ => self,
        }
    }

    /// Gets sRGB version of this format
    pub const fn to_srgb(self) -> Self {
        match self {
            Self::R8G8B8A8_UNORM => Self::R8G8B8A8_SRGB,
            Self::B8G8R8A8_UNORM => Self::B8G8R8A8_SRGB,
            Self::BC1_RGB_UNORM => Self::BC1_RGB_SRGB,
            Self::BC1_RGBA_UNORM => Self::BC1_RGBA_SRGB,
            Self::BC2_UNORM => Self::BC2_SRGB,
            Self::BC3_UNORM => Self::BC3_SRGB,
            Self::BC7_UNORM => Self::BC7_SRGB,
            _ => self,
        }
    }
}

/// Format class
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum FormatClass {
    /// Unknown format class
    #[default]
    Unknown = 0,
    /// R8 class
    R8      = 1,
    /// RG8 class
    RG8     = 2,
    /// RGB8 class
    RGB8    = 3,
    /// RGBA8 class
    RGBA8   = 4,
    /// R16 class
    R16     = 5,
    /// RG16 class
    RG16    = 6,
    /// RGB16 class
    RGB16   = 7,
    /// RGBA16 class
    RGBA16  = 8,
    /// R32 class
    R32     = 9,
    /// RG32 class
    RG32    = 10,
    /// RGB32 class
    RGB32   = 11,
    /// RGBA32 class
    RGBA32  = 12,
    /// D16 class
    D16     = 13,
    /// D32 class
    D32     = 14,
    /// D24S8 class
    D24S8   = 15,
    /// D32S8 class
    D32S8   = 16,
    /// BC1 class
    BC1     = 17,
    /// BC2 class
    BC2     = 18,
    /// BC3 class
    BC3     = 19,
    /// BC4 class
    BC4     = 20,
    /// BC5 class
    BC5     = 21,
    /// BC6H class
    BC6H    = 22,
    /// BC7 class
    BC7     = 23,
    /// ETC2 class
    ETC2    = 24,
    /// ASTC class
    ASTC    = 25,
}

bitflags::bitflags! {
    /// Format aspects
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct FormatAspects: u32 {
        /// Color aspect
        const COLOR = 1 << 0;
        /// Depth aspect
        const DEPTH = 1 << 1;
        /// Stencil aspect
        const STENCIL = 1 << 2;
        /// Metadata aspect
        const METADATA = 1 << 3;
        /// Plane 0 aspect
        const PLANE_0 = 1 << 4;
        /// Plane 1 aspect
        const PLANE_1 = 1 << 5;
        /// Plane 2 aspect
        const PLANE_2 = 1 << 6;
    }
}

impl FormatAspects {
    /// Depth + Stencil aspects
    pub const DEPTH_STENCIL: Self =
        Self::from_bits_truncate(Self::DEPTH.bits() | Self::STENCIL.bits());
}

/// Format feature flags
bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct FormatFeatureFlags: u64 {
        /// Sampled image
        const SAMPLED_IMAGE = 1 << 0;
        /// Storage image
        const STORAGE_IMAGE = 1 << 1;
        /// Storage image atomic
        const STORAGE_IMAGE_ATOMIC = 1 << 2;
        /// Uniform texel buffer
        const UNIFORM_TEXEL_BUFFER = 1 << 3;
        /// Storage texel buffer
        const STORAGE_TEXEL_BUFFER = 1 << 4;
        /// Storage texel buffer atomic
        const STORAGE_TEXEL_BUFFER_ATOMIC = 1 << 5;
        /// Vertex buffer
        const VERTEX_BUFFER = 1 << 6;
        /// Color attachment
        const COLOR_ATTACHMENT = 1 << 7;
        /// Color attachment blend
        const COLOR_ATTACHMENT_BLEND = 1 << 8;
        /// Depth stencil attachment
        const DEPTH_STENCIL_ATTACHMENT = 1 << 9;
        /// Blit source
        const BLIT_SRC = 1 << 10;
        /// Blit destination
        const BLIT_DST = 1 << 11;
        /// Sampled image filter linear
        const SAMPLED_IMAGE_FILTER_LINEAR = 1 << 12;
        /// Transfer source
        const TRANSFER_SRC = 1 << 14;
        /// Transfer destination
        const TRANSFER_DST = 1 << 15;
        /// Sampled image filter minmax
        const SAMPLED_IMAGE_FILTER_MINMAX = 1 << 16;
        /// Midpoint chroma samples
        const MIDPOINT_CHROMA_SAMPLES = 1 << 17;
        /// Sampled image ycbcr conversion linear filter
        const SAMPLED_IMAGE_YCBCR_CONVERSION_LINEAR_FILTER = 1 << 18;
        /// Sampled image ycbcr conversion separate reconstruction filter
        const SAMPLED_IMAGE_YCBCR_CONVERSION_SEPARATE_RECONSTRUCTION_FILTER = 1 << 19;
        /// Sampled image ycbcr conversion chroma reconstruction explicit
        const SAMPLED_IMAGE_YCBCR_CONVERSION_CHROMA_RECONSTRUCTION_EXPLICIT = 1 << 20;
        /// Sampled image ycbcr conversion chroma reconstruction explicit forceable
        const SAMPLED_IMAGE_YCBCR_CONVERSION_CHROMA_RECONSTRUCTION_EXPLICIT_FORCEABLE = 1 << 21;
        /// Disjoint
        const DISJOINT = 1 << 22;
        /// Cosited chroma samples
        const COSITED_CHROMA_SAMPLES = 1 << 23;
        /// Fragment shading rate attachment
        const FRAGMENT_SHADING_RATE_ATTACHMENT = 1 << 30;
    }
}

impl FormatFeatureFlags {
    /// Typical color attachment features
    pub const COLOR_ATTACHMENT_ALL: Self = Self::from_bits_truncate(
        Self::COLOR_ATTACHMENT.bits()
            | Self::COLOR_ATTACHMENT_BLEND.bits()
            | Self::BLIT_SRC.bits()
            | Self::BLIT_DST.bits(),
    );

    /// Typical texture features
    pub const TEXTURE: Self = Self::from_bits_truncate(
        Self::SAMPLED_IMAGE.bits()
            | Self::SAMPLED_IMAGE_FILTER_LINEAR.bits()
            | Self::TRANSFER_SRC.bits()
            | Self::TRANSFER_DST.bits(),
    );
}

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
    /// Can be used as color attachment (optimal tiling)
    pub fn supports_color_attachment(&self) -> bool {
        self.optimal_tiling_features
            .contains(FormatFeatureFlags::COLOR_ATTACHMENT)
    }

    /// Can be used as depth attachment
    pub fn supports_depth_attachment(&self) -> bool {
        self.optimal_tiling_features
            .contains(FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT)
    }

    /// Can be sampled with linear filtering
    pub fn supports_linear_filter(&self) -> bool {
        self.optimal_tiling_features
            .contains(FormatFeatureFlags::SAMPLED_IMAGE_FILTER_LINEAR)
    }

    /// Can be used as storage image
    pub fn supports_storage(&self) -> bool {
        self.optimal_tiling_features
            .contains(FormatFeatureFlags::STORAGE_IMAGE)
    }
}

/// Drm format modifier
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DrmFormatModifier {
    /// DRM format modifier
    pub drm_format_modifier: u64,
    /// Plane count
    pub drm_format_modifier_plane_count: u32,
    /// Tiling features
    pub drm_format_modifier_tiling_features: FormatFeatureFlags,
}

/// Common DRM modifiers
pub mod drm_modifiers {
    /// Linear (no tiling)
    pub const LINEAR: u64 = 0;
    /// Invalid modifier
    pub const INVALID: u64 = 0x00ffffffffffffff;
    /// Intel X-tiling
    pub const INTEL_X_TILED: u64 = 0x0100000000000001;
    /// Intel Y-tiling
    pub const INTEL_Y_TILED: u64 = 0x0100000000000002;
    /// Intel Yf-tiling
    pub const INTEL_YF_TILED: u64 = 0x0100000000000003;
    /// AMD GFX9 64KB S
    pub const AMD_GFX9_64KB_S: u64 = 0x0200000000000001;
    /// AMD GFX9 64KB D
    pub const AMD_GFX9_64KB_D: u64 = 0x0200000000000002;
}
