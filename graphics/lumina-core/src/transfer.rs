//! Transfer Operations for Lumina
//!
//! This module provides buffer and image copy, blit, clear, and fill operations
//! for GPU data transfer.

// ============================================================================
// Buffer Copy
// ============================================================================

/// Buffer copy region
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BufferCopy {
    /// Source offset
    pub src_offset: u64,
    /// Destination offset
    pub dst_offset: u64,
    /// Size in bytes
    pub size: u64,
}

impl BufferCopy {
    /// Creates new buffer copy
    #[inline]
    pub const fn new(src_offset: u64, dst_offset: u64, size: u64) -> Self {
        Self {
            src_offset,
            dst_offset,
            size,
        }
    }

    /// Copy from start
    #[inline]
    pub const fn from_start(size: u64) -> Self {
        Self::new(0, 0, size)
    }

    /// With source offset
    #[inline]
    pub const fn with_src_offset(mut self, offset: u64) -> Self {
        self.src_offset = offset;
        self
    }

    /// With destination offset
    #[inline]
    pub const fn with_dst_offset(mut self, offset: u64) -> Self {
        self.dst_offset = offset;
        self
    }
}

impl Default for BufferCopy {
    fn default() -> Self {
        Self::from_start(0)
    }
}

/// Buffer copy info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BufferCopyInfo {
    /// Source buffer
    pub src_buffer: u64,
    /// Destination buffer
    pub dst_buffer: u64,
    /// Regions
    pub regions: &'static [BufferCopy],
}

impl BufferCopyInfo {
    /// Creates new info
    #[inline]
    pub const fn new(src_buffer: u64, dst_buffer: u64, regions: &'static [BufferCopy]) -> Self {
        Self {
            src_buffer,
            dst_buffer,
            regions,
        }
    }
}

// ============================================================================
// Image Copy
// ============================================================================

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
    /// Extent
    pub extent: Extent3D,
}

impl ImageCopy {
    /// Creates new image copy
    #[inline]
    pub const fn new(
        src_subresource: ImageSubresourceLayers,
        src_offset: Offset3D,
        dst_subresource: ImageSubresourceLayers,
        dst_offset: Offset3D,
        extent: Extent3D,
    ) -> Self {
        Self {
            src_subresource,
            src_offset,
            dst_subresource,
            dst_offset,
            extent,
        }
    }

    /// Simple copy (same subresource layers)
    #[inline]
    pub const fn simple(extent: Extent3D) -> Self {
        Self {
            src_subresource: ImageSubresourceLayers::COLOR,
            src_offset: Offset3D::ZERO,
            dst_subresource: ImageSubresourceLayers::COLOR,
            dst_offset: Offset3D::ZERO,
            extent,
        }
    }

    /// Copy mip level
    #[inline]
    pub const fn mip_level(mip: u32, extent: Extent3D) -> Self {
        Self {
            src_subresource: ImageSubresourceLayers::mip(mip),
            src_offset: Offset3D::ZERO,
            dst_subresource: ImageSubresourceLayers::mip(mip),
            dst_offset: Offset3D::ZERO,
            extent,
        }
    }

    /// Copy array layer
    #[inline]
    pub const fn array_layer(layer: u32, extent: Extent3D) -> Self {
        Self {
            src_subresource: ImageSubresourceLayers::layer(layer),
            src_offset: Offset3D::ZERO,
            dst_subresource: ImageSubresourceLayers::layer(layer),
            dst_offset: Offset3D::ZERO,
            extent,
        }
    }
}

impl Default for ImageCopy {
    fn default() -> Self {
        Self::simple(Extent3D::default())
    }
}

/// Image copy info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ImageCopyInfo {
    /// Source image
    pub src_image: u64,
    /// Source layout
    pub src_layout: ImageLayout,
    /// Destination image
    pub dst_image: u64,
    /// Destination layout
    pub dst_layout: ImageLayout,
    /// Regions
    pub regions: &'static [ImageCopy],
}

impl ImageCopyInfo {
    /// Creates new info
    #[inline]
    pub const fn new(
        src_image: u64,
        src_layout: ImageLayout,
        dst_image: u64,
        dst_layout: ImageLayout,
        regions: &'static [ImageCopy],
    ) -> Self {
        Self {
            src_image,
            src_layout,
            dst_image,
            dst_layout,
            regions,
        }
    }
}

// ============================================================================
// Buffer-Image Copy
// ============================================================================

/// Buffer to image copy region
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BufferImageCopy {
    /// Buffer offset
    pub buffer_offset: u64,
    /// Buffer row length (0 = tightly packed)
    pub buffer_row_length: u32,
    /// Buffer image height (0 = tightly packed)
    pub buffer_image_height: u32,
    /// Image subresource
    pub image_subresource: ImageSubresourceLayers,
    /// Image offset
    pub image_offset: Offset3D,
    /// Image extent
    pub image_extent: Extent3D,
}

impl BufferImageCopy {
    /// Creates new buffer-image copy
    #[inline]
    pub const fn new(
        buffer_offset: u64,
        image_subresource: ImageSubresourceLayers,
        image_offset: Offset3D,
        image_extent: Extent3D,
    ) -> Self {
        Self {
            buffer_offset,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource,
            image_offset,
            image_extent,
        }
    }

    /// Simple copy (tightly packed, from origin)
    #[inline]
    pub const fn simple(extent: Extent3D) -> Self {
        Self {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: ImageSubresourceLayers::COLOR,
            image_offset: Offset3D::ZERO,
            image_extent: extent,
        }
    }

    /// With buffer offset
    #[inline]
    pub const fn with_buffer_offset(mut self, offset: u64) -> Self {
        self.buffer_offset = offset;
        self
    }

    /// With buffer layout
    #[inline]
    pub const fn with_buffer_layout(mut self, row_length: u32, image_height: u32) -> Self {
        self.buffer_row_length = row_length;
        self.buffer_image_height = image_height;
        self
    }

    /// With mip level
    #[inline]
    pub const fn with_mip_level(mut self, mip: u32) -> Self {
        self.image_subresource.mip_level = mip;
        self
    }

    /// With array layer
    #[inline]
    pub const fn with_array_layer(mut self, layer: u32) -> Self {
        self.image_subresource.base_array_layer = layer;
        self
    }
}

impl Default for BufferImageCopy {
    fn default() -> Self {
        Self::simple(Extent3D::default())
    }
}

/// Copy buffer to image info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct CopyBufferToImageInfo {
    /// Source buffer
    pub src_buffer: u64,
    /// Destination image
    pub dst_image: u64,
    /// Destination layout
    pub dst_layout: ImageLayout,
    /// Regions
    pub regions: &'static [BufferImageCopy],
}

impl CopyBufferToImageInfo {
    /// Creates new info
    #[inline]
    pub const fn new(
        src_buffer: u64,
        dst_image: u64,
        dst_layout: ImageLayout,
        regions: &'static [BufferImageCopy],
    ) -> Self {
        Self {
            src_buffer,
            dst_image,
            dst_layout,
            regions,
        }
    }
}

/// Copy image to buffer info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct CopyImageToBufferInfo {
    /// Source image
    pub src_image: u64,
    /// Source layout
    pub src_layout: ImageLayout,
    /// Destination buffer
    pub dst_buffer: u64,
    /// Regions
    pub regions: &'static [BufferImageCopy],
}

impl CopyImageToBufferInfo {
    /// Creates new info
    #[inline]
    pub const fn new(
        src_image: u64,
        src_layout: ImageLayout,
        dst_buffer: u64,
        regions: &'static [BufferImageCopy],
    ) -> Self {
        Self {
            src_image,
            src_layout,
            dst_buffer,
            regions,
        }
    }
}

// ============================================================================
// Image Blit
// ============================================================================

/// Image blit region
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ImageBlit {
    /// Source subresource
    pub src_subresource: ImageSubresourceLayers,
    /// Source offsets (top-left and bottom-right)
    pub src_offsets: [Offset3D; 2],
    /// Destination subresource
    pub dst_subresource: ImageSubresourceLayers,
    /// Destination offsets (top-left and bottom-right)
    pub dst_offsets: [Offset3D; 2],
}

impl ImageBlit {
    /// Creates new blit region
    #[inline]
    pub const fn new(
        src_subresource: ImageSubresourceLayers,
        src_offsets: [Offset3D; 2],
        dst_subresource: ImageSubresourceLayers,
        dst_offsets: [Offset3D; 2],
    ) -> Self {
        Self {
            src_subresource,
            src_offsets,
            dst_subresource,
            dst_offsets,
        }
    }

    /// Full blit from source to destination
    #[inline]
    pub const fn full(src_extent: Extent3D, dst_extent: Extent3D) -> Self {
        Self {
            src_subresource: ImageSubresourceLayers::COLOR,
            src_offsets: [Offset3D::ZERO, Offset3D::from_extent(src_extent)],
            dst_subresource: ImageSubresourceLayers::COLOR,
            dst_offsets: [Offset3D::ZERO, Offset3D::from_extent(dst_extent)],
        }
    }

    /// Generate mipmap level
    #[inline]
    pub const fn mipmap_level(
        src_mip: u32,
        src_extent: Extent3D,
        dst_mip: u32,
        dst_extent: Extent3D,
    ) -> Self {
        Self {
            src_subresource: ImageSubresourceLayers::mip(src_mip),
            src_offsets: [Offset3D::ZERO, Offset3D::from_extent(src_extent)],
            dst_subresource: ImageSubresourceLayers::mip(dst_mip),
            dst_offsets: [Offset3D::ZERO, Offset3D::from_extent(dst_extent)],
        }
    }
}

/// Image blit info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ImageBlitInfo {
    /// Source image
    pub src_image: u64,
    /// Source layout
    pub src_layout: ImageLayout,
    /// Destination image
    pub dst_image: u64,
    /// Destination layout
    pub dst_layout: ImageLayout,
    /// Regions
    pub regions: &'static [ImageBlit],
    /// Filter
    pub filter: Filter,
}

impl ImageBlitInfo {
    /// Creates new info
    #[inline]
    pub const fn new(
        src_image: u64,
        src_layout: ImageLayout,
        dst_image: u64,
        dst_layout: ImageLayout,
        regions: &'static [ImageBlit],
        filter: Filter,
    ) -> Self {
        Self {
            src_image,
            src_layout,
            dst_image,
            dst_layout,
            regions,
            filter,
        }
    }
}

/// Filter mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum Filter {
    /// Nearest neighbor
    #[default]
    Nearest  = 0,
    /// Linear interpolation
    Linear   = 1,
    /// Cubic interpolation (EXT)
    CubicExt = 2,
}

// ============================================================================
// Image Resolve
// ============================================================================

/// Image resolve region
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
    /// Extent
    pub extent: Extent3D,
}

impl ImageResolve {
    /// Creates new resolve region
    #[inline]
    pub const fn new(
        src_subresource: ImageSubresourceLayers,
        dst_subresource: ImageSubresourceLayers,
        extent: Extent3D,
    ) -> Self {
        Self {
            src_subresource,
            src_offset: Offset3D::ZERO,
            dst_subresource,
            dst_offset: Offset3D::ZERO,
            extent,
        }
    }

    /// Simple resolve
    #[inline]
    pub const fn simple(extent: Extent3D) -> Self {
        Self::new(
            ImageSubresourceLayers::COLOR,
            ImageSubresourceLayers::COLOR,
            extent,
        )
    }
}

/// Image resolve info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ImageResolveInfo {
    /// Source image
    pub src_image: u64,
    /// Source layout
    pub src_layout: ImageLayout,
    /// Destination image
    pub dst_image: u64,
    /// Destination layout
    pub dst_layout: ImageLayout,
    /// Regions
    pub regions: &'static [ImageResolve],
}

impl ImageResolveInfo {
    /// Creates new info
    #[inline]
    pub const fn new(
        src_image: u64,
        src_layout: ImageLayout,
        dst_image: u64,
        dst_layout: ImageLayout,
        regions: &'static [ImageResolve],
    ) -> Self {
        Self {
            src_image,
            src_layout,
            dst_image,
            dst_layout,
            regions,
        }
    }
}

// ============================================================================
// Buffer Fill
// ============================================================================

/// Buffer fill info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BufferFillInfo {
    /// Buffer
    pub buffer: u64,
    /// Offset
    pub offset: u64,
    /// Size (WHOLE_SIZE for entire buffer)
    pub size: u64,
    /// Data (4-byte value repeated)
    pub data: u32,
}

impl BufferFillInfo {
    /// Whole buffer constant
    pub const WHOLE_SIZE: u64 = u64::MAX;

    /// Creates new fill info
    #[inline]
    pub const fn new(buffer: u64, offset: u64, size: u64, data: u32) -> Self {
        Self {
            buffer,
            offset,
            size,
            data,
        }
    }

    /// Fill entire buffer
    #[inline]
    pub const fn whole(buffer: u64, data: u32) -> Self {
        Self::new(buffer, 0, Self::WHOLE_SIZE, data)
    }

    /// Fill with zeros
    #[inline]
    pub const fn zero(buffer: u64) -> Self {
        Self::whole(buffer, 0)
    }
}

// ============================================================================
// Buffer Update
// ============================================================================

/// Buffer update info (inline data)
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BufferUpdateInfo {
    /// Buffer
    pub buffer: u64,
    /// Offset
    pub offset: u64,
    /// Data pointer
    pub data: *const u8,
    /// Size
    pub size: u64,
}

impl BufferUpdateInfo {
    /// Maximum inline update size
    pub const MAX_SIZE: u64 = 65536;

    /// Creates new update info
    #[inline]
    pub const fn new(buffer: u64, offset: u64, data: *const u8, size: u64) -> Self {
        Self {
            buffer,
            offset,
            data,
            size,
        }
    }

    /// From slice
    #[inline]
    pub fn from_slice(buffer: u64, offset: u64, data: &[u8]) -> Self {
        Self {
            buffer,
            offset,
            data: data.as_ptr(),
            size: data.len() as u64,
        }
    }
}

// ============================================================================
// Image Clear
// ============================================================================

/// Clear color image info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ClearColorImageInfo {
    /// Image
    pub image: u64,
    /// Layout
    pub layout: ImageLayout,
    /// Clear color
    pub clear_color: ClearColorValue,
    /// Ranges
    pub ranges: &'static [ImageSubresourceRange],
}

impl ClearColorImageInfo {
    /// Creates new info
    #[inline]
    pub const fn new(
        image: u64,
        layout: ImageLayout,
        clear_color: ClearColorValue,
        ranges: &'static [ImageSubresourceRange],
    ) -> Self {
        Self {
            image,
            layout,
            clear_color,
            ranges,
        }
    }
}

/// Clear depth-stencil image info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ClearDepthStencilImageInfo {
    /// Image
    pub image: u64,
    /// Layout
    pub layout: ImageLayout,
    /// Clear value
    pub clear_value: ClearDepthStencilValue,
    /// Ranges
    pub ranges: &'static [ImageSubresourceRange],
}

impl ClearDepthStencilImageInfo {
    /// Creates new info
    #[inline]
    pub const fn new(
        image: u64,
        layout: ImageLayout,
        clear_value: ClearDepthStencilValue,
        ranges: &'static [ImageSubresourceRange],
    ) -> Self {
        Self {
            image,
            layout,
            clear_value,
            ranges,
        }
    }
}

// ============================================================================
// Clear Values
// ============================================================================

/// Clear color value
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub union ClearColorValue {
    /// Float values
    pub float32: [f32; 4],
    /// Int32 values
    pub int32: [i32; 4],
    /// Uint32 values
    pub uint32: [u32; 4],
}

impl ClearColorValue {
    /// Black transparent
    pub const ZERO: Self = Self {
        float32: [0.0, 0.0, 0.0, 0.0],
    };
    /// Black opaque
    pub const BLACK: Self = Self {
        float32: [0.0, 0.0, 0.0, 1.0],
    };
    /// White opaque
    pub const WHITE: Self = Self {
        float32: [1.0, 1.0, 1.0, 1.0],
    };
    /// Red
    pub const RED: Self = Self {
        float32: [1.0, 0.0, 0.0, 1.0],
    };
    /// Green
    pub const GREEN: Self = Self {
        float32: [0.0, 1.0, 0.0, 1.0],
    };
    /// Blue
    pub const BLUE: Self = Self {
        float32: [0.0, 0.0, 1.0, 1.0],
    };

    /// Creates from float values
    #[inline]
    pub const fn float(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            float32: [r, g, b, a],
        }
    }

    /// Creates from int values
    #[inline]
    pub const fn int(r: i32, g: i32, b: i32, a: i32) -> Self {
        Self {
            int32: [r, g, b, a],
        }
    }

    /// Creates from uint values
    #[inline]
    pub const fn uint(r: u32, g: u32, b: u32, a: u32) -> Self {
        Self {
            uint32: [r, g, b, a],
        }
    }
}

impl Default for ClearColorValue {
    fn default() -> Self {
        Self::ZERO
    }
}

/// Clear depth-stencil value
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ClearDepthStencilValue {
    /// Depth
    pub depth: f32,
    /// Stencil
    pub stencil: u32,
}

impl ClearDepthStencilValue {
    /// Default (depth 1.0, stencil 0)
    pub const DEFAULT: Self = Self {
        depth: 1.0,
        stencil: 0,
    };
    /// Reverse-Z (depth 0.0, stencil 0)
    pub const REVERSE_Z: Self = Self {
        depth: 0.0,
        stencil: 0,
    };

    /// Creates new value
    #[inline]
    pub const fn new(depth: f32, stencil: u32) -> Self {
        Self { depth, stencil }
    }
}

impl Default for ClearDepthStencilValue {
    fn default() -> Self {
        Self::DEFAULT
    }
}

// ============================================================================
// Image Subresource
// ============================================================================

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

impl ImageSubresourceLayers {
    /// Color aspect, mip 0, layer 0
    pub const COLOR: Self = Self {
        aspect_mask: ImageAspectFlags::COLOR,
        mip_level: 0,
        base_array_layer: 0,
        layer_count: 1,
    };

    /// Depth aspect
    pub const DEPTH: Self = Self {
        aspect_mask: ImageAspectFlags::DEPTH,
        mip_level: 0,
        base_array_layer: 0,
        layer_count: 1,
    };

    /// Stencil aspect
    pub const STENCIL: Self = Self {
        aspect_mask: ImageAspectFlags::STENCIL,
        mip_level: 0,
        base_array_layer: 0,
        layer_count: 1,
    };

    /// Creates for mip level
    #[inline]
    pub const fn mip(level: u32) -> Self {
        Self {
            aspect_mask: ImageAspectFlags::COLOR,
            mip_level: level,
            base_array_layer: 0,
            layer_count: 1,
        }
    }

    /// Creates for array layer
    #[inline]
    pub const fn layer(layer: u32) -> Self {
        Self {
            aspect_mask: ImageAspectFlags::COLOR,
            mip_level: 0,
            base_array_layer: layer,
            layer_count: 1,
        }
    }

    /// Creates for multiple layers
    #[inline]
    pub const fn layers(base: u32, count: u32) -> Self {
        Self {
            aspect_mask: ImageAspectFlags::COLOR,
            mip_level: 0,
            base_array_layer: base,
            layer_count: count,
        }
    }

    /// All layers constant
    pub const ALL_LAYERS: u32 = u32::MAX;
}

impl Default for ImageSubresourceLayers {
    fn default() -> Self {
        Self::COLOR
    }
}

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
    /// All mips and layers constant
    pub const ALL: u32 = u32::MAX;

    /// Color, all mips and layers
    pub const COLOR_ALL: Self = Self {
        aspect_mask: ImageAspectFlags::COLOR,
        base_mip_level: 0,
        level_count: Self::ALL,
        base_array_layer: 0,
        layer_count: Self::ALL,
    };

    /// Depth, all mips and layers
    pub const DEPTH_ALL: Self = Self {
        aspect_mask: ImageAspectFlags::DEPTH,
        base_mip_level: 0,
        level_count: Self::ALL,
        base_array_layer: 0,
        layer_count: Self::ALL,
    };

    /// Creates new range
    #[inline]
    pub const fn new(
        aspect_mask: ImageAspectFlags,
        base_mip_level: u32,
        level_count: u32,
        base_array_layer: u32,
        layer_count: u32,
    ) -> Self {
        Self {
            aspect_mask,
            base_mip_level,
            level_count,
            base_array_layer,
            layer_count,
        }
    }

    /// Single mip level
    #[inline]
    pub const fn mip(level: u32) -> Self {
        Self {
            aspect_mask: ImageAspectFlags::COLOR,
            base_mip_level: level,
            level_count: 1,
            base_array_layer: 0,
            layer_count: Self::ALL,
        }
    }

    /// Single array layer
    #[inline]
    pub const fn layer(layer: u32) -> Self {
        Self {
            aspect_mask: ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: Self::ALL,
            base_array_layer: layer,
            layer_count: 1,
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
    /// None
    pub const NONE: Self = Self(0);
    /// Color
    pub const COLOR: Self = Self(1 << 0);
    /// Depth
    pub const DEPTH: Self = Self(1 << 1);
    /// Stencil
    pub const STENCIL: Self = Self(1 << 2);
    /// Metadata
    pub const METADATA: Self = Self(1 << 3);
    /// Plane 0
    pub const PLANE_0: Self = Self(1 << 4);
    /// Plane 1
    pub const PLANE_1: Self = Self(1 << 5);
    /// Plane 2
    pub const PLANE_2: Self = Self(1 << 6);
    /// Depth and stencil
    pub const DEPTH_STENCIL: Self = Self(Self::DEPTH.0 | Self::STENCIL.0);

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
// Common Types
// ============================================================================

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
    /// Zero offset
    pub const ZERO: Self = Self { x: 0, y: 0, z: 0 };

    /// Creates new offset
    #[inline]
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// From extent (as corner)
    #[inline]
    pub const fn from_extent(extent: Extent3D) -> Self {
        Self {
            x: extent.width as i32,
            y: extent.height as i32,
            z: extent.depth as i32,
        }
    }
}

/// 3D extent
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    /// Unit extent
    pub const UNIT: Self = Self {
        width: 1,
        height: 1,
        depth: 1,
    };

    /// Creates new extent
    #[inline]
    pub const fn new(width: u32, height: u32, depth: u32) -> Self {
        Self {
            width,
            height,
            depth,
        }
    }

    /// 2D extent
    #[inline]
    pub const fn d2(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            depth: 1,
        }
    }

    /// Volume
    #[inline]
    pub const fn volume(&self) -> u64 {
        self.width as u64 * self.height as u64 * self.depth as u64
    }

    /// Half size (for mipmap)
    #[inline]
    pub const fn half(&self) -> Self {
        Self {
            width: if self.width > 1 { self.width / 2 } else { 1 },
            height: if self.height > 1 { self.height / 2 } else { 1 },
            depth: if self.depth > 1 { self.depth / 2 } else { 1 },
        }
    }

    /// Mip size at level
    #[inline]
    pub const fn mip_size(&self, level: u32) -> Self {
        Self {
            width: (self.width >> level).max(1),
            height: (self.height >> level).max(1),
            depth: (self.depth >> level).max(1),
        }
    }
}

impl Default for Extent3D {
    fn default() -> Self {
        Self::UNIT
    }
}

/// Image layout
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ImageLayout {
    /// Undefined
    #[default]
    Undefined          = 0,
    /// General
    General            = 1,
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
    Preinitialized     = 8,
    /// Present source (for swapchain)
    PresentSrc         = 1000001002,
    /// Shared present
    SharedPresent      = 1000111000,
    /// Fragment shading rate attachment optimal
    FragmentShadingRateAttachmentOptimal = 1000164003,
    /// Read-only optimal (depth/stencil read + shader read)
    ReadOnlyOptimal    = 1000314000,
    /// Attachment optimal
    AttachmentOptimal  = 1000314001,
}

impl ImageLayout {
    /// Is read-only
    #[inline]
    pub const fn is_read_only(&self) -> bool {
        matches!(
            self,
            Self::DepthStencilReadOnlyOptimal
                | Self::ShaderReadOnlyOptimal
                | Self::TransferSrcOptimal
                | Self::ReadOnlyOptimal
        )
    }

    /// Is attachment
    #[inline]
    pub const fn is_attachment(&self) -> bool {
        matches!(
            self,
            Self::ColorAttachmentOptimal
                | Self::DepthStencilAttachmentOptimal
                | Self::AttachmentOptimal
        )
    }
}
