//! PackNet: Pruning-based continual learning.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use crate::continual::utils::lcg_next;

/// Mask for network pruning
#[derive(Debug, Clone)]
pub struct PruningMask {
    /// Per-layer masks (1.0 = active, 0.0 = pruned)
    pub masks: Vec<Vec<f64>>,
    /// Task this mask belongs to
    pub task_id: u64,
}

impl PruningMask {
    /// Create a full mask (all weights active)
    pub fn full(layer_sizes: &[(usize, usize)], task_id: u64) -> Self {
        let masks = layer_sizes
            .iter()
            .map(|&(rows, cols)| vec![1.0; rows * cols])
            .collect();

        Self { masks, task_id }
    }

    /// Apply mask to weights
    pub fn apply(&self, weights: &[Vec<f64>]) -> Vec<Vec<f64>> {
        weights
            .iter()
            .zip(self.masks.iter())
            .map(|(w, m)| w.iter().zip(m.iter()).map(|(wi, mi)| wi * mi).collect())
            .collect()
    }

    /// Count active weights
    pub fn active_count(&self) -> usize {
        self.masks
            .iter()
            .flat_map(|m| m.iter())
            .filter(|&&v| v > 0.5)
            .count()
    }

    /// Total weights
    pub fn total_count(&self) -> usize {
        self.masks.iter().map(|m| m.len()).sum()
    }

    /// Sparsity ratio
    pub fn sparsity(&self) -> f64 {
        let active = self.active_count();
        let total = self.total_count();
        if total == 0 {
            0.0
        } else {
            1.0 - (active as f64 / total as f64)
        }
    }
}

/// PackNet continual learning
pub struct PackNet {
    /// Network weights (flattened per layer)
    pub weights: Vec<Vec<f64>>,
    /// Biases per layer
    pub biases: Vec<Vec<f64>>,
    /// Masks per task
    pub task_masks: Vec<PruningMask>,
    /// Available mask (weights not yet assigned)
    pub available: Vec<Vec<f64>>,
    /// Layer sizes (input, output)
    pub layer_sizes: Vec<(usize, usize)>,
    /// Pruning ratio per task
    pub prune_ratio: f64,
}

impl PackNet {
    /// Create a new PackNet
    pub fn new(layer_sizes: Vec<(usize, usize)>, prune_ratio: f64, seed: u64) -> Self {
        let mut weights = Vec::new();
        let mut biases = Vec::new();
        let mut available = Vec::new();
        let mut rng = seed;

        for &(in_size, out_size) in &layer_sizes {
            let n_weights = in_size * out_size;

            let mut layer_weights = Vec::with_capacity(n_weights);
            for _ in 0..n_weights {
                rng = lcg_next(rng);
                let w = (rng as f64 / u64::MAX as f64) * 2.0 - 1.0;
                layer_weights.push(w * 0.1);
            }
            weights.push(layer_weights);

            biases.push(vec![0.0; out_size]);
            available.push(vec![1.0; n_weights]); // All weights available initially
        }

        Self {
            weights,
            biases,
            task_masks: Vec::new(),
            available,
            layer_sizes,
            prune_ratio,
        }
    }

    /// Prune and assign weights to a task
    pub fn assign_task(&mut self, task_id: u64) {
        let mut mask = PruningMask::full(&self.layer_sizes, task_id);

        // Only use available weights
        for (layer_mask, layer_avail) in mask.masks.iter_mut().zip(self.available.iter()) {
            for (m, a) in layer_mask.iter_mut().zip(layer_avail.iter()) {
                *m *= a; // Only consider available weights
            }
        }

        // Prune based on magnitude
        for (layer_idx, layer_mask) in mask.masks.iter_mut().enumerate() {
            let layer_weights = &self.weights[layer_idx];

            // Get indices of available weights
            let mut indices: Vec<(usize, f64)> = layer_mask
                .iter()
                .enumerate()
                .filter(|(_, &m)| m > 0.5)
                .map(|(i, _)| (i, libm::fabs(layer_weights[i])))
                .collect();

            // Sort by magnitude
            indices.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

            // Prune smallest weights
            let n_to_prune = (indices.len() as f64 * self.prune_ratio) as usize;
            for (idx, _) in indices.iter().take(n_to_prune) {
                layer_mask[*idx] = 0.0;
            }
        }

        // Update available weights
        for (layer_avail, layer_mask) in self.available.iter_mut().zip(mask.masks.iter()) {
            for (a, m) in layer_avail.iter_mut().zip(layer_mask.iter()) {
                if *m > 0.5 {
                    *a = 0.0; // No longer available
                }
            }
        }

        self.task_masks.push(mask);
    }

    /// Forward pass for a specific task
    pub fn forward(&self, task_id: u64, input: &[f64]) -> Vec<f64> {
        let mask = self.task_masks.iter().find(|m| m.task_id == task_id);

        if mask.is_none() {
            return Vec::new();
        }

        let mask = mask.unwrap();
        let masked_weights = mask.apply(&self.weights);

        let mut current = input.to_vec();

        for (layer_idx, &(in_size, out_size)) in self.layer_sizes.iter().enumerate() {
            let mut output = Vec::with_capacity(out_size);

            for j in 0..out_size {
                let mut sum = self.biases[layer_idx][j];
                for i in 0..in_size {
                    let w_idx = j * in_size + i;
                    if w_idx < masked_weights[layer_idx].len() && i < current.len() {
                        sum += masked_weights[layer_idx][w_idx] * current[i];
                    }
                }
                output.push(libm::tanh(sum));
            }

            current = output;
        }

        current
    }

    /// Get remaining capacity
    pub fn remaining_capacity(&self) -> f64 {
        let available: usize = self
            .available
            .iter()
            .flat_map(|a| a.iter())
            .filter(|&&v| v > 0.5)
            .count();

        let total: usize = self.available.iter().map(|a| a.len()).sum();

        if total == 0 {
            0.0
        } else {
            available as f64 / total as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pruning_mask() {
        let layer_sizes = vec![(4, 8), (8, 2)];
        let mask = PruningMask::full(&layer_sizes, 0);

        assert_eq!(mask.active_count(), 4 * 8 + 8 * 2);
        assert!((mask.sparsity()).abs() < 1e-10);
    }

    #[test]
    fn test_packnet() {
        let layer_sizes = vec![(4, 8), (8, 2)];
        let mut packnet = PackNet::new(layer_sizes, 0.5, 12345);

        packnet.assign_task(0);

        let input = vec![1.0, 0.5, -0.5, 0.0];
        let output = packnet.forward(0, &input);

        assert_eq!(output.len(), 2);
        assert!(packnet.remaining_capacity() > 0.0);
    }
}
