//! Interpolation and easing functions
//!
//! This module provides common interpolation methods and easing functions
//! for animations and smooth transitions.

use crate::vec::{Vec2, Vec3, Vec4};
use core::f32::consts::PI;

/// Linearly interpolates between two values
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Inverse linear interpolation - finds t given a, b, and a value between them
#[inline]
pub fn inverse_lerp(a: f32, b: f32, v: f32) -> f32 {
    if (b - a).abs() < f32::EPSILON {
        0.0
    } else {
        (v - a) / (b - a)
    }
}

/// Remaps a value from one range to another
#[inline]
pub fn remap(value: f32, from_min: f32, from_max: f32, to_min: f32, to_max: f32) -> f32 {
    let t = inverse_lerp(from_min, from_max, value);
    lerp(to_min, to_max, t)
}

/// Smoothly interpolates between 0 and 1 (3rd order)
#[inline]
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Smoother interpolation between 0 and 1 (5th order)
#[inline]
pub fn smootherstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Clamps a value to [0, 1]
#[inline]
pub fn saturate(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}

/// Bezier curve evaluation
pub mod bezier {
    use super::*;

    /// Evaluates a quadratic Bezier curve
    #[inline]
    pub fn quadratic(p0: Vec2, p1: Vec2, p2: Vec2, t: f32) -> Vec2 {
        let t2 = t * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        p0 * mt2 + p1 * (2.0 * mt * t) + p2 * t2
    }

    /// Evaluates a quadratic Bezier curve derivative
    #[inline]
    pub fn quadratic_derivative(p0: Vec2, p1: Vec2, p2: Vec2, t: f32) -> Vec2 {
        let mt = 1.0 - t;
        (p1 - p0) * (2.0 * mt) + (p2 - p1) * (2.0 * t)
    }

    /// Evaluates a cubic Bezier curve
    #[inline]
    pub fn cubic(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let mt3 = mt2 * mt;
        p0 * mt3 + p1 * (3.0 * mt2 * t) + p2 * (3.0 * mt * t2) + p3 * t3
    }

    /// Evaluates a cubic Bezier curve derivative
    #[inline]
    pub fn cubic_derivative(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
        let t2 = t * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        (p1 - p0) * (3.0 * mt2) + (p2 - p1) * (6.0 * mt * t) + (p3 - p2) * (3.0 * t2)
    }

    /// Evaluates a 3D quadratic Bezier curve
    #[inline]
    pub fn quadratic_3d(p0: Vec3, p1: Vec3, p2: Vec3, t: f32) -> Vec3 {
        let t2 = t * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        p0 * mt2 + p1 * (2.0 * mt * t) + p2 * t2
    }

    /// Evaluates a 3D cubic Bezier curve
    #[inline]
    pub fn cubic_3d(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let mt3 = mt2 * mt;
        p0 * mt3 + p1 * (3.0 * mt2 * t) + p2 * (3.0 * mt * t2) + p3 * t3
    }
}

/// Catmull-Rom spline evaluation
pub mod catmull_rom {
    use super::*;

    /// Evaluates a Catmull-Rom spline at t (0-1)
    /// p0 and p3 are control points, the curve passes through p1 and p2
    #[inline]
    pub fn evaluate(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
        let t2 = t * t;
        let t3 = t2 * t;

        let v0 = (p2 - p0) * 0.5;
        let v1 = (p3 - p1) * 0.5;

        let a = p1 * 2.0 - p2 * 2.0 + v0 + v1;
        let b = p1 * -3.0 + p2 * 3.0 - v0 * 2.0 - v1;

        a * t3 + b * t2 + v0 * t + p1
    }

    /// Evaluates the derivative of a Catmull-Rom spline at t
    #[inline]
    pub fn derivative(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
        let t2 = t * t;

        let v0 = (p2 - p0) * 0.5;
        let v1 = (p3 - p1) * 0.5;

        let a = p1 * 2.0 - p2 * 2.0 + v0 + v1;
        let b = p1 * -3.0 + p2 * 3.0 - v0 * 2.0 - v1;

        a * (3.0 * t2) + b * (2.0 * t) + v0
    }

    /// Evaluates a 2D Catmull-Rom spline at t
    #[inline]
    pub fn evaluate_2d(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
        let t2 = t * t;
        let t3 = t2 * t;

        let v0 = (p2 - p0) * 0.5;
        let v1 = (p3 - p1) * 0.5;

        let a = p1 * 2.0 - p2 * 2.0 + v0 + v1;
        let b = p1 * -3.0 + p2 * 3.0 - v0 * 2.0 - v1;

        a * t3 + b * t2 + v0 * t + p1
    }
}

/// Hermite spline evaluation
pub mod hermite {
    use super::*;

    /// Evaluates a Hermite spline
    /// p0, p1: endpoints
    /// m0, m1: tangents at endpoints
    #[inline]
    pub fn evaluate(p0: Vec3, m0: Vec3, p1: Vec3, m1: Vec3, t: f32) -> Vec3 {
        let t2 = t * t;
        let t3 = t2 * t;

        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;

        p0 * h00 + m0 * h10 + p1 * h01 + m1 * h11
    }

    /// Evaluates the derivative of a Hermite spline
    #[inline]
    pub fn derivative(p0: Vec3, m0: Vec3, p1: Vec3, m1: Vec3, t: f32) -> Vec3 {
        let t2 = t * t;

        let dh00 = 6.0 * t2 - 6.0 * t;
        let dh10 = 3.0 * t2 - 4.0 * t + 1.0;
        let dh01 = -6.0 * t2 + 6.0 * t;
        let dh11 = 3.0 * t2 - 2.0 * t;

        p0 * dh00 + m0 * dh10 + p1 * dh01 + m1 * dh11
    }
}

/// Easing functions for animations
pub mod ease {
    use super::*;

    // Sine easing
    #[inline]
    pub fn in_sine(t: f32) -> f32 {
        1.0 - (t * PI * 0.5).cos()
    }

    #[inline]
    pub fn out_sine(t: f32) -> f32 {
        (t * PI * 0.5).sin()
    }

    #[inline]
    pub fn in_out_sine(t: f32) -> f32 {
        -((t * PI).cos() - 1.0) * 0.5
    }

    // Quadratic easing
    #[inline]
    pub fn in_quad(t: f32) -> f32 {
        t * t
    }

    #[inline]
    pub fn out_quad(t: f32) -> f32 {
        1.0 - (1.0 - t) * (1.0 - t)
    }

    #[inline]
    pub fn in_out_quad(t: f32) -> f32 {
        if t < 0.5 {
            2.0 * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(2) * 0.5
        }
    }

    // Cubic easing
    #[inline]
    pub fn in_cubic(t: f32) -> f32 {
        t * t * t
    }

    #[inline]
    pub fn out_cubic(t: f32) -> f32 {
        1.0 - (1.0 - t).powi(3)
    }

    #[inline]
    pub fn in_out_cubic(t: f32) -> f32 {
        if t < 0.5 {
            4.0 * t * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(3) * 0.5
        }
    }

    // Quartic easing
    #[inline]
    pub fn in_quart(t: f32) -> f32 {
        t * t * t * t
    }

    #[inline]
    pub fn out_quart(t: f32) -> f32 {
        1.0 - (1.0 - t).powi(4)
    }

    #[inline]
    pub fn in_out_quart(t: f32) -> f32 {
        if t < 0.5 {
            8.0 * t * t * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(4) * 0.5
        }
    }

    // Quintic easing
    #[inline]
    pub fn in_quint(t: f32) -> f32 {
        t * t * t * t * t
    }

    #[inline]
    pub fn out_quint(t: f32) -> f32 {
        1.0 - (1.0 - t).powi(5)
    }

    #[inline]
    pub fn in_out_quint(t: f32) -> f32 {
        if t < 0.5 {
            16.0 * t * t * t * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(5) * 0.5
        }
    }

    // Exponential easing
    #[inline]
    pub fn in_expo(t: f32) -> f32 {
        if t == 0.0 {
            0.0
        } else {
            2.0_f32.powf(10.0 * t - 10.0)
        }
    }

    #[inline]
    pub fn out_expo(t: f32) -> f32 {
        if t == 1.0 {
            1.0
        } else {
            1.0 - 2.0_f32.powf(-10.0 * t)
        }
    }

    #[inline]
    pub fn in_out_expo(t: f32) -> f32 {
        if t == 0.0 {
            0.0
        } else if t == 1.0 {
            1.0
        } else if t < 0.5 {
            2.0_f32.powf(20.0 * t - 10.0) * 0.5
        } else {
            (2.0 - 2.0_f32.powf(-20.0 * t + 10.0)) * 0.5
        }
    }

    // Circular easing
    #[inline]
    pub fn in_circ(t: f32) -> f32 {
        1.0 - (1.0 - t * t).sqrt()
    }

    #[inline]
    pub fn out_circ(t: f32) -> f32 {
        (1.0 - (t - 1.0).powi(2)).sqrt()
    }

    #[inline]
    pub fn in_out_circ(t: f32) -> f32 {
        if t < 0.5 {
            (1.0 - (1.0 - (2.0 * t).powi(2)).sqrt()) * 0.5
        } else {
            ((1.0 - (-2.0 * t + 2.0).powi(2)).sqrt() + 1.0) * 0.5
        }
    }

    // Back easing (overshooting)
    const C1: f32 = 1.70158;
    const C2: f32 = C1 * 1.525;
    const C3: f32 = C1 + 1.0;

    #[inline]
    pub fn in_back(t: f32) -> f32 {
        C3 * t * t * t - C1 * t * t
    }

    #[inline]
    pub fn out_back(t: f32) -> f32 {
        1.0 + C3 * (t - 1.0).powi(3) + C1 * (t - 1.0).powi(2)
    }

    #[inline]
    pub fn in_out_back(t: f32) -> f32 {
        if t < 0.5 {
            ((2.0 * t).powi(2) * ((C2 + 1.0) * 2.0 * t - C2)) * 0.5
        } else {
            ((2.0 * t - 2.0).powi(2) * ((C2 + 1.0) * (t * 2.0 - 2.0) + C2) + 2.0) * 0.5
        }
    }

    // Elastic easing
    const C4: f32 = (2.0 * PI) / 3.0;
    const C5: f32 = (2.0 * PI) / 4.5;

    #[inline]
    pub fn in_elastic(t: f32) -> f32 {
        if t == 0.0 {
            0.0
        } else if t == 1.0 {
            1.0
        } else {
            -(2.0_f32.powf(10.0 * t - 10.0)) * ((t * 10.0 - 10.75) * C4).sin()
        }
    }

    #[inline]
    pub fn out_elastic(t: f32) -> f32 {
        if t == 0.0 {
            0.0
        } else if t == 1.0 {
            1.0
        } else {
            2.0_f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * C4).sin() + 1.0
        }
    }

    #[inline]
    pub fn in_out_elastic(t: f32) -> f32 {
        if t == 0.0 {
            0.0
        } else if t == 1.0 {
            1.0
        } else if t < 0.5 {
            -(2.0_f32.powf(20.0 * t - 10.0) * ((20.0 * t - 11.125) * C5).sin()) * 0.5
        } else {
            (2.0_f32.powf(-20.0 * t + 10.0) * ((20.0 * t - 11.125) * C5).sin()) * 0.5 + 1.0
        }
    }

    // Bounce easing
    const N1: f32 = 7.5625;
    const D1: f32 = 2.75;

    #[inline]
    pub fn out_bounce(t: f32) -> f32 {
        if t < 1.0 / D1 {
            N1 * t * t
        } else if t < 2.0 / D1 {
            let t = t - 1.5 / D1;
            N1 * t * t + 0.75
        } else if t < 2.5 / D1 {
            let t = t - 2.25 / D1;
            N1 * t * t + 0.9375
        } else {
            let t = t - 2.625 / D1;
            N1 * t * t + 0.984375
        }
    }

    #[inline]
    pub fn in_bounce(t: f32) -> f32 {
        1.0 - out_bounce(1.0 - t)
    }

    #[inline]
    pub fn in_out_bounce(t: f32) -> f32 {
        if t < 0.5 {
            (1.0 - out_bounce(1.0 - 2.0 * t)) * 0.5
        } else {
            (1.0 + out_bounce(2.0 * t - 1.0)) * 0.5
        }
    }
}

/// Applies an easing function to interpolate between two values
#[inline]
pub fn ease_lerp<F>(a: f32, b: f32, t: f32, ease_fn: F) -> f32
where
    F: Fn(f32) -> f32,
{
    lerp(a, b, ease_fn(t))
}

/// Applies an easing function to interpolate between two Vec2 values
#[inline]
pub fn ease_lerp_vec2<F>(a: Vec2, b: Vec2, t: f32, ease_fn: F) -> Vec2
where
    F: Fn(f32) -> f32,
{
    a.lerp(b, ease_fn(t))
}

/// Applies an easing function to interpolate between two Vec3 values
#[inline]
pub fn ease_lerp_vec3<F>(a: Vec3, b: Vec3, t: f32, ease_fn: F) -> Vec3
where
    F: Fn(f32) -> f32,
{
    a.lerp(b, ease_fn(t))
}

/// Applies an easing function to interpolate between two Vec4 values
#[inline]
pub fn ease_lerp_vec4<F>(a: Vec4, b: Vec4, t: f32, ease_fn: F) -> Vec4
where
    F: Fn(f32) -> f32,
{
    a.lerp(b, ease_fn(t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lerp() {
        assert!((lerp(0.0, 10.0, 0.5) - 5.0).abs() < 1e-5);
        assert!((lerp(0.0, 10.0, 0.0) - 0.0).abs() < 1e-5);
        assert!((lerp(0.0, 10.0, 1.0) - 10.0).abs() < 1e-5);
    }

    #[test]
    fn test_inverse_lerp() {
        assert!((inverse_lerp(0.0, 10.0, 5.0) - 0.5).abs() < 1e-5);
        assert!((inverse_lerp(0.0, 10.0, 0.0) - 0.0).abs() < 1e-5);
        assert!((inverse_lerp(0.0, 10.0, 10.0) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_smoothstep() {
        assert!((smoothstep(0.0, 1.0, 0.0) - 0.0).abs() < 1e-5);
        assert!((smoothstep(0.0, 1.0, 1.0) - 1.0).abs() < 1e-5);
        assert!((smoothstep(0.0, 1.0, 0.5) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn test_easing_endpoints() {
        // All easing functions should return 0 at t=0 and 1 at t=1
        let funcs: &[fn(f32) -> f32] = &[
            ease::in_sine,
            ease::out_sine,
            ease::in_out_sine,
            ease::in_quad,
            ease::out_quad,
            ease::in_out_quad,
            ease::in_cubic,
            ease::out_cubic,
            ease::in_out_cubic,
        ];

        for f in funcs {
            assert!((f(0.0) - 0.0).abs() < 1e-5, "f(0) should be 0");
            assert!((f(1.0) - 1.0).abs() < 1e-5, "f(1) should be 1");
        }
    }

    #[test]
    fn test_bezier_endpoints() {
        let p0 = Vec2::new(0.0, 0.0);
        let p1 = Vec2::new(0.5, 1.0);
        let p2 = Vec2::new(1.0, 0.0);

        let start = bezier::quadratic(p0, p1, p2, 0.0);
        let end = bezier::quadratic(p0, p1, p2, 1.0);

        assert!((start.x - p0.x).abs() < 1e-5);
        assert!((start.y - p0.y).abs() < 1e-5);
        assert!((end.x - p2.x).abs() < 1e-5);
        assert!((end.y - p2.y).abs() < 1e-5);
    }
}
