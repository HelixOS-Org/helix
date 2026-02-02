//! Quaternion implementation for rotations
//!
//! Quaternions provide an efficient and gimbal-lock-free representation
//! for 3D rotations.

use core::ops::{Add, Mul, Neg, Sub};

use crate::mat::{Mat3, Mat4};
use crate::vec::Vec3;

/// A quaternion representing a rotation in 3D space.
///
/// Stored as (x, y, z, w) where (x, y, z) is the vector part
/// and w is the scalar part.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct Quat {
    /// X component (vector part)
    pub x: f32,
    /// Y component (vector part)
    pub y: f32,
    /// Z component (vector part)
    pub z: f32,
    /// W component (scalar part)
    pub w: f32,
}

impl Quat {
    /// Identity quaternion (no rotation)
    pub const IDENTITY: Self = Self::new(0.0, 0.0, 0.0, 1.0);

    /// Creates a new quaternion from components
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    /// Creates a quaternion from an axis and angle (in radians)
    #[inline]
    pub fn from_axis_angle(axis: Vec3, angle: f32) -> Self {
        let half_angle = angle * 0.5;
        let s = half_angle.sin();
        let c = half_angle.cos();
        let axis = axis.normalize();
        Self::new(axis.x * s, axis.y * s, axis.z * s, c)
    }

    /// Creates a quaternion from Euler angles (in radians)
    /// Order: ZYX (yaw, pitch, roll)
    #[inline]
    pub fn from_euler(roll: f32, pitch: f32, yaw: f32) -> Self {
        let (sr, cr) = (roll * 0.5).sin_cos();
        let (sp, cp) = (pitch * 0.5).sin_cos();
        let (sy, cy) = (yaw * 0.5).sin_cos();

        Self::new(
            sr * cp * cy - cr * sp * sy,
            cr * sp * cy + sr * cp * sy,
            cr * cp * sy - sr * sp * cy,
            cr * cp * cy + sr * sp * sy,
        )
    }

    /// Creates a quaternion from a rotation matrix
    #[inline]
    pub fn from_rotation_matrix(m: &Mat3) -> Self {
        let trace = m.x_axis.x + m.y_axis.y + m.z_axis.z;

        if trace > 0.0 {
            let s = (trace + 1.0).sqrt() * 2.0;
            Self::new(
                (m.y_axis.z - m.z_axis.y) / s,
                (m.z_axis.x - m.x_axis.z) / s,
                (m.x_axis.y - m.y_axis.x) / s,
                0.25 * s,
            )
        } else if m.x_axis.x > m.y_axis.y && m.x_axis.x > m.z_axis.z {
            let s = (1.0 + m.x_axis.x - m.y_axis.y - m.z_axis.z).sqrt() * 2.0;
            Self::new(
                0.25 * s,
                (m.y_axis.x + m.x_axis.y) / s,
                (m.z_axis.x + m.x_axis.z) / s,
                (m.y_axis.z - m.z_axis.y) / s,
            )
        } else if m.y_axis.y > m.z_axis.z {
            let s = (1.0 + m.y_axis.y - m.x_axis.x - m.z_axis.z).sqrt() * 2.0;
            Self::new(
                (m.y_axis.x + m.x_axis.y) / s,
                0.25 * s,
                (m.z_axis.y + m.y_axis.z) / s,
                (m.z_axis.x - m.x_axis.z) / s,
            )
        } else {
            let s = (1.0 + m.z_axis.z - m.x_axis.x - m.y_axis.y).sqrt() * 2.0;
            Self::new(
                (m.z_axis.x + m.x_axis.z) / s,
                (m.z_axis.y + m.y_axis.z) / s,
                0.25 * s,
                (m.x_axis.y - m.y_axis.x) / s,
            )
        }
    }

    /// Creates a quaternion that rotates from one vector to another
    #[inline]
    pub fn from_rotation_arc(from: Vec3, to: Vec3) -> Self {
        let from = from.normalize();
        let to = to.normalize();

        let dot = from.dot(to);

        if dot > 0.99999 {
            return Self::IDENTITY;
        }

        if dot < -0.99999 {
            // Vectors are opposite, find an orthogonal axis
            let mut axis = Vec3::X.cross(from);
            if axis.length_squared() < 0.0001 {
                axis = Vec3::Y.cross(from);
            }
            return Self::from_axis_angle(axis.normalize(), core::f32::consts::PI);
        }

        let axis = from.cross(to);
        let s = ((1.0 + dot) * 2.0).sqrt();
        let inv_s = 1.0 / s;

        Self::new(axis.x * inv_s, axis.y * inv_s, axis.z * inv_s, s * 0.5)
    }

    /// Creates a quaternion for rotation around the X axis
    #[inline]
    pub fn from_rotation_x(angle: f32) -> Self {
        let (s, c) = (angle * 0.5).sin_cos();
        Self::new(s, 0.0, 0.0, c)
    }

    /// Creates a quaternion for rotation around the Y axis
    #[inline]
    pub fn from_rotation_y(angle: f32) -> Self {
        let (s, c) = (angle * 0.5).sin_cos();
        Self::new(0.0, s, 0.0, c)
    }

    /// Creates a quaternion for rotation around the Z axis
    #[inline]
    pub fn from_rotation_z(angle: f32) -> Self {
        let (s, c) = (angle * 0.5).sin_cos();
        Self::new(0.0, 0.0, s, c)
    }

    /// Returns the conjugate of this quaternion
    #[inline]
    pub fn conjugate(self) -> Self {
        Self::new(-self.x, -self.y, -self.z, self.w)
    }

    /// Returns the inverse of this quaternion
    #[inline]
    pub fn inverse(self) -> Self {
        let len_sq = self.length_squared();
        if len_sq > 0.0 {
            let inv_len_sq = 1.0 / len_sq;
            Self::new(
                -self.x * inv_len_sq,
                -self.y * inv_len_sq,
                -self.z * inv_len_sq,
                self.w * inv_len_sq,
            )
        } else {
            Self::IDENTITY
        }
    }

    /// Returns the squared length of this quaternion
    #[inline]
    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w
    }

    /// Returns the length of this quaternion
    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Returns a normalized version of this quaternion
    #[inline]
    pub fn normalize(self) -> Self {
        let len = self.length();
        if len > 0.0 {
            let inv_len = 1.0 / len;
            Self::new(
                self.x * inv_len,
                self.y * inv_len,
                self.z * inv_len,
                self.w * inv_len,
            )
        } else {
            Self::IDENTITY
        }
    }

    /// Returns true if this quaternion is normalized
    #[inline]
    pub fn is_normalized(self) -> bool {
        (self.length_squared() - 1.0).abs() < 1e-5
    }

    /// Dot product of two quaternions
    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }

    /// Spherical linear interpolation between two quaternions
    #[inline]
    pub fn slerp(self, other: Self, t: f32) -> Self {
        let mut dot = self.dot(other);

        // If the dot product is negative, negate one quaternion to take the shorter path
        let other = if dot < 0.0 {
            dot = -dot;
            -other
        } else {
            other
        };

        // Use linear interpolation for very close quaternions
        if dot > 0.9995 {
            return Self::new(
                self.x + t * (other.x - self.x),
                self.y + t * (other.y - self.y),
                self.z + t * (other.z - self.z),
                self.w + t * (other.w - self.w),
            )
            .normalize();
        }

        let theta_0 = dot.acos();
        let theta = theta_0 * t;

        let sin_theta = theta.sin();
        let sin_theta_0 = theta_0.sin();

        let s0 = (theta_0 - theta).cos() - dot * sin_theta / sin_theta_0;
        let s1 = sin_theta / sin_theta_0;

        Self::new(
            s0 * self.x + s1 * other.x,
            s0 * self.y + s1 * other.y,
            s0 * self.z + s1 * other.z,
            s0 * self.w + s1 * other.w,
        )
    }

    /// Normalized linear interpolation (faster but less accurate than slerp)
    #[inline]
    pub fn nlerp(self, other: Self, t: f32) -> Self {
        let mut other = other;
        if self.dot(other) < 0.0 {
            other = -other;
        }

        Self::new(
            self.x + t * (other.x - self.x),
            self.y + t * (other.y - self.y),
            self.z + t * (other.z - self.z),
            self.w + t * (other.w - self.w),
        )
        .normalize()
    }

    /// Rotates a vector by this quaternion
    #[inline]
    pub fn rotate_vec3(self, v: Vec3) -> Vec3 {
        let qv = Vec3::new(self.x, self.y, self.z);
        let uv = qv.cross(v);
        let uuv = qv.cross(uv);
        v + ((uv * self.w) + uuv) * 2.0
    }

    /// Converts this quaternion to a 3x3 rotation matrix
    #[inline]
    pub fn to_mat3(self) -> Mat3 {
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

        Mat3::from_cols(
            Vec3::new(1.0 - (yy + zz), xy + wz, xz - wy),
            Vec3::new(xy - wz, 1.0 - (xx + zz), yz + wx),
            Vec3::new(xz + wy, yz - wx, 1.0 - (xx + yy)),
        )
    }

    /// Converts this quaternion to a 4x4 rotation matrix
    #[inline]
    pub fn to_mat4(self) -> Mat4 {
        let mat3 = self.to_mat3();
        Mat4::from_cols(
            mat3.x_axis.extend(0.0),
            mat3.y_axis.extend(0.0),
            mat3.z_axis.extend(0.0),
            crate::vec::Vec4::W,
        )
    }

    /// Returns the axis and angle of this quaternion
    #[inline]
    pub fn to_axis_angle(self) -> (Vec3, f32) {
        let angle = 2.0 * self.w.acos();
        let s = (1.0 - self.w * self.w).sqrt();

        if s < 0.0001 {
            (Vec3::X, angle)
        } else {
            (Vec3::new(self.x / s, self.y / s, self.z / s), angle)
        }
    }

    /// Returns Euler angles (roll, pitch, yaw) in radians
    #[inline]
    pub fn to_euler(self) -> (f32, f32, f32) {
        // Roll (x-axis rotation)
        let sinr_cosp = 2.0 * (self.w * self.x + self.y * self.z);
        let cosr_cosp = 1.0 - 2.0 * (self.x * self.x + self.y * self.y);
        let roll = sinr_cosp.atan2(cosr_cosp);

        // Pitch (y-axis rotation)
        let sinp = 2.0 * (self.w * self.y - self.z * self.x);
        let pitch = if sinp.abs() >= 1.0 {
            core::f32::consts::FRAC_PI_2.copysign(sinp)
        } else {
            sinp.asin()
        };

        // Yaw (z-axis rotation)
        let siny_cosp = 2.0 * (self.w * self.z + self.x * self.y);
        let cosy_cosp = 1.0 - 2.0 * (self.y * self.y + self.z * self.z);
        let yaw = siny_cosp.atan2(cosy_cosp);

        (roll, pitch, yaw)
    }

    /// Returns the vector part of this quaternion
    #[inline]
    pub fn xyz(self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }

    /// Returns the forward direction (negative Z) after rotation
    #[inline]
    pub fn forward(self) -> Vec3 {
        self.rotate_vec3(-Vec3::Z)
    }

    /// Returns the right direction (positive X) after rotation
    #[inline]
    pub fn right(self) -> Vec3 {
        self.rotate_vec3(Vec3::X)
    }

    /// Returns the up direction (positive Y) after rotation
    #[inline]
    pub fn up(self) -> Vec3 {
        self.rotate_vec3(Vec3::Y)
    }
}

impl Default for Quat {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Neg for Quat {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z, -self.w)
    }
}

impl Add for Quat {
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

impl Sub for Quat {
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

impl Mul for Quat {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self::new(
            self.w * rhs.x + self.x * rhs.w + self.y * rhs.z - self.z * rhs.y,
            self.w * rhs.y - self.x * rhs.z + self.y * rhs.w + self.z * rhs.x,
            self.w * rhs.z + self.x * rhs.y - self.y * rhs.x + self.z * rhs.w,
            self.w * rhs.w - self.x * rhs.x - self.y * rhs.y - self.z * rhs.z,
        )
    }
}

impl Mul<f32> for Quat {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: f32) -> Self {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs, self.w * rhs)
    }
}

impl Mul<Vec3> for Quat {
    type Output = Vec3;

    #[inline]
    fn mul(self, rhs: Vec3) -> Vec3 {
        self.rotate_vec3(rhs)
    }
}

impl From<[f32; 4]> for Quat {
    fn from(arr: [f32; 4]) -> Self {
        Self::new(arr[0], arr[1], arr[2], arr[3])
    }
}

impl From<Quat> for [f32; 4] {
    fn from(q: Quat) -> Self {
        [q.x, q.y, q.z, q.w]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity() {
        let q = Quat::IDENTITY;
        let v = Vec3::new(1.0, 2.0, 3.0);
        let rotated = q.rotate_vec3(v);
        assert!((rotated.x - v.x).abs() < 1e-5);
        assert!((rotated.y - v.y).abs() < 1e-5);
        assert!((rotated.z - v.z).abs() < 1e-5);
    }

    #[test]
    fn test_rotation_x() {
        let q = Quat::from_rotation_x(core::f32::consts::FRAC_PI_2);
        let v = Vec3::Y;
        let rotated = q.rotate_vec3(v);
        assert!((rotated.z - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_normalize() {
        let q = Quat::new(1.0, 2.0, 3.0, 4.0);
        let n = q.normalize();
        assert!((n.length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_inverse() {
        let q = Quat::from_axis_angle(Vec3::Y, 0.5);
        let inv = q.inverse();
        let result = q * inv;
        assert!((result.x).abs() < 1e-5);
        assert!((result.y).abs() < 1e-5);
        assert!((result.z).abs() < 1e-5);
        assert!((result.w - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_slerp() {
        let a = Quat::IDENTITY;
        let b = Quat::from_rotation_y(core::f32::consts::PI);
        let mid = a.slerp(b, 0.5);
        let (_, angle) = mid.to_axis_angle();
        assert!((angle - core::f32::consts::FRAC_PI_2).abs() < 1e-4);
    }
}
