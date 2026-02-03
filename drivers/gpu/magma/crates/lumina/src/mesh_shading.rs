//! Mesh shading types
//!
//! This module provides types for mesh and task shader pipelines.

extern crate alloc;
use alloc::vec::Vec;

use crate::descriptor::PipelineLayoutHandle;
use crate::graphics_pipeline::{
    ColorTargetState, DepthStencilState, MultisampleState, PrimitiveState,
};
use crate::shader::ShaderModuleDesc;

/// Handle to a mesh pipeline
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MeshPipelineHandle(pub u64);

impl MeshPipelineHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Checks if this is a null handle
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

/// Mesh pipeline description
#[derive(Clone, Debug)]
pub struct MeshPipelineDesc {
    /// Task shader (optional)
    pub task_shader: Option<ShaderStageDesc>,
    /// Mesh shader (required)
    pub mesh_shader: ShaderStageDesc,
    /// Fragment shader
    pub fragment_shader: Option<ShaderStageDesc>,
    /// Pipeline layout
    pub layout: PipelineLayoutHandle,
    /// Primitive state
    pub primitive: PrimitiveState,
    /// Depth stencil state
    pub depth_stencil: Option<DepthStencilState>,
    /// Multisample state
    pub multisample: MultisampleState,
    /// Color targets
    pub color_targets: Vec<ColorTargetState>,
}

impl MeshPipelineDesc {
    /// Creates a new mesh pipeline description
    pub fn new(mesh_shader: ShaderStageDesc) -> Self {
        Self {
            task_shader: None,
            mesh_shader,
            fragment_shader: None,
            layout: PipelineLayoutHandle::NULL,
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            color_targets: Vec::new(),
        }
    }

    /// Sets the task shader
    pub fn with_task_shader(mut self, shader: ShaderStageDesc) -> Self {
        self.task_shader = Some(shader);
        self
    }

    /// Sets the fragment shader
    pub fn with_fragment_shader(mut self, shader: ShaderStageDesc) -> Self {
        self.fragment_shader = Some(shader);
        self
    }

    /// Sets the pipeline layout
    pub fn with_layout(mut self, layout: PipelineLayoutHandle) -> Self {
        self.layout = layout;
        self
    }

    /// Adds a color target
    pub fn add_color_target(mut self, target: ColorTargetState) -> Self {
        self.color_targets.push(target);
        self
    }

    /// Sets depth stencil state
    pub fn with_depth_stencil(mut self, state: DepthStencilState) -> Self {
        self.depth_stencil = Some(state);
        self
    }
}

/// Shader stage description
#[derive(Clone, Debug)]
pub struct ShaderStageDesc {
    /// Shader module
    pub module: ShaderModuleDesc,
    /// Entry point name
    pub entry_point: alloc::string::String,
    /// Specialization constants
    pub specialization: Vec<SpecConstant>,
}

impl ShaderStageDesc {
    /// Creates a new shader stage
    pub fn new(module: ShaderModuleDesc, entry_point: &str) -> Self {
        Self {
            module,
            entry_point: alloc::string::String::from(entry_point),
            specialization: Vec::new(),
        }
    }

    /// Adds a specialization constant
    pub fn add_spec_constant(mut self, constant: SpecConstant) -> Self {
        self.specialization.push(constant);
        self
    }
}

/// Specialization constant
#[derive(Clone, Copy, Debug)]
pub struct SpecConstant {
    /// Constant ID
    pub id: u32,
    /// Constant value
    pub value: SpecConstantValue,
}

/// Specialization constant value
#[derive(Clone, Copy, Debug)]
pub enum SpecConstantValue {
    /// Boolean
    Bool(bool),
    /// 32-bit integer
    I32(i32),
    /// 32-bit unsigned integer
    U32(u32),
    /// 32-bit float
    F32(f32),
}

impl SpecConstant {
    /// Creates a boolean constant
    pub const fn bool(id: u32, value: bool) -> Self {
        Self {
            id,
            value: SpecConstantValue::Bool(value),
        }
    }

    /// Creates a u32 constant
    pub const fn u32(id: u32, value: u32) -> Self {
        Self {
            id,
            value: SpecConstantValue::U32(value),
        }
    }
}

/// Draw mesh tasks command
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DrawMeshTasksCommand {
    /// Number of task workgroups in X
    pub group_count_x: u32,
    /// Number of task workgroups in Y
    pub group_count_y: u32,
    /// Number of task workgroups in Z
    pub group_count_z: u32,
}

impl DrawMeshTasksCommand {
    /// Creates a new draw mesh tasks command
    pub const fn new(x: u32, y: u32, z: u32) -> Self {
        Self {
            group_count_x: x,
            group_count_y: y,
            group_count_z: z,
        }
    }

    /// Creates a 1D dispatch
    pub const fn d1(x: u32) -> Self {
        Self::new(x, 1, 1)
    }

    /// Creates a 2D dispatch
    pub const fn d2(x: u32, y: u32) -> Self {
        Self::new(x, y, 1)
    }
}

/// Draw mesh tasks indirect command
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DrawMeshTasksIndirectCommand {
    /// Number of task workgroups in X
    pub group_count_x: u32,
    /// Number of task workgroups in Y
    pub group_count_y: u32,
    /// Number of task workgroups in Z
    pub group_count_z: u32,
}

/// Mesh shader properties
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MeshShaderProperties {
    /// Maximum number of output meshlets
    pub max_draw_mesh_tasks_count: u32,
    /// Maximum workgroup size X
    pub max_task_work_group_size_x: u32,
    /// Maximum workgroup size Y
    pub max_task_work_group_size_y: u32,
    /// Maximum workgroup size Z
    pub max_task_work_group_size_z: u32,
    /// Maximum total workgroup invocations
    pub max_task_work_group_total_count: u32,
    /// Maximum task shader output count
    pub max_task_output_count: u32,
    /// Maximum mesh workgroup size X
    pub max_mesh_work_group_size_x: u32,
    /// Maximum mesh workgroup size Y
    pub max_mesh_work_group_size_y: u32,
    /// Maximum mesh workgroup size Z
    pub max_mesh_work_group_size_z: u32,
    /// Maximum total mesh workgroup invocations
    pub max_mesh_work_group_total_count: u32,
    /// Maximum mesh output vertices
    pub max_mesh_output_vertices: u32,
    /// Maximum mesh output primitives
    pub max_mesh_output_primitives: u32,
    /// Maximum mesh output layers
    pub max_mesh_output_layers: u32,
    /// Maximum mesh shared memory size
    pub max_mesh_shared_memory_size: u32,
    /// Maximum task shared memory size
    pub max_task_shared_memory_size: u32,
    /// Maximum mesh multiview view count
    pub max_mesh_multiview_view_count: u32,
    /// Mesh output per vertex granularity
    pub mesh_output_per_vertex_granularity: u32,
    /// Mesh output per primitive granularity
    pub mesh_output_per_primitive_granularity: u32,
    /// Preferred task workgroup invocations
    pub preferred_task_work_group_invocations: u32,
    /// Preferred mesh workgroup invocations
    pub preferred_mesh_work_group_invocations: u32,
}

/// Meshlet description for mesh shading
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Meshlet {
    /// Offset to first vertex in the vertex buffer
    pub vertex_offset: u32,
    /// Number of vertices in this meshlet
    pub vertex_count: u32,
    /// Offset to first triangle (index triplets)
    pub triangle_offset: u32,
    /// Number of triangles
    pub triangle_count: u32,
}

impl Meshlet {
    /// Maximum vertices per meshlet (typical limit)
    pub const MAX_VERTICES: u32 = 64;
    /// Maximum triangles per meshlet (typical limit)
    pub const MAX_TRIANGLES: u32 = 126;

    /// Creates a new meshlet
    pub const fn new(
        vertex_offset: u32,
        vertex_count: u32,
        triangle_offset: u32,
        triangle_count: u32,
    ) -> Self {
        Self {
            vertex_offset,
            vertex_count,
            triangle_offset,
            triangle_count,
        }
    }

    /// Checks if meshlet has room for more vertices
    pub const fn has_vertex_room(&self, count: u32) -> bool {
        self.vertex_count + count <= Self::MAX_VERTICES
    }

    /// Checks if meshlet has room for more triangles
    pub const fn has_triangle_room(&self, count: u32) -> bool {
        self.triangle_count + count <= Self::MAX_TRIANGLES
    }
}

/// Meshlet bounds for culling
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MeshletBounds {
    /// Bounding sphere center
    pub center: [f32; 3],
    /// Bounding sphere radius
    pub radius: f32,
    /// Normal cone apex
    pub cone_apex: [f32; 3],
    /// Normal cone axis (normalized)
    pub cone_axis: [f32; 3],
    /// Normal cone cutoff (cos of half-angle)
    pub cone_cutoff: f32,
}

impl MeshletBounds {
    /// Creates bounds from sphere and cone
    pub fn new(
        center: [f32; 3],
        radius: f32,
        cone_apex: [f32; 3],
        cone_axis: [f32; 3],
        cone_cutoff: f32,
    ) -> Self {
        Self {
            center,
            radius,
            cone_apex,
            cone_axis,
            cone_cutoff,
        }
    }

    /// Checks if meshlet is completely back-facing
    pub fn is_backface_cluster(&self, view_pos: [f32; 3]) -> bool {
        let dx = view_pos[0] - self.cone_apex[0];
        let dy = view_pos[1] - self.cone_apex[1];
        let dz = view_pos[2] - self.cone_apex[2];

        let dot = dx * self.cone_axis[0] + dy * self.cone_axis[1] + dz * self.cone_axis[2];

        dot >= self.cone_cutoff * (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// Meshlet culling flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct MeshletCullFlags(pub u32);

impl MeshletCullFlags {
    /// No culling
    pub const NONE: Self = Self(0);
    /// Frustum culling
    pub const FRUSTUM: Self = Self(1 << 0);
    /// Occlusion culling
    pub const OCCLUSION: Self = Self(1 << 1);
    /// Backface cluster culling
    pub const BACKFACE: Self = Self(1 << 2);
    /// Small primitive culling
    pub const SMALL_PRIMITIVE: Self = Self(1 << 3);
    /// LOD selection
    pub const LOD: Self = Self(1 << 4);

    /// All culling enabled
    pub const ALL: Self = Self(
        Self::FRUSTUM.0
            | Self::OCCLUSION.0
            | Self::BACKFACE.0
            | Self::SMALL_PRIMITIVE.0
            | Self::LOD.0,
    );
}

impl core::ops::BitOr for MeshletCullFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Mesh shader output topology
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum MeshOutputTopology {
    /// Output points
    Points,
    /// Output lines
    Lines,
    /// Output triangles
    #[default]
    Triangles,
}

/// Task shader payload description
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct TaskPayloadDesc {
    /// Size of the payload in bytes
    pub size: u32,
    /// Maximum number of task invocations
    pub max_task_count: u32,
}

impl TaskPayloadDesc {
    /// Creates a new task payload description
    pub const fn new(size: u32, max_task_count: u32) -> Self {
        Self {
            size,
            max_task_count,
        }
    }
}

/// Mesh shader statistics for debugging
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MeshShaderStats {
    /// Number of task shader invocations
    pub task_invocations: u64,
    /// Number of mesh shader invocations
    pub mesh_invocations: u64,
    /// Number of output vertices
    pub output_vertices: u64,
    /// Number of output primitives
    pub output_primitives: u64,
    /// Number of meshlets culled
    pub meshlets_culled: u64,
}
