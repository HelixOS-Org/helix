//! Mesh Rendering Types for Lumina
//!
//! This module provides mesh rendering infrastructure
//! for efficient batch rendering and mesh pipelines.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Mesh Renderer Handles
// ============================================================================

/// Mesh renderer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MeshRendererHandle(pub u64);

impl MeshRendererHandle {
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

impl Default for MeshRendererHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Mesh batch handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MeshBatchHandle(pub u64);

impl MeshBatchHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for MeshBatchHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Draw command buffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DrawCommandBufferHandle(pub u64);

impl DrawCommandBufferHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for DrawCommandBufferHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Mesh instance buffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MeshInstanceBufferHandle(pub u64);

impl MeshInstanceBufferHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for MeshInstanceBufferHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Mesh Renderer
// ============================================================================

/// Mesh renderer create info
#[derive(Clone, Debug)]
pub struct MeshRendererCreateInfo {
    /// Name
    pub name: String,
    /// Renderer type
    pub renderer_type: MeshRendererType,
    /// Max meshes
    pub max_meshes: u32,
    /// Max instances
    pub max_instances: u32,
    /// Max draw commands
    pub max_draw_commands: u32,
    /// Features
    pub features: MeshRendererFeatures,
    /// Culling mode
    pub culling_mode: MeshCullingMode,
}

impl MeshRendererCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            renderer_type: MeshRendererType::Forward,
            max_meshes: 10000,
            max_instances: 100000,
            max_draw_commands: 50000,
            features: MeshRendererFeatures::empty(),
            culling_mode: MeshCullingMode::Frustum,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With type
    pub fn with_type(mut self, renderer_type: MeshRendererType) -> Self {
        self.renderer_type = renderer_type;
        self
    }

    /// With max meshes
    pub fn with_max_meshes(mut self, max: u32) -> Self {
        self.max_meshes = max;
        self
    }

    /// With max instances
    pub fn with_max_instances(mut self, max: u32) -> Self {
        self.max_instances = max;
        self
    }

    /// With features
    pub fn with_features(mut self, features: MeshRendererFeatures) -> Self {
        self.features |= features;
        self
    }

    /// With culling mode
    pub fn with_culling(mut self, mode: MeshCullingMode) -> Self {
        self.culling_mode = mode;
        self
    }

    /// Forward renderer preset
    pub fn forward() -> Self {
        Self::new()
            .with_type(MeshRendererType::Forward)
            .with_features(MeshRendererFeatures::BATCHING)
    }

    /// Deferred renderer preset
    pub fn deferred() -> Self {
        Self::new()
            .with_type(MeshRendererType::Deferred)
            .with_features(MeshRendererFeatures::BATCHING | MeshRendererFeatures::GPU_CULLING)
    }

    /// GPU-driven renderer preset
    pub fn gpu_driven() -> Self {
        Self::new()
            .with_type(MeshRendererType::GpuDriven)
            .with_features(
                MeshRendererFeatures::GPU_CULLING |
                MeshRendererFeatures::INDIRECT_DRAW |
                MeshRendererFeatures::MULTI_DRAW_INDIRECT |
                MeshRendererFeatures::PERSISTENT_MAPPING
            )
            .with_max_instances(1_000_000)
    }

    /// Visibility buffer renderer preset
    pub fn visibility_buffer() -> Self {
        Self::new()
            .with_type(MeshRendererType::VisibilityBuffer)
            .with_features(
                MeshRendererFeatures::GPU_CULLING |
                MeshRendererFeatures::MESHLET_RENDERING |
                MeshRendererFeatures::BINDLESS
            )
    }
}

impl Default for MeshRendererCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Mesh renderer type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum MeshRendererType {
    /// Forward rendering
    #[default]
    Forward = 0,
    /// Deferred rendering
    Deferred = 1,
    /// Forward+ rendering
    ForwardPlus = 2,
    /// GPU-driven rendering
    GpuDriven = 3,
    /// Visibility buffer rendering
    VisibilityBuffer = 4,
    /// Custom
    Custom = 5,
}

bitflags::bitflags! {
    /// Mesh renderer features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct MeshRendererFeatures: u32 {
        /// None
        const NONE = 0;
        /// Automatic batching
        const BATCHING = 1 << 0;
        /// GPU-based culling
        const GPU_CULLING = 1 << 1;
        /// Indirect draw
        const INDIRECT_DRAW = 1 << 2;
        /// Multi-draw indirect
        const MULTI_DRAW_INDIRECT = 1 << 3;
        /// Persistent buffer mapping
        const PERSISTENT_MAPPING = 1 << 4;
        /// Meshlet rendering
        const MESHLET_RENDERING = 1 << 5;
        /// Bindless resources
        const BINDLESS = 1 << 6;
        /// Instancing
        const INSTANCING = 1 << 7;
        /// LOD selection
        const LOD_SELECTION = 1 << 8;
        /// Sorting
        const SORTING = 1 << 9;
        /// Occlusion culling
        const OCCLUSION_CULLING = 1 << 10;
    }
}

/// Mesh culling mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum MeshCullingMode {
    /// No culling
    None = 0,
    /// Frustum culling only
    #[default]
    Frustum = 1,
    /// Frustum + occlusion culling
    FrustumOcclusion = 2,
    /// Hierarchical culling
    Hierarchical = 3,
    /// GPU-driven culling
    GpuDriven = 4,
}

// ============================================================================
// Mesh Batch
// ============================================================================

/// Mesh batch create info
#[derive(Clone, Debug)]
pub struct MeshBatchCreateInfo {
    /// Name
    pub name: String,
    /// Material
    pub material: u64,
    /// Mesh
    pub mesh: u64,
    /// Initial capacity
    pub initial_capacity: u32,
    /// Max instances
    pub max_instances: u32,
    /// Dynamic
    pub dynamic: bool,
    /// Sorting mode
    pub sorting: BatchSortingMode,
}

impl MeshBatchCreateInfo {
    /// Creates new info
    pub fn new(mesh: u64, material: u64) -> Self {
        Self {
            name: String::new(),
            material,
            mesh,
            initial_capacity: 100,
            max_instances: 10000,
            dynamic: false,
            sorting: BatchSortingMode::None,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With initial capacity
    pub fn with_capacity(mut self, capacity: u32) -> Self {
        self.initial_capacity = capacity;
        self
    }

    /// With max instances
    pub fn with_max_instances(mut self, max: u32) -> Self {
        self.max_instances = max;
        self
    }

    /// Dynamic batch
    pub fn dynamic(mut self) -> Self {
        self.dynamic = true;
        self
    }

    /// With sorting
    pub fn with_sorting(mut self, mode: BatchSortingMode) -> Self {
        self.sorting = mode;
        self
    }

    /// Static batch (unchanging)
    pub fn static_batch(mesh: u64, material: u64, count: u32) -> Self {
        Self::new(mesh, material)
            .with_capacity(count)
            .with_max_instances(count)
    }

    /// Dynamic batch (frequently changing)
    pub fn dynamic_batch(mesh: u64, material: u64) -> Self {
        Self::new(mesh, material).dynamic()
    }
}

impl Default for MeshBatchCreateInfo {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

/// Batch sorting mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BatchSortingMode {
    /// No sorting
    #[default]
    None = 0,
    /// Front-to-back (opaque)
    FrontToBack = 1,
    /// Back-to-front (transparent)
    BackToFront = 2,
    /// By material
    ByMaterial = 3,
    /// By mesh
    ByMesh = 4,
    /// By state
    ByState = 5,
}

// ============================================================================
// Mesh Instance
// ============================================================================

/// Mesh instance data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MeshInstanceData {
    /// World transform (4x4 matrix, row-major)
    pub transform: [[f32; 4]; 4],
    /// Previous transform (for motion vectors)
    pub prev_transform: [[f32; 4]; 4],
    /// Object ID
    pub object_id: u32,
    /// Material ID
    pub material_id: u32,
    /// LOD level
    pub lod_level: u32,
    /// Flags
    pub flags: u32,
    /// Custom data
    pub custom: [f32; 4],
}

impl MeshInstanceData {
    /// Creates new instance data
    pub fn new() -> Self {
        Self {
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            prev_transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            object_id: 0,
            material_id: 0,
            lod_level: 0,
            flags: 0,
            custom: [0.0; 4],
        }
    }

    /// With translation
    pub fn with_translation(mut self, x: f32, y: f32, z: f32) -> Self {
        self.transform[3][0] = x;
        self.transform[3][1] = y;
        self.transform[3][2] = z;
        self
    }

    /// With scale
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.transform[0][0] = scale;
        self.transform[1][1] = scale;
        self.transform[2][2] = scale;
        self
    }

    /// With scale XYZ
    pub fn with_scale_xyz(mut self, sx: f32, sy: f32, sz: f32) -> Self {
        self.transform[0][0] = sx;
        self.transform[1][1] = sy;
        self.transform[2][2] = sz;
        self
    }

    /// With object ID
    pub fn with_object_id(mut self, id: u32) -> Self {
        self.object_id = id;
        self
    }

    /// With material ID
    pub fn with_material_id(mut self, id: u32) -> Self {
        self.material_id = id;
        self
    }

    /// With LOD level
    pub fn with_lod(mut self, level: u32) -> Self {
        self.lod_level = level;
        self
    }

    /// Copy previous transform
    pub fn copy_prev_transform(&mut self) {
        self.prev_transform = self.transform;
    }

    /// Instance size in bytes
    pub const fn size() -> usize {
        core::mem::size_of::<Self>()
    }
}

/// Mesh instance flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct MeshInstanceFlags(pub u32);

impl MeshInstanceFlags {
    /// Visible
    pub const VISIBLE: Self = Self(1 << 0);
    /// Cast shadows
    pub const CAST_SHADOWS: Self = Self(1 << 1);
    /// Receive shadows
    pub const RECEIVE_SHADOWS: Self = Self(1 << 2);
    /// Static
    pub const STATIC: Self = Self(1 << 3);
    /// Two-sided
    pub const TWO_SIDED: Self = Self(1 << 4);
    /// Selected
    pub const SELECTED: Self = Self(1 << 5);
    /// Highlighted
    pub const HIGHLIGHTED: Self = Self(1 << 6);

    /// Default flags
    pub const DEFAULT: Self = Self(
        Self::VISIBLE.0 |
        Self::CAST_SHADOWS.0 |
        Self::RECEIVE_SHADOWS.0
    );
}

// ============================================================================
// Draw Commands
// ============================================================================

/// Indexed draw command
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DrawIndexedCommand {
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

impl DrawIndexedCommand {
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
    pub const fn with_first_index(mut self, first: u32) -> Self {
        self.first_index = first;
        self
    }

    /// With vertex offset
    pub const fn with_vertex_offset(mut self, offset: i32) -> Self {
        self.vertex_offset = offset;
        self
    }

    /// With first instance
    pub const fn with_first_instance(mut self, first: u32) -> Self {
        self.first_instance = first;
        self
    }

    /// Command size
    pub const fn size() -> usize {
        core::mem::size_of::<Self>()
    }
}

/// Non-indexed draw command
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DrawCommand {
    /// Vertex count
    pub vertex_count: u32,
    /// Instance count
    pub instance_count: u32,
    /// First vertex
    pub first_vertex: u32,
    /// First instance
    pub first_instance: u32,
}

impl DrawCommand {
    /// Creates new command
    pub const fn new(vertex_count: u32, instance_count: u32) -> Self {
        Self {
            vertex_count,
            instance_count,
            first_vertex: 0,
            first_instance: 0,
        }
    }

    /// With first vertex
    pub const fn with_first_vertex(mut self, first: u32) -> Self {
        self.first_vertex = first;
        self
    }

    /// With first instance
    pub const fn with_first_instance(mut self, first: u32) -> Self {
        self.first_instance = first;
        self
    }

    /// Command size
    pub const fn size() -> usize {
        core::mem::size_of::<Self>()
    }
}

/// Mesh task draw command (mesh shading)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DrawMeshTasksCommand {
    /// Group count X
    pub group_count_x: u32,
    /// Group count Y
    pub group_count_y: u32,
    /// Group count Z
    pub group_count_z: u32,
}

impl DrawMeshTasksCommand {
    /// Creates new command
    pub const fn new(groups_x: u32, groups_y: u32, groups_z: u32) -> Self {
        Self {
            group_count_x: groups_x,
            group_count_y: groups_y,
            group_count_z: groups_z,
        }
    }

    /// 1D dispatch
    pub const fn dispatch_1d(groups: u32) -> Self {
        Self::new(groups, 1, 1)
    }

    /// Command size
    pub const fn size() -> usize {
        core::mem::size_of::<Self>()
    }
}

// ============================================================================
// Draw Command Buffer
// ============================================================================

/// Draw command buffer create info
#[derive(Clone, Debug)]
pub struct DrawCommandBufferCreateInfo {
    /// Name
    pub name: String,
    /// Command type
    pub command_type: DrawCommandType,
    /// Max commands
    pub max_commands: u32,
    /// CPU accessible
    pub cpu_accessible: bool,
    /// Count buffer offset
    pub count_buffer_offset: u64,
}

impl DrawCommandBufferCreateInfo {
    /// Creates new info
    pub fn new(command_type: DrawCommandType, max_commands: u32) -> Self {
        Self {
            name: String::new(),
            command_type,
            max_commands,
            cpu_accessible: false,
            count_buffer_offset: 0,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// CPU accessible
    pub fn cpu_accessible(mut self) -> Self {
        self.cpu_accessible = true;
        self
    }

    /// With count buffer offset
    pub fn with_count_offset(mut self, offset: u64) -> Self {
        self.count_buffer_offset = offset;
        self
    }

    /// Indexed commands
    pub fn indexed(max_commands: u32) -> Self {
        Self::new(DrawCommandType::Indexed, max_commands)
    }

    /// Non-indexed commands
    pub fn non_indexed(max_commands: u32) -> Self {
        Self::new(DrawCommandType::NonIndexed, max_commands)
    }

    /// Mesh tasks commands
    pub fn mesh_tasks(max_commands: u32) -> Self {
        Self::new(DrawCommandType::MeshTasks, max_commands)
    }
}

impl Default for DrawCommandBufferCreateInfo {
    fn default() -> Self {
        Self::indexed(1000)
    }
}

/// Draw command type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DrawCommandType {
    /// Indexed draw
    #[default]
    Indexed = 0,
    /// Non-indexed draw
    NonIndexed = 1,
    /// Mesh tasks
    MeshTasks = 2,
}

impl DrawCommandType {
    /// Command size
    pub const fn command_size(&self) -> usize {
        match self {
            Self::Indexed => DrawIndexedCommand::size(),
            Self::NonIndexed => DrawCommand::size(),
            Self::MeshTasks => DrawMeshTasksCommand::size(),
        }
    }
}

// ============================================================================
// Instance Buffer
// ============================================================================

/// Mesh instance buffer create info
#[derive(Clone, Debug)]
pub struct MeshInstanceBufferCreateInfo {
    /// Name
    pub name: String,
    /// Instance data layout
    pub layout: InstanceDataLayout,
    /// Max instances
    pub max_instances: u32,
    /// Dynamic
    pub dynamic: bool,
    /// CPU accessible
    pub cpu_accessible: bool,
}

impl MeshInstanceBufferCreateInfo {
    /// Creates new info
    pub fn new(max_instances: u32) -> Self {
        Self {
            name: String::new(),
            layout: InstanceDataLayout::Standard,
            max_instances,
            dynamic: false,
            cpu_accessible: false,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With layout
    pub fn with_layout(mut self, layout: InstanceDataLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Dynamic buffer
    pub fn dynamic(mut self) -> Self {
        self.dynamic = true;
        self
    }

    /// CPU accessible
    pub fn cpu_accessible(mut self) -> Self {
        self.cpu_accessible = true;
        self
    }

    /// Standard layout
    pub fn standard(max_instances: u32) -> Self {
        Self::new(max_instances).with_layout(InstanceDataLayout::Standard)
    }

    /// Minimal layout (transform only)
    pub fn minimal(max_instances: u32) -> Self {
        Self::new(max_instances).with_layout(InstanceDataLayout::Minimal)
    }
}

impl Default for MeshInstanceBufferCreateInfo {
    fn default() -> Self {
        Self::new(10000)
    }
}

/// Instance data layout
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum InstanceDataLayout {
    /// Standard layout (full MeshInstanceData)
    #[default]
    Standard = 0,
    /// Minimal layout (transform only)
    Minimal = 1,
    /// Extended layout (with custom data)
    Extended = 2,
    /// Custom layout
    Custom = 3,
}

impl InstanceDataLayout {
    /// Instance size for layout
    pub const fn instance_size(&self) -> usize {
        match self {
            Self::Standard => MeshInstanceData::size(),
            Self::Minimal => 64,  // 4x4 matrix
            Self::Extended => MeshInstanceData::size() + 64,  // Extra custom data
            Self::Custom => 0,  // User-defined
        }
    }
}

// ============================================================================
// Mesh Render Pass
// ============================================================================

/// Mesh render pass info
#[derive(Clone, Debug)]
pub struct MeshRenderPassInfo {
    /// Pass type
    pub pass_type: MeshPassType,
    /// Renderer
    pub renderer: MeshRendererHandle,
    /// View projection matrix
    pub view_projection: [[f32; 4]; 4],
    /// Camera position
    pub camera_position: [f32; 3],
    /// View frustum planes
    pub frustum_planes: [[f32; 4]; 6],
    /// Sorting mode
    pub sorting: BatchSortingMode,
    /// Filters
    pub filters: MeshPassFilters,
}

impl MeshRenderPassInfo {
    /// Creates new info
    pub fn new(pass_type: MeshPassType) -> Self {
        Self {
            pass_type,
            renderer: MeshRendererHandle::NULL,
            view_projection: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            camera_position: [0.0, 0.0, 0.0],
            frustum_planes: [[0.0; 4]; 6],
            sorting: BatchSortingMode::None,
            filters: MeshPassFilters::default(),
        }
    }

    /// With renderer
    pub fn with_renderer(mut self, renderer: MeshRendererHandle) -> Self {
        self.renderer = renderer;
        self
    }

    /// With view projection
    pub fn with_view_projection(mut self, vp: [[f32; 4]; 4]) -> Self {
        self.view_projection = vp;
        self
    }

    /// With camera position
    pub fn with_camera(mut self, pos: [f32; 3]) -> Self {
        self.camera_position = pos;
        self
    }

    /// With sorting
    pub fn with_sorting(mut self, mode: BatchSortingMode) -> Self {
        self.sorting = mode;
        self
    }

    /// Opaque pass
    pub fn opaque() -> Self {
        Self::new(MeshPassType::Opaque)
            .with_sorting(BatchSortingMode::FrontToBack)
    }

    /// Transparent pass
    pub fn transparent() -> Self {
        Self::new(MeshPassType::Transparent)
            .with_sorting(BatchSortingMode::BackToFront)
    }

    /// Shadow pass
    pub fn shadow() -> Self {
        Self::new(MeshPassType::Shadow)
    }

    /// G-buffer pass
    pub fn gbuffer() -> Self {
        Self::new(MeshPassType::GBuffer)
            .with_sorting(BatchSortingMode::ByMaterial)
    }
}

impl Default for MeshRenderPassInfo {
    fn default() -> Self {
        Self::opaque()
    }
}

/// Mesh pass type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum MeshPassType {
    /// Opaque geometry
    #[default]
    Opaque = 0,
    /// Transparent geometry
    Transparent = 1,
    /// Shadow pass
    Shadow = 2,
    /// G-buffer pass
    GBuffer = 3,
    /// Depth prepass
    DepthPrepass = 4,
    /// Motion vectors
    MotionVectors = 5,
    /// Custom pass
    Custom = 6,
}

/// Mesh pass filters
#[derive(Clone, Debug, Default)]
pub struct MeshPassFilters {
    /// Include layers
    pub include_layers: u32,
    /// Exclude layers
    pub exclude_layers: u32,
    /// Include materials
    pub include_materials: Vec<u64>,
    /// Exclude materials
    pub exclude_materials: Vec<u64>,
}

impl MeshPassFilters {
    /// No filtering
    pub fn none() -> Self {
        Self::default()
    }

    /// Include layer
    pub fn with_layer(mut self, layer: u32) -> Self {
        self.include_layers |= 1 << layer;
        self
    }

    /// Exclude layer
    pub fn without_layer(mut self, layer: u32) -> Self {
        self.exclude_layers |= 1 << layer;
        self
    }

    /// Include material
    pub fn with_material(mut self, material: u64) -> Self {
        self.include_materials.push(material);
        self
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Mesh renderer statistics
#[derive(Clone, Debug, Default)]
pub struct MeshRendererStats {
    /// Total meshes
    pub total_meshes: u32,
    /// Total instances
    pub total_instances: u32,
    /// Visible instances (after culling)
    pub visible_instances: u32,
    /// Draw calls
    pub draw_calls: u32,
    /// Triangles rendered
    pub triangles_rendered: u64,
    /// Vertices rendered
    pub vertices_rendered: u64,
    /// Batches
    pub batches: u32,
    /// Instances per batch (average)
    pub avg_instances_per_batch: f32,
    /// Culling time (microseconds)
    pub culling_time_us: u64,
    /// Sorting time (microseconds)
    pub sorting_time_us: u64,
}

impl MeshRendererStats {
    /// Culling efficiency (0.0 - 1.0)
    pub fn culling_efficiency(&self) -> f32 {
        if self.total_instances == 0 {
            return 0.0;
        }
        1.0 - (self.visible_instances as f32 / self.total_instances as f32)
    }

    /// Average triangles per draw call
    pub fn avg_triangles_per_draw(&self) -> f32 {
        if self.draw_calls == 0 {
            return 0.0;
        }
        self.triangles_rendered as f32 / self.draw_calls as f32
    }
}
