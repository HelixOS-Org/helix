//! Modality alignment strategies for projecting modalities to shared spaces.

extern crate alloc;

use alloc::vec::Vec;

use crate::multimodal::encoder::ModalityEncoder;
use crate::multimodal::utils::{create_layer, lcg_next, project_vec};

/// Contrastive alignment for modalities
#[derive(Debug, Clone)]
pub struct ContrastiveAlignment {
    /// Shared dimension
    pub shared_dim: usize,
    /// Temperature for contrastive loss
    pub temperature: f64,
    /// Modality projectors
    pub projectors: Vec<ModalityEncoder>,
}

impl ContrastiveAlignment {
    /// Create a new contrastive alignment
    pub fn new(modality_dims: &[usize], shared_dim: usize, seed: u64) -> Self {
        let mut projectors = Vec::new();
        let mut rng = seed;

        for &dim in modality_dims {
            projectors.push(ModalityEncoder::new(dim, shared_dim, rng));
            rng = lcg_next(rng);
        }

        Self {
            shared_dim,
            temperature: 0.07,
            projectors,
        }
    }

    /// Project all modalities to shared space
    pub fn project_all(&self, inputs: &[&[f64]]) -> Vec<Vec<f64>> {
        inputs
            .iter()
            .zip(self.projectors.iter())
            .map(|(input, projector)| {
                let mut projected = projector.encode(input);
                // L2 normalize
                let norm: f64 = libm::sqrt(projected.iter().map(|x| x * x).sum());
                if norm > 1e-10 {
                    for v in &mut projected {
                        *v /= norm;
                    }
                }
                projected
            })
            .collect()
    }

    /// Compute alignment loss (InfoNCE)
    pub fn alignment_loss(&self, projected: &[Vec<f64>]) -> f64 {
        if projected.len() < 2 {
            return 0.0;
        }

        let mut total_loss = 0.0;
        let num_pairs = projected.len() * (projected.len() - 1) / 2;

        for i in 0..projected.len() {
            for j in (i + 1)..projected.len() {
                // Cosine similarity
                let sim: f64 = projected[i]
                    .iter()
                    .zip(projected[j].iter())
                    .map(|(&a, &b)| a * b)
                    .sum();

                // InfoNCE loss component
                let loss = -sim / self.temperature;
                total_loss += loss;
            }
        }

        total_loss / num_pairs as f64
    }
}

/// Canonical Correlation Analysis alignment
#[derive(Debug, Clone)]
pub struct CCAAlignment {
    /// Projection for modality A
    pub proj_a: Vec<Vec<f64>>,
    /// Projection for modality B
    pub proj_b: Vec<Vec<f64>>,
    /// Correlation dimension
    pub corr_dim: usize,
}

impl CCAAlignment {
    /// Create a new CCA alignment
    pub fn new(dim_a: usize, dim_b: usize, corr_dim: usize, seed: u64) -> Self {
        let (proj_a, _, rng2) = create_layer(dim_a, corr_dim, seed);
        let (proj_b, _, _) = create_layer(dim_b, corr_dim, rng2);

        Self {
            proj_a,
            proj_b,
            corr_dim,
        }
    }

    /// Project both modalities
    pub fn project(&self, input_a: &[f64], input_b: &[f64]) -> (Vec<f64>, Vec<f64>) {
        let proj_a = project_vec(&self.proj_a, input_a);
        let proj_b = project_vec(&self.proj_b, input_b);

        (proj_a, proj_b)
    }

    /// Compute correlation
    pub fn correlation(&self, input_a: &[f64], input_b: &[f64]) -> f64 {
        let (proj_a, proj_b) = self.project(input_a, input_b);

        let norm_a: f64 = libm::sqrt(proj_a.iter().map(|x| x * x).sum());
        let norm_b: f64 = libm::sqrt(proj_b.iter().map(|x| x * x).sum());

        if norm_a < 1e-10 || norm_b < 1e-10 {
            return 0.0;
        }

        let dot: f64 = proj_a.iter().zip(proj_b.iter()).map(|(&a, &b)| a * b).sum();

        dot / (norm_a * norm_b)
    }
}
