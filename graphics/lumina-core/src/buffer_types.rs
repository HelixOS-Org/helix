//! Buffer types and management
//!
//! This module provides comprehensive types for GPU buffer creation and usage.

use core::num::NonZeroU32;

/// Buffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BufferHandle(pub NonZeroU32);

impl BufferHandle {
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

/// Buffer view handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BufferViewHandle(pub NonZeroU32);

impl BufferViewHandle {
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

/// Buffer creation info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BufferCreateInfo {
    /// Size in bytes
    pub size: u64,
    /// Usage flags
    pub usage: BufferUsageFlags,
    /// Sharing mode
    pub sharing_mode: SharingMode,
    /// Creation flags
    pub flags: BufferCreateFlags,
}

/// Sharing mode
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum SharingMode {
    /// Exclusive access
    #[default]
    Exclusive  = 0,
    /// Concurrent access
    Concurrent = 1,
}

impl BufferCreateInfo {
    /// Creates a vertex buffer
    pub const fn vertex(size: u64) -> Self {
        Self {
            size,
            usage: BufferUsageFlags::VERTEX_BUFFER,
            sharing_mode: SharingMode::Exclusive,
            flags: BufferCreateFlags::empty(),
        }
    }

    /// Creates an index buffer
    pub const fn index(size: u64) -> Self {
        Self {
            size,
            usage: BufferUsageFlags::INDEX_BUFFER,
            sharing_mode: SharingMode::Exclusive,
            flags: BufferCreateFlags::empty(),
        }
    }

    /// Creates a uniform buffer
    pub const fn uniform(size: u64) -> Self {
        Self {
            size,
            usage: BufferUsageFlags::UNIFORM_BUFFER,
            sharing_mode: SharingMode::Exclusive,
            flags: BufferCreateFlags::empty(),
        }
    }

    /// Creates a storage buffer
    pub const fn storage(size: u64) -> Self {
        Self {
            size,
            usage: BufferUsageFlags::STORAGE_BUFFER,
            sharing_mode: SharingMode::Exclusive,
            flags: BufferCreateFlags::empty(),
        }
    }

    /// Creates a staging buffer
    pub const fn staging(size: u64) -> Self {
        Self {
            size,
            usage: BufferUsageFlags::TRANSFER_SRC,
            sharing_mode: SharingMode::Exclusive,
            flags: BufferCreateFlags::empty(),
        }
    }

    /// Creates an indirect buffer
    pub const fn indirect(size: u64) -> Self {
        Self {
            size,
            usage: BufferUsageFlags::INDIRECT_BUFFER,
            sharing_mode: SharingMode::Exclusive,
            flags: BufferCreateFlags::empty(),
        }
    }

    /// Creates an acceleration structure buffer
    pub const fn acceleration_structure(size: u64) -> Self {
        Self {
            size,
            usage: BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE,
            sharing_mode: SharingMode::Exclusive,
            flags: BufferCreateFlags::empty(),
        }
    }

    /// Adds transfer source capability
    pub const fn with_transfer_src(mut self) -> Self {
        self.usage = BufferUsageFlags::from_bits_truncate(
            self.usage.bits() | BufferUsageFlags::TRANSFER_SRC.bits(),
        );
        self
    }

    /// Adds transfer destination capability
    pub const fn with_transfer_dst(mut self) -> Self {
        self.usage = BufferUsageFlags::from_bits_truncate(
            self.usage.bits() | BufferUsageFlags::TRANSFER_DST.bits(),
        );
        self
    }

    /// Enables device address
    pub const fn with_device_address(mut self) -> Self {
        self.usage = BufferUsageFlags::from_bits_truncate(
            self.usage.bits() | BufferUsageFlags::SHADER_DEVICE_ADDRESS.bits(),
        );
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
        const SHADER_DEVICE_ADDRESS = 1 << 9;
        /// Conditional rendering
        const CONDITIONAL_RENDERING = 1 << 10;
        /// Transform feedback buffer
        const TRANSFORM_FEEDBACK_BUFFER = 1 << 11;
        /// Transform feedback counter buffer
        const TRANSFORM_FEEDBACK_COUNTER_BUFFER = 1 << 12;
        /// Acceleration structure build input read-only
        const ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY = 1 << 13;
        /// Acceleration structure storage
        const ACCELERATION_STRUCTURE_STORAGE = 1 << 14;
        /// Shader binding table
        const SHADER_BINDING_TABLE = 1 << 15;
    }
}

impl BufferUsageFlags {
    /// Vertex buffer with upload support
    pub const VERTEX_UPLOAD: Self =
        Self::from_bits_truncate(Self::VERTEX_BUFFER.bits() | Self::TRANSFER_DST.bits());

    /// Index buffer with upload support
    pub const INDEX_UPLOAD: Self =
        Self::from_bits_truncate(Self::INDEX_BUFFER.bits() | Self::TRANSFER_DST.bits());

    /// Uniform buffer with upload support
    pub const UNIFORM_UPLOAD: Self =
        Self::from_bits_truncate(Self::UNIFORM_BUFFER.bits() | Self::TRANSFER_DST.bits());

    /// Storage buffer with readback
    pub const STORAGE_READBACK: Self =
        Self::from_bits_truncate(Self::STORAGE_BUFFER.bits() | Self::TRANSFER_SRC.bits());
}

bitflags::bitflags! {
    /// Buffer creation flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct BufferCreateFlags: u32 {
        /// Sparse binding
        const SPARSE_BINDING = 1 << 0;
        /// Sparse residency
        const SPARSE_RESIDENCY = 1 << 1;
        /// Sparse aliased
        const SPARSE_ALIASED = 1 << 2;
        /// Device address capture replay
        const DEVICE_ADDRESS_CAPTURE_REPLAY = 1 << 4;
    }
}

impl BufferCreateFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}

/// Buffer view creation info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BufferViewCreateInfo {
    /// Buffer to view
    pub buffer: BufferHandle,
    /// Format
    pub format: Format,
    /// Offset in bytes
    pub offset: u64,
    /// Range in bytes
    pub range: u64,
}

/// Format (simplified, full version in image_types)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum Format {
    /// Undefined
    #[default]
    Undefined          = 0,
    /// R32 float
    R32Sfloat          = 100,
    /// R32G32 float
    R32g32Sfloat       = 103,
    /// R32G32B32 float
    R32g32b32Sfloat    = 106,
    /// R32G32B32A32 float
    R32g32b32a32Sfloat = 109,
    /// R32 uint
    R32Uint            = 98,
    /// R32G32 uint
    R32g32Uint         = 101,
    /// R32G32B32 uint
    R32g32b32Uint      = 104,
    /// R32G32B32A32 uint
    R32g32b32a32Uint   = 107,
    /// R16G16B16A16 float
    R16g16b16a16Sfloat = 97,
}

impl BufferViewCreateInfo {
    /// Creates a view of the whole buffer
    pub const fn whole(buffer: BufferHandle, format: Format) -> Self {
        Self {
            buffer,
            format,
            offset: 0,
            range: u64::MAX,
        }
    }

    /// Creates a view of a range
    pub const fn range(buffer: BufferHandle, format: Format, offset: u64, range: u64) -> Self {
        Self {
            buffer,
            format,
            offset,
            range,
        }
    }
}

/// Buffer device address info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BufferDeviceAddressInfo {
    /// Buffer handle
    pub buffer: BufferHandle,
}

/// Device address
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct DeviceAddress(pub u64);

impl DeviceAddress {
    /// Null address
    pub const NULL: Self = Self(0);

    /// Creates from raw address
    pub const fn from_raw(addr: u64) -> Self {
        Self(addr)
    }

    /// Gets the raw address
    pub const fn raw(&self) -> u64 {
        self.0
    }

    /// Is this a null address
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }

    /// Adds an offset
    pub const fn offset(&self, bytes: u64) -> Self {
        Self(self.0 + bytes)
    }
}

/// Buffer copy region
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BufferCopyRegion {
    /// Source offset
    pub src_offset: u64,
    /// Destination offset
    pub dst_offset: u64,
    /// Size in bytes
    pub size: u64,
}

impl BufferCopyRegion {
    /// Whole buffer copy (assumes equal sizes)
    pub const fn whole(size: u64) -> Self {
        Self {
            src_offset: 0,
            dst_offset: 0,
            size,
        }
    }

    /// Copy with offsets
    pub const fn with_offsets(src_offset: u64, dst_offset: u64, size: u64) -> Self {
        Self {
            src_offset,
            dst_offset,
            size,
        }
    }
}

/// Memory requirements
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MemoryRequirements {
    /// Size in bytes
    pub size: u64,
    /// Alignment in bytes
    pub alignment: u64,
    /// Memory type bits
    pub memory_type_bits: u32,
}

impl MemoryRequirements {
    /// Calculates aligned offset
    pub const fn aligned_offset(&self, offset: u64) -> u64 {
        (offset + self.alignment - 1) & !(self.alignment - 1)
    }
}

/// Mapped memory range
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MappedMemoryRange {
    /// Memory handle
    pub memory: DeviceMemoryHandle,
    /// Offset
    pub offset: u64,
    /// Size
    pub size: u64,
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

impl MappedMemoryRange {
    /// Whole memory
    pub const fn whole(memory: DeviceMemoryHandle) -> Self {
        Self {
            memory,
            offset: 0,
            size: u64::MAX,
        }
    }

    /// Range
    pub const fn range(memory: DeviceMemoryHandle, offset: u64, size: u64) -> Self {
        Self {
            memory,
            offset,
            size,
        }
    }
}

/// Bind buffer memory info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BindBufferMemoryInfo {
    /// Buffer to bind
    pub buffer: BufferHandle,
    /// Memory to bind
    pub memory: DeviceMemoryHandle,
    /// Offset within memory
    pub memory_offset: u64,
}

/// Buffer memory barrier
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BufferMemoryBarrier {
    /// Source access mask
    pub src_access_mask: AccessFlags,
    /// Destination access mask
    pub dst_access_mask: AccessFlags,
    /// Source queue family
    pub src_queue_family_index: u32,
    /// Destination queue family
    pub dst_queue_family_index: u32,
    /// Buffer handle
    pub buffer: BufferHandle,
    /// Offset
    pub offset: u64,
    /// Size
    pub size: u64,
}

/// Queue family ignored value
pub const QUEUE_FAMILY_IGNORED: u32 = u32::MAX;
/// Whole size value
pub const WHOLE_SIZE: u64 = u64::MAX;

impl BufferMemoryBarrier {
    /// Creates a barrier for the whole buffer
    pub const fn whole(buffer: BufferHandle) -> Self {
        Self {
            src_access_mask: AccessFlags::empty(),
            dst_access_mask: AccessFlags::empty(),
            src_queue_family_index: QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: QUEUE_FAMILY_IGNORED,
            buffer,
            offset: 0,
            size: WHOLE_SIZE,
        }
    }

    /// Sets source access
    pub const fn from_access(mut self, access: AccessFlags) -> Self {
        self.src_access_mask = access;
        self
    }

    /// Sets destination access
    pub const fn to_access(mut self, access: AccessFlags) -> Self {
        self.dst_access_mask = access;
        self
    }
}

bitflags::bitflags! {
    /// Access flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct AccessFlags: u32 {
        /// Indirect command read
        const INDIRECT_COMMAND_READ = 1 << 0;
        /// Index read
        const INDEX_READ = 1 << 1;
        /// Vertex attribute read
        const VERTEX_ATTRIBUTE_READ = 1 << 2;
        /// Uniform read
        const UNIFORM_READ = 1 << 3;
        /// Input attachment read
        const INPUT_ATTACHMENT_READ = 1 << 4;
        /// Shader read
        const SHADER_READ = 1 << 5;
        /// Shader write
        const SHADER_WRITE = 1 << 6;
        /// Color attachment read
        const COLOR_ATTACHMENT_READ = 1 << 7;
        /// Color attachment write
        const COLOR_ATTACHMENT_WRITE = 1 << 8;
        /// Depth-stencil attachment read
        const DEPTH_STENCIL_ATTACHMENT_READ = 1 << 9;
        /// Depth-stencil attachment write
        const DEPTH_STENCIL_ATTACHMENT_WRITE = 1 << 10;
        /// Transfer read
        const TRANSFER_READ = 1 << 11;
        /// Transfer write
        const TRANSFER_WRITE = 1 << 12;
        /// Host read
        const HOST_READ = 1 << 13;
        /// Host write
        const HOST_WRITE = 1 << 14;
        /// Memory read
        const MEMORY_READ = 1 << 15;
        /// Memory write
        const MEMORY_WRITE = 1 << 16;
    }
}

impl AccessFlags {
    /// No access
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }

    /// All read access
    pub const ALL_READ: Self = Self::from_bits_truncate(
        Self::INDIRECT_COMMAND_READ.bits()
            | Self::INDEX_READ.bits()
            | Self::VERTEX_ATTRIBUTE_READ.bits()
            | Self::UNIFORM_READ.bits()
            | Self::SHADER_READ.bits()
            | Self::COLOR_ATTACHMENT_READ.bits()
            | Self::DEPTH_STENCIL_ATTACHMENT_READ.bits()
            | Self::TRANSFER_READ.bits()
            | Self::HOST_READ.bits()
            | Self::MEMORY_READ.bits(),
    );

    /// All write access
    pub const ALL_WRITE: Self = Self::from_bits_truncate(
        Self::SHADER_WRITE.bits()
            | Self::COLOR_ATTACHMENT_WRITE.bits()
            | Self::DEPTH_STENCIL_ATTACHMENT_WRITE.bits()
            | Self::TRANSFER_WRITE.bits()
            | Self::HOST_WRITE.bits()
            | Self::MEMORY_WRITE.bits(),
    );
}

/// Sparse buffer memory bind info
#[derive(Clone, Debug)]
pub struct SparseBufferMemoryBindInfo {
    /// Buffer
    pub buffer: BufferHandle,
    /// Binds
    pub binds: alloc::vec::Vec<SparseMemoryBind>,
}

/// Sparse memory bind
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SparseMemoryBind {
    /// Resource offset
    pub resource_offset: u64,
    /// Size
    pub size: u64,
    /// Memory
    pub memory: Option<DeviceMemoryHandle>,
    /// Memory offset
    pub memory_offset: u64,
    /// Flags
    pub flags: SparseMemoryBindFlags,
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

use alloc::vec::Vec;

/// Ring buffer for streaming data
#[derive(Clone, Debug)]
pub struct RingBuffer {
    /// Buffer handle
    pub buffer: BufferHandle,
    /// Total size
    pub size: u64,
    /// Write offset
    pub write_offset: u64,
    /// Frame offsets for multi-buffering
    pub frame_offsets: alloc::vec::Vec<u64>,
}

impl RingBuffer {
    /// Creates a new ring buffer descriptor
    pub fn new(buffer: BufferHandle, size: u64, frame_count: usize) -> Self {
        Self {
            buffer,
            size,
            write_offset: 0,
            frame_offsets: alloc::vec![0; frame_count],
        }
    }

    /// Allocates space in the ring buffer
    pub fn allocate(&mut self, size: u64, alignment: u64) -> Option<u64> {
        let aligned_offset = (self.write_offset + alignment - 1) & !(alignment - 1);

        if aligned_offset + size > self.size {
            // Wrap around
            if size > self.size {
                return None;
            }
            self.write_offset = size;
            Some(0)
        } else {
            self.write_offset = aligned_offset + size;
            Some(aligned_offset)
        }
    }

    /// Resets the ring buffer
    pub fn reset(&mut self) {
        self.write_offset = 0;
    }
}
