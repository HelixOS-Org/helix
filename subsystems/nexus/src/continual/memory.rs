//! Memory replay system for continual learning.

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;

use crate::continual::utils::lcg_next;

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
    #[inline(always)]
    pub fn update_priority(&mut self, td_error: f64) {
        self.priority = libm::fabs(td_error) + 0.01; // Small epsilon for stability
        self.replay_count += 1;
    }
}

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
#[repr(align(64))]
pub struct MemoryBuffer {
    /// Stored samples
    pub samples: Vec<MemorySample>,
    /// Configuration
    pub config: ReplayConfig,
    /// Per-task sample counts
    pub task_counts: LinearMap<usize, 64>,
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
            task_counts: LinearMap::new(),
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
        self.task_counts.add(task_id, 1);
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
#[repr(align(64))]
pub struct BufferStats {
    pub total_samples: usize,
    pub num_tasks: usize,
    pub task_distribution: LinearMap<usize, 64>,
    pub avg_priority: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_sample() {
        let mut sample = MemorySample::new(vec![1.0, 2.0], vec![0.5], 0);
        assert_eq!(sample.priority, 1.0);

        sample.update_priority(0.5);
        assert!(sample.priority > 0.0);
        assert_eq!(sample.replay_count, 1);
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
}
