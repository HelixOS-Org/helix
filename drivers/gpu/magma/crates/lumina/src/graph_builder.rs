//! Render Graph Builder Types for Lumina
//!
//! This module provides advanced render graph building infrastructure
//! for frame-graph based rendering with automatic scheduling.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Graph Builder Handles
// ============================================================================

/// Graph builder handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GraphBuilderHandle(pub u64);

impl GraphBuilderHandle {
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

impl Default for GraphBuilderHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Pass builder handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PassBuilderHandle(pub u64);

impl PassBuilderHandle {
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

impl Default for PassBuilderHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Blackboard handle (for sharing data between passes)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BlackboardHandle(pub u64);

impl BlackboardHandle {
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

impl Default for BlackboardHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Graph Builder
// ============================================================================

/// Graph builder create info
#[derive(Clone, Debug)]
pub struct GraphBuilderCreateInfo {
    /// Name
    pub name: String,
    /// Frame count (for temporal resources)
    pub frame_count: u32,
    /// Enable pass culling
    pub pass_culling: bool,
    /// Enable resource aliasing
    pub resource_aliasing: bool,
    /// Memory budget
    pub memory_budget: u64,
}

impl GraphBuilderCreateInfo {
    /// Creates info
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            frame_count: 2,
            pass_culling: true,
            resource_aliasing: true,
            memory_budget: 256 * 1024 * 1024,
        }
    }

    /// With frame count
    pub fn with_frame_count(mut self, count: u32) -> Self {
        self.frame_count = count;
        self
    }

    /// Without culling
    pub fn without_culling(mut self) -> Self {
        self.pass_culling = false;
        self
    }
}

impl Default for GraphBuilderCreateInfo {
    fn default() -> Self {
        Self::new("RenderGraph")
    }
}

// ============================================================================
// Pass Builder
// ============================================================================

/// Pass builder
#[derive(Clone, Debug)]
pub struct PassBuilder {
    /// Name
    pub name: String,
    /// Queue
    pub queue: PassQueue,
    /// Reads
    pub reads: Vec<ResourceRef>,
    /// Writes
    pub writes: Vec<ResourceRef>,
    /// Create resources
    pub creates: Vec<ResourceCreate>,
    /// Depth attachment
    pub depth: Option<DepthAttachment>,
    /// Color attachments
    pub colors: Vec<ColorAttachment>,
    /// Side effects
    pub side_effects: bool,
}

impl PassBuilder {
    /// Creates builder
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            queue: PassQueue::Graphics,
            reads: Vec::new(),
            writes: Vec::new(),
            creates: Vec::new(),
            depth: None,
            colors: Vec::new(),
            side_effects: false,
        }
    }

    /// Compute pass
    pub fn compute(name: &str) -> Self {
        Self {
            queue: PassQueue::Compute,
            ..Self::new(name)
        }
    }

    /// Read resource
    pub fn read(mut self, resource: ResourceRef) -> Self {
        self.reads.push(resource);
        self
    }

    /// Write resource
    pub fn write(mut self, resource: ResourceRef) -> Self {
        self.writes.push(resource);
        self
    }

    /// Create resource
    pub fn create(mut self, resource: ResourceCreate) -> Self {
        self.creates.push(resource);
        self
    }

    /// Add color attachment
    pub fn color(mut self, attachment: ColorAttachment) -> Self {
        self.colors.push(attachment);
        self
    }

    /// Set depth attachment
    pub fn depth(mut self, attachment: DepthAttachment) -> Self {
        self.depth = Some(attachment);
        self
    }

    /// Has side effects (prevents culling)
    pub fn with_side_effects(mut self) -> Self {
        self.side_effects = true;
        self
    }
}

impl Default for PassBuilder {
    fn default() -> Self {
        Self::new("Pass")
    }
}

/// Pass queue
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PassQueue {
    /// Graphics queue
    #[default]
    Graphics = 0,
    /// Compute queue
    Compute = 1,
    /// Async compute
    AsyncCompute = 2,
    /// Transfer
    Transfer = 3,
}

// ============================================================================
// Resource References
// ============================================================================

/// Resource reference
#[derive(Clone, Debug)]
pub struct ResourceRef {
    /// Resource ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Access
    pub access: ResourceRefAccess,
    /// Stage
    pub stage: PipelineStage,
}

impl ResourceRef {
    /// Creates reference
    pub fn new(id: u64, name: &str) -> Self {
        Self {
            id,
            name: String::from(name),
            access: ResourceRefAccess::Read,
            stage: PipelineStage::Fragment,
        }
    }

    /// Read in fragment shader
    pub fn fragment_read(id: u64, name: &str) -> Self {
        Self::new(id, name)
    }

    /// Read in compute shader
    pub fn compute_read(id: u64, name: &str) -> Self {
        Self {
            stage: PipelineStage::Compute,
            ..Self::new(id, name)
        }
    }

    /// Write in compute shader
    pub fn compute_write(id: u64, name: &str) -> Self {
        Self {
            access: ResourceRefAccess::Write,
            stage: PipelineStage::Compute,
            ..Self::new(id, name)
        }
    }

    /// With stage
    pub fn at_stage(mut self, stage: PipelineStage) -> Self {
        self.stage = stage;
        self
    }
}

/// Resource reference access
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ResourceRefAccess {
    /// Read
    #[default]
    Read = 0,
    /// Write
    Write = 1,
    /// Read-write
    ReadWrite = 2,
}

/// Pipeline stage
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PipelineStage {
    /// Vertex shader
    Vertex = 0,
    /// Fragment shader
    #[default]
    Fragment = 1,
    /// Compute shader
    Compute = 2,
    /// Transfer
    Transfer = 3,
    /// All graphics
    AllGraphics = 4,
    /// All commands
    AllCommands = 5,
}

/// Resource create info
#[derive(Clone, Debug)]
pub struct ResourceCreate {
    /// Name
    pub name: String,
    /// Type
    pub resource_type: ResourceCreateType,
    /// Size info
    pub size: ResourceSize,
    /// Format
    pub format: u32,
    /// Initial state
    pub initial_state: u32,
}

impl ResourceCreate {
    /// Create texture
    pub fn texture(name: &str, size: ResourceSize, format: u32) -> Self {
        Self {
            name: String::from(name),
            resource_type: ResourceCreateType::Texture2D,
            size,
            format,
            initial_state: 0,
        }
    }

    /// Create buffer
    pub fn buffer(name: &str, size: u64) -> Self {
        Self {
            name: String::from(name),
            resource_type: ResourceCreateType::Buffer,
            size: ResourceSize::Absolute(size as u32, 0),
            format: 0,
            initial_state: 0,
        }
    }
}

impl Default for ResourceCreate {
    fn default() -> Self {
        Self {
            name: String::new(),
            resource_type: ResourceCreateType::Texture2D,
            size: ResourceSize::FullResolution,
            format: 0,
            initial_state: 0,
        }
    }
}

/// Resource create type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ResourceCreateType {
    /// 2D texture
    #[default]
    Texture2D = 0,
    /// 3D texture
    Texture3D = 1,
    /// Cube texture
    TextureCube = 2,
    /// Buffer
    Buffer = 3,
}

/// Resource size
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ResourceSize {
    /// Full resolution
    FullResolution,
    /// Half resolution
    HalfResolution,
    /// Quarter resolution
    QuarterResolution,
    /// Scaled
    Scaled(f32),
    /// Absolute size
    Absolute(u32, u32),
}

impl ResourceSize {
    /// Calculate actual size
    pub fn calculate(&self, width: u32, height: u32) -> (u32, u32) {
        match self {
            Self::FullResolution => (width, height),
            Self::HalfResolution => (width / 2, height / 2),
            Self::QuarterResolution => (width / 4, height / 4),
            Self::Scaled(s) => ((width as f32 * s) as u32, (height as f32 * s) as u32),
            Self::Absolute(w, h) => (*w, *h),
        }
    }
}

impl Default for ResourceSize {
    fn default() -> Self {
        Self::FullResolution
    }
}

// ============================================================================
// Attachments
// ============================================================================

/// Color attachment
#[derive(Clone, Debug)]
pub struct ColorAttachment {
    /// Resource ID
    pub resource: u64,
    /// Load operation
    pub load_op: LoadOp,
    /// Store operation
    pub store_op: StoreOp,
    /// Clear color
    pub clear_color: [f32; 4],
}

impl ColorAttachment {
    /// Creates attachment
    pub fn new(resource: u64) -> Self {
        Self {
            resource,
            load_op: LoadOp::Clear,
            store_op: StoreOp::Store,
            clear_color: [0.0, 0.0, 0.0, 1.0],
        }
    }

    /// Load existing content
    pub fn load(resource: u64) -> Self {
        Self {
            load_op: LoadOp::Load,
            ..Self::new(resource)
        }
    }

    /// Dont care about content
    pub fn dont_care(resource: u64) -> Self {
        Self {
            load_op: LoadOp::DontCare,
            ..Self::new(resource)
        }
    }

    /// With clear color
    pub fn with_clear(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.clear_color = [r, g, b, a];
        self
    }
}

impl Default for ColorAttachment {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Depth attachment
#[derive(Clone, Debug)]
pub struct DepthAttachment {
    /// Resource ID
    pub resource: u64,
    /// Depth load operation
    pub depth_load_op: LoadOp,
    /// Depth store operation
    pub depth_store_op: StoreOp,
    /// Stencil load operation
    pub stencil_load_op: LoadOp,
    /// Stencil store operation
    pub stencil_store_op: StoreOp,
    /// Clear depth
    pub clear_depth: f32,
    /// Clear stencil
    pub clear_stencil: u32,
    /// Read only
    pub read_only: bool,
}

impl DepthAttachment {
    /// Creates attachment
    pub fn new(resource: u64) -> Self {
        Self {
            resource,
            depth_load_op: LoadOp::Clear,
            depth_store_op: StoreOp::Store,
            stencil_load_op: LoadOp::DontCare,
            stencil_store_op: StoreOp::DontCare,
            clear_depth: 1.0,
            clear_stencil: 0,
            read_only: false,
        }
    }

    /// Read only depth
    pub fn read_only(resource: u64) -> Self {
        Self {
            depth_load_op: LoadOp::Load,
            depth_store_op: StoreOp::None,
            read_only: true,
            ..Self::new(resource)
        }
    }

    /// With clear depth
    pub fn with_clear(mut self, depth: f32) -> Self {
        self.clear_depth = depth;
        self
    }
}

impl Default for DepthAttachment {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Load operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum LoadOp {
    /// Load existing content
    Load = 0,
    /// Clear to value
    #[default]
    Clear = 1,
    /// Don't care
    DontCare = 2,
}

/// Store operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum StoreOp {
    /// Store content
    #[default]
    Store = 0,
    /// Don't care
    DontCare = 1,
    /// None (don't store)
    None = 2,
}

// ============================================================================
// Blackboard
// ============================================================================

/// Blackboard entry
#[derive(Clone, Debug)]
pub struct BlackboardEntry {
    /// Key
    pub key: String,
    /// Value type
    pub value_type: BlackboardValueType,
    /// Data (packed)
    pub data: [u64; 4],
}

impl BlackboardEntry {
    /// Creates entry
    pub fn new(key: &str) -> Self {
        Self {
            key: String::from(key),
            value_type: BlackboardValueType::None,
            data: [0; 4],
        }
    }

    /// Resource entry
    pub fn resource(key: &str, handle: u64) -> Self {
        Self {
            key: String::from(key),
            value_type: BlackboardValueType::Resource,
            data: [handle, 0, 0, 0],
        }
    }

    /// Integer entry
    pub fn integer(key: &str, value: i64) -> Self {
        Self {
            key: String::from(key),
            value_type: BlackboardValueType::Integer,
            data: [value as u64, 0, 0, 0],
        }
    }

    /// Float entry
    pub fn float(key: &str, value: f64) -> Self {
        Self {
            key: String::from(key),
            value_type: BlackboardValueType::Float,
            data: [value.to_bits(), 0, 0, 0],
        }
    }
}

impl Default for BlackboardEntry {
    fn default() -> Self {
        Self::new("")
    }
}

/// Blackboard value type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BlackboardValueType {
    /// None
    #[default]
    None = 0,
    /// Resource handle
    Resource = 1,
    /// Integer
    Integer = 2,
    /// Float
    Float = 3,
    /// Vector4
    Vector4 = 4,
}

// ============================================================================
// Execution
// ============================================================================

/// Graph execution context
#[derive(Clone, Debug)]
pub struct ExecutionContext {
    /// Frame index
    pub frame_index: u64,
    /// Delta time
    pub delta_time: f32,
    /// Render width
    pub render_width: u32,
    /// Render height
    pub render_height: u32,
    /// Output width
    pub output_width: u32,
    /// Output height
    pub output_height: u32,
}

impl ExecutionContext {
    /// Creates context
    pub fn new(frame_index: u64, width: u32, height: u32) -> Self {
        Self {
            frame_index,
            delta_time: 0.016,
            render_width: width,
            render_height: height,
            output_width: width,
            output_height: height,
        }
    }

    /// With different output size
    pub fn with_output(mut self, width: u32, height: u32) -> Self {
        self.output_width = width;
        self.output_height = height;
        self
    }
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self::new(0, 1920, 1080)
    }
}

/// Pass execution info
#[derive(Clone, Debug, Default)]
pub struct PassExecutionInfo {
    /// Pass index
    pub index: u32,
    /// Name
    pub name: String,
    /// Start time (ns)
    pub start_time_ns: u64,
    /// End time (ns)
    pub end_time_ns: u64,
    /// GPU start time (ns)
    pub gpu_start_ns: u64,
    /// GPU end time (ns)
    pub gpu_end_ns: u64,
}

impl PassExecutionInfo {
    /// CPU duration (microseconds)
    pub fn cpu_duration_us(&self) -> f64 {
        (self.end_time_ns - self.start_time_ns) as f64 / 1000.0
    }

    /// GPU duration (microseconds)
    pub fn gpu_duration_us(&self) -> f64 {
        (self.gpu_end_ns - self.gpu_start_ns) as f64 / 1000.0
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Graph builder statistics
#[derive(Clone, Debug, Default)]
pub struct GraphBuilderStats {
    /// Passes built
    pub passes_built: u32,
    /// Resources created
    pub resources_created: u32,
    /// Reads declared
    pub reads: u32,
    /// Writes declared
    pub writes: u32,
    /// Build time (microseconds)
    pub build_time_us: u64,
}
