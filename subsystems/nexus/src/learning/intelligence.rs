//! Continuous learning intelligence engine
//!
//! This module provides the main learning intelligence.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::AtomicBool;

use super::curriculum::{CurriculumLearner, LessonDifficulty};
use super::feedback::{FeedbackEntry, FeedbackLoop, FeedbackType};
use super::generalizer::Generalizer;
use super::hypothesis::HypothesisManager;
use super::regression::{RegressionDetector, RegressionEvent};
use super::safety::SafeLearner;
use super::transfer::KnowledgeTransfer;
use super::types::{ExperienceId, HypothesisId, RuleId, SessionId, Timestamp};

/// Learning analysis
#[derive(Debug, Clone)]
pub struct LearningAnalysis {
    /// Total experiences
    pub total_experiences: u64,
    /// Total rules learned
    pub total_rules: u64,
    /// Reliable rules
    pub reliable_rules: u64,
    /// Curriculum progress
    pub curriculum_progress: f32,
    /// Current difficulty
    pub current_difficulty: DifficultyLevel,
    /// Safety violations
    pub safety_violations: u64,
    /// Regressions detected
    pub regressions: u64,
    /// Hypotheses confirmed
    pub hypotheses_confirmed: u64,
    /// Knowledge items
    pub knowledge_items: u64,
    /// Learning health score
    pub health_score: f32,
}

/// Continuous learning intelligence
pub struct ContinuousLearningIntelligence {
    /// Feedback loop
    feedback: FeedbackLoop,
    /// Generalizer
    generalizer: Generalizer,
    /// Curriculum learner
    curriculum: CurriculumLearner,
    /// Safe learner
    safe_learner: SafeLearner,
    /// Regression detector
    regression: RegressionDetector,
    /// Hypothesis manager
    hypotheses: HypothesisManager,
    /// Knowledge transfer
    knowledge: KnowledgeTransfer,
    /// Enabled
    enabled: AtomicBool,
    /// Learning session
    session: SessionId,
}

impl ContinuousLearningIntelligence {
    /// Create new learning intelligence
    pub fn new(session_id: u64) -> Self {
        Self {
            feedback: FeedbackLoop::new(),
            generalizer: Generalizer::new(),
            curriculum: CurriculumLearner::new(),
            safe_learner: SafeLearner::new(),
            regression: RegressionDetector::new(),
            hypotheses: HypothesisManager::new(),
            knowledge: KnowledgeTransfer::new(),
            enabled: AtomicBool::new(true),
            session: SessionId::new(session_id),
        }
    }

    /// Record experience
    pub fn record_experience(
        &mut self,
        action: &str,
        feedback_type: FeedbackType,
        timestamp: u64,
    ) -> ExperienceId {
        self.feedback.record(action, feedback_type, timestamp)
    }

    /// Record with context
    pub fn record_with_context(
        &mut self,
        action: &str,
        context: BTreeMap<String, String>,
        feedback_type: FeedbackType,
        timestamp: u64,
    ) -> ExperienceId {
        let ts = Timestamp::new(timestamp);
        let entry = FeedbackEntry::new(
            ExperienceId::new(0), // Will be set by record_full
            ts,
            String::from(action),
            feedback_type,
        );
        let entry = entry.with_context("_raw", "true");
        // Add all context
        let mut updated = entry;
        for (k, v) in context {
            updated.context.insert(k, v);
        }
        self.feedback.record_full(updated)
    }

    /// Learn from recent experiences
    pub fn learn(&mut self) -> Vec<RuleId> {
        // Get recent experiences
        let recent = self.feedback.recent(100);

        // Generalize
        self.generalizer.generalize(recent)
    }

    /// Find best action
    pub fn best_action(
        &self,
        candidates: &[String],
        context: &BTreeMap<String, String>,
    ) -> Option<String> {
        // First check rules
        let matching_rules = self.generalizer.find_matching(context);
        if let Some(rule) = matching_rules
            .iter()
            .filter(|r| r.is_reliable())
            .max_by(|a, b| {
                a.confidence
                    .partial_cmp(&b.confidence)
                    .unwrap_or(core::cmp::Ordering::Equal)
            })
        {
            if candidates.contains(&rule.action) {
                return Some(rule.action.clone());
            }
        }

        // Fall back to feedback-based selection
        self.feedback.best_action(candidates).cloned()
    }

    /// Check if action is safe
    pub fn is_safe(&self, action: &str, context: &BTreeMap<String, String>) -> bool {
        self.safe_learner.is_safe(action, context)
    }

    /// Record metric
    pub fn record_metric(&mut self, name: &str, value: f32, timestamp: u64) {
        self.regression.record(name, value, timestamp);
    }

    /// Check for regressions
    pub fn check_regressions(&mut self, timestamp: u64) -> Vec<RegressionEvent> {
        self.regression.check(timestamp)
    }

    /// Create hypothesis
    pub fn create_hypothesis(&mut self, statement: &str) -> HypothesisId {
        self.hypotheses.create(String::from(statement))
    }

    /// Test hypothesis
    pub fn test_hypothesis(&mut self, id: HypothesisId, supports: bool, timestamp: u64) {
        self.hypotheses.add_evidence(id, supports, timestamp);
    }

    /// Get analysis
    pub fn analyze(&self) -> LearningAnalysis {
        let reliable_rules = self.generalizer.find_reliable().len() as u64;
        let total_rules = self.generalizer.count() as u64;

        let health_score = if total_rules > 0 {
            (reliable_rules as f32 / total_rules as f32) * 50.0
                + self.curriculum.overall_progress() * 30.0
                + (1.0 - (self.safe_learner.violation_count() as f32 / 100.0).min(1.0)) * 20.0
        } else {
            50.0
        };

        LearningAnalysis {
            total_experiences: self.feedback.history_len() as u64,
            total_rules,
            reliable_rules,
            curriculum_progress: self.curriculum.overall_progress(),
            current_difficulty: self.curriculum.current_difficulty(),
            safety_violations: self.safe_learner.violation_count() as u64,
            regressions: self.regression.regression_count() as u64,
            hypotheses_confirmed: self.hypotheses.find_confirmed().len() as u64,
            knowledge_items: self.knowledge.count() as u64,
            health_score,
        }
    }

    /// Get feedback loop
    pub fn feedback(&self) -> &FeedbackLoop {
        &self.feedback
    }

    /// Get generalizer
    pub fn generalizer(&self) -> &Generalizer {
        &self.generalizer
    }

    /// Get curriculum
    pub fn curriculum(&self) -> &CurriculumLearner {
        &self.curriculum
    }

    /// Get safe learner
    pub fn safe_learner(&self) -> &SafeLearner {
        &self.safe_learner
    }
}
