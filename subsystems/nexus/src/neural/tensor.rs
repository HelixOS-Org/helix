//! NEXUS Year 2: Tensor Operations
//!
//! Efficient tensor data structures for neural network computations.
//! Pure Rust, no_std compatible.

#![allow(dead_code)]

use alloc::boxed::Box;
use alloc::vec::Vec;

// ============================================================================
// Tensor Shape
// ============================================================================

/// Maximum number of dimensions supported
pub const MAX_DIMS: usize = 4;

/// Shape of a tensor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TensorShape {
    dims: [usize; MAX_DIMS],
    ndim: usize,
}

impl TensorShape {
    pub fn scalar() -> Self {
        Self {
            dims: [1, 1, 1, 1],
            ndim: 0,
        }
    }

    pub fn vector(size: usize) -> Self {
        Self {
            dims: [size, 1, 1, 1],
            ndim: 1,
        }
    }

    pub fn matrix(rows: usize, cols: usize) -> Self {
        Self {
            dims: [rows, cols, 1, 1],
            ndim: 2,
        }
    }

    pub fn tensor3d(d0: usize, d1: usize, d2: usize) -> Self {
        Self {
            dims: [d0, d1, d2, 1],
            ndim: 3,
        }
    }

    pub fn tensor4d(d0: usize, d1: usize, d2: usize, d3: usize) -> Self {
        Self {
            dims: [d0, d1, d2, d3],
            ndim: 4,
        }
    }

    pub fn from_slice(dims: &[usize]) -> Self {
        let mut shape = Self {
            dims: [1; MAX_DIMS],
            ndim: dims.len().min(MAX_DIMS),
        };
        for (i, &d) in dims.iter().take(MAX_DIMS).enumerate() {
            shape.dims[i] = d;
        }
        shape
    }

    pub fn ndim(&self) -> usize {
        self.ndim
    }

    pub fn dim(&self, i: usize) -> usize {
        if i < self.ndim { self.dims[i] } else { 1 }
    }

    pub fn total_elements(&self) -> usize {
        self.dims[..self.ndim.max(1)].iter().product()
    }

    pub fn strides(&self) -> [usize; MAX_DIMS] {
        let mut strides = [1; MAX_DIMS];
        for i in (0..self.ndim.saturating_sub(1)).rev() {
            strides[i] = strides[i + 1] * self.dims[i + 1];
        }
        strides
    }

    pub fn is_compatible_for_matmul(&self, other: &TensorShape) -> bool {
        if self.ndim < 2 || other.ndim < 2 {
            return false;
        }
        self.dims[self.ndim - 1] == other.dims[other.ndim - 2]
    }

    pub fn matmul_result_shape(&self, other: &TensorShape) -> Option<TensorShape> {
        if !self.is_compatible_for_matmul(other) {
            return None;
        }

        Some(TensorShape::matrix(
            self.dims[self.ndim - 2],
            other.dims[other.ndim - 1],
        ))
    }
}

// ============================================================================
// Tensor
// ============================================================================

/// A multi-dimensional array of floating point values
#[derive(Debug, Clone)]
pub struct Tensor {
    data: Vec<f32>,
    shape: TensorShape,
}

impl Tensor {
    /// Create a new tensor with zeros
    pub fn zeros(shape: TensorShape) -> Self {
        let size = shape.total_elements();
        Self {
            data: alloc::vec![0.0; size],
            shape,
        }
    }

    /// Create a new tensor with ones
    pub fn ones(shape: TensorShape) -> Self {
        let size = shape.total_elements();
        Self {
            data: alloc::vec![1.0; size],
            shape,
        }
    }

    /// Create a tensor filled with a value
    pub fn full(shape: TensorShape, value: f32) -> Self {
        let size = shape.total_elements();
        Self {
            data: alloc::vec![value; size],
            shape,
        }
    }

    /// Create a tensor from data
    pub fn from_data(shape: TensorShape, data: Vec<f32>) -> Self {
        debug_assert_eq!(data.len(), shape.total_elements());
        Self { data, shape }
    }

    /// Create a 1D tensor from slice
    pub fn from_slice(data: &[f32]) -> Self {
        Self {
            data: data.to_vec(),
            shape: TensorShape::vector(data.len()),
        }
    }

    /// Create a random tensor (using simple LCG)
    pub fn random(shape: TensorShape, seed: u64) -> Self {
        let size = shape.total_elements();
        let mut data = Vec::with_capacity(size);
        let mut rng = seed;

        for _ in 0..size {
            rng = rng
                .wrapping_mul(0x5851f42d4c957f2d)
                .wrapping_add(0x14057b7ef767814f);
            rng ^= rng >> 33;
            // Map to [-1, 1]
            let value = ((rng as i64) as f32) / (i64::MAX as f32);
            data.push(value);
        }

        Self { data, shape }
    }

    /// Create random tensor with Xavier initialization
    pub fn xavier(shape: TensorShape, seed: u64) -> Self {
        let mut tensor = Self::random(shape, seed);
        let fan_in = if shape.ndim() >= 2 {
            shape.dim(shape.ndim() - 1)
        } else {
            1
        };
        let fan_out = if shape.ndim() >= 2 {
            shape.dim(shape.ndim() - 2)
        } else {
            1
        };
        let scale = libm::sqrtf(2.0 / (fan_in + fan_out) as f32);

        for v in &mut tensor.data {
            *v *= scale;
        }

        tensor
    }

    pub fn shape(&self) -> &TensorShape {
        &self.shape
    }

    pub fn data(&self) -> &[f32] {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut [f32] {
        &mut self.data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get element at index
    pub fn get(&self, indices: &[usize]) -> Option<f32> {
        let idx = self.flat_index(indices)?;
        self.data.get(idx).copied()
    }

    /// Set element at index
    pub fn set(&mut self, indices: &[usize], value: f32) -> bool {
        if let Some(idx) = self.flat_index(indices) {
            if idx < self.data.len() {
                self.data[idx] = value;
                return true;
            }
        }
        false
    }

    fn flat_index(&self, indices: &[usize]) -> Option<usize> {
        if indices.len() != self.shape.ndim() {
            return None;
        }

        let strides = self.shape.strides();
        let mut idx = 0;

        for (i, &index) in indices.iter().enumerate() {
            if index >= self.shape.dim(i) {
                return None;
            }
            idx += index * strides[i];
        }

        Some(idx)
    }

    /// Reshape tensor (must have same total elements)
    pub fn reshape(&self, new_shape: TensorShape) -> Option<Self> {
        if new_shape.total_elements() != self.shape.total_elements() {
            return None;
        }

        Some(Self {
            data: self.data.clone(),
            shape: new_shape,
        })
    }

    /// Flatten to 1D
    pub fn flatten(&self) -> Self {
        Self {
            data: self.data.clone(),
            shape: TensorShape::vector(self.data.len()),
        }
    }

    // ========== Element-wise Operations ==========

    /// Element-wise addition
    pub fn add(&self, other: &Tensor) -> Option<Self> {
        if self.shape != other.shape {
            return None;
        }

        let data: Vec<f32> = self
            .data
            .iter()
            .zip(other.data.iter())
            .map(|(a, b)| a + b)
            .collect();

        Some(Self {
            data,
            shape: self.shape,
        })
    }

    /// Element-wise subtraction
    pub fn sub(&self, other: &Tensor) -> Option<Self> {
        if self.shape != other.shape {
            return None;
        }

        let data: Vec<f32> = self
            .data
            .iter()
            .zip(other.data.iter())
            .map(|(a, b)| a - b)
            .collect();

        Some(Self {
            data,
            shape: self.shape,
        })
    }

    /// Element-wise multiplication (Hadamard product)
    pub fn mul(&self, other: &Tensor) -> Option<Self> {
        if self.shape != other.shape {
            return None;
        }

        let data: Vec<f32> = self
            .data
            .iter()
            .zip(other.data.iter())
            .map(|(a, b)| a * b)
            .collect();

        Some(Self {
            data,
            shape: self.shape,
        })
    }

    /// Element-wise division
    pub fn div(&self, other: &Tensor) -> Option<Self> {
        if self.shape != other.shape {
            return None;
        }

        let data: Vec<f32> = self
            .data
            .iter()
            .zip(other.data.iter())
            .map(|(a, b)| if b.abs() > 1e-10 { a / b } else { 0.0 })
            .collect();

        Some(Self {
            data,
            shape: self.shape,
        })
    }

    /// Scalar addition
    pub fn add_scalar(&self, scalar: f32) -> Self {
        let data: Vec<f32> = self.data.iter().map(|x| x + scalar).collect();
        Self {
            data,
            shape: self.shape,
        }
    }

    /// Scalar multiplication
    pub fn mul_scalar(&self, scalar: f32) -> Self {
        let data: Vec<f32> = self.data.iter().map(|x| x * scalar).collect();
        Self {
            data,
            shape: self.shape,
        }
    }

    /// Element-wise negation
    pub fn neg(&self) -> Self {
        let data: Vec<f32> = self.data.iter().map(|x| -x).collect();
        Self {
            data,
            shape: self.shape,
        }
    }

    /// Element-wise absolute value
    pub fn abs(&self) -> Self {
        let data: Vec<f32> = self.data.iter().map(|x| x.abs()).collect();
        Self {
            data,
            shape: self.shape,
        }
    }

    /// Element-wise square
    pub fn square(&self) -> Self {
        let data: Vec<f32> = self.data.iter().map(|x| x * x).collect();
        Self {
            data,
            shape: self.shape,
        }
    }

    /// Element-wise square root
    pub fn sqrt(&self) -> Self {
        let data: Vec<f32> = self.data.iter().map(|x| libm::sqrtf(x.max(0.0))).collect();
        Self {
            data,
            shape: self.shape,
        }
    }

    /// Element-wise exponential
    pub fn exp(&self) -> Self {
        let data: Vec<f32> = self.data.iter().map(|x| libm::expf(*x)).collect();
        Self {
            data,
            shape: self.shape,
        }
    }

    /// Element-wise natural logarithm
    pub fn log(&self) -> Self {
        let data: Vec<f32> = self.data.iter().map(|x| libm::logf(x.max(1e-10))).collect();
        Self {
            data,
            shape: self.shape,
        }
    }

    /// Element-wise clamp
    pub fn clamp(&self, min: f32, max: f32) -> Self {
        let data: Vec<f32> = self.data.iter().map(|x| x.clamp(min, max)).collect();
        Self {
            data,
            shape: self.shape,
        }
    }

    // ========== Reduction Operations ==========

    /// Sum of all elements
    pub fn sum(&self) -> f32 {
        self.data.iter().sum()
    }

    /// Mean of all elements
    pub fn mean(&self) -> f32 {
        if self.data.is_empty() {
            0.0
        } else {
            self.sum() / self.data.len() as f32
        }
    }

    /// Maximum element
    pub fn max(&self) -> f32 {
        self.data.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
    }

    /// Minimum element
    pub fn min(&self) -> f32 {
        self.data.iter().cloned().fold(f32::INFINITY, f32::min)
    }

    /// Variance
    pub fn variance(&self) -> f32 {
        let mean = self.mean();
        let sq_diff: f32 = self.data.iter().map(|x| (x - mean) * (x - mean)).sum();
        sq_diff / self.data.len() as f32
    }

    /// Standard deviation
    pub fn std(&self) -> f32 {
        libm::sqrtf(self.variance())
    }

    /// L2 norm (Frobenius norm)
    pub fn norm(&self) -> f32 {
        let sq_sum: f32 = self.data.iter().map(|x| x * x).sum();
        libm::sqrtf(sq_sum)
    }

    /// Argmax - index of maximum element
    pub fn argmax(&self) -> usize {
        self.data
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(core::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    /// Argmin - index of minimum element
    pub fn argmin(&self) -> usize {
        self.data
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(core::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    // ========== Matrix Operations ==========

    /// Transpose a 2D tensor
    pub fn transpose(&self) -> Option<Self> {
        if self.shape.ndim() != 2 {
            return None;
        }

        let rows = self.shape.dim(0);
        let cols = self.shape.dim(1);
        let mut data = vec![0.0f32; rows * cols];

        for i in 0..rows {
            for j in 0..cols {
                data[j * rows + i] = self.data[i * cols + j];
            }
        }

        Some(Self {
            data,
            shape: TensorShape::matrix(cols, rows),
        })
    }

    /// Matrix multiplication
    pub fn matmul(&self, other: &Tensor) -> Option<Self> {
        if self.shape.ndim() != 2 || other.shape.ndim() != 2 {
            return None;
        }

        let m = self.shape.dim(0);
        let k = self.shape.dim(1);
        let n = other.shape.dim(1);

        if k != other.shape.dim(0) {
            return None;
        }

        let mut data = vec![0.0f32; m * n];

        for i in 0..m {
            for j in 0..n {
                let mut sum = 0.0f32;
                for l in 0..k {
                    sum += self.data[i * k + l] * other.data[l * n + j];
                }
                data[i * n + j] = sum;
            }
        }

        Some(Self {
            data,
            shape: TensorShape::matrix(m, n),
        })
    }

    /// Matrix-vector multiplication
    pub fn matvec(&self, vec: &Tensor) -> Option<Self> {
        if self.shape.ndim() != 2 || vec.shape.ndim() != 1 {
            return None;
        }

        let m = self.shape.dim(0);
        let n = self.shape.dim(1);

        if n != vec.shape.dim(0) {
            return None;
        }

        let mut data = vec![0.0f32; m];

        for i in 0..m {
            let mut sum = 0.0f32;
            for j in 0..n {
                sum += self.data[i * n + j] * vec.data[j];
            }
            data[i] = sum;
        }

        Some(Self {
            data,
            shape: TensorShape::vector(m),
        })
    }

    /// Dot product of two vectors
    pub fn dot(&self, other: &Tensor) -> Option<f32> {
        if self.shape.ndim() != 1 || other.shape.ndim() != 1 {
            return None;
        }

        if self.shape.dim(0) != other.shape.dim(0) {
            return None;
        }

        let dot: f32 = self
            .data
            .iter()
            .zip(other.data.iter())
            .map(|(a, b)| a * b)
            .sum();

        Some(dot)
    }

    /// Outer product of two vectors
    pub fn outer(&self, other: &Tensor) -> Option<Self> {
        if self.shape.ndim() != 1 || other.shape.ndim() != 1 {
            return None;
        }

        let m = self.shape.dim(0);
        let n = other.shape.dim(0);
        let mut data = vec![0.0f32; m * n];

        for i in 0..m {
            for j in 0..n {
                data[i * n + j] = self.data[i] * other.data[j];
            }
        }

        Some(Self {
            data,
            shape: TensorShape::matrix(m, n),
        })
    }

    // ========== Broadcasting ==========

    /// Add with broadcasting (other must be smaller)
    pub fn add_broadcast(&self, other: &Tensor) -> Self {
        let mut result = self.clone();

        // Simple broadcast: if other is 1D and matches last dimension
        if other.shape.ndim() == 1 && self.shape.dim(self.shape.ndim() - 1) == other.shape.dim(0) {
            let stride = other.len();
            for i in 0..self.len() {
                result.data[i] += other.data[i % stride];
            }
        }

        result
    }
}

// ============================================================================
// Tensor View (for slicing without copying)
// ============================================================================

/// A view into a tensor without copying data
pub struct TensorView<'a> {
    data: &'a [f32],
    shape: TensorShape,
    offset: usize,
    strides: [usize; MAX_DIMS],
}

impl<'a> TensorView<'a> {
    pub fn from_tensor(tensor: &'a Tensor) -> Self {
        Self {
            data: &tensor.data,
            shape: tensor.shape,
            offset: 0,
            strides: tensor.shape.strides(),
        }
    }

    pub fn shape(&self) -> &TensorShape {
        &self.shape
    }

    pub fn get(&self, indices: &[usize]) -> Option<f32> {
        if indices.len() != self.shape.ndim() {
            return None;
        }

        let mut idx = self.offset;
        for (i, &index) in indices.iter().enumerate() {
            if index >= self.shape.dim(i) {
                return None;
            }
            idx += index * self.strides[i];
        }

        self.data.get(idx).copied()
    }

    pub fn to_tensor(&self) -> Tensor {
        // Copy data from view
        let size = self.shape.total_elements();
        let mut data = Vec::with_capacity(size);

        // This is a simplified copy - full implementation would handle arbitrary slices
        for &v in &self.data[self.offset..self.offset + size] {
            data.push(v);
        }

        Tensor {
            data,
            shape: self.shape,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tensor_shape() {
        let shape = TensorShape::matrix(3, 4);
        assert_eq!(shape.ndim(), 2);
        assert_eq!(shape.dim(0), 3);
        assert_eq!(shape.dim(1), 4);
        assert_eq!(shape.total_elements(), 12);
    }

    #[test]
    fn test_tensor_creation() {
        let t = Tensor::zeros(TensorShape::vector(10));
        assert_eq!(t.len(), 10);
        assert_eq!(t.sum(), 0.0);

        let t = Tensor::ones(TensorShape::matrix(2, 3));
        assert_eq!(t.len(), 6);
        assert_eq!(t.sum(), 6.0);
    }

    #[test]
    fn test_tensor_operations() {
        let a = Tensor::from_slice(&[1.0, 2.0, 3.0]);
        let b = Tensor::from_slice(&[4.0, 5.0, 6.0]);

        let c = a.add(&b).unwrap();
        assert_eq!(c.data(), &[5.0, 7.0, 9.0]);

        let d = a.mul(&b).unwrap();
        assert_eq!(d.data(), &[4.0, 10.0, 18.0]);
    }

    #[test]
    fn test_tensor_matmul() {
        // 2x3 * 3x2 = 2x2
        let a = Tensor::from_data(TensorShape::matrix(2, 3), alloc::vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0
        ]);
        let b = Tensor::from_data(TensorShape::matrix(3, 2), alloc::vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0
        ]);

        let c = a.matmul(&b).unwrap();
        assert_eq!(c.shape().dim(0), 2);
        assert_eq!(c.shape().dim(1), 2);
    }

    #[test]
    fn test_tensor_reductions() {
        let t = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(t.sum(), 15.0);
        assert_eq!(t.mean(), 3.0);
        assert_eq!(t.max(), 5.0);
        assert_eq!(t.min(), 1.0);
        assert_eq!(t.argmax(), 4);
    }
}
