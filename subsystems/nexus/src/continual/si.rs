//! Synaptic Intelligence (SI) implementation.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

/// Online importance estimation for SI
pub struct SynapticIntelligence {
    /// Omega values (accumulated importance)
    pub omega: Vec<f64>,
    /// Running sum of gradients * parameter change
    pub path_integral: Vec<f64>,
    /// Previous parameter values
    pub prev_params: Vec<f64>,
    /// Damping factor
    pub damping: f64,
    /// SI strength
    pub c: f64,
}

impl SynapticIntelligence {
    /// Create a new SI learner
    pub fn new(param_count: usize, c: f64) -> Self {
        Self {
            omega: vec![0.0; param_count],
            path_integral: vec![0.0; param_count],
            prev_params: vec![0.0; param_count],
            damping: 0.1,
            c,
        }
    }

    /// Initialize with starting parameters
    #[inline]
    pub fn init_params(&mut self, params: &[f64]) {
        self.prev_params = params.to_vec();
        for pi in &mut self.path_integral {
            *pi = 0.0;
        }
    }

    /// Update after a training step
    #[inline]
    pub fn update_step(&mut self, params: &[f64], gradients: &[f64]) {
        for (i, (&p, &prev)) in params.iter().zip(self.prev_params.iter()).enumerate() {
            let delta = p - prev;
            if i < gradients.len() && i < self.path_integral.len() {
                // Accumulate gradient * delta
                self.path_integral[i] += -gradients[i] * delta;
            }
        }

        self.prev_params = params.to_vec();
    }

    /// Consolidate at task boundary
    pub fn consolidate(&mut self, final_params: &[f64], initial_params: &[f64]) {
        for (i, (&final_p, &init_p)) in final_params.iter().zip(initial_params.iter()).enumerate() {
            let delta_sq = (final_p - init_p).powi(2);

            if delta_sq > 1e-10 && i < self.path_integral.len() && i < self.omega.len() {
                // Normalize path integral by parameter change
                let omega_new = self.path_integral[i] / (delta_sq + self.damping);
                self.omega[i] += omega_new.max(0.0);
            }
        }

        // Reset path integral for next task
        for pi in &mut self.path_integral {
            *pi = 0.0;
        }
    }

    /// Compute SI penalty
    pub fn penalty(&self, current_params: &[f64], reference_params: &[f64]) -> f64 {
        let mut penalty = 0.0;

        for (i, (&curr, &ref_p)) in current_params
            .iter()
            .zip(reference_params.iter())
            .enumerate()
        {
            if i < self.omega.len() {
                let diff = curr - ref_p;
                penalty += self.omega[i] * diff * diff;
            }
        }

        0.5 * self.c * penalty
    }

    /// Get importance-weighted gradient
    pub fn weighted_gradient(
        &self,
        current_params: &[f64],
        reference_params: &[f64],
        task_grad: &[f64],
    ) -> Vec<f64> {
        let mut grad = task_grad.to_vec();

        for (i, (g, (&curr, &ref_p))) in grad
            .iter_mut()
            .zip(current_params.iter().zip(reference_params.iter()))
            .enumerate()
        {
            if i < self.omega.len() {
                *g += self.c * self.omega[i] * (curr - ref_p);
            }
        }

        grad
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synaptic_intelligence() {
        let mut si = SynapticIntelligence::new(3, 1.0);

        si.init_params(&[0.0, 0.0, 0.0]);
        si.update_step(&[0.1, 0.1, 0.1], &[0.5, 0.5, 0.5]);
        si.update_step(&[0.2, 0.2, 0.2], &[0.4, 0.4, 0.4]);

        si.consolidate(&[0.2, 0.2, 0.2], &[0.0, 0.0, 0.0]);

        let penalty = si.penalty(&[0.3, 0.3, 0.3], &[0.2, 0.2, 0.2]);
        assert!(penalty >= 0.0);
    }
}
