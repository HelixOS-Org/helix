//! Adversarial perturbation representation and operations.

use alloc::vec;
use alloc::vec::Vec;

use crate::adversarial::types::PerturbationType;

/// Adversarial perturbation
#[derive(Debug, Clone)]
pub struct Perturbation {
    /// Perturbation vector
    pub delta: Vec<f64>,
    /// Perturbation type
    pub pert_type: PerturbationType,
    /// Magnitude
    pub epsilon: f64,
    /// Success indicator
    pub success: bool,
    /// Number of iterations used
    pub iterations: usize,
}

impl Perturbation {
    /// Create a new perturbation
    pub fn new(dim: usize, pert_type: PerturbationType, epsilon: f64) -> Self {
        Self {
            delta: vec![0.0; dim],
            pert_type,
            epsilon,
            success: false,
            iterations: 0,
        }
    }

    /// Get L-infinity norm
    #[inline]
    pub fn linf_norm(&self) -> f64 {
        self.delta
            .iter()
            .map(|&x| libm::fabs(x))
            .fold(0.0, f64::max)
    }

    /// Get L2 norm
    #[inline(always)]
    pub fn l2_norm(&self) -> f64 {
        let sum_sq: f64 = self.delta.iter().map(|x| x * x).sum();
        libm::sqrt(sum_sq)
    }

    /// Get L1 norm
    #[inline(always)]
    pub fn l1_norm(&self) -> f64 {
        self.delta.iter().map(|&x| libm::fabs(x)).sum()
    }

    /// Project onto epsilon ball
    pub fn project(&mut self) {
        match self.pert_type {
            PerturbationType::LInf => {
                for d in &mut self.delta {
                    *d = d.clamp(-self.epsilon, self.epsilon);
                }
            }
            PerturbationType::L2 => {
                let norm = self.l2_norm();
                if norm > self.epsilon {
                    for d in &mut self.delta {
                        *d *= self.epsilon / norm;
                    }
                }
            }
            PerturbationType::L1 => {
                let norm = self.l1_norm();
                if norm > self.epsilon {
                    // Simplex projection (simplified)
                    for d in &mut self.delta {
                        *d *= self.epsilon / norm;
                    }
                }
            }
            _ => {}
        }
    }

    /// Apply perturbation to input
    #[inline]
    pub fn apply(&self, input: &[f64]) -> Vec<f64> {
        input
            .iter()
            .zip(self.delta.iter())
            .map(|(&x, &d)| x + d)
            .collect()
    }
}
