//! Debug utilities and validation layer for LUMINA
//!
//! This module provides debugging capabilities similar to Vulkan validation layers:
//! - Object naming and labeling
//! - Debug markers for command buffers
//! - Validation callbacks
//! - GPU crash analysis

#[cfg(feature = "alloc")]
use alloc::{boxed::Box, string::String, vec::Vec};
use core::fmt;

use crate::handle::Handle;

// ============================================================================
// Debug Severity & Type
// ============================================================================

/// Severity level of debug messages
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum DebugSeverity {
    /// Verbose diagnostic information
    Verbose = 0x0001,
    /// Informational message
    Info    = 0x0010,
    /// Warning - potential issue
    Warning = 0x0100,
    /// Error - invalid usage
    Error   = 0x1000,
}

impl DebugSeverity {
    /// Returns a human-readable name
    pub const fn name(self) -> &'static str {
        match self {
            Self::Verbose => "VERBOSE",
            Self::Info => "INFO",
            Self::Warning => "WARNING",
            Self::Error => "ERROR",
        }
    }

    /// Returns ANSI color code for terminal output
    pub const fn ansi_color(self) -> &'static str {
        match self {
            Self::Verbose => "\x1b[90m", // Gray
            Self::Info => "\x1b[36m",    // Cyan
            Self::Warning => "\x1b[33m", // Yellow
            Self::Error => "\x1b[31m",   // Red
        }
    }
}

/// Type of debug message
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum DebugType {
    /// General information
    General       = 0x0001,
    /// Validation error/warning
    Validation    = 0x0002,
    /// Performance warning
    Performance   = 0x0004,
    /// Device address related
    DeviceAddress = 0x0008,
}

impl DebugType {
    /// Returns a human-readable name
    pub const fn name(self) -> &'static str {
        match self {
            Self::General => "GENERAL",
            Self::Validation => "VALIDATION",
            Self::Performance => "PERFORMANCE",
            Self::DeviceAddress => "DEVICE_ADDRESS",
        }
    }
}

// ============================================================================
// Object Types for Debugging
// ============================================================================

/// Types of objects that can be named/labeled
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ObjectType {
    Unknown             = 0,
    Instance            = 1,
    PhysicalDevice      = 2,
    Device              = 3,
    Queue               = 4,
    Semaphore           = 5,
    CommandBuffer       = 6,
    Fence               = 7,
    DeviceMemory        = 8,
    Buffer              = 9,
    Image               = 10,
    Event               = 11,
    QueryPool           = 12,
    BufferView          = 13,
    ImageView           = 14,
    ShaderModule        = 15,
    PipelineCache       = 16,
    PipelineLayout      = 17,
    RenderPass          = 18,
    Pipeline            = 19,
    DescriptorSetLayout = 20,
    Sampler             = 21,
    DescriptorPool      = 22,
    DescriptorSet       = 23,
    Framebuffer         = 24,
    CommandPool         = 25,
    Surface             = 26,
    Swapchain           = 27,
    DebugMessenger      = 28,
    AccelerationStructure = 29,
    DeferredOperation   = 30,
}

impl ObjectType {
    /// Returns a human-readable name
    pub const fn name(self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::Instance => "Instance",
            Self::PhysicalDevice => "PhysicalDevice",
            Self::Device => "Device",
            Self::Queue => "Queue",
            Self::Semaphore => "Semaphore",
            Self::CommandBuffer => "CommandBuffer",
            Self::Fence => "Fence",
            Self::DeviceMemory => "DeviceMemory",
            Self::Buffer => "Buffer",
            Self::Image => "Image",
            Self::Event => "Event",
            Self::QueryPool => "QueryPool",
            Self::BufferView => "BufferView",
            Self::ImageView => "ImageView",
            Self::ShaderModule => "ShaderModule",
            Self::PipelineCache => "PipelineCache",
            Self::PipelineLayout => "PipelineLayout",
            Self::RenderPass => "RenderPass",
            Self::Pipeline => "Pipeline",
            Self::DescriptorSetLayout => "DescriptorSetLayout",
            Self::Sampler => "Sampler",
            Self::DescriptorPool => "DescriptorPool",
            Self::DescriptorSet => "DescriptorSet",
            Self::Framebuffer => "Framebuffer",
            Self::CommandPool => "CommandPool",
            Self::Surface => "Surface",
            Self::Swapchain => "Swapchain",
            Self::DebugMessenger => "DebugMessenger",
            Self::AccelerationStructure => "AccelerationStructure",
            Self::DeferredOperation => "DeferredOperation",
        }
    }
}

// ============================================================================
// Debug Message
// ============================================================================

/// A debug message from the validation layer or driver
#[derive(Clone, Debug)]
pub struct DebugMessage<'a> {
    /// Severity of the message
    pub severity: DebugSeverity,
    /// Type of the message
    pub message_type: DebugType,
    /// Unique message ID (for filtering)
    pub message_id: i32,
    /// Message ID name (if available)
    pub message_id_name: Option<&'a str>,
    /// The actual message text
    pub message: &'a str,
    /// Queue labels active when message was generated
    pub queue_labels: &'a [DebugLabel<'a>],
    /// Command buffer labels active when message was generated
    pub cmd_labels: &'a [DebugLabel<'a>],
    /// Objects related to this message
    pub objects: &'a [DebugObjectInfo<'a>],
}

impl<'a> DebugMessage<'a> {
    /// Check if this message should be filtered based on severity
    pub fn should_log(&self, min_severity: DebugSeverity) -> bool {
        self.severity >= min_severity
    }
}

impl<'a> fmt::Display for DebugMessage<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] [{:?}] {}",
            self.severity.name(),
            self.message_type,
            self.message
        )?;

        if !self.objects.is_empty() {
            write!(f, "\n  Objects:")?;
            for obj in self.objects {
                write!(
                    f,
                    "\n    - {:?} (0x{:x})",
                    obj.object_type, obj.object_handle
                )?;
                if let Some(name) = obj.object_name {
                    write!(f, " \"{}\"", name)?;
                }
            }
        }

        Ok(())
    }
}

/// Debug label for marking regions
#[derive(Clone, Debug)]
pub struct DebugLabel<'a> {
    /// Label name
    pub name: &'a str,
    /// Label color (RGBA, 0.0-1.0)
    pub color: [f32; 4],
}

impl<'a> DebugLabel<'a> {
    /// Create a new debug label with default color
    pub const fn new(name: &'a str) -> Self {
        Self {
            name,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }

    /// Create a new debug label with custom color
    pub const fn with_color(name: &'a str, color: [f32; 4]) -> Self {
        Self { name, color }
    }
}

/// Information about an object in a debug message
#[derive(Clone, Debug)]
pub struct DebugObjectInfo<'a> {
    /// Type of the object
    pub object_type: ObjectType,
    /// Handle value of the object
    pub object_handle: u64,
    /// Name of the object (if set)
    pub object_name: Option<&'a str>,
}

// ============================================================================
// Debug Callback
// ============================================================================

/// Callback type for debug messages
pub type DebugCallback = fn(message: &DebugMessage) -> bool;

/// Configuration for debug messenger
#[derive(Clone, Debug)]
pub struct DebugMessengerCreateInfo {
    /// Severity levels to receive
    pub severity_filter: u32,
    /// Message types to receive
    pub type_filter: u32,
    /// User callback (if using function pointer)
    pub callback: Option<DebugCallback>,
}

impl Default for DebugMessengerCreateInfo {
    fn default() -> Self {
        Self {
            severity_filter: DebugSeverity::Warning as u32 | DebugSeverity::Error as u32,
            type_filter: DebugType::General as u32
                | DebugType::Validation as u32
                | DebugType::Performance as u32,
            callback: None,
        }
    }
}

impl DebugMessengerCreateInfo {
    /// Create with all messages enabled
    pub fn all() -> Self {
        Self {
            severity_filter: DebugSeverity::Verbose as u32
                | DebugSeverity::Info as u32
                | DebugSeverity::Warning as u32
                | DebugSeverity::Error as u32,
            type_filter: DebugType::General as u32
                | DebugType::Validation as u32
                | DebugType::Performance as u32
                | DebugType::DeviceAddress as u32,
            callback: None,
        }
    }

    /// Create with only errors enabled
    pub fn errors_only() -> Self {
        Self {
            severity_filter: DebugSeverity::Error as u32,
            type_filter: DebugType::General as u32 | DebugType::Validation as u32,
            callback: None,
        }
    }

    /// Set the callback function
    pub fn with_callback(mut self, callback: DebugCallback) -> Self {
        self.callback = Some(callback);
        self
    }
}

// ============================================================================
// Debug Messenger Handle
// ============================================================================

/// Handle to a debug messenger
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DebugMessenger {
    handle: u64,
}

impl DebugMessenger {
    /// Create a null handle
    pub const fn null() -> Self {
        Self { handle: 0 }
    }

    /// Check if handle is valid
    pub const fn is_valid(&self) -> bool {
        self.handle != 0
    }

    /// Get raw handle value
    pub const fn raw(&self) -> u64 {
        self.handle
    }

    /// Create from raw handle (unsafe)
    pub const unsafe fn from_raw(handle: u64) -> Self {
        Self { handle }
    }
}

// ============================================================================
// Object Naming
// ============================================================================

/// Information for naming an object
#[derive(Clone, Debug)]
pub struct DebugObjectNameInfo<'a> {
    /// Type of object
    pub object_type: ObjectType,
    /// Handle value
    pub object_handle: u64,
    /// Name to assign
    pub object_name: &'a str,
}

impl<'a> DebugObjectNameInfo<'a> {
    /// Create a new naming info
    pub const fn new(object_type: ObjectType, handle: u64, name: &'a str) -> Self {
        Self {
            object_type,
            object_handle: handle,
            object_name: name,
        }
    }
}

/// Information for tagging an object with arbitrary data
#[derive(Clone, Debug)]
pub struct DebugObjectTagInfo<'a> {
    /// Type of object
    pub object_type: ObjectType,
    /// Handle value
    pub object_handle: u64,
    /// Tag name/ID
    pub tag_name: u64,
    /// Tag data
    pub tag_data: &'a [u8],
}

// ============================================================================
// Command Buffer Debug Markers
// ============================================================================

/// Marker operations for command buffers
pub trait CommandBufferDebugMarkers {
    /// Begin a debug label region
    fn begin_debug_label(&mut self, label: &DebugLabel);

    /// End the current debug label region
    fn end_debug_label(&mut self);

    /// Insert a single debug label
    fn insert_debug_label(&mut self, label: &DebugLabel);
}

/// Marker operations for queues
pub trait QueueDebugMarkers {
    /// Begin a debug label region on this queue
    fn begin_debug_label(&self, label: &DebugLabel);

    /// End the current debug label region
    fn end_debug_label(&self);

    /// Insert a single debug label
    fn insert_debug_label(&self, label: &DebugLabel);
}

// ============================================================================
// GPU Crash Dump Analysis
// ============================================================================

/// Information about a GPU crash
#[derive(Clone, Debug)]
pub struct GpuCrashInfo {
    /// Device that crashed
    pub device_name: [u8; 256],
    /// Queue that was executing
    pub queue_family_index: u32,
    pub queue_index: u32,
    /// Last known good command buffer
    pub last_command_buffer: u64,
    /// Approximate command index
    pub approximate_command_index: u32,
    /// Crash reason (driver-specific)
    pub reason: GpuCrashReason,
    /// Additional driver data
    pub driver_data: [u8; 1024],
}

/// Reason for GPU crash
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuCrashReason {
    Unknown,
    PageFault,
    InvalidInstruction,
    Timeout,
    ContextReset,
    OutOfMemory,
    InfiniteLoop,
    StackOverflow,
    DeviceLost,
}

impl GpuCrashReason {
    /// Human-readable description
    pub const fn description(self) -> &'static str {
        match self {
            Self::Unknown => "Unknown GPU crash",
            Self::PageFault => "GPU page fault (invalid memory access)",
            Self::InvalidInstruction => "Invalid shader instruction",
            Self::Timeout => "GPU command timeout (possible infinite loop)",
            Self::ContextReset => "GPU context was reset",
            Self::OutOfMemory => "GPU ran out of memory",
            Self::InfiniteLoop => "Detected infinite loop in shader",
            Self::StackOverflow => "Shader stack overflow (too much recursion)",
            Self::DeviceLost => "GPU device was lost",
        }
    }
}

// ============================================================================
// Validation Checks
// ============================================================================

/// Configuration for validation checks
#[derive(Clone, Debug)]
pub struct ValidationConfig {
    /// Enable parameter validation
    pub validate_parameters: bool,
    /// Enable state tracking validation
    pub validate_state: bool,
    /// Enable memory validation
    pub validate_memory: bool,
    /// Enable synchronization validation
    pub validate_sync: bool,
    /// Enable shader validation
    pub validate_shaders: bool,
    /// Enable best practices warnings
    pub best_practices: bool,
    /// Enable GPU-assisted validation
    pub gpu_assisted: bool,
    /// Enable printf in shaders
    pub shader_printf: bool,
    /// Break on error
    pub break_on_error: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            validate_parameters: true,
            validate_state: true,
            validate_memory: true,
            validate_sync: true,
            validate_shaders: true,
            best_practices: false,
            gpu_assisted: false,
            shader_printf: false,
            break_on_error: false,
        }
    }
}

impl ValidationConfig {
    /// All validations enabled
    pub fn all() -> Self {
        Self {
            validate_parameters: true,
            validate_state: true,
            validate_memory: true,
            validate_sync: true,
            validate_shaders: true,
            best_practices: true,
            gpu_assisted: true,
            shader_printf: true,
            break_on_error: false,
        }
    }

    /// Minimal validation for release builds
    pub fn minimal() -> Self {
        Self {
            validate_parameters: false,
            validate_state: false,
            validate_memory: false,
            validate_sync: false,
            validate_shaders: false,
            best_practices: false,
            gpu_assisted: false,
            shader_printf: false,
            break_on_error: false,
        }
    }
}

// ============================================================================
// Debug Utils Extension Functions
// ============================================================================

/// Debug utilities interface
pub trait DebugUtils {
    /// Set object name
    fn set_object_name(&self, info: &DebugObjectNameInfo) -> crate::error::Result<()>;

    /// Set object tag
    fn set_object_tag(&self, info: &DebugObjectTagInfo) -> crate::error::Result<()>;

    /// Create debug messenger
    fn create_debug_messenger(
        &self,
        info: &DebugMessengerCreateInfo,
    ) -> crate::error::Result<DebugMessenger>;

    /// Destroy debug messenger
    fn destroy_debug_messenger(&self, messenger: DebugMessenger);

    /// Submit a debug message manually
    fn submit_debug_message(&self, severity: DebugSeverity, message_type: DebugType, message: &str);
}

// ============================================================================
// Printf Buffer for Shader Debugging
// ============================================================================

/// Buffer for capturing shader printf output
#[derive(Debug)]
pub struct ShaderPrintfBuffer {
    /// Raw buffer data
    data: [u8; Self::BUFFER_SIZE],
    /// Current write offset
    write_offset: u32,
    /// Overflow flag
    overflow: bool,
}

impl ShaderPrintfBuffer {
    /// Default buffer size (1MB)
    pub const BUFFER_SIZE: usize = 1024 * 1024;

    /// Create a new printf buffer
    pub const fn new() -> Self {
        Self {
            data: [0; Self::BUFFER_SIZE],
            write_offset: 0,
            overflow: false,
        }
    }

    /// Reset the buffer
    pub fn reset(&mut self) {
        self.write_offset = 0;
        self.overflow = false;
    }

    /// Check if buffer has overflowed
    pub const fn has_overflow(&self) -> bool {
        self.overflow
    }

    /// Get the current data slice
    pub fn data(&self) -> &[u8] {
        &self.data[..self.write_offset as usize]
    }
}

impl Default for ShaderPrintfBuffer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Debug Statistics
// ============================================================================

/// Debug statistics counters
#[derive(Clone, Debug, Default)]
pub struct DebugStatistics {
    /// Number of validation errors
    pub validation_errors: u64,
    /// Number of validation warnings
    pub validation_warnings: u64,
    /// Number of performance warnings
    pub performance_warnings: u64,
    /// Number of API calls
    pub api_calls: u64,
    /// Number of draw calls
    pub draw_calls: u64,
    /// Number of dispatch calls
    pub dispatch_calls: u64,
    /// Number of pipeline binds
    pub pipeline_binds: u64,
    /// Number of descriptor set binds
    pub descriptor_binds: u64,
    /// Number of render pass begins
    pub render_pass_begins: u64,
    /// Number of buffer uploads
    pub buffer_uploads: u64,
    /// Bytes uploaded to buffers
    pub buffer_bytes_uploaded: u64,
    /// Number of texture uploads
    pub texture_uploads: u64,
    /// Bytes uploaded to textures
    pub texture_bytes_uploaded: u64,
}

impl DebugStatistics {
    /// Reset all counters
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

// ============================================================================
// Scoped Debug Label
// ============================================================================

/// RAII guard for debug labels
pub struct ScopedDebugLabel<'a, T: CommandBufferDebugMarkers> {
    cmd: &'a mut T,
}

impl<'a, T: CommandBufferDebugMarkers> ScopedDebugLabel<'a, T> {
    /// Create a new scoped label
    pub fn new(cmd: &'a mut T, label: &DebugLabel) -> Self {
        cmd.begin_debug_label(label);
        Self { cmd }
    }
}

impl<'a, T: CommandBufferDebugMarkers> Drop for ScopedDebugLabel<'a, T> {
    fn drop(&mut self) {
        self.cmd.end_debug_label();
    }
}

/// Macro for scoped debug labels
#[macro_export]
macro_rules! debug_scope {
    ($cmd:expr, $name:expr) => {
        let _label = $crate::debug::DebugLabel::new($name);
        let _scope = $crate::debug::ScopedDebugLabel::new($cmd, &_label);
    };
    ($cmd:expr, $name:expr, $color:expr) => {
        let _label = $crate::debug::DebugLabel::with_color($name, $color);
        let _scope = $crate::debug::ScopedDebugLabel::new($cmd, &_label);
    };
}
