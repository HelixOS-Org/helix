//! Model update (gradient or delta) representation.

use alloc::vec::Vec;

/// Model update (gradient or delta)
#[derive(Debug, Clone)]
pub struct ModelUpdate {
    /// Update vector
    pub delta: Vec<f64>,
    /// Client ID
    pub client_id: u32,
    /// Number of samples used
    pub num_samples: usize,
    /// Training loss
    pub loss: f64,
    /// Update timestamp
    pub timestamp: u64,
}

impl ModelUpdate {
    /// Create a new update
    pub fn new(delta: Vec<f64>, client_id: u32, num_samples: usize) -> Self {
        Self {
            delta,
            client_id,
            num_samples,
            loss: 0.0,
            timestamp: 0,
        }
    }

    /// Update norm
    #[inline(always)]
    pub fn norm(&self) -> f64 {
        libm::sqrt(self.delta.iter().map(|x| x * x).sum())
    }

    /// Clip update to bound
    #[inline]
    pub fn clip(&mut self, bound: f64) {
        let norm = self.norm();
        if norm > bound {
            let scale = bound / norm;
            for d in &mut self.delta {
                *d *= scale;
            }
        }
    }
}
