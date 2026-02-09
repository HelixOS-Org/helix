//! Continual Learning Manager.

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::continual::ewc::EwcLearner;
use crate::continual::gem::GemConstraint;
use crate::continual::memory::{MemoryBuffer, MemorySample, ReplayConfig};
use crate::continual::packnet::PackNet;
use crate::continual::progressive::ProgressiveNetwork;
use crate::continual::si::SynapticIntelligence;
use crate::continual::task::Task;
use crate::continual::types::ContinualStrategy;
use crate::continual::utils::average_gradients;

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
    pub forgetting: LinearMap<f64, 64>,
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
            }
            ContinualStrategy::SI => {
                manager.si = Some(SynapticIntelligence::new(config.param_count, config.si_c));
            }
            ContinualStrategy::GEM => {
                manager.gem = Some(GemConstraint::new(100, 0.1));
            }
            _ => {}
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
    #[inline]
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
    #[inline]
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
    #[inline]
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
    #[inline]
    pub fn average_forgetting(&self) -> f64 {
        if self.history.forgetting.is_empty() {
            return 0.0;
        }

        let sum: f64 = self.history.forgetting.values().sum();
        sum / self.history.forgetting.len() as f64
    }

    /// Get learning summary
    #[inline]
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
