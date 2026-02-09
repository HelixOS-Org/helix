//! Gradient Episodic Memory (GEM) implementation.

extern crate alloc;

use alloc::vec::Vec;

/// GEM constraint for gradient projection
pub struct GemConstraint {
    /// Reference gradients per task
    pub reference_grads: Vec<Vec<f64>>,
    /// Task IDs
    pub task_ids: Vec<u64>,
    /// Memory budget per task
    pub memory_budget: usize,
    /// Constraint margin
    pub margin: f64,
}

impl GemConstraint {
    /// Create a new GEM constraint
    pub fn new(memory_budget: usize, margin: f64) -> Self {
        Self {
            reference_grads: Vec::new(),
            task_ids: Vec::new(),
            memory_budget,
            margin,
        }
    }

    /// Add reference gradient for a task
    #[inline(always)]
    pub fn add_reference(&mut self, task_id: u64, gradient: Vec<f64>) {
        self.task_ids.push(task_id);
        self.reference_grads.push(gradient);
    }

    /// Check if gradient violates any constraint
    pub fn violates(&self, gradient: &[f64]) -> Vec<(usize, f64)> {
        let mut violations = Vec::new();

        for (i, ref_grad) in self.reference_grads.iter().enumerate() {
            // Compute dot product
            let dot: f64 = gradient
                .iter()
                .zip(ref_grad.iter())
                .map(|(g, r)| g * r)
                .sum();

            if dot < -self.margin {
                violations.push((i, dot));
            }
        }

        violations
    }

    /// Project gradient to satisfy constraints (simplified)
    pub fn project(&self, gradient: &[f64]) -> Vec<f64> {
        let violations = self.violates(gradient);

        if violations.is_empty() {
            return gradient.to_vec();
        }

        let mut projected = gradient.to_vec();

        // Simple projection: subtract component along violating reference gradients
        for (task_idx, _) in violations {
            let ref_grad = &self.reference_grads[task_idx];

            // Compute dot products
            let g_dot_r: f64 = projected
                .iter()
                .zip(ref_grad.iter())
                .map(|(g, r)| g * r)
                .sum();
            let r_dot_r: f64 = ref_grad.iter().map(|r| r * r).sum();

            if r_dot_r > 1e-10 {
                let scale = g_dot_r / r_dot_r;

                for (p, r) in projected.iter_mut().zip(ref_grad.iter()) {
                    *p -= scale * r;
                }
            }
        }

        projected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gem_constraint() {
        let mut gem = GemConstraint::new(100, 0.0);

        gem.add_reference(0, vec![1.0, 0.0, 0.0]);

        let gradient = vec![-0.5, 0.5, 0.5];
        let violations = gem.violates(&gradient);

        assert!(!violations.is_empty());

        let projected = gem.project(&gradient);
        let new_violations = gem.violates(&projected);
        assert!(new_violations.is_empty() || new_violations[0].1 >= -0.1);
    }
}
