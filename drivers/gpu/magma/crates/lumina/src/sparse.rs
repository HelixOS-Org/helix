//! Sparse resource types
//!
//! This module provides types for sparse binding and virtual allocations.

extern crate alloc;
use alloc::vec::Vec;

use crate::buffer::BufferHandle;
use crate::memory::DeviceMemory;
use crate::sync::SemaphoreHandle;
use crate::texture::TextureHandle;
use crate::types::Format;

/// Sparse buffer memory bind info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SparseBufferMemoryBind {
    /// Offset into the buffer resource
    pub resource_offset: u64,
    /// Size of the memory region to bind
    pub size: u64,
    /// Memory to bind (None to unbind)
    pub memory: Option<DeviceMemory>,
    /// Offset into the memory
    pub memory_offset: u64,
    /// Bind flags
    pub flags: SparseMemoryBindFlags,
}

impl SparseBufferMemoryBind {
    /// Creates a new sparse buffer bind
    pub const fn new(resource_offset: u64, size: u64, memory: DeviceMemory, memory_offset: u64) -> Self {
        Self {
            resource_offset,
            size,
            memory: Some(memory),
            memory_offset,
            flags: SparseMemoryBindFlags::NONE,
        }
    }

    /// Creates an unbind operation
    pub const fn unbind(resource_offset: u64, size: u64) -> Self {
        Self {
            resource_offset,
            size,
            memory: None,
            memory_offset: 0,
            flags: SparseMemoryBindFlags::NONE,
        }
    }
}

/// Sparse memory bind flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct SparseMemoryBindFlags(pub u32);

impl SparseMemoryBindFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Bind metadata
    pub const METADATA: Self = Self(1 << 0);
}

/// Sparse image memory bind info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SparseImageMemoryBind {
    /// Image subresource
    pub subresource: ImageSubresource,
    /// Offset in texels
    pub offset: ImageOffset,
    /// Extent in texels
    pub extent: ImageExtent,
    /// Memory to bind
    pub memory: Option<DeviceMemory>,
    /// Offset into memory
    pub memory_offset: u64,
    /// Bind flags
    pub flags: SparseMemoryBindFlags,
}

impl SparseImageMemoryBind {
    /// Creates a new sparse image bind
    pub const fn new(
        subresource: ImageSubresource,
        offset: ImageOffset,
        extent: ImageExtent,
        memory: DeviceMemory,
        memory_offset: u64,
    ) -> Self {
        Self {
            subresource,
            offset,
            extent,
            memory: Some(memory),
            memory_offset,
            flags: SparseMemoryBindFlags::NONE,
        }
    }
}

/// Image subresource for sparse binding
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ImageSubresource {
    /// Aspect mask
    pub aspect: ImageAspect,
    /// Mip level
    pub mip_level: u32,
    /// Array layer
    pub array_layer: u32,
}

impl ImageSubresource {
    /// Creates color subresource
    pub const fn color(mip_level: u32, array_layer: u32) -> Self {
        Self {
            aspect: ImageAspect::Color,
            mip_level,
            array_layer,
        }
    }
}

/// Image aspect
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum ImageAspect {
    /// Color aspect
    #[default]
    Color,
    /// Depth aspect
    Depth,
    /// Stencil aspect
    Stencil,
    /// Metadata aspect
    Metadata,
}

/// Image offset
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ImageOffset {
    /// X offset
    pub x: i32,
    /// Y offset
    pub y: i32,
    /// Z offset
    pub z: i32,
}

impl ImageOffset {
    /// Zero offset
    pub const ZERO: Self = Self { x: 0, y: 0, z: 0 };

    /// Creates a new offset
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

/// Image extent
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ImageExtent {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Depth
    pub depth: u32,
}

impl ImageExtent {
    /// Creates a new extent
    pub const fn new(width: u32, height: u32, depth: u32) -> Self {
        Self { width, height, depth }
    }

    /// Creates a 2D extent
    pub const fn d2(width: u32, height: u32) -> Self {
        Self::new(width, height, 1)
    }
}

/// Sparse buffer bind info
#[derive(Clone, Debug)]
pub struct SparseBufferBindInfo {
    /// Buffer to bind
    pub buffer: BufferHandle,
    /// Binds for this buffer
    pub binds: Vec<SparseBufferMemoryBind>,
}

impl SparseBufferBindInfo {
    /// Creates new sparse buffer bind info
    pub fn new(buffer: BufferHandle) -> Self {
        Self {
            buffer,
            binds: Vec::new(),
        }
    }

    /// Adds a bind
    pub fn add_bind(mut self, bind: SparseBufferMemoryBind) -> Self {
        self.binds.push(bind);
        self
    }
}

/// Sparse image opaque bind info
#[derive(Clone, Debug)]
pub struct SparseImageOpaqueBindInfo {
    /// Image to bind
    pub image: TextureHandle,
    /// Opaque binds (for mip tail)
    pub binds: Vec<SparseBufferMemoryBind>,
}

impl SparseImageOpaqueBindInfo {
    /// Creates new sparse image opaque bind info
    pub fn new(image: TextureHandle) -> Self {
        Self {
            image,
            binds: Vec::new(),
        }
    }
}

/// Sparse image bind info
#[derive(Clone, Debug)]
pub struct SparseImageBindInfo {
    /// Image to bind
    pub image: TextureHandle,
    /// Binds for this image
    pub binds: Vec<SparseImageMemoryBind>,
}

impl SparseImageBindInfo {
    /// Creates new sparse image bind info
    pub fn new(image: TextureHandle) -> Self {
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

/// Bind sparse info
#[derive(Clone, Debug, Default)]
pub struct BindSparseInfo {
    /// Semaphores to wait on
    pub wait_semaphores: Vec<SemaphoreHandle>,
    /// Buffer binds
    pub buffer_binds: Vec<SparseBufferBindInfo>,
    /// Image opaque binds
    pub image_opaque_binds: Vec<SparseImageOpaqueBindInfo>,
    /// Image binds
    pub image_binds: Vec<SparseImageBindInfo>,
    /// Semaphores to signal
    pub signal_semaphores: Vec<SemaphoreHandle>,
}

impl BindSparseInfo {
    /// Creates new bind sparse info
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a buffer bind
    pub fn add_buffer_bind(mut self, bind: SparseBufferBindInfo) -> Self {
        self.buffer_binds.push(bind);
        self
    }

    /// Adds an image bind
    pub fn add_image_bind(mut self, bind: SparseImageBindInfo) -> Self {
        self.image_binds.push(bind);
        self
    }

    /// Adds a wait semaphore
    pub fn wait_on(mut self, semaphore: SemaphoreHandle) -> Self {
        self.wait_semaphores.push(semaphore);
        self
    }

    /// Adds a signal semaphore
    pub fn signal(mut self, semaphore: SemaphoreHandle) -> Self {
        self.signal_semaphores.push(semaphore);
        self
    }
}

/// Sparse image format properties
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SparseImageFormatProperties {
    /// Sparse image aspect flags
    pub aspect_mask: SparseImageAspectFlags,
    /// Sparse image granularity
    pub image_granularity: ImageExtent,
    /// Sparse image flags
    pub flags: SparseImageFormatFlags,
}

/// Sparse image aspect flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct SparseImageAspectFlags(pub u32);

impl SparseImageAspectFlags {
    /// Color aspect
    pub const COLOR: Self = Self(1 << 0);
    /// Depth aspect
    pub const DEPTH: Self = Self(1 << 1);
    /// Stencil aspect
    pub const STENCIL: Self = Self(1 << 2);
    /// Metadata aspect
    pub const METADATA: Self = Self(1 << 3);
}

/// Sparse image format flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct SparseImageFormatFlags(pub u32);

impl SparseImageFormatFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Single mip tail
    pub const SINGLE_MIPTAIL: Self = Self(1 << 0);
    /// Aligned mip size
    pub const ALIGNED_MIP_SIZE: Self = Self(1 << 1);
    /// Non-standard block size
    pub const NONSTANDARD_BLOCK_SIZE: Self = Self(1 << 2);
}

/// Sparse image memory requirements
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SparseImageMemoryRequirements {
    /// Format properties
    pub format_properties: SparseImageFormatProperties,
    /// First mip level in the mip tail
    pub image_mip_tail_first_lod: u32,
    /// Size of the mip tail region
    pub image_mip_tail_size: u64,
    /// Offset to the mip tail
    pub image_mip_tail_offset: u64,
    /// Stride between array layers in mip tail
    pub image_mip_tail_stride: u64,
}

/// Sparse resource memory type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SparseResidencyType {
    /// No sparse residency
    None,
    /// Buffer sparse residency
    Buffer,
    /// Image 2D sparse residency
    Image2D,
    /// Image 3D sparse residency
    Image3D,
    /// Multi-sample 2x sparse residency
    Image2DMultisample2,
    /// Multi-sample 4x sparse residency
    Image2DMultisample4,
    /// Multi-sample 8x sparse residency
    Image2DMultisample8,
    /// Multi-sample 16x sparse residency
    Image2DMultisample16,
}

/// Sparse texture info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SparseTextureInfo {
    /// Number of mip levels
    pub mip_levels: u32,
    /// Number of array layers
    pub array_layers: u32,
    /// Sample count
    pub samples: u32,
    /// Texture format
    pub format: Format,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Depth
    pub depth: u32,
}

impl SparseTextureInfo {
    /// Creates 2D texture info
    pub const fn d2(width: u32, height: u32, format: Format) -> Self {
        Self {
            mip_levels: 1,
            array_layers: 1,
            samples: 1,
            format,
            width,
            height,
            depth: 1,
        }
    }

    /// Sets mip levels
    pub const fn with_mip_levels(mut self, levels: u32) -> Self {
        self.mip_levels = levels;
        self
    }

    /// Sets array layers
    pub const fn with_array_layers(mut self, layers: u32) -> Self {
        self.array_layers = layers;
        self
    }
}

/// Virtual allocation handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct VirtualAllocationHandle(pub u64);

impl VirtualAllocationHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

/// Virtual allocation info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct VirtualAllocationInfo {
    /// Size of the allocation
    pub size: u64,
    /// Alignment requirement
    pub alignment: u64,
    /// User data
    pub user_data: u64,
}

impl VirtualAllocationInfo {
    /// Creates new allocation info
    pub const fn new(size: u64) -> Self {
        Self {
            size,
            alignment: 1,
            user_data: 0,
        }
    }

    /// Sets alignment
    pub const fn with_alignment(mut self, alignment: u64) -> Self {
        self.alignment = alignment;
        self
    }
}

/// Virtual block handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct VirtualBlockHandle(pub u64);

impl VirtualBlockHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

/// Virtual block create info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct VirtualBlockCreateInfo {
    /// Size of the virtual block
    pub size: u64,
    /// Flags for creation
    pub flags: VirtualBlockFlags,
}

impl VirtualBlockCreateInfo {
    /// Creates new virtual block info
    pub const fn new(size: u64) -> Self {
        Self {
            size,
            flags: VirtualBlockFlags::NONE,
        }
    }
}

/// Virtual block flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct VirtualBlockFlags(pub u32);

impl VirtualBlockFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Use linear algorithm
    pub const LINEAR: Self = Self(1 << 0);
}

/// Page table entry for virtual textures
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct PageTableEntry {
    /// Physical page index (u32::MAX if not resident)
    pub physical_page: u32,
    /// Flags
    pub flags: PageFlags,
}

impl PageTableEntry {
    /// Non-resident page
    pub const NON_RESIDENT: Self = Self {
        physical_page: u32::MAX,
        flags: PageFlags::NONE,
    };

    /// Creates a resident page entry
    pub const fn resident(page: u32) -> Self {
        Self {
            physical_page: page,
            flags: PageFlags::RESIDENT,
        }
    }
}

/// Page flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct PageFlags(pub u32);

impl PageFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Page is resident
    pub const RESIDENT: Self = Self(1 << 0);
    /// Page is dirty
    pub const DIRTY: Self = Self(1 << 1);
    /// Page is locked
    pub const LOCKED: Self = Self(1 << 2);
}
