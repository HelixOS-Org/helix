//! GPU Batch Rendering System for Lumina
//!
//! This module provides GPU-accelerated draw call batching including
//! automatic instancing, state sorting, and indirect rendering.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Batch System Handles
// ============================================================================

/// GPU batch system handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuBatchSystemHandle(pub u64);

impl GpuBatchSystemHandle {
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

impl Default for GpuBatchSystemHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Batch handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BatchHandle(pub u64);

impl BatchHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Is null
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for BatchHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Batch group handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BatchGroupHandle(pub u64);

impl BatchGroupHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for BatchGroupHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Draw command handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DrawCommandHandle(pub u64);

impl DrawCommandHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for DrawCommandHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Batch System Creation
// ============================================================================

/// GPU batch system create info
#[derive(Clone, Debug)]
pub struct GpuBatchSystemCreateInfo {
    /// Name
    pub name: String,
    /// Max batches
    pub max_batches: u32,
    /// Max draw commands
    pub max_draw_commands: u32,
    /// Max instances per batch
    pub max_instances_per_batch: u32,
    /// Features
    pub features: BatchFeatures,
    /// Sort mode
    pub sort_mode: BatchSortMode,
}

impl GpuBatchSystemCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            max_batches: 4096,
            max_draw_commands: 65536,
            max_instances_per_batch: 1024,
            features: BatchFeatures::all(),
            sort_mode: BatchSortMode::StateFirst,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With max batches
    pub fn with_max_batches(mut self, count: u32) -> Self {
        self.max_batches = count;
        self
    }

    /// With max draw commands
    pub fn with_max_commands(mut self, count: u32) -> Self {
        self.max_draw_commands = count;
        self
    }

    /// With features
    pub fn with_features(mut self, features: BatchFeatures) -> Self {
        self.features |= features;
        self
    }

    /// With sort mode
    pub fn with_sort_mode(mut self, mode: BatchSortMode) -> Self {
        self.sort_mode = mode;
        self
    }

    /// Standard preset
    pub fn standard() -> Self {
        Self::new()
    }

    /// High capacity preset
    pub fn high_capacity() -> Self {
        Self::new()
            .with_max_batches(16384)
            .with_max_commands(262144)
    }

    /// Mobile preset
    pub fn mobile() -> Self {
        Self::new()
            .with_max_batches(1024)
            .with_max_commands(16384)
            .with_features(BatchFeatures::BASIC)
    }
}

impl Default for GpuBatchSystemCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Batch features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct BatchFeatures: u32 {
        /// None
        const NONE = 0;
        /// Automatic instancing
        const AUTO_INSTANCE = 1 << 0;
        /// State sorting
        const STATE_SORT = 1 << 1;
        /// Indirect rendering
        const INDIRECT = 1 << 2;
        /// GPU culling
        const GPU_CULLING = 1 << 3;
        /// Batch merging
        const MERGING = 1 << 4;
        /// Dynamic batching
        const DYNAMIC = 1 << 5;
        /// Static batching
        const STATIC = 1 << 6;
        /// Multi-draw indirect
        const MULTI_DRAW = 1 << 7;
        /// Basic features
        const BASIC = Self::AUTO_INSTANCE.bits() | Self::STATE_SORT.bits();
        /// All
        const ALL = 0xFF;
    }
}

impl Default for BatchFeatures {
    fn default() -> Self {
        Self::all()
    }
}

/// Batch sort mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BatchSortMode {
    /// No sorting
    None        = 0,
    /// Sort by state first
    #[default]
    StateFirst  = 1,
    /// Sort by distance (front to back)
    FrontToBack = 2,
    /// Sort by distance (back to front)
    BackToFront = 3,
    /// Sort by material
    Material    = 4,
    /// Custom sort key
    Custom      = 5,
}

// ============================================================================
// Batch Creation
// ============================================================================

/// Batch create info
#[derive(Clone, Debug)]
pub struct BatchCreateInfo {
    /// Name
    pub name: String,
    /// Batch type
    pub batch_type: BatchType,
    /// Material handle
    pub material: u64,
    /// Mesh handle
    pub mesh: u64,
    /// Render state
    pub render_state: BatchRenderState,
    /// Batch group
    pub group: BatchGroupHandle,
}

impl BatchCreateInfo {
    /// Creates new info
    pub fn new(mesh: u64, material: u64) -> Self {
        Self {
            name: String::new(),
            batch_type: BatchType::Static,
            material,
            mesh,
            render_state: BatchRenderState::default(),
            group: BatchGroupHandle::NULL,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With batch type
    pub fn with_type(mut self, batch_type: BatchType) -> Self {
        self.batch_type = batch_type;
        self
    }

    /// With render state
    pub fn with_state(mut self, state: BatchRenderState) -> Self {
        self.render_state = state;
        self
    }

    /// In group
    pub fn in_group(mut self, group: BatchGroupHandle) -> Self {
        self.group = group;
        self
    }

    /// Static batch
    pub fn static_batch(mesh: u64, material: u64) -> Self {
        Self::new(mesh, material).with_type(BatchType::Static)
    }

    /// Dynamic batch
    pub fn dynamic_batch(mesh: u64, material: u64) -> Self {
        Self::new(mesh, material).with_type(BatchType::Dynamic)
    }

    /// Instanced batch
    pub fn instanced(mesh: u64, material: u64) -> Self {
        Self::new(mesh, material).with_type(BatchType::Instanced)
    }
}

impl Default for BatchCreateInfo {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

/// Batch type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BatchType {
    /// Static (geometry baked)
    #[default]
    Static    = 0,
    /// Dynamic (updated each frame)
    Dynamic   = 1,
    /// Instanced (GPU instancing)
    Instanced = 2,
    /// Indirect (GPU-driven)
    Indirect  = 3,
}

/// Batch render state
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BatchRenderState {
    /// Depth test
    pub depth_test: bool,
    /// Depth write
    pub depth_write: bool,
    /// Cull mode
    pub cull_mode: CullMode,
    /// Blend mode
    pub blend_mode: BlendMode,
    /// Stencil settings
    pub stencil: StencilSettings,
    /// Render layer
    pub render_layer: u32,
    /// Sort priority
    pub sort_priority: i32,
}

impl BatchRenderState {
    /// Creates default state
    pub const fn new() -> Self {
        Self {
            depth_test: true,
            depth_write: true,
            cull_mode: CullMode::Back,
            blend_mode: BlendMode::Opaque,
            stencil: StencilSettings::disabled(),
            render_layer: 0,
            sort_priority: 0,
        }
    }

    /// With depth
    pub const fn with_depth(mut self, test: bool, write: bool) -> Self {
        self.depth_test = test;
        self.depth_write = write;
        self
    }

    /// With cull mode
    pub const fn with_cull(mut self, mode: CullMode) -> Self {
        self.cull_mode = mode;
        self
    }

    /// With blend mode
    pub const fn with_blend(mut self, mode: BlendMode) -> Self {
        self.blend_mode = mode;
        self
    }

    /// With layer
    pub const fn with_layer(mut self, layer: u32) -> Self {
        self.render_layer = layer;
        self
    }

    /// Opaque preset
    pub const fn opaque() -> Self {
        Self::new()
    }

    /// Transparent preset
    pub const fn transparent() -> Self {
        Self::new()
            .with_blend(BlendMode::AlphaBlend)
            .with_depth(true, false)
    }

    /// Additive preset
    pub const fn additive() -> Self {
        Self::new()
            .with_blend(BlendMode::Additive)
            .with_depth(true, false)
    }

    /// UI preset
    pub const fn ui() -> Self {
        Self::new()
            .with_depth(false, false)
            .with_cull(CullMode::None)
            .with_blend(BlendMode::AlphaBlend)
    }
}

impl Default for BatchRenderState {
    fn default() -> Self {
        Self::new()
    }
}

/// Cull mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CullMode {
    /// No culling
    None  = 0,
    /// Cull front faces
    Front = 1,
    /// Cull back faces
    #[default]
    Back  = 2,
}

/// Blend mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BlendMode {
    /// Opaque
    #[default]
    Opaque        = 0,
    /// Alpha blend
    AlphaBlend    = 1,
    /// Additive
    Additive      = 2,
    /// Multiply
    Multiply      = 3,
    /// Premultiplied alpha
    Premultiplied = 4,
    /// Custom
    Custom        = 5,
}

/// Stencil settings
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct StencilSettings {
    /// Enabled
    pub enabled: bool,
    /// Reference value
    pub reference: u8,
    /// Read mask
    pub read_mask: u8,
    /// Write mask
    pub write_mask: u8,
    /// Compare func
    pub compare: StencilCompare,
    /// Pass op
    pub pass_op: StencilOp,
    /// Fail op
    pub fail_op: StencilOp,
    /// Depth fail op
    pub depth_fail_op: StencilOp,
}

impl StencilSettings {
    /// Disabled
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            reference: 0,
            read_mask: 0xFF,
            write_mask: 0xFF,
            compare: StencilCompare::Always,
            pass_op: StencilOp::Keep,
            fail_op: StencilOp::Keep,
            depth_fail_op: StencilOp::Keep,
        }
    }

    /// Write preset
    pub const fn write(value: u8) -> Self {
        Self {
            enabled: true,
            reference: value,
            read_mask: 0xFF,
            write_mask: 0xFF,
            compare: StencilCompare::Always,
            pass_op: StencilOp::Replace,
            fail_op: StencilOp::Keep,
            depth_fail_op: StencilOp::Keep,
        }
    }

    /// Test preset
    pub const fn test(value: u8) -> Self {
        Self {
            enabled: true,
            reference: value,
            read_mask: 0xFF,
            write_mask: 0x00,
            compare: StencilCompare::Equal,
            pass_op: StencilOp::Keep,
            fail_op: StencilOp::Keep,
            depth_fail_op: StencilOp::Keep,
        }
    }
}

impl Default for StencilSettings {
    fn default() -> Self {
        Self::disabled()
    }
}

/// Stencil compare
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum StencilCompare {
    /// Never
    Never        = 0,
    /// Less
    Less         = 1,
    /// Equal
    Equal        = 2,
    /// Less or equal
    LessEqual    = 3,
    /// Greater
    Greater      = 4,
    /// Not equal
    NotEqual     = 5,
    /// Greater or equal
    GreaterEqual = 6,
    /// Always
    #[default]
    Always       = 7,
}

/// Stencil operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum StencilOp {
    /// Keep
    #[default]
    Keep      = 0,
    /// Zero
    Zero      = 1,
    /// Replace
    Replace   = 2,
    /// Increment clamp
    IncrClamp = 3,
    /// Decrement clamp
    DecrClamp = 4,
    /// Invert
    Invert    = 5,
    /// Increment wrap
    IncrWrap  = 6,
    /// Decrement wrap
    DecrWrap  = 7,
}

// ============================================================================
// Batch Group
// ============================================================================

/// Batch group create info
#[derive(Clone, Debug)]
pub struct BatchGroupCreateInfo {
    /// Name
    pub name: String,
    /// Sort mode
    pub sort_mode: BatchSortMode,
    /// Render order
    pub render_order: i32,
    /// Camera mask
    pub camera_mask: u32,
}

impl BatchGroupCreateInfo {
    /// Creates new info
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            sort_mode: BatchSortMode::StateFirst,
            render_order: 0,
            camera_mask: 0xFFFFFFFF,
        }
    }

    /// With sort mode
    pub fn with_sort(mut self, mode: BatchSortMode) -> Self {
        self.sort_mode = mode;
        self
    }

    /// With render order
    pub fn with_order(mut self, order: i32) -> Self {
        self.render_order = order;
        self
    }

    /// Opaque group preset
    pub fn opaque() -> Self {
        Self::new("Opaque")
            .with_sort(BatchSortMode::FrontToBack)
            .with_order(0)
    }

    /// Transparent group preset
    pub fn transparent() -> Self {
        Self::new("Transparent")
            .with_sort(BatchSortMode::BackToFront)
            .with_order(100)
    }

    /// UI group preset
    pub fn ui() -> Self {
        Self::new("UI")
            .with_sort(BatchSortMode::None)
            .with_order(1000)
    }
}

impl Default for BatchGroupCreateInfo {
    fn default() -> Self {
        Self::new("Group")
    }
}

// ============================================================================
// Draw Commands
// ============================================================================

/// Draw command
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DrawCommand {
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

impl DrawCommand {
    /// Creates new command
    pub const fn new(index_count: u32, instance_count: u32) -> Self {
        Self {
            index_count,
            instance_count,
            first_index: 0,
            vertex_offset: 0,
            first_instance: 0,
        }
    }

    /// With first index
    pub const fn with_first_index(mut self, index: u32) -> Self {
        self.first_index = index;
        self
    }

    /// With vertex offset
    pub const fn with_vertex_offset(mut self, offset: i32) -> Self {
        self.vertex_offset = offset;
        self
    }

    /// With first instance
    pub const fn with_first_instance(mut self, instance: u32) -> Self {
        self.first_instance = instance;
        self
    }

    /// Single draw
    pub const fn single(index_count: u32) -> Self {
        Self::new(index_count, 1)
    }

    /// Instanced draw
    pub const fn instanced(index_count: u32, instance_count: u32) -> Self {
        Self::new(index_count, instance_count)
    }
}

impl Default for DrawCommand {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

/// Draw command indirect (for indirect rendering)
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DrawCommandIndirect {
    /// Draw command
    pub command: DrawCommand,
    /// Material ID
    pub material_id: u32,
    /// Mesh ID
    pub mesh_id: u32,
    /// Sort key
    pub sort_key: u64,
}

impl DrawCommandIndirect {
    /// Creates new indirect command
    pub const fn new(command: DrawCommand, material_id: u32, mesh_id: u32) -> Self {
        Self {
            command,
            material_id,
            mesh_id,
            sort_key: 0,
        }
    }

    /// With sort key
    pub const fn with_sort_key(mut self, key: u64) -> Self {
        self.sort_key = key;
        self
    }
}

impl Default for DrawCommandIndirect {
    fn default() -> Self {
        Self::new(DrawCommand::default(), 0, 0)
    }
}

// ============================================================================
// Instance Data
// ============================================================================

/// Instance data
#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
pub struct InstanceData {
    /// Model matrix row 0
    pub model_row0: [f32; 4],
    /// Model matrix row 1
    pub model_row1: [f32; 4],
    /// Model matrix row 2
    pub model_row2: [f32; 4],
    /// Custom data
    pub custom_data: [f32; 4],
}

impl InstanceData {
    /// Creates new instance data
    pub const fn new() -> Self {
        Self {
            model_row0: [1.0, 0.0, 0.0, 0.0],
            model_row1: [0.0, 1.0, 0.0, 0.0],
            model_row2: [0.0, 0.0, 1.0, 0.0],
            custom_data: [0.0; 4],
        }
    }

    /// From transform
    pub fn from_transform(position: [f32; 3], rotation: [f32; 4], scale: [f32; 3]) -> Self {
        // Simplified - actual impl would compute full matrix from quaternion
        Self {
            model_row0: [scale[0], 0.0, 0.0, position[0]],
            model_row1: [0.0, scale[1], 0.0, position[1]],
            model_row2: [0.0, 0.0, scale[2], position[2]],
            custom_data: [rotation[0], rotation[1], rotation[2], rotation[3]],
        }
    }

    /// At position
    pub fn at_position(x: f32, y: f32, z: f32) -> Self {
        Self {
            model_row0: [1.0, 0.0, 0.0, x],
            model_row1: [0.0, 1.0, 0.0, y],
            model_row2: [0.0, 0.0, 1.0, z],
            custom_data: [0.0; 4],
        }
    }

    /// With custom data
    pub const fn with_custom(mut self, data: [f32; 4]) -> Self {
        self.custom_data = data;
        self
    }
}

impl Default for InstanceData {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Batch Submission
// ============================================================================

/// Batch submission info
#[derive(Clone, Debug)]
pub struct BatchSubmission {
    /// Batches to render
    pub batches: Vec<BatchHandle>,
    /// Override camera
    pub camera_override: Option<BatchCamera>,
    /// Render target
    pub render_target: Option<u64>,
    /// Clear flags
    pub clear_flags: ClearFlags,
}

impl BatchSubmission {
    /// Creates new submission
    pub fn new() -> Self {
        Self {
            batches: Vec::new(),
            camera_override: None,
            render_target: None,
            clear_flags: ClearFlags::empty(),
        }
    }

    /// Add batch
    pub fn add_batch(mut self, batch: BatchHandle) -> Self {
        self.batches.push(batch);
        self
    }

    /// With camera
    pub fn with_camera(mut self, camera: BatchCamera) -> Self {
        self.camera_override = Some(camera);
        self
    }

    /// With render target
    pub fn with_target(mut self, target: u64) -> Self {
        self.render_target = Some(target);
        self
    }

    /// With clear
    pub fn with_clear(mut self, flags: ClearFlags) -> Self {
        self.clear_flags = flags;
        self
    }
}

impl Default for BatchSubmission {
    fn default() -> Self {
        Self::new()
    }
}

/// Batch camera
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BatchCamera {
    /// View matrix
    pub view: [[f32; 4]; 4],
    /// Projection matrix
    pub projection: [[f32; 4]; 4],
    /// Position
    pub position: [f32; 3],
    /// Near plane
    pub near: f32,
    /// Far plane
    pub far: f32,
}

impl Default for BatchCamera {
    fn default() -> Self {
        Self {
            view: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            projection: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            position: [0.0; 3],
            near: 0.1,
            far: 1000.0,
        }
    }
}

bitflags::bitflags! {
    /// Clear flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct ClearFlags: u32 {
        /// None
        const NONE = 0;
        /// Clear color
        const COLOR = 1 << 0;
        /// Clear depth
        const DEPTH = 1 << 1;
        /// Clear stencil
        const STENCIL = 1 << 2;
        /// Clear all
        const ALL = Self::COLOR.bits() | Self::DEPTH.bits() | Self::STENCIL.bits();
    }
}

// ============================================================================
// GPU Parameters
// ============================================================================

/// GPU batch data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C, align(16))]
pub struct GpuBatchData {
    /// Batch ID
    pub batch_id: u32,
    /// Material ID
    pub material_id: u32,
    /// Mesh ID
    pub mesh_id: u32,
    /// Instance offset
    pub instance_offset: u32,
    /// Instance count
    pub instance_count: u32,
    /// Sort key
    pub sort_key: u32,
    /// Flags
    pub flags: u32,
    /// Pad
    pub _pad: u32,
}

/// GPU batch constants
#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
pub struct GpuBatchConstants {
    /// View matrix
    pub view: [[f32; 4]; 4],
    /// Projection matrix
    pub projection: [[f32; 4]; 4],
    /// View projection matrix
    pub view_projection: [[f32; 4]; 4],
    /// Camera position
    pub camera_position: [f32; 3],
    /// Time
    pub time: f32,
    /// Screen size
    pub screen_size: [f32; 2],
    /// Near/far planes
    pub near_far: [f32; 2],
}

impl Default for GpuBatchConstants {
    fn default() -> Self {
        Self {
            view: [[0.0; 4]; 4],
            projection: [[0.0; 4]; 4],
            view_projection: [[0.0; 4]; 4],
            camera_position: [0.0; 3],
            time: 0.0,
            screen_size: [1920.0, 1080.0],
            near_far: [0.1, 1000.0],
        }
    }
}

/// GPU draw indirect command
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuDrawIndirectCommand {
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

// ============================================================================
// Batch Statistics
// ============================================================================

/// Batch system statistics
#[derive(Clone, Debug, Default)]
pub struct GpuBatchStats {
    /// Total batches
    pub total_batches: u32,
    /// Active batches
    pub active_batches: u32,
    /// Draw calls
    pub draw_calls: u32,
    /// Draw calls before batching
    pub draw_calls_before: u32,
    /// Instances rendered
    pub instances_rendered: u32,
    /// Triangles rendered
    pub triangles_rendered: u64,
    /// State changes
    pub state_changes: u32,
    /// Material changes
    pub material_changes: u32,
    /// Batching time (ms)
    pub batching_time_ms: f32,
    /// Render time (ms)
    pub render_time_ms: f32,
}

impl GpuBatchStats {
    /// Batch efficiency
    pub fn batch_efficiency(&self) -> f32 {
        if self.draw_calls_before > 0 {
            1.0 - (self.draw_calls as f32 / self.draw_calls_before as f32)
        } else {
            0.0
        }
    }

    /// Average instances per batch
    pub fn avg_instances_per_batch(&self) -> f32 {
        if self.active_batches > 0 {
            self.instances_rendered as f32 / self.active_batches as f32
        } else {
            0.0
        }
    }

    /// Draw call reduction ratio
    pub fn reduction_ratio(&self) -> f32 {
        if self.draw_calls > 0 {
            self.draw_calls_before as f32 / self.draw_calls as f32
        } else {
            1.0
        }
    }

    /// Triangles in millions
    pub fn triangles_millions(&self) -> f32 {
        self.triangles_rendered as f32 / 1_000_000.0
    }
}
