//! Mesh Shaders
//!
//! Next-generation geometry processing with task and mesh shaders.
//! Replaces traditional vertex/geometry pipeline with GPU-driven approach.
//!
//! # Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────────┐
//! │                    Mesh Shader Pipeline                            │
//! ├────────────────────────────────────────────────────────────────────┤
//! │                                                                    │
//! │  ┌──────────────┐    ┌──────────────┐    ┌──────────────────────┐ │
//! │  │ Task Shader  │───▶│ Mesh Shader  │───▶│ Fragment Shader      │ │
//! │  │ (Optional)   │    │              │    │                      │ │
//! │  │ Amplification│    │ Vertices +   │    │ Per-pixel shading    │ │
//! │  │ LOD, Culling │    │ Primitives   │    │                      │ │
//! │  └──────────────┘    └──────────────┘    └──────────────────────┘ │
//! │         │                   │                                      │
//! │         ▼                   ▼                                      │
//! │  ┌──────────────┐    ┌──────────────┐                            │
//! │  │ Meshlet Data │    │ Output       │                            │
//! │  │ (GPU Buffer) │    │ Vertices &   │                            │
//! │  │              │    │ Primitives   │                            │
//! │  └──────────────┘    └──────────────┘                            │
//! │                                                                    │
//! └────────────────────────────────────────────────────────────────────┘
//! ```

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use crate::buffer::BufferHandle;

// ============================================================================
// Meshlet Types
// ============================================================================

/// Meshlet - a small cluster of triangles.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Meshlet {
    /// Offset into vertex indices buffer.
    pub vertex_offset: u32,
    /// Offset into primitive indices buffer.
    pub triangle_offset: u32,
    /// Number of vertices.
    pub vertex_count: u32,
    /// Number of triangles.
    pub triangle_count: u32,
}

impl Default for Meshlet {
    fn default() -> Self {
        Self {
            vertex_offset: 0,
            triangle_offset: 0,
            vertex_count: 0,
            triangle_count: 0,
        }
    }
}

impl Meshlet {
    /// Maximum vertices per meshlet.
    pub const MAX_VERTICES: u32 = 64;
    /// Maximum triangles per meshlet.
    pub const MAX_TRIANGLES: u32 = 124;
    /// Maximum primitives per meshlet.
    pub const MAX_PRIMITIVES: u32 = 126;

    /// Create a new meshlet.
    pub fn new(
        vertex_offset: u32,
        triangle_offset: u32,
        vertex_count: u32,
        triangle_count: u32,
    ) -> Self {
        Self {
            vertex_offset,
            triangle_offset,
            vertex_count,
            triangle_count,
        }
    }

    /// Check if meshlet is valid.
    pub fn is_valid(&self) -> bool {
        self.vertex_count > 0
            && self.triangle_count > 0
            && self.vertex_count <= Self::MAX_VERTICES
            && self.triangle_count <= Self::MAX_TRIANGLES
    }
}

/// Meshlet bounding information for culling.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MeshletBounds {
    /// Bounding sphere center.
    pub center: [f32; 3],
    /// Bounding sphere radius.
    pub radius: f32,
    /// Cone axis (for backface culling).
    pub cone_axis: [f32; 3],
    /// Cone cutoff (cos of angle).
    pub cone_cutoff: f32,
    /// Cone apex (for backface culling).
    pub cone_apex: [f32; 3],
    /// Padding.
    pub _padding: f32,
}

impl Default for MeshletBounds {
    fn default() -> Self {
        Self {
            center: [0.0; 3],
            radius: 0.0,
            cone_axis: [0.0, 0.0, 1.0],
            cone_cutoff: 1.0,
            cone_apex: [0.0; 3],
            _padding: 0.0,
        }
    }
}

impl MeshletBounds {
    /// Create bounds from AABB.
    pub fn from_aabb(min: [f32; 3], max: [f32; 3]) -> Self {
        let center = [
            (min[0] + max[0]) * 0.5,
            (min[1] + max[1]) * 0.5,
            (min[2] + max[2]) * 0.5,
        ];
        let dx = max[0] - min[0];
        let dy = max[1] - min[1];
        let dz = max[2] - min[2];
        let radius = (dx * dx + dy * dy + dz * dz).sqrt() * 0.5;

        Self {
            center,
            radius,
            ..Default::default()
        }
    }

    /// Check if visible from frustum (simplified).
    pub fn is_visible(&self, frustum_planes: &[[f32; 4]; 6]) -> bool {
        for plane in frustum_planes {
            let dist = plane[0] * self.center[0]
                + plane[1] * self.center[1]
                + plane[2] * self.center[2]
                + plane[3];
            if dist < -self.radius {
                return false;
            }
        }
        true
    }

    /// Check backface culling.
    pub fn is_backface(&self, view_pos: [f32; 3]) -> bool {
        let to_apex = [
            view_pos[0] - self.cone_apex[0],
            view_pos[1] - self.cone_apex[1],
            view_pos[2] - self.cone_apex[2],
        ];
        let len =
            (to_apex[0] * to_apex[0] + to_apex[1] * to_apex[1] + to_apex[2] * to_apex[2]).sqrt();
        if len < 0.0001 {
            return false;
        }
        let dir = [to_apex[0] / len, to_apex[1] / len, to_apex[2] / len];
        let dot =
            dir[0] * self.cone_axis[0] + dir[1] * self.cone_axis[1] + dir[2] * self.cone_axis[2];
        dot < self.cone_cutoff
    }
}

// ============================================================================
// Meshlet Mesh
// ============================================================================

/// A mesh organized into meshlets.
#[derive(Debug, Clone)]
pub struct MeshletMesh {
    /// Meshlets.
    pub meshlets: Vec<Meshlet>,
    /// Meshlet bounds.
    pub bounds: Vec<MeshletBounds>,
    /// Vertex indices (local to meshlet).
    pub vertex_indices: Vec<u32>,
    /// Triangle indices (packed).
    pub triangle_indices: Vec<u8>,
    /// Total vertex count.
    pub vertex_count: u32,
    /// Total triangle count.
    pub triangle_count: u32,
}

impl MeshletMesh {
    /// Create empty meshlet mesh.
    pub fn new() -> Self {
        Self {
            meshlets: Vec::new(),
            bounds: Vec::new(),
            vertex_indices: Vec::new(),
            triangle_indices: Vec::new(),
            vertex_count: 0,
            triangle_count: 0,
        }
    }

    /// Get meshlet count.
    pub fn meshlet_count(&self) -> usize {
        self.meshlets.len()
    }

    /// Add a meshlet.
    pub fn add_meshlet(&mut self, vertices: &[u32], triangles: &[[u8; 3]], bounds: MeshletBounds) {
        let vertex_offset = self.vertex_indices.len() as u32;
        let triangle_offset = self.triangle_indices.len() as u32;

        let meshlet = Meshlet {
            vertex_offset,
            triangle_offset,
            vertex_count: vertices.len() as u32,
            triangle_count: triangles.len() as u32,
        };

        self.meshlets.push(meshlet);
        self.bounds.push(bounds);
        self.vertex_indices.extend_from_slice(vertices);

        for tri in triangles {
            self.triangle_indices.push(tri[0]);
            self.triangle_indices.push(tri[1]);
            self.triangle_indices.push(tri[2]);
        }

        self.vertex_count += vertices.len() as u32;
        self.triangle_count += triangles.len() as u32;
    }
}

impl Default for MeshletMesh {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Meshlet Generator
// ============================================================================

/// Configuration for meshlet generation.
#[derive(Debug, Clone, Copy)]
pub struct MeshletConfig {
    /// Maximum vertices per meshlet.
    pub max_vertices: u32,
    /// Maximum triangles per meshlet.
    pub max_triangles: u32,
    /// Cone weight for clustering.
    pub cone_weight: f32,
}

impl Default for MeshletConfig {
    fn default() -> Self {
        Self {
            max_vertices: Meshlet::MAX_VERTICES,
            max_triangles: Meshlet::MAX_TRIANGLES,
            cone_weight: 0.5,
        }
    }
}

/// Meshlet generator.
pub struct MeshletGenerator {
    /// Configuration.
    config: MeshletConfig,
}

impl MeshletGenerator {
    /// Create new generator.
    pub fn new(config: MeshletConfig) -> Self {
        Self { config }
    }

    /// Generate meshlets from triangle mesh.
    pub fn generate(&self, vertices: &[[f32; 3]], indices: &[u32]) -> MeshletMesh {
        let mut result = MeshletMesh::new();
        let triangle_count = indices.len() / 3;

        if triangle_count == 0 {
            return result;
        }

        // Simple greedy meshlet builder
        let mut used_triangles = vec![false; triangle_count];
        let mut current_vertices: Vec<u32> = Vec::new();
        let mut current_triangles: Vec<[u8; 3]> = Vec::new();
        let mut vertex_map: Vec<Option<u8>> = vec![None; vertices.len()];

        for tri_idx in 0..triangle_count {
            if used_triangles[tri_idx] {
                continue;
            }

            // Start new meshlet
            current_vertices.clear();
            current_triangles.clear();
            for v in vertex_map.iter_mut() {
                *v = None;
            }

            // Add triangles to meshlet
            self.fill_meshlet(
                indices,
                &mut used_triangles,
                &mut current_vertices,
                &mut current_triangles,
                &mut vertex_map,
                tri_idx,
            );

            // Calculate bounds
            let bounds = self.calculate_bounds(vertices, &current_vertices);

            // Add meshlet
            result.add_meshlet(&current_vertices, &current_triangles, bounds);
        }

        result
    }

    fn fill_meshlet(
        &self,
        indices: &[u32],
        used_triangles: &mut [bool],
        current_vertices: &mut Vec<u32>,
        current_triangles: &mut Vec<[u8; 3]>,
        vertex_map: &mut [Option<u8>],
        start_tri: usize,
    ) {
        let triangle_count = indices.len() / 3;
        let mut candidates = vec![start_tri];

        while !candidates.is_empty() {
            let tri_idx = candidates.pop().unwrap();

            if used_triangles[tri_idx] {
                continue;
            }

            let i0 = indices[tri_idx * 3] as usize;
            let i1 = indices[tri_idx * 3 + 1] as usize;
            let i2 = indices[tri_idx * 3 + 2] as usize;

            // Count new vertices needed
            let mut new_verts = 0;
            if vertex_map[i0].is_none() {
                new_verts += 1;
            }
            if vertex_map[i1].is_none() {
                new_verts += 1;
            }
            if vertex_map[i2].is_none() {
                new_verts += 1;
            }

            // Check limits
            if current_vertices.len() + new_verts > self.config.max_vertices as usize
                || current_triangles.len() >= self.config.max_triangles as usize
            {
                continue;
            }

            // Add vertices
            let local_i0 = self.get_or_add_vertex(i0, current_vertices, vertex_map);
            let local_i1 = self.get_or_add_vertex(i1, current_vertices, vertex_map);
            let local_i2 = self.get_or_add_vertex(i2, current_vertices, vertex_map);

            current_triangles.push([local_i0, local_i1, local_i2]);
            used_triangles[tri_idx] = true;

            // Add neighboring triangles as candidates
            for next_tri in 0..triangle_count {
                if !used_triangles[next_tri] {
                    let ni0 = indices[next_tri * 3] as usize;
                    let ni1 = indices[next_tri * 3 + 1] as usize;
                    let ni2 = indices[next_tri * 3 + 2] as usize;

                    // Check if shares vertex with current meshlet
                    if vertex_map[ni0].is_some()
                        || vertex_map[ni1].is_some()
                        || vertex_map[ni2].is_some()
                    {
                        candidates.push(next_tri);
                    }
                }
            }
        }
    }

    fn get_or_add_vertex(
        &self,
        vertex_idx: usize,
        vertices: &mut Vec<u32>,
        vertex_map: &mut [Option<u8>],
    ) -> u8 {
        if let Some(local) = vertex_map[vertex_idx] {
            local
        } else {
            let local = vertices.len() as u8;
            vertices.push(vertex_idx as u32);
            vertex_map[vertex_idx] = Some(local);
            local
        }
    }

    fn calculate_bounds(&self, vertices: &[[f32; 3]], indices: &[u32]) -> MeshletBounds {
        if indices.is_empty() {
            return MeshletBounds::default();
        }

        let mut min = [f32::MAX; 3];
        let mut max = [f32::MIN; 3];

        for &idx in indices {
            let v = vertices[idx as usize];
            for i in 0..3 {
                min[i] = min[i].min(v[i]);
                max[i] = max[i].max(v[i]);
            }
        }

        MeshletBounds::from_aabb(min, max)
    }
}

impl Default for MeshletGenerator {
    fn default() -> Self {
        Self::new(MeshletConfig::default())
    }
}

// ============================================================================
// Mesh Shader Pipeline
// ============================================================================

/// Mesh shader stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MeshShaderStage {
    /// Task shader (amplification).
    Task,
    /// Mesh shader.
    Mesh,
}

/// Task shader output.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TaskPayload {
    /// Base meshlet index.
    pub base_meshlet: u32,
    /// Meshlet count to dispatch.
    pub meshlet_count: u32,
    /// LOD level.
    pub lod_level: u32,
    /// User data.
    pub user_data: [u32; 13],
}

impl Default for TaskPayload {
    fn default() -> Self {
        Self {
            base_meshlet: 0,
            meshlet_count: 1,
            lod_level: 0,
            user_data: [0; 13],
        }
    }
}

/// Mesh shader output vertex.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MeshOutputVertex {
    /// Position (clip space).
    pub position: [f32; 4],
    /// Normal.
    pub normal: [f32; 3],
    /// Texture coordinates.
    pub texcoord: [f32; 2],
}

impl Default for MeshOutputVertex {
    fn default() -> Self {
        Self {
            position: [0.0; 4],
            normal: [0.0, 0.0, 1.0],
            texcoord: [0.0; 2],
        }
    }
}

/// Mesh shader output primitive.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MeshOutputPrimitive {
    /// Primitive indices.
    pub indices: [u32; 3],
    /// Primitive ID.
    pub primitive_id: u32,
}

impl Default for MeshOutputPrimitive {
    fn default() -> Self {
        Self {
            indices: [0; 3],
            primitive_id: 0,
        }
    }
}

// ============================================================================
// Mesh Shader Limits
// ============================================================================

/// Mesh shader limits.
#[derive(Debug, Clone, Copy)]
pub struct MeshShaderLimits {
    /// Maximum output vertices per mesh shader.
    pub max_output_vertices: u32,
    /// Maximum output primitives per mesh shader.
    pub max_output_primitives: u32,
    /// Maximum task shader workgroup invocations.
    pub max_task_workgroup_invocations: u32,
    /// Maximum mesh shader workgroup invocations.
    pub max_mesh_workgroup_invocations: u32,
    /// Maximum task payload size.
    pub max_task_payload_size: u32,
    /// Maximum task shader total memory size.
    pub max_task_total_memory_size: u32,
    /// Maximum mesh shader total memory size.
    pub max_mesh_total_memory_size: u32,
    /// Maximum preferred task workgroup invocations.
    pub preferred_task_workgroup_invocations: u32,
    /// Maximum preferred mesh workgroup invocations.
    pub preferred_mesh_workgroup_invocations: u32,
}

impl Default for MeshShaderLimits {
    fn default() -> Self {
        Self {
            max_output_vertices: 256,
            max_output_primitives: 256,
            max_task_workgroup_invocations: 128,
            max_mesh_workgroup_invocations: 128,
            max_task_payload_size: 16384,
            max_task_total_memory_size: 32768,
            max_mesh_total_memory_size: 32768,
            preferred_task_workgroup_invocations: 32,
            preferred_mesh_workgroup_invocations: 32,
        }
    }
}

/// Mesh shader features.
#[derive(Debug, Clone, Copy, Default)]
pub struct MeshShaderFeatures {
    /// Task shaders supported.
    pub task_shader: bool,
    /// Mesh shaders supported.
    pub mesh_shader: bool,
    /// Multiview supported in mesh shaders.
    pub multiview_mesh_shader: bool,
    /// Primitive fragment shading rate.
    pub primitive_fragment_shading_rate: bool,
}

// ============================================================================
// Mesh Shader Pipeline Description
// ============================================================================

/// Mesh shader stage description.
#[derive(Debug, Clone)]
pub struct MeshShaderStageDesc {
    /// Stage type.
    pub stage: MeshShaderStage,
    /// Shader module.
    pub module: u64,
    /// Entry point.
    pub entry_point: String,
}

/// Mesh shader pipeline description.
#[derive(Debug, Clone)]
pub struct MeshShaderPipelineDesc {
    /// Debug name.
    pub name: Option<String>,
    /// Task shader (optional).
    pub task_shader: Option<MeshShaderStageDesc>,
    /// Mesh shader (required).
    pub mesh_shader: MeshShaderStageDesc,
    /// Fragment shader.
    pub fragment_shader: Option<MeshShaderStageDesc>,
    /// Pipeline layout.
    pub layout: u64,
    /// Render target formats.
    pub color_formats: Vec<u32>,
    /// Depth format.
    pub depth_format: Option<u32>,
    /// Stencil format.
    pub stencil_format: Option<u32>,
    /// Sample count.
    pub sample_count: u32,
}

impl Default for MeshShaderPipelineDesc {
    fn default() -> Self {
        Self {
            name: None,
            task_shader: None,
            mesh_shader: MeshShaderStageDesc {
                stage: MeshShaderStage::Mesh,
                module: 0,
                entry_point: String::new(),
            },
            fragment_shader: None,
            layout: 0,
            color_formats: Vec::new(),
            depth_format: None,
            stencil_format: None,
            sample_count: 1,
        }
    }
}

// ============================================================================
// Draw Mesh Tasks Command
// ============================================================================

/// Parameters for draw mesh tasks.
#[derive(Debug, Clone, Copy)]
pub struct DrawMeshTasksDesc {
    /// Number of task workgroups X.
    pub group_count_x: u32,
    /// Number of task workgroups Y.
    pub group_count_y: u32,
    /// Number of task workgroups Z.
    pub group_count_z: u32,
}

impl Default for DrawMeshTasksDesc {
    fn default() -> Self {
        Self {
            group_count_x: 1,
            group_count_y: 1,
            group_count_z: 1,
        }
    }
}

/// Parameters for indirect draw mesh tasks.
#[derive(Debug, Clone, Copy)]
pub struct DrawMeshTasksIndirectDesc {
    /// Indirect buffer.
    pub buffer: BufferHandle,
    /// Offset in buffer.
    pub offset: u64,
    /// Draw count.
    pub draw_count: u32,
    /// Stride between commands.
    pub stride: u32,
}

/// Indirect draw mesh tasks command layout.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DrawMeshTasksIndirectCommand {
    /// Group count X.
    pub group_count_x: u32,
    /// Group count Y.
    pub group_count_y: u32,
    /// Group count Z.
    pub group_count_z: u32,
}

impl Default for DrawMeshTasksIndirectCommand {
    fn default() -> Self {
        Self {
            group_count_x: 1,
            group_count_y: 1,
            group_count_z: 1,
        }
    }
}

// ============================================================================
// Mesh Shader Manager
// ============================================================================

/// Mesh shader statistics.
#[derive(Debug, Clone, Copy, Default)]
pub struct MeshShaderStatistics {
    /// Total meshlets processed.
    pub meshlets_processed: u64,
    /// Meshlets culled.
    pub meshlets_culled: u64,
    /// Task shader invocations.
    pub task_invocations: u64,
    /// Mesh shader invocations.
    pub mesh_invocations: u64,
    /// Output vertices.
    pub output_vertices: u64,
    /// Output primitives.
    pub output_primitives: u64,
}

/// Mesh shader manager.
pub struct MeshShaderManager {
    /// Features.
    features: MeshShaderFeatures,
    /// Limits.
    limits: MeshShaderLimits,
    /// Statistics.
    statistics: MeshShaderStatistics,
    /// Meshlet generator.
    generator: MeshletGenerator,
}

impl MeshShaderManager {
    /// Create new manager.
    pub fn new() -> Self {
        Self {
            features: MeshShaderFeatures::default(),
            limits: MeshShaderLimits::default(),
            statistics: MeshShaderStatistics::default(),
            generator: MeshletGenerator::default(),
        }
    }

    /// Initialize with device features.
    pub fn initialize(&mut self, features: MeshShaderFeatures, limits: MeshShaderLimits) {
        self.features = features;
        self.limits = limits;
    }

    /// Check if mesh shaders are supported.
    pub fn is_supported(&self) -> bool {
        self.features.mesh_shader
    }

    /// Check if task shaders are supported.
    pub fn task_shaders_supported(&self) -> bool {
        self.features.task_shader
    }

    /// Get limits.
    pub fn limits(&self) -> &MeshShaderLimits {
        &self.limits
    }

    /// Get features.
    pub fn features(&self) -> &MeshShaderFeatures {
        &self.features
    }

    /// Get statistics.
    pub fn statistics(&self) -> &MeshShaderStatistics {
        &self.statistics
    }

    /// Generate meshlets from mesh.
    pub fn generate_meshlets(&self, vertices: &[[f32; 3]], indices: &[u32]) -> MeshletMesh {
        self.generator.generate(vertices, indices)
    }

    /// Generate meshlets with custom config.
    pub fn generate_meshlets_with_config(
        &self,
        vertices: &[[f32; 3]],
        indices: &[u32],
        config: MeshletConfig,
    ) -> MeshletMesh {
        let generator = MeshletGenerator::new(config);
        generator.generate(vertices, indices)
    }

    /// Calculate optimal workgroup count.
    pub fn calculate_workgroup_count(&self, meshlet_count: u32) -> (u32, u32, u32) {
        let preferred = self.limits.preferred_mesh_workgroup_invocations;
        let x = (meshlet_count + preferred - 1) / preferred;
        (x, 1, 1)
    }

    /// Reset statistics.
    pub fn reset_statistics(&mut self) {
        self.statistics = MeshShaderStatistics::default();
    }

    /// Record meshlet processing.
    pub fn record_processing(&mut self, meshlets: u64, culled: u64) {
        self.statistics.meshlets_processed += meshlets;
        self.statistics.meshlets_culled += culled;
    }

    /// Record shader invocations.
    pub fn record_invocations(&mut self, task: u64, mesh: u64) {
        self.statistics.task_invocations += task;
        self.statistics.mesh_invocations += mesh;
    }

    /// Record output.
    pub fn record_output(&mut self, vertices: u64, primitives: u64) {
        self.statistics.output_vertices += vertices;
        self.statistics.output_primitives += primitives;
    }
}

impl Default for MeshShaderManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// GPU Culling
// ============================================================================

/// Culling flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CullingFlags(u32);

impl CullingFlags {
    /// No culling.
    pub const NONE: Self = Self(0);
    /// Frustum culling.
    pub const FRUSTUM: Self = Self(1 << 0);
    /// Occlusion culling.
    pub const OCCLUSION: Self = Self(1 << 1);
    /// Backface culling.
    pub const BACKFACE: Self = Self(1 << 2);
    /// Small primitive culling.
    pub const SMALL_PRIMITIVE: Self = Self(1 << 3);
    /// Cluster culling.
    pub const CLUSTER: Self = Self(1 << 4);

    /// All culling.
    pub const ALL: Self = Self(0x1F);

    /// Combine flags.
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Check flag.
    pub const fn contains(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }
}

impl Default for CullingFlags {
    fn default() -> Self {
        Self::FRUSTUM.union(Self::BACKFACE)
    }
}

/// GPU culling data.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CullingData {
    /// View-projection matrix.
    pub view_proj: [[f32; 4]; 4],
    /// Camera position.
    pub camera_pos: [f32; 3],
    /// Near plane.
    pub near_plane: f32,
    /// Frustum planes (6 planes, xyz = normal, w = distance).
    pub frustum_planes: [[f32; 4]; 6],
    /// Screen size.
    pub screen_size: [f32; 2],
    /// Culling flags.
    pub flags: u32,
    /// LOD bias.
    pub lod_bias: f32,
}

impl Default for CullingData {
    fn default() -> Self {
        Self {
            view_proj: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            camera_pos: [0.0; 3],
            near_plane: 0.1,
            frustum_planes: [[0.0; 4]; 6],
            screen_size: [1920.0, 1080.0],
            flags: CullingFlags::default().0,
            lod_bias: 0.0,
        }
    }
}

/// Visibility buffer entry.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VisibilityEntry {
    /// Meshlet index.
    pub meshlet_index: u32,
    /// Instance index.
    pub instance_index: u32,
    /// LOD level.
    pub lod_level: u32,
    /// Flags.
    pub flags: u32,
}

impl Default for VisibilityEntry {
    fn default() -> Self {
        Self {
            meshlet_index: 0,
            instance_index: 0,
            lod_level: 0,
            flags: 0,
        }
    }
}
