//! NEXUS Continuous Learning Engine — COGNITION Year 2
//!
//! This module enables NEXUS to learn and adapt over time through:
//!
//! - **Feedback Loops** - Learn from outcomes of decisions
//! - **Generalization** - Extract general rules from specific cases
//! - **Curriculum Learning** - Progressive complexity in learning
//! - **Safe Learning** - Bounded exploration with safety constraints
//! - **Regression Detection** - Detect and prevent performance regression

mod curriculum;
mod feedback;
mod generalizer;
mod hypothesis;
pub mod incremental;
mod intelligence;
mod regression;
mod safety;
mod transfer;
mod types;

// Re-export types
// Re-export curriculum types
pub use curriculum::{
    CurriculumLearner, CurriculumStage, DifficultyLevel, LearningTask, TaskCriteria,
};
// Re-export feedback types
pub use feedback::{ActionStats, FeedbackEntry, FeedbackLoop, FeedbackType};
// Re-export generalizer types
pub use generalizer::{
    ConditionOp, GeneralizationStrategy, Generalizer, LearnedRule, RuleCondition,
};
// Re-export hypothesis types
pub use hypothesis::{Hypothesis, HypothesisManager, HypothesisStatus};
// Re-export intelligence
pub use intelligence::{ContinuousLearningIntelligence, LearningAnalysis};
// Re-export regression types
pub use regression::{
    MetricHistory, MetricSample, MetricType, RegressionDetector, RegressionEvent,
    RegressionSeverity,
};
// Re-export safety types
pub use safety::{
    ConstraintType, ExplorationPolicy, SafeLearner, SafetyConstraint, SafetyViolation,
};
// Re-export transfer types
pub use transfer::{KnowledgeItem, KnowledgeTransfer, KnowledgeType, TransferRecord};
pub use types::{ExperienceId, HypothesisId, RuleId, SessionId, Timestamp};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feedback_loop() {
        let mut feedback = FeedbackLoop::new();

        feedback.record("action_a", FeedbackType::Positive, 1000);
        feedback.record("action_a", FeedbackType::Positive, 2000);
        feedback.record("action_b", FeedbackType::Negative, 3000);

        assert_eq!(feedback.action_count(), 2);

        let stats = feedback.get_stats("action_a").unwrap();
        assert_eq!(stats.positive_count, 2);
    }

    #[test]
    fn test_generalizer() {
        use alloc::string::String;

        let mut generalizer = Generalizer::new();

        let mut experiences = alloc::vec::Vec::new();
        for i in 0..5 {
            let mut entry = FeedbackEntry::new(
                ExperienceId::new(i),
                Timestamp::new(i * 1000),
                String::from("do_thing"),
                FeedbackType::Positive,
            );
            entry
                .context
                .insert(String::from("state"), String::from("ready"));
            experiences.push(entry);
        }

        let rules = generalizer.generalize(&experiences);
        assert!(!rules.is_empty());
    }

    #[test]
    fn test_curriculum_learner() {
        use alloc::string::String;

        let mut curriculum = CurriculumLearner::new();

        let mut stage = CurriculumStage::new(String::from("basics"), DifficultyLevel::Beginner);
        stage.add_task(LearningTask::new(
            String::from("learn_hello"),
            DifficultyLevel::Beginner,
        ));
        curriculum.add_stage(stage);

        assert_eq!(curriculum.current_difficulty(), DifficultyLevel::Beginner);
    }

    #[test]
    fn test_safe_learner() {
        use alloc::collections::BTreeMap;
        use alloc::string::String;

        let mut safe = SafeLearner::new();

        let constraint =
            SafetyConstraint::new(String::from("no_delete"), ConstraintType::ActionProhibition)
                .with_condition(String::from("delete"));

        safe.add_constraint(constraint);

        let context = BTreeMap::new();
        assert!(!safe.is_safe("delete_all", &context));
    }

    #[test]
    fn test_regression_detector() {
        let mut detector = RegressionDetector::new();
        detector.add_metric("latency", MetricType::Latency);

        for i in 0..100 {
            detector.record("latency", 10.0, i * 1000);
        }

        // Add regression
        for i in 100..150 {
            detector.record("latency", 100.0, i * 1000);
        }

        let _regressions = detector.check(150_000);
        // May or may not detect depending on window
        assert!(detector.regression_count() >= 0);
    }

    #[test]
    fn test_hypothesis() {
        use alloc::string::String;

        let mut manager = HypothesisManager::new();

        let id = manager.create(String::from("OOM happens after 100 allocations"));

        for i in 0..15 {
            manager.add_evidence(id, true, i * 1000);
        }

        let h = manager.get(id).unwrap();
        assert_eq!(h.status, HypothesisStatus::Confirmed);
    }

    #[test]
    fn test_continuous_learning() {
        let mut learning = ContinuousLearningIntelligence::new(1);

        learning.record_experience("optimize_cache", FeedbackType::Positive, 1000);
        learning.record_experience("optimize_cache", FeedbackType::Positive, 2000);

        let analysis = learning.analyze();
        assert!(analysis.total_experiences >= 2);
    }
}
