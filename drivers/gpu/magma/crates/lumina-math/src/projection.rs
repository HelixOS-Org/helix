//! Projection matrices
//!
//! This module provides common projection matrix constructors.

use crate::mat::Mat4;

/// Creates a perspective projection matrix (right-handed, zero-to-one depth)
///
/// # Arguments
/// * `fov_y` - Vertical field of view in radians
/// * `aspect` - Aspect ratio (width / height)
/// * `z_near` - Near clipping plane distance
/// * `z_far` - Far clipping plane distance
#[inline]
pub fn perspective_rh_zo(fov_y: f32, aspect: f32, z_near: f32, z_far: f32) -> Mat4 {
    let tan_half_fov = (fov_y / 2.0).tan();
    let f = 1.0 / tan_half_fov;

    let mut m = Mat4::ZERO;
    m.m[0] = f / aspect;
    m.m[5] = f;
    m.m[10] = z_far / (z_near - z_far);
    m.m[11] = -1.0;
    m.m[14] = (z_near * z_far) / (z_near - z_far);
    m
}

/// Creates a perspective projection matrix (right-handed, negative-one-to-one depth)
///
/// # Arguments
/// * `fov_y` - Vertical field of view in radians
/// * `aspect` - Aspect ratio (width / height)
/// * `z_near` - Near clipping plane distance
/// * `z_far` - Far clipping plane distance
#[inline]
pub fn perspective_rh_no(fov_y: f32, aspect: f32, z_near: f32, z_far: f32) -> Mat4 {
    let tan_half_fov = (fov_y / 2.0).tan();
    let f = 1.0 / tan_half_fov;

    let mut m = Mat4::ZERO;
    m.m[0] = f / aspect;
    m.m[5] = f;
    m.m[10] = (z_far + z_near) / (z_near - z_far);
    m.m[11] = -1.0;
    m.m[14] = (2.0 * z_near * z_far) / (z_near - z_far);
    m
}

/// Creates a perspective projection matrix (left-handed, zero-to-one depth)
#[inline]
pub fn perspective_lh_zo(fov_y: f32, aspect: f32, z_near: f32, z_far: f32) -> Mat4 {
    let tan_half_fov = (fov_y / 2.0).tan();
    let f = 1.0 / tan_half_fov;

    let mut m = Mat4::ZERO;
    m.m[0] = f / aspect;
    m.m[5] = f;
    m.m[10] = z_far / (z_far - z_near);
    m.m[11] = 1.0;
    m.m[14] = -(z_near * z_far) / (z_far - z_near);
    m
}

/// Creates a perspective projection matrix (left-handed, negative-one-to-one depth)
#[inline]
pub fn perspective_lh_no(fov_y: f32, aspect: f32, z_near: f32, z_far: f32) -> Mat4 {
    let tan_half_fov = (fov_y / 2.0).tan();
    let f = 1.0 / tan_half_fov;

    let mut m = Mat4::ZERO;
    m.m[0] = f / aspect;
    m.m[5] = f;
    m.m[10] = (z_far + z_near) / (z_far - z_near);
    m.m[11] = 1.0;
    m.m[14] = -(2.0 * z_near * z_far) / (z_far - z_near);
    m
}

/// Creates an infinite perspective projection matrix (right-handed, zero-to-one depth)
///
/// Useful for rendering skyboxes and very distant objects.
#[inline]
pub fn perspective_infinite_rh_zo(fov_y: f32, aspect: f32, z_near: f32) -> Mat4 {
    let tan_half_fov = (fov_y / 2.0).tan();
    let f = 1.0 / tan_half_fov;

    let mut m = Mat4::ZERO;
    m.m[0] = f / aspect;
    m.m[5] = f;
    m.m[10] = -1.0;
    m.m[11] = -1.0;
    m.m[14] = -z_near;
    m
}

/// Creates an infinite perspective projection matrix (right-handed, reversed Z)
///
/// Provides better depth precision for large scenes.
#[inline]
pub fn perspective_infinite_reverse_rh_zo(fov_y: f32, aspect: f32, z_near: f32) -> Mat4 {
    let tan_half_fov = (fov_y / 2.0).tan();
    let f = 1.0 / tan_half_fov;

    let mut m = Mat4::ZERO;
    m.m[0] = f / aspect;
    m.m[5] = f;
    m.m[10] = 0.0;
    m.m[11] = -1.0;
    m.m[14] = z_near;
    m
}

/// Creates an orthographic projection matrix (right-handed, zero-to-one depth)
///
/// # Arguments
/// * `left` - Left clipping plane
/// * `right` - Right clipping plane
/// * `bottom` - Bottom clipping plane
/// * `top` - Top clipping plane
/// * `z_near` - Near clipping plane
/// * `z_far` - Far clipping plane
#[inline]
pub fn orthographic_rh_zo(
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    z_near: f32,
    z_far: f32,
) -> Mat4 {
    let mut m = Mat4::IDENTITY;
    m.m[0] = 2.0 / (right - left);
    m.m[5] = 2.0 / (top - bottom);
    m.m[10] = 1.0 / (z_near - z_far);
    m.m[12] = -(right + left) / (right - left);
    m.m[13] = -(top + bottom) / (top - bottom);
    m.m[14] = z_near / (z_near - z_far);
    m
}

/// Creates an orthographic projection matrix (right-handed, negative-one-to-one depth)
#[inline]
pub fn orthographic_rh_no(
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    z_near: f32,
    z_far: f32,
) -> Mat4 {
    let mut m = Mat4::IDENTITY;
    m.m[0] = 2.0 / (right - left);
    m.m[5] = 2.0 / (top - bottom);
    m.m[10] = 2.0 / (z_near - z_far);
    m.m[12] = -(right + left) / (right - left);
    m.m[13] = -(top + bottom) / (top - bottom);
    m.m[14] = (z_far + z_near) / (z_near - z_far);
    m
}

/// Creates an orthographic projection matrix (left-handed, zero-to-one depth)
#[inline]
pub fn orthographic_lh_zo(
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    z_near: f32,
    z_far: f32,
) -> Mat4 {
    let mut m = Mat4::IDENTITY;
    m.m[0] = 2.0 / (right - left);
    m.m[5] = 2.0 / (top - bottom);
    m.m[10] = 1.0 / (z_far - z_near);
    m.m[12] = -(right + left) / (right - left);
    m.m[13] = -(top + bottom) / (top - bottom);
    m.m[14] = -z_near / (z_far - z_near);
    m
}

/// Creates a centered orthographic projection (symmetric around origin)
#[inline]
pub fn orthographic_symmetric_rh_zo(width: f32, height: f32, z_near: f32, z_far: f32) -> Mat4 {
    let hw = width / 2.0;
    let hh = height / 2.0;
    orthographic_rh_zo(-hw, hw, -hh, hh, z_near, z_far)
}

/// Creates a 2D orthographic projection for UI/2D rendering
///
/// Maps screen coordinates directly (0,0 at top-left)
#[inline]
pub fn orthographic_2d(width: f32, height: f32) -> Mat4 {
    orthographic_rh_zo(0.0, width, height, 0.0, -1.0, 1.0)
}

/// Default perspective projection (Vulkan-style: RH, zero-to-one)
#[inline]
pub fn perspective(fov_y: f32, aspect: f32, z_near: f32, z_far: f32) -> Mat4 {
    perspective_rh_zo(fov_y, aspect, z_near, z_far)
}

/// Default orthographic projection (Vulkan-style: RH, zero-to-one)
#[inline]
pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, z_near: f32, z_far: f32) -> Mat4 {
    orthographic_rh_zo(left, right, bottom, top, z_near, z_far)
}

#[cfg(test)]
mod tests {
    use core::f32::consts::FRAC_PI_4;

    use super::*;
    use crate::vec::Vec4;

    #[test]
    fn test_perspective_frustum() {
        let proj = perspective(FRAC_PI_4, 1.0, 0.1, 100.0);

        // Point on near plane should map to z=0
        let near_point = proj * Vec4::new(0.0, 0.0, -0.1, 1.0);
        let near_ndc = near_point / near_point.w;
        assert!((near_ndc.z - 0.0).abs() < 0.01);

        // Point on far plane should map to z=1
        let far_point = proj * Vec4::new(0.0, 0.0, -100.0, 1.0);
        let far_ndc = far_point / far_point.w;
        assert!((far_ndc.z - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_orthographic_corners() {
        let proj = orthographic(-1.0, 1.0, -1.0, 1.0, 0.0, 1.0);

        // Corner should map to NDC corner
        let corner = proj * Vec4::new(1.0, 1.0, 0.0, 1.0);
        assert!((corner.x - 1.0).abs() < 0.01);
        assert!((corner.y - 1.0).abs() < 0.01);
        assert!((corner.z - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_orthographic_2d() {
        let proj = orthographic_2d(800.0, 600.0);

        // Top-left corner (0,0) should map to (-1, 1)
        let tl = proj * Vec4::new(0.0, 0.0, 0.0, 1.0);
        assert!((tl.x - (-1.0)).abs() < 0.01);
        assert!((tl.y - 1.0).abs() < 0.01);

        // Bottom-right corner should map to (1, -1)
        let br = proj * Vec4::new(800.0, 600.0, 0.0, 1.0);
        assert!((br.x - 1.0).abs() < 0.01);
        assert!((br.y - (-1.0)).abs() < 0.01);
    }
}
