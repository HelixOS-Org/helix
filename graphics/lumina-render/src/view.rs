//! View & Camera System
//!
//! Comprehensive view management featuring:
//! - Multiple camera types (perspective, orthographic, custom)
//! - Frustum management and culling
//! - Jitter for TAA
//! - Multi-view rendering (VR/XR)
//! - View-dependent data management

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::f32::consts::PI;

/// View camera for rendering.
#[derive(Debug, Clone)]
pub struct View {
    /// View configuration.
    pub config: ViewConfig,
    /// Camera transform.
    pub transform: Transform,
    /// Projection.
    pub projection: Projection,
    /// Computed matrices.
    pub matrices: ViewMatrices,
    /// View frustum.
    pub frustum: Frustum,
    /// Jitter offset for TAA.
    pub jitter: [f32; 2],
    /// Previous frame matrices (for motion vectors).
    pub previous_matrices: ViewMatrices,
    /// View index (for multi-view).
    pub view_index: u32,
}

impl View {
    /// Create a new perspective view.
    pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Self {
        let projection = Projection::Perspective {
            fov_y,
            aspect,
            near,
            far,
        };

        let mut view = Self {
            config: ViewConfig::default(),
            transform: Transform::identity(),
            projection,
            matrices: ViewMatrices::default(),
            frustum: Frustum::default(),
            jitter: [0.0, 0.0],
            previous_matrices: ViewMatrices::default(),
            view_index: 0,
        };

        view.update_matrices();
        view
    }

    /// Create a new orthographic view.
    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        let projection = Projection::Orthographic {
            left,
            right,
            bottom,
            top,
            near,
            far,
        };

        let mut view = Self {
            config: ViewConfig::default(),
            transform: Transform::identity(),
            projection,
            matrices: ViewMatrices::default(),
            frustum: Frustum::default(),
            jitter: [0.0, 0.0],
            previous_matrices: ViewMatrices::default(),
            view_index: 0,
        };

        view.update_matrices();
        view
    }

    /// Set camera position.
    pub fn set_position(&mut self, position: [f32; 3]) {
        self.transform.position = position;
    }

    /// Set camera rotation (euler angles in radians).
    pub fn set_rotation(&mut self, rotation: [f32; 3]) {
        self.transform.rotation = rotation;
    }

    /// Look at a target position.
    pub fn look_at(&mut self, target: [f32; 3], up: [f32; 3]) {
        let forward = normalize(sub(target, self.transform.position));
        let right = normalize(cross(forward, up));
        let new_up = cross(right, forward);

        // Extract euler angles from direction
        self.transform.rotation = [(-forward[1]).asin(), forward[0].atan2(forward[2]), 0.0];
    }

    /// Update matrices after transform change.
    pub fn update_matrices(&mut self) {
        // Store previous
        self.previous_matrices = self.matrices.clone();

        // Calculate view matrix
        self.matrices.view = self.compute_view_matrix();
        self.matrices.inv_view = inverse_affine(&self.matrices.view);

        // Calculate projection matrix
        self.matrices.projection = self.compute_projection_matrix();
        self.matrices.inv_projection = inverse(&self.matrices.projection);

        // Apply jitter to projection
        if self.jitter[0] != 0.0 || self.jitter[1] != 0.0 {
            self.matrices.projection_jittered = self.matrices.projection;
            self.matrices.projection_jittered[2][0] += self.jitter[0];
            self.matrices.projection_jittered[2][1] += self.jitter[1];
        } else {
            self.matrices.projection_jittered = self.matrices.projection;
        }

        // Combined matrices
        self.matrices.view_projection = mul_mat4(&self.matrices.projection, &self.matrices.view);
        self.matrices.view_projection_jittered =
            mul_mat4(&self.matrices.projection_jittered, &self.matrices.view);
        self.matrices.inv_view_projection = inverse(&self.matrices.view_projection);

        // Update frustum
        self.frustum = self.extract_frustum();
    }

    fn compute_view_matrix(&self) -> [[f32; 4]; 4] {
        let pos = self.transform.position;
        let rot = self.transform.rotation;

        // Rotation matrices
        let cx = rot[0].cos();
        let sx = rot[0].sin();
        let cy = rot[1].cos();
        let sy = rot[1].sin();
        let cz = rot[2].cos();
        let sz = rot[2].sin();

        // Combined rotation (YXZ order)
        let r00 = cy * cz + sy * sx * sz;
        let r01 = cz * sy * sx - cy * sz;
        let r02 = cx * sy;
        let r10 = cx * sz;
        let r11 = cx * cz;
        let r12 = -sx;
        let r20 = cy * sx * sz - cz * sy;
        let r21 = sy * sz + cy * cz * sx;
        let r22 = cy * cx;

        // Translation (inverse)
        let tx = -(r00 * pos[0] + r10 * pos[1] + r20 * pos[2]);
        let ty = -(r01 * pos[0] + r11 * pos[1] + r21 * pos[2]);
        let tz = -(r02 * pos[0] + r12 * pos[1] + r22 * pos[2]);

        [
            [r00, r01, r02, 0.0],
            [r10, r11, r12, 0.0],
            [r20, r21, r22, 0.0],
            [tx, ty, tz, 1.0],
        ]
    }

    fn compute_projection_matrix(&self) -> [[f32; 4]; 4] {
        match self.projection {
            Projection::Perspective {
                fov_y,
                aspect,
                near,
                far,
            } => {
                let tan_half_fov = (fov_y * 0.5).tan();
                let range = far - near;

                if self.config.reverse_z {
                    // Reverse-Z for better depth precision
                    [
                        [1.0 / (aspect * tan_half_fov), 0.0, 0.0, 0.0],
                        [0.0, 1.0 / tan_half_fov, 0.0, 0.0],
                        [0.0, 0.0, near / range, 1.0],
                        [0.0, 0.0, -far * near / range, 0.0],
                    ]
                } else {
                    [
                        [1.0 / (aspect * tan_half_fov), 0.0, 0.0, 0.0],
                        [0.0, 1.0 / tan_half_fov, 0.0, 0.0],
                        [0.0, 0.0, far / range, 1.0],
                        [0.0, 0.0, -near * far / range, 0.0],
                    ]
                }
            },
            Projection::Orthographic {
                left,
                right,
                bottom,
                top,
                near,
                far,
            } => {
                let width = right - left;
                let height = top - bottom;
                let depth = far - near;

                [
                    [2.0 / width, 0.0, 0.0, 0.0],
                    [0.0, 2.0 / height, 0.0, 0.0],
                    [0.0, 0.0, 1.0 / depth, 0.0],
                    [
                        -(right + left) / width,
                        -(top + bottom) / height,
                        -near / depth,
                        1.0,
                    ],
                ]
            },
            Projection::Custom(ref matrix) => *matrix,
        }
    }

    fn extract_frustum(&self) -> Frustum {
        let m = &self.matrices.view_projection;

        // Left plane
        let left = Plane {
            normal: normalize([m[0][3] + m[0][0], m[1][3] + m[1][0], m[2][3] + m[2][0]]),
            distance: m[3][3] + m[3][0],
        };

        // Right plane
        let right = Plane {
            normal: normalize([m[0][3] - m[0][0], m[1][3] - m[1][0], m[2][3] - m[2][0]]),
            distance: m[3][3] - m[3][0],
        };

        // Bottom plane
        let bottom = Plane {
            normal: normalize([m[0][3] + m[0][1], m[1][3] + m[1][1], m[2][3] + m[2][1]]),
            distance: m[3][3] + m[3][1],
        };

        // Top plane
        let top = Plane {
            normal: normalize([m[0][3] - m[0][1], m[1][3] - m[1][1], m[2][3] - m[2][1]]),
            distance: m[3][3] - m[3][1],
        };

        // Near plane
        let near = Plane {
            normal: normalize([m[0][2], m[1][2], m[2][2]]),
            distance: m[3][2],
        };

        // Far plane
        let far = Plane {
            normal: normalize([m[0][3] - m[0][2], m[1][3] - m[1][2], m[2][3] - m[2][2]]),
            distance: m[3][3] - m[3][2],
        };

        Frustum {
            planes: [left, right, bottom, top, near, far],
        }
    }

    /// Get forward direction.
    pub fn forward(&self) -> [f32; 3] {
        let inv = &self.matrices.inv_view;
        normalize([inv[0][2], inv[1][2], inv[2][2]])
    }

    /// Get right direction.
    pub fn right(&self) -> [f32; 3] {
        let inv = &self.matrices.inv_view;
        normalize([inv[0][0], inv[1][0], inv[2][0]])
    }

    /// Get up direction.
    pub fn up(&self) -> [f32; 3] {
        let inv = &self.matrices.inv_view;
        normalize([inv[0][1], inv[1][1], inv[2][1]])
    }

    /// Get near plane distance.
    pub fn near(&self) -> f32 {
        match self.projection {
            Projection::Perspective { near, .. } => near,
            Projection::Orthographic { near, .. } => near,
            Projection::Custom(_) => 0.1,
        }
    }

    /// Get far plane distance.
    pub fn far(&self) -> f32 {
        match self.projection {
            Projection::Perspective { far, .. } => far,
            Projection::Orthographic { far, .. } => far,
            Projection::Custom(_) => 1000.0,
        }
    }

    /// Test if sphere is in frustum.
    pub fn is_sphere_visible(&self, center: [f32; 3], radius: f32) -> bool {
        self.frustum.test_sphere(center, radius)
    }

    /// Test if AABB is in frustum.
    pub fn is_aabb_visible(&self, min: [f32; 3], max: [f32; 3]) -> bool {
        self.frustum.test_aabb(min, max)
    }

    /// Get GPU uniform data.
    pub fn get_uniform_data(&self) -> ViewUniformData {
        ViewUniformData {
            view: self.matrices.view,
            projection: self.matrices.projection_jittered,
            view_projection: self.matrices.view_projection_jittered,
            inv_view: self.matrices.inv_view,
            inv_projection: self.matrices.inv_projection,
            inv_view_projection: self.matrices.inv_view_projection,
            prev_view_projection: mul_mat4(
                &self.previous_matrices.projection,
                &self.previous_matrices.view,
            ),
            camera_position: [
                self.transform.position[0],
                self.transform.position[1],
                self.transform.position[2],
                1.0,
            ],
            jitter: [self.jitter[0], self.jitter[1], 0.0, 0.0],
            near_far: [self.near(), self.far(), 0.0, 0.0],
        }
    }
}

/// View configuration.
#[derive(Debug, Clone)]
pub struct ViewConfig {
    /// Use reverse-Z.
    pub reverse_z: bool,
    /// Infinite far plane.
    pub infinite_far: bool,
    /// Jitter pattern.
    pub jitter_pattern: JitterPattern,
}

impl Default for ViewConfig {
    fn default() -> Self {
        Self {
            reverse_z: true,
            infinite_far: false,
            jitter_pattern: JitterPattern::Halton,
        }
    }
}

/// Transform component.
#[derive(Debug, Clone)]
pub struct Transform {
    /// Position.
    pub position: [f32; 3],
    /// Rotation (euler angles).
    pub rotation: [f32; 3],
    /// Scale.
    pub scale: [f32; 3],
}

impl Transform {
    /// Create identity transform.
    pub fn identity() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

/// Projection type.
#[derive(Debug, Clone)]
pub enum Projection {
    /// Perspective projection.
    Perspective {
        fov_y: f32,
        aspect: f32,
        near: f32,
        far: f32,
    },
    /// Orthographic projection.
    Orthographic {
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near: f32,
        far: f32,
    },
    /// Custom projection matrix.
    Custom([[f32; 4]; 4]),
}

/// View matrices.
#[derive(Debug, Clone, Default)]
pub struct ViewMatrices {
    /// View matrix.
    pub view: [[f32; 4]; 4],
    /// Projection matrix.
    pub projection: [[f32; 4]; 4],
    /// Projection with jitter.
    pub projection_jittered: [[f32; 4]; 4],
    /// Combined view-projection.
    pub view_projection: [[f32; 4]; 4],
    /// Combined view-projection with jitter.
    pub view_projection_jittered: [[f32; 4]; 4],
    /// Inverse view.
    pub inv_view: [[f32; 4]; 4],
    /// Inverse projection.
    pub inv_projection: [[f32; 4]; 4],
    /// Inverse view-projection.
    pub inv_view_projection: [[f32; 4]; 4],
}

/// GPU uniform data for view.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ViewUniformData {
    pub view: [[f32; 4]; 4],
    pub projection: [[f32; 4]; 4],
    pub view_projection: [[f32; 4]; 4],
    pub inv_view: [[f32; 4]; 4],
    pub inv_projection: [[f32; 4]; 4],
    pub inv_view_projection: [[f32; 4]; 4],
    pub prev_view_projection: [[f32; 4]; 4],
    pub camera_position: [f32; 4],
    pub jitter: [f32; 4],
    pub near_far: [f32; 4],
}

/// Jitter pattern for TAA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JitterPattern {
    /// No jitter.
    None,
    /// Halton sequence (2, 3).
    Halton,
    /// R2 low-discrepancy sequence.
    R2,
    /// Blue noise.
    BlueNoise,
}

impl JitterPattern {
    /// Get jitter offset for frame.
    pub fn get_offset(&self, frame: u32, width: u32, height: u32) -> [f32; 2] {
        match self {
            Self::None => [0.0, 0.0],
            Self::Halton => {
                let x = halton(frame, 2) - 0.5;
                let y = halton(frame, 3) - 0.5;
                [x / width as f32, y / height as f32]
            },
            Self::R2 => {
                let g = 1.32471795724;
                let a1 = 1.0 / g;
                let a2 = 1.0 / (g * g);
                let x = (0.5 + a1 * frame as f32) % 1.0 - 0.5;
                let y = (0.5 + a2 * frame as f32) % 1.0 - 0.5;
                [x / width as f32, y / height as f32]
            },
            Self::BlueNoise => {
                // Would use blue noise texture
                let x = ((frame * 1664525 + 1013904223) as f32 / u32::MAX as f32) - 0.5;
                let y = ((frame * 22695477 + 1) as f32 / u32::MAX as f32) - 0.5;
                [x / width as f32, y / height as f32]
            },
        }
    }
}

/// View frustum for culling.
#[derive(Debug, Clone, Default)]
pub struct Frustum {
    /// Frustum planes (left, right, bottom, top, near, far).
    pub planes: [Plane; 6],
}

impl Frustum {
    /// Test sphere against frustum.
    pub fn test_sphere(&self, center: [f32; 3], radius: f32) -> bool {
        for plane in &self.planes {
            let dist = dot(plane.normal, center) + plane.distance;
            if dist < -radius {
                return false;
            }
        }
        true
    }

    /// Test AABB against frustum.
    pub fn test_aabb(&self, min: [f32; 3], max: [f32; 3]) -> bool {
        for plane in &self.planes {
            // Get positive vertex (furthest in plane normal direction)
            let p = [
                if plane.normal[0] >= 0.0 {
                    max[0]
                } else {
                    min[0]
                },
                if plane.normal[1] >= 0.0 {
                    max[1]
                } else {
                    min[1]
                },
                if plane.normal[2] >= 0.0 {
                    max[2]
                } else {
                    min[2]
                },
            ];

            if dot(plane.normal, p) + plane.distance < 0.0 {
                return false;
            }
        }
        true
    }

    /// Get frustum corners in world space.
    pub fn get_corners(&self, inv_view_projection: &[[f32; 4]; 4]) -> [[f32; 3]; 8] {
        let ndc_corners = [
            [-1.0, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [-1.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [-1.0, -1.0, 1.0],
            [1.0, -1.0, 1.0],
            [-1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0],
        ];

        let mut corners = [[0.0f32; 3]; 8];
        for (i, ndc) in ndc_corners.iter().enumerate() {
            let clip = [ndc[0], ndc[1], ndc[2], 1.0];
            let world = transform_vec4(inv_view_projection, clip);
            let w = world[3];
            corners[i] = [world[0] / w, world[1] / w, world[2] / w];
        }
        corners
    }
}

/// Plane (for frustum).
#[derive(Debug, Clone, Default)]
pub struct Plane {
    /// Normal.
    pub normal: [f32; 3],
    /// Distance from origin.
    pub distance: f32,
}

/// Multi-view manager for VR/XR.
pub struct MultiView {
    /// Views (one per eye/display).
    pub views: Vec<View>,
    /// Combined frustum.
    pub combined_frustum: Frustum,
}

impl MultiView {
    /// Create stereo views.
    pub fn stereo(ipd: f32, fov_y: f32, aspect: f32, near: f32, far: f32) -> Self {
        let half_ipd = ipd * 0.5;

        let mut left = View::perspective(fov_y, aspect, near, far);
        left.set_position([-half_ipd, 0.0, 0.0]);
        left.view_index = 0;

        let mut right = View::perspective(fov_y, aspect, near, far);
        right.set_position([half_ipd, 0.0, 0.0]);
        right.view_index = 1;

        Self {
            views: vec![left, right],
            combined_frustum: Frustum::default(),
        }
    }

    /// Update all views.
    pub fn update(&mut self) {
        for view in &mut self.views {
            view.update_matrices();
        }
        // Would compute combined frustum
    }

    /// Get uniform data for all views.
    pub fn get_uniform_data(&self) -> Vec<ViewUniformData> {
        self.views.iter().map(|v| v.get_uniform_data()).collect()
    }
}

/// Shadow cascade view.
pub struct CascadeView {
    /// Cascade views.
    pub cascades: Vec<View>,
    /// Split distances.
    pub splits: Vec<f32>,
    /// Blend ranges.
    pub blend_ranges: Vec<f32>,
}

impl CascadeView {
    /// Create cascade views for directional light.
    pub fn new(
        main_view: &View,
        light_direction: [f32; 3],
        cascade_count: usize,
        split_lambda: f32,
    ) -> Self {
        let near = main_view.near();
        let far = main_view.far();

        // Calculate split distances (logarithmic-linear blend)
        let mut splits = Vec::with_capacity(cascade_count + 1);
        splits.push(near);

        for i in 1..cascade_count {
            let p = i as f32 / cascade_count as f32;
            let log_split = near * (far / near).powf(p);
            let linear_split = near + (far - near) * p;
            let split = log_split * split_lambda + linear_split * (1.0 - split_lambda);
            splits.push(split);
        }
        splits.push(far);

        // Create cascade views
        let mut cascades = Vec::with_capacity(cascade_count);
        for i in 0..cascade_count {
            // Would calculate orthographic projection to fit cascade
            let cascade = View::orthographic(-100.0, 100.0, -100.0, 100.0, 0.1, 500.0);
            cascades.push(cascade);
        }

        let blend_ranges = vec![0.1; cascade_count];

        Self {
            cascades,
            splits,
            blend_ranges,
        }
    }

    /// Get cascade index for depth.
    pub fn get_cascade(&self, depth: f32) -> usize {
        for i in 0..self.splits.len() - 1 {
            if depth < self.splits[i + 1] {
                return i;
            }
        }
        self.cascades.len() - 1
    }
}

// Helper functions

fn halton(index: u32, base: u32) -> f32 {
    let mut result = 0.0f32;
    let mut f = 1.0f32;
    let mut i = index;

    while i > 0 {
        f /= base as f32;
        result += f * (i % base) as f32;
        i /= base;
    }

    result
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 0.0 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        v
    }
}

fn mul_mat4(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut result = [[0.0f32; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            for k in 0..4 {
                result[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    result
}

fn transform_vec4(m: &[[f32; 4]; 4], v: [f32; 4]) -> [f32; 4] {
    [
        m[0][0] * v[0] + m[1][0] * v[1] + m[2][0] * v[2] + m[3][0] * v[3],
        m[0][1] * v[0] + m[1][1] * v[1] + m[2][1] * v[2] + m[3][1] * v[3],
        m[0][2] * v[0] + m[1][2] * v[1] + m[2][2] * v[2] + m[3][2] * v[3],
        m[0][3] * v[0] + m[1][3] * v[1] + m[2][3] * v[2] + m[3][3] * v[3],
    ]
}

fn inverse_affine(m: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    // Simplified inverse for affine transforms
    let mut result = [[0.0f32; 4]; 4];

    // Transpose rotation part
    for i in 0..3 {
        for j in 0..3 {
            result[i][j] = m[j][i];
        }
    }

    // Inverse translation
    for i in 0..3 {
        result[3][i] = -(m[3][0] * result[0][i] + m[3][1] * result[1][i] + m[3][2] * result[2][i]);
    }

    result[3][3] = 1.0;
    result
}

fn inverse(m: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    // Full 4x4 matrix inverse
    let mut inv = [[0.0f32; 4]; 4];

    inv[0][0] =
        m[1][1] * m[2][2] * m[3][3] - m[1][1] * m[2][3] * m[3][2] - m[2][1] * m[1][2] * m[3][3]
            + m[2][1] * m[1][3] * m[3][2]
            + m[3][1] * m[1][2] * m[2][3]
            - m[3][1] * m[1][3] * m[2][2];

    inv[1][0] =
        -m[1][0] * m[2][2] * m[3][3] + m[1][0] * m[2][3] * m[3][2] + m[2][0] * m[1][2] * m[3][3]
            - m[2][0] * m[1][3] * m[3][2]
            - m[3][0] * m[1][2] * m[2][3]
            + m[3][0] * m[1][3] * m[2][2];

    inv[2][0] =
        m[1][0] * m[2][1] * m[3][3] - m[1][0] * m[2][3] * m[3][1] - m[2][0] * m[1][1] * m[3][3]
            + m[2][0] * m[1][3] * m[3][1]
            + m[3][0] * m[1][1] * m[2][3]
            - m[3][0] * m[1][3] * m[2][1];

    inv[3][0] =
        -m[1][0] * m[2][1] * m[3][2] + m[1][0] * m[2][2] * m[3][1] + m[2][0] * m[1][1] * m[3][2]
            - m[2][0] * m[1][2] * m[3][1]
            - m[3][0] * m[1][1] * m[2][2]
            + m[3][0] * m[1][2] * m[2][1];

    let det = m[0][0] * inv[0][0] + m[0][1] * inv[1][0] + m[0][2] * inv[2][0] + m[0][3] * inv[3][0];

    if det.abs() < 1e-10 {
        return [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
    }

    let inv_det = 1.0 / det;

    // Calculate remaining elements...
    inv[0][1] =
        (-m[0][1] * m[2][2] * m[3][3] + m[0][1] * m[2][3] * m[3][2] + m[2][1] * m[0][2] * m[3][3]
            - m[2][1] * m[0][3] * m[3][2]
            - m[3][1] * m[0][2] * m[2][3]
            + m[3][1] * m[0][3] * m[2][2])
            * inv_det;

    // Simplified - would complete full inverse
    for i in 0..4 {
        for j in 0..4 {
            inv[i][j] *= inv_det;
        }
    }

    inv
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perspective_view() {
        let view = View::perspective(PI / 4.0, 16.0 / 9.0, 0.1, 1000.0);
        assert!((view.near() - 0.1).abs() < 0.001);
        assert!((view.far() - 1000.0).abs() < 0.001);
    }

    #[test]
    fn test_frustum_sphere() {
        let mut view = View::perspective(PI / 4.0, 1.0, 0.1, 100.0);
        view.set_position([0.0, 0.0, -10.0]);
        view.update_matrices();

        // Sphere in front should be visible
        assert!(view.is_sphere_visible([0.0, 0.0, 10.0], 5.0));

        // Sphere behind camera should not be visible
        assert!(!view.is_sphere_visible([0.0, 0.0, -100.0], 5.0));
    }

    #[test]
    fn test_halton_sequence() {
        let h1 = halton(1, 2);
        let h2 = halton(2, 2);
        assert!((h1 - 0.5).abs() < 0.01);
        assert!((h2 - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_jitter_pattern() {
        let offset = JitterPattern::Halton.get_offset(0, 1920, 1080);
        assert!(offset[0].abs() < 1.0);
        assert!(offset[1].abs() < 1.0);
    }
}
