//! Driver health monitoring.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::metrics::DriverMetrics;
use super::types::DriverId;
use crate::core::NexusTimestamp;

// ============================================================================
// DRIVER HEALTH MONITOR
// ============================================================================

/// Monitors driver health
pub struct DriverHealthMonitor {
    /// Health samples per driver
    samples: BTreeMap<DriverId, Vec<HealthSample>>,
    /// Max samples per driver
    max_samples: usize,
    /// Health scores
    scores: BTreeMap<DriverId, f64>,
    /// Health events
    events: Vec<HealthEvent>,
    /// Max events
    max_events: usize,
}

/// Health sample
#[derive(Debug, Clone, Copy)]
struct HealthSample {
    /// Timestamp
    #[allow(dead_code)]
    timestamp: u64,
    /// Health score (0.0 - 1.0)
    score: f64,
    /// Error occurred
    #[allow(dead_code)]
    error: bool,
}

/// Health event
#[derive(Debug, Clone)]
pub struct HealthEvent {
    /// Driver ID
    pub driver_id: DriverId,
    /// Event type
    pub event_type: HealthEventType,
    /// Timestamp
    pub timestamp: NexusTimestamp,
    /// Details
    pub details: String,
}

/// Health event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthEventType {
    /// Health improved
    Improved,
    /// Health degraded
    Degraded,
    /// Driver crashed
    Crashed,
    /// Driver recovered
    Recovered,
    /// Warning threshold
    Warning,
    /// Critical threshold
    Critical,
}

impl DriverHealthMonitor {
    /// Create new monitor
    pub fn new() -> Self {
        Self {
            samples: BTreeMap::new(),
            max_samples: 1000,
            scores: BTreeMap::new(),
            events: Vec::new(),
            max_events: 10000,
        }
    }

    /// Record health sample
    pub fn record(&mut self, driver_id: DriverId, metrics: &DriverMetrics) -> Option<HealthEvent> {
        // Calculate health score
        let success_rate = metrics.success_rate();
        let latency_factor = if metrics.avg_latency_ns > 1_000_000.0 {
            0.8
        } else {
            1.0
        };
        let score = success_rate * latency_factor;

        let sample = HealthSample {
            timestamp: NexusTimestamp::now().raw(),
            score,
            error: metrics.failure_rate() > 0.01,
        };

        let samples = self.samples.entry(driver_id).or_default();
        samples.push(sample);
        if samples.len() > self.max_samples {
            samples.remove(0);
        }

        // Update score
        let prev_score = self.scores.get(&driver_id).copied().unwrap_or(1.0);
        self.scores.insert(driver_id, score);

        // Check for events
        self.check_health_change(driver_id, prev_score, score)
    }

    /// Check for health change events
    fn check_health_change(
        &mut self,
        driver_id: DriverId,
        prev: f64,
        current: f64,
    ) -> Option<HealthEvent> {
        if current < 0.5 && prev >= 0.5 {
            Some(self.record_event(
                driver_id,
                HealthEventType::Critical,
                alloc::format!("Health critical: {:.1}%", current * 100.0),
            ))
        } else if current < 0.8 && prev >= 0.8 {
            Some(self.record_event(
                driver_id,
                HealthEventType::Degraded,
                alloc::format!("Health degraded: {:.1}%", current * 100.0),
            ))
        } else if current >= 0.9 && prev < 0.9 {
            Some(self.record_event(
                driver_id,
                HealthEventType::Improved,
                alloc::format!("Health improved: {:.1}%", current * 100.0),
            ))
        } else {
            None
        }
    }

    /// Record health event
    fn record_event(
        &mut self,
        driver_id: DriverId,
        event_type: HealthEventType,
        details: String,
    ) -> HealthEvent {
        let event = HealthEvent {
            driver_id,
            event_type,
            timestamp: NexusTimestamp::now(),
            details,
        };

        self.events.push(event.clone());
        if self.events.len() > self.max_events {
            self.events.remove(0);
        }

        event
    }

    /// Get current health score
    pub fn get_score(&self, driver_id: DriverId) -> f64 {
        self.scores.get(&driver_id).copied().unwrap_or(1.0)
    }

    /// Get health trend
    pub fn get_trend(&self, driver_id: DriverId) -> f64 {
        let samples = match self.samples.get(&driver_id) {
            Some(s) if s.len() >= 10 => s,
            _ => return 0.0,
        };

        let len = samples.len();
        let first_half: f64 =
            samples[..len / 2].iter().map(|s| s.score).sum::<f64>() / (len / 2) as f64;
        let second_half: f64 =
            samples[len / 2..].iter().map(|s| s.score).sum::<f64>() / (len - len / 2) as f64;

        second_half - first_half
    }

    /// Get unhealthy drivers
    pub fn unhealthy_drivers(&self) -> Vec<(DriverId, f64)> {
        self.scores
            .iter()
            .filter(|&(_, score)| *score < 0.8)
            .map(|(&id, &score)| (id, score))
            .collect()
    }

    /// Get recent events
    pub fn recent_events(&self, n: usize) -> &[HealthEvent] {
        let start = self.events.len().saturating_sub(n);
        &self.events[start..]
    }
}

impl Default for DriverHealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}
