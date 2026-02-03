//! # Shadow Mapping
//!
//! Various shadow mapping techniques.

use alloc::vec::Vec;

/// Shadow mapper
pub struct ShadowMapper {
    config: ShadowConfig,
    cascades: Vec<CascadeData>,
    shadow_map_size: u32,
}

impl ShadowMapper {
    pub fn new(config: ShadowConfig) -> Self {
        let mut cascades = Vec::with_capacity(config.cascade_count as usize);
        for _ in 0..config.cascade_count {
            cascades.push(CascadeData::default());
        }

        Self {
            config,
            cascades,
            shadow_map_size: 2048,
        }
    }

    /// Update cascade splits based on camera
    pub fn update(&mut self, view: [[f32; 4]; 4], proj: [[f32; 4]; 4], near: f32, far: f32) {
        let lambda = self.config.cascade_split_lambda;

        for i in 0..self.config.cascade_count as usize {
            let p = (i as f32 + 1.0) / self.config.cascade_count as f32;

            // Logarithmic split
            let log_split = near * (far / near).powf(p);
            // Uniform split
            let uniform_split = near + (far - near) * p;
            // Practical split scheme
            let split = lambda * log_split + (1.0 - lambda) * uniform_split;

            self.cascades[i].split_depth = split;
        }

        // Calculate light matrices for each cascade
        for cascade in &mut self.cascades {
            cascade.update_matrix(view, proj, self.config.light_direction);
        }
    }

    /// Get shadow data for shader
    pub fn get_shadow_data(&self) -> ShadowData {
        let mut matrices = [[[0.0f32; 4]; 4]; 4];
        let mut split_depths = [0.0f32; 4];

        for (i, cascade) in self.cascades.iter().enumerate() {
            matrices[i] = cascade.light_view_proj;
            split_depths[i] = cascade.split_depth;
        }

        ShadowData {
            matrices,
            split_depths,
            cascade_count: self.config.cascade_count,
            bias: self.config.bias,
            normal_bias: self.config.normal_bias,
            filter_size: self.config.filter_size,
        }
    }
}

/// Shadow configuration
#[derive(Debug, Clone)]
pub struct ShadowConfig {
    pub cascade_count: u32,
    pub cascade_split_lambda: f32,
    pub light_direction: [f32; 3],
    pub bias: f32,
    pub normal_bias: f32,
    pub filter_size: f32,
    pub technique: ShadowTechnique,
}

impl Default for ShadowConfig {
    fn default() -> Self {
        Self {
            cascade_count: 4,
            cascade_split_lambda: 0.95,
            light_direction: [0.5, -1.0, 0.5],
            bias: 0.0005,
            normal_bias: 0.02,
            filter_size: 1.0,
            technique: ShadowTechnique::Pcf,
        }
    }
}

/// Shadow technique
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowTechnique {
    /// Basic shadow mapping
    Hard,
    /// Percentage Closer Filtering
    Pcf,
    /// Percentage Closer Soft Shadows
    Pcss,
    /// Variance Shadow Maps
    Vsm,
    /// Exponential Shadow Maps
    Esm,
    /// Moment Shadow Maps
    Msm,
}

/// Cascade data
#[derive(Debug, Clone, Default)]
struct CascadeData {
    split_depth: f32,
    light_view_proj: [[f32; 4]; 4],
    frustum_corners: [[f32; 3]; 8],
}

impl CascadeData {
    fn update_matrix(&mut self, _view: [[f32; 4]; 4], _proj: [[f32; 4]; 4], light_dir: [f32; 3]) {
        // Calculate frustum corners in world space
        // Then fit orthographic projection to cascade

        let center = self.frustum_center();
        let radius = self.frustum_radius();

        // Snap to texel grid to reduce shadow swimming
        let texel_size = radius * 2.0 / 2048.0;
        let center = [
            (center[0] / texel_size).floor() * texel_size,
            (center[1] / texel_size).floor() * texel_size,
            (center[2] / texel_size).floor() * texel_size,
        ];

        let light_view = look_at(
            [
                center[0] - light_dir[0] * radius,
                center[1] - light_dir[1] * radius,
                center[2] - light_dir[2] * radius,
            ],
            center,
            [0.0, 1.0, 0.0],
        );

        let light_proj = orthographic(-radius, radius, -radius, radius, 0.0, radius * 2.0);

        self.light_view_proj = mul_mat4(light_proj, light_view);
    }

    fn frustum_center(&self) -> [f32; 3] {
        let mut center = [0.0f32; 3];
        for corner in &self.frustum_corners {
            center[0] += corner[0];
            center[1] += corner[1];
            center[2] += corner[2];
        }
        [center[0] / 8.0, center[1] / 8.0, center[2] / 8.0]
    }

    fn frustum_radius(&self) -> f32 {
        let center = self.frustum_center();
        let mut max_dist = 0.0f32;

        for corner in &self.frustum_corners {
            let dx = corner[0] - center[0];
            let dy = corner[1] - center[1];
            let dz = corner[2] - center[2];
            max_dist = max_dist.max((dx * dx + dy * dy + dz * dz).sqrt());
        }

        max_dist
    }
}

/// Shadow data for GPU
#[derive(Debug, Clone)]
#[repr(C)]
pub struct ShadowData {
    pub matrices: [[[f32; 4]; 4]; 4],
    pub split_depths: [f32; 4],
    pub cascade_count: u32,
    pub bias: f32,
    pub normal_bias: f32,
    pub filter_size: f32,
}

/// PCF shadow sampling
pub struct PcfSampler;

impl PcfSampler {
    /// 9-tap PCF
    pub fn sample_3x3(shadow_map_size: f32) -> Vec<[f32; 2]> {
        let texel = 1.0 / shadow_map_size;
        vec![
            [-texel, -texel],
            [0.0, -texel],
            [texel, -texel],
            [-texel, 0.0],
            [0.0, 0.0],
            [texel, 0.0],
            [-texel, texel],
            [0.0, texel],
            [texel, texel],
        ]
    }

    /// 25-tap PCF
    pub fn sample_5x5(shadow_map_size: f32) -> Vec<[f32; 2]> {
        let texel = 1.0 / shadow_map_size;
        let mut samples = Vec::with_capacity(25);

        for y in -2i32..=2 {
            for x in -2i32..=2 {
                samples.push([x as f32 * texel, y as f32 * texel]);
            }
        }

        samples
    }

    /// Poisson disk sampling
    pub fn poisson_disk() -> Vec<[f32; 2]> {
        vec![
            [-0.94201624, -0.39906216],
            [0.94558609, -0.76890725],
            [-0.09418410, -0.92938870],
            [0.34495938, 0.29387760],
            [-0.91588581, 0.45771432],
            [-0.81544232, -0.87912464],
            [-0.38277543, 0.27676845],
            [0.97484398, 0.75648379],
            [0.44323325, -0.97511554],
            [0.53742981, -0.47373420],
            [-0.26496911, -0.41893023],
            [0.79197514, 0.19090188],
            [-0.24188840, 0.99706507],
            [-0.81409955, 0.91437590],
            [0.19984126, 0.78641367],
            [0.14383161, -0.14100790],
        ]
    }
}

/// Point light shadow cubemap
pub struct PointShadow {
    pub light_pos: [f32; 3],
    pub far_plane: f32,
    pub matrices: [[[f32; 4]; 4]; 6],
}

impl PointShadow {
    pub fn new(light_pos: [f32; 3], near: f32, far: f32) -> Self {
        let proj = perspective(90.0f32.to_radians(), 1.0, near, far);

        // +X, -X, +Y, -Y, +Z, -Z
        let targets = [
            [light_pos[0] + 1.0, light_pos[1], light_pos[2]],
            [light_pos[0] - 1.0, light_pos[1], light_pos[2]],
            [light_pos[0], light_pos[1] + 1.0, light_pos[2]],
            [light_pos[0], light_pos[1] - 1.0, light_pos[2]],
            [light_pos[0], light_pos[1], light_pos[2] + 1.0],
            [light_pos[0], light_pos[1], light_pos[2] - 1.0],
        ];

        let ups = [
            [0.0, -1.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0],
            [0.0, -1.0, 0.0],
            [0.0, -1.0, 0.0],
        ];

        let mut matrices = [[[0.0f32; 4]; 4]; 6];
        for i in 0..6 {
            let view = look_at(light_pos, targets[i], ups[i]);
            matrices[i] = mul_mat4(proj, view);
        }

        Self {
            light_pos,
            far_plane: far,
            matrices,
        }
    }
}

// Matrix helpers

fn look_at(eye: [f32; 3], target: [f32; 3], up: [f32; 3]) -> [[f32; 4]; 4] {
    let f = normalize([target[0] - eye[0], target[1] - eye[1], target[2] - eye[2]]);
    let s = normalize(cross(f, up));
    let u = cross(s, f);

    [
        [s[0], u[0], -f[0], 0.0],
        [s[1], u[1], -f[1], 0.0],
        [s[2], u[2], -f[2], 0.0],
        [-dot(s, eye), -dot(u, eye), dot(f, eye), 1.0],
    ]
}

fn orthographic(
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    near: f32,
    far: f32,
) -> [[f32; 4]; 4] {
    [
        [2.0 / (right - left), 0.0, 0.0, 0.0],
        [0.0, 2.0 / (top - bottom), 0.0, 0.0],
        [0.0, 0.0, -2.0 / (far - near), 0.0],
        [
            -(right + left) / (right - left),
            -(top + bottom) / (top - bottom),
            -(far + near) / (far - near),
            1.0,
        ],
    ]
}

fn perspective(fov: f32, aspect: f32, near: f32, far: f32) -> [[f32; 4]; 4] {
    let f = 1.0 / (fov / 2.0).tan();
    [
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, (far + near) / (near - far), -1.0],
        [0.0, 0.0, (2.0 * far * near) / (near - far), 0.0],
    ]
}

fn mul_mat4(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
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

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 0.0 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        v
    }
}

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
