//! Command buffer types and recording primitives
//!
//! This module provides types for command buffer management and command recording.

use core::num::NonZeroU32;

/// Command buffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CommandBufferHandle(pub NonZeroU32);

impl CommandBufferHandle {
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

/// Command pool handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CommandPoolHandle(pub NonZeroU32);

impl CommandPoolHandle {
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

/// Command buffer level
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum CommandBufferLevel {
    /// Primary command buffer (can be submitted to queues)
    #[default]
    Primary = 0,
    /// Secondary command buffer (executed from primary)
    Secondary = 1,
}

impl CommandBufferLevel {
    /// Is primary level
    pub const fn is_primary(self) -> bool {
        matches!(self, Self::Primary)
    }

    /// Is secondary level
    pub const fn is_secondary(self) -> bool {
        matches!(self, Self::Secondary)
    }
}

/// Command pool creation info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct CommandPoolCreateInfo {
    /// Queue family index
    pub queue_family_index: u32,
    /// Pool creation flags
    pub flags: CommandPoolCreateFlags,
}

impl CommandPoolCreateInfo {
    /// Creates info for graphics commands
    pub const fn graphics() -> Self {
        Self {
            queue_family_index: 0,
            flags: CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
        }
    }

    /// Creates info for compute commands
    pub const fn compute(queue_family: u32) -> Self {
        Self {
            queue_family_index: queue_family,
            flags: CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
        }
    }

    /// Creates info for transfer commands
    pub const fn transfer(queue_family: u32) -> Self {
        Self {
            queue_family_index: queue_family,
            flags: CommandPoolCreateFlags::TRANSIENT,
        }
    }

    /// With flags
    pub const fn with_flags(mut self, flags: CommandPoolCreateFlags) -> Self {
        self.flags = flags;
        self
    }
}

bitflags::bitflags! {
    /// Command pool creation flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct CommandPoolCreateFlags: u32 {
        /// Command buffers are short-lived
        const TRANSIENT = 1 << 0;
        /// Command buffers can be reset individually
        const RESET_COMMAND_BUFFER = 1 << 1;
        /// Pool is protected
        const PROTECTED = 1 << 2;
    }
}

impl CommandPoolCreateFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}

/// Command pool reset flags
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum CommandPoolResetFlags {
    /// Normal reset
    None = 0,
    /// Release resources back to the system
    ReleaseResources = 1,
}

/// Command buffer allocation info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct CommandBufferAllocateInfo {
    /// Command pool to allocate from
    pub command_pool: CommandPoolHandle,
    /// Command buffer level
    pub level: CommandBufferLevel,
    /// Number of command buffers to allocate
    pub count: u32,
}

impl CommandBufferAllocateInfo {
    /// Allocate primary command buffers
    pub const fn primary(pool: CommandPoolHandle, count: u32) -> Self {
        Self {
            command_pool: pool,
            level: CommandBufferLevel::Primary,
            count,
        }
    }

    /// Allocate secondary command buffers
    pub const fn secondary(pool: CommandPoolHandle, count: u32) -> Self {
        Self {
            command_pool: pool,
            level: CommandBufferLevel::Secondary,
            count,
        }
    }
}

/// Command buffer begin info
#[derive(Clone, Debug)]
pub struct CommandBufferBeginInfo {
    /// Usage flags
    pub flags: CommandBufferUsageFlags,
    /// Inheritance info (for secondary command buffers)
    pub inheritance_info: Option<CommandBufferInheritanceInfo>,
}

impl CommandBufferBeginInfo {
    /// Default begin info
    pub const fn new() -> Self {
        Self {
            flags: CommandBufferUsageFlags::empty(),
            inheritance_info: None,
        }
    }

    /// For one-time submit
    pub const fn one_time_submit() -> Self {
        Self {
            flags: CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            inheritance_info: None,
        }
    }

    /// For render pass continue (secondary buffers)
    pub fn render_pass_continue(inheritance: CommandBufferInheritanceInfo) -> Self {
        Self {
            flags: CommandBufferUsageFlags::RENDER_PASS_CONTINUE,
            inheritance_info: Some(inheritance),
        }
    }

    /// For simultaneous use
    pub const fn simultaneous_use() -> Self {
        Self {
            flags: CommandBufferUsageFlags::SIMULTANEOUS_USE,
            inheritance_info: None,
        }
    }
}

impl Default for CommandBufferBeginInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Command buffer usage flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct CommandBufferUsageFlags: u32 {
        /// Buffer will be submitted once and reset
        const ONE_TIME_SUBMIT = 1 << 0;
        /// Secondary buffer is inside a render pass
        const RENDER_PASS_CONTINUE = 1 << 1;
        /// Buffer can be resubmitted while pending
        const SIMULTANEOUS_USE = 1 << 2;
    }
}

impl CommandBufferUsageFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}

/// Command buffer inheritance info (for secondary buffers)
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct CommandBufferInheritanceInfo {
    /// Render pass handle (or None for dynamic rendering)
    pub render_pass: Option<crate::render_pass_types::RenderPassHandle>,
    /// Subpass index
    pub subpass: u32,
    /// Framebuffer handle
    pub framebuffer: Option<crate::render_pass_types::FramebufferHandle>,
    /// Occlusion query enable
    pub occlusion_query_enable: bool,
    /// Query flags
    pub query_flags: QueryControlFlags,
    /// Pipeline statistics
    pub pipeline_statistics: PipelineStatisticsFlags,
}

impl CommandBufferInheritanceInfo {
    /// Creates inheritance info
    pub const fn new() -> Self {
        Self {
            render_pass: None,
            subpass: 0,
            framebuffer: None,
            occlusion_query_enable: false,
            query_flags: QueryControlFlags::empty(),
            pipeline_statistics: PipelineStatisticsFlags::empty(),
        }
    }

    /// For render pass
    pub const fn for_render_pass(
        render_pass: crate::render_pass_types::RenderPassHandle,
        subpass: u32,
    ) -> Self {
        Self {
            render_pass: Some(render_pass),
            subpass,
            framebuffer: None,
            occlusion_query_enable: false,
            query_flags: QueryControlFlags::empty(),
            pipeline_statistics: PipelineStatisticsFlags::empty(),
        }
    }
}

impl Default for CommandBufferInheritanceInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Query control flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct QueryControlFlags: u32 {
        /// Require precise occlusion query results
        const PRECISE = 1 << 0;
    }
}

impl QueryControlFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}

bitflags::bitflags! {
    /// Pipeline statistics flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct PipelineStatisticsFlags: u32 {
        /// Input assembly vertex count
        const INPUT_ASSEMBLY_VERTICES = 1 << 0;
        /// Input assembly primitive count
        const INPUT_ASSEMBLY_PRIMITIVES = 1 << 1;
        /// Vertex shader invocations
        const VERTEX_SHADER_INVOCATIONS = 1 << 2;
        /// Geometry shader invocations
        const GEOMETRY_SHADER_INVOCATIONS = 1 << 3;
        /// Geometry shader primitives
        const GEOMETRY_SHADER_PRIMITIVES = 1 << 4;
        /// Clipping invocations
        const CLIPPING_INVOCATIONS = 1 << 5;
        /// Clipping primitives
        const CLIPPING_PRIMITIVES = 1 << 6;
        /// Fragment shader invocations
        const FRAGMENT_SHADER_INVOCATIONS = 1 << 7;
        /// Tessellation control patches
        const TESSELLATION_CONTROL_SHADER_PATCHES = 1 << 8;
        /// Tessellation evaluation invocations
        const TESSELLATION_EVALUATION_SHADER_INVOCATIONS = 1 << 9;
        /// Compute shader invocations
        const COMPUTE_SHADER_INVOCATIONS = 1 << 10;
        /// Task shader invocations
        const TASK_SHADER_INVOCATIONS = 1 << 11;
        /// Mesh shader invocations
        const MESH_SHADER_INVOCATIONS = 1 << 12;
    }
}

impl PipelineStatisticsFlags {
    /// No statistics
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }

    /// All vertex stage statistics
    pub const ALL_VERTEX: Self = Self::from_bits_truncate(
        Self::INPUT_ASSEMBLY_VERTICES.bits()
            | Self::INPUT_ASSEMBLY_PRIMITIVES.bits()
            | Self::VERTEX_SHADER_INVOCATIONS.bits(),
    );

    /// All fragment stage statistics
    pub const ALL_FRAGMENT: Self = Self::FRAGMENT_SHADER_INVOCATIONS;
}

/// Command buffer reset flags
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum CommandBufferResetFlags {
    /// Normal reset
    None = 0,
    /// Release resources
    ReleaseResources = 1,
}

/// Submit info for queue submission
#[derive(Clone, Debug)]
pub struct SubmitInfo {
    /// Wait semaphores
    pub wait_semaphores: alloc::vec::Vec<SemaphoreSubmitInfo>,
    /// Command buffers to execute
    pub command_buffers: alloc::vec::Vec<CommandBufferSubmitInfo>,
    /// Signal semaphores
    pub signal_semaphores: alloc::vec::Vec<SemaphoreSubmitInfo>,
}

use alloc::vec::Vec;

impl SubmitInfo {
    /// Creates empty submit info
    pub fn new() -> Self {
        Self {
            wait_semaphores: Vec::new(),
            command_buffers: Vec::new(),
            signal_semaphores: Vec::new(),
        }
    }

    /// Adds a command buffer
    pub fn add_command_buffer(mut self, cmd: CommandBufferHandle) -> Self {
        self.command_buffers.push(CommandBufferSubmitInfo {
            command_buffer: cmd,
            device_mask: 0,
        });
        self
    }

    /// Waits on a semaphore
    pub fn wait_semaphore(mut self, semaphore: SemaphoreHandle, stage_mask: PipelineStageFlags2) -> Self {
        self.wait_semaphores.push(SemaphoreSubmitInfo {
            semaphore,
            value: 0,
            stage_mask,
            device_index: 0,
        });
        self
    }

    /// Signals a semaphore
    pub fn signal_semaphore(mut self, semaphore: SemaphoreHandle, stage_mask: PipelineStageFlags2) -> Self {
        self.signal_semaphores.push(SemaphoreSubmitInfo {
            semaphore,
            value: 0,
            stage_mask,
            device_index: 0,
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
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SemaphoreSubmitInfo {
    /// Semaphore handle
    pub semaphore: SemaphoreHandle,
    /// Timeline value (0 for binary semaphores)
    pub value: u64,
    /// Stage mask
    pub stage_mask: PipelineStageFlags2,
    /// Device index (for multi-GPU)
    pub device_index: u32,
}

/// Command buffer submit info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct CommandBufferSubmitInfo {
    /// Command buffer handle
    pub command_buffer: CommandBufferHandle,
    /// Device mask (for multi-GPU)
    pub device_mask: u32,
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

bitflags::bitflags! {
    /// Pipeline stage flags (VK 1.3 / KHR_synchronization2)
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct PipelineStageFlags2: u64 {
        /// No stage
        const NONE = 0;
        /// Top of pipe
        const TOP_OF_PIPE = 1 << 0;
        /// Draw indirect
        const DRAW_INDIRECT = 1 << 1;
        /// Vertex input
        const VERTEX_INPUT = 1 << 2;
        /// Vertex shader
        const VERTEX_SHADER = 1 << 3;
        /// Tessellation control shader
        const TESSELLATION_CONTROL_SHADER = 1 << 4;
        /// Tessellation evaluation shader
        const TESSELLATION_EVALUATION_SHADER = 1 << 5;
        /// Geometry shader
        const GEOMETRY_SHADER = 1 << 6;
        /// Fragment shader
        const FRAGMENT_SHADER = 1 << 7;
        /// Early fragment tests
        const EARLY_FRAGMENT_TESTS = 1 << 8;
        /// Late fragment tests
        const LATE_FRAGMENT_TESTS = 1 << 9;
        /// Color attachment output
        const COLOR_ATTACHMENT_OUTPUT = 1 << 10;
        /// Compute shader
        const COMPUTE_SHADER = 1 << 11;
        /// Transfer
        const TRANSFER = 1 << 12;
        /// Bottom of pipe
        const BOTTOM_OF_PIPE = 1 << 13;
        /// Host operations
        const HOST = 1 << 14;
        /// All graphics stages
        const ALL_GRAPHICS = 1 << 15;
        /// All commands
        const ALL_COMMANDS = 1 << 16;
        /// Copy
        const COPY = 1 << 32;
        /// Resolve
        const RESOLVE = 1 << 33;
        /// Blit
        const BLIT = 1 << 34;
        /// Clear
        const CLEAR = 1 << 35;
        /// Index input
        const INDEX_INPUT = 1 << 36;
        /// Vertex attribute input
        const VERTEX_ATTRIBUTE_INPUT = 1 << 37;
        /// Pre-rasterization shaders
        const PRE_RASTERIZATION_SHADERS = 1 << 38;
        /// Task shader
        const TASK_SHADER = 1 << 39;
        /// Mesh shader
        const MESH_SHADER = 1 << 40;
        /// Ray tracing shader
        const RAY_TRACING_SHADER = 1 << 41;
        /// Fragment shading rate attachment
        const FRAGMENT_SHADING_RATE_ATTACHMENT = 1 << 42;
        /// Acceleration structure build
        const ACCELERATION_STRUCTURE_BUILD = 1 << 43;
        /// Acceleration structure copy
        const ACCELERATION_STRUCTURE_COPY = 1 << 44;
    }
}

impl PipelineStageFlags2 {
    /// All transfer stages
    pub const ALL_TRANSFER: Self = Self::from_bits_truncate(
        Self::COPY.bits() | Self::RESOLVE.bits() | Self::BLIT.bits() | Self::CLEAR.bits(),
    );
}

/// Device queue info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DeviceQueueInfo {
    /// Queue family index
    pub queue_family_index: u32,
    /// Queue index within family
    pub queue_index: u32,
    /// Queue flags
    pub flags: DeviceQueueCreateFlags,
}

bitflags::bitflags! {
    /// Device queue creation flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct DeviceQueueCreateFlags: u32 {
        /// Queue is protected
        const PROTECTED = 1 << 0;
    }
}

impl DeviceQueueCreateFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}

/// Queue family properties
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct QueueFamilyProperties {
    /// Queue capability flags
    pub queue_flags: QueueFlags,
    /// Number of queues in this family
    pub queue_count: u32,
    /// Timestamp valid bits
    pub timestamp_valid_bits: u32,
    /// Minimum image transfer granularity
    pub min_image_transfer_granularity: Extent3D,
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

bitflags::bitflags! {
    /// Queue capability flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct QueueFlags: u32 {
        /// Supports graphics operations
        const GRAPHICS = 1 << 0;
        /// Supports compute operations
        const COMPUTE = 1 << 1;
        /// Supports transfer operations
        const TRANSFER = 1 << 2;
        /// Supports sparse binding
        const SPARSE_BINDING = 1 << 3;
        /// Protected queue
        const PROTECTED = 1 << 4;
        /// Supports video decode
        const VIDEO_DECODE = 1 << 5;
        /// Supports video encode
        const VIDEO_ENCODE = 1 << 6;
        /// Supports optical flow
        const OPTICAL_FLOW = 1 << 7;
    }
}

impl QueueFlags {
    /// No capabilities
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }

    /// Supports graphics
    pub const fn supports_graphics(self) -> bool {
        self.contains(Self::GRAPHICS)
    }

    /// Supports compute
    pub const fn supports_compute(self) -> bool {
        self.contains(Self::COMPUTE)
    }

    /// Supports transfer
    pub const fn supports_transfer(self) -> bool {
        self.contains(Self::TRANSFER)
    }
}

/// Indirect command argument types
pub mod indirect {
    /// Draw indirect command
    #[derive(Clone, Copy, Debug, Default)]
    #[repr(C)]
    pub struct DrawIndirectCommand {
        /// Vertex count
        pub vertex_count: u32,
        /// Instance count
        pub instance_count: u32,
        /// First vertex
        pub first_vertex: u32,
        /// First instance
        pub first_instance: u32,
    }

    impl DrawIndirectCommand {
        /// Size in bytes
        pub const SIZE: usize = 16;

        /// Creates a draw command
        pub const fn new(vertex_count: u32, instance_count: u32) -> Self {
            Self {
                vertex_count,
                instance_count,
                first_vertex: 0,
                first_instance: 0,
            }
        }
    }

    /// Draw indexed indirect command
    #[derive(Clone, Copy, Debug, Default)]
    #[repr(C)]
    pub struct DrawIndexedIndirectCommand {
        /// Index count
        pub index_count: u32,
        /// Instance count
        pub instance_count: u32,
        /// First index
        pub first_index: u32,
        /// Vertex offset
        pub vertex_offset: i32,
        /// First instance
        pub first_instance: u32,
    }

    impl DrawIndexedIndirectCommand {
        /// Size in bytes
        pub const SIZE: usize = 20;

        /// Creates an indexed draw command
        pub const fn new(index_count: u32, instance_count: u32) -> Self {
            Self {
                index_count,
                instance_count,
                first_index: 0,
                vertex_offset: 0,
                first_instance: 0,
            }
        }
    }

    /// Dispatch indirect command
    #[derive(Clone, Copy, Debug)]
    #[repr(C)]
    pub struct DispatchIndirectCommand {
        /// Workgroup count X
        pub x: u32,
        /// Workgroup count Y
        pub y: u32,
        /// Workgroup count Z
        pub z: u32,
    }

    impl DispatchIndirectCommand {
        /// Size in bytes
        pub const SIZE: usize = 12;

        /// Creates a dispatch command
        pub const fn new(x: u32, y: u32, z: u32) -> Self {
            Self { x, y, z }
        }
    }

    /// Draw mesh tasks indirect command
    #[derive(Clone, Copy, Debug)]
    #[repr(C)]
    pub struct DrawMeshTasksIndirectCommand {
        /// Task group count X
        pub group_count_x: u32,
        /// Task group count Y
        pub group_count_y: u32,
        /// Task group count Z
        pub group_count_z: u32,
    }

    impl DrawMeshTasksIndirectCommand {
        /// Size in bytes
        pub const SIZE: usize = 12;

        /// Creates a mesh tasks command
        pub const fn new(x: u32, y: u32, z: u32) -> Self {
            Self {
                group_count_x: x,
                group_count_y: y,
                group_count_z: z,
            }
        }
    }

    /// Trace rays indirect command
    #[derive(Clone, Copy, Debug)]
    #[repr(C)]
    pub struct TraceRaysIndirectCommand {
        /// Width
        pub width: u32,
        /// Height
        pub height: u32,
        /// Depth
        pub depth: u32,
    }

    impl TraceRaysIndirectCommand {
        /// Size in bytes
        pub const SIZE: usize = 12;

        /// Creates a trace rays command
        pub const fn new(width: u32, height: u32, depth: u32) -> Self {
            Self { width, height, depth }
        }

        /// For 2D tracing
        pub const fn d2(width: u32, height: u32) -> Self {
            Self::new(width, height, 1)
        }
    }
}

pub use indirect::*;
