//! Indirect drawing and GPU-driven rendering types
//!
//! This module provides types for indirect draw commands and GPU-driven rendering.

/// Indirect draw command (non-indexed)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DrawIndirectCommand {
    /// Number of vertices to draw
    pub vertex_count: u32,
    /// Number of instances
    pub instance_count: u32,
    /// First vertex index
    pub first_vertex: u32,
    /// First instance
    pub first_instance: u32,
}

impl DrawIndirectCommand {
    /// Size in bytes
    pub const SIZE: u32 = 16;

    /// Creates a new draw command
    pub const fn new(vertex_count: u32, instance_count: u32) -> Self {
        Self {
            vertex_count,
            instance_count,
            first_vertex: 0,
            first_instance: 0,
        }
    }

    /// Single instance draw
    pub const fn single(vertex_count: u32) -> Self {
        Self::new(vertex_count, 1)
    }

    /// With first vertex offset
    pub const fn with_first_vertex(mut self, first: u32) -> Self {
        self.first_vertex = first;
        self
    }

    /// With first instance offset
    pub const fn with_first_instance(mut self, first: u32) -> Self {
        self.first_instance = first;
        self
    }
}

/// Indexed indirect draw command
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DrawIndexedIndirectCommand {
    /// Number of indices to draw
    pub index_count: u32,
    /// Number of instances
    pub instance_count: u32,
    /// First index offset
    pub first_index: u32,
    /// Vertex offset added to each index
    pub vertex_offset: i32,
    /// First instance
    pub first_instance: u32,
}

impl DrawIndexedIndirectCommand {
    /// Size in bytes
    pub const SIZE: u32 = 20;

    /// Creates a new indexed draw command
    pub const fn new(index_count: u32, instance_count: u32) -> Self {
        Self {
            index_count,
            instance_count,
            first_index: 0,
            vertex_offset: 0,
            first_instance: 0,
        }
    }

    /// Single instance draw
    pub const fn single(index_count: u32) -> Self {
        Self::new(index_count, 1)
    }

    /// With first index offset
    pub const fn with_first_index(mut self, first: u32) -> Self {
        self.first_index = first;
        self
    }

    /// With vertex offset
    pub const fn with_vertex_offset(mut self, offset: i32) -> Self {
        self.vertex_offset = offset;
        self
    }
}

/// Indirect dispatch command (compute)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DispatchIndirectCommand {
    /// Workgroups in X
    pub x: u32,
    /// Workgroups in Y
    pub y: u32,
    /// Workgroups in Z
    pub z: u32,
}

impl DispatchIndirectCommand {
    /// Size in bytes
    pub const SIZE: u32 = 12;

    /// Creates a new dispatch command
    pub const fn new(x: u32, y: u32, z: u32) -> Self {
        Self { x, y, z }
    }

    /// 1D dispatch
    pub const fn x_only(x: u32) -> Self {
        Self::new(x, 1, 1)
    }

    /// 2D dispatch
    pub const fn xy(x: u32, y: u32) -> Self {
        Self::new(x, y, 1)
    }

    /// Total workgroups
    pub const fn total(&self) -> u32 {
        self.x * self.y * self.z
    }

    /// Calculates dispatch for given element count and workgroup size
    pub fn for_elements(element_count: u32, workgroup_size: u32) -> Self {
        Self::x_only((element_count + workgroup_size - 1) / workgroup_size)
    }
}

/// Draw indirect count command (multi-draw)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DrawIndirectCount {
    /// Maximum draw count
    pub max_draw_count: u32,
    /// Stride between commands
    pub stride: u32,
    /// Offset in count buffer
    pub count_buffer_offset: u64,
}

impl DrawIndirectCount {
    /// Creates a new draw indirect count
    pub const fn new(max_draw_count: u32) -> Self {
        Self {
            max_draw_count,
            stride: DrawIndirectCommand::SIZE,
            count_buffer_offset: 0,
        }
    }

    /// For indexed draws
    pub const fn indexed(max_draw_count: u32) -> Self {
        Self {
            max_draw_count,
            stride: DrawIndexedIndirectCommand::SIZE,
            count_buffer_offset: 0,
        }
    }

    /// With custom stride
    pub const fn with_stride(mut self, stride: u32) -> Self {
        self.stride = stride;
        self
    }
}

/// Mesh shader indirect command
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DrawMeshTasksIndirectCommand {
    /// Task groups in X
    pub group_count_x: u32,
    /// Task groups in Y
    pub group_count_y: u32,
    /// Task groups in Z
    pub group_count_z: u32,
}

impl DrawMeshTasksIndirectCommand {
    /// Size in bytes
    pub const SIZE: u32 = 12;

    /// Creates a new mesh tasks command
    pub const fn new(x: u32, y: u32, z: u32) -> Self {
        Self {
            group_count_x: x,
            group_count_y: y,
            group_count_z: z,
        }
    }

    /// 1D dispatch
    pub const fn x_only(x: u32) -> Self {
        Self::new(x, 1, 1)
    }
}

/// Ray tracing indirect command
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct TraceRaysIndirectCommand {
    /// Width of ray launch
    pub width: u32,
    /// Height of ray launch
    pub height: u32,
    /// Depth of ray launch
    pub depth: u32,
}

impl TraceRaysIndirectCommand {
    /// Size in bytes
    pub const SIZE: u32 = 12;

    /// Creates a 2D trace
    pub const fn new_2d(width: u32, height: u32) -> Self {
        Self { width, height, depth: 1 }
    }

    /// Creates a 1D trace
    pub const fn new_1d(count: u32) -> Self {
        Self { width: count, height: 1, depth: 1 }
    }

    /// Creates a 3D trace
    pub const fn new_3d(width: u32, height: u32, depth: u32) -> Self {
        Self { width, height, depth }
    }

    /// Total rays
    pub const fn total_rays(&self) -> u32 {
        self.width * self.height * self.depth
    }
}

/// GPU draw command with object data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuDrawCommand {
    /// Draw command
    pub draw: DrawIndexedIndirectCommand,
    /// Object index for fetching transforms/materials
    pub object_index: u32,
    /// LOD level
    pub lod_level: u32,
    /// Distance from camera (for sorting)
    pub distance: f32,
    /// Padding
    _pad: u32,
}

impl GpuDrawCommand {
    /// Size in bytes
    pub const SIZE: u32 = 32;

    /// Creates a new GPU draw command
    pub const fn new(draw: DrawIndexedIndirectCommand, object_index: u32) -> Self {
        Self {
            draw,
            object_index,
            lod_level: 0,
            distance: 0.0,
            _pad: 0,
        }
    }
}

/// Culling result flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct CullFlags(pub u32);

impl CullFlags {
    /// Visible
    pub const VISIBLE: Self = Self(1 << 0);
    /// Frustum culled
    pub const FRUSTUM_CULLED: Self = Self(1 << 1);
    /// Occlusion culled
    pub const OCCLUSION_CULLED: Self = Self(1 << 2);
    /// Distance culled
    pub const DISTANCE_CULLED: Self = Self(1 << 3);
    /// Small object culled
    pub const SIZE_CULLED: Self = Self(1 << 4);
    /// Backface culled
    pub const BACKFACE_CULLED: Self = Self(1 << 5);

    /// Any culling occurred
    pub const ANY_CULLED: Self = Self(
        Self::FRUSTUM_CULLED.0
            | Self::OCCLUSION_CULLED.0
            | Self::DISTANCE_CULLED.0
            | Self::SIZE_CULLED.0
            | Self::BACKFACE_CULLED.0
    );

    /// Is visible
    pub const fn is_visible(&self) -> bool {
        (self.0 & Self::VISIBLE.0) != 0
    }

    /// Was culled
    pub const fn is_culled(&self) -> bool {
        (self.0 & Self::ANY_CULLED.0) != 0
    }
}

/// GPU instance data for culling
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuInstance {
    /// World transform (4x3 matrix, column major)
    pub transform: [[f32; 4]; 3],
    /// Bounding sphere (xyz = center, w = radius)
    pub bounding_sphere: [f32; 4],
    /// Object index
    pub object_index: u32,
    /// Mesh index
    pub mesh_index: u32,
    /// Material index
    pub material_index: u32,
    /// Flags
    pub flags: u32,
}

impl GpuInstance {
    /// Size in bytes
    pub const SIZE: u32 = 80;
}

/// Mesh descriptor for GPU
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuMeshDescriptor {
    /// Vertex buffer offset
    pub vertex_offset: u32,
    /// Index buffer offset
    pub index_offset: u32,
    /// Index count
    pub index_count: u32,
    /// Material index
    pub material_index: u32,
    /// Bounding sphere (xyz = center, w = radius)
    pub bounding_sphere: [f32; 4],
    /// LOD distances
    pub lod_distances: [f32; 4],
    /// LOD index counts
    pub lod_index_counts: [u32; 4],
    /// LOD index offsets
    pub lod_index_offsets: [u32; 4],
}

impl GpuMeshDescriptor {
    /// Size in bytes
    pub const SIZE: u32 = 80;

    /// Gets LOD level for distance
    pub fn lod_for_distance(&self, distance: f32) -> u32 {
        for (i, &lod_dist) in self.lod_distances.iter().enumerate() {
            if distance < lod_dist || lod_dist <= 0.0 {
                return i as u32;
            }
        }
        3
    }
}

/// Draw call compaction result
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct CompactionResult {
    /// Number of visible draws
    pub visible_count: u32,
    /// Number of culled draws
    pub culled_count: u32,
    /// Total triangles in visible draws
    pub visible_triangles: u32,
    /// Total triangles culled
    pub culled_triangles: u32,
}

impl CompactionResult {
    /// Culling efficiency (0-100%)
    pub fn culling_efficiency(&self) -> f32 {
        let total = self.visible_count + self.culled_count;
        if total > 0 {
            (self.culled_count as f32 / total as f32) * 100.0
        } else {
            0.0
        }
    }

    /// Triangle reduction (0-100%)
    pub fn triangle_reduction(&self) -> f32 {
        let total = self.visible_triangles + self.culled_triangles;
        if total > 0 {
            (self.culled_triangles as f32 / total as f32) * 100.0
        } else {
            0.0
        }
    }
}

/// Predicate for conditional rendering
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct RenderPredicate {
    /// Value to compare against
    pub value: u64,
}

impl RenderPredicate {
    /// Creates a non-zero predicate
    pub const fn non_zero() -> Self {
        Self { value: 0 }
    }

    /// Creates an any-hit predicate
    pub const fn any_hit() -> Self {
        Self { value: 0 }
    }
}
