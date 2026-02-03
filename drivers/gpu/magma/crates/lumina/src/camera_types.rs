//! Camera Types for Lumina
//!
//! This module provides camera types, projections, view matrices,
//! frustum culling, and camera controller utilities.

extern crate alloc;

use alloc::vec::Vec;

// ============================================================================
// Camera Handle
// ============================================================================

/// Camera handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CameraHandle(pub u64);

impl CameraHandle {
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

impl Default for CameraHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Camera
// ============================================================================

/// Camera data
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    /// Position
    pub position: [f32; 3],
    /// Forward direction
    pub forward: [f32; 3],
    /// Up direction
    pub up: [f32; 3],
    /// Right direction
    pub right: [f32; 3],
    /// Projection type
    pub projection: Projection,
    /// Near plane
    pub near: f32,
    /// Far plane
    pub far: f32,
    /// Aspect ratio
    pub aspect: f32,
}

impl Camera {
    /// Creates perspective camera
    pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            forward: [0.0, 0.0, -1.0],
            up: [0.0, 1.0, 0.0],
            right: [1.0, 0.0, 0.0],
            projection: Projection::Perspective { fov_y },
            near,
            far,
            aspect,
        }
    }

    /// Creates orthographic camera
    pub fn orthographic(width: f32, height: f32, near: f32, far: f32) -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            forward: [0.0, 0.0, -1.0],
            up: [0.0, 1.0, 0.0],
            right: [1.0, 0.0, 0.0],
            projection: Projection::Orthographic { width, height },
            near,
            far,
            aspect: width / height,
        }
    }

    /// Look at target
    pub fn look_at(&mut self, target: [f32; 3], up: [f32; 3]) {
        // Forward = normalize(target - position)
        let dx = target[0] - self.position[0];
        let dy = target[1] - self.position[1];
        let dz = target[2] - self.position[2];
        let len = (dx * dx + dy * dy + dz * dz).sqrt();

        if len > 0.0 {
            self.forward = [dx / len, dy / len, dz / len];
        }

        // Right = normalize(cross(forward, up))
        let rx = self.forward[1] * up[2] - self.forward[2] * up[1];
        let ry = self.forward[2] * up[0] - self.forward[0] * up[2];
        let rz = self.forward[0] * up[1] - self.forward[1] * up[0];
        let len = (rx * rx + ry * ry + rz * rz).sqrt();

        if len > 0.0 {
            self.right = [rx / len, ry / len, rz / len];
        }

        // Up = cross(right, forward)
        self.up = [
            self.right[1] * self.forward[2] - self.right[2] * self.forward[1],
            self.right[2] * self.forward[0] - self.right[0] * self.forward[2],
            self.right[0] * self.forward[1] - self.right[1] * self.forward[0],
        ];
    }

    /// Set position
    pub fn set_position(&mut self, position: [f32; 3]) {
        self.position = position;
    }

    /// View matrix (row-major)
    pub fn view_matrix(&self) -> [[f32; 4]; 4] {
        let tx = -(self.right[0] * self.position[0]
            + self.right[1] * self.position[1]
            + self.right[2] * self.position[2]);
        let ty = -(self.up[0] * self.position[0]
            + self.up[1] * self.position[1]
            + self.up[2] * self.position[2]);
        let tz = self.forward[0] * self.position[0]
            + self.forward[1] * self.position[1]
            + self.forward[2] * self.position[2];

        [
            [self.right[0], self.up[0], -self.forward[0], 0.0],
            [self.right[1], self.up[1], -self.forward[1], 0.0],
            [self.right[2], self.up[2], -self.forward[2], 0.0],
            [tx, ty, tz, 1.0],
        ]
    }

    /// Projection matrix (row-major, Vulkan NDC: Y down, Z 0-1)
    pub fn projection_matrix(&self) -> [[f32; 4]; 4] {
        match self.projection {
            Projection::Perspective { fov_y } => self.perspective_matrix_vulkan(fov_y),
            Projection::Orthographic { width, height } => {
                self.orthographic_matrix_vulkan(width, height)
            },
        }
    }

    /// Perspective matrix (Vulkan NDC)
    fn perspective_matrix_vulkan(&self, fov_y: f32) -> [[f32; 4]; 4] {
        let tan_half_fov = (fov_y * 0.5).tan();
        let f = 1.0 / tan_half_fov;

        // Reversed-Z for better precision
        let a = self.near / (self.far - self.near);
        let b = self.far * self.near / (self.far - self.near);

        [
            [f / self.aspect, 0.0, 0.0, 0.0],
            [0.0, -f, 0.0, 0.0], // Y flipped for Vulkan
            [0.0, 0.0, a, -1.0],
            [0.0, 0.0, b, 0.0],
        ]
    }

    /// Orthographic matrix (Vulkan NDC)
    fn orthographic_matrix_vulkan(&self, width: f32, height: f32) -> [[f32; 4]; 4] {
        let rml = width;
        let tmb = height;
        let fmn = self.far - self.near;

        [
            [2.0 / rml, 0.0, 0.0, 0.0],
            [0.0, -2.0 / tmb, 0.0, 0.0], // Y flipped for Vulkan
            [0.0, 0.0, -1.0 / fmn, 0.0],
            [0.0, 0.0, -self.near / fmn, 1.0],
        ]
    }

    /// View-projection matrix
    pub fn view_projection_matrix(&self) -> [[f32; 4]; 4] {
        let view = self.view_matrix();
        let proj = self.projection_matrix();
        matrix_multiply(&view, &proj)
    }

    /// Get frustum planes
    pub fn frustum(&self) -> Frustum {
        Frustum::from_view_projection(&self.view_projection_matrix())
    }

    /// Get field of view (for perspective)
    pub fn fov_y(&self) -> Option<f32> {
        match self.projection {
            Projection::Perspective { fov_y } => Some(fov_y),
            _ => None,
        }
    }

    /// Set field of view (for perspective)
    pub fn set_fov_y(&mut self, fov_y: f32) {
        if let Projection::Perspective { fov_y: ref mut fov } = self.projection {
            *fov = fov_y;
        }
    }

    /// Set aspect ratio
    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::perspective(
            core::f32::consts::FRAC_PI_4, // 45 degrees
            16.0 / 9.0,
            0.1,
            1000.0,
        )
    }
}

// ============================================================================
// Projection
// ============================================================================

/// Projection type
#[derive(Clone, Copy, Debug)]
pub enum Projection {
    /// Perspective projection
    Perspective {
        /// Vertical field of view in radians
        fov_y: f32,
    },
    /// Orthographic projection
    Orthographic {
        /// View width
        width: f32,
        /// View height
        height: f32,
    },
}

impl Default for Projection {
    fn default() -> Self {
        Self::Perspective {
            fov_y: core::f32::consts::FRAC_PI_4,
        }
    }
}

// ============================================================================
// Camera Uniform Data
// ============================================================================

/// Camera uniform data for GPU
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct CameraUniformData {
    /// View matrix
    pub view: [[f32; 4]; 4],
    /// Projection matrix
    pub projection: [[f32; 4]; 4],
    /// View-projection matrix
    pub view_projection: [[f32; 4]; 4],
    /// Inverse view matrix
    pub inverse_view: [[f32; 4]; 4],
    /// Inverse projection matrix
    pub inverse_projection: [[f32; 4]; 4],
    /// Inverse view-projection matrix
    pub inverse_view_projection: [[f32; 4]; 4],
    /// Camera position (world space)
    pub position: [f32; 4],
    /// Camera forward direction
    pub forward: [f32; 4],
    /// Near/far planes and aspect ratio
    pub planes: [f32; 4], // near, far, aspect, fov_y
    /// Screen size
    pub screen_size: [f32; 4], // width, height, 1/width, 1/height
}

impl CameraUniformData {
    /// Creates from camera and screen dimensions
    pub fn from_camera(camera: &Camera, screen_width: u32, screen_height: u32) -> Self {
        let view = camera.view_matrix();
        let projection = camera.projection_matrix();
        let view_projection = matrix_multiply(&view, &projection);

        let inverse_view = matrix_inverse(&view);
        let inverse_projection = matrix_inverse(&projection);
        let inverse_view_projection = matrix_inverse(&view_projection);

        let fov_y = camera.fov_y().unwrap_or(0.0);

        Self {
            view,
            projection,
            view_projection,
            inverse_view,
            inverse_projection,
            inverse_view_projection,
            position: [
                camera.position[0],
                camera.position[1],
                camera.position[2],
                1.0,
            ],
            forward: [camera.forward[0], camera.forward[1], camera.forward[2], 0.0],
            planes: [camera.near, camera.far, camera.aspect, fov_y],
            screen_size: [
                screen_width as f32,
                screen_height as f32,
                1.0 / screen_width as f32,
                1.0 / screen_height as f32,
            ],
        }
    }
}

// ============================================================================
// Frustum
// ============================================================================

/// View frustum for culling
#[derive(Clone, Copy, Debug, Default)]
pub struct Frustum {
    /// Planes (left, right, bottom, top, near, far)
    pub planes: [Plane; 6],
}

impl Frustum {
    /// Frustum plane indices
    pub const LEFT: usize = 0;
    pub const RIGHT: usize = 1;
    pub const BOTTOM: usize = 2;
    pub const TOP: usize = 3;
    pub const NEAR: usize = 4;
    pub const FAR: usize = 5;

    /// Creates from view-projection matrix
    pub fn from_view_projection(vp: &[[f32; 4]; 4]) -> Self {
        let mut planes = [Plane::default(); 6];

        // Extract planes from the view-projection matrix
        // Left plane
        planes[Self::LEFT] = Plane::new(
            vp[0][3] + vp[0][0],
            vp[1][3] + vp[1][0],
            vp[2][3] + vp[2][0],
            vp[3][3] + vp[3][0],
        );

        // Right plane
        planes[Self::RIGHT] = Plane::new(
            vp[0][3] - vp[0][0],
            vp[1][3] - vp[1][0],
            vp[2][3] - vp[2][0],
            vp[3][3] - vp[3][0],
        );

        // Bottom plane
        planes[Self::BOTTOM] = Plane::new(
            vp[0][3] + vp[0][1],
            vp[1][3] + vp[1][1],
            vp[2][3] + vp[2][1],
            vp[3][3] + vp[3][1],
        );

        // Top plane
        planes[Self::TOP] = Plane::new(
            vp[0][3] - vp[0][1],
            vp[1][3] - vp[1][1],
            vp[2][3] - vp[2][1],
            vp[3][3] - vp[3][1],
        );

        // Near plane
        planes[Self::NEAR] = Plane::new(vp[0][2], vp[1][2], vp[2][2], vp[3][2]);

        // Far plane
        planes[Self::FAR] = Plane::new(
            vp[0][3] - vp[0][2],
            vp[1][3] - vp[1][2],
            vp[2][3] - vp[2][2],
            vp[3][3] - vp[3][2],
        );

        // Normalize all planes
        for plane in &mut planes {
            plane.normalize();
        }

        Self { planes }
    }

    /// Test point against frustum
    pub fn contains_point(&self, point: [f32; 3]) -> bool {
        for plane in &self.planes {
            if plane.distance(point) < 0.0 {
                return false;
            }
        }
        true
    }

    /// Test sphere against frustum
    pub fn intersects_sphere(&self, center: [f32; 3], radius: f32) -> FrustumResult {
        let mut result = FrustumResult::Inside;

        for plane in &self.planes {
            let distance = plane.distance(center);

            if distance < -radius {
                return FrustumResult::Outside;
            } else if distance < radius {
                result = FrustumResult::Intersect;
            }
        }

        result
    }

    /// Test AABB against frustum
    pub fn intersects_aabb(&self, min: [f32; 3], max: [f32; 3]) -> FrustumResult {
        let mut result = FrustumResult::Inside;

        for plane in &self.planes {
            // Get the p-vertex (most positive vertex relative to plane normal)
            let px = if plane.normal[0] >= 0.0 {
                max[0]
            } else {
                min[0]
            };
            let py = if plane.normal[1] >= 0.0 {
                max[1]
            } else {
                min[1]
            };
            let pz = if plane.normal[2] >= 0.0 {
                max[2]
            } else {
                min[2]
            };

            // Get the n-vertex (most negative vertex relative to plane normal)
            let nx = if plane.normal[0] >= 0.0 {
                min[0]
            } else {
                max[0]
            };
            let ny = if plane.normal[1] >= 0.0 {
                min[1]
            } else {
                max[1]
            };
            let nz = if plane.normal[2] >= 0.0 {
                min[2]
            } else {
                max[2]
            };

            if plane.distance([px, py, pz]) < 0.0 {
                return FrustumResult::Outside;
            }

            if plane.distance([nx, ny, nz]) < 0.0 {
                result = FrustumResult::Intersect;
            }
        }

        result
    }
}

/// Frustum intersection result
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrustumResult {
    /// Completely outside
    Outside,
    /// Partially intersecting
    Intersect,
    /// Completely inside
    Inside,
}

/// Plane
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Plane {
    /// Normal
    pub normal: [f32; 3],
    /// Distance
    pub distance: f32,
}

impl Plane {
    /// Creates new plane from coefficients
    pub fn new(a: f32, b: f32, c: f32, d: f32) -> Self {
        Self {
            normal: [a, b, c],
            distance: d,
        }
    }

    /// Creates from normal and point
    pub fn from_normal_point(normal: [f32; 3], point: [f32; 3]) -> Self {
        let d = -(normal[0] * point[0] + normal[1] * point[1] + normal[2] * point[2]);
        Self {
            normal,
            distance: d,
        }
    }

    /// Normalize plane
    pub fn normalize(&mut self) {
        let len = (self.normal[0] * self.normal[0]
            + self.normal[1] * self.normal[1]
            + self.normal[2] * self.normal[2])
            .sqrt();

        if len > 0.0 {
            let inv_len = 1.0 / len;
            self.normal[0] *= inv_len;
            self.normal[1] *= inv_len;
            self.normal[2] *= inv_len;
            self.distance *= inv_len;
        }
    }

    /// Signed distance to point
    pub fn distance(&self, point: [f32; 3]) -> f32 {
        self.normal[0] * point[0]
            + self.normal[1] * point[1]
            + self.normal[2] * point[2]
            + self.distance
    }
}

// ============================================================================
// Camera Controller Types
// ============================================================================

/// First-person camera controller state
#[derive(Clone, Copy, Debug)]
pub struct FirstPersonController {
    /// Yaw angle (radians)
    pub yaw: f32,
    /// Pitch angle (radians)
    pub pitch: f32,
    /// Movement speed
    pub move_speed: f32,
    /// Look sensitivity
    pub sensitivity: f32,
    /// Pitch limits
    pub pitch_limit: f32,
}

impl FirstPersonController {
    /// Creates new controller
    pub fn new() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            move_speed: 5.0,
            sensitivity: 0.002,
            pitch_limit: core::f32::consts::FRAC_PI_2 - 0.01,
        }
    }

    /// Update with mouse delta
    pub fn rotate(&mut self, delta_x: f32, delta_y: f32) {
        self.yaw += delta_x * self.sensitivity;
        self.pitch -= delta_y * self.sensitivity;
        self.pitch = self.pitch.clamp(-self.pitch_limit, self.pitch_limit);
    }

    /// Get forward direction
    pub fn forward(&self) -> [f32; 3] {
        let cos_pitch = self.pitch.cos();
        [
            self.yaw.cos() * cos_pitch,
            self.pitch.sin(),
            self.yaw.sin() * cos_pitch,
        ]
    }

    /// Get right direction
    pub fn right(&self) -> [f32; 3] {
        [
            (self.yaw + core::f32::consts::FRAC_PI_2).cos(),
            0.0,
            (self.yaw + core::f32::consts::FRAC_PI_2).sin(),
        ]
    }

    /// Apply to camera
    pub fn apply(&self, camera: &mut Camera) {
        let forward = self.forward();
        let right = self.right();
        let up = [0.0, 1.0, 0.0];

        camera.forward = forward;
        camera.right = right;
        camera.up = up;
    }

    /// Move forward/backward
    pub fn move_forward(&self, camera: &mut Camera, delta: f32) {
        let forward = self.forward();
        camera.position[0] += forward[0] * delta * self.move_speed;
        camera.position[1] += forward[1] * delta * self.move_speed;
        camera.position[2] += forward[2] * delta * self.move_speed;
    }

    /// Move right/left
    pub fn move_right(&self, camera: &mut Camera, delta: f32) {
        let right = self.right();
        camera.position[0] += right[0] * delta * self.move_speed;
        camera.position[2] += right[2] * delta * self.move_speed;
    }

    /// Move up/down
    pub fn move_up(&self, camera: &mut Camera, delta: f32) {
        camera.position[1] += delta * self.move_speed;
    }
}

impl Default for FirstPersonController {
    fn default() -> Self {
        Self::new()
    }
}

/// Orbit camera controller state
#[derive(Clone, Copy, Debug)]
pub struct OrbitController {
    /// Target point
    pub target: [f32; 3],
    /// Distance from target
    pub distance: f32,
    /// Azimuth angle (radians)
    pub azimuth: f32,
    /// Elevation angle (radians)
    pub elevation: f32,
    /// Orbit sensitivity
    pub sensitivity: f32,
    /// Zoom sensitivity
    pub zoom_sensitivity: f32,
    /// Min distance
    pub min_distance: f32,
    /// Max distance
    pub max_distance: f32,
    /// Elevation limits
    pub elevation_min: f32,
    /// Elevation max
    pub elevation_max: f32,
}

impl OrbitController {
    /// Creates new controller
    pub fn new() -> Self {
        Self {
            target: [0.0, 0.0, 0.0],
            distance: 10.0,
            azimuth: 0.0,
            elevation: 0.3,
            sensitivity: 0.01,
            zoom_sensitivity: 0.1,
            min_distance: 1.0,
            max_distance: 100.0,
            elevation_min: -core::f32::consts::FRAC_PI_2 + 0.1,
            elevation_max: core::f32::consts::FRAC_PI_2 - 0.1,
        }
    }

    /// Orbit with mouse delta
    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        self.azimuth += delta_x * self.sensitivity;
        self.elevation -= delta_y * self.sensitivity;
        self.elevation = self.elevation.clamp(self.elevation_min, self.elevation_max);
    }

    /// Zoom
    pub fn zoom(&mut self, delta: f32) {
        self.distance *= 1.0 - delta * self.zoom_sensitivity;
        self.distance = self.distance.clamp(self.min_distance, self.max_distance);
    }

    /// Pan target
    pub fn pan(&mut self, right: f32, up: f32) {
        let cos_azimuth = self.azimuth.cos();
        let sin_azimuth = self.azimuth.sin();

        self.target[0] += (-sin_azimuth * right) * self.distance * 0.001;
        self.target[1] += up * self.distance * 0.001;
        self.target[2] += (cos_azimuth * right) * self.distance * 0.001;
    }

    /// Apply to camera
    pub fn apply(&self, camera: &mut Camera) {
        let cos_elevation = self.elevation.cos();
        let sin_elevation = self.elevation.sin();
        let cos_azimuth = self.azimuth.cos();
        let sin_azimuth = self.azimuth.sin();

        let offset = [
            cos_elevation * cos_azimuth * self.distance,
            sin_elevation * self.distance,
            cos_elevation * sin_azimuth * self.distance,
        ];

        camera.position = [
            self.target[0] + offset[0],
            self.target[1] + offset[1],
            self.target[2] + offset[2],
        ];

        camera.look_at(self.target, [0.0, 1.0, 0.0]);
    }
}

impl Default for OrbitController {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Matrix Utilities
// ============================================================================

/// Matrix multiply (row-major)
fn matrix_multiply(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut result = [[0.0f32; 4]; 4];

    for i in 0..4 {
        for j in 0..4 {
            result[i][j] =
                a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j] + a[i][3] * b[3][j];
        }
    }

    result
}

/// Matrix inverse (simplified for view/projection matrices)
fn matrix_inverse(m: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut result = [[0.0f32; 4]; 4];

    // Calculate cofactors
    let c00 = m[1][1] * (m[2][2] * m[3][3] - m[2][3] * m[3][2])
        - m[1][2] * (m[2][1] * m[3][3] - m[2][3] * m[3][1])
        + m[1][3] * (m[2][1] * m[3][2] - m[2][2] * m[3][1]);

    let c01 = -(m[1][0] * (m[2][2] * m[3][3] - m[2][3] * m[3][2])
        - m[1][2] * (m[2][0] * m[3][3] - m[2][3] * m[3][0])
        + m[1][3] * (m[2][0] * m[3][2] - m[2][2] * m[3][0]));

    let c02 = m[1][0] * (m[2][1] * m[3][3] - m[2][3] * m[3][1])
        - m[1][1] * (m[2][0] * m[3][3] - m[2][3] * m[3][0])
        + m[1][3] * (m[2][0] * m[3][1] - m[2][1] * m[3][0]);

    let c03 = -(m[1][0] * (m[2][1] * m[3][2] - m[2][2] * m[3][1])
        - m[1][1] * (m[2][0] * m[3][2] - m[2][2] * m[3][0])
        + m[1][2] * (m[2][0] * m[3][1] - m[2][1] * m[3][0]));

    let det = m[0][0] * c00 + m[0][1] * c01 + m[0][2] * c02 + m[0][3] * c03;

    if det.abs() < 1e-10 {
        return result; // Return zero matrix if singular
    }

    let inv_det = 1.0 / det;

    // First row of cofactors (already calculated)
    result[0][0] = c00 * inv_det;
    result[1][0] = c01 * inv_det;
    result[2][0] = c02 * inv_det;
    result[3][0] = c03 * inv_det;

    // Second row of cofactors
    result[0][1] = -(m[0][1] * (m[2][2] * m[3][3] - m[2][3] * m[3][2])
        - m[0][2] * (m[2][1] * m[3][3] - m[2][3] * m[3][1])
        + m[0][3] * (m[2][1] * m[3][2] - m[2][2] * m[3][1]))
        * inv_det;

    result[1][1] = (m[0][0] * (m[2][2] * m[3][3] - m[2][3] * m[3][2])
        - m[0][2] * (m[2][0] * m[3][3] - m[2][3] * m[3][0])
        + m[0][3] * (m[2][0] * m[3][2] - m[2][2] * m[3][0]))
        * inv_det;

    result[2][1] = -(m[0][0] * (m[2][1] * m[3][3] - m[2][3] * m[3][1])
        - m[0][1] * (m[2][0] * m[3][3] - m[2][3] * m[3][0])
        + m[0][3] * (m[2][0] * m[3][1] - m[2][1] * m[3][0]))
        * inv_det;

    result[3][1] = (m[0][0] * (m[2][1] * m[3][2] - m[2][2] * m[3][1])
        - m[0][1] * (m[2][0] * m[3][2] - m[2][2] * m[3][0])
        + m[0][2] * (m[2][0] * m[3][1] - m[2][1] * m[3][0]))
        * inv_det;

    // Third row of cofactors
    result[0][2] = (m[0][1] * (m[1][2] * m[3][3] - m[1][3] * m[3][2])
        - m[0][2] * (m[1][1] * m[3][3] - m[1][3] * m[3][1])
        + m[0][3] * (m[1][1] * m[3][2] - m[1][2] * m[3][1]))
        * inv_det;

    result[1][2] = -(m[0][0] * (m[1][2] * m[3][3] - m[1][3] * m[3][2])
        - m[0][2] * (m[1][0] * m[3][3] - m[1][3] * m[3][0])
        + m[0][3] * (m[1][0] * m[3][2] - m[1][2] * m[3][0]))
        * inv_det;

    result[2][2] = (m[0][0] * (m[1][1] * m[3][3] - m[1][3] * m[3][1])
        - m[0][1] * (m[1][0] * m[3][3] - m[1][3] * m[3][0])
        + m[0][3] * (m[1][0] * m[3][1] - m[1][1] * m[3][0]))
        * inv_det;

    result[3][2] = -(m[0][0] * (m[1][1] * m[3][2] - m[1][2] * m[3][1])
        - m[0][1] * (m[1][0] * m[3][2] - m[1][2] * m[3][0])
        + m[0][2] * (m[1][0] * m[3][1] - m[1][1] * m[3][0]))
        * inv_det;

    // Fourth row of cofactors
    result[0][3] = -(m[0][1] * (m[1][2] * m[2][3] - m[1][3] * m[2][2])
        - m[0][2] * (m[1][1] * m[2][3] - m[1][3] * m[2][1])
        + m[0][3] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]))
        * inv_det;

    result[1][3] = (m[0][0] * (m[1][2] * m[2][3] - m[1][3] * m[2][2])
        - m[0][2] * (m[1][0] * m[2][3] - m[1][3] * m[2][0])
        + m[0][3] * (m[1][0] * m[2][2] - m[1][2] * m[2][0]))
        * inv_det;

    result[2][3] = -(m[0][0] * (m[1][1] * m[2][3] - m[1][3] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][3] - m[1][3] * m[2][0])
        + m[0][3] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]))
        * inv_det;

    result[3][3] = (m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]))
        * inv_det;

    result
}

// ============================================================================
// Jittered Camera (for TAA)
// ============================================================================

/// Halton sequence for jitter
pub struct HaltonSequence {
    index: u32,
    base_x: u32,
    base_y: u32,
}

impl HaltonSequence {
    /// Creates new Halton sequence
    pub fn new() -> Self {
        Self {
            index: 0,
            base_x: 2,
            base_y: 3,
        }
    }

    /// Get next jitter values
    pub fn next(&mut self) -> (f32, f32) {
        self.index += 1;
        (
            Self::halton(self.index, self.base_x),
            Self::halton(self.index, self.base_y),
        )
    }

    /// Reset sequence
    pub fn reset(&mut self) {
        self.index = 0;
    }

    /// Halton sequence value
    fn halton(mut index: u32, base: u32) -> f32 {
        let mut result = 0.0f32;
        let mut f = 1.0 / base as f32;

        while index > 0 {
            result += f * (index % base) as f32;
            index /= base;
            f /= base as f32;
        }

        result
    }
}

impl Default for HaltonSequence {
    fn default() -> Self {
        Self::new()
    }
}

/// Apply jitter to projection matrix for TAA
pub fn apply_projection_jitter(
    projection: &mut [[f32; 4]; 4],
    jitter_x: f32,
    jitter_y: f32,
    width: f32,
    height: f32,
) {
    projection[2][0] += jitter_x * 2.0 / width;
    projection[2][1] += jitter_y * 2.0 / height;
}
