//! # CORTEX Adaptive Learning
//!
//! This module enables CORTEX to learn from its decisions and improve over time.
//! Unlike traditional machine learning, this system is:
//!
//! - **Deterministic**: Same input always produces same output
//! - **Explainable**: Every decision has a traceable reasoning chain
//! - **Bounded**: Learning has strict time and memory limits
//! - **Verifiable**: Learned behaviors can be formally verified
//! - **Reversible**: Learning can be rolled back if it causes problems
//!
//! ## Learning Mechanisms
//!
//! 1. **Reinforcement**: Successful decisions are reinforced
//! 2. **Weakening**: Failed decisions are weakened
//! 3. **Generalization**: Patterns are extracted from specific cases
//! 4. **Specialization**: General rules are refined for specific cases
//! 5. **Pruning**: Unused or harmful rules are removed

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::Ordering as CmpOrdering;

use crate::neural::{Decision, DecisionId, Pattern, PatternId};
use crate::{CortexResult, DecisionAction, SubsystemId, Timestamp};

// =============================================================================
// FEEDBACK
// =============================================================================

/// Feedback type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackType {
    /// Decision was successful
    Success,

    /// Decision was partially successful
    Partial,

    /// Decision had no effect
    Neutral,

    /// Decision made things worse
    Negative,

    /// Decision caused failure
    Failure,
}

impl FeedbackType {
    /// Convert to numeric weight
    pub fn to_weight(&self) -> f64 {
        match self {
            Self::Success => 1.0,
            Self::Partial => 0.5,
            Self::Neutral => 0.0,
            Self::Negative => -0.5,
            Self::Failure => -1.0,
        }
    }
}

/// Decision feedback
#[derive(Clone)]
pub struct Feedback {
    /// Decision ID
    pub decision_id: DecisionId,

    /// Feedback type
    pub feedback_type: FeedbackType,

    /// Timestamp
    pub timestamp: Timestamp,

    /// Context at feedback time
    pub context: FeedbackContext,

    /// Explanation
    pub explanation: String,
}

/// Context at feedback time
#[derive(Clone, Default)]
pub struct FeedbackContext {
    /// Memory usage before decision
    pub memory_before: u64,

    /// Memory usage after decision
    pub memory_after: u64,

    /// CPU load before decision
    pub cpu_before: u8,

    /// CPU load after decision
    pub cpu_after: u8,

    /// Latency before (microseconds)
    pub latency_before_us: u64,

    /// Latency after (microseconds)
    pub latency_after_us: u64,

    /// Error count before
    pub errors_before: u32,

    /// Error count after
    pub errors_after: u32,
}

impl FeedbackContext {
    /// Calculate improvement score
    pub fn improvement_score(&self) -> f64 {
        let mut score = 0.0;

        // Memory improvement (lower is better)
        if self.memory_before > 0 {
            let memory_change = (self.memory_after as i64 - self.memory_before as i64) as f64
                / self.memory_before as f64;
            score -= memory_change * 0.3;
        }

        // CPU improvement (lower is better)
        if self.cpu_before > 0 {
            let cpu_change =
                (self.cpu_after as i64 - self.cpu_before as i64) as f64 / self.cpu_before as f64;
            score -= cpu_change * 0.3;
        }

        // Latency improvement (lower is better)
        if self.latency_before_us > 0 {
            let latency_change = (self.latency_after_us as i64 - self.latency_before_us as i64)
                as f64
                / self.latency_before_us as f64;
            score -= latency_change * 0.2;
        }

        // Error improvement (lower is better)
        if self.errors_before > 0 {
            let error_change = (self.errors_after as i64 - self.errors_before as i64) as f64
                / self.errors_before as f64;
            score -= error_change * 0.2;
        }

        score.clamp(-1.0, 1.0)
    }
}

// =============================================================================
// LEARNED RULE
// =============================================================================

/// Rule identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RuleId(pub u64);

/// Rule status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleStatus {
    /// Rule is being learned
    Learning,

    /// Rule is active and being used
    Active,

    /// Rule is being tested
    Testing,

    /// Rule is deprecated (will be removed)
    Deprecated,

    /// Rule is disabled
    Disabled,
}

/// A learned rule
#[derive(Clone)]
pub struct Rule {
    /// Rule ID
    pub id: RuleId,

    /// Rule name
    pub name: String,

    /// Pattern that triggers this rule
    pub trigger_pattern: Option<PatternId>,

    /// Condition (as expression string)
    pub condition: String,

    /// Action to take
    pub action: DecisionAction,

    /// Confidence (0.0 to 1.0)
    pub confidence: f64,

    /// Number of times used
    pub use_count: u64,

    /// Number of successful uses
    pub success_count: u64,

    /// Number of failed uses
    pub failure_count: u64,

    /// Status
    pub status: RuleStatus,

    /// Created timestamp
    pub created_at: Timestamp,

    /// Last used timestamp
    pub last_used: Timestamp,

    /// Last modified timestamp
    pub last_modified: Timestamp,

    /// Source (how was this rule created?)
    pub source: RuleSource,
}

/// How was a rule created?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleSource {
    /// Pre-defined by developer
    Builtin,

    /// Learned from feedback
    Learned,

    /// Generalized from specific rules
    Generalized,

    /// Specialized from general rules
    Specialized,

    /// Imported from external source
    Imported,
}

impl Rule {
    /// Create new rule
    pub fn new(id: RuleId, name: &str, condition: &str, action: DecisionAction) -> Self {
        Self {
            id,
            name: String::from(name),
            trigger_pattern: None,
            condition: String::from(condition),
            action,
            confidence: 0.5,
            use_count: 0,
            success_count: 0,
            failure_count: 0,
            status: RuleStatus::Learning,
            created_at: 0,
            last_used: 0,
            last_modified: 0,
            source: RuleSource::Learned,
        }
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        if self.use_count == 0 {
            0.5
        } else {
            self.success_count as f64 / self.use_count as f64
        }
    }

    /// Apply feedback
    pub fn apply_feedback(&mut self, feedback: &Feedback, timestamp: Timestamp) {
        self.use_count += 1;
        self.last_used = timestamp;
        self.last_modified = timestamp;

        match feedback.feedback_type {
            FeedbackType::Success => {
                self.success_count += 1;
                self.reinforce();
            },
            FeedbackType::Partial => {
                self.success_count += 1;
                // Smaller reinforcement
                self.confidence = (self.confidence + self.success_rate()) / 2.0;
            },
            FeedbackType::Neutral => {
                // No change
            },
            FeedbackType::Negative => {
                self.failure_count += 1;
                self.weaken();
            },
            FeedbackType::Failure => {
                self.failure_count += 1;
                self.weaken();
                self.weaken();
            },
        }

        // Update status based on performance
        self.update_status();
    }

    /// Reinforce rule (increase confidence)
    fn reinforce(&mut self) {
        // Asymptotic increase towards 1.0
        self.confidence = self.confidence + (1.0 - self.confidence) * 0.1;
    }

    /// Weaken rule (decrease confidence)
    fn weaken(&mut self) {
        // Decrease towards 0.0
        self.confidence = self.confidence * 0.9;
    }

    /// Update status based on performance
    fn update_status(&mut self) {
        // Promote from learning to active if enough successes
        if self.status == RuleStatus::Learning {
            if self.use_count >= 10 && self.success_rate() >= 0.7 {
                self.status = RuleStatus::Active;
            }
        }

        // Deprecate if too many failures
        if self.success_rate() < 0.3 && self.use_count >= 5 {
            self.status = RuleStatus::Deprecated;
        }

        // Disable if confidence too low
        if self.confidence < 0.1 {
            self.status = RuleStatus::Disabled;
        }
    }

    /// Is rule usable?
    pub fn is_usable(&self) -> bool {
        matches!(
            self.status,
            RuleStatus::Active | RuleStatus::Learning | RuleStatus::Testing
        )
    }
}

// =============================================================================
// EXPERIENCE
// =============================================================================

/// Experience entry (decision + outcome)
#[derive(Clone)]
pub struct Experience {
    /// Decision made
    pub decision: Decision,

    /// Context at decision time
    pub context: ExperienceContext,

    /// Feedback received
    pub feedback: Option<Feedback>,

    /// Timestamp
    pub timestamp: Timestamp,
}

/// Context at decision time
#[derive(Clone, Default)]
pub struct ExperienceContext {
    /// Active patterns
    pub patterns: Vec<PatternId>,

    /// System state snapshot
    pub state: SystemStateSnapshot,

    /// Subsystem involved
    pub subsystem: Option<SubsystemId>,
}

/// System state snapshot
#[derive(Clone, Default)]
pub struct SystemStateSnapshot {
    /// Memory usage (bytes)
    pub memory_used: u64,

    /// CPU load (percent)
    pub cpu_load: u8,

    /// I/O wait (percent)
    pub io_wait: u8,

    /// Active threads
    pub active_threads: u32,

    /// Queue depths
    pub queue_depths: Vec<u32>,

    /// Pending interrupts
    pub pending_interrupts: u32,
}

// =============================================================================
// PATTERN LEARNER
// =============================================================================

/// Pattern occurrence counter
#[derive(Clone, Default)]
pub struct PatternCounter {
    /// Occurrences
    pub count: u64,

    /// Successful outcomes
    pub successes: u64,

    /// Failed outcomes
    pub failures: u64,

    /// Average time between occurrences
    pub avg_interval: u64,

    /// Last occurrence
    pub last_seen: Timestamp,
}

/// Pattern learner
pub struct PatternLearner {
    /// Pattern occurrences
    pattern_counts: BTreeMap<PatternId, PatternCounter>,

    /// Pattern sequences (pattern A followed by pattern B)
    sequences: BTreeMap<(PatternId, PatternId), u64>,

    /// Pattern correlations (patterns that appear together)
    correlations: BTreeMap<(PatternId, PatternId), f64>,

    /// Minimum occurrences for pattern to be considered significant
    min_occurrences: u64,

    /// Correlation threshold
    correlation_threshold: f64,
}

impl PatternLearner {
    /// Create new pattern learner
    pub fn new() -> Self {
        Self {
            pattern_counts: BTreeMap::new(),
            sequences: BTreeMap::new(),
            correlations: BTreeMap::new(),
            min_occurrences: 5,
            correlation_threshold: 0.7,
        }
    }

    /// Record pattern occurrence
    pub fn record(
        &mut self,
        pattern: PatternId,
        timestamp: Timestamp,
        previous: Option<PatternId>,
    ) {
        let counter = self.pattern_counts.entry(pattern).or_default();

        // Update interval
        if counter.last_seen > 0 {
            let interval = timestamp - counter.last_seen;
            counter.avg_interval =
                (counter.avg_interval * counter.count + interval) / (counter.count + 1);
        }

        counter.count += 1;
        counter.last_seen = timestamp;

        // Record sequence
        if let Some(prev) = previous {
            *self.sequences.entry((prev, pattern)).or_default() += 1;
        }
    }

    /// Record outcome
    pub fn record_outcome(&mut self, pattern: PatternId, success: bool) {
        if let Some(counter) = self.pattern_counts.get_mut(&pattern) {
            if success {
                counter.successes += 1;
            } else {
                counter.failures += 1;
            }
        }
    }

    /// Find significant patterns
    pub fn significant_patterns(&self) -> Vec<PatternId> {
        self.pattern_counts
            .iter()
            .filter(|(_, c)| c.count >= self.min_occurrences)
            .map(|(p, _)| *p)
            .collect()
    }

    /// Find patterns that predict another pattern
    pub fn predictors(&self, target: PatternId) -> Vec<(PatternId, f64)> {
        let mut predictors = Vec::new();

        for (&(prev, next), &count) in &self.sequences {
            if next == target {
                if let Some(prev_counter) = self.pattern_counts.get(&prev) {
                    if prev_counter.count >= self.min_occurrences {
                        let probability = count as f64 / prev_counter.count as f64;
                        if probability >= 0.3 {
                            predictors.push((prev, probability));
                        }
                    }
                }
            }
        }

        predictors.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(CmpOrdering::Equal));
        predictors
    }

    /// Get pattern statistics
    pub fn stats(&self, pattern: PatternId) -> Option<&PatternCounter> {
        self.pattern_counts.get(&pattern)
    }
}

impl Default for PatternLearner {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// ADAPTIVE LEARNER
// =============================================================================

/// Learning configuration
#[derive(Clone)]
pub struct LearningConfig {
    /// Enable learning
    pub enabled: bool,

    /// Maximum rules
    pub max_rules: usize,

    /// Maximum experience entries
    pub max_experiences: usize,

    /// Minimum confidence for rule activation
    pub min_confidence: f64,

    /// Learning rate
    pub learning_rate: f64,

    /// Pruning threshold (rules below this success rate are removed)
    pub prune_threshold: f64,

    /// Generalization threshold (how many similar cases before generalizing)
    pub generalization_threshold: usize,
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_rules: 1000,
            max_experiences: 10000,
            min_confidence: 0.6,
            learning_rate: 0.1,
            prune_threshold: 0.2,
            generalization_threshold: 5,
        }
    }
}

/// Adaptive learner
pub struct AdaptiveLearner {
    /// Configuration
    config: LearningConfig,

    /// Learned rules
    rules: BTreeMap<RuleId, Rule>,

    /// Next rule ID
    next_rule_id: u64,

    /// Experience buffer
    experiences: Vec<Experience>,

    /// Pattern learner
    pattern_learner: PatternLearner,

    /// Pending decisions (awaiting feedback)
    pending: BTreeMap<DecisionId, (Decision, ExperienceContext, Timestamp)>,

    /// Total decisions
    total_decisions: u64,

    /// Total feedback received
    total_feedback: u64,

    /// Rules created
    rules_created: u64,

    /// Rules pruned
    rules_pruned: u64,
}

impl AdaptiveLearner {
    /// Create new learner
    pub fn new(config: LearningConfig) -> Self {
        Self {
            config,
            rules: BTreeMap::new(),
            next_rule_id: 1,
            experiences: Vec::new(),
            pattern_learner: PatternLearner::new(),
            pending: BTreeMap::new(),
            total_decisions: 0,
            total_feedback: 0,
            rules_created: 0,
            rules_pruned: 0,
        }
    }

    /// Record a decision
    pub fn record_decision(
        &mut self,
        decision: Decision,
        context: ExperienceContext,
        timestamp: Timestamp,
    ) {
        if !self.config.enabled {
            return;
        }

        self.total_decisions += 1;

        // Store pending decision
        self.pending
            .insert(decision.id, (decision.clone(), context.clone(), timestamp));

        // Record patterns
        for (i, &pattern) in context.patterns.iter().enumerate() {
            let prev = if i > 0 {
                Some(context.patterns[i - 1])
            } else {
                None
            };
            self.pattern_learner.record(pattern, timestamp, prev);
        }
    }

    /// Record feedback for a decision
    pub fn record_feedback(
        &mut self,
        decision_id: DecisionId,
        feedback_type: FeedbackType,
        feedback_context: FeedbackContext,
        explanation: &str,
        timestamp: Timestamp,
    ) -> CortexResult {
        if !self.config.enabled {
            return CortexResult::Ignored;
        }

        self.total_feedback += 1;

        // Find pending decision
        let (decision, context, decision_time) = match self.pending.remove(&decision_id) {
            Some(d) => d,
            None => return CortexResult::Ignored,
        };

        let feedback = Feedback {
            decision_id,
            feedback_type,
            timestamp,
            context: feedback_context,
            explanation: String::from(explanation),
        };

        // Create experience
        let experience = Experience {
            decision: decision.clone(),
            context: context.clone(),
            feedback: Some(feedback.clone()),
            timestamp: decision_time,
        };

        // Store experience
        if self.experiences.len() >= self.config.max_experiences {
            self.experiences.remove(0);
        }
        self.experiences.push(experience);

        // Update pattern outcomes
        for pattern in &context.patterns {
            let success = matches!(feedback_type, FeedbackType::Success | FeedbackType::Partial);
            self.pattern_learner.record_outcome(*pattern, success);
        }

        // Update rules
        self.update_rules(&decision, &feedback, timestamp);

        // Try to learn new rules
        self.try_learn_rule(&decision, &context, &feedback, timestamp);

        // Prune bad rules
        self.prune_rules();

        CortexResult::Observed
    }

    /// Update existing rules based on feedback
    fn update_rules(&mut self, decision: &Decision, feedback: &Feedback, timestamp: Timestamp) {
        // Find rules that produced this action
        for rule in self.rules.values_mut() {
            if rule.action == decision.action && rule.is_usable() {
                rule.apply_feedback(feedback, timestamp);
            }
        }
    }

    /// Try to learn a new rule from experience
    fn try_learn_rule(
        &mut self,
        decision: &Decision,
        context: &ExperienceContext,
        feedback: &Feedback,
        timestamp: Timestamp,
    ) {
        // Only learn from successes
        if !matches!(
            feedback.feedback_type,
            FeedbackType::Success | FeedbackType::Partial
        ) {
            return;
        }

        // Check if we already have a rule for this
        let existing = self.rules.values().any(|r| {
            r.action == decision.action
                && r.trigger_pattern
                    .map_or(false, |p| context.patterns.contains(&p))
        });

        if existing {
            return;
        }

        // Check if we've seen similar patterns enough times
        let significant_patterns: Vec<_> = context
            .patterns
            .iter()
            .filter(|p| {
                self.pattern_learner.stats(**p).map_or(false, |s| {
                    s.count >= self.config.generalization_threshold as u64
                })
            })
            .collect();

        if significant_patterns.is_empty() {
            return;
        }

        // Create new rule
        if self.rules.len() < self.config.max_rules {
            let id = RuleId(self.next_rule_id);
            self.next_rule_id += 1;

            let mut rule = Rule::new(
                id,
                &alloc::format!("learned_rule_{}", id.0),
                "pattern_match",
                decision.action.clone(),
            );

            rule.trigger_pattern = significant_patterns.first().copied().copied();
            rule.created_at = timestamp;
            rule.last_modified = timestamp;
            rule.confidence = 0.6;
            rule.success_count = 1;
            rule.use_count = 1;

            self.rules.insert(id, rule);
            self.rules_created += 1;
        }
    }

    /// Prune underperforming rules
    fn prune_rules(&mut self) {
        let to_remove: Vec<_> = self
            .rules
            .iter()
            .filter(|(_, r)| r.use_count >= 10 && r.success_rate() < self.config.prune_threshold)
            .map(|(id, _)| *id)
            .collect();

        for id in to_remove {
            self.rules.remove(&id);
            self.rules_pruned += 1;
        }
    }

    /// Get applicable rules for a decision
    pub fn get_rules(&self, patterns: &[PatternId]) -> Vec<&Rule> {
        self.rules
            .values()
            .filter(|r| {
                r.is_usable()
                    && r.confidence >= self.config.min_confidence
                    && r.trigger_pattern.map_or(true, |p| patterns.contains(&p))
            })
            .collect()
    }

    /// Get best action for patterns
    pub fn suggest_action(&self, patterns: &[PatternId]) -> Option<(DecisionAction, f64)> {
        let rules = self.get_rules(patterns);

        // Find highest confidence rule
        rules
            .iter()
            .max_by(|a, b| {
                a.confidence
                    .partial_cmp(&b.confidence)
                    .unwrap_or(CmpOrdering::Equal)
            })
            .map(|r| (r.action.clone(), r.confidence))
    }

    /// Get statistics
    pub fn stats(&self) -> LearningStats {
        LearningStats {
            total_decisions: self.total_decisions,
            total_feedback: self.total_feedback,
            active_rules: self
                .rules
                .values()
                .filter(|r| r.status == RuleStatus::Active)
                .count(),
            learning_rules: self
                .rules
                .values()
                .filter(|r| r.status == RuleStatus::Learning)
                .count(),
            deprecated_rules: self
                .rules
                .values()
                .filter(|r| r.status == RuleStatus::Deprecated)
                .count(),
            rules_created: self.rules_created,
            rules_pruned: self.rules_pruned,
            experience_count: self.experiences.len(),
            pending_count: self.pending.len(),
            avg_success_rate: self
                .rules
                .values()
                .filter(|r| r.use_count > 0)
                .map(|r| r.success_rate())
                .sum::<f64>()
                / self.rules.len().max(1) as f64,
        }
    }

    /// Export rules (for persistence)
    pub fn export_rules(&self) -> Vec<Rule> {
        self.rules
            .values()
            .filter(|r| r.status == RuleStatus::Active)
            .cloned()
            .collect()
    }

    /// Import rules (from persistence)
    pub fn import_rules(&mut self, rules: Vec<Rule>) {
        for rule in rules {
            let id = RuleId(self.next_rule_id);
            self.next_rule_id += 1;

            let mut imported = rule;
            imported.id = id;
            imported.source = RuleSource::Imported;

            self.rules.insert(id, imported);
        }
    }
}

impl Default for AdaptiveLearner {
    fn default() -> Self {
        Self::new(LearningConfig::default())
    }
}

/// Learning statistics
#[derive(Debug, Clone, Default)]
pub struct LearningStats {
    /// Total decisions recorded
    pub total_decisions: u64,

    /// Total feedback received
    pub total_feedback: u64,

    /// Active rules
    pub active_rules: usize,

    /// Rules being learned
    pub learning_rules: usize,

    /// Deprecated rules
    pub deprecated_rules: usize,

    /// Total rules created
    pub rules_created: u64,

    /// Total rules pruned
    pub rules_pruned: u64,

    /// Experience buffer size
    pub experience_count: usize,

    /// Pending decisions
    pub pending_count: usize,

    /// Average success rate
    pub avg_success_rate: f64,
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feedback_weight() {
        assert_eq!(FeedbackType::Success.to_weight(), 1.0);
        assert_eq!(FeedbackType::Failure.to_weight(), -1.0);
    }

    #[test]
    fn test_rule_success_rate() {
        let mut rule = Rule::new(RuleId(1), "test", "true", DecisionAction::NoOp);

        assert_eq!(rule.success_rate(), 0.5);

        rule.use_count = 10;
        rule.success_count = 7;

        assert_eq!(rule.success_rate(), 0.7);
    }

    #[test]
    fn test_pattern_learner() {
        let mut learner = PatternLearner::new();

        learner.record(PatternId(1), 100, None);
        learner.record(PatternId(2), 200, Some(PatternId(1)));
        learner.record(PatternId(2), 300, Some(PatternId(1)));

        let stats = learner.stats(PatternId(1));
        assert!(stats.is_some());
        assert_eq!(stats.unwrap().count, 1);
    }

    #[test]
    fn test_improvement_score() {
        let context = FeedbackContext {
            memory_before: 1000,
            memory_after: 800, // 20% improvement
            cpu_before: 80,
            cpu_after: 60, // 25% improvement
            latency_before_us: 100,
            latency_after_us: 50, // 50% improvement
            errors_before: 10,
            errors_after: 5, // 50% improvement
        };

        let score = context.improvement_score();
        assert!(score > 0.0); // Should be positive (improvement)
    }
}
