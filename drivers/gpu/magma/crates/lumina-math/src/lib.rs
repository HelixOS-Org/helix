//! # Lumina Math
//!
//! GPU-compatible math library for Lumina.
//! Provides vector and matrix types that work identically on CPU and GPU.

#![no_std]
#![warn(missing_docs)]

mod mat;
mod vec;

pub use mat::{Mat2, Mat3, Mat4};
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
}

/// Converts degrees to radians
#[inline]
pub fn radians(degrees: f32) -> f32 {
    degrees * (consts::PI / 180.0)
}

/// Converts radians to degrees
#[inline]
pub fn degrees(radians: f32) -> f32 {
    radians * (180.0 / consts::PI)
}

/// Linear interpolation
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Clamps a value between min and max
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

/// Smooth step interpolation
#[inline]
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Smoother step interpolation (5th order polynomial)
#[inline]
pub fn smootherstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}
