//! Image and texture types
//!
//! This module provides comprehensive types for image/texture creation and management.

use core::num::NonZeroU32;

/// Image handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ImageHandle(pub NonZeroU32);

impl ImageHandle {
    /// Creates a new handle from raw ID
    pub const fn from_raw(id: u32) -> Option<Self> {
        match NonZeroU32::new(id) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Gets the raw ID
    pub const fn raw(&self) -> u32 {
        self.0.get()
    }
}

/// Image view handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ImageViewHandle(pub NonZeroU32);

impl ImageViewHandle {
    /// Creates a new handle from raw ID
    pub const fn from_raw(id: u32) -> Option<Self> {
        match NonZeroU32::new(id) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Gets the raw ID
    pub const fn raw(&self) -> u32 {
        self.0.get()
    }
}

/// Image type
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum ImageType {
    /// 1D image
    Type1D = 0,
    /// 2D image
    #[default]
    Type2D = 1,
    /// 3D image
    Type3D = 2,
}

/// Image view type
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum ImageViewType {
    /// 1D image view
    Type1D = 0,
    /// 2D image view
    #[default]
    Type2D = 1,
    /// 3D image view
    Type3D = 2,
    /// Cube image view
    Cube = 3,
    /// 1D array image view
    Array1D = 4,
    /// 2D array image view
    Array2D = 5,
    /// Cube array image view
    CubeArray = 6,
}

impl ImageViewType {
    /// Is this a 2D view type
    pub const fn is_2d(self) -> bool {
        matches!(self, Self::Type2D | Self::Array2D)
    }

    /// Is this an array view type
    pub const fn is_array(self) -> bool {
        matches!(self, Self::Array1D | Self::Array2D | Self::CubeArray)
    }

    /// Is this a cube view type
    pub const fn is_cube(self) -> bool {
        matches!(self, Self::Cube | Self::CubeArray)
    }
}

/// Image format
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum Format {
    /// Undefined
    #[default]
    Undefined = 0,
    /// R4G4 unsigned normalized packed
    R4g4UnormPack8 = 1,
    /// R4G4B4A4 unsigned normalized packed
    R4g4b4a4UnormPack16 = 2,
    /// B4G4R4A4 unsigned normalized packed
    B4g4r4a4UnormPack16 = 3,
    /// R5G6B5 unsigned normalized packed
    R5g6b5UnormPack16 = 4,
    /// B5G6R5 unsigned normalized packed
    B5g6r5UnormPack16 = 5,
    /// R5G5B5A1 unsigned normalized packed
    R5g5b5a1UnormPack16 = 6,
    /// B5G5R5A1 unsigned normalized packed
    B5g5r5a1UnormPack16 = 7,
    /// A1R5G5B5 unsigned normalized packed
    A1r5g5b5UnormPack16 = 8,
    /// R8 unsigned normalized
    R8Unorm = 9,
    /// R8 signed normalized
    R8Snorm = 10,
    /// R8 unsigned scaled
    R8Uscaled = 11,
    /// R8 signed scaled
    R8Sscaled = 12,
    /// R8 unsigned integer
    R8Uint = 13,
    /// R8 signed integer
    R8Sint = 14,
    /// R8 sRGB
    R8Srgb = 15,
    /// R8G8 unsigned normalized
    R8g8Unorm = 16,
    /// R8G8 signed normalized
    R8g8Snorm = 17,
    /// R8G8 unsigned scaled
    R8g8Uscaled = 18,
    /// R8G8 signed scaled
    R8g8Sscaled = 19,
    /// R8G8 unsigned integer
    R8g8Uint = 20,
    /// R8G8 signed integer
    R8g8Sint = 21,
    /// R8G8 sRGB
    R8g8Srgb = 22,
    /// R8G8B8 unsigned normalized
    R8g8b8Unorm = 23,
    /// R8G8B8 signed normalized
    R8g8b8Snorm = 24,
    /// R8G8B8 unsigned scaled
    R8g8b8Uscaled = 25,
    /// R8G8B8 signed scaled
    R8g8b8Sscaled = 26,
    /// R8G8B8 unsigned integer
    R8g8b8Uint = 27,
    /// R8G8B8 signed integer
    R8g8b8Sint = 28,
    /// R8G8B8 sRGB
    R8g8b8Srgb = 29,
    /// B8G8R8 unsigned normalized
    B8g8r8Unorm = 30,
    /// B8G8R8 signed normalized
    B8g8r8Snorm = 31,
    /// B8G8R8 unsigned scaled
    B8g8r8Uscaled = 32,
    /// B8G8R8 signed scaled
    B8g8r8Sscaled = 33,
    /// B8G8R8 unsigned integer
    B8g8r8Uint = 34,
    /// B8G8R8 signed integer
    B8g8r8Sint = 35,
    /// B8G8R8 sRGB
    B8g8r8Srgb = 36,
    /// R8G8B8A8 unsigned normalized
    R8g8b8a8Unorm = 37,
    /// R8G8B8A8 signed normalized
    R8g8b8a8Snorm = 38,
    /// R8G8B8A8 unsigned scaled
    R8g8b8a8Uscaled = 39,
    /// R8G8B8A8 signed scaled
    R8g8b8a8Sscaled = 40,
    /// R8G8B8A8 unsigned integer
    R8g8b8a8Uint = 41,
    /// R8G8B8A8 signed integer
    R8g8b8a8Sint = 42,
    /// R8G8B8A8 sRGB
    R8g8b8a8Srgb = 43,
    /// B8G8R8A8 unsigned normalized
    B8g8r8a8Unorm = 44,
    /// B8G8R8A8 signed normalized
    B8g8r8a8Snorm = 45,
    /// B8G8R8A8 unsigned scaled
    B8g8r8a8Uscaled = 46,
    /// B8G8R8A8 signed scaled
    B8g8r8a8Sscaled = 47,
    /// B8G8R8A8 unsigned integer
    B8g8r8a8Uint = 48,
    /// B8G8R8A8 signed integer
    B8g8r8a8Sint = 49,
    /// B8G8R8A8 sRGB
    B8g8r8a8Srgb = 50,
    /// R16 unsigned normalized
    R16Unorm = 70,
    /// R16 signed normalized
    R16Snorm = 71,
    /// R16 unsigned scaled
    R16Uscaled = 72,
    /// R16 signed scaled
    R16Sscaled = 73,
    /// R16 unsigned integer
    R16Uint = 74,
    /// R16 signed integer
    R16Sint = 75,
    /// R16 signed float
    R16Sfloat = 76,
    /// R16G16 unsigned normalized
    R16g16Unorm = 77,
    /// R16G16 signed normalized
    R16g16Snorm = 78,
    /// R16G16 unsigned scaled
    R16g16Uscaled = 79,
    /// R16G16 signed scaled
    R16g16Sscaled = 80,
    /// R16G16 unsigned integer
    R16g16Uint = 81,
    /// R16G16 signed integer
    R16g16Sint = 82,
    /// R16G16 signed float
    R16g16Sfloat = 83,
    /// R16G16B16 unsigned normalized
    R16g16b16Unorm = 84,
    /// R16G16B16 signed normalized
    R16g16b16Snorm = 85,
    /// R16G16B16 unsigned scaled
    R16g16b16Uscaled = 86,
    /// R16G16B16 signed scaled
    R16g16b16Sscaled = 87,
    /// R16G16B16 unsigned integer
    R16g16b16Uint = 88,
    /// R16G16B16 signed integer
    R16g16b16Sint = 89,
    /// R16G16B16 signed float
    R16g16b16Sfloat = 90,
    /// R16G16B16A16 unsigned normalized
    R16g16b16a16Unorm = 91,
    /// R16G16B16A16 signed normalized
    R16g16b16a16Snorm = 92,
    /// R16G16B16A16 unsigned scaled
    R16g16b16a16Uscaled = 93,
    /// R16G16B16A16 signed scaled
    R16g16b16a16Sscaled = 94,
    /// R16G16B16A16 unsigned integer
    R16g16b16a16Uint = 95,
    /// R16G16B16A16 signed integer
    R16g16b16a16Sint = 96,
    /// R16G16B16A16 signed float
    R16g16b16a16Sfloat = 97,
    /// R32 unsigned integer
    R32Uint = 98,
    /// R32 signed integer
    R32Sint = 99,
    /// R32 signed float
    R32Sfloat = 100,
    /// R32G32 unsigned integer
    R32g32Uint = 101,
    /// R32G32 signed integer
    R32g32Sint = 102,
    /// R32G32 signed float
    R32g32Sfloat = 103,
    /// R32G32B32 unsigned integer
    R32g32b32Uint = 104,
    /// R32G32B32 signed integer
    R32g32b32Sint = 105,
    /// R32G32B32 signed float
    R32g32b32Sfloat = 106,
    /// R32G32B32A32 unsigned integer
    R32g32b32a32Uint = 107,
    /// R32G32B32A32 signed integer
    R32g32b32a32Sint = 108,
    /// R32G32B32A32 signed float
    R32g32b32a32Sfloat = 109,
    /// R64 unsigned integer
    R64Uint = 110,
    /// R64 signed integer
    R64Sint = 111,
    /// R64 signed float
    R64Sfloat = 112,
    /// R64G64 unsigned integer
    R64g64Uint = 113,
    /// R64G64 signed integer
    R64g64Sint = 114,
    /// R64G64 signed float
    R64g64Sfloat = 115,
    /// R64G64B64 unsigned integer
    R64g64b64Uint = 116,
    /// R64G64B64 signed integer
    R64g64b64Sint = 117,
    /// R64G64B64 signed float
    R64g64b64Sfloat = 118,
    /// R64G64B64A64 unsigned integer
    R64g64b64a64Uint = 119,
    /// R64G64B64A64 signed integer
    R64g64b64a64Sint = 120,
    /// R64G64B64A64 signed float
    R64g64b64a64Sfloat = 121,
    /// D16 unsigned normalized depth
    D16Unorm = 124,
    /// D32 signed float depth
    D32Sfloat = 126,
    /// S8 unsigned integer stencil
    S8Uint = 127,
    /// D16 unsigned normalized depth, S8 unsigned integer stencil
    D16UnormS8Uint = 128,
    /// D24 unsigned normalized depth, S8 unsigned integer stencil
    D24UnormS8Uint = 129,
    /// D32 signed float depth, S8 unsigned integer stencil
    D32SfloatS8Uint = 130,
    /// BC1 RGB unsigned normalized
    Bc1RgbUnormBlock = 131,
    /// BC1 RGB sRGB
    Bc1RgbSrgbBlock = 132,
    /// BC1 RGBA unsigned normalized
    Bc1RgbaUnormBlock = 133,
    /// BC1 RGBA sRGB
    Bc1RgbaSrgbBlock = 134,
    /// BC2 unsigned normalized
    Bc2UnormBlock = 135,
    /// BC2 sRGB
    Bc2SrgbBlock = 136,
    /// BC3 unsigned normalized
    Bc3UnormBlock = 137,
    /// BC3 sRGB
    Bc3SrgbBlock = 138,
    /// BC4 unsigned normalized
    Bc4UnormBlock = 139,
    /// BC4 signed normalized
    Bc4SnormBlock = 140,
    /// BC5 unsigned normalized
    Bc5UnormBlock = 141,
    /// BC5 signed normalized
    Bc5SnormBlock = 142,
    /// BC6H unsigned float
    Bc6hUfloatBlock = 143,
    /// BC6H signed float
    Bc6hSfloatBlock = 144,
    /// BC7 unsigned normalized
    Bc7UnormBlock = 145,
    /// BC7 sRGB
    Bc7SrgbBlock = 146,
}

impl Format {
    /// Is depth format
    pub const fn is_depth(self) -> bool {
        matches!(
            self,
            Self::D16Unorm
                | Self::D32Sfloat
                | Self::D16UnormS8Uint
                | Self::D24UnormS8Uint
                | Self::D32SfloatS8Uint
        )
    }

    /// Is stencil format
    pub const fn is_stencil(self) -> bool {
        matches!(
            self,
            Self::S8Uint | Self::D16UnormS8Uint | Self::D24UnormS8Uint | Self::D32SfloatS8Uint
        )
    }

    /// Is depth-stencil format
    pub const fn is_depth_stencil(self) -> bool {
        self.is_depth() && self.is_stencil()
    }

    /// Is color format
    pub const fn is_color(self) -> bool {
        !self.is_depth() && !self.is_stencil() && !matches!(self, Self::Undefined)
    }

    /// Is compressed format
    pub const fn is_compressed(self) -> bool {
        matches!(
            self,
            Self::Bc1RgbUnormBlock
                | Self::Bc1RgbSrgbBlock
                | Self::Bc1RgbaUnormBlock
                | Self::Bc1RgbaSrgbBlock
                | Self::Bc2UnormBlock
                | Self::Bc2SrgbBlock
                | Self::Bc3UnormBlock
                | Self::Bc3SrgbBlock
                | Self::Bc4UnormBlock
                | Self::Bc4SnormBlock
                | Self::Bc5UnormBlock
                | Self::Bc5SnormBlock
                | Self::Bc6hUfloatBlock
                | Self::Bc6hSfloatBlock
                | Self::Bc7UnormBlock
                | Self::Bc7SrgbBlock
        )
    }

    /// Is sRGB format
    pub const fn is_srgb(self) -> bool {
        matches!(
            self,
            Self::R8Srgb
                | Self::R8g8Srgb
                | Self::R8g8b8Srgb
                | Self::B8g8r8Srgb
                | Self::R8g8b8a8Srgb
                | Self::B8g8r8a8Srgb
                | Self::Bc1RgbSrgbBlock
                | Self::Bc1RgbaSrgbBlock
                | Self::Bc2SrgbBlock
                | Self::Bc3SrgbBlock
                | Self::Bc7SrgbBlock
        )
    }

    /// Bytes per pixel (or block for compressed)
    pub const fn bytes_per_pixel(self) -> u32 {
        match self {
            Self::Undefined => 0,
            Self::R4g4UnormPack8 => 1,
            Self::R4g4b4a4UnormPack16 | Self::B4g4r4a4UnormPack16 => 2,
            Self::R5g6b5UnormPack16
            | Self::B5g6r5UnormPack16
            | Self::R5g5b5a1UnormPack16
            | Self::B5g5r5a1UnormPack16
            | Self::A1r5g5b5UnormPack16 => 2,
            Self::R8Unorm
            | Self::R8Snorm
            | Self::R8Uscaled
            | Self::R8Sscaled
            | Self::R8Uint
            | Self::R8Sint
            | Self::R8Srgb => 1,
            Self::R8g8Unorm
            | Self::R8g8Snorm
            | Self::R8g8Uscaled
            | Self::R8g8Sscaled
            | Self::R8g8Uint
            | Self::R8g8Sint
            | Self::R8g8Srgb => 2,
            Self::R8g8b8Unorm
            | Self::R8g8b8Snorm
            | Self::R8g8b8Uscaled
            | Self::R8g8b8Sscaled
            | Self::R8g8b8Uint
            | Self::R8g8b8Sint
            | Self::R8g8b8Srgb => 3,
            Self::B8g8r8Unorm
            | Self::B8g8r8Snorm
            | Self::B8g8r8Uscaled
            | Self::B8g8r8Sscaled
            | Self::B8g8r8Uint
            | Self::B8g8r8Sint
            | Self::B8g8r8Srgb => 3,
            Self::R8g8b8a8Unorm
            | Self::R8g8b8a8Snorm
            | Self::R8g8b8a8Uscaled
            | Self::R8g8b8a8Sscaled
            | Self::R8g8b8a8Uint
            | Self::R8g8b8a8Sint
            | Self::R8g8b8a8Srgb => 4,
            Self::B8g8r8a8Unorm
            | Self::B8g8r8a8Snorm
            | Self::B8g8r8a8Uscaled
            | Self::B8g8r8a8Sscaled
            | Self::B8g8r8a8Uint
            | Self::B8g8r8a8Sint
            | Self::B8g8r8a8Srgb => 4,
            Self::R16Unorm
            | Self::R16Snorm
            | Self::R16Uscaled
            | Self::R16Sscaled
            | Self::R16Uint
            | Self::R16Sint
            | Self::R16Sfloat => 2,
            Self::R16g16Unorm
            | Self::R16g16Snorm
            | Self::R16g16Uscaled
            | Self::R16g16Sscaled
            | Self::R16g16Uint
            | Self::R16g16Sint
            | Self::R16g16Sfloat => 4,
            Self::R16g16b16Unorm
            | Self::R16g16b16Snorm
            | Self::R16g16b16Uscaled
            | Self::R16g16b16Sscaled
            | Self::R16g16b16Uint
            | Self::R16g16b16Sint
            | Self::R16g16b16Sfloat => 6,
            Self::R16g16b16a16Unorm
            | Self::R16g16b16a16Snorm
            | Self::R16g16b16a16Uscaled
            | Self::R16g16b16a16Sscaled
            | Self::R16g16b16a16Uint
            | Self::R16g16b16a16Sint
            | Self::R16g16b16a16Sfloat => 8,
            Self::R32Uint | Self::R32Sint | Self::R32Sfloat => 4,
            Self::R32g32Uint | Self::R32g32Sint | Self::R32g32Sfloat => 8,
            Self::R32g32b32Uint | Self::R32g32b32Sint | Self::R32g32b32Sfloat => 12,
            Self::R32g32b32a32Uint | Self::R32g32b32a32Sint | Self::R32g32b32a32Sfloat => 16,
            Self::R64Uint | Self::R64Sint | Self::R64Sfloat => 8,
            Self::R64g64Uint | Self::R64g64Sint | Self::R64g64Sfloat => 16,
            Self::R64g64b64Uint | Self::R64g64b64Sint | Self::R64g64b64Sfloat => 24,
            Self::R64g64b64a64Uint | Self::R64g64b64a64Sint | Self::R64g64b64a64Sfloat => 32,
            Self::D16Unorm => 2,
            Self::D32Sfloat => 4,
            Self::S8Uint => 1,
            Self::D16UnormS8Uint => 3,
            Self::D24UnormS8Uint => 4,
            Self::D32SfloatS8Uint => 5,
            // Compressed formats (bytes per block)
            Self::Bc1RgbUnormBlock
            | Self::Bc1RgbSrgbBlock
            | Self::Bc1RgbaUnormBlock
            | Self::Bc1RgbaSrgbBlock => 8,
            Self::Bc2UnormBlock | Self::Bc2SrgbBlock => 16,
            Self::Bc3UnormBlock | Self::Bc3SrgbBlock => 16,
            Self::Bc4UnormBlock | Self::Bc4SnormBlock => 8,
            Self::Bc5UnormBlock | Self::Bc5SnormBlock => 16,
            Self::Bc6hUfloatBlock | Self::Bc6hSfloatBlock => 16,
            Self::Bc7UnormBlock | Self::Bc7SrgbBlock => 16,
        }
    }

    /// Block size for compressed formats (1 for uncompressed)
    pub const fn block_size(self) -> (u32, u32) {
        if self.is_compressed() {
            (4, 4)
        } else {
            (1, 1)
        }
    }
}

/// Image tiling
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum ImageTiling {
    /// Optimal tiling (implementation-defined)
    #[default]
    Optimal = 0,
    /// Linear tiling (row-major)
    Linear = 1,
}

/// Image creation info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ImageCreateInfo {
    /// Image type
    pub image_type: ImageType,
    /// Format
    pub format: Format,
    /// Extent
    pub extent: Extent3D,
    /// Mip levels
    pub mip_levels: u32,
    /// Array layers
    pub array_layers: u32,
    /// Sample count
    pub samples: SampleCount,
    /// Tiling
    pub tiling: ImageTiling,
    /// Usage flags
    pub usage: ImageUsageFlags,
    /// Sharing mode
    pub sharing_mode: SharingMode,
    /// Initial layout
    pub initial_layout: ImageLayout,
    /// Creation flags
    pub flags: ImageCreateFlags,
}

/// 3D extent
#[derive(Clone, Copy, Debug, Default)]
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
    /// Creates a 2D extent (depth = 1)
    pub const fn d2(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            depth: 1,
        }
    }

    /// Creates a 3D extent
    pub const fn d3(width: u32, height: u32, depth: u32) -> Self {
        Self {
            width,
            height,
            depth,
        }
    }

    /// Total number of texels
    pub const fn texel_count(&self) -> u64 {
        self.width as u64 * self.height as u64 * self.depth as u64
    }
}

/// Sample count
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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

/// Sharing mode
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum SharingMode {
    /// Exclusive access
    #[default]
    Exclusive = 0,
    /// Concurrent access
    Concurrent = 1,
}

/// Image layout
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum ImageLayout {
    /// Undefined layout
    #[default]
    Undefined = 0,
    /// General layout
    General = 1,
    /// Color attachment optimal
    ColorAttachmentOptimal = 2,
    /// Depth-stencil attachment optimal
    DepthStencilAttachmentOptimal = 3,
    /// Depth-stencil read-only optimal
    DepthStencilReadOnlyOptimal = 4,
    /// Shader read-only optimal
    ShaderReadOnlyOptimal = 5,
    /// Transfer source optimal
    TransferSrcOptimal = 6,
    /// Transfer destination optimal
    TransferDstOptimal = 7,
    /// Preinitialized
    Preinitialized = 8,
    /// Present source
    PresentSrc = 1000001002,
}

bitflags::bitflags! {
    /// Image usage flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct ImageUsageFlags: u32 {
        /// Transfer source
        const TRANSFER_SRC = 1 << 0;
        /// Transfer destination
        const TRANSFER_DST = 1 << 1;
        /// Sampled image
        const SAMPLED = 1 << 2;
        /// Storage image
        const STORAGE = 1 << 3;
        /// Color attachment
        const COLOR_ATTACHMENT = 1 << 4;
        /// Depth-stencil attachment
        const DEPTH_STENCIL_ATTACHMENT = 1 << 5;
        /// Transient attachment
        const TRANSIENT_ATTACHMENT = 1 << 6;
        /// Input attachment
        const INPUT_ATTACHMENT = 1 << 7;
        /// Fragment shading rate attachment
        const FRAGMENT_SHADING_RATE_ATTACHMENT = 1 << 8;
    }
}

impl ImageUsageFlags {
    /// Typical texture usage
    pub const TEXTURE: Self = Self::from_bits_truncate(
        Self::TRANSFER_DST.bits() | Self::SAMPLED.bits()
    );

    /// Render target usage
    pub const RENDER_TARGET: Self = Self::from_bits_truncate(
        Self::COLOR_ATTACHMENT.bits() | Self::SAMPLED.bits()
    );

    /// Depth buffer usage
    pub const DEPTH_BUFFER: Self = Self::DEPTH_STENCIL_ATTACHMENT;

    /// Storage image usage
    pub const STORAGE_IMAGE: Self = Self::from_bits_truncate(
        Self::STORAGE.bits() | Self::SAMPLED.bits()
    );
}

bitflags::bitflags! {
    /// Image creation flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct ImageCreateFlags: u32 {
        /// Sparse binding
        const SPARSE_BINDING = 1 << 0;
        /// Sparse residency
        const SPARSE_RESIDENCY = 1 << 1;
        /// Sparse aliased
        const SPARSE_ALIASED = 1 << 2;
        /// Mutable format
        const MUTABLE_FORMAT = 1 << 3;
        /// Cube compatible
        const CUBE_COMPATIBLE = 1 << 4;
        /// 2D array compatible
        const ARRAY_2D_COMPATIBLE = 1 << 5;
        /// Block texel view compatible
        const BLOCK_TEXEL_VIEW_COMPATIBLE = 1 << 6;
    }
}

impl ImageCreateFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}

impl ImageCreateInfo {
    /// Creates info for a 2D texture
    pub const fn texture_2d(format: Format, width: u32, height: u32) -> Self {
        Self {
            image_type: ImageType::Type2D,
            format,
            extent: Extent3D::d2(width, height),
            mip_levels: 1,
            array_layers: 1,
            samples: SampleCount::S1,
            tiling: ImageTiling::Optimal,
            usage: ImageUsageFlags::TEXTURE,
            sharing_mode: SharingMode::Exclusive,
            initial_layout: ImageLayout::Undefined,
            flags: ImageCreateFlags::empty(),
        }
    }

    /// Creates info for a render target
    pub const fn render_target(format: Format, width: u32, height: u32) -> Self {
        Self {
            image_type: ImageType::Type2D,
            format,
            extent: Extent3D::d2(width, height),
            mip_levels: 1,
            array_layers: 1,
            samples: SampleCount::S1,
            tiling: ImageTiling::Optimal,
            usage: ImageUsageFlags::RENDER_TARGET,
            sharing_mode: SharingMode::Exclusive,
            initial_layout: ImageLayout::Undefined,
            flags: ImageCreateFlags::empty(),
        }
    }

    /// Creates info for a depth buffer
    pub const fn depth_buffer(format: Format, width: u32, height: u32) -> Self {
        Self {
            image_type: ImageType::Type2D,
            format,
            extent: Extent3D::d2(width, height),
            mip_levels: 1,
            array_layers: 1,
            samples: SampleCount::S1,
            tiling: ImageTiling::Optimal,
            usage: ImageUsageFlags::DEPTH_BUFFER,
            sharing_mode: SharingMode::Exclusive,
            initial_layout: ImageLayout::Undefined,
            flags: ImageCreateFlags::empty(),
        }
    }

    /// Creates info for a cube map
    pub const fn cube_map(format: Format, size: u32) -> Self {
        Self {
            image_type: ImageType::Type2D,
            format,
            extent: Extent3D::d2(size, size),
            mip_levels: 1,
            array_layers: 6,
            samples: SampleCount::S1,
            tiling: ImageTiling::Optimal,
            usage: ImageUsageFlags::TEXTURE,
            sharing_mode: SharingMode::Exclusive,
            initial_layout: ImageLayout::Undefined,
            flags: ImageCreateFlags::CUBE_COMPATIBLE,
        }
    }

    /// With mip levels
    pub const fn with_mip_levels(mut self, levels: u32) -> Self {
        self.mip_levels = levels;
        self
    }

    /// With multisampling
    pub const fn with_samples(mut self, samples: SampleCount) -> Self {
        self.samples = samples;
        self
    }

    /// Calculate maximum mip levels
    pub const fn max_mip_levels(&self) -> u32 {
        let max_dim = if self.extent.width > self.extent.height {
            self.extent.width
        } else {
            self.extent.height
        };
        32 - max_dim.leading_zeros()
    }
}

/// Image view creation info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ImageViewCreateInfo {
    /// Image to create view of
    pub image: ImageHandle,
    /// View type
    pub view_type: ImageViewType,
    /// Format
    pub format: Format,
    /// Component swizzle
    pub components: ComponentMapping,
    /// Subresource range
    pub subresource_range: ImageSubresourceRange,
    /// Creation flags
    pub flags: ImageViewCreateFlags,
}

/// Component swizzle
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum ComponentSwizzle {
    /// Identity
    #[default]
    Identity = 0,
    /// Zero
    Zero = 1,
    /// One
    One = 2,
    /// Red channel
    R = 3,
    /// Green channel
    G = 4,
    /// Blue channel
    B = 5,
    /// Alpha channel
    A = 6,
}

/// Component mapping
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ComponentMapping {
    /// Red component swizzle
    pub r: ComponentSwizzle,
    /// Green component swizzle
    pub g: ComponentSwizzle,
    /// Blue component swizzle
    pub b: ComponentSwizzle,
    /// Alpha component swizzle
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
}

/// Image subresource range
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ImageSubresourceRange {
    /// Aspect mask
    pub aspect_mask: ImageAspectFlags,
    /// Base mip level
    pub base_mip_level: u32,
    /// Mip level count
    pub level_count: u32,
    /// Base array layer
    pub base_array_layer: u32,
    /// Layer count
    pub layer_count: u32,
}

impl ImageSubresourceRange {
    /// All color subresources
    pub const fn all_color() -> Self {
        Self {
            aspect_mask: ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: u32::MAX,
            base_array_layer: 0,
            layer_count: u32::MAX,
        }
    }

    /// All depth subresources
    pub const fn all_depth() -> Self {
        Self {
            aspect_mask: ImageAspectFlags::DEPTH,
            base_mip_level: 0,
            level_count: u32::MAX,
            base_array_layer: 0,
            layer_count: u32::MAX,
        }
    }
}

bitflags::bitflags! {
    /// Image aspect flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct ImageAspectFlags: u32 {
        /// Color aspect
        const COLOR = 1 << 0;
        /// Depth aspect
        const DEPTH = 1 << 1;
        /// Stencil aspect
        const STENCIL = 1 << 2;
        /// Metadata aspect
        const METADATA = 1 << 3;
    }
}

impl ImageAspectFlags {
    /// Depth and stencil
    pub const DEPTH_STENCIL: Self = Self::from_bits_truncate(Self::DEPTH.bits() | Self::STENCIL.bits());
}

bitflags::bitflags! {
    /// Image view creation flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct ImageViewCreateFlags: u32 {
        /// Fragment density map dynamic
        const FRAGMENT_DENSITY_MAP_DYNAMIC = 1 << 0;
    }
}

impl ImageViewCreateFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}

impl ImageViewCreateInfo {
    /// Creates a 2D view of a 2D image
    pub const fn d2(image: ImageHandle, format: Format) -> Self {
        Self {
            image,
            view_type: ImageViewType::Type2D,
            format,
            components: ComponentMapping::IDENTITY,
            subresource_range: ImageSubresourceRange::all_color(),
            flags: ImageViewCreateFlags::empty(),
        }
    }

    /// Creates a cube view
    pub const fn cube(image: ImageHandle, format: Format) -> Self {
        Self {
            image,
            view_type: ImageViewType::Cube,
            format,
            components: ComponentMapping::IDENTITY,
            subresource_range: ImageSubresourceRange::all_color(),
            flags: ImageViewCreateFlags::empty(),
        }
    }
}
