//! Event Synchronization for Lumina
//!
//! This module provides event types for fine-grained GPU synchronization
//! within command buffers.

// ============================================================================
// Event Handle
// ============================================================================

/// Event handle for fine-grained synchronization
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct EventHandle(pub u64);

impl EventHandle {
    /// Null event handle
    pub const NULL: Self = Self(0);

    /// Creates a new event handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }

    /// Raw value
    #[inline]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

impl Default for EventHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Event Configuration
// ============================================================================

/// Event creation configuration
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct EventConfig {
    /// Event flags
    pub flags: EventFlags,
    /// Debug name hash
    pub debug_name_hash: u32,
}

impl EventConfig {
    /// Creates new event config
    #[inline]
    pub const fn new() -> Self {
        Self {
            flags: EventFlags::NONE,
            debug_name_hash: 0,
        }
    }

    /// Device-only event (optimized)
    #[inline]
    pub const fn device_only() -> Self {
        Self {
            flags: EventFlags::DEVICE_ONLY,
            debug_name_hash: 0,
        }
    }

    /// With flags
    #[inline]
    pub const fn with_flags(mut self, flags: EventFlags) -> Self {
        self.flags = flags;
        self
    }

    /// With debug name hash
    #[inline]
    pub const fn with_name(mut self, hash: u32) -> Self {
        self.debug_name_hash = hash;
        self
    }
}

impl Default for EventConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Event flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct EventFlags(pub u32);

impl EventFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Device-only event (cannot be signaled/waited from host)
    pub const DEVICE_ONLY: Self = Self(1 << 0);

    /// Contains flag
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Is device only
    #[inline]
    pub const fn is_device_only(&self) -> bool {
        self.contains(Self::DEVICE_ONLY)
    }
}

// ============================================================================
// Event State
// ============================================================================

/// Event state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum EventState {
    /// Event is reset
    #[default]
    Reset = 0,
    /// Event is set
    Set   = 1,
}

impl EventState {
    /// Is set
    #[inline]
    pub const fn is_set(&self) -> bool {
        matches!(self, Self::Set)
    }

    /// Is reset
    #[inline]
    pub const fn is_reset(&self) -> bool {
        matches!(self, Self::Reset)
    }
}

// ============================================================================
// Event Dependency
// ============================================================================

/// Event dependency info for synchronization
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct EventDependency {
    /// Source stage mask
    pub src_stage_mask: PipelineStageFlags,
    /// Destination stage mask
    pub dst_stage_mask: PipelineStageFlags,
    /// Dependency flags
    pub dependency_flags: DependencyFlags,
}

impl EventDependency {
    /// Creates new event dependency
    #[inline]
    pub const fn new(src_stage: PipelineStageFlags, dst_stage: PipelineStageFlags) -> Self {
        Self {
            src_stage_mask: src_stage,
            dst_stage_mask: dst_stage,
            dependency_flags: DependencyFlags::NONE,
        }
    }

    /// Compute to compute dependency
    pub const fn compute_to_compute() -> Self {
        Self::new(
            PipelineStageFlags::COMPUTE_SHADER,
            PipelineStageFlags::COMPUTE_SHADER,
        )
    }

    /// Compute to graphics dependency
    pub const fn compute_to_graphics() -> Self {
        Self::new(
            PipelineStageFlags::COMPUTE_SHADER,
            PipelineStageFlags::ALL_GRAPHICS,
        )
    }

    /// Graphics to compute dependency
    pub const fn graphics_to_compute() -> Self {
        Self::new(
            PipelineStageFlags::ALL_GRAPHICS,
            PipelineStageFlags::COMPUTE_SHADER,
        )
    }

    /// Transfer to graphics
    pub const fn transfer_to_graphics() -> Self {
        Self::new(
            PipelineStageFlags::TRANSFER,
            PipelineStageFlags::ALL_GRAPHICS,
        )
    }

    /// Graphics to transfer
    pub const fn graphics_to_transfer() -> Self {
        Self::new(
            PipelineStageFlags::ALL_GRAPHICS,
            PipelineStageFlags::TRANSFER,
        )
    }

    /// With dependency flags
    #[inline]
    pub const fn with_flags(mut self, flags: DependencyFlags) -> Self {
        self.dependency_flags = flags;
        self
    }

    /// By region (for render pass dependencies)
    #[inline]
    pub const fn by_region(mut self) -> Self {
        self.dependency_flags = self.dependency_flags.union(DependencyFlags::BY_REGION);
        self
    }
}

impl Default for EventDependency {
    fn default() -> Self {
        Self::new(
            PipelineStageFlags::ALL_COMMANDS,
            PipelineStageFlags::ALL_COMMANDS,
        )
    }
}

/// Pipeline stage flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct PipelineStageFlags(pub u64);

impl PipelineStageFlags {
    /// No stage
    pub const NONE: Self = Self(0);
    /// Top of pipe
    pub const TOP_OF_PIPE: Self = Self(1 << 0);
    /// Draw indirect
    pub const DRAW_INDIRECT: Self = Self(1 << 1);
    /// Vertex input
    pub const VERTEX_INPUT: Self = Self(1 << 2);
    /// Vertex shader
    pub const VERTEX_SHADER: Self = Self(1 << 3);
    /// Tessellation control shader
    pub const TESSELLATION_CONTROL_SHADER: Self = Self(1 << 4);
    /// Tessellation evaluation shader
    pub const TESSELLATION_EVALUATION_SHADER: Self = Self(1 << 5);
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
    /// Task shader
    pub const TASK_SHADER: Self = Self(1 << 24);
    /// Mesh shader
    pub const MESH_SHADER: Self = Self(1 << 25);
    /// Ray tracing shader
    pub const RAY_TRACING_SHADER: Self = Self(1 << 26);
    /// Acceleration structure build
    pub const ACCELERATION_STRUCTURE_BUILD: Self = Self(1 << 27);
    /// Acceleration structure copy
    pub const ACCELERATION_STRUCTURE_COPY: Self = Self(1 << 28);
    /// Fragment density process
    pub const FRAGMENT_DENSITY_PROCESS: Self = Self(1 << 29);
    /// Fragment shading rate attachment
    pub const FRAGMENT_SHADING_RATE_ATTACHMENT: Self = Self(1 << 30);
    /// Command preprocess
    pub const COMMAND_PREPROCESS: Self = Self(1 << 31);
    /// Conditional rendering
    pub const CONDITIONAL_RENDERING: Self = Self(1 << 32);
    /// Transform feedback
    pub const TRANSFORM_FEEDBACK: Self = Self(1 << 33);
    /// Video decode
    pub const VIDEO_DECODE: Self = Self(1 << 34);
    /// Video encode
    pub const VIDEO_ENCODE: Self = Self(1 << 35);
    /// Optical flow
    pub const OPTICAL_FLOW: Self = Self(1 << 36);

    /// Contains flag
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Intersection
    #[inline]
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }
}

/// Dependency flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct DependencyFlags(pub u32);

impl DependencyFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// By region (for local framebuffer dependencies)
    pub const BY_REGION: Self = Self(1 << 0);
    /// Device group (for multi-GPU)
    pub const DEVICE_GROUP: Self = Self(1 << 1);
    /// View local
    pub const VIEW_LOCAL: Self = Self(1 << 2);
    /// Feedback loop
    pub const FEEDBACK_LOOP: Self = Self(1 << 3);

    /// Contains flag
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
// Event Wait Info
// ============================================================================

/// Event wait info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct EventWaitInfo {
    /// Event handle
    pub event: EventHandle,
    /// Wait stages
    pub wait_stages: PipelineStageFlags,
}

impl EventWaitInfo {
    /// Creates new wait info
    #[inline]
    pub const fn new(event: EventHandle, stages: PipelineStageFlags) -> Self {
        Self {
            event,
            wait_stages: stages,
        }
    }

    /// Wait for all commands
    #[inline]
    pub const fn all_commands(event: EventHandle) -> Self {
        Self::new(event, PipelineStageFlags::ALL_COMMANDS)
    }

    /// Wait for compute
    #[inline]
    pub const fn compute(event: EventHandle) -> Self {
        Self::new(event, PipelineStageFlags::COMPUTE_SHADER)
    }

    /// Wait for transfer
    #[inline]
    pub const fn transfer(event: EventHandle) -> Self {
        Self::new(event, PipelineStageFlags::TRANSFER)
    }
}

impl Default for EventWaitInfo {
    fn default() -> Self {
        Self::new(EventHandle::NULL, PipelineStageFlags::ALL_COMMANDS)
    }
}

/// Multiple event wait
#[derive(Clone, Debug)]
#[repr(C)]
pub struct MultiEventWait {
    /// Events to wait for
    pub events: [EventHandle; 16],
    /// Event count
    pub event_count: u32,
    /// Wait stages
    pub wait_stages: PipelineStageFlags,
}

impl MultiEventWait {
    /// Creates new multi-event wait
    pub fn new(events: &[EventHandle], stages: PipelineStageFlags) -> Self {
        let mut handles = [EventHandle::NULL; 16];
        let count = events.len().min(16);
        handles[..count].copy_from_slice(&events[..count]);
        Self {
            events: handles,
            event_count: count as u32,
            wait_stages: stages,
        }
    }

    /// Wait for all commands
    #[inline]
    pub fn all_commands(events: &[EventHandle]) -> Self {
        Self::new(events, PipelineStageFlags::ALL_COMMANDS)
    }
}

impl Default for MultiEventWait {
    fn default() -> Self {
        Self {
            events: [EventHandle::NULL; 16],
            event_count: 0,
            wait_stages: PipelineStageFlags::ALL_COMMANDS,
        }
    }
}

// ============================================================================
// Event Signal Info
// ============================================================================

/// Event signal info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct EventSignalInfo {
    /// Event handle
    pub event: EventHandle,
    /// Signal stages
    pub signal_stages: PipelineStageFlags,
}

impl EventSignalInfo {
    /// Creates new signal info
    #[inline]
    pub const fn new(event: EventHandle, stages: PipelineStageFlags) -> Self {
        Self {
            event,
            signal_stages: stages,
        }
    }

    /// Signal after all commands
    #[inline]
    pub const fn all_commands(event: EventHandle) -> Self {
        Self::new(event, PipelineStageFlags::ALL_COMMANDS)
    }

    /// Signal after compute
    #[inline]
    pub const fn compute(event: EventHandle) -> Self {
        Self::new(event, PipelineStageFlags::COMPUTE_SHADER)
    }

    /// Signal after transfer
    #[inline]
    pub const fn transfer(event: EventHandle) -> Self {
        Self::new(event, PipelineStageFlags::TRANSFER)
    }

    /// Signal after fragment shader
    #[inline]
    pub const fn fragment(event: EventHandle) -> Self {
        Self::new(event, PipelineStageFlags::FRAGMENT_SHADER)
    }
}

impl Default for EventSignalInfo {
    fn default() -> Self {
        Self::new(EventHandle::NULL, PipelineStageFlags::ALL_COMMANDS)
    }
}

// ============================================================================
// Event Pool
// ============================================================================

/// Event pool configuration
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct EventPoolConfig {
    /// Initial pool size
    pub initial_size: u32,
    /// Maximum pool size
    pub max_size: u32,
    /// Device-only events
    pub device_only: bool,
}

impl EventPoolConfig {
    /// Default pool
    pub const DEFAULT: Self = Self {
        initial_size: 16,
        max_size: 128,
        device_only: true,
    };

    /// Small pool
    pub const SMALL: Self = Self {
        initial_size: 8,
        max_size: 32,
        device_only: true,
    };

    /// Large pool
    pub const LARGE: Self = Self {
        initial_size: 32,
        max_size: 256,
        device_only: true,
    };

    /// Creates new pool config
    #[inline]
    pub const fn new(initial: u32, max: u32) -> Self {
        Self {
            initial_size: initial,
            max_size: max,
            device_only: true,
        }
    }

    /// With device-only flag
    #[inline]
    pub const fn with_device_only(mut self, device_only: bool) -> Self {
        self.device_only = device_only;
        self
    }
}

impl Default for EventPoolConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Event pool statistics
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct EventPoolStats {
    /// Total events created
    pub total_created: u32,
    /// Currently available
    pub available: u32,
    /// Currently in use
    pub in_use: u32,
    /// Peak usage
    pub peak_usage: u32,
}

impl EventPoolStats {
    /// Creates empty stats
    #[inline]
    pub const fn new() -> Self {
        Self {
            total_created: 0,
            available: 0,
            in_use: 0,
            peak_usage: 0,
        }
    }
}
