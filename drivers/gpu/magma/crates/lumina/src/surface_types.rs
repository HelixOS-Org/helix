//! Surface and Swapchain Types for Lumina
//!
//! This module provides surface configuration, swapchain creation,
//! and presentation types.

// ============================================================================
// Surface Handle
// ============================================================================

/// Surface handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SurfaceHandle(pub u64);

impl SurfaceHandle {
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

impl Default for SurfaceHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Swapchain Handle
// ============================================================================

/// Swapchain handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SwapchainHandle(pub u64);

impl SwapchainHandle {
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

impl Default for SwapchainHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Surface Capabilities
// ============================================================================

/// Surface capabilities
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SurfaceCapabilities {
    /// Minimum image count
    pub min_image_count: u32,
    /// Maximum image count (0 = unlimited)
    pub max_image_count: u32,
    /// Current extent
    pub current_extent: Extent2D,
    /// Minimum image extent
    pub min_image_extent: Extent2D,
    /// Maximum image extent
    pub max_image_extent: Extent2D,
    /// Maximum image array layers
    pub max_image_array_layers: u32,
    /// Supported transforms
    pub supported_transforms: SurfaceTransformFlags,
    /// Current transform
    pub current_transform: SurfaceTransformFlags,
    /// Supported composite alpha
    pub supported_composite_alpha: CompositeAlphaFlags,
    /// Supported usage flags
    pub supported_usage_flags: ImageUsageFlags,
}

impl SurfaceCapabilities {
    /// Creates default capabilities
    #[inline]
    pub const fn new() -> Self {
        Self {
            min_image_count: 2,
            max_image_count: 8,
            current_extent: Extent2D::UNDEFINED,
            min_image_extent: Extent2D::new(1, 1),
            max_image_extent: Extent2D::new(16384, 16384),
            max_image_array_layers: 1,
            supported_transforms: SurfaceTransformFlags::IDENTITY,
            current_transform: SurfaceTransformFlags::IDENTITY,
            supported_composite_alpha: CompositeAlphaFlags::OPAQUE,
            supported_usage_flags: ImageUsageFlags::COLOR_ATTACHMENT,
        }
    }

    /// Optimal image count
    #[inline]
    pub const fn optimal_image_count(&self) -> u32 {
        // Prefer triple buffering
        let count = 3;
        if count < self.min_image_count {
            self.min_image_count
        } else if self.max_image_count > 0 && count > self.max_image_count {
            self.max_image_count
        } else {
            count
        }
    }

    /// Has undefined extent
    #[inline]
    pub const fn has_undefined_extent(&self) -> bool {
        self.current_extent.width == 0xFFFFFFFF || self.current_extent.height == 0xFFFFFFFF
    }

    /// Clamp extent to valid range
    #[inline]
    pub const fn clamp_extent(&self, extent: Extent2D) -> Extent2D {
        Extent2D {
            width: if extent.width < self.min_image_extent.width {
                self.min_image_extent.width
            } else if extent.width > self.max_image_extent.width {
                self.max_image_extent.width
            } else {
                extent.width
            },
            height: if extent.height < self.min_image_extent.height {
                self.min_image_extent.height
            } else if extent.height > self.max_image_extent.height {
                self.max_image_extent.height
            } else {
                extent.height
            },
        }
    }
}

impl Default for SurfaceCapabilities {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Extent 2D
// ============================================================================

/// 2D extent
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Extent2D {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
}

impl Extent2D {
    /// Unit extent (1x1)
    pub const UNIT: Self = Self {
        width: 1,
        height: 1,
    };

    /// Undefined extent (special value)
    pub const UNDEFINED: Self = Self {
        width: 0xFFFFFFFF,
        height: 0xFFFFFFFF,
    };

    /// Common resolutions
    pub const R_720P: Self = Self::new(1280, 720);
    pub const R_1080P: Self = Self::new(1920, 1080);
    pub const R_1440P: Self = Self::new(2560, 1440);
    pub const R_4K: Self = Self::new(3840, 2160);
    pub const R_8K: Self = Self::new(7680, 4320);

    /// Creates new extent
    #[inline]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Area (total pixels)
    #[inline]
    pub const fn area(&self) -> u64 {
        self.width as u64 * self.height as u64
    }

    /// Aspect ratio
    #[inline]
    pub const fn aspect_ratio(&self) -> f32 {
        if self.height == 0 {
            0.0
        } else {
            self.width as f32 / self.height as f32
        }
    }

    /// Is landscape
    #[inline]
    pub const fn is_landscape(&self) -> bool {
        self.width > self.height
    }

    /// Is portrait
    #[inline]
    pub const fn is_portrait(&self) -> bool {
        self.height > self.width
    }

    /// Is square
    #[inline]
    pub const fn is_square(&self) -> bool {
        self.width == self.height
    }

    /// Is undefined
    #[inline]
    pub const fn is_undefined(&self) -> bool {
        self.width == 0xFFFFFFFF || self.height == 0xFFFFFFFF
    }
}

// ============================================================================
// Surface Transform Flags
// ============================================================================

/// Surface transform flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct SurfaceTransformFlags(pub u32);

impl SurfaceTransformFlags {
    /// Identity (no transform)
    pub const IDENTITY: Self = Self(1 << 0);
    /// Rotate 90 degrees
    pub const ROTATE_90: Self = Self(1 << 1);
    /// Rotate 180 degrees
    pub const ROTATE_180: Self = Self(1 << 2);
    /// Rotate 270 degrees
    pub const ROTATE_270: Self = Self(1 << 3);
    /// Horizontal mirror
    pub const HORIZONTAL_MIRROR: Self = Self(1 << 4);
    /// Horizontal mirror rotate 90
    pub const HORIZONTAL_MIRROR_ROTATE_90: Self = Self(1 << 5);
    /// Horizontal mirror rotate 180
    pub const HORIZONTAL_MIRROR_ROTATE_180: Self = Self(1 << 6);
    /// Horizontal mirror rotate 270
    pub const HORIZONTAL_MIRROR_ROTATE_270: Self = Self(1 << 7);
    /// Inherit
    pub const INHERIT: Self = Self(1 << 8);

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

    /// Rotation angle in degrees
    #[inline]
    pub const fn rotation_degrees(&self) -> u32 {
        match self.0 {
            0x02 | 0x20 => 90,
            0x04 | 0x40 => 180,
            0x08 | 0x80 => 270,
            _ => 0,
        }
    }

    /// Is mirrored
    #[inline]
    pub const fn is_mirrored(&self) -> bool {
        (self.0 & 0xF0) != 0
    }
}

// ============================================================================
// Composite Alpha Flags
// ============================================================================

/// Composite alpha flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct CompositeAlphaFlags(pub u32);

impl CompositeAlphaFlags {
    /// Opaque
    pub const OPAQUE: Self = Self(1 << 0);
    /// Pre-multiplied
    pub const PRE_MULTIPLIED: Self = Self(1 << 1);
    /// Post-multiplied
    pub const POST_MULTIPLIED: Self = Self(1 << 2);
    /// Inherit
    pub const INHERIT: Self = Self(1 << 3);
    /// All
    pub const ALL: Self = Self(0x0F);

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
// Image Usage Flags
// ============================================================================

/// Image usage flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ImageUsageFlags(pub u32);

impl ImageUsageFlags {
    /// No usage
    pub const NONE: Self = Self(0);
    /// Transfer source
    pub const TRANSFER_SRC: Self = Self(1 << 0);
    /// Transfer destination
    pub const TRANSFER_DST: Self = Self(1 << 1);
    /// Sampled image
    pub const SAMPLED: Self = Self(1 << 2);
    /// Storage image
    pub const STORAGE: Self = Self(1 << 3);
    /// Color attachment
    pub const COLOR_ATTACHMENT: Self = Self(1 << 4);
    /// Depth stencil attachment
    pub const DEPTH_STENCIL_ATTACHMENT: Self = Self(1 << 5);
    /// Transient attachment
    pub const TRANSIENT_ATTACHMENT: Self = Self(1 << 6);
    /// Input attachment
    pub const INPUT_ATTACHMENT: Self = Self(1 << 7);
    /// Fragment density map
    pub const FRAGMENT_DENSITY_MAP: Self = Self(1 << 9);
    /// Fragment shading rate attachment
    pub const FRAGMENT_SHADING_RATE_ATTACHMENT: Self = Self(1 << 8);
    /// Video decode destination
    pub const VIDEO_DECODE_DST: Self = Self(1 << 10);
    /// Video decode source
    pub const VIDEO_DECODE_SRC: Self = Self(1 << 11);
    /// Video decode DPB
    pub const VIDEO_DECODE_DPB: Self = Self(1 << 12);
    /// Video encode destination
    pub const VIDEO_ENCODE_DST: Self = Self(1 << 13);
    /// Video encode source
    pub const VIDEO_ENCODE_SRC: Self = Self(1 << 14);
    /// Video encode DPB
    pub const VIDEO_ENCODE_DPB: Self = Self(1 << 15);
    /// Invocation mask
    pub const INVOCATION_MASK: Self = Self(1 << 18);
    /// Attachment feedback loop
    pub const ATTACHMENT_FEEDBACK_LOOP: Self = Self(1 << 19);
    /// Host transfer
    pub const HOST_TRANSFER: Self = Self(1 << 22);

    /// Common render target
    pub const RENDER_TARGET: Self = Self(
        Self::COLOR_ATTACHMENT.0 | Self::SAMPLED.0 | Self::TRANSFER_SRC.0 | Self::TRANSFER_DST.0,
    );

    /// Common depth buffer
    pub const DEPTH_BUFFER: Self = Self(Self::DEPTH_STENCIL_ATTACHMENT.0 | Self::SAMPLED.0);

    /// Common texture
    pub const TEXTURE: Self = Self(Self::SAMPLED.0 | Self::TRANSFER_DST.0 | Self::TRANSFER_SRC.0);

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

    /// Intersection
    #[inline]
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Is empty
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }
}

// ============================================================================
// Surface Format
// ============================================================================

/// Surface format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct SurfaceFormat {
    /// Format
    pub format: Format,
    /// Color space
    pub color_space: ColorSpace,
}

impl SurfaceFormat {
    /// BGRA8 SRGB (most common)
    pub const BGRA8_SRGB: Self = Self {
        format: Format::B8G8R8A8_SRGB,
        color_space: ColorSpace::SrgbNonlinear,
    };

    /// BGRA8 UNORM
    pub const BGRA8_UNORM: Self = Self {
        format: Format::B8G8R8A8_UNORM,
        color_space: ColorSpace::SrgbNonlinear,
    };

    /// RGBA8 SRGB
    pub const RGBA8_SRGB: Self = Self {
        format: Format::R8G8B8A8_SRGB,
        color_space: ColorSpace::SrgbNonlinear,
    };

    /// RGBA8 UNORM
    pub const RGBA8_UNORM: Self = Self {
        format: Format::R8G8B8A8_UNORM,
        color_space: ColorSpace::SrgbNonlinear,
    };

    /// RGBA16F HDR
    pub const RGBA16F_HDR: Self = Self {
        format: Format::R16G16B16A16_SFLOAT,
        color_space: ColorSpace::ExtendedSrgbLinear,
    };

    /// RGB10A2 HDR
    pub const RGB10A2_HDR: Self = Self {
        format: Format::A2B10G10R10_UNORM_PACK32,
        color_space: ColorSpace::Hdr10St2084,
    };

    /// Creates new format
    #[inline]
    pub const fn new(format: Format, color_space: ColorSpace) -> Self {
        Self {
            format,
            color_space,
        }
    }

    /// Is HDR
    #[inline]
    pub const fn is_hdr(&self) -> bool {
        self.color_space.is_hdr()
    }

    /// Is sRGB
    #[inline]
    pub const fn is_srgb(&self) -> bool {
        matches!(self.color_space, ColorSpace::SrgbNonlinear)
    }
}

impl Default for SurfaceFormat {
    fn default() -> Self {
        Self::BGRA8_SRGB
    }
}

// ============================================================================
// Format
// ============================================================================

/// Image/buffer format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Format {
    /// Undefined
    Undefined            = 0,

    // 8-bit formats
    R8_UNORM             = 9,
    R8_SNORM             = 10,
    R8_UINT              = 13,
    R8_SINT              = 14,
    R8_SRGB              = 15,

    // 16-bit formats
    R8G8_UNORM           = 16,
    R8G8_SNORM           = 17,
    R8G8_UINT            = 20,
    R8G8_SINT            = 21,
    R8G8_SRGB            = 22,
    R16_UNORM            = 70,
    R16_SNORM            = 71,
    R16_UINT             = 74,
    R16_SINT             = 75,
    R16_SFLOAT           = 76,

    // 24-bit formats
    R8G8B8_UNORM         = 23,
    R8G8B8_SNORM         = 24,
    R8G8B8_UINT          = 27,
    R8G8B8_SINT          = 28,
    R8G8B8_SRGB          = 29,
    B8G8R8_UNORM         = 30,
    B8G8R8_SNORM         = 31,
    B8G8R8_UINT          = 34,
    B8G8R8_SINT          = 35,
    B8G8R8_SRGB          = 36,

    // 32-bit formats
    R8G8B8A8_UNORM       = 37,
    R8G8B8A8_SNORM       = 38,
    R8G8B8A8_UINT        = 41,
    R8G8B8A8_SINT        = 42,
    R8G8B8A8_SRGB        = 43,
    B8G8R8A8_UNORM       = 44,
    B8G8R8A8_SNORM       = 45,
    B8G8R8A8_UINT        = 48,
    B8G8R8A8_SINT        = 49,
    B8G8R8A8_SRGB        = 50,
    A2R10G10B10_UNORM_PACK32 = 58,
    A2R10G10B10_UINT_PACK32 = 60,
    A2B10G10R10_UNORM_PACK32 = 64,
    A2B10G10R10_UINT_PACK32 = 66,
    R16G16_UNORM         = 77,
    R16G16_SNORM         = 78,
    R16G16_UINT          = 81,
    R16G16_SINT          = 82,
    R16G16_SFLOAT        = 83,
    R32_UINT             = 98,
    R32_SINT             = 99,
    R32_SFLOAT           = 100,

    // 48-bit formats
    R16G16B16_UNORM      = 84,
    R16G16B16_SNORM      = 85,
    R16G16B16_UINT       = 88,
    R16G16B16_SINT       = 89,
    R16G16B16_SFLOAT     = 90,

    // 64-bit formats
    R16G16B16A16_UNORM   = 91,
    R16G16B16A16_SNORM   = 92,
    R16G16B16A16_UINT    = 95,
    R16G16B16A16_SINT    = 96,
    R16G16B16A16_SFLOAT  = 97,
    R32G32_UINT          = 101,
    R32G32_SINT          = 102,
    R32G32_SFLOAT        = 103,
    R64_UINT             = 110,
    R64_SINT             = 111,
    R64_SFLOAT           = 112,

    // 96-bit formats
    R32G32B32_UINT       = 104,
    R32G32B32_SINT       = 105,
    R32G32B32_SFLOAT     = 106,

    // 128-bit formats
    R32G32B32A32_UINT    = 107,
    R32G32B32A32_SINT    = 108,
    R32G32B32A32_SFLOAT  = 109,
    R64G64_UINT          = 113,
    R64G64_SINT          = 114,
    R64G64_SFLOAT        = 115,

    // Depth formats
    D16_UNORM            = 124,
    X8_D24_UNORM_PACK32  = 125,
    D32_SFLOAT           = 126,
    S8_UINT              = 127,
    D16_UNORM_S8_UINT    = 128,
    D24_UNORM_S8_UINT    = 129,
    D32_SFLOAT_S8_UINT   = 130,

    // Compressed formats - BC
    BC1_RGB_UNORM_BLOCK  = 131,
    BC1_RGB_SRGB_BLOCK   = 132,
    BC1_RGBA_UNORM_BLOCK = 133,
    BC1_RGBA_SRGB_BLOCK  = 134,
    BC2_UNORM_BLOCK      = 135,
    BC2_SRGB_BLOCK       = 136,
    BC3_UNORM_BLOCK      = 137,
    BC3_SRGB_BLOCK       = 138,
    BC4_UNORM_BLOCK      = 139,
    BC4_SNORM_BLOCK      = 140,
    BC5_UNORM_BLOCK      = 141,
    BC5_SNORM_BLOCK      = 142,
    BC6H_UFLOAT_BLOCK    = 143,
    BC6H_SFLOAT_BLOCK    = 144,
    BC7_UNORM_BLOCK      = 145,
    BC7_SRGB_BLOCK       = 146,

    // Compressed formats - ETC2
    ETC2_R8G8B8_UNORM_BLOCK = 147,
    ETC2_R8G8B8_SRGB_BLOCK = 148,
    ETC2_R8G8B8A1_UNORM_BLOCK = 149,
    ETC2_R8G8B8A1_SRGB_BLOCK = 150,
    ETC2_R8G8B8A8_UNORM_BLOCK = 151,
    ETC2_R8G8B8A8_SRGB_BLOCK = 152,

    // Compressed formats - ASTC
    ASTC_4X4_UNORM_BLOCK = 157,
    ASTC_4X4_SRGB_BLOCK  = 158,
    ASTC_5X4_UNORM_BLOCK = 159,
    ASTC_5X4_SRGB_BLOCK  = 160,
    ASTC_5X5_UNORM_BLOCK = 161,
    ASTC_5X5_SRGB_BLOCK  = 162,
    ASTC_6X5_UNORM_BLOCK = 163,
    ASTC_6X5_SRGB_BLOCK  = 164,
    ASTC_6X6_UNORM_BLOCK = 165,
    ASTC_6X6_SRGB_BLOCK  = 166,
    ASTC_8X5_UNORM_BLOCK = 167,
    ASTC_8X5_SRGB_BLOCK  = 168,
    ASTC_8X6_UNORM_BLOCK = 169,
    ASTC_8X6_SRGB_BLOCK  = 170,
    ASTC_8X8_UNORM_BLOCK = 171,
    ASTC_8X8_SRGB_BLOCK  = 172,
    ASTC_10X5_UNORM_BLOCK = 173,
    ASTC_10X5_SRGB_BLOCK = 174,
    ASTC_10X6_UNORM_BLOCK = 175,
    ASTC_10X6_SRGB_BLOCK = 176,
    ASTC_10X8_UNORM_BLOCK = 177,
    ASTC_10X8_SRGB_BLOCK = 178,
    ASTC_10X10_UNORM_BLOCK = 179,
    ASTC_10X10_SRGB_BLOCK = 180,
    ASTC_12X10_UNORM_BLOCK = 181,
    ASTC_12X10_SRGB_BLOCK = 182,
    ASTC_12X12_UNORM_BLOCK = 183,
    ASTC_12X12_SRGB_BLOCK = 184,
}

impl Format {
    /// Bytes per pixel (0 for compressed)
    #[inline]
    pub const fn bytes_per_pixel(&self) -> u32 {
        match self {
            Self::Undefined => 0,
            Self::R8_UNORM
            | Self::R8_SNORM
            | Self::R8_UINT
            | Self::R8_SINT
            | Self::R8_SRGB
            | Self::S8_UINT => 1,
            Self::R8G8_UNORM
            | Self::R8G8_SNORM
            | Self::R8G8_UINT
            | Self::R8G8_SINT
            | Self::R8G8_SRGB
            | Self::R16_UNORM
            | Self::R16_SNORM
            | Self::R16_UINT
            | Self::R16_SINT
            | Self::R16_SFLOAT
            | Self::D16_UNORM => 2,
            Self::R8G8B8_UNORM
            | Self::R8G8B8_SNORM
            | Self::R8G8B8_UINT
            | Self::R8G8B8_SINT
            | Self::R8G8B8_SRGB
            | Self::B8G8R8_UNORM
            | Self::B8G8R8_SNORM
            | Self::B8G8R8_UINT
            | Self::B8G8R8_SINT
            | Self::B8G8R8_SRGB
            | Self::D16_UNORM_S8_UINT => 3,
            Self::R8G8B8A8_UNORM
            | Self::R8G8B8A8_SNORM
            | Self::R8G8B8A8_UINT
            | Self::R8G8B8A8_SINT
            | Self::R8G8B8A8_SRGB
            | Self::B8G8R8A8_UNORM
            | Self::B8G8R8A8_SNORM
            | Self::B8G8R8A8_UINT
            | Self::B8G8R8A8_SINT
            | Self::B8G8R8A8_SRGB
            | Self::A2R10G10B10_UNORM_PACK32
            | Self::A2R10G10B10_UINT_PACK32
            | Self::A2B10G10R10_UNORM_PACK32
            | Self::A2B10G10R10_UINT_PACK32
            | Self::R16G16_UNORM
            | Self::R16G16_SNORM
            | Self::R16G16_UINT
            | Self::R16G16_SINT
            | Self::R16G16_SFLOAT
            | Self::R32_UINT
            | Self::R32_SINT
            | Self::R32_SFLOAT
            | Self::X8_D24_UNORM_PACK32
            | Self::D32_SFLOAT
            | Self::D24_UNORM_S8_UINT => 4,
            Self::D32_SFLOAT_S8_UINT => 5,
            Self::R16G16B16_UNORM
            | Self::R16G16B16_SNORM
            | Self::R16G16B16_UINT
            | Self::R16G16B16_SINT
            | Self::R16G16B16_SFLOAT => 6,
            Self::R16G16B16A16_UNORM
            | Self::R16G16B16A16_SNORM
            | Self::R16G16B16A16_UINT
            | Self::R16G16B16A16_SINT
            | Self::R16G16B16A16_SFLOAT
            | Self::R32G32_UINT
            | Self::R32G32_SINT
            | Self::R32G32_SFLOAT
            | Self::R64_UINT
            | Self::R64_SINT
            | Self::R64_SFLOAT => 8,
            Self::R32G32B32_UINT | Self::R32G32B32_SINT | Self::R32G32B32_SFLOAT => 12,
            Self::R32G32B32A32_UINT
            | Self::R32G32B32A32_SINT
            | Self::R32G32B32A32_SFLOAT
            | Self::R64G64_UINT
            | Self::R64G64_SINT
            | Self::R64G64_SFLOAT => 16,
            _ => 0, // Compressed formats
        }
    }

    /// Is depth format
    #[inline]
    pub const fn is_depth(&self) -> bool {
        matches!(
            self,
            Self::D16_UNORM
                | Self::X8_D24_UNORM_PACK32
                | Self::D32_SFLOAT
                | Self::D16_UNORM_S8_UINT
                | Self::D24_UNORM_S8_UINT
                | Self::D32_SFLOAT_S8_UINT
        )
    }

    /// Is stencil format
    #[inline]
    pub const fn is_stencil(&self) -> bool {
        matches!(
            self,
            Self::S8_UINT
                | Self::D16_UNORM_S8_UINT
                | Self::D24_UNORM_S8_UINT
                | Self::D32_SFLOAT_S8_UINT
        )
    }

    /// Is depth stencil format
    #[inline]
    pub const fn is_depth_stencil(&self) -> bool {
        matches!(
            self,
            Self::D16_UNORM_S8_UINT | Self::D24_UNORM_S8_UINT | Self::D32_SFLOAT_S8_UINT
        )
    }

    /// Is compressed format
    #[inline]
    pub const fn is_compressed(&self) -> bool {
        (*self as u32) >= 131
    }

    /// Is sRGB format
    #[inline]
    pub const fn is_srgb(&self) -> bool {
        matches!(
            self,
            Self::R8_SRGB
                | Self::R8G8_SRGB
                | Self::R8G8B8_SRGB
                | Self::B8G8R8_SRGB
                | Self::R8G8B8A8_SRGB
                | Self::B8G8R8A8_SRGB
                | Self::BC1_RGB_SRGB_BLOCK
                | Self::BC1_RGBA_SRGB_BLOCK
                | Self::BC2_SRGB_BLOCK
                | Self::BC3_SRGB_BLOCK
                | Self::BC7_SRGB_BLOCK
                | Self::ETC2_R8G8B8_SRGB_BLOCK
                | Self::ETC2_R8G8B8A1_SRGB_BLOCK
                | Self::ETC2_R8G8B8A8_SRGB_BLOCK
                | Self::ASTC_4X4_SRGB_BLOCK
                | Self::ASTC_5X4_SRGB_BLOCK
                | Self::ASTC_5X5_SRGB_BLOCK
                | Self::ASTC_6X5_SRGB_BLOCK
                | Self::ASTC_6X6_SRGB_BLOCK
                | Self::ASTC_8X5_SRGB_BLOCK
                | Self::ASTC_8X6_SRGB_BLOCK
                | Self::ASTC_8X8_SRGB_BLOCK
                | Self::ASTC_10X5_SRGB_BLOCK
                | Self::ASTC_10X6_SRGB_BLOCK
                | Self::ASTC_10X8_SRGB_BLOCK
                | Self::ASTC_10X10_SRGB_BLOCK
                | Self::ASTC_12X10_SRGB_BLOCK
                | Self::ASTC_12X12_SRGB_BLOCK
        )
    }

    /// Number of components
    #[inline]
    pub const fn components(&self) -> u32 {
        match self {
            Self::Undefined => 0,
            Self::R8_UNORM
            | Self::R8_SNORM
            | Self::R8_UINT
            | Self::R8_SINT
            | Self::R8_SRGB
            | Self::R16_UNORM
            | Self::R16_SNORM
            | Self::R16_UINT
            | Self::R16_SINT
            | Self::R16_SFLOAT
            | Self::R32_UINT
            | Self::R32_SINT
            | Self::R32_SFLOAT
            | Self::R64_UINT
            | Self::R64_SINT
            | Self::R64_SFLOAT
            | Self::D16_UNORM
            | Self::X8_D24_UNORM_PACK32
            | Self::D32_SFLOAT
            | Self::S8_UINT => 1,
            Self::R8G8_UNORM
            | Self::R8G8_SNORM
            | Self::R8G8_UINT
            | Self::R8G8_SINT
            | Self::R8G8_SRGB
            | Self::R16G16_UNORM
            | Self::R16G16_SNORM
            | Self::R16G16_UINT
            | Self::R16G16_SINT
            | Self::R16G16_SFLOAT
            | Self::R32G32_UINT
            | Self::R32G32_SINT
            | Self::R32G32_SFLOAT
            | Self::R64G64_UINT
            | Self::R64G64_SINT
            | Self::R64G64_SFLOAT
            | Self::D16_UNORM_S8_UINT
            | Self::D24_UNORM_S8_UINT
            | Self::D32_SFLOAT_S8_UINT => 2,
            Self::R8G8B8_UNORM
            | Self::R8G8B8_SNORM
            | Self::R8G8B8_UINT
            | Self::R8G8B8_SINT
            | Self::R8G8B8_SRGB
            | Self::B8G8R8_UNORM
            | Self::B8G8R8_SNORM
            | Self::B8G8R8_UINT
            | Self::B8G8R8_SINT
            | Self::B8G8R8_SRGB
            | Self::R16G16B16_UNORM
            | Self::R16G16B16_SNORM
            | Self::R16G16B16_UINT
            | Self::R16G16B16_SINT
            | Self::R16G16B16_SFLOAT
            | Self::R32G32B32_UINT
            | Self::R32G32B32_SINT
            | Self::R32G32B32_SFLOAT => 3,
            _ => 4,
        }
    }
}

impl Default for Format {
    fn default() -> Self {
        Self::Undefined
    }
}

// ============================================================================
// Color Space
// ============================================================================

/// Color space
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ColorSpace {
    /// sRGB non-linear
    SrgbNonlinear      = 0,
    /// Display P3 non-linear
    DisplayP3Nonlinear = 1000104001,
    /// Extended sRGB linear
    ExtendedSrgbLinear = 1000104002,
    /// Display P3 linear
    DisplayP3Linear    = 1000104003,
    /// DCI-P3 non-linear
    DciP3Nonlinear     = 1000104004,
    /// BT709 linear
    Bt709Linear        = 1000104005,
    /// BT709 non-linear
    Bt709Nonlinear     = 1000104006,
    /// BT2020 linear
    Bt2020Linear       = 1000104007,
    /// HDR10 ST2084 (PQ)
    Hdr10St2084        = 1000104008,
    /// Dolby Vision
    DolbyVision        = 1000104009,
    /// HDR10 HLG
    Hdr10Hlg           = 1000104010,
    /// Adobe RGB linear
    AdobeRgbLinear     = 1000104011,
    /// Adobe RGB non-linear
    AdobeRgbNonlinear  = 1000104012,
    /// Pass through
    PassThrough        = 1000104013,
    /// Extended sRGB non-linear
    ExtendedSrgbNonlinear = 1000104014,
    /// Display native AMD
    DisplayNativeAmd   = 1000213000,
}

impl ColorSpace {
    /// Is HDR color space
    #[inline]
    pub const fn is_hdr(&self) -> bool {
        matches!(
            self,
            Self::Hdr10St2084
                | Self::Hdr10Hlg
                | Self::DolbyVision
                | Self::Bt2020Linear
                | Self::ExtendedSrgbLinear
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

    /// Name
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::SrgbNonlinear => "sRGB Non-Linear",
            Self::DisplayP3Nonlinear => "Display P3 Non-Linear",
            Self::ExtendedSrgbLinear => "Extended sRGB Linear",
            Self::DisplayP3Linear => "Display P3 Linear",
            Self::DciP3Nonlinear => "DCI-P3 Non-Linear",
            Self::Bt709Linear => "BT.709 Linear",
            Self::Bt709Nonlinear => "BT.709 Non-Linear",
            Self::Bt2020Linear => "BT.2020 Linear",
            Self::Hdr10St2084 => "HDR10 ST2084 (PQ)",
            Self::DolbyVision => "Dolby Vision",
            Self::Hdr10Hlg => "HDR10 HLG",
            Self::AdobeRgbLinear => "Adobe RGB Linear",
            Self::AdobeRgbNonlinear => "Adobe RGB Non-Linear",
            Self::PassThrough => "Pass Through",
            Self::ExtendedSrgbNonlinear => "Extended sRGB Non-Linear",
            Self::DisplayNativeAmd => "Display Native AMD",
        }
    }
}

impl Default for ColorSpace {
    fn default() -> Self {
        Self::SrgbNonlinear
    }
}

// ============================================================================
// Present Mode
// ============================================================================

/// Present mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum PresentMode {
    /// Immediate (no vsync, may tear)
    Immediate           = 0,
    /// Mailbox (triple buffering, low latency)
    Mailbox             = 1,
    /// FIFO (vsync, no tearing)
    Fifo                = 2,
    /// FIFO relaxed (vsync, may tear if late)
    FifoRelaxed         = 3,
    /// Shared demand refresh
    SharedDemandRefresh = 1000111000,
    /// Shared continuous refresh
    SharedContinuousRefresh = 1000111001,
}

impl PresentMode {
    /// Is vsync enabled
    #[inline]
    pub const fn is_vsync(&self) -> bool {
        matches!(self, Self::Fifo | Self::FifoRelaxed)
    }

    /// May tear
    #[inline]
    pub const fn may_tear(&self) -> bool {
        matches!(self, Self::Immediate | Self::FifoRelaxed)
    }

    /// Name
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Immediate => "Immediate",
            Self::Mailbox => "Mailbox (Triple Buffering)",
            Self::Fifo => "FIFO (V-Sync)",
            Self::FifoRelaxed => "FIFO Relaxed",
            Self::SharedDemandRefresh => "Shared Demand Refresh",
            Self::SharedContinuousRefresh => "Shared Continuous Refresh",
        }
    }
}

impl Default for PresentMode {
    fn default() -> Self {
        Self::Fifo
    }
}

// ============================================================================
// Swapchain Create Info
// ============================================================================

/// Swapchain create info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SwapchainCreateInfo {
    /// Flags
    pub flags: SwapchainCreateFlags,
    /// Surface handle
    pub surface: SurfaceHandle,
    /// Minimum image count
    pub min_image_count: u32,
    /// Image format
    pub image_format: Format,
    /// Image color space
    pub image_color_space: ColorSpace,
    /// Image extent
    pub image_extent: Extent2D,
    /// Image array layers
    pub image_array_layers: u32,
    /// Image usage
    pub image_usage: ImageUsageFlags,
    /// Image sharing mode
    pub image_sharing_mode: SharingMode,
    /// Pre-transform
    pub pre_transform: SurfaceTransformFlags,
    /// Composite alpha
    pub composite_alpha: CompositeAlphaFlags,
    /// Present mode
    pub present_mode: PresentMode,
    /// Clipped
    pub clipped: bool,
    /// Old swapchain (for recreation)
    pub old_swapchain: SwapchainHandle,
}

impl SwapchainCreateInfo {
    /// Creates new info
    #[inline]
    pub const fn new(surface: SurfaceHandle, extent: Extent2D) -> Self {
        Self {
            flags: SwapchainCreateFlags::NONE,
            surface,
            min_image_count: 3,
            image_format: Format::B8G8R8A8_SRGB,
            image_color_space: ColorSpace::SrgbNonlinear,
            image_extent: extent,
            image_array_layers: 1,
            image_usage: ImageUsageFlags::COLOR_ATTACHMENT,
            image_sharing_mode: SharingMode::Exclusive,
            pre_transform: SurfaceTransformFlags::IDENTITY,
            composite_alpha: CompositeAlphaFlags::OPAQUE,
            present_mode: PresentMode::Fifo,
            clipped: true,
            old_swapchain: SwapchainHandle::NULL,
        }
    }

    /// With format
    #[inline]
    pub const fn with_format(mut self, format: SurfaceFormat) -> Self {
        self.image_format = format.format;
        self.image_color_space = format.color_space;
        self
    }

    /// With present mode
    #[inline]
    pub const fn with_present_mode(mut self, mode: PresentMode) -> Self {
        self.present_mode = mode;
        self
    }

    /// With image count
    #[inline]
    pub const fn with_image_count(mut self, count: u32) -> Self {
        self.min_image_count = count;
        self
    }

    /// With usage
    #[inline]
    pub const fn with_usage(mut self, usage: ImageUsageFlags) -> Self {
        self.image_usage = usage;
        self
    }

    /// With transform
    #[inline]
    pub const fn with_transform(mut self, transform: SurfaceTransformFlags) -> Self {
        self.pre_transform = transform;
        self
    }

    /// With composite alpha
    #[inline]
    pub const fn with_composite_alpha(mut self, alpha: CompositeAlphaFlags) -> Self {
        self.composite_alpha = alpha;
        self
    }

    /// With old swapchain (for recreation)
    #[inline]
    pub const fn with_old_swapchain(mut self, old: SwapchainHandle) -> Self {
        self.old_swapchain = old;
        self
    }

    /// With clipping disabled
    #[inline]
    pub const fn without_clipping(mut self) -> Self {
        self.clipped = false;
        self
    }

    /// For VR (stereo, 2 layers)
    #[inline]
    pub const fn for_vr(mut self) -> Self {
        self.image_array_layers = 2;
        self
    }
}

impl Default for SwapchainCreateInfo {
    fn default() -> Self {
        Self::new(SurfaceHandle::NULL, Extent2D::R_1080P)
    }
}

/// Swapchain create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct SwapchainCreateFlags(pub u32);

impl SwapchainCreateFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Split instance bind regions
    pub const SPLIT_INSTANCE_BIND_REGIONS: Self = Self(1 << 0);
    /// Protected
    pub const PROTECTED: Self = Self(1 << 1);
    /// Mutable format
    pub const MUTABLE_FORMAT: Self = Self(1 << 2);
    /// Deferred memory allocation
    pub const DEFERRED_MEMORY_ALLOCATION: Self = Self(1 << 3);

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

/// Sharing mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SharingMode {
    /// Exclusive to one queue family
    #[default]
    Exclusive  = 0,
    /// Concurrent across queue families
    Concurrent = 1,
}

// ============================================================================
// Present Info
// ============================================================================

/// Present info
#[derive(Clone, Debug)]
#[repr(C)]
pub struct PresentInfo<'a> {
    /// Wait semaphores
    pub wait_semaphores: &'a [u64],
    /// Swapchains
    pub swapchains: &'a [SwapchainHandle],
    /// Image indices
    pub image_indices: &'a [u32],
    /// Results (optional)
    pub results: Option<&'a mut [PresentResult]>,
}

impl<'a> PresentInfo<'a> {
    /// Creates new info
    #[inline]
    pub const fn new(
        swapchains: &'a [SwapchainHandle],
        image_indices: &'a [u32],
        wait_semaphores: &'a [u64],
    ) -> Self {
        Self {
            wait_semaphores,
            swapchains,
            image_indices,
            results: None,
        }
    }

    /// Single swapchain present
    #[inline]
    pub const fn single(
        swapchain: &'a [SwapchainHandle],
        image_index: &'a [u32],
        wait: &'a [u64],
    ) -> Self {
        Self::new(swapchain, image_index, wait)
    }
}

impl Default for PresentInfo<'_> {
    fn default() -> Self {
        Self {
            wait_semaphores: &[],
            swapchains: &[],
            image_indices: &[],
            results: None,
        }
    }
}

/// Present result
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum PresentResult {
    /// Success
    Success     = 0,
    /// Suboptimal
    Suboptimal  = 1000001003,
    /// Out of date
    OutOfDate   = -1000001004,
    /// Surface lost
    SurfaceLost = -1000000000,
    /// Full screen exclusive lost
    FullScreenExclusiveLost = -1000255000,
}

impl PresentResult {
    /// Is success
    #[inline]
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Success | Self::Suboptimal)
    }

    /// Needs recreation
    #[inline]
    pub const fn needs_recreation(&self) -> bool {
        matches!(self, Self::OutOfDate | Self::Suboptimal)
    }
}

impl Default for PresentResult {
    fn default() -> Self {
        Self::Success
    }
}

// ============================================================================
// Acquire Image Info
// ============================================================================

/// Acquire next image info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct AcquireNextImageInfo {
    /// Swapchain
    pub swapchain: SwapchainHandle,
    /// Timeout in nanoseconds
    pub timeout: u64,
    /// Semaphore to signal
    pub semaphore: u64,
    /// Fence to signal
    pub fence: u64,
    /// Device mask
    pub device_mask: u32,
}

impl AcquireNextImageInfo {
    /// Infinite timeout
    pub const INFINITE: u64 = u64::MAX;

    /// Creates new info
    #[inline]
    pub const fn new(swapchain: SwapchainHandle) -> Self {
        Self {
            swapchain,
            timeout: Self::INFINITE,
            semaphore: 0,
            fence: 0,
            device_mask: 0,
        }
    }

    /// With timeout
    #[inline]
    pub const fn with_timeout(mut self, timeout_ns: u64) -> Self {
        self.timeout = timeout_ns;
        self
    }

    /// With semaphore
    #[inline]
    pub const fn with_semaphore(mut self, semaphore: u64) -> Self {
        self.semaphore = semaphore;
        self
    }

    /// With fence
    #[inline]
    pub const fn with_fence(mut self, fence: u64) -> Self {
        self.fence = fence;
        self
    }
}

impl Default for AcquireNextImageInfo {
    fn default() -> Self {
        Self::new(SwapchainHandle::NULL)
    }
}

/// Acquire result
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct AcquireResult {
    /// Image index
    pub image_index: u32,
    /// Status
    pub status: AcquireStatus,
}

/// Acquire status
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum AcquireStatus {
    /// Success
    Success     = 0,
    /// Timeout
    Timeout     = 2,
    /// Not ready
    NotReady    = 1,
    /// Suboptimal
    Suboptimal  = 1000001003,
    /// Out of date
    OutOfDate   = -1000001004,
    /// Surface lost
    SurfaceLost = -1000000000,
    /// Full screen exclusive lost
    FullScreenExclusiveLost = -1000255000,
}

impl AcquireStatus {
    /// Is success
    #[inline]
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Success | Self::Suboptimal)
    }

    /// Needs recreation
    #[inline]
    pub const fn needs_recreation(&self) -> bool {
        matches!(self, Self::OutOfDate | Self::Suboptimal)
    }

    /// Is not available yet
    #[inline]
    pub const fn is_not_available(&self) -> bool {
        matches!(self, Self::Timeout | Self::NotReady)
    }
}

impl Default for AcquireStatus {
    fn default() -> Self {
        Self::Success
    }
}

// ============================================================================
// Full Screen Exclusive
// ============================================================================

/// Full screen exclusive mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FullScreenExclusive {
    /// Default behavior
    #[default]
    Default    = 0,
    /// Allowed
    Allowed    = 1,
    /// Disallowed
    Disallowed = 2,
    /// Application controlled
    ApplicationControlled = 3,
}

impl FullScreenExclusive {
    /// Name
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Allowed => "Allowed",
            Self::Disallowed => "Disallowed",
            Self::ApplicationControlled => "Application Controlled",
        }
    }
}

// ============================================================================
// Display Mode
// ============================================================================

/// Display mode handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DisplayModeHandle(pub u64);

impl DisplayModeHandle {
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

impl Default for DisplayModeHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Display mode properties
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DisplayModeProperties {
    /// Display mode handle
    pub display_mode: DisplayModeHandle,
    /// Visible region
    pub visible_region: Extent2D,
    /// Refresh rate in millihertz
    pub refresh_rate: u32,
}

impl DisplayModeProperties {
    /// Refresh rate in Hz
    #[inline]
    pub const fn refresh_rate_hz(&self) -> f32 {
        self.refresh_rate as f32 / 1000.0
    }
}
