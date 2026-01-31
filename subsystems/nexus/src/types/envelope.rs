//! Domain Envelope Types
//!
//! Types for passing data between cognitive domains.

#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;

use super::identifiers::*;
use super::temporal::{Duration, Timestamp};
use super::confidence::Confidence;
use super::severity::Priority;
use super::metrics::MetricUnit;
use super::tags::Tags;

// ============================================================================
// SIGNAL (from SENSE)
// ============================================================================

/// Signal from perception domain
#[derive(Debug, Clone)]
pub struct Signal {
    /// Signal ID
    pub id: SignalId,
    /// Source probe
    pub probe: ProbeId,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Raw value
    pub value: f64,
    /// Unit
    pub unit: MetricUnit,
    /// Quality (0-1, for sensor reliability)
    pub quality: f32,
}

impl Signal {
    /// Create new signal
    pub fn new(probe: ProbeId, value: f64, unit: MetricUnit) -> Self {
        Self {
            id: SignalId::generate(),
            probe,
            timestamp: Timestamp::now(),
            value,
            unit,
            quality: 1.0,
        }
    }

    /// With quality
    pub fn with_quality(mut self, quality: f32) -> Self {
        self.quality = quality.clamp(0.0, 1.0);
        self
    }

    /// Is high quality signal?
    pub fn is_high_quality(&self) -> bool {
        self.quality >= 0.9
    }
}

// ============================================================================
// KNOWLEDGE (from UNDERSTAND)
// ============================================================================

/// Knowledge from comprehension domain
#[derive(Debug, Clone)]
pub struct Knowledge {
    /// Knowledge ID
    pub id: KnowledgeId,
    /// Source pattern/model
    pub source: Option<PatternId>,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Confidence
    pub confidence: Confidence,
    /// Tags
    pub tags: Tags,
    /// Content type
    pub content_type: KnowledgeType,
}

/// Type of knowledge
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnowledgeType {
    /// A detected pattern
    Pattern,
    /// A feature extracted from data
    Feature,
    /// A classification result
    Classification,
    /// An anomaly detection
    Anomaly,
    /// A trend observation
    Trend,
    /// A correlation
    Correlation,
}

impl Knowledge {
    /// Create new knowledge
    pub fn new(content_type: KnowledgeType, confidence: Confidence) -> Self {
        Self {
            id: KnowledgeId::generate(),
            source: None,
            timestamp: Timestamp::now(),
            confidence,
            tags: Tags::new(),
            content_type,
        }
    }

    /// With source pattern
    pub fn with_source(mut self, pattern: PatternId) -> Self {
        self.source = Some(pattern);
        self
    }

    /// With tags
    pub fn with_tags(mut self, tags: Tags) -> Self {
        self.tags = tags;
        self
    }
}

// ============================================================================
// CONCLUSION (from REASON)
// ============================================================================

/// Conclusion from reasoning domain
#[derive(Debug, Clone)]
pub struct Conclusion {
    /// Conclusion ID
    pub id: ConclusionId,
    /// Type of conclusion
    pub conclusion_type: ConclusionType,
    /// Confidence
    pub confidence: Confidence,
    /// Supporting evidence
    pub evidence: Vec<KnowledgeId>,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Description
    pub description: String,
}

/// Types of conclusions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConclusionType {
    /// Causal relationship identified
    Causal,
    /// Prediction of future state
    Prediction,
    /// Anomaly confirmed
    Anomaly,
    /// Trend identified
    Trend,
    /// Hypothesis formed
    Hypothesis,
    /// Root cause identified
    RootCause,
    /// Correlation confirmed
    Correlation,
}

impl Conclusion {
    /// Create new conclusion
    pub fn new(conclusion_type: ConclusionType, confidence: Confidence) -> Self {
        Self {
            id: ConclusionId::generate(),
            conclusion_type,
            confidence,
            evidence: Vec::new(),
            timestamp: Timestamp::now(),
            description: String::new(),
        }
    }

    /// With evidence
    pub fn with_evidence(mut self, evidence: Vec<KnowledgeId>) -> Self {
        self.evidence = evidence;
        self
    }

    /// With description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Is actionable (high enough confidence)?
    pub fn is_actionable(&self) -> bool {
        self.confidence.is_actionable()
    }
}

// ============================================================================
// INTENT (from DECIDE)
// ============================================================================

/// Intent from decision domain
#[derive(Debug, Clone)]
pub struct Intent {
    /// Intent ID
    pub id: IntentId,
    /// Action type
    pub action_type: ActionType,
    /// Target
    pub target: String,
    /// Priority
    pub priority: Priority,
    /// Deadline
    pub deadline: Option<Timestamp>,
    /// Confidence in decision
    pub confidence: Confidence,
    /// Source policy
    pub policy: PolicyId,
}

/// Types of actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionType {
    /// Do nothing (always safe)
    NoOp,
    /// Adjust parameter
    Adjust,
    /// Enable feature
    Enable,
    /// Disable feature
    Disable,
    /// Migrate resource
    Migrate,
    /// Scale resource
    Scale,
    /// Heal issue
    Heal,
    /// Alert operator
    Alert,
    /// Emergency stop
    EmergencyStop,
    /// Restart component
    Restart,
    /// Reconfigure
    Reconfigure,
    /// Throttle
    Throttle,
    /// Boost
    Boost,
}

impl ActionType {
    /// Is this action type safe (reversible)
    pub const fn is_safe(&self) -> bool {
        matches!(
            self,
            Self::NoOp | Self::Adjust | Self::Enable | Self::Disable | Self::Alert | Self::Throttle
        )
    }

    /// Is this action type critical
    pub const fn is_critical(&self) -> bool {
        matches!(self, Self::EmergencyStop | Self::Heal | Self::Restart)
    }

    /// Requires approval?
    pub const fn requires_approval(&self) -> bool {
        matches!(
            self,
            Self::EmergencyStop | Self::Migrate | Self::Restart | Self::Reconfigure
        )
    }
}

impl Intent {
    /// Create new intent
    pub fn new(action_type: ActionType, target: impl Into<String>, policy: PolicyId) -> Self {
        Self {
            id: IntentId::generate(),
            action_type,
            target: target.into(),
            priority: Priority::NORMAL,
            deadline: None,
            confidence: Confidence::MEDIUM,
            policy,
        }
    }

    /// With priority
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// With deadline
    pub fn with_deadline(mut self, deadline: Timestamp) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// With confidence
    pub fn with_confidence(mut self, confidence: Confidence) -> Self {
        self.confidence = confidence;
        self
    }
}

// ============================================================================
// EFFECT (from ACT)
// ============================================================================

/// Effect from execution domain
#[derive(Debug, Clone)]
pub struct Effect {
    /// Action ID
    pub action: ActionId,
    /// Transaction ID
    pub transaction: TransactionId,
    /// Success
    pub success: bool,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Duration
    pub duration: Duration,
    /// Changes made
    pub changes: Vec<Change>,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// A change made to the system
#[derive(Debug, Clone)]
pub struct Change {
    /// What was changed
    pub target: String,
    /// Previous value
    pub before: String,
    /// New value
    pub after: String,
    /// Change type
    pub change_type: ChangeType,
}

/// Type of change
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    /// Created new
    Create,
    /// Updated existing
    Update,
    /// Deleted
    Delete,
    /// Enabled
    Enable,
    /// Disabled
    Disable,
}

impl Effect {
    /// Create successful effect
    pub fn success(action: ActionId, transaction: TransactionId, duration: Duration) -> Self {
        Self {
            action,
            transaction,
            success: true,
            timestamp: Timestamp::now(),
            duration,
            changes: Vec::new(),
            error: None,
        }
    }

    /// Create failed effect
    pub fn failure(
        action: ActionId,
        transaction: TransactionId,
        duration: Duration,
        error: impl Into<String>,
    ) -> Self {
        Self {
            action,
            transaction,
            success: false,
            timestamp: Timestamp::now(),
            duration,
            changes: Vec::new(),
            error: Some(error.into()),
        }
    }

    /// With changes
    pub fn with_changes(mut self, changes: Vec<Change>) -> Self {
        self.changes = changes;
        self
    }

    /// Add a change
    pub fn add_change(&mut self, change: Change) {
        self.changes.push(change);
    }
}

impl Change {
    /// Create new change
    pub fn new(
        target: impl Into<String>,
        before: impl Into<String>,
        after: impl Into<String>,
        change_type: ChangeType,
    ) -> Self {
        Self {
            target: target.into(),
            before: before.into(),
            after: after.into(),
            change_type,
        }
    }

    /// Create update change
    pub fn update(target: impl Into<String>, before: impl Into<String>, after: impl Into<String>) -> Self {
        Self::new(target, before, after, ChangeType::Update)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal() {
        let signal = Signal::new(ProbeId::generate(), 42.0, MetricUnit::Percent);
        assert!(signal.is_high_quality());
    }

    #[test]
    fn test_knowledge() {
        let k = Knowledge::new(KnowledgeType::Pattern, Confidence::HIGH);
        assert!(k.confidence.meets(Confidence::MEDIUM));
    }

    #[test]
    fn test_conclusion() {
        let c = Conclusion::new(ConclusionType::Anomaly, Confidence::HIGH)
            .with_description("Test anomaly");
        assert!(c.is_actionable());
    }

    #[test]
    fn test_action_type() {
        assert!(ActionType::NoOp.is_safe());
        assert!(ActionType::EmergencyStop.is_critical());
        assert!(ActionType::Restart.requires_approval());
    }

    #[test]
    fn test_effect() {
        let effect = Effect::success(
            ActionId::generate(),
            TransactionId::generate(),
            Duration::from_millis(100),
        );
        assert!(effect.success);
    }
}
