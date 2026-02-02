//! GPU Texture types and operations
//!
//! This module provides typed GPU textures with automatic format handling
//! and layout transitions.

use alloc::vec::Vec;
use core::marker::PhantomData;

use crate::error::{Error, Result};
use crate::types::TextureHandle;

/// Texture format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextureFormat {
    // 8-bit formats
    /// Single channel 8-bit unsigned normalized
    R8,
    /// Two channel 8-bit unsigned normalized
    Rg8,
    /// Four channel 8-bit unsigned normalized
    Rgba8,
    /// Four channel 8-bit unsigned normalized (sRGB)
    Rgba8Srgb,
    /// Four channel 8-bit unsigned normalized (BGRA order)
    Bgra8,
    /// Four channel 8-bit unsigned normalized (BGRA sRGB)
    Bgra8Srgb,

    // 16-bit formats
    /// Single channel 16-bit float
    R16F,
    /// Two channel 16-bit float
    Rg16F,
    /// Four channel 16-bit float
    Rgba16F,

    // 32-bit formats
    /// Single channel 32-bit float
    R32F,
    /// Two channel 32-bit float
    Rg32F,
    /// Four channel 32-bit float
    Rgba32F,

    // Depth/stencil formats
    /// 16-bit depth
    Depth16,
    /// 32-bit float depth
    Depth32F,
    /// 24-bit depth + 8-bit stencil
    Depth24Stencil8,
    /// 32-bit float depth + 8-bit stencil
    Depth32FStencil8,

    // Compressed formats
    /// BC1 (DXT1) RGB
    Bc1Rgb,
    /// BC1 (DXT1) RGBA
    Bc1Rgba,
    /// BC2 (DXT3) RGBA
    Bc2,
    /// BC3 (DXT5) RGBA
    Bc3,
    /// BC4 single channel
    Bc4,
    /// BC5 two channel
    Bc5,
    /// BC6H HDR
    Bc6H,
    /// BC7 high quality RGBA
    Bc7,
}

impl TextureFormat {
    /// Returns the Vulkan format constant
    pub const fn vk_format(self) -> u32 {
        match self {
            Self::R8 => 9,          // VK_FORMAT_R8_UNORM
            Self::Rg8 => 16,        // VK_FORMAT_R8G8_UNORM
            Self::Rgba8 => 37,      // VK_FORMAT_R8G8B8A8_UNORM
            Self::Rgba8Srgb => 43,  // VK_FORMAT_R8G8B8A8_SRGB
            Self::Bgra8 => 44,      // VK_FORMAT_B8G8R8A8_UNORM
            Self::Bgra8Srgb => 50,  // VK_FORMAT_B8G8R8A8_SRGB
            Self::R16F => 76,       // VK_FORMAT_R16_SFLOAT
            Self::Rg16F => 83,      // VK_FORMAT_R16G16_SFLOAT
            Self::Rgba16F => 97,    // VK_FORMAT_R16G16B16A16_SFLOAT
            Self::R32F => 100,      // VK_FORMAT_R32_SFLOAT
            Self::Rg32F => 103,     // VK_FORMAT_R32G32_SFLOAT
            Self::Rgba32F => 109,   // VK_FORMAT_R32G32B32A32_SFLOAT
            Self::Depth16 => 124,   // VK_FORMAT_D16_UNORM
            Self::Depth32F => 126,  // VK_FORMAT_D32_SFLOAT
            Self::Depth24Stencil8 => 129,   // VK_FORMAT_D24_UNORM_S8_UINT
            Self::Depth32FStencil8 => 130,  // VK_FORMAT_D32_SFLOAT_S8_UINT
            Self::Bc1Rgb => 131,    // VK_FORMAT_BC1_RGB_UNORM_BLOCK
            Self::Bc1Rgba => 133,   // VK_FORMAT_BC1_RGBA_UNORM_BLOCK
            Self::Bc2 => 135,       // VK_FORMAT_BC2_UNORM_BLOCK
            Self::Bc3 => 137,       // VK_FORMAT_BC3_UNORM_BLOCK
            Self::Bc4 => 139,       // VK_FORMAT_BC4_UNORM_BLOCK
            Self::Bc5 => 141,       // VK_FORMAT_BC5_UNORM_BLOCK
            Self::Bc6H => 143,      // VK_FORMAT_BC6H_UFLOAT_BLOCK
            Self::Bc7 => 145,       // VK_FORMAT_BC7_UNORM_BLOCK
        }
    }

    /// Returns the bytes per pixel (or block for compressed)
    pub const fn bytes_per_pixel(self) -> usize {
        match self {
            Self::R8 => 1,
            Self::Rg8 => 2,
            Self::Rgba8 | Self::Rgba8Srgb | Self::Bgra8 | Self::Bgra8Srgb => 4,
            Self::R16F => 2,
            Self::Rg16F => 4,
            Self::Rgba16F => 8,
            Self::R32F => 4,
            Self::Rg32F => 8,
            Self::Rgba32F => 16,
            Self::Depth16 => 2,
            Self::Depth32F => 4,
            Self::Depth24Stencil8 => 4,
            Self::Depth32FStencil8 => 5,
            // Compressed formats: bytes per 4x4 block
            Self::Bc1Rgb | Self::Bc1Rgba | Self::Bc4 => 8,
            Self::Bc2 | Self::Bc3 | Self::Bc5 | Self::Bc6H | Self::Bc7 => 16,
        }
    }

    /// Returns true if this is a depth format
    pub const fn is_depth(self) -> bool {
        matches!(
            self,
            Self::Depth16 | Self::Depth32F | Self::Depth24Stencil8 | Self::Depth32FStencil8
        )
    }

    /// Returns true if this is a stencil format
    pub const fn has_stencil(self) -> bool {
        matches!(self, Self::Depth24Stencil8 | Self::Depth32FStencil8)
    }

    /// Returns true if this is an sRGB format
    pub const fn is_srgb(self) -> bool {
        matches!(self, Self::Rgba8Srgb | Self::Bgra8Srgb)
    }

    /// Returns true if this is a compressed format
    pub const fn is_compressed(self) -> bool {
        matches!(
            self,
            Self::Bc1Rgb
                | Self::Bc1Rgba
                | Self::Bc2
                | Self::Bc3
                | Self::Bc4
                | Self::Bc5
                | Self::Bc6H
                | Self::Bc7
        )
    }

    /// Returns the number of channels
    pub const fn channels(self) -> usize {
        match self {
            Self::R8 | Self::R16F | Self::R32F | Self::Bc4 => 1,
            Self::Rg8 | Self::Rg16F | Self::Rg32F | Self::Bc5 => 2,
            Self::Bc1Rgb => 3,
            _ => 4,
        }
    }
}

/// Texture dimension
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureDimension {
    /// 1D texture
    D1,
    /// 2D texture
    D2,
    /// 3D texture
    D3,
    /// Cube map
    Cube,
    /// 2D array
    Array2D,
    /// Cube map array
    ArrayCube,
}

/// Texture usage flags
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureUsage(u32);

impl TextureUsage {
    /// Texture can be sampled in shaders
    pub const SAMPLED: Self = Self(0x00000004);
    /// Texture can be used as a storage image
    pub const STORAGE: Self = Self(0x00000008);
    /// Texture can be used as a color attachment
    pub const COLOR_ATTACHMENT: Self = Self(0x00000010);
    /// Texture can be used as a depth/stencil attachment
    pub const DEPTH_STENCIL_ATTACHMENT: Self = Self(0x00000020);
    /// Texture can be transferred from
    pub const TRANSFER_SRC: Self = Self(0x00000001);
    /// Texture can be transferred to
    pub const TRANSFER_DST: Self = Self(0x00000002);

    /// Returns the raw flags
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Combines two usage flags
    pub const fn and(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl core::ops::BitOr for TextureUsage {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

/// Description for creating a texture
#[derive(Clone, Debug)]
pub struct TextureDesc {
    /// Texture format
    pub format: TextureFormat,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Depth (for 3D textures) or array layers
    pub depth: u32,
    /// Number of mip levels
    pub mip_levels: u32,
    /// Sample count for multisampling
    pub samples: u32,
    /// Dimension
    pub dimension: TextureDimension,
    /// Usage flags
    pub usage: TextureUsage,
}

impl Default for TextureDesc {
    fn default() -> Self {
        Self {
            format: TextureFormat::Rgba8,
            width: 1,
            height: 1,
            depth: 1,
            mip_levels: 1,
            samples: 1,
            dimension: TextureDimension::D2,
            usage: TextureUsage::SAMPLED | TextureUsage::TRANSFER_DST,
        }
    }
}

impl TextureDesc {
    /// Creates a description for a 2D texture
    pub fn d2(format: TextureFormat, width: u32, height: u32) -> Self {
        Self {
            format,
            width,
            height,
            depth: 1,
            mip_levels: 1,
            samples: 1,
            dimension: TextureDimension::D2,
            usage: TextureUsage::SAMPLED | TextureUsage::TRANSFER_DST,
        }
    }

    /// Creates a description for a render target
    pub fn render_target(format: TextureFormat, width: u32, height: u32) -> Self {
        let usage = if format.is_depth() {
            TextureUsage::DEPTH_STENCIL_ATTACHMENT | TextureUsage::SAMPLED
        } else {
            TextureUsage::COLOR_ATTACHMENT | TextureUsage::SAMPLED
        };

        Self {
            format,
            width,
            height,
            depth: 1,
            mip_levels: 1,
            samples: 1,
            dimension: TextureDimension::D2,
            usage,
        }
    }

    /// Sets the number of mip levels
    pub fn with_mips(mut self, levels: u32) -> Self {
        self.mip_levels = levels;
        self
    }

    /// Calculates the maximum number of mip levels
    pub fn max_mip_levels(&self) -> u32 {
        (self.width.max(self.height) as f32).log2().floor() as u32 + 1
    }

    /// Enables full mip chain
    pub fn with_full_mips(mut self) -> Self {
        self.mip_levels = self.max_mip_levels();
        self
    }

    /// Sets multisampling
    pub fn with_samples(mut self, samples: u32) -> Self {
        self.samples = samples;
        self
    }
}

/// A GPU texture
pub struct GpuTexture<F = ()> {
    handle: TextureHandle,
    desc: TextureDesc,
    staging: Option<Vec<u8>>,
    _format: PhantomData<F>,
}

impl<F> GpuTexture<F> {
    /// Creates a new texture with the given description
    pub fn new(desc: TextureDesc) -> Self {
        Self {
            handle: TextureHandle::null(),
            desc,
            staging: None,
            _format: PhantomData,
        }
    }

    /// Creates a 2D texture from RGBA data
    pub fn from_rgba(width: u32, height: u32, data: &[u8]) -> GpuTexture<crate::Rgba8> {
        assert_eq!(
            data.len(),
            (width * height * 4) as usize,
            "Data size mismatch"
        );

        let desc = TextureDesc::d2(TextureFormat::Rgba8, width, height);

        GpuTexture {
            handle: TextureHandle::null(),
            desc,
            staging: Some(data.to_vec()),
            _format: PhantomData,
        }
    }

    /// Returns the texture description
    #[inline]
    pub fn desc(&self) -> &TextureDesc {
        &self.desc
    }

    /// Returns the width
    #[inline]
    pub fn width(&self) -> u32 {
        self.desc.width
    }

    /// Returns the height
    #[inline]
    pub fn height(&self) -> u32 {
        self.desc.height
    }

    /// Returns the format
    #[inline]
    pub fn format(&self) -> TextureFormat {
        self.desc.format
    }

    /// Returns the number of mip levels
    #[inline]
    pub fn mip_levels(&self) -> u32 {
        self.desc.mip_levels
    }

    /// Returns the underlying handle
    #[inline]
    pub(crate) fn handle(&self) -> TextureHandle {
        self.handle
    }

    /// Sets the underlying handle
    #[inline]
    pub(crate) fn set_handle(&mut self, handle: TextureHandle) {
        self.handle = handle;
    }

    /// Takes the staging data
    pub(crate) fn take_staging(&mut self) -> Option<Vec<u8>> {
        self.staging.take()
    }
}

/// A 2D texture view for sampling
pub struct Texture2D<'a, F> {
    texture: &'a GpuTexture<F>,
    base_mip: u32,
    mip_count: u32,
}

impl<'a, F> Texture2D<'a, F> {
    /// Creates a view of the entire texture
    pub fn new(texture: &'a GpuTexture<F>) -> Self {
        Self {
            texture,
            base_mip: 0,
            mip_count: texture.mip_levels(),
        }
    }

    /// Creates a view of a specific mip level
    pub fn mip_level(texture: &'a GpuTexture<F>, level: u32) -> Self {
        assert!(level < texture.mip_levels(), "Mip level out of range");
        Self {
            texture,
            base_mip: level,
            mip_count: 1,
        }
    }

    /// Returns the underlying handle
    #[inline]
    pub(crate) fn handle(&self) -> TextureHandle {
        self.texture.handle
    }

    /// Returns the base mip level
    #[inline]
    pub fn base_mip(&self) -> u32 {
        self.base_mip
    }

    /// Returns the mip count
    #[inline]
    pub fn mip_count(&self) -> u32 {
        self.mip_count
    }
}
