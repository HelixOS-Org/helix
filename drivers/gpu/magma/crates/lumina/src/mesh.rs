//! Mesh representation and utilities
//!
//! This module provides types for representing and manipulating GPU meshes.

use alloc::vec::Vec;

use crate::buffer::{BufferUsage, GpuBuffer, IndexBuffer};
use crate::types::{GpuData, GpuVertex};
use lumina_math::Vec3;

/// A GPU mesh combining vertices and optional indices
pub struct GpuMesh {
    vertex_count: usize,
    index_count: usize,
    vertex_handle: Option<crate::types::BufferHandle>,
    index_handle: Option<crate::types::BufferHandle>,
}

impl GpuMesh {
    /// Creates a mesh from vertex data (no indices)
    pub fn from_vertices<V: GpuVertex>(vertices: &[V]) -> Self {
        Self {
            vertex_count: vertices.len(),
            index_count: 0,
            vertex_handle: None,
            index_handle: None,
        }
    }

    /// Creates a mesh from vertices and indices
    pub fn from_indexed<V: GpuVertex>(vertices: &[V], indices: &[u32]) -> Self {
        Self {
            vertex_count: vertices.len(),
            index_count: indices.len(),
            vertex_handle: None,
            index_handle: None,
        }
    }

    /// Creates a unit cube mesh
    ///
    /// The cube is centered at the origin with side length `size`.
    pub fn cube(size: f32) -> Self {
        // 8 vertices, 36 indices (6 faces × 2 triangles × 3 vertices)
        Self {
            vertex_count: 8,
            index_count: 36,
            vertex_handle: None,
            index_handle: None,
        }
    }

    /// Creates a unit sphere mesh
    pub fn sphere(radius: f32, segments: u32, rings: u32) -> Self {
        let vertex_count = ((segments + 1) * (rings + 1)) as usize;
        let index_count = (segments * rings * 6) as usize;

        Self {
            vertex_count,
            index_count,
            vertex_handle: None,
            index_handle: None,
        }
    }

    /// Creates a plane mesh
    pub fn plane(width: f32, height: f32, subdivisions: u32) -> Self {
        let verts_per_side = subdivisions + 1;
        let vertex_count = (verts_per_side * verts_per_side) as usize;
        let index_count = (subdivisions * subdivisions * 6) as usize;

        Self {
            vertex_count,
            index_count,
            vertex_handle: None,
            index_handle: None,
        }
    }

    /// Assigns vertex colors to the mesh
    pub fn with_colors(self, _colors: impl AsRef<[Vec3]>) -> Self {
        // TODO: Store colors in vertex buffer
        self
    }

    /// Returns the number of vertices
    #[inline]
    pub fn vertex_count(&self) -> usize {
        self.vertex_count
    }

    /// Returns the number of indices
    #[inline]
    pub fn index_count(&self) -> usize {
        self.index_count
    }

    /// Returns true if this mesh is indexed
    #[inline]
    pub fn is_indexed(&self) -> bool {
        self.index_count > 0
    }

    /// Returns the vertex buffer handle
    #[inline]
    pub(crate) fn vertex_handle(&self) -> Option<crate::types::BufferHandle> {
        self.vertex_handle
    }

    /// Returns the index buffer handle
    #[inline]
    pub(crate) fn index_handle(&self) -> Option<crate::types::BufferHandle> {
        self.index_handle
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROCEDURAL MESH GENERATION
// ═══════════════════════════════════════════════════════════════════════════

/// Vertex format for procedural meshes
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ProceduralVertex {
    /// Position
    pub position: Vec3,
    /// Normal
    pub normal: Vec3,
    /// Texture coordinates
    pub uv: [f32; 2],
    /// Vertex color
    pub color: Vec3,
}

// Safety: ProceduralVertex is a plain-old-data type
unsafe impl GpuData for ProceduralVertex {}

impl GpuVertex for ProceduralVertex {
    fn attributes() -> &'static [crate::types::VertexAttribute] {
        use crate::types::{AttributeFormat, VertexAttribute};

        static ATTRS: [VertexAttribute; 4] = [
            VertexAttribute {
                location: 0,
                offset: 0,
                format: AttributeFormat::Vec3,
            },
            VertexAttribute {
                location: 1,
                offset: 12,
                format: AttributeFormat::Vec3,
            },
            VertexAttribute {
                location: 2,
                offset: 24,
                format: AttributeFormat::Vec2,
            },
            VertexAttribute {
                location: 3,
                offset: 32,
                format: AttributeFormat::Vec3,
            },
        ];

        &ATTRS
    }
}

/// Generates cube vertices and indices
pub fn generate_cube(size: f32) -> (Vec<ProceduralVertex>, Vec<u32>) {
    let half = size / 2.0;

    let positions = [
        // Front face
        Vec3::new(-half, -half, half),
        Vec3::new(half, -half, half),
        Vec3::new(half, half, half),
        Vec3::new(-half, half, half),
        // Back face
        Vec3::new(-half, -half, -half),
        Vec3::new(-half, half, -half),
        Vec3::new(half, half, -half),
        Vec3::new(half, -half, -half),
    ];

    let normals = [
        Vec3::new(0.0, 0.0, 1.0),   // Front
        Vec3::new(0.0, 0.0, -1.0),  // Back
        Vec3::new(0.0, 1.0, 0.0),   // Top
        Vec3::new(0.0, -1.0, 0.0),  // Bottom
        Vec3::new(1.0, 0.0, 0.0),   // Right
        Vec3::new(-1.0, 0.0, 0.0),  // Left
    ];

    // Generate 24 vertices (4 per face to have correct normals)
    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);

    let faces = [
        // Front
        ([0, 1, 2, 3], 0),
        // Back
        ([4, 5, 6, 7], 1),
        // Top
        ([3, 2, 6, 5], 2),
        // Bottom
        ([0, 4, 7, 1], 3),
        // Right
        ([1, 7, 6, 2], 4),
        // Left
        ([0, 3, 5, 4], 5),
    ];

    for (face_indices, normal_idx) in faces {
        let base = vertices.len() as u32;
        let normal = normals[normal_idx];

        for (i, &pos_idx) in face_indices.iter().enumerate() {
            vertices.push(ProceduralVertex {
                position: positions[pos_idx],
                normal,
                uv: match i {
                    0 => [0.0, 1.0],
                    1 => [1.0, 1.0],
                    2 => [1.0, 0.0],
                    3 => [0.0, 0.0],
                    _ => unreachable!(),
                },
                color: Vec3::new(1.0, 1.0, 1.0),
            });
        }

        // Two triangles per face
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    (vertices, indices)
}

/// Generates sphere vertices and indices
pub fn generate_sphere(radius: f32, segments: u32, rings: u32) -> (Vec<ProceduralVertex>, Vec<u32>) {
    let mut vertices = Vec::with_capacity(((segments + 1) * (rings + 1)) as usize);
    let mut indices = Vec::with_capacity((segments * rings * 6) as usize);

    for ring in 0..=rings {
        let phi = core::f32::consts::PI * ring as f32 / rings as f32;
        let sin_phi = phi.sin();
        let cos_phi = phi.cos();

        for segment in 0..=segments {
            let theta = 2.0 * core::f32::consts::PI * segment as f32 / segments as f32;
            let sin_theta = theta.sin();
            let cos_theta = theta.cos();

            let x = cos_theta * sin_phi;
            let y = cos_phi;
            let z = sin_theta * sin_phi;

            let position = Vec3::new(x * radius, y * radius, z * radius);
            let normal = Vec3::new(x, y, z);

            vertices.push(ProceduralVertex {
                position,
                normal,
                uv: [segment as f32 / segments as f32, ring as f32 / rings as f32],
                color: Vec3::new(1.0, 1.0, 1.0),
            });
        }
    }

    for ring in 0..rings {
        for segment in 0..segments {
            let current = ring * (segments + 1) + segment;
            let next = current + segments + 1;

            indices.extend_from_slice(&[current, next, current + 1, current + 1, next, next + 1]);
        }
    }

    (vertices, indices)
}

/// Generates plane vertices and indices
pub fn generate_plane(
    width: f32,
    height: f32,
    subdivisions: u32,
) -> (Vec<ProceduralVertex>, Vec<u32>) {
    let verts_per_side = subdivisions + 1;
    let mut vertices = Vec::with_capacity((verts_per_side * verts_per_side) as usize);
    let mut indices = Vec::with_capacity((subdivisions * subdivisions * 6) as usize);

    let half_width = width / 2.0;
    let half_height = height / 2.0;

    for z in 0..verts_per_side {
        for x in 0..verts_per_side {
            let fx = x as f32 / subdivisions as f32;
            let fz = z as f32 / subdivisions as f32;

            vertices.push(ProceduralVertex {
                position: Vec3::new(fx * width - half_width, 0.0, fz * height - half_height),
                normal: Vec3::new(0.0, 1.0, 0.0),
                uv: [fx, fz],
                color: Vec3::new(1.0, 1.0, 1.0),
            });
        }
    }

    for z in 0..subdivisions {
        for x in 0..subdivisions {
            let current = z * verts_per_side + x;
            let next = current + verts_per_side;

            indices.extend_from_slice(&[current, next, current + 1, current + 1, next, next + 1]);
        }
    }

    (vertices, indices)
}
