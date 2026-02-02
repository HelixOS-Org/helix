//! Matrix types

use super::vec::{Vec3, Vec4};
use core::ops::{Mul, MulAssign};

/// A 2x2 matrix
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct Mat2 {
    /// Column 0
    pub x_axis: [f32; 2],
    /// Column 1
    pub y_axis: [f32; 2],
}

impl Mat2 {
    /// Identity matrix
    pub const IDENTITY: Self = Self {
        x_axis: [1.0, 0.0],
        y_axis: [0.0, 1.0],
    };

    /// Zero matrix
    pub const ZERO: Self = Self {
        x_axis: [0.0, 0.0],
        y_axis: [0.0, 0.0],
    };
}

impl Default for Mat2 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// A 3x3 matrix
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct Mat3 {
    /// Column 0
    pub x_axis: Vec3,
    /// Column 1
    pub y_axis: Vec3,
    /// Column 2
    pub z_axis: Vec3,
}

impl Mat3 {
    /// Identity matrix
    pub const IDENTITY: Self = Self {
        x_axis: Vec3::X,
        y_axis: Vec3::Y,
        z_axis: Vec3::Z,
    };

    /// Zero matrix
    pub const ZERO: Self = Self {
        x_axis: Vec3::ZERO,
        y_axis: Vec3::ZERO,
        z_axis: Vec3::ZERO,
    };

    /// Creates a 3x3 matrix from column vectors
    #[inline]
    pub const fn from_cols(x_axis: Vec3, y_axis: Vec3, z_axis: Vec3) -> Self {
        Self {
            x_axis,
            y_axis,
            z_axis,
        }
    }
}

impl Default for Mat3 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// A 4x4 matrix (column-major)
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct Mat4 {
    /// Column 0
    pub x_axis: Vec4,
    /// Column 1
    pub y_axis: Vec4,
    /// Column 2
    pub z_axis: Vec4,
    /// Column 3 (translation)
    pub w_axis: Vec4,
}

impl Mat4 {
    /// Identity matrix
    pub const IDENTITY: Self = Self {
        x_axis: Vec4::X,
        y_axis: Vec4::Y,
        z_axis: Vec4::Z,
        w_axis: Vec4::W,
    };

    /// Zero matrix
    pub const ZERO: Self = Self {
        x_axis: Vec4::ZERO,
        y_axis: Vec4::ZERO,
        z_axis: Vec4::ZERO,
        w_axis: Vec4::ZERO,
    };

    /// Creates a matrix from column vectors
    #[inline]
    pub const fn from_cols(x_axis: Vec4, y_axis: Vec4, z_axis: Vec4, w_axis: Vec4) -> Self {
        Self {
            x_axis,
            y_axis,
            z_axis,
            w_axis,
        }
    }

    /// Creates a translation matrix
    #[inline]
    pub fn from_translation(translation: Vec3) -> Self {
        Self {
            x_axis: Vec4::X,
            y_axis: Vec4::Y,
            z_axis: Vec4::Z,
            w_axis: translation.extend(1.0),
        }
    }

    /// Creates a uniform scale matrix
    #[inline]
    pub fn from_scale(scale: Vec3) -> Self {
        Self {
            x_axis: Vec4::new(scale.x, 0.0, 0.0, 0.0),
            y_axis: Vec4::new(0.0, scale.y, 0.0, 0.0),
            z_axis: Vec4::new(0.0, 0.0, scale.z, 0.0),
            w_axis: Vec4::W,
        }
    }

    /// Creates a rotation matrix around the X axis
    #[inline]
    pub fn from_rotation_x(angle: f32) -> Self {
        let (sin, cos) = (angle.sin(), angle.cos());
        Self {
            x_axis: Vec4::X,
            y_axis: Vec4::new(0.0, cos, sin, 0.0),
            z_axis: Vec4::new(0.0, -sin, cos, 0.0),
            w_axis: Vec4::W,
        }
    }

    /// Creates a rotation matrix around the Y axis
    #[inline]
    pub fn from_rotation_y(angle: f32) -> Self {
        let (sin, cos) = (angle.sin(), angle.cos());
        Self {
            x_axis: Vec4::new(cos, 0.0, -sin, 0.0),
            y_axis: Vec4::Y,
            z_axis: Vec4::new(sin, 0.0, cos, 0.0),
            w_axis: Vec4::W,
        }
    }

    /// Creates a rotation matrix around the Z axis
    #[inline]
    pub fn from_rotation_z(angle: f32) -> Self {
        let (sin, cos) = (angle.sin(), angle.cos());
        Self {
            x_axis: Vec4::new(cos, sin, 0.0, 0.0),
            y_axis: Vec4::new(-sin, cos, 0.0, 0.0),
            z_axis: Vec4::Z,
            w_axis: Vec4::W,
        }
    }

    /// Creates a look-at view matrix
    #[inline]
    pub fn look_at(eye: Vec3, center: Vec3, up: Vec3) -> Self {
        let f = (center - eye).normalize();
        let s = f.cross(up).normalize();
        let u = s.cross(f);

        Self {
            x_axis: Vec4::new(s.x, u.x, -f.x, 0.0),
            y_axis: Vec4::new(s.y, u.y, -f.y, 0.0),
            z_axis: Vec4::new(s.z, u.z, -f.z, 0.0),
            w_axis: Vec4::new(-s.dot(eye), -u.dot(eye), f.dot(eye), 1.0),
        }
    }

    /// Creates a perspective projection matrix
    #[inline]
    pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Self {
        let tan_half_fov = (fov_y / 2.0).tan();
        let range = far - near;

        Self {
            x_axis: Vec4::new(1.0 / (aspect * tan_half_fov), 0.0, 0.0, 0.0),
            y_axis: Vec4::new(0.0, 1.0 / tan_half_fov, 0.0, 0.0),
            z_axis: Vec4::new(0.0, 0.0, -(far + near) / range, -1.0),
            w_axis: Vec4::new(0.0, 0.0, -(2.0 * far * near) / range, 0.0),
        }
    }

    /// Creates an orthographic projection matrix
    #[inline]
    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        let rml = right - left;
        let tmb = top - bottom;
        let fmn = far - near;

        Self {
            x_axis: Vec4::new(2.0 / rml, 0.0, 0.0, 0.0),
            y_axis: Vec4::new(0.0, 2.0 / tmb, 0.0, 0.0),
            z_axis: Vec4::new(0.0, 0.0, -2.0 / fmn, 0.0),
            w_axis: Vec4::new(
                -(right + left) / rml,
                -(top + bottom) / tmb,
                -(far + near) / fmn,
                1.0,
            ),
        }
    }

    /// Transposes the matrix
    #[inline]
    pub fn transpose(self) -> Self {
        Self {
            x_axis: Vec4::new(self.x_axis.x, self.y_axis.x, self.z_axis.x, self.w_axis.x),
            y_axis: Vec4::new(self.x_axis.y, self.y_axis.y, self.z_axis.y, self.w_axis.y),
            z_axis: Vec4::new(self.x_axis.z, self.y_axis.z, self.z_axis.z, self.w_axis.z),
            w_axis: Vec4::new(self.x_axis.w, self.y_axis.w, self.z_axis.w, self.w_axis.w),
        }
    }

    /// Computes the determinant
    #[inline]
    pub fn determinant(self) -> f32 {
        let a = self.x_axis.x;
        let b = self.y_axis.x;
        let c = self.z_axis.x;
        let d = self.w_axis.x;
        let e = self.x_axis.y;
        let f = self.y_axis.y;
        let g = self.z_axis.y;
        let h = self.w_axis.y;
        let i = self.x_axis.z;
        let j = self.y_axis.z;
        let k = self.z_axis.z;
        let l = self.w_axis.z;
        let m = self.x_axis.w;
        let n = self.y_axis.w;
        let o = self.z_axis.w;
        let p = self.w_axis.w;

        let kp_lo = k * p - l * o;
        let jp_ln = j * p - l * n;
        let jo_kn = j * o - k * n;
        let ip_lm = i * p - l * m;
        let io_km = i * o - k * m;
        let in_jm = i * n - j * m;

        a * (f * kp_lo - g * jp_ln + h * jo_kn)
            - b * (e * kp_lo - g * ip_lm + h * io_km)
            + c * (e * jp_ln - f * ip_lm + h * in_jm)
            - d * (e * jo_kn - f * io_km + g * in_jm)
    }

    /// Computes the inverse (returns None if singular)
    pub fn inverse(self) -> Option<Self> {
        let det = self.determinant();
        if det.abs() < 1e-10 {
            return None;
        }

        let inv_det = 1.0 / det;

        // Compute adjugate matrix and multiply by 1/det
        // This is the standard 4x4 matrix inverse computation
        Some(self.adjugate() * inv_det)
    }

    /// Computes the adjugate matrix
    fn adjugate(self) -> Self {
        let a = self.x_axis.x;
        let b = self.y_axis.x;
        let c = self.z_axis.x;
        let d = self.w_axis.x;
        let e = self.x_axis.y;
        let f = self.y_axis.y;
        let g = self.z_axis.y;
        let h = self.w_axis.y;
        let i = self.x_axis.z;
        let j = self.y_axis.z;
        let k = self.z_axis.z;
        let l = self.w_axis.z;
        let m = self.x_axis.w;
        let n = self.y_axis.w;
        let o = self.z_axis.w;
        let p = self.w_axis.w;

        Self {
            x_axis: Vec4::new(
                f * (k * p - l * o) - g * (j * p - l * n) + h * (j * o - k * n),
                -(e * (k * p - l * o) - g * (i * p - l * m) + h * (i * o - k * m)),
                e * (j * p - l * n) - f * (i * p - l * m) + h * (i * n - j * m),
                -(e * (j * o - k * n) - f * (i * o - k * m) + g * (i * n - j * m)),
            ),
            y_axis: Vec4::new(
                -(b * (k * p - l * o) - c * (j * p - l * n) + d * (j * o - k * n)),
                a * (k * p - l * o) - c * (i * p - l * m) + d * (i * o - k * m),
                -(a * (j * p - l * n) - b * (i * p - l * m) + d * (i * n - j * m)),
                a * (j * o - k * n) - b * (i * o - k * m) + c * (i * n - j * m),
            ),
            z_axis: Vec4::new(
                b * (g * p - h * o) - c * (f * p - h * n) + d * (f * o - g * n),
                -(a * (g * p - h * o) - c * (e * p - h * m) + d * (e * o - g * m)),
                a * (f * p - h * n) - b * (e * p - h * m) + d * (e * n - f * m),
                -(a * (f * o - g * n) - b * (e * o - g * m) + c * (e * n - f * m)),
            ),
            w_axis: Vec4::new(
                -(b * (g * l - h * k) - c * (f * l - h * j) + d * (f * k - g * j)),
                a * (g * l - h * k) - c * (e * l - h * i) + d * (e * k - g * i),
                -(a * (f * l - h * j) - b * (e * l - h * i) + d * (e * j - f * i)),
                a * (f * k - g * j) - b * (e * k - g * i) + c * (e * j - f * i),
            ),
        }
    }
}

impl Default for Mat4 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Mul<Mat4> for Mat4 {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Mat4) -> Self {
        Self {
            x_axis: self * rhs.x_axis,
            y_axis: self * rhs.y_axis,
            z_axis: self * rhs.z_axis,
            w_axis: self * rhs.w_axis,
        }
    }
}

impl MulAssign<Mat4> for Mat4 {
    #[inline]
    fn mul_assign(&mut self, rhs: Mat4) {
        *self = *self * rhs;
    }
}

impl Mul<Vec4> for Mat4 {
    type Output = Vec4;

    #[inline]
    fn mul(self, rhs: Vec4) -> Vec4 {
        Vec4::new(
            self.x_axis.x * rhs.x
                + self.y_axis.x * rhs.y
                + self.z_axis.x * rhs.z
                + self.w_axis.x * rhs.w,
            self.x_axis.y * rhs.x
                + self.y_axis.y * rhs.y
                + self.z_axis.y * rhs.z
                + self.w_axis.y * rhs.w,
            self.x_axis.z * rhs.x
                + self.y_axis.z * rhs.y
                + self.z_axis.z * rhs.z
                + self.w_axis.z * rhs.w,
            self.x_axis.w * rhs.x
                + self.y_axis.w * rhs.y
                + self.z_axis.w * rhs.z
                + self.w_axis.w * rhs.w,
        )
    }
}

impl Mul<Vec3> for Mat4 {
    type Output = Vec3;

    /// Transforms a point (w=1)
    #[inline]
    fn mul(self, rhs: Vec3) -> Vec3 {
        let result = self * rhs.extend(1.0);
        Vec3::new(result.x, result.y, result.z)
    }
}

impl Mul<f32> for Mat4 {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: f32) -> Self {
        Self {
            x_axis: self.x_axis * rhs,
            y_axis: self.y_axis * rhs,
            z_axis: self.z_axis * rhs,
            w_axis: self.w_axis * rhs,
        }
    }
}
