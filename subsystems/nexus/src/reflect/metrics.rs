//! Metrics — Cognitive domain telemetry
//!
//! Types for tracking the health and performance of cognitive domains.

use alloc::vec::Vec;

use crate::bus::Domain;
use crate::types::*;

// ============================================================================
// DOMAIN METRICS
// ============================================================================

/// Metrics for a cognitive domain
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct DomainMetrics {
    /// Domain
    pub domain: Domain,
    /// Health score (0-100)
    pub health_score: u8,
    /// Messages processed
    pub messages_processed: u64,
    /// Average latency in microseconds
    pub avg_latency_us: u64,
    /// P99 latency in microseconds
    pub p99_latency_us: u64,
    /// Error rate (0.0 to 1.0)
    pub error_rate: f32,
    /// Queue depth
    pub queue_depth: usize,
    /// Last active tick
    pub last_tick: u64,
    /// Timestamp
    pub timestamp: Timestamp,
}

impl DomainMetrics {
    /// Create new domain metrics
    pub fn new(domain: Domain) -> Self {
        Self {
            domain,
            health_score: 100,
            messages_processed: 0,
            avg_latency_us: 0,
            p99_latency_us: 0,
            error_rate: 0.0,
            queue_depth: 0,
            last_tick: 0,
            timestamp: Timestamp::now(),
        }
    }

    /// Is healthy?
    #[inline(always)]
    pub fn is_healthy(&self) -> bool {
        self.health_score >= 70
    }

    /// Is critical?
    #[inline(always)]
    pub fn is_critical(&self) -> bool {
        self.health_score < 30
    }

    /// Has high latency?
    #[inline(always)]
    pub fn has_high_latency(&self) -> bool {
        self.avg_latency_us > 10000 // 10ms
    }

    /// Has high error rate?
    #[inline(always)]
    pub fn has_high_error_rate(&self) -> bool {
        self.error_rate > 0.1 // 10%
    }

    /// Has queue backlog?
    #[inline(always)]
    pub fn has_queue_backlog(&self) -> bool {
        self.queue_depth > 1000
    }
}

// ============================================================================
// COGNITIVE METRICS
// ============================================================================

/// Overall cognitive system metrics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CognitiveMetrics {
    /// Domain-level metrics
    pub domains: Vec<DomainMetrics>,
    /// Overall health
    pub overall_health: u8,
    /// Total messages per second
    pub messages_per_second: u64,
    /// Total decisions made
    pub decisions_made: u64,
    /// Total actions taken
    pub actions_taken: u64,
    /// Decision accuracy (0.0 to 1.0)
    pub decision_accuracy: f32,
    /// Prediction accuracy (0.0 to 1.0)
    pub prediction_accuracy: f32,
    /// Cognitive load (0.0 to 1.0)
    pub cognitive_load: f32,
    /// Uptime
    pub uptime: Duration,
}

impl CognitiveMetrics {
    /// Create new cognitive metrics
    pub fn new() -> Self {
        Self {
            domains: Vec::new(),
            overall_health: 100,
            messages_per_second: 0,
            decisions_made: 0,
            actions_taken: 0,
            decision_accuracy: 1.0,
            prediction_accuracy: 1.0,
            cognitive_load: 0.0,
            uptime: Duration::ZERO,
        }
    }

    /// Add domain metrics
    #[inline(always)]
    pub fn add_domain(&mut self, metrics: DomainMetrics) {
        self.domains.push(metrics);
        self.recalculate_overall();
    }

    /// Recalculate overall health
    fn recalculate_overall(&mut self) {
        if self.domains.is_empty() {
            self.overall_health = 100;
            return;
        }

        let sum: u32 = self.domains.iter().map(|d| d.health_score as u32).sum();
        self.overall_health = (sum / self.domains.len() as u32) as u8;
    }

    /// Is system healthy?
    #[inline(always)]
    pub fn is_healthy(&self) -> bool {
        self.overall_health >= 70
    }

    /// Get unhealthy domains
    #[inline(always)]
    pub fn unhealthy_domains(&self) -> Vec<&DomainMetrics> {
        self.domains.iter().filter(|d| !d.is_healthy()).collect()
    }
}

impl Default for CognitiveMetrics {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_metrics() {
        let metrics = DomainMetrics::new(Domain::Sense);
        assert!(metrics.is_healthy());
        assert!(!metrics.is_critical());
    }

    #[test]
    fn test_domain_metrics_unhealthy() {
        let mut metrics = DomainMetrics::new(Domain::Reason);
        metrics.health_score = 50;
        metrics.avg_latency_us = 20000;
        metrics.error_rate = 0.2;

        assert!(!metrics.is_healthy());
        assert!(metrics.has_high_latency());
        assert!(metrics.has_high_error_rate());
    }

    #[test]
    fn test_cognitive_metrics() {
        let mut metrics = CognitiveMetrics::new();

        metrics.add_domain(DomainMetrics::new(Domain::Sense));
        metrics.add_domain(DomainMetrics::new(Domain::Reason));

        assert_eq!(metrics.overall_health, 100);
        assert!(metrics.is_healthy());
    }
}
