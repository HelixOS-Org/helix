//! # Lumina Math
//!
//! GPU-compatible math library for Lumina.
//! Provides vector and matrix types that work identically on CPU and GPU.
//!
//! ## Features
//!
//! - **Vectors**: `Vec2`, `Vec3`, `Vec4`, `IVec2`, `IVec3`, `IVec4`, `UVec2`, `UVec3`, `UVec4`
//! - **Matrices**: `Mat2`, `Mat3`, `Mat4`
//! - **Quaternions**: `Quat` for rotation representation
//! - **Transforms**: `Transform`, `Transform2D`, `Isometry`
//! - **Geometry**: `Ray`, `Plane`, `AABB`, `Sphere`, `Frustum`, `Rect`
//! - **Colors**: `Color`, `LinearColor` with HSV/HSL conversion
//! - **Noise**: Perlin, Simplex, Worley, Value noise
//! - **Interpolation**: Bezier, Catmull-Rom, Hermite, Easing functions
//! - **Projections**: Perspective, Orthographic matrices
//! - **SIMD**: Optional x86_64 SIMD optimizations

#![no_std]
#![warn(missing_docs)]

mod color;
mod geometry;
mod int_vec;
mod interpolation;
mod mat;
mod noise;
mod projection;
mod quat;
mod transform;
mod vec;

#[cfg(feature = "simd")]
mod simd;

pub use color::{Color, LinearColor};
pub use geometry::{Frustum, Intersection, Plane, Ray, Rect, Sphere, AABB};
pub use int_vec::{IVec2, IVec3, IVec4, UVec2, UVec3, UVec4};
pub use interpolation::{
    bezier, catmull_rom, ease, ease_lerp, ease_lerp_vec2, ease_lerp_vec3, ease_lerp_vec4, hermite,
    inverse_lerp, lerp, remap, saturate, smootherstep, smoothstep,
};
pub use mat::{Mat2, Mat3, Mat4};
pub use noise::{fbm, perlin, simplex, value, worley};
pub use projection::{
    orthographic, orthographic_2d, orthographic_lh_zo, orthographic_rh_no, orthographic_rh_zo,
    orthographic_symmetric_rh_zo, perspective, perspective_infinite_reverse_rh_zo,
    perspective_infinite_rh_zo, perspective_lh_no, perspective_lh_zo, perspective_rh_no,
    perspective_rh_zo,
};
pub use quat::Quat;
#[cfg(feature = "simd")]
pub use simd::{
    batch_transform_points, batch_transform_vectors, cross3_simd, dot4_simd, mat4_mul_mat4_simd,
    mat4_mul_vec4_simd, SimdMat4, SimdVec4,
};
pub use transform::{Isometry, Transform, Transform2D};
pub use vec::{Vec2, Vec3, Vec4};

/// Common mathematical constants
pub mod consts {
    /// Pi (π)
    pub const PI: f32 = core::f32::consts::PI;
    /// Tau (2π)
    pub const TAU: f32 = PI * 2.0;
    /// E (Euler's number)
    pub const E: f32 = core::f32::consts::E;
    /// Square root of 2
    pub const SQRT_2: f32 = core::f32::consts::SQRT_2;
    /// Golden ratio (φ)
    pub const PHI: f32 = 1.618033988749895;
    /// Degrees to radians multiplier
    pub const DEG2RAD: f32 = PI / 180.0;
    /// Radians to degrees multiplier
    pub const RAD2DEG: f32 = 180.0 / PI;
}

/// Converts degrees to radians
#[inline]
pub fn radians(degrees: f32) -> f32 {
    degrees * consts::DEG2RAD
}

/// Converts radians to degrees
#[inline]
pub fn degrees(radians: f32) -> f32 {
    radians * consts::RAD2DEG
}

/// Clamps a value between min and max
#[inline]
pub fn clamp(value: f32, min: f32, max: f32) -> f32 {
    value.max(min).min(max)
}

/// Returns the fractional part of a float
#[inline]
pub fn fract(x: f32) -> f32 {
    x - x.floor()
}

/// Returns the sign of a value (-1, 0, or 1)
#[inline]
pub fn sign(x: f32) -> f32 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// Returns 1.0 if x >= edge, else 0.0
#[inline]
pub fn step(edge: f32, x: f32) -> f32 {
    if x >= edge {
        1.0
    } else {
        0.0
    }
}

/// Wraps value to the range [0, max]
#[inline]
pub fn wrap(value: f32, max: f32) -> f32 {
    ((value % max) + max) % max
}

/// Returns the minimum of three values
#[inline]
pub fn min3(a: f32, b: f32, c: f32) -> f32 {
    a.min(b).min(c)
}

/// Returns the maximum of three values
#[inline]
pub fn max3(a: f32, b: f32, c: f32) -> f32 {
    a.max(b).max(c)
}

/// Approximately compares two floats
#[inline]
pub fn approx_eq(a: f32, b: f32, epsilon: f32) -> bool {
    (a - b).abs() < epsilon
}

/// Checks if a float is approximately zero
#[inline]
pub fn approx_zero(x: f32, epsilon: f32) -> bool {
    x.abs() < epsilon
}
