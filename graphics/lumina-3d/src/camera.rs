//! # Camera System
//!
//! Camera types and controls.

/// Camera
#[derive(Debug, Clone)]
pub struct Camera {
    pub position: [f32; 3],
    pub rotation: [f32; 4], // Quaternion
    pub fov: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
    pub projection: Projection,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            position: [0.0, 0.0, 5.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            fov: 60.0f32.to_radians(),
            aspect: 16.0 / 9.0,
            near: 0.1,
            far: 1000.0,
            projection: Projection::Perspective,
        }
    }

    /// Create perspective camera
    pub fn perspective(fov: f32, aspect: f32, near: f32, far: f32) -> Self {
        Self {
            projection: Projection::Perspective,
            fov: fov.to_radians(),
            aspect,
            near,
            far,
            ..Self::new()
        }
    }

    /// Create orthographic camera
    pub fn orthographic(width: f32, height: f32, near: f32, far: f32) -> Self {
        Self {
            projection: Projection::Orthographic { width, height },
            near,
            far,
            ..Self::new()
        }
    }

    /// Get view matrix
    pub fn view_matrix(&self) -> [[f32; 4]; 4] {
        let (x, y, z, w) = (
            self.rotation[0],
            self.rotation[1],
            self.rotation[2],
            self.rotation[3],
        );

        // Rotation matrix from quaternion
        let r = [
            [
                1.0 - 2.0 * (y * y + z * z),
                2.0 * (x * y - z * w),
                2.0 * (x * z + y * w),
                0.0,
            ],
            [
                2.0 * (x * y + z * w),
                1.0 - 2.0 * (x * x + z * z),
                2.0 * (y * z - x * w),
                0.0,
            ],
            [
                2.0 * (x * z - y * w),
                2.0 * (y * z + x * w),
                1.0 - 2.0 * (x * x + y * y),
                0.0,
            ],
            [0.0, 0.0, 0.0, 1.0],
        ];

        // Translation
        let t = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [-self.position[0], -self.position[1], -self.position[2], 1.0],
        ];

        mul_mat4(r, t)
    }

    /// Get projection matrix
    pub fn projection_matrix(&self) -> [[f32; 4]; 4] {
        match self.projection {
            Projection::Perspective => {
                let f = 1.0 / (self.fov / 2.0).tan();
                let nf = 1.0 / (self.near - self.far);

                [
                    [f / self.aspect, 0.0, 0.0, 0.0],
                    [0.0, f, 0.0, 0.0],
                    [0.0, 0.0, (self.far + self.near) * nf, -1.0],
                    [0.0, 0.0, 2.0 * self.far * self.near * nf, 0.0],
                ]
            },
            Projection::Orthographic { width, height } => {
                let hw = width / 2.0;
                let hh = height / 2.0;

                [
                    [1.0 / hw, 0.0, 0.0, 0.0],
                    [0.0, 1.0 / hh, 0.0, 0.0],
                    [0.0, 0.0, -2.0 / (self.far - self.near), 0.0],
                    [
                        0.0,
                        0.0,
                        -(self.far + self.near) / (self.far - self.near),
                        1.0,
                    ],
                ]
            },
        }
    }

    /// Get forward direction
    pub fn forward(&self) -> [f32; 3] {
        let (x, y, z, w) = (
            self.rotation[0],
            self.rotation[1],
            self.rotation[2],
            self.rotation[3],
        );
        [
            2.0 * (x * z + w * y),
            2.0 * (y * z - w * x),
            1.0 - 2.0 * (x * x + y * y),
        ]
    }

    /// Get right direction
    pub fn right(&self) -> [f32; 3] {
        let (x, y, z, w) = (
            self.rotation[0],
            self.rotation[1],
            self.rotation[2],
            self.rotation[3],
        );
        [
            1.0 - 2.0 * (y * y + z * z),
            2.0 * (x * y + w * z),
            2.0 * (x * z - w * y),
        ]
    }

    /// Get up direction
    pub fn up(&self) -> [f32; 3] {
        let (x, y, z, w) = (
            self.rotation[0],
            self.rotation[1],
            self.rotation[2],
            self.rotation[3],
        );
        [
            2.0 * (x * y - w * z),
            1.0 - 2.0 * (x * x + z * z),
            2.0 * (y * z + w * x),
        ]
    }

    /// Look at a target
    pub fn look_at(&mut self, target: [f32; 3], up: [f32; 3]) {
        let forward = normalize([
            target[0] - self.position[0],
            target[1] - self.position[1],
            target[2] - self.position[2],
        ]);
        let right = normalize(cross(forward, up));
        let up = cross(right, forward);

        // Convert rotation matrix to quaternion
        let trace = right[0] + up[1] - forward[2];

        if trace > 0.0 {
            let s = 0.5 / (trace + 1.0).sqrt();
            self.rotation = [
                (up[2] - (-forward[1])) * s,
                ((-forward[0]) - right[2]) * s,
                (right[1] - up[0]) * s,
                0.25 / s,
            ];
        } else if right[0] > up[1] && right[0] > -forward[2] {
            let s = 2.0 * (1.0 + right[0] - up[1] - (-forward[2])).sqrt();
            self.rotation = [
                0.25 * s,
                (right[1] + up[0]) / s,
                ((-forward[0]) + right[2]) / s,
                (up[2] - (-forward[1])) / s,
            ];
        } else if up[1] > -forward[2] {
            let s = 2.0 * (1.0 + up[1] - right[0] - (-forward[2])).sqrt();
            self.rotation = [
                (right[1] + up[0]) / s,
                0.25 * s,
                (up[2] + (-forward[1])) / s,
                ((-forward[0]) - right[2]) / s,
            ];
        } else {
            let s = 2.0 * (1.0 + (-forward[2]) - right[0] - up[1]).sqrt();
            self.rotation = [
                ((-forward[0]) + right[2]) / s,
                (up[2] + (-forward[1])) / s,
                0.25 * s,
                (right[1] - up[0]) / s,
            ];
        }
    }

    /// Get frustum planes for culling
    pub fn frustum_planes(&self) -> [Plane; 6] {
        let vp = mul_mat4(self.projection_matrix(), self.view_matrix());

        [
            // Left
            Plane::from_coefficients(
                vp[0][3] + vp[0][0],
                vp[1][3] + vp[1][0],
                vp[2][3] + vp[2][0],
                vp[3][3] + vp[3][0],
            ),
            // Right
            Plane::from_coefficients(
                vp[0][3] - vp[0][0],
                vp[1][3] - vp[1][0],
                vp[2][3] - vp[2][0],
                vp[3][3] - vp[3][0],
            ),
            // Bottom
            Plane::from_coefficients(
                vp[0][3] + vp[0][1],
                vp[1][3] + vp[1][1],
                vp[2][3] + vp[2][1],
                vp[3][3] + vp[3][1],
            ),
            // Top
            Plane::from_coefficients(
                vp[0][3] - vp[0][1],
                vp[1][3] - vp[1][1],
                vp[2][3] - vp[2][1],
                vp[3][3] - vp[3][1],
            ),
            // Near
            Plane::from_coefficients(
                vp[0][3] + vp[0][2],
                vp[1][3] + vp[1][2],
                vp[2][3] + vp[2][2],
                vp[3][3] + vp[3][2],
            ),
            // Far
            Plane::from_coefficients(
                vp[0][3] - vp[0][2],
                vp[1][3] - vp[1][2],
                vp[2][3] - vp[2][2],
                vp[3][3] - vp[3][2],
            ),
        ]
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}

/// Projection type
#[derive(Debug, Clone, Copy)]
pub enum Projection {
    Perspective,
    Orthographic { width: f32, height: f32 },
}

/// Frustum plane
#[derive(Debug, Clone, Copy)]
pub struct Plane {
    pub normal: [f32; 3],
    pub distance: f32,
}

impl Plane {
    pub fn from_coefficients(a: f32, b: f32, c: f32, d: f32) -> Self {
        let len = (a * a + b * b + c * c).sqrt();
        Self {
            normal: [a / len, b / len, c / len],
            distance: d / len,
        }
    }

    pub fn distance_to_point(&self, point: [f32; 3]) -> f32 {
        self.normal[0] * point[0]
            + self.normal[1] * point[1]
            + self.normal[2] * point[2]
            + self.distance
    }
}

/// Camera controller trait
pub trait CameraController {
    fn update(&mut self, camera: &mut Camera, dt: f32);
    fn on_input(&mut self, event: &CameraInputEvent);
}

/// Camera input event
#[derive(Debug, Clone)]
pub enum CameraInputEvent {
    MouseMove { delta_x: f32, delta_y: f32 },
    MouseScroll { delta: f32 },
    KeyDown { key: u32 },
    KeyUp { key: u32 },
}

/// FPS camera controller
pub struct FpsCameraController {
    pub move_speed: f32,
    pub look_sensitivity: f32,
    pub yaw: f32,
    pub pitch: f32,
    move_forward: bool,
    move_backward: bool,
    move_left: bool,
    move_right: bool,
    move_up: bool,
    move_down: bool,
}

impl FpsCameraController {
    pub fn new() -> Self {
        Self {
            move_speed: 10.0,
            look_sensitivity: 0.1,
            yaw: 0.0,
            pitch: 0.0,
            move_forward: false,
            move_backward: false,
            move_left: false,
            move_right: false,
            move_up: false,
            move_down: false,
        }
    }
}

impl Default for FpsCameraController {
    fn default() -> Self {
        Self::new()
    }
}

impl CameraController for FpsCameraController {
    fn update(&mut self, camera: &mut Camera, dt: f32) {
        // Update rotation from yaw/pitch
        let cy = (self.yaw * 0.5).cos();
        let sy = (self.yaw * 0.5).sin();
        let cp = (self.pitch * 0.5).cos();
        let sp = (self.pitch * 0.5).sin();

        camera.rotation = [cy * sp, sy * cp, -sy * sp, cy * cp];

        // Movement
        let forward = camera.forward();
        let right = camera.right();
        let up = [0.0, 1.0, 0.0];

        let mut velocity = [0.0f32; 3];

        if self.move_forward {
            velocity[0] += forward[0];
            velocity[1] += forward[1];
            velocity[2] += forward[2];
        }
        if self.move_backward {
            velocity[0] -= forward[0];
            velocity[1] -= forward[1];
            velocity[2] -= forward[2];
        }
        if self.move_right {
            velocity[0] += right[0];
            velocity[1] += right[1];
            velocity[2] += right[2];
        }
        if self.move_left {
            velocity[0] -= right[0];
            velocity[1] -= right[1];
            velocity[2] -= right[2];
        }
        if self.move_up {
            velocity[0] += up[0];
            velocity[1] += up[1];
            velocity[2] += up[2];
        }
        if self.move_down {
            velocity[0] -= up[0];
            velocity[1] -= up[1];
            velocity[2] -= up[2];
        }

        let len =
            (velocity[0] * velocity[0] + velocity[1] * velocity[1] + velocity[2] * velocity[2])
                .sqrt();
        if len > 0.0 {
            camera.position[0] += velocity[0] / len * self.move_speed * dt;
            camera.position[1] += velocity[1] / len * self.move_speed * dt;
            camera.position[2] += velocity[2] / len * self.move_speed * dt;
        }
    }

    fn on_input(&mut self, event: &CameraInputEvent) {
        match event {
            CameraInputEvent::MouseMove { delta_x, delta_y } => {
                self.yaw -= delta_x.to_radians() * self.look_sensitivity;
                self.pitch -= delta_y.to_radians() * self.look_sensitivity;
                self.pitch = self
                    .pitch
                    .clamp(-89.0f32.to_radians(), 89.0f32.to_radians());
            },
            CameraInputEvent::KeyDown { key } => {
                match key {
                    87 => self.move_forward = true,  // W
                    83 => self.move_backward = true, // S
                    65 => self.move_left = true,     // A
                    68 => self.move_right = true,    // D
                    32 => self.move_up = true,       // Space
                    16 => self.move_down = true,     // Shift
                    _ => {},
                }
            },
            CameraInputEvent::KeyUp { key } => match key {
                87 => self.move_forward = false,
                83 => self.move_backward = false,
                65 => self.move_left = false,
                68 => self.move_right = false,
                32 => self.move_up = false,
                16 => self.move_down = false,
                _ => {},
            },
            _ => {},
        }
    }
}

/// Orbit camera controller
pub struct OrbitCameraController {
    pub target: [f32; 3],
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    pub rotate_sensitivity: f32,
    pub zoom_sensitivity: f32,
}

impl OrbitCameraController {
    pub fn new(target: [f32; 3], distance: f32) -> Self {
        Self {
            target,
            distance,
            yaw: 0.0,
            pitch: 0.3,
            min_distance: 1.0,
            max_distance: 100.0,
            rotate_sensitivity: 0.01,
            zoom_sensitivity: 0.1,
        }
    }
}

impl CameraController for OrbitCameraController {
    fn update(&mut self, camera: &mut Camera, _dt: f32) {
        let x = self.distance * self.yaw.sin() * self.pitch.cos();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * self.yaw.cos() * self.pitch.cos();

        camera.position = [self.target[0] + x, self.target[1] + y, self.target[2] + z];

        camera.look_at(self.target, [0.0, 1.0, 0.0]);
    }

    fn on_input(&mut self, event: &CameraInputEvent) {
        match event {
            CameraInputEvent::MouseMove { delta_x, delta_y } => {
                self.yaw -= *delta_x * self.rotate_sensitivity;
                self.pitch += *delta_y * self.rotate_sensitivity;
                self.pitch = self.pitch.clamp(0.1, core::f32::consts::FRAC_PI_2 - 0.1);
            },
            CameraInputEvent::MouseScroll { delta } => {
                self.distance *= 1.0 - delta * self.zoom_sensitivity;
                self.distance = self.distance.clamp(self.min_distance, self.max_distance);
            },
            _ => {},
        }
    }
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
