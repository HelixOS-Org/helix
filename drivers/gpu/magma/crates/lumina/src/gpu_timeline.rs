//! GPU Timeline Types for Lumina
//!
//! This module provides GPU timeline synchronization infrastructure
//! including timeline semaphores, fences, and synchronization primitives.

extern crate alloc;

use alloc::vec::Vec;

// ============================================================================
// Timeline Handles
// ============================================================================

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

/// GPU fence handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuFenceHandle(pub u64);

impl GpuFenceHandle {
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

impl Default for GpuFenceHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Event handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuEventHandle(pub u64);

impl GpuEventHandle {
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

impl Default for GpuEventHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Timeline Semaphore
// ============================================================================

/// Timeline semaphore create info
#[derive(Clone, Debug)]
pub struct TimelineSemaphoreCreateInfo {
    /// Initial value
    pub initial_value: u64,
    /// Name (for debugging)
    pub name: Option<&'static str>,
}

impl TimelineSemaphoreCreateInfo {
    /// Creates info
    pub fn new(initial_value: u64) -> Self {
        Self {
            initial_value,
            name: None,
        }
    }

    /// With name
    pub fn with_name(mut self, name: &'static str) -> Self {
        self.name = Some(name);
        self
    }
}

impl Default for TimelineSemaphoreCreateInfo {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Timeline wait info
#[derive(Clone, Debug)]
pub struct TimelineWaitInfo {
    /// Semaphore
    pub semaphore: TimelineSemaphoreHandle,
    /// Value to wait for
    pub value: u64,
    /// Timeout (nanoseconds, 0 = infinite)
    pub timeout_ns: u64,
}

impl TimelineWaitInfo {
    /// Creates info
    pub fn new(semaphore: TimelineSemaphoreHandle, value: u64) -> Self {
        Self {
            semaphore,
            value,
            timeout_ns: u64::MAX,
        }
    }

    /// With timeout
    pub fn with_timeout(mut self, timeout_ns: u64) -> Self {
        self.timeout_ns = timeout_ns;
        self
    }

    /// With timeout milliseconds
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ns = timeout_ms * 1_000_000;
        self
    }
}

impl Default for TimelineWaitInfo {
    fn default() -> Self {
        Self::new(TimelineSemaphoreHandle::NULL, 0)
    }
}

/// Timeline signal info
#[derive(Clone, Debug)]
pub struct TimelineSignalInfo {
    /// Semaphore
    pub semaphore: TimelineSemaphoreHandle,
    /// Value to signal
    pub value: u64,
}

impl TimelineSignalInfo {
    /// Creates info
    pub fn new(semaphore: TimelineSemaphoreHandle, value: u64) -> Self {
        Self { semaphore, value }
    }
}

impl Default for TimelineSignalInfo {
    fn default() -> Self {
        Self::new(TimelineSemaphoreHandle::NULL, 0)
    }
}

// ============================================================================
// Fence
// ============================================================================

/// Fence create info
#[derive(Clone, Debug)]
pub struct FenceCreateInfo {
    /// Initially signaled
    pub signaled: bool,
    /// Name (for debugging)
    pub name: Option<&'static str>,
}

impl FenceCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            signaled: false,
            name: None,
        }
    }

    /// Initially signaled
    pub fn signaled() -> Self {
        Self {
            signaled: true,
            name: None,
        }
    }

    /// With name
    pub fn with_name(mut self, name: &'static str) -> Self {
        self.name = Some(name);
        self
    }
}

impl Default for FenceCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Fence status
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FenceStatus {
    /// Not signaled
    #[default]
    NotSignaled = 0,
    /// Signaled
    Signaled    = 1,
}

impl FenceStatus {
    /// Is signaled
    pub fn is_signaled(&self) -> bool {
        matches!(self, Self::Signaled)
    }
}

// ============================================================================
// Event
// ============================================================================

/// Event create info
#[derive(Clone, Debug)]
pub struct EventCreateInfo {
    /// Name (for debugging)
    pub name: Option<&'static str>,
    /// Device only (no host signaling)
    pub device_only: bool,
}

impl EventCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            name: None,
            device_only: true,
        }
    }

    /// With name
    pub fn with_name(mut self, name: &'static str) -> Self {
        self.name = Some(name);
        self
    }

    /// Host signalable
    pub fn host_signalable(mut self) -> Self {
        self.device_only = false;
        self
    }
}

impl Default for EventCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Synchronization Scope
// ============================================================================

/// Pipeline stage flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PipelineStageFlags(pub u32);

impl PipelineStageFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Top of pipe
    pub const TOP_OF_PIPE: Self = Self(1 << 0);
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
    pub const COMPUTE_SHADER: Self = Self(1 << 11);
    /// Transfer
    pub const TRANSFER: Self = Self(1 << 12);
    /// Bottom of pipe
    pub const BOTTOM_OF_PIPE: Self = Self(1 << 13);
    /// Host
    pub const HOST: Self = Self(1 << 14);
    /// All graphics
    pub const ALL_GRAPHICS: Self = Self(0x7FF);
    /// All commands
    pub const ALL_COMMANDS: Self = Self(0x7FFF);

    /// Has flag
    pub const fn has(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }
}

impl Default for PipelineStageFlags {
    fn default() -> Self {
        Self::NONE
    }
}

impl core::ops::BitOr for PipelineStageFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Access flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AccessFlags(pub u32);

impl AccessFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Indirect command read
    pub const INDIRECT_COMMAND_READ: Self = Self(1 << 0);
    /// Index read
    pub const INDEX_READ: Self = Self(1 << 1);
    /// Vertex attribute read
    pub const VERTEX_ATTRIBUTE_READ: Self = Self(1 << 2);
    /// Uniform read
    pub const UNIFORM_READ: Self = Self(1 << 3);
    /// Input attachment read
    pub const INPUT_ATTACHMENT_READ: Self = Self(1 << 4);
    /// Shader read
    pub const SHADER_READ: Self = Self(1 << 5);
    /// Shader write
    pub const SHADER_WRITE: Self = Self(1 << 6);
    /// Color attachment read
    pub const COLOR_ATTACHMENT_READ: Self = Self(1 << 7);
    /// Color attachment write
    pub const COLOR_ATTACHMENT_WRITE: Self = Self(1 << 8);
    /// Depth stencil attachment read
    pub const DEPTH_STENCIL_READ: Self = Self(1 << 9);
    /// Depth stencil attachment write
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

    /// Has flag
    pub const fn has(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }

    /// Is read access
    pub fn is_read(&self) -> bool {
        const READ_MASK: u32 = 0x2FFF;
        (self.0 & READ_MASK) != 0
    }

    /// Is write access
    pub fn is_write(&self) -> bool {
        const WRITE_MASK: u32 = 0x1D1C0;
        (self.0 & WRITE_MASK) != 0
    }
}

impl Default for AccessFlags {
    fn default() -> Self {
        Self::NONE
    }
}

impl core::ops::BitOr for AccessFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

// ============================================================================
// Memory Barrier
// ============================================================================

/// Memory barrier
#[derive(Clone, Copy, Debug, Default)]
pub struct MemoryBarrier {
    /// Source access
    pub src_access: AccessFlags,
    /// Destination access
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
        Self::new(
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
        )
    }
}

/// Buffer memory barrier
#[derive(Clone, Copy, Debug, Default)]
pub struct BufferMemoryBarrier {
    /// Buffer handle
    pub buffer: u64,
    /// Source access
    pub src_access: AccessFlags,
    /// Destination access
    pub dst_access: AccessFlags,
    /// Offset
    pub offset: u64,
    /// Size (u64::MAX for whole buffer)
    pub size: u64,
    /// Source queue family
    pub src_queue_family: u32,
    /// Destination queue family
    pub dst_queue_family: u32,
}

impl BufferMemoryBarrier {
    /// Creates barrier
    pub fn new(buffer: u64, src: AccessFlags, dst: AccessFlags) -> Self {
        Self {
            buffer,
            src_access: src,
            dst_access: dst,
            offset: 0,
            size: u64::MAX,
            src_queue_family: u32::MAX,
            dst_queue_family: u32::MAX,
        }
    }

    /// With range
    pub fn with_range(mut self, offset: u64, size: u64) -> Self {
        self.offset = offset;
        self.size = size;
        self
    }

    /// Queue transfer
    pub fn with_queue_transfer(mut self, src: u32, dst: u32) -> Self {
        self.src_queue_family = src;
        self.dst_queue_family = dst;
        self
    }
}

/// Image memory barrier
#[derive(Clone, Copy, Debug, Default)]
pub struct ImageMemoryBarrier {
    /// Image handle
    pub image: u64,
    /// Source access
    pub src_access: AccessFlags,
    /// Destination access
    pub dst_access: AccessFlags,
    /// Old layout
    pub old_layout: ImageLayout,
    /// New layout
    pub new_layout: ImageLayout,
    /// Base mip level
    pub base_mip: u32,
    /// Mip level count
    pub mip_count: u32,
    /// Base array layer
    pub base_layer: u32,
    /// Layer count
    pub layer_count: u32,
    /// Aspect
    pub aspect: ImageAspect,
    /// Source queue family
    pub src_queue_family: u32,
    /// Destination queue family
    pub dst_queue_family: u32,
}

impl ImageMemoryBarrier {
    /// Creates barrier
    pub fn new(image: u64, old_layout: ImageLayout, new_layout: ImageLayout) -> Self {
        Self {
            image,
            src_access: AccessFlags::NONE,
            dst_access: AccessFlags::NONE,
            old_layout,
            new_layout,
            base_mip: 0,
            mip_count: u32::MAX,
            base_layer: 0,
            layer_count: u32::MAX,
            aspect: ImageAspect::Color,
            src_queue_family: u32::MAX,
            dst_queue_family: u32::MAX,
        }
    }

    /// With access
    pub fn with_access(mut self, src: AccessFlags, dst: AccessFlags) -> Self {
        self.src_access = src;
        self.dst_access = dst;
        self
    }

    /// With subresource
    pub fn with_subresource(
        mut self,
        base_mip: u32,
        mip_count: u32,
        base_layer: u32,
        layer_count: u32,
    ) -> Self {
        self.base_mip = base_mip;
        self.mip_count = mip_count;
        self.base_layer = base_layer;
        self.layer_count = layer_count;
        self
    }

    /// With aspect
    pub fn with_aspect(mut self, aspect: ImageAspect) -> Self {
        self.aspect = aspect;
        self
    }
}

/// Image layout
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ImageLayout {
    /// Undefined
    #[default]
    Undefined            = 0,
    /// General
    General              = 1,
    /// Color attachment
    ColorAttachment      = 2,
    /// Depth stencil attachment
    DepthStencilAttachment = 3,
    /// Depth stencil read only
    DepthStencilReadOnly = 4,
    /// Shader read only
    ShaderReadOnly       = 5,
    /// Transfer source
    TransferSrc          = 6,
    /// Transfer destination
    TransferDst          = 7,
    /// Present
    Present              = 8,
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

// ============================================================================
// Submission
// ============================================================================

/// Submit info
#[derive(Clone, Debug)]
pub struct SubmitInfo {
    /// Wait semaphores
    pub wait_semaphores: Vec<SemaphoreSubmitInfo>,
    /// Signal semaphores
    pub signal_semaphores: Vec<SemaphoreSubmitInfo>,
    /// Command buffers
    pub command_buffers: Vec<u64>,
}

impl SubmitInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            wait_semaphores: Vec::new(),
            signal_semaphores: Vec::new(),
            command_buffers: Vec::new(),
        }
    }

    /// Add command buffer
    pub fn add_command_buffer(mut self, cmd: u64) -> Self {
        self.command_buffers.push(cmd);
        self
    }

    /// Wait on timeline semaphore
    pub fn wait_timeline(
        mut self,
        semaphore: TimelineSemaphoreHandle,
        value: u64,
        stage: PipelineStageFlags,
    ) -> Self {
        self.wait_semaphores.push(SemaphoreSubmitInfo {
            semaphore: semaphore.0,
            value,
            stage,
            is_timeline: true,
        });
        self
    }

    /// Signal timeline semaphore
    pub fn signal_timeline(
        mut self,
        semaphore: TimelineSemaphoreHandle,
        value: u64,
        stage: PipelineStageFlags,
    ) -> Self {
        self.signal_semaphores.push(SemaphoreSubmitInfo {
            semaphore: semaphore.0,
            value,
            stage,
            is_timeline: true,
        });
        self
    }
}

impl Default for SubmitInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Semaphore submit info
#[derive(Clone, Copy, Debug, Default)]
pub struct SemaphoreSubmitInfo {
    /// Semaphore handle
    pub semaphore: u64,
    /// Timeline value
    pub value: u64,
    /// Pipeline stage
    pub stage: PipelineStageFlags,
    /// Is timeline semaphore
    pub is_timeline: bool,
}

// ============================================================================
// Statistics
// ============================================================================

/// Synchronization statistics
#[derive(Clone, Debug, Default)]
pub struct SyncStats {
    /// Fences waited
    pub fences_waited: u32,
    /// Semaphores waited
    pub semaphores_waited: u32,
    /// Semaphores signaled
    pub semaphores_signaled: u32,
    /// Barriers issued
    pub barriers_issued: u32,
    /// Wait time (microseconds)
    pub wait_time_us: u64,
}
