//! Synchronization primitives for GPU operations
//!
//! This module provides types for semaphores, fences, events, and barriers.

use core::num::NonZeroU32;

/// Semaphore type
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum SemaphoreType {
    /// Binary semaphore (signaled/unsignaled)
    #[default]
    Binary = 0,
    /// Timeline semaphore (counter-based)
    Timeline = 1,
}

/// Semaphore creation info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SemaphoreCreateInfo {
    /// Semaphore type
    pub semaphore_type: SemaphoreType,
    /// Initial value (for timeline semaphores)
    pub initial_value: u64,
    /// Flags
    pub flags: SemaphoreCreateFlags,
}

impl SemaphoreCreateInfo {
    /// Creates a binary semaphore
    pub const fn binary() -> Self {
        Self {
            semaphore_type: SemaphoreType::Binary,
            initial_value: 0,
            flags: SemaphoreCreateFlags::empty(),
        }
    }

    /// Creates a timeline semaphore
    pub const fn timeline(initial_value: u64) -> Self {
        Self {
            semaphore_type: SemaphoreType::Timeline,
            initial_value,
            flags: SemaphoreCreateFlags::empty(),
        }
    }
}

impl Default for SemaphoreCreateInfo {
    fn default() -> Self {
        Self::binary()
    }
}

bitflags::bitflags! {
    /// Semaphore creation flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct SemaphoreCreateFlags: u32 {
        // Reserved for future use
    }
}

impl SemaphoreCreateFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}

/// Timeline semaphore wait info
#[derive(Clone, Debug)]
pub struct SemaphoreWaitInfo {
    /// Semaphores to wait on
    pub semaphores: alloc::vec::Vec<SemaphoreHandle>,
    /// Values to wait for
    pub values: alloc::vec::Vec<u64>,
    /// Wait flags
    pub flags: SemaphoreWaitFlags,
}

use alloc::vec::Vec;

use crate::command_buffer::SemaphoreHandle;

impl SemaphoreWaitInfo {
    /// Creates wait info
    pub fn new() -> Self {
        Self {
            semaphores: Vec::new(),
            values: Vec::new(),
            flags: SemaphoreWaitFlags::empty(),
        }
    }

    /// Adds a semaphore to wait on
    pub fn wait_on(mut self, semaphore: SemaphoreHandle, value: u64) -> Self {
        self.semaphores.push(semaphore);
        self.values.push(value);
        self
    }

    /// Waits for any semaphore (instead of all)
    pub fn wait_any(mut self) -> Self {
        self.flags |= SemaphoreWaitFlags::ANY;
        self
    }
}

impl Default for SemaphoreWaitInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Semaphore wait flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct SemaphoreWaitFlags: u32 {
        /// Wait for any semaphore instead of all
        const ANY = 1 << 0;
    }
}

impl SemaphoreWaitFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}

/// Timeline semaphore signal info
#[derive(Clone, Debug)]
pub struct SemaphoreSignalInfo {
    /// Semaphore to signal
    pub semaphore: SemaphoreHandle,
    /// Value to signal
    pub value: u64,
}

impl SemaphoreSignalInfo {
    /// Creates signal info
    pub const fn new(semaphore: SemaphoreHandle, value: u64) -> Self {
        Self { semaphore, value }
    }
}

/// Fence handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct FenceHandle(pub NonZeroU32);

impl FenceHandle {
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

/// Fence creation info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FenceCreateInfo {
    /// Flags
    pub flags: FenceCreateFlags,
}

impl FenceCreateInfo {
    /// Creates unsignaled fence
    pub const fn unsignaled() -> Self {
        Self {
            flags: FenceCreateFlags::empty(),
        }
    }

    /// Creates signaled fence
    pub const fn signaled() -> Self {
        Self {
            flags: FenceCreateFlags::SIGNALED,
        }
    }
}

impl Default for FenceCreateInfo {
    fn default() -> Self {
        Self::unsignaled()
    }
}

bitflags::bitflags! {
    /// Fence creation flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct FenceCreateFlags: u32 {
        /// Fence starts in signaled state
        const SIGNALED = 1 << 0;
    }
}

impl FenceCreateFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}

/// Event handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct EventHandle(pub NonZeroU32);

impl EventHandle {
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

/// Event creation info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct EventCreateInfo {
    /// Flags
    pub flags: EventCreateFlags,
}

impl EventCreateInfo {
    /// Creates event info
    pub const fn new() -> Self {
        Self {
            flags: EventCreateFlags::empty(),
        }
    }

    /// Device-only event
    pub const fn device_only() -> Self {
        Self {
            flags: EventCreateFlags::DEVICE_ONLY,
        }
    }
}

impl Default for EventCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Event creation flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct EventCreateFlags: u32 {
        /// Event is device-only (VK 1.3)
        const DEVICE_ONLY = 1 << 0;
    }
}

impl EventCreateFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}

/// Memory barrier
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MemoryBarrier2 {
    /// Source stage mask
    pub src_stage_mask: PipelineStageFlags2,
    /// Source access mask
    pub src_access_mask: AccessFlags2,
    /// Destination stage mask
    pub dst_stage_mask: PipelineStageFlags2,
    /// Destination access mask
    pub dst_access_mask: AccessFlags2,
}

use crate::command_buffer::PipelineStageFlags2;

impl MemoryBarrier2 {
    /// Full pipeline barrier
    pub const fn full() -> Self {
        Self {
            src_stage_mask: PipelineStageFlags2::ALL_COMMANDS,
            src_access_mask: AccessFlags2::MEMORY_READ.union(AccessFlags2::MEMORY_WRITE),
            dst_stage_mask: PipelineStageFlags2::ALL_COMMANDS,
            dst_access_mask: AccessFlags2::MEMORY_READ.union(AccessFlags2::MEMORY_WRITE),
        }
    }

    /// Compute to compute barrier
    pub const fn compute_to_compute() -> Self {
        Self {
            src_stage_mask: PipelineStageFlags2::COMPUTE_SHADER,
            src_access_mask: AccessFlags2::SHADER_WRITE,
            dst_stage_mask: PipelineStageFlags2::COMPUTE_SHADER,
            dst_access_mask: AccessFlags2::SHADER_READ,
        }
    }

    /// Compute to fragment barrier
    pub const fn compute_to_fragment() -> Self {
        Self {
            src_stage_mask: PipelineStageFlags2::COMPUTE_SHADER,
            src_access_mask: AccessFlags2::SHADER_WRITE,
            dst_stage_mask: PipelineStageFlags2::FRAGMENT_SHADER,
            dst_access_mask: AccessFlags2::SHADER_READ,
        }
    }

    /// Transfer to shader barrier
    pub const fn transfer_to_shader() -> Self {
        Self {
            src_stage_mask: PipelineStageFlags2::TRANSFER,
            src_access_mask: AccessFlags2::TRANSFER_WRITE,
            dst_stage_mask: PipelineStageFlags2::ALL_GRAPHICS,
            dst_access_mask: AccessFlags2::SHADER_READ,
        }
    }
}

/// Buffer memory barrier
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BufferMemoryBarrier2 {
    /// Source stage mask
    pub src_stage_mask: PipelineStageFlags2,
    /// Source access mask
    pub src_access_mask: AccessFlags2,
    /// Destination stage mask
    pub dst_stage_mask: PipelineStageFlags2,
    /// Destination access mask
    pub dst_access_mask: AccessFlags2,
    /// Source queue family
    pub src_queue_family_index: u32,
    /// Destination queue family
    pub dst_queue_family_index: u32,
    /// Buffer handle
    pub buffer: BufferHandle,
    /// Offset in buffer
    pub offset: u64,
    /// Size of region
    pub size: u64,
}

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

/// Queue family index for ignored transitions
pub const QUEUE_FAMILY_IGNORED: u32 = u32::MAX;

/// Whole size constant
pub const WHOLE_SIZE: u64 = u64::MAX;

impl BufferMemoryBarrier2 {
    /// Creates a buffer barrier
    pub fn new(buffer: BufferHandle) -> Self {
        Self {
            src_stage_mask: PipelineStageFlags2::NONE,
            src_access_mask: AccessFlags2::NONE,
            dst_stage_mask: PipelineStageFlags2::NONE,
            dst_access_mask: AccessFlags2::NONE,
            src_queue_family_index: QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: QUEUE_FAMILY_IGNORED,
            buffer,
            offset: 0,
            size: WHOLE_SIZE,
        }
    }

    /// Sets source access
    pub const fn from(mut self, stage: PipelineStageFlags2, access: AccessFlags2) -> Self {
        self.src_stage_mask = stage;
        self.src_access_mask = access;
        self
    }

    /// Sets destination access
    pub const fn to(mut self, stage: PipelineStageFlags2, access: AccessFlags2) -> Self {
        self.dst_stage_mask = stage;
        self.dst_access_mask = access;
        self
    }

    /// Sets range
    pub const fn range(mut self, offset: u64, size: u64) -> Self {
        self.offset = offset;
        self.size = size;
        self
    }

    /// Queue family transfer
    pub const fn queue_family_transfer(mut self, src_family: u32, dst_family: u32) -> Self {
        self.src_queue_family_index = src_family;
        self.dst_queue_family_index = dst_family;
        self
    }
}

/// Image memory barrier
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ImageMemoryBarrier2 {
    /// Source stage mask
    pub src_stage_mask: PipelineStageFlags2,
    /// Source access mask
    pub src_access_mask: AccessFlags2,
    /// Destination stage mask
    pub dst_stage_mask: PipelineStageFlags2,
    /// Destination access mask
    pub dst_access_mask: AccessFlags2,
    /// Old image layout
    pub old_layout: ImageLayout,
    /// New image layout
    pub new_layout: ImageLayout,
    /// Source queue family
    pub src_queue_family_index: u32,
    /// Destination queue family
    pub dst_queue_family_index: u32,
    /// Image handle
    pub image: ImageHandle,
    /// Subresource range
    pub subresource_range: ImageSubresourceRange,
}

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

/// Image layout
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum ImageLayout {
    /// Undefined layout (initial or don't care)
    #[default]
    Undefined = 0,
    /// General layout (all operations, suboptimal)
    General = 1,
    /// Optimal for color attachment
    ColorAttachmentOptimal = 2,
    /// Optimal for depth/stencil attachment
    DepthStencilAttachmentOptimal = 3,
    /// Optimal for depth/stencil read-only
    DepthStencilReadOnlyOptimal = 4,
    /// Optimal for shader read
    ShaderReadOnlyOptimal = 5,
    /// Optimal for transfer source
    TransferSrcOptimal = 6,
    /// Optimal for transfer destination
    TransferDstOptimal = 7,
    /// Preinitialized layout
    Preinitialized = 8,
    /// Optimal for presentation
    PresentSrc = 1000001002,
    /// Shared present
    SharedPresent = 1000111000,
    /// Depth read-only, stencil attachment
    DepthReadOnlyStencilAttachment = 1000117000,
    /// Depth attachment, stencil read-only
    DepthAttachmentStencilReadOnly = 1000117001,
    /// Depth attachment optimal
    DepthAttachmentOptimal = 1000241000,
    /// Depth read-only optimal
    DepthReadOnlyOptimal = 1000241001,
    /// Stencil attachment optimal
    StencilAttachmentOptimal = 1000241002,
    /// Stencil read-only optimal
    StencilReadOnlyOptimal = 1000241003,
    /// Read-only optimal (VK 1.3)
    ReadOnlyOptimal = 1000314000,
    /// Attachment optimal (VK 1.3)
    AttachmentOptimal = 1000314001,
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
        /// Metadata
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
    /// Depth and stencil
    pub const DEPTH_STENCIL: Self =
        Self::from_bits_truncate(Self::DEPTH.bits() | Self::STENCIL.bits());
}

/// All mip levels remaining
pub const REMAINING_MIP_LEVELS: u32 = u32::MAX;
/// All array layers remaining
pub const REMAINING_ARRAY_LAYERS: u32 = u32::MAX;

impl ImageSubresourceRange {
    /// All color mips and layers
    pub const fn all_color() -> Self {
        Self {
            aspect_mask: ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: REMAINING_MIP_LEVELS,
            base_array_layer: 0,
            layer_count: REMAINING_ARRAY_LAYERS,
        }
    }

    /// All depth mips and layers
    pub const fn all_depth() -> Self {
        Self {
            aspect_mask: ImageAspectFlags::DEPTH,
            base_mip_level: 0,
            level_count: REMAINING_MIP_LEVELS,
            base_array_layer: 0,
            layer_count: REMAINING_ARRAY_LAYERS,
        }
    }

    /// Single mip level
    pub const fn mip(level: u32) -> Self {
        Self {
            aspect_mask: ImageAspectFlags::COLOR,
            base_mip_level: level,
            level_count: 1,
            base_array_layer: 0,
            layer_count: REMAINING_ARRAY_LAYERS,
        }
    }

    /// Single layer
    pub const fn layer(layer: u32) -> Self {
        Self {
            aspect_mask: ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: REMAINING_MIP_LEVELS,
            base_array_layer: layer,
            layer_count: 1,
        }
    }
}

impl ImageMemoryBarrier2 {
    /// Creates an image barrier
    pub fn new(image: ImageHandle) -> Self {
        Self {
            src_stage_mask: PipelineStageFlags2::NONE,
            src_access_mask: AccessFlags2::NONE,
            dst_stage_mask: PipelineStageFlags2::NONE,
            dst_access_mask: AccessFlags2::NONE,
            old_layout: ImageLayout::Undefined,
            new_layout: ImageLayout::Undefined,
            src_queue_family_index: QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: QUEUE_FAMILY_IGNORED,
            image,
            subresource_range: ImageSubresourceRange::all_color(),
        }
    }

    /// Sets source access
    pub const fn from(mut self, stage: PipelineStageFlags2, access: AccessFlags2) -> Self {
        self.src_stage_mask = stage;
        self.src_access_mask = access;
        self
    }

    /// Sets destination access
    pub const fn to(mut self, stage: PipelineStageFlags2, access: AccessFlags2) -> Self {
        self.dst_stage_mask = stage;
        self.dst_access_mask = access;
        self
    }

    /// Sets layout transition
    pub const fn layout(mut self, old: ImageLayout, new: ImageLayout) -> Self {
        self.old_layout = old;
        self.new_layout = new;
        self
    }

    /// Sets subresource range
    pub const fn subresource(mut self, range: ImageSubresourceRange) -> Self {
        self.subresource_range = range;
        self
    }

    /// Transition to color attachment
    pub fn to_color_attachment(mut self) -> Self {
        self.dst_stage_mask = PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT;
        self.dst_access_mask = AccessFlags2::COLOR_ATTACHMENT_WRITE;
        self.new_layout = ImageLayout::ColorAttachmentOptimal;
        self
    }

    /// Transition to shader read
    pub fn to_shader_read(mut self) -> Self {
        self.dst_stage_mask = PipelineStageFlags2::FRAGMENT_SHADER;
        self.dst_access_mask = AccessFlags2::SHADER_READ;
        self.new_layout = ImageLayout::ShaderReadOnlyOptimal;
        self
    }

    /// Transition to present
    pub fn to_present(mut self) -> Self {
        self.dst_stage_mask = PipelineStageFlags2::BOTTOM_OF_PIPE;
        self.dst_access_mask = AccessFlags2::NONE;
        self.new_layout = ImageLayout::PresentSrc;
        self
    }
}

bitflags::bitflags! {
    /// Access flags (VK 1.3 / KHR_synchronization2)
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct AccessFlags2: u64 {
        /// No access
        const NONE = 0;
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
        /// Depth/stencil attachment read
        const DEPTH_STENCIL_ATTACHMENT_READ = 1 << 9;
        /// Depth/stencil attachment write
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
        /// Shader sampled read
        const SHADER_SAMPLED_READ = 1 << 32;
        /// Shader storage read
        const SHADER_STORAGE_READ = 1 << 33;
        /// Shader storage write
        const SHADER_STORAGE_WRITE = 1 << 34;
        /// Video decode read
        const VIDEO_DECODE_READ = 1 << 35;
        /// Video decode write
        const VIDEO_DECODE_WRITE = 1 << 36;
        /// Video encode read
        const VIDEO_ENCODE_READ = 1 << 37;
        /// Video encode write
        const VIDEO_ENCODE_WRITE = 1 << 38;
        /// Acceleration structure read
        const ACCELERATION_STRUCTURE_READ = 1 << 39;
        /// Acceleration structure write
        const ACCELERATION_STRUCTURE_WRITE = 1 << 40;
        /// Fragment shading rate read
        const FRAGMENT_SHADING_RATE_ATTACHMENT_READ = 1 << 41;
    }
}

impl AccessFlags2 {
    /// All read accesses
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

    /// All write accesses
    pub const ALL_WRITE: Self = Self::from_bits_truncate(
        Self::SHADER_WRITE.bits()
            | Self::COLOR_ATTACHMENT_WRITE.bits()
            | Self::DEPTH_STENCIL_ATTACHMENT_WRITE.bits()
            | Self::TRANSFER_WRITE.bits()
            | Self::HOST_WRITE.bits()
            | Self::MEMORY_WRITE.bits(),
    );
}

/// Dependency info for pipeline barriers
#[derive(Clone, Debug)]
pub struct DependencyInfo {
    /// Dependency flags
    pub flags: DependencyFlags,
    /// Memory barriers
    pub memory_barriers: alloc::vec::Vec<MemoryBarrier2>,
    /// Buffer memory barriers
    pub buffer_memory_barriers: alloc::vec::Vec<BufferMemoryBarrier2>,
    /// Image memory barriers
    pub image_memory_barriers: alloc::vec::Vec<ImageMemoryBarrier2>,
}

impl DependencyInfo {
    /// Creates empty dependency info
    pub fn new() -> Self {
        Self {
            flags: DependencyFlags::empty(),
            memory_barriers: Vec::new(),
            buffer_memory_barriers: Vec::new(),
            image_memory_barriers: Vec::new(),
        }
    }

    /// Adds a memory barrier
    pub fn add_memory_barrier(mut self, barrier: MemoryBarrier2) -> Self {
        self.memory_barriers.push(barrier);
        self
    }

    /// Adds a buffer memory barrier
    pub fn add_buffer_barrier(mut self, barrier: BufferMemoryBarrier2) -> Self {
        self.buffer_memory_barriers.push(barrier);
        self
    }

    /// Adds an image memory barrier
    pub fn add_image_barrier(mut self, barrier: ImageMemoryBarrier2) -> Self {
        self.image_memory_barriers.push(barrier);
        self
    }

    /// Sets by-region flag
    pub fn by_region(mut self) -> Self {
        self.flags |= DependencyFlags::BY_REGION;
        self
    }

    /// Sets view-local flag
    pub fn view_local(mut self) -> Self {
        self.flags |= DependencyFlags::VIEW_LOCAL;
        self
    }
}

impl Default for DependencyInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Dependency flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct DependencyFlags: u32 {
        /// Dependency is per-region
        const BY_REGION = 1 << 0;
        /// Dependency is view-local
        const VIEW_LOCAL = 1 << 1;
        /// Device group dependency
        const DEVICE_GROUP = 1 << 2;
    }
}

impl DependencyFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}
