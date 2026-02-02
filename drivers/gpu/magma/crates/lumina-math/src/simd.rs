//! SIMD optimizations for vector and matrix operations
//!
//! This module provides optimized implementations using x86_64 SIMD
//! (SSE/AVX) instructions when available.

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
use core::arch::x86_64::*;

use crate::mat::Mat4;
use crate::vec::{Vec3, Vec4};

/// SIMD-optimized 4-component vector using __m128
#[cfg(all(target_arch = "x86_64", feature = "simd"))]
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct SimdVec4(pub(crate) __m128);

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
impl SimdVec4 {
    /// Creates a new SIMD vector
    #[inline]
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        unsafe { Self(_mm_set_ps(w, z, y, x)) }
    }

    /// Creates a vector with all components set to zero
    #[inline]
    pub fn zero() -> Self {
        unsafe { Self(_mm_setzero_ps()) }
    }

    /// Creates a vector with all components set to the same value
    #[inline]
    pub fn splat(v: f32) -> Self {
        unsafe { Self(_mm_set1_ps(v)) }
    }

    /// Loads from a Vec4
    #[inline]
    pub fn from_vec4(v: Vec4) -> Self {
        unsafe { Self(_mm_loadu_ps(&v.x as *const f32)) }
    }

    /// Stores to a Vec4
    #[inline]
    pub fn to_vec4(self) -> Vec4 {
        let mut result = Vec4::ZERO;
        unsafe {
            _mm_storeu_ps(&mut result.x as *mut f32, self.0);
        }
        result
    }

    /// Gets the X component
    #[inline]
    pub fn x(self) -> f32 {
        unsafe { _mm_cvtss_f32(self.0) }
    }

    /// Gets the Y component
    #[inline]
    pub fn y(self) -> f32 {
        unsafe { _mm_cvtss_f32(_mm_shuffle_ps(self.0, self.0, 0b01_01_01_01)) }
    }

    /// Gets the Z component
    #[inline]
    pub fn z(self) -> f32 {
        unsafe { _mm_cvtss_f32(_mm_shuffle_ps(self.0, self.0, 0b10_10_10_10)) }
    }

    /// Gets the W component
    #[inline]
    pub fn w(self) -> f32 {
        unsafe { _mm_cvtss_f32(_mm_shuffle_ps(self.0, self.0, 0b11_11_11_11)) }
    }

    /// Adds two vectors
    #[inline]
    pub fn add(self, other: Self) -> Self {
        unsafe { Self(_mm_add_ps(self.0, other.0)) }
    }

    /// Subtracts two vectors
    #[inline]
    pub fn sub(self, other: Self) -> Self {
        unsafe { Self(_mm_sub_ps(self.0, other.0)) }
    }

    /// Multiplies two vectors element-wise
    #[inline]
    pub fn mul(self, other: Self) -> Self {
        unsafe { Self(_mm_mul_ps(self.0, other.0)) }
    }

    /// Divides two vectors element-wise
    #[inline]
    pub fn div(self, other: Self) -> Self {
        unsafe { Self(_mm_div_ps(self.0, other.0)) }
    }

    /// Negates the vector
    #[inline]
    pub fn neg(self) -> Self {
        unsafe { Self(_mm_sub_ps(_mm_setzero_ps(), self.0)) }
    }

    /// Computes the dot product
    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        unsafe {
            let mul = _mm_mul_ps(self.0, other.0);
            let sum1 = _mm_hadd_ps(mul, mul);
            let sum2 = _mm_hadd_ps(sum1, sum1);
            _mm_cvtss_f32(sum2)
        }
    }

    /// Computes the dot product, returning a SIMD vector with the result in all lanes
    #[inline]
    pub fn dot_splat(self, other: Self) -> Self {
        unsafe {
            let mul = _mm_mul_ps(self.0, other.0);
            let sum1 = _mm_hadd_ps(mul, mul);
            let sum2 = _mm_hadd_ps(sum1, sum1);
            Self(sum2)
        }
    }

    /// Computes the squared length
    #[inline]
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    /// Computes the length
    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Normalizes the vector
    #[inline]
    pub fn normalize(self) -> Self {
        let len = Self::splat(self.length());
        self.div(len)
    }

    /// Fast approximate normalization using rsqrt
    #[inline]
    pub fn normalize_fast(self) -> Self {
        unsafe {
            let len_sq = self.dot_splat(self);
            let inv_len = Self(_mm_rsqrt_ps(len_sq.0));
            self.mul(inv_len)
        }
    }

    /// Component-wise minimum
    #[inline]
    pub fn min(self, other: Self) -> Self {
        unsafe { Self(_mm_min_ps(self.0, other.0)) }
    }

    /// Component-wise maximum
    #[inline]
    pub fn max(self, other: Self) -> Self {
        unsafe { Self(_mm_max_ps(self.0, other.0)) }
    }

    /// Component-wise absolute value
    #[inline]
    pub fn abs(self) -> Self {
        unsafe {
            let mask = _mm_set1_ps(f32::from_bits(0x7FFFFFFF));
            Self(_mm_and_ps(self.0, mask))
        }
    }

    /// Linear interpolation
    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        let t_vec = Self::splat(t);
        let diff = other.sub(self);
        self.add(diff.mul(t_vec))
    }

    /// Clamps components to a range
    #[inline]
    pub fn clamp(self, min: Self, max: Self) -> Self {
        self.max(min).min(max)
    }
}

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
impl core::ops::Add for SimdVec4 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        SimdVec4::add(self, rhs)
    }
}

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
impl core::ops::Sub for SimdVec4 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        SimdVec4::sub(self, rhs)
    }
}

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
impl core::ops::Mul for SimdVec4 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        SimdVec4::mul(self, rhs)
    }
}

/// SIMD-optimized 4x4 matrix using four __m128 vectors
#[cfg(all(target_arch = "x86_64", feature = "simd"))]
#[derive(Clone, Copy)]
#[repr(C)]
pub struct SimdMat4 {
    pub(crate) cols: [__m128; 4],
}

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
impl SimdMat4 {
    /// Creates an identity matrix
    #[inline]
    pub fn identity() -> Self {
        unsafe {
            Self {
                cols: [
                    _mm_set_ps(0.0, 0.0, 0.0, 1.0),
                    _mm_set_ps(0.0, 0.0, 1.0, 0.0),
                    _mm_set_ps(0.0, 1.0, 0.0, 0.0),
                    _mm_set_ps(1.0, 0.0, 0.0, 0.0),
                ],
            }
        }
    }

    /// Loads from a Mat4
    #[inline]
    pub fn from_mat4(m: &Mat4) -> Self {
        unsafe {
            Self {
                cols: [
                    _mm_loadu_ps(m.m.as_ptr()),
                    _mm_loadu_ps(m.m.as_ptr().add(4)),
                    _mm_loadu_ps(m.m.as_ptr().add(8)),
                    _mm_loadu_ps(m.m.as_ptr().add(12)),
                ],
            }
        }
    }

    /// Stores to a Mat4
    #[inline]
    pub fn to_mat4(self) -> Mat4 {
        let mut result = Mat4::IDENTITY;
        unsafe {
            _mm_storeu_ps(result.m.as_mut_ptr(), self.cols[0]);
            _mm_storeu_ps(result.m.as_mut_ptr().add(4), self.cols[1]);
            _mm_storeu_ps(result.m.as_mut_ptr().add(8), self.cols[2]);
            _mm_storeu_ps(result.m.as_mut_ptr().add(12), self.cols[3]);
        }
        result
    }

    /// Matrix-vector multiplication
    #[inline]
    pub fn mul_vec4(self, v: SimdVec4) -> SimdVec4 {
        unsafe {
            // Broadcast each component
            let xxxx = _mm_shuffle_ps(v.0, v.0, 0b00_00_00_00);
            let yyyy = _mm_shuffle_ps(v.0, v.0, 0b01_01_01_01);
            let zzzz = _mm_shuffle_ps(v.0, v.0, 0b10_10_10_10);
            let wwww = _mm_shuffle_ps(v.0, v.0, 0b11_11_11_11);

            // Multiply and add
            let c0 = _mm_mul_ps(self.cols[0], xxxx);
            let c1 = _mm_mul_ps(self.cols[1], yyyy);
            let c2 = _mm_mul_ps(self.cols[2], zzzz);
            let c3 = _mm_mul_ps(self.cols[3], wwww);

            let sum01 = _mm_add_ps(c0, c1);
            let sum23 = _mm_add_ps(c2, c3);
            SimdVec4(_mm_add_ps(sum01, sum23))
        }
    }

    /// Matrix-matrix multiplication
    #[inline]
    pub fn mul_mat4(self, other: Self) -> Self {
        unsafe {
            let mut result = [_mm_setzero_ps(); 4];

            for i in 0..4 {
                let xxxx = _mm_shuffle_ps(other.cols[i], other.cols[i], 0b00_00_00_00);
                let yyyy = _mm_shuffle_ps(other.cols[i], other.cols[i], 0b01_01_01_01);
                let zzzz = _mm_shuffle_ps(other.cols[i], other.cols[i], 0b10_10_10_10);
                let wwww = _mm_shuffle_ps(other.cols[i], other.cols[i], 0b11_11_11_11);

                let c0 = _mm_mul_ps(self.cols[0], xxxx);
                let c1 = _mm_mul_ps(self.cols[1], yyyy);
                let c2 = _mm_mul_ps(self.cols[2], zzzz);
                let c3 = _mm_mul_ps(self.cols[3], wwww);

                let sum01 = _mm_add_ps(c0, c1);
                let sum23 = _mm_add_ps(c2, c3);
                result[i] = _mm_add_ps(sum01, sum23);
            }

            Self { cols: result }
        }
    }

    /// Transposes the matrix
    #[inline]
    pub fn transpose(self) -> Self {
        unsafe {
            let t0 = _mm_unpacklo_ps(self.cols[0], self.cols[1]);
            let t1 = _mm_unpackhi_ps(self.cols[0], self.cols[1]);
            let t2 = _mm_unpacklo_ps(self.cols[2], self.cols[3]);
            let t3 = _mm_unpackhi_ps(self.cols[2], self.cols[3]);

            Self {
                cols: [
                    _mm_movelh_ps(t0, t2),
                    _mm_movehl_ps(t2, t0),
                    _mm_movelh_ps(t1, t3),
                    _mm_movehl_ps(t3, t1),
                ],
            }
        }
    }

    /// Adds two matrices
    #[inline]
    pub fn add(self, other: Self) -> Self {
        unsafe {
            Self {
                cols: [
                    _mm_add_ps(self.cols[0], other.cols[0]),
                    _mm_add_ps(self.cols[1], other.cols[1]),
                    _mm_add_ps(self.cols[2], other.cols[2]),
                    _mm_add_ps(self.cols[3], other.cols[3]),
                ],
            }
        }
    }

    /// Subtracts two matrices
    #[inline]
    pub fn sub(self, other: Self) -> Self {
        unsafe {
            Self {
                cols: [
                    _mm_sub_ps(self.cols[0], other.cols[0]),
                    _mm_sub_ps(self.cols[1], other.cols[1]),
                    _mm_sub_ps(self.cols[2], other.cols[2]),
                    _mm_sub_ps(self.cols[3], other.cols[3]),
                ],
            }
        }
    }

    /// Scales the matrix by a scalar
    #[inline]
    pub fn scale(self, s: f32) -> Self {
        unsafe {
            let scalar = _mm_set1_ps(s);
            Self {
                cols: [
                    _mm_mul_ps(self.cols[0], scalar),
                    _mm_mul_ps(self.cols[1], scalar),
                    _mm_mul_ps(self.cols[2], scalar),
                    _mm_mul_ps(self.cols[3], scalar),
                ],
            }
        }
    }
}

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
impl core::ops::Mul for SimdMat4 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        self.mul_mat4(rhs)
    }
}

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
impl core::ops::Mul<SimdVec4> for SimdMat4 {
    type Output = SimdVec4;
    fn mul(self, rhs: SimdVec4) -> SimdVec4 {
        self.mul_vec4(rhs)
    }
}

/// SIMD-optimized cross product for Vec3
#[cfg(all(target_arch = "x86_64", feature = "simd"))]
#[inline]
pub fn cross3_simd(a: Vec3, b: Vec3) -> Vec3 {
    unsafe {
        let a_vec = _mm_set_ps(0.0, a.z, a.y, a.x);
        let b_vec = _mm_set_ps(0.0, b.z, b.y, b.x);

        // a.yzx * b.zxy - a.zxy * b.yzx
        let a_yzx = _mm_shuffle_ps(a_vec, a_vec, 0b11_00_10_01);
        let b_zxy = _mm_shuffle_ps(b_vec, b_vec, 0b11_01_00_10);
        let a_zxy = _mm_shuffle_ps(a_vec, a_vec, 0b11_01_00_10);
        let b_yzx = _mm_shuffle_ps(b_vec, b_vec, 0b11_00_10_01);

        let term1 = _mm_mul_ps(a_yzx, b_zxy);
        let term2 = _mm_mul_ps(a_zxy, b_yzx);
        let result = _mm_sub_ps(term1, term2);

        let mut out = [0.0f32; 4];
        _mm_storeu_ps(out.as_mut_ptr(), result);
        Vec3::new(out[0], out[1], out[2])
    }
}

/// SIMD-optimized dot product for Vec4
#[cfg(all(target_arch = "x86_64", feature = "simd"))]
#[inline]
pub fn dot4_simd(a: Vec4, b: Vec4) -> f32 {
    unsafe {
        let a_vec = _mm_loadu_ps(&a.x as *const f32);
        let b_vec = _mm_loadu_ps(&b.x as *const f32);
        let mul = _mm_mul_ps(a_vec, b_vec);
        let sum1 = _mm_hadd_ps(mul, mul);
        let sum2 = _mm_hadd_ps(sum1, sum1);
        _mm_cvtss_f32(sum2)
    }
}

/// SIMD-optimized matrix-vector multiplication
#[cfg(all(target_arch = "x86_64", feature = "simd"))]
#[inline]
pub fn mat4_mul_vec4_simd(m: &Mat4, v: Vec4) -> Vec4 {
    let simd_mat = SimdMat4::from_mat4(m);
    let simd_vec = SimdVec4::from_vec4(v);
    simd_mat.mul_vec4(simd_vec).to_vec4()
}

/// SIMD-optimized matrix-matrix multiplication
#[cfg(all(target_arch = "x86_64", feature = "simd"))]
#[inline]
pub fn mat4_mul_mat4_simd(a: &Mat4, b: &Mat4) -> Mat4 {
    let simd_a = SimdMat4::from_mat4(a);
    let simd_b = SimdMat4::from_mat4(b);
    simd_a.mul_mat4(simd_b).to_mat4()
}

// Fallback implementations for non-x86_64 or non-SIMD
#[cfg(not(all(target_arch = "x86_64", feature = "simd")))]
pub use crate::mat::Mat4 as SimdMat4;
#[cfg(not(all(target_arch = "x86_64", feature = "simd")))]
pub use crate::vec::Vec4 as SimdVec4;

#[cfg(not(all(target_arch = "x86_64", feature = "simd")))]
#[inline]
pub fn cross3_simd(a: Vec3, b: Vec3) -> Vec3 {
    a.cross(b)
}

#[cfg(not(all(target_arch = "x86_64", feature = "simd")))]
#[inline]
pub fn dot4_simd(a: Vec4, b: Vec4) -> f32 {
    a.dot(b)
}

#[cfg(not(all(target_arch = "x86_64", feature = "simd")))]
#[inline]
pub fn mat4_mul_vec4_simd(m: &Mat4, v: Vec4) -> Vec4 {
    *m * v
}

#[cfg(not(all(target_arch = "x86_64", feature = "simd")))]
#[inline]
pub fn mat4_mul_mat4_simd(a: &Mat4, b: &Mat4) -> Mat4 {
    *a * *b
}

/// Batch transforms multiple points using SIMD
#[inline]
pub fn batch_transform_points(matrix: &Mat4, points: &[Vec3], output: &mut [Vec3]) {
    assert_eq!(points.len(), output.len());

    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    {
        let simd_mat = SimdMat4::from_mat4(matrix);
        for (i, point) in points.iter().enumerate() {
            let v = SimdVec4::new(point.x, point.y, point.z, 1.0);
            let result = simd_mat.mul_vec4(v);
            output[i] = Vec3::new(result.x(), result.y(), result.z());
        }
    }

    #[cfg(not(all(target_arch = "x86_64", feature = "simd")))]
    {
        for (i, point) in points.iter().enumerate() {
            let v = Vec4::new(point.x, point.y, point.z, 1.0);
            let result = *matrix * v;
            output[i] = result.xyz();
        }
    }
}

/// Batch transforms multiple vectors (no translation) using SIMD
#[inline]
pub fn batch_transform_vectors(matrix: &Mat4, vectors: &[Vec3], output: &mut [Vec3]) {
    assert_eq!(vectors.len(), output.len());

    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    {
        let simd_mat = SimdMat4::from_mat4(matrix);
        for (i, vector) in vectors.iter().enumerate() {
            let v = SimdVec4::new(vector.x, vector.y, vector.z, 0.0);
            let result = simd_mat.mul_vec4(v);
            output[i] = Vec3::new(result.x(), result.y(), result.z());
        }
    }

    #[cfg(not(all(target_arch = "x86_64", feature = "simd")))]
    {
        for (i, vector) in vectors.iter().enumerate() {
            let v = Vec4::new(vector.x, vector.y, vector.z, 0.0);
            let result = *matrix * v;
            output[i] = result.xyz();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_transform_identity() {
        let m = Mat4::IDENTITY;
        let points = [Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, 5.0, 6.0)];
        let mut output = [Vec3::ZERO; 2];
        batch_transform_points(&m, &points, &mut output);

        for (i, p) in points.iter().enumerate() {
            assert!((output[i].x - p.x).abs() < 1e-5);
            assert!((output[i].y - p.y).abs() < 1e-5);
            assert!((output[i].z - p.z).abs() < 1e-5);
        }
    }
}
