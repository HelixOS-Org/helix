//! Degradation detection engine.

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::core::ComponentId;

use super::baseline::MetricBaseline;
use super::event::DegradationEvent;
use super::types::{DegradationSeverity, DegradationType};

/// Degradation detector
pub struct DegradationDetector {
    /// Baselines by metric name
    baselines: BTreeMap<String, MetricBaseline>,
    /// Recent events
    events: VecDeque<DegradationEvent>,
    /// Maximum events to keep
    max_events: usize,
    /// EMA alpha (higher = more weight to recent values)
    ema_alpha: f64,
    /// Z-score threshold for detection
    z_threshold: f64,
    /// Percentage threshold
    pct_threshold: f64,
    /// Warmup samples before detection
    warmup_samples: u64,
    /// Total detections
    total_detections: AtomicU64,
}

impl DegradationDetector {
    /// Create a new detector
    pub fn new() -> Self {
        Self {
            baselines: BTreeMap::new(),
            events: VecDeque::new(),
            max_events: 1000,
            ema_alpha: 0.1,
            z_threshold: 2.5,
            pct_threshold: 20.0,
            warmup_samples: 100,
            total_detections: AtomicU64::new(0),
        }
    }

    /// Set z-score threshold
    #[inline(always)]
    pub fn with_z_threshold(mut self, threshold: f64) -> Self {
        self.z_threshold = threshold;
        self
    }

    /// Set percentage threshold
    #[inline(always)]
    pub fn with_pct_threshold(mut self, threshold: f64) -> Self {
        self.pct_threshold = threshold;
        self
    }

    /// Record a metric value
    pub fn record(
        &mut self,
        name: &str,
        value: f64,
        degradation_type: DegradationType,
        component: Option<ComponentId>,
    ) -> Option<DegradationEvent> {
        let baseline = self
            .baselines
            .entry(name.into())
            .or_insert_with(|| MetricBaseline::new(value));

        baseline.update(value, self.ema_alpha);

        // Skip detection during warmup
        if baseline.samples < self.warmup_samples {
            return None;
        }

        // Check for degradation
        let z_score = baseline.z_score();
        let degradation_pct = baseline.degradation();

        // For performance metrics, positive degradation is bad
        // For throughput, negative degradation is bad
        let is_degraded = match degradation_type {
            DegradationType::Throughput => {
                z_score < -self.z_threshold || degradation_pct < -self.pct_threshold
            }
            _ => z_score > self.z_threshold || degradation_pct > self.pct_threshold,
        };

        if is_degraded {
            self.total_detections.fetch_add(1, Ordering::Relaxed);

            let mut event =
                DegradationEvent::new(degradation_type, baseline.mean, baseline.current);

            if let Some(comp) = component {
                event = event.with_component(comp);
            }

            event = event.with_trend(z_score);

            // Add to events
            if self.events.len() >= self.max_events {
                self.events.pop_front();
            }
            self.events.push_back(event.clone());

            Some(event)
        } else {
            None
        }
    }

    /// Record latency
    #[inline(always)]
    pub fn record_latency(
        &mut self,
        name: &str,
        latency: u64,
        component: Option<ComponentId>,
    ) -> Option<DegradationEvent> {
        self.record(name, latency as f64, DegradationType::Performance, component)
    }

    /// Record error rate
    #[inline]
    pub fn record_error_rate(
        &mut self,
        name: &str,
        errors: u64,
        total: u64,
        component: Option<ComponentId>,
    ) -> Option<DegradationEvent> {
        let rate = if total > 0 {
            (errors as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        self.record(name, rate, DegradationType::ErrorRate, component)
    }

    /// Record memory usage
    #[inline]
    pub fn record_memory(
        &mut self,
        name: &str,
        used: u64,
        total: u64,
        component: Option<ComponentId>,
    ) -> Option<DegradationEvent> {
        let pct = if total > 0 {
            (used as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        self.record(name, pct, DegradationType::MemoryUsage, component)
    }

    /// Record throughput
    #[inline(always)]
    pub fn record_throughput(
        &mut self,
        name: &str,
        ops_per_sec: f64,
        component: Option<ComponentId>,
    ) -> Option<DegradationEvent> {
        self.record(name, ops_per_sec, DegradationType::Throughput, component)
    }

    /// Get recent events
    #[inline(always)]
    pub fn events(&self) -> &[DegradationEvent] {
        &self.events
    }

    /// Get events by severity
    #[inline]
    pub fn events_by_severity(&self, severity: DegradationSeverity) -> Vec<&DegradationEvent> {
        self.events
            .iter()
            .filter(|e| e.severity >= severity)
            .collect()
    }

    /// Get current degradation for a metric
    #[inline(always)]
    pub fn current_degradation(&self, name: &str) -> Option<f64> {
        self.baselines.get(name).map(|b| b.degradation())
    }

    /// Get baseline for a metric
    #[inline(always)]
    pub fn baseline(&self, name: &str) -> Option<(f64, f64)> {
        self.baselines.get(name).map(|b| (b.mean, b.std_dev))
    }

    /// Reset baseline for a metric
    #[inline]
    pub fn reset_baseline(&mut self, name: &str) {
        if let Some(baseline) = self.baselines.get_mut(name) {
            baseline.mean = baseline.current;
            baseline.std_dev = 0.0;
            baseline.samples = 1;
        }
    }

    /// Clear all baselines
    #[inline(always)]
    pub fn clear(&mut self) {
        self.baselines.clear();
        self.events.clear();
    }

    /// Get statistics
    pub fn stats(&self) -> DegradationStats {
        let mut by_severity = [0u32; 5];

        for event in &self.events {
            by_severity[event.severity as usize] += 1;
        }

        DegradationStats {
            total_metrics: self.baselines.len(),
            total_events: self.events.len(),
            total_detections: self.total_detections.load(Ordering::Relaxed),
            minor: by_severity[0],
            moderate: by_severity[1],
            significant: by_severity[2],
            severe: by_severity[3],
            critical: by_severity[4],
        }
    }
}

impl Default for DegradationDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Degradation statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct DegradationStats {
    /// Total metrics tracked
    pub total_metrics: usize,
    /// Total events in history
    pub total_events: usize,
    /// Total detections ever
    pub total_detections: u64,
    /// Minor count
    pub minor: u32,
    /// Moderate count
    pub moderate: u32,
    /// Significant count
    pub significant: u32,
    /// Severe count
    pub severe: u32,
    /// Critical count
    pub critical: u32,
}
