//! Debug Utilities for Lumina
//!
//! This module provides GPU debugging utilities, markers, labels,
//! and debug messenger types for development and profiling.

use core::fmt;

// ============================================================================
// Debug Messenger
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

/// Debug messenger configuration
#[derive(Clone, Debug)]
#[repr(C)]
pub struct DebugMessengerConfig {
    /// Message severity filter
    pub message_severity: DebugMessageSeverity,
    /// Message type filter
    pub message_type: DebugMessageType,
    /// Flags
    pub flags: DebugMessengerFlags,
}

impl DebugMessengerConfig {
    /// Creates new config with all messages
    #[inline]
    pub const fn all() -> Self {
        Self {
            message_severity: DebugMessageSeverity::ALL,
            message_type: DebugMessageType::ALL,
            flags: DebugMessengerFlags::NONE,
        }
    }

    /// Creates config for errors only
    #[inline]
    pub const fn errors_only() -> Self {
        Self {
            message_severity: DebugMessageSeverity::ERROR,
            message_type: DebugMessageType::ALL,
            flags: DebugMessengerFlags::NONE,
        }
    }

    /// Creates config for warnings and errors
    #[inline]
    pub const fn warnings_and_errors() -> Self {
        Self {
            message_severity: DebugMessageSeverity::WARNING.union(DebugMessageSeverity::ERROR),
            message_type: DebugMessageType::ALL,
            flags: DebugMessengerFlags::NONE,
        }
    }

    /// Creates config for validation messages
    #[inline]
    pub const fn validation() -> Self {
        Self {
            message_severity: DebugMessageSeverity::ALL,
            message_type: DebugMessageType::VALIDATION,
            flags: DebugMessengerFlags::NONE,
        }
    }

    /// Creates config for performance messages
    #[inline]
    pub const fn performance() -> Self {
        Self {
            message_severity: DebugMessageSeverity::WARNING.union(DebugMessageSeverity::INFO),
            message_type: DebugMessageType::PERFORMANCE,
            flags: DebugMessengerFlags::NONE,
        }
    }

    /// With severity filter
    #[inline]
    pub const fn with_severity(mut self, severity: DebugMessageSeverity) -> Self {
        self.message_severity = severity;
        self
    }

    /// With type filter
    #[inline]
    pub const fn with_type(mut self, message_type: DebugMessageType) -> Self {
        self.message_type = message_type;
        self
    }
}

impl Default for DebugMessengerConfig {
    fn default() -> Self {
        Self::warnings_and_errors()
    }
}

/// Debug messenger flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct DebugMessengerFlags(pub u32);

impl DebugMessengerFlags {
    /// No flags
    pub const NONE: Self = Self(0);

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
// Debug Message Severity
// ============================================================================

/// Debug message severity
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct DebugMessageSeverity(pub u32);

impl DebugMessageSeverity {
    /// Verbose (diagnostic)
    pub const VERBOSE: Self = Self(0x00000001);
    /// Info
    pub const INFO: Self = Self(0x00000010);
    /// Warning
    pub const WARNING: Self = Self(0x00000100);
    /// Error
    pub const ERROR: Self = Self(0x00001000);
    /// All severities
    pub const ALL: Self = Self(0x00001111);

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

    /// Is error
    #[inline]
    pub const fn is_error(&self) -> bool {
        self.contains(Self::ERROR)
    }

    /// Is warning
    #[inline]
    pub const fn is_warning(&self) -> bool {
        self.contains(Self::WARNING)
    }

    /// Severity name
    #[inline]
    pub const fn name(&self) -> &'static str {
        if self.0 & Self::ERROR.0 != 0 {
            "ERROR"
        } else if self.0 & Self::WARNING.0 != 0 {
            "WARNING"
        } else if self.0 & Self::INFO.0 != 0 {
            "INFO"
        } else {
            "VERBOSE"
        }
    }
}

impl fmt::Display for DebugMessageSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

// ============================================================================
// Debug Message Type
// ============================================================================

/// Debug message type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct DebugMessageType(pub u32);

impl DebugMessageType {
    /// General messages
    pub const GENERAL: Self = Self(0x00000001);
    /// Validation messages
    pub const VALIDATION: Self = Self(0x00000002);
    /// Performance messages
    pub const PERFORMANCE: Self = Self(0x00000004);
    /// Device address binding messages
    pub const DEVICE_ADDRESS_BINDING: Self = Self(0x00000008);
    /// All types
    pub const ALL: Self = Self(0x0000000F);

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

    /// Type name
    #[inline]
    pub const fn name(&self) -> &'static str {
        if self.0 & Self::VALIDATION.0 != 0 {
            "VALIDATION"
        } else if self.0 & Self::PERFORMANCE.0 != 0 {
            "PERFORMANCE"
        } else if self.0 & Self::DEVICE_ADDRESS_BINDING.0 != 0 {
            "DEVICE_ADDRESS_BINDING"
        } else {
            "GENERAL"
        }
    }
}

impl fmt::Display for DebugMessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

// ============================================================================
// Debug Callback Data
// ============================================================================

/// Debug callback data
#[derive(Clone, Debug)]
#[repr(C)]
pub struct DebugCallbackData<'a> {
    /// Message ID name (optional)
    pub message_id_name: Option<&'a str>,
    /// Message ID number
    pub message_id_number: i32,
    /// Message
    pub message: &'a str,
    /// Queue labels
    pub queue_labels: &'a [DebugLabel<'a>],
    /// Command buffer labels
    pub cmd_buf_labels: &'a [DebugLabel<'a>],
    /// Objects
    pub objects: &'a [DebugObjectInfo<'a>],
}

impl<'a> DebugCallbackData<'a> {
    /// Creates new callback data
    #[inline]
    pub const fn new(message: &'a str) -> Self {
        Self {
            message_id_name: None,
            message_id_number: 0,
            message,
            queue_labels: &[],
            cmd_buf_labels: &[],
            objects: &[],
        }
    }

    /// With message ID
    #[inline]
    pub const fn with_message_id(mut self, name: &'a str, number: i32) -> Self {
        self.message_id_name = Some(name);
        self.message_id_number = number;
        self
    }
}

// ============================================================================
// Debug Labels
// ============================================================================

/// Debug label
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DebugLabel<'a> {
    /// Label name
    pub label_name: &'a str,
    /// Color (RGBA)
    pub color: [f32; 4],
}

impl<'a> DebugLabel<'a> {
    /// Creates new label
    #[inline]
    pub const fn new(name: &'a str) -> Self {
        Self {
            label_name: name,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }

    /// With color
    #[inline]
    pub const fn with_color(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.color = [r, g, b, a];
        self
    }

    /// Red label
    #[inline]
    pub const fn red(name: &'a str) -> Self {
        Self {
            label_name: name,
            color: [1.0, 0.0, 0.0, 1.0],
        }
    }

    /// Green label
    #[inline]
    pub const fn green(name: &'a str) -> Self {
        Self {
            label_name: name,
            color: [0.0, 1.0, 0.0, 1.0],
        }
    }

    /// Blue label
    #[inline]
    pub const fn blue(name: &'a str) -> Self {
        Self {
            label_name: name,
            color: [0.0, 0.0, 1.0, 1.0],
        }
    }

    /// Yellow label
    #[inline]
    pub const fn yellow(name: &'a str) -> Self {
        Self {
            label_name: name,
            color: [1.0, 1.0, 0.0, 1.0],
        }
    }

    /// Cyan label
    #[inline]
    pub const fn cyan(name: &'a str) -> Self {
        Self {
            label_name: name,
            color: [0.0, 1.0, 1.0, 1.0],
        }
    }

    /// Magenta label
    #[inline]
    pub const fn magenta(name: &'a str) -> Self {
        Self {
            label_name: name,
            color: [1.0, 0.0, 1.0, 1.0],
        }
    }

    /// Orange label
    #[inline]
    pub const fn orange(name: &'a str) -> Self {
        Self {
            label_name: name,
            color: [1.0, 0.5, 0.0, 1.0],
        }
    }

    /// Purple label
    #[inline]
    pub const fn purple(name: &'a str) -> Self {
        Self {
            label_name: name,
            color: [0.5, 0.0, 1.0, 1.0],
        }
    }
}

impl Default for DebugLabel<'_> {
    fn default() -> Self {
        Self::new("")
    }
}

/// Debug label info (owned version)
#[derive(Clone, Debug, Default)]
#[repr(C)]
pub struct DebugLabelInfo {
    /// Label name
    pub label_name: [u8; 256],
    /// Label name length
    pub label_name_len: u32,
    /// Color (RGBA)
    pub color: [f32; 4],
}

impl DebugLabelInfo {
    /// Creates new info
    pub fn new(name: &str) -> Self {
        let mut label_name = [0u8; 256];
        let len = name.len().min(255);
        label_name[..len].copy_from_slice(&name.as_bytes()[..len]);
        Self {
            label_name,
            label_name_len: len as u32,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }

    /// With color
    #[inline]
    pub const fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    /// Get name as str
    #[inline]
    pub fn name_str(&self) -> &str {
        core::str::from_utf8(&self.label_name[..self.label_name_len as usize]).unwrap_or("")
    }
}

// ============================================================================
// Debug Object Info
// ============================================================================

/// Debug object info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DebugObjectInfo<'a> {
    /// Object type
    pub object_type: DebugObjectType,
    /// Object handle
    pub object_handle: u64,
    /// Object name (optional)
    pub object_name: Option<&'a str>,
}

impl<'a> DebugObjectInfo<'a> {
    /// Creates new info
    #[inline]
    pub const fn new(object_type: DebugObjectType, handle: u64) -> Self {
        Self {
            object_type,
            object_handle: handle,
            object_name: None,
        }
    }

    /// With name
    #[inline]
    pub const fn with_name(mut self, name: &'a str) -> Self {
        self.object_name = Some(name);
        self
    }
}

/// Debug object type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DebugObjectType {
    /// Unknown
    #[default]
    Unknown             = 0,
    /// Instance
    Instance            = 1,
    /// Physical device
    PhysicalDevice      = 2,
    /// Device
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
    /// Image
    Image               = 10,
    /// Event
    Event               = 11,
    /// Query pool
    QueryPool           = 12,
    /// Buffer view
    BufferView          = 13,
    /// Image view
    ImageView           = 14,
    /// Shader module
    ShaderModule        = 15,
    /// Pipeline cache
    PipelineCache       = 16,
    /// Pipeline layout
    PipelineLayout      = 17,
    /// Render pass
    RenderPass          = 18,
    /// Pipeline
    Pipeline            = 19,
    /// Descriptor set layout
    DescriptorSetLayout = 20,
    /// Sampler
    Sampler             = 21,
    /// Descriptor pool
    DescriptorPool      = 22,
    /// Descriptor set
    DescriptorSet       = 23,
    /// Framebuffer
    Framebuffer         = 24,
    /// Command pool
    CommandPool         = 25,
    /// Sampler Ycbcr conversion
    SamplerYcbcrConversion = 1000156000,
    /// Descriptor update template
    DescriptorUpdateTemplate = 1000085000,
    /// Private data slot
    PrivateDataSlot     = 1000295000,
    /// Surface
    Surface             = 1000000000,
    /// Swapchain
    Swapchain           = 1000001000,
    /// Display
    Display             = 1000002000,
    /// Display mode
    DisplayMode         = 1000002001,
    /// Debug messenger
    DebugMessenger      = 1000128000,
    /// Acceleration structure
    AccelerationStructure = 1000150000,
    /// Validation cache
    ValidationCache     = 1000160000,
    /// Shader binding table
    ShaderBindingTable  = 1000150000,
    /// Micromap
    Micromap            = 1000396000,
    /// Optical flow session
    OpticalFlowSession  = 1000464000,
    /// Video session
    VideoSession        = 1000023000,
    /// Video session parameters
    VideoSessionParameters = 1000023001,
    /// Shader
    Shader              = 1000482000,
}

impl DebugObjectType {
    /// Type name
    #[inline]
    pub const fn name(&self) -> &'static str {
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
            Self::SamplerYcbcrConversion => "SamplerYcbcrConversion",
            Self::DescriptorUpdateTemplate => "DescriptorUpdateTemplate",
            Self::PrivateDataSlot => "PrivateDataSlot",
            Self::Surface => "Surface",
            Self::Swapchain => "Swapchain",
            Self::Display => "Display",
            Self::DisplayMode => "DisplayMode",
            Self::DebugMessenger => "DebugMessenger",
            Self::AccelerationStructure => "AccelerationStructure",
            Self::ValidationCache => "ValidationCache",
            Self::ShaderBindingTable => "ShaderBindingTable",
            Self::Micromap => "Micromap",
            Self::OpticalFlowSession => "OpticalFlowSession",
            Self::VideoSession => "VideoSession",
            Self::VideoSessionParameters => "VideoSessionParameters",
            Self::Shader => "Shader",
        }
    }
}

impl fmt::Display for DebugObjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

// ============================================================================
// Debug Object Name Info
// ============================================================================

/// Debug object name info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DebugObjectNameInfo<'a> {
    /// Object type
    pub object_type: DebugObjectType,
    /// Object handle
    pub object_handle: u64,
    /// Object name
    pub object_name: &'a str,
}

impl<'a> DebugObjectNameInfo<'a> {
    /// Creates new info
    #[inline]
    pub const fn new(object_type: DebugObjectType, handle: u64, name: &'a str) -> Self {
        Self {
            object_type,
            object_handle: handle,
            object_name: name,
        }
    }

    /// Buffer
    #[inline]
    pub const fn buffer(handle: u64, name: &'a str) -> Self {
        Self::new(DebugObjectType::Buffer, handle, name)
    }

    /// Image
    #[inline]
    pub const fn image(handle: u64, name: &'a str) -> Self {
        Self::new(DebugObjectType::Image, handle, name)
    }

    /// Pipeline
    #[inline]
    pub const fn pipeline(handle: u64, name: &'a str) -> Self {
        Self::new(DebugObjectType::Pipeline, handle, name)
    }

    /// Shader module
    #[inline]
    pub const fn shader_module(handle: u64, name: &'a str) -> Self {
        Self::new(DebugObjectType::ShaderModule, handle, name)
    }

    /// Command buffer
    #[inline]
    pub const fn command_buffer(handle: u64, name: &'a str) -> Self {
        Self::new(DebugObjectType::CommandBuffer, handle, name)
    }

    /// Descriptor set
    #[inline]
    pub const fn descriptor_set(handle: u64, name: &'a str) -> Self {
        Self::new(DebugObjectType::DescriptorSet, handle, name)
    }

    /// Render pass
    #[inline]
    pub const fn render_pass(handle: u64, name: &'a str) -> Self {
        Self::new(DebugObjectType::RenderPass, handle, name)
    }

    /// Framebuffer
    #[inline]
    pub const fn framebuffer(handle: u64, name: &'a str) -> Self {
        Self::new(DebugObjectType::Framebuffer, handle, name)
    }
}

// ============================================================================
// Debug Object Tag Info
// ============================================================================

/// Debug object tag info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DebugObjectTagInfo<'a> {
    /// Object type
    pub object_type: DebugObjectType,
    /// Object handle
    pub object_handle: u64,
    /// Tag name (as u64 for compatibility)
    pub tag_name: u64,
    /// Tag data
    pub tag_data: &'a [u8],
}

impl<'a> DebugObjectTagInfo<'a> {
    /// Creates new tag info
    #[inline]
    pub const fn new(
        object_type: DebugObjectType,
        handle: u64,
        tag_name: u64,
        data: &'a [u8],
    ) -> Self {
        Self {
            object_type,
            object_handle: handle,
            tag_name,
            tag_data: data,
        }
    }
}

// ============================================================================
// Debug Marker Region
// ============================================================================

/// Debug marker region (for profiling tools)
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DebugMarkerRegion<'a> {
    /// Region name
    pub name: &'a str,
    /// Color (RGBA)
    pub color: [f32; 4],
}

impl<'a> DebugMarkerRegion<'a> {
    /// Creates new region
    #[inline]
    pub const fn new(name: &'a str) -> Self {
        Self {
            name,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }

    /// With color
    #[inline]
    pub const fn with_color(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.color = [r, g, b, a];
        self
    }

    /// Frame region (orange)
    #[inline]
    pub const fn frame(name: &'a str) -> Self {
        Self {
            name,
            color: [1.0, 0.5, 0.0, 1.0],
        }
    }

    /// Render pass region (blue)
    #[inline]
    pub const fn render_pass(name: &'a str) -> Self {
        Self {
            name,
            color: [0.2, 0.4, 0.8, 1.0],
        }
    }

    /// Compute region (green)
    #[inline]
    pub const fn compute(name: &'a str) -> Self {
        Self {
            name,
            color: [0.2, 0.8, 0.2, 1.0],
        }
    }

    /// Transfer region (purple)
    #[inline]
    pub const fn transfer(name: &'a str) -> Self {
        Self {
            name,
            color: [0.6, 0.2, 0.8, 1.0],
        }
    }

    /// Ray tracing region (red)
    #[inline]
    pub const fn ray_tracing(name: &'a str) -> Self {
        Self {
            name,
            color: [0.9, 0.1, 0.1, 1.0],
        }
    }

    /// Barrier region (yellow)
    #[inline]
    pub const fn barrier(name: &'a str) -> Self {
        Self {
            name,
            color: [0.9, 0.9, 0.1, 1.0],
        }
    }
}

impl Default for DebugMarkerRegion<'_> {
    fn default() -> Self {
        Self::new("")
    }
}

// ============================================================================
// Debug Utils Insert
// ============================================================================

/// Debug marker insert (single point marker)
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DebugMarkerInsert<'a> {
    /// Marker name
    pub name: &'a str,
    /// Color (RGBA)
    pub color: [f32; 4],
}

impl<'a> DebugMarkerInsert<'a> {
    /// Creates new insert marker
    #[inline]
    pub const fn new(name: &'a str) -> Self {
        Self {
            name,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }

    /// With color
    #[inline]
    pub const fn with_color(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.color = [r, g, b, a];
        self
    }
}

// ============================================================================
// Validation Features
// ============================================================================

/// Validation feature enable
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ValidationFeatureEnable {
    /// GPU-assisted validation
    GpuAssisted   = 0,
    /// GPU-assisted reserve binding slot
    GpuAssistedReserveBindingSlot = 1,
    /// Best practices
    BestPractices = 2,
    /// Debug printf
    DebugPrintf   = 3,
    /// Synchronization validation
    SynchronizationValidation = 4,
}

/// Validation feature disable
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ValidationFeatureDisable {
    /// All validation
    All             = 0,
    /// Shaders
    Shaders         = 1,
    /// Thread safety
    ThreadSafety    = 2,
    /// API parameters
    ApiParameters   = 3,
    /// Object lifetimes
    ObjectLifetimes = 4,
    /// Core checks
    CoreChecks      = 5,
    /// Unique handles
    UniqueHandles   = 6,
    /// Shader validation cache
    ShaderValidationCache = 7,
}

/// Validation features configuration
#[derive(Clone, Debug, Default)]
#[repr(C)]
pub struct ValidationFeaturesConfig {
    /// Enabled features
    pub enabled_features: &'static [ValidationFeatureEnable],
    /// Disabled features
    pub disabled_features: &'static [ValidationFeatureDisable],
}

impl ValidationFeaturesConfig {
    /// Full validation
    pub const FULL: Self = Self {
        enabled_features: &[
            ValidationFeatureEnable::GpuAssisted,
            ValidationFeatureEnable::BestPractices,
            ValidationFeatureEnable::SynchronizationValidation,
        ],
        disabled_features: &[],
    };

    /// Minimal validation
    pub const MINIMAL: Self = Self {
        enabled_features: &[],
        disabled_features: &[
            ValidationFeatureDisable::Shaders,
            ValidationFeatureDisable::UniqueHandles,
        ],
    };

    /// Debug printf only
    pub const DEBUG_PRINTF: Self = Self {
        enabled_features: &[ValidationFeatureEnable::DebugPrintf],
        disabled_features: &[ValidationFeatureDisable::All],
    };

    /// Sync validation
    pub const SYNC_VALIDATION: Self = Self {
        enabled_features: &[ValidationFeatureEnable::SynchronizationValidation],
        disabled_features: &[],
    };

    /// Creates new config
    #[inline]
    pub const fn new() -> Self {
        Self {
            enabled_features: &[],
            disabled_features: &[],
        }
    }
}

// ============================================================================
// Debug Report
// ============================================================================

/// Debug report flags (legacy)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct DebugReportFlags(pub u32);

impl DebugReportFlags {
    /// Information
    pub const INFORMATION: Self = Self(0x00000001);
    /// Warning
    pub const WARNING: Self = Self(0x00000002);
    /// Performance warning
    pub const PERFORMANCE_WARNING: Self = Self(0x00000004);
    /// Error
    pub const ERROR: Self = Self(0x00000008);
    /// Debug
    pub const DEBUG: Self = Self(0x00000010);
    /// All flags
    pub const ALL: Self = Self(0x0000001F);

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
// Debug Statistics
// ============================================================================

/// Debug statistics
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DebugStatistics {
    /// Total messages
    pub total_messages: u64,
    /// Error count
    pub error_count: u64,
    /// Warning count
    pub warning_count: u64,
    /// Info count
    pub info_count: u64,
    /// Verbose count
    pub verbose_count: u64,
    /// Validation messages
    pub validation_messages: u64,
    /// Performance messages
    pub performance_messages: u64,
}

impl DebugStatistics {
    /// Creates new statistics
    #[inline]
    pub const fn new() -> Self {
        Self {
            total_messages: 0,
            error_count: 0,
            warning_count: 0,
            info_count: 0,
            verbose_count: 0,
            validation_messages: 0,
            performance_messages: 0,
        }
    }

    /// Has errors
    #[inline]
    pub const fn has_errors(&self) -> bool {
        self.error_count > 0
    }

    /// Has warnings
    #[inline]
    pub const fn has_warnings(&self) -> bool {
        self.warning_count > 0
    }

    /// Is clean (no errors or warnings)
    #[inline]
    pub const fn is_clean(&self) -> bool {
        self.error_count == 0 && self.warning_count == 0
    }
}
