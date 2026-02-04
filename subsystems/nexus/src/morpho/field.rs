//! Morphogen concentration field implementation.

extern crate alloc;

use alloc::vec::Vec;

use super::types::MorphogenType;

/// Morphogen concentration field
#[derive(Debug, Clone)]
pub struct MorphogenField {
    /// Grid size
    pub(crate) size: usize,
    /// Morphogen type
    morphogen_type: MorphogenType,
    /// Concentration values (3D grid)
    pub(crate) concentrations: Vec<f64>,
    /// Diffusion coefficient
    diffusion: f64,
    /// Decay rate
    decay: f64,
}

impl MorphogenField {
    /// Create a new morphogen field
    pub fn new(size: usize, morphogen_type: MorphogenType) -> Self {
        let total = size * size * size;
        Self {
            size,
            morphogen_type,
            concentrations: alloc::vec![0.0; total],
            diffusion: 0.1,
            decay: 0.01,
        }
    }

    /// Set diffusion coefficient
    pub fn with_diffusion(mut self, diffusion: f64) -> Self {
        self.diffusion = diffusion;
        self
    }

    /// Set decay rate
    pub fn with_decay(mut self, decay: f64) -> Self {
        self.decay = decay;
        self
    }

    /// Get index from coordinates
    fn index(&self, x: usize, y: usize, z: usize) -> usize {
        x + y * self.size + z * self.size * self.size
    }

    /// Get concentration at position
    pub fn get(&self, x: usize, y: usize, z: usize) -> f64 {
        if x < self.size && y < self.size && z < self.size {
            self.concentrations[self.index(x, y, z)]
        } else {
            0.0
        }
    }

    /// Set concentration at position
    pub fn set(&mut self, x: usize, y: usize, z: usize, value: f64) {
        if x < self.size && y < self.size && z < self.size {
            let idx = self.index(x, y, z);
            self.concentrations[idx] = value;
        }
    }

    /// Add concentration (source)
    pub fn add_source(&mut self, x: usize, y: usize, z: usize, amount: f64) {
        if x < self.size && y < self.size && z < self.size {
            let idx = self.index(x, y, z);
            self.concentrations[idx] += amount;
        }
    }

    /// Simulate one time step (diffusion + decay)
    pub fn step(&mut self, dt: f64) {
        let mut new_concentrations = self.concentrations.clone();

        for z in 0..self.size {
            for y in 0..self.size {
                for x in 0..self.size {
                    let idx = self.index(x, y, z);
                    let current = self.concentrations[idx];

                    // Laplacian (discrete)
                    let mut laplacian = -6.0 * current;

                    if x > 0 {
                        laplacian += self.get(x - 1, y, z);
                    }
                    if x < self.size - 1 {
                        laplacian += self.get(x + 1, y, z);
                    }
                    if y > 0 {
                        laplacian += self.get(x, y - 1, z);
                    }
                    if y < self.size - 1 {
                        laplacian += self.get(x, y + 1, z);
                    }
                    if z > 0 {
                        laplacian += self.get(x, y, z - 1);
                    }
                    if z < self.size - 1 {
                        laplacian += self.get(x, y, z + 1);
                    }

                    // Diffusion + decay
                    let change = self.diffusion * laplacian - self.decay * current;
                    new_concentrations[idx] = (current + change * dt).max(0.0);
                }
            }
        }

        self.concentrations = new_concentrations;
    }

    /// Get gradient at position
    pub fn gradient(&self, x: usize, y: usize, z: usize) -> (f64, f64, f64) {
        let dx = if x > 0 && x < self.size - 1 {
            (self.get(x + 1, y, z) - self.get(x - 1, y, z)) / 2.0
        } else {
            0.0
        };

        let dy = if y > 0 && y < self.size - 1 {
            (self.get(x, y + 1, z) - self.get(x, y - 1, z)) / 2.0
        } else {
            0.0
        };

        let dz = if z > 0 && z < self.size - 1 {
            (self.get(x, y, z + 1) - self.get(x, y, z - 1)) / 2.0
        } else {
            0.0
        };

        (dx, dy, dz)
    }

    /// Get total concentration
    pub fn total(&self) -> f64 {
        self.concentrations.iter().sum()
    }
}
