//! GPU Timeline and Synchronization for Lumina
//!
//! This module provides GPU synchronization primitives including
//! timeline semaphores, fences, barriers, and command ordering.

extern crate alloc;

use alloc::vec::Vec;

// ============================================================================
// Synchronization Handles
// ============================================================================

/// Fence handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct FenceHandle(pub u64);

impl FenceHandle {
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

impl Default for FenceHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Semaphore handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SemaphoreHandle(pub u64);

impl SemaphoreHandle {
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

impl Default for SemaphoreHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Timeline semaphore handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TimelineSemaphoreHandle(pub u64);

impl TimelineSemaphoreHandle {
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

impl Default for TimelineSemaphoreHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Event handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct EventHandle(pub u64);

impl EventHandle {
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

impl Default for EventHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Fence
// ============================================================================

/// Fence create info
#[derive(Clone, Debug)]
pub struct FenceCreateInfo {
    /// Initial signaled state
    pub signaled: bool,
    /// Debug label
    pub label: Option<&'static str>,
}

impl FenceCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            signaled: false,
            label: None,
        }
    }

    /// Initially signaled
    pub fn signaled() -> Self {
        Self {
            signaled: true,
            label: None,
        }
    }

    /// With label
    pub fn with_label(mut self, label: &'static str) -> Self {
        self.label = Some(label);
        self
    }
}

impl Default for FenceCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Fence status
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum FenceStatus {
    /// Not signaled
    NotReady = 0,
    /// Signaled
    Signaled = 1,
}

impl FenceStatus {
    /// Is signaled
    pub const fn is_signaled(&self) -> bool {
        matches!(self, Self::Signaled)
    }
}

// ============================================================================
// Semaphore
// ============================================================================

/// Semaphore create info
#[derive(Clone, Debug)]
pub struct SemaphoreCreateInfo {
    /// Semaphore type
    pub semaphore_type: SemaphoreType,
    /// Initial value (for timeline)
    pub initial_value: u64,
    /// Debug label
    pub label: Option<&'static str>,
}

impl SemaphoreCreateInfo {
    /// Creates binary semaphore
    pub fn binary() -> Self {
        Self {
            semaphore_type: SemaphoreType::Binary,
            initial_value: 0,
            label: None,
        }
    }

    /// Creates timeline semaphore
    pub fn timeline(initial_value: u64) -> Self {
        Self {
            semaphore_type: SemaphoreType::Timeline,
            initial_value,
            label: None,
        }
    }

    /// With label
    pub fn with_label(mut self, label: &'static str) -> Self {
        self.label = Some(label);
        self
    }
}

impl Default for SemaphoreCreateInfo {
    fn default() -> Self {
        Self::binary()
    }
}

/// Semaphore type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SemaphoreType {
    /// Binary semaphore
    #[default]
    Binary   = 0,
    /// Timeline semaphore
    Timeline = 1,
}

/// Timeline semaphore wait/signal info
#[derive(Clone, Copy, Debug)]
pub struct TimelineWaitInfo {
    /// Semaphore handle
    pub semaphore: TimelineSemaphoreHandle,
    /// Value to wait for
    pub value: u64,
}

impl TimelineWaitInfo {
    /// Creates wait info
    pub fn new(semaphore: TimelineSemaphoreHandle, value: u64) -> Self {
        Self { semaphore, value }
    }
}

/// Timeline signal info
#[derive(Clone, Copy, Debug)]
pub struct TimelineSignalInfo {
    /// Semaphore handle
    pub semaphore: TimelineSemaphoreHandle,
    /// Value to signal
    pub value: u64,
}

impl TimelineSignalInfo {
    /// Creates signal info
    pub fn new(semaphore: TimelineSemaphoreHandle, value: u64) -> Self {
        Self { semaphore, value }
    }
}

// ============================================================================
// Event
// ============================================================================

/// Event create info
#[derive(Clone, Debug)]
pub struct EventCreateInfo {
    /// Event flags
    pub flags: EventFlags,
    /// Debug label
    pub label: Option<&'static str>,
}

impl EventCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            flags: EventFlags::empty(),
            label: None,
        }
    }

    /// Device only (no host operations)
    pub fn device_only() -> Self {
        Self {
            flags: EventFlags::DEVICE_ONLY,
            label: None,
        }
    }

    /// With label
    pub fn with_label(mut self, label: &'static str) -> Self {
        self.label = Some(label);
        self
    }
}

impl Default for EventCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Event flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct EventFlags(u32);

impl EventFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Device-only event
    pub const DEVICE_ONLY: Self = Self(1 << 0);

    /// Is empty
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Contains flag
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

// ============================================================================
// Barriers
// ============================================================================

/// Memory barrier
#[derive(Clone, Copy, Debug)]
pub struct MemoryBarrier {
    /// Source access mask
    pub src_access: AccessFlags,
    /// Destination access mask
    pub dst_access: AccessFlags,
}

impl MemoryBarrier {
    /// Creates barrier
    pub fn new(src: AccessFlags, dst: AccessFlags) -> Self {
        Self {
            src_access: src,
            dst_access: dst,
        }
    }

    /// Full barrier
    pub fn full() -> Self {
        Self::new(AccessFlags::all(), AccessFlags::all())
    }

    /// Write to read
    pub fn write_to_read() -> Self {
        Self::new(AccessFlags::MEMORY_WRITE, AccessFlags::MEMORY_READ)
    }

    /// Shader write to shader read
    pub fn shader_write_to_read() -> Self {
        Self::new(AccessFlags::SHADER_WRITE, AccessFlags::SHADER_READ)
    }
}

impl Default for MemoryBarrier {
    fn default() -> Self {
        Self::full()
    }
}

/// Buffer memory barrier
#[derive(Clone, Copy, Debug)]
pub struct BufferBarrier {
    /// Buffer handle
    pub buffer: u64,
    /// Offset
    pub offset: u64,
    /// Size (0 = whole buffer)
    pub size: u64,
    /// Source access
    pub src_access: AccessFlags,
    /// Destination access
    pub dst_access: AccessFlags,
    /// Source queue family
    pub src_queue_family: u32,
    /// Destination queue family
    pub dst_queue_family: u32,
}

impl BufferBarrier {
    /// Creates barrier
    pub fn new(buffer: u64, src: AccessFlags, dst: AccessFlags) -> Self {
        Self {
            buffer,
            offset: 0,
            size: 0,
            src_access: src,
            dst_access: dst,
            src_queue_family: QUEUE_FAMILY_IGNORED,
            dst_queue_family: QUEUE_FAMILY_IGNORED,
        }
    }

    /// With range
    pub fn with_range(mut self, offset: u64, size: u64) -> Self {
        self.offset = offset;
        self.size = size;
        self
    }

    /// With queue transfer
    pub fn with_queue_transfer(mut self, src: u32, dst: u32) -> Self {
        self.src_queue_family = src;
        self.dst_queue_family = dst;
        self
    }
}

/// Image memory barrier
#[derive(Clone, Copy, Debug)]
pub struct ImageBarrier {
    /// Image handle
    pub image: u64,
    /// Subresource range
    pub subresource: ImageSubresourceRange,
    /// Source access
    pub src_access: AccessFlags,
    /// Destination access
    pub dst_access: AccessFlags,
    /// Old layout
    pub old_layout: ImageLayout,
    /// New layout
    pub new_layout: ImageLayout,
    /// Source queue family
    pub src_queue_family: u32,
    /// Destination queue family
    pub dst_queue_family: u32,
}

impl ImageBarrier {
    /// Creates barrier
    pub fn new(image: u64, old_layout: ImageLayout, new_layout: ImageLayout) -> Self {
        Self {
            image,
            subresource: ImageSubresourceRange::all(),
            src_access: AccessFlags::empty(),
            dst_access: AccessFlags::empty(),
            old_layout,
            new_layout,
            src_queue_family: QUEUE_FAMILY_IGNORED,
            dst_queue_family: QUEUE_FAMILY_IGNORED,
        }
    }

    /// With access
    pub fn with_access(mut self, src: AccessFlags, dst: AccessFlags) -> Self {
        self.src_access = src;
        self.dst_access = dst;
        self
    }

    /// With subresource
    pub fn with_subresource(mut self, range: ImageSubresourceRange) -> Self {
        self.subresource = range;
        self
    }

    /// Color attachment to shader read
    pub fn color_to_shader_read(image: u64) -> Self {
        Self::new(
            image,
            ImageLayout::ColorAttachment,
            ImageLayout::ShaderReadOnly,
        )
        .with_access(AccessFlags::COLOR_WRITE, AccessFlags::SHADER_READ)
    }

    /// Undefined to transfer dst
    pub fn undefined_to_transfer(image: u64) -> Self {
        Self::new(image, ImageLayout::Undefined, ImageLayout::TransferDst)
            .with_access(AccessFlags::empty(), AccessFlags::TRANSFER_WRITE)
    }

    /// Transfer to shader read
    pub fn transfer_to_shader_read(image: u64) -> Self {
        Self::new(image, ImageLayout::TransferDst, ImageLayout::ShaderReadOnly)
            .with_access(AccessFlags::TRANSFER_WRITE, AccessFlags::SHADER_READ)
    }
}

/// Queue family ignored constant
pub const QUEUE_FAMILY_IGNORED: u32 = u32::MAX;

/// Image subresource range
#[derive(Clone, Copy, Debug)]
pub struct ImageSubresourceRange {
    /// Aspect mask
    pub aspect: ImageAspect,
    /// Base mip level
    pub base_mip: u32,
    /// Mip count
    pub mip_count: u32,
    /// Base array layer
    pub base_layer: u32,
    /// Layer count
    pub layer_count: u32,
}

impl ImageSubresourceRange {
    /// All subresources
    pub fn all() -> Self {
        Self {
            aspect: ImageAspect::Color,
            base_mip: 0,
            mip_count: u32::MAX,
            base_layer: 0,
            layer_count: u32::MAX,
        }
    }

    /// Color subresource
    pub fn color() -> Self {
        Self::all()
    }

    /// Depth subresource
    pub fn depth() -> Self {
        Self {
            aspect: ImageAspect::Depth,
            ..Self::all()
        }
    }

    /// Stencil subresource
    pub fn stencil() -> Self {
        Self {
            aspect: ImageAspect::Stencil,
            ..Self::all()
        }
    }

    /// Depth-stencil subresource
    pub fn depth_stencil() -> Self {
        Self {
            aspect: ImageAspect::DepthStencil,
            ..Self::all()
        }
    }

    /// Single mip
    pub fn mip(level: u32) -> Self {
        Self {
            base_mip: level,
            mip_count: 1,
            ..Self::all()
        }
    }

    /// Single layer
    pub fn layer(index: u32) -> Self {
        Self {
            base_layer: index,
            layer_count: 1,
            ..Self::all()
        }
    }
}

impl Default for ImageSubresourceRange {
    fn default() -> Self {
        Self::all()
    }
}

/// Image aspect
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ImageAspect {
    /// Color
    #[default]
    Color        = 0,
    /// Depth
    Depth        = 1,
    /// Stencil
    Stencil      = 2,
    /// Depth and stencil
    DepthStencil = 3,
}

/// Image layout
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ImageLayout {
    /// Undefined
    #[default]
    Undefined            = 0,
    /// General (any access)
    General              = 1,
    /// Color attachment
    ColorAttachment      = 2,
    /// Depth-stencil attachment
    DepthStencilAttachment = 3,
    /// Depth-stencil read only
    DepthStencilReadOnly = 4,
    /// Shader read only
    ShaderReadOnly       = 5,
    /// Transfer source
    TransferSrc          = 6,
    /// Transfer destination
    TransferDst          = 7,
    /// Preinitialized
    Preinitialized       = 8,
    /// Present (swapchain)
    Present              = 9,
    /// Attachment optimal
    AttachmentOptimal    = 10,
    /// Read only optimal
    ReadOnlyOptimal      = 11,
}

// ============================================================================
// Access Flags
// ============================================================================

/// Access flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct AccessFlags(u32);

impl AccessFlags {
    /// No access
    pub const fn empty() -> Self {
        Self(0)
    }

    /// All access
    pub const fn all() -> Self {
        Self(0xFFFFFFFF)
    }

    /// Indirect command read
    pub const INDIRECT_READ: Self = Self(1 << 0);
    /// Index buffer read
    pub const INDEX_READ: Self = Self(1 << 1);
    /// Vertex buffer read
    pub const VERTEX_READ: Self = Self(1 << 2);
    /// Uniform buffer read
    pub const UNIFORM_READ: Self = Self(1 << 3);
    /// Input attachment read
    pub const INPUT_READ: Self = Self(1 << 4);
    /// Shader read
    pub const SHADER_READ: Self = Self(1 << 5);
    /// Shader write
    pub const SHADER_WRITE: Self = Self(1 << 6);
    /// Color attachment read
    pub const COLOR_READ: Self = Self(1 << 7);
    /// Color attachment write
    pub const COLOR_WRITE: Self = Self(1 << 8);
    /// Depth-stencil read
    pub const DEPTH_STENCIL_READ: Self = Self(1 << 9);
    /// Depth-stencil write
    pub const DEPTH_STENCIL_WRITE: Self = Self(1 << 10);
    /// Transfer read
    pub const TRANSFER_READ: Self = Self(1 << 11);
    /// Transfer write
    pub const TRANSFER_WRITE: Self = Self(1 << 12);
    /// Host read
    pub const HOST_READ: Self = Self(1 << 13);
    /// Host write
    pub const HOST_WRITE: Self = Self(1 << 14);
    /// Memory read
    pub const MEMORY_READ: Self = Self(1 << 15);
    /// Memory write
    pub const MEMORY_WRITE: Self = Self(1 << 16);

    /// Is empty
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Contains flag
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Combine flags
    pub const fn or(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl core::ops::BitOr for AccessFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitOrAssign for AccessFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

// ============================================================================
// Pipeline Stages
// ============================================================================

/// Pipeline stage flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct PipelineStages(u32);

impl PipelineStages {
    /// No stages
    pub const fn empty() -> Self {
        Self(0)
    }

    /// All stages
    pub const fn all() -> Self {
        Self(0xFFFFFFFF)
    }

    /// Top of pipe
    pub const TOP: Self = Self(1 << 0);
    /// Draw indirect
    pub const DRAW_INDIRECT: Self = Self(1 << 1);
    /// Vertex input
    pub const VERTEX_INPUT: Self = Self(1 << 2);
    /// Vertex shader
    pub const VERTEX_SHADER: Self = Self(1 << 3);
    /// Tessellation control
    pub const TESSELLATION_CONTROL: Self = Self(1 << 4);
    /// Tessellation evaluation
    pub const TESSELLATION_EVALUATION: Self = Self(1 << 5);
    /// Geometry shader
    pub const GEOMETRY_SHADER: Self = Self(1 << 6);
    /// Fragment shader
    pub const FRAGMENT_SHADER: Self = Self(1 << 7);
    /// Early fragment tests
    pub const EARLY_FRAGMENT_TESTS: Self = Self(1 << 8);
    /// Late fragment tests
    pub const LATE_FRAGMENT_TESTS: Self = Self(1 << 9);
    /// Color attachment output
    pub const COLOR_ATTACHMENT_OUTPUT: Self = Self(1 << 10);
    /// Compute shader
    pub const COMPUTE: Self = Self(1 << 11);
    /// Transfer
    pub const TRANSFER: Self = Self(1 << 12);
    /// Bottom of pipe
    pub const BOTTOM: Self = Self(1 << 13);
    /// Host
    pub const HOST: Self = Self(1 << 14);
    /// All graphics
    pub const ALL_GRAPHICS: Self = Self(1 << 15);
    /// All commands
    pub const ALL_COMMANDS: Self = Self(1 << 16);

    /// Is empty
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Contains stage
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Combine stages
    pub const fn or(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl core::ops::BitOr for PipelineStages {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitOrAssign for PipelineStages {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

// ============================================================================
// Dependency Info
// ============================================================================

/// Dependency info for pipeline barrier
#[derive(Clone, Debug, Default)]
pub struct DependencyInfo {
    /// Memory barriers
    pub memory_barriers: Vec<MemoryBarrier>,
    /// Buffer barriers
    pub buffer_barriers: Vec<BufferBarrier>,
    /// Image barriers
    pub image_barriers: Vec<ImageBarrier>,
    /// Source stages
    pub src_stages: PipelineStages,
    /// Destination stages
    pub dst_stages: PipelineStages,
}

impl DependencyInfo {
    /// Creates empty info
    pub fn new() -> Self {
        Self {
            memory_barriers: Vec::new(),
            buffer_barriers: Vec::new(),
            image_barriers: Vec::new(),
            src_stages: PipelineStages::empty(),
            dst_stages: PipelineStages::empty(),
        }
    }

    /// With memory barrier
    pub fn with_memory_barrier(mut self, barrier: MemoryBarrier) -> Self {
        self.memory_barriers.push(barrier);
        self
    }

    /// With buffer barrier
    pub fn with_buffer_barrier(mut self, barrier: BufferBarrier) -> Self {
        self.buffer_barriers.push(barrier);
        self
    }

    /// With image barrier
    pub fn with_image_barrier(mut self, barrier: ImageBarrier) -> Self {
        self.image_barriers.push(barrier);
        self
    }

    /// With stages
    pub fn with_stages(mut self, src: PipelineStages, dst: PipelineStages) -> Self {
        self.src_stages = src;
        self.dst_stages = dst;
        self
    }
}

// ============================================================================
// Submit Info
// ============================================================================

/// Submit info for queue submission
#[derive(Clone, Debug, Default)]
pub struct SubmitInfo {
    /// Wait semaphores
    pub wait_semaphores: Vec<SemaphoreWaitInfo>,
    /// Signal semaphores
    pub signal_semaphores: Vec<SemaphoreHandle>,
    /// Timeline wait info
    pub timeline_waits: Vec<TimelineWaitInfo>,
    /// Timeline signal info
    pub timeline_signals: Vec<TimelineSignalInfo>,
    /// Command buffers
    pub command_buffers: Vec<u64>,
}

impl SubmitInfo {
    /// Creates info
    pub fn new() -> Self {
        Self::default()
    }

    /// With command buffer
    pub fn with_command_buffer(mut self, cmd: u64) -> Self {
        self.command_buffers.push(cmd);
        self
    }

    /// With wait semaphore
    pub fn with_wait(mut self, semaphore: SemaphoreHandle, stages: PipelineStages) -> Self {
        self.wait_semaphores
            .push(SemaphoreWaitInfo { semaphore, stages });
        self
    }

    /// With signal semaphore
    pub fn with_signal(mut self, semaphore: SemaphoreHandle) -> Self {
        self.signal_semaphores.push(semaphore);
        self
    }

    /// With timeline wait
    pub fn with_timeline_wait(mut self, wait: TimelineWaitInfo) -> Self {
        self.timeline_waits.push(wait);
        self
    }

    /// With timeline signal
    pub fn with_timeline_signal(mut self, signal: TimelineSignalInfo) -> Self {
        self.timeline_signals.push(signal);
        self
    }
}

/// Semaphore wait info
#[derive(Clone, Copy, Debug)]
pub struct SemaphoreWaitInfo {
    /// Semaphore
    pub semaphore: SemaphoreHandle,
    /// Wait stages
    pub stages: PipelineStages,
}

// ============================================================================
// Frame Synchronization
// ============================================================================

/// Frame synchronization resources
#[derive(Clone, Debug)]
pub struct FrameSync {
    /// Image available semaphore
    pub image_available: SemaphoreHandle,
    /// Render finished semaphore
    pub render_finished: SemaphoreHandle,
    /// In-flight fence
    pub in_flight: FenceHandle,
    /// Frame index
    pub frame_index: u32,
}

impl FrameSync {
    /// Creates frame sync
    pub fn new(
        image_available: SemaphoreHandle,
        render_finished: SemaphoreHandle,
        in_flight: FenceHandle,
        frame_index: u32,
    ) -> Self {
        Self {
            image_available,
            render_finished,
            in_flight,
            frame_index,
        }
    }
}

/// Frame in flight tracking
#[derive(Clone, Debug)]
pub struct FramesInFlight {
    /// Frame sync resources
    pub frames: Vec<FrameSync>,
    /// Current frame
    pub current: usize,
    /// Max frames in flight
    pub max_frames: usize,
}

impl FramesInFlight {
    /// Creates frames in flight
    pub fn new(max_frames: usize) -> Self {
        Self {
            frames: Vec::with_capacity(max_frames),
            current: 0,
            max_frames,
        }
    }

    /// Advance to next frame
    pub fn advance(&mut self) {
        self.current = (self.current + 1) % self.max_frames;
    }

    /// Current frame sync
    pub fn current(&self) -> Option<&FrameSync> {
        self.frames.get(self.current)
    }

    /// Current frame index
    pub fn current_index(&self) -> usize {
        self.current
    }
}

impl Default for FramesInFlight {
    fn default() -> Self {
        Self::new(2)
    }
}
