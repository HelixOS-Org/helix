//! Alert definitions and management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;

use crate::core::NexusTimestamp;

// ============================================================================
// ALERT SEVERITY AND STATE
// ============================================================================

/// Alert severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertSeverity {
    /// Informational
    Info      = 0,
    /// Warning
    Warning   = 1,
    /// Critical
    Critical  = 2,
    /// Emergency
    Emergency = 3,
}

/// Alert state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertState {
    /// Alert is not firing
    OK,
    /// Alert is pending (condition met but for duration not elapsed)
    Pending,
    /// Alert is firing
    Firing,
    /// Alert was resolved
    Resolved,
}

// ============================================================================
// ALERT CONDITION
// ============================================================================

/// Alert condition
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertCondition {
    /// Greater than threshold
    GreaterThan,
    /// Less than threshold
    LessThan,
    /// Greater than or equal
    GreaterOrEqual,
    /// Less than or equal
    LessOrEqual,
    /// Equal to threshold
    Equal,
    /// Not equal to threshold
    NotEqual,
    /// Absent (no data)
    Absent,
}

impl AlertCondition {
    /// Evaluate condition
    #[inline]
    pub fn evaluate(&self, value: f64, threshold: f64) -> bool {
        match self {
            Self::GreaterThan => value > threshold,
            Self::LessThan => value < threshold,
            Self::GreaterOrEqual => value >= threshold,
            Self::LessOrEqual => value <= threshold,
            Self::Equal => (value - threshold).abs() < f64::EPSILON,
            Self::NotEqual => (value - threshold).abs() >= f64::EPSILON,
            Self::Absent => false, // Handled separately
        }
    }
}

// ============================================================================
// ALERT RULE
// ============================================================================

/// Alert definition
#[derive(Debug, Clone)]
pub struct AlertRule {
    /// Rule name
    pub name: String,
    /// Description
    pub description: String,
    /// Metric name to monitor
    pub metric: String,
    /// Condition
    pub condition: AlertCondition,
    /// Threshold
    pub threshold: f64,
    /// Duration before firing (cycles)
    pub for_duration: u64,
    /// Severity
    pub severity: AlertSeverity,
    /// Labels
    pub labels: BTreeMap<String, String>,
}

impl AlertRule {
    /// Create new alert rule
    pub fn new(name: impl Into<String>, metric: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            metric: metric.into(),
            condition: AlertCondition::GreaterThan,
            threshold: 0.0,
            for_duration: 0,
            severity: AlertSeverity::Warning,
            labels: BTreeMap::new(),
        }
    }

    /// Set description
    #[inline(always)]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set condition
    #[inline]
    pub fn when(mut self, condition: AlertCondition, threshold: f64) -> Self {
        self.condition = condition;
        self.threshold = threshold;
        self
    }

    /// Set for duration
    #[inline(always)]
    pub fn for_cycles(mut self, duration: u64) -> Self {
        self.for_duration = duration;
        self
    }

    /// Set severity
    #[inline(always)]
    pub fn severity(mut self, severity: AlertSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// Add label
    #[inline(always)]
    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }
}

// ============================================================================
// ACTIVE ALERT
// ============================================================================

/// Active alert
#[derive(Debug, Clone)]
pub struct Alert {
    /// Rule that triggered
    pub rule: String,
    /// Current state
    pub state: AlertState,
    /// Current value
    pub value: f64,
    /// Started at
    pub started_at: NexusTimestamp,
    /// Ended at (if resolved)
    pub ended_at: Option<NexusTimestamp>,
    /// Severity
    pub severity: AlertSeverity,
    /// Labels
    pub labels: BTreeMap<String, String>,
}

impl Alert {
    /// Create new alert
    pub fn new(rule: &AlertRule, value: f64) -> Self {
        Self {
            rule: rule.name.clone(),
            state: AlertState::Pending,
            value,
            started_at: NexusTimestamp::now(),
            ended_at: None,
            severity: rule.severity,
            labels: rule.labels.clone(),
        }
    }

    /// Fire the alert
    #[inline(always)]
    pub fn fire(&mut self) {
        self.state = AlertState::Firing;
    }

    /// Resolve the alert
    #[inline(always)]
    pub fn resolve(&mut self) {
        self.state = AlertState::Resolved;
        self.ended_at = Some(NexusTimestamp::now());
    }

    /// Duration
    #[inline(always)]
    pub fn duration(&self) -> u64 {
        let end = self.ended_at.unwrap_or_else(NexusTimestamp::now);
        end.duration_since(self.started_at)
    }
}
