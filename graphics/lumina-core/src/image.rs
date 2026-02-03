//! Image and texture operations
//!
//! This module provides types for image manipulation and blitting.

extern crate alloc;
use alloc::vec::Vec;

use crate::types::{TextureFormat, TextureHandle, TextureViewHandle};
use crate::framebuffer::ImageLayout;

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
    /// Number of layers
    pub layer_count: u32,
}

impl ImageSubresourceLayers {
    /// Creates layers for color
    pub const fn color(mip_level: u32) -> Self {
        Self {
            aspect_mask: ImageAspectFlags::COLOR,
            mip_level,
            base_array_layer: 0,
            layer_count: 1,
        }
    }

    /// Creates layers for depth
    pub const fn depth(mip_level: u32) -> Self {
        Self {
            aspect_mask: ImageAspectFlags::DEPTH,
            mip_level,
            base_array_layer: 0,
            layer_count: 1,
        }
    }

    /// Creates layers for all mip 0
    pub const fn all_mip0() -> Self {
        Self {
            aspect_mask: ImageAspectFlags::COLOR,
            mip_level: 0,
            base_array_layer: 0,
            layer_count: u32::MAX, // All layers
        }
    }
}

/// Image aspect flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ImageAspectFlags(pub u32);

impl ImageAspectFlags {
    /// No aspects
    pub const NONE: Self = Self(0);
    /// Color aspect
    pub const COLOR: Self = Self(1 << 0);
    /// Depth aspect
    pub const DEPTH: Self = Self(1 << 1);
    /// Stencil aspect
    pub const STENCIL: Self = Self(1 << 2);
    /// Metadata aspect
    pub const METADATA: Self = Self(1 << 3);
    /// Depth and stencil
    pub const DEPTH_STENCIL: Self = Self(Self::DEPTH.0 | Self::STENCIL.0);

    /// Checks if an aspect is set
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl core::ops::BitOr for ImageAspectFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Image subresource range
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ImageSubresourceRange {
    /// Aspect mask
    pub aspect_mask: ImageAspectFlags,
    /// Base mip level
    pub base_mip_level: u32,
    /// Number of mip levels
    pub level_count: u32,
    /// Base array layer
    pub base_array_layer: u32,
    /// Number of array layers
    pub layer_count: u32,
}

impl ImageSubresourceRange {
    /// Creates a range for all subresources
    pub const fn all(aspect_mask: ImageAspectFlags) -> Self {
        Self {
            aspect_mask,
            base_mip_level: 0,
            level_count: u32::MAX,
            base_array_layer: 0,
            layer_count: u32::MAX,
        }
    }

    /// Creates a range for color
    pub const fn color() -> Self {
        Self::all(ImageAspectFlags::COLOR)
    }

    /// Creates a range for depth
    pub const fn depth() -> Self {
        Self::all(ImageAspectFlags::DEPTH)
    }

    /// Creates a range for a single mip level
    pub const fn single_mip(aspect_mask: ImageAspectFlags, mip_level: u32) -> Self {
        Self {
            aspect_mask,
            base_mip_level: mip_level,
            level_count: 1,
            base_array_layer: 0,
            layer_count: u32::MAX,
        }
    }
}

/// 3D offset
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Offset3D {
    /// X offset
    pub x: i32,
    /// Y offset
    pub y: i32,
    /// Z offset
    pub z: i32,
}

impl Offset3D {
    /// Zero offset
    pub const ZERO: Self = Self { x: 0, y: 0, z: 0 };

    /// Creates a new offset
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// Creates a 2D offset (z = 0)
    pub const fn xy(x: i32, y: i32) -> Self {
        Self { x, y, z: 0 }
    }
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
    /// Creates a new extent
    pub const fn new(width: u32, height: u32, depth: u32) -> Self {
        Self {
            width,
            height,
            depth,
        }
    }

    /// Creates a 2D extent (depth = 1)
    pub const fn d2(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            depth: 1,
        }
    }

    /// Creates a 1D extent (height = 1, depth = 1)
    pub const fn d1(width: u32) -> Self {
        Self {
            width,
            height: 1,
            depth: 1,
        }
    }

    /// Calculates the mip level extent
    pub const fn mip_level(&self, level: u32) -> Self {
        Self {
            width: (self.width >> level).max(1),
            height: (self.height >> level).max(1),
            depth: (self.depth >> level).max(1),
        }
    }

    /// Total number of texels
    pub const fn texel_count(&self) -> u64 {
        self.width as u64 * self.height as u64 * self.depth as u64
    }
}

/// Image copy region
#[derive(Clone, Copy, Debug)]
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
    /// Extent to copy
    pub extent: Extent3D,
}

impl ImageCopy {
    /// Creates a simple 2D copy
    pub fn d2(src_mip: u32, dst_mip: u32, width: u32, height: u32) -> Self {
        Self {
            src_subresource: ImageSubresourceLayers::color(src_mip),
            src_offset: Offset3D::ZERO,
            dst_subresource: ImageSubresourceLayers::color(dst_mip),
            dst_offset: Offset3D::ZERO,
            extent: Extent3D::d2(width, height),
        }
    }
}

/// Image blit region (with scaling)
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ImageBlit {
    /// Source subresource
    pub src_subresource: ImageSubresourceLayers,
    /// Source region (two corners)
    pub src_offsets: [Offset3D; 2],
    /// Destination subresource
    pub dst_subresource: ImageSubresourceLayers,
    /// Destination region (two corners)
    pub dst_offsets: [Offset3D; 2],
}

impl ImageBlit {
    /// Creates a blit from full source to full destination
    pub fn full(src_extent: Extent3D, dst_extent: Extent3D) -> Self {
        Self {
            src_subresource: ImageSubresourceLayers::color(0),
            src_offsets: [
                Offset3D::ZERO,
                Offset3D::new(src_extent.width as i32, src_extent.height as i32, src_extent.depth as i32),
            ],
            dst_subresource: ImageSubresourceLayers::color(0),
            dst_offsets: [
                Offset3D::ZERO,
                Offset3D::new(dst_extent.width as i32, dst_extent.height as i32, dst_extent.depth as i32),
            ],
        }
    }
}

/// Filter mode for blitting
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum BlitFilter {
    /// Nearest neighbor filtering
    #[default]
    Nearest,
    /// Linear filtering
    Linear,
    /// Cubic filtering (if supported)
    Cubic,
}

/// Image resolve region (MSAA to single sample)
#[derive(Clone, Copy, Debug)]
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
    /// Extent to resolve
    pub extent: Extent3D,
}

impl ImageResolve {
    /// Creates a full image resolve
    pub fn full(extent: Extent3D) -> Self {
        Self {
            src_subresource: ImageSubresourceLayers::color(0),
            src_offset: Offset3D::ZERO,
            dst_subresource: ImageSubresourceLayers::color(0),
            dst_offset: Offset3D::ZERO,
            extent,
        }
    }
}

/// Buffer to image copy region
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BufferImageCopy {
    /// Buffer offset in bytes
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

impl BufferImageCopy {
    /// Creates a simple 2D copy
    pub fn d2(width: u32, height: u32) -> Self {
        Self {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: ImageSubresourceLayers::color(0),
            image_offset: Offset3D::ZERO,
            image_extent: Extent3D::d2(width, height),
        }
    }

    /// Sets the buffer offset
    pub const fn with_buffer_offset(mut self, offset: u64) -> Self {
        self.buffer_offset = offset;
        self
    }

    /// Sets the mip level
    pub const fn with_mip_level(mut self, level: u32) -> Self {
        self.image_subresource.mip_level = level;
        self
    }
}

/// Clear color value union
#[derive(Clone, Copy)]
#[repr(C)]
pub union ClearColorValue {
    /// Float values
    pub float32: [f32; 4],
    /// Signed integer values
    pub int32: [i32; 4],
    /// Unsigned integer values
    pub uint32: [u32; 4],
}

impl Default for ClearColorValue {
    fn default() -> Self {
        Self {
            float32: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

impl ClearColorValue {
    /// Black color
    pub const BLACK: Self = Self {
        float32: [0.0, 0.0, 0.0, 1.0],
    };

    /// White color
    pub const WHITE: Self = Self {
        float32: [1.0, 1.0, 1.0, 1.0],
    };

    /// Creates a float color
    pub const fn float(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            float32: [r, g, b, a],
        }
    }
}

/// Clear depth stencil value
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ClearDepthStencilValue {
    /// Depth value
    pub depth: f32,
    /// Stencil value
    pub stencil: u32,
}

impl ClearDepthStencilValue {
    /// Default depth clear (1.0)
    pub const DEFAULT: Self = Self {
        depth: 1.0,
        stencil: 0,
    };

    /// Reversed depth clear (0.0)
    pub const REVERSED: Self = Self {
        depth: 0.0,
        stencil: 0,
    };
}

/// Image memory barrier
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ImageMemoryBarrier {
    /// Source access mask
    pub src_access_mask: u32,
    /// Destination access mask
    pub dst_access_mask: u32,
    /// Old layout
    pub old_layout: ImageLayout,
    /// New layout
    pub new_layout: ImageLayout,
    /// Source queue family
    pub src_queue_family_index: u32,
    /// Destination queue family
    pub dst_queue_family_index: u32,
    /// Image handle
    pub image: TextureHandle,
    /// Subresource range
    pub subresource_range: ImageSubresourceRange,
}

impl ImageMemoryBarrier {
    /// Queue family ignored constant
    pub const QUEUE_FAMILY_IGNORED: u32 = u32::MAX;

    /// Creates a simple layout transition
    pub fn layout_transition(
        image: TextureHandle,
        old_layout: ImageLayout,
        new_layout: ImageLayout,
        subresource_range: ImageSubresourceRange,
    ) -> Self {
        Self {
            src_access_mask: 0,
            dst_access_mask: 0,
            old_layout,
            new_layout,
            src_queue_family_index: Self::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: Self::QUEUE_FAMILY_IGNORED,
            image,
            subresource_range,
        }
    }
}

/// Texture view description
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct TextureViewDesc {
    /// Source texture
    pub texture: TextureHandle,
    /// View type
    pub view_type: TextureViewType,
    /// Format
    pub format: TextureFormat,
    /// Component mapping
    pub components: ComponentMapping,
    /// Subresource range
    pub subresource_range: ImageSubresourceRange,
}

/// Texture view type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum TextureViewType {
    /// 1D texture
    D1,
    /// 2D texture
    #[default]
    D2,
    /// 3D texture
    D3,
    /// Cube map
    Cube,
    /// 1D array
    D1Array,
    /// 2D array
    D2Array,
    /// Cube map array
    CubeArray,
}

/// Component swizzle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum ComponentSwizzle {
    /// Identity (no swizzle)
    #[default]
    Identity,
    /// Zero constant
    Zero,
    /// One constant
    One,
    /// Red component
    R,
    /// Green component
    G,
    /// Blue component
    B,
    /// Alpha component
    A,
}

/// Component mapping (swizzle)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ComponentMapping {
    /// Red component source
    pub r: ComponentSwizzle,
    /// Green component source
    pub g: ComponentSwizzle,
    /// Blue component source
    pub b: ComponentSwizzle,
    /// Alpha component source
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

    /// Creates a new component mapping
    pub const fn new(
        r: ComponentSwizzle,
        g: ComponentSwizzle,
        b: ComponentSwizzle,
        a: ComponentSwizzle,
    ) -> Self {
        Self { r, g, b, a }
    }
}

/// Mipmap generation mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum MipmapGenerationMode {
    /// Box filter (average)
    #[default]
    Box,
    /// Lanczos filter
    Lanczos,
    /// Kaiser filter
    Kaiser,
}

/// Mipmap generation parameters
#[derive(Clone, Copy, Debug)]
pub struct MipmapGenerationParams {
    /// Filter mode
    pub filter: MipmapGenerationMode,
    /// Source mip level
    pub src_mip: u32,
    /// Number of mips to generate
    pub mip_count: u32,
    /// Base array layer
    pub base_array_layer: u32,
    /// Number of array layers
    pub layer_count: u32,
}

impl Default for MipmapGenerationParams {
    fn default() -> Self {
        Self {
            filter: MipmapGenerationMode::Box,
            src_mip: 0,
            mip_count: u32::MAX,
            base_array_layer: 0,
            layer_count: 1,
        }
    }
}

impl MipmapGenerationParams {
    /// Creates new mipmap generation parameters
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the number of mips to generate
    pub fn with_mip_count(mut self, count: u32) -> Self {
        self.mip_count = count;
        self
    }

    /// Sets array layer range
    pub fn with_layers(mut self, base: u32, count: u32) -> Self {
        self.base_array_layer = base;
        self.layer_count = count;
        self
    }
}

/// Image format properties
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

/// Format feature flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct FormatFeatureFlags(pub u32);

impl FormatFeatureFlags {
    /// No features
    pub const NONE: Self = Self(0);
    /// Can be sampled
    pub const SAMPLED_IMAGE: Self = Self(1 << 0);
    /// Can be storage image
    pub const STORAGE_IMAGE: Self = Self(1 << 1);
    /// Supports atomic operations
    pub const STORAGE_IMAGE_ATOMIC: Self = Self(1 << 2);
    /// Can be uniform texel buffer
    pub const UNIFORM_TEXEL_BUFFER: Self = Self(1 << 3);
    /// Can be storage texel buffer
    pub const STORAGE_TEXEL_BUFFER: Self = Self(1 << 4);
    /// Storage texel buffer atomic
    pub const STORAGE_TEXEL_BUFFER_ATOMIC: Self = Self(1 << 5);
    /// Can be vertex buffer
    pub const VERTEX_BUFFER: Self = Self(1 << 6);
    /// Can be color attachment
    pub const COLOR_ATTACHMENT: Self = Self(1 << 7);
    /// Color attachment blend
    pub const COLOR_ATTACHMENT_BLEND: Self = Self(1 << 8);
    /// Can be depth stencil attachment
    pub const DEPTH_STENCIL_ATTACHMENT: Self = Self(1 << 9);
    /// Can be blit source
    pub const BLIT_SRC: Self = Self(1 << 10);
    /// Can be blit destination
    pub const BLIT_DST: Self = Self(1 << 11);
    /// Sampled image filter linear
    pub const SAMPLED_IMAGE_FILTER_LINEAR: Self = Self(1 << 12);
    /// Transfer source
    pub const TRANSFER_SRC: Self = Self(1 << 14);
    /// Transfer destination
    pub const TRANSFER_DST: Self = Self(1 << 15);

    /// Checks if a feature is supported
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl core::ops::BitOr for FormatFeatureFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}
