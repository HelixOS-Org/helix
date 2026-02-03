//! Copy and blit operations
//!
//! This module provides types for GPU copy and blit operations.

/// Buffer copy region
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct BufferCopyRegion {
    /// Source offset in bytes
    pub src_offset: u64,
    /// Destination offset in bytes
    pub dst_offset: u64,
    /// Size in bytes
    pub size: u64,
}

impl BufferCopyRegion {
    /// Creates a new copy region
    pub const fn new(src_offset: u64, dst_offset: u64, size: u64) -> Self {
        Self {
            src_offset,
            dst_offset,
            size,
        }
    }

    /// Copy from start
    pub const fn from_start(size: u64) -> Self {
        Self {
            src_offset: 0,
            dst_offset: 0,
            size,
        }
    }
}

/// Image subresource layers
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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

impl Default for ImageSubresourceLayers {
    fn default() -> Self {
        Self::color()
    }
}

impl ImageSubresourceLayers {
    /// Color subresource
    pub const fn color() -> Self {
        Self {
            aspect_mask: ImageAspectFlags::COLOR,
            mip_level: 0,
            base_array_layer: 0,
            layer_count: 1,
        }
    }

    /// Depth subresource
    pub const fn depth() -> Self {
        Self {
            aspect_mask: ImageAspectFlags::DEPTH,
            mip_level: 0,
            base_array_layer: 0,
            layer_count: 1,
        }
    }

    /// With mip level
    pub const fn with_mip_level(mut self, level: u32) -> Self {
        self.mip_level = level;
        self
    }

    /// With layers
    pub const fn with_layers(mut self, base: u32, count: u32) -> Self {
        self.base_array_layer = base;
        self.layer_count = count;
        self
    }
}

/// Image aspect flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ImageAspectFlags(pub u32);

impl ImageAspectFlags {
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
    /// Depth and stencil
    pub const DEPTH_STENCIL: Self = Self(Self::DEPTH.0 | Self::STENCIL.0);

    /// Plane 0 (multi-planar)
    pub const PLANE_0: Self = Self(1 << 4);
    /// Plane 1 (multi-planar)
    pub const PLANE_1: Self = Self(1 << 5);
    /// Plane 2 (multi-planar)
    pub const PLANE_2: Self = Self(1 << 6);

    /// Contains
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

/// 3D offset
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
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
    /// Creates new offset
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// Zero offset
    pub const ZERO: Self = Self { x: 0, y: 0, z: 0 };
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
    pub const fn new(width: u32, height: u32, depth: u32) -> Self {
        Self {
            width,
            height,
            depth,
        }
    }

    /// 2D extent (depth = 1)
    pub const fn d2(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            depth: 1,
        }
    }
}

/// Buffer to image copy region
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BufferImageCopyRegion {
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

impl BufferImageCopyRegion {
    /// Creates for 2D image
    pub const fn d2(
        buffer_offset: u64,
        width: u32,
        height: u32,
        subresource: ImageSubresourceLayers,
    ) -> Self {
        Self {
            buffer_offset,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: subresource,
            image_offset: Offset3D::ZERO,
            image_extent: Extent3D::d2(width, height),
        }
    }

    /// Creates for full image
    pub const fn full(extent: Extent3D, subresource: ImageSubresourceLayers) -> Self {
        Self {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: subresource,
            image_offset: Offset3D::ZERO,
            image_extent: extent,
        }
    }

    /// With buffer offset
    pub const fn with_buffer_offset(mut self, offset: u64) -> Self {
        self.buffer_offset = offset;
        self
    }

    /// With image offset
    pub const fn with_image_offset(mut self, offset: Offset3D) -> Self {
        self.image_offset = offset;
        self
    }
}

/// Image copy region
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ImageCopyRegion {
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

impl ImageCopyRegion {
    /// Creates for full image
    pub const fn full(extent: Extent3D) -> Self {
        Self {
            src_subresource: ImageSubresourceLayers::color(),
            src_offset: Offset3D::ZERO,
            dst_subresource: ImageSubresourceLayers::color(),
            dst_offset: Offset3D::ZERO,
            extent,
        }
    }

    /// Creates for mip level
    pub const fn mip_level(src_mip: u32, dst_mip: u32, extent: Extent3D) -> Self {
        Self {
            src_subresource: ImageSubresourceLayers {
                aspect_mask: ImageAspectFlags::COLOR,
                mip_level: src_mip,
                base_array_layer: 0,
                layer_count: 1,
            },
            src_offset: Offset3D::ZERO,
            dst_subresource: ImageSubresourceLayers {
                aspect_mask: ImageAspectFlags::COLOR,
                mip_level: dst_mip,
                base_array_layer: 0,
                layer_count: 1,
            },
            dst_offset: Offset3D::ZERO,
            extent,
        }
    }
}

/// Blit filter
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum BlitFilter {
    /// Nearest neighbor
    Nearest = 0,
    /// Linear interpolation
    #[default]
    Linear = 1,
    /// Cubic interpolation
    Cubic = 2,
}

/// Image blit region
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ImageBlitRegion {
    /// Source subresource
    pub src_subresource: ImageSubresourceLayers,
    /// Source bounds (min and max corners)
    pub src_offsets: [Offset3D; 2],
    /// Destination subresource
    pub dst_subresource: ImageSubresourceLayers,
    /// Destination bounds (min and max corners)
    pub dst_offsets: [Offset3D; 2],
}

impl ImageBlitRegion {
    /// Creates blit from full source to full destination
    pub fn full(src_extent: Extent3D, dst_extent: Extent3D) -> Self {
        Self {
            src_subresource: ImageSubresourceLayers::color(),
            src_offsets: [
                Offset3D::ZERO,
                Offset3D::new(src_extent.width as i32, src_extent.height as i32, src_extent.depth as i32),
            ],
            dst_subresource: ImageSubresourceLayers::color(),
            dst_offsets: [
                Offset3D::ZERO,
                Offset3D::new(dst_extent.width as i32, dst_extent.height as i32, dst_extent.depth as i32),
            ],
        }
    }

    /// Creates downsample blit (mip generation)
    pub fn downsample(src_mip: u32, src_extent: Extent3D, dst_mip: u32, dst_extent: Extent3D) -> Self {
        Self {
            src_subresource: ImageSubresourceLayers::color().with_mip_level(src_mip),
            src_offsets: [
                Offset3D::ZERO,
                Offset3D::new(src_extent.width as i32, src_extent.height as i32, 1),
            ],
            dst_subresource: ImageSubresourceLayers::color().with_mip_level(dst_mip),
            dst_offsets: [
                Offset3D::ZERO,
                Offset3D::new(dst_extent.width as i32, dst_extent.height as i32, 1),
            ],
        }
    }
}

/// Image resolve region
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ImageResolveRegion {
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

impl ImageResolveRegion {
    /// Creates for full image
    pub const fn full(extent: Extent3D) -> Self {
        Self {
            src_subresource: ImageSubresourceLayers::color(),
            src_offset: Offset3D::ZERO,
            dst_subresource: ImageSubresourceLayers::color(),
            dst_offset: Offset3D::ZERO,
            extent,
        }
    }
}

/// Clear color value
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub union ClearColorValue {
    /// Float values
    pub float32: [f32; 4],
    /// Signed int values
    pub int32: [i32; 4],
    /// Unsigned int values
    pub uint32: [u32; 4],
}

impl Default for ClearColorValue {
    fn default() -> Self {
        Self {
            float32: [0.0, 0.0, 0.0, 0.0],
        }
    }
}

impl ClearColorValue {
    /// Creates from float values
    pub const fn from_float(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            float32: [r, g, b, a],
        }
    }

    /// Creates from int values
    pub const fn from_int(r: i32, g: i32, b: i32, a: i32) -> Self {
        Self {
            int32: [r, g, b, a],
        }
    }

    /// Creates from uint values
    pub const fn from_uint(r: u32, g: u32, b: u32, a: u32) -> Self {
        Self {
            uint32: [r, g, b, a],
        }
    }

    /// Black
    pub const BLACK: Self = Self {
        float32: [0.0, 0.0, 0.0, 1.0],
    };

    /// White
    pub const WHITE: Self = Self {
        float32: [1.0, 1.0, 1.0, 1.0],
    };

    /// Transparent
    pub const TRANSPARENT: Self = Self {
        float32: [0.0, 0.0, 0.0, 0.0],
    };
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
    /// Creates new value
    pub const fn new(depth: f32, stencil: u32) -> Self {
        Self { depth, stencil }
    }

    /// Default depth (1.0, 0)
    pub const DEFAULT: Self = Self {
        depth: 1.0,
        stencil: 0,
    };

    /// Reversed-Z depth (0.0, 0)
    pub const REVERSED_Z: Self = Self {
        depth: 0.0,
        stencil: 0,
    };
}

/// Clear value (color or depth-stencil)
#[derive(Clone, Copy)]
#[repr(C)]
pub union ClearValue {
    /// Color clear value
    pub color: ClearColorValue,
    /// Depth-stencil clear value
    pub depth_stencil: ClearDepthStencilValue,
}

impl Default for ClearValue {
    fn default() -> Self {
        Self {
            color: ClearColorValue::BLACK,
        }
    }
}

impl ClearValue {
    /// Creates color clear value
    pub const fn color(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            color: ClearColorValue::from_float(r, g, b, a),
        }
    }

    /// Creates depth clear value
    pub const fn depth(depth: f32) -> Self {
        Self {
            depth_stencil: ClearDepthStencilValue {
                depth,
                stencil: 0,
            },
        }
    }

    /// Creates depth-stencil clear value
    pub const fn depth_stencil(depth: f32, stencil: u32) -> Self {
        Self {
            depth_stencil: ClearDepthStencilValue { depth, stencil },
        }
    }
}

/// Fill buffer info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FillBufferInfo {
    /// Destination buffer
    pub dst_buffer: u64,
    /// Offset in bytes (must be multiple of 4)
    pub offset: u64,
    /// Size in bytes (must be multiple of 4)
    pub size: u64,
    /// Data to fill (u32)
    pub data: u32,
}

impl FillBufferInfo {
    /// Creates fill info
    pub const fn new(buffer: u64, offset: u64, size: u64, data: u32) -> Self {
        Self {
            dst_buffer: buffer,
            offset,
            size,
            data,
        }
    }

    /// Fill with zero
    pub const fn zero(buffer: u64, offset: u64, size: u64) -> Self {
        Self {
            dst_buffer: buffer,
            offset,
            size,
            data: 0,
        }
    }
}

/// Update buffer info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct UpdateBufferInfo {
    /// Destination buffer
    pub dst_buffer: u64,
    /// Offset in bytes
    pub offset: u64,
    /// Size in bytes (max 65536)
    pub size: u64,
}

impl UpdateBufferInfo {
    /// Maximum inline update size
    pub const MAX_SIZE: u64 = 65536;

    /// Creates update info
    pub const fn new(buffer: u64, offset: u64, size: u64) -> Self {
        Self {
            dst_buffer: buffer,
            offset,
            size,
        }
    }
}
