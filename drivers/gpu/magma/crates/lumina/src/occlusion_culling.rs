//! Occlusion Culling Types for Lumina
//!
//! This module provides occlusion culling infrastructure including
//! hierarchical Z-buffer, software rasterization, and GPU-driven culling.

extern crate alloc;

use alloc::vec::Vec;

// ============================================================================
// Culling Handles
// ============================================================================

/// Occlusion system handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct OcclusionSystemHandle(pub u64);

impl OcclusionSystemHandle {
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

impl Default for OcclusionSystemHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Occluder handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct OccluderHandle(pub u64);

impl OccluderHandle {
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

impl Default for OccluderHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Bounding Volumes
// ============================================================================

/// Axis-aligned bounding box
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Aabb {
    /// Minimum corner
    pub min: [f32; 3],
    /// Maximum corner
    pub max: [f32; 3],
}

impl Aabb {
    /// Creates AABB
    pub const fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        Self { min, max }
    }

    /// Creates from center and half extents
    pub fn from_center_half_extents(center: [f32; 3], half_extents: [f32; 3]) -> Self {
        Self {
            min: [
                center[0] - half_extents[0],
                center[1] - half_extents[1],
                center[2] - half_extents[2],
            ],
            max: [
                center[0] + half_extents[0],
                center[1] + half_extents[1],
                center[2] + half_extents[2],
            ],
        }
    }

    /// Center point
    pub fn center(&self) -> [f32; 3] {
        [
            (self.min[0] + self.max[0]) * 0.5,
            (self.min[1] + self.max[1]) * 0.5,
            (self.min[2] + self.max[2]) * 0.5,
        ]
    }

    /// Half extents
    pub fn half_extents(&self) -> [f32; 3] {
        [
            (self.max[0] - self.min[0]) * 0.5,
            (self.max[1] - self.min[1]) * 0.5,
            (self.max[2] - self.min[2]) * 0.5,
        ]
    }

    /// Size
    pub fn size(&self) -> [f32; 3] {
        [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ]
    }

    /// Surface area
    pub fn surface_area(&self) -> f32 {
        let size = self.size();
        2.0 * (size[0] * size[1] + size[1] * size[2] + size[2] * size[0])
    }

    /// Volume
    pub fn volume(&self) -> f32 {
        let size = self.size();
        size[0] * size[1] * size[2]
    }

    /// Contains point
    pub fn contains_point(&self, point: [f32; 3]) -> bool {
        point[0] >= self.min[0]
            && point[0] <= self.max[0]
            && point[1] >= self.min[1]
            && point[1] <= self.max[1]
            && point[2] >= self.min[2]
            && point[2] <= self.max[2]
    }

    /// Intersects another AABB
    pub fn intersects(&self, other: &Self) -> bool {
        self.min[0] <= other.max[0]
            && self.max[0] >= other.min[0]
            && self.min[1] <= other.max[1]
            && self.max[1] >= other.min[1]
            && self.min[2] <= other.max[2]
            && self.max[2] >= other.min[2]
    }

    /// Merge with another AABB
    pub fn merge(&self, other: &Self) -> Self {
        Self {
            min: [
                self.min[0].min(other.min[0]),
                self.min[1].min(other.min[1]),
                self.min[2].min(other.min[2]),
            ],
            max: [
                self.max[0].max(other.max[0]),
                self.max[1].max(other.max[1]),
                self.max[2].max(other.max[2]),
            ],
        }
    }

    /// Expand by point
    pub fn expand(&mut self, point: [f32; 3]) {
        self.min[0] = self.min[0].min(point[0]);
        self.min[1] = self.min[1].min(point[1]);
        self.min[2] = self.min[2].min(point[2]);
        self.max[0] = self.max[0].max(point[0]);
        self.max[1] = self.max[1].max(point[1]);
        self.max[2] = self.max[2].max(point[2]);
    }

    /// Get corner points
    pub fn corners(&self) -> [[f32; 3]; 8] {
        [
            [self.min[0], self.min[1], self.min[2]],
            [self.max[0], self.min[1], self.min[2]],
            [self.min[0], self.max[1], self.min[2]],
            [self.max[0], self.max[1], self.min[2]],
            [self.min[0], self.min[1], self.max[2]],
            [self.max[0], self.min[1], self.max[2]],
            [self.min[0], self.max[1], self.max[2]],
            [self.max[0], self.max[1], self.max[2]],
        ]
    }
}

/// Bounding sphere
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct BoundingSphere {
    /// Center
    pub center: [f32; 3],
    /// Radius
    pub radius: f32,
}

impl BoundingSphere {
    /// Creates sphere
    pub const fn new(center: [f32; 3], radius: f32) -> Self {
        Self { center, radius }
    }

    /// From AABB
    pub fn from_aabb(aabb: &Aabb) -> Self {
        let center = aabb.center();
        let half = aabb.half_extents();
        let radius = (half[0] * half[0] + half[1] * half[1] + half[2] * half[2]).sqrt();
        Self { center, radius }
    }

    /// Contains point
    pub fn contains_point(&self, point: [f32; 3]) -> bool {
        let dx = point[0] - self.center[0];
        let dy = point[1] - self.center[1];
        let dz = point[2] - self.center[2];
        dx * dx + dy * dy + dz * dz <= self.radius * self.radius
    }

    /// Intersects another sphere
    pub fn intersects(&self, other: &Self) -> bool {
        let dx = other.center[0] - self.center[0];
        let dy = other.center[1] - self.center[1];
        let dz = other.center[2] - self.center[2];
        let dist_sq = dx * dx + dy * dy + dz * dz;
        let radius_sum = self.radius + other.radius;
        dist_sq <= radius_sum * radius_sum
    }
}

/// Oriented bounding box
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Obb {
    /// Center
    pub center: [f32; 3],
    /// Half extents
    pub half_extents: [f32; 3],
    /// Orientation (3x3 rotation matrix, row-major)
    pub orientation: [[f32; 3]; 3],
}

impl Obb {
    /// Creates OBB
    pub fn new(center: [f32; 3], half_extents: [f32; 3], orientation: [[f32; 3]; 3]) -> Self {
        Self {
            center,
            half_extents,
            orientation,
        }
    }

    /// From AABB (identity orientation)
    pub fn from_aabb(aabb: &Aabb) -> Self {
        Self {
            center: aabb.center(),
            half_extents: aabb.half_extents(),
            orientation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        }
    }

    /// Get corners
    pub fn corners(&self) -> [[f32; 3]; 8] {
        let mut corners = [[0.0f32; 3]; 8];
        let signs = [
            [-1.0, -1.0, -1.0],
            [1.0, -1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [1.0, 1.0, -1.0],
            [-1.0, -1.0, 1.0],
            [1.0, -1.0, 1.0],
            [-1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0],
        ];

        for (i, sign) in signs.iter().enumerate() {
            let local = [
                sign[0] * self.half_extents[0],
                sign[1] * self.half_extents[1],
                sign[2] * self.half_extents[2],
            ];
            corners[i] = [
                self.center[0]
                    + local[0] * self.orientation[0][0]
                    + local[1] * self.orientation[1][0]
                    + local[2] * self.orientation[2][0],
                self.center[1]
                    + local[0] * self.orientation[0][1]
                    + local[1] * self.orientation[1][1]
                    + local[2] * self.orientation[2][1],
                self.center[2]
                    + local[0] * self.orientation[0][2]
                    + local[1] * self.orientation[1][2]
                    + local[2] * self.orientation[2][2],
            ];
        }
        corners
    }
}

impl Default for Obb {
    fn default() -> Self {
        Self {
            center: [0.0, 0.0, 0.0],
            half_extents: [0.5, 0.5, 0.5],
            orientation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        }
    }
}

// ============================================================================
// Frustum
// ============================================================================

/// View frustum for culling
#[derive(Clone, Copy, Debug)]
pub struct Frustum {
    /// Planes (left, right, bottom, top, near, far)
    pub planes: [FrustumPlane; 6],
}

impl Frustum {
    /// Creates frustum from view-projection matrix
    pub fn from_matrix(vp: [[f32; 4]; 4]) -> Self {
        let planes = [
            // Left
            FrustumPlane::new(
                vp[0][3] + vp[0][0],
                vp[1][3] + vp[1][0],
                vp[2][3] + vp[2][0],
                vp[3][3] + vp[3][0],
            ),
            // Right
            FrustumPlane::new(
                vp[0][3] - vp[0][0],
                vp[1][3] - vp[1][0],
                vp[2][3] - vp[2][0],
                vp[3][3] - vp[3][0],
            ),
            // Bottom
            FrustumPlane::new(
                vp[0][3] + vp[0][1],
                vp[1][3] + vp[1][1],
                vp[2][3] + vp[2][1],
                vp[3][3] + vp[3][1],
            ),
            // Top
            FrustumPlane::new(
                vp[0][3] - vp[0][1],
                vp[1][3] - vp[1][1],
                vp[2][3] - vp[2][1],
                vp[3][3] - vp[3][1],
            ),
            // Near
            FrustumPlane::new(vp[0][2], vp[1][2], vp[2][2], vp[3][2]),
            // Far
            FrustumPlane::new(
                vp[0][3] - vp[0][2],
                vp[1][3] - vp[1][2],
                vp[2][3] - vp[2][2],
                vp[3][3] - vp[3][2],
            ),
        ];

        Self { planes }
    }

    /// Test AABB against frustum
    pub fn test_aabb(&self, aabb: &Aabb) -> FrustumTestResult {
        let mut result = FrustumTestResult::Inside;

        for plane in &self.planes {
            let p_vertex = [
                if plane.normal[0] > 0.0 { aabb.max[0] } else { aabb.min[0] },
                if plane.normal[1] > 0.0 { aabb.max[1] } else { aabb.min[1] },
                if plane.normal[2] > 0.0 { aabb.max[2] } else { aabb.min[2] },
            ];

            let n_vertex = [
                if plane.normal[0] < 0.0 { aabb.max[0] } else { aabb.min[0] },
                if plane.normal[1] < 0.0 { aabb.max[1] } else { aabb.min[1] },
                if plane.normal[2] < 0.0 { aabb.max[2] } else { aabb.min[2] },
            ];

            if plane.distance_to_point(p_vertex) < 0.0 {
                return FrustumTestResult::Outside;
            }

            if plane.distance_to_point(n_vertex) < 0.0 {
                result = FrustumTestResult::Intersecting;
            }
        }

        result
    }

    /// Test sphere against frustum
    pub fn test_sphere(&self, sphere: &BoundingSphere) -> FrustumTestResult {
        let mut result = FrustumTestResult::Inside;

        for plane in &self.planes {
            let distance = plane.distance_to_point(sphere.center);
            if distance < -sphere.radius {
                return FrustumTestResult::Outside;
            }
            if distance < sphere.radius {
                result = FrustumTestResult::Intersecting;
            }
        }

        result
    }

    /// Test point against frustum
    pub fn test_point(&self, point: [f32; 3]) -> bool {
        for plane in &self.planes {
            if plane.distance_to_point(point) < 0.0 {
                return false;
            }
        }
        true
    }
}

impl Default for Frustum {
    fn default() -> Self {
        Self {
            planes: [FrustumPlane::default(); 6],
        }
    }
}

/// Frustum plane
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct FrustumPlane {
    /// Normal
    pub normal: [f32; 3],
    /// Distance
    pub distance: f32,
}

impl FrustumPlane {
    /// Creates plane from equation ax + by + cz + d = 0
    pub fn new(a: f32, b: f32, c: f32, d: f32) -> Self {
        let len = (a * a + b * b + c * c).sqrt();
        if len > 0.0 {
            Self {
                normal: [a / len, b / len, c / len],
                distance: d / len,
            }
        } else {
            Self::default()
        }
    }

    /// Distance to point (positive = in front)
    pub fn distance_to_point(&self, point: [f32; 3]) -> f32 {
        self.normal[0] * point[0]
            + self.normal[1] * point[1]
            + self.normal[2] * point[2]
            + self.distance
    }
}

/// Frustum test result
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum FrustumTestResult {
    /// Fully outside
    Outside = 0,
    /// Fully inside
    Inside = 1,
    /// Intersecting
    Intersecting = 2,
}

impl FrustumTestResult {
    /// Is visible
    pub const fn is_visible(&self) -> bool {
        !matches!(self, Self::Outside)
    }
}

// ============================================================================
// Occlusion Culling Settings
// ============================================================================

/// Occlusion culling settings
#[derive(Clone, Debug)]
pub struct OcclusionSettings {
    /// Enable occlusion culling
    pub enabled: bool,
    /// Culling method
    pub method: OcclusionMethod,
    /// Hi-Z buffer width
    pub hi_z_width: u32,
    /// Hi-Z buffer height
    pub hi_z_height: u32,
    /// Software rasterizer resolution
    pub software_resolution: u32,
    /// Conservative rasterization
    pub conservative: bool,
    /// Max occluders
    pub max_occluders: u32,
    /// Occluder size threshold
    pub size_threshold: f32,
}

impl OcclusionSettings {
    /// Creates default settings
    pub fn new() -> Self {
        Self {
            enabled: true,
            method: OcclusionMethod::HierarchicalZ,
            hi_z_width: 512,
            hi_z_height: 256,
            software_resolution: 256,
            conservative: true,
            max_occluders: 256,
            size_threshold: 0.01,
        }
    }

    /// High quality
    pub fn high_quality() -> Self {
        Self {
            hi_z_width: 1024,
            hi_z_height: 512,
            software_resolution: 512,
            ..Self::new()
        }
    }

    /// Performance mode
    pub fn performance() -> Self {
        Self {
            hi_z_width: 256,
            hi_z_height: 128,
            software_resolution: 128,
            ..Self::new()
        }
    }

    /// With resolution
    pub fn with_resolution(mut self, width: u32, height: u32) -> Self {
        self.hi_z_width = width;
        self.hi_z_height = height;
        self
    }
}

impl Default for OcclusionSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Occlusion culling method
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum OcclusionMethod {
    /// Hierarchical Z-buffer
    #[default]
    HierarchicalZ = 0,
    /// Software rasterization
    SoftwareRasterization = 1,
    /// Hardware occlusion queries
    HardwareQueries = 2,
    /// GPU-driven culling
    GpuDriven = 3,
}

// ============================================================================
// Occluder Mesh
// ============================================================================

/// Occluder mesh data
#[derive(Clone, Debug)]
pub struct OccluderMesh {
    /// Vertices
    pub vertices: Vec<[f32; 3]>,
    /// Indices (triangles)
    pub indices: Vec<u32>,
    /// Bounding box
    pub bounds: Aabb,
}

impl OccluderMesh {
    /// Creates empty occluder
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            bounds: Aabb::default(),
        }
    }

    /// Creates box occluder
    pub fn box_occluder(min: [f32; 3], max: [f32; 3]) -> Self {
        let vertices = vec![
            [min[0], min[1], min[2]],
            [max[0], min[1], min[2]],
            [max[0], max[1], min[2]],
            [min[0], max[1], min[2]],
            [min[0], min[1], max[2]],
            [max[0], min[1], max[2]],
            [max[0], max[1], max[2]],
            [min[0], max[1], max[2]],
        ];

        let indices = vec![
            // Front
            0, 1, 2, 0, 2, 3,
            // Back
            5, 4, 7, 5, 7, 6,
            // Left
            4, 0, 3, 4, 3, 7,
            // Right
            1, 5, 6, 1, 6, 2,
            // Top
            3, 2, 6, 3, 6, 7,
            // Bottom
            4, 5, 1, 4, 1, 0,
        ];

        Self {
            vertices,
            indices,
            bounds: Aabb::new(min, max),
        }
    }

    /// Creates quad occluder
    pub fn quad(corners: [[f32; 3]; 4]) -> Self {
        let mut bounds = Aabb::new(corners[0], corners[0]);
        for corner in &corners[1..] {
            bounds.expand(*corner);
        }

        Self {
            vertices: corners.to_vec(),
            indices: vec![0, 1, 2, 0, 2, 3],
            bounds,
        }
    }

    /// Triangle count
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Recalculate bounds
    pub fn recalculate_bounds(&mut self) {
        if self.vertices.is_empty() {
            self.bounds = Aabb::default();
            return;
        }

        self.bounds = Aabb::new(self.vertices[0], self.vertices[0]);
        for vertex in &self.vertices[1..] {
            self.bounds.expand(*vertex);
        }
    }
}

impl Default for OccluderMesh {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Culling Results
// ============================================================================

/// Culling input object
#[derive(Clone, Copy, Debug)]
pub struct CullInput {
    /// Object ID
    pub id: u64,
    /// Bounding box
    pub bounds: Aabb,
    /// Is occluder
    pub is_occluder: bool,
    /// LOD bias
    pub lod_bias: f32,
}

impl CullInput {
    /// Creates input
    pub fn new(id: u64, bounds: Aabb) -> Self {
        Self {
            id,
            bounds,
            is_occluder: false,
            lod_bias: 0.0,
        }
    }

    /// As occluder
    pub fn occluder(mut self) -> Self {
        self.is_occluder = true;
        self
    }
}

/// Culling result
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum CullResult {
    /// Visible
    Visible = 0,
    /// Frustum culled
    FrustumCulled = 1,
    /// Occlusion culled
    OcclusionCulled = 2,
    /// Distance culled
    DistanceCulled = 3,
    /// Size culled (too small)
    SizeCulled = 4,
}

impl CullResult {
    /// Is visible
    pub const fn is_visible(&self) -> bool {
        matches!(self, Self::Visible)
    }

    /// Is culled
    pub const fn is_culled(&self) -> bool {
        !self.is_visible()
    }
}

/// Culling output
#[derive(Clone, Debug)]
pub struct CullOutput {
    /// Object ID
    pub id: u64,
    /// Culling result
    pub result: CullResult,
    /// Screen size (normalized)
    pub screen_size: f32,
    /// Distance to camera
    pub distance: f32,
    /// Recommended LOD level
    pub lod_level: u32,
}

/// Culling statistics
#[derive(Clone, Debug, Default)]
pub struct CullingStats {
    /// Total objects tested
    pub total_objects: u32,
    /// Visible objects
    pub visible: u32,
    /// Frustum culled
    pub frustum_culled: u32,
    /// Occlusion culled
    pub occlusion_culled: u32,
    /// Distance culled
    pub distance_culled: u32,
    /// Size culled
    pub size_culled: u32,
    /// Culling time (microseconds)
    pub time_us: u64,
}

impl CullingStats {
    /// Cull rate
    pub fn cull_rate(&self) -> f32 {
        if self.total_objects == 0 {
            0.0
        } else {
            1.0 - (self.visible as f32 / self.total_objects as f32)
        }
    }
}

// ============================================================================
// Hi-Z Buffer
// ============================================================================

/// Hierarchical Z-buffer
#[derive(Clone, Debug)]
pub struct HiZBuffer {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Mip levels
    pub mip_levels: u32,
    /// Depth data (per mip)
    pub data: Vec<Vec<f32>>,
}

impl HiZBuffer {
    /// Creates Hi-Z buffer
    pub fn new(width: u32, height: u32) -> Self {
        let mip_levels = (width.max(height) as f32).log2().ceil() as u32 + 1;
        let mut data = Vec::with_capacity(mip_levels as usize);

        let mut w = width;
        let mut h = height;
        for _ in 0..mip_levels {
            data.push(vec![1.0; (w * h) as usize]);
            w = (w / 2).max(1);
            h = (h / 2).max(1);
        }

        Self {
            width,
            height,
            mip_levels,
            data,
        }
    }

    /// Clears buffer
    pub fn clear(&mut self) {
        for mip in &mut self.data {
            mip.fill(1.0);
        }
    }

    /// Gets depth at position (mip 0)
    pub fn get_depth(&self, x: u32, y: u32) -> f32 {
        if x >= self.width || y >= self.height {
            return 1.0;
        }
        self.data[0][(y * self.width + x) as usize]
    }

    /// Gets depth at mip level
    pub fn get_depth_mip(&self, x: u32, y: u32, mip: u32) -> f32 {
        if mip >= self.mip_levels {
            return 1.0;
        }
        let w = (self.width >> mip).max(1);
        let h = (self.height >> mip).max(1);
        if x >= w || y >= h {
            return 1.0;
        }
        self.data[mip as usize][(y * w + x) as usize]
    }

    /// Tests rectangle visibility
    pub fn test_rect(&self, min_x: u32, min_y: u32, max_x: u32, max_y: u32, depth: f32) -> bool {
        // Find appropriate mip level
        let rect_size = (max_x - min_x).max(max_y - min_y);
        let mip = (rect_size as f32).log2().ceil() as u32;
        let mip = mip.min(self.mip_levels - 1);

        // Sample at mip level
        let scale = 1u32 << mip;
        let mx = min_x / scale;
        let my = min_y / scale;

        let hi_z_depth = self.get_depth_mip(mx, my, mip);
        depth <= hi_z_depth
    }
}

impl Default for HiZBuffer {
    fn default() -> Self {
        Self::new(512, 256)
    }
}

// ============================================================================
// GPU Culling Data
// ============================================================================

/// GPU culling input buffer format
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuCullInput {
    /// Bounding sphere (xyz = center, w = radius)
    pub bounding_sphere: [f32; 4],
    /// Bounding box min
    pub aabb_min: [f32; 3],
    /// Object index
    pub object_index: u32,
    /// Bounding box max
    pub aabb_max: [f32; 3],
    /// Flags
    pub flags: u32,
}

/// GPU culling output buffer format
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuCullOutput {
    /// Object index
    pub object_index: u32,
    /// Is visible
    pub visible: u32,
    /// LOD level
    pub lod_level: u32,
    /// Screen size
    pub screen_size: f32,
}

/// Indirect draw command
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct IndirectDrawCommand {
    /// Vertex count
    pub vertex_count: u32,
    /// Instance count
    pub instance_count: u32,
    /// First vertex
    pub first_vertex: u32,
    /// First instance
    pub first_instance: u32,
}

/// Indirect indexed draw command
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct IndirectIndexedDrawCommand {
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
