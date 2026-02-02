//! Transform types for 3D transformations
//!
//! This module provides types for representing and manipulating
//! 3D transformations including translation, rotation, and scale.

use crate::mat::Mat4;
use crate::quat::Quat;
use crate::vec::{Vec3, Vec4};

/// A 3D affine transform consisting of translation, rotation, and scale.
///
/// Transforms are applied in the order: Scale -> Rotate -> Translate
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct Transform {
    /// Translation component
    pub translation: Vec3,
    /// Rotation component (quaternion)
    pub rotation: Quat,
    /// Scale component
    pub scale: Vec3,
}

impl Transform {
    /// Identity transform (no transformation)
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    /// Creates a new transform from components
    #[inline]
    pub const fn new(translation: Vec3, rotation: Quat, scale: Vec3) -> Self {
        Self {
            translation,
            rotation,
            scale,
        }
    }

    /// Creates a transform with only translation
    #[inline]
    pub fn from_translation(translation: Vec3) -> Self {
        Self {
            translation,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    /// Creates a transform with only rotation
    #[inline]
    pub fn from_rotation(rotation: Quat) -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation,
            scale: Vec3::ONE,
        }
    }

    /// Creates a transform with only scale
    #[inline]
    pub fn from_scale(scale: Vec3) -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale,
        }
    }

    /// Creates a transform with uniform scale
    #[inline]
    pub fn from_uniform_scale(scale: f32) -> Self {
        Self::from_scale(Vec3::splat(scale))
    }

    /// Creates a transform from translation and rotation
    #[inline]
    pub fn from_translation_rotation(translation: Vec3, rotation: Quat) -> Self {
        Self {
            translation,
            rotation,
            scale: Vec3::ONE,
        }
    }

    /// Creates a transform from a 4x4 matrix (assuming affine)
    #[inline]
    pub fn from_matrix(m: Mat4) -> Self {
        // Extract translation
        let translation = Vec3::new(m.w_axis.x, m.w_axis.y, m.w_axis.z);

        // Extract scale
        let scale_x = Vec3::new(m.x_axis.x, m.x_axis.y, m.x_axis.z).length();
        let scale_y = Vec3::new(m.y_axis.x, m.y_axis.y, m.y_axis.z).length();
        let scale_z = Vec3::new(m.z_axis.x, m.z_axis.y, m.z_axis.z).length();
        let scale = Vec3::new(scale_x, scale_y, scale_z);

        // Extract rotation by removing scale from the matrix
        let inv_scale_x = if scale_x != 0.0 { 1.0 / scale_x } else { 0.0 };
        let inv_scale_y = if scale_y != 0.0 { 1.0 / scale_y } else { 0.0 };
        let inv_scale_z = if scale_z != 0.0 { 1.0 / scale_z } else { 0.0 };

        let rot_mat = crate::mat::Mat3::from_cols(
            Vec3::new(m.x_axis.x * inv_scale_x, m.x_axis.y * inv_scale_x, m.x_axis.z * inv_scale_x),
            Vec3::new(m.y_axis.x * inv_scale_y, m.y_axis.y * inv_scale_y, m.y_axis.z * inv_scale_y),
            Vec3::new(m.z_axis.x * inv_scale_z, m.z_axis.y * inv_scale_z, m.z_axis.z * inv_scale_z),
        );

        let rotation = Quat::from_rotation_matrix(&rot_mat);

        Self {
            translation,
            rotation,
            scale,
        }
    }

    /// Converts this transform to a 4x4 matrix
    #[inline]
    pub fn to_matrix(self) -> Mat4 {
        let rotation_matrix = self.rotation.to_mat4();

        // Apply scale to rotation matrix
        let x_axis = rotation_matrix.x_axis * self.scale.x;
        let y_axis = rotation_matrix.y_axis * self.scale.y;
        let z_axis = rotation_matrix.z_axis * self.scale.z;

        Mat4::from_cols(
            x_axis,
            y_axis,
            z_axis,
            self.translation.extend(1.0),
        )
    }

    /// Returns the inverse of this transform
    #[inline]
    pub fn inverse(self) -> Self {
        let inv_scale = Vec3::new(
            if self.scale.x != 0.0 { 1.0 / self.scale.x } else { 0.0 },
            if self.scale.y != 0.0 { 1.0 / self.scale.y } else { 0.0 },
            if self.scale.z != 0.0 { 1.0 / self.scale.z } else { 0.0 },
        );

        let inv_rotation = self.rotation.conjugate();
        let inv_translation = inv_rotation.rotate_vec3(-self.translation) * inv_scale;

        Self {
            translation: inv_translation,
            rotation: inv_rotation,
            scale: inv_scale,
        }
    }

    /// Transforms a point by this transform
    #[inline]
    pub fn transform_point(self, point: Vec3) -> Vec3 {
        self.rotation.rotate_vec3(point * self.scale) + self.translation
    }

    /// Transforms a vector by this transform (ignores translation)
    #[inline]
    pub fn transform_vector(self, vector: Vec3) -> Vec3 {
        self.rotation.rotate_vec3(vector * self.scale)
    }

    /// Transforms a direction by this transform (ignores translation and scale)
    #[inline]
    pub fn transform_direction(self, direction: Vec3) -> Vec3 {
        self.rotation.rotate_vec3(direction)
    }

    /// Multiplies two transforms (self * other)
    #[inline]
    pub fn mul_transform(self, other: Self) -> Self {
        Self {
            translation: self.transform_point(other.translation),
            rotation: self.rotation * other.rotation,
            scale: self.scale * other.scale,
        }
    }

    /// Interpolates between two transforms
    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            translation: self.translation.lerp(other.translation, t),
            rotation: self.rotation.slerp(other.rotation, t),
            scale: self.scale.lerp(other.scale, t),
        }
    }

    /// Rotates the transform around a point
    #[inline]
    pub fn rotate_around(self, point: Vec3, rotation: Quat) -> Self {
        let offset = self.translation - point;
        let rotated_offset = rotation.rotate_vec3(offset);

        Self {
            translation: point + rotated_offset,
            rotation: rotation * self.rotation,
            scale: self.scale,
        }
    }

    /// Returns the forward direction of this transform
    #[inline]
    pub fn forward(self) -> Vec3 {
        self.rotation.forward()
    }

    /// Returns the right direction of this transform
    #[inline]
    pub fn right(self) -> Vec3 {
        self.rotation.right()
    }

    /// Returns the up direction of this transform
    #[inline]
    pub fn up(self) -> Vec3 {
        self.rotation.up()
    }

    /// Makes this transform look at a target position
    #[inline]
    pub fn look_at(self, target: Vec3, up: Vec3) -> Self {
        let forward = (target - self.translation).normalize();
        let right = up.cross(forward).normalize();
        let actual_up = forward.cross(right);

        let rotation = Quat::from_rotation_matrix(&crate::mat::Mat3::from_cols(
            right,
            actual_up,
            -forward,
        ));

        Self {
            translation: self.translation,
            rotation,
            scale: self.scale,
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl core::ops::Mul for Transform {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self {
        self.mul_transform(rhs)
    }
}

impl core::ops::Mul<Vec3> for Transform {
    type Output = Vec3;

    #[inline]
    fn mul(self, rhs: Vec3) -> Vec3 {
        self.transform_point(rhs)
    }
}

// Multiply Vec3's scale component
impl core::ops::Mul<Vec3> for Vec3 {
    type Output = Vec3;

    #[inline]
    fn mul(self, rhs: Vec3) -> Vec3 {
        Vec3::new(self.x * rhs.x, self.y * rhs.y, self.z * rhs.z)
    }
}

/// A 2D affine transform
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct Transform2D {
    /// Translation component
    pub translation: crate::vec::Vec2,
    /// Rotation angle in radians
    pub rotation: f32,
    /// Scale component
    pub scale: crate::vec::Vec2,
}

impl Transform2D {
    /// Identity transform
    pub const IDENTITY: Self = Self {
        translation: crate::vec::Vec2::ZERO,
        rotation: 0.0,
        scale: crate::vec::Vec2::ONE,
    };

    /// Creates a new 2D transform
    #[inline]
    pub const fn new(translation: crate::vec::Vec2, rotation: f32, scale: crate::vec::Vec2) -> Self {
        Self {
            translation,
            rotation,
            scale,
        }
    }

    /// Creates a transform with only translation
    #[inline]
    pub fn from_translation(translation: crate::vec::Vec2) -> Self {
        Self {
            translation,
            rotation: 0.0,
            scale: crate::vec::Vec2::ONE,
        }
    }

    /// Creates a transform with only rotation
    #[inline]
    pub fn from_rotation(rotation: f32) -> Self {
        Self {
            translation: crate::vec::Vec2::ZERO,
            rotation,
            scale: crate::vec::Vec2::ONE,
        }
    }

    /// Creates a transform with only scale
    #[inline]
    pub fn from_scale(scale: crate::vec::Vec2) -> Self {
        Self {
            translation: crate::vec::Vec2::ZERO,
            rotation: 0.0,
            scale,
        }
    }

    /// Converts to a 3x3 matrix
    #[inline]
    pub fn to_matrix(self) -> [[f32; 3]; 3] {
        let (sin, cos) = self.rotation.sin_cos();

        [
            [cos * self.scale.x, sin * self.scale.x, 0.0],
            [-sin * self.scale.y, cos * self.scale.y, 0.0],
            [self.translation.x, self.translation.y, 1.0],
        ]
    }

    /// Transforms a point
    #[inline]
    pub fn transform_point(self, point: crate::vec::Vec2) -> crate::vec::Vec2 {
        let (sin, cos) = self.rotation.sin_cos();
        let scaled = crate::vec::Vec2::new(point.x * self.scale.x, point.y * self.scale.y);
        let rotated = crate::vec::Vec2::new(
            scaled.x * cos - scaled.y * sin,
            scaled.x * sin + scaled.y * cos,
        );
        rotated + self.translation
    }

    /// Returns the inverse transform
    #[inline]
    pub fn inverse(self) -> Self {
        let inv_scale = crate::vec::Vec2::new(
            if self.scale.x != 0.0 { 1.0 / self.scale.x } else { 0.0 },
            if self.scale.y != 0.0 { 1.0 / self.scale.y } else { 0.0 },
        );

        let inv_rotation = -self.rotation;
        let (sin, cos) = inv_rotation.sin_cos();

        let inv_translation = crate::vec::Vec2::new(
            (-self.translation.x * cos + self.translation.y * sin) * inv_scale.x,
            (-self.translation.x * sin - self.translation.y * cos) * inv_scale.y,
        );

        Self {
            translation: inv_translation,
            rotation: inv_rotation,
            scale: inv_scale,
        }
    }

    /// Interpolates between two transforms
    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            translation: self.translation.lerp(other.translation, t),
            rotation: self.rotation + (other.rotation - self.rotation) * t,
            scale: self.scale.lerp(other.scale, t),
        }
    }
}

impl Default for Transform2D {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// A rigid body transform (translation + rotation only, no scale)
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct Isometry {
    /// Translation component
    pub translation: Vec3,
    /// Rotation component
    pub rotation: Quat,
}

impl Isometry {
    /// Identity isometry
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
    };

    /// Creates a new isometry
    #[inline]
    pub const fn new(translation: Vec3, rotation: Quat) -> Self {
        Self { translation, rotation }
    }

    /// Creates from translation only
    #[inline]
    pub fn from_translation(translation: Vec3) -> Self {
        Self {
            translation,
            rotation: Quat::IDENTITY,
        }
    }

    /// Creates from rotation only
    #[inline]
    pub fn from_rotation(rotation: Quat) -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation,
        }
    }

    /// Converts to a 4x4 matrix
    #[inline]
    pub fn to_matrix(self) -> Mat4 {
        let rotation_matrix = self.rotation.to_mat4();
        Mat4::from_cols(
            rotation_matrix.x_axis,
            rotation_matrix.y_axis,
            rotation_matrix.z_axis,
            self.translation.extend(1.0),
        )
    }

    /// Returns the inverse isometry
    #[inline]
    pub fn inverse(self) -> Self {
        let inv_rotation = self.rotation.conjugate();
        Self {
            translation: inv_rotation.rotate_vec3(-self.translation),
            rotation: inv_rotation,
        }
    }

    /// Transforms a point
    #[inline]
    pub fn transform_point(self, point: Vec3) -> Vec3 {
        self.rotation.rotate_vec3(point) + self.translation
    }

    /// Transforms a vector (ignores translation)
    #[inline]
    pub fn transform_vector(self, vector: Vec3) -> Vec3 {
        self.rotation.rotate_vec3(vector)
    }

    /// Multiplies two isometries
    #[inline]
    pub fn mul_isometry(self, other: Self) -> Self {
        Self {
            translation: self.transform_point(other.translation),
            rotation: self.rotation * other.rotation,
        }
    }

    /// Interpolates between two isometries
    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            translation: self.translation.lerp(other.translation, t),
            rotation: self.rotation.slerp(other.rotation, t),
        }
    }
}

impl Default for Isometry {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl core::ops::Mul for Isometry {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self {
        self.mul_isometry(rhs)
    }
}

impl core::ops::Mul<Vec3> for Isometry {
    type Output = Vec3;

    #[inline]
    fn mul(self, rhs: Vec3) -> Vec3 {
        self.transform_point(rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_identity() {
        let t = Transform::IDENTITY;
        let p = Vec3::new(1.0, 2.0, 3.0);
        let result = t.transform_point(p);
        assert!((result.x - p.x).abs() < 1e-5);
        assert!((result.y - p.y).abs() < 1e-5);
        assert!((result.z - p.z).abs() < 1e-5);
    }

    #[test]
    fn test_transform_translation() {
        let t = Transform::from_translation(Vec3::new(1.0, 2.0, 3.0));
        let p = Vec3::ZERO;
        let result = t.transform_point(p);
        assert!((result.x - 1.0).abs() < 1e-5);
        assert!((result.y - 2.0).abs() < 1e-5);
        assert!((result.z - 3.0).abs() < 1e-5);
    }

    #[test]
    fn test_transform_inverse() {
        let t = Transform::new(
            Vec3::new(1.0, 2.0, 3.0),
            Quat::from_rotation_y(0.5),
            Vec3::new(2.0, 2.0, 2.0),
        );
        let inv = t.inverse();
        let p = Vec3::new(5.0, 6.0, 7.0);
        let transformed = t.transform_point(p);
        let result = inv.transform_point(transformed);
        assert!((result.x - p.x).abs() < 1e-4);
        assert!((result.y - p.y).abs() < 1e-4);
        assert!((result.z - p.z).abs() < 1e-4);
    }
}
