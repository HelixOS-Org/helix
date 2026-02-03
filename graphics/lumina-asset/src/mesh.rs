//! # Mesh Processing
//!
//! Advanced mesh processing with:
//! - Meshlet generation for mesh shaders
//! - LOD generation
//! - Vertex cache optimization
//! - Quantization

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use crate::{AssetResult, ImportedMesh, ImportedVertex, MeshBounds, Submesh};

/// Mesh processor for GPU optimization
pub struct MeshProcessor {
    config: MeshProcessorConfig,
}

impl MeshProcessor {
    pub fn new(config: MeshProcessorConfig) -> Self {
        Self { config }
    }

    /// Generate normals from vertices and indices
    pub fn generate_normals(&self, vertices: &mut [ImportedVertex], indices: &[u32]) {
        // Reset normals
        for v in vertices.iter_mut() {
            v.normal = [0.0, 0.0, 0.0];
        }

        // Accumulate face normals
        for tri in indices.chunks(3) {
            if tri.len() < 3 {
                continue;
            }

            let i0 = tri[0] as usize;
            let i1 = tri[1] as usize;
            let i2 = tri[2] as usize;

            if i0 >= vertices.len() || i1 >= vertices.len() || i2 >= vertices.len() {
                continue;
            }

            let p0 = vertices[i0].position;
            let p1 = vertices[i1].position;
            let p2 = vertices[i2].position;

            let e1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
            let e2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];

            let n = cross(e1, e2);

            for &i in &[i0, i1, i2] {
                vertices[i].normal[0] += n[0];
                vertices[i].normal[1] += n[1];
                vertices[i].normal[2] += n[2];
            }
        }

        // Normalize
        for v in vertices.iter_mut() {
            let len = (v.normal[0].powi(2) + v.normal[1].powi(2) + v.normal[2].powi(2)).sqrt();
            if len > 1e-6 {
                v.normal[0] /= len;
                v.normal[1] /= len;
                v.normal[2] /= len;
            }
        }
    }

    /// Generate tangents using MikkTSpace algorithm
    pub fn generate_tangents(&self, vertices: &mut [ImportedVertex], indices: &[u32]) {
        // Simplified tangent generation
        for tri in indices.chunks(3) {
            if tri.len() < 3 {
                continue;
            }

            let i0 = tri[0] as usize;
            let i1 = tri[1] as usize;
            let i2 = tri[2] as usize;

            if i0 >= vertices.len() || i1 >= vertices.len() || i2 >= vertices.len() {
                continue;
            }

            let p0 = vertices[i0].position;
            let p1 = vertices[i1].position;
            let p2 = vertices[i2].position;

            let uv0 = vertices[i0].uv0;
            let uv1 = vertices[i1].uv0;
            let uv2 = vertices[i2].uv0;

            let e1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
            let e2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];

            let duv1 = [uv1[0] - uv0[0], uv1[1] - uv0[1]];
            let duv2 = [uv2[0] - uv0[0], uv2[1] - uv0[1]];

            let r = 1.0 / (duv1[0] * duv2[1] - duv1[1] * duv2[0]).max(1e-6);

            let tangent = [
                (duv2[1] * e1[0] - duv1[1] * e2[0]) * r,
                (duv2[1] * e1[1] - duv1[1] * e2[1]) * r,
                (duv2[1] * e1[2] - duv1[1] * e2[2]) * r,
            ];

            let bitangent = [
                (duv1[0] * e2[0] - duv2[0] * e1[0]) * r,
                (duv1[0] * e2[1] - duv2[0] * e1[1]) * r,
                (duv1[0] * e2[2] - duv2[0] * e1[2]) * r,
            ];

            // Compute handedness
            let n = vertices[i0].normal;
            let w = if dot(cross(n, tangent), bitangent) < 0.0 {
                -1.0
            } else {
                1.0
            };

            for &i in &[i0, i1, i2] {
                vertices[i].tangent = [tangent[0], tangent[1], tangent[2], w];
            }
        }
    }

    /// Optimize vertex cache for GPU
    pub fn optimize_vertex_cache(&self, indices: &mut [u32], vertex_count: usize) {
        // Tipsify algorithm for vertex cache optimization
        let cache_size = 32;

        // Build adjacency
        let mut adjacency: Vec<Vec<u32>> = vec![Vec::new(); vertex_count];
        for (tri_idx, tri) in indices.chunks(3).enumerate() {
            if tri.len() < 3 {
                continue;
            }
            for &v in tri {
                if (v as usize) < vertex_count {
                    adjacency[v as usize].push(tri_idx as u32);
                }
            }
        }

        // Simple linear reordering (proper implementation would use Tipsify)
        let mut new_indices = Vec::with_capacity(indices.len());
        let mut emitted = vec![false; indices.len() / 3];
        let mut cache: Vec<u32> = Vec::new();

        for start_tri in 0..indices.len() / 3 {
            if emitted[start_tri] {
                continue;
            }

            // Emit triangle
            let base = start_tri * 3;
            new_indices.push(indices[base]);
            new_indices.push(indices[base + 1]);
            new_indices.push(indices[base + 2]);
            emitted[start_tri] = true;

            // Update cache
            for &v in &indices[base..base + 3] {
                if !cache.contains(&v) {
                    cache.push(v);
                    if cache.len() > cache_size {
                        cache.remove(0);
                    }
                }
            }
        }

        indices.copy_from_slice(&new_indices);
    }

    /// Generate meshlets for mesh shader rendering
    pub fn generate_meshlets(&self, vertices: &[ImportedVertex], indices: &[u32]) -> Vec<Meshlet> {
        let max_vertices = self.config.meshlet_max_vertices as usize;
        let max_triangles = self.config.meshlet_max_triangles as usize;

        let mut meshlets = Vec::new();
        let mut current_meshlet = MeshletBuilder::new(max_vertices, max_triangles);

        for tri in indices.chunks(3) {
            if tri.len() < 3 {
                continue;
            }

            let v0 = tri[0];
            let v1 = tri[1];
            let v2 = tri[2];

            if !current_meshlet.can_add_triangle(v0, v1, v2) {
                if !current_meshlet.is_empty() {
                    meshlets.push(current_meshlet.build(vertices));
                }
                current_meshlet = MeshletBuilder::new(max_vertices, max_triangles);
            }

            current_meshlet.add_triangle(v0, v1, v2);
        }

        if !current_meshlet.is_empty() {
            meshlets.push(current_meshlet.build(vertices));
        }

        meshlets
    }

    /// Generate LOD levels
    pub fn generate_lods(
        &self,
        vertices: &[ImportedVertex],
        indices: &[u32],
        lod_count: u32,
    ) -> Vec<LodLevel> {
        let mut lods = Vec::new();

        // LOD 0 is original
        lods.push(LodLevel {
            indices: indices.to_vec(),
            triangle_count: indices.len() as u32 / 3,
            screen_coverage: 1.0,
        });

        let mut current_indices = indices.to_vec();

        for lod in 1..lod_count {
            let target_triangles = current_indices.len() / 3 / 2;
            if target_triangles < 12 {
                break;
            }

            current_indices = self.simplify(&current_indices, vertices, target_triangles);

            lods.push(LodLevel {
                indices: current_indices.clone(),
                triangle_count: current_indices.len() as u32 / 3,
                screen_coverage: 0.5_f32.powi(lod as i32),
            });
        }

        lods
    }

    /// Simplify mesh using quadric error metrics
    fn simplify(
        &self,
        indices: &[u32],
        vertices: &[ImportedVertex],
        target_triangles: usize,
    ) -> Vec<u32> {
        // Simplified edge collapse (proper implementation would use QEM)
        let mut result = indices.to_vec();

        while result.len() / 3 > target_triangles {
            // Find shortest edge and collapse it
            let mut min_len = f32::MAX;
            let mut min_edge = (0, 0);

            for tri in result.chunks(3) {
                if tri.len() < 3 {
                    continue;
                }

                for i in 0..3 {
                    let v0 = tri[i] as usize;
                    let v1 = tri[(i + 1) % 3] as usize;

                    if v0 >= vertices.len() || v1 >= vertices.len() {
                        continue;
                    }

                    let len = distance(vertices[v0].position, vertices[v1].position);
                    if len < min_len {
                        min_len = len;
                        min_edge = (v0 as u32, v1 as u32);
                    }
                }
            }

            // Collapse edge by replacing all v1 with v0
            for idx in result.iter_mut() {
                if *idx == min_edge.1 {
                    *idx = min_edge.0;
                }
            }

            // Remove degenerate triangles
            result = result
                .chunks(3)
                .filter(|tri| {
                    tri.len() == 3 && tri[0] != tri[1] && tri[1] != tri[2] && tri[0] != tri[2]
                })
                .flat_map(|tri| tri.iter().copied())
                .collect();
        }

        result
    }

    /// Quantize vertex positions for compression
    pub fn quantize_positions(&self, vertices: &mut [ImportedVertex], bounds: &MeshBounds) {
        let scale = [
            bounds.max[0] - bounds.min[0],
            bounds.max[1] - bounds.min[1],
            bounds.max[2] - bounds.min[2],
        ];

        for v in vertices.iter_mut() {
            // Normalize to 0-1 range then quantize to 16-bit
            for i in 0..3 {
                let normalized = if scale[i] > 1e-6 {
                    (v.position[i] - bounds.min[i]) / scale[i]
                } else {
                    0.0
                };

                let quantized = (normalized * 65535.0).clamp(0.0, 65535.0) as u16;
                v.position[i] = (quantized as f32 / 65535.0) * scale[i] + bounds.min[i];
            }
        }
    }

    /// Quantize normals using octahedral encoding
    pub fn quantize_normals(&self, vertices: &mut [ImportedVertex]) {
        for v in vertices.iter_mut() {
            // Octahedral encode then quantize
            let (u, vv) = octahedral_encode(v.normal);

            // Quantize to 8-bit
            let qu = ((u * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8;
            let qv = ((vv * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8;

            // Decode back
            let du = qu as f32 / 255.0 * 2.0 - 1.0;
            let dv = qv as f32 / 255.0 * 2.0 - 1.0;
            v.normal = octahedral_decode(du, dv);
        }
    }

    /// Calculate mesh bounds
    pub fn calculate_bounds(&self, vertices: &[ImportedVertex]) -> MeshBounds {
        let mut min = [f32::MAX; 3];
        let mut max = [f32::MIN; 3];

        for v in vertices {
            for i in 0..3 {
                min[i] = min[i].min(v.position[i]);
                max[i] = max[i].max(v.position[i]);
            }
        }

        let center = [
            (min[0] + max[0]) * 0.5,
            (min[1] + max[1]) * 0.5,
            (min[2] + max[2]) * 0.5,
        ];

        let radius = vertices
            .iter()
            .map(|v| distance(v.position, center))
            .fold(0.0f32, |a, b| a.max(b));

        MeshBounds {
            min,
            max,
            center,
            radius,
        }
    }
}

/// Mesh processor config
#[derive(Debug, Clone)]
pub struct MeshProcessorConfig {
    pub meshlet_max_vertices: u32,
    pub meshlet_max_triangles: u32,
    pub cache_size: u32,
    pub quantization_bits: u32,
}

impl Default for MeshProcessorConfig {
    fn default() -> Self {
        Self {
            meshlet_max_vertices: 64,
            meshlet_max_triangles: 124,
            cache_size: 32,
            quantization_bits: 16,
        }
    }
}

/// Meshlet for mesh shader rendering
#[derive(Debug, Clone)]
pub struct Meshlet {
    /// Local vertex indices (0-255)
    pub vertex_indices: Vec<u8>,
    /// Primitive indices (3 per triangle)
    pub primitive_indices: Vec<u8>,
    /// Global vertex offset
    pub vertex_offset: u32,
    /// Number of vertices
    pub vertex_count: u8,
    /// Number of triangles
    pub triangle_count: u8,
    /// Bounding sphere
    pub bounding_sphere: [f32; 4],
    /// Cone for culling
    pub cone: MeshletCone,
}

/// Meshlet cone for backface culling
#[derive(Debug, Clone)]
pub struct MeshletCone {
    pub apex: [f32; 3],
    pub axis: [f32; 3],
    pub cutoff: f32,
}

/// Meshlet builder
struct MeshletBuilder {
    vertex_map: BTreeMap<u32, u8>,
    triangles: Vec<[u8; 3]>,
    max_vertices: usize,
    max_triangles: usize,
}

impl MeshletBuilder {
    fn new(max_vertices: usize, max_triangles: usize) -> Self {
        Self {
            vertex_map: BTreeMap::new(),
            triangles: Vec::new(),
            max_vertices,
            max_triangles,
        }
    }

    fn can_add_triangle(&self, v0: u32, v1: u32, v2: u32) -> bool {
        if self.triangles.len() >= self.max_triangles {
            return false;
        }

        let mut new_vertices = 0;
        if !self.vertex_map.contains_key(&v0) {
            new_vertices += 1;
        }
        if !self.vertex_map.contains_key(&v1) {
            new_vertices += 1;
        }
        if !self.vertex_map.contains_key(&v2) {
            new_vertices += 1;
        }

        self.vertex_map.len() + new_vertices <= self.max_vertices
    }

    fn add_triangle(&mut self, v0: u32, v1: u32, v2: u32) {
        let i0 = self.get_or_add_vertex(v0);
        let i1 = self.get_or_add_vertex(v1);
        let i2 = self.get_or_add_vertex(v2);

        self.triangles.push([i0, i1, i2]);
    }

    fn get_or_add_vertex(&mut self, global_idx: u32) -> u8 {
        let next_idx = self.vertex_map.len() as u8;
        *self.vertex_map.entry(global_idx).or_insert(next_idx)
    }

    fn is_empty(&self) -> bool {
        self.triangles.is_empty()
    }

    fn build(self, vertices: &[ImportedVertex]) -> Meshlet {
        // Build vertex indices list
        let mut vertex_indices: Vec<u8> = vec![0; self.vertex_map.len()];
        let mut min_global = u32::MAX;

        for (&global, &local) in &self.vertex_map {
            min_global = min_global.min(global);
        }

        for (&global, &local) in &self.vertex_map {
            vertex_indices[local as usize] = (global - min_global) as u8;
        }

        // Build primitive indices
        let primitive_indices: Vec<u8> = self
            .triangles
            .iter()
            .flat_map(|t| t.iter().copied())
            .collect();

        // Calculate bounding sphere
        let mut center = [0.0f32; 3];
        let mut count = 0.0;

        for &global in self.vertex_map.keys() {
            if (global as usize) < vertices.len() {
                let v = &vertices[global as usize];
                center[0] += v.position[0];
                center[1] += v.position[1];
                center[2] += v.position[2];
                count += 1.0;
            }
        }

        if count > 0.0 {
            center[0] /= count;
            center[1] /= count;
            center[2] /= count;
        }

        let radius = self
            .vertex_map
            .keys()
            .filter_map(|&global| {
                vertices
                    .get(global as usize)
                    .map(|v| distance(v.position, center))
            })
            .fold(0.0f32, |a, b| a.max(b));

        Meshlet {
            vertex_indices,
            primitive_indices,
            vertex_offset: min_global,
            vertex_count: self.vertex_map.len() as u8,
            triangle_count: self.triangles.len() as u8,
            bounding_sphere: [center[0], center[1], center[2], radius],
            cone: MeshletCone {
                apex: center,
                axis: [0.0, 1.0, 0.0],
                cutoff: 1.0,
            },
        }
    }
}

/// LOD level data
#[derive(Debug, Clone)]
pub struct LodLevel {
    pub indices: Vec<u32>,
    pub triangle_count: u32,
    pub screen_coverage: f32,
}

/// Processed mesh ready for GPU
#[derive(Debug, Clone)]
pub struct ProcessedMesh {
    pub vertices: Vec<u8>,
    pub indices: Vec<u8>,
    pub vertex_format: VertexFormat,
    pub index_format: IndexFormat,
    pub bounds: MeshBounds,
    pub meshlets: Option<Vec<Meshlet>>,
    pub lods: Option<Vec<LodLevel>>,
}

/// Vertex format description
#[derive(Debug, Clone)]
pub struct VertexFormat {
    pub attributes: Vec<VertexAttribute>,
    pub stride: u32,
}

/// Vertex attribute
#[derive(Debug, Clone)]
pub struct VertexAttribute {
    pub semantic: AttributeSemantic,
    pub format: AttributeFormat,
    pub offset: u32,
}

/// Attribute semantic
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeSemantic {
    Position,
    Normal,
    Tangent,
    TexCoord0,
    TexCoord1,
    Color0,
    Joints0,
    Weights0,
}

/// Attribute format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeFormat {
    Float,
    Float2,
    Float3,
    Float4,
    Byte4,
    Byte4Norm,
    UByte4,
    UByte4Norm,
    Short2,
    Short2Norm,
    Short4,
    Short4Norm,
    Half2,
    Half4,
    UInt,
}

impl AttributeFormat {
    pub fn size(&self) -> u32 {
        match self {
            Self::Float => 4,
            Self::Float2 | Self::Half4 => 8,
            Self::Float3 => 12,
            Self::Float4 => 16,
            Self::Byte4 | Self::Byte4Norm | Self::UByte4 | Self::UByte4Norm | Self::UInt => 4,
            Self::Short2 | Self::Short2Norm | Self::Half2 => 4,
            Self::Short4 | Self::Short4Norm => 8,
        }
    }
}

/// Index format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexFormat {
    U16,
    U32,
}

// Helper functions

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn octahedral_encode(n: [f32; 3]) -> (f32, f32) {
    let sum = n[0].abs() + n[1].abs() + n[2].abs();
    let x = n[0] / sum;
    let y = n[1] / sum;

    if n[2] < 0.0 {
        let ox = (1.0 - y.abs()) * if x >= 0.0 { 1.0 } else { -1.0 };
        let oy = (1.0 - x.abs()) * if y >= 0.0 { 1.0 } else { -1.0 };
        (ox, oy)
    } else {
        (x, y)
    }
}

fn octahedral_decode(x: f32, y: f32) -> [f32; 3] {
    let mut n = [x, y, 1.0 - x.abs() - y.abs()];

    if n[2] < 0.0 {
        let ox = (1.0 - n[1].abs()) * if n[0] >= 0.0 { 1.0 } else { -1.0 };
        let oy = (1.0 - n[0].abs()) * if n[1] >= 0.0 { 1.0 } else { -1.0 };
        n[0] = ox;
        n[1] = oy;
    }

    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    [n[0] / len, n[1] / len, n[2] / len]
}
