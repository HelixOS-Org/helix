//! Meshlet System
//!
//! GPU-driven meshlet-based rendering system for efficient mesh shader
//! pipelines. Meshlets are small fixed-size clusters of triangles that
//! enable fine-grained GPU culling and optimal cache utilization.

use alloc::{string::String, vec::Vec};
use crate::mesh::{Mesh, MeshHandle, AABB, BoundingSphere, Vertex};

// ============================================================================
// Meshlet Constants
// ============================================================================

/// Maximum vertices per meshlet.
pub const MESHLET_MAX_VERTICES: usize = 64;

/// Maximum triangles per meshlet.
pub const MESHLET_MAX_TRIANGLES: usize = 126;

/// Maximum primitives per meshlet (triangles * 3 indices).
pub const MESHLET_MAX_PRIMITIVE_INDICES: usize = MESHLET_MAX_TRIANGLES * 3;

// ============================================================================
// Meshlet Data
// ============================================================================

/// A single meshlet - a small cluster of triangles.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct Meshlet {
    /// Offset into the vertex index buffer.
    pub vertex_offset: u32,
    /// Number of vertices.
    pub vertex_count: u8,
    /// Offset into the primitive index buffer.
    pub triangle_offset: u32,
    /// Number of triangles.
    pub triangle_count: u8,
}

impl Meshlet {
    /// Create a new meshlet.
    pub fn new() -> Self {
        Self {
            vertex_offset: 0,
            vertex_count: 0,
            triangle_offset: 0,
            triangle_count: 0,
        }
    }

    /// Check if meshlet has room for more vertices.
    pub fn can_add_vertex(&self) -> bool {
        (self.vertex_count as usize) < MESHLET_MAX_VERTICES
    }

    /// Check if meshlet has room for more triangles.
    pub fn can_add_triangle(&self) -> bool {
        (self.triangle_count as usize) < MESHLET_MAX_TRIANGLES
    }

    /// Check if meshlet is empty.
    pub fn is_empty(&self) -> bool {
        self.vertex_count == 0 || self.triangle_count == 0
    }
}

impl Default for Meshlet {
    fn default() -> Self {
        Self::new()
    }
}

/// Meshlet bounding information for culling.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct MeshletBounds {
    /// Center of bounding sphere.
    pub center: [f32; 3],
    /// Radius of bounding sphere.
    pub radius: f32,
    /// Cone apex (for backface culling).
    pub cone_apex: [f32; 3],
    /// Cone axis (normalized).
    pub cone_axis: [f32; 3],
    /// Cone cutoff (dot product threshold).
    pub cone_cutoff: f32,
    /// AABB min.
    pub aabb_min: [f32; 3],
    /// AABB max.
    pub aabb_max: [f32; 3],
}

impl MeshletBounds {
    /// Create from vertices.
    pub fn from_vertices(vertices: &[Vertex]) -> Self {
        if vertices.is_empty() {
            return Self::default();
        }

        // Calculate AABB
        let mut aabb_min = [f32::MAX; 3];
        let mut aabb_max = [f32::MIN; 3];
        let mut normal_sum = [0.0f32; 3];

        for v in vertices {
            for i in 0..3 {
                aabb_min[i] = aabb_min[i].min(v.position[i]);
                aabb_max[i] = aabb_max[i].max(v.position[i]);
            }
            normal_sum[0] += v.normal[0];
            normal_sum[1] += v.normal[1];
            normal_sum[2] += v.normal[2];
        }

        // Calculate bounding sphere
        let center = [
            (aabb_min[0] + aabb_max[0]) * 0.5,
            (aabb_min[1] + aabb_max[1]) * 0.5,
            (aabb_min[2] + aabb_max[2]) * 0.5,
        ];

        let mut radius = 0.0f32;
        for v in vertices {
            let dx = v.position[0] - center[0];
            let dy = v.position[1] - center[1];
            let dz = v.position[2] - center[2];
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            radius = radius.max(dist);
        }

        // Calculate cone for backface culling
        let len = (normal_sum[0] * normal_sum[0] + normal_sum[1] * normal_sum[1] + normal_sum[2] * normal_sum[2]).sqrt();
        let cone_axis = if len > 0.0 {
            [normal_sum[0] / len, normal_sum[1] / len, normal_sum[2] / len]
        } else {
            [0.0, 1.0, 0.0]
        };

        // Find minimum dot product (maximum angle from average normal)
        let mut min_dot = 1.0f32;
        for v in vertices {
            let dot = v.normal[0] * cone_axis[0] + v.normal[1] * cone_axis[1] + v.normal[2] * cone_axis[2];
            min_dot = min_dot.min(dot);
        }

        // Cone apex is center offset in opposite direction of cone axis
        let cone_apex = [
            center[0] - cone_axis[0] * radius,
            center[1] - cone_axis[1] * radius,
            center[2] - cone_axis[2] * radius,
        ];

        Self {
            center,
            radius,
            cone_apex,
            cone_axis,
            cone_cutoff: min_dot,
            aabb_min,
            aabb_max,
        }
    }

    /// Test if meshlet is potentially visible from a viewpoint.
    pub fn is_visible(&self, view_pos: [f32; 3], frustum_planes: &[[f32; 4]; 6]) -> bool {
        // Frustum culling against bounding sphere
        for plane in frustum_planes {
            let dist = plane[0] * self.center[0] + plane[1] * self.center[1] + plane[2] * self.center[2] + plane[3];
            if dist < -self.radius {
                return false;
            }
        }

        // Backface cone culling
        if self.cone_cutoff < 1.0 {
            let dx = view_pos[0] - self.cone_apex[0];
            let dy = view_pos[1] - self.cone_apex[1];
            let dz = view_pos[2] - self.cone_apex[2];
            let len = (dx * dx + dy * dy + dz * dz).sqrt();
            if len > 0.0 {
                let dir = [dx / len, dy / len, dz / len];
                let dot = dir[0] * self.cone_axis[0] + dir[1] * self.cone_axis[1] + dir[2] * self.cone_axis[2];
                if dot < self.cone_cutoff {
                    return false;
                }
            }
        }

        true
    }
}

/// GPU-ready meshlet data.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct GpuMeshlet {
    /// Vertex offset in global vertex buffer.
    pub vertex_offset: u32,
    /// Primitive offset in global primitive buffer.
    pub triangle_offset: u32,
    /// Vertex count.
    pub vertex_count: u32,
    /// Triangle count.
    pub triangle_count: u32,
}

impl GpuMeshlet {
    /// Size in bytes.
    pub const SIZE: usize = 16;

    /// Create from meshlet.
    pub fn from_meshlet(meshlet: &Meshlet) -> Self {
        Self {
            vertex_offset: meshlet.vertex_offset,
            triangle_offset: meshlet.triangle_offset,
            vertex_count: meshlet.vertex_count as u32,
            triangle_count: meshlet.triangle_count as u32,
        }
    }
}

/// GPU-ready meshlet bounds.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct GpuMeshletBounds {
    /// Center (xyz) and radius (w).
    pub sphere: [f32; 4],
    /// Cone apex.
    pub cone_apex: [f32; 4],
    /// Cone axis (xyz) and cutoff (w).
    pub cone: [f32; 4],
}

impl GpuMeshletBounds {
    /// Size in bytes.
    pub const SIZE: usize = 48;

    /// Create from bounds.
    pub fn from_bounds(bounds: &MeshletBounds) -> Self {
        Self {
            sphere: [bounds.center[0], bounds.center[1], bounds.center[2], bounds.radius],
            cone_apex: [bounds.cone_apex[0], bounds.cone_apex[1], bounds.cone_apex[2], 0.0],
            cone: [bounds.cone_axis[0], bounds.cone_axis[1], bounds.cone_axis[2], bounds.cone_cutoff],
        }
    }
}

// ============================================================================
// Meshlet Data Container
// ============================================================================

/// Container for all meshlet data of a mesh.
#[derive(Debug, Clone)]
pub struct MeshletData {
    /// Meshlets.
    pub meshlets: Vec<Meshlet>,
    /// Meshlet bounds.
    pub bounds: Vec<MeshletBounds>,
    /// Vertex indices (local to global mapping).
    pub vertex_indices: Vec<u32>,
    /// Primitive indices (3 u8 per triangle).
    pub primitive_indices: Vec<u8>,
}

impl MeshletData {
    /// Create empty meshlet data.
    pub fn new() -> Self {
        Self {
            meshlets: Vec::new(),
            bounds: Vec::new(),
            vertex_indices: Vec::new(),
            primitive_indices: Vec::new(),
        }
    }

    /// Get meshlet count.
    pub fn meshlet_count(&self) -> usize {
        self.meshlets.len()
    }

    /// Get total vertex index count.
    pub fn vertex_index_count(&self) -> usize {
        self.vertex_indices.len()
    }

    /// Get total primitive count.
    pub fn primitive_count(&self) -> usize {
        self.primitive_indices.len() / 3
    }

    /// Calculate memory usage.
    pub fn memory_usage(&self) -> usize {
        self.meshlets.len() * core::mem::size_of::<Meshlet>()
            + self.bounds.len() * core::mem::size_of::<MeshletBounds>()
            + self.vertex_indices.len() * 4
            + self.primitive_indices.len()
    }
}

impl Default for MeshletData {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Meshlet Mesh
// ============================================================================

/// A mesh that has been processed into meshlets.
pub struct MeshletMesh {
    /// Original mesh handle.
    source_mesh: MeshHandle,
    /// Name.
    name: String,
    /// Meshlet data.
    data: MeshletData,
    /// Total bounding box.
    bounds: AABB,
    /// Total bounding sphere.
    sphere: BoundingSphere,
    /// Statistics.
    stats: MeshletStats,
    /// GPU buffers (platform-specific handles).
    gpu_meshlet_buffer: u64,
    gpu_bounds_buffer: u64,
    gpu_vertex_indices: u64,
    gpu_primitive_indices: u64,
}

/// Meshlet statistics.
#[derive(Debug, Clone, Default)]
pub struct MeshletStats {
    /// Total meshlet count.
    pub meshlet_count: u32,
    /// Total triangle count.
    pub triangle_count: u32,
    /// Total vertex count.
    pub vertex_count: u32,
    /// Average triangles per meshlet.
    pub avg_triangles_per_meshlet: f32,
    /// Average vertices per meshlet.
    pub avg_vertices_per_meshlet: f32,
    /// Memory usage in bytes.
    pub memory_bytes: u64,
}

impl MeshletMesh {
    /// Create from meshlet data.
    pub fn new(source: MeshHandle, name: impl Into<String>, data: MeshletData) -> Self {
        let mut bounds = AABB::INVALID;
        for b in &data.bounds {
            bounds.expand_aabb(&AABB::new(b.aabb_min, b.aabb_max));
        }
        let sphere = BoundingSphere::from_aabb(&bounds);

        let triangle_count: u32 = data.meshlets.iter().map(|m| m.triangle_count as u32).sum();
        let vertex_count: u32 = data.meshlets.iter().map(|m| m.vertex_count as u32).sum();
        let meshlet_count = data.meshlets.len() as u32;

        let stats = MeshletStats {
            meshlet_count,
            triangle_count,
            vertex_count,
            avg_triangles_per_meshlet: if meshlet_count > 0 {
                triangle_count as f32 / meshlet_count as f32
            } else {
                0.0
            },
            avg_vertices_per_meshlet: if meshlet_count > 0 {
                vertex_count as f32 / meshlet_count as f32
            } else {
                0.0
            },
            memory_bytes: data.memory_usage() as u64,
        };

        Self {
            source_mesh: source,
            name: name.into(),
            data,
            bounds,
            sphere,
            stats,
            gpu_meshlet_buffer: 0,
            gpu_bounds_buffer: 0,
            gpu_vertex_indices: 0,
            gpu_primitive_indices: 0,
        }
    }

    /// Get source mesh.
    pub fn source_mesh(&self) -> MeshHandle {
        self.source_mesh
    }

    /// Get name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get meshlet data.
    pub fn data(&self) -> &MeshletData {
        &self.data
    }

    /// Get bounds.
    pub fn bounds(&self) -> &AABB {
        &self.bounds
    }

    /// Get bounding sphere.
    pub fn bounding_sphere(&self) -> &BoundingSphere {
        &self.sphere
    }

    /// Get statistics.
    pub fn stats(&self) -> &MeshletStats {
        &self.stats
    }

    /// Get meshlet count.
    pub fn meshlet_count(&self) -> usize {
        self.data.meshlets.len()
    }

    /// Get meshlets.
    pub fn meshlets(&self) -> &[Meshlet] {
        &self.data.meshlets
    }

    /// Get meshlet bounds.
    pub fn meshlet_bounds(&self) -> &[MeshletBounds] {
        &self.data.bounds
    }

    /// Set GPU buffer handles.
    pub fn set_gpu_buffers(&mut self, meshlets: u64, bounds: u64, vertices: u64, primitives: u64) {
        self.gpu_meshlet_buffer = meshlets;
        self.gpu_bounds_buffer = bounds;
        self.gpu_vertex_indices = vertices;
        self.gpu_primitive_indices = primitives;
    }

    /// Cull meshlets against frustum.
    pub fn cull(&self, view_pos: [f32; 3], frustum: &[[f32; 4]; 6]) -> Vec<u32> {
        let mut visible = Vec::with_capacity(self.data.meshlets.len());

        for (i, bounds) in self.data.bounds.iter().enumerate() {
            if bounds.is_visible(view_pos, frustum) {
                visible.push(i as u32);
            }
        }

        visible
    }
}

// ============================================================================
// Meshlet Generator
// ============================================================================

/// Configuration for meshlet generation.
#[derive(Debug, Clone)]
pub struct MeshletConfig {
    /// Maximum vertices per meshlet.
    pub max_vertices: usize,
    /// Maximum triangles per meshlet.
    pub max_triangles: usize,
    /// Optimize for cache efficiency.
    pub optimize_cache: bool,
    /// Optimize for overdraw.
    pub optimize_overdraw: bool,
    /// Generate culling cones.
    pub generate_cones: bool,
}

impl Default for MeshletConfig {
    fn default() -> Self {
        Self {
            max_vertices: MESHLET_MAX_VERTICES,
            max_triangles: MESHLET_MAX_TRIANGLES,
            optimize_cache: true,
            optimize_overdraw: true,
            generate_cones: true,
        }
    }
}

/// Meshlet generator.
pub struct MeshletGenerator {
    /// Configuration.
    config: MeshletConfig,
}

impl MeshletGenerator {
    /// Create a new generator.
    pub fn new(config: MeshletConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(MeshletConfig::default())
    }

    /// Generate meshlets from a mesh.
    pub fn generate(&self, mesh: &Mesh) -> MeshletMesh {
        let data = mesh.data();
        let vertex_count = data.vertex_count();
        let index_count = data.index_count();

        if index_count == 0 {
            return MeshletMesh::new(mesh.handle(), mesh.name(), MeshletData::new());
        }

        // Get indices
        let mut indices = Vec::with_capacity(index_count);
        for i in 0..index_count {
            if let Some(idx) = data.indices.get(i) {
                indices.push(idx);
            }
        }

        // Build adjacency information for optimized meshlet building
        let adjacency = self.build_adjacency(&indices, vertex_count);

        // Generate meshlets using greedy algorithm
        let meshlet_data = self.generate_meshlets(&indices, &adjacency, data);

        MeshletMesh::new(mesh.handle(), mesh.name(), meshlet_data)
    }

    /// Build adjacency information.
    fn build_adjacency(&self, indices: &[u32], _vertex_count: usize) -> Vec<Vec<u32>> {
        let triangle_count = indices.len() / 3;
        let mut adjacency = vec![Vec::new(); triangle_count];

        // Build triangle-to-triangle adjacency based on shared edges
        for i in 0..triangle_count {
            let i0 = indices[i * 3] as u32;
            let i1 = indices[i * 3 + 1] as u32;
            let i2 = indices[i * 3 + 2] as u32;

            for j in (i + 1)..triangle_count {
                let j0 = indices[j * 3];
                let j1 = indices[j * 3 + 1];
                let j2 = indices[j * 3 + 2];

                // Check for shared edge
                let shared = Self::count_shared_vertices([i0, i1, i2], [j0, j1, j2]);
                if shared >= 2 {
                    adjacency[i].push(j as u32);
                    adjacency[j].push(i as u32);
                }
            }
        }

        adjacency
    }

    /// Count shared vertices between two triangles.
    fn count_shared_vertices(t1: [u32; 3], t2: [u32; 3]) -> usize {
        let mut count = 0;
        for v1 in t1 {
            for v2 in t2 {
                if v1 == v2 {
                    count += 1;
                }
            }
        }
        count
    }

    /// Generate meshlets using greedy algorithm.
    fn generate_meshlets(
        &self,
        indices: &[u32],
        adjacency: &[Vec<u32>],
        mesh_data: &crate::mesh::MeshData,
    ) -> MeshletData {
        let triangle_count = indices.len() / 3;
        let mut used = vec![false; triangle_count];
        let mut meshlets = Vec::new();
        let mut bounds = Vec::new();
        let mut vertex_indices = Vec::new();
        let mut primitive_indices = Vec::new();

        while meshlets.len() < triangle_count {
            // Find unused triangle to start new meshlet
            let start = used.iter().position(|&u| !u);
            if start.is_none() {
                break;
            }
            let start = start.unwrap();

            // Build meshlet
            let mut meshlet = Meshlet::new();
            meshlet.vertex_offset = vertex_indices.len() as u32;
            meshlet.triangle_offset = primitive_indices.len() as u32;

            let mut local_vertices: Vec<u32> = Vec::new();
            let mut vertex_map = alloc::collections::BTreeMap::new();
            let mut triangles: Vec<usize> = Vec::new();

            // Add triangles greedily
            let mut queue = alloc::vec![start];

            while let Some(tri) = queue.pop() {
                if used[tri] {
                    continue;
                }

                // Check if triangle fits in meshlet
                let t0 = indices[tri * 3];
                let t1 = indices[tri * 3 + 1];
                let t2 = indices[tri * 3 + 2];

                let new_verts = [t0, t1, t2]
                    .iter()
                    .filter(|&&v| !vertex_map.contains_key(&v))
                    .count();

                if local_vertices.len() + new_verts > self.config.max_vertices {
                    continue;
                }
                if triangles.len() >= self.config.max_triangles {
                    break;
                }

                // Add triangle
                used[tri] = true;
                triangles.push(tri);

                // Add vertices
                for &v in &[t0, t1, t2] {
                    if !vertex_map.contains_key(&v) {
                        let local_idx = local_vertices.len();
                        vertex_map.insert(v, local_idx);
                        local_vertices.push(v);
                    }
                }

                // Add adjacent triangles to queue
                for &adj in &adjacency[tri] {
                    if !used[adj as usize] {
                        queue.push(adj as usize);
                    }
                }
            }

            if triangles.is_empty() {
                // Mark as used to prevent infinite loop
                used[start] = true;
                continue;
            }

            // Write meshlet data
            meshlet.vertex_count = local_vertices.len() as u8;
            meshlet.triangle_count = triangles.len() as u8;

            // Add vertex indices
            vertex_indices.extend_from_slice(&local_vertices);

            // Add primitive indices (local triangle indices)
            for &tri in &triangles {
                let t0 = indices[tri * 3];
                let t1 = indices[tri * 3 + 1];
                let t2 = indices[tri * 3 + 2];

                primitive_indices.push(*vertex_map.get(&t0).unwrap() as u8);
                primitive_indices.push(*vertex_map.get(&t1).unwrap() as u8);
                primitive_indices.push(*vertex_map.get(&t2).unwrap() as u8);
            }

            // Calculate bounds
            let meshlet_vertices: Vec<Vertex> = local_vertices
                .iter()
                .filter_map(|&idx| mesh_data.get_vertex::<Vertex>(idx as usize).copied())
                .collect();
            bounds.push(MeshletBounds::from_vertices(&meshlet_vertices));

            meshlets.push(meshlet);
        }

        MeshletData {
            meshlets,
            bounds,
            vertex_indices,
            primitive_indices,
        }
    }
}

// ============================================================================
// Meshlet Draw Commands
// ============================================================================

/// Draw command for mesh shader dispatch.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct MeshletDrawCommand {
    /// First meshlet index.
    pub first_meshlet: u32,
    /// Meshlet count.
    pub meshlet_count: u32,
    /// Instance index.
    pub instance_id: u32,
    /// Material index.
    pub material_id: u32,
}

/// Indirect dispatch arguments for mesh shaders.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct MeshletDispatchIndirect {
    /// Task shader workgroups X.
    pub group_count_x: u32,
    /// Task shader workgroups Y.
    pub group_count_y: u32,
    /// Task shader workgroups Z.
    pub group_count_z: u32,
}

impl MeshletDispatchIndirect {
    /// Create for a number of meshlets.
    pub fn for_meshlets(meshlet_count: u32, meshlets_per_group: u32) -> Self {
        Self {
            group_count_x: (meshlet_count + meshlets_per_group - 1) / meshlets_per_group,
            group_count_y: 1,
            group_count_z: 1,
        }
    }
}
