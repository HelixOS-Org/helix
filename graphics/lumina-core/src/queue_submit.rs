//! Queue Submission Types for Lumina
//!
//! This module provides GPU queue management, command submission,
//! and cross-queue synchronization infrastructure.

use alloc::vec::Vec;

// ============================================================================
// Submission Handle
// ============================================================================

/// Submission handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SubmissionHandle(pub u64);

impl SubmissionHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates a new handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Checks if the handle is valid
    #[inline]
    pub const fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

// ============================================================================
// Queue Submission Info
// ============================================================================

/// Queue submission configuration
#[derive(Clone, Debug)]
#[repr(C)]
pub struct QueueSubmission {
    /// Queue family index
    pub queue_family: u32,
    /// Queue index within family
    pub queue_index: u32,
    /// Submission priority
    pub priority: SubmitPriority,
    /// Wait operations
    pub waits: Vec<WaitOperation>,
    /// Command buffer handles
    pub command_buffers: Vec<u64>,
    /// Signal operations
    pub signals: Vec<SignalOperation>,
    /// Fence to signal on completion
    pub fence: u64,
    /// Submission flags
    pub flags: SubmitFlags,
}

impl QueueSubmission {
    /// Creates a new submission
    #[inline]
    pub fn new(queue_family: u32, queue_index: u32) -> Self {
        Self {
            queue_family,
            queue_index,
            priority: SubmitPriority::Normal,
            waits: Vec::new(),
            command_buffers: Vec::new(),
            signals: Vec::new(),
            fence: 0,
            flags: SubmitFlags::NONE,
        }
    }

    /// Graphics queue submission
    #[inline]
    pub fn graphics() -> Self {
        Self::new(0, 0)
    }

    /// Compute queue submission
    #[inline]
    pub fn compute() -> Self {
        Self::new(1, 0)
    }

    /// Transfer queue submission
    #[inline]
    pub fn transfer() -> Self {
        Self::new(2, 0)
    }

    /// Adds a wait operation
    #[inline]
    pub fn wait(mut self, semaphore: u64, value: u64, stage: SubmitStage) -> Self {
        self.waits.push(WaitOperation::new(semaphore, value, stage));
        self
    }

    /// Adds a wait for binary semaphore
    #[inline]
    pub fn wait_binary(mut self, semaphore: u64, stage: SubmitStage) -> Self {
        self.waits.push(WaitOperation::binary(semaphore, stage));
        self
    }

    /// Adds a signal operation
    #[inline]
    pub fn signal(mut self, semaphore: u64, value: u64, stage: SubmitStage) -> Self {
        self.signals
            .push(SignalOperation::new(semaphore, value, stage));
        self
    }

    /// Adds a signal for binary semaphore
    #[inline]
    pub fn signal_binary(mut self, semaphore: u64, stage: SubmitStage) -> Self {
        self.signals.push(SignalOperation::binary(semaphore, stage));
        self
    }

    /// Adds a command buffer
    #[inline]
    pub fn command_buffer(mut self, cmd: u64) -> Self {
        self.command_buffers.push(cmd);
        self
    }

    /// Adds multiple command buffers
    #[inline]
    pub fn command_buffers(mut self, cmds: impl IntoIterator<Item = u64>) -> Self {
        self.command_buffers.extend(cmds);
        self
    }

    /// Sets the fence to signal
    #[inline]
    pub fn with_fence(mut self, fence: u64) -> Self {
        self.fence = fence;
        self
    }

    /// Sets submission priority
    #[inline]
    pub fn with_priority(mut self, priority: SubmitPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Sets submission flags
    #[inline]
    pub fn with_flags(mut self, flags: SubmitFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Simple single command buffer submission
    #[inline]
    pub fn single(cmd: u64) -> Self {
        Self::graphics().command_buffer(cmd)
    }

    /// Submission with wait and signal semaphores
    #[inline]
    pub fn with_sync(cmd: u64, wait: u64, signal: u64) -> Self {
        Self::graphics()
            .wait_binary(wait, SubmitStage::AllCommands)
            .command_buffer(cmd)
            .signal_binary(signal, SubmitStage::AllCommands)
    }
}

/// Submission priority
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SubmitPriority {
    /// Low priority
    Low      = 0,
    /// Normal priority
    #[default]
    Normal   = 1,
    /// High priority
    High     = 2,
    /// Real-time priority
    Realtime = 3,
}

impl SubmitPriority {
    /// Priority value (0.0 - 1.0)
    #[inline]
    pub const fn value(&self) -> f32 {
        match self {
            Self::Low => 0.0,
            Self::Normal => 0.5,
            Self::High => 0.75,
            Self::Realtime => 1.0,
        }
    }
}

/// Submission flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct SubmitFlags(pub u32);

impl SubmitFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Protected submission
    pub const PROTECTED: Self = Self(1 << 0);

    /// Checks if flag is set
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Combines flags
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

// ============================================================================
// Wait/Signal Operations
// ============================================================================

/// Wait operation for semaphore
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct WaitOperation {
    /// Semaphore handle
    pub semaphore: u64,
    /// Timeline value (0 for binary)
    pub value: u64,
    /// Pipeline stage to wait at
    pub stage: SubmitStage,
    /// Device index (multi-GPU)
    pub device_index: u32,
}

impl WaitOperation {
    /// Creates a new wait operation
    #[inline]
    pub const fn new(semaphore: u64, value: u64, stage: SubmitStage) -> Self {
        Self {
            semaphore,
            value,
            stage,
            device_index: 0,
        }
    }

    /// Binary semaphore wait
    #[inline]
    pub const fn binary(semaphore: u64, stage: SubmitStage) -> Self {
        Self::new(semaphore, 0, stage)
    }

    /// Timeline semaphore wait
    #[inline]
    pub const fn timeline(semaphore: u64, value: u64, stage: SubmitStage) -> Self {
        Self::new(semaphore, value, stage)
    }

    /// With device index
    #[inline]
    pub const fn with_device(mut self, device_index: u32) -> Self {
        self.device_index = device_index;
        self
    }
}

/// Signal operation for semaphore
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SignalOperation {
    /// Semaphore handle
    pub semaphore: u64,
    /// Timeline value (0 for binary)
    pub value: u64,
    /// Pipeline stage to signal at
    pub stage: SubmitStage,
    /// Device index (multi-GPU)
    pub device_index: u32,
}

impl SignalOperation {
    /// Creates a new signal operation
    #[inline]
    pub const fn new(semaphore: u64, value: u64, stage: SubmitStage) -> Self {
        Self {
            semaphore,
            value,
            stage,
            device_index: 0,
        }
    }

    /// Binary semaphore signal
    #[inline]
    pub const fn binary(semaphore: u64, stage: SubmitStage) -> Self {
        Self::new(semaphore, 0, stage)
    }

    /// Timeline semaphore signal
    #[inline]
    pub const fn timeline(semaphore: u64, value: u64, stage: SubmitStage) -> Self {
        Self::new(semaphore, value, stage)
    }

    /// With device index
    #[inline]
    pub const fn with_device(mut self, device_index: u32) -> Self {
        self.device_index = device_index;
        self
    }
}

/// Pipeline stage for synchronization
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u64)]
pub enum SubmitStage {
    /// No stage
    None                 = 0,
    /// Top of pipe
    TopOfPipe            = 1 << 0,
    /// Draw indirect
    DrawIndirect         = 1 << 1,
    /// Vertex input
    VertexInput          = 1 << 2,
    /// Vertex shader
    VertexShader         = 1 << 3,
    /// Tessellation control shader
    TessellationControl  = 1 << 4,
    /// Tessellation evaluation shader
    TessellationEvaluation = 1 << 5,
    /// Geometry shader
    GeometryShader       = 1 << 6,
    /// Fragment shader
    FragmentShader       = 1 << 7,
    /// Early fragment tests
    EarlyFragmentTests   = 1 << 8,
    /// Late fragment tests
    LateFragmentTests    = 1 << 9,
    /// Color attachment output
    ColorAttachmentOutput = 1 << 10,
    /// Compute shader
    ComputeShader        = 1 << 11,
    /// Transfer operations
    Transfer             = 1 << 12,
    /// Bottom of pipe
    BottomOfPipe         = 1 << 13,
    /// Host operations
    Host                 = 1 << 14,
    /// All graphics stages
    AllGraphics          = 1 << 15,
    /// All commands
    #[default]
    AllCommands          = 1 << 16,
    /// Copy operations
    Copy                 = 1 << 17,
    /// Resolve operations
    Resolve              = 1 << 18,
    /// Blit operations
    Blit                 = 1 << 19,
    /// Clear operations
    Clear                = 1 << 20,
    /// Index input
    IndexInput           = 1 << 21,
    /// Vertex attribute input
    VertexAttributeInput = 1 << 22,
    /// Pre-rasterization shaders
    PreRasterization     = 1 << 23,
    /// Video decode
    VideoDecode          = 1 << 24,
    /// Video encode
    VideoEncode          = 1 << 25,
    /// Acceleration structure build
    AccelerationStructureBuild = 1 << 26,
    /// Ray tracing shader
    RayTracingShader     = 1 << 27,
    /// Fragment shading rate attachment
    FragmentShadingRate  = 1 << 28,
    /// Fragment density process
    FragmentDensity      = 1 << 29,
    /// Task shader
    TaskShader           = 1 << 30,
    /// Mesh shader
    MeshShader           = 1 << 31,
}

impl SubmitStage {
    /// All stages
    pub const ALL: Self = Self::AllCommands;

    /// Checks if this is a shader stage
    #[inline]
    pub const fn is_shader(&self) -> bool {
        matches!(
            self,
            Self::VertexShader
                | Self::TessellationControl
                | Self::TessellationEvaluation
                | Self::GeometryShader
                | Self::FragmentShader
                | Self::ComputeShader
                | Self::RayTracingShader
                | Self::TaskShader
                | Self::MeshShader
        )
    }

    /// Checks if this is a graphics stage
    #[inline]
    pub const fn is_graphics(&self) -> bool {
        matches!(
            self,
            Self::DrawIndirect
                | Self::VertexInput
                | Self::VertexShader
                | Self::TessellationControl
                | Self::TessellationEvaluation
                | Self::GeometryShader
                | Self::FragmentShader
                | Self::EarlyFragmentTests
                | Self::LateFragmentTests
                | Self::ColorAttachmentOutput
                | Self::AllGraphics
        )
    }
}

// ============================================================================
// Batch Submission
// ============================================================================

/// Batch submission for multiple queues
#[derive(Clone, Debug, Default)]
#[repr(C)]
pub struct BatchSubmission {
    /// Individual submissions
    pub submissions: Vec<QueueSubmission>,
}

impl BatchSubmission {
    /// Creates a new batch
    #[inline]
    pub fn new() -> Self {
        Self {
            submissions: Vec::new(),
        }
    }

    /// Adds a submission
    #[inline]
    pub fn add(mut self, submission: QueueSubmission) -> Self {
        self.submissions.push(submission);
        self
    }

    /// Single submission batch
    #[inline]
    pub fn single(submission: QueueSubmission) -> Self {
        Self {
            submissions: vec![submission],
        }
    }

    /// Total command buffer count
    #[inline]
    pub fn command_buffer_count(&self) -> usize {
        self.submissions
            .iter()
            .map(|s| s.command_buffers.len())
            .sum()
    }
}

// ============================================================================
// Submission Result
// ============================================================================

/// Submission result
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum SubmitResult {
    /// Success
    Success           = 0,
    /// Not ready (for async)
    NotReady          = 1,
    /// Timeout
    Timeout           = 2,
    /// Device lost
    DeviceLost        = -1,
    /// Out of host memory
    OutOfHostMemory   = -2,
    /// Out of device memory
    OutOfDeviceMemory = -3,
    /// Invalid operation
    InvalidOperation  = -4,
}

impl SubmitResult {
    /// Is success
    #[inline]
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }

    /// Is error
    #[inline]
    pub const fn is_error(&self) -> bool {
        (*self as i32) < 0
    }
}

// ============================================================================
// Queue Family Selection
// ============================================================================

/// Queue family type hint for selection
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum QueueFamilyType {
    /// Graphics-capable queue
    #[default]
    Graphics      = 0,
    /// Compute-only queue (async compute)
    Compute       = 1,
    /// Transfer-only queue (DMA)
    Transfer      = 2,
    /// Video decode queue
    VideoDecode   = 3,
    /// Video encode queue
    VideoEncode   = 4,
    /// Protected content queue
    Protected     = 5,
    /// Sparse binding queue
    SparseBinding = 6,
    /// Optical flow queue
    OpticalFlow   = 7,
}

impl QueueFamilyType {
    /// Preferred queue family name
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Graphics => "Graphics",
            Self::Compute => "Compute",
            Self::Transfer => "Transfer",
            Self::VideoDecode => "Video Decode",
            Self::VideoEncode => "Video Encode",
            Self::Protected => "Protected",
            Self::SparseBinding => "Sparse Binding",
            Self::OpticalFlow => "Optical Flow",
        }
    }

    /// Required capability flags
    #[inline]
    pub const fn required_capabilities(&self) -> u32 {
        match self {
            Self::Graphics => 0b111, // Graphics + Compute + Transfer
            Self::Compute => 0b110,  // Compute + Transfer
            Self::Transfer => 0b100, // Transfer
            Self::VideoDecode => 0b100000,
            Self::VideoEncode => 0b1000000,
            Self::Protected => 0b10000,
            Self::SparseBinding => 0b1000,
            Self::OpticalFlow => 0b10000000,
        }
    }
}

/// Queue family selection criteria
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct QueueFamilySelector {
    /// Required type
    pub queue_type: QueueFamilyType,
    /// Prefer dedicated queue
    pub prefer_dedicated: bool,
    /// Minimum queue count
    pub min_queue_count: u32,
    /// Required surface support (WSI)
    pub require_present: bool,
}

impl QueueFamilySelector {
    /// Creates a new selector
    #[inline]
    pub const fn new(queue_type: QueueFamilyType) -> Self {
        Self {
            queue_type,
            prefer_dedicated: false,
            min_queue_count: 1,
            require_present: false,
        }
    }

    /// Graphics queue with present
    #[inline]
    pub const fn graphics_with_present() -> Self {
        Self {
            queue_type: QueueFamilyType::Graphics,
            prefer_dedicated: false,
            min_queue_count: 1,
            require_present: true,
        }
    }

    /// Dedicated compute queue
    #[inline]
    pub const fn async_compute() -> Self {
        Self {
            queue_type: QueueFamilyType::Compute,
            prefer_dedicated: true,
            min_queue_count: 1,
            require_present: false,
        }
    }

    /// Dedicated transfer queue
    #[inline]
    pub const fn dma_transfer() -> Self {
        Self {
            queue_type: QueueFamilyType::Transfer,
            prefer_dedicated: true,
            min_queue_count: 1,
            require_present: false,
        }
    }

    /// With minimum queue count
    #[inline]
    pub const fn with_min_count(mut self, count: u32) -> Self {
        self.min_queue_count = count;
        self
    }

    /// Prefer dedicated queue
    #[inline]
    pub const fn prefer_dedicated(mut self) -> Self {
        self.prefer_dedicated = true;
        self
    }
}

impl Default for QueueFamilySelector {
    fn default() -> Self {
        Self::new(QueueFamilyType::Graphics)
    }
}

// ============================================================================
// Command Buffer Submit Info
// ============================================================================

/// Command buffer info for submission
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct CommandBufferInfo {
    /// Command buffer handle
    pub handle: u64,
    /// Device mask for multi-GPU
    pub device_mask: u32,
    /// Flags
    pub flags: CommandBufferSubmitFlags,
}

impl CommandBufferInfo {
    /// Creates new command buffer info
    #[inline]
    pub const fn new(handle: u64) -> Self {
        Self {
            handle,
            device_mask: 1,
            flags: CommandBufferSubmitFlags::NONE,
        }
    }

    /// With device mask
    #[inline]
    pub const fn with_device_mask(mut self, mask: u32) -> Self {
        self.device_mask = mask;
        self
    }

    /// All devices
    #[inline]
    pub const fn all_devices(mut self) -> Self {
        self.device_mask = u32::MAX;
        self
    }
}

/// Command buffer submit flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct CommandBufferSubmitFlags(pub u32);

impl CommandBufferSubmitFlags {
    /// No flags
    pub const NONE: Self = Self(0);

    /// Checks if flag is set
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

// ============================================================================
// Cross-Queue Sync
// ============================================================================

/// Cross-queue synchronization point
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct CrossQueueSync {
    /// Source queue family
    pub src_queue_family: u32,
    /// Destination queue family
    pub dst_queue_family: u32,
    /// Semaphore for sync
    pub semaphore: u64,
    /// Timeline value
    pub value: u64,
}

impl CrossQueueSync {
    /// Creates a new sync point
    #[inline]
    pub const fn new(
        src_queue_family: u32,
        dst_queue_family: u32,
        semaphore: u64,
        value: u64,
    ) -> Self {
        Self {
            src_queue_family,
            dst_queue_family,
            semaphore,
            value,
        }
    }

    /// Graphics to compute sync
    #[inline]
    pub const fn graphics_to_compute(semaphore: u64, value: u64) -> Self {
        Self::new(0, 1, semaphore, value)
    }

    /// Compute to graphics sync
    #[inline]
    pub const fn compute_to_graphics(semaphore: u64, value: u64) -> Self {
        Self::new(1, 0, semaphore, value)
    }

    /// Transfer to graphics sync
    #[inline]
    pub const fn transfer_to_graphics(semaphore: u64, value: u64) -> Self {
        Self::new(2, 0, semaphore, value)
    }

    /// Graphics to transfer sync
    #[inline]
    pub const fn graphics_to_transfer(semaphore: u64, value: u64) -> Self {
        Self::new(0, 2, semaphore, value)
    }
}

// ============================================================================
// Present Wait
// ============================================================================

/// Present wait info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PresentWaitInfo {
    /// Swapchain handle
    pub swapchain: u64,
    /// Present ID to wait for
    pub present_id: u64,
    /// Timeout in nanoseconds
    pub timeout_ns: u64,
}

impl PresentWaitInfo {
    /// Creates new present wait info
    #[inline]
    pub const fn new(swapchain: u64, present_id: u64) -> Self {
        Self {
            swapchain,
            present_id,
            timeout_ns: u64::MAX,
        }
    }

    /// With timeout in nanoseconds
    #[inline]
    pub const fn with_timeout_ns(mut self, ns: u64) -> Self {
        self.timeout_ns = ns;
        self
    }

    /// With timeout in milliseconds
    #[inline]
    pub const fn with_timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ns = ms * 1_000_000;
        self
    }

    /// Non-blocking check
    #[inline]
    pub const fn non_blocking(swapchain: u64, present_id: u64) -> Self {
        Self {
            swapchain,
            present_id,
            timeout_ns: 0,
        }
    }
}

/// Present wait result
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum PresentWaitResult {
    /// Success - present completed
    Success     = 0,
    /// Timeout - still pending
    Timeout     = 1,
    /// Not ready - non-blocking check
    NotReady    = 2,
    /// Out of date - swapchain invalid
    OutOfDate   = -1,
    /// Surface lost
    SurfaceLost = -2,
    /// Device lost
    DeviceLost  = -3,
}

impl PresentWaitResult {
    /// Is success
    #[inline]
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }

    /// Is pending
    #[inline]
    pub const fn is_pending(&self) -> bool {
        matches!(self, Self::Timeout | Self::NotReady)
    }

    /// Is error
    #[inline]
    pub const fn is_error(&self) -> bool {
        (*self as i32) < 0
    }
}
