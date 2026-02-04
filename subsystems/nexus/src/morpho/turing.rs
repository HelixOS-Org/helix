//! Turing pattern generator (reaction-diffusion system).

extern crate alloc;

use super::field::MorphogenField;
use super::types::MorphogenType;

/// Turing pattern parameters
#[derive(Debug, Clone, Copy)]
pub struct TuringParams {
    /// Activator production rate
    pub alpha: f64,
    /// Inhibitor production rate
    pub beta: f64,
    /// Activator self-activation
    pub gamma: f64,
    /// Inhibitor effect on activator
    pub delta: f64,
    /// Activator effect on inhibitor
    pub epsilon: f64,
    /// Inhibitor self-decay
    pub zeta: f64,
}

impl Default for TuringParams {
    fn default() -> Self {
        Self {
            alpha: 1.0,
            beta: 0.5,
            gamma: 0.1,
            delta: 1.0,
            epsilon: 1.0,
            zeta: 0.1,
        }
    }
}

/// Turing pattern generator (reaction-diffusion)
#[derive(Debug, Clone)]
pub struct TuringPattern {
    /// Activator field
    pub(crate) activator: MorphogenField,
    /// Inhibitor field
    pub(crate) inhibitor: MorphogenField,
    /// Reaction parameters
    params: TuringParams,
}

impl TuringPattern {
    /// Create a new Turing pattern generator
    pub fn new(size: usize, params: TuringParams) -> Self {
        Self {
            activator: MorphogenField::new(size, MorphogenType::Activator).with_diffusion(0.01),
            inhibitor: MorphogenField::new(size, MorphogenType::Inhibitor).with_diffusion(0.05), // Inhibitor diffuses faster
            params,
        }
    }

    /// Initialize with random perturbation
    pub fn initialize_random(&mut self, rng: &mut u64) {
        for i in 0..self.activator.concentrations.len() {
            *rng ^= *rng << 13;
            *rng ^= *rng >> 7;
            *rng ^= *rng << 17;

            self.activator.concentrations[i] = 1.0 + 0.1 * ((*rng as f64 / u64::MAX as f64) - 0.5);

            *rng ^= *rng << 13;
            *rng ^= *rng >> 7;

            self.inhibitor.concentrations[i] = 1.0 + 0.1 * ((*rng as f64 / u64::MAX as f64) - 0.5);
        }
    }

    /// Simulate one time step
    pub fn step(&mut self, dt: f64) {
        let _size = self.activator.size;
        let p = &self.params;

        // Store old concentrations
        let old_a = self.activator.concentrations.clone();
        let old_b = self.inhibitor.concentrations.clone();

        for i in 0..old_a.len() {
            let a = old_a[i];
            let b = old_b[i];

            // Reaction terms (Gray-Scott like)
            let da = p.alpha * a * a / (1.0 + p.delta * b) - p.gamma * a;
            let db = p.epsilon * a * a - p.zeta * b;

            self.activator.concentrations[i] = (a + da * dt).max(0.0);
            self.inhibitor.concentrations[i] = (b + db * dt).max(0.0);
        }

        // Diffusion
        self.activator.step(dt);
        self.inhibitor.step(dt);
    }

    /// Get activator concentration
    pub fn get_activator(&self, x: usize, y: usize, z: usize) -> f64 {
        self.activator.get(x, y, z)
    }

    /// Get inhibitor concentration
    pub fn get_inhibitor(&self, x: usize, y: usize, z: usize) -> f64 {
        self.inhibitor.get(x, y, z)
    }

    /// Check if pattern has stabilized
    pub fn is_stable(&self, previous_total: f64, threshold: f64) -> bool {
        let current = self.activator.total() + self.inhibitor.total();
        (current - previous_total).abs() < threshold
    }
}
