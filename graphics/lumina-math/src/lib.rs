//! # LUMINA Math
//!
//! Mathematics library for LUMINA graphics API.
//!
//! Provides optimized math types and operations for graphics programming:
//!
//! ## Types
//!
//! - Vectors: `Vec2`, `Vec3`, `Vec4`
//! - Matrices: `Mat2`, `Mat3`, `Mat4`
//! - Quaternions: `Quat`
//! - Colors: `Color`, `LinearColor`
//! - Geometric primitives: `AABB`, `Sphere`, `Plane`, `Ray`, `Frustum`
//!
//! ## Features
//!
//! - SIMD-optimized operations (when available)
//! - no_std compatible
//! - Interoperability with lumina-core

#![no_std]
#![cfg_attr(feature = "alloc", feature(alloc))]
#![allow(unused)]

use core::ops::{Add, Div, Mul, Neg, Sub};

// ============================================================================
// Vector Types
// ============================================================================

/// 2D vector
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

/// 3D vector
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// 4D vector
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

/// 2D integer vector
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IVec2 {
    pub x: i32,
    pub y: i32,
}

/// 3D integer vector
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IVec3 {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// 4D integer vector
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IVec4 {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub w: i32,
}

/// 2D unsigned integer vector
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UVec2 {
    pub x: u32,
    pub y: u32,
}

/// 3D unsigned integer vector
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UVec3 {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

/// 4D unsigned integer vector
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UVec4 {
    pub x: u32,
    pub y: u32,
    pub z: u32,
    pub w: u32,
}

// ============================================================================
// Vec2 Implementation
// ============================================================================

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    pub const ONE: Self = Self { x: 1.0, y: 1.0 };
    pub const X: Self = Self { x: 1.0, y: 0.0 };
    pub const Y: Self = Self { x: 0.0, y: 1.0 };

    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline]
    pub const fn splat(v: f32) -> Self {
        Self { x: v, y: v }
    }

    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    #[inline]
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    #[inline]
    pub fn normalize(self) -> Self {
        let len = self.length();
        if len > 0.0 {
            self / len
        } else {
            Self::ZERO
        }
    }

    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }
}

impl Add for Vec2 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl Sub for Vec2 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f32) -> Self {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl Div<f32> for Vec2 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: f32) -> Self {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl Neg for Vec2 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y)
    }
}

// ============================================================================
// Vec3 Implementation
// ============================================================================

impl Vec3 {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    pub const ONE: Self = Self {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    };
    pub const X: Self = Self {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };
    pub const Y: Self = Self {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
    pub const Z: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    };
    pub const UP: Self = Self::Y;
    pub const DOWN: Self = Self {
        x: 0.0,
        y: -1.0,
        z: 0.0,
    };
    pub const FORWARD: Self = Self {
        x: 0.0,
        y: 0.0,
        z: -1.0,
    };
    pub const BACK: Self = Self::Z;

    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    #[inline]
    pub const fn splat(v: f32) -> Self {
        Self { x: v, y: v, z: v }
    }

    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    #[inline]
    pub fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    #[inline]
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    #[inline]
    pub fn normalize(self) -> Self {
        let len = self.length();
        if len > 0.0 {
            self / len
        } else {
            Self::ZERO
        }
    }

    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }

    #[inline]
    pub fn reflect(self, normal: Self) -> Self {
        self - normal * 2.0 * self.dot(normal)
    }

    #[inline]
    pub fn extend(self, w: f32) -> Vec4 {
        Vec4::new(self.x, self.y, self.z, w)
    }

    #[inline]
    pub fn truncate(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }
}

impl Add for Vec3 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub for Vec3 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f32) -> Self {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl Div<f32> for Vec3 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: f32) -> Self {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

impl Neg for Vec3 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}

// ============================================================================
// Vec4 Implementation
// ============================================================================

impl Vec4 {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 0.0,
    };
    pub const ONE: Self = Self {
        x: 1.0,
        y: 1.0,
        z: 1.0,
        w: 1.0,
    };
    pub const X: Self = Self {
        x: 1.0,
        y: 0.0,
        z: 0.0,
        w: 0.0,
    };
    pub const Y: Self = Self {
        x: 0.0,
        y: 1.0,
        z: 0.0,
        w: 0.0,
    };
    pub const Z: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 1.0,
        w: 0.0,
    };
    pub const W: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    };

    #[inline]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    #[inline]
    pub const fn splat(v: f32) -> Self {
        Self {
            x: v,
            y: v,
            z: v,
            w: v,
        }
    }

    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }

    #[inline]
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    #[inline]
    pub fn normalize(self) -> Self {
        let len = self.length();
        if len > 0.0 {
            self / len
        } else {
            Self::ZERO
        }
    }

    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }

    #[inline]
    pub fn truncate(self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }

    #[inline]
    pub fn xyz(self) -> Vec3 {
        self.truncate()
    }

    #[inline]
    pub fn xy(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }
}

impl Add for Vec4 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self::new(
            self.x + rhs.x,
            self.y + rhs.y,
            self.z + rhs.z,
            self.w + rhs.w,
        )
    }
}

impl Sub for Vec4 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self::new(
            self.x - rhs.x,
            self.y - rhs.y,
            self.z - rhs.z,
            self.w - rhs.w,
        )
    }
}

impl Mul<f32> for Vec4 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f32) -> Self {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs, self.w * rhs)
    }
}

impl Div<f32> for Vec4 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: f32) -> Self {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs, self.w / rhs)
    }
}

impl Neg for Vec4 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z, -self.w)
    }
}

// ============================================================================
// Matrix Types
// ============================================================================

/// 2x2 matrix (column-major)
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat2 {
    pub cols: [Vec2; 2],
}

/// 3x3 matrix (column-major)
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat3 {
    pub cols: [Vec3; 3],
}

/// 4x4 matrix (column-major)
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat4 {
    pub cols: [Vec4; 4],
}

impl Mat4 {
    pub const IDENTITY: Self = Self {
        cols: [Vec4::X, Vec4::Y, Vec4::Z, Vec4::W],
    };

    pub const ZERO: Self = Self {
        cols: [Vec4::ZERO, Vec4::ZERO, Vec4::ZERO, Vec4::ZERO],
    };

    #[inline]
    pub const fn from_cols(c0: Vec4, c1: Vec4, c2: Vec4, c3: Vec4) -> Self {
        Self {
            cols: [c0, c1, c2, c3],
        }
    }

    #[inline]
    pub fn from_scale(scale: Vec3) -> Self {
        Self::from_cols(
            Vec4::new(scale.x, 0.0, 0.0, 0.0),
            Vec4::new(0.0, scale.y, 0.0, 0.0),
            Vec4::new(0.0, 0.0, scale.z, 0.0),
            Vec4::W,
        )
    }

    #[inline]
    pub fn from_translation(translation: Vec3) -> Self {
        Self::from_cols(Vec4::X, Vec4::Y, Vec4::Z, translation.extend(1.0))
    }

    #[inline]
    pub fn from_rotation_x(angle: f32) -> Self {
        let (sin, cos) = (angle.sin(), angle.cos());
        Self::from_cols(
            Vec4::X,
            Vec4::new(0.0, cos, sin, 0.0),
            Vec4::new(0.0, -sin, cos, 0.0),
            Vec4::W,
        )
    }

    #[inline]
    pub fn from_rotation_y(angle: f32) -> Self {
        let (sin, cos) = (angle.sin(), angle.cos());
        Self::from_cols(
            Vec4::new(cos, 0.0, -sin, 0.0),
            Vec4::Y,
            Vec4::new(sin, 0.0, cos, 0.0),
            Vec4::W,
        )
    }

    #[inline]
    pub fn from_rotation_z(angle: f32) -> Self {
        let (sin, cos) = (angle.sin(), angle.cos());
        Self::from_cols(
            Vec4::new(cos, sin, 0.0, 0.0),
            Vec4::new(-sin, cos, 0.0, 0.0),
            Vec4::Z,
            Vec4::W,
        )
    }

    /// Creates a perspective projection matrix
    #[inline]
    pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Self {
        let f = 1.0 / (fov_y * 0.5).tan();
        let range_inv = 1.0 / (near - far);

        Self::from_cols(
            Vec4::new(f / aspect, 0.0, 0.0, 0.0),
            Vec4::new(0.0, f, 0.0, 0.0),
            Vec4::new(0.0, 0.0, (near + far) * range_inv, -1.0),
            Vec4::new(0.0, 0.0, 2.0 * near * far * range_inv, 0.0),
        )
    }

    /// Creates an orthographic projection matrix
    #[inline]
    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        let rml = right - left;
        let tmb = top - bottom;
        let fmn = far - near;

        Self::from_cols(
            Vec4::new(2.0 / rml, 0.0, 0.0, 0.0),
            Vec4::new(0.0, 2.0 / tmb, 0.0, 0.0),
            Vec4::new(0.0, 0.0, -2.0 / fmn, 0.0),
            Vec4::new(
                -(right + left) / rml,
                -(top + bottom) / tmb,
                -(far + near) / fmn,
                1.0,
            ),
        )
    }

    /// Creates a look-at view matrix
    #[inline]
    pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let f = (target - eye).normalize();
        let s = f.cross(up).normalize();
        let u = s.cross(f);

        Self::from_cols(
            Vec4::new(s.x, u.x, -f.x, 0.0),
            Vec4::new(s.y, u.y, -f.y, 0.0),
            Vec4::new(s.z, u.z, -f.z, 0.0),
            Vec4::new(-s.dot(eye), -u.dot(eye), f.dot(eye), 1.0),
        )
    }

    /// Matrix multiplication
    #[inline]
    pub fn mul_mat4(self, rhs: Self) -> Self {
        let mut result = Self::ZERO;
        for i in 0..4 {
            for j in 0..4 {
                let row = Vec4::new(
                    self.cols[0].x * rhs.cols[i].x
                        + self.cols[1].x * rhs.cols[i].y
                        + self.cols[2].x * rhs.cols[i].z
                        + self.cols[3].x * rhs.cols[i].w,
                    self.cols[0].y * rhs.cols[i].x
                        + self.cols[1].y * rhs.cols[i].y
                        + self.cols[2].y * rhs.cols[i].z
                        + self.cols[3].y * rhs.cols[i].w,
                    self.cols[0].z * rhs.cols[i].x
                        + self.cols[1].z * rhs.cols[i].y
                        + self.cols[2].z * rhs.cols[i].z
                        + self.cols[3].z * rhs.cols[i].w,
                    self.cols[0].w * rhs.cols[i].x
                        + self.cols[1].w * rhs.cols[i].y
                        + self.cols[2].w * rhs.cols[i].z
                        + self.cols[3].w * rhs.cols[i].w,
                );
                result.cols[i] = row;
            }
        }
        result
    }

    /// Transform a Vec4
    #[inline]
    pub fn mul_vec4(self, v: Vec4) -> Vec4 {
        Vec4::new(
            self.cols[0].x * v.x
                + self.cols[1].x * v.y
                + self.cols[2].x * v.z
                + self.cols[3].x * v.w,
            self.cols[0].y * v.x
                + self.cols[1].y * v.y
                + self.cols[2].y * v.z
                + self.cols[3].y * v.w,
            self.cols[0].z * v.x
                + self.cols[1].z * v.y
                + self.cols[2].z * v.z
                + self.cols[3].z * v.w,
            self.cols[0].w * v.x
                + self.cols[1].w * v.y
                + self.cols[2].w * v.z
                + self.cols[3].w * v.w,
        )
    }

    /// Transform a point (w=1)
    #[inline]
    pub fn transform_point(self, p: Vec3) -> Vec3 {
        self.mul_vec4(p.extend(1.0)).truncate()
    }

    /// Transform a vector (w=0)
    #[inline]
    pub fn transform_vector(self, v: Vec3) -> Vec3 {
        self.mul_vec4(v.extend(0.0)).truncate()
    }

    /// Transpose
    #[inline]
    pub fn transpose(self) -> Self {
        Self::from_cols(
            Vec4::new(
                self.cols[0].x,
                self.cols[1].x,
                self.cols[2].x,
                self.cols[3].x,
            ),
            Vec4::new(
                self.cols[0].y,
                self.cols[1].y,
                self.cols[2].y,
                self.cols[3].y,
            ),
            Vec4::new(
                self.cols[0].z,
                self.cols[1].z,
                self.cols[2].z,
                self.cols[3].z,
            ),
            Vec4::new(
                self.cols[0].w,
                self.cols[1].w,
                self.cols[2].w,
                self.cols[3].w,
            ),
        )
    }
}

impl Mul for Mat4 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        self.mul_mat4(rhs)
    }
}

impl Mul<Vec4> for Mat4 {
    type Output = Vec4;
    #[inline]
    fn mul(self, rhs: Vec4) -> Vec4 {
        self.mul_vec4(rhs)
    }
}

// ============================================================================
// Quaternion
// ============================================================================

/// Quaternion for rotations
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quat {
    pub const IDENTITY: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    };

    #[inline]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    #[inline]
    pub fn from_axis_angle(axis: Vec3, angle: f32) -> Self {
        let half = angle * 0.5;
        let s = half.sin();
        let c = half.cos();
        Self::new(axis.x * s, axis.y * s, axis.z * s, c)
    }

    #[inline]
    pub fn from_euler(pitch: f32, yaw: f32, roll: f32) -> Self {
        let (sp, cp) = (pitch * 0.5).sin_cos();
        let (sy, cy) = (yaw * 0.5).sin_cos();
        let (sr, cr) = (roll * 0.5).sin_cos();

        Self::new(
            sp * cy * cr + cp * sy * sr,
            cp * sy * cr - sp * cy * sr,
            cp * cy * sr + sp * sy * cr,
            cp * cy * cr - sp * sy * sr,
        )
    }

    #[inline]
    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w
    }

    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    #[inline]
    pub fn normalize(self) -> Self {
        let len = self.length();
        if len > 0.0 {
            Self::new(self.x / len, self.y / len, self.z / len, self.w / len)
        } else {
            Self::IDENTITY
        }
    }

    #[inline]
    pub fn conjugate(self) -> Self {
        Self::new(-self.x, -self.y, -self.z, self.w)
    }

    #[inline]
    pub fn inverse(self) -> Self {
        let len_sq = self.length_squared();
        if len_sq > 0.0 {
            let conj = self.conjugate();
            Self::new(
                conj.x / len_sq,
                conj.y / len_sq,
                conj.z / len_sq,
                conj.w / len_sq,
            )
        } else {
            Self::IDENTITY
        }
    }

    #[inline]
    pub fn mul_quat(self, rhs: Self) -> Self {
        Self::new(
            self.w * rhs.x + self.x * rhs.w + self.y * rhs.z - self.z * rhs.y,
            self.w * rhs.y - self.x * rhs.z + self.y * rhs.w + self.z * rhs.x,
            self.w * rhs.z + self.x * rhs.y - self.y * rhs.x + self.z * rhs.w,
            self.w * rhs.w - self.x * rhs.x - self.y * rhs.y - self.z * rhs.z,
        )
    }

    #[inline]
    pub fn rotate_vec3(self, v: Vec3) -> Vec3 {
        let qv = Vec3::new(self.x, self.y, self.z);
        let uv = qv.cross(v);
        let uuv = qv.cross(uv);
        v + (uv * self.w + uuv) * 2.0
    }

    #[inline]
    pub fn to_mat4(self) -> Mat4 {
        let x2 = self.x + self.x;
        let y2 = self.y + self.y;
        let z2 = self.z + self.z;

        let xx = self.x * x2;
        let xy = self.x * y2;
        let xz = self.x * z2;
        let yy = self.y * y2;
        let yz = self.y * z2;
        let zz = self.z * z2;
        let wx = self.w * x2;
        let wy = self.w * y2;
        let wz = self.w * z2;

        Mat4::from_cols(
            Vec4::new(1.0 - yy - zz, xy + wz, xz - wy, 0.0),
            Vec4::new(xy - wz, 1.0 - xx - zz, yz + wx, 0.0),
            Vec4::new(xz + wy, yz - wx, 1.0 - xx - yy, 0.0),
            Vec4::W,
        )
    }

    /// Spherical linear interpolation
    #[inline]
    pub fn slerp(self, other: Self, t: f32) -> Self {
        let mut dot = self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w;

        let other = if dot < 0.0 {
            dot = -dot;
            Self::new(-other.x, -other.y, -other.z, -other.w)
        } else {
            other
        };

        if dot > 0.9995 {
            // Linear interpolation for nearly identical quaternions
            Self::new(
                self.x + t * (other.x - self.x),
                self.y + t * (other.y - self.y),
                self.z + t * (other.z - self.z),
                self.w + t * (other.w - self.w),
            )
            .normalize()
        } else {
            let theta = dot.acos();
            let sin_theta = theta.sin();
            let s0 = ((1.0 - t) * theta).sin() / sin_theta;
            let s1 = (t * theta).sin() / sin_theta;

            Self::new(
                s0 * self.x + s1 * other.x,
                s0 * self.y + s1 * other.y,
                s0 * self.z + s1 * other.z,
                s0 * self.w + s1 * other.w,
            )
        }
    }
}

impl Mul for Quat {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        self.mul_quat(rhs)
    }
}

impl Mul<Vec3> for Quat {
    type Output = Vec3;
    #[inline]
    fn mul(self, rhs: Vec3) -> Vec3 {
        self.rotate_vec3(rhs)
    }
}

// ============================================================================
// Geometric Primitives
// ============================================================================

/// Axis-Aligned Bounding Box
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AABB {
    pub min: Vec3,
    pub max: Vec3,
}

impl AABB {
    pub const EMPTY: Self = Self {
        min: Vec3 {
            x: f32::MAX,
            y: f32::MAX,
            z: f32::MAX,
        },
        max: Vec3 {
            x: f32::MIN,
            y: f32::MIN,
            z: f32::MIN,
        },
    };

    #[inline]
    pub const fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    #[inline]
    pub fn from_center_extents(center: Vec3, extents: Vec3) -> Self {
        Self::new(center - extents, center + extents)
    }

    #[inline]
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    #[inline]
    pub fn extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }

    #[inline]
    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }

    #[inline]
    pub fn contains(&self, point: Vec3) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }

    #[inline]
    pub fn intersects(&self, other: &Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    #[inline]
    pub fn expand(&mut self, point: Vec3) {
        self.min.x = self.min.x.min(point.x);
        self.min.y = self.min.y.min(point.y);
        self.min.z = self.min.z.min(point.z);
        self.max.x = self.max.x.max(point.x);
        self.max.y = self.max.y.max(point.y);
        self.max.z = self.max.z.max(point.z);
    }

    #[inline]
    pub fn merge(&self, other: &Self) -> Self {
        Self::new(
            Vec3::new(
                self.min.x.min(other.min.x),
                self.min.y.min(other.min.y),
                self.min.z.min(other.min.z),
            ),
            Vec3::new(
                self.max.x.max(other.max.x),
                self.max.y.max(other.max.y),
                self.max.z.max(other.max.z),
            ),
        )
    }
}

/// Bounding Sphere
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
}

impl Sphere {
    #[inline]
    pub const fn new(center: Vec3, radius: f32) -> Self {
        Self { center, radius }
    }

    #[inline]
    pub fn contains(&self, point: Vec3) -> bool {
        (point - self.center).length_squared() <= self.radius * self.radius
    }

    #[inline]
    pub fn intersects(&self, other: &Self) -> bool {
        let dist_sq = (self.center - other.center).length_squared();
        let radius_sum = self.radius + other.radius;
        dist_sq <= radius_sum * radius_sum
    }
}

/// 3D Plane (ax + by + cz + d = 0)
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plane {
    pub normal: Vec3,
    pub distance: f32,
}

impl Plane {
    #[inline]
    pub const fn new(normal: Vec3, distance: f32) -> Self {
        Self { normal, distance }
    }

    #[inline]
    pub fn from_point_normal(point: Vec3, normal: Vec3) -> Self {
        let n = normal.normalize();
        Self::new(n, -n.dot(point))
    }

    #[inline]
    pub fn signed_distance(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.distance
    }

    #[inline]
    pub fn normalize(&self) -> Self {
        let len = self.normal.length();
        if len > 0.0 {
            Self::new(self.normal / len, self.distance / len)
        } else {
            *self
        }
    }
}

/// Ray for raycasting
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    #[inline]
    pub const fn new(origin: Vec3, direction: Vec3) -> Self {
        Self { origin, direction }
    }

    #[inline]
    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }

    /// Returns t if ray intersects AABB, None otherwise
    pub fn intersect_aabb(&self, aabb: &AABB) -> Option<f32> {
        let inv_dir = Vec3::new(
            1.0 / self.direction.x,
            1.0 / self.direction.y,
            1.0 / self.direction.z,
        );

        let t1 = (aabb.min.x - self.origin.x) * inv_dir.x;
        let t2 = (aabb.max.x - self.origin.x) * inv_dir.x;
        let t3 = (aabb.min.y - self.origin.y) * inv_dir.y;
        let t4 = (aabb.max.y - self.origin.y) * inv_dir.y;
        let t5 = (aabb.min.z - self.origin.z) * inv_dir.z;
        let t6 = (aabb.max.z - self.origin.z) * inv_dir.z;

        let tmin = t1.min(t2).max(t3.min(t4)).max(t5.min(t6));
        let tmax = t1.max(t2).min(t3.max(t4)).min(t5.max(t6));

        if tmax < 0.0 || tmin > tmax {
            None
        } else {
            Some(if tmin < 0.0 { tmax } else { tmin })
        }
    }

    /// Returns t if ray intersects sphere, None otherwise
    pub fn intersect_sphere(&self, sphere: &Sphere) -> Option<f32> {
        let oc = self.origin - sphere.center;
        let a = self.direction.dot(self.direction);
        let b = 2.0 * oc.dot(self.direction);
        let c = oc.dot(oc) - sphere.radius * sphere.radius;
        let discriminant = b * b - 4.0 * a * c;

        if discriminant < 0.0 {
            None
        } else {
            let t = (-b - discriminant.sqrt()) / (2.0 * a);
            if t > 0.0 {
                Some(t)
            } else {
                let t = (-b + discriminant.sqrt()) / (2.0 * a);
                if t > 0.0 {
                    Some(t)
                } else {
                    None
                }
            }
        }
    }
}

/// View frustum (6 planes)
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Frustum {
    pub planes: [Plane; 6],
}

impl Frustum {
    /// Indices for frustum planes
    pub const LEFT: usize = 0;
    pub const RIGHT: usize = 1;
    pub const BOTTOM: usize = 2;
    pub const TOP: usize = 3;
    pub const NEAR: usize = 4;
    pub const FAR: usize = 5;

    /// Extract frustum planes from view-projection matrix
    pub fn from_view_projection(vp: Mat4) -> Self {
        let mut planes = [Plane::new(Vec3::ZERO, 0.0); 6];

        // Left plane
        planes[Self::LEFT] = Plane::new(
            Vec3::new(
                vp.cols[0].w + vp.cols[0].x,
                vp.cols[1].w + vp.cols[1].x,
                vp.cols[2].w + vp.cols[2].x,
            ),
            vp.cols[3].w + vp.cols[3].x,
        )
        .normalize();

        // Right plane
        planes[Self::RIGHT] = Plane::new(
            Vec3::new(
                vp.cols[0].w - vp.cols[0].x,
                vp.cols[1].w - vp.cols[1].x,
                vp.cols[2].w - vp.cols[2].x,
            ),
            vp.cols[3].w - vp.cols[3].x,
        )
        .normalize();

        // Bottom plane
        planes[Self::BOTTOM] = Plane::new(
            Vec3::new(
                vp.cols[0].w + vp.cols[0].y,
                vp.cols[1].w + vp.cols[1].y,
                vp.cols[2].w + vp.cols[2].y,
            ),
            vp.cols[3].w + vp.cols[3].y,
        )
        .normalize();

        // Top plane
        planes[Self::TOP] = Plane::new(
            Vec3::new(
                vp.cols[0].w - vp.cols[0].y,
                vp.cols[1].w - vp.cols[1].y,
                vp.cols[2].w - vp.cols[2].y,
            ),
            vp.cols[3].w - vp.cols[3].y,
        )
        .normalize();

        // Near plane
        planes[Self::NEAR] = Plane::new(
            Vec3::new(
                vp.cols[0].w + vp.cols[0].z,
                vp.cols[1].w + vp.cols[1].z,
                vp.cols[2].w + vp.cols[2].z,
            ),
            vp.cols[3].w + vp.cols[3].z,
        )
        .normalize();

        // Far plane
        planes[Self::FAR] = Plane::new(
            Vec3::new(
                vp.cols[0].w - vp.cols[0].z,
                vp.cols[1].w - vp.cols[1].z,
                vp.cols[2].w - vp.cols[2].z,
            ),
            vp.cols[3].w - vp.cols[3].z,
        )
        .normalize();

        Self { planes }
    }

    /// Test if AABB is inside or intersects frustum
    pub fn contains_aabb(&self, aabb: &AABB) -> bool {
        for plane in &self.planes {
            let p = Vec3::new(
                if plane.normal.x >= 0.0 {
                    aabb.max.x
                } else {
                    aabb.min.x
                },
                if plane.normal.y >= 0.0 {
                    aabb.max.y
                } else {
                    aabb.min.y
                },
                if plane.normal.z >= 0.0 {
                    aabb.max.z
                } else {
                    aabb.min.z
                },
            );

            if plane.signed_distance(p) < 0.0 {
                return false;
            }
        }
        true
    }

    /// Test if sphere is inside or intersects frustum
    pub fn contains_sphere(&self, sphere: &Sphere) -> bool {
        for plane in &self.planes {
            if plane.signed_distance(sphere.center) < -sphere.radius {
                return false;
            }
        }
        true
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Linearly interpolate between two values
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Clamp value between min and max
#[inline]
pub fn clamp(value: f32, min: f32, max: f32) -> f32 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

/// Saturate (clamp to 0..1)
#[inline]
pub fn saturate(value: f32) -> f32 {
    clamp(value, 0.0, 1.0)
}

/// Smoothstep interpolation
#[inline]
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = saturate((x - edge0) / (edge1 - edge0));
    t * t * (3.0 - 2.0 * t)
}

/// Convert degrees to radians
#[inline]
pub fn radians(degrees: f32) -> f32 {
    degrees * core::f32::consts::PI / 180.0
}

/// Convert radians to degrees
#[inline]
pub fn degrees(radians: f32) -> f32 {
    radians * 180.0 / core::f32::consts::PI
}

// ============================================================================
// Color Types
// ============================================================================

/// Linear color (0.0 - 1.0+ HDR)
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinearColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl LinearColor {
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const RED: Self = Self {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const GREEN: Self = Self {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    pub const BLUE: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };

    #[inline]
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    #[inline]
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    #[inline]
    pub fn to_vec4(self) -> Vec4 {
        Vec4::new(self.r, self.g, self.b, self.a)
    }

    #[inline]
    pub fn from_vec4(v: Vec4) -> Self {
        Self::new(v.x, v.y, v.z, v.w)
    }

    /// Convert sRGB to linear
    pub fn from_srgb(srgb: [u8; 4]) -> Self {
        fn to_linear(v: u8) -> f32 {
            let v = v as f32 / 255.0;
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        }

        Self::new(
            to_linear(srgb[0]),
            to_linear(srgb[1]),
            to_linear(srgb[2]),
            srgb[3] as f32 / 255.0,
        )
    }

    /// Convert to sRGB
    pub fn to_srgb(self) -> [u8; 4] {
        fn to_srgb(v: f32) -> u8 {
            let v = if v <= 0.0031308 {
                v * 12.92
            } else {
                1.055 * v.powf(1.0 / 2.4) - 0.055
            };
            (v.clamp(0.0, 1.0) * 255.0) as u8
        }

        [
            to_srgb(self.r),
            to_srgb(self.g),
            to_srgb(self.b),
            (self.a.clamp(0.0, 1.0) * 255.0) as u8,
        ]
    }
}

impl Add for LinearColor {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self::new(
            self.r + rhs.r,
            self.g + rhs.g,
            self.b + rhs.b,
            self.a + rhs.a,
        )
    }
}

impl Mul<f32> for LinearColor {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f32) -> Self {
        Self::new(self.r * rhs, self.g * rhs, self.b * rhs, self.a * rhs)
    }
}

impl Mul for LinearColor {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self::new(
            self.r * rhs.r,
            self.g * rhs.g,
            self.b * rhs.b,
            self.a * rhs.a,
        )
    }
}

// ============================================================================
// Version
// ============================================================================

/// LUMINA Math version
pub const LUMINA_MATH_VERSION: (u32, u32, u32) = (1, 0, 0);
