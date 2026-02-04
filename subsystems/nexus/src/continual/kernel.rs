//! Kernel-specific continual learning integration.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use crate::continual::manager::{ContinualConfig, ContinualLearningManager};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_continual_learner() {
        let config = ContinualConfig::default();
        let mut learner = KernelContinualLearner::new(config, 12345);

        let _task_id = learner.start_kernel_task(
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
}
