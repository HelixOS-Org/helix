//! GPU Debugging Types for Lumina
//!
//! This module provides GPU debugging infrastructure including
//! debug markers, labels, and validation layer integration.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Debug Handles
// ============================================================================

/// Debug messenger handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DebugMessengerHandle(pub u64);

impl DebugMessengerHandle {
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

impl Default for DebugMessengerHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Debug label handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DebugLabelHandle(pub u64);

impl DebugLabelHandle {
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

impl Default for DebugLabelHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Debug Messenger
// ============================================================================

/// Debug messenger create info
#[derive(Clone, Debug)]
pub struct DebugMessengerCreateInfo {
    /// Name
    pub name: String,
    /// Severity filter
    pub severity: DebugSeverityFlags,
    /// Type filter
    pub message_type: DebugTypeFlags,
    /// Break on error
    pub break_on_error: bool,
    /// Log to console
    pub log_to_console: bool,
    /// Max messages to store
    pub max_messages: u32,
}

impl DebugMessengerCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            severity: DebugSeverityFlags::ALL,
            message_type: DebugTypeFlags::ALL,
            break_on_error: false,
            log_to_console: true,
            max_messages: 1000,
        }
    }

    /// Errors only
    pub fn errors_only() -> Self {
        Self {
            severity: DebugSeverityFlags::ERROR,
            break_on_error: true,
            ..Self::new()
        }
    }

    /// Warnings and errors
    pub fn warnings_and_errors() -> Self {
        Self {
            severity: DebugSeverityFlags::WARNING | DebugSeverityFlags::ERROR,
            ..Self::new()
        }
    }

    /// Verbose (all messages)
    pub fn verbose() -> Self {
        Self {
            severity: DebugSeverityFlags::ALL,
            message_type: DebugTypeFlags::ALL,
            ..Self::new()
        }
    }

    /// With break on error
    pub fn with_break_on_error(mut self) -> Self {
        self.break_on_error = true;
        self
    }

    /// Filter validation only
    pub fn validation_only(mut self) -> Self {
        self.message_type = DebugTypeFlags::VALIDATION;
        self
    }
}

impl Default for DebugMessengerCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Debug severity flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DebugSeverityFlags(pub u32);

impl DebugSeverityFlags {
    /// Verbose
    pub const VERBOSE: Self = Self(1 << 0);
    /// Info
    pub const INFO: Self = Self(1 << 1);
    /// Warning
    pub const WARNING: Self = Self(1 << 2);
    /// Error
    pub const ERROR: Self = Self(1 << 3);
    /// All severities
    pub const ALL: Self = Self(0xF);

    /// Has flag
    pub const fn has(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }
}

impl Default for DebugSeverityFlags {
    fn default() -> Self {
        Self::ALL
    }
}

impl core::ops::BitOr for DebugSeverityFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Debug type flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DebugTypeFlags(pub u32);

impl DebugTypeFlags {
    /// General messages
    pub const GENERAL: Self = Self(1 << 0);
    /// Validation layer messages
    pub const VALIDATION: Self = Self(1 << 1);
    /// Performance warnings
    pub const PERFORMANCE: Self = Self(1 << 2);
    /// Device address binding messages
    pub const DEVICE_ADDRESS_BINDING: Self = Self(1 << 3);
    /// All types
    pub const ALL: Self = Self(0xF);

    /// Has flag
    pub const fn has(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }
}

impl Default for DebugTypeFlags {
    fn default() -> Self {
        Self::ALL
    }
}

impl core::ops::BitOr for DebugTypeFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

// ============================================================================
// Debug Messages
// ============================================================================

/// Debug message
#[derive(Clone, Debug)]
pub struct DebugMessage {
    /// Severity
    pub severity: DebugSeverity,
    /// Type
    pub message_type: DebugType,
    /// Message ID
    pub message_id: i32,
    /// Message ID name
    pub message_id_name: String,
    /// Message text
    pub message: String,
    /// Queue labels
    pub queue_labels: Vec<String>,
    /// Command buffer labels
    pub cmd_labels: Vec<String>,
    /// Objects involved
    pub objects: Vec<DebugObjectInfo>,
    /// Timestamp
    pub timestamp_us: u64,
}

impl DebugMessage {
    /// Creates message
    pub fn new(severity: DebugSeverity, message: &str) -> Self {
        Self {
            severity,
            message_type: DebugType::General,
            message_id: 0,
            message_id_name: String::new(),
            message: String::from(message),
            queue_labels: Vec::new(),
            cmd_labels: Vec::new(),
            objects: Vec::new(),
            timestamp_us: 0,
        }
    }

    /// Error message
    pub fn error(message: &str) -> Self {
        Self::new(DebugSeverity::Error, message)
    }

    /// Warning message
    pub fn warning(message: &str) -> Self {
        Self::new(DebugSeverity::Warning, message)
    }

    /// Info message
    pub fn info(message: &str) -> Self {
        Self::new(DebugSeverity::Info, message)
    }

    /// Is error
    pub fn is_error(&self) -> bool {
        matches!(self.severity, DebugSeverity::Error)
    }

    /// Is warning
    pub fn is_warning(&self) -> bool {
        matches!(self.severity, DebugSeverity::Warning)
    }
}

impl Default for DebugMessage {
    fn default() -> Self {
        Self::info("")
    }
}

/// Debug severity
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DebugSeverity {
    /// Verbose
    Verbose = 0,
    /// Info
    #[default]
    Info = 1,
    /// Warning
    Warning = 2,
    /// Error
    Error = 3,
}

impl DebugSeverity {
    /// To string
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Verbose => "VERBOSE",
            Self::Info => "INFO",
            Self::Warning => "WARNING",
            Self::Error => "ERROR",
        }
    }
}

/// Debug type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DebugType {
    /// General
    #[default]
    General = 0,
    /// Validation
    Validation = 1,
    /// Performance
    Performance = 2,
    /// Device address
    DeviceAddress = 3,
}

impl DebugType {
    /// To string
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::General => "GENERAL",
            Self::Validation => "VALIDATION",
            Self::Performance => "PERFORMANCE",
            Self::DeviceAddress => "DEVICE_ADDRESS",
        }
    }
}

/// Debug object info
#[derive(Clone, Debug)]
pub struct DebugObjectInfo {
    /// Object type
    pub object_type: DebugObjectType,
    /// Object handle
    pub handle: u64,
    /// Object name
    pub name: String,
}

impl DebugObjectInfo {
    /// Creates info
    pub fn new(object_type: DebugObjectType, handle: u64) -> Self {
        Self {
            object_type,
            handle,
            name: String::new(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = String::from(name);
        self
    }
}

impl Default for DebugObjectInfo {
    fn default() -> Self {
        Self::new(DebugObjectType::Unknown, 0)
    }
}

/// Debug object type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DebugObjectType {
    /// Unknown
    #[default]
    Unknown = 0,
    /// Instance
    Instance = 1,
    /// Device
    Device = 2,
    /// Queue
    Queue = 3,
    /// CommandBuffer
    CommandBuffer = 4,
    /// Buffer
    Buffer = 5,
    /// Image
    Image = 6,
    /// ImageView
    ImageView = 7,
    /// Sampler
    Sampler = 8,
    /// Pipeline
    Pipeline = 9,
    /// PipelineLayout
    PipelineLayout = 10,
    /// RenderPass
    RenderPass = 11,
    /// Framebuffer
    Framebuffer = 12,
    /// DescriptorSet
    DescriptorSet = 13,
    /// DescriptorPool
    DescriptorPool = 14,
    /// Shader
    Shader = 15,
    /// Fence
    Fence = 16,
    /// Semaphore
    Semaphore = 17,
    /// Swapchain
    Swapchain = 18,
    /// AccelerationStructure
    AccelerationStructure = 19,
}

impl DebugObjectType {
    /// To string
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::Instance => "Instance",
            Self::Device => "Device",
            Self::Queue => "Queue",
            Self::CommandBuffer => "CommandBuffer",
            Self::Buffer => "Buffer",
            Self::Image => "Image",
            Self::ImageView => "ImageView",
            Self::Sampler => "Sampler",
            Self::Pipeline => "Pipeline",
            Self::PipelineLayout => "PipelineLayout",
            Self::RenderPass => "RenderPass",
            Self::Framebuffer => "Framebuffer",
            Self::DescriptorSet => "DescriptorSet",
            Self::DescriptorPool => "DescriptorPool",
            Self::Shader => "Shader",
            Self::Fence => "Fence",
            Self::Semaphore => "Semaphore",
            Self::Swapchain => "Swapchain",
            Self::AccelerationStructure => "AccelerationStructure",
        }
    }
}

// ============================================================================
// Debug Markers
// ============================================================================

/// Debug marker
#[derive(Clone, Debug)]
pub struct DebugMarker {
    /// Label name
    pub name: String,
    /// Color (RGBA)
    pub color: [f32; 4],
}

impl DebugMarker {
    /// Creates marker
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }

    /// With color
    pub fn with_color(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.color = [r, g, b, a];
        self
    }

    /// Red marker
    pub fn red(name: &str) -> Self {
        Self::new(name).with_color(1.0, 0.0, 0.0, 1.0)
    }

    /// Green marker
    pub fn green(name: &str) -> Self {
        Self::new(name).with_color(0.0, 1.0, 0.0, 1.0)
    }

    /// Blue marker
    pub fn blue(name: &str) -> Self {
        Self::new(name).with_color(0.0, 0.0, 1.0, 1.0)
    }

    /// Yellow marker
    pub fn yellow(name: &str) -> Self {
        Self::new(name).with_color(1.0, 1.0, 0.0, 1.0)
    }

    /// Purple marker
    pub fn purple(name: &str) -> Self {
        Self::new(name).with_color(0.5, 0.0, 1.0, 1.0)
    }
}

impl Default for DebugMarker {
    fn default() -> Self {
        Self::new("Marker")
    }
}

/// Debug label info (for object naming)
#[derive(Clone, Debug)]
pub struct DebugLabelInfo {
    /// Object type
    pub object_type: DebugObjectType,
    /// Object handle
    pub handle: u64,
    /// Label name
    pub name: String,
}

impl DebugLabelInfo {
    /// Creates info
    pub fn new(object_type: DebugObjectType, handle: u64, name: &str) -> Self {
        Self {
            object_type,
            handle,
            name: String::from(name),
        }
    }

    /// Buffer
    pub fn buffer(handle: u64, name: &str) -> Self {
        Self::new(DebugObjectType::Buffer, handle, name)
    }

    /// Image
    pub fn image(handle: u64, name: &str) -> Self {
        Self::new(DebugObjectType::Image, handle, name)
    }

    /// Pipeline
    pub fn pipeline(handle: u64, name: &str) -> Self {
        Self::new(DebugObjectType::Pipeline, handle, name)
    }
}

impl Default for DebugLabelInfo {
    fn default() -> Self {
        Self::new(DebugObjectType::Unknown, 0, "")
    }
}

// ============================================================================
// GPU Capture
// ============================================================================

/// GPU capture settings
#[derive(Clone, Debug)]
pub struct GpuCaptureSettings {
    /// Capture path
    pub output_path: String,
    /// Capture format
    pub format: CaptureFormat,
    /// Include resources
    pub include_resources: bool,
    /// Include shaders
    pub include_shaders: bool,
    /// Max frames
    pub max_frames: u32,
}

impl GpuCaptureSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            output_path: String::from("capture"),
            format: CaptureFormat::RenderDoc,
            include_resources: true,
            include_shaders: true,
            max_frames: 1,
        }
    }

    /// RenderDoc capture
    pub fn renderdoc() -> Self {
        Self {
            format: CaptureFormat::RenderDoc,
            ..Self::new()
        }
    }

    /// PIX capture
    pub fn pix() -> Self {
        Self {
            format: CaptureFormat::Pix,
            ..Self::new()
        }
    }

    /// Nsight capture
    pub fn nsight() -> Self {
        Self {
            format: CaptureFormat::Nsight,
            ..Self::new()
        }
    }

    /// With path
    pub fn with_path(mut self, path: &str) -> Self {
        self.output_path = String::from(path);
        self
    }

    /// Multi-frame
    pub fn multi_frame(mut self, count: u32) -> Self {
        self.max_frames = count;
        self
    }
}

impl Default for GpuCaptureSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Capture format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CaptureFormat {
    /// RenderDoc
    #[default]
    RenderDoc = 0,
    /// PIX (Windows)
    Pix = 1,
    /// Nsight (NVIDIA)
    Nsight = 2,
    /// AMD GPU Profiler
    Rgp = 3,
}

// ============================================================================
// Validation Features
// ============================================================================

/// Validation features
#[derive(Clone, Copy, Debug)]
pub struct ValidationFeatures {
    /// GPU-assisted validation
    pub gpu_assisted: bool,
    /// GPU-assisted reserve binding slot
    pub gpu_assisted_reserve_binding_slot: bool,
    /// Best practices warnings
    pub best_practices: bool,
    /// Debug printf
    pub debug_printf: bool,
    /// Synchronization validation
    pub synchronization: bool,
    /// Shader validation
    pub shader_validation: bool,
}

impl ValidationFeatures {
    /// Creates features
    pub fn new() -> Self {
        Self {
            gpu_assisted: false,
            gpu_assisted_reserve_binding_slot: false,
            best_practices: true,
            debug_printf: false,
            synchronization: true,
            shader_validation: true,
        }
    }

    /// All features
    pub fn all() -> Self {
        Self {
            gpu_assisted: true,
            gpu_assisted_reserve_binding_slot: true,
            best_practices: true,
            debug_printf: true,
            synchronization: true,
            shader_validation: true,
        }
    }

    /// Minimal (fast)
    pub fn minimal() -> Self {
        Self {
            gpu_assisted: false,
            gpu_assisted_reserve_binding_slot: false,
            best_practices: false,
            debug_printf: false,
            synchronization: false,
            shader_validation: true,
        }
    }

    /// Performance focused
    pub fn performance() -> Self {
        Self {
            best_practices: true,
            ..Self::minimal()
        }
    }
}

impl Default for ValidationFeatures {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Debug Utils
// ============================================================================

/// Debug utility settings
#[derive(Clone, Debug)]
pub struct DebugUtilsSettings {
    /// Enable debug markers
    pub markers_enabled: bool,
    /// Enable object labels
    pub labels_enabled: bool,
    /// Enable validation layers
    pub validation_enabled: bool,
    /// Validation features
    pub validation_features: ValidationFeatures,
    /// Messenger settings
    pub messenger: DebugMessengerCreateInfo,
}

impl DebugUtilsSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            markers_enabled: true,
            labels_enabled: true,
            validation_enabled: true,
            validation_features: ValidationFeatures::default(),
            messenger: DebugMessengerCreateInfo::default(),
        }
    }

    /// Debug build settings
    pub fn debug_build() -> Self {
        Self {
            validation_enabled: true,
            validation_features: ValidationFeatures::all(),
            messenger: DebugMessengerCreateInfo::verbose(),
            ..Self::new()
        }
    }

    /// Release build settings
    pub fn release_build() -> Self {
        Self {
            validation_enabled: false,
            markers_enabled: false,
            labels_enabled: false,
            ..Self::new()
        }
    }

    /// Profile build settings
    pub fn profile_build() -> Self {
        Self {
            validation_enabled: false,
            markers_enabled: true,
            labels_enabled: true,
            ..Self::new()
        }
    }
}

impl Default for DebugUtilsSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Debug statistics
#[derive(Clone, Debug, Default)]
pub struct DebugStats {
    /// Total messages
    pub message_count: u32,
    /// Error count
    pub error_count: u32,
    /// Warning count
    pub warning_count: u32,
    /// Labeled objects
    pub labeled_objects: u32,
    /// Active debug regions
    pub active_regions: u32,
}
