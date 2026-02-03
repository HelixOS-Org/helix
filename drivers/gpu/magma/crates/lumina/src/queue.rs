//! Queue and submission types
//!
//! This module provides types for GPU queue management and command submission.

extern crate alloc;
use alloc::vec::Vec;

use crate::command::CommandBufferHandle;
use crate::sync::{FenceHandle, SemaphoreHandle, TimelineSemaphoreSubmitInfo};

/// Queue handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct QueueHandle(pub u64);

impl QueueHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Checks if null
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

/// Queue family index
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct QueueFamilyIndex(pub u32);

impl QueueFamilyIndex {
    /// External queue family
    pub const EXTERNAL: Self = Self(u32::MAX - 1);
    /// Foreign queue family
    pub const FOREIGN: Self = Self(u32::MAX - 2);
    /// Ignored queue family
    pub const IGNORED: Self = Self(u32::MAX);
}

/// Queue type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum QueueType {
    /// Graphics queue (supports all operations)
    Graphics,
    /// Compute-only queue
    Compute,
    /// Transfer-only queue
    Transfer,
    /// Sparse binding queue
    SparseBinding,
    /// Video decode queue
    VideoDecode,
    /// Video encode queue
    VideoEncode,
    /// Protected queue
    Protected,
}

impl QueueType {
    /// Checks if this queue type supports graphics
    pub const fn supports_graphics(&self) -> bool {
        matches!(self, Self::Graphics)
    }

    /// Checks if this queue type supports compute
    pub const fn supports_compute(&self) -> bool {
        matches!(self, Self::Graphics | Self::Compute)
    }

    /// Checks if this queue type supports transfer
    pub const fn supports_transfer(&self) -> bool {
        // All queue types support transfer
        true
    }
}

/// Queue capability flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct QueueCapabilities(pub u32);

impl QueueCapabilities {
    /// Graphics operations
    pub const GRAPHICS: Self = Self(1 << 0);
    /// Compute operations
    pub const COMPUTE: Self = Self(1 << 1);
    /// Transfer operations
    pub const TRANSFER: Self = Self(1 << 2);
    /// Sparse binding operations
    pub const SPARSE_BINDING: Self = Self(1 << 3);
    /// Protected operations
    pub const PROTECTED: Self = Self(1 << 4);
    /// Video decode
    pub const VIDEO_DECODE: Self = Self(1 << 5);
    /// Video encode
    pub const VIDEO_ENCODE: Self = Self(1 << 6);
    /// Optical flow
    pub const OPTICAL_FLOW: Self = Self(1 << 7);

    /// Checks if capability is set
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Checks if supports graphics
    pub const fn supports_graphics(&self) -> bool {
        self.contains(Self::GRAPHICS)
    }

    /// Checks if supports compute
    pub const fn supports_compute(&self) -> bool {
        self.contains(Self::COMPUTE)
    }

    /// Checks if supports transfer
    pub const fn supports_transfer(&self) -> bool {
        self.contains(Self::TRANSFER)
    }
}

impl core::ops::BitOr for QueueCapabilities {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitAnd for QueueCapabilities {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

/// Queue family properties
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct QueueFamilyProperties {
    /// Capabilities
    pub capabilities: QueueCapabilities,
    /// Number of queues in this family
    pub queue_count: u32,
    /// Timestamp valid bits
    pub timestamp_valid_bits: u32,
    /// Minimum image transfer granularity
    pub min_image_transfer_granularity: [u32; 3],
}

impl QueueFamilyProperties {
    /// Checks if this family supports graphics
    pub const fn supports_graphics(&self) -> bool {
        self.capabilities.supports_graphics()
    }

    /// Checks if this family supports compute
    pub const fn supports_compute(&self) -> bool {
        self.capabilities.supports_compute()
    }

    /// Checks if this family supports transfer
    pub const fn supports_transfer(&self) -> bool {
        self.capabilities.supports_transfer()
    }

    /// Checks if this family supports presentation
    pub const fn supports_present(&self) -> bool {
        // Typically graphics queues support present
        self.supports_graphics()
    }
}

/// Queue priority
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(transparent)]
pub struct QueuePriority(pub f32);

impl QueuePriority {
    /// Low priority
    pub const LOW: Self = Self(0.0);
    /// Medium priority (default)
    pub const MEDIUM: Self = Self(0.5);
    /// High priority
    pub const HIGH: Self = Self(1.0);

    /// Creates a new queue priority
    pub const fn new(priority: f32) -> Self {
        Self(priority)
    }
}

impl Default for QueuePriority {
    fn default() -> Self {
        Self::MEDIUM
    }
}

/// Queue creation info
#[derive(Clone, Debug, Default)]
pub struct QueueCreateInfo {
    /// Queue family index
    pub family_index: QueueFamilyIndex,
    /// Priorities for each queue
    pub priorities: Vec<QueuePriority>,
    /// Queue creation flags
    pub flags: QueueCreateFlags,
}

impl QueueCreateInfo {
    /// Creates new queue create info
    pub fn new(family_index: u32) -> Self {
        Self {
            family_index: QueueFamilyIndex(family_index),
            priorities: Vec::new(),
            flags: QueueCreateFlags::NONE,
        }
    }

    /// Adds a queue with priority
    pub fn add_queue(mut self, priority: QueuePriority) -> Self {
        self.priorities.push(priority);
        self
    }

    /// Adds N queues with medium priority
    pub fn with_count(mut self, count: u32) -> Self {
        for _ in 0..count {
            self.priorities.push(QueuePriority::MEDIUM);
        }
        self
    }

    /// Sets flags
    pub fn with_flags(mut self, flags: QueueCreateFlags) -> Self {
        self.flags = flags;
        self
    }
}

/// Queue creation flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct QueueCreateFlags(pub u32);

impl QueueCreateFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Protected queue
    pub const PROTECTED: Self = Self(1 << 0);
}

/// Submit info for queue submission
#[derive(Clone, Debug, Default)]
pub struct SubmitInfo {
    /// Semaphores to wait on
    pub wait_semaphores: Vec<SemaphoreWaitInfo>,
    /// Command buffers to execute
    pub command_buffers: Vec<CommandBufferHandle>,
    /// Semaphores to signal
    pub signal_semaphores: Vec<SemaphoreHandle>,
}

impl SubmitInfo {
    /// Creates new submit info
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a command buffer
    pub fn add_command_buffer(mut self, cmd: CommandBufferHandle) -> Self {
        self.command_buffers.push(cmd);
        self
    }

    /// Adds wait semaphore with stage mask
    pub fn wait_for(mut self, semaphore: SemaphoreHandle, stages: PipelineStages) -> Self {
        self.wait_semaphores.push(SemaphoreWaitInfo {
            semaphore,
            stages,
            value: 0,
        });
        self
    }

    /// Adds signal semaphore
    pub fn signal(mut self, semaphore: SemaphoreHandle) -> Self {
        self.signal_semaphores.push(semaphore);
        self
    }
}

/// Semaphore wait info
#[derive(Clone, Copy, Debug)]
pub struct SemaphoreWaitInfo {
    /// Semaphore to wait on
    pub semaphore: SemaphoreHandle,
    /// Pipeline stages to wait at
    pub stages: PipelineStages,
    /// Timeline value (for timeline semaphores)
    pub value: u64,
}

impl SemaphoreWaitInfo {
    /// Creates a new wait info
    pub const fn new(semaphore: SemaphoreHandle, stages: PipelineStages) -> Self {
        Self {
            semaphore,
            stages,
            value: 0,
        }
    }

    /// Creates a timeline wait info
    pub const fn timeline(semaphore: SemaphoreHandle, stages: PipelineStages, value: u64) -> Self {
        Self {
            semaphore,
            stages,
            value,
        }
    }
}

/// Pipeline stage flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct PipelineStages(pub u32);

impl PipelineStages {
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
    /// All graphics stages
    pub const ALL_GRAPHICS: Self = Self(1 << 15);
    /// All commands
    pub const ALL_COMMANDS: Self = Self(1 << 16);
    /// Copy
    pub const COPY: Self = Self(1 << 17);
    /// Resolve
    pub const RESOLVE: Self = Self(1 << 18);
    /// Blit
    pub const BLIT: Self = Self(1 << 19);
    /// Clear
    pub const CLEAR: Self = Self(1 << 20);
    /// Index input
    pub const INDEX_INPUT: Self = Self(1 << 21);
    /// Vertex attribute input
    pub const VERTEX_ATTRIBUTE_INPUT: Self = Self(1 << 22);
    /// Pre-rasterization shaders
    pub const PRE_RASTERIZATION_SHADERS: Self = Self(1 << 23);
    /// Acceleration structure build
    pub const ACCELERATION_STRUCTURE_BUILD: Self = Self(1 << 24);
    /// Ray tracing shader
    pub const RAY_TRACING_SHADER: Self = Self(1 << 25);
    /// Task shader
    pub const TASK_SHADER: Self = Self(1 << 26);
    /// Mesh shader
    pub const MESH_SHADER: Self = Self(1 << 27);

    /// All stages
    pub const ALL: Self = Self(0xFFFFFFFF);
}

impl core::ops::BitOr for PipelineStages {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitAnd for PipelineStages {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

/// Extended submit info (for timeline semaphores)
#[derive(Clone, Debug, Default)]
pub struct SubmitInfo2 {
    /// Submit flags
    pub flags: SubmitFlags,
    /// Wait semaphore infos
    pub wait_semaphore_infos: Vec<SemaphoreSubmitInfo>,
    /// Command buffer infos
    pub command_buffer_infos: Vec<CommandBufferSubmitInfo>,
    /// Signal semaphore infos
    pub signal_semaphore_infos: Vec<SemaphoreSubmitInfo>,
}

impl SubmitInfo2 {
    /// Creates new submit info
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a command buffer
    pub fn add_command_buffer(mut self, cmd: CommandBufferHandle) -> Self {
        self.command_buffer_infos.push(CommandBufferSubmitInfo {
            command_buffer: cmd,
            device_mask: 0,
        });
        self
    }

    /// Adds wait semaphore
    pub fn wait_for(mut self, info: SemaphoreSubmitInfo) -> Self {
        self.wait_semaphore_infos.push(info);
        self
    }

    /// Adds signal semaphore
    pub fn signal(mut self, info: SemaphoreSubmitInfo) -> Self {
        self.signal_semaphore_infos.push(info);
        self
    }
}

/// Submit flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct SubmitFlags(pub u32);

impl SubmitFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Protected submission
    pub const PROTECTED: Self = Self(1 << 0);
}

/// Semaphore submit info
#[derive(Clone, Copy, Debug)]
pub struct SemaphoreSubmitInfo {
    /// Semaphore
    pub semaphore: SemaphoreHandle,
    /// Timeline value
    pub value: u64,
    /// Pipeline stage mask
    pub stage_mask: PipelineStages,
    /// Device index (for multi-GPU)
    pub device_index: u32,
}

impl SemaphoreSubmitInfo {
    /// Creates new semaphore submit info
    pub const fn new(semaphore: SemaphoreHandle, stage_mask: PipelineStages) -> Self {
        Self {
            semaphore,
            value: 0,
            stage_mask,
            device_index: 0,
        }
    }

    /// Creates timeline semaphore info
    pub const fn timeline(
        semaphore: SemaphoreHandle,
        value: u64,
        stage_mask: PipelineStages,
    ) -> Self {
        Self {
            semaphore,
            value,
            stage_mask,
            device_index: 0,
        }
    }
}

/// Command buffer submit info
#[derive(Clone, Copy, Debug)]
pub struct CommandBufferSubmitInfo {
    /// Command buffer
    pub command_buffer: CommandBufferHandle,
    /// Device mask for multi-GPU
    pub device_mask: u32,
}

impl CommandBufferSubmitInfo {
    /// Creates new submit info
    pub const fn new(command_buffer: CommandBufferHandle) -> Self {
        Self {
            command_buffer,
            device_mask: 0,
        }
    }
}

/// Present info
#[derive(Clone, Debug, Default)]
pub struct PresentInfo {
    /// Semaphores to wait on
    pub wait_semaphores: Vec<SemaphoreHandle>,
    /// Swapchains to present to
    pub swapchains: Vec<SwapchainPresentInfo>,
}

impl PresentInfo {
    /// Creates new present info
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds wait semaphore
    pub fn wait_for(mut self, semaphore: SemaphoreHandle) -> Self {
        self.wait_semaphores.push(semaphore);
        self
    }

    /// Adds swapchain to present
    pub fn add_swapchain(mut self, swapchain: SwapchainHandle, image_index: u32) -> Self {
        self.swapchains.push(SwapchainPresentInfo {
            swapchain,
            image_index,
        });
        self
    }
}

/// Swapchain present info
#[derive(Clone, Copy, Debug)]
pub struct SwapchainPresentInfo {
    /// Swapchain handle
    pub swapchain: SwapchainHandle,
    /// Image index to present
    pub image_index: u32,
}

/// Swapchain handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SwapchainHandle(pub u64);

impl SwapchainHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

/// Queue wait idle info
#[derive(Clone, Copy, Debug, Default)]
pub struct QueueWaitIdleInfo {
    /// Timeout in nanoseconds (0 = wait forever)
    pub timeout_ns: u64,
}

impl QueueWaitIdleInfo {
    /// Wait forever
    pub const FOREVER: Self = Self { timeout_ns: 0 };

    /// Wait with timeout
    pub const fn with_timeout_ms(ms: u64) -> Self {
        Self {
            timeout_ns: ms * 1_000_000,
        }
    }
}

/// Queue global priority
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum QueueGlobalPriority {
    /// Low priority
    Low = 0,
    /// Medium priority
    Medium = 1,
    /// High priority
    High = 2,
    /// Realtime priority
    Realtime = 3,
}

impl Default for QueueGlobalPriority {
    fn default() -> Self {
        Self::Medium
    }
}

/// Checkpoint data for device lost debugging
#[derive(Clone, Debug)]
pub struct CheckpointData {
    /// Stage at which checkpoint was executed
    pub stage: PipelineStages,
    /// Checkpoint marker data
    pub marker_data: u64,
}

impl CheckpointData {
    /// Creates new checkpoint data
    pub const fn new(stage: PipelineStages, marker_data: u64) -> Self {
        Self { stage, marker_data }
    }
}

/// Queue family global priority properties
#[derive(Clone, Copy, Debug, Default)]
pub struct QueueFamilyGlobalPriorityProperties {
    /// Number of supported priorities
    pub priority_count: u32,
    /// Supported priorities
    pub priorities: [QueueGlobalPriority; 4],
}

/// Performance query pool info
#[derive(Clone, Copy, Debug)]
pub struct PerformanceQueryCreateInfo {
    /// Queue family index
    pub queue_family_index: u32,
    /// Counter indices to query
    pub counter_indices: [u32; 16],
    /// Number of counters
    pub counter_count: u32,
}

impl PerformanceQueryCreateInfo {
    /// Creates new performance query info
    pub const fn new(queue_family_index: u32) -> Self {
        Self {
            queue_family_index,
            counter_indices: [0; 16],
            counter_count: 0,
        }
    }
}
