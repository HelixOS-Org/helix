//! # Cognitive Trigger System
//!
//! Event-driven trigger system for cognitive operations.
//! Enables reactive behaviors and automated responses.

#![allow(dead_code)]

extern crate alloc;
use alloc::boxed::Box;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// TRIGGER TYPES
// ============================================================================

/// A trigger definition
#[derive(Debug, Clone)]
pub struct Trigger {
    /// Trigger ID
    pub id: u64,
    /// Trigger name
    pub name: String,
    /// Event pattern to match
    pub event_pattern: EventPattern,
    /// Condition to evaluate
    pub condition: Option<TriggerCondition>,
    /// Actions to execute
    pub actions: Vec<TriggerAction>,
    /// Owner domain
    pub owner: DomainId,
    /// Enabled
    pub enabled: bool,
    /// Priority
    pub priority: u32,
    /// Cooldown (ns) - minimum time between activations
    pub cooldown_ns: u64,
    /// Last activation time
    pub last_activation: Option<Timestamp>,
    /// Activation count
    pub activation_count: u64,
}

/// Event pattern to match
#[derive(Debug, Clone)]
pub struct EventPattern {
    /// Event type pattern
    pub event_type: PatternMatcher,
    /// Source domain pattern
    pub source: Option<DomainId>,
    /// Payload patterns
    pub payload_patterns: Vec<PayloadPattern>,
}

/// Pattern matcher
#[derive(Debug, Clone)]
pub enum PatternMatcher {
    /// Exact match
    Exact(String),
    /// Prefix match
    Prefix(String),
    /// Suffix match
    Suffix(String),
    /// Contains
    Contains(String),
    /// Regex-like pattern
    Pattern(String),
    /// Match any
    Any,
}

impl PatternMatcher {
    /// Check if pattern matches
    pub fn matches(&self, value: &str) -> bool {
        match self {
            Self::Exact(s) => value == s,
            Self::Prefix(s) => value.starts_with(s),
            Self::Suffix(s) => value.ends_with(s),
            Self::Contains(s) => value.contains(s),
            Self::Pattern(_) => true, // Would need regex
            Self::Any => true,
        }
    }
}

/// Payload pattern
#[derive(Debug, Clone)]
pub struct PayloadPattern {
    /// Field path
    pub path: String,
    /// Expected value pattern
    pub pattern: ValuePattern,
}

/// Value pattern
#[derive(Debug, Clone)]
pub enum ValuePattern {
    /// Exact value
    Equals(TriggerValue),
    /// Not equal
    NotEquals(TriggerValue),
    /// Greater than
    GreaterThan(f64),
    /// Less than
    LessThan(f64),
    /// In range
    InRange(f64, f64),
    /// Contains string
    Contains(String),
    /// Exists (field is present)
    Exists,
}

/// Trigger value
#[derive(Debug, Clone)]
pub enum TriggerValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

/// Trigger condition
#[derive(Debug, Clone)]
pub enum TriggerCondition {
    /// Simple expression
    Expression(String),
    /// Threshold condition
    Threshold {
        metric: String,
        operator: ThresholdOp,
        value: f64,
    },
    /// Time-based condition
    TimeWindow {
        start_hour: u8,
        end_hour: u8,
    },
    /// Rate condition
    Rate {
        event_type: String,
        threshold: u64,
        window_ns: u64,
    },
    /// Composite condition
    And(Vec<TriggerCondition>),
    Or(Vec<TriggerCondition>),
    Not(Box<TriggerCondition>),
}

/// Threshold operator
#[derive(Debug, Clone, Copy)]
pub enum ThresholdOp {
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Equal,
    NotEqual,
}

/// Trigger action
#[derive(Debug, Clone)]
pub struct TriggerAction {
    /// Action type
    pub action_type: ActionType,
    /// Parameters
    pub params: BTreeMap<String, String>,
    /// Async execution
    pub async_exec: bool,
    /// Timeout (ns)
    pub timeout_ns: Option<u64>,
}

/// Action type
#[derive(Debug, Clone)]
pub enum ActionType {
    /// Emit event
    EmitEvent(String),
    /// Call handler
    CallHandler(String),
    /// Send message
    SendMessage { target: DomainId, topic: String },
    /// Set flag
    SetFlag { name: String, value: bool },
    /// Update metric
    UpdateMetric { name: String, delta: f64 },
    /// Log
    Log { level: LogLevel, message: String },
    /// Alert
    Alert { severity: AlertSeverity, message: String },
    /// Chain to another trigger
    Chain { trigger_id: u64 },
    /// Custom action
    Custom { handler: String },
}

/// Log level
#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Alert severity
#[derive(Debug, Clone, Copy)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

// ============================================================================
// EVENT
// ============================================================================

/// An event that can trigger actions
#[derive(Debug, Clone)]
pub struct Event {
    /// Event ID
    pub id: u64,
    /// Event type
    pub event_type: String,
    /// Source domain
    pub source: DomainId,
    /// Payload
    pub payload: BTreeMap<String, TriggerValue>,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Correlation ID
    pub correlation_id: Option<u64>,
}

// ============================================================================
// TRIGGER ENGINE
// ============================================================================

/// Trigger engine
pub struct TriggerEngine {
    /// Registered triggers
    triggers: BTreeMap<u64, Trigger>,
    /// Triggers by event type
    by_event_type: BTreeMap<String, Vec<u64>>,
    /// Event history (for rate limiting)
    event_history: Vec<Event>,
    /// Metrics (for threshold conditions)
    metrics: BTreeMap<String, f64>,
    /// Flags
    flags: BTreeMap<String, bool>,
    /// Next trigger ID
    next_trigger_id: AtomicU64,
    /// Next event ID
    next_event_id: AtomicU64,
    /// Configuration
    config: TriggerConfig,
    /// Statistics
    stats: TriggerStats,
}

/// Trigger configuration
#[derive(Debug, Clone)]
pub struct TriggerConfig {
    /// Maximum triggers
    pub max_triggers: usize,
    /// Maximum event history
    pub max_history: usize,
    /// Default cooldown (ns)
    pub default_cooldown_ns: u64,
    /// Maximum actions per trigger
    pub max_actions: usize,
    /// Maximum chain depth
    pub max_chain_depth: usize,
}

impl Default for TriggerConfig {
    fn default() -> Self {
        Self {
            max_triggers: 1000,
            max_history: 10000,
            default_cooldown_ns: 1_000_000_000, // 1 second
            max_actions: 10,
            max_chain_depth: 5,
        }
    }
}

/// Trigger statistics
#[derive(Debug, Clone, Default)]
pub struct TriggerStats {
    /// Total events processed
    pub total_events: u64,
    /// Triggers activated
    pub triggers_activated: u64,
    /// Actions executed
    pub actions_executed: u64,
    /// Cooldown skips
    pub cooldown_skips: u64,
    /// Condition failures
    pub condition_failures: u64,
}

/// Trigger activation result
#[derive(Debug, Clone)]
pub struct ActivationResult {
    /// Trigger ID
    pub trigger_id: u64,
    /// Activated
    pub activated: bool,
    /// Actions executed
    pub actions_executed: usize,
    /// Chained triggers
    pub chained: Vec<u64>,
    /// Errors
    pub errors: Vec<String>,
}

impl TriggerEngine {
    /// Create new trigger engine
    pub fn new(config: TriggerConfig) -> Self {
        Self {
            triggers: BTreeMap::new(),
            by_event_type: BTreeMap::new(),
            event_history: Vec::new(),
            metrics: BTreeMap::new(),
            flags: BTreeMap::new(),
            next_trigger_id: AtomicU64::new(1),
            next_event_id: AtomicU64::new(1),
            config,
            stats: TriggerStats::default(),
        }
    }

    /// Register a trigger
    pub fn register(
        &mut self,
        name: &str,
        event_pattern: EventPattern,
        condition: Option<TriggerCondition>,
        actions: Vec<TriggerAction>,
        owner: DomainId,
    ) -> u64 {
        let id = self.next_trigger_id.fetch_add(1, Ordering::Relaxed);

        let trigger = Trigger {
            id,
            name: name.into(),
            event_pattern: event_pattern.clone(),
            condition,
            actions,
            owner,
            enabled: true,
            priority: 0,
            cooldown_ns: self.config.default_cooldown_ns,
            last_activation: None,
            activation_count: 0,
        };

        self.triggers.insert(id, trigger);

        // Index by event type if exact match
        if let PatternMatcher::Exact(event_type) = &event_pattern.event_type {
            self.by_event_type
                .entry(event_type.clone())
                .or_insert_with(Vec::new)
                .push(id);
        }

        id
    }

    /// Unregister a trigger
    pub fn unregister(&mut self, id: u64) -> bool {
        if let Some(trigger) = self.triggers.remove(&id) {
            if let PatternMatcher::Exact(event_type) = &trigger.event_pattern.event_type {
                if let Some(ids) = self.by_event_type.get_mut(event_type) {
                    ids.retain(|&tid| tid != id);
                }
            }
            true
        } else {
            false
        }
    }

    /// Enable/disable trigger
    pub fn set_enabled(&mut self, id: u64, enabled: bool) {
        if let Some(trigger) = self.triggers.get_mut(&id) {
            trigger.enabled = enabled;
        }
    }

    /// Set metric value
    pub fn set_metric(&mut self, name: &str, value: f64) {
        self.metrics.insert(name.into(), value);
    }

    /// Set flag value
    pub fn set_flag(&mut self, name: &str, value: bool) {
        self.flags.insert(name.into(), value);
    }

    /// Emit an event
    pub fn emit(&mut self, event_type: &str, source: DomainId, payload: BTreeMap<String, TriggerValue>) -> Vec<ActivationResult> {
        let id = self.next_event_id.fetch_add(1, Ordering::Relaxed);

        let event = Event {
            id,
            event_type: event_type.into(),
            source,
            payload,
            timestamp: Timestamp::now(),
            correlation_id: None,
        };

        self.process_event(event, 0)
    }

    /// Process an event
    fn process_event(&mut self, event: Event, depth: usize) -> Vec<ActivationResult> {
        self.stats.total_events += 1;

        // Store in history
        if self.event_history.len() >= self.config.max_history {
            self.event_history.remove(0);
        }
        self.event_history.push(event.clone());

        // Find matching triggers
        let matching: Vec<u64> = self.triggers.values()
            .filter(|t| t.enabled && self.matches_pattern(&event, &t.event_pattern))
            .map(|t| t.id)
            .collect();

        let mut results = Vec::new();

        for trigger_id in matching {
            if let Some(result) = self.activate_trigger(trigger_id, &event, depth) {
                results.push(result);
            }
        }

        results
    }

    /// Check if event matches pattern
    fn matches_pattern(&self, event: &Event, pattern: &EventPattern) -> bool {
        // Check event type
        if !pattern.event_type.matches(&event.event_type) {
            return false;
        }

        // Check source
        if let Some(source) = pattern.source {
            if event.source != source {
                return false;
            }
        }

        // Check payload patterns
        for pp in &pattern.payload_patterns {
            if let Some(value) = event.payload.get(&pp.path) {
                if !self.matches_value(value, &pp.pattern) {
                    return false;
                }
            } else if !matches!(pp.pattern, ValuePattern::Exists) {
                return false;
            }
        }

        true
    }

    /// Check if value matches pattern
    fn matches_value(&self, value: &TriggerValue, pattern: &ValuePattern) -> bool {
        match pattern {
            ValuePattern::Exists => true,
            ValuePattern::Equals(expected) => {
                match (value, expected) {
                    (TriggerValue::Bool(a), TriggerValue::Bool(b)) => a == b,
                    (TriggerValue::Int(a), TriggerValue::Int(b)) => a == b,
                    (TriggerValue::Float(a), TriggerValue::Float(b)) => (a - b).abs() < f64::EPSILON,
                    (TriggerValue::String(a), TriggerValue::String(b)) => a == b,
                    (TriggerValue::Null, TriggerValue::Null) => true,
                    _ => false,
                }
            }
            ValuePattern::NotEquals(expected) => {
                !self.matches_value(value, &ValuePattern::Equals(expected.clone()))
            }
            ValuePattern::GreaterThan(threshold) => {
                match value {
                    TriggerValue::Int(v) => (*v as f64) > *threshold,
                    TriggerValue::Float(v) => v > threshold,
                    _ => false,
                }
            }
            ValuePattern::LessThan(threshold) => {
                match value {
                    TriggerValue::Int(v) => (*v as f64) < *threshold,
                    TriggerValue::Float(v) => v < threshold,
                    _ => false,
                }
            }
            ValuePattern::InRange(min, max) => {
                match value {
                    TriggerValue::Int(v) => {
                        let f = *v as f64;
                        f >= *min && f <= *max
                    }
                    TriggerValue::Float(v) => v >= min && v <= max,
                    _ => false,
                }
            }
            ValuePattern::Contains(s) => {
                match value {
                    TriggerValue::String(v) => v.contains(s),
                    _ => false,
                }
            }
        }
    }

    /// Activate a trigger
    fn activate_trigger(&mut self, trigger_id: u64, event: &Event, depth: usize) -> Option<ActivationResult> {
        let trigger = self.triggers.get(&trigger_id)?.clone();
        let now = Timestamp::now();

        // Check cooldown
        if let Some(last) = trigger.last_activation {
            if now.elapsed_since(last) < trigger.cooldown_ns {
                self.stats.cooldown_skips += 1;
                return None;
            }
        }

        // Check condition
        if let Some(condition) = &trigger.condition {
            if !self.evaluate_condition(condition, event) {
                self.stats.condition_failures += 1;
                return None;
            }
        }

        // Execute actions
        let mut actions_executed = 0;
        let mut chained = Vec::new();
        let mut errors = Vec::new();

        for action in &trigger.actions {
            match self.execute_action(action, event, depth) {
                Ok(chain) => {
                    actions_executed += 1;
                    self.stats.actions_executed += 1;
                    if let Some(chain_id) = chain {
                        chained.push(chain_id);
                    }
                }
                Err(e) => {
                    errors.push(e.into());
                }
            }
        }

        // Update trigger
        if let Some(trigger) = self.triggers.get_mut(&trigger_id) {
            trigger.last_activation = Some(now);
            trigger.activation_count += 1;
        }

        self.stats.triggers_activated += 1;

        Some(ActivationResult {
            trigger_id,
            activated: true,
            actions_executed,
            chained,
            errors,
        })
    }

    /// Evaluate condition
    fn evaluate_condition(&self, condition: &TriggerCondition, _event: &Event) -> bool {
        match condition {
            TriggerCondition::Expression(_) => true,
            TriggerCondition::Threshold { metric, operator, value } => {
                let actual = self.metrics.get(metric).copied().unwrap_or(0.0);
                match operator {
                    ThresholdOp::Greater => actual > *value,
                    ThresholdOp::GreaterEqual => actual >= *value,
                    ThresholdOp::Less => actual < *value,
                    ThresholdOp::LessEqual => actual <= *value,
                    ThresholdOp::Equal => (actual - value).abs() < f64::EPSILON,
                    ThresholdOp::NotEqual => (actual - value).abs() >= f64::EPSILON,
                }
            }
            TriggerCondition::TimeWindow { .. } => true, // Would need time-of-day
            TriggerCondition::Rate { event_type, threshold, window_ns } => {
                let now = Timestamp::now();
                let count = self.event_history.iter()
                    .filter(|e| e.event_type == *event_type && now.elapsed_since(e.timestamp) < *window_ns)
                    .count() as u64;
                count >= *threshold
            }
            TriggerCondition::And(conditions) => {
                conditions.iter().all(|c| self.evaluate_condition(c, _event))
            }
            TriggerCondition::Or(conditions) => {
                conditions.iter().any(|c| self.evaluate_condition(c, _event))
            }
            TriggerCondition::Not(condition) => {
                !self.evaluate_condition(condition, _event)
            }
        }
    }

    /// Execute action
    fn execute_action(&mut self, action: &TriggerAction, event: &Event, depth: usize) -> Result<Option<u64>, &'static str> {
        match &action.action_type {
            ActionType::SetFlag { name, value } => {
                self.flags.insert(name.clone(), *value);
                Ok(None)
            }
            ActionType::UpdateMetric { name, delta } => {
                let current = self.metrics.get(name).copied().unwrap_or(0.0);
                self.metrics.insert(name.clone(), current + delta);
                Ok(None)
            }
            ActionType::Chain { trigger_id } => {
                if depth < self.config.max_chain_depth {
                    self.activate_trigger(*trigger_id, event, depth + 1);
                    Ok(Some(*trigger_id))
                } else {
                    Err("Max chain depth exceeded")
                }
            }
            ActionType::EmitEvent(event_type) => {
                if depth < self.config.max_chain_depth {
                    let payload = event.payload.clone();
                    self.emit(event_type, event.source, payload);
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    /// Get trigger
    pub fn get_trigger(&self, id: u64) -> Option<&Trigger> {
        self.triggers.get(&id)
    }

    /// Get triggers for domain
    pub fn triggers_for(&self, owner: DomainId) -> Vec<&Trigger> {
        self.triggers.values()
            .filter(|t| t.owner == owner)
            .collect()
    }

    /// Get statistics
    pub fn stats(&self) -> &TriggerStats {
        &self.stats
    }
}

impl Default for TriggerEngine {
    fn default() -> Self {
        Self::new(TriggerConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trigger_registration() {
        let mut engine = TriggerEngine::default();
        let domain = DomainId::new(1);

        let id = engine.register(
            "test_trigger",
            EventPattern {
                event_type: PatternMatcher::Exact("test.event".into()),
                source: None,
                payload_patterns: vec![],
            },
            None,
            vec![TriggerAction {
                action_type: ActionType::SetFlag { name: "triggered".into(), value: true },
                params: BTreeMap::new(),
                async_exec: false,
                timeout_ns: None,
            }],
            domain,
        );

        assert!(engine.get_trigger(id).is_some());
    }

    #[test]
    fn test_event_triggering() {
        let mut engine = TriggerEngine::default();
        let domain = DomainId::new(1);

        engine.register(
            "test",
            EventPattern {
                event_type: PatternMatcher::Exact("test.event".into()),
                source: None,
                payload_patterns: vec![],
            },
            None,
            vec![TriggerAction {
                action_type: ActionType::SetFlag { name: "triggered".into(), value: true },
                params: BTreeMap::new(),
                async_exec: false,
                timeout_ns: None,
            }],
            domain,
        );

        let results = engine.emit("test.event", domain, BTreeMap::new());
        assert!(!results.is_empty());
        assert!(results[0].activated);

        assert!(engine.flags.get("triggered").copied().unwrap_or(false));
    }

    #[test]
    fn test_condition() {
        let mut engine = TriggerEngine::default();
        let domain = DomainId::new(1);

        engine.register(
            "threshold_trigger",
            EventPattern {
                event_type: PatternMatcher::Any,
                source: None,
                payload_patterns: vec![],
            },
            Some(TriggerCondition::Threshold {
                metric: "cpu".into(),
                operator: ThresholdOp::Greater,
                value: 80.0,
            }),
            vec![],
            domain,
        );

        // Metric below threshold - should not trigger
        engine.set_metric("cpu", 50.0);
        let results = engine.emit("any", domain, BTreeMap::new());
        assert!(results.is_empty() || !results[0].activated);

        // Metric above threshold - should trigger
        engine.set_metric("cpu", 90.0);
        let results = engine.emit("any", domain, BTreeMap::new());
        assert!(!results.is_empty());
    }

    #[test]
    fn test_pattern_matching() {
        let prefix = PatternMatcher::Prefix("test.".into());
        assert!(prefix.matches("test.event"));
        assert!(!prefix.matches("other.event"));

        let contains = PatternMatcher::Contains("foo".into());
        assert!(contains.matches("test.foo.bar"));
        assert!(!contains.matches("test.bar"));
    }
}
