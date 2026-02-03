//! GPU-Driven Rendering Types for Lumina
//!
//! This module provides GPU-driven rendering infrastructure including
//! indirect draw commands, GPU culling, and indirect dispatch.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// GPU-Driven Handles
// ============================================================================

/// Indirect buffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct IndirectBufferHandle(pub u64);

impl IndirectBufferHandle {
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

impl Default for IndirectBufferHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Draw commands buffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DrawCommandsBufferHandle(pub u64);

impl DrawCommandsBufferHandle {
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

impl Default for DrawCommandsBufferHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Instance buffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuInstanceBufferHandle(pub u64);

impl GpuInstanceBufferHandle {
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

impl Default for GpuInstanceBufferHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Indirect Draw Commands
// ============================================================================

/// Indirect draw command (non-indexed)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DrawIndirectCommand {
    /// Vertex count
    pub vertex_count: u32,
    /// Instance count
    pub instance_count: u32,
    /// First vertex
    pub first_vertex: u32,
    /// First instance
    pub first_instance: u32,
}

impl DrawIndirectCommand {
    /// Size in bytes
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Creates command
    pub const fn new(vertex_count: u32, instance_count: u32) -> Self {
        Self {
            vertex_count,
            instance_count,
            first_vertex: 0,
            first_instance: 0,
        }
    }

    /// Empty command (no draw)
    pub const fn empty() -> Self {
        Self::new(0, 0)
    }

    /// Single instance
    pub const fn single(vertex_count: u32) -> Self {
        Self::new(vertex_count, 1)
    }

    /// Triangle (3 vertices)
    pub const fn triangle() -> Self {
        Self::single(3)
    }

    /// Quad (6 vertices, 2 triangles)
    pub const fn quad() -> Self {
        Self::single(6)
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
}

/// Indexed indirect draw command
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DrawIndexedIndirectCommand {
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

impl DrawIndexedIndirectCommand {
    /// Size in bytes
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Creates command
    pub const fn new(index_count: u32, instance_count: u32) -> Self {
        Self {
            index_count,
            instance_count,
            first_index: 0,
            vertex_offset: 0,
            first_instance: 0,
        }
    }

    /// Empty command
    pub const fn empty() -> Self {
        Self::new(0, 0)
    }

    /// Single instance
    pub const fn single(index_count: u32) -> Self {
        Self::new(index_count, 1)
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
}

/// Dispatch indirect command
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DispatchIndirectCommand {
    /// X dimension
    pub x: u32,
    /// Y dimension
    pub y: u32,
    /// Z dimension
    pub z: u32,
}

impl DispatchIndirectCommand {
    /// Size in bytes
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Creates command
    pub const fn new(x: u32, y: u32, z: u32) -> Self {
        Self { x, y, z }
    }

    /// Empty dispatch
    pub const fn empty() -> Self {
        Self::new(0, 0, 0)
    }

    /// 1D dispatch
    pub const fn dispatch_1d(x: u32) -> Self {
        Self::new(x, 1, 1)
    }

    /// 2D dispatch
    pub const fn dispatch_2d(x: u32, y: u32) -> Self {
        Self::new(x, y, 1)
    }

    /// 3D dispatch
    pub const fn dispatch_3d(x: u32, y: u32, z: u32) -> Self {
        Self::new(x, y, z)
    }

    /// Total workgroups
    pub const fn total_workgroups(&self) -> u64 {
        self.x as u64 * self.y as u64 * self.z as u64
    }
}

// ============================================================================
// Multi-Draw Indirect
// ============================================================================

/// Multi-draw indirect count command
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MultiDrawIndirectCount {
    /// Draw commands buffer
    pub commands_buffer: u64,
    /// Commands offset
    pub commands_offset: u64,
    /// Count buffer
    pub count_buffer: u64,
    /// Count offset
    pub count_offset: u64,
    /// Max draw count
    pub max_draw_count: u32,
    /// Stride
    pub stride: u32,
}

impl MultiDrawIndirectCount {
    /// Creates count info
    pub const fn new(
        commands_buffer: u64,
        count_buffer: u64,
        max_draw_count: u32,
    ) -> Self {
        Self {
            commands_buffer,
            commands_offset: 0,
            count_buffer,
            count_offset: 0,
            max_draw_count,
            stride: DrawIndexedIndirectCommand::SIZE as u32,
        }
    }

    /// With offsets
    pub const fn with_offsets(mut self, commands_offset: u64, count_offset: u64) -> Self {
        self.commands_offset = commands_offset;
        self.count_offset = count_offset;
        self
    }

    /// With stride
    pub const fn with_stride(mut self, stride: u32) -> Self {
        self.stride = stride;
        self
    }
}

// ============================================================================
// GPU Culling
// ============================================================================

/// GPU culling pipeline create info
#[derive(Clone, Debug)]
pub struct GpuCullingPipelineCreateInfo {
    /// Name
    pub name: String,
    /// Culling mode
    pub mode: GpuCullingMode,
    /// Enable frustum culling
    pub frustum_culling: bool,
    /// Enable occlusion culling
    pub occlusion_culling: bool,
    /// Enable distance culling
    pub distance_culling: bool,
    /// Enable backface culling
    pub backface_culling: bool,
    /// Enable small primitive culling
    pub small_primitive_culling: bool,
    /// Max instances
    pub max_instances: u32,
}

impl GpuCullingPipelineCreateInfo {
    /// Creates info
    pub fn new(max_instances: u32) -> Self {
        Self {
            name: String::new(),
            mode: GpuCullingMode::SinglePass,
            frustum_culling: true,
            occlusion_culling: false,
            distance_culling: true,
            backface_culling: false,
            small_primitive_culling: false,
            max_instances,
        }
    }

    /// Full culling (all features)
    pub fn full(max_instances: u32) -> Self {
        Self {
            name: String::from("FullCulling"),
            mode: GpuCullingMode::TwoPass,
            frustum_culling: true,
            occlusion_culling: true,
            distance_culling: true,
            backface_culling: true,
            small_primitive_culling: true,
            max_instances,
        }
    }

    /// Fast culling (frustum only)
    pub fn fast(max_instances: u32) -> Self {
        Self {
            name: String::from("FastCulling"),
            mode: GpuCullingMode::SinglePass,
            frustum_culling: true,
            occlusion_culling: false,
            distance_culling: false,
            backface_culling: false,
            small_primitive_culling: false,
            max_instances,
        }
    }

    /// With mode
    pub fn with_mode(mut self, mode: GpuCullingMode) -> Self {
        self.mode = mode;
        self
    }

    /// With occlusion culling
    pub fn with_occlusion_culling(mut self) -> Self {
        self.occlusion_culling = true;
        self
    }

    /// With backface culling
    pub fn with_backface_culling(mut self) -> Self {
        self.backface_culling = true;
        self
    }
}

impl Default for GpuCullingPipelineCreateInfo {
    fn default() -> Self {
        Self::new(100000)
    }
}

/// GPU culling mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum GpuCullingMode {
    /// Single pass culling
    #[default]
    SinglePass = 0,
    /// Two pass hierarchical culling
    TwoPass = 1,
    /// Multi-view culling (VR/shadow cascades)
    MultiView = 2,
    /// Meshlet culling
    Meshlet = 3,
}

/// GPU culling params (for compute shader)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuCullingParams {
    /// View-projection matrix
    pub view_proj: [[f32; 4]; 4],
    /// Frustum planes
    pub frustum_planes: [[f32; 4]; 6],
    /// Camera position
    pub camera_pos: [f32; 4],
    /// Instance count
    pub instance_count: u32,
    /// Near plane
    pub near_plane: f32,
    /// Far plane
    pub far_plane: f32,
    /// Small primitive threshold (pixels)
    pub small_primitive_threshold: f32,
    /// Flags
    pub flags: u32,
    /// Padding
    pub _padding: [u32; 3],
}

impl GpuCullingParams {
    /// Size in bytes
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Frustum cull flag
    pub const FLAG_FRUSTUM_CULL: u32 = 1 << 0;
    /// Occlusion cull flag
    pub const FLAG_OCCLUSION_CULL: u32 = 1 << 1;
    /// Distance cull flag
    pub const FLAG_DISTANCE_CULL: u32 = 1 << 2;
    /// Backface cull flag
    pub const FLAG_BACKFACE_CULL: u32 = 1 << 3;
    /// Small primitive cull flag
    pub const FLAG_SMALL_PRIMITIVE_CULL: u32 = 1 << 4;
}

// ============================================================================
// GPU Scene
// ============================================================================

/// GPU scene create info
#[derive(Clone, Debug)]
pub struct GpuSceneCreateInfo {
    /// Name
    pub name: String,
    /// Max meshes
    pub max_meshes: u32,
    /// Max instances
    pub max_instances: u32,
    /// Max materials
    pub max_materials: u32,
    /// Max draw commands
    pub max_draw_commands: u32,
    /// Enable GPU culling
    pub gpu_culling: bool,
}

impl GpuSceneCreateInfo {
    /// Creates info
    pub fn new(max_instances: u32) -> Self {
        Self {
            name: String::new(),
            max_meshes: max_instances / 10,
            max_instances,
            max_materials: 1000,
            max_draw_commands: max_instances,
            gpu_culling: true,
        }
    }

    /// Small scene
    pub fn small() -> Self {
        Self::new(10000)
    }

    /// Medium scene
    pub fn medium() -> Self {
        Self::new(100000)
    }

    /// Large scene
    pub fn large() -> Self {
        Self::new(1000000)
    }

    /// With culling
    pub fn with_culling(mut self, enabled: bool) -> Self {
        self.gpu_culling = enabled;
        self
    }
}

impl Default for GpuSceneCreateInfo {
    fn default() -> Self {
        Self::medium()
    }
}

/// GPU mesh data (uploaded to GPU)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuMeshData {
    /// Vertex buffer offset
    pub vertex_buffer_offset: u32,
    /// Index buffer offset
    pub index_buffer_offset: u32,
    /// Vertex count
    pub vertex_count: u32,
    /// Index count
    pub index_count: u32,
    /// Bounding sphere center
    pub bounding_center: [f32; 3],
    /// Bounding sphere radius
    pub bounding_radius: f32,
    /// AABB min
    pub aabb_min: [f32; 3],
    /// Material index
    pub material_index: u32,
    /// AABB max
    pub aabb_max: [f32; 3],
    /// LOD level
    pub lod_level: u32,
}

impl GpuMeshData {
    /// Size in bytes
    pub const SIZE: usize = core::mem::size_of::<Self>();
}

/// GPU instance data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuInstanceData {
    /// Transform (model matrix) - row major
    pub transform: [[f32; 4]; 3],
    /// Mesh index
    pub mesh_index: u32,
    /// Instance ID
    pub instance_id: u32,
    /// Flags
    pub flags: u32,
    /// Custom data
    pub custom_data: u32,
}

impl GpuInstanceData {
    /// Size in bytes
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Visible flag
    pub const FLAG_VISIBLE: u32 = 1 << 0;
    /// Casts shadow flag
    pub const FLAG_CAST_SHADOW: u32 = 1 << 1;
    /// Dynamic flag
    pub const FLAG_DYNAMIC: u32 = 1 << 2;
}

// ============================================================================
// Indirect Draw Builder
// ============================================================================

/// Indirect draw buffer create info
#[derive(Clone, Debug)]
pub struct IndirectBufferCreateInfo {
    /// Name
    pub name: String,
    /// Max draw commands
    pub max_commands: u32,
    /// Command type
    pub command_type: IndirectCommandType,
    /// GPU writable
    pub gpu_writable: bool,
    /// Include count buffer
    pub with_count_buffer: bool,
}

impl IndirectBufferCreateInfo {
    /// Creates info
    pub fn new(max_commands: u32) -> Self {
        Self {
            name: String::new(),
            max_commands,
            command_type: IndirectCommandType::DrawIndexed,
            gpu_writable: true,
            with_count_buffer: true,
        }
    }

    /// Draw commands (non-indexed)
    pub fn draw(max_commands: u32) -> Self {
        Self {
            command_type: IndirectCommandType::Draw,
            ..Self::new(max_commands)
        }
    }

    /// Draw indexed commands
    pub fn draw_indexed(max_commands: u32) -> Self {
        Self {
            command_type: IndirectCommandType::DrawIndexed,
            ..Self::new(max_commands)
        }
    }

    /// Dispatch commands
    pub fn dispatch(max_commands: u32) -> Self {
        Self {
            command_type: IndirectCommandType::Dispatch,
            ..Self::new(max_commands)
        }
    }

    /// CPU only (no GPU writes)
    pub fn cpu_only(mut self) -> Self {
        self.gpu_writable = false;
        self
    }

    /// Buffer size in bytes
    pub fn buffer_size(&self) -> u64 {
        let command_size = match self.command_type {
            IndirectCommandType::Draw => DrawIndirectCommand::SIZE,
            IndirectCommandType::DrawIndexed => DrawIndexedIndirectCommand::SIZE,
            IndirectCommandType::Dispatch => DispatchIndirectCommand::SIZE,
        };
        (command_size as u64) * (self.max_commands as u64)
    }
}

impl Default for IndirectBufferCreateInfo {
    fn default() -> Self {
        Self::draw_indexed(10000)
    }
}

/// Indirect command type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum IndirectCommandType {
    /// Draw (non-indexed)
    Draw = 0,
    /// Draw indexed
    #[default]
    DrawIndexed = 1,
    /// Dispatch
    Dispatch = 2,
}

// ============================================================================
// Meshlet Types (for mesh shaders)
// ============================================================================

/// Meshlet data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Meshlet {
    /// Vertex offset
    pub vertex_offset: u32,
    /// Triangle offset
    pub triangle_offset: u32,
    /// Vertex count
    pub vertex_count: u8,
    /// Triangle count
    pub triangle_count: u8,
    /// Padding
    pub _padding: [u8; 2],
    /// Bounding sphere center
    pub bounding_center: [f32; 3],
    /// Bounding sphere radius
    pub bounding_radius: f32,
    /// Cone axis
    pub cone_axis: [i8; 3],
    /// Cone cutoff
    pub cone_cutoff: i8,
}

impl Meshlet {
    /// Size in bytes
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Max vertices per meshlet
    pub const MAX_VERTICES: u32 = 64;
    /// Max triangles per meshlet
    pub const MAX_TRIANGLES: u32 = 124;
}

/// Meshlet draw info (for dispatch)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MeshletDrawInfo {
    /// Instance index
    pub instance_index: u32,
    /// Meshlet offset
    pub meshlet_offset: u32,
    /// Meshlet count
    pub meshlet_count: u32,
    /// Material index
    pub material_index: u32,
}

impl MeshletDrawInfo {
    /// Size in bytes
    pub const SIZE: usize = core::mem::size_of::<Self>();
}

// ============================================================================
// Draw Command Generation
// ============================================================================

/// Draw command generator settings
#[derive(Clone, Debug)]
pub struct DrawCommandGeneratorSettings {
    /// Instance buffer
    pub instance_buffer: GpuInstanceBufferHandle,
    /// Output commands buffer
    pub commands_buffer: DrawCommandsBufferHandle,
    /// Count buffer
    pub count_buffer: IndirectBufferHandle,
    /// Sort by material
    pub sort_by_material: bool,
    /// Sort by depth
    pub sort_by_depth: bool,
    /// Merge identical draws
    pub merge_draws: bool,
}

impl DrawCommandGeneratorSettings {
    /// Creates settings
    pub fn new(
        instances: GpuInstanceBufferHandle,
        commands: DrawCommandsBufferHandle,
    ) -> Self {
        Self {
            instance_buffer: instances,
            commands_buffer: commands,
            count_buffer: IndirectBufferHandle::NULL,
            sort_by_material: true,
            sort_by_depth: false,
            merge_draws: true,
        }
    }

    /// With count buffer
    pub fn with_count_buffer(mut self, buffer: IndirectBufferHandle) -> Self {
        self.count_buffer = buffer;
        self
    }

    /// With depth sort
    pub fn with_depth_sort(mut self) -> Self {
        self.sort_by_depth = true;
        self
    }
}

impl Default for DrawCommandGeneratorSettings {
    fn default() -> Self {
        Self::new(
            GpuInstanceBufferHandle::NULL,
            DrawCommandsBufferHandle::NULL,
        )
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// GPU-driven rendering statistics
#[derive(Clone, Debug, Default)]
pub struct GpuDrivenStats {
    /// Total instances
    pub total_instances: u64,
    /// Visible instances (after culling)
    pub visible_instances: u64,
    /// Draw commands generated
    pub draw_commands: u64,
    /// Triangles submitted
    pub triangles_submitted: u64,
    /// Culling time (microseconds)
    pub culling_time_us: u64,
    /// Command generation time (microseconds)
    pub command_gen_time_us: u64,
    /// Memory usage (bytes)
    pub memory_usage: u64,
}

impl GpuDrivenStats {
    /// Cull ratio
    pub fn cull_ratio(&self) -> f32 {
        if self.total_instances > 0 {
            1.0 - (self.visible_instances as f32 / self.total_instances as f32)
        } else {
            0.0
        }
    }

    /// Average triangles per draw
    pub fn triangles_per_draw(&self) -> f32 {
        if self.draw_commands > 0 {
            self.triangles_submitted as f32 / self.draw_commands as f32
        } else {
            0.0
        }
    }
}
