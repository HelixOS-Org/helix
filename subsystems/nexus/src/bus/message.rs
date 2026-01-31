//! Message Definitions
//!
//! Message types, payloads, and priority for inter-domain communication.

#![allow(dead_code)]

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use crate::types::*;
use super::domain::Domain;

// ============================================================================
// MESSAGE PRIORITY
// ============================================================================

/// Message priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MessagePriority {
    /// Low priority (background)
    Low      = 0,
    /// Normal priority
    Normal   = 1,
    /// High priority
    High     = 2,
    /// Urgent (process immediately)
    Urgent   = 3,
    /// Critical (emergency)
    Critical = 4,
}

impl MessagePriority {
    /// Get priority name
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
            Self::Urgent => "urgent",
            Self::Critical => "critical",
        }
    }
}

impl Default for MessagePriority {
    fn default() -> Self {
        Self::Normal
    }
}

// ============================================================================
// MESSAGE PAYLOAD
// ============================================================================

/// Message payload variants
#[derive(Debug, Clone)]
pub enum MessagePayload {
    // ─── Perception → Comprehension ───
    /// Raw signal from sensor
    Signal(Signal),
    /// Batch of signals
    SignalBatch(Vec<Signal>),
    /// Kernel event
    KernelEvent(KernelEventData),

    // ─── Comprehension → Reasoning ───
    /// Extracted knowledge
    Knowledge(Knowledge),
    /// Detected pattern
    PatternDetected(PatternInfo),
    /// State model update
    StateUpdate(StateModelUpdate),

    // ─── Reasoning → Decision ───
    /// Reasoning conclusion
    Conclusion(Conclusion),
    /// Prediction
    Prediction(PredictionInfo),
    /// Anomaly detected
    AnomalyAlert(AnomalyInfo),

    // ─── Decision → Execution ───
    /// Intent to execute
    Intent(Intent),
    /// Batch of intents
    IntentBatch(Vec<Intent>),

    // ─── Execution → Memory ───
    /// Effect record
    Effect(Effect),
    /// Transaction complete
    TransactionComplete(TransactionId),

    // ─── Memory notifications ───
    /// Episode recorded
    EpisodeRecorded(EpisodeId),
    /// Memory consolidated
    MemoryConsolidated(u64),

    // ─── Reflection ───
    /// Health check request
    HealthCheckRequest,
    /// Health check response
    HealthCheckResponse(HealthCheckData),
    /// Calibration request
    CalibrationRequest,
    /// Insight generated
    Insight(InsightData),

    // ─── Control messages ───
    /// Pause domain
    Pause,
    /// Resume domain
    Resume,
    /// Shutdown domain
    Shutdown,
    /// Configuration update
    ConfigUpdate(String),

    // ─── Generic ───
    /// Custom data
    Custom { topic: String, data: String },
}

impl MessagePayload {
    /// Get payload type name
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Signal(_) => "Signal",
            Self::SignalBatch(_) => "SignalBatch",
            Self::KernelEvent(_) => "KernelEvent",
            Self::Knowledge(_) => "Knowledge",
            Self::PatternDetected(_) => "PatternDetected",
            Self::StateUpdate(_) => "StateUpdate",
            Self::Conclusion(_) => "Conclusion",
            Self::Prediction(_) => "Prediction",
            Self::AnomalyAlert(_) => "AnomalyAlert",
            Self::Intent(_) => "Intent",
            Self::IntentBatch(_) => "IntentBatch",
            Self::Effect(_) => "Effect",
            Self::TransactionComplete(_) => "TransactionComplete",
            Self::EpisodeRecorded(_) => "EpisodeRecorded",
            Self::MemoryConsolidated(_) => "MemoryConsolidated",
            Self::HealthCheckRequest => "HealthCheckRequest",
            Self::HealthCheckResponse(_) => "HealthCheckResponse",
            Self::CalibrationRequest => "CalibrationRequest",
            Self::Insight(_) => "Insight",
            Self::Pause => "Pause",
            Self::Resume => "Resume",
            Self::Shutdown => "Shutdown",
            Self::ConfigUpdate(_) => "ConfigUpdate",
            Self::Custom { .. } => "Custom",
        }
    }

    /// Is control message?
    pub const fn is_control(&self) -> bool {
        matches!(self, Self::Pause | Self::Resume | Self::Shutdown | Self::ConfigUpdate(_))
    }

    /// Is health-related?
    pub const fn is_health(&self) -> bool {
        matches!(self, Self::HealthCheckRequest | Self::HealthCheckResponse(_) | Self::CalibrationRequest)
    }
}

// ============================================================================
// PAYLOAD DATA TYPES
// ============================================================================

/// Kernel event data
#[derive(Debug, Clone)]
pub struct KernelEventData {
    /// Event type
    pub event_type: String,
    /// Event ID
    pub event_id: EventId,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Event data
    pub data: BTreeMap<String, String>,
}

impl KernelEventData {
    /// Create new kernel event
    pub fn new(event_type: impl Into<String>, event_id: EventId) -> Self {
        Self {
            event_type: event_type.into(),
            event_id,
            timestamp: Timestamp::now(),
            data: BTreeMap::new(),
        }
    }

    /// Add data
    pub fn with_data(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.data.insert(key.into(), value.into());
        self
    }
}

/// Pattern info
#[derive(Debug, Clone)]
pub struct PatternInfo {
    /// Pattern ID
    pub pattern_id: PatternId,
    /// Pattern name
    pub name: String,
    /// Confidence
    pub confidence: Confidence,
    /// Timestamp
    pub timestamp: Timestamp,
}

impl PatternInfo {
    /// Create new pattern info
    pub fn new(pattern_id: PatternId, name: impl Into<String>) -> Self {
        Self {
            pattern_id,
            name: name.into(),
            confidence: Confidence::HIGH,
            timestamp: Timestamp::now(),
        }
    }

    /// With confidence
    pub fn with_confidence(mut self, confidence: Confidence) -> Self {
        self.confidence = confidence;
        self
    }
}

/// State model update
#[derive(Debug, Clone)]
pub struct StateModelUpdate {
    /// Model ID
    pub model_id: ModelId,
    /// Update type
    pub update_type: String,
    /// New state summary
    pub state: String,
    /// Timestamp
    pub timestamp: Timestamp,
}

impl StateModelUpdate {
    /// Create new state update
    pub fn new(model_id: ModelId, update_type: impl Into<String>) -> Self {
        Self {
            model_id,
            update_type: update_type.into(),
            state: String::new(),
            timestamp: Timestamp::now(),
        }
    }

    /// With state
    pub fn with_state(mut self, state: impl Into<String>) -> Self {
        self.state = state.into();
        self
    }
}

/// Prediction info
#[derive(Debug, Clone)]
pub struct PredictionInfo {
    /// What is being predicted
    pub target: String,
    /// Predicted value
    pub value: f64,
    /// Confidence
    pub confidence: Confidence,
    /// Time horizon
    pub horizon: Duration,
    /// Timestamp
    pub timestamp: Timestamp,
}

impl PredictionInfo {
    /// Create new prediction
    pub fn new(target: impl Into<String>, value: f64, horizon: Duration) -> Self {
        Self {
            target: target.into(),
            value,
            confidence: Confidence::MEDIUM,
            horizon,
            timestamp: Timestamp::now(),
        }
    }

    /// With confidence
    pub fn with_confidence(mut self, confidence: Confidence) -> Self {
        self.confidence = confidence;
        self
    }
}

/// Anomaly info
#[derive(Debug, Clone)]
pub struct AnomalyInfo {
    /// Anomaly type
    pub anomaly_type: String,
    /// Severity
    pub severity: Severity,
    /// Description
    pub description: String,
    /// Confidence
    pub confidence: Confidence,
    /// Timestamp
    pub timestamp: Timestamp,
}

impl AnomalyInfo {
    /// Create new anomaly info
    pub fn new(anomaly_type: impl Into<String>, severity: Severity) -> Self {
        Self {
            anomaly_type: anomaly_type.into(),
            severity,
            description: String::new(),
            confidence: Confidence::MEDIUM,
            timestamp: Timestamp::now(),
        }
    }

    /// With description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }
}

/// Health check data
#[derive(Debug, Clone)]
pub struct HealthCheckData {
    /// Domain
    pub domain: Domain,
    /// Is healthy
    pub healthy: bool,
    /// Health score (0-100)
    pub score: u8,
    /// Issues
    pub issues: Vec<String>,
    /// Timestamp
    pub timestamp: Timestamp,
}

impl HealthCheckData {
    /// Create healthy response
    pub fn healthy(domain: Domain, score: u8) -> Self {
        Self {
            domain,
            healthy: true,
            score,
            issues: Vec::new(),
            timestamp: Timestamp::now(),
        }
    }

    /// Create unhealthy response
    pub fn unhealthy(domain: Domain, issues: Vec<String>) -> Self {
        Self {
            domain,
            healthy: false,
            score: 0,
            issues,
            timestamp: Timestamp::now(),
        }
    }
}

/// Insight data
#[derive(Debug, Clone)]
pub struct InsightData {
    /// Insight ID
    pub insight_id: InsightId,
    /// Insight type
    pub insight_type: String,
    /// Description
    pub description: String,
    /// Confidence
    pub confidence: Confidence,
    /// Affected domains
    pub affected: Vec<Domain>,
    /// Timestamp
    pub timestamp: Timestamp,
}

impl InsightData {
    /// Create new insight
    pub fn new(insight_id: InsightId, insight_type: impl Into<String>) -> Self {
        Self {
            insight_id,
            insight_type: insight_type.into(),
            description: String::new(),
            confidence: Confidence::MEDIUM,
            affected: Vec::new(),
            timestamp: Timestamp::now(),
        }
    }

    /// With description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Add affected domain
    pub fn affects(mut self, domain: Domain) -> Self {
        self.affected.push(domain);
        self
    }
}

// ============================================================================
// MESSAGE
// ============================================================================

/// A message on the bus
#[derive(Debug, Clone)]
pub struct Message {
    /// Message ID
    pub id: MessageId,
    /// Correlation ID (for request/response)
    pub correlation_id: Option<CorrelationId>,
    /// Source domain
    pub source: Domain,
    /// Target domain
    pub target: Domain,
    /// Priority
    pub priority: MessagePriority,
    /// Payload
    pub payload: MessagePayload,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Time-to-live (optional)
    pub ttl: Option<Duration>,
}

impl Message {
    /// Create new message
    pub fn new(source: Domain, target: Domain, payload: MessagePayload) -> Self {
        Self {
            id: MessageId::generate(),
            correlation_id: None,
            source,
            target,
            priority: MessagePriority::Normal,
            payload,
            timestamp: Timestamp::now(),
            ttl: None,
        }
    }

    /// With priority
    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
        self
    }

    /// With correlation ID
    pub fn with_correlation(mut self, correlation_id: CorrelationId) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    /// With TTL
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = Some(ttl);
        self
    }

    /// Check if expired
    pub fn is_expired(&self, now: Timestamp) -> bool {
        if let Some(ttl) = self.ttl {
            now.elapsed_since(self.timestamp).0 > ttl.0
        } else {
            false
        }
    }

    /// Is control message?
    pub const fn is_control(&self) -> bool {
        self.payload.is_control()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_priority() {
        assert!(MessagePriority::Critical > MessagePriority::Urgent);
        assert!(MessagePriority::Urgent > MessagePriority::High);
        assert!(MessagePriority::High > MessagePriority::Normal);
        assert!(MessagePriority::Normal > MessagePriority::Low);
    }

    #[test]
    fn test_message_creation() {
        let msg = Message::new(
            Domain::Sense,
            Domain::Understand,
            MessagePayload::HealthCheckRequest,
        );

        assert_eq!(msg.source, Domain::Sense);
        assert_eq!(msg.target, Domain::Understand);
        assert_eq!(msg.priority, MessagePriority::Normal);
    }

    #[test]
    fn test_message_expiry() {
        let msg = Message::new(
            Domain::Sense,
            Domain::Understand,
            MessagePayload::HealthCheckRequest,
        )
        .with_ttl(Duration::from_millis(100));

        assert!(!msg.is_expired(msg.timestamp));

        let future = Timestamp(msg.timestamp.0 + 200);
        assert!(msg.is_expired(future));
    }
}
