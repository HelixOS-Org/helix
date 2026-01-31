//! Intent — Decision output
//!
//! An Intent is the output of the DECIDE domain, representing
//! a chosen action to be executed by the ACT domain.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

use crate::types::*;
use super::options::Option;

// ============================================================================
// INTENT
// ============================================================================

/// An intent - the output of a decision
#[derive(Debug, Clone)]
pub struct Intent {
    /// Intent ID
    pub id: IntentId,
    /// Selected option
    pub selected_option: Option,
    /// Final score
    pub score: f32,
    /// Confidence
    pub confidence: Confidence,
    /// Requires confirmation
    pub requires_confirmation: bool,
    /// Is rate limited
    pub rate_limited: bool,
    /// Justification
    pub justification: String,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Expires at
    pub expires_at: Timestamp,
    /// Source conclusion
    pub source_conclusion: Option<ConclusionId>,
}

impl Intent {
    /// Create a new intent
    pub fn new(option: Option, confidence: Confidence) -> Self {
        let now = Timestamp::now();
        Self {
            id: IntentId::generate(),
            selected_option: option,
            score: 0.0,
            confidence,
            requires_confirmation: false,
            rate_limited: false,
            justification: String::new(),
            timestamp: now,
            expires_at: Timestamp::new(now.as_nanos() + Duration::from_secs(30).as_nanos()),
            source_conclusion: None,
        }
    }

    /// Set score
    pub fn with_score(mut self, score: f32) -> Self {
        self.score = score;
        self
    }

    /// Set justification
    pub fn with_justification(mut self, justification: impl Into<String>) -> Self {
        self.justification = justification.into();
        self
    }

    /// Set source conclusion
    pub fn with_source(mut self, conclusion_id: ConclusionId) -> Self {
        self.source_conclusion = Some(conclusion_id);
        self
    }

    /// Set expiration
    pub fn with_expiry(mut self, ttl: Duration) -> Self {
        self.expires_at = Timestamp::new(self.timestamp.as_nanos() + ttl.as_nanos());
        self
    }

    /// Require confirmation
    pub fn require_confirmation(mut self) -> Self {
        self.requires_confirmation = true;
        self
    }

    /// Set rate limited
    pub fn set_rate_limited(mut self) -> Self {
        self.rate_limited = true;
        self
    }

    /// Check if intent has expired
    pub fn is_expired(&self, now: Timestamp) -> bool {
        now.as_nanos() > self.expires_at.as_nanos()
    }

    /// Check if intent can be executed immediately
    pub fn can_execute(&self) -> bool {
        !self.requires_confirmation && !self.rate_limited
    }

    /// Get action type
    pub fn action_type(&self) -> super::options::ActionType {
        self.selected_option.action_type
    }

    /// Get target
    pub fn target(&self) -> &super::options::ActionTarget {
        &self.selected_option.target
    }

    /// Get time until expiry
    pub fn time_until_expiry(&self, now: Timestamp) -> Duration {
        if self.is_expired(now) {
            Duration::ZERO
        } else {
            Duration::from_nanos(self.expires_at.as_nanos() - now.as_nanos())
        }
    }
}

// ============================================================================
// INTENT BATCH
// ============================================================================

/// A batch of intents
#[derive(Debug, Clone, Default)]
pub struct IntentBatch {
    /// Intents in this batch
    intents: Vec<Intent>,
    /// Batch timestamp
    timestamp: Timestamp,
}

impl IntentBatch {
    /// Create new batch
    pub fn new() -> Self {
        Self {
            intents: Vec::new(),
            timestamp: Timestamp::now(),
        }
    }

    /// Add intent
    pub fn add(&mut self, intent: Intent) {
        self.intents.push(intent);
    }

    /// Get intents
    pub fn intents(&self) -> &[Intent] {
        &self.intents
    }

    /// Get mutable intents
    pub fn intents_mut(&mut self) -> &mut Vec<Intent> {
        &mut self.intents
    }

    /// Count
    pub fn len(&self) -> usize {
        self.intents.len()
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.intents.is_empty()
    }

    /// Get batch timestamp
    pub fn timestamp(&self) -> Timestamp {
        self.timestamp
    }

    /// Filter expired intents
    pub fn filter_expired(&mut self, now: Timestamp) {
        self.intents.retain(|i| !i.is_expired(now));
    }

    /// Get executable intents
    pub fn executable(&self) -> impl Iterator<Item = &Intent> {
        self.intents.iter().filter(|i| i.can_execute())
    }

    /// Get intents requiring confirmation
    pub fn requiring_confirmation(&self) -> impl Iterator<Item = &Intent> {
        self.intents.iter().filter(|i| i.requires_confirmation)
    }

    /// Sort by score (highest first)
    pub fn sort_by_score(&mut self) {
        self.intents.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(core::cmp::Ordering::Equal)
        });
    }

    /// Sort by confidence (highest first)
    pub fn sort_by_confidence(&mut self) {
        self.intents.sort_by(|a, b| {
            b.confidence.value().partial_cmp(&a.confidence.value()).unwrap_or(core::cmp::Ordering::Equal)
        });
    }
}

// ============================================================================
// INTENT QUEUE
// ============================================================================

/// A priority queue for intents
#[derive(Debug)]
pub struct IntentQueue {
    /// High priority intents
    high: Vec<Intent>,
    /// Normal priority intents
    normal: Vec<Intent>,
    /// Low priority intents
    low: Vec<Intent>,
    /// Maximum queue size
    max_size: usize,
}

impl IntentQueue {
    /// Create new queue
    pub fn new(max_size: usize) -> Self {
        Self {
            high: Vec::new(),
            normal: Vec::new(),
            low: Vec::new(),
            max_size,
        }
    }

    /// Enqueue intent
    pub fn enqueue(&mut self, intent: Intent) -> bool {
        if self.len() >= self.max_size {
            return false;
        }

        // Priority based on score
        if intent.score >= 0.8 {
            self.high.push(intent);
        } else if intent.score >= 0.4 {
            self.normal.push(intent);
        } else {
            self.low.push(intent);
        }

        true
    }

    /// Dequeue next intent
    pub fn dequeue(&mut self) -> Option<Intent> {
        if !self.high.is_empty() {
            Some(self.high.remove(0))
        } else if !self.normal.is_empty() {
            Some(self.normal.remove(0))
        } else if !self.low.is_empty() {
            Some(self.low.remove(0))
        } else {
            None
        }
    }

    /// Peek next intent
    pub fn peek(&self) -> Option<&Intent> {
        self.high.first()
            .or_else(|| self.normal.first())
            .or_else(|| self.low.first())
    }

    /// Total length
    pub fn len(&self) -> usize {
        self.high.len() + self.normal.len() + self.low.len()
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.high.is_empty() && self.normal.is_empty() && self.low.is_empty()
    }

    /// Clear all
    pub fn clear(&mut self) {
        self.high.clear();
        self.normal.clear();
        self.low.clear();
    }

    /// Remove expired intents
    pub fn remove_expired(&mut self, now: Timestamp) {
        self.high.retain(|i| !i.is_expired(now));
        self.normal.retain(|i| !i.is_expired(now));
        self.low.retain(|i| !i.is_expired(now));
    }
}

impl Default for IntentQueue {
    fn default() -> Self {
        Self::new(1000)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::options::{ActionType, ActionTarget, ActionParameters, ActionCost, ExpectedOutcome, OptionSource, OptionId};

    fn make_test_intent(score: f32) -> Intent {
        let option = Option {
            id: OptionId::generate(),
            action_type: ActionType::Log,
            description: String::from("Test"),
            target: ActionTarget::System,
            parameters: ActionParameters::new(),
            expected_outcome: ExpectedOutcome::default(),
            reversible: true,
            cost: ActionCost::default(),
            source: OptionSource::Default,
        };

        Intent::new(option, Confidence::MEDIUM).with_score(score)
    }

    #[test]
    fn test_intent_creation() {
        let intent = make_test_intent(0.75);
        assert!(intent.can_execute());
        assert!(!intent.requires_confirmation);
    }

    #[test]
    fn test_intent_batch() {
        let mut batch = IntentBatch::new();
        batch.add(make_test_intent(0.8));
        batch.add(make_test_intent(0.5));

        assert_eq!(batch.len(), 2);
        assert_eq!(batch.executable().count(), 2);
    }

    #[test]
    fn test_intent_queue() {
        let mut queue = IntentQueue::new(100);

        queue.enqueue(make_test_intent(0.9)); // High
        queue.enqueue(make_test_intent(0.5)); // Normal
        queue.enqueue(make_test_intent(0.2)); // Low

        // High priority comes first
        let first = queue.dequeue().unwrap();
        assert!(first.score >= 0.8);
    }
}
