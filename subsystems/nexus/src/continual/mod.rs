//! # Continual Learning Engine for Helix OS Kernel
//!
//! Year 3 "EVOLUTION" - Revolutionary lifelong learning system that enables
//! the kernel to continuously learn from experience without catastrophic forgetting.
//!
//! ## Key Features
//!
//! - **Elastic Weight Consolidation (EWC)**: Protects important weights
//! - **Progressive Neural Networks**: Lateral connections for transfer
//! - **Memory Replay**: Experience replay with prioritization
//! - **Synaptic Intelligence**: Online importance estimation
//! - **PackNet**: Network pruning for task-specific subnetworks
//! - **Meta-Continual Learning**: Learning to learn continuously
//!
//! ## Kernel Applications
//!
//! - Adapt to new workloads without forgetting old ones
//! - Transfer learning between kernel components
//! - Online adaptation to hardware changes
//! - Continuous security policy learning

#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

// ============================================================================
// CORE TYPES
// ============================================================================

/// Types of continual learning strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContinualStrategy {
    /// Elastic Weight Consolidation
    EWC,
    /// Synaptic Intelligence
    SI,
    /// Learning without Forgetting
    LwF,
    /// Progressive Neural Networks
    Progressive,
    /// Experience Replay
    Replay,
    /// Memory Aware Synapses
    MAS,
    /// PackNet (pruning-based)
    PackNet,
    /// Gradient Episodic Memory
    GEM,
}

/// A task in the continual learning setting
#[derive(Debug, Clone)]
pub struct Task {
    /// Task identifier
    pub id: u64,
    /// Task name
    pub name: String,
    /// Number of training samples
    pub num_samples: usize,
    /// Number of epochs trained
    pub epochs_trained: u32,
    /// Final accuracy on this task
    pub accuracy: f64,
    /// Is this task currently active?
    pub is_active: bool,
    /// Task-specific metadata
    pub metadata: BTreeMap<String, f64>,
}

impl Task {
    /// Create a new task
    pub fn new(id: u64, name: String) -> Self {
        Self {
            id,
            name,
            num_samples: 0,
            epochs_trained: 0,
            accuracy: 0.0,
            is_active: true,
            metadata: BTreeMap::new(),
        }
    }
}

/// A memory sample for replay
#[derive(Debug, Clone)]
pub struct MemorySample {
    /// Input features
    pub input: Vec<f64>,
    /// Target output
    pub target: Vec<f64>,
    /// Task ID this sample belongs to
    pub task_id: u64,
    /// Priority for sampling
    pub priority: f64,
    /// Number of times replayed
    pub replay_count: u32,
    /// Last replay timestamp
    pub last_replay: u64,
}

impl MemorySample {
    /// Create a new memory sample
    pub fn new(input: Vec<f64>, target: Vec<f64>, task_id: u64) -> Self {
        Self {
            input,
            target,
            task_id,
            priority: 1.0,
            replay_count: 0,
            last_replay: 0,
        }
    }

    /// Update priority after replay
    pub fn update_priority(&mut self, td_error: f64) {
        self.priority = libm::fabs(td_error) + 0.01; // Small epsilon for stability
        self.replay_count += 1;
    }
}

// ============================================================================
// ELASTIC WEIGHT CONSOLIDATION (EWC)
// ============================================================================

/// Fisher information diagonal for EWC
#[derive(Debug, Clone)]
pub struct FisherInformation {
    /// Fisher diagonal values per parameter
    pub fisher: Vec<f64>,
    /// Optimal parameter values after task
    pub optimal_params: Vec<f64>,
    /// Task ID this Fisher was computed for
    pub task_id: u64,
}

impl FisherInformation {
    /// Create empty Fisher information
    pub fn new(param_count: usize, task_id: u64) -> Self {
        Self {
            fisher: vec![0.0; param_count],
            optimal_params: vec![0.0; param_count],
            task_id,
        }
    }

    /// Estimate Fisher information from gradients
    pub fn estimate(&mut self, gradients: &[Vec<f64>]) {
        if gradients.is_empty() {
            return;
        }

        let n_samples = gradients.len() as f64;

        // Fisher = E[grad * grad^T] diagonal
        for grad in gradients {
            for (i, &g) in grad.iter().enumerate() {
                if i < self.fisher.len() {
                    self.fisher[i] += g * g;
                }
            }
        }

        // Average
        for f in &mut self.fisher {
            *f /= n_samples;
        }
    }

    /// Store optimal parameters
    pub fn set_optimal(&mut self, params: &[f64]) {
        self.optimal_params = params.to_vec();
    }

    /// Compute EWC penalty for current parameters
    pub fn penalty(&self, current_params: &[f64], lambda: f64) -> f64 {
        let mut penalty = 0.0;

        for (i, (&current, &optimal)) in current_params
            .iter()
            .zip(self.optimal_params.iter())
            .enumerate()
        {
            if i < self.fisher.len() {
                let diff = current - optimal;
                penalty += self.fisher[i] * diff * diff;
            }
        }

        0.5 * lambda * penalty
    }

    /// Compute gradient of EWC penalty
    pub fn penalty_gradient(&self, current_params: &[f64], lambda: f64) -> Vec<f64> {
        let mut grad = vec![0.0; current_params.len()];

        for (i, (&current, &optimal)) in current_params
            .iter()
            .zip(self.optimal_params.iter())
            .enumerate()
        {
            if i < self.fisher.len() {
                grad[i] = lambda * self.fisher[i] * (current - optimal);
            }
        }

        grad
    }
}

/// EWC-based continual learner
pub struct EwcLearner {
    /// Fisher information per task
    pub fishers: Vec<FisherInformation>,
    /// Current parameters
    pub params: Vec<f64>,
    /// Lambda coefficient for EWC penalty
    pub lambda: f64,
    /// Online mode (accumulate Fisher)
    pub online: bool,
    /// Decay factor for online EWC
    pub gamma: f64,
}

impl EwcLearner {
    /// Create a new EWC learner
    pub fn new(param_count: usize, lambda: f64) -> Self {
        Self {
            fishers: Vec::new(),
            params: vec![0.0; param_count],
            lambda,
            online: false,
            gamma: 0.9,
        }
    }

    /// Enable online EWC mode
    pub fn enable_online(&mut self, gamma: f64) {
        self.online = true;
        self.gamma = gamma;
    }

    /// Register a completed task
    pub fn register_task(&mut self, task_id: u64, gradients: &[Vec<f64>], params: &[f64]) {
        let mut fisher = FisherInformation::new(params.len(), task_id);
        fisher.estimate(gradients);
        fisher.set_optimal(params);

        if self.online && !self.fishers.is_empty() {
            // Merge with previous Fisher using decay
            let prev = self.fishers.last_mut().unwrap();
            for (i, f) in fisher.fisher.iter_mut().enumerate() {
                if i < prev.fisher.len() {
                    *f = self.gamma * prev.fisher[i] + *f;
                }
            }
            // Update in place
            *self.fishers.last_mut().unwrap() = fisher;
        } else {
            self.fishers.push(fisher);
        }

        self.params = params.to_vec();
    }

    /// Compute total EWC penalty
    pub fn total_penalty(&self, current_params: &[f64]) -> f64 {
        self.fishers
            .iter()
            .map(|f| f.penalty(current_params, self.lambda))
            .sum()
    }

    /// Compute gradient including EWC penalty
    pub fn regularized_gradient(&self, current_params: &[f64], task_gradient: &[f64]) -> Vec<f64> {
        let mut grad = task_gradient.to_vec();

        for fisher in &self.fishers {
            let ewc_grad = fisher.penalty_gradient(current_params, self.lambda);
            for (g, eg) in grad.iter_mut().zip(ewc_grad.iter()) {
                *g += eg;
            }
        }

        grad
    }
}

// ============================================================================
// SYNAPTIC INTELLIGENCE (SI)
// ============================================================================

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
    pub fn init_params(&mut self, params: &[f64]) {
        self.prev_params = params.to_vec();
        for pi in &mut self.path_integral {
            *pi = 0.0;
        }
    }

    /// Update after a training step
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

// ============================================================================
// MEMORY REPLAY SYSTEM
// ============================================================================

/// Configuration for memory replay
#[derive(Debug, Clone)]
pub struct ReplayConfig {
    /// Maximum memory buffer size
    pub buffer_size: usize,
    /// Replay batch size
    pub batch_size: usize,
    /// Use prioritized replay
    pub prioritized: bool,
    /// Priority exponent
    pub alpha: f64,
    /// Importance sampling exponent
    pub beta: f64,
    /// Samples per task (for balanced replay)
    pub samples_per_task: usize,
}

impl Default for ReplayConfig {
    fn default() -> Self {
        Self {
            buffer_size: 5000,
            batch_size: 32,
            prioritized: true,
            alpha: 0.6,
            beta: 0.4,
            samples_per_task: 100,
        }
    }
}

/// Memory buffer for experience replay
pub struct MemoryBuffer {
    /// Stored samples
    pub samples: Vec<MemorySample>,
    /// Configuration
    pub config: ReplayConfig,
    /// Per-task sample counts
    pub task_counts: BTreeMap<u64, usize>,
    /// Random state
    rng_state: u64,
    /// Sum tree for prioritized sampling
    priority_sum: f64,
}

impl MemoryBuffer {
    /// Create a new memory buffer
    pub fn new(config: ReplayConfig, seed: u64) -> Self {
        Self {
            samples: Vec::with_capacity(config.buffer_size),
            task_counts: BTreeMap::new(),
            priority_sum: 0.0,
            rng_state: seed,
            config,
        }
    }

    /// Add a sample to the buffer
    pub fn add(&mut self, sample: MemorySample) {
        let task_id = sample.task_id;

        if self.samples.len() >= self.config.buffer_size {
            // Reservoir sampling with priority
            self.rng_state = lcg_next(self.rng_state);
            let idx = self.rng_state as usize % self.samples.len();

            // Decrease count for removed task
            if let Some(count) = self.task_counts.get_mut(&self.samples[idx].task_id) {
                *count = count.saturating_sub(1);
            }

            self.priority_sum -= self.samples[idx].priority.powf(self.config.alpha);
            self.samples[idx] = sample;
        } else {
            self.samples.push(sample);
        }

        let sample_priority = self.samples.last().unwrap().priority;
        self.priority_sum += sample_priority.powf(self.config.alpha);
        *self.task_counts.entry(task_id).or_insert(0) += 1;
    }

    /// Sample a batch from the buffer
    pub fn sample_batch(&mut self) -> Vec<(usize, &MemorySample, f64)> {
        let mut batch = Vec::with_capacity(self.config.batch_size);

        if self.samples.is_empty() {
            return batch;
        }

        if self.config.prioritized {
            // Prioritized sampling
            for _ in 0..self.config.batch_size {
                self.rng_state = lcg_next(self.rng_state);
                let target = (self.rng_state as f64 / u64::MAX as f64) * self.priority_sum;

                let mut cumsum = 0.0;
                for (idx, sample) in self.samples.iter().enumerate() {
                    cumsum += sample.priority.powf(self.config.alpha);
                    if cumsum >= target {
                        // Importance sampling weight
                        let prob = sample.priority.powf(self.config.alpha) / self.priority_sum;
                        let weight =
                            (1.0 / (self.samples.len() as f64 * prob)).powf(self.config.beta);
                        batch.push((idx, sample, weight));
                        break;
                    }
                }
            }
        } else {
            // Uniform sampling
            for _ in 0..self.config.batch_size {
                self.rng_state = lcg_next(self.rng_state);
                let idx = self.rng_state as usize % self.samples.len();
                batch.push((idx, &self.samples[idx], 1.0));
            }
        }

        batch
    }

    /// Update priorities after training
    pub fn update_priorities(&mut self, updates: &[(usize, f64)]) {
        for &(idx, td_error) in updates {
            if idx < self.samples.len() {
                let old_priority = self.samples[idx].priority;
                self.samples[idx].update_priority(td_error);
                let new_priority = self.samples[idx].priority;

                self.priority_sum -= old_priority.powf(self.config.alpha);
                self.priority_sum += new_priority.powf(self.config.alpha);
            }
        }
    }

    /// Get balanced samples across tasks
    pub fn balanced_sample(&mut self) -> Vec<&MemorySample> {
        let mut samples = Vec::new();
        let tasks: Vec<u64> = self.task_counts.keys().copied().collect();

        for task_id in tasks {
            let task_samples: Vec<usize> = self
                .samples
                .iter()
                .enumerate()
                .filter(|(_, s)| s.task_id == task_id)
                .map(|(i, _)| i)
                .collect();

            let n = task_samples.len().min(self.config.samples_per_task);
            for _ in 0..n {
                self.rng_state = lcg_next(self.rng_state);
                let idx = task_samples[self.rng_state as usize % task_samples.len()];
                samples.push(&self.samples[idx]);
            }
        }

        samples
    }

    /// Get buffer statistics
    pub fn stats(&self) -> BufferStats {
        BufferStats {
            total_samples: self.samples.len(),
            num_tasks: self.task_counts.len(),
            task_distribution: self.task_counts.clone(),
            avg_priority: if self.samples.is_empty() {
                0.0
            } else {
                self.samples.iter().map(|s| s.priority).sum::<f64>() / self.samples.len() as f64
            },
        }
    }
}

/// Memory buffer statistics
#[derive(Debug, Clone)]
pub struct BufferStats {
    pub total_samples: usize,
    pub num_tasks: usize,
    pub task_distribution: BTreeMap<u64, usize>,
    pub avg_priority: f64,
}

// ============================================================================
// PROGRESSIVE NEURAL NETWORKS
// ============================================================================

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
            let target_col = &self.columns[col_idx];
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
    pub fn num_tasks(&self) -> usize {
        self.columns.len()
    }
}

// ============================================================================
// PACKNET: PRUNING-BASED APPROACH
// ============================================================================

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

// ============================================================================
// GRADIENT EPISODIC MEMORY (GEM)
// ============================================================================

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

// ============================================================================
// CONTINUAL LEARNING MANAGER
// ============================================================================

/// Configuration for the continual learning manager
#[derive(Debug, Clone)]
pub struct ContinualConfig {
    /// Primary strategy
    pub strategy: ContinualStrategy,
    /// EWC lambda
    pub ewc_lambda: f64,
    /// SI strength
    pub si_c: f64,
    /// Replay enabled
    pub use_replay: bool,
    /// Replay configuration
    pub replay_config: ReplayConfig,
    /// Parameter count
    pub param_count: usize,
}

impl Default for ContinualConfig {
    fn default() -> Self {
        Self {
            strategy: ContinualStrategy::EWC,
            ewc_lambda: 1000.0,
            si_c: 1.0,
            use_replay: true,
            replay_config: ReplayConfig::default(),
            param_count: 1000,
        }
    }
}

/// Main continual learning manager
pub struct ContinualLearningManager {
    /// Configuration
    pub config: ContinualConfig,
    /// Tasks encountered
    pub tasks: Vec<Task>,
    /// Current task
    pub current_task: Option<u64>,
    /// EWC learner
    pub ewc: Option<EwcLearner>,
    /// SI learner
    pub si: Option<SynapticIntelligence>,
    /// Memory buffer
    pub memory: Option<MemoryBuffer>,
    /// Progressive network
    pub progressive: Option<ProgressiveNetwork>,
    /// PackNet
    pub packnet: Option<PackNet>,
    /// GEM constraints
    pub gem: Option<GemConstraint>,
    /// Training history
    pub history: ContinualHistory,
    /// Random seed
    seed: u64,
}

/// Training history
#[derive(Debug, Clone, Default)]
pub struct ContinualHistory {
    /// Accuracy per task over time
    pub task_accuracy: BTreeMap<u64, Vec<f64>>,
    /// Backward transfer (change in old task performance)
    pub backward_transfer: Vec<f64>,
    /// Forward transfer (boost from previous tasks)
    pub forward_transfer: Vec<f64>,
    /// Forgetting measure
    pub forgetting: BTreeMap<u64, f64>,
}

impl ContinualLearningManager {
    /// Create a new manager
    pub fn new(config: ContinualConfig, seed: u64) -> Self {
        let mut manager = Self {
            ewc: None,
            si: None,
            memory: None,
            progressive: None,
            packnet: None,
            gem: None,
            tasks: Vec::new(),
            current_task: None,
            history: ContinualHistory::default(),
            config: config.clone(),
            seed,
        };

        // Initialize based on strategy
        match config.strategy {
            ContinualStrategy::EWC => {
                manager.ewc = Some(EwcLearner::new(config.param_count, config.ewc_lambda));
            },
            ContinualStrategy::SI => {
                manager.si = Some(SynapticIntelligence::new(config.param_count, config.si_c));
            },
            ContinualStrategy::GEM => {
                manager.gem = Some(GemConstraint::new(100, 0.1));
            },
            _ => {},
        }

        if config.use_replay {
            manager.memory = Some(MemoryBuffer::new(config.replay_config.clone(), seed));
        }

        manager
    }

    /// Start a new task
    pub fn start_task(&mut self, name: String) -> u64 {
        let task_id = self.tasks.len() as u64;
        let task = Task::new(task_id, name);

        self.tasks.push(task);
        self.current_task = Some(task_id);

        // Initialize task in progressive network if used
        if let Some(ref mut prog) = self.progressive {
            prog.add_column(task_id);
        }

        // Initialize task in PackNet if used
        if let Some(ref mut pack) = self.packnet {
            pack.assign_task(task_id);
        }

        task_id
    }

    /// End current task
    pub fn end_task(&mut self, final_params: &[f64], gradients: &[Vec<f64>]) {
        let task_id = match self.current_task {
            Some(id) => id,
            None => return,
        };

        // Register with EWC
        if let Some(ref mut ewc) = self.ewc {
            ewc.register_task(task_id, gradients, final_params);
        }

        // Consolidate SI
        if let Some(ref mut si) = self.si {
            // Get initial params (simplified - use zeros)
            let initial = vec![0.0; final_params.len()];
            si.consolidate(final_params, &initial);
        }

        // Add reference gradient for GEM
        if let Some(ref mut gem) = self.gem {
            if let Some(avg_grad) = average_gradients(gradients) {
                gem.add_reference(task_id, avg_grad);
            }
        }

        // Update task as inactive
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
            task.is_active = false;
        }

        self.current_task = None;
    }

    /// Record a training sample
    pub fn record_sample(&mut self, input: Vec<f64>, target: Vec<f64>) {
        if let (Some(ref mut memory), Some(task_id)) = (&mut self.memory, self.current_task) {
            let sample = MemorySample::new(input, target, task_id);
            memory.add(sample);
        }
    }

    /// Get regularized gradient
    pub fn regularize_gradient(&self, params: &[f64], task_gradient: &[f64]) -> Vec<f64> {
        let mut grad = task_gradient.to_vec();

        // Apply EWC regularization
        if let Some(ref ewc) = self.ewc {
            grad = ewc.regularized_gradient(params, &grad);
        }

        // Apply SI regularization
        if let Some(ref si) = self.si {
            let reference = vec![0.0; params.len()]; // Simplified
            grad = si.weighted_gradient(params, &reference, &grad);
        }

        // Apply GEM projection
        if let Some(ref gem) = self.gem {
            grad = gem.project(&grad);
        }

        grad
    }

    /// Get replay samples
    pub fn get_replay_batch(&mut self) -> Option<Vec<(Vec<f64>, Vec<f64>)>> {
        let memory = self.memory.as_mut()?;

        let batch = memory.sample_batch();
        let samples: Vec<_> = batch
            .into_iter()
            .map(|(_, s, _)| (s.input.clone(), s.target.clone()))
            .collect();

        if samples.is_empty() {
            None
        } else {
            Some(samples)
        }
    }

    /// Update task accuracy
    pub fn update_accuracy(&mut self, task_id: u64, accuracy: f64) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
            task.accuracy = accuracy;
        }

        self.history
            .task_accuracy
            .entry(task_id)
            .or_default()
            .push(accuracy);
    }

    /// Calculate forgetting
    pub fn calculate_forgetting(&mut self) {
        for (task_id, accuracies) in &self.history.task_accuracy {
            if accuracies.len() >= 2 {
                let max_acc = accuracies.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let current = accuracies.last().copied().unwrap_or(0.0);
                let forgetting = (max_acc - current).max(0.0);
                self.history.forgetting.insert(*task_id, forgetting);
            }
        }
    }

    /// Get average forgetting
    pub fn average_forgetting(&self) -> f64 {
        if self.history.forgetting.is_empty() {
            return 0.0;
        }

        let sum: f64 = self.history.forgetting.values().sum();
        sum / self.history.forgetting.len() as f64
    }

    /// Get learning summary
    pub fn get_summary(&self) -> ContinualSummary {
        ContinualSummary {
            num_tasks: self.tasks.len(),
            current_task: self.current_task,
            strategy: self.config.strategy,
            average_forgetting: self.average_forgetting(),
            memory_samples: self.memory.as_ref().map(|m| m.samples.len()).unwrap_or(0),
            task_accuracies: self.tasks.iter().map(|t| (t.id, t.accuracy)).collect(),
        }
    }
}

/// Summary of continual learning state
#[derive(Debug, Clone)]
pub struct ContinualSummary {
    pub num_tasks: usize,
    pub current_task: Option<u64>,
    pub strategy: ContinualStrategy,
    pub average_forgetting: f64,
    pub memory_samples: usize,
    pub task_accuracies: Vec<(u64, f64)>,
}

// ============================================================================
// KERNEL INTEGRATION
// ============================================================================

/// Types of kernel learning tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelLearningTask {
    /// Scheduler optimization for a workload type
    SchedulerOptimization,
    /// Memory management adaptation
    MemoryManagement,
    /// I/O pattern learning
    IoOptimization,
    /// Security policy learning
    SecurityPolicy,
    /// Power management
    PowerManagement,
    /// Network optimization
    NetworkOptimization,
}

/// Kernel continual learning manager
pub struct KernelContinualLearner {
    /// Core continual learning manager
    pub manager: ContinualLearningManager,
    /// Task type mapping
    pub task_types: BTreeMap<u64, KernelLearningTask>,
    /// Performance baselines per task type
    pub baselines: BTreeMap<KernelLearningTask, f64>,
    /// Transfer learning gains
    pub transfer_gains: Vec<(u64, u64, f64)>, // (from_task, to_task, gain)
}

impl KernelContinualLearner {
    /// Create a new kernel continual learner
    pub fn new(config: ContinualConfig, seed: u64) -> Self {
        Self {
            manager: ContinualLearningManager::new(config, seed),
            task_types: BTreeMap::new(),
            baselines: BTreeMap::new(),
            transfer_gains: Vec::new(),
        }
    }

    /// Start a kernel learning task
    pub fn start_kernel_task(&mut self, task_type: KernelLearningTask, name: String) -> u64 {
        let task_id = self.manager.start_task(name);
        self.task_types.insert(task_id, task_type);
        task_id
    }

    /// Record baseline performance
    pub fn record_baseline(&mut self, task_type: KernelLearningTask, performance: f64) {
        self.baselines.insert(task_type, performance);
    }

    /// End kernel task
    pub fn end_kernel_task(
        &mut self,
        final_params: &[f64],
        gradients: &[Vec<f64>],
        final_performance: f64,
    ) {
        if let Some(task_id) = self.manager.current_task {
            // Calculate transfer gain if baseline exists
            if let Some(&task_type) = self.task_types.get(&task_id) {
                if let Some(&baseline) = self.baselines.get(&task_type) {
                    let gain = final_performance - baseline;

                    // Record gains from previous tasks
                    for prev_task in &self.manager.tasks {
                        if prev_task.id != task_id && !prev_task.is_active {
                            self.transfer_gains.push((prev_task.id, task_id, gain));
                        }
                    }
                }
            }

            self.manager.update_accuracy(task_id, final_performance);
        }

        self.manager.end_task(final_params, gradients);
    }

    /// Get forward transfer for current task
    pub fn get_forward_transfer(&self) -> f64 {
        if let Some(task_id) = self.manager.current_task {
            let gains: f64 = self
                .transfer_gains
                .iter()
                .filter(|(_, to, _)| *to == task_id)
                .map(|(_, _, gain)| *gain)
                .sum();
            return gains;
        }
        0.0
    }

    /// Check if catastrophic forgetting is occurring
    pub fn detect_forgetting(&self, threshold: f64) -> Vec<u64> {
        self.manager
            .history
            .forgetting
            .iter()
            .filter(|(_, &f)| f > threshold)
            .map(|(&id, _)| id)
            .collect()
    }
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Linear congruential generator
fn lcg_next(state: u64) -> u64 {
    state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
}

/// Average a list of gradients
fn average_gradients(gradients: &[Vec<f64>]) -> Option<Vec<f64>> {
    if gradients.is_empty() {
        return None;
    }

    let n = gradients.len() as f64;
    let dim = gradients[0].len();
    let mut avg = vec![0.0; dim];

    for grad in gradients {
        for (i, &g) in grad.iter().enumerate() {
            if i < avg.len() {
                avg[i] += g;
            }
        }
    }

    for v in &mut avg {
        *v /= n;
    }

    Some(avg)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new(0, alloc::string::String::from("test_task"));
        assert_eq!(task.id, 0);
        assert!(task.is_active);
    }

    #[test]
    fn test_memory_sample() {
        let mut sample = MemorySample::new(vec![1.0, 2.0], vec![0.5], 0);
        assert_eq!(sample.priority, 1.0);

        sample.update_priority(0.5);
        assert!(sample.priority > 0.0);
        assert_eq!(sample.replay_count, 1);
    }

    #[test]
    fn test_fisher_information() {
        let mut fisher = FisherInformation::new(3, 0);

        let gradients = vec![vec![0.1, 0.2, 0.3], vec![0.2, 0.1, 0.4]];

        fisher.estimate(&gradients);
        fisher.set_optimal(&[1.0, 2.0, 3.0]);

        let penalty = fisher.penalty(&[1.1, 2.1, 3.1], 1.0);
        assert!(penalty > 0.0);
    }

    #[test]
    fn test_ewc_learner() {
        let mut ewc = EwcLearner::new(3, 100.0);

        let gradients = vec![vec![0.1, 0.2, 0.3]];
        let params = vec![1.0, 2.0, 3.0];

        ewc.register_task(0, &gradients, &params);

        let penalty = ewc.total_penalty(&[1.1, 2.1, 3.1]);
        assert!(penalty > 0.0);

        let task_grad = vec![0.5, 0.5, 0.5];
        let reg_grad = ewc.regularized_gradient(&[1.1, 2.1, 3.1], &task_grad);
        assert_eq!(reg_grad.len(), 3);
    }

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

    #[test]
    fn test_memory_buffer() {
        let config = ReplayConfig {
            buffer_size: 100,
            batch_size: 10,
            ..Default::default()
        };

        let mut buffer = MemoryBuffer::new(config, 12345);

        for i in 0..50 {
            let sample = MemorySample::new(vec![i as f64], vec![i as f64 * 2.0], i % 3);
            buffer.add(sample);
        }

        let stats = buffer.stats();
        assert_eq!(stats.total_samples, 50);
        assert_eq!(stats.num_tasks, 3);

        let batch = buffer.sample_batch();
        assert!(!batch.is_empty());
    }

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

    #[test]
    fn test_continual_learning_manager() {
        let config = ContinualConfig {
            strategy: ContinualStrategy::EWC,
            param_count: 10,
            ewc_lambda: 100.0,
            use_replay: true,
            ..Default::default()
        };

        let mut manager = ContinualLearningManager::new(config, 12345);

        let task_id = manager.start_task(alloc::string::String::from("Task1"));
        assert_eq!(task_id, 0);

        // Record samples
        manager.record_sample(vec![1.0, 2.0], vec![0.5]);
        manager.record_sample(vec![2.0, 3.0], vec![1.0]);

        // End task
        let params = vec![0.1; 10];
        let gradients = vec![vec![0.01; 10]];
        manager.end_task(&params, &gradients);

        // Start new task
        let task2 = manager.start_task(alloc::string::String::from("Task2"));
        assert_eq!(task2, 1);

        // Get regularized gradient
        let task_grad = vec![0.1; 10];
        let reg_grad = manager.regularize_gradient(&params, &task_grad);
        assert_eq!(reg_grad.len(), 10);

        // Get summary
        let summary = manager.get_summary();
        assert_eq!(summary.num_tasks, 2);
    }

    #[test]
    fn test_kernel_continual_learner() {
        let config = ContinualConfig::default();
        let mut learner = KernelContinualLearner::new(config, 12345);

        let task_id = learner.start_kernel_task(
            KernelLearningTask::SchedulerOptimization,
            alloc::string::String::from("Scheduler"),
        );

        learner.record_baseline(KernelLearningTask::SchedulerOptimization, 0.5);

        let params = vec![0.1; 1000];
        let gradients = vec![vec![0.01; 1000]];
        learner.end_kernel_task(&params, &gradients, 0.8);

        // Check for forgetting
        let forgetting = learner.detect_forgetting(0.1);
        assert!(forgetting.is_empty()); // No forgetting on first task
    }

    #[test]
    fn test_average_gradients() {
        let grads = vec![vec![1.0, 2.0, 3.0], vec![3.0, 2.0, 1.0]];

        let avg = average_gradients(&grads).unwrap();
        assert_eq!(avg, vec![2.0, 2.0, 2.0]);
    }
}
