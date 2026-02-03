//! Queue Family Types for Lumina
//!
//! This module provides queue family types, queue configuration,
//! and queue submission structures.

// ============================================================================
// Queue Family Properties
// ============================================================================

/// Queue family properties
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct QueueFamilyProperties {
    /// Queue flags (capabilities)
    pub queue_flags: QueueFlags,
    /// Queue count in this family
    pub queue_count: u32,
    /// Timestamp valid bits
    pub timestamp_valid_bits: u32,
    /// Minimum image transfer granularity
    pub min_image_transfer_granularity: Extent3D,
}

impl QueueFamilyProperties {
    /// Creates new properties
    #[inline]
    pub const fn new(flags: QueueFlags, count: u32) -> Self {
        Self {
            queue_flags: flags,
            queue_count: count,
            timestamp_valid_bits: 64,
            min_image_transfer_granularity: Extent3D::UNIT,
        }
    }

    /// Graphics queue family
    #[inline]
    pub const fn graphics(count: u32) -> Self {
        Self::new(
            QueueFlags::GRAPHICS
                .union(QueueFlags::COMPUTE)
                .union(QueueFlags::TRANSFER),
            count,
        )
    }

    /// Compute-only queue family
    #[inline]
    pub const fn compute(count: u32) -> Self {
        Self::new(QueueFlags::COMPUTE.union(QueueFlags::TRANSFER), count)
    }

    /// Transfer-only queue family
    #[inline]
    pub const fn transfer(count: u32) -> Self {
        Self::new(QueueFlags::TRANSFER, count)
    }

    /// Supports graphics
    #[inline]
    pub const fn supports_graphics(&self) -> bool {
        self.queue_flags.contains(QueueFlags::GRAPHICS)
    }

    /// Supports compute
    #[inline]
    pub const fn supports_compute(&self) -> bool {
        self.queue_flags.contains(QueueFlags::COMPUTE)
    }

    /// Supports transfer
    #[inline]
    pub const fn supports_transfer(&self) -> bool {
        self.queue_flags.contains(QueueFlags::TRANSFER)
    }

    /// Supports sparse binding
    #[inline]
    pub const fn supports_sparse_binding(&self) -> bool {
        self.queue_flags.contains(QueueFlags::SPARSE_BINDING)
    }

    /// Supports protected
    #[inline]
    pub const fn supports_protected(&self) -> bool {
        self.queue_flags.contains(QueueFlags::PROTECTED)
    }

    /// Supports video decode
    #[inline]
    pub const fn supports_video_decode(&self) -> bool {
        self.queue_flags.contains(QueueFlags::VIDEO_DECODE)
    }

    /// Supports video encode
    #[inline]
    pub const fn supports_video_encode(&self) -> bool {
        self.queue_flags.contains(QueueFlags::VIDEO_ENCODE)
    }

    /// Is dedicated compute (no graphics)
    #[inline]
    pub const fn is_dedicated_compute(&self) -> bool {
        self.supports_compute() && !self.supports_graphics()
    }

    /// Is dedicated transfer (no compute or graphics)
    #[inline]
    pub const fn is_dedicated_transfer(&self) -> bool {
        self.supports_transfer() && !self.supports_compute() && !self.supports_graphics()
    }
}

impl Default for QueueFamilyProperties {
    fn default() -> Self {
        Self::graphics(1)
    }
}

// ============================================================================
// Queue Flags
// ============================================================================

/// Queue capability flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct QueueFlags(pub u32);

impl QueueFlags {
    /// No capabilities
    pub const NONE: Self = Self(0);
    /// Graphics operations
    pub const GRAPHICS: Self = Self(1 << 0);
    /// Compute operations
    pub const COMPUTE: Self = Self(1 << 1);
    /// Transfer operations
    pub const TRANSFER: Self = Self(1 << 2);
    /// Sparse binding operations
    pub const SPARSE_BINDING: Self = Self(1 << 3);
    /// Protected memory operations
    pub const PROTECTED: Self = Self(1 << 4);
    /// Video decode operations
    pub const VIDEO_DECODE: Self = Self(1 << 5);
    /// Video encode operations
    pub const VIDEO_ENCODE: Self = Self(1 << 6);
    /// Optical flow operations
    pub const OPTICAL_FLOW: Self = Self(1 << 7);
    /// All operations
    pub const ALL: Self = Self(0xFF);

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

    /// Intersection
    #[inline]
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Is empty
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }
}

// ============================================================================
// Queue Handle
// ============================================================================

/// Queue handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct QueueHandle(pub u64);

impl QueueHandle {
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

impl Default for QueueHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Queue Create Info
// ============================================================================

/// Queue create info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct QueueCreateInfo {
    /// Queue family index
    pub queue_family_index: u32,
    /// Queue count
    pub queue_count: u32,
    /// Queue priorities (up to 8)
    pub priorities: [f32; 8],
    /// Flags
    pub flags: QueueCreateFlags,
}

impl QueueCreateInfo {
    /// Maximum queues per family
    pub const MAX_QUEUES: usize = 8;

    /// Creates new info
    #[inline]
    pub const fn new(family_index: u32, count: u32) -> Self {
        Self {
            queue_family_index: family_index,
            queue_count: count,
            priorities: [1.0; 8],
            flags: QueueCreateFlags::NONE,
        }
    }

    /// Single queue with default priority
    #[inline]
    pub const fn single(family_index: u32) -> Self {
        Self::new(family_index, 1)
    }

    /// With priorities
    #[inline]
    pub fn with_priorities(mut self, priorities: &[f32]) -> Self {
        let count = priorities.len().min(8);
        for (i, &p) in priorities.iter().take(count).enumerate() {
            self.priorities[i] = p;
        }
        self.queue_count = count as u32;
        self
    }

    /// With flags
    #[inline]
    pub const fn with_flags(mut self, flags: QueueCreateFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Protected queue
    #[inline]
    pub const fn protected(mut self) -> Self {
        self.flags = self.flags.union(QueueCreateFlags::PROTECTED);
        self
    }
}

impl Default for QueueCreateInfo {
    fn default() -> Self {
        Self::single(0)
    }
}

/// Queue create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct QueueCreateFlags(pub u32);

impl QueueCreateFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Protected queue
    pub const PROTECTED: Self = Self(1 << 0);

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
// Queue Submit Info
// ============================================================================

/// Queue submit info
#[derive(Clone, Debug)]
#[repr(C)]
pub struct QueueSubmitInfo<'a> {
    /// Wait semaphores
    pub wait_semaphores: &'a [SemaphoreSubmitInfo],
    /// Command buffers
    pub command_buffers: &'a [CommandBufferSubmitInfo],
    /// Signal semaphores
    pub signal_semaphores: &'a [SemaphoreSubmitInfo],
    /// Flags
    pub flags: SubmitFlags,
}

impl<'a> QueueSubmitInfo<'a> {
    /// Creates new submit info
    #[inline]
    pub const fn new(command_buffers: &'a [CommandBufferSubmitInfo]) -> Self {
        Self {
            wait_semaphores: &[],
            command_buffers,
            signal_semaphores: &[],
            flags: SubmitFlags::NONE,
        }
    }

    /// With wait semaphores
    #[inline]
    pub const fn with_wait(mut self, semaphores: &'a [SemaphoreSubmitInfo]) -> Self {
        self.wait_semaphores = semaphores;
        self
    }

    /// With signal semaphores
    #[inline]
    pub const fn with_signal(mut self, semaphores: &'a [SemaphoreSubmitInfo]) -> Self {
        self.signal_semaphores = semaphores;
        self
    }

    /// With flags
    #[inline]
    pub const fn with_flags(mut self, flags: SubmitFlags) -> Self {
        self.flags = flags;
        self
    }
}

impl Default for QueueSubmitInfo<'_> {
    fn default() -> Self {
        Self {
            wait_semaphores: &[],
            command_buffers: &[],
            signal_semaphores: &[],
            flags: SubmitFlags::NONE,
        }
    }
}

/// Submit flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct SubmitFlags(pub u32);

impl SubmitFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Protected submit
    pub const PROTECTED: Self = Self(1 << 0);

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
// Semaphore Submit Info
// ============================================================================

/// Semaphore submit info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SemaphoreSubmitInfo {
    /// Semaphore handle
    pub semaphore: u64,
    /// Value (for timeline semaphores)
    pub value: u64,
    /// Stage mask
    pub stage_mask: PipelineStageFlags2,
    /// Device index (for multi-GPU)
    pub device_index: u32,
}

impl SemaphoreSubmitInfo {
    /// Creates new info
    #[inline]
    pub const fn new(semaphore: u64, stage_mask: PipelineStageFlags2) -> Self {
        Self {
            semaphore,
            value: 0,
            stage_mask,
            device_index: 0,
        }
    }

    /// Binary semaphore
    #[inline]
    pub const fn binary(semaphore: u64, stage: PipelineStageFlags2) -> Self {
        Self::new(semaphore, stage)
    }

    /// Timeline semaphore
    #[inline]
    pub const fn timeline(semaphore: u64, value: u64, stage: PipelineStageFlags2) -> Self {
        Self {
            semaphore,
            value,
            stage_mask: stage,
            device_index: 0,
        }
    }

    /// Wait at all commands
    #[inline]
    pub const fn wait_all(semaphore: u64) -> Self {
        Self::new(semaphore, PipelineStageFlags2::ALL_COMMANDS)
    }

    /// Wait at top of pipe
    #[inline]
    pub const fn wait_top(semaphore: u64) -> Self {
        Self::new(semaphore, PipelineStageFlags2::TOP_OF_PIPE)
    }

    /// Signal at bottom of pipe
    #[inline]
    pub const fn signal_bottom(semaphore: u64) -> Self {
        Self::new(semaphore, PipelineStageFlags2::BOTTOM_OF_PIPE)
    }

    /// With device index
    #[inline]
    pub const fn with_device_index(mut self, index: u32) -> Self {
        self.device_index = index;
        self
    }
}

impl Default for SemaphoreSubmitInfo {
    fn default() -> Self {
        Self::new(0, PipelineStageFlags2::NONE)
    }
}

// ============================================================================
// Command Buffer Submit Info
// ============================================================================

/// Command buffer submit info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct CommandBufferSubmitInfo {
    /// Command buffer handle
    pub command_buffer: u64,
    /// Device mask (for multi-GPU)
    pub device_mask: u32,
}

impl CommandBufferSubmitInfo {
    /// Creates new info
    #[inline]
    pub const fn new(command_buffer: u64) -> Self {
        Self {
            command_buffer,
            device_mask: 0,
        }
    }

    /// With device mask
    #[inline]
    pub const fn with_device_mask(mut self, mask: u32) -> Self {
        self.device_mask = mask;
        self
    }
}

impl Default for CommandBufferSubmitInfo {
    fn default() -> Self {
        Self::new(0)
    }
}

// ============================================================================
// Pipeline Stage Flags 2
// ============================================================================

/// Pipeline stage flags (VK_PIPELINE_STAGE_2)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct PipelineStageFlags2(pub u64);

impl PipelineStageFlags2 {
    /// No stages
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
    /// All transfer
    pub const ALL_TRANSFER: Self = Self(1 << 12);
    /// Bottom of pipe
    pub const BOTTOM_OF_PIPE: Self = Self(1 << 13);
    /// Host
    pub const HOST: Self = Self(1 << 14);
    /// All graphics
    pub const ALL_GRAPHICS: Self = Self(1 << 15);
    /// All commands
    pub const ALL_COMMANDS: Self = Self(1 << 16);
    /// Copy
    pub const COPY: Self = Self(1 << 32);
    /// Resolve
    pub const RESOLVE: Self = Self(1 << 33);
    /// Blit
    pub const BLIT: Self = Self(1 << 34);
    /// Clear
    pub const CLEAR: Self = Self(1 << 35);
    /// Index input
    pub const INDEX_INPUT: Self = Self(1 << 36);
    /// Vertex attribute input
    pub const VERTEX_ATTRIBUTE_INPUT: Self = Self(1 << 37);
    /// Pre-rasterization shaders
    pub const PRE_RASTERIZATION_SHADERS: Self = Self(1 << 38);
    /// Video decode
    pub const VIDEO_DECODE: Self = Self(1 << 26);
    /// Video encode
    pub const VIDEO_ENCODE: Self = Self(1 << 27);
    /// Transform feedback
    pub const TRANSFORM_FEEDBACK: Self = Self(1 << 24);
    /// Conditional rendering
    pub const CONDITIONAL_RENDERING: Self = Self(1 << 18);
    /// Acceleration structure build
    pub const ACCELERATION_STRUCTURE_BUILD: Self = Self(1 << 25);
    /// Ray tracing shader
    pub const RAY_TRACING_SHADER: Self = Self(1 << 21);
    /// Fragment density process
    pub const FRAGMENT_DENSITY_PROCESS: Self = Self(1 << 23);
    /// Fragment shading rate attachment
    pub const FRAGMENT_SHADING_RATE_ATTACHMENT: Self = Self(1 << 22);
    /// Task shader
    pub const TASK_SHADER: Self = Self(1 << 19);
    /// Mesh shader
    pub const MESH_SHADER: Self = Self(1 << 20);
    /// Subpass shading
    pub const SUBPASS_SHADING: Self = Self(1 << 39);
    /// Invocation mask
    pub const INVOCATION_MASK: Self = Self(1 << 40);
    /// Acceleration structure copy
    pub const ACCELERATION_STRUCTURE_COPY: Self = Self(1 << 28);
    /// Micromap build
    pub const MICROMAP_BUILD: Self = Self(1 << 30);
    /// Optical flow
    pub const OPTICAL_FLOW: Self = Self(1 << 29);

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

    /// Intersection
    #[inline]
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Is empty
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }
}

// ============================================================================
// Extent 3D
// ============================================================================

/// 3D extent
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
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
    /// Unit extent (1x1x1)
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
        Self::new(width, height, 1)
    }

    /// Volume (total pixels/voxels)
    #[inline]
    pub const fn volume(&self) -> u64 {
        self.width as u64 * self.height as u64 * self.depth as u64
    }
}

// ============================================================================
// Queue Priority
// ============================================================================

/// Queue priority presets
pub mod queue_priority {
    /// Low priority
    pub const LOW: f32 = 0.25;
    /// Normal priority
    pub const NORMAL: f32 = 0.5;
    /// High priority
    pub const HIGH: f32 = 0.75;
    /// Realtime priority
    pub const REALTIME: f32 = 1.0;
}

// ============================================================================
// Queue Selection
// ============================================================================

/// Queue selection criteria
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct QueueSelection {
    /// Required flags
    pub required_flags: QueueFlags,
    /// Preferred flags
    pub preferred_flags: QueueFlags,
    /// Avoid flags
    pub avoid_flags: QueueFlags,
    /// Prefer dedicated
    pub prefer_dedicated: bool,
}

impl QueueSelection {
    /// Graphics queue
    pub const GRAPHICS: Self = Self {
        required_flags: QueueFlags::GRAPHICS,
        preferred_flags: QueueFlags::COMPUTE,
        avoid_flags: QueueFlags::NONE,
        prefer_dedicated: false,
    };

    /// Compute queue (prefer dedicated)
    pub const COMPUTE: Self = Self {
        required_flags: QueueFlags::COMPUTE,
        preferred_flags: QueueFlags::NONE,
        avoid_flags: QueueFlags::GRAPHICS,
        prefer_dedicated: true,
    };

    /// Transfer queue (prefer dedicated)
    pub const TRANSFER: Self = Self {
        required_flags: QueueFlags::TRANSFER,
        preferred_flags: QueueFlags::NONE,
        avoid_flags: QueueFlags::GRAPHICS.union(QueueFlags::COMPUTE),
        prefer_dedicated: true,
    };

    /// Video decode queue
    pub const VIDEO_DECODE: Self = Self {
        required_flags: QueueFlags::VIDEO_DECODE,
        preferred_flags: QueueFlags::NONE,
        avoid_flags: QueueFlags::GRAPHICS,
        prefer_dedicated: true,
    };

    /// Video encode queue
    pub const VIDEO_ENCODE: Self = Self {
        required_flags: QueueFlags::VIDEO_ENCODE,
        preferred_flags: QueueFlags::NONE,
        avoid_flags: QueueFlags::GRAPHICS,
        prefer_dedicated: true,
    };

    /// Creates new selection
    #[inline]
    pub const fn new(required: QueueFlags) -> Self {
        Self {
            required_flags: required,
            preferred_flags: QueueFlags::NONE,
            avoid_flags: QueueFlags::NONE,
            prefer_dedicated: false,
        }
    }

    /// With preferred flags
    #[inline]
    pub const fn with_preferred(mut self, flags: QueueFlags) -> Self {
        self.preferred_flags = flags;
        self
    }

    /// With avoid flags
    #[inline]
    pub const fn with_avoid(mut self, flags: QueueFlags) -> Self {
        self.avoid_flags = flags;
        self
    }

    /// Prefer dedicated
    #[inline]
    pub const fn dedicated(mut self) -> Self {
        self.prefer_dedicated = true;
        self
    }

    /// Scores a queue family
    #[inline]
    pub const fn score(&self, family: &QueueFamilyProperties) -> i32 {
        // Must have required flags
        if !family.queue_flags.contains(self.required_flags) {
            return -1;
        }

        let mut score: i32 = 100;

        // Prefer having preferred flags
        if family.queue_flags.contains(self.preferred_flags) {
            score += 10;
        }

        // Avoid having avoid flags
        if !self.avoid_flags.is_empty() && family.queue_flags.intersection(self.avoid_flags).0 != 0
        {
            score -= 50;
        }

        // Bonus for dedicated queues
        if self.prefer_dedicated {
            // Count capabilities - fewer is better for dedicated
            let capabilities = family.queue_flags.0.count_ones();
            score -= capabilities as i32 * 5;
        }

        score
    }
}

impl Default for QueueSelection {
    fn default() -> Self {
        Self::GRAPHICS
    }
}

// ============================================================================
// Queue Wait Idle
// ============================================================================

/// Queue wait info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct QueueWaitInfo {
    /// Queue handle
    pub queue: QueueHandle,
    /// Timeout in nanoseconds (0 = no wait, u64::MAX = infinite)
    pub timeout: u64,
}

impl QueueWaitInfo {
    /// Wait forever
    pub const INFINITE: u64 = u64::MAX;
    /// No wait
    pub const NO_WAIT: u64 = 0;

    /// Creates new info
    #[inline]
    pub const fn new(queue: QueueHandle) -> Self {
        Self {
            queue,
            timeout: Self::INFINITE,
        }
    }

    /// With timeout
    #[inline]
    pub const fn with_timeout(mut self, timeout_ns: u64) -> Self {
        self.timeout = timeout_ns;
        self
    }

    /// With timeout in milliseconds
    #[inline]
    pub const fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout = timeout_ms * 1_000_000;
        self
    }
}

// ============================================================================
// Bind Sparse Info
// ============================================================================

/// Bind sparse info
#[derive(Clone, Debug)]
#[repr(C)]
pub struct BindSparseInfo<'a> {
    /// Wait semaphores
    pub wait_semaphores: &'a [u64],
    /// Buffer binds
    pub buffer_binds: &'a [SparseBufferMemoryBindInfo<'a>],
    /// Image opaque binds
    pub image_opaque_binds: &'a [SparseImageOpaqueMemoryBindInfo<'a>],
    /// Image binds
    pub image_binds: &'a [SparseImageMemoryBindInfo<'a>],
    /// Signal semaphores
    pub signal_semaphores: &'a [u64],
}

impl Default for BindSparseInfo<'_> {
    fn default() -> Self {
        Self {
            wait_semaphores: &[],
            buffer_binds: &[],
            image_opaque_binds: &[],
            image_binds: &[],
            signal_semaphores: &[],
        }
    }
}

/// Sparse buffer memory bind info
#[derive(Clone, Debug)]
#[repr(C)]
pub struct SparseBufferMemoryBindInfo<'a> {
    /// Buffer handle
    pub buffer: u64,
    /// Binds
    pub binds: &'a [SparseMemoryBind],
}

/// Sparse image opaque memory bind info
#[derive(Clone, Debug)]
#[repr(C)]
pub struct SparseImageOpaqueMemoryBindInfo<'a> {
    /// Image handle
    pub image: u64,
    /// Binds
    pub binds: &'a [SparseMemoryBind],
}

/// Sparse image memory bind info
#[derive(Clone, Debug)]
#[repr(C)]
pub struct SparseImageMemoryBindInfo<'a> {
    /// Image handle
    pub image: u64,
    /// Binds
    pub binds: &'a [SparseImageMemoryBind],
}

/// Sparse memory bind
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SparseMemoryBind {
    /// Resource offset
    pub resource_offset: u64,
    /// Size
    pub size: u64,
    /// Memory handle
    pub memory: u64,
    /// Memory offset
    pub memory_offset: u64,
    /// Flags
    pub flags: SparseMemoryBindFlags,
}

/// Sparse image memory bind
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SparseImageMemoryBind {
    /// Subresource
    pub subresource: ImageSubresource,
    /// Offset
    pub offset: Offset3D,
    /// Extent
    pub extent: Extent3D,
    /// Memory handle
    pub memory: u64,
    /// Memory offset
    pub memory_offset: u64,
    /// Flags
    pub flags: SparseMemoryBindFlags,
}

/// Image subresource
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ImageSubresource {
    /// Aspect mask
    pub aspect_mask: u32,
    /// Mip level
    pub mip_level: u32,
    /// Array layer
    pub array_layer: u32,
}

/// 3D offset
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
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
}

/// Sparse memory bind flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct SparseMemoryBindFlags(pub u32);

impl SparseMemoryBindFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Metadata bind
    pub const METADATA: Self = Self(1 << 0);

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
