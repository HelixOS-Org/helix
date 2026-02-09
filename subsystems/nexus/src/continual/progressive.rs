//! Progressive Neural Networks implementation.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use crate::continual::utils::lcg_next;

/// A column in a progressive network
#[derive(Debug, Clone)]
pub struct ProgressiveColumn {
    /// Column ID (task ID)
    pub id: u64,
    /// Layer weights
    pub weights: Vec<Vec<Vec<f64>>>,
    /// Layer biases
    pub biases: Vec<Vec<f64>>,
    /// Is this column frozen?
    pub frozen: bool,
}

impl ProgressiveColumn {
    /// Create a new column
    pub fn new(id: u64, layer_sizes: &[usize], seed: u64) -> Self {
        let mut weights = Vec::new();
        let mut biases = Vec::new();
        let mut rng = seed;

        for i in 0..layer_sizes.len() - 1 {
            let in_size = layer_sizes[i];
            let out_size = layer_sizes[i + 1];

            let mut layer_weights = Vec::with_capacity(out_size);
            let mut layer_biases = Vec::with_capacity(out_size);

            for _ in 0..out_size {
                let mut neuron_weights = Vec::with_capacity(in_size);
                for _ in 0..in_size {
                    rng = lcg_next(rng);
                    let w = (rng as f64 / u64::MAX as f64) * 2.0 - 1.0;
                    neuron_weights.push(w * 0.1);
                }
                layer_weights.push(neuron_weights);
                layer_biases.push(0.0);
            }

            weights.push(layer_weights);
            biases.push(layer_biases);
        }

        Self {
            id,
            weights,
            biases,
            frozen: false,
        }
    }

    /// Freeze this column
    #[inline(always)]
    pub fn freeze(&mut self) {
        self.frozen = true;
    }

    /// Forward pass through this column
    pub fn forward(&self, input: &[f64]) -> Vec<Vec<f64>> {
        let mut activations = vec![input.to_vec()];
        let mut current = input.to_vec();

        for (layer_w, layer_b) in self.weights.iter().zip(self.biases.iter()) {
            let mut output = Vec::with_capacity(layer_w.len());

            for (neuron_w, &b) in layer_w.iter().zip(layer_b.iter()) {
                let sum: f64 = neuron_w
                    .iter()
                    .zip(current.iter())
                    .map(|(w, x)| w * x)
                    .sum::<f64>()
                    + b;
                output.push(libm::tanh(sum)); // ReLU activation
            }

            activations.push(output.clone());
            current = output;
        }

        activations
    }

    /// Get output for a layer
    #[inline(always)]
    pub fn get_layer_output(&self, layer: usize, activations: &[Vec<f64>]) -> Option<&Vec<f64>> {
        activations.get(layer + 1)
    }
}

/// Lateral adapter for connecting columns
#[derive(Debug, Clone)]
pub struct LateralAdapter {
    /// From column
    pub from_column: u64,
    /// To column
    pub to_column: u64,
    /// Layer index
    pub layer: usize,
    /// Adapter weights
    pub weights: Vec<Vec<f64>>,
}

impl LateralAdapter {
    /// Create a new adapter
    pub fn new(
        from: u64,
        to: u64,
        layer: usize,
        from_size: usize,
        to_size: usize,
        seed: u64,
    ) -> Self {
        let mut weights = Vec::with_capacity(to_size);
        let mut rng = seed;

        for _ in 0..to_size {
            let mut row = Vec::with_capacity(from_size);
            for _ in 0..from_size {
                rng = lcg_next(rng);
                let w = (rng as f64 / u64::MAX as f64) * 2.0 - 1.0;
                row.push(w * 0.01); // Small initial weights
            }
            weights.push(row);
        }

        Self {
            from_column: from,
            to_column: to,
            layer,
            weights,
        }
    }

    /// Apply adapter
    #[inline]
    pub fn apply(&self, from_activation: &[f64]) -> Vec<f64> {
        self.weights
            .iter()
            .map(|row| {
                row.iter()
                    .zip(from_activation.iter())
                    .map(|(w, x)| w * x)
                    .sum()
            })
            .collect()
    }
}

/// Progressive neural network
pub struct ProgressiveNetwork {
    /// All columns (one per task)
    pub columns: Vec<ProgressiveColumn>,
    /// Lateral adapters
    pub adapters: Vec<LateralAdapter>,
    /// Layer sizes
    pub layer_sizes: Vec<usize>,
    /// Random seed
    seed: u64,
}

impl ProgressiveNetwork {
    /// Create a new progressive network
    pub fn new(layer_sizes: Vec<usize>, seed: u64) -> Self {
        Self {
            columns: Vec::new(),
            adapters: Vec::new(),
            layer_sizes,
            seed,
        }
    }

    /// Add a new column for a new task
    pub fn add_column(&mut self, task_id: u64) {
        // Freeze previous columns
        for col in &mut self.columns {
            col.freeze();
        }

        self.seed = lcg_next(self.seed);
        let new_column = ProgressiveColumn::new(task_id, &self.layer_sizes, self.seed);

        // Create lateral adapters from all previous columns
        for prev_col in &self.columns {
            for layer in 0..self.layer_sizes.len() - 1 {
                let from_size = self.layer_sizes[layer + 1]; // Previous layer output
                let to_size = self.layer_sizes[layer + 1]; // Same size

                self.seed = lcg_next(self.seed);
                let adapter =
                    LateralAdapter::new(prev_col.id, task_id, layer, from_size, to_size, self.seed);
                self.adapters.push(adapter);
            }
        }

        self.columns.push(new_column);
    }

    /// Forward pass for a specific task
    pub fn forward(&self, task_id: u64, input: &[f64]) -> Vec<f64> {
        // Find the target column
        let col_idx = self.columns.iter().position(|c| c.id == task_id);
        if col_idx.is_none() {
            return Vec::new();
        }
        let col_idx = col_idx.unwrap();

        // Compute activations for all columns up to and including target
        let mut all_activations: Vec<Vec<Vec<f64>>> = Vec::new();

        for col in &self.columns[..=col_idx] {
            let activations = col.forward(input);
            all_activations.push(activations);
        }

        // Apply lateral connections for the target column
        if col_idx > 0 {
            let mut final_output = all_activations[col_idx].last().unwrap().clone();

            // Add lateral contributions
            for adapter in &self.adapters {
                if adapter.to_column == task_id {
                    if let Some(from_idx) = self
                        .columns
                        .iter()
                        .position(|c| c.id == adapter.from_column)
                    {
                        if let Some(from_act) = all_activations[from_idx].get(adapter.layer + 1) {
                            let lateral = adapter.apply(from_act);
                            for (f, l) in final_output.iter_mut().zip(lateral.iter()) {
                                *f += l;
                            }
                        }
                    }
                }
            }

            // Apply final activation
            for v in &mut final_output {
                *v = libm::tanh(*v);
            }

            return final_output;
        }

        all_activations[col_idx].last().cloned().unwrap_or_default()
    }

    /// Get number of columns (tasks)
    #[inline(always)]
    pub fn num_tasks(&self) -> usize {
        self.columns.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progressive_column() {
        let column = ProgressiveColumn::new(0, &[4, 8, 2], 12345);

        let input = vec![1.0, 0.5, -0.5, 0.0];
        let activations = column.forward(&input);

        assert_eq!(activations.len(), 3); // input, hidden, output
        assert_eq!(activations[0].len(), 4);
        assert_eq!(activations[1].len(), 8);
        assert_eq!(activations[2].len(), 2);
    }

    #[test]
    fn test_progressive_network() {
        let mut net = ProgressiveNetwork::new(vec![4, 8, 2], 12345);

        net.add_column(0);
        net.add_column(1);

        assert_eq!(net.num_tasks(), 2);

        let input = vec![1.0, 0.5, -0.5, 0.0];
        let output = net.forward(1, &input);

        assert_eq!(output.len(), 2);
    }
}
