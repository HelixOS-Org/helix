//! Sparse resource types
//!
//! This module provides types for sparse resource management (virtual memory on GPU).

use core::num::NonZeroU32;

/// Sparse buffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SparseBufferHandle(pub NonZeroU32);

impl SparseBufferHandle {
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

/// Sparse image handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SparseImageHandle(pub NonZeroU32);

impl SparseImageHandle {
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

/// Device memory handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DeviceMemoryHandle(pub NonZeroU32);

impl DeviceMemoryHandle {
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

/// Sparse memory bind info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SparseMemoryBind {
    /// Resource offset (bytes for buffers, must be tile-aligned for images)
    pub resource_offset: u64,
    /// Size to bind (bytes)
    pub size: u64,
    /// Memory to bind (None = unbind)
    pub memory: Option<DeviceMemoryHandle>,
    /// Offset in memory
    pub memory_offset: u64,
    /// Bind flags
    pub flags: SparseMemoryBindFlags,
}

impl SparseMemoryBind {
    /// Creates a bind operation
    pub const fn bind(
        resource_offset: u64,
        size: u64,
        memory: DeviceMemoryHandle,
        memory_offset: u64,
    ) -> Self {
        Self {
            resource_offset,
            size,
            memory: Some(memory),
            memory_offset,
            flags: SparseMemoryBindFlags::empty(),
        }
    }

    /// Creates an unbind operation
    pub const fn unbind(resource_offset: u64, size: u64) -> Self {
        Self {
            resource_offset,
            size,
            memory: None,
            memory_offset: 0,
            flags: SparseMemoryBindFlags::empty(),
        }
    }

    /// With metadata
    pub const fn with_metadata(mut self) -> Self {
        self.flags = self.flags.union(SparseMemoryBindFlags::METADATA);
        self
    }
}

bitflags::bitflags! {
    /// Sparse memory bind flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct SparseMemoryBindFlags: u32 {
        /// Bind metadata
        const METADATA = 1 << 0;
    }
}

impl SparseMemoryBindFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}

/// Sparse image memory bind
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SparseImageMemoryBind {
    /// Subresource
    pub subresource: ImageSubresource,
    /// Offset in texels
    pub offset: Offset3D,
    /// Extent in texels
    pub extent: Extent3D,
    /// Memory to bind
    pub memory: Option<DeviceMemoryHandle>,
    /// Offset in memory
    pub memory_offset: u64,
    /// Bind flags
    pub flags: SparseMemoryBindFlags,
}

impl SparseImageMemoryBind {
    /// Bind a tile
    pub const fn bind_tile(
        subresource: ImageSubresource,
        offset: Offset3D,
        extent: Extent3D,
        memory: DeviceMemoryHandle,
        memory_offset: u64,
    ) -> Self {
        Self {
            subresource,
            offset,
            extent,
            memory: Some(memory),
            memory_offset,
            flags: SparseMemoryBindFlags::empty(),
        }
    }

    /// Unbind a tile
    pub const fn unbind_tile(
        subresource: ImageSubresource,
        offset: Offset3D,
        extent: Extent3D,
    ) -> Self {
        Self {
            subresource,
            offset,
            extent,
            memory: None,
            memory_offset: 0,
            flags: SparseMemoryBindFlags::empty(),
        }
    }
}

/// Image subresource
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ImageSubresource {
    /// Aspect mask
    pub aspect_mask: ImageAspectFlags,
    /// Mip level
    pub mip_level: u32,
    /// Array layer
    pub array_layer: u32,
}

impl ImageSubresource {
    /// Color mip 0 layer 0
    pub const fn color() -> Self {
        Self {
            aspect_mask: ImageAspectFlags::COLOR,
            mip_level: 0,
            array_layer: 0,
        }
    }

    /// Depth mip 0 layer 0
    pub const fn depth() -> Self {
        Self {
            aspect_mask: ImageAspectFlags::DEPTH,
            mip_level: 0,
            array_layer: 0,
        }
    }

    /// Specific mip level
    pub const fn at_mip(mut self, mip: u32) -> Self {
        self.mip_level = mip;
        self
    }

    /// Specific array layer
    pub const fn at_layer(mut self, layer: u32) -> Self {
        self.array_layer = layer;
        self
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
        /// Plane 0
        const PLANE_0 = 1 << 4;
        /// Plane 1
        const PLANE_1 = 1 << 5;
        /// Plane 2
        const PLANE_2 = 1 << 6;
    }
}

impl ImageAspectFlags {
    /// Depth + Stencil
    pub const DEPTH_STENCIL: Self =
        Self::from_bits_truncate(Self::DEPTH.bits() | Self::STENCIL.bits());
}

/// 3D offset
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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

    /// Creates an offset
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

/// 3D extent
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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
    /// Creates an extent
    pub const fn new(width: u32, height: u32, depth: u32) -> Self {
        Self {
            width,
            height,
            depth,
        }
    }

    /// 2D extent (depth = 1)
    pub const fn d2(width: u32, height: u32) -> Self {
        Self::new(width, height, 1)
    }

    /// Total texels
    pub const fn texel_count(&self) -> u64 {
        self.width as u64 * self.height as u64 * self.depth as u64
    }
}

/// Sparse buffer create info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SparseBufferCreateInfo {
    /// Total size in bytes
    pub size: u64,
    /// Usage flags
    pub usage: BufferUsageFlags,
    /// Sparse binding flags
    pub sparse_flags: SparseBindingFlags,
}

impl SparseBufferCreateInfo {
    /// Creates sparse buffer info
    pub const fn new(size: u64, usage: BufferUsageFlags) -> Self {
        Self {
            size,
            usage,
            sparse_flags: SparseBindingFlags::SPARSE_BINDING,
        }
    }

    /// With residency tracking
    pub const fn with_residency(mut self) -> Self {
        self.sparse_flags = self
            .sparse_flags
            .union(SparseBindingFlags::SPARSE_RESIDENCY);
        self
    }

    /// With aliasing
    pub const fn with_aliasing(mut self) -> Self {
        self.sparse_flags = self.sparse_flags.union(SparseBindingFlags::SPARSE_ALIASED);
        self
    }
}

bitflags::bitflags! {
    /// Buffer usage flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct BufferUsageFlags: u32 {
        /// Transfer source
        const TRANSFER_SRC = 1 << 0;
        /// Transfer destination
        const TRANSFER_DST = 1 << 1;
        /// Uniform texel buffer
        const UNIFORM_TEXEL_BUFFER = 1 << 2;
        /// Storage texel buffer
        const STORAGE_TEXEL_BUFFER = 1 << 3;
        /// Uniform buffer
        const UNIFORM_BUFFER = 1 << 4;
        /// Storage buffer
        const STORAGE_BUFFER = 1 << 5;
        /// Index buffer
        const INDEX_BUFFER = 1 << 6;
        /// Vertex buffer
        const VERTEX_BUFFER = 1 << 7;
        /// Indirect buffer
        const INDIRECT_BUFFER = 1 << 8;
        /// Shader device address
        const SHADER_DEVICE_ADDRESS = 1 << 17;
    }
}

bitflags::bitflags! {
    /// Sparse binding flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct SparseBindingFlags: u32 {
        /// Sparse binding
        const SPARSE_BINDING = 1 << 0;
        /// Sparse residency
        const SPARSE_RESIDENCY = 1 << 1;
        /// Sparse aliased
        const SPARSE_ALIASED = 1 << 2;
    }
}

impl SparseBindingFlags {
    /// No sparse binding
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}

/// Sparse image create info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SparseImageCreateInfo {
    /// Image type
    pub image_type: ImageType,
    /// Format
    pub format: ImageFormat,
    /// Extent
    pub extent: Extent3D,
    /// Mip levels
    pub mip_levels: u32,
    /// Array layers
    pub array_layers: u32,
    /// Samples
    pub samples: SampleCount,
    /// Usage
    pub usage: ImageUsageFlags,
    /// Sparse flags
    pub sparse_flags: SparseBindingFlags,
}

impl SparseImageCreateInfo {
    /// Creates a sparse 2D texture
    pub const fn texture_2d(width: u32, height: u32, format: ImageFormat) -> Self {
        Self {
            image_type: ImageType::Type2D,
            format,
            extent: Extent3D::d2(width, height),
            mip_levels: 1,
            array_layers: 1,
            samples: SampleCount::S1,
            usage: ImageUsageFlags::SAMPLED.union(ImageUsageFlags::TRANSFER_DST),
            sparse_flags: SparseBindingFlags::SPARSE_BINDING
                .union(SparseBindingFlags::SPARSE_RESIDENCY),
        }
    }

    /// Creates a sparse virtual texture
    pub const fn virtual_texture(width: u32, height: u32, format: ImageFormat, mips: u32) -> Self {
        Self {
            mip_levels: mips,
            ..Self::texture_2d(width, height, format)
        }
    }

    /// With full mip chain
    pub fn with_full_mip_chain(mut self) -> Self {
        let max_dim = self.extent.width.max(self.extent.height);
        self.mip_levels = (max_dim as f32).log2() as u32 + 1;
        self
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

/// Image format
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum ImageFormat {
    /// RGBA8 UNORM
    #[default]
    RGBA8      = 37,
    /// RGBA8 SRGB
    RGBA8_SRGB = 43,
    /// BGRA8 UNORM
    BGRA8      = 44,
    /// RGBA16F
    RGBA16F    = 97,
    /// BC1
    BC1        = 131,
    /// BC3
    BC3        = 137,
    /// BC7
    BC7        = 145,
}

/// Sample count
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum SampleCount {
    /// 1 sample
    #[default]
    S1 = 1,
    /// 4 samples
    S4 = 4,
    /// 8 samples
    S8 = 8,
}

bitflags::bitflags! {
    /// Image usage flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct ImageUsageFlags: u32 {
        /// Transfer source
        const TRANSFER_SRC = 1 << 0;
        /// Transfer destination
        const TRANSFER_DST = 1 << 1;
        /// Sampled
        const SAMPLED = 1 << 2;
        /// Storage
        const STORAGE = 1 << 3;
        /// Color attachment
        const COLOR_ATTACHMENT = 1 << 4;
        /// Depth-stencil attachment
        const DEPTH_STENCIL_ATTACHMENT = 1 << 5;
    }
}

/// Sparse image format properties
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SparseImageFormatProperties {
    /// Aspect mask
    pub aspect_mask: ImageAspectFlags,
    /// Image granularity (tile size)
    pub image_granularity: Extent3D,
    /// Flags
    pub flags: SparseImageFormatFlags,
}

impl SparseImageFormatProperties {
    /// Common 64KB tile
    pub const TILE_64KB: Self = Self {
        aspect_mask: ImageAspectFlags::COLOR,
        image_granularity: Extent3D::new(256, 256, 1), // For RGBA8
        flags: SparseImageFormatFlags::empty(),
    };
}

bitflags::bitflags! {
    /// Sparse image format flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct SparseImageFormatFlags: u32 {
        /// Single mip tail
        const SINGLE_MIPTAIL = 1 << 0;
        /// Aligned mip size
        const ALIGNED_MIP_SIZE = 1 << 1;
        /// Non-standard block size
        const NONSTANDARD_BLOCK_SIZE = 1 << 2;
    }
}

impl SparseImageFormatFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}

/// Sparse image memory requirements
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SparseImageMemoryRequirements {
    /// Format properties
    pub format_properties: SparseImageFormatProperties,
    /// First mip level in mip tail
    pub image_mip_tail_first_lod: u32,
    /// Mip tail size
    pub image_mip_tail_size: u64,
    /// Mip tail offset
    pub image_mip_tail_offset: u64,
    /// Mip tail stride
    pub image_mip_tail_stride: u64,
}

impl SparseImageMemoryRequirements {
    /// Has mip tail
    pub const fn has_mip_tail(&self) -> bool {
        self.image_mip_tail_size > 0
    }

    /// Number of sparse mip levels
    pub const fn sparse_mip_count(&self) -> u32 {
        self.image_mip_tail_first_lod
    }
}

/// Semaphore handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SemaphoreHandle(pub NonZeroU32);

impl SemaphoreHandle {
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

/// Bind sparse info
#[derive(Clone, Debug)]
pub struct BindSparseInfo {
    /// Wait semaphores
    pub wait_semaphores: alloc::vec::Vec<SemaphoreHandle>,
    /// Buffer binds
    pub buffer_binds: alloc::vec::Vec<SparseBufferMemoryBindInfo>,
    /// Image opaque binds
    pub image_opaque_binds: alloc::vec::Vec<SparseImageOpaqueMemoryBindInfo>,
    /// Image binds
    pub image_binds: alloc::vec::Vec<SparseImageMemoryBindInfo>,
    /// Signal semaphores
    pub signal_semaphores: alloc::vec::Vec<SemaphoreHandle>,
}

use alloc::vec::Vec;

impl BindSparseInfo {
    /// Creates empty bind info
    pub fn new() -> Self {
        Self {
            wait_semaphores: Vec::new(),
            buffer_binds: Vec::new(),
            image_opaque_binds: Vec::new(),
            image_binds: Vec::new(),
            signal_semaphores: Vec::new(),
        }
    }

    /// Adds a wait semaphore
    pub fn wait_on(mut self, semaphore: SemaphoreHandle) -> Self {
        self.wait_semaphores.push(semaphore);
        self
    }

    /// Adds a buffer bind
    pub fn bind_buffer(mut self, info: SparseBufferMemoryBindInfo) -> Self {
        self.buffer_binds.push(info);
        self
    }

    /// Adds an image bind
    pub fn bind_image(mut self, info: SparseImageMemoryBindInfo) -> Self {
        self.image_binds.push(info);
        self
    }

    /// Adds a signal semaphore
    pub fn signal(mut self, semaphore: SemaphoreHandle) -> Self {
        self.signal_semaphores.push(semaphore);
        self
    }
}

impl Default for BindSparseInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Sparse buffer memory bind info
#[derive(Clone, Debug)]
pub struct SparseBufferMemoryBindInfo {
    /// Buffer
    pub buffer: SparseBufferHandle,
    /// Binds
    pub binds: Vec<SparseMemoryBind>,
}

impl SparseBufferMemoryBindInfo {
    /// Creates buffer bind info
    pub fn new(buffer: SparseBufferHandle) -> Self {
        Self {
            buffer,
            binds: Vec::new(),
        }
    }

    /// Adds a bind
    pub fn add_bind(mut self, bind: SparseMemoryBind) -> Self {
        self.binds.push(bind);
        self
    }
}

/// Sparse image opaque memory bind info
#[derive(Clone, Debug)]
pub struct SparseImageOpaqueMemoryBindInfo {
    /// Image
    pub image: SparseImageHandle,
    /// Binds
    pub binds: Vec<SparseMemoryBind>,
}

impl SparseImageOpaqueMemoryBindInfo {
    /// Creates opaque bind info
    pub fn new(image: SparseImageHandle) -> Self {
        Self {
            image,
            binds: Vec::new(),
        }
    }

    /// Adds a bind
    pub fn add_bind(mut self, bind: SparseMemoryBind) -> Self {
        self.binds.push(bind);
        self
    }
}

/// Sparse image memory bind info
#[derive(Clone, Debug)]
pub struct SparseImageMemoryBindInfo {
    /// Image
    pub image: SparseImageHandle,
    /// Binds
    pub binds: Vec<SparseImageMemoryBind>,
}

impl SparseImageMemoryBindInfo {
    /// Creates image bind info
    pub fn new(image: SparseImageHandle) -> Self {
        Self {
            image,
            binds: Vec::new(),
        }
    }

    /// Adds a bind
    pub fn add_bind(mut self, bind: SparseImageMemoryBind) -> Self {
        self.binds.push(bind);
        self
    }
}

/// Virtual texture system configuration
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VirtualTextureConfig {
    /// Page size (typically 64KB)
    pub page_size: u32,
    /// Maximum resident pages
    pub max_resident_pages: u32,
    /// Page table format
    pub page_table_format: ImageFormat,
    /// Feedback buffer size
    pub feedback_buffer_size: u32,
}

impl VirtualTextureConfig {
    /// Default configuration
    pub const fn default() -> Self {
        Self {
            page_size: 65536,
            max_resident_pages: 4096,
            page_table_format: ImageFormat::RGBA8,
            feedback_buffer_size: 1024 * 1024,
        }
    }

    /// Low memory configuration
    pub const fn low_memory() -> Self {
        Self {
            page_size: 65536,
            max_resident_pages: 1024,
            page_table_format: ImageFormat::RGBA8,
            feedback_buffer_size: 256 * 1024,
        }
    }
}

impl Default for VirtualTextureConfig {
    fn default() -> Self {
        Self::default()
    }
}

/// Page request
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PageRequest {
    /// Texture ID
    pub texture_id: u32,
    /// Mip level
    pub mip_level: u32,
    /// Page X
    pub page_x: u32,
    /// Page Y
    pub page_y: u32,
}

impl PageRequest {
    /// Creates a page request
    pub const fn new(texture_id: u32, mip: u32, x: u32, y: u32) -> Self {
        Self {
            texture_id,
            mip_level: mip,
            page_x: x,
            page_y: y,
        }
    }

    /// Hash for deduplication
    pub const fn hash(&self) -> u64 {
        let t = self.texture_id as u64;
        let m = self.mip_level as u64;
        let x = self.page_x as u64;
        let y = self.page_y as u64;
        t | (m << 16) | (x << 24) | (y << 40)
    }
}
