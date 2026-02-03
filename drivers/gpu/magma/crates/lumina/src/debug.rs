//! Debug utilities and markers
//!
//! This module provides types for GPU debugging, profiling, and markers.

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

/// Debug label color
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DebugColor {
    /// Red component (0.0-1.0)
    pub r: f32,
    /// Green component (0.0-1.0)
    pub g: f32,
    /// Blue component (0.0-1.0)
    pub b: f32,
    /// Alpha component (0.0-1.0)
    pub a: f32,
}

impl DebugColor {
    /// White color
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0, 1.0);
    /// Red color
    pub const RED: Self = Self::new(1.0, 0.0, 0.0, 1.0);
    /// Green color
    pub const GREEN: Self = Self::new(0.0, 1.0, 0.0, 1.0);
    /// Blue color
    pub const BLUE: Self = Self::new(0.0, 0.0, 1.0, 1.0);
    /// Yellow color
    pub const YELLOW: Self = Self::new(1.0, 1.0, 0.0, 1.0);
    /// Cyan color
    pub const CYAN: Self = Self::new(0.0, 1.0, 1.0, 1.0);
    /// Magenta color
    pub const MAGENTA: Self = Self::new(1.0, 0.0, 1.0, 1.0);
    /// Orange color
    pub const ORANGE: Self = Self::new(1.0, 0.5, 0.0, 1.0);
    /// Purple color
    pub const PURPLE: Self = Self::new(0.5, 0.0, 1.0, 1.0);

    /// Creates a new debug color
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Creates from RGB values (0.0-1.0)
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::new(r, g, b, 1.0)
    }

    /// Creates from RGB bytes (0-255)
    pub fn from_rgb8(r: u8, g: u8, b: u8) -> Self {
        Self::rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
    }

    /// Converts to array
    pub const fn to_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

/// Debug label for regions
#[derive(Clone, Debug)]
pub struct DebugLabel {
    /// Label name
    pub name: String,
    /// Label color
    pub color: DebugColor,
}

impl DebugLabel {
    /// Creates a new debug label
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            color: DebugColor::WHITE,
        }
    }

    /// Sets the color
    pub fn with_color(mut self, color: DebugColor) -> Self {
        self.color = color;
        self
    }

    /// Creates a render pass label (orange)
    pub fn render_pass(name: &str) -> Self {
        Self::new(name).with_color(DebugColor::ORANGE)
    }

    /// Creates a compute label (purple)
    pub fn compute(name: &str) -> Self {
        Self::new(name).with_color(DebugColor::PURPLE)
    }

    /// Creates a transfer label (cyan)
    pub fn transfer(name: &str) -> Self {
        Self::new(name).with_color(DebugColor::CYAN)
    }
}

/// Debug object name info
#[derive(Clone, Debug)]
pub struct DebugObjectInfo {
    /// Object type
    pub object_type: DebugObjectType,
    /// Object handle value
    pub object_handle: u64,
    /// Object name
    pub object_name: String,
}

impl DebugObjectInfo {
    /// Creates new debug object info
    pub fn new(object_type: DebugObjectType, handle: u64, name: &str) -> Self {
        Self {
            object_type,
            object_handle: handle,
            object_name: String::from(name),
        }
    }

    /// Creates buffer debug info
    pub fn buffer(handle: u64, name: &str) -> Self {
        Self::new(DebugObjectType::Buffer, handle, name)
    }

    /// Creates texture debug info
    pub fn texture(handle: u64, name: &str) -> Self {
        Self::new(DebugObjectType::Texture, handle, name)
    }

    /// Creates shader debug info
    pub fn shader(handle: u64, name: &str) -> Self {
        Self::new(DebugObjectType::Shader, handle, name)
    }
}

/// Debug object types
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum DebugObjectType {
    /// Unknown object
    Unknown             = 0,
    /// Instance object
    Instance            = 1,
    /// Physical device
    PhysicalDevice      = 2,
    /// Logical device
    Device              = 3,
    /// Queue
    Queue               = 4,
    /// Semaphore
    Semaphore           = 5,
    /// Command buffer
    CommandBuffer       = 6,
    /// Fence
    Fence               = 7,
    /// Device memory
    DeviceMemory        = 8,
    /// Buffer
    Buffer              = 9,
    /// Image/Texture
    Texture             = 10,
    /// Event
    Event               = 11,
    /// Query pool
    QueryPool           = 12,
    /// Buffer view
    BufferView          = 13,
    /// Image view
    TextureView         = 14,
    /// Shader module
    Shader              = 15,
    /// Pipeline cache
    PipelineCache       = 16,
    /// Pipeline layout
    PipelineLayout      = 17,
    /// Render pass
    RenderPass          = 18,
    /// Graphics pipeline
    GraphicsPipeline    = 19,
    /// Compute pipeline
    ComputePipeline     = 20,
    /// Descriptor set layout
    DescriptorSetLayout = 21,
    /// Sampler
    Sampler             = 22,
    /// Descriptor pool
    DescriptorPool      = 23,
    /// Descriptor set
    DescriptorSet       = 24,
    /// Framebuffer
    Framebuffer         = 25,
    /// Command pool
    CommandPool         = 26,
    /// Surface
    Surface             = 27,
    /// Swapchain
    Swapchain           = 28,
    /// Acceleration structure
    AccelerationStructure = 29,
}

/// Debug message severity
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum DebugSeverity {
    /// Verbose debug info
    Verbose = 0,
    /// Informational message
    Info    = 1,
    /// Warning message
    Warning = 2,
    /// Error message
    Error   = 3,
}

/// Debug message type flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct DebugMessageType(pub u32);

impl DebugMessageType {
    /// General message
    pub const GENERAL: Self = Self(1 << 0);
    /// Validation message
    pub const VALIDATION: Self = Self(1 << 1);
    /// Performance message
    pub const PERFORMANCE: Self = Self(1 << 2);
    /// Device address binding
    pub const DEVICE_ADDRESS: Self = Self(1 << 3);

    /// All message types
    pub const ALL: Self =
        Self(Self::GENERAL.0 | Self::VALIDATION.0 | Self::PERFORMANCE.0 | Self::DEVICE_ADDRESS.0);
}

impl core::ops::BitOr for DebugMessageType {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Debug message
#[derive(Clone, Debug)]
pub struct DebugMessage {
    /// Message severity
    pub severity: DebugSeverity,
    /// Message type
    pub message_type: DebugMessageType,
    /// Message ID
    pub message_id: i32,
    /// Message ID name
    pub message_id_name: String,
    /// Message text
    pub message: String,
    /// Associated objects
    pub objects: Vec<DebugObjectInfo>,
}

impl DebugMessage {
    /// Creates a new debug message
    pub fn new(severity: DebugSeverity, message: &str) -> Self {
        Self {
            severity,
            message_type: DebugMessageType::GENERAL,
            message_id: 0,
            message_id_name: String::new(),
            message: String::from(message),
            objects: Vec::new(),
        }
    }

    /// Creates a validation error
    pub fn validation_error(message: &str) -> Self {
        Self {
            severity: DebugSeverity::Error,
            message_type: DebugMessageType::VALIDATION,
            message_id: 0,
            message_id_name: String::new(),
            message: String::from(message),
            objects: Vec::new(),
        }
    }

    /// Creates a performance warning
    pub fn performance_warning(message: &str) -> Self {
        Self {
            severity: DebugSeverity::Warning,
            message_type: DebugMessageType::PERFORMANCE,
            message_id: 0,
            message_id_name: String::new(),
            message: String::from(message),
            objects: Vec::new(),
        }
    }
}

/// Debug callback info
#[derive(Clone, Debug)]
pub struct DebugCallbackInfo {
    /// Severity filter
    pub severity_filter: DebugSeverityFilter,
    /// Message type filter
    pub message_type_filter: DebugMessageType,
}

/// Debug severity filter
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct DebugSeverityFilter(pub u32);

impl DebugSeverityFilter {
    /// Verbose and above
    pub const VERBOSE: Self = Self(1 << 0);
    /// Info and above
    pub const INFO: Self = Self(1 << 1);
    /// Warning and above
    pub const WARNING: Self = Self(1 << 2);
    /// Error only
    pub const ERROR: Self = Self(1 << 3);

    /// All severities
    pub const ALL: Self = Self(Self::VERBOSE.0 | Self::INFO.0 | Self::WARNING.0 | Self::ERROR.0);

    /// Warnings and errors
    pub const WARNING_AND_ERROR: Self = Self(Self::WARNING.0 | Self::ERROR.0);
}

impl core::ops::BitOr for DebugSeverityFilter {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// GPU marker scope
pub struct MarkerScope {
    /// Scope name
    pub name: String,
    /// Scope color
    pub color: DebugColor,
    /// Start timestamp
    pub start_timestamp: u64,
}

impl MarkerScope {
    /// Creates a new marker scope
    pub fn new(name: &str, color: DebugColor) -> Self {
        Self {
            name: String::from(name),
            color,
            start_timestamp: 0,
        }
    }
}

/// GPU timing info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuTimingInfo {
    /// Start timestamp in nanoseconds
    pub start_ns: u64,
    /// End timestamp in nanoseconds
    pub end_ns: u64,
}

impl GpuTimingInfo {
    /// Returns the duration in nanoseconds
    pub const fn duration_ns(&self) -> u64 {
        self.end_ns.saturating_sub(self.start_ns)
    }

    /// Returns the duration in microseconds
    pub fn duration_us(&self) -> f64 {
        self.duration_ns() as f64 / 1000.0
    }

    /// Returns the duration in milliseconds
    pub fn duration_ms(&self) -> f64 {
        self.duration_ns() as f64 / 1_000_000.0
    }
}

/// Frame timing statistics
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct FrameTimingStats {
    /// GPU time for the frame
    pub gpu_time_ns: u64,
    /// CPU time for the frame
    pub cpu_time_ns: u64,
    /// Time spent waiting on GPU
    pub wait_time_ns: u64,
    /// Time from present to display
    pub present_time_ns: u64,
}

impl FrameTimingStats {
    /// Total frame time
    pub const fn total_ns(&self) -> u64 {
        self.gpu_time_ns
            .max(self.cpu_time_ns)
            .saturating_add(self.wait_time_ns)
    }

    /// Is GPU bound?
    pub const fn is_gpu_bound(&self) -> bool {
        self.gpu_time_ns > self.cpu_time_ns
    }

    /// Is CPU bound?
    pub const fn is_cpu_bound(&self) -> bool {
        self.cpu_time_ns > self.gpu_time_ns
    }
}

/// Validation features to enable
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ValidationFeatures(pub u32);

impl ValidationFeatures {
    /// No validation
    pub const NONE: Self = Self(0);
    /// GPU-assisted validation
    pub const GPU_ASSISTED: Self = Self(1 << 0);
    /// GPU-assisted reserve binding slot
    pub const GPU_ASSISTED_RESERVE_BINDING_SLOT: Self = Self(1 << 1);
    /// Best practices validation
    pub const BEST_PRACTICES: Self = Self(1 << 2);
    /// Debug printf
    pub const DEBUG_PRINTF: Self = Self(1 << 3);
    /// Synchronization validation
    pub const SYNCHRONIZATION: Self = Self(1 << 4);

    /// All validation
    pub const ALL: Self = Self(
        Self::GPU_ASSISTED.0
            | Self::GPU_ASSISTED_RESERVE_BINDING_SLOT.0
            | Self::BEST_PRACTICES.0
            | Self::DEBUG_PRINTF.0
            | Self::SYNCHRONIZATION.0,
    );
}

impl core::ops::BitOr for ValidationFeatures {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Shader debug info
#[derive(Clone, Debug)]
pub struct ShaderDebugInfo {
    /// Shader name
    pub name: String,
    /// Source file path
    pub source_file: String,
    /// Entry point
    pub entry_point: String,
    /// Compilation flags
    pub compilation_flags: String,
}

impl ShaderDebugInfo {
    /// Creates new shader debug info
    pub fn new(name: &str, entry_point: &str) -> Self {
        Self {
            name: String::from(name),
            source_file: String::new(),
            entry_point: String::from(entry_point),
            compilation_flags: String::new(),
        }
    }

    /// Sets source file
    pub fn with_source_file(mut self, path: &str) -> Self {
        self.source_file = String::from(path);
        self
    }
}

/// Debug marker helper for RAII
pub struct ScopedDebugMarker {
    /// Marker label
    _label: DebugLabel,
}

impl ScopedDebugMarker {
    /// Creates a new scoped marker
    pub fn new(label: DebugLabel) -> Self {
        // In a real implementation, this would call the begin label command
        Self { _label: label }
    }
}

// In a real implementation, Drop would call the end label command

/// Pipeline statistics for debugging
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct PipelineDebugStats {
    /// Number of vertex shader invocations
    pub vertex_shader_invocations: u64,
    /// Number of fragment shader invocations
    pub fragment_shader_invocations: u64,
    /// Number of compute shader invocations
    pub compute_shader_invocations: u64,
    /// Number of primitives generated
    pub primitives_generated: u64,
    /// Number of clipping invocations
    pub clipping_invocations: u64,
    /// Number of clipping primitives
    pub clipping_primitives: u64,
    /// Number of tessellation patches
    pub tessellation_patches: u64,
    /// Number of geometry shader invocations
    pub geometry_shader_invocations: u64,
    /// Number of geometry shader primitives
    pub geometry_shader_primitives: u64,
}

/// Memory debug info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MemoryDebugInfo {
    /// Total device memory allocated
    pub total_device_memory: u64,
    /// Total host memory allocated
    pub total_host_memory: u64,
    /// Number of allocations
    pub allocation_count: u32,
    /// Peak device memory
    pub peak_device_memory: u64,
    /// Peak host memory
    pub peak_host_memory: u64,
}

impl MemoryDebugInfo {
    /// Returns device memory in MB
    pub fn device_memory_mb(&self) -> f64 {
        self.total_device_memory as f64 / (1024.0 * 1024.0)
    }

    /// Returns host memory in MB
    pub fn host_memory_mb(&self) -> f64 {
        self.total_host_memory as f64 / (1024.0 * 1024.0)
    }
}

/// Debug report flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct DebugReportFlags(pub u32);

impl DebugReportFlags {
    /// Information
    pub const INFO: Self = Self(1 << 0);
    /// Warning
    pub const WARNING: Self = Self(1 << 1);
    /// Performance warning
    pub const PERFORMANCE: Self = Self(1 << 2);
    /// Error
    pub const ERROR: Self = Self(1 << 3);
    /// Debug
    pub const DEBUG: Self = Self(1 << 4);

    /// All flags
    pub const ALL: Self =
        Self(Self::INFO.0 | Self::WARNING.0 | Self::PERFORMANCE.0 | Self::ERROR.0 | Self::DEBUG.0);
}

impl core::ops::BitOr for DebugReportFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}
