//! Fast Gradient Sign Method (FGSM) attack implementation.

use crate::adversarial::perturbation::Perturbation;
use crate::adversarial::types::PerturbationType;
use crate::adversarial::utils::sign;

/// Fast Gradient Sign Method (FGSM)
#[derive(Debug, Clone)]
pub struct FGSM {
    /// Perturbation budget
    pub epsilon: f64,
    /// Perturbation type
    pub pert_type: PerturbationType,
}

impl FGSM {
    /// Create a new FGSM attacker
    pub fn new(epsilon: f64) -> Self {
        Self {
            epsilon,
            pert_type: PerturbationType::LInf,
        }
    }

    /// Generate adversarial example
    pub fn attack(&self, input: &[f64], gradient: &[f64]) -> Perturbation {
        let mut perturbation = Perturbation::new(input.len(), self.pert_type, self.epsilon);

        for (d, &g) in perturbation.delta.iter_mut().zip(gradient.iter()) {
            *d = self.epsilon * sign(g);
        }

        perturbation.iterations = 1;
        perturbation
    }
}
