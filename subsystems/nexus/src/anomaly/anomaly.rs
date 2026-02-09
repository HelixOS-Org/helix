//! Anomaly representation

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::types::{AnomalySeverity, AnomalyType};
use crate::core::{ComponentId, NexusTimestamp};

// ============================================================================
// ANOMALY
// ============================================================================

/// A detected anomaly
#[derive(Debug, Clone)]
pub struct Anomaly {
    /// Unique anomaly ID
    pub id: u64,
    /// Timestamp
    pub timestamp: NexusTimestamp,
    /// Anomaly type
    pub anomaly_type: AnomalyType,
    /// Severity
    pub severity: AnomalySeverity,
    /// Score (0.0 - 1.0)
    pub score: f64,
    /// Source metric/feature
    pub source: String,
    /// Component (if identifiable)
    pub component: Option<ComponentId>,
    /// Current value
    pub current_value: f64,
    /// Expected value
    pub expected_value: f64,
    /// Deviation from expected
    pub deviation: f64,
    /// Context information
    pub context: Vec<(String, f64)>,
}

impl Anomaly {
    /// Create a new anomaly
    pub fn new(
        anomaly_type: AnomalyType,
        source: impl Into<String>,
        current: f64,
        expected: f64,
    ) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);

        let deviation = (current - expected).abs();
        let score = (deviation / expected.max(1.0)).min(1.0);
        let severity = AnomalySeverity::from_score(score);

        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            timestamp: NexusTimestamp::now(),
            anomaly_type,
            severity,
            score,
            source: source.into(),
            component: None,
            current_value: current,
            expected_value: expected,
            deviation,
            context: Vec::new(),
        }
    }

    /// Set component
    #[inline(always)]
    pub fn with_component(mut self, component: ComponentId) -> Self {
        self.component = Some(component);
        self
    }

    /// Add context
    #[inline(always)]
    pub fn with_context(mut self, key: impl Into<String>, value: f64) -> Self {
        self.context.push((key.into(), value));
        self
    }

    /// Set severity
    #[inline(always)]
    pub fn with_severity(mut self, severity: AnomalySeverity) -> Self {
        self.severity = severity;
        self
    }

    /// Is this anomaly critical?
    #[inline(always)]
    pub fn is_critical(&self) -> bool {
        self.severity >= AnomalySeverity::Critical
    }
}
