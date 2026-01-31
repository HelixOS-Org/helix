//! Workload classification.

use alloc::vec::Vec;

use super::types::{TaskFeatures, WorkloadType};

// ============================================================================
// TASK CLASSIFIER
// ============================================================================

/// Workload classifier using decision boundaries
pub struct WorkloadClassifier {
    /// Classification thresholds
    thresholds: ClassificationThresholds,
    /// Classification history for learning
    history: Vec<(TaskFeatures, WorkloadType)>,
    /// Max history size
    max_history: usize,
    /// Enable learning
    learning_enabled: bool,
}

/// Classification thresholds (learned over time)
#[derive(Debug, Clone)]
struct ClassificationThresholds {
    cpu_bound_threshold: f64,
    io_bound_threshold: f64,
    memory_bound_threshold: f64,
    interactive_threshold: f64,
    realtime_variance_threshold: f64,
}

impl Default for ClassificationThresholds {
    fn default() -> Self {
        Self {
            cpu_bound_threshold: 0.8,
            io_bound_threshold: 0.3,
            memory_bound_threshold: 0.5,
            interactive_threshold: 1000.0,
            realtime_variance_threshold: 0.1,
        }
    }
}

impl WorkloadClassifier {
    /// Create new classifier
    pub fn new() -> Self {
        Self {
            thresholds: ClassificationThresholds::default(),
            history: Vec::new(),
            max_history: 10000,
            learning_enabled: true,
        }
    }

    /// Classify a task based on its features
    pub fn classify(&self, features: &TaskFeatures) -> WorkloadType {
        if features.runtime_variance < self.thresholds.realtime_variance_threshold
            && features.involuntary_switches < 10.0
        {
            return WorkloadType::RealTime;
        }

        if features.avg_runtime < self.thresholds.interactive_threshold
            && features.voluntary_switches > 50.0
        {
            return WorkloadType::Interactive;
        }

        let cpu_score = features.avg_cpu_usage;
        let io_score = features.io_ops_per_sec / 1000.0 + features.avg_io_wait / 100000.0;
        let memory_score = features.cache_miss_rate + features.memory_access_rate / 1000.0;

        if cpu_score > self.thresholds.cpu_bound_threshold && cpu_score > io_score * 2.0 {
            WorkloadType::CpuBound
        } else if io_score > self.thresholds.io_bound_threshold && io_score > cpu_score * 2.0 {
            WorkloadType::IoBound
        } else if memory_score > self.thresholds.memory_bound_threshold {
            WorkloadType::MemoryBound
        } else if features.sleep_frequency > 100.0 && features.avg_cpu_usage < 0.1 {
            WorkloadType::Background
        } else if cpu_score > 0.3 && io_score > 0.2 {
            WorkloadType::Mixed
        } else {
            WorkloadType::Unknown
        }
    }

    /// Classify with confidence score
    pub fn classify_with_confidence(&self, features: &TaskFeatures) -> (WorkloadType, f64) {
        let workload_type = self.classify(features);

        let confidence = match workload_type {
            WorkloadType::CpuBound => {
                let excess = features.avg_cpu_usage - self.thresholds.cpu_bound_threshold;
                0.7 + (excess * 1.5).min(0.3)
            },
            WorkloadType::IoBound => {
                let io_score = features.io_ops_per_sec / 1000.0 + features.avg_io_wait / 100000.0;
                let excess = io_score - self.thresholds.io_bound_threshold;
                0.7 + (excess * 1.5).min(0.3)
            },
            WorkloadType::Interactive => 0.9,
            WorkloadType::RealTime => 0.95,
            WorkloadType::Background => 0.8,
            WorkloadType::Mixed => 0.5,
            _ => 0.3,
        };

        (workload_type, confidence.clamp(0.0, 1.0))
    }

    /// Record classification for learning
    pub fn record_classification(&mut self, features: TaskFeatures, actual: WorkloadType) {
        if !self.learning_enabled {
            return;
        }

        self.history.push((features, actual));

        if self.history.len() > self.max_history {
            self.history.remove(0);
        }

        if self.history.len() % 100 == 0 {
            self.update_thresholds();
        }
    }

    /// Update classification thresholds based on history
    fn update_thresholds(&mut self) {
        if self.history.len() < 100 {
            return;
        }

        let cpu_bound: Vec<_> = self
            .history
            .iter()
            .filter(|(_, t)| *t == WorkloadType::CpuBound)
            .collect();

        if !cpu_bound.is_empty() {
            let avg_cpu: f64 = cpu_bound.iter().map(|(f, _)| f.avg_cpu_usage).sum::<f64>()
                / cpu_bound.len() as f64;
            self.thresholds.cpu_bound_threshold =
                0.9 * self.thresholds.cpu_bound_threshold + 0.1 * (avg_cpu * 0.9);
        }

        let io_bound: Vec<_> = self
            .history
            .iter()
            .filter(|(_, t)| *t == WorkloadType::IoBound)
            .collect();

        if !io_bound.is_empty() {
            let avg_io: f64 = io_bound
                .iter()
                .map(|(f, _)| f.io_ops_per_sec / 1000.0 + f.avg_io_wait / 100000.0)
                .sum::<f64>()
                / io_bound.len() as f64;
            self.thresholds.io_bound_threshold =
                0.9 * self.thresholds.io_bound_threshold + 0.1 * (avg_io * 0.8);
        }
    }

    /// Enable learning
    pub fn enable_learning(&mut self) {
        self.learning_enabled = true;
    }

    /// Disable learning
    pub fn disable_learning(&mut self) {
        self.learning_enabled = false;
    }

    /// Get history size
    pub fn history_size(&self) -> usize {
        self.history.len()
    }
}

impl Default for WorkloadClassifier {
    fn default() -> Self {
        Self::new()
    }
}
