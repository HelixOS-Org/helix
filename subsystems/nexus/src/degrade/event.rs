//! Degradation events.

use alloc::string::String;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::core::{ComponentId, NexusTimestamp};

use super::types::{DegradationSeverity, DegradationType};

/// A detected degradation event
#[derive(Debug, Clone)]
pub struct DegradationEvent {
    /// Unique ID
    pub id: u64,
    /// Component affected
    pub component: Option<ComponentId>,
    /// Type of degradation
    pub degradation_type: DegradationType,
    /// Severity
    pub severity: DegradationSeverity,
    /// Baseline value
    pub baseline: f64,
    /// Current value
    pub current: f64,
    /// Degradation percentage
    pub degradation_pct: f64,
    /// Trend (positive = getting worse)
    pub trend: f64,
    /// Timestamp
    pub timestamp: NexusTimestamp,
    /// Description
    pub description: String,
}

impl DegradationEvent {
    /// Create a new event
    pub fn new(degradation_type: DegradationType, baseline: f64, current: f64) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);

        let degradation_pct = if baseline != 0.0 {
            ((current - baseline) / baseline).abs() * 100.0
        } else {
            0.0
        };

        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            component: None,
            degradation_type,
            severity: DegradationSeverity::from_percentage(degradation_pct),
            baseline,
            current,
            degradation_pct,
            trend: 0.0,
            timestamp: NexusTimestamp::now(),
            description: String::new(),
        }
    }

    /// Set component
    #[inline(always)]
    pub fn with_component(mut self, component: ComponentId) -> Self {
        self.component = Some(component);
        self
    }

    /// Set trend
    #[inline(always)]
    pub fn with_trend(mut self, trend: f64) -> Self {
        self.trend = trend;
        self
    }

    /// Set description
    #[inline(always)]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }
}
