//! Render Queue Types for Lumina
//!
//! This module provides render command queue management
//! for efficient batching and submission.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Queue Handles
// ============================================================================

/// Render queue handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RenderQueueHandle(pub u64);

impl RenderQueueHandle {
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

impl Default for RenderQueueHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Command batch handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CommandBatchHandle(pub u64);

impl CommandBatchHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for CommandBatchHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Sort key handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SortKeyHandle(pub u64);

impl SortKeyHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for SortKeyHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Draw call handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DrawCallHandle(pub u64);

impl DrawCallHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for DrawCallHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Render Queue Creation
// ============================================================================

/// Render queue create info
#[derive(Clone, Debug)]
pub struct RenderQueueCreateInfo {
    /// Name
    pub name: String,
    /// Max draw calls
    pub max_draw_calls: u32,
    /// Max batches
    pub max_batches: u32,
    /// Queue type
    pub queue_type: RenderQueueType,
    /// Sort mode
    pub sort_mode: QueueSortMode,
    /// Batching mode
    pub batching_mode: BatchingMode,
    /// Features
    pub features: QueueFeatures,
}

impl RenderQueueCreateInfo {
    /// Creates new info
    pub fn new(max_draw_calls: u32) -> Self {
        Self {
            name: String::new(),
            max_draw_calls,
            max_batches: 1024,
            queue_type: RenderQueueType::Opaque,
            sort_mode: QueueSortMode::FrontToBack,
            batching_mode: BatchingMode::Automatic,
            features: QueueFeatures::empty(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With max batches
    pub fn with_max_batches(mut self, max: u32) -> Self {
        self.max_batches = max;
        self
    }

    /// With queue type
    pub fn with_queue_type(mut self, queue_type: RenderQueueType) -> Self {
        self.queue_type = queue_type;
        self
    }

    /// With sort mode
    pub fn with_sort_mode(mut self, mode: QueueSortMode) -> Self {
        self.sort_mode = mode;
        self
    }

    /// With batching mode
    pub fn with_batching(mut self, mode: BatchingMode) -> Self {
        self.batching_mode = mode;
        self
    }

    /// With features
    pub fn with_features(mut self, features: QueueFeatures) -> Self {
        self.features |= features;
        self
    }

    /// Opaque queue preset
    pub fn opaque() -> Self {
        Self::new(65536)
            .with_queue_type(RenderQueueType::Opaque)
            .with_sort_mode(QueueSortMode::FrontToBack)
    }

    /// Transparent queue preset
    pub fn transparent() -> Self {
        Self::new(16384)
            .with_queue_type(RenderQueueType::Transparent)
            .with_sort_mode(QueueSortMode::BackToFront)
    }

    /// Shadow queue preset
    pub fn shadow() -> Self {
        Self::new(65536)
            .with_queue_type(RenderQueueType::Shadow)
            .with_sort_mode(QueueSortMode::FrontToBack)
    }

    /// UI queue preset
    pub fn ui() -> Self {
        Self::new(8192)
            .with_queue_type(RenderQueueType::Ui)
            .with_sort_mode(QueueSortMode::SubmissionOrder)
    }

    /// Post-process queue preset
    pub fn post_process() -> Self {
        Self::new(256)
            .with_queue_type(RenderQueueType::PostProcess)
            .with_sort_mode(QueueSortMode::SubmissionOrder)
    }

    /// GPU-driven queue preset
    pub fn gpu_driven() -> Self {
        Self::new(1048576)
            .with_features(QueueFeatures::GPU_DRIVEN | QueueFeatures::INDIRECT_DRAW)
    }
}

impl Default for RenderQueueCreateInfo {
    fn default() -> Self {
        Self::opaque()
    }
}

/// Render queue type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum RenderQueueType {
    /// Opaque geometry
    #[default]
    Opaque = 0,
    /// Transparent geometry
    Transparent = 1,
    /// Shadow casting
    Shadow = 2,
    /// UI elements
    Ui = 3,
    /// Post-processing
    PostProcess = 4,
    /// Debug overlays
    Debug = 5,
    /// Custom
    Custom = 100,
}

impl RenderQueueType {
    /// Default sort order
    pub const fn default_sort_order(&self) -> i32 {
        match self {
            Self::Shadow => 0,
            Self::Opaque => 1000,
            Self::Transparent => 2000,
            Self::PostProcess => 3000,
            Self::Ui => 4000,
            Self::Debug => 5000,
            Self::Custom => 10000,
        }
    }
}

/// Queue sort mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum QueueSortMode {
    /// No sorting
    None = 0,
    /// Front to back (depth)
    #[default]
    FrontToBack = 1,
    /// Back to front (depth)
    BackToFront = 2,
    /// By material
    ByMaterial = 3,
    /// By mesh
    ByMesh = 4,
    /// By pipeline state
    ByPipelineState = 5,
    /// Submission order
    SubmissionOrder = 6,
    /// Custom sort key
    CustomKey = 7,
}

/// Batching mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BatchingMode {
    /// No batching
    None = 0,
    /// Automatic batching
    #[default]
    Automatic = 1,
    /// Static batching
    StaticBatching = 2,
    /// Dynamic batching
    DynamicBatching = 3,
    /// GPU instancing
    GpuInstancing = 4,
    /// Indirect draw
    IndirectDraw = 5,
}

bitflags::bitflags! {
    /// Queue features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct QueueFeatures: u32 {
        /// None
        const NONE = 0;
        /// GPU-driven
        const GPU_DRIVEN = 1 << 0;
        /// Indirect draw
        const INDIRECT_DRAW = 1 << 1;
        /// Multi-draw indirect
        const MULTI_DRAW_INDIRECT = 1 << 2;
        /// Bindless
        const BINDLESS = 1 << 3;
        /// Parallel sorting
        const PARALLEL_SORT = 1 << 4;
        /// Command caching
        const COMMAND_CACHING = 1 << 5;
        /// Automatic LOD
        const AUTOMATIC_LOD = 1 << 6;
    }
}

// ============================================================================
// Draw Commands
// ============================================================================

/// Draw command
#[derive(Clone, Debug, Default)]
pub struct DrawCommand {
    /// Mesh handle
    pub mesh: u64,
    /// Material handle
    pub material: u64,
    /// Pipeline handle
    pub pipeline: u64,
    /// Instance data
    pub instance_data: u64,
    /// Transform
    pub transform: DrawTransform,
    /// Vertex count
    pub vertex_count: u32,
    /// Index count
    pub index_count: u32,
    /// Instance count
    pub instance_count: u32,
    /// First vertex
    pub first_vertex: u32,
    /// First index
    pub first_index: u32,
    /// First instance
    pub first_instance: u32,
    /// Base vertex
    pub base_vertex: i32,
    /// Sort key
    pub sort_key: SortKey,
    /// Flags
    pub flags: DrawFlags,
}

impl DrawCommand {
    /// Creates new command
    pub fn new() -> Self {
        Self {
            instance_count: 1,
            ..Default::default()
        }
    }

    /// With mesh
    pub fn with_mesh(mut self, mesh: u64) -> Self {
        self.mesh = mesh;
        self
    }

    /// With material
    pub fn with_material(mut self, material: u64) -> Self {
        self.material = material;
        self
    }

    /// With pipeline
    pub fn with_pipeline(mut self, pipeline: u64) -> Self {
        self.pipeline = pipeline;
        self
    }

    /// With transform
    pub fn with_transform(mut self, transform: DrawTransform) -> Self {
        self.transform = transform;
        self
    }

    /// With indexed draw
    pub fn indexed(mut self, index_count: u32, first_index: u32, base_vertex: i32) -> Self {
        self.index_count = index_count;
        self.first_index = first_index;
        self.base_vertex = base_vertex;
        self.flags |= DrawFlags::INDEXED;
        self
    }

    /// With non-indexed draw
    pub fn non_indexed(mut self, vertex_count: u32, first_vertex: u32) -> Self {
        self.vertex_count = vertex_count;
        self.first_vertex = first_vertex;
        self
    }

    /// With instancing
    pub fn instanced(mut self, instance_count: u32, first_instance: u32) -> Self {
        self.instance_count = instance_count;
        self.first_instance = first_instance;
        self.flags |= DrawFlags::INSTANCED;
        self
    }

    /// With sort key
    pub fn with_sort_key(mut self, key: SortKey) -> Self {
        self.sort_key = key;
        self
    }

    /// With flags
    pub fn with_flags(mut self, flags: DrawFlags) -> Self {
        self.flags |= flags;
        self
    }

    /// Is indexed
    pub fn is_indexed(&self) -> bool {
        self.flags.contains(DrawFlags::INDEXED)
    }

    /// Is instanced
    pub fn is_instanced(&self) -> bool {
        self.instance_count > 1 || self.flags.contains(DrawFlags::INSTANCED)
    }
}

/// Draw transform
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DrawTransform {
    /// Model matrix row 0
    pub row0: [f32; 4],
    /// Model matrix row 1
    pub row1: [f32; 4],
    /// Model matrix row 2
    pub row2: [f32; 4],
    /// Position
    pub position: [f32; 4],
}

impl DrawTransform {
    /// Identity
    pub const fn identity() -> Self {
        Self {
            row0: [1.0, 0.0, 0.0, 0.0],
            row1: [0.0, 1.0, 0.0, 0.0],
            row2: [0.0, 0.0, 1.0, 0.0],
            position: [0.0, 0.0, 0.0, 1.0],
        }
    }

    /// From position
    pub fn from_position(x: f32, y: f32, z: f32) -> Self {
        Self {
            position: [x, y, z, 1.0],
            ..Self::identity()
        }
    }
}

bitflags::bitflags! {
    /// Draw flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct DrawFlags: u32 {
        /// None
        const NONE = 0;
        /// Indexed draw
        const INDEXED = 1 << 0;
        /// Instanced draw
        const INSTANCED = 1 << 1;
        /// Indirect draw
        const INDIRECT = 1 << 2;
        /// Two-sided
        const TWO_SIDED = 1 << 3;
        /// Wireframe
        const WIREFRAME = 1 << 4;
        /// Skip culling
        const SKIP_CULLING = 1 << 5;
        /// Cast shadows
        const CAST_SHADOWS = 1 << 6;
    }
}

// ============================================================================
// Sort Keys
// ============================================================================

/// Sort key
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SortKey {
    /// Primary key
    pub primary: u64,
    /// Secondary key
    pub secondary: u64,
}

impl SortKey {
    /// Creates new key
    pub const fn new(primary: u64, secondary: u64) -> Self {
        Self { primary, secondary }
    }

    /// From depth
    pub fn from_depth(depth: f32, material: u32, mesh: u32) -> Self {
        let depth_bits = depth.to_bits() as u64;
        Self {
            primary: depth_bits,
            secondary: ((material as u64) << 32) | (mesh as u64),
        }
    }

    /// From material (state-based sorting)
    pub fn from_material(material: u32, mesh: u32, depth: f32) -> Self {
        let depth_bits = depth.to_bits() as u64;
        Self {
            primary: ((material as u64) << 32) | (mesh as u64),
            secondary: depth_bits,
        }
    }

    /// Combined key for comparison
    pub fn combined(&self) -> u128 {
        ((self.primary as u128) << 64) | (self.secondary as u128)
    }
}

impl PartialEq for SortKey {
    fn eq(&self, other: &Self) -> bool {
        self.primary == other.primary && self.secondary == other.secondary
    }
}

impl Eq for SortKey {}

impl PartialOrd for SortKey {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SortKey {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        match self.primary.cmp(&other.primary) {
            core::cmp::Ordering::Equal => self.secondary.cmp(&other.secondary),
            ord => ord,
        }
    }
}

// ============================================================================
// Command Batches
// ============================================================================

/// Command batch create info
#[derive(Clone, Debug)]
pub struct CommandBatchCreateInfo {
    /// Name
    pub name: String,
    /// Max commands
    pub max_commands: u32,
    /// Pipeline
    pub pipeline: u64,
    /// Material
    pub material: u64,
    /// Flags
    pub flags: BatchFlags,
}

impl CommandBatchCreateInfo {
    /// Creates new info
    pub fn new(max_commands: u32) -> Self {
        Self {
            name: String::new(),
            max_commands,
            pipeline: 0,
            material: 0,
            flags: BatchFlags::empty(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With pipeline
    pub fn with_pipeline(mut self, pipeline: u64) -> Self {
        self.pipeline = pipeline;
        self
    }

    /// With material
    pub fn with_material(mut self, material: u64) -> Self {
        self.material = material;
        self
    }

    /// With flags
    pub fn with_flags(mut self, flags: BatchFlags) -> Self {
        self.flags |= flags;
        self
    }
}

impl Default for CommandBatchCreateInfo {
    fn default() -> Self {
        Self::new(1024)
    }
}

bitflags::bitflags! {
    /// Batch flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct BatchFlags: u32 {
        /// None
        const NONE = 0;
        /// Sorted
        const SORTED = 1 << 0;
        /// Merged
        const MERGED = 1 << 1;
        /// Indirect
        const INDIRECT = 1 << 2;
        /// Instanced
        const INSTANCED = 1 << 3;
    }
}

/// Command batch
#[derive(Clone, Debug, Default)]
pub struct CommandBatch {
    /// Batch handle
    pub handle: CommandBatchHandle,
    /// Draw commands
    pub commands: Vec<DrawCommand>,
    /// Pipeline
    pub pipeline: u64,
    /// Material
    pub material: u64,
    /// Total vertices
    pub total_vertices: u32,
    /// Total indices
    pub total_indices: u32,
    /// Total instances
    pub total_instances: u32,
    /// Flags
    pub flags: BatchFlags,
}

impl CommandBatch {
    /// Creates new batch
    pub fn new() -> Self {
        Self::default()
    }

    /// Add command
    pub fn add_command(&mut self, command: DrawCommand) {
        if command.is_indexed() {
            self.total_indices += command.index_count * command.instance_count;
        } else {
            self.total_vertices += command.vertex_count * command.instance_count;
        }
        self.total_instances += command.instance_count;
        self.commands.push(command);
    }

    /// Command count
    pub fn command_count(&self) -> usize {
        self.commands.len()
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Clear
    pub fn clear(&mut self) {
        self.commands.clear();
        self.total_vertices = 0;
        self.total_indices = 0;
        self.total_instances = 0;
    }

    /// Sort by key
    pub fn sort(&mut self) {
        self.commands.sort_by_key(|c| c.sort_key);
        self.flags |= BatchFlags::SORTED;
    }
}

// ============================================================================
// GPU Data Structures
// ============================================================================

/// GPU draw indirect command
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuDrawIndirectCommand {
    /// Vertex count
    pub vertex_count: u32,
    /// Instance count
    pub instance_count: u32,
    /// First vertex
    pub first_vertex: u32,
    /// First instance
    pub first_instance: u32,
}

/// GPU draw indexed indirect command
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuDrawIndexedIndirectCommand {
    /// Index count
    pub index_count: u32,
    /// Instance count
    pub instance_count: u32,
    /// First index
    pub first_index: u32,
    /// Base vertex
    pub base_vertex: i32,
    /// First instance
    pub first_instance: u32,
}

/// GPU draw count indirect command
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuDrawCountIndirectCommand {
    /// Max draw count
    pub max_draw_count: u32,
    /// Stride
    pub stride: u32,
}

/// GPU mesh draw command
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuMeshDrawCommand {
    /// Object index
    pub object_index: u32,
    /// Mesh index
    pub mesh_index: u32,
    /// Material index
    pub material_index: u32,
    /// LOD level
    pub lod_level: u32,
    /// Vertex offset
    pub vertex_offset: u32,
    /// Index offset
    pub index_offset: u32,
    /// Index count
    pub index_count: u32,
    /// Instance count
    pub instance_count: u32,
}

// ============================================================================
// Statistics
// ============================================================================

/// Render queue statistics
#[derive(Clone, Debug, Default)]
pub struct RenderQueueStats {
    /// Total draw calls
    pub total_draw_calls: u32,
    /// Batched draw calls
    pub batched_draw_calls: u32,
    /// Total batches
    pub total_batches: u32,
    /// Total vertices
    pub total_vertices: u64,
    /// Total indices
    pub total_indices: u64,
    /// Total instances
    pub total_instances: u32,
    /// State changes
    pub state_changes: u32,
    /// Sort time (ms)
    pub sort_time_ms: f32,
    /// Batch time (ms)
    pub batch_time_ms: f32,
    /// Submit time (ms)
    pub submit_time_ms: f32,
}

impl RenderQueueStats {
    /// Batching efficiency
    pub fn batching_efficiency(&self) -> f32 {
        if self.total_draw_calls == 0 {
            return 1.0;
        }
        1.0 - (self.batched_draw_calls as f32 / self.total_draw_calls as f32)
    }

    /// Average batch size
    pub fn average_batch_size(&self) -> f32 {
        if self.total_batches == 0 {
            return 0.0;
        }
        self.total_draw_calls as f32 / self.total_batches as f32
    }

    /// Total processing time
    pub fn total_time_ms(&self) -> f32 {
        self.sort_time_ms + self.batch_time_ms + self.submit_time_ms
    }
}
