//! Central scheduler intelligence coordinator.

use super::affinity::AffinityPredictor;
use super::classifier::WorkloadClassifier;
use super::load::LoadPredictor;
use super::preemption::PreemptionIntelligence;
use super::priority::PriorityLearner;
use super::types::{TaskFeatures, WorkloadType};

// ============================================================================
// SCHEDULER INTELLIGENCE
// ============================================================================

/// Central scheduler intelligence coordinator
pub struct SchedulerIntelligence {
    /// Workload classifier
    classifier: WorkloadClassifier,
    /// Priority learner
    priority_learner: PriorityLearner,
    /// Affinity predictor
    affinity_predictor: AffinityPredictor,
    /// Preemption intelligence
    preemption: PreemptionIntelligence,
    /// Load predictor
    load_predictor: LoadPredictor,
    /// Enabled features
    enabled: SchedulerFeatures,
    /// Statistics
    stats: SchedulerStats,
}

/// Enabled scheduler intelligence features
#[derive(Debug, Clone)]
pub struct SchedulerFeatures {
    /// Enable workload classification
    pub classification: bool,
    /// Enable priority learning
    pub priority_learning: bool,
    /// Enable affinity prediction
    pub affinity_prediction: bool,
    /// Enable preemption intelligence
    pub preemption_intelligence: bool,
    /// Enable load prediction
    pub load_prediction: bool,
}

impl Default for SchedulerFeatures {
    fn default() -> Self {
        Self {
            classification: true,
            priority_learning: true,
            affinity_prediction: true,
            preemption_intelligence: true,
            load_prediction: true,
        }
    }
}

/// Scheduler statistics
#[derive(Debug, Clone, Default)]
pub struct SchedulerStats {
    /// Tasks classified
    pub tasks_classified: u64,
    /// Priority adjustments made
    pub priority_adjustments: u64,
    /// Affinity predictions
    pub affinity_predictions: u64,
    /// Preemption decisions
    pub preemption_decisions: u64,
    /// Load predictions
    pub load_predictions: u64,
}

impl SchedulerIntelligence {
    /// Create new scheduler intelligence
    pub fn new(num_cores: usize) -> Self {
        Self {
            classifier: WorkloadClassifier::new(),
            priority_learner: PriorityLearner::new(),
            affinity_predictor: AffinityPredictor::new(num_cores),
            preemption: PreemptionIntelligence::new(),
            load_predictor: LoadPredictor::new(),
            enabled: SchedulerFeatures::default(),
            stats: SchedulerStats::default(),
        }
    }

    /// Classify task workload
    pub fn classify_task(&mut self, features: &TaskFeatures) -> WorkloadType {
        if !self.enabled.classification {
            return WorkloadType::Unknown;
        }

        self.stats.tasks_classified += 1;
        self.classifier.classify(features)
    }

    /// Get priority adjustment
    pub fn get_priority_adjustment(&mut self, task_hash: u64, features: &TaskFeatures) -> i32 {
        if !self.enabled.priority_learning {
            return 0;
        }

        self.stats.priority_adjustments += 1;
        self.priority_learner.get_adjustment(task_hash, features)
    }

    /// Predict best core
    pub fn predict_core(&mut self, task_hash: u64, features: &TaskFeatures) -> usize {
        if !self.enabled.affinity_prediction {
            return 0;
        }

        self.stats.affinity_predictions += 1;
        self.affinity_predictor
            .predict_best_core(task_hash, features)
    }

    /// Should preempt?
    pub fn should_preempt(
        &mut self,
        current_type: WorkloadType,
        current_runtime: u64,
        current_remaining: u64,
        incoming_type: WorkloadType,
        incoming_priority: i32,
        current_priority: i32,
    ) -> bool {
        if !self.enabled.preemption_intelligence {
            return incoming_priority > current_priority;
        }

        self.stats.preemption_decisions += 1;
        self.preemption.should_preempt(
            current_type,
            current_runtime,
            current_remaining,
            incoming_type,
            incoming_priority,
            current_priority,
        )
    }

    /// Predict future load
    pub fn predict_load(&mut self, hour: u8) -> f64 {
        if !self.enabled.load_prediction {
            return 0.5;
        }

        self.stats.load_predictions += 1;
        self.load_predictor.predict(hour)
    }

    /// Set features
    pub fn set_features(&mut self, features: SchedulerFeatures) {
        self.enabled = features;
    }

    /// Get statistics
    pub fn stats(&self) -> &SchedulerStats {
        &self.stats
    }

    /// Get classifier
    pub fn classifier(&self) -> &WorkloadClassifier {
        &self.classifier
    }

    /// Get mutable classifier
    pub fn classifier_mut(&mut self) -> &mut WorkloadClassifier {
        &mut self.classifier
    }

    /// Get priority learner
    pub fn priority_learner(&self) -> &PriorityLearner {
        &self.priority_learner
    }

    /// Get mutable priority learner
    pub fn priority_learner_mut(&mut self) -> &mut PriorityLearner {
        &mut self.priority_learner
    }

    /// Get affinity predictor
    pub fn affinity_predictor(&self) -> &AffinityPredictor {
        &self.affinity_predictor
    }

    /// Get mutable affinity predictor
    pub fn affinity_predictor_mut(&mut self) -> &mut AffinityPredictor {
        &mut self.affinity_predictor
    }

    /// Get load predictor
    pub fn load_predictor(&self) -> &LoadPredictor {
        &self.load_predictor
    }

    /// Get mutable load predictor
    pub fn load_predictor_mut(&mut self) -> &mut LoadPredictor {
        &mut self.load_predictor
    }
}
